// ABI and Aura-surface tests for the Playground WebAssembly runtime.
//
// These run the *real* wasm artifact produced from this repository through the
// same `runtime.mjs` loader the browser Worker uses: no reimplementation, no
// mocked semantics.
//
// Usage: node playground/tests/node/abi.test.mjs <path-to.wasm>

import { readFileSync } from "node:fs";
import { AuraRuntime } from "../../web/runtime.mjs";

const wasmPath = process.argv[2];
if (!wasmPath) {
  console.error("usage: node abi.test.mjs <runtime.wasm>");
  process.exit(2);
}

const bytes = readFileSync(wasmPath);
const runtime = await AuraRuntime.fromBytes(bytes, "test");

let passed = 0;
let failed = 0;

function check(name, cond, detail) {
  if (cond) {
    passed += 1;
  } else {
    failed += 1;
    console.error(`FAIL ${name}${detail ? `: ${detail}` : ""}`);
  }
}

function run(source, options) {
  return runtime.run(source, options);
}

function expectOk(name, source, stdout, options) {
  const r = run(source, options);
  check(name, r.status === "ok", `${r.status} ${JSON.stringify(r.diagnostics)}`);
  if (stdout !== undefined) {
    check(`${name} [stdout]`, r.stdout === stdout, JSON.stringify(r.stdout));
  }
}

function expectCode(name, source, code, options) {
  const r = run(source, options);
  check(name, r.status !== "ok", "expected failure");
  check(
    `${name} [code]`,
    r.diagnostics.some((d) => d.code === code),
    JSON.stringify(r.diagnostics),
  );
}

// --- ABI identity ---------------------------------------------------------
check("abi version is 1", runtime.abiVersion === 1, String(runtime.abiVersion));
check("language version is 0.0.1", runtime.languageVersion === "0.0.1", runtime.languageVersion);
check(
  "module has zero imports",
  WebAssembly.Module.imports(await WebAssembly.compile(bytes)).length === 0,
);

// --- core language surfaces ----------------------------------------------
expectOk("print arithmetic", 'fn main() {\n print(1 + 2)\n}', "3\n");
expectOk("functions", "fn add(a, b) { a + b }\nfn main() { print(add(2, 3)) }", "5\n");
expectOk(
  "closures",
  "fn main() {\n let n = 10\n let f = (x) -> x + n\n print(f(5))\n}",
  "15\n",
);
expectOk(
  "structs",
  "struct P { x: int, y: int }\nfn main() {\n let p = P { x: 1, y: 2 }\n print(p.x + p.y)\n}",
  "3\n",
);
expectOk(
  "enums/match",
  "enum Color { Red, Green, Blue }\nfn main() {\n let c = Green()\n match c {\n   Red -> print(\"r\")\n   Green -> print(\"g\")\n   Blue -> print(\"b\")\n }\n}",
  "g\n",
);
expectOk(
  "collections",
  "fn main() {\n let xs = [1, 2, 3]\n let m = {\"a\": 1}\n print(len(xs) + len(m))\n}",
  "4\n",
);
expectOk(
  "control flow",
  "fn main() {\n let mut s = 0\n for i in range(0, 5) { s = s + i }\n print(s)\n}",
  "10\n",
);
expectOk(
  "try/catch",
  'fn main() {\n try { throw "boom" } catch e -> { print("caught " + e) }\n}',
  "caught boom\n",
);

// --- host capabilities ----------------------------------------------------
expectOk("args", "fn main() { print(args()) }", '["a", "b"]\n', { args: ["a", "b"] });
expectOk("stdin", "fn main() { print(read_line()) }", "hello\n", { stdin: "hello\n" });
expectCode("read_file E5002", 'fn main() { read_file("/x") }', 5002);
expectCode("write_file E5002", 'fn main() { write_file("/x", "y") }', 5002);
expectCode("clock E5002", "fn main() { time_unix() }", 5002);
expectCode("sleep E5002", "fn main() { sleep_ms(1) }", 5002);

// --- structured diagnostics ----------------------------------------------
expectCode("uncaught throw E4026", 'fn main() { throw "x" }', 4026);
expectCode("recursion E4011", "fn f() { f() }\nfn main() { f() }", 4011);
expectCode("type mismatch E3001", 'fn main() { let x = 1 + "a"\n print(x) }', 3001);
expectCode("nesting E1015", `fn main() { print(${"[".repeat(300)}1${"]".repeat(300)}) }`, 1015);
expectCode("capability E5002", "fn main() { time_now() }", 5002);

// --- no-main fallback (eval semantics) ------------------------------------
{
  const r = run("1 + 2");
  check("eval fallback ok", r.status === "ok", JSON.stringify(r));
  check("eval fallback result", r.result === "3", String(r.result));
}

// --- determinism ----------------------------------------------------------
{
  const a = run('fn main() { print("x") }');
  const b = run('fn main() { print("x") }');
  check("determinism", JSON.stringify(a) === JSON.stringify(b));
}

console.log(`\nABI: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
