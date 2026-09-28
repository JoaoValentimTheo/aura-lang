// Run the full Playground test suite.
//
// `node playground/tests/node/run-all.mjs`
//
// Always runs the runtime-independent suites (manifest/immutability, ABI via
// the real wasm artifact). The browser suites (integration + Worker lifecycle)
// run only when Playwright's Chromium is available; otherwise they are
// reported as skipped, never silently passed.

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const playground = resolve(here, "../..");
const manifestPath = join(playground, "runtimes/manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const current = manifest.versions.find((v) => v.id === manifest.current && v.available);
if (!current) {
  console.error("manifest has no current available version");
  process.exit(1);
}
const wasm = join(playground, "runtimes", current.artifact);

function run(name, args) {
  process.stdout.write(`\n=== ${name} ===\n`);
  execFileSync("node", args, { cwd: playground, stdio: "inherit" });
}

if (!existsSync(wasm)) {
  console.error(`missing runtime artifact ${wasm}; run: node playground/build.mjs`);
  process.exit(1);
}

run("manifest", [join(here, "manifest.test.mjs")]);
run("abi", [join(here, "abi.test.mjs"), wasm]);
// Load-time SHA-256 verification of the fetched artifact bytes (the loader
// must refuse a mismatched artifact before instantiation).
run("integrity", [join(here, "integrity.test.mjs")]);

// The native/wasm differential harness needs the native runner built; build it
// on demand so the parity gate is always exercised. A parity failure is a hard
// failure: it must never be reported as a skip, or CI would go green while
// native and wasm disagree. Only the build step may be skipped, and only when
// the native target genuinely cannot be built in this environment.
let differentialRan = false;
try {
  execFileSync(
    "cargo",
    ["build", "--release", "--bin", "aura-playground-native"],
    { cwd: join(playground, "runtime"), stdio: "pipe" },
  );
} catch (err) {
  console.error(
    `\n=== differential ===\nFAILED: could not build the native runner: ${String(err.message || err)}`,
  );
  process.exit(1);
}
{
  const nativeBin = join(playground, "runtime/target/release/aura-playground-native");
  if (!existsSync(nativeBin)) {
    console.error(`\n=== differential ===\nFAILED: native runner missing at ${nativeBin}`);
    process.exit(1);
  }
  // `run` propagates a non-zero exit (execFileSync throws), so a parity
  // failure stops the suite here with a non-zero status.
  run("differential", [join(here, "differential.test.mjs"), wasm, nativeBin]);
  run("syntax conformance", [join(here, "syntax.test.mjs"), wasm, nativeBin]);
  differentialRan = true;
}
if (!differentialRan) {
  console.error("\n=== differential ===\nFAILED: not run");
  process.exit(1);
}

let havePlaywright = false;
try {
  await import("playwright");
  havePlaywright = true;
} catch {
  havePlaywright = false;
}

if (havePlaywright) {
  run("browser", [join(here, "browser.test.mjs")]);
  run("worker", [join(here, "worker.test.mjs")]);
  run("cache", [join(here, "cache.test.mjs")]);
} else {
  console.log("\n=== browser/worker/cache ===\nSKIPPED: Playwright not installed.");
  console.log("Install with: (cd playground && npm install && npx playwright install chromium)");
}

console.log("\nPlayground suite complete.");
