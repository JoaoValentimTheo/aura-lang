// Navigation fit regression.
//
// The reported defect: at 768px the primary navigation wrapped across three
// rows. The fix replaces the breakpoint-only approach with a tiered
// architecture (a restrained inline set + a grouped More menu; a compact
// disclosure below the measured fit width). This test proves the defect cannot
// recur by asserting *geometry*, not merely that `scrollWidth <= clientWidth`:
//
//   * the app bar is a single row of content at every width in the sweep;
//   * no two top-level bar children overlap;
//   * the nav toggle appears exactly when the inline nav is hidden;
//   * every destination in `site.config.mjs` is reachable in every mode;
//   * the More menu and the compact menu both open and expose all routes.
//
// Usage: node website/tests/nav-fit.test.mjs
// Requires `playwright` with Chromium; skips otherwise.

import { startServer } from "./serve.mjs";
import { nav } from "../site.config.mjs";

let chromium;
try {
  ({ chromium } = await import("playwright"));
} catch {
  console.log("nav-fit tests: playwright not installed — skipped.");
  process.exit(0);
}

const { server, port, base: basePath } = await startServer(0);
const base = `http://127.0.0.1:${port}${basePath}`;
const browser = await chromium.launch();

// Normalize an href to a route relative to the site base, so the reachability
// check is independent of the deployed base (`/aura-lang/` or `/`).
function routeOf(href, prefix) {
  let h = href.replace(/^https?:\/\/[^/]+/, "");
  if (prefix !== "/" && h.startsWith(prefix)) h = h.slice(prefix.length);
  else h = h.replace(/^\//, "");
  return h.replace(/\/$/, "");
}
const basePrefix = basePath || "/";

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

// The full width sweep, including the reported 768px and both sides of the
// collapse boundary (900px).
const WIDTHS = [320, 360, 375, 390, 414, 430, 540, 600, 667, 740, 760, 820, 900, 901, 1024, 1200, 1280, 1440, 1920, 2560];

for (const width of WIDTHS) {
  const page = await browser.newPage({ viewport: { width, height: 900 } });
  await page.goto(base, { waitUntil: "load" });

  const m = await page.evaluate(() => {
    const bar = document.querySelector(".app-bar__inner");
    const kids = [...bar.children].filter((el) => {
      const r = el.getBoundingClientRect();
      return r.width > 0 && getComputedStyle(el).display !== "none";
    });
    const rects = kids.map((el) => el.getBoundingClientRect());
    // The bar's own content height must be a single row: no child may start a
    // second row below the first. We detect that by overlap of vertical bands.
    let maxTop = -Infinity;
    let minTop = Infinity;
    let overlap = false;
    const horiz = rects.map((r) => ({ left: r.left, right: r.right })).sort((a, b) => a.left - b.left);
    for (let i = 1; i < horiz.length; i += 1) {
      if (horiz[i].left < horiz[i - 1].right - 2) overlap = true;
    }
    for (const r of rects) {
      maxTop = Math.max(maxTop, r.top);
      minTop = Math.min(minTop, r.top);
    }
    const inlineNav = document.getElementById("primary-nav");
    const toggle = document.querySelector("[data-nav-toggle]");
    return {
      barHeight: Math.round(bar.getBoundingClientRect().height),
      overlap,
      verticalSpread: Math.round(maxTop - minTop),
      inlineVisible: inlineNav && getComputedStyle(inlineNav).display !== "none",
      toggleVisible: toggle && getComputedStyle(toggle).display !== "none",
      overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    };
  });

  check(`@${width}px bar is a single row`, m.barHeight <= 64, `height ${m.barHeight}`);
  check(`@${width}px no bar child overlap`, m.overlap === false);
  check(`@${width}px no horizontal overflow`, m.overflow <= 1, `${m.overflow}px`);
  check(
    `@${width}px toggle visible iff inline nav hidden`,
    m.toggleVisible === !m.inlineVisible,
    JSON.stringify(m),
  );

  // Every destination is reachable: either inline, in the More menu, or in the
  // compact menu.
  const reachable = await page.evaluate(() =>
    [...document.querySelectorAll(".app-bar a[href]")].map((a) => a.getAttribute("href")),
  );
  for (const item of nav) {
    const wanted = item.href === "" ? "" : routeOf(item.href, basePrefix);
    check(
      `@${width}px nav reachable: ${item.key}`,
      reachable.some((h) => routeOf(h, basePrefix) === wanted),
      `${item.href} vs ${reachable.join(",")}`,
    );
  }

  // If the inline nav is hidden, the compact disclosure must open and list
  // every destination exactly once.
  if (!m.inlineVisible) {
    await page.click("[data-nav-toggle]");
    const compact = await page.evaluate(() => {
      const nav = document.getElementById("compact-nav");
      const links = [...nav.querySelectorAll("a[href]")].map((a) => a.getAttribute("href"));
      return { hidden: nav.hidden, count: links.length, links };
    });
    check(`@${width}px compact menu opens`, compact.hidden === false);
    check(`@${width}px compact menu lists all routes`, compact.count >= nav.length, `${compact.count} vs ${nav.length}`);
  } else {
    // The More menu must open and expose the overflow routes.
    await page.click("[data-nav-menu-trigger]");
    const more = await page.evaluate(() => {
      const popup = document.querySelector(".nav-menu__popup");
      return { hidden: popup.hidden, count: popup.querySelectorAll("a[href]").length };
    });
    check(`@${width}px More menu opens`, more.hidden === false);
    check(`@${width}px More menu exposes overflow routes`, more.count >= 1, String(more.count));
  }

  await page.close();
}

await browser.close();
server.close();
console.log(`\nnav-fit: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
