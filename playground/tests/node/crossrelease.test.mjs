// Cross-release compatibility (TD-06): every *frozen* historical runtime must
// still run the fixtures that were valid when it shipped, producing the same
// output. This proves the project does not retroactively change released
// behavior, and that a frozen artifact remains usable.
//
// Each frozen runtime implements a specific language line; a fixture is only
// checked against runtimes whose language line accepts it. The point is not
// "old runtime accepts new syntax" (it must not) but "old runtime still runs
// the programs it always did, with unchanged output".
//
// Usage: node playground/tests/node/crossrelease.test.mjs

import { readFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../..");
const { AuraRuntime } = await import(join(repo, "playground/web/runtime.mjs"));

let passed = 0;
let failed = 0;
function check(name, cond, detail) {
  if (cond) passed += 1;
  else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

// Programs valid on every runtime line, from `0.0.1` to the current dev line.
// Their output is the compatibility contract for a released runtime.
const BASE_FIXTURES = [
  { id: "hello", src: 'fn main() { print("hello, Aura") }', out: "hello, Aura\n" },
  { id: "arith", src: "fn main() { print(1 + 2 * 3) }", out: "7\n" },
  { id: "fib", src: "fn fib(n) -> int { if n < 2 { return n }\n return fib(n-1) + fib(n-2) }\nfn main() { print(fib(10)) }", out: "55\n" },
  { id: "list", src: "fn main() {\n let mut xs = [1, 2]\n xs.push(3)\n print(xs)\n print(len(xs))\n}", out: "[1, 2, 3]\n3\n" },
  { id: "map", src: 'fn main() {\n let m = {"a": 1, "b": 2}\n print(len(m))\n print(m["a"])\n}', out: "2\n1\n" },
  { id: "string", src: 'fn main() { print(len("héllo"))\n print("a" + "b") }', out: "5\nab\n" },
  { id: "error", src: 'fn main() { print(1 / 0) }', out: null, expectError: true },
];

// Language features introduced on the `0.2.x` line (enums, `for`), so only
// runtimes whose language line includes them are checked against them.
const LINE_0_2_FIXTURES = [
  { id: "enum", src: 'enum Shape { Circle(int) }\nfn area(s) -> int { return match s { Circle(r) -> r * r } }\nfn main() { print(area(Shape::Circle(3))) }', out: "9\n" },
  { id: "for", src: "fn main() {\n let mut total = 0\n for i in 0..5 { total = total + i }\n print(total)\n}", out: "10\n" },
];

function fixturesFor(runtimeId, languageVersion) {
  // `languageVersion` is the language line the runtime implements. The 0.0.x
  // line predates enums and `for`.
  const is02OrLater = languageVersion !== undefined && !languageVersion.startsWith("0.0");
  return is02OrLater ? [...BASE_FIXTURES, ...LINE_0_2_FIXTURES] : BASE_FIXTURES;
}

const manifest = JSON.parse(
  readFileSync(join(repo, "playground/runtimes/manifest.json"), "utf8"),
);

// Every available runtime artifact, oldest first.
const runtimes = manifest.versions
  .filter((v) => v.available)
  .sort((a, b) => a.id.localeCompare(b.id, undefined, { numeric: true }));

check("the manifest exposes frozen runtimes", runtimes.length > 0);

for (const rt of runtimes) {
  const wasmPath = join(repo, "playground/runtimes", rt.artifact);
  if (!existsSync(wasmPath)) {
    failed += 1;
    console.error(`FAIL runtime ${rt.id}: artifact missing at ${rt.artifact}`);
    continue;
  }
  const runtime = await AuraRuntime.fromBytes(readFileSync(wasmPath), rt.id);
  const fixtures = fixturesFor(rt.id, rt.language_version);
  for (const fx of fixtures) {
    let result;
    try {
      result = runtime.run(fx.src, { args: [], stdin: null });
    } catch (e) {
      failed += 1;
      console.error(`FAIL ${rt.id}/${fx.id}: threw ${e.message}`);
      continue;
    }
    if (fx.expectError) {
      // A runtime error is still a structured, non-crashing result.
      check(
        `${rt.id}/${fx.id} errors structurally`,
        result.status !== "ok" && result.status !== "internal",
        `status=${result.status}`,
      );
      continue;
    }
    if (result.status !== "ok") {
      failed += 1;
      console.error(
        `FAIL ${rt.id}/${fx.id}: status=${result.status} ${JSON.stringify(result.diagnostics)}`,
      );
      continue;
    }
    check(`${rt.id}/${fx.id}`, result.stdout === fx.out, `got ${JSON.stringify(result.stdout)}`);
  }
}

console.log(`cross-release: ${passed} passed, ${failed} failed across ${runtimes.length} runtimes`);
if (failed > 0) process.exit(1);
