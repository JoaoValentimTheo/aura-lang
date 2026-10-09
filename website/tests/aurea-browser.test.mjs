// Aurea design-system browser validation.
//
// The static contract lives in `aurea.test.mjs`. This test proves the *built*
// site actually renders under Aurea in a real engine:
//
//   * the active shell loads only the Aurea stylesheets (no Material sheet);
//   * the `--au-*` tokens resolve to real values at runtime (not `""`);
//   * dark and light are genuinely different, token-driven themes;
//   * no element's computed style references a `--md-*` value;
//   * code blocks scroll locally instead of widening the page;
//   * the primary navigation is keyboard-operable and marked current.
//
// Usage: node website/tests/aurea-browser.test.mjs
// Requires `playwright` with Chromium; skips otherwise.

import { startServer } from "./serve.mjs";

let chromium;
try {
  ({ chromium } = await import("playwright"));
} catch {
  console.log("aurea-browser tests: playwright not installed — skipped.");
  process.exit(0);
}

const { server, port, base: basePath } = await startServer(0);
const base = `http://127.0.0.1:${port}${basePath}`;

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

const browser = await chromium.launch();

/* ---------------------------------------------- stylesheet chain */
{
  const page = await browser.newPage();
  await page.goto(base);
  const hrefs = await page.$$eval('link[rel="stylesheet"]', (ls) => ls.map((l) => l.getAttribute("href")));
  check("shell loads exactly two stylesheets", hrefs.length === 2, hrefs.join(", "));
  check("shell loads aurea.css", hrefs.some((h) => h.includes("aurea.css")));
  check("shell loads styles.css", hrefs.some((h) => h.includes("styles.css")));
  check("shell does not load a Material tokens sheet", !hrefs.some((h) => h.includes("tokens.css")));

  const sheets = await page.evaluate(() =>
    [...document.styleSheets].map((s) => {
      try {
        return [...s.cssRules].map((r) => r.cssText).join("\n");
      } catch {
        return "";
      }
    }).join("\n"),
  );
  check("no --md-* custom property is declared in loaded CSS", !/--md-/.test(sheets));
  await page.close();
}

/* ---------------------------------------------- tokens resolve */
{
  const page = await browser.newPage();
  await page.goto(base);
  const tokens = await page.evaluate(() => {
    const cs = getComputedStyle(document.documentElement);
    const names = [
      "--au-ink-0", "--au-text-strong", "--au-accent", "--au-line",
      "--au-space-4", "--au-radius-md", "--au-syntax-keyword", "--au-focus",
    ];
    return Object.fromEntries(names.map((n) => [n, cs.getPropertyValue(n).trim()]));
  });
  for (const [name, value] of Object.entries(tokens)) {
    check(`token resolves: ${name}`, value.length > 0, `"${value}"`);
  }
  await page.close();
}

/* ---------------------------------------------- dark vs light */
{
  const read = async (theme) => {
    const page = await browser.newPage();
    await page.addInitScript((t) => {
      try { localStorage.setItem("aura-theme", t); } catch {}
    }, theme);
    await page.goto(base);
    const v = await page.evaluate(() => {
      const cs = getComputedStyle(document.body);
      return {
        theme: document.documentElement.getAttribute("data-theme"),
        bg: cs.backgroundColor,
        fg: cs.color,
        accent: getComputedStyle(document.documentElement).getPropertyValue("--au-accent").trim(),
        colorScheme: getComputedStyle(document.documentElement).colorScheme,
      };
    });
    await page.close();
    return v;
  };
  const dark = await read("dark");
  const light = await read("light");
  check("dark theme applies data-theme", dark.theme === "dark", dark.theme);
  check("light theme applies data-theme", light.theme === "light", light.theme);
  check("dark and light backgrounds differ", dark.bg !== light.bg, `${dark.bg} vs ${light.bg}`);
  check("dark and light foregrounds differ", dark.fg !== light.fg, `${dark.fg} vs ${light.fg}`);
  check("dark and light accents differ", dark.accent !== light.accent, `${dark.accent} vs ${light.accent}`);
  check("dark color-scheme is dark", dark.colorScheme.includes("dark"), dark.colorScheme);
  check("light color-scheme is light", light.colorScheme.includes("light"), light.colorScheme);

  // The light theme is a structural inversion: a white-ish page in light,
  // a black-first page in dark.
  check("dark background is near-black", /rgb\(\s*(\d+)/.test(dark.bg) && parseInt(dark.bg.match(/\d+/)[0], 10) < 40, dark.bg);
  check("light background is near-white", /rgb\(\s*(\d+)/.test(light.bg) && parseInt(light.bg.match(/\d+/)[0], 10) > 220, light.bg);
}

/* ---------------------------------------------- code block containment */
{
  const page = await browser.newPage({ viewport: { width: 375, height: 800 } });
  await page.goto(`${base}docs/guide-collections/`);
  const code = await page.evaluate(() => {
    const block = document.querySelector(".code-block");
    if (!block) return null;
    const pre = block.querySelector("pre");
    const cs = getComputedStyle(pre);
    const overflow = document.documentElement.scrollWidth - document.documentElement.clientWidth;
    return { overflowX: cs.overflowX, pageOverflow: overflow };
  });
  check("code block exists on a docs page", code !== null);
  if (code) {
    check("code block scrolls locally (overflow-x auto)", code.overflowX === "auto", code.overflowX);
    check("code block does not widen the page at 375px", code.pageOverflow <= 1, `${code.pageOverflow}px`);
  }
  await page.close();
}

/* ---------------------------------------------- navigation a11y */
{
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(`${base}docs/guide-collections/`);
  const nav = await page.evaluate(() => {
    const nav = document.getElementById("primary-nav");
    const current = document.querySelector('[aria-current="page"]');
    const toggle = document.querySelector("[data-nav-toggle]");
    return {
      hasNav: Boolean(nav),
      labelled: nav?.getAttribute("aria-label"),
      currentText: current?.textContent?.trim(),
      toggleExpanded: toggle?.getAttribute("aria-expanded"),
      skip: Boolean(document.querySelector(".skip-link")),
    };
  });
  check("primary nav exists and is labelled", nav.hasNav && !!nav.labelled, JSON.stringify(nav));
  check("active page is marked aria-current", Boolean(nav.currentText), JSON.stringify(nav));
  check("nav toggle exposes aria-expanded", nav.toggleExpanded === "false", nav.toggleExpanded);
  check("skip link exists", nav.skip);
  await page.close();
}

/* ---------------------------------------------- keyboard focus visible */
{
  const page = await browser.newPage();
  await page.goto(base);
  // Tab to the first focusable element (the skip link) and confirm a focus ring.
  await page.keyboard.press("Tab");
  const focus = await page.evaluate(() => {
    const el = document.activeElement;
    const cs = getComputedStyle(el);
    return { tag: el?.tagName, outlineStyle: cs.outlineStyle, outlineWidth: cs.outlineWidth };
  });
  check("first tab stop is focusable", focus.tag === "A", JSON.stringify(focus));
  check(
    "focused element shows a visible outline",
    focus.outlineStyle !== "none" && parseFloat(focus.outlineWidth) > 0,
    JSON.stringify(focus),
  );
  await page.close();
}

await browser.close();
server.close();

console.log(`\naurea-browser: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
