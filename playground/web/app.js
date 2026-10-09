// The Aura Playground orchestration layer.
//
// This file contains no Aura semantics. It:
//   * applies a host site's selected-example handoff once, then leaves the
//     project's files as the single authoritative source of execution;
//   * holds the multi-file *project state* (files, active file, entry file) and
//     translates it into the runtime's virtual-project transport. It resolves
//     nothing: Aura module names, visibility, and canonical names come only
//     from the source text and are the resolver's business;
//   * resolves the versioned runtime manifest and lets the user pick a version;
//   * spawns a fresh Worker per run so execution is isolated from the UI thread;
//   * terminates the Worker on Stop or completion (hard cancellation);
//   * ignores stale messages by execution generation id;
//   * renders the structured ExecutionResult as a developer-facing Output /
//     Problems view, attributing each diagnostic to the source that produced
//     it when the runtime reports one.
//
// The version selector is real: the chosen entry's immutable artifact URL is
// what the Worker fetches and executes.

import { highlight } from "./highlight.js";
import { candidates } from "./completion.js";
import { Project, DEFAULT_SOURCE_NAME } from "./project.js";
import { workbenchMarkup } from "./workbench.mjs";

// The standalone Playground ships a bare `<main id="workbench">` and injects
// the shared workbench markup here, so it renders the *identical* interface the
// integrated website Playground embeds statically. The website shell already
// contains the markup (generated at build time), so this is a no-op there.
function mountWorkbench() {
  const mount = document.querySelector("[data-playground]");
  if (mount && !document.getElementById("source")) {
    mount.innerHTML = workbenchMarkup();
  }
}
mountWorkbench();

// The standalone shell exposes a theme control; wire it before the controller
// reads any element. On the website the app bar owns theming, so the button is
// absent and this is skipped.
(function wireStandaloneChrome() {
  const button = document.getElementById("pg-theme");
  if (!button) return;
  const root = document.documentElement;
  const sync = () => {
    const theme = root.getAttribute("data-theme") || "dark";
    button.textContent = theme === "dark" ? "Light" : "Dark";
  };
  button.addEventListener("click", () => {
    const theme = root.getAttribute("data-theme") === "dark" ? "light" : "dark";
    root.setAttribute("data-theme", theme);
    try {
      localStorage.setItem("aura-theme", theme);
    } catch {
      /* storage is optional */
    }
    sync();
  });
  sync();
})();

const els = {
  version: document.getElementById("version"),
  run: document.getElementById("run"),
  stop: document.getElementById("stop"),
  source: document.getElementById("source"),
  args: document.getElementById("args"),
  stdin: document.getElementById("stdin"),
  stdout: document.getElementById("stdout"),
  outputNote: document.getElementById("output-note"),
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
  fileTabs: document.getElementById("file-tabs"),
  fileAdd: document.getElementById("file-add"),
  fileRename: document.getElementById("file-rename"),
  fileEntry: document.getElementById("file-entry"),
  fileDelete: document.getElementById("file-delete"),
  projectReset: document.getElementById("project-reset"),
  projectNote: document.getElementById("project-note"),
  completion: document.getElementById("completion"),
  searchBar: document.getElementById("search"),
  searchToggle: document.getElementById("search-toggle"),
  searchInput: document.getElementById("search-input"),
  searchCount: document.getElementById("search-count"),
  searchPrev: document.getElementById("search-prev"),
  searchNext: document.getElementById("search-next"),
  searchCase: document.getElementById("search-case"),
  searchClose: document.getElementById("search-close"),
};

// Small, valid examples that teach one idea each. They are shown in the
// Explorer; selecting one replaces the whole project.
//
// An example is either single-source (`source`) or multi-file (`files`). A
// multi-file example names the file that is the entry point; its Aura module
// structure still comes entirely from the source text, never from the file
// names.
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
    id: "modules",
    title: "modules (3 files)",
    entry: "main.aura",
    files: [
      {
        name: "main.aura",
        text: `fn main() {
    let cart = ["apple", "pear", "plum"]
    print(pricing::describe(cart))
    print(f"total: {pricing::total(cart)}")
}
`,
        children: [{ name: "pricing", to: "pricing.aura" }],
      },
      {
        name: "pricing.aura",
        text: `use catalog

pub fn total(items: [string]) -> int {
    let mut running = 0
    for i in range(0, items.len()) {
        running = running + catalog::price(items[i])
    }
    return running
}

pub fn describe(items: [string]) -> string {
    return f"{items.len()} item(s) at {total(items)} cents"
}
`,
        children: [{ name: "catalog", to: "catalog.aura" }],
      },
      {
        name: "catalog.aura",
        text: `pub fn price(item: string) -> int {
    if item == "apple" { return 120 }
    if item == "pear" { return 90 }
    return 45
}
`,
        children: [],
      },
    ],
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

// The Playground's project state. The model owns files, keys, and the active/
// entry selection; this file owns the DOM binding around it.
const project = Project.default();

// The most recent structured result, so a diagnostic click can resolve the
// source it came from even after the files have been edited.
let lastResult = null;

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
  // A handoff carries a single program, so it replaces the whole project with
  // the canonical one-file project. The historical behavior — the payload
  // becomes the editor's program, once — is unchanged.
  if (handoff.source != null) {
    project.reset();
    project.setText(project.activeKey, handoff.source);
    // The editor is the active file's editing surface, so it must show the
    // replaced project before anything reads or executes it.
    loadActiveIntoEditor();
    renderFileTabs();
  }
  els.args.value = handoff.args.join("\n");
  els.stdin.value = handoff.stdin;
}

// The Playground state machine (Keystone §31). Every execution transitions
// through a vocabulary the user can reason about; a failure of one execution
// returns the editor to a *ready* state rather than killing the environment.
//
//   ready     — nothing running; the editor is editable and Run is enabled
//   running   — an execution owns a worker; Stop is enabled
//   completed — the last execution finished normally
//   failed    — the last execution ended in a program diagnostic or an
//               internal error; the environment is still ready
//   stopped   — the user or a new run cancelled the last execution
//
// The editor source is never cleared by a failure, and a subsequent run
// always starts from whatever is on screen.
const PLAYGROUND_STATE = {
  READY: "ready",
  RUNNING: "running",
  COMPLETED: "completed",
  FAILED: "failed",
  STOPPED: "stopped",
};

// The visible status text for each state, so the vocabulary is presented
// consistently instead of by ad-hoc strings at each call site.
const STATE_LABEL = {
  ready: "Ready",
  running: "Running…",
  completed: "Completed",
  failed: "Failed",
  stopped: "Stopped",
};

let playgroundState = PLAYGROUND_STATE.READY;

function setStatus(text, kind) {
  els.status.textContent = text;
  els.status.className = `status${kind ? ` ${kind}` : ""}`;
}

// Enter a state, updating the status line and the control availability
// together so the two can never disagree.
function enterState(state, detail) {
  playgroundState = state;
  const label = STATE_LABEL[state] || state;
  const kind =
    state === "failed" ? "error" : state === "running" ? "running" : "ok";
  setStatus(detail ? `${label} — ${detail}` : label, kind);
  els.run.disabled = state === "running";
  els.stop.disabled = state !== "running";
}

// ------------------------------------------------------------- project state
//
// The UI side of the project: a compact file-tab strip plus the file actions.
// Every mutation goes through the model (`web/project.js`), which keeps the
// active and entry keys valid and the display names unique. Nothing here
// decides anything about Aura: a filename is UI identity only.

/** Human-readable label for a file tab, marking the entry file. */
function fileTabLabel(source) {
  return source.key === project.entryKey ? `${source.name} ★` : source.name;
}

function renderFileTabs() {
  if (!els.fileTabs) return;
  els.fileTabs.replaceChildren();
  for (const source of project.sources) {
    const tab = document.createElement("button");
    tab.type = "button";
    tab.className = "tab file-tab";
    tab.setAttribute("role", "tab");
    tab.dataset.key = source.key;
    const isActive = source.key === project.activeKey;
    const isEntry = source.key === project.entryKey;
    tab.setAttribute("aria-selected", String(isActive));
    tab.tabIndex = isActive ? 0 : -1;
    tab.title = isEntry
      ? `${source.name} — the entry point`
      : source.name;
    const label = document.createElement("span");
    label.className = "file-tab__name";
    label.textContent = source.name;
    tab.append(label);
    if (isEntry) {
      const mark = document.createElement("span");
      mark.className = "file-tab__entry";
      mark.textContent = "★";
      mark.setAttribute("aria-hidden", "true");
      tab.append(mark);
      tab.setAttribute("aria-label", `${source.name} (entry point)`);
    }
    tab.addEventListener("click", () => selectFile(source.key));
    els.fileTabs.append(tab);
  }
  if (els.fileDelete) {
    els.fileDelete.disabled = project.size <= 1;
  }
  if (els.fileEntry) {
    els.fileEntry.disabled = project.activeKey === project.entryKey;
  }
  if (els.projectReset) {
    els.projectReset.disabled = false;
  }
}

/** A short, non-fatal message about project capability or a refused action. */
function setProjectNote(text) {
  if (!els.projectNote) return;
  if (!text) {
    els.projectNote.hidden = true;
    els.projectNote.textContent = "";
    return;
  }
  els.projectNote.hidden = false;
  els.projectNote.textContent = text;
}

/** Store the editor's current text into the model before leaving a file. */
function commitEditor() {
  project.setText(project.activeKey, els.source.value);
}

/**
 * Declare the *active* file's provider child links.
 *
 * A link is `{ name, to }`: `name` is the Aura logical module name the parent
 * refers to, and `to` is the display name of the file that provides it. The UI
 * does not interpret either one — ownership, collisions, and ordering belong to
 * the runtime's graph builder — and it never derives a module name from a file
 * name.
 *
 * This is the project-state entry point for a module layout. Interactive link
 * editing is deliberately deferred (FSM-P6 Q3: no module-tree UI); a project
 * loaded from an example declares its links directly.
 *
 * @returns {boolean} whether the declaration was accepted
 */
function setActiveChildren(children) {
  commitEditor();
  const active = project.active();
  if (!active) return false;
  const list = Array.isArray(children) ? children.map((c) => ({ ...c })) : [];
  // A child link belongs to the *provider source* that declares it. If the
  // project's storage replaced its source records (a load or a reset), the
  // in-memory record must be updated too, or the declaration would be silently
  // discarded when the request is built.
  active.children = list;
  const stored = project.get(active.key);
  if (stored) stored.children = list.map((c) => ({ ...c }));
  renderFileTabs();
  return true;
}

/** Load a file into the editor, preserving nothing but the text itself. */
function loadActiveIntoEditor() {
  els.source.value = project.active().text;
  closeCompletion();
  refreshEditor();
}

/** Select a file: commit the current edits first, then swap the buffer. */
function selectFile(key) {
  if (key === project.activeKey) return;
  commitEditor();
  const r = project.selectSource(key);
  if (!r.ok) return;
  loadActiveIntoEditor();
  renderFileTabs();
  setProjectNote("");
}

function addFile() {
  commitEditor();
  const r = project.createSource();
  if (!r.ok) {
    setProjectNote(r.error);
    return;
  }
  loadActiveIntoEditor();
  renderFileTabs();
  setProjectNote("");
  els.source.focus();
}

function renameFile() {
  commitEditor();
  const current = project.active();
  const next = window.prompt("Rename file", current.name);
  if (next === null) return;
  const r = project.renameSource(current.key, next);
  if (!r.ok) {
    setProjectNote(r.error);
    return;
  }
  renderFileTabs();
  setProjectNote(
    project.size > 1
      ? "A file name is display identity only: Aura module names still come from the source text."
      : "",
  );
}

function setEntryFile() {
  commitEditor();
  const r = project.setEntry(project.activeKey);
  if (!r.ok) {
    setProjectNote(r.error);
    return;
  }
  renderFileTabs();
  setProjectNote(`"${project.active().name}" is now the entry point.`);
}

function deleteFile() {
  commitEditor();
  const current = project.active();
  const r = project.deleteSource(current.key);
  if (!r.ok) {
    setProjectNote(r.error);
    return;
  }
  loadActiveIntoEditor();
  renderFileTabs();
  setProjectNote("");
}

function resetProject() {
  project.reset();
  els.args.value = "";
  els.stdin.value = "";
  loadActiveIntoEditor();
  renderFileTabs();
  setProjectNote("");
}

/** Replace the whole project with an example, single- or multi-file. */
function loadExample(example) {
  if (Array.isArray(example.files)) {
    project.loadFiles(example.files);
    if (example.entry) {
      const entry = project.sources.find((s) => s.name === example.entry);
      if (entry) project.setEntry(entry.key);
    }
  } else {
    project.reset();
    project.setText(project.activeKey, example.source);
    project.renameSource(project.activeKey, example.title || DEFAULT_SOURCE_NAME);
  }
  els.args.value = "";
  els.stdin.value = "";
  loadActiveIntoEditor();
  renderFileTabs();
  setProjectNote(
    project.size > 1
      ? `${project.size} files loaded. Run executes the whole project through the runtime's virtual-project path.`
      : "",
  );
}

function installProjectControls() {
  if (els.fileAdd) els.fileAdd.addEventListener("click", addFile);
  if (els.fileRename) els.fileRename.addEventListener("click", renameFile);
  if (els.fileEntry) els.fileEntry.addEventListener("click", setEntryFile);
  if (els.fileDelete) els.fileDelete.addEventListener("click", deleteFile);
  if (els.projectReset) els.projectReset.addEventListener("click", resetProject);
}

// ------------------------------------------------------------------ editor
//
// The editor is a transparent <textarea> layered over a highlighted <pre>.
// The textarea is the editing surface for the *active* file: it stays the
// single source of truth for selection, undo/redo, clipboard, and
// accessibility, and the project model holds the text of every file. The
// layers below only paint colour, line numbers, and the current-line band, and
// are aria-hidden.

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

/**
 * Focus a diagnostic: activate the source it came from, then move to its line.
 *
 * A project diagnostic carries the runtime's `source` display name; a
 * single-source diagnostic does not, and belongs to the active file. This
 * activates a file and moves the caret — it never reinterprets a code, a
 * message, or a location.
 */
function focusDiagnostic(diagnostic) {
  if (diagnostic && typeof diagnostic.source === "string") {
    const owner = project.sources.find((s) => s.name === diagnostic.source);
    if (owner && owner.key !== project.activeKey) {
      commitEditor();
      project.selectSource(owner.key);
      loadActiveIntoEditor();
      renderFileTabs();
    }
  }
  focusLine(diagnostic && diagnostic.line);
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
  let changed = false;
  const updated = lines
    .map((l) => {
      if (outdent) {
        if (l.startsWith(unit)) {
          changed = true;
          return l.slice(unit.length);
        }
        const m = l.match(/^ {1,4}/);
        if (m) {
          changed = true;
          return l.slice(m[0].length);
        }
        return l;
      }
      changed = true;
      return unit + l;
    })
    .join("\n");
  if (!changed) return false;
  ta.setRangeText(updated, lineStart, lineEnd, "select");
  refreshEditor();
  return true;
}

/** Whether the current line has any non-whitespace before the caret. */
function lineHasContentBeforeCaret() {
  const { value, selectionStart } = els.source;
  const lineStart = value.lastIndexOf("\n", selectionStart - 1) + 1;
  return value.slice(lineStart, selectionStart).trim().length > 0;
}

/** Whether the caret's whole line is empty or whitespace only. */
function currentLineIsBlank() {
  const { value, selectionStart } = els.source;
  const lineStart = value.lastIndexOf("\n", selectionStart - 1) + 1;
  let lineEnd = value.indexOf("\n", selectionStart);
  if (lineEnd === -1) lineEnd = value.length;
  return value.slice(lineStart, lineEnd).trim().length === 0;
}

/** Whether the selection spans more than one line. */
function selectionSpansLines() {
  const { value, selectionStart, selectionEnd } = els.source;
  return value.slice(selectionStart, selectionEnd).includes("\n");
}

/** A predictable escape route: move focus to the primary toolbar control. */
function focusToolbar() {
  const target = els.run && !els.run.disabled ? els.run : els.version;
  if (target) target.focus();
}

// ------------------------------------------------------------- completion
//
// A lightweight suggestion popup driven by the shared language metadata. It is
// not a language server and never claims a suggestion is valid.

const completion = {
  items: [],
  active: -1,
};

function completionOpen() {
  return completion.items.length > 0 && !els.completion.hidden;
}

function closeCompletion() {
  completion.items = [];
  completion.active = -1;
  if (els.completion) {
    els.completion.hidden = true;
    els.completion.innerHTML = "";
  }
}

function renderCompletion() {
  if (!els.completion) return;
  els.completion.innerHTML = "";
  completion.items.forEach((item, i) => {
    const li = document.createElement("li");
    li.className = `completion__item${i === completion.active ? " is-active" : ""}`;
    li.setAttribute("role", "option");
    li.setAttribute("aria-selected", i === completion.active ? "true" : "false");
    li.dataset.index = String(i);
    li.innerHTML = `<span class="completion__label">${item.label}</span><span class="completion__kind">${item.kind}</span>`;
    li.addEventListener("mousedown", (event) => {
      // mousedown, not click, so the textarea keeps its selection.
      event.preventDefault();
      acceptCompletion(i);
    });
    li.addEventListener("mouseenter", () => {
      completion.active = i;
      renderCompletion();
    });
    els.completion.appendChild(li);
  });
  const active = els.completion.querySelector(".is-active");
  if (active && active.scrollIntoView) active.scrollIntoView({ block: "nearest" });
}

function openCompletion() {
  if (!els.completion || !els.source) return;
  const pos = els.source.selectionStart;
  if (pos !== els.source.selectionEnd) return closeCompletion();
  const text = els.source.value;
  const items = candidates(text, pos);
  if (items.length === 0) return closeCompletion();
  completion.items = items;
  completion.active = 0;
  els.completion.hidden = false;
  renderCompletion();
}

function acceptCompletion(index) {
  const item = completion.items[index];
  if (!item) return;
  const ta = els.source;
  const pos = ta.selectionStart;
  const text = ta.value;
  let start = pos;
  while (start > 0 && /[A-Za-z0-9_]/.test(text[start - 1])) start -= 1;
  if (item.insert) {
    // A snippet: replace the fragment with the snippet body and place the
    // caret at the `$0` marker.
    const marker = item.insert.indexOf("$0");
    const body = item.insert.replace("$0", "");
    ta.setRangeText(body, start, pos, "end");
    const caret = start + (marker === -1 ? body.length : marker);
    ta.setSelectionRange(caret, caret);
  } else {
    ta.setRangeText(item.label, start, pos, "end");
  }
  closeCompletion();
  sourceDirty = true;
  refreshEditor();
}

// ---------------------------------------------------------------- search
//
// Single-buffer find. Matches are highlighted through the existing highlight
// layer by marking the current match with a selection; Ctrl/Cmd+F focuses the
// search box. This is deliberately not project-wide or filesystem search.

const search = { matches: [], index: 0 };

function searchVisible() {
  return els.searchBar && !els.searchBar.hidden;
}

function openSearch() {
  if (!els.searchBar) return;
  els.searchBar.hidden = false;
  els.searchInput.focus();
  els.searchInput.select();
  runSearch();
}

function closeSearch() {
  if (!els.searchBar) return;
  els.searchBar.hidden = true;
  els.source.focus();
}

function runSearch() {
  if (!searchVisible()) return;
  const needle = els.searchInput.value;
  const hay = els.source.value;
  const caseSensitive = els.searchCase.checked;
  search.matches = [];
  if (needle) {
    const a = caseSensitive ? hay : hay.toLowerCase();
    const b = caseSensitive ? needle : needle.toLowerCase();
    let from = 0;
    for (;;) {
      const at = a.indexOf(b, from);
      if (at === -1) break;
      search.matches.push(at);
      from = at + b.length;
    }
  }
  if (search.index >= search.matches.length) search.index = 0;
  updateSearchCount();
  highlightCurrentMatch();
}

function updateSearchCount() {
  if (!els.searchCount) return;
  const total = search.matches.length;
  els.searchCount.textContent = total === 0 ? "no matches" : `${search.index + 1} / ${total}`;
}

function highlightCurrentMatch() {
  if (search.matches.length === 0) return;
  const at = search.matches[search.index];
  const len = els.searchInput.value.length;
  els.source.focus();
  els.source.setSelectionRange(at, at + len);
  // Bring it into view.
  const lineHeight = parseFloat(getComputedStyle(els.source).lineHeight) || 0;
  const line = els.source.value.slice(0, at).split("\n").length;
  els.source.scrollTop = Math.max(0, (line - 3) * lineHeight);
  paintCurrentLine();
  syncEditorScroll();
}

function searchStep(delta) {
  if (search.matches.length === 0) return;
  search.index = (search.index + delta + search.matches.length) % search.matches.length;
  updateSearchCount();
  highlightCurrentMatch();
}

function installSearch() {
  if (!els.searchBar) return;
  els.searchToggle?.addEventListener("click", openSearch);
  els.searchClose?.addEventListener("click", closeSearch);
  els.searchNext?.addEventListener("click", () => searchStep(1));
  els.searchPrev?.addEventListener("click", () => searchStep(-1));
  els.searchInput?.addEventListener("input", () => {
    search.index = 0;
    runSearch();
  });
  els.searchCase?.addEventListener("change", () => {
    search.index = 0;
    runSearch();
  });
  els.searchInput?.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      searchStep(event.shiftKey ? -1 : 1);
    } else if (event.key === "Escape") {
      event.preventDefault();
      closeSearch();
    }
  });
}

function installEditor() {
  els.source.addEventListener("input", () => {
    sourceDirty = true;
    refreshEditor();
    openCompletion();
  });
  els.source.addEventListener("scroll", syncEditorScroll);
  els.source.addEventListener("keyup", () => {
    paintCurrentLine();
    syncEditorScroll();
  });
  els.source.addEventListener("click", () => {
    paintCurrentLine();
    syncEditorScroll();
    closeCompletion();
  });
  els.source.addEventListener("blur", closeCompletion);
  els.source.addEventListener("keydown", (event) => {
    // Ctrl/Cmd + Enter runs.
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
      event.preventDefault();
      run();
      return;
    }
    // Ctrl/Cmd + F opens find.
    if ((event.metaKey || event.ctrlKey) && (event.key === "f" || event.key === "F")) {
      event.preventDefault();
      openSearch();
      return;
    }
    // Completion popup navigation takes precedence while it is open.
    if (completionOpen()) {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        completion.active = (completion.active + 1) % completion.items.length;
        renderCompletion();
        return;
      }
      if (event.key === "ArrowUp") {
        event.preventDefault();
        completion.active =
          (completion.active - 1 + completion.items.length) % completion.items.length;
        renderCompletion();
        return;
      }
      if (event.key === "Enter" || event.key === "Tab") {
        event.preventDefault();
        acceptCompletion(completion.active);
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        closeCompletion();
        return;
      }
    }
    // Escape precedence: completion, then search, then a running program, then
    // the toolbar (LANGUAGE_SPEC §61).
    if (event.key === "Escape") {
      if (completionOpen()) {
        event.preventDefault();
        closeCompletion();
      } else if (searchVisible()) {
        event.preventDefault();
        closeSearch();
      } else if (currentRun) {
        event.preventDefault();
        stopCurrent("stopped");
      } else {
        event.preventDefault();
        focusToolbar();
      }
      return;
    }
    // Tab indents when there is an editing reason to, and otherwise lets focus
    // move out of the editor so it is never a keyboard trap:
    //   * a multi-line selection        -> indent the selection
    //   * Shift+Tab with indentation    -> unindent, and stay in the editor
    //   * Shift+Tab with nothing to cut -> allow normal (backwards) traversal
    //   * Tab on a blank line at col 0  -> allow normal (forwards) traversal
    //   * any other Tab                 -> insert one indentation unit
    if (event.key === "Tab") {
      const spans = selectionSpansLines();
      if (event.shiftKey) {
        if (spans || els.source.selectionStart !== els.source.selectionEnd) {
          event.preventDefault();
          indentSelection(true);
        } else if (!indentSelection(true)) {
          // Nothing to unindent: let focus leave the editor.
        } else {
          event.preventDefault();
        }
        return;
      }
      if (spans) {
        event.preventDefault();
        indentSelection(false);
        return;
      }
      // Plain Tab with a collapsed caret: release focus only on a blank line
      // with nothing before the caret; otherwise indent.
      if (currentLineIsBlank() && !lineHasContentBeforeCaret()) {
        return; // allow default focus traversal
      }
      event.preventDefault();
      const ta = els.source;
      ta.setRangeText("    ", ta.selectionStart, ta.selectionEnd, "end");
      refreshEditor();
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
      for (const other of els.examples.querySelectorAll(".explorer__item")) {
        other.removeAttribute("aria-current");
      }
      btn.setAttribute("aria-current", "true");
      loadExample(ex);
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
    // The runtime attributes a project diagnostic to the source that produced
    // it. Show that name, and let a click activate that file before moving to
    // the line. The code, message, line, and column are never reinterpreted.
    if (typeof d.source === "string" && d.source.length > 0) {
      const owner = document.createElement("span");
      owner.className = "diag-source";
      owner.textContent = d.source;
      head.append(owner);
    }
    head.addEventListener("click", () => focusDiagnostic(d));
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
    //
    // A superseded development runtime is a historical preview: it is grouped
    // under "Beta" so a user can still pick it deliberately without it reading
    // as a current release. The codename, when a line has one, is metadata
    // from the manifest (never invented here).
    const channel = v.channel === "development" ? "development" : "release";
    const superseded =
      channel === "development" && v.id !== manifest.current;
    const codename = typeof v.codename === "string" ? ` "${v.codename}"` : "";
    const label =
      channel === "development"
        ? superseded
          ? `Beta — Aura ${v.id}${codename} — historical preview`
          : `Aura ${v.id}${codename} — development runtime`
        : `Aura ${v.release_version || v.id}${codename} — release`;
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
  // A stop (whether the Stop button or a superseding run) ends the execution
  // but never the environment: the source stays and Run is immediately
  // available again.
  enterState(PLAYGROUND_STATE.STOPPED, reason === "stopped" ? undefined : reason);
}

function run() {
  const entry = selectedVersion();
  if (!entry || !entry.available) return;
  // The editor is the editing surface for the active file, so commit it before
  // building the request. Run always executes exactly what is on screen.
  commitEditor();
  // Cancel any previous execution before starting a new one.
  if (currentRun && currentRun.worker) {
    currentRun.worker.terminate();
  }
  generation += 1;
  const runId = generation;
  clearOutput();
  showNote("");
  setProjectNote("");
  enterState(PLAYGROUND_STATE.RUNNING);

  const worker = new Worker("./web/worker.js");
  const record = { runId, worker, stopped: false };
  currentRun = record;

  const args = els.args.value.split("\n").filter((l) => l.length > 0);
  const stdinRaw = els.stdin.value;
  const stdin = stdinRaw.length > 0 ? stdinRaw : null;

  // A single-file project keeps the historical single-source transport
  // byte-for-byte (the runtime applies its eval fallback to exactly that
  // shape). A genuinely multi-file project goes through the virtual-project
  // transport.
  const single = project.isSingleSource();
  let projectRequest = null;
  if (!single) {
    const limits = project.checkLimits();
    if (!limits.ok) {
      renderDiagnostics([
        { code: 4999, code_text: "E4999", message: limits.error, line: 1, column: 1 },
      ]);
      setStatus("project too large", "error");
      stopCurrent("project too large");
      return;
    }
    projectRequest = project.toRequest();
  }

  worker.onmessage = (event) => {
    // Ignore any message that is not from the current generation: a Worker
    // terminated mid-flight cannot resurrect a stale run.
    if (!currentRun || currentRun.runId !== runId) return;
    const msg = event.data || {};
    if (msg.kind === "loaded") {
      // The worker reports the artifact it actually instantiated. The state
      // stays `running`; the detail just gains the runtime identity.
      if (playgroundState === PLAYGROUND_STATE.RUNNING) {
        setStatus(`Running — runtime ${msg.runtimeVersion}, ABI ${msg.abiVersion}`, "running");
      }
      return;
    }
    if (msg.kind === "result") {
      finishRun(record, msg.result);
      return;
    }
    if (msg.kind === "error") {
      // An integrity failure is a *host/loader* condition, not an Aura
      // diagnostic: the program never ran. It is surfaced under the documented
      // internal-error code `E4999` (the same code the UI uses for any worker
      // failure), with a message that names the integrity check explicitly. The
      // structured `msg.code` (`RUNTIME_INTEGRITY`) remains available to
      // programmatic consumers; the UI does not invent an undocumented code.
      const integrity = msg.code === "RUNTIME_INTEGRITY";
      const capability = msg.code === "NO_VIRTUAL_PROJECTS";
      renderDiagnostics([
        {
          code: 4999,
          code_text: "E4999",
          message: integrity
            ? `runtime integrity check failed: ${msg.message}`
            : capability
              ? `this runtime cannot run a multi-file project: ${msg.message}`
              : `worker ${msg.phase} error: ${msg.message}`,
          line: 1,
          column: 1,
        },
      ]);
      if (capability) {
        // Say what is true: the selected runtime has no virtual-project
        // capability, and a single file still runs normally on it.
        setProjectNote(
          `Runtime ${entry.id} does not support virtual projects, so a multi-file project cannot run on it. Select a runtime that provides them, or reduce the project to one file.`,
        );
      }
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
    // The manifest-declared SHA-256 of the immutable artifact. The Worker
    // verifies the fetched bytes against it before instantiation, so a
    // substituted or corrupted artifact is refused rather than executed.
    expectedSha256: entry.sha256,
    // Single authoritative source: Run executes exactly what the project holds
    // right now. It never reconstructs a program from the default source or
    // from any consumed handoff state.
    source: project.active().text,
    // Present only for a genuinely multi-file project; `null` selects the
    // historical single-source path.
    project: projectRequest,
    args,
    stdin,
  });
}

// Present the output footer: when the preview dropped bytes, say so plainly
// and give the exact counts, so a truncated preview is never mistaken for the
// complete output. The marker is the only place this is surfaced; the numbers
// come straight from the runtime's sink accounting.
function renderOutputNote(result) {
  const note = els.outputNote;
  if (!note) return;
  const omitted = Number(result.stdout_omitted || 0);
  if (omitted > 0) {
    const retained = Number(result.stdout_retained || 0);
    const kind = result.stdout_capped
      ? "Output capped (complete mode)"
      : "Output truncated (preview mode)";
    note.textContent = `${kind}: showing ${retained.toLocaleString()} of ${(retained + omitted).toLocaleString()} bytes (${omitted.toLocaleString()} not captured).`;
    note.hidden = false;
  } else {
    note.textContent = "";
    note.hidden = true;
  }
}

function finishRun(record, result) {
  lastResult = result;
  els.stdout.textContent = result.stdout || "";
  renderOutputNote(result);
  renderDiagnostics(result.diagnostics || []);
  if (result.status === "ok") {
    enterState(PLAYGROUND_STATE.COMPLETED);
  } else if (result.status === "diagnostic") {
    // A program diagnostic is a failed *execution*, not a broken
    // environment: the source is intact and Run is available.
    enterState(PLAYGROUND_STATE.FAILED, "program diagnostic");
  } else {
    enterState(PLAYGROUND_STATE.FAILED, "internal error");
  }
  // The worker finished its work; release it. The environment stays ready and
  // the source is untouched, so the next run starts cleanly.
  if (record.worker) {
    record.worker.terminate();
    record.worker = null;
  }
  if (currentRun === record) {
    currentRun = null;
  }
}

els.run.addEventListener("click", run);
els.stop.addEventListener("click", () => stopCurrent("stopped"));
els.version.addEventListener("change", onVersionChange);
// The canonical one-file project is the initial state; the editor shows its
// only file. A handoff or an example may replace the project before any run.
els.source.value = project.active().text;
mountExamples();
installProjectControls();
renderFileTabs();
installEditor();
installSearch();
// Declare the active file's provider child links (Aura logical module name →
// display file name). This is *project state*, not resolution: the runtime's
// graph builder remains authoritative for ownership, collisions, and order.
// It exists so a test or an embedder can declare a multi-file module layout
// without inventing a UI that pretends a file name is a module name.
window.__setChildren = (children) => setActiveChildren(children);
if (els.outputTab) els.outputTab.addEventListener("click", () => showTab("output"));
if (els.problemsTab) els.problemsTab.addEventListener("click", () => showTab("problems"));
showTab("output");
// Enter the machine's initial state through the one transition function, so
// the state variable, the status text, and the control availability are set
// together. The static markup's placeholder text is never a state.
enterState(PLAYGROUND_STATE.READY);

// The initial program is the default one; a handoff from a host site's
// "Run in Playground" link — carried by the navigation itself — replaces it,
// along with the example's arguments/standard input, before any execution.
applyHandoff();
renderFileTabs();
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
