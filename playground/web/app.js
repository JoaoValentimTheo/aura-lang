// The Aura Playground orchestration layer.
//
// This file contains no Aura semantics. It:
//   * applies a host site's selected-example handoff once, then leaves the
//     editor as the single authoritative source of execution;
//   * resolves the versioned runtime manifest and lets the user pick a version;
//   * spawns a fresh Worker per run so execution is isolated from the UI thread;
//   * terminates the Worker on Stop or completion (hard cancellation);
//   * ignores stale messages by execution generation id;
//   * renders the structured ExecutionResult as a developer-facing Output /
//     Problems view.
//
// The version selector is real: the chosen entry's immutable artifact URL is
// what the Worker fetches and executes.

import { highlight } from "./highlight.js";

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
  // Optional presentation elements (present in both shells; guarded anyway).
  highlight: document.getElementById("highlight"),
  gutter: document.getElementById("gutter"),
  problemsCount: document.getElementById("problems-count"),
  problemsTab: document.getElementById("tab-problems"),
  outputTab: document.getElementById("tab-output"),
  outputPanel: document.getElementById("panel-output"),
  problemsPanel: document.getElementById("panel-problems"),
  examples: document.getElementById("examples"),
  editor: document.getElementById("editor"),
};

const DEFAULT_SOURCE = `fn main() {
    print("hello, Aura")

    let xs = [1, 2, 3, 4, 5]
    let evens = xs.filter((x) -> x % 2 == 0)
    print(evens.map((x) -> x * x))
}
`;

// Small, valid examples that teach one idea each. They are shown in the
// Explorer; selecting one replaces the current buffer (there is a single source
// buffer — the Playground does not pretend to have a filesystem).
const EXAMPLES = [
  {
    id: "hello",
    title: "hello.aura",
    source: `fn main() {
    print("hello, Aura")
}
`,
  },
  {
    id: "functions",
    title: "functions.aura",
    source: `fn fib(n) -> int {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}

fn main() {
    print(f"fib(10) = {fib(10)}")
}
`,
  },
  {
    id: "structs",
    title: "structs.aura",
    source: `struct Point { x: int, y: int }

fn main() {
    let p = Point { x: 3, y: 4 }
    print(f"({p.x}, {p.y})")
}
`,
  },
  {
    id: "methods",
    title: "methods.aura",
    source: `struct User { name: string, id: int }

impl User {
    fn greet(self) {
        print("Hello " + self.name)
    }
    fn label(self) -> string {
        return self.name + "#" + to_string(self.id)
    }
}

fn main() {
    let u = User { name: "Ada", id: 1 }
    u.greet()
    print(u.label())
}
`,
  },
  {
    id: "composition",
    title: "composition.aura",
    source: `struct Engine { power: int }

impl Engine {
    fn describe(self) { return f"{self.power}hp" }
}

struct Car { engine: Engine, name: string }

impl Car {
    fn describe(self) {
        return self.name + " (" + self.engine.describe() + ")"
    }
}

fn main() {
    let car = Car { engine: Engine { power: 120 }, name: "Aura GT" }
    print(car.describe())
}
`,
  },
];

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

// ------------------------------------------------------------------ editor
//
// The editor is a transparent <textarea> layered over a highlighted <pre>.
// The textarea remains the single source of truth (selection, undo/redo,
// clipboard, and accessibility all behave normally); the layers below only
// paint colour, line numbers, and the current-line band, and are aria-hidden.

let sourceDirty = true;

function refreshEditor() {
  const text = els.source.value;
  if (els.highlight) {
    els.highlight.innerHTML = `${highlight(text)}\n`;
  }
  if (els.gutter) {
    const lines = text.split("\n").length;
    let g = "";
    for (let i = 1; i <= lines; i += 1) g += `${i}\n`;
    els.gutter.textContent = g;
  }
  syncEditorScroll();
  paintCurrentLine();
  sourceDirty = false;
}

function syncEditorScroll() {
  if (!els.highlight || !els.editor) return;
  const pre = els.highlight;
  pre.scrollTop = els.source.scrollTop;
  pre.scrollLeft = els.source.scrollLeft;
  if (els.gutter) els.gutter.scrollTop = els.source.scrollTop;
}

/** The 1-based line the caret is on, derived from the textarea's selection. */
function caretLine() {
  const upToCaret = els.source.value.slice(0, els.source.selectionStart);
  return upToCaret.split("\n").length;
}

function paintCurrentLine() {
  if (!els.highlight) return;
  const line = caretLine();
  const height = els.source.scrollHeight;
  const lineHeight = parseFloat(getComputedStyle(els.source).lineHeight) || 0;
  const pad = parseFloat(getComputedStyle(els.source).paddingTop) || 0;
  const top = pad + (line - 1) * lineHeight;
  els.highlight.style.setProperty("--current-line-top", `${top}px`);
  els.highlight.style.setProperty("--current-line-height", `${lineHeight}px`);
  void height;
}

/** Select the whole line a diagnostic points at (1-based line/column). */
function focusLine(line) {
  if (!line || line < 1) return;
  const lines = els.source.value.split("\n");
  let start = 0;
  for (let i = 0; i < line - 1 && i < lines.length; i += 1) start += lines[i].length + 1;
  const end = start + (lines[line - 1] ? lines[line - 1].length : 0);
  els.source.focus();
  els.source.setSelectionRange(start, end);
  // Bring the caret into view.
  const lineHeight = parseFloat(getComputedStyle(els.source).lineHeight) || 0;
  els.source.scrollTop = Math.max(0, (line - 3) * lineHeight);
  paintCurrentLine();
  syncEditorScroll();
}

function indentSelection(outdent) {
  const ta = els.source;
  const { selectionStart: s, selectionEnd: e, value } = ta;
  const lineStart = value.lastIndexOf("\n", s - 1) + 1;
  let lineEnd = value.indexOf("\n", e);
  if (lineEnd === -1) lineEnd = value.length;
  const block = value.slice(lineStart, lineEnd);
  const lines = block.split("\n");
  const unit = "    ";
  const changed = lines
    .map((l) => {
      if (outdent) return l.startsWith(unit) ? l.slice(unit.length) : l.replace(/^ {1,4}/, "");
      return unit + l;
    })
    .join("\n");
  ta.setRangeText(changed, lineStart, lineEnd, "select");
  refreshEditor();
}

function installEditor() {
  els.source.addEventListener("input", () => {
    sourceDirty = true;
    refreshEditor();
  });
  els.source.addEventListener("scroll", syncEditorScroll);
  els.source.addEventListener("keyup", () => {
    paintCurrentLine();
    syncEditorScroll();
  });
  els.source.addEventListener("click", () => {
    paintCurrentLine();
    syncEditorScroll();
  });
  els.source.addEventListener("keydown", (event) => {
    // Ctrl/Cmd + Enter runs.
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
      event.preventDefault();
      run();
      return;
    }
    // Escape stops a running program.
    if (event.key === "Escape" && currentRun) {
      event.preventDefault();
      stopCurrent("stopped");
      return;
    }
    // Tab indents, Shift+Tab unindents.
    if (event.key === "Tab") {
      event.preventDefault();
      indentSelection(event.shiftKey);
    }
  });
  // Keep the highlight layer aligned when the window or pane resizes.
  window.addEventListener("resize", syncEditorScroll);
  if (sourceDirty) refreshEditor();
}

function mountExamples() {
  if (!els.examples) return;
  els.examples.replaceChildren();
  for (const ex of EXAMPLES) {
    const li = document.createElement("li");
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "explorer__item";
    btn.textContent = ex.title;
    btn.dataset.example = ex.id;
    btn.addEventListener("click", () => {
      els.source.value = ex.source;
      for (const other of els.examples.querySelectorAll(".explorer__item")) {
        other.removeAttribute("aria-current");
      }
      btn.setAttribute("aria-current", "true");
      refreshEditor();
      els.source.focus();
    });
    li.append(btn);
    els.examples.append(li);
  }
}

function showTab(which) {
  const output = which === "output";
  if (els.outputPanel) els.outputPanel.hidden = !output;
  if (els.problemsPanel) els.problemsPanel.hidden = output;
  if (els.outputTab) els.outputTab.setAttribute("aria-selected", String(output));
  if (els.problemsTab) els.problemsTab.setAttribute("aria-selected", String(!output));
}

function setProblemsCount(n) {
  if (els.problemsCount) els.problemsCount.textContent = n > 0 ? ` (${n})` : "";
}

// ------------------------------------------------------------------ run/stop

function clearOutput() {
  els.stdout.textContent = "";
  els.diagnostics.replaceChildren();
  setProblemsCount(0);
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

// Aura's phase model groups diagnostics by authority; the Playground only
// labels where a diagnostic came from, never changing its code or message.
function phaseLabel(codeText) {
  const n = parseInt(String(codeText).replace(/^E/, ""), 10);
  if (Number.isNaN(n)) return "";
  if (n >= 1000 && n < 2000) return "lex/parse";
  if (n >= 2000 && n < 3000) return "check";
  if (n >= 3000 && n < 4000) return "type";
  if (n >= 4000 && n < 5000) return "runtime";
  return "host";
}

function renderDiagnostics(diagnostics) {
  els.diagnostics.replaceChildren();
  const list = diagnostics || [];
  setProblemsCount(list.length);
  if (list.length === 0) {
    const li = document.createElement("li");
    li.className = "empty";
    li.textContent = "No problems.";
    els.diagnostics.append(li);
    showTab("output");
    return;
  }
  showTab("problems");
  for (const d of list) {
    const codeText = d.code_text || `E${String(d.code).padStart(4, "0")}`;
    const li = document.createElement("li");
    li.className = "problem";
    const head = document.createElement("button");
    head.type = "button";
    head.className = "problem__head";
    const code = document.createElement("span");
    code.className = "diag-code";
    code.textContent = codeText;
    const phase = document.createElement("span");
    phase.className = "diag-phase";
    phase.textContent = phaseLabel(codeText);
    const loc = document.createElement("span");
    loc.className = "diag-loc";
    loc.textContent = `${d.line}:${d.column}`;
    head.append(code, loc, phase);
    head.addEventListener("click", () => focusLine(d.line));
    const msg = document.createElement("div");
    msg.className = "problem__msg";
    msg.textContent = d.message;
    li.append(head, msg);
    els.diagnostics.append(li);
  }
}

function selectedVersion() {
  if (!manifest) return null;
  const id = els.version.value;
  return manifest.versions.find((v) => v.id === id) || null;
}

async function loadManifest() {
  // The runtime *manifest* is mutable metadata: it changes whenever a runtime
  // version is added. The immutable artifacts it names are content-addressed by
  // version and can be cached forever, but the manifest itself must be
  // revalidated on every load, or a browser holding a pre-deploy copy would
  // render a stale version list (for example, missing a newly published
  // development runtime). `no-cache` allows a cached response only after the
  // server confirms it is still current (a cheap 304), so no manual
  // cache-busting token is needed and the behavior is deterministic.
  const response = await fetch("./runtimes/manifest.json", { cache: "no-cache" });
  if (!response.ok) throw new Error(`cannot load manifest (${response.status})`);
  manifest = await response.json();
  els.version.replaceChildren();
  for (const v of manifest.versions) {
    const option = document.createElement("option");
    option.value = v.id;
    // The selector chooses a *runtime artifact*. A development runtime is
    // labelled as such and never presented as a published release; the release
    // identity and the language semantics it implements are shown alongside so
    // the three identities are never confused.
    const channel = v.channel === "development" ? "development" : "release";
    const label =
      channel === "development"
        ? `Aura ${v.id} — development runtime`
        : `Aura ${v.release_version || v.id} — release`;
    option.textContent = v.available ? label : `${label} (unavailable)`;
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
  } else if (entry && entry.channel === "development") {
    showNote(
      "Development runtime: not a published release. It exercises the current language and is replaced as development advances.",
    );
    els.run.disabled = false;
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

  const args = els.args.value.split("\n").filter((l) => l.length > 0);
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
mountExamples();
installEditor();
if (els.outputTab) els.outputTab.addEventListener("click", () => showTab("output"));
if (els.problemsTab) els.problemsTab.addEventListener("click", () => showTab("problems"));
showTab("output");

// The initial program is the default one; a handoff from a host site's
// "Run in Playground" link — carried by the navigation itself — replaces it,
// along with the example's arguments/standard input, before any execution.
applyHandoff();
refreshEditor();

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
