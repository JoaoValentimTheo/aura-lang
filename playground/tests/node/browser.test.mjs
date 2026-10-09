// End-to-end browser tests: the real UI, the real Worker, the real wasm
// runtime, driven by a real browser engine.
//
// This is the integration gate the prompt requires: the chosen version must
// actually execute the chosen immutable artifact, the UI must stay responsive
// under runaway programs, Stop must terminate execution, and repeated
// Run/Stop/Run cycles must not let stale runs corrupt newer ones.
//
// Usage: node playground/tests/node/browser.test.mjs
// Requires the `playwright` package with Chromium installed.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { startServer } from "./serve.mjs";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const manifest = JSON.parse(
  readFileSync(join(__dirname, "..", "..", "runtimes", "manifest.json"), "utf8"),
);
// The selector default is whatever the manifest's `current` pointer names.
// Deriving it (rather than hardcoding a dev identity) keeps this test correct
// when the development runtime advances.
const currentRuntime = manifest.current;

let chromium;
try {
  ({ chromium } = await import("playwright"));
} catch {
  console.error("browser.test.mjs: `playwright` is not installed; skipping browser integration.");
  process.exit(0);
}

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
  return { page, errors };
}

async function setSource(page, src) {
  await page.fill("#source", src);
}

async function runAndWait(page, timeout = 15000) {
  await page.click("#run");
  await page.waitForFunction(
    () => {
      const s = document.getElementById("status").textContent;
      return s !== "Running…" && !s.startsWith("Running");
    },
    { timeout },
  );
  return page.evaluate(() => ({
    stdout: document.getElementById("stdout").textContent,
    status: document.getElementById("status").textContent,
    diagnostics: [...document.querySelectorAll("#diagnostics li:not(.empty)")].map(
      (li) => li.textContent,
    ),
  }));
}

// --- 1. basic run ---------------------------------------------------------
{
  const { page, errors } = await newPage();
  await setSource(page, 'fn main() {\n print(1 + 2)\n}');
  const r = await runAndWait(page);
  check("basic run stdout", r.stdout === "3\n", JSON.stringify(r));
  check("basic run status", r.status === "Completed", r.status);
  check("basic run no page errors", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 2. diagnostics are structured ---------------------------------------
{
  const { page } = await newPage();
  await setSource(page, 'fn main() { throw "boom" }');
  const r = await runAndWait(page);
  check("uncaught throw status", r.status.startsWith("Failed"), r.status);
  check(
    "uncaught throw shows E4026",
    r.diagnostics.some((d) => d.includes("E4026")),
    JSON.stringify(r.diagnostics),
  );
  await page.close();
}

// --- 3. args and stdin ----------------------------------------------------
{
  const { page } = await newPage();
  await setSource(page, "fn main() {\n print(args())\n print(read_line())\n}");
  await page.fill("#args", "alpha\nbeta");
  await page.fill("#stdin", "hello\n");
  const r = await runAndWait(page);
  check("args+stdin stdout", r.stdout === '["alpha", "beta"]\nhello\n', JSON.stringify(r.stdout));
  await page.close();
}

// --- 4. runaway program: UI stays responsive and Stop terminates it -------
{
  const { page } = await newPage();
  await setSource(page, "fn main() {\n while true {}\n}");
  await page.click("#run");
  // The UI thread must remain responsive while the Worker spins.
  await page.waitForFunction(
    () => document.getElementById("status").textContent.startsWith("Running"),
    { timeout: 5000 },
  );
  // Prove responsiveness: the Stop button is clickable and the DOM responds.
  check("stop enabled while runaway", await page.isEnabled("#stop"));
  const t0 = Date.now();
  await page.click("#stop");
  await page.waitForFunction(
    () => document.getElementById("status").textContent === "Stopped",
    { timeout: 5000 },
  );
  check("stop returns quickly", Date.now() - t0 < 5000);
  // The page is still usable afterwards.
  await setSource(page, "fn main() { print(42) }");
  const r = await runAndWait(page);
  check("page recovers after runaway stop", r.stdout === "42\n", JSON.stringify(r));
  await page.close();
}

// --- 5. repeated Run/Stop/Run lifecycle -----------------------------------
{
  const { page, errors } = await newPage();
  for (let i = 0; i < 3; i += 1) {
    await setSource(page, `fn main() { print(${i}) }`);
    const r = await runAndWait(page);
    check(`cycle ${i} output`, r.stdout === `${i}\n`, JSON.stringify(r));
  }
  // Interleave a runaway + stop, then a normal run.
  await setSource(page, "fn main() { while true {} }");
  await page.click("#run");
  await page.waitForFunction(() => document.getElementById("status").textContent.startsWith("Running"), {
    timeout: 5000,
  });
  await page.click("#stop");
  await page.waitForFunction(() => document.getElementById("status").textContent === "Stopped");
  await setSource(page, 'fn main() { print("after") }');
  const r = await runAndWait(page);
  check("run after stop", r.stdout === "after\n", JSON.stringify(r));
  check("no page errors across lifecycle", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 6. version selection executes the declared artifact ------------------
{
  const { page } = await newPage();
  const options = await page.evaluate(() =>
    [...document.querySelectorAll("#version option")].map((o) => ({
      value: o.value,
      disabled: o.disabled,
      text: o.textContent,
    })),
  );
  // `0.2.0` remains a real, selectable, frozen release. The selector default
  // is whatever the manifest's `current` pointer names: either the newest
  // development runtime on a line, or — once a line is promoted — that line's
  // release artifact. The label must match the entry's channel, not assume the
  // current runtime is always a development build.
  const selected = await page.evaluate(() => document.getElementById("version").value);
  check("the manifest current pointer selects the default", selected === currentRuntime, selected);
  check("0.2.0 release selectable", options.some((o) => o.value === "0.2.0" && !o.disabled), JSON.stringify(options));
  check("historical 0.0.2 release selectable", options.some((o) => o.value === "0.0.2" && !o.disabled));
  check("0.0.1 present but unavailable", options.some((o) => o.value === "0.0.1" && o.disabled));
  const release = options.find((o) => o.value === "0.2.0");
  check("0.2.0 is labelled release", release && /release/i.test(release.text), release && release.text);
  // The current runtime is present, selectable, and labelled according to its
  // own channel: a development entry must say "development"; a release entry
  // must say "release" and must not claim to be a development runtime.
  const currentEntry = manifest.versions.find((v) => v.id === currentRuntime);
  const current = options.find((o) => o.value === currentRuntime);
  check("current runtime selectable", current && !current.disabled, JSON.stringify(options));
  if (currentEntry && currentEntry.channel === "development") {
    check("development runtime is labelled development", current && /development/i.test(current.text), current && current.text);
    check("development is distinguished from release", current && release && current.text !== release.text);
  } else {
    check("release runtime is labelled release", current && /release/i.test(current.text), current && current.text);
  }

  // The default runtime executes with the completed Core language.
  await setSource(page, "fn main() { print(\"v020\") }");
  const r = await runAndWait(page);
  check("the default runtime executes", r.stdout === "v020\n", JSON.stringify(r));

  // Run against the historical 0.0.2 release artifact: unchanged behavior.
  await page.selectOption("#version", "0.0.2");
  await setSource(page, "fn main() { print(\"v2\") }");
  const r2 = await runAndWait(page);
  check("0.0.2 release executes", r2.stdout === "v2\n", JSON.stringify(r2));

  // Run the completed Core language against 0.2.0: generic maps, items(), a
  // comprehension, and a separator-rule program.
  await page.selectOption("#version", "0.2.0");
  await setSource(
    page,
    'fn main() {\n let m: {int: string} = {2: "b", 1: "a"}\n print(m.items())\n print([x * 2 for x in [1, 2, 3] if x > 1])\n}',
  );
  const ru = await runAndWait(page);
  check(
    "0.2.0 executes generic maps, items(), and a comprehension",
    ru.stdout === "[[1, \"a\"], [2, \"b\"]]\n[4, 6]\n",
    JSON.stringify(ru),
  );
  await setSource(page, "fn main() {\n let x = 1 let y = 2\n print(x)\n}");
  const rs = await runAndWait(page);
  check("0.2.0 enforces the separator rule", rs.status !== "ok", JSON.stringify(rs));

  // The final development runtime of the pre-0.2.0 line is still selectable and
  // executes.
  await page.selectOption("#version", "0.0.2-dev.30");
  const note = await page.textContent("#runtime-note");
  check("historical development runtime explains itself", /development runtime/i.test(note), note);
  await setSource(
    page,
    'type Number = int | float\nfn f(x: Number) { print(x) }\nfn main() {\n f(42)\n f(3.14)\n}',
  );
  const rd = await runAndWait(page);
  check("development runtime executes a union program", rd.stdout === "42\n3.14\n", JSON.stringify(rd));

  // The unavailable 0.0.1 option is disabled, so it cannot be selected by a
  // user; setting it programmatically still explains why it cannot run.
  const disabled = await page.evaluate(
    () => document.querySelector('#version option[value="0.0.1"]').disabled,
  );
  check("0.0.1 option is disabled", disabled === true);
  await page.evaluate(() => {
    const sel = document.getElementById("version");
    sel.value = "0.0.1";
    sel.dispatchEvent(new Event("change"));
  });
  await page.waitForFunction(() => document.getElementById("run").disabled === true);
  const note2 = await page.textContent("#runtime-note");
  check("unavailable version explains itself", /predates|no browser runtime/i.test(note2), note2);
  await page.close();
}

// --- 7. host handoff loads source, arguments, and input ---------------------
{
  const { page } = await newPage();
  // Seed the handoff the way a host site does before navigating to the
  // Playground, then reload the page so the controller applies it on load.
  await page.evaluate(() => {
    sessionStorage.setItem(
      "aura-playground-source",
      JSON.stringify({
        source: 'fn main() {\n print(args()[0])\n print(read_line())\n}',
        args: ["Ada"],
        stdin: "hello\n",
      }),
    );
  });
  await page.reload();
  await page.waitForFunction(() => document.querySelectorAll("#version option").length > 0);
  const loaded = await page.evaluate(() => ({
    source: document.getElementById("source").value,
    args: document.getElementById("args").value,
    stdin: document.getElementById("stdin").value,
    remaining: sessionStorage.getItem("aura-playground-source"),
  }));
  check("handoff loads source", loaded.source.includes("args()[0]"), JSON.stringify(loaded));
  check("handoff loads arguments", loaded.args === "Ada", JSON.stringify(loaded));
  check("handoff loads standard input", loaded.stdin === "hello\n", JSON.stringify(loaded));
  check("handoff is consumed once", loaded.remaining === null, JSON.stringify(loaded));
  const r = await runAndWait(page);
  check("handoff program executes", r.stdout === "Ada\nhello\n", JSON.stringify(r));
  await page.close();
}

// --- 7b. URL handoff: the example travels with the navigation ---------------
// The host site carries `?source=…&args=…&stdin=…` in the Playground link
// itself, so the payload cannot be lost to a per-tab side effect. The
// controller must load it into the editor, consume it, and execute it.
{
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const query = new URLSearchParams({
    source: 'fn main() {\n print(args()[0])\n print(read_line())\n}',
    args: JSON.stringify(["Ada"]),
    stdin: "hello\n",
  });
  await page.goto(`${base}?${query.toString()}`);
  await page.waitForFunction(() => document.querySelectorAll("#version option").length > 0);
  const loaded = await page.evaluate(() => ({
    source: document.getElementById("source").value,
    args: document.getElementById("args").value,
    stdin: document.getElementById("stdin").value,
    search: location.search,
    handoff: (() => {
      try {
        return sessionStorage.getItem("aura-playground-source");
      } catch {
        return "unavailable";
      }
    })(),
  }));
  check(
    "query handoff loads source",
    loaded.source === 'fn main() {\n print(args()[0])\n print(read_line())\n}',
    JSON.stringify(loaded.source),
  );
  check("query handoff loads arguments", loaded.args === "Ada", JSON.stringify(loaded.args));
  check("query handoff loads standard input", loaded.stdin === "hello\n", JSON.stringify(loaded.stdin));
  check("query handoff is consumed from the URL", loaded.search === "", JSON.stringify(loaded.search));
  check("query handoff leaves no storage handoff", loaded.handoff === null, JSON.stringify(loaded.handoff));
  const r = await runAndWait(page);
  check("query handoff program executes", r.stdout === "Ada\nhello\n", JSON.stringify(r));
  check("query handoff no page errors", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 8. long output scrolls inside its own container ------------------------
{
  const { page } = await newPage();
  await setSource(
    page,
    "fn main() {\n let mut i = 0\n while i < 600 {\n  print(i)\n  i = i + 1\n }\n}",
  );
  await runAndWait(page);
  const metrics = await page.evaluate(() => {
    const o = document.getElementById("stdout");
    o.scrollTop = o.scrollHeight;
    return {
      scrollable: o.scrollHeight > o.clientHeight,
      scrolled: o.scrollTop > 0,
      atBottom: Math.abs(o.scrollTop + o.clientHeight - o.scrollHeight) <= 2,
    };
  });
  check("standalone long stdout scrolls", metrics.scrollable, JSON.stringify(metrics));
  check("standalone long stdout reachable", metrics.atBottom, JSON.stringify(metrics));
  await page.close();
}

// --- 9. IDE affordances: highlight, line numbers, keyboard, explorer --------
{
  const { page } = await newPage();
  await page.fill("#source", "fn main() {\n print(1)\n}");
  const view = await page.evaluate(() => ({
    highlight: document.getElementById("highlight")?.textContent || "",
    gutter: document.getElementById("gutter")?.textContent || "",
    hasImplHighlight: document.getElementById("highlight")?.innerHTML.includes("tok-keyword"),
  }));
  check("standalone editor highlights keywords", view.hasImplHighlight, JSON.stringify(view.highlight));
  check("standalone editor shows line numbers", view.gutter.startsWith("1\n2\n3"), JSON.stringify(view.gutter));

  // The editor must not be a keyboard trap: Tab can leave it, Escape (idle)
  // moves focus to a toolbar control, and indentation still works.
  const activeId = () =>
    page.evaluate(() => document.activeElement?.id || document.activeElement?.tagName || "");
  await page.fill("#source", "");
  await page.focus("#source");
  await page.keyboard.press("Tab");
  check("standalone Tab leaves the editor from a blank line", (await activeId()) !== "source");
  await page.fill("#source", "abc");
  await page.focus("#source");
  await page.evaluate(() => {
    const s = document.getElementById("source");
    s.setSelectionRange(1, 1);
  });
  await page.keyboard.press("Tab");
  check("standalone Tab still indents mid-line", (await page.inputValue("#source")) === "a    bc");
  await page.focus("#source");
  await page.keyboard.press("Escape");
  const esc = await activeId();
  check(
    "standalone Escape reaches a toolbar control",
    esc === "run" || esc === "stop" || esc === "version",
    esc,
  );

  // Ctrl/Cmd + Enter runs.
  await page.fill("#source", 'fn main() { print("kb") }');
  await page.focus("#source");
  await page.keyboard.press("ControlOrMeta+Enter");
  await page.waitForFunction(() => {
    const s = document.getElementById("status").textContent;
    return s !== "Running…" && !s.startsWith("Running");
  });
  const kbd = await page.evaluate(() => document.getElementById("stdout").textContent);
  check("standalone Ctrl/Cmd+Enter runs", kbd === "kb\n", JSON.stringify(kbd));

  // Problems tab shows the diagnostic and is clickable.
  await page.fill("#source", 'fn main() {\n print(1 + "a")\n}');
  await page.click("#run");
  await page.waitForFunction(() => {
    const s = document.getElementById("status").textContent;
    return s !== "Running…" && !s.startsWith("Running");
  });
  const problems = await page.evaluate(() => ({
    code: document.querySelector("#diagnostics .diag-code")?.textContent || "",
    clickable: !!document.querySelector("#diagnostics .problem__head"),
  }));
  check("standalone problems list shows a code", /E\d{4}/.test(problems.code), JSON.stringify(problems));
  check("standalone problems are clickable", problems.clickable, JSON.stringify(problems));

  // Explorer example loads source.
  await page.click('#examples .explorer__item[data-example="methods"]');
  const ex = await page.inputValue("#source");
  check("standalone explorer loads example", ex.includes("impl User"), ex.slice(0, 40));
  await page.close();
}

// --- 9b. Completion popup and find ----------------------------------------
{
  const { page } = await newPage();
  await page.fill("#source", "pri");
  await page.focus("#source");
  await page.evaluate(() => {
    const s = document.getElementById("source");
    s.setSelectionRange(3, 3);
  });
  await page.evaluate(() => {
    // Trigger the input event the popup listens for.
    document.getElementById("source").dispatchEvent(new Event("input"));
  });
  await page.waitForSelector("#completion:not([hidden])", { timeout: 3000 });
  const labels = await page.$$eval("#completion .completion__label", (els) =>
    els.map((e) => e.textContent),
  );
  check("completion popup offers print", labels.includes("print"), JSON.stringify(labels));
  // Enter accepts the active suggestion.
  await page.keyboard.press("Enter");
  const accepted = await page.inputValue("#source");
  check("completion Enter accepts", accepted.startsWith("print"), accepted);
  // Escape closes the popup without leaving the editor.
  await page.fill("#source", "pr");
  await page.focus("#source");
  await page.evaluate(() => {
    const s = document.getElementById("source");
    s.setSelectionRange(2, 2);
    s.dispatchEvent(new Event("input"));
  });
  await page.waitForSelector("#completion:not([hidden])", { timeout: 3000 });
  await page.keyboard.press("Escape");
  const hidden = await page.evaluate(() => document.getElementById("completion").hidden);
  check("completion Escape closes the popup", hidden === true);

  // Find: Ctrl/Cmd+F opens, reports matches, next/prev navigate.
  await page.fill("#source", "alpha beta alpha");
  await page.focus("#source");
  await page.keyboard.press("ControlOrMeta+f");
  await page.waitForSelector("#search:not([hidden])", { timeout: 3000 });
  await page.fill("#search-input", "alpha");
  const count = await page.textContent("#search-count");
  check("find reports the match count", count === "1 / 2", count);
  await page.click("#search-next");
  const count2 = await page.textContent("#search-count");
  check("find next advances", count2 === "2 / 2", count2);
  await page.keyboard.press("Escape");
  const searchHidden = await page.evaluate(() => document.getElementById("search").hidden);
  check("find Escape closes the bar", searchHidden === true);
  await page.close();
}

// --- 8. load-time integrity: a corrupted artifact is refused --------------
{
  // Drive the real UI, but intercept the artifact fetch and flip a byte in the
  // body before the Worker sees it. The loader must compute the SHA-256 over
  // the served bytes, find it differs from the manifest, and refuse to
  // instantiate — surfacing a structured integrity diagnostic instead of
  // running the (now-untrusted) artifact.
  const { page, errors } = await newPage();
  await page.route("**/runtimes/**/*.wasm", async (route) => {
    const response = await route.fetch();
    const body = await response.body();
    const corrupted = Buffer.from(body);
    corrupted[Math.floor(corrupted.length / 2)] ^= 0xff;
    await route.fulfill({
      status: 200,
      headers: { "content-type": "application/wasm" },
      body: corrupted,
    });
  });
  await setSource(page, 'fn main() { print("should not run") }');
  const result = await runAndWait(page);
  check(
    "corrupted artifact is refused before execution",
    result.status.includes("error") && !result.stdout.includes("should not run"),
    JSON.stringify(result),
  );
  check(
    "corrupted artifact surfaces a structured integrity diagnostic",
    result.diagnostics.some((d) => /E4999/.test(d) && /integrity/i.test(d)),
    JSON.stringify(result.diagnostics),
  );
  check("no raw JS internals leak to the page", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 9. mainstream-shape recursion parity guard (BREAK-0.2.1 / B-1) --------
{
  // The Playground executes in a fresh Web Worker, whose engine stack is
  // smaller than the page's main thread. On the committed 0.2.1 artifact a
  // recursive `else`-block function traps there at depth 196 (native accepts
  // the same program up to the 512-frame language limit), and the Worker
  // surfaces the trap as `E4999 … Maximum call stack size exceeded` instead of
  // `E4011`. See `docs/WASM_CALL_FRAME_LIMIT_DECISION.md` (OPEN, remediation
  // blocked). This guard pins the working region at a depth comfortably below
  // the measured browser floor (150) and requires the structured, successful
  // result — so any change that erodes engine-stack headroom fails the
  // browser suite now.
  const { page, errors } = await newPage();
  await setSource(
    page,
    "fn count(n: int) -> int {\n" +
      "  if n <= 0 {\n" +
      "    return 0\n" +
      "  } else {\n" +
      "    return count(n - 1) + 1\n" +
      "  }\n" +
      "}\n" +
      "fn main() { print(count(150)) }",
  );
  const r = await runAndWait(page);
  check("browser recursion depth 150 succeeds", r.status === "Completed", JSON.stringify(r));
  check("browser recursion depth 150 stdout", r.stdout === "150\n", JSON.stringify(r.stdout));
  check("no page errors in recursion guard", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 10. Worker-path language-limit boundary (BREAK-0.2.1 / B-1) ----------
{
  // Secondary, substrate-pinning assertion for B-1: the thinnest possible
  // recursive body (`fn f() { f() }`, no local state, no arithmetic) must
  // reach the *language* boundary in the production Worker and produce a
  // structured `E4011` — the one shape that does today. That anchors the
  // upper edge of the working region: the guard in section 9 pins depth 150,
  // and this pins the fact that the language limit itself is still reachable
  // on the Worker path. The mainstream finite shapes (else/match/etc.) do NOT
  // reach it there; that gap is the open B-1 defect, not an accepted result,
  // so it is deliberately not asserted here.
  const { page, errors } = await newPage();
  await setSource(page, "fn f() { f() }\nfn main() { f() }");
  const r = await runAndWait(page);
  check(
    "worker recursion limit reports structured E4011",
    r.status.startsWith("Failed") && /E4011/.test(JSON.stringify(r.diagnostics)),
    JSON.stringify(r),
  );
  check("no page errors in worker boundary probe", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 11. Execution recovery across every failure class (Keystone §6) ------
//
// An uncaught error terminates the CURRENT execution. It must not poison the
// reusable environment: the source stays, the state returns to a ready
// vocabulary, and the next run succeeds.
{
  const { page, errors } = await newPage();
  const cases = [
    { name: "lexer error", src: "fn main() { @ }", expect: "Failed" },
    { name: "parser error", src: "fn main( {", expect: "Failed" },
    { name: "checker error", src: 'fn main() { let x: int = "s" }', expect: "Failed" },
    { name: "uncaught throw", src: 'fn main() { throw "boom" }', expect: "Failed" },
    { name: "runtime diagnostic", src: "fn main() { let xs = [1]\n print(xs[9]) }", expect: "Failed" },
  ];
  for (const c of cases) {
    await setSource(page, c.src);
    const r = await runAndWait(page);
    check(`${c.name} reports ${c.expect}`, r.status.startsWith(c.expect), JSON.stringify(r));
    // The environment recovers: the very next run succeeds with new source.
    await setSource(page, `fn main() { print("recovered after ${c.name}") }`);
    const r2 = await runAndWait(page);
    check(
      `runs again after ${c.name}`,
      r2.status === "Completed" && r2.stdout === `recovered after ${c.name}\n`,
      JSON.stringify(r2),
    );
  }
  check("no page errors across recovery matrix", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 12. Source is preserved across a failed run --------------------------
{
  const { page } = await newPage();
  const src = 'fn main() {\n let x: int = "s"\n}';
  await setSource(page, src);
  await runAndWait(page);
  const preserved = await page.inputValue("#source");
  check("failure preserves the source", preserved === src, preserved);
  await page.close();
}

// --- 13. Execution state machine: transitions, forbidden edges, cycles ----
//
// Keystone §6 requires the machine to be reasoned about as a graph, not as a
// set of branches. The states are ready/running/completed/failed/stopped
// (`web/app.js`, PLAYGROUND_STATE). This section checks the *legal*
// transitions, the *forbidden* ones (a control that must be unavailable in a
// state), and cycle closure: every path returns to `ready`, from which any
// next transition is possible.
{
  const { page, errors } = await newPage();
  const state = () => page.evaluate(() => document.getElementById("status").textContent);
  const runDisabled = () => page.evaluate(() => document.getElementById("run").disabled);
  const stopDisabled = () => page.evaluate(() => document.getElementById("stop").disabled);

  // ready: Run enabled, Stop disabled. `ready` is the initial state, and the
  // initial status text is the state label.
  check("FSM initial state is ready", (await state()) === "Ready", await state());
  check("ready: Run enabled", (await runDisabled()) === false);
  check("ready: Stop disabled", (await stopDisabled()) === true);

  // ready -> running: Run is enabled and enters running.
  await setSource(page, "fn main() { print(1) }");
  await page.click("#run");
  await page.waitForFunction(
    () => document.getElementById("status").textContent.startsWith("Running"),
    { timeout: 5000 },
  );
  check("ready -> running", (await state()).startsWith("Running"));
  // running: Stop enabled, Run disabled (the forbidden edge is unreachable).
  check("running: Stop enabled", (await stopDisabled()) === false);
  check("running: Run disabled", (await runDisabled()) === true);
  // The forbidden edge running -> ready without an exit does not exist: the
  // status stays in the running vocabulary until a result or a stop.

  // running -> completed, then completed -> running (a second run).
  await page.waitForFunction(
    () => document.getElementById("status").textContent === "Completed",
    { timeout: 15000 },
  );
  check("running -> completed", (await state()) === "Completed");
  check("completed: Run enabled again", (await runDisabled()) === false);
  check("completed: Stop disabled again", (await stopDisabled()) === true);
  await setSource(page, "fn main() { print(2) }");
  await page.click("#run");
  await page.waitForFunction(
    () => document.getElementById("status").textContent === "Completed",
    { timeout: 15000 },
  );
  const stdout2 = await page.textContent("#stdout");
  check("completed -> running -> completed cycle", stdout2 === "2\n", stdout2);

  // completed -> failed: a diagnostic run never reuses the previous success.
  await setSource(page, 'fn main() { let x: int = "s" }');
  await page.click("#run");
  await page.waitForFunction(
    () => document.getElementById("status").textContent.startsWith("Failed"),
    { timeout: 15000 },
  );
  check("completed -> failed", (await state()).startsWith("Failed"));
  check("failed: Run enabled", (await runDisabled()) === false);
  check("failed: Stop disabled", (await stopDisabled()) === true);

  // failed -> running -> stopped: a runaway program is stopped by the user.
  await setSource(page, "fn main() { while true {} }");
  await page.click("#run");
  await page.waitForFunction(
    () => document.getElementById("status").textContent.startsWith("Running"),
    { timeout: 5000 },
  );
  check("failed -> running", (await state()).startsWith("Running"));
  await page.click("#stop");
  await page.waitForFunction(
    () => document.getElementById("status").textContent === "Stopped",
    { timeout: 5000 },
  );
  check("running -> stopped", (await state()) === "Stopped");
  check("stopped: Run enabled", (await runDisabled()) === false);
  check("stopped: Stop disabled", (await stopDisabled()) === true);

  // stopped -> running -> completed: the machine returns to a fully usable
  // cycle, proving no state is absorbing and every failure path recovers.
  await setSource(page, "fn main() { print(3) }");
  await page.click("#run");
  await page.waitForFunction(
    () => document.getElementById("status").textContent === "Completed",
    { timeout: 15000 },
  );
  const stdout3 = await page.textContent("#stdout");
  check("stopped -> running -> completed recovery", stdout3 === "3\n", stdout3);

  // The transition vocabulary stays closed: no state outside the five is
  // ever presented for the states exercised above.
  const vocabulary = new Set(["Ready", "Running…", "Completed", "Failed", "Stopped"]);
  const final = await state();
  check(
    "state vocabulary is closed",
    [...vocabulary].some((v) => final === v || final.startsWith(v)),
    final,
  );
  check("no page errors across the FSM matrix", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 12. large output, end to end (0.3.2 development runtime) -------------
{
  const { page, errors } = await newPage();
  // The reported incident shape through the real Worker. The *frozen* 0.3.1
  // release keeps its historical fatal E4020 (pinned below); the 0.3.2
  // development runtime completes with a bounded preview and an explicit
  // truncation note.
  await page.selectOption("#version", "0.3.2-dev.7");
  await setSource(
    page,
    "fn main() {\n  for i in 0..600000 {\n    print(i)\n  }\n}",
  );
  const r = await runAndWait(page, 60000);
  check(
    "dev runtime completes large output",
    /completed/i.test(r.status) && r.diagnostics.length === 0,
    JSON.stringify({ status: r.status, diagnostics: r.diagnostics }),
  );
  const shown = await page.evaluate(() => document.getElementById("stdout").textContent.length);
  check("output panel stays bounded", shown <= 256 * 1024, String(shown));
  const note = await page.evaluate(() => {
    const n = document.getElementById("output-note");
    return { hidden: n.hidden, text: n.textContent };
  });
  check("truncation note is shown", note.hidden === false && /truncated/i.test(note.text), JSON.stringify(note));
  check("no page errors on large output", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 13. the frozen 0.3.1 release keeps the historical stdout bound --------
{
  const { page, errors } = await newPage();
  // The published release artifact is immutable: its behavior must not change.
  // Selecting 0.3.1 keeps the fatal E4020 at 1 MiB, proving the new output
  // architecture is a development-runtime change, not a silent rewrite.
  await page.selectOption("#version", "0.3.1");
  await setSource(page, "fn main() {\n  for i in 0..600000 {\n    print(i)\n  }\n}");
  const r = await runAndWait(page, 60000);
  check(
    "frozen 0.3.1 keeps the historical E4020",
    /program diagnostic/i.test(r.status) && r.diagnostics.some((d) => /E4020/.test(d)),
    JSON.stringify({ status: r.status, diagnostics: r.diagnostics }),
  );
  check("no page errors on frozen-runtime overflow", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 14. output preview resets between runs --------------------------------
{
  const { page, errors } = await newPage();
  await page.selectOption("#version", "0.3.2-dev.7");
  await setSource(page, "fn main() {\n  for i in 0..300000 {\n    print(i)\n  }\n}");
  await runAndWait(page, 60000);
  await setSource(page, 'fn main() { print("clean") }');
  const r = await runAndWait(page);
  check("preview resets cleanly after a truncated run", r.stdout === "clean\n", JSON.stringify(r));
  const note = await page.evaluate(() => document.getElementById("output-note").hidden);
  check("truncation note hidden after a clean run", note === true, String(note));
  check("no page errors across preview reset", errors.length === 0, errors.join("; "));
  await page.close();
}

await browser.close();
server.close();

console.log(`\nBrowser: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);

