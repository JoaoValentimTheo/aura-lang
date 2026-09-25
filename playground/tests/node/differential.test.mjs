// Native/WASM differential parity for the Playground engine.
//
// The architectural rule established by Gaiola 6 is that a program valid under
// the semantic AST limit must be accepted on every execution substrate. This
// test runs a corpus through both engines -- the native build of the same
// `execute` engine, and the real wasm artifact via the browser ABI -- and
// requires the structured results (status, stdout, diagnostics) to agree.
//
// Usage:
//   node playground/tests/node/differential.test.mjs <runtime.wasm> <native-bin>

import { readFileSync, writeFileSync, mkdtempSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { AuraRuntime } from "../../web/runtime.mjs";

const wasmPath = process.argv[2];
const nativeBin = process.argv[3];
if (!wasmPath || !nativeBin) {
  console.error("usage: differential.test.mjs <runtime.wasm> <native-bin>");
  process.exit(2);
}

const runtime = await AuraRuntime.fromBytes(readFileSync(wasmPath), "differential");
const dir = mkdtempSync(join(tmpdir(), "aura-diff-"));

// Cases spanning the G6 boundary battery and the core language surface.
const cases = [
  ['print(1 + 2)', 'fn main() {\n print(1 + 2)\n}'],
  ['functions', 'fn add(a, b) -> int { return a + b }\nfn main() { print(add(2, 3)) }'],
  ['recursion fib', 'fn f(n) -> int {\n if n < 2 { return n }\n return f(n - 1) + f(n - 2)\n}\nfn main() { print(f(12)) }'],
  ['closures', 'fn main() {\n let n = 7\n let g = (x) -> x + n\n print(g(3))\n}'],
  ['structs', 'struct P { x: int, y: int }\nfn main() {\n let p = P { x: 2, y: 5 }\n print(p.x * p.y)\n}'],
  ['enums', 'enum E { A(int), B(int) }\nfn main() { print(match A(5) { A(v) -> v\n B(w) -> w }) }'],
  ['collections', 'fn main() {\n let m = {"k": [1, 2, 3]}\n print(len(m))\n print(len(m["k"]))\n}'],
  ['control flow', 'fn main() {\n let mut s = 0\n for i in range(0, 10) { if i == 5 { break }\n s = s + i }\n print(s)\n}'],
  // Language evolution: general unions, range literal, multiline comments.
  ['union annotation', 'type Number = int | float\nfn main() {\n let a: Number = 1\n let b: Number = 2.5\n print(a)\n print(b)\n}'],
  ['union alias chain', 'type ID = string | int\ntype UserID = ID\nfn main() { let u: UserID = 5\n print(u) }'],
  ['union mismatch', 'type Number = int | float\nfn main() { let x: Number = "s" }'],
  ['range literal', 'fn main() {\n let mut s = 0\n for i in 0..5 { s = s + i }\n print(s)\n}'],
  ['range equivalence', 'fn main() { print(range(0, 10) == 0..10)\n print([1..3]) }'],
  ['range float bound', 'fn main() { print(1.5..3) }'],
  ['multiline comment', 'fn main() {\n <!-- a\n multi line\n comment --!>\n print(42)\n}'],
  ['multibyte comment', 'fn main() { <!-- λ🎉 世界 --!> print(1) }'],
  ['unterminated comment', 'fn main() { print(1) } <!-- nope'],
  ['try/catch', 'fn main() {\n try { throw "x" } catch e -> { print("c " + e) }\n}'],
  ['args', 'fn main() { print(args()) }'],
  ['stdin', 'fn main() { print(read_line()) }'],
  ['uncaught throw', 'fn main() { throw "boom" }'],
  ['div zero', 'fn main() { print(1 / 0) }'],
  ['overflow', 'fn main() { print(9223372036854775807 + 1) }'],
  ['type mismatch', 'fn main() { print(1 + "a") }'],
  ['undefined', 'fn main() { print(nope) }'],
  ['index', 'fn main() { print([1, 2][9]) }'],
  ['no match', 'fn main() { print(match 5 { 1 -> "a" }) }'],
  ['recursion limit', 'fn f() { f() }\nfn main() { f() }'],
  ['cyclic value', 'fn main() {\n let mut xs = [1]\n print(xs == xs)\n}'],
  ['fs unavailable', 'fn main() { read_file("/x") }'],
  ['clock unavailable', 'fn main() { time_unix() }'],
  ['sleep unavailable', 'fn main() { sleep_ms(1) }'],
  // AST-limit-plus-grouping (the Gaiola 6 G6-01 case).
  [
    'ast limit + grouping',
    `fn main() { print(${"[".repeat(253)}1${"]".repeat(253)}) }`,
  ],
  // Over-limit grouping must be E1015 on every substrate.
  [
    'over-limit grouping',
    `fn main() { print(${"[".repeat(400)}1${"]".repeat(400)}) }`,
  ],
  ['empty program', ''],
  ['just expr', '1 + 2'],
  ['unicode', 'fn main() { print("héllo λ 世界") }'],
];

const options = { args: ["alpha", "beta"], stdin: "line one\nline two\n" };

// Encode the options exactly as `runtime.mjs` does, so the native harness
// receives the same bytes the wasm ABI would.
const optionsFile = join(dir, "options.bin");
{
  const enc = new TextEncoder();
  const chunks = [];
  for (const a of options.args) chunks.push(enc.encode(`arg ${a}\n`));
  const body = enc.encode(options.stdin);
  chunks.push(enc.encode(`stdin-bytes ${body.length}\n`), body);
  writeFileSync(optionsFile, Buffer.concat(chunks.map((c) => Buffer.from(c))));
}

function norm(r) {
  // Compare only substrate-independent structure: diagnostics are compared by
  // code, not by human message wording.
  return JSON.stringify({
    status: r.status,
    stdout: r.stdout,
    result: r.result ?? null,
    codes: (r.diagnostics || []).map((d) => d.code),
  });
}

let passed = 0;
let failed = 0;
for (const [name, src] of cases) {
  const wasm = runtime.run(src, options);
  const srcFile = join(dir, `${name.replace(/[^a-z0-9]+/gi, "_")}.aura`);
  writeFileSync(srcFile, src);
  const nativeJson = execFileSync(nativeBin, [srcFile, optionsFile], { encoding: "utf8" }).trim();
  const native = JSON.parse(nativeJson);
  const a = norm(wasm);
  const b = norm(native);
  if (a === b) {
    passed += 1;
  } else {
    failed += 1;
    console.error(`FAIL ${name}\n  wasm:   ${a}\n  native: ${b}`);
  }
}

console.log(`\nDifferential: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
