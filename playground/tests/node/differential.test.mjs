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
  // Nested call arguments are the most frame-expensive recursive parser path
  // (expr -> unary -> postfix -> atom -> call_args -> cons_arg -> expr). On a
  // default 1 MiB wasm linear stack it exhausted the stack at ~907 frames,
  // below the 1024-frame wasm backstop, and trapped (`memory access out of
  // bounds`) where native reported E1015. `playground/runtime/build.rs`
  // reserves a 4 MiB wasm stack so every path reaches the backstop. These
  // depths bracket the historical onset (905/907/1000/1200).
  ...[905, 907, 1000, 1200].map((d) => [
    `nested len calls d=${d}`,
    `fn main() { print(${"len(".repeat(d)}[1]${")".repeat(d)}) }`,
  ]),
  ...[907, 1000, 1200].map((d) => [
    `nested id calls d=${d}`,
    `fn id(x) { return x }\nfn main() { print(${"id(".repeat(d)}1${")".repeat(d)}) }`,
  ]),
  ['empty program', ''],
  ['just expr', '1 + 2'],
  ['unicode', 'fn main() { print("héllo λ 世界") }'],
  // A function parameter list declaring the same name twice is a same-scope
  // redeclaration: E2007 on every substrate (formerly E1006 from the parser,
  // which disagreed with the lambda form).
  ['duplicate fn parameter', 'fn f(a, a) { return a }\nfn main() { }'],
  [
    'duplicate fn parameter annotated',
    'fn f(a: int, a: string) { return a }\nfn main() { }',
  ],
  ['duplicate lambda parameter', 'fn main() { let g = (a, a) -> a }'],
  // Struct methods (`impl`, explicit `self`) — §17.6. Behavior must be
  // identical on native and wasm, including diagnostics.
  [
    'method reads fields',
    'struct Point { x: int, y: int }\nimpl Point {\n fn sum(self) { return self.x + self.y }\n}\nfn main() { print(Point { x: 1, y: 2 }.sum()) }',
  ],
  [
    'method mutates receiver',
    'struct C { n: int }\nimpl C {\n fn bump(self, by: int) { self.n = self.n + by }\n}\nfn main() { let c = C { n: 0 }\n c.bump(5)\n print(c.n) }',
  ],
  [
    'method calls method',
    'struct P { x: int }\nimpl P {\n fn base(self) { return self.x }\n fn twice(self) { return self.base() * 2 }\n}\nfn main() { print(P { x: 5 }.twice()) }',
  ],
  [
    'method through composition',
    'struct Inner { v: int }\nstruct Outer { inner: Inner }\nimpl Inner {\n fn get(self) { return self.v }\n}\nfn main() { let o = Outer { inner: Inner { v: 7 } }\n print(o.inner.get()) }',
  ],
  [
    'method via alias',
    'struct P { x: int }\nimpl P {\n fn get(self) { return self.x }\n}\ntype Q = P\nfn main() { let q: Q = P { x: 3 }\n print(q.get()) }',
  ],
  [
    'methods keep equality',
    'struct P { x: int }\nimpl P {\n fn get(self) { return self.x }\n}\nfn main() { print(P { x: 1 } == P { x: 1 }) }',
  ],
  [
    'unknown method on struct',
    'struct P { x: int }\nfn main() { let p = P { x: 1 }\n print(p.nope()) }',
  ],
  [
    'field method collision',
    'struct P { x: int }\nimpl P {\n fn x(self) { return 1 }\n}\nfn main() { print(1) }',
  ],
  [
    'method without parens is field read',
    'struct P { x: int }\nimpl P {\n fn m(self) { return 1 }\n}\nfn main() { let p = P { x: 1 }\n print(p.m) }',
  ],
  [
    'method arguments',
    'struct P { x: int }\nimpl P {\n fn add(self, n: int) { return self.x + n }\n}\nfn main() { print(P { x: 1 }.add(2)) }',
  ],
  [
    'method names are type scoped',
    'struct A { x: int }\nstruct B { y: int }\nimpl A {\n fn get(self) { return self.x }\n}\nimpl B {\n fn get(self) { return self.y }\n}\nfn main() { print(A { x: 1 }.get())\n print(B { y: 2 }.get()) }',
  ],
  [
    'mutation through shared reference',
    'struct C { n: int }\nimpl C {\n fn set(self, v: int) { self.n = v }\n}\nfn main() { let a = C { n: 0 }\n let b = a\n a.set(9)\n print(b.n) }',
  ],
  [
    'unknown impl target',
    'impl Nope {\n fn f(self) { return 1 }\n}\nfn main() { print(1) }',
  ],
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

/** Run on wasm, turning a guest trap into a comparable sentinel. */
function wasmResult(src) {
  try {
    return { text: norm(runtime.run(src, options)), trap: null };
  } catch (err) {
    return { text: `trap:${err && err.message ? err.message : String(err)}`, trap: err };
  }
}

let passed = 0;
let failed = 0;
for (const [name, src] of cases) {
  const wasm = wasmResult(src);
  const srcFile = join(dir, `${name.replace(/[^a-z0-9]+/gi, "_")}.aura`);
  writeFileSync(srcFile, src);
  const nativeJson = execFileSync(nativeBin, [srcFile, optionsFile], { encoding: "utf8" }).trim();
  const native = JSON.parse(nativeJson);
  const a = wasm.text;
  const b = norm(native);
  if (a === b) {
    passed += 1;
  } else {
    failed += 1;
    console.error(`FAIL ${name}\n  wasm:   ${a}\n  native: ${b}`);
  }
}

// A rejected over-deep program must leave the wasm instance usable: the trap
// below the backstop used to poison the instance for every later run
// (`memory access out of bounds` on all subsequent calls). Run a known-good
// program after the deepest rejected one and require it to still succeed.
const postError = wasmResult("fn main() { print(1 + 2) }");
if (postError.text === norm({ status: "ok", stdout: "3\n", result: null, diagnostics: [] })) {
  passed += 1;
} else {
  failed += 1;
  console.error(`FAIL instance recovery after rejected deep input\n  wasm: ${postError.text}`);
}

console.log(`\nDifferential: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
