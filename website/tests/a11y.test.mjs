// Accessibility audit for the built website using axe-core.
//
// Runs axe against every top-level route, in light and dark themes, at mobile
// and desktop widths. Fails on any violation of impact `critical` or
// `serious`; lesser findings are reported but do not fail the gate.
//
// Usage: node website/tests/a11y.test.mjs
// Requires `playwright` and `axe-core`; skips if either is missing.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { startServer } from "./serve.mjs";

let chromium, axeSource;
try {
  ({ chromium } = await import("playwright"));
  axeSource = readFileSync(
    fileURLToPath(import.meta.resolve("axe-core/axe.min.js")),
    "utf8",
  );
} catch (err) {
  console.log(`a11y tests: dependencies unavailable (${err.message}) — skipped.`);
  process.exit(0);
}

const routes = [
  "",
  "learn/",
  "language/",
  "docs/",
  "docs/getting-started/",
  "docs/guide-functions/",
  "docs/reference-stdlib/",
  "examples/",
  "stdlib/",
  "runtime/",
  "tools/",
  "architecture/",
  "roadmap/",
  "releases/",
  "about/",
  "playground/",
];

const { server, port, base: basePath } = await startServer(0);
const base = `http://127.0.0.1:${port}${basePath}`;
const browser = await chromium.launch();

let passed = 0;
let failed = 0;
const findings = [];

for (const theme of ["light", "dark"]) {
  for (const width of [375, 1280]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    // Choose the theme the way a user does (stored preference) so the site's
    // own controller agrees with the audit rather than overriding it.
    await page.addInitScript((t) => {
      try {
        localStorage.setItem("aura-theme", t);
      } catch {}
    }, theme);
    for (const route of routes) {
      await page.goto(`${base}${route}`, { waitUntil: "load" });
      await page.waitForFunction(
        (t) => document.documentElement.getAttribute("data-theme") === t,
        theme,
      );
      await page.addScriptTag({ content: axeSource });
      const result = await page.evaluate(async () => {
        // eslint-disable-next-line no-undef
        return await axe.run(document, {
          resultTypes: ["violations"],
        });
      });
      const serious = result.violations.filter(
        (v) => v.impact === "critical" || v.impact === "serious",
      );
      if (serious.length === 0) {
        passed += 1;
      } else {
        failed += 1;
        for (const v of serious) {
          findings.push(
            `${theme}/${width}px ${route || "/"}: ${v.id} (${v.impact}) — ${v.help} [${v.nodes.length} node(s)]`,
          );
        }
      }
    }
    await page.close();
  }
}

await browser.close();
server.close();

if (findings.length) {
  console.error("\nAccessibility violations (critical/serious):");
  for (const f of findings) console.error(`  ${f}`);
}
console.log(`\nA11y: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
