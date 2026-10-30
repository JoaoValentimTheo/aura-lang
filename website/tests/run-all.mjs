// Run the full website validation suite.
//
//   node website/tests/run-all.mjs
//
// Static suites always run. The four browser suites (aurea-browser, nav-fit,
// browser, a11y) all need Playwright, and a11y also needs axe-core, so their
// availability is decided *here*, once: when the dependencies are absent this
// runner prints one SKIPPED line naming every suite it did not run. It never
// lets a suite exit 0 on its own behalf, because an unconditional
// `process.exit(0)` inside a suite is indistinguishable from passing.
//
// The build must exist and be current. Seven of the suites below read
// `website/dist` directly, so a stale build is worse than a missing one: it
// silently validates old HTML against new sources. `ensureSite` (shared with
// the Playground suites, which have the same dependency) builds only when the
// served copy is older than the sources it was generated from.

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { detectAll, skipExitCode } from "./playwright-deps.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../..");
const dist = join(repoRoot, "website", "dist");

const { ensureSite } = await import(
  join(repoRoot, "playground", "tests", "node", "ensure-site.mjs")
);
ensureSite();

if (!existsSync(dist)) {
  console.error("website/tests: dist missing; run `node website/build.mjs` first");
  process.exit(1);
}

function run(name, script, args = []) {
  process.stdout.write(`\n=== ${name} ===\n`);
  execFileSync("node", [join(here, script), ...args], { stdio: "inherit" });
}

run("examples", "validate-examples.mjs");
run("aura-comments", "check-aura-comments.mjs");
run("links", "check-links.mjs");
run("base", "check-base.mjs");
run("release-version", "release-version.test.mjs");
run("stdlib-consistency", "stdlib-consistency.test.mjs");
run("docs-consistency", "docs-consistency.test.mjs");
run("ais-doc", "ais-doc.test.mjs");
run("aurea", "aurea.test.mjs");
run("playground-parity", "playground-parity.test.mjs");

// `detectAll` throws if playwright is present but broken, so a failed `npm ci`
// or a half-downloaded browser can never be reported as a clean skip. It also
// checks `axe-core`, because the a11y suite needs it and the runner must reach
// the same verdict the suite would.
const deps = detectAll(import.meta.url, ["axe-core/axe.min.js"]);
if (deps.ok) {
  run("aurea-browser", "aurea-browser.test.mjs");
  run("nav-fit", "nav-fit.test.mjs");
  run("browser", "browser.test.mjs");
  run("a11y", "a11y.test.mjs");
} else {
  const code = skipExitCode(deps, "aurea-browser/nav-fit/browser/a11y");
  if (code !== 0) process.exit(code);
}

// Also re-verify the Playground's own suite, since the website reuses its
// engine and must not regress it.
const playgroundSuite = join(repoRoot, "playground", "tests", "node", "run-all.mjs");
if (existsSync(playgroundSuite)) {
  process.stdout.write("\n=== playground (reused engine) ===\n");
  execFileSync("node", [playgroundSuite], { stdio: "inherit" });
}

console.log("\nWebsite suite complete.");
