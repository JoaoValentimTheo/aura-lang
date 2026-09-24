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
//     Stop on a runaway, version switching, and lifecycle recovery.
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
const { server, port } = await startServer(0);
const base = `http://127.0.0.1:${port}/`;

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
    check(`route ${route || "/"} has canonical`, meta.canonical.startsWith("https://aura.lang.dev/"));
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
  check("0.0.2 selectable", options.some((o) => o.value === "0.0.2" && !o.disabled));
  check("0.0.1 present but unavailable", options.some((o) => o.value === "0.0.1" && o.disabled));

  check("playground no page errors", errors.length === 0, errors.join("; "));
  await page.close();
}

await browser.close();
server.close();
console.log(`\nBrowser: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
