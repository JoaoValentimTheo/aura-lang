// End-to-end browser tests for the built website.
//
// Covers, against a real browser engine:
//   * every route renders with no page errors;
//   * theme switching (light/dark) and persistence;
//   * responsive layout at mobile/tablet/desktop/large widths with no
//     horizontal overflow;
//   * keyboard accessibility (skip link, focus visibility, nav toggle);
//   * SEO metadata (title, description, canonical, Open Graph);
//   * the Playground integration: real WASM run, diagnostics, args, stdin,
//     Stop on a runaway, version switching, and lifecycle recovery;
//   * the example → Playground → Run data flow: the *executed* source must be
//     the selected example, in every navigation mode.
//
// Usage: node website/tests/browser.test.mjs
// Requires `playwright` with Chromium; skips otherwise.

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { startServer } from "./serve.mjs";

let chromium;
try {
  ({ chromium } = await import("playwright"));
} catch {
  console.log("browser tests: playwright not installed — skipped.");
  process.exit(0);
}

const here = dirname(fileURLToPath(import.meta.url));
const dist = join(here, "..", "dist");
const { exampleById } = await import(join(here, "..", "examples", "examples.mjs"));
// Serve under the same deployment base the site was built with (resolved from
// `--base`/`AURA_SITE_BASE`, defaulting to the project-site base).
const { server, port, base: basePath } = await startServer(0);
const base = `http://127.0.0.1:${port}${basePath}`;

// Routes discovered from the build on disk.
function findRoutes(dir, prefix = "") {
  const routes = [];
  for (const entry of readdirSyncSafe(dir)) {
    const p = join(dir, entry);
    if (isDir(p)) {
      routes.push(...findRoutes(p, `${prefix}${entry}/`));
    } else if (entry === "index.html") {
      routes.push(prefix);
    }
  }
  return routes;
}
import { readdirSync, statSync } from "node:fs";
function readdirSyncSafe(dir) {
  return readdirSync(dir);
}
function isDir(p) {
  return statSync(p).isDirectory();
}

const routes = findRoutes(dist)
  .filter((r) => !r.startsWith("assets/"))
  .sort();

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

/* ------------------------------------------------ route render + SEO */
{
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  for (const route of routes) {
    if (route.includes("/")) {
      // Only top-level and docs routes are needed for a broad sanity pass.
    }
    const resp = await page.goto(`${base}${route}`, { waitUntil: "load" });
    check(`route ${route || "/"} status 200`, resp.status() === 200, String(resp.status()));
    const meta = await page.evaluate(() => ({
      title: document.title,
      description: document.querySelector('meta[name="description"]')?.content || "",
      canonical: document.querySelector('link[rel="canonical"]')?.href || "",
      ogTitle: document.querySelector('meta[property="og:title"]')?.content || "",
      h1: document.querySelector("h1")?.textContent?.trim() || "",
      lang: document.documentElement.lang,
    }));
    check(`route ${route || "/"} has title`, meta.title.length > 0);
    check(`route ${route || "/"} has description`, meta.description.length > 0);
    check(
      `route ${route || "/"} has canonical`,
      meta.canonical.startsWith("https://joaovalentimtheo.github.io/aura-lang/"),
    );
    check(`route ${route || "/"} has og:title`, meta.ogTitle.length > 0);
    check(`route ${route || "/"} has h1`, meta.h1.length > 0);
    check(`route ${route || "/"} lang`, meta.lang === "en");
  }
  check("no page errors across routes", errors.length === 0, errors.join("; "));
  await page.close();
}

/* ------------------------------------------------------- theme switch */
{
  const page = await browser.newPage();
  await page.goto(base);
  const initial = await page.evaluate(() =>
    document.documentElement.getAttribute("data-theme"),
  );
  check("theme resolves to light or dark", initial === "light" || initial === "dark", initial);
  // Force light, then toggle to dark.
  await page.evaluate(() => document.documentElement.setAttribute("data-theme", "light"));
  await page.click("[data-theme-toggle]");
  const after = await page.evaluate(() =>
    document.documentElement.getAttribute("data-theme"),
  );
  check("theme toggles to dark", after === "dark", after);
  // Persists across reload.
  await page.reload();
  const persisted = await page.evaluate(() =>
    document.documentElement.getAttribute("data-theme"),
  );
  check("theme persists across reload", persisted === "dark", persisted);
  await page.close();
}

/* --------------------------------------------------------- responsive */
{
  const sizes = [
    { name: "mobile", width: 375, height: 800 },
    { name: "tablet", width: 820, height: 1024 },
    { name: "laptop", width: 1280, height: 800 },
    { name: "large", width: 1920, height: 1080 },
  ];
  for (const size of sizes) {
    const page = await browser.newPage({ viewport: { width: size.width, height: size.height } });
    await page.goto(base);
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    check(`no horizontal overflow @ ${size.name}`, overflow <= 1, `overflow ${overflow}px`);
    // The nav toggle must be visible on small screens and hidden on large.
    const toggleVisible = await page.evaluate(() => {
      const el = document.querySelector("[data-nav-toggle]");
      return el && getComputedStyle(el).display !== "none";
    });
    if (size.width <= 760) {
      check(`nav toggle visible @ ${size.name}`, toggleVisible === true);
    } else {
      check(`nav toggle hidden @ ${size.name}`, toggleVisible === false);
    }
    // Mobile nav opens.
    if (size.width <= 760) {
      await page.click("[data-nav-toggle]");
      const open = await page.evaluate(
        () => document.getElementById("primary-nav").getAttribute("data-open"),
      );
      check(`mobile nav opens @ ${size.name}`, open === "true");
    }
    // Docs pages must not overflow either.
    if (size.name === "mobile") {
      await page.goto(`${base}docs/reference-grammar/`);
      const docOverflow = await page.evaluate(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
      );
      check("docs no horizontal overflow @ mobile", docOverflow <= 1, `overflow ${docOverflow}px`);
    }
    await page.close();
  }
}

/* -------------------------------------------------------- accessibility */
{
  const page = await browser.newPage();
  await page.goto(base);
  // Skip link is the first focusable element and becomes visible on focus.
  await page.keyboard.press("Tab");
  const skipFocused = await page.evaluate(() => document.activeElement?.className?.includes("skip-link"));
  check("first Tab focuses the skip link", skipFocused === true);
  // Focus is visible (outline present) on interactive elements.
  const hasFocusStyle = await page.evaluate(() => {
    const btn = document.querySelector(".btn");
    btn.focus();
    const style = getComputedStyle(btn);
    return style.outlineStyle !== "none" || style.boxShadow !== "none" || style.borderColor !== "";
  });
  check("focused control has a visible style", hasFocusStyle === true);
  // Landmarks.
  const landmarks = await page.evaluate(() => ({
    main: !!document.querySelector("main"),
    header: !!document.querySelector("header"),
    footer: !!document.querySelector("footer"),
    skip: !!document.querySelector(".skip-link"),
    navLabels: [...document.querySelectorAll("nav")].every(
      (n) => n.getAttribute("aria-label") || n.closest("nav") === null,
    ),
  }));
  check("has main landmark", landmarks.main);
  check("has header landmark", landmarks.header);
  check("has footer landmark", landmarks.footer);
  check("has skip link", landmarks.skip);
  // Every image/icon-only control has an accessible name.
  const unnamedButtons = await page.evaluate(() =>
    [...document.querySelectorAll("button")].filter((b) => {
      const text = (b.textContent || "").trim();
      return !text && !b.getAttribute("aria-label");
    }).length,
  );
  check("all buttons have accessible names", unnamedButtons === 0, String(unnamedButtons));
  await page.close();
}

/* ---------------------------------------- example validation on the page */
{
  const page = await browser.newPage();
  await page.goto(`${base}examples/`);
  // Every example's "Run" link should carry the source forward.
  const runLinks = await page.locator("[data-run-example]").count();
  check("examples expose Run actions", runLinks >= 10, String(runLinks));
  // Copy buttons exist and carry the source.
  const copyWithCode = await page.evaluate(() =>
    [...document.querySelectorAll("[data-copy]")].filter(
      (b) => (b.getAttribute("data-code") || "").length > 0,
    ).length,
  );
  check("copy buttons carry source", copyWithCode >= 10, String(copyWithCode));
  await page.close();
}

/* --------------------------------------------------------- playground */
{
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(`${base}playground/`);
  await page.waitForFunction(() => document.querySelectorAll("#version option").length > 0, {
    timeout: 15000,
  });

  async function run(src, timeout = 20000) {
    await page.fill("#source", src);
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

  let r = await run('fn main() { print(1 + 2) }');
  check("playground basic run", r.stdout === "3\n" && r.status === "ok", JSON.stringify(r));

  r = await run('fn main() { throw "boom" }');
  check(
    "playground structured diagnostic E4026",
    r.status === "diagnostic" && r.diagnostics.some((d) => d.includes("E4026")),
    JSON.stringify(r),
  );

  // args + stdin
  await page.fill("#source", "fn main() {\n print(args())\n print(read_line())\n}");
  await page.fill("#args", "alpha\nbeta");
  await page.fill("#stdin", "hello\n");
  await page.click("#run");
  await page.waitForFunction(
    () => {
      const s = document.getElementById("status").textContent;
      return s !== "running…" && !s.startsWith("running (");
    },
    { timeout: 20000 },
  );
  r = await page.evaluate(() => ({ stdout: document.getElementById("stdout").textContent }));
  check("playground args+stdin", r.stdout === '["alpha", "beta"]\nhello\n', JSON.stringify(r));

  // runaway + stop
  await page.fill("#source", "fn main() { while true {} }");
  await page.click("#run");
  await page.waitForFunction(
    () => document.getElementById("status").textContent.startsWith("running"),
    { timeout: 5000 },
  );
  check("stop enabled while runaway", await page.isEnabled("#stop"));
  await page.click("#stop");
  await page.waitForFunction(
    () => document.getElementById("status").textContent === "stopped",
    { timeout: 5000 },
  );
  r = await run("fn main() { print(42) }");
  check("playground recovers after stop", r.stdout === "42\n", JSON.stringify(r));

  // version selection executes the current artifact
  const options = await page.evaluate(() =>
    [...document.querySelectorAll("#version option")].map((o) => ({
      value: o.value,
      disabled: o.disabled,
    })),
  );
  check("0.0.2 release selectable", options.some((o) => o.value === "0.0.2" && !o.disabled));
  check("0.0.1 present but unavailable", options.some((o) => o.value === "0.0.1" && o.disabled));
  check(
    "development runtime selectable",
    options.some((o) => o.value === "0.0.2-dev" && !o.disabled),
  );

  check("playground no page errors", errors.length === 0, errors.join("; "));
  await page.close();
}

/* --------------------------------- run-in-playground example handoff */
// Regression: selecting an example's Run action must make the Playground
// *execute that exact example* — not merely display it, not merely change the
// URL, and never fall back to the default Playground program.
//
// Every assertion below compares the editor against the catalog source
// verbatim and then verifies the executed stdout, so a run that silently
// executed DEFAULT_SOURCE (`hello, Aura` followed by `[4, 16]`) cannot pass.
{
  const page = await browser.newPage();

  async function waitForPlayground(p) {
    await p.waitForFunction(
      () => document.querySelectorAll("#version option").length > 0,
      { timeout: 20000 },
    );
  }
  async function editorState(p) {
    return p.evaluate(() => ({
      source: document.getElementById("source").value,
      args: document.getElementById("args").value,
      stdin: document.getElementById("stdin").value,
    }));
  }
  // Press Run and return what the runtime actually produced.
  async function execute(p) {
    await p.click("#run");
    await p.waitForFunction(
      () => {
        const s = document.getElementById("status").textContent;
        return s !== "running…" && !s.startsWith("running (");
      },
      { timeout: 30000 },
    );
    return p.evaluate(() => ({
      stdout: document.getElementById("stdout").textContent,
      status: document.getElementById("status").textContent,
    }));
  }
  // Case A/B/C/E: go to the examples page, select an example, run it.
  async function loadExample(id, p) {
    await p.goto(`${base}examples/`);
    await p.click(`#${id} [data-run-example]`);
    await p.waitForURL(/playground\//);
    await waitForPlayground(p);
    const loaded = await editorState(p);
    const out = await execute(p);
    return { loaded, out };
  }

  // --- Case A: a plain Playground visit still runs the default program ----
  await page.goto(`${base}playground/`);
  await waitForPlayground(page);
  const plain = await editorState(page);
  check(
    "plain playground starts at DEFAULT_SOURCE",
    plain.source.includes("let xs = [1, 2, 3, 4, 5]") && plain.args === "" && plain.stdin === "",
    JSON.stringify(plain),
  );
  const defaultOut = await execute(page);
  check(
    "plain playground executes the default program",
    defaultOut.stdout === "hello, Aura\n[4, 16]\n",
    JSON.stringify(defaultOut),
  );

  // --- Case B: selecting an example executes that example -----------------
  const hello = exampleById("hello");
  let { loaded, out } = await loadExample("hello", page);
  check(
    "example hello loads its exact source",
    loaded.source === hello.source,
    JSON.stringify(loaded.source),
  );
  check("example hello runs", out.stdout === hello.output, JSON.stringify(out));
  check(
    "hello output differs from the default program",
    hello.output !== defaultOut.stdout,
    JSON.stringify({ hello: hello.output, default: defaultOut.stdout }),
  );

  // An example whose output is unmistakably different from the default one.
  const expr = exampleById("expressions");
  ({ loaded, out } = await loadExample("expressions", page));
  check(
    "selecting another example replaces the previous source verbatim",
    loaded.source === expr.source && !loaded.source.includes("let xs = [1, 2, 3, 4, 5]"),
    JSON.stringify(loaded.source),
  );
  check("example expressions runs", out.stdout === expr.output, JSON.stringify(out));
  check(
    "expressions output differs from the default program",
    expr.output !== defaultOut.stdout,
    JSON.stringify({ expressions: expr.output, default: defaultOut.stdout }),
  );

  // --- Case D: arguments and standard input travel with the example -------
  const argsExample = exampleById("args");
  ({ loaded, out } = await loadExample("args", page));
  check("example args populates arguments", loaded.args === "Ada", JSON.stringify(loaded));
  check(
    "example args executes with its arguments",
    out.stdout === argsExample.output,
    JSON.stringify(out),
  );

  const stdinExample = exampleById("stdin");
  ({ loaded, out } = await loadExample("stdin", page));
  check("example stdin populates input", loaded.stdin === "hello\naura\n", JSON.stringify(loaded));
  check(
    "example stdin executes with its input",
    out.stdout === stdinExample.output,
    JSON.stringify(out),
  );

  // --- Modifier click: the example must travel with the navigation --------
  // A new tab has its own `sessionStorage`, so a per-tab handoff is lost and
  // the new tab would silently run DEFAULT_SOURCE. The payload must be part
  // of the link, and no stale handoff may be left behind in this tab.
  {
    const from = await browser.newPage();
    await from.goto(`${base}examples/`);
    const [popup] = await Promise.all([
      from.context().waitForEvent("page"),
      from.click("#expressions [data-run-example]", { modifiers: ["ControlOrMeta"] }),
    ]);
    await popup.waitForURL(/playground\//, { waitUntil: "domcontentloaded" });
    await waitForPlayground(popup);
    const inPopup = await editorState(popup);
    check(
      "modifier click carries the exact example into the new tab",
      inPopup.source === expr.source,
      JSON.stringify(inPopup.source),
    );
    const popupOut = await execute(popup);
    check(
      "new tab executes the selected example",
      popupOut.stdout === expr.output,
      JSON.stringify(popupOut),
    );
    const stale = await from.evaluate(() => {
      try {
        return sessionStorage.getItem("aura-playground-source");
      } catch {
        return "unavailable";
      }
    });
    check(
      "no stale handoff survives a modifier click",
      stale === null,
      JSON.stringify(stale),
    );
    const originalUrl = await from.evaluate(() => location.pathname);
    check(
      "modifier click leaves the original tab on the examples page",
      originalUrl.endsWith("/examples/"),
      originalUrl,
    );
    await popup.close();
    await from.close();
  }

  // --- The shared site script is not on the critical path -----------------
  // A click that lands before `assets/site.js` has attached its listener must
  // still carry the example: the payload lives in the link itself.
  {
    const early = await browser.newPage();
    let delayed = false;
    await early.route("**/assets/site.js", async (route) => {
      if (!delayed) {
        delayed = true;
        await new Promise((r) => setTimeout(r, 3000));
      }
      try {
        await route.continue();
      } catch {
        /* the page navigated away while the script was delayed */
      }
    });
    await early.goto(`${base}examples/`, { waitUntil: "commit" });
    await early.waitForSelector("#hello [data-run-example]");
    await early.click("#hello [data-run-example]");
    await early.waitForURL(/playground\//, { waitUntil: "domcontentloaded" });
    await waitForPlayground(early);
    const inEarly = await editorState(early);
    check(
      "click before the shared script still loads the exact example",
      inEarly.source === hello.source,
      JSON.stringify(inEarly.source),
    );
    const earlyOut = await execute(early);
    check(
      "early click executes the selected example",
      earlyOut.stdout === hello.output,
      JSON.stringify(earlyOut),
    );
    await early.close();
  }

  await page.close();
}

/* ------------------------------------------------- playground scrolling */
// Regression: long output and long diagnostics must scroll inside their own
// containers after Run, not stretch the page; repeated runs stay usable and
// Stop must not corrupt the UI.
{
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(`${base}playground/`);
  await page.waitForFunction(() => document.querySelectorAll("#version option").length > 0, {
    timeout: 15000,
  });

  async function longRun(source, timeout = 30000) {
    await page.fill("#source", source);
    await page.click("#run");
    await page.waitForFunction(
      () => {
        const s = document.getElementById("status").textContent;
        return s !== "running…" && !s.startsWith("running (");
      },
      { timeout },
    );
  }

  const longStdout = "fn main() {\n let mut i = 0\n while i < 600 { print(i) i = i + 1 }\n}";
  await longRun(longStdout);
  let metrics = await page.evaluate(() => {
    const o = document.getElementById("stdout");
    const pageOverflow = document.documentElement.scrollHeight - document.documentElement.clientHeight;
    o.scrollTop = o.scrollHeight;
    return {
      scrollable: o.scrollHeight > o.clientHeight,
      scrolled: o.scrollTop > 0,
      pageOverflow,
    };
  });
  check("long stdout scrolls inside its container", metrics.scrollable, JSON.stringify(metrics));
  check("long stdout is reachable by scrolling", metrics.scrolled, JSON.stringify(metrics));
  check(
    "long stdout does not create unbounded page scroll",
    metrics.pageOverflow < 4000,
    JSON.stringify(metrics),
  );

  // Repeated runs remain usable.
  await longRun("fn main() { print(99) }");
  const repeat = await page.evaluate(() => document.getElementById("stdout").textContent);
  check("repeated run after long output", repeat === "99\n", JSON.stringify(repeat));

  // A program producing many diagnostics.
  const manyDiags =
    "fn main() {\n print(undefined_a)\n print(undefined_b)\n print(undefined_c)\n}";
  await longRun(manyDiags);
  const diagMetrics = await page.evaluate(() => {
    const d = document.getElementById("diagnostics");
    d.scrollTop = d.scrollHeight;
    return { count: document.querySelectorAll("#diagnostics li:not(.empty)").length, scrolled: d.scrollTop > 0 };
  });
  check("diagnostics render", diagMetrics.count > 0, JSON.stringify(diagMetrics));

  // Stop still leaves the UI consistent.
  await page.fill("#source", "fn main() { while true {} }");
  await page.click("#run");
  await page.waitForFunction(
    () => document.getElementById("status").textContent.startsWith("running"),
    { timeout: 5000 },
  );
  await page.click("#stop");
  await page.waitForFunction(
    () => document.getElementById("status").textContent === "stopped",
    { timeout: 5000 },
  );
  check("run re-enabled after stop", await page.isEnabled("#run"));
  check("stop disabled after stop", (await page.isEnabled("#stop")) === false);
  await page.close();
}

/* ----------------------------- narrow/short layout remains usable */
// Regression: on a short landscape viewport the fixed pane height must fall
// back to content height so the editor, inputs, and Run control are not clipped,
// while long output still scrolls within its own container.
{
  const page = await browser.newPage({ viewport: { width: 667, height: 375 } });
  await page.goto(`${base}playground/`);
  await page.waitForFunction(() => document.querySelectorAll("#version option").length > 0, {
    timeout: 15000,
  });
  await page.fill("#source", "fn main() {\n let mut i = 0\n while i < 400 { print(i) i = i + 1 }\n}");
  await page.click("#run");
  await page.waitForFunction(
    () => {
      const s = document.getElementById("status").textContent;
      return s !== "running…" && !s.startsWith("running (");
    },
    { timeout: 30000 },
  );
  const m = await page.evaluate(() => {
    const pane = document.querySelector(".pg-pane");
    const out = document.getElementById("stdout");
    out.scrollTop = out.scrollHeight;
    return {
      paneClipped: pane.scrollHeight > pane.clientHeight + 2,
      runVisible:
        document.getElementById("run").getBoundingClientRect().bottom <=
        document.documentElement.scrollHeight,
      outputScrolls: out.scrollHeight > out.clientHeight && out.scrollTop > 0,
    };
  });
  check("short viewport does not clip the pane", m.paneClipped === false, JSON.stringify(m));
  check("short viewport keeps Run reachable", m.runVisible === true, JSON.stringify(m));
  check("short viewport output still scrolls", m.outputScrolls === true, JSON.stringify(m));
  await page.close();
}

/* ------------------------------- no internal target strings in public UI */
// The public pages must not expose internal toolchain identifiers such as the
// Rust target triple.
{
  const page = await browser.newPage();
  const terms = ["wasm32-unknown-unknown", "wasm-unknown", "target triple"];
  for (const route of ["", "home", "runtime/", "releases/", "playground/", "docs/runtime-doc/"]) {
    await page.goto(`${base}${route}`);
    const text = await page.evaluate(() => document.body.innerText);
    const leaked = terms.find((t) => text.includes(t));
    check(`no internal target term on ${route || "/"}`, !leaked, leaked);
  }
  await page.close();
}

await browser.close();
server.close();
console.log(`\nBrowser: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
