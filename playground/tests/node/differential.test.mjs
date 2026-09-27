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
  // (expr -> unary -> postfix -> atom -> call_args -> cons_arg -> expr). The
  // semantic AST-node limit (§31.1) is enforced *during* parsing, so a chain
  // deeper than the limit reports E1015 before the engine stack is reached on
  // every substrate. These depths (all well past the limit) must yield E1015,
  // never a trap, on native and wasm alike. They bracket and extend the
  // historical onset depths.
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
  // Traits (`trait`, `impl Trait for Struct`) — §17.7. Static/nominal: the
  // contract is checked, the methods merge into the struct's method surface,
  // and there is no dynamic dispatch.
  [
    'trait basic',
    'trait P { fn show(self) }\nstruct Person { name: string }\nimpl P for Person { fn show(self) { print(self.name) } }\nfn main() { Person { name: "Ada" }.show() }',
  ],
  [
    'trait args and return',
    'trait S { fn a(self) -> int\n fn b(self, k: int) -> int }\nstruct X { v: int }\nimpl S for X { fn a(self) { return self.v }\n fn b(self, k: int) { return self.v * k } }\nfn main() { let x = X { v: 3 }\n print(x.a())\n print(x.b(4)) }',
  ],
  [
    'trait method mutation',
    'trait I { fn bump(self) }\nstruct C { n: int }\nimpl I for C { fn bump(self) { self.n = self.n + 1 } }\nfn main() { let c = C { n: 0 }\n c.bump()\n c.bump()\n print(c.n) }',
  ],
  [
    'trait composition',
    'trait V { fn val(self) -> int }\nstruct E { p: int }\nstruct Car { e: E }\nimpl V for E { fn val(self) -> int { return self.p } }\nimpl V for Car { fn val(self) -> int { return self.e.val() } }\nfn main() { print(Car { e: E { p: 120 } }.val()) }',
  ],
  [
    'trait via alias',
    'trait T { fn a(self) }\nstruct S { n: int }\nimpl T for S { fn a(self) { print(self.n) } }\ntype Q = S\nfn main() { let q: Q = S { n: 5 }\n q.a() }',
  ],
  [
    'trait missing method',
    'trait T { fn a(self)\n fn b(self) }\nstruct S { x: int }\nimpl T for S { fn a(self) { print(self.x) } }\nfn main() { print(1) }',
  ],
  [
    'trait wrong signature',
    'trait T { fn a(self, x: int) }\nstruct S { x: int }\nimpl T for S { fn a(self) { print(self.x) } }\nfn main() { print(1) }',
  ],
  [
    'trait extra method',
    'trait T { fn a(self) }\nstruct S { x: int }\nimpl T for S { fn a(self) { print(self.x) }\n fn b(self) { print(2) } }\nfn main() { print(1) }',
  ],
  [
    'trait unknown trait',
    'struct S { x: int }\nimpl Nope for S { fn a(self) { print(1) } }\nfn main() { print(1) }',
  ],
  [
    'trait duplicate impl',
    'trait T { fn a(self) }\nstruct S { x: int }\nimpl T for S { fn a(self) { print(1) } }\nimpl T for S { fn a(self) { print(2) } }\nfn main() { print(1) }',
  ],
  [
    'trait inherent method clash',
    'trait T { fn a(self) }\nstruct S { x: int }\nimpl S { fn a(self) { print(1) } }\nimpl T for S { fn a(self) { print(2) } }\nfn main() { print(1) }',
  ],
  [
    'trait field clash',
    'trait T { fn x(self) }\nstruct S { x: int }\nimpl T for S { fn x(self) { print(1) } }\nfn main() { print(1) }',
  ],
  [
    'trait method on unknown receiver',
    'trait T { fn a(self) }\nstruct S { n: int }\nimpl T for S { fn a(self) { print(self.n) } }\nfn call(x) { x.a() }\nfn main() { call(S { n: 4 }) }',
  ],
  [
    'trait union receiver',
    'trait T { fn a(self) -> int }\nstruct A { x: int }\nstruct B { y: int }\nimpl T for A { fn a(self) -> int { return self.x } }\nimpl T for B { fn a(self) -> int { return self.y } }\ntype U = A | B\nfn main() { let u: U = A { x: 1 }\n print(u.a()) }',
  ],
  [
    'trait is contextual',
    'fn main() { let trait = 1\n let impl = 2\n print(trait + impl) }',
  ],
  // Deterministic, declaration-ordered reporting: the missing/signature-bad
  // method is named consistently on both substrates, never by hash order.
  [
    'trait missing method ordering',
    'trait T { fn a(self)\n fn b(self)\n fn c(self)\n fn d(self) }\nstruct S { x: int }\nimpl T for S { fn a(self) { print(self.x) } }\nfn main() { print(1) }',
  ],
  [
    'trait signature mismatch ordering',
    'trait T { fn a(self, x: int)\n fn b(self, y: int)\n fn c(self, z: int) }\nstruct S { n: int }\nimpl T for S { fn a(self) { print(1) }\n fn b(self) { print(2) }\n fn c(self) { print(3) } }\nfn main() { print(1) }',
  ],
  // A trait method may not declare a parameter name twice (E2007), matching
  // functions and inherent methods.
  [
    'trait duplicate parameter',
    'trait T { fn a(self, x: int, x: int) }\nfn main() { print(1) }',
  ],
  // LSCS: `const` is the canonical module constant; `let` remains compatible.
  [
    'const declaration',
    'const PI = 3\nconst N: int = 5\nfn main() { print(PI)\n print(N) }',
  ],
  ['const lowercase rejected', 'const pi = 3\nfn main() { print(pi) }'],
  ['const forward reference', 'const A = B\nconst B = 1\nfn main() { print(A) }'],
  ['const duplicate', 'const A = 1\nconst A = 2\nfn main() { print(A) }'],
  [
    'const interior mutability',
    'const XS = [1, 2]\nfn main() { push(XS, 3)\n print(XS) }',
  ],
  [
    'const and let share namespace',
    'const A = 1\nlet A = 2\nfn main() { print(A) }',
  ],
  [
    'value and type namespaces are separate',
    'fn S() { return 1 }\nstruct S { a: int }\nfn main() { print(S { a: 5 }.a) }',
  ],
  // Shadowing: `let`/`let mut` create a new binding; `const` does not shadow.
  ['shadow same scope', 'fn main() { let x = 1\n let x = 2\n print(x) }'],
  [
    'shadow initializer reads previous binding',
    'fn main() { let x = 11\n let x = x + 10\n print(x) }',
  ],
  [
    'mut shadowed by immutable then assign',
    'fn main() { let mut x = 10\n let x = 20\n x = 30 }',
  ],
  [
    'immutable shadowed by mutable then assign',
    'fn main() { let x = 1\n let mut x = 2\n x = 3\n print(x) }',
  ],
  [
    'nested shadow restores',
    'fn main() { let x = 1\n { let x = 2\n print(x) }\n print(x) }',
  ],
  [
    'closure keeps captured binding across shadow',
    'fn main() { let x = 10\n let f = () -> x\n let x = 20\n print(f())\n print(x) }',
  ],
  ['const is not shadowable', 'const X = 1\nconst X = 2\nfn main() { print(X) }'],
  // LSCS: f-string interpolation diagnostics carry an absolute span.
  ['fstring undefined name', 'fn main() {\n print(f"value {nope} end")\n}'],
  ['fstring empty interpolation', 'fn main() {\n print(f"x{ }y")\n}'],
  [
    'fstring expression and escapes',
    'fn main() { let x = 5\n print(f"v={x + 1} {{lit}}") }',
  ],
  // LSCS: operators deliberately absent.
  ['bitwise rejected', 'fn main() { print(6 & 3) }'],
  ['shift rejected', 'fn main() { print(8 >> 1) }'],
  ['increment rejected', 'fn main() { let mut x = 1\n x++\n print(x) }'],
  ['caret is power', 'fn main() { print(2 ^ 10) }'],
  // Bitwise operators and compound assignments.
  ['bitwise and or', 'fn main() { print(6 & 3)\n print(6 | 1) }'],
  ['bitwise not', 'fn main() { print(~0) }'],
  ['shift', 'fn main() { print(1 << 4)\n print(256 >> 4) }'],
  ['bitwise precedence', 'fn main() { print(1 | 2 & 3)\n print(1 << 2 + 1) }'],
  ['shift out of range', 'fn main() { print(1 << 64) }'],
  ['bitwise requires int', 'fn main() { print(1.0 & 2) }'],
  ['compound bitwise assign', 'fn main() { let mut x = 6\n x |= 1\n x &= 3\n print(x) }'],
  ['compound shift assign', 'fn main() { let mut x = 1\n x <<= 5\n x >>= 2\n print(x) }'],
  // Mutation capability.
  ['push immutable rejected', 'fn main() { let xs = [1]\n xs.push(2) }'],
  ['push mutable allowed', 'fn main() { let mut xs = [1]\n xs.push(2)\n print(xs) }'],
  ['index immutable rejected', 'fn main() { let xs = [1]\n xs[0] = 9 }'],
  ['field immutable rejected', 'struct S { n: int }\nfn main() { let s = S { n: 0 }\n s.n = 1 }'],
  [
    'mut self required to mutate receiver',
    'struct S { n: int }\nimpl S { fn bump(self) { self.n = self.n + 1 } }\nfn main() { let mut s = S { n: 0 }\n s.bump() }',
  ],
  [
    'mut self with mutable receiver',
    'struct S { n: int }\nimpl S { fn bump(mut self) { self.n = self.n + 1 } }\nfn main() { let mut s = S { n: 0 }\n s.bump()\n print(s.n) }',
  ],
  ['const mutation rejected', 'const XS = [1]\nfn main() { push(XS, 2) }'],
  // f-string formatting.
  ['fstring precision', 'fn main() { print(f"{3.14159:.2f}") }'],
  ['fstring width align', "fn main() { print(f\"[{42:>6}]\\n[{'hi':<6}]\\n[{'hi':^6}]\") }"],
  ['fstring zero pad', 'fn main() { print(f"{42:06d}") }'],
  ['fstring sign', 'fn main() { print(f"{5:+}") }'],
  ['fstring hex bin', 'fn main() { print(f"{255:x} {10:b} {10:o}") }'],
  ['fstring percent exp', 'fn main() { print(f"{0.5:.1%} {1234.5:.2e}") }'],
  ['fstring type mismatch', "fn main() { print(f\"{'s':d}\") }"],
  ['fstring unknown type', 'fn main() { print(f"{5:z}") }'],
  ['fstring escapes still', 'fn main() { print(f"{{x}} {1 + 1}") }'],
  // Trailing comma uniformity.
  [
    'trailing comma everywhere',
    'struct S { a: int, }\nenum E { A(int, int), }\nfn f(a, b,) { return a + b }\nfn main() { print([1, 2,])\n print({\"k\": 1,})\n print(f(1, 2,))\n print(S { a: 1, }.a)\n match A(1, 2,) { A(x, y) -> print(x + y) } }',
  ],
  // Contextual `impl`/`self`: both stay ordinary identifiers outside a method
  // receiver / behavior-block context (§3.3).
  [
    'impl and self as identifiers',
    'fn main() { let impl = 1\n let self = 2\n print(impl + self) }',
  ],
  [
    'impl as function name, self as param',
    'fn self(x) { return x }\nstruct S { impl: int }\nfn main() { print(self(S { impl: 5 }.impl)) }',
  ],
  [
    'contextual impl block beside identifier impl',
    'struct P { x: int }\nimpl P { fn g(self) { return self.x } }\nlet impl = 2\nfn main() { print(P { x: 5 }.g())\n print(impl) }',
  ],
  [
    'for binding named self',
    'fn main() { for self in [1, 2] { print(self) } }',
  ],
  // Unknown receiver may resolve a user struct method (C4).
  [
    'unknown receiver user method',
    'struct P { x: int }\nimpl P { fn set(self, v: int) { self.x = v }\n fn get(self) { return self.x } }\nfn f(p) { p.set(7)\n print(p.get()) }\nfn main() { f(P { x: 0 }) }',
  ],
  // Union signature compatibility (C5): incompatible arity is rejected.
  [
    'union incompatible method signatures',
    'struct A { x: int }\nstruct B { y: int }\nimpl A { fn m(self, n: int) { return n } }\nimpl B { fn m(self) { return self.y } }\ntype U = A | B\nfn main() { let u: U = A { x: 1 }\n print(u.m()) }',
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
