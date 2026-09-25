// The Aura Playground orchestration layer.
//
// This file contains no Aura semantics. It:
//   * applies a host site's selected-example handoff once, then leaves the
//     editor as the single authoritative source of execution;
//   * resolves the versioned runtime manifest and lets the user pick a version;
//   * spawns a fresh Worker per run so execution is isolated from the UI thread;
//   * terminates the Worker on Stop or completion (hard cancellation);
//   * ignores stale messages by execution generation id;
//   * renders the structured ExecutionResult.
//
// The version selector is real: the chosen entry's immutable artifact URL is
// what the Worker fetches and executes.

const els = {
  version: document.getElementById("version"),
  run: document.getElementById("run"),
  stop: document.getElementById("stop"),
  source: document.getElementById("source"),
  args: document.getElementById("args"),
  stdin: document.getElementById("stdin"),
  stdout: document.getElementById("stdout"),
  diagnostics: document.getElementById("diagnostics"),
  status: document.getElementById("status"),
  runtimeNote: document.getElementById("runtime-note"),
};

const DEFAULT_SOURCE = `fn main() {
    print("hello, Aura")

    let xs = [1, 2, 3, 4, 5]
    let evens = xs.filter((x) -> x % 2 == 0)
    print(evens.map((x) -> x * x))
}
`;

// The ways a host site (the Aura website) can hand a selected example to the
// Playground:
//
//   1. *in the navigation itself* — `?source=…&args=…&stdin=…` on the
//      Playground URL. This is the authoritative channel: it cannot be lost
//      to a per-tab side effect, a modifier click that opens a new tab, or a
//      click that lands before the host page's script has run.
//   2. `sessionStorage` under HANDOFF_KEY, JSON `{ source, args, stdin }`; a
//      bare source string is also accepted for backwards compatibility.
//
// Whichever channel delivers, the payload is consumed once: the editor takes
// ownership of the program and the payload is removed, so a later manual
// reload starts fresh.
const HANDOFF_KEY = "aura-playground-source";
const HANDOFF_PARAMS = ["source", "args", "stdin"];

let manifest = null;
let currentRun = null;
let generation = 0;

function parseArgsParam(raw) {
  if (raw == null || raw === "") return [];
  try {
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) return parsed.map(String);
  } catch {
    /* malformed metadata is ignored; the source still loads */
  }
  return [];
}

/**
 * Read the example payload carried by the navigation itself.
 *
 * On success the payload is consumed: the editor becomes the single
 * authoritative source of execution, and the URL is stripped of it so the
 * address bar never lies about what will run after the user edits.
 */
function takeUrlHandoff() {
  let params;
  try {
    params = new URLSearchParams(window.location.search);
  } catch {
    return null;
  }
  if (!params.has("source")) return null;
  const handoff = {
    source: params.get("source"),
    args: parseArgsParam(params.get("args")),
    stdin: params.get("stdin") || "",
  };
  try {
    const url = new URL(window.location.href);
    for (const name of HANDOFF_PARAMS) url.searchParams.delete(name);
    history.replaceState(null, "", `${url.pathname}${url.search}${url.hash}`);
  } catch {
    /* history is unavailable; the payload is already consumed either way */
  }
  return handoff;
}

/** Read (and clear) the legacy `sessionStorage` handoff. */
function takeSessionHandoff() {
  let raw = null;
  try {
    raw = sessionStorage.getItem(HANDOFF_KEY);
    sessionStorage.removeItem(HANDOFF_KEY);
  } catch {
    return null;
  }
  if (raw == null) return null;
  try {
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === "object") {
      return {
        source: typeof parsed.source === "string" ? parsed.source : null,
        args: Array.isArray(parsed.args) ? parsed.args.map(String) : [],
        stdin: typeof parsed.stdin === "string" ? parsed.stdin : "",
      };
    }
  } catch {
    /* not JSON: fall through to the legacy bare-source form */
  }
  return { source: raw, args: [], stdin: "" };
}

function takeHandoff() {
  // Any leftover storage copy is dropped even when the URL carried the
  // payload, so a stale handoff can never survive to a later, unrelated load.
  const fromUrl = takeUrlHandoff();
  const fromStorage = takeSessionHandoff();
  return fromUrl || fromStorage;
}

function applyHandoff() {
  const handoff = takeHandoff();
  if (!handoff) return;
  if (handoff.source != null) els.source.value = handoff.source;
  els.args.value = handoff.args.join("\n");
  els.stdin.value = handoff.stdin;
}

function setStatus(text, kind) {
  els.status.textContent = text;
  els.status.className = `status${kind ? ` ${kind}` : ""}`;
}

function clearOutput() {
  els.stdout.textContent = "";
  els.diagnostics.replaceChildren();
}

function showNote(text) {
  if (!text) {
    els.runtimeNote.hidden = true;
    els.runtimeNote.textContent = "";
    return;
  }
  els.runtimeNote.hidden = false;
  els.runtimeNote.textContent = text;
}

function renderDiagnostics(diagnostics) {
  els.diagnostics.replaceChildren();
  if (!diagnostics || diagnostics.length === 0) {
    const li = document.createElement("li");
    li.className = "empty";
    li.textContent = "No diagnostics.";
    els.diagnostics.append(li);
    return;
  }
  for (const d of diagnostics) {
    const li = document.createElement("li");
    const code = document.createElement("span");
    code.className = "diag-code";
    code.textContent = d.code_text || `E${String(d.code).padStart(4, "0")}`;
    const loc = document.createElement("span");
    loc.className = "diag-loc";
    loc.textContent = `  ${d.line}:${d.column}`;
    const msg = document.createElement("div");
    msg.textContent = d.message;
    li.append(code, loc, msg);
    els.diagnostics.append(li);
  }
}

function selectedVersion() {
  if (!manifest) return null;
  const id = els.version.value;
  return manifest.versions.find((v) => v.id === id) || null;
}

async function loadManifest() {
  const response = await fetch("./runtimes/manifest.json");
  if (!response.ok) throw new Error(`cannot load manifest (${response.status})`);
  manifest = await response.json();
  els.version.replaceChildren();
  for (const v of manifest.versions) {
    const option = document.createElement("option");
    option.value = v.id;
    // The selector chooses a *release/runtime*; the language semantics it
    // implements are shown alongside so the two identities are never confused.
    option.textContent = v.available
      ? `Aura ${v.release_version || v.id}  (language ${v.language_version})`
      : `Aura ${v.release_version || v.id}  (unavailable)`;
    option.disabled = !v.available;
    els.version.append(option);
  }
  els.version.value = manifest.current;
  onVersionChange();
}

function onVersionChange() {
  const entry = selectedVersion();
  if (entry && !entry.available) {
    showNote(entry.reason || "This version has no browser runtime.");
    els.run.disabled = true;
  } else {
    showNote("");
    els.run.disabled = false;
  }
}

function stopCurrent(reason) {
  if (currentRun && currentRun.worker) {
    currentRun.worker.terminate();
    currentRun.worker = null;
  }
  if (currentRun) {
    currentRun.stopped = true;
  }
  setStatus(reason || "stopped", "error");
  els.run.disabled = false;
  els.stop.disabled = true;
}

function run() {
  const entry = selectedVersion();
  if (!entry || !entry.available) return;
  // Cancel any previous execution before starting a new one.
  if (currentRun && currentRun.worker) {
    currentRun.worker.terminate();
  }
  generation += 1;
  const runId = generation;
  clearOutput();
  showNote("");
  setStatus("running…");
  els.run.disabled = true;
  els.stop.disabled = false;

  const worker = new Worker("./web/worker.js");
  const record = { runId, worker, stopped: false };
  currentRun = record;

  const args = els.args.value
    .split("\n")
    .filter((l) => l.length > 0);
  const stdinRaw = els.stdin.value;
  const stdin = stdinRaw.length > 0 ? stdinRaw : null;

  worker.onmessage = (event) => {
    // Ignore any message that is not from the current generation: a Worker
    // terminated mid-flight cannot resurrect a stale run.
    if (!currentRun || currentRun.runId !== runId) return;
    const msg = event.data || {};
    if (msg.kind === "loaded") {
      setStatus(`running (runtime ${msg.runtimeVersion}, ABI ${msg.abiVersion})`);
      return;
    }
    if (msg.kind === "result") {
      finishRun(record, msg.result);
      return;
    }
    if (msg.kind === "error") {
      renderDiagnostics([
        {
          code: 4999,
          code_text: "E4999",
          message: `worker ${msg.phase} error: ${msg.message}`,
          line: 1,
          column: 1,
        },
      ]);
      setStatus("worker error", "error");
      stopCurrent("worker error");
    }
  };

  worker.onerror = (err) => {
    if (!currentRun || currentRun.runId !== runId) return;
    renderDiagnostics([
      {
        code: 4999,
        code_text: "E4999",
        message: `worker error: ${err.message || "unknown"}`,
        line: 1,
        column: 1,
      },
    ]);
    setStatus("worker error", "error");
    stopCurrent("worker error");
  };

  // Resolve the artifact to an absolute URL on the page's origin so the
  // Worker (whose own base URL is ./web/) fetches the immutable artifact, not
  // a path relative to the worker script.
  const artifactUrl = new URL(`./runtimes/${entry.artifact}`, window.location.href).href;

  worker.postMessage({
    runId,
    artifactUrl,
    expectedAbi: entry.host_abi_version,
    // Single authoritative source: Run executes exactly what the editor holds
    // right now. It never reconstructs a program from the default source or
    // from any consumed handoff state.
    source: els.source.value,
    args,
    stdin,
  });
}

function finishRun(record, result) {
  els.stdout.textContent = result.stdout || "";
  renderDiagnostics(result.diagnostics || []);
  if (result.status === "ok") {
    setStatus("ok", "ok");
  } else if (result.status === "diagnostic") {
    setStatus("diagnostic", "error");
  } else {
    setStatus("internal error", "error");
  }
  if (record.worker) {
    record.worker.terminate();
    record.worker = null;
  }
  if (currentRun === record) {
    currentRun = null;
  }
  els.run.disabled = false;
  els.stop.disabled = true;
}

els.run.addEventListener("click", run);
els.stop.addEventListener("click", () => stopCurrent("stopped"));
els.version.addEventListener("change", onVersionChange);
els.source.value = DEFAULT_SOURCE;
// The initial program is the default one; a handoff from a host site's
// "Run in Playground" link — carried by the navigation itself — replaces it,
// along with the example's arguments/standard input, before any execution.
applyHandoff();

// The Worker is the primary cancellation mechanism; terminate it if the page
// itself is going away so nothing keeps running invisibly.
window.addEventListener("beforeunload", () => {
  if (currentRun && currentRun.worker) currentRun.worker.terminate();
});

loadManifest().catch((err) => {
  setStatus("cannot load runtimes", "error");
  renderDiagnostics([
    {
      code: 4999,
      code_text: "E4999",
      message: err.message,
      line: 1,
      column: 1,
    },
  ]);
});
