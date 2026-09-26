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

// Focused assertions for the Playground's primary controls, which axe's
// page-level scan does not exercise directly: the runtime version combobox and
// the source editor must be labelled, keyboard-focusable, and expose selection.
{
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await page.goto(`${base}playground/`, { waitUntil: "load" });
  await page.waitForFunction(
    () => document.querySelectorAll("#version option").length > 0,
    { timeout: 15000 },
  );
  const facts = await page.evaluate(() => {
    const sel = document.getElementById("version");
    const label = document.querySelector('label[for="version"]');
    const source = document.getElementById("source");
    sel.focus();
    const selFocused = document.activeElement === sel;
    source.focus();
    const srcFocused = document.activeElement === source;
    return {
      hasSelect: !!sel,
      labelled: !!label && label.textContent.trim().length > 0,
      hasAriaLabel: sel.getAttribute("aria-label"),
      selected: sel.value,
      selectedExposed: sel.selectedOptions.length === 1,
      selFocused,
      srcLabelled: !!source.getAttribute("aria-label"),
      srcFocused,
    };
  });
  const checks = [
    ["combobox exists", facts.hasSelect],
    ["combobox has an accessible label", facts.labelled && !!facts.hasAriaLabel],
    ["combobox exposes the selected runtime", facts.selectedExposed && facts.selected.length > 0],
    ["combobox is keyboard-focusable", facts.selFocused],
    ["editor is labelled", facts.srcLabelled],
    ["editor is keyboard-focusable", facts.srcFocused],
  ];
  for (const [name, ok] of checks) {
    if (ok) passed += 1;
    else {
      failed += 1;
      findings.push(`playground control a11y: ${name}`);
    }
  }
  await page.close();
}

await browser.close();
server.close();

if (findings.length) {
  console.error("\nAccessibility violations (critical/serious):");
  for (const f of findings) console.error(`  ${f}`);
}
console.log(`\nA11y: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
