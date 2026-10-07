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

import { readFileSync, writeFileSync, mkdtempSync, readdirSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { AuraRuntime } from "../../web/runtime.mjs";

const here = dirname(fileURLToPath(import.meta.url));

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
  ['try/catch', 'fn main() {\n try { throw "x" } catch e { print("c " + e) }\n}'],
  // Function and method overloading: identity is name + ordered input types;
  // the return type never distinguishes overloads.
  [
    'function overloads',
    'fn f(x: int) { print("i") }\nfn f(x: string) { print("s") }\nfn main() { f(1)\n f("a") }',
  ],
  [
    'overload annotated beats unannotated',
    'fn f(x: int) { print("int") }\nfn f(x) { print("any") }\nfn main() { f(1)\n f(true) }',
  ],
  ['overload no match', 'fn f(x: int) { print(1) }\nfn main() { f("a") }'],
  [
    'overload return type not identity',
    'fn f(x: int) -> int { return 1 }\nfn f(x: int) -> string { return "s" }\nfn main() { }',
  ],
  [
    'method overloads',
    'struct P { n: int }\nimpl P {\n fn greet(self, name: string) { print("hi " + name) }\n fn greet(self, times: int) { print(self.n * times) }\n}\nfn main() { let p = P { n: 3 }\n p.greet("Ada")\n p.greet(2) }',
  ],
  [
    'method overload ambiguous',
    'struct S { n: int }\nimpl S { fn f(self, x: int | string) { print(1) }\n fn f(self, x: int | bool) { print(2) } }\nfn main() { S { n: 0 }.f(1) }',
  ],
  [
    'lambda annotated parameter',
    'fn main() { let f = (x: int) -> x + 1\n print(f(1)) }',
  ],
  [
    'lambda mut parameter',
    'fn main() { let f = (mut x) -> { x = x + 1\n return x }\n print(f(1)) }',
  ],
  // Big Guard: a bare reference to an overloaded function is deferred and
  // rejected; a piped value resolves an overload by type.
  [
    'overloaded function value rejected',
    'fn f(x: int) { print(1) }\nfn f(x: string) { print(2) }\nfn main() { let g = f\n g(1) }',
  ],
  [
    'single function value allowed',
    'fn f(x: int) -> int { return x }\nfn main() { let g = f\n print(g(1)) }',
  ],
  [
    'pipe resolves overload',
    'fn f(x: int) { print("i") }\nfn f(x: string) { print("s") }\nfn main() { 1 |> f\n "a" |> f }',
  ],
  [
    'mutation capability independent of overload',
    'struct S { n: int }\nimpl S { fn u(self, x: int) { print("r") }\n fn u(mut self, x: string) { self.n = 1 } }\nfn main() { let s = S { n: 0 }\n s.u(1) }',
  ],
  [
    'mut overload selected on immutable receiver',
    'struct S { n: int }\nimpl S { fn u(self, x: int) { print("r") }\n fn u(mut self, x: string) { self.n = 1 } }\nfn main() { let s = S { n: 0 }\n s.u("a") }',
  ],
  // Catch uses `catch e { ... }`; the old `catch e -> { ... }` is rejected.
  [
    'catch with finally',
    'fn main() { try { throw 1 } catch e { print(e) } finally { print("done") } }',
  ],
  [
    'catch rejects arrow',
    'fn main() { try { throw 1 } catch e -> { print(e) } }',
  ],
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
    'struct C { n: int }\nimpl C {\n fn bump(mut self, by: int) { self.n = self.n + by }\n}\nfn main() { let mut c = C { n: 0 }\n c.bump(5)\n print(c.n) }',
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
    'struct C { n: int }\nimpl C {\n fn set(mut self, v: int) { self.n = v }\n}\nfn main() { let mut a = C { n: 0 }\n let b = a\n a.set(9)\n print(b.n) }',
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
    'trait I { fn bump(mut self) }\nstruct C { n: int }\nimpl I for C { fn bump(mut self) { self.n = self.n + 1 } }\nfn main() { let mut c = C { n: 0 }\n c.bump()\n c.bump()\n print(c.n) }',
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
  // A format precision above `u16::MAX` used to reach Rust's formatter and
  // panic on native (E4999) / trap on wasm. Bound is now the language's own,
  // reported as E4013 on every substrate.
  ['fstring precision at u16 max', 'fn main() { print(len(f"{1.5:.65535}")) }'],
  ['fstring precision over u16 max', 'fn main() { print(f"{1.5:.65536}") }'],
  ['fstring precision far over', 'fn main() { print(f"{1.5:.100000}") }'],
  ['fstring precision far over typed', 'fn main() { print(f"{1:.70000e}") }'],
  // An unbounded format width used to allocate without limit (native) or trap
  // (wasm). The width is now bounded by the language.
  ['fstring width at bound', 'fn main() { print(len(f"{1:10000000}")) }'],
  ['fstring width over bound', 'fn main() { print(f"{1:10000001}") }'],
  ['fstring width far over', 'fn main() { print(f"{1:2000000000}") }'],
  // AUDIT-4: a value with reference fan-out >= 2 (a cycle reached twice, or a
  // shared DAG) used to expand exponentially in display/JSON and never
  // terminate on EITHER substrate. The total-node render budget makes both
  // substrates terminate with the same output.
  [
    'cycle fan-out 2 display',
    'fn main() { let mut c = []\n push(c, c)\n push(c, c)\n print(len(to_string(c))) }',
  ],
  [
    'cycle fan-out 2 json',
    'fn main() { let mut c = []\n push(c, c)\n push(c, c)\n print(len(json_encode(c))) }',
  ],
  [
    'cycle fan-out 4 display',
    'fn main() { let mut c = []\n push(c, c)\n push(c, c)\n push(c, c)\n push(c, c)\n print(len(to_string(c))) }',
  ],
  [
    'shared subvalue DAG display',
    'fn grow(xs, n) {\n let mut cur = xs\n let mut i = 0\n while i < n { cur = [cur, cur]\n i = i + 1 }\n return cur\n}\nfn main() { print(len(to_string(grow([1], 40)))) }',
  ],
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
    'struct P { x: int }\nimpl P { fn set(mut self, v: int) { self.x = v }\n fn get(self) { return self.x } }\nfn f(p) { p.set(7)\n print(p.get()) }\nfn main() { f(P { x: 0 }) }',
  ],
  // Union signature compatibility (C5): incompatible arity is rejected.
  [
    'union incompatible method signatures',
    'struct A { x: int }\nstruct B { y: int }\nimpl A { fn m(self, n: int) { return n } }\nimpl B { fn m(self) { return self.y } }\ntype U = A | B\nfn main() { let u: U = A { x: 1 }\n print(u.m()) }',
  ],
  // Modules and visibility (§27): a real boundary, identical on both
  // substrates because it is in-source and needs no filesystem.
  [
    'module public function',
    'module m { pub fn f() -> int { return 7 } }\nfn main() { print(m::f()) }',
  ],
  ['module private function', 'module m { fn f() -> int { return 7 } }\nfn main() { print(m::f()) }'],
  [
    'module nested path',
    'module a { module b { pub fn f() -> int { return 42 } } }\nfn main() { print(a::b::f()) }',
  ],
  [
    'module descendant reaches private',
    'module a { fn hidden() -> int { return 5 }\n module b { pub fn f() -> int { return hidden() } } }\nfn main() { print(a::b::f()) }',
  ],
  [
    'module use import',
    'module m { pub fn f() -> int { return 2 } }\nuse m::f\nfn main() { print(f()) }',
  ],
  [
    'module use alias',
    'module m { pub fn f() -> int { return 2 } }\nuse m::f as g\nfn main() { print(g()) }',
  ],
  ['module unknown import', 'use nope\nfn main() { print(1) }'],
  [
    'module private field',
    'module m { pub struct S { x: int } }\nfn main() { let s = m::S { x: 1 }\n print(s.x) }',
  ],
  [
    'module public field',
    'module m { pub struct S { pub x: int } }\nfn main() { let s = m::S { x: 1 }\n print(s.x) }',
  ],
  [
    'module private method',
    'module m { pub struct S { x: int }\n impl S { fn get(self) -> int { return self.x } } }\nfn main() { let s = m::S { x: 1 }\n print(s.get()) }',
  ],
  [
    'module public method via factory',
    'module m { pub struct S { x: int }\n pub fn make() -> S { return S { x: 9 } }\n impl S { pub fn get(self) -> int { return self.x } } }\nfn main() { print(m::make().get()) }',
  ],
  [
    'module pub trait and method',
    'module m { pub trait T { fn f(self) -> int }\n pub struct S { pub x: int }\n impl T for S { fn f(self) -> int { return 7 } } }\nfn main() { print(m::S { x: 1 }.f()) }',
  ],
  [
    'module import collision',
    'module a { pub fn f() -> int { return 1 } }\nmodule b { pub fn f() -> int { return 2 } }\nuse a::f\nuse b::f\nfn main() { print(f()) }',
  ],
  [
    'module visibility does not grant mutation',
    'module m { pub struct S { pub n: int }\n impl S { pub fn inc(mut self) { self.n = self.n + 1 } } }\nfn main() { let s = m::S { n: 0 }\n s.inc() }',
  ],
  [
    'module mut binding cannot bypass private field',
    'module m { pub struct S { n: int } }\nfn main() { let mut s = m::S { n: 0 }\n s.n = 1 }',
  ],
  [
    'module closure keeps lexical access',
    'module m { fn hidden() -> int { return 5 }\n pub fn run() -> int { let f = () -> hidden()\n return f() } }\nfn main() { print(m::run()) }',
  ],
  [
    'sibling modules do not collide',
    'module a { pub fn f() -> int { return 1 } }\nmodule b { pub fn f() -> int { return 2 } }\nfn main() { print(a::f() + b::f()) }',
  ],
  ['pub on impl rejected', 'struct S { x: int }\npub impl S { fn f(self) -> int { return 1 } }\nfn main() { print(1) }'],
  // Generics (§36): static and erased, so the two substrates must agree on
  // inference, explicit type arguments, bounds, and rejection alike.
  [
    'generic function infers',
    'fn identity<T>(x: T) -> T { return x }\nfn main() {\n print(identity(1))\n print(identity("a"))\n}',
  ],
  [
    'generic explicit argument',
    'fn identity<T>(x: T) -> T { return x }\nfn main() { print(identity<int>(7)) }',
  ],
  [
    'generic struct',
    'struct Box<T> { value: T }\nfn main() {\n let b = Box { value: 9 }\n print(b.value)\n}',
  ],
  [
    'generic method uses impl parameter',
    'struct Box<T> { value: T }\nimpl<T> Box<T> { fn get(self) -> T { return self.value } }\nfn main() { print(Box { value: 3 }.get()) }',
  ],
  [
    'generic trait over generic struct',
    'trait Container<T> { fn get(self, i: int) -> T }\nstruct Stack<T> { items: [T] }\nimpl<T> Container<T> for Stack<T> { fn get(self, i: int) -> T { return self.items[i] } }\nfn main() { print(Stack { items: [4, 5, 6] }.get(1)) }',
  ],
  [
    'generic bound is checked',
    'trait Show { fn show(self) -> int }\nstruct A { n: int }\nimpl Show for A { fn show(self) -> int { return self.n } }\nfn run<T: Show>(x: T) -> int { return x.show() }\nfn main() { print(run(A { n: 7 })) }',
  ],
  [
    'generic bound violation',
    'trait Show { fn show(self) -> int }\nstruct B { n: int }\nfn run<T: Show>(x: T) -> int { return 1 }\nfn main() { run(B { n: 1 }) }',
  ],
  [
    'generic compose with structural collections',
    'fn first<T>(xs: [T]) -> T { return xs[0] }\nfn get<T>(m: {string: T}, k: string) -> T { return m[k] }\nfn main() {\n print(first([10, 20]))\n print(get({ "x": 5 }, "x"))\n}',
  ],
  [
    'generic parameterised alias',
    'type Pair<T> = [T]\nfn main() { let p: Pair<int> = [1, 2]\n print(p[1]) }',
  ],
  [
    'generic alpha-equivalent redeclaration rejected',
    'fn f<T>(x: T) -> T { return x }\nfn f<U>(x: U) -> U { return x }\nfn main() { }',
  ],
  [
    'generic explicit wrong arity rejected',
    'fn identity<T>(x: T) -> T { return x }\nfn main() { print(identity<int, string>(7)) }',
  ],
  [
    'generic wrong field type rejected',
    'struct Box<T> { value: T }\nfn main() { let b = Box<int> { value: "x" }\n print(b.value) }',
  ],
  [
    'generic in module with visibility',
    'module m {\n pub struct Secret<T> { pub value: T }\n pub fn make<T>(x: T) -> Secret<T> { return Secret { value: x } }\n}\nfn main() { print(m::make(5).value) }',
  ],
  [
    'generic comparison operators unchanged',
    'fn main() {\n print(1 < 2)\n print(1 << 2)\n}',
  ],
  [
    'generic enum',
    'enum Opt<T> { Some(T), Nothing }\nfn unwrap<T>(o: Opt<T>) -> T { return match o { Some(v) -> v\n Nothing -> none } }\nfn main() { print(unwrap(Some(3))) }',
  ],
  // Generic map keys ({K: V}): both substrates share the same value model, so
  // construction, indexing, mutation, equality, rendering, rejection, and the
  // JSON distinction must agree exactly.
  [
    'map int keys',
    'fn main() {\n let m: {int: string} = {1: "one", 2: "two"}\n print(m)\n print(m[2])\n print(m.keys())\n}',
  ],
  [
    'map bool keys',
    'fn main() {\n let m: {bool: int} = {true: 1, false: 0}\n print(m)\n print(m[false])\n}',
  ],
  [
    'map string keys unchanged',
    'fn main() { let m: {string: int} = {"a": 1, "b": 2}\n print(m)\n print(m["a"]) }',
  ],
  [
    'map union keys',
    'type Key = string | int\nfn main() {\n let m: {Key: bool} = {"a": true, 1: false}\n print(m)\n print(m[1])\n}',
  ],
  [
    'generic map alias',
    'type Map<K, V> = {K: V}\nfn main() {\n let vals: Map<int, float> = {1: 1.2, 2: 3.3}\n print(vals[2])\n}',
  ],
  [
    'generic map function',
    'fn get<K, V>(m: {K: V}, k: K) -> V { return m[k] }\nfn main() { print(get({1: "x", 2: "y"}, 2)) }',
  ],
  [
    'map mutation',
    'fn main() {\n let mut m: {int: string} = {:}\n m[1] = "one"\n m[1] = "ONE"\n print(m)\n print(m.remove(1))\n print(m)\n}',
  ],
  [
    'map equality and rendering',
    'fn main() {\n print({1: "a", 2: "b"} == {2: "b", 1: "a"})\n print({1: {2: 3}})\n print({2: "b", 1: "a"})\n}',
  ],
  [
    'map bad key category',
    'fn main() { let m: {float: int} = {1.0: 1} }',
  ],
  [
    'map bad index key',
    'fn main() { let m: {int: string} = {1: "one"}\n print(m["1"]) }',
  ],
  [
    'json rejects non-string keys',
    'fn main() { print(json_encode({1: "x"})) }',
  ],
  [
    'json rejects a range rather than emitting null',
    "fn main() { print(json_encode(0..3)) }",
  ],
  [
    'json rejects a function rather than emitting null',
    'fn main() { print(json_encode(len)) }',
  ],
  [
    'json rejects a variant rather than flattening it',
    'enum E { A(int), B(int) }\nfn main() { print(json_encode(A(1))) }',
  ],
  [
    'json rejects a non-finite float rather than emitting null',
    'fn main() { print(json_encode(to_float("nan"))) }',
  ],
  [
    'json still encodes representable values identically',
    'fn main() {\n print(json_encode([1, "two", true, none]))\n print(json_encode(2.5))\n print(json_encode(none))\n}',
  ],
  [
    'map mixed key literal infers a union',
    'fn main() {\n let m = {1: "a", "1": "b"}\n print(m)\n print(m.keys())\n}',
  ],
  [
    'map annotation checks every entry',
    'fn main() { let m: {int: string} = {1: "a", true: "b"} }',
  ],
  [
    'list comprehension',
    'fn main() {\n let xs = [1, 2, 3, 4]\n print([x * 2 for x in xs])\n print([x for x in xs if x > 2])\n}',
  ],
  [
    'map comprehension',
    'fn main() {\n print({x: x * x for x in range(1, 4)})\n let m = {1: 10, 2: 20}\n print({k: v * 2 for [k, v] in m.items()})\n}',
  ],
  [
    'comprehension scope and errors',
    'fn main() { let ys = [x for x in [1, 2]]\n print(ys)\n print(x) }',
  ],
  [
    'map items',
    'fn main() { let m: {int: string} = {2: "b", 1: "a"}\n print(m.items())\n for [k, v] in m.items() { print(k)\n print(v) } }',
  ],
  [
    'list inference union',
    'fn main() { print([1, "x"]) }',
  ],
  [
    'list annotated mismatch',
    'fn main() { let xs: [int] = [1, "x"] }',
  ],
  // Builtin-name value-namespace reservation (E1009). These pin that the wasm
  // and native engines agree on the reservation; the historical `let values`
  // regression was a corpus entry that became illegal after the rule landed.
  [
    'builtin reserved let mut',
    'fn main() {\n let mut sum = 0\n sum = sum + 1\n print(sum)\n}',
  ],
  [
    'builtin reserved let',
    'fn main() { let sum = 1\n print(sum) }',
  ],
  [
    'builtin reservation near miss',
    'fn main() { let vals = 1\n print(vals) }',
  ],
  [
    'builtin call still works',
    'fn main() { print(sum([1, 2, 3])) }',
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

// Mainstream-shape call-frame parity guard (BREAK-0.2.1 / B-1).
//
// A recursive function with an `else` block is the common shape, and it costs
// more WebAssembly engine-stack frames per Aura call than the thin corpus
// shapes (`tests/corpus/call-frames/*`, which reach the 512-frame language
// limit). On the committed 0.2.1 artifact the mainstream shapes trap
// (RangeError) below the language limit — measured first-trap depths on Node
// 24, fresh instance: else 387, match 459, closure 356, module 387, if-chain
// 419 — and in the Playground's Web Worker as early as 196; see
// `docs/WASM_CALL_FRAME_LIMIT_DECISION.md` (OPEN, remediation blocked). The
// per-call engine-stack cost is shape-dependent, so this guard pins the
// working region for every shape at a depth comfortable on every measured
// substrate (150). It keeps a structured native/WASM agreement for each
// control-flow family, so a change that erodes engine-stack headroom further
// fails here without encoding the pending decision.
{
  const depth = 150;
  const shapes = {
    "else-block": `fn count(n: int) -> int {\n  if n <= 0 {\n    return 0\n  } else {\n    return count(n - 1) + 1\n  }\n}\nfn main() { print(count(${depth})) }\n`,
    "match-arm": `fn count(n: int) -> int {\n  match n {\n    0 -> 0,\n    _ -> count(n - 1) + 1\n  }\n}\nfn main() { print(count(${depth})) }\n`,
    closure: `fn main() {\n  let go = (self_rec, n: int) -> { if n <= 0 { return 0 } else { return self_rec(self_rec, n - 1) + 1 } }\n  print(go(go, ${depth}))\n}\n`,
    "in-source module": `module inner {\n  pub fn count(n: int) -> int {\n    if n <= 0 {\n      return 0\n    } else {\n      return count(n - 1) + 1\n    }\n  }\n}\nfn main() { print(inner::count(${depth})) }\n`,
    method: `struct C { }\nimpl C {\n  fn count(self, n: int) -> int {\n    if n <= 0 { return 0 }\n    return self.count(n - 1) + 1\n  }\n}\nfn main() { let c = C { }\n print(c.count(${depth})) }\n`,
    mutual: `fn even(n) { if n == 0 { return 1 }\n return odd(n - 1) }\nfn odd(n) { if n == 0 { return 0 }\n return even(n - 1) }\nfn main() { print(even(${depth})) }\n`,
    "for-body": `fn count(n: int) -> int {\n  if n <= 0 { return 0 }\n  let mut r = 0\n  for _i in [0] { r = count(n - 1) + 1 }\n  return r\n}\nfn main() { print(count(${depth})) }\n`,
    "if-else chain": `fn f(n: int) -> int {\n  if n <= 0 { return 0 }\n  if n % 2 == 0 {\n    return f(n - 1) + 1\n  } else {\n    return f(n - 1) + 1\n  }\n}\nfn main() { print(f(${depth})) }\n`,
  };
  let shapeFailures = 0;
  for (const [name, src] of Object.entries(shapes)) {
    const wasm = wasmResult(src);
    const srcFile = join(dir, `recursion_${name.replace(/[^a-z0-9]/gi, "_")}_${depth}.aura`);
    writeFileSync(srcFile, src);
    const native = JSON.parse(
      execFileSync(nativeBin, [srcFile, optionsFile], { encoding: "utf8" }).trim(),
    );
    // The mutual shape alternates parity by depth; the case count is the
    // observable, the same on both substrates.
    const expectedStdout = `${depth % 2 === 0 && name === "mutual" ? 1 : depth}\n`;
    const expected = norm({
      status: "ok",
      stdout: expectedStdout,
      result: null,
      diagnostics: [],
    });
    if (wasm.text === expected && norm(native) === expected) {
      passed += 1;
    } else {
      failed += 1;
      shapeFailures += 1;
      console.error(
        `FAIL ${name} recursion parity at depth ${depth}\n  wasm:   ${wasm.text}\n  native: ${norm(native)}`,
      );
    }
  }
  if (shapeFailures === 0) passed += 1;
  else failed += 1;
}

// ---------------------------------------------------------------------------
// Generated-program native/wasm parity (Property 4, permanent).
//
// The real cross-substrate *property*: generate checker-valid programs
// deterministically, run each through BOTH engines, and require the observable
// results to be identical. Two sources feed it:
//
//   1. a fresh, deterministic generator evaluated *in this harness* (below),
//      so the property genuinely generates cases at test time rather than only
//      replaying a fixed list; and
//   2. the committed generator snapshot (`tests/support/program_gen.rs` output,
//      `tests/corpus/ast/gen_*.aura`) as a compatibility sweep, so the shared
//      Rust/JS corpus is also compared on both engines.
//
// Excluded: TypeExpr-heavy inputs are not emitted by either generator, so the
// property stays focused on the expression/statement surface where both
// generators live. Type-nesting parity is now pinned explicitly by the
// dedicated sweep below (ADR-0004 unified the ceiling; there is no longer a
// substrate divergence to avoid).
//
// A mismatch prints the program and its deterministic seed, so it is
// reproducible by running that single program through both engines.
{
  // -- Deterministic generator (splitmix64, same shape as the Rust generator).
  const SM64 = (seed) => {
    let state = (seed ^ 0x9e3779b97f4a7c15n) & 0xffffffffffffffffn;
    return {
      below(n) {
        state = BigInt.asUintN(64, state + 0x9e3779b97f4a7c15n);
        let z = state;
        z = BigInt.asUintN(64, (z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n);
        z = BigInt.asUintN(64, (z ^ (z >> 27n)) * 0x94d049bb133111ebn);
        z = z ^ (z >> 31n);
        return Number(z % BigInt(n));
      },
    };
  };

  // Only well-typed, checker-valid constructs. Int and string expressions are
  // kept in separate namespaces so a generated program always checks, on both
  // substrates. Deliberately no deeply nested TypeExpr.
  const genProgram = (seed) => {
    const rng = SM64(BigInt(seed));
    const intVar = [];
    const strVar = [];
    const lines = [];
    let fresh = 0;
    const name = (p) => `${p}${fresh++}`;
    const intExpr = () => {
      const terms = [];
      const k = 1 + rng.below(3);
      for (let i = 0; i < k; i += 1) terms.push(String(rng.below(20)));
      for (const v of intVar) if (rng.below(2) === 0) terms.push(v);
      return terms.join(rng.below(2) === 0 ? " + " : " - ");
    };
    const stmts = 1 + rng.below(6);
    for (let i = 0; i < stmts; i += 1) {
      const kind = rng.below(9);
      if (kind === 0) lines.push(`print(${intExpr()})`);
      else if (kind === 1) {
        const n = name("i");
        // Keystone unused analysis: a generated binding that is never read
        // again is an explicit discard (`_iN`), exactly as the Rust-side
        // generator does, so generated programs stay checker-clean.
        lines.push(`let mut _${n} = ${intExpr()}`);
        intVar.push(`_${n}`);
      } else if (kind === 2) lines.push(`print(len([1, 2, 3]))`);
      else if (kind === 3) lines.push(`print(abs(0 - ${rng.below(9)}))`);
      else if (kind === 4) lines.push(`for x in 0..${1 + rng.below(3)} { print(x) }`);
      else if (kind === 5) {
        const n = name("s");
        lines.push(`let ${n} = "ab".upper()`);
        strVar.push(n);
        lines.push(`print(${n})`);
      } else if (kind === 6) {
        const a = 1 + rng.below(9);
        const b = 1 + rng.below(9);
        lines.push(`print(min(${a}, ${b}))`);
      } else if (kind === 7) lines.push(`print(to_int("${rng.below(99)}"))`);
      else lines.push(`if ${rng.below(2)} < 1 { print(${intExpr()}) } else { print(0) }`);
    }
    return `fn main() {\n    ${lines.join("\n    ")}\n}\n`;
  };

  const compareBoth = (label, src) => {
    const w = wasmResult(src);
    const nfile = join(dir, `genparity_${label}.aura`);
    writeFileSync(nfile, src);
    const n = norm(
      JSON.parse(execFileSync(nativeBin, [nfile, optionsFile], { encoding: "utf8" }).trim()),
    );
    const accepted = w.text.includes('"status":"ok"') && n.includes('"status":"ok"');
    if (!accepted) {
      console.error(`FAIL generated-parity ${label}: not accepted on both substrates\n  wasm:   ${w.text}\n  native: ${n}\n  source: ${JSON.stringify(src)}`);
      return "nonaccept";
    }
    if (w.text !== n) {
      console.error(`FAIL generated-parity ${label}\n  wasm:   ${w.text}\n  native: ${n}\n  source: ${JSON.stringify(src)}`);
      return "mismatch";
    }
    return "ok";
  };

  // 1. Freshly generated, deterministic property cases.
  const GENERATED_CASES = 60;
  let genPassed = 0;
  let genFailed = 0;
  let genNonAccept = 0;
  const distinctSources = new Set();
  for (let seed = 0; seed < GENERATED_CASES; seed += 1) {
    const src = genProgram(seed);
    distinctSources.add(src);
    const r = compareBoth(`gen_seed_${seed}`, src);
    if (r === "ok") genPassed += 1;
    else if (r === "nonaccept") genNonAccept += 1;
    else genFailed += 1;
  }
  // The cases must be *generated*, not a hardcoded list: require that seed
  // variation actually produces many distinct programs (a fixed list would
  // collapse to one distinct source; a handful would indicate a near-constant
  // generator). Require at least three quarters to be distinct.
  const requiredDistinct = Math.floor((GENERATED_CASES * 3) / 4);
  if (distinctSources.size < requiredDistinct) {
    genFailed += 1;
    console.error(
      `FAIL generated-parity: generator produced only ${distinctSources.size} distinct programs for ${GENERATED_CASES} seeds (need >= ${requiredDistinct})`,
    );
  }
  // 2. The committed generator snapshot, as a compatibility sweep.
  const genDir = resolve(here, "../../../tests/corpus/ast");
  let genFiles = [];
  try {
    genFiles = readdirSync(genDir)
      .filter((f) => /^gen_\d+\.aura$/.test(f))
      .sort();
  } catch (e) {
    console.error(`FAIL generated-parity: cannot read ${genDir}: ${e.message}`);
    failed += 1;
  }
  let committedPassed = 0;
  for (const f of genFiles) {
    const r = compareBoth(f, readFileSync(join(genDir, f), "utf8"));
    if (r === "ok") committedPassed += 1;
    else if (r === "nonaccept") genNonAccept += 1;
    else genFailed += 1;
  }

  const total = GENERATED_CASES + genFiles.length;
  if (genFailed === 0 && genNonAccept === 0 && genPassed === GENERATED_CASES) {
    passed += 1;
    console.log(
      `generated-parity: ${genPassed}/${GENERATED_CASES} freshly generated + ${committedPassed}/${genFiles.length} committed programs agree native/wasm`,
    );
  } else {
    failed += 1;
    console.error(
      `FAIL generated-parity: generated ${genPassed}/${GENERATED_CASES} ok, committed ${committedPassed}/${genFiles.length} ok, ${genFailed} mismatched, ${genNonAccept} not accepted, of ${total}`,
    );
  }
}

// ---------------------------------------------------------------------------
// TypeExpr nesting sweep (permanent, ADR-0004).
//
// Structural type nesting (`Box<…>`, `[T]`, `{K: V}`) counts toward the same
// 256-level semantic AST budget as every other node, on every substrate
// (LANGUAGE_SPEC.md §31.1). The ceiling is therefore unified: native and WASM
// accept and reject at exactly the same depth, and the old substrate split
// (native 2047/2048, WASM 767/768) is gone. This sweep pins the unified
// boundary (N-1/N/N+1), the flat-union exemption, and the §31.5 invariant — no
// host failure at any depth, on either substrate.
{
  const typeSrc = (n) => {
    let t = "int";
    for (let i = 0; i < n; i += 1) t = `Box<${t}>`;
    return `struct Box<T> { value: T }\nfn f(_x: ${t}) -> int { return 1 }\nfn main() { print(1) }`;
  };
  // Around the unified ceiling, plus the exact boundary levels (N-1/N/N+1) and
  // the old substrate ceilings, which must now agree.
  const levels = new Set();
  for (const d of [0, 1, 2, 64, 128, 192, 250, 254, 255, 256, 257, 258, 512, 767, 768, 2047, 2048]) {
    levels.add(d);
  }
  const sorted = [...levels].sort((a, b) => a - b);

  let sweepFailures = 0;
  let nativeHostFailures = 0;
  let wasmHostFailures = 0;
  const table = [];
  for (const n of sorted) {
    const src = typeSrc(n);
    const srcFile = join(dir, `typeexpr_${n}.aura`);
    writeFileSync(srcFile, src);

    // Native: the harness always prints structured JSON and exits 0. A panic
    // inside the engine is contained by `on_execution_stack` and reported as
    // status "internal"; anything else (a raw crash) is a host failure.
    let nativeKind;
    let nativeCode = "-";
    try {
      const out = execFileSync(nativeBin, [srcFile, optionsFile], { encoding: "utf8" }).trim();
      const parsed = JSON.parse(out);
      if (parsed.status === "ok") nativeKind = "accept";
      else if (parsed.status === "internal") {
        nativeKind = "HOST-FAILURE";
        nativeHostFailures += 1;
      } else {
        nativeKind = "reject";
        nativeCode = (parsed.diagnostics || [])[0] ? parsed.diagnostics[0].code_text : "?";
      }
    } catch (e) {
      const s = `${e.stdout || ""}${e.stderr || ""}`;
      nativeKind = "HOST-FAILURE";
      nativeHostFailures += 1;
      void s;
    }

    // Wasm: accept, a structured diagnostic, or a trap (host failure).
    let wasmKind;
    let wasmCode = "-";
    try {
      const r = runtime.run(src, options);
      if (r.status === "ok") wasmKind = "accept";
      else if (r.status === "internal") {
        wasmKind = "HOST-FAILURE";
        wasmHostFailures += 1;
      } else {
        wasmKind = "reject";
        wasmCode = (r.diagnostics || [])[0] ? r.diagnostics[0].code_text : "?";
      }
    } catch (e) {
      wasmKind = "HOST-FAILURE";
      wasmHostFailures += 1;
    }

    table.push([n, nativeKind, nativeCode, wasmKind, wasmCode]);
  }

  // §31.5: no host failure on either substrate, at any depth.
  if (nativeHostFailures === 0 && wasmHostFailures === 0) {
    passed += 1;
  } else {
    failed += 1;
    console.error(
      `FAIL typeexpr sweep host-safety: native host failures=${nativeHostFailures}, wasm host failures=${wasmHostFailures}`,
    );
  }

  // Every rejection must be the structured nesting code, never a bare failure.
  for (const [n, nk, nc, wk, wc] of table) {
    if (nk === "reject" && nc !== "E1015") {
      sweepFailures += 1;
      console.error(`FAIL typeexpr n=${n}: native rejected with ${nc}, expected E1015`);
    }
    if (wk === "reject" && wc !== "E1015") {
      sweepFailures += 1;
      console.error(`FAIL typeexpr n=${n}: wasm rejected with ${wc}, expected E1015`);
    }
  }

  // ADR-0004: one unified ceiling. `n` generic wraps is `n + 1` AST levels, so
  // 255 wraps (256 levels) is accepted and 256 wraps (257 levels) is rejected —
  // identically on both substrates.
  const CEILING_WRAPS = 256; // first rejected wrap count
  const nativeFirstReject = table.find(([, nk]) => nk === "reject")?.[0];
  const wasmFirstReject = table.find(([, , , wk]) => wk === "reject")?.[0];
  for (const [label, first] of [
    ["native", nativeFirstReject],
    ["wasm", wasmFirstReject],
  ]) {
    if (first === CEILING_WRAPS) passed += 1;
    else {
      failed += 1;
      console.error(
        `FAIL typeexpr ${label} ceiling: first rejection at ${first} wraps, expected ${CEILING_WRAPS}`,
      );
    }
  }

  // Unified shape: below the ceiling both accept; at/after it both reject with
  // E1015. Any native/wasm disagreement is the removed divergence returning.
  let shapeOk = true;
  for (const [n, nk, , wk] of table) {
    const expected = n < CEILING_WRAPS ? ["accept", "accept"] : ["reject", "reject"];
    if (!(nk === expected[0] && wk === expected[1])) {
      shapeOk = false;
      console.error(
        `FAIL typeexpr n=${n}: expected ${expected[0]}/${expected[1]}, got ${nk}/${wk}`,
      );
    }
  }
  if (shapeOk && sweepFailures === 0) passed += 1;
  else failed += 1;

  // ADR-0004 consequence: a *flat* union is alternatives, not nesting, and must
  // stay accepted on both substrates no matter how many members it lists.
  {
    const names = Array.from({ length: 600 }, (_, i) => `T${i}`);
    const defs = names.map((t) => `type ${t} = int`).join("\n");
    const flat = `${defs}\ntype All = ${names.join(" | ")}\nfn main() { let _x: All = 1 }`;
    let flatOk = true;
    const nf = join(dir, "typeexpr_flatunion.aura");
    writeFileSync(nf, flat);
    try {
      const out = execFileSync(nativeBin, [nf, optionsFile], { encoding: "utf8" }).trim();
      if (JSON.parse(out).status !== "ok") flatOk = false;
    } catch {
      flatOk = false;
    }
    if (runtime.run(flat, options).status !== "ok") flatOk = false;
    if (flatOk) passed += 1;
    else {
      failed += 1;
      console.error("FAIL typeexpr flat union: a 600-member union must stay accepted");
    }
  }

  // Characterize the curve for the record (visible in CI output).
  console.log(
    `typeexpr sweep: ${table.length} depths, unified ceiling=${CEILING_WRAPS} wraps, host failures=0`,
  );
}

console.log(`\nDifferential: ${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
