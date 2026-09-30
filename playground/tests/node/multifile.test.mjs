// End-to-end multi-file Playground tests.
//
// These drive the real UI, the real Worker, and the real WebAssembly runtime
// through the *virtual project* path, so the whole chain is the production one:
//
//   file tabs / project state
//     → worker `project` message
//     → runtime.runProject
//     → aura_project_* (Host ABI 1, additive)
//     → InMemorySourceProvider
//     → ModuleGraphBuilder
//     → canonical resolver → checker → runtime
//     → structured result/diagnostics (with `source`)
//     → worker → UI
//
// Nothing here reimplements Aura semantics: the assertions read the runtime's
// own structured result. The suite requires the runtime artifact that carries
// the virtual-project exports (`0.2.0-dev.1`); it selects that version
// explicitly so the test never depends on which entry is the manifest default.
//
// Usage: node playground/tests/node/multifile.test.mjs
// Requires the `playwright` package with Chromium installed.

import { startServer } from "./serve.mjs";

let chromium;
try {
  ({ chromium } = await import("playwright"));
} catch {
  console.error("multifile.test.mjs: `playwright` is not installed; skipping.");
  process.exit(0);
}

/** The development runtime that carries the additive virtual-project exports. */
const PROJECT_RUNTIME = "0.2.0-dev.1";

/** A runtime that predates `aura_project_*`. */
const LEGACY_RUNTIME = "0.0.2";

const { server, port } = await startServer(0);
const base = `http://127.0.0.1:${port}/`;
const browser = await chromium.launch();

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

async function newPage() {
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(base);
  await page.waitForFunction(() => document.querySelectorAll("#version option").length > 0);
  await page.selectOption("#version", PROJECT_RUNTIME);
  return { page, errors };
}

/**
 * Execute the project and wait for the run to finish.
 *
 * The Worker is created by the page, so the only reliable way to observe the
 * exact payload that reaches the runtime is to capture `postMessage` from the
 * page itself and wait for it before waiting on the status.
 */
async function runAndWait(page, timeout = 20000) {
  const sent = await page.evaluate(() => {
    return new Promise((resolve) => {
      const original = Worker.prototype.postMessage;
      Worker.prototype.postMessage = function (message) {
        Worker.prototype.postMessage = original;
        resolve({
          source: message.source,
          entry: message.project ? message.project.entry : null,
          sources: message.project
            ? message.project.sources.map((s) => ({
                key: s.key,
                name: s.name,
                children: s.children,
              }))
            : null,
        });
        return original.call(this, message);
      };
      document.getElementById("run").click();
    });
  });
  await page.waitForFunction(
    () => {
      const s = document.getElementById("status").textContent;
      return s !== "running…" && !s.startsWith("running (");
    },
    { timeout },
  );
  const result = await page.evaluate(() => ({
    stdout: document.getElementById("stdout").textContent,
    status: document.getElementById("status").textContent,
    diagnostics: [...document.querySelectorAll("#diagnostics li:not(.empty)")].map((li) => ({
      text: li.textContent,
      source: li.querySelector(".diag-source")?.textContent ?? null,
    })),
  }));
  return { ...result, sent };
}

/**
 * Assert that the payload the page actually posted carried the expected
 * provider child links.
 *
 * This is the only way to prove the *transport* is correct: the Worker is
 * created by the page, so nothing outside it can observe the request. It also
 * makes a future regression that silently drops ownership links fail loudly
 * instead of surfacing as a confusing `E2003`.
 */
function checkSentLinks(sent, expected) {
  const actual = sent.sources
    ? sent.sources.map((s) => ({ key: s.key, children: s.children.map((c) => c.name) }))
    : null;
  check(
    `posted request carries ${JSON.stringify(expected)}`,
    JSON.stringify(actual) === JSON.stringify(expected),
    JSON.stringify(sent),
  );
}

/** Read the whole project state as the UI holds it. */
async function projectState(page) {
  return page.evaluate(() => ({
    tabs: [...document.querySelectorAll("#file-tabs .file-tab")].map((t) => ({
      name: t.querySelector(".file-tab__name").textContent,
      entry: !!t.querySelector(".file-tab__entry"),
      active: t.getAttribute("aria-selected") === "true",
      key: t.dataset.key,
    })),
    editor: document.getElementById("source").value,
    // The tab marked active must be the file whose text the editor shows.
    editorMatchesActive:
      [...document.querySelectorAll("#file-tabs .file-tab")].find(
        (t) => t.getAttribute("aria-selected") === "true",
      )?.dataset.key ?? null,
    note: document.getElementById("project-note").hidden
      ? null
      : document.getElementById("project-note").textContent,
  }));
}

/**
 * Declare provider child links on the file that declares them, then make
 * `entryTab` the active file.
 *
 * A child link belongs to the *parent* source: the entry file declares which
 * Aura logical module each sibling provides. This helper therefore selects the
 * declaring file first, declares there, and only then returns to the entry.
 *
 * This is project *state*: the runtime's graph builder remains authoritative
 * for ownership, collisions, and ordering. Declaring links is what makes a set
 * of files a project rather than a pile of unrelated sources.
 */
async function declareChildren(page, children, { on, then }) {
  await selectTab(page, on);
  await page.evaluate((c) => window.__setChildren(c), children);
  await selectTab(page, then);
}

async function addFile(page) {
  await page.click("#file-add");
}
async function selectTab(page, name) {
  await page.click(`#file-tabs .file-tab:has-text("${name}")`);
}

// --- 1. create, edit, switch, and edits survive ----------------------------
{
  const { page, errors } = await newPage();

  let state = await projectState(page);
  check("starts as a single-file project", state.tabs.length === 1, JSON.stringify(state.tabs));
  check("the only file is the entry point", state.tabs[0].entry === true);

  await page.fill("#source", 'fn main() { print(helper::value()) }');
  await addFile(page);
  state = await projectState(page);
  check("adding a file creates a second tab", state.tabs.length === 2, JSON.stringify(state.tabs));
  check("the new file becomes active", state.tabs[1].active === true, JSON.stringify(state.tabs));
  check("the entry stays on the first file", state.tabs[0].entry === true);

  // The child's *module name* is `helper`; the file's display name is not
  // module identity. A provider child link declares that ownership.
  await page.fill("#source", 'pub fn value() -> int { return 41 }');
  await declareChildren(page, [{ name: "helper", to: "main-2.aura" }], {
    on: "main.aura",
    then: "main.aura",
  });
  state = await projectState(page);
  check(
    "switching back restores the first file's edits",
    state.editor === 'fn main() { print(helper::value()) }',
    JSON.stringify(state.editor),
  );
  await selectTab(page, "main-2.aura");
  state = await projectState(page);
  check(
    "switching forward restores the second file's edits",
    state.editor === 'pub fn value() -> int { return 41 }',
    JSON.stringify(state.editor),
  );

  // The Aura module identity still comes from the *link*, not the file name.
  await selectTab(page, "main.aura");
  const r = await runAndWait(page);
  check("multi-file project executes", r.status === "ok", JSON.stringify(r));
  check("multi-file project output", r.stdout === "41\n", JSON.stringify(r.stdout));
  check("multi-file run reports no diagnostics", r.diagnostics.length === 0, JSON.stringify(r));
  checkSentLinks(r.sent, [
    { key: "s1", children: ["helper"] },
    { key: "s2", children: [] },
  ]);
  check("no page errors", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 2. set entry, rename, and delete -------------------------------------
{
  const { page } = await newPage();
  await page.fill("#source", 'fn main() { print(other::twice(21)) }');
  await addFile(page);
  await page.fill("#source", 'pub fn twice(n: int) -> int { return n + n }');
  // Rename the second file; its opaque key must not change.
  page.once("dialog", (d) => d.accept("other.aura"));
  await page.click("#file-rename");
  let state = await projectState(page);
  check("rename changes the tab label", state.tabs[1].name === "other.aura", JSON.stringify(state.tabs));
  check("rename keeps the same opaque key", state.tabs[1].key === "s2", JSON.stringify(state.tabs));
  check("rename does not change the entry", state.tabs[0].entry === true);

  // The entry file declares that `other` is provided by the sibling. The link
  // is keyed by file *name*, so declaring it after the rename proves a rename
  // does not disturb ownership; the Aura module name `other` never changes.
  await declareChildren(page, [{ name: "other", to: "other.aura" }], {
    on: "main.aura",
    then: "main.aura",
  });
  const r = await runAndWait(page);
  check("renamed file still participates in the project", r.status === "ok", JSON.stringify(r));
  check("renamed file output", r.stdout === "42\n", JSON.stringify(r.stdout));

  // Deleting the active (second) file leaves a valid project.
  await selectTab(page, "other.aura");
  await page.click("#file-delete");
  state = await projectState(page);
  check("deleting the active file leaves one tab", state.tabs.length === 1, JSON.stringify(state.tabs));
  check("the surviving file is active", state.tabs[0].active === true);
  check("the surviving file is the entry", state.tabs[0].entry === true);
  // The deleted file's link is gone with it, so the project falls back to the
  // historical one-file path and still runs.
  await page.fill("#source", 'fn main() { print("after delete") }');
  const r2 = await runAndWait(page);
  check("project still runs after a delete", r2.status === "ok", JSON.stringify(r2));
  check("the surviving file runs", r2.stdout === "after delete\n", JSON.stringify(r2.stdout));
  check("the deleted file's link is gone", r2.sent.sources === null || r2.sent.sources.length === 1, JSON.stringify(r2.sent));
  await page.close();
}

// --- 3. diagnostics attribute to the owning file and focus it --------------
{
  const { page } = await newPage();
  await page.fill("#source", 'fn main() { print(helper::boom()) }');
  await addFile(page);
  await page.fill("#source", 'pub fn boom() -> int { return "wrong" }');
  await declareChildren(page, [{ name: "helper", to: "main-2.aura" }], {
    on: "main.aura",
    then: "main.aura",
  });
  const r = await runAndWait(page);
  check("a child diagnostic is reported", r.status === "diagnostic", JSON.stringify(r));
  check(
    "the diagnostic carries the owning source name",
    r.diagnostics[0] && r.diagnostics[0].source === "main-2.aura",
    JSON.stringify(r.diagnostics),
  );
  // Clicking the diagnostic activates the file that produced it.
  await page.click("#diagnostics .problem__head");
  const state = await projectState(page);
  check(
    "clicking a diagnostic activates the owning file",
    state.tabs[1].active === true,
    JSON.stringify(state.tabs),
  );
  check(
    "the owning file's text is loaded into the editor",
    state.editor.includes("boom"),
    JSON.stringify(state.editor),
  );
  await page.close();
}

// --- 4. duplicate names are refused, and the project stays valid -----------
{
  const { page } = await newPage();
  await addFile(page);
  let state = await projectState(page);
  check("two files exist", state.tabs.length === 2, JSON.stringify(state.tabs));
  page.once("dialog", (d) => d.accept("main.aura"));
  await page.click("#file-rename");
  state = await projectState(page);
  const names = state.tabs.map((t) => t.name);
  check(
    "a duplicate file name is refused",
    new Set(names).size === names.length && names.includes("main.aura"),
    JSON.stringify(names),
  );
  check("the refusal is explained", /already exists/.test(state.note || ""), state.note);
  const r = await runAndWait(page);
  check("the project still runs after a refused rename", r.status !== "worker error", JSON.stringify(r));
  await page.close();
}

// --- 5. a legacy runtime is told the truth ---------------------------------
{
  const { page, errors } = await newPage();
  await page.selectOption("#version", LEGACY_RUNTIME);
  await page.fill("#source", 'fn main() { print("single") }');
  const r = await runAndWait(page);
  check("a legacy runtime still runs a single source", r.stdout === "single\n", JSON.stringify(r));

  await addFile(page);
  await page.fill("#source", 'pub fn v() -> int { return 1 }');
  const r2 = await runAndWait(page);
  check("a legacy runtime does not crash on a project", r2.status !== "ok", JSON.stringify(r2));
  check(
    "the legacy limitation is explained",
    r2.diagnostics.some((d) => /virtual project/i.test(d.text)),
    JSON.stringify(r2.diagnostics),
  );
  const state = await projectState(page);
  check(
    "the legacy limitation is also stated in the project note",
    /does not support virtual projects/.test(state.note || ""),
    state.note,
  );
  check("no page errors on the legacy path", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 6. the multi-file example loads and runs ------------------------------
{
  const { page } = await newPage();
  await page.click('#examples button:has-text("modules")');
  const state = await projectState(page);
  check("the example loads three files", state.tabs.length === 3, JSON.stringify(state.tabs));
  check("the example marks main.aura as the entry", state.tabs[0].entry === true, JSON.stringify(state.tabs));
  const r = await runAndWait(page);
  check("the multi-file example executes", r.status === "ok", JSON.stringify(r));
  check(
    "the multi-file example prints the expected output",
    r.stdout === "3 item(s) at 255 cents\ntotal: 255\n",
    JSON.stringify(r.stdout),
  );
  await page.close();
}

// --- 7. Stop and restart around a project run ------------------------------
{
  const { page } = await newPage();
  await page.fill("#source", 'fn main() { while true {} }');
  await addFile(page);
  await page.fill("#source", 'pub fn v() -> int { return 1 }');
  await selectTab(page, "main.aura");
  await page.click("#run");
  await page.waitForFunction(() => document.getElementById("status").textContent.startsWith("running"), {
    timeout: 15000,
  });
  await page.click("#stop");
  await page.waitForFunction(() => document.getElementById("status").textContent === "stopped");
  // Run -> edit -> run: the next project run must reflect the edited text.
  await page.fill("#source", 'fn main() { print("after stop") }');
  const r = await runAndWait(page);
  check("a project run restarts cleanly after Stop", r.stdout === "after stop\n", JSON.stringify(r));
  await page.close();
}

// --- 8. one-source backwards compatibility ---------------------------------
{
  const { page } = await newPage();
  await page.fill("#source", "1 + 2");
  const r = await runAndWait(page);
  check("a one-file project keeps the eval fallback", r.status === "ok", JSON.stringify(r));
  await page.close();
}

await browser.close();
server.close();
console.log(`Multi-file: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
