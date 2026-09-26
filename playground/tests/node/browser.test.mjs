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

import { startServer } from "./serve.mjs";

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
      return s !== "running…" && !s.startsWith("running (");
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
  check("basic run status", r.status === "ok", r.status);
  check("basic run no page errors", errors.length === 0, errors.join("; "));
  await page.close();
}

// --- 2. diagnostics are structured ---------------------------------------
{
  const { page } = await newPage();
  await setSource(page, 'fn main() { throw "boom" }');
  const r = await runAndWait(page);
  check("uncaught throw status", r.status === "diagnostic", r.status);
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
    () => document.getElementById("status").textContent.startsWith("running"),
    { timeout: 5000 },
  );
  // Prove responsiveness: the Stop button is clickable and the DOM responds.
  check("stop enabled while runaway", await page.isEnabled("#stop"));
  const t0 = Date.now();
  await page.click("#stop");
  await page.waitForFunction(
    () => document.getElementById("status").textContent === "stopped",
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
  await page.waitForFunction(() => document.getElementById("status").textContent.startsWith("running"), {
    timeout: 5000,
  });
  await page.click("#stop");
  await page.waitForFunction(() => document.getElementById("status").textContent === "stopped");
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
  check("0.0.2 release selectable", options.some((o) => o.value === "0.0.2" && !o.disabled), JSON.stringify(options));
  check("0.0.1 present but unavailable", options.some((o) => o.value === "0.0.1" && o.disabled));
  // The development runtime is present, selectable, and labelled as a
  // development runtime rather than a release.
  const dev = options.find((o) => o.value === "0.0.2-dev.5");
  check("development runtime selectable", dev && !dev.disabled, JSON.stringify(options));
  check("development runtime is labelled development", dev && /development/i.test(dev.text), dev && dev.text);

  // Run against the published 0.0.2 release artifact: unchanged behavior.
  await page.selectOption("#version", "0.0.2");
  await setSource(page, "fn main() { print(\"v2\") }");
  const r = await runAndWait(page);
  check("0.0.2 release executes", r.stdout === "v2\n", JSON.stringify(r));

  // Run the evolved language against the development runtime.
  await page.selectOption("#version", "0.0.2-dev.5");
  const note = await page.textContent("#runtime-note");
  check("development runtime explains itself", /development runtime/i.test(note), note);
  await setSource(
    page,
    'type Number = int | float\nfn f(x: Number) { print(x) }\nfn main() {\n f(42)\n f(3.14)\n}',
  );
  const ru = await runAndWait(page);
  check("development runtime executes a union program", ru.stdout === "42\n3.14\n", JSON.stringify(ru));
  await setSource(page, "fn main() {\n for i in 0..3 { print(i) }\n}");
  const rr = await runAndWait(page);
  check("development runtime executes a range literal", rr.stdout === "0\n1\n2\n", JSON.stringify(rr));
  await setSource(page, "<!--\nthis should disappear\n--!>\nfn main() { print(42) }");
  const rc = await runAndWait(page);
  check("development runtime executes through a multiline comment", rc.stdout === "42\n", JSON.stringify(rc));

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
  await setSource(page, "fn main() {\n let mut i = 0\n while i < 600 { print(i) i = i + 1 }\n}");
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

await browser.close();
server.close();

console.log(`\nBrowser: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
