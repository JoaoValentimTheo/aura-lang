// Validate every runnable website example against the real Aura WebAssembly
// runtime. A published example must be a tested example: this script fails if
// any example is rejected by the runtime or produces output different from the
// declared `output`.
//
// Usage:
//   node website/tests/validate-examples.mjs [path/to/runtime.wasm]
//
// Defaults to the current Playground runtime from the manifest.

import { readFileSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../..");
const { examples } = await import(join(here, "../examples/examples.mjs"));
const { AuraRuntime } = await import(join(repoRoot, "playground/web/runtime.mjs"));

function resolveRuntime() {
  if (process.argv[2]) return process.argv[2];
  const manifest = JSON.parse(
    readFileSync(join(repoRoot, "playground/runtimes/manifest.json"), "utf8"),
  );
  const current = manifest.versions.find(
    (v) => v.id === manifest.current && v.available,
  );
  return join(repoRoot, "playground/runtimes", current.artifact);
}

const wasmPath = resolveRuntime();
if (!existsSync(wasmPath)) {
  console.error(`validate-examples: runtime not found: ${wasmPath}`);
  process.exit(2);
}

const runtime = await AuraRuntime.fromBytes(readFileSync(wasmPath), "examples");

let passed = 0;
let failed = 0;
for (const ex of examples) {
  const options = {
    args: ex.id === "args" ? ["Ada"] : [],
    stdin: ex.id === "stdin" ? "hello\naura\n" : null,
  };
  const result = runtime.run(ex.source, options);
  if (result.status !== "ok") {
    failed += 1;
    console.error(
      `FAIL ${ex.id}: status=${result.status} ${JSON.stringify(result.diagnostics)}`,
    );
    continue;
  }
  if (ex.output !== undefined && result.stdout !== ex.output) {
    failed += 1;
    console.error(
      `FAIL ${ex.id}: output mismatch\n  expected ${JSON.stringify(ex.output)}\n  actual   ${JSON.stringify(result.stdout)}`,
    );
    continue;
  }
  passed += 1;
}

// Also verify the repository's own examples/tour.aura still runs, so the
// website's tour example and the repository example cannot drift silently.
try {
  const tour = readFileSync(join(repoRoot, "examples/tour.aura"), "utf8");
  const r = runtime.run(tour, {});
  if (r.status === "ok") {
    passed += 1;
  } else {
    failed += 1;
    console.error(`FAIL repo examples/tour.aura: ${JSON.stringify(r.diagnostics)}`);
  }
} catch (err) {
  failed += 1;
  console.error(`FAIL repo examples/tour.aura: ${err.message}`);
}

console.log(`\nExamples: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
