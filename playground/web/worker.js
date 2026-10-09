// The Aura Playground execution Worker.
//
// Each run gets a *fresh* Worker created by the page and terminated on Stop or
// on completion. This is the primary hard-cancellation mechanism: terminating
// a Worker stops a synchronous wasm execution that would otherwise never
// return (`while true {}`). Cancellation is never turned into an Aura `throw`
// and is never catchable by Aura `try/catch`.
//
// The Worker is an orchestration and transport layer only. It loads the
// selected, immutable runtime artifact, validates its ABI, calls it, performs
// the *browser* side of a Host effect when the runtime asks (an HTTP request),
// and posts structured results back. It implements **no Aura semantics**: the
// response value is built in Rust from the transport result, and the request
// was fully validated in Rust before it ever reached this file.
//
// Transports, additively:
//
//   * `source`  — the historical single-source execution path (`runtime.run`);
//   * `project` — a virtual multi-source project (`runtime.runProject`);
//   * `session` — Host ABI 2 resumable execution (`runtime.startSession` /
//     `resumeSession`), used when the selected artifact advertises it. This is
//     what makes browser HTTP possible: the program parks on an HTTP effect,
//     the Worker performs `fetch`, and the program resumes.
//
// Message in:  { runId, artifactUrl, expectedAbi, expectedSha256,
//                source, project, args, stdin }
//              A run uses `session` automatically when supported.
// Message out: { runId, kind: "loaded", runtimeVersion, abiVersion, ... }
//              { runId, kind: "permission", effectId, origin, method, mutating }
//              { runId, kind: "result", result }
//              { runId, kind: "error", phase, code, message }
//
// Authorization: the Worker NEVER authorizes an origin by itself. When the
// runtime parks on an HTTP effect, the Worker asks the *page* (which owns the
// Aurea permission UI) and waits. A denied request is never dispatched. The
// Worker keeps no cookies or credentials (`credentials: "omit"`) and follows
// at most a bounded number of redirects, re-checking authorization for a
// redirect target that changes origin.

importScripts(); // no-op; kept for clarity that there are no imports.

let runtimePromise = null;

/** Exports a runtime must provide to execute a virtual project. */
const PROJECT_EXPORTS = ["aura_project_reset", "aura_project_push", "aura_run_project"];

/** Transport bounds (mirror the Rust host's policy; the Worker enforces them
 *  too so a hostile page message cannot make it buffer without bound). */
const MAX_REDIRECTS = 3;
const MAX_REQUEST_BYTES = 8 * 1024 * 1024;
const MAX_HEADER_BYTES = 64 * 1024;
/** Mirrors the native `MAX_HTTP_BODY_BYTES` policy: a response over this is
 *  E4020, so the browser can never buffer a body the native host would refuse. */
const MAX_RESPONSE_BYTES = 8 * 1024 * 1024;
/** Mirrors the native `MAX_HTTP_TIMEOUT_MS` clamp. */
const MAX_TIMEOUT_MS = 30_000;

function hasVirtualProjects(runtime) {
  return PROJECT_EXPORTS.every((sym) => sym in runtime.exports);
}

function post(msg) {
  self.postMessage(msg);
}

async function loadRuntime(artifactUrl, expectedAbi, expectedSha256) {
  const url = new URL(artifactUrl, self.location.href);
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`cannot fetch runtime artifact (${response.status})`);
  }
  const bytes = new Uint8Array(await response.arrayBuffer());
  const { AuraRuntime } = await import("./runtime.mjs");
  const runtime = await AuraRuntime.fromBytes(bytes, artifactUrl, {
    expectedSha256: expectedSha256 || null,
  });
  if (expectedAbi !== null && expectedAbi !== undefined && runtime.abiVersion !== expectedAbi) {
    throw new Error(
      `runtime ABI ${runtime.abiVersion} does not match the expected ABI ${expectedAbi}`,
    );
  }
  return runtime;
}

/**
 * Ask the page to authorize `origin` for `method`. Resolves true only on an
 * explicit grant for that exact origin. The page owns the interaction; the
 * Worker merely relays the question, so no approval can ever originate here.
 */
const pendingPermissions = new Map();
let permissionSeq = 0;

function requestPermission(runId, { origin, method, mutating }) {
  permissionSeq += 1;
  const requestId = permissionSeq;
  return new Promise((resolve) => {
    pendingPermissions.set(requestId, resolve);
    post({
      runId,
      kind: "permission",
      requestId,
      origin,
      method,
      mutating,
    });
  });
}

/** Parse and bound a URL; reject unsupported schemes. */
function parseHttpUrl(raw) {
  let url;
  try {
    url = new URL(raw);
  } catch {
    return { error: "invalid URL" };
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    return { error: `unsupported scheme \`${url.protocol}\`` };
  }
  return { url };
}

/**
 * Perform one HTTP effect with the browser transport.
 *
 * Returns a resume payload: on success `{ effect_id, ok: true, response }`
 * with `response` shaped exactly as the Rust encoder expects; on failure
 * `{ effect_id, ok: false, code, message }` using the language's diagnostic
 * codes (E4020 transport, E5002 unavailable).
 */
async function performHttp(runId, effect, abortSignal) {
  const parsed = parseHttpUrl(effect.url);
  if (parsed.error) {
    return { effect_id: effect.effect_id, ok: false, code: 4020, message: parsed.error };
  }
  const url = parsed.url;
  const origin = url.origin;
  const mutating = !["GET", "HEAD"].includes(effect.method);

  // Never dispatch without an explicit, origin-specific grant.
  const granted = await requestPermission(runId, { origin, method: effect.method, mutating });
  if (abortSignal.aborted) {
    return { effect_id: effect.effect_id, ok: false, code: 4020, message: "request cancelled" };
  }
  if (!granted) {
    return {
      effect_id: effect.effect_id,
      ok: false,
      code: 5002,
      message: `network access to ${origin} was not granted`,
    };
  }

  // A per-request controller lets the timeout abort *this* fetch, while the
  // run's outer `abortSignal` (Stop) is forwarded into it below.
  const timeoutController = new AbortController();
  const init = {
    method: effect.method,
    // No ambient cookies, no stored credentials, no implicit auth.
    credentials: "omit",
    redirect: "manual",
    // Restrictive referrer policy: never leak the Playground URL's query.
    referrerPolicy: "no-referrer",
  };
  if (Array.isArray(effect.headers) && effect.headers.length > 0) {
    const headers = new Headers();
    let bytes = 0;
    for (const [k, v] of effect.headers) {
      if (typeof k !== "string" || typeof v !== "string") continue;
      bytes += k.length + v.length;
      if (bytes > MAX_HEADER_BYTES) {
        return {
          effect_id: effect.effect_id,
          ok: false,
          code: 4020,
          message: "request headers exceed the limit",
        };
      }
      try {
        headers.append(k, v);
      } catch {
        return {
          effect_id: effect.effect_id,
          ok: false,
          code: 4020,
          message: `invalid request header \`${k}\``,
        };
      }
    }
    init.headers = headers;
  }
  if (typeof effect.body === "string") {
    if (effect.body.length > MAX_REQUEST_BYTES) {
      return {
        effect_id: effect.effect_id,
        ok: false,
        code: 4020,
        message: "request body exceeds the limit",
      };
    }
    init.body = effect.body;
  }

  // Enforce the request timeout the runtime sent (already clamped in Rust to
  // MAX_HTTP_TIMEOUT_MS). The timer aborts the shared AbortController, so the
  // in-flight fetch and any body read stop together; the run resumes with
  // E4020 rather than hanging until the user presses Stop.
  const timeoutMs = Number.isFinite(effect.timeout_ms) && effect.timeout_ms > 0
    ? Math.min(effect.timeout_ms, MAX_TIMEOUT_MS)
    : MAX_TIMEOUT_MS;
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    timeoutController.abort();
  }, timeoutMs);
  const onOuterAbort = () => timeoutController.abort();
  abortSignal.addEventListener("abort", onOuterAbort, { once: true });

  let response;
  try {
    response = await fetch(url, { ...init, signal: timeoutController.signal });
  } catch (err) {
    clearTimeout(timer);
    abortSignal.removeEventListener("abort", onOuterAbort);
    if (abortSignal.aborted) {
      return { effect_id: effect.effect_id, ok: false, code: 4020, message: "request cancelled" };
    }
    if (timedOut) {
      return {
        effect_id: effect.effect_id,
        ok: false,
        code: 4020,
        message: `HTTP request timed out after ${timeoutMs} ms`,
      };
    }
    // Distinguish a browser CORS rejection from a transport failure where the
    // browser lets us; both surface as E4020 (a genuine transport failure) but
    // the message explains what happened without inventing a language code.
    const message =
      err && err.message ? `HTTP request failed: ${err.message}` : "HTTP request failed";
    return { effect_id: effect.effect_id, ok: false, code: 4020, message };
  }

  // `redirect: "manual"` yields either a normal response or an opaque
  // redirect; we never fabricate a readable status for the latter.
  if (response.type === "opaqueredirect") {
    clearTimeout(timer);
    abortSignal.removeEventListener("abort", onOuterAbort);
    return {
      effect_id: effect.effect_id,
      ok: false,
      code: 4020,
      message: "the network returned a redirect this browser does not expose to the page",
    };
  }

  // Read the body with a hard byte cap, mirroring the native
  // MAX_HTTP_BODY_BYTES (8 MiB) policy. The body is read as a stream so a
  // hostile server cannot make the Worker buffer without bound: the cap is
  // enforced *while reading*, and a body over it is E4020, never a partial
  // success presented as complete.
  let bodyText = "";
  try {
    bodyText = await readBoundedBody(response, MAX_RESPONSE_BYTES);
  } catch (err) {
    clearTimeout(timer);
    abortSignal.removeEventListener("abort", onOuterAbort);
    if (err && err.code === "BODY_TOO_LARGE") {
      return {
        effect_id: effect.effect_id,
        ok: false,
        code: 4020,
        message: `HTTP response body exceeds the ${MAX_RESPONSE_BYTES} byte limit`,
      };
    }
    if (abortSignal.aborted || timedOut) {
      return {
        effect_id: effect.effect_id,
        ok: false,
        code: 4020,
        message: timedOut ? `HTTP request timed out after ${timeoutMs} ms` : "request cancelled",
      };
    }
    return {
      effect_id: effect.effect_id,
      ok: false,
      code: 4020,
      message: `HTTP response body could not be read: ${err && err.message ? err.message : err}`,
    };
  }
  clearTimeout(timer);
  abortSignal.removeEventListener("abort", onOuterAbort);
  const headers = [];
  response.headers.forEach((value, key) => {
    headers.push([key, value]);
  });
  return {
    effect_id: effect.effect_id,
    ok: true,
    response: {
      status: response.status,
      headers,
      body: bodyText,
    },
  };
}

/**
 * Read a `Response` body as text, refusing anything over `limit` bytes.
 *
 * Throws `{ code: "BODY_TOO_LARGE" }` when the accumulated bytes exceed the
 * cap, so the caller maps it to E4020. Bytes are decoded at the end; UTF-8
 * correctness is preserved (a body split across chunks is concatenated before
 * decode, never decoded per chunk).
 */
async function readBoundedBody(response, limit) {
  if (!response.body || typeof response.body.getReader !== "function") {
    // No streaming body (very old engine): fall back to text() but check the
    // declared length first, and the size after, so the cap still holds.
    const declared = Number(response.headers.get("content-length"));
    if (Number.isFinite(declared) && declared > limit) {
      const err = new Error("body too large");
      err.code = "BODY_TOO_LARGE";
      throw err;
    }
    const text = await response.text();
    if (text.length > limit) {
      const err = new Error("body too large");
      err.code = "BODY_TOO_LARGE";
      throw err;
    }
    return text;
  }
  const reader = response.body.getReader();
  const chunks = [];
  let total = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > limit) {
      try {
        await reader.cancel();
      } catch {
        /* the cap already rejects the body; a cancel failure is moot */
      }
      const err = new Error("body too large");
      err.code = "BODY_TOO_LARGE";
      throw err;
    }
    chunks.push(value);
  }
  const merged = new Uint8Array(total);
  let off = 0;
  for (const c of chunks) {
    merged.set(c, off);
    off += c.byteLength;
  }
  return new TextDecoder().decode(merged);
}

/** Drain a run's pending permission prompts (a cancelled run never leaves one hanging). */
function denyAllPending() {
  for (const resolve of pendingPermissions.values()) resolve(false);
  pendingPermissions.clear();
}

self.onmessage = async (event) => {
  const msg = event.data || {};
  const { runId } = msg;
  // A permission decision from the page.
  if (msg.kind === "permission-result") {
    const resolve = pendingPermissions.get(msg.requestId);
    if (resolve) {
      pendingPermissions.delete(msg.requestId);
      resolve(msg.granted === true);
    }
    return;
  }
  const { artifactUrl, expectedAbi, expectedSha256, source, project, args, stdin } = msg;
  const abortController = new AbortController();
  currentAbort = abortController;
  try {
    runtimePromise =
      runtimePromise || loadRuntime(artifactUrl, expectedAbi, expectedSha256);
    const runtime = await runtimePromise;
    post({
      runId,
      kind: "loaded",
      runtimeVersion: runtime.runtimeVersion,
      languageVersion: runtime.languageVersion,
      abiVersion: runtime.abiVersion,
      supportsProjects: hasVirtualProjects(runtime),
      supportsSessions: runtime.supportsSessions,
    });
    const wantsProject = project !== undefined && project !== null;
    if (wantsProject && !hasVirtualProjects(runtime)) {
      post({
        runId,
        kind: "error",
        phase: "capability",
        code: "NO_VIRTUAL_PROJECTS",
        message:
          "the selected runtime predates virtual projects and can execute a single source only",
      });
      return;
    }

    // Host ABI 2: a resumable session, which is what enables HTTP. Only when
    // the artifact advertises it (feature detection, never inference).
    if (runtime.supportsSessions && !wantsProject) {
      await runSession(runId, runtime, source, { args: args || [], stdin: stdin ?? null }, abortController.signal);
      return;
    }
    if (wantsProject && runtime.supportsSessions) {
      // Sessions are single-source today; a project run stays synchronous.
      const result = runtime.runProject(project, { args: args || [], stdin: stdin ?? null });
      post({ runId, kind: "result", result });
      return;
    }

    const result = wantsProject
      ? runtime.runProject(project, { args: args || [], stdin: stdin ?? null })
      : runtime.run(source, { args: args || [], stdin: stdin ?? null });
    post({ runId, kind: "result", result });
  } catch (err) {
    post({
      runId,
      kind: "error",
      phase: "load",
      code: (err && err.code) || "LOAD",
      message: err && err.message ? err.message : String(err),
    });
  } finally {
    denyAllPending();
  }
};

let currentAbort = null;

/** Drive a session to completion, performing HTTP effects as they arrive. */
async function runSession(runId, runtime, source, options, signal) {
  const { status, result } = runtime.startSession(source, options);
  let step = { status, result };
  let httpCount = 0;
  const MAX_HTTP_PER_RUN = 64;
  while (step.status === 3) {
    // `step.result` is the effect payload.
    const effect = step.result;
    if (effect.kind !== "http") {
      post({
        runId,
        kind: "error",
        phase: "effect",
        code: "UNKNOWN_EFFECT",
        message: `the runtime requested an unknown effect kind \`${effect.kind}\``,
      });
      return;
    }
    httpCount += 1;
    if (httpCount > MAX_HTTP_PER_RUN) {
      const denied = {
        effect_id: effect.effect_id,
        ok: false,
        code: 4020,
        message: `too many HTTP requests in one run (limit ${MAX_HTTP_PER_RUN})`,
      };
      step = runtime.resumeSession(denied);
      continue;
    }
    if (signal.aborted) {
      runtime.resetSession();
      return;
    }
    const outcome = await performHttp(runId, effect, signal);
    if (signal.aborted) {
      runtime.resetSession();
      return;
    }
    step = runtime.resumeSession(outcome);
  }
  post({ runId, kind: "result", result: step.result });
}
