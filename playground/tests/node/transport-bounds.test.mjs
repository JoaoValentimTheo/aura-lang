// Transport byte-accounting tests for the Playground Worker.
//
// The Worker caps a request body (8 MiB) and its request headers (64 KiB) so a
// hostile page message cannot make it buffer without bound. Those caps were
// originally measured with JavaScript `.length`, which counts UTF-16 code
// units. `"一"` is one unit but three UTF-8 bytes, so a body the cap called
// 8 MiB actually occupied 24 MiB and the cap never engaged.
//
// A third bound has no native counterpart at all: a run may issue at most 64
// HTTP requests. That counter lives in `runSession`, so the last block below
// drives it with a stub runtime instead of calling `performHttp` directly.
//
// The Worker is a classic (non-module) worker started by `new Worker(...)`, so
// it cannot be imported. These tests evaluate the *real* file inside a sandbox
// that supplies the handful of globals it touches, then drive the shipped
// functions directly. Nothing here reimplements the arithmetic under test.
//
// Usage: node playground/tests/node/transport-bounds.test.mjs

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const here = dirname(fileURLToPath(import.meta.url));
const workerPath = resolve(here, "../../web/worker.js");
const source = readFileSync(workerPath, "utf8");

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) {
    passed += 1;
  } else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

const noNetwork = async () => {
  throw new Error("no test may reach the network");
};

/**
 * Load the shipped worker file in a sandbox.
 *
 * Returns the internals these tests drive plus the messages the Worker posted,
 * so a permission request can be granted the way the page grants it.
 */
function loadWorker({ fetchImpl = noNetwork, autoGrant = false } = {}) {
  const posted = [];
  const self = {
    onmessage: null,
    location: { href: "http://localhost/web/worker.js" },
    postMessage(msg) {
      posted.push(msg);
      // Grant through the Worker's own message handler, so the permission
      // round-trip under test is the real one. The prompt is posted after the
      // resolver is registered, so answering from inside `postMessage` is
      // safe and mirrors a page that replies immediately.
      if (autoGrant && msg.kind === "permission" && typeof self.onmessage === "function") {
        void self.onmessage({
          data: { kind: "permission-result", requestId: msg.requestId, granted: true },
        });
      }
    },
  };
  const sandbox = {
    self,
    console,
    TextEncoder,
    TextDecoder,
    Uint8Array,
    ArrayBuffer,
    AbortController,
    URL,
    Headers,
    setTimeout,
    clearTimeout,
    fetch: fetchImpl,
    // The worker file calls this once as an explicit "no imports" marker.
    importScripts() {},
  };
  sandbox.globalThis = sandbox;
  vm.createContext(sandbox);
  vm.runInContext(
    `${source}\n;globalThis.__test = { performHttp, runSession, utf8Bytes };`,
    sandbox,
    { filename: workerPath },
  );
  return { internals: sandbox.__test, self: sandbox.self, posted };
}

/**
 * Start `performHttp` and approve its permission prompt.
 *
 * `requestPermission` posts its question synchronously, before the first
 * suspension, so the prompt is already visible when this returns.
 */
function startHttp(worker, runId, effect) {
  const pending = worker.internals.performHttp(runId, effect, new AbortController().signal);
  const prompt = worker.posted.find((m) => m.kind === "permission" && m.runId === runId);
  if (prompt) {
    void worker.self.onmessage({
      data: { kind: "permission-result", requestId: prompt.requestId, granted: true },
    });
  }
  return pending;
}

const CAP_BYTES = 8 * 1024 * 1024;
const CAP_HEADER_BYTES = 64 * 1024;

// --- the measurement itself ----------------------------------------------
{
  const { internals } = loadWorker();
  check("ascii is one byte per character", internals.utf8Bytes("abc") === 3);
  check("accented latin is two bytes", internals.utf8Bytes("é") === 2, String(internals.utf8Bytes("é")));
  check("CJK is three bytes", internals.utf8Bytes("一") === 3, String(internals.utf8Bytes("一")));
  check(
    "astral is four bytes from two UTF-16 units",
    internals.utf8Bytes("😀") === 4 && "😀".length === 2,
    `bytes=${internals.utf8Bytes("😀")} units=${"😀".length}`,
  );
}

// --- the headroom `.length` used to grant --------------------------------
// 3 Mi code units of CJK passes an 8 MiB unit-based cap while its UTF-8
// encoding is exactly 3x that: the case the fix must reject.
const smuggledBody = "一".repeat(3 * 1024 * 1024);
{
  const { internals } = loadWorker();
  check(
    "CJK body slips under the old unit-based cap",
    smuggledBody.length < CAP_BYTES,
    `units=${smuggledBody.length}`,
  );
  check(
    "CJK body is three times larger in UTF-8 bytes",
    internals.utf8Bytes(smuggledBody) === 3 * smuggledBody.length,
    String(internals.utf8Bytes(smuggledBody)),
  );
}

// --- an oversized non-ASCII body is rejected, not dispatched -------------
{
  // Count dispatches: a rejection must happen *before* `fetch`, so a transport
  // error standing in for the cap would be caught here rather than pass.
  const dispatched = [];
  const worker = loadWorker({
    fetchImpl: async (url) => {
      dispatched.push(String(url));
      throw new Error("no test may reach the network");
    },
  });
  const result = await startHttp(worker, "run-body", {
    effect_id: "eff-body",
    method: "POST",
    url: "http://127.0.0.1:1/echo",
    headers: [],
    body: smuggledBody,
    timeout_ms: 1000,
  });
  check("body over the byte cap is E4020", result.ok === false && result.code === 4020, JSON.stringify(result));
  check(
    "body rejection names the body",
    typeof result.message === "string" && /body/i.test(result.message),
    result.message,
  );
  check("rejected body is never dispatched", dispatched.length === 0, JSON.stringify(dispatched));
  check("rejected body posts only the permission prompt", worker.posted.length === 1, JSON.stringify(worker.posted));
}

// --- an oversized non-ASCII header is rejected, not truncated -----------
{
  // A header value must be a ByteString, so the reachable undercount is Latin-1:
  // `"é"` is one code unit but two UTF-8 bytes. 40 Ki of them is 40 Ki units,
  // under the 64 KiB cap, while the encoded value is 80 KiB and over it.
  const value = "é".repeat(40 * 1024);
  check("header value slips under the old unit-based cap", value.length < CAP_HEADER_BYTES);
  check("header value is twice as large encoded", loadWorker().internals.utf8Bytes(value) === 2 * value.length);
  const dispatched = [];
  const worker = loadWorker({
    fetchImpl: async (url) => {
      dispatched.push(String(url));
      throw new Error("no test may reach the network");
    },
  });
  const result = await startHttp(worker, "run-header", {
    effect_id: "eff-header",
    method: "GET",
    url: "http://127.0.0.1:1/echo",
    headers: [["x-long", value]],
    body: null,
    timeout_ms: 1000,
  });
  check(
    "header over the byte cap is E4020",
    result.ok === false && result.code === 4020,
    JSON.stringify(result),
  );
  check(
    "header rejection names the headers",
    typeof result.message === "string" && /header/i.test(result.message),
    result.message,
  );
  check("rejected header is never dispatched", dispatched.length === 0, JSON.stringify(dispatched));
}

// --- an ASCII body over the cap is still rejected cheaply ----------------
{
  const dispatched = [];
  const worker = loadWorker({
    fetchImpl: async (url) => {
      dispatched.push(String(url));
      throw new Error("no test may reach the network");
    },
  });
  const result = await startHttp(worker, "run-ascii", {
    effect_id: "eff-ascii",
    method: "POST",
    url: "http://127.0.0.1:1/echo",
    headers: [],
    body: "a".repeat(CAP_BYTES + 1),
    timeout_ms: 1000,
  });
  check(
    "ASCII body over the cap is E4020",
    result.ok === false && result.code === 4020,
    JSON.stringify(result),
  );
  check("ASCII body over the cap is never dispatched", dispatched.length === 0, JSON.stringify(dispatched));
}

// --- a small non-ASCII body still passes --------------------------------
{
  const seen = [];
  const okResponse = () => ({
    status: 200,
    type: "basic",
    ok: true,
    headers: new Headers([["content-type", "text/plain"]]),
    body: null,
    arrayBuffer: async () => new TextEncoder().encode("4").buffer,
  });
  const worker = loadWorker({
    fetchImpl: async (url, init) => {
      seen.push({ url: String(url), init });
      return okResponse();
    },
  });
  const result = await startHttp(worker, "run-small", {
    effect_id: "eff-small",
    method: "POST",
    url: "http://127.0.0.1:1/echo",
    headers: [["x-name", "é"]],
    body: "日本語",
    timeout_ms: 1000,
  });
  check("small non-ASCII body is dispatched", seen.length === 1, String(seen.length));
  check("small non-ASCII body keeps its text", seen[0]?.init?.body === "日本語", seen[0]?.init?.body);
  check("small non-ASCII body succeeds", result.ok === true, JSON.stringify(result));
  check(
    "success carries byte-accurate body_bytes",
    Array.isArray(result.response?.body_bytes) && result.response.body_bytes.length === 1,
    JSON.stringify(result.response?.body_bytes),
  );
}

// --- the per-run request budget is enforced and reported ------------------
// `MAX_HTTP_PER_RUN` is a Playground denial-of-service bound with no native
// counterpart: a run may issue 64 requests, and the 65th must be refused with
// E4020 *without* being dispatched. The stub runtime below parks on an HTTP
// effect forever, so the limit is the only thing that can end the loop.
{
  const PER_RUN_LIMIT = 64;
  let performed = 0;
  const worker = loadWorker({
    autoGrant: true,
    fetchImpl: async () => {
      performed += 1;
      return {
        status: 200,
        type: "basic",
        ok: true,
        headers: new Headers([["content-type", "text/plain"]]),
        body: null,
        arrayBuffer: async () => new ArrayBuffer(0),
      };
    },
  });

  // Park on an HTTP effect forever, but stop after a few effects past the
  // budget. Without that bound, removing the guard would hang the test instead
  // of failing it, and a hang is not a regression signal.
  const PARK_LIMIT = PER_RUN_LIMIT + 5;
  let parked = 0;
  const deniedEffects = [];
  const done = () => ({ status: 0, result: { status: 0, stdout: "", result: null, diagnostics: [] } });
  const httpEffect = () => ({
    status: 3,
    result: {
      kind: "http",
      effect_id: `eff-${parked}`,
      method: "GET",
      url: "http://127.0.0.1:1/r",
      headers: [],
      body: null,
    },
  });
  const stubRuntime = {
    startSession: () => httpEffect(),
    resumeSession: (payload) => {
      if (payload.ok === false) deniedEffects.push(payload);
      parked += 1;
      // The denial is terminal for this stub: the Worker resumes once more with
      // the 4020 and the run finishes.
      if (deniedEffects.length > 0 || parked >= PARK_LIMIT) return done();
      return httpEffect();
    },
    resetSession: () => {},
  };

  await worker.internals.runSession(
    "run-limit",
    stubRuntime,
    "main { }",
    {},
    new AbortController().signal,
  );

  check(
    "a run may dispatch exactly the per-run request budget",
    performed === PER_RUN_LIMIT,
    `dispatched=${performed}`,
  );
  check(
    "the request past the budget is denied, not dispatched",
    deniedEffects.length === 1,
    JSON.stringify(deniedEffects.map((d) => d.message)),
  );
  check(
    "the over-budget denial is E4020",
    deniedEffects[0]?.code === 4020,
    String(deniedEffects[0]?.code),
  );
  check(
    "the over-budget denial reports the limit",
    /too many HTTP requests in one run \(limit 64\)/.test(deniedEffects[0]?.message ?? ""),
    deniedEffects[0]?.message,
  );
  check(
    "the run still completes after the denial",
    worker.posted.some((m) => m.kind === "result"),
    JSON.stringify(worker.posted.filter((m) => m.kind === "error")),
  );
}

process.stdout.write(`\ntransport-bounds: ${passed} passed, ${failed} failed\n`);
process.exit(failed ? 1 : 0);
