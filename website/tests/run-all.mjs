// Run the full website validation suite.
//
//   node website/tests/run-all.mjs
//
// Always runs: example validation against the real runtime, link/asset
// validation, and the browser integration + responsive + accessibility suites
// (the latter require Playwright + axe-core and are skipped, never silently
// passed, when absent).
//
// The build must exist first: `node website/build.mjs`.

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../..");
const dist = join(repoRoot, "website", "dist");

function run(name, script, args = []) {
  process.stdout.write(`\n=== ${name} ===\n`);
  execFileSync("node", [join(here, script), ...args], { stdio: "inherit" });
}

if (!existsSync(dist)) {
  console.error("website/tests: dist missing; run `node website/build.mjs` first");
  process.exit(1);
}

run("examples", "validate-examples.mjs");
run("links", "check-links.mjs");
run("base", "check-base.mjs");

function haveDeps() {
  try {
    const url = import.meta.resolve("playwright");
    return existsSync(fileURLToPath(url));
  } catch {
    return false;
  }
}

if (haveDeps()) {
  run("browser", "browser.test.mjs");
  run("a11y", "a11y.test.mjs");
} else {
  console.log("\n=== browser/a11y ===\nSKIPPED: Playwright not installed.");
  console.log("Install with: (cd website && npm install && npx playwright install chromium)");
}

// Also re-verify the Playground's own suite, since the website reuses its
// engine and must not regress it.
const playgroundSuite = join(repoRoot, "playground", "tests", "node", "run-all.mjs");
if (existsSync(playgroundSuite)) {
  process.stdout.write("\n=== playground (reused engine) ===\n");
  execFileSync("node", [playgroundSuite], { stdio: "inherit" });
}

console.log("\nWebsite suite complete.");
