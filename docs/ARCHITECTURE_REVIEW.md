# Aura v3 — Architecture & Language Maturity Review

**Baseline reviewed:** commit `657c5c2` (`rewrite/v3-rust`, = `origin/master`)
**Method:** source inspection + adversarial probing; no code changed.
**Reported baseline verified:** 137 tests (all features) / 134 (no `py`);
latest CI run on `657c5c2` is green.

This document is an engineering assessment, not a patch. It records what is
sound, what is fragile, and where the architecture will fight future work.

---

## A. Executive summary

### Genuinely strong

* **The front end is honest.** Lexer → parser → AST → checker → evaluator is
  a clean, readable pipeline with no hidden global state. The AST carries
  spans on every node; diagnostics render `file:line:col`.
* **Mutation is explicit and enforced.** `let` vs `let mut` is checked
  statically; reassignment of an immutable binding is `E2001`; top-level
  `let mut` is rejected.
* **Resource limits are real and tested.** Expression nesting (`E1015`) and
  call frames (`E4011`) are language-level and enforced at parse time, which
  closes the P0 host-crash class. No `unsafe`, no `unwrap`/`expect`/`panic`
  in production paths.
* **The feature boundary works.** `--no-default-features` produces a binary
  with no CPython in the dependency graph; `py_*` becomes a deliberate
  `E5002`.
* **Diagnostics have stable codes and are documented**, and a test asserts
  every documented code is reachable.

### Fragile

* **There is a live P1 correctness bug:** `float + float` (and any addition
  involving a float) returns `E4999 internal error`. `numeric()` handles
  `Sub`/`Mul`/`Div`/`Rem`/`Pow` but not `Add`, which only ever reached
  `numeric` for the `Int`/`Str`/`List` cases in `binary()`. See F-01.
* **The checker and evaluator independently implement semantics** that are
  not derived from one representation: type inference, callability, method
  existence, ordering, equality. They can — and do — disagree (F-02, F-05).
* **Equality is not reflexive for functions/closures** (`f == f` is
  `false`). See F-06.
* **The Python bridge silently loses data** (big ints → float, non-string
  dict keys collide). See F-10.
* **`pub` and `use` are accepted anywhere but only `pub fn` is even parsed
  into the AST; the rest is silently dropped.** See F-08.

### Acceptable for alpha

* Four near-duplicate execution entry points (`run_source`,
  `run_toplevel_stdout`, `run_program`, plus CLI/REPL wiring) — acceptable
  now, but it is the seed of divergence (F-14).
* `try/finally` without `catch` is impossible; `finally` precedence over
  `return`/`break`/`throw` is implemented but undocumented (F-07).
* `match` arms cannot be bare control-flow statements (F-12).
* `(a, b)` is a list; there is no distinct tuple (F-11).

---

## B. Architecture diagram

```
source (UTF-8 str)
    │  lex(src) -> Result<Vec<Token>>                    [src/lex]
    ▼
Token { tok: Tok, span: Span }
    │  parse(src) -> Result<Module>                      [src/parse]
    │    • builds AST (Box/Vec/Rc-free; owned data)
    │    • enforce_depth(): iterative, rejects >256 nesting (E1015)
    ▼
Module { items: Vec<Item> }                              [src/ast]
    │  Checker::module(&Module) / module_with_main        [src/check]
    │    • hoist: types, functions, const order
    │    • scope/name/mutability/type/pattern/loop checks
    │    • E1015 descent guard
    ▼
Result<()>  (first Diag wins)
    │  run::Interp::run(&Module)                          [src/run]
    │    pass 1: declare fns/structs/enums
    │    pass 2: eval consts/exprs in source order
    │    pass 3: call main
    ▼
Value { Int,Float,Str,Bool,None,List,Map,Instance,Variant,Closure,Native,Range }
    │  control flow: Ctl { Val, Return, Break, Continue, Throw }
    ▼
stdlib (natives + method tables)   Python bridge (feature `py`)
    ▼
CLI (run/check/eval/repl)          REPL (statement-first, persistent globals)
```

**Invariants assumed across boundaries**

| Boundary | Data | Owner | Assumed invariant | Validated where |
|---|---|---|---|---|
| lex→parse | `Vec<Token>` | parser | last token is `Eof`; spans valid | lexer |
| parse→check | `Module` | caller | no nesting > 256 | `enforce_depth` |
| check→run | `Module` | caller | names/types valid | checker only |
| run→stdlib | `Vec<Value>` | interp | arity/types correct | **runtime only** |
| AST→run | `Module` | caller | `Item::Use`/`alias` inert | nowhere |

The critical observation: **the checker validates a strict subset of what
the runtime enforces.** Everything the runtime checks that the checker does
not is a potential "checker accepts, runtime rejects" divergence.

---

## C. Findings

Severity uses the requested scale. "Now?" states whether it blocks new
language work.

### F-01 — `float + float` returns an internal error — **P1** — required now

* **Area:** evaluator, arithmetic.
* **Evidence:** `numeric()` (`src/run/mod.rs:1198`) matches
  `Sub|Mul|Div|Rem|Pow` on the float path; `Add` is absent and falls into
  the `INTERNAL` arm at line ~1259. `binary()` handles `Add` only for
  `Int`/`Str`/`List` (`src/run/mod.rs:1178`) and delegates every other pair
  to `numeric()`.
* **Reproduction:** `print(1.5 + 2.5)` → `E4999 internal error:
  non-arithmetic operator reached the numeric path`. Also `1 + 2.5`.
* **Why it matters:** addition of floats is fundamental; returning an
  internal error is a correctness and trust failure. It also shows the
  hardening pass replaced `unreachable!()` with an error *without* covering
  the arm it protected.
* **Direction:** add `Add` to the float match; ideally make `numeric()`
  total over `BinOp` so the compiler forces every operator to be handled.
* **Now?** Yes — this is a shipped correctness bug.

### F-02 — Methods are never statically validated — **P2** — required before "richer stdlib"

* **Area:** checker/runtime asymmetry.
* **Evidence:** `Expr::Method` in the checker (`src/check/mod.rs:866`) only
  checks the receiver and arguments; it does not consult any method table.
  `stdlib::method` (`src/stdlib/mod.rs:473`) dispatches at runtime and
  returns `E2003` for an unknown method.
* **Reproduction:** `aura check` accepts `[1].nope()`, `"s".nope()`,
  `{"a":1}.nope()`; `aura run` rejects with `E2003`.
* **Why it matters:** every method-name typo passes the checker. As the
  stdlib grows this is a large class of "check ok, run fails".
* **Direction:** a per-receiver-type method table shared between the
  checker and the runtime (one registry, two consumers).
* **Now?** Yes, before adding methods.

### F-03 — Builtin arity/type rules exist only at runtime — **P2** — required

* **Area:** checker/runtime asymmetry.
* **Evidence:** `builtin_names()` tells the checker a name exists, but the
  checker has no signatures. Arity/type errors surface only when the native
  runs (`arity()` in `src/stdlib/mod.rs`).
* **Reproduction:** `aura check` accepts `len([1], [2])`; `aura run` fails
  `E3001`.
* **Why it matters:** the "rejects at check time" table in the contract
  lists `E3001` as a static rule, but for builtins it is runtime-only.
* **Direction:** a signature table keyed by builtin name; the checker
  consults it, the runtime enforces it.
* **Now?** Yes, together with F-02.

### F-04 — Function return types are not inferable, so provable mismatches pass — **P2** — soon

* **Area:** checker soundness.
* **Evidence:** `infer()` returns `Ty::Unknown` for `Expr::Call`
  (`src/check/mod.rs:540`). So `let x: int = f()` where `f() -> string` is
  accepted.
* **Reproduction:** `fn f() -> string { return "s" }\nfn main() { let x:
  int = f() }` → `aura check` passes (runtime error `E3001` only when it
  reaches `x`).
* **Why it matters:** the contract promises annotations are checked; a call
  whose declared return type is known should be checkable.
* **Direction:** record function signatures in the checker (already
  partially available via `functions`) and return the declared `Ty` from
  `infer(Call)`. This is a bounded, sound addition.
* **Now?** Soon. It is the highest-value type-system improvement and does
  not require a new type system.

### F-05 — Ordering is implemented twice (checker inference vs runtime) — **P2** — soon

* **Area:** duplicated semantics.
* **Evidence:** the checker infers comparisons as `Bool` for any operands
  (`src/check/mod.rs:519`); the runtime decides comparability in
  `cmp_val`/`comparable_with` (`src/run/value.rs`). Lists, `none`, and
  structs are unordered at runtime but the checker treats any comparison as
  well-typed.
* **Reproduction:** `[1] < [2]` passes check, fails `E3001` at runtime.
* **Why it matters:** same rule (which types are orderable) lives in two
  places.
* **Direction:** expose the "orderable" predicate from one place and have
  the checker use it when it can infer both operand types.
* **Now?** Soon.

### F-06 — Equality is not reflexive for functions — **P2** — soon

* **Area:** value model.
* **Evidence:** `Value::equals` (`src/run/value.rs`) has no `Closure`/
  `Native` arm; the fallthrough returns `false`. `g == g` and `f == f` are
  `false`.
* **Reproduction:** `fn g() { return 1 }\nfn main() { print(g == g) }`
  → `false`.
* **Why it matters:** `x == x` is false for a value class; any user code
  relying on the reflexive property is subtly wrong. Also `f != f` is true.
* **Direction:** choose and document: either reference identity for
  closures or an explicit `E3001` "functions are not comparable". Silence
  (false) is the one option that should not stand.
* **Now?** Soon (small, high clarity).

### F-07 — `finally` precedence over other control flow is unspecified — **P2** — soon

* **Area:** control-flow semantics.
* **Evidence:** `Stmt::Try` in the evaluator runs `finally` and, if it
  produces a non-`Val` outcome, **replaces** the pending result
  (`src/run/mod.rs`). Thus `try { return 1 } catch e -> {} finally { return 2 }`
  → 2, and `try { throw "a" } ... finally { throw "b" }` → "b".
* **Reproduction:** cf1/cf9 in this review; verified.
* **Why it matters:** the behavior is defensible but undocumented; the
  contract only says `finally` runs "exactly once".
* **Direction:** document the rule ("a control-flow signal raised in
  `finally` overrides the pending outcome"). No code change needed.
* **Now?** Soon (documentation).

### F-08 — `pub` is inconsistently handled — **P3** — cheap

* **Area:** parser/AST.
* **Evidence:** `item()` reads `pub` (`src/parse/mod.rs:312`) but forwards it
  only to `fn_item`; `pub struct`/`pub enum`/`pub type`/`pub use`/`pub let`
  silently drop it. `Item::Fn.public` is never read by checker or runtime.
* **Reproduction:** `pub struct S { x: int }` parses and runs, `pub` ignored.
* **Why it matters:** accepted syntax with discarded semantics is an
  abstraction leak; it is documented as "inert", but the inconsistency
  (some items carry the flag, some don't) is not.
* **Direction:** either reject `pub` on non-`fn` items, or record it on all
  items uniformly. Documented as inert, so this is low priority.
* **Now?** Cheap; do with F-13.

### F-09 — `Expr::Tuple` is semantically a list — **P3** — defer

* **Area:** AST/runtime.
* **Evidence:** the parser builds `Expr::Tuple` for `(a, b)`
  (`src/parse/mod.rs:1022`); the evaluator lowers it to `Value::list`
  (`src/run/mod.rs:940`). Equality/display make tuples indistinguishable
  from lists.
* **Why it matters:** a parser-only distinction leaks into the AST with no
  runtime meaning. Documented in the grammar as list sugar, so it is
  intentional, but the AST node could be removed to reduce surface.
* **Direction:** document as intentional (already) and consider lowering at
  parse time to `Expr::List`.
* **Now?** Defer.

### F-10 — Python bridge silently loses data — **P2** — soon

* **Area:** Python boundary.
* **Evidence:** `to_value` tries `i64` then `f64` then `String` for dict
  keys via `k.str()`. A Python integer beyond `i64` becomes a lossy `f64`
  (`2**63` → `9223372036854776000`); a dict with keys `1` and `"1"` collapses
  to one entry (`{"1": 2}`).
* **Reproduction:** verified with `py_eval("2**63")` and
  `py_eval("{1: 1, \"1\": 2}")`.
* **Why it matters:** the boundary is advertised as structural conversion;
  silent collision/loss is a correctness hazard.
* **Direction:** reject unconvertible big ints explicitly, and reject
  non-string dict keys rather than stringifying them.
* **Now?** Soon (only affects the `py` feature).

### F-11 — No `finally` without `catch` — **P3** — defer

* **Area:** grammar/parser.
* **Evidence:** `try_stmt = "try" block "catch" IDENT "->" block [ "finally"
  block ]`; the parser requires `catch`.
* **Why it matters:** a common construct (`try { } finally { }` for cleanup
  without a catch) cannot be written. It is a deliberate grammar choice, but
  should be recorded as such.
* **Now?** Defer (language-design decision).

### F-12 — `match` arms cannot be bare control-flow statements — **P3** — defer

* **Area:** parser.
* **Evidence:** an arm body is `block | expr terminator`; `1 -> break` is an
  expression parse error. `1 -> { break }` works.
* **Now?** Defer.

### F-13 — `use` is parsed but has no semantics — **P3** — intentional

* **Area:** language surface.
* **Evidence:** `Item::Use` is ignored by checker and runtime; the contract
  documents this.
* **Now?** Intentional; keep documented.

### F-14 — Four execution entry points duplicate wiring — **P2** — soon

* **Area:** architecture.
* **Evidence:** `run_source`, `run_toplevel_stdout`, `run_program` in
  `src/lib.rs`, plus `main.rs` and `repl.rs` each assembling parse+check+run.
  `aura check` uses `Checker::module` while `aura run` uses
  `module_with_main`.
* **Why it matters:** the semantics of "what is a valid program" now live in
  five places. A future change (e.g. a new precondition) must be applied in
  all of them or they silently diverge.
* **Direction:** one `compile()` that returns a checked `Module`, and one
  `execute(mode, module)`; CLI/REPL are thin adapters over it.
* **Now?** Soon — this is the highest-leverage refactor and is a
  prerequisite for cleanly adding modules/imports.

### F-15 — No direct checker/evaluator differential test — **P2** — soon

* **Area:** test architecture.
* **Evidence:** `tests/property.rs` runs `run_source` (which checks) and
  separately calls `Checker::module` twice for determinism, but never
  asserts the cross-layer invariant. A 20k-case random differential probe
  found no contradictions, but random text rarely produces well-formed
  programs, so its signal is weak.
* **Why it matters:** F-01 is exactly the bug class this would catch; it was
  found by hand, not by the suite.
* **Direction:** a *grammar-directed* generator that produces well-typed
  programs and asserts `checker_rejects ⇒ run_rejects_with_same_code` and
  `checker_accepts ⇒ run never fails with E1xxx/E4999`.
* **Now?** Soon.

### F-16 — `Range` equality/ordering is unspecified in the contract — **P3** — cheap

* **Area:** value model / docs.
* **Evidence:** `Range` has an equality arm (`start`/`end`) and display
  `a..b`; ordering falls through to "incomparable".
* **Now?** Cheap documentation.

---

## D. Semantic consistency matrix

Legend: ✅ agree · ⚠️ partial/undocumented · ❌ disagree

| Rule | Contract | Grammar | Parser | Checker | Runtime | Test | Status |
|---|---|---|---|---|---|---|---|
| `let` immutable / `let mut` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| top-level `let mut` rejected | ✅ | ✅ | ✅ | — | — | ✅ | ✅ |
| assignment type respects annotation | ✅ | — | — | ✅ | ✅ | ✅ | ✅ |
| immutable assignment `E2001` | ✅ | — | — | ✅ | ✅ | ✅ | ✅ |
| integer overflow `E4013` | ✅ | — | ✅ | — | ✅ | ✅ | ✅ |
| division by zero `E4007` | ✅ | — | — | — | ✅ | ✅ | ✅ |
| **float addition** | (implied) | — | ✅ | ⚠️ | ❌ E4999 | ❌ | ❌ **F-01** |
| NaN ordering → false | ✅ | — | — | — | ✅ | ✅ | ✅ |
| equality structural | ⚠️ (lists/maps/structs/enums) | — | — | — | ✅ | ✅ | ⚠️ funcs false (F-06) |
| ordering domain | ⚠️ | — | — | ❌ infers Bool | ✅ | ✅ | ❌ F-05 |
| pipeline first-arg | ✅ | ❌ grammar not updated | ✅ | — | ✅ | ✅ | ⚠️ grammar stale |
| `if` without `else` → none | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `try` catches only `throw` | ✅ | — | — | — | ✅ | ✅ | ✅ |
| `finally` override precedence | ❌ missing | — | — | — | ✅ | ✅ | ⚠️ F-07 |
| `try` requires `catch` | ⚠️ (shown in §5) | ✅ | ✅ | ✅ | ✅ | — | ⚠️ F-11 |
| method exists | — | — | ✅ | ❌ none | ✅ | — | ❌ F-02 |
| builtin arity/type | ✅ static | — | — | ❌ none | ✅ | ✅ | ❌ F-03 |
| call return type | ✅ checked | — | — | ❌ Unknown | ✅ | ⚠️ | ❌ F-04 |
| enum variant tag unique | ✅ | — | ✅ | ✅ | ✅ | ✅ | ✅ |
| `pub` semantics | ✅ inert | ⚠️ | ⚠️ partial | — | ✅ | — | ⚠️ F-08 |
| `use` semantics | ✅ inert | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| recursion limit 512 | ✅ | — | — | — | ✅ | ✅ | ✅ |
| nesting limit 256 | ✅ | — | ✅ | ✅ | ✅ | ✅ | ✅ |
| magic: no aliases | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

The grammar is **stale** on the pipeline: the parser was changed to pass the
left operand as the first argument, but `docs/grammar.md` still says
`pipe = logic_or { "|>" logic_or }`, which describes `f(args)` as a single
operand. (F-17 — P3 documentation drift.)

---

## E. Technical debt map

**Must fix before features**
1. F-01 float addition (correctness).
2. F-02/F-03 method + builtin tables shared by checker and runtime.
3. F-14 single compile/execute pipeline.

**Should fix soon**
4. F-04 infer function return types.
5. F-05 single "orderable" predicate.
6. F-06 define function equality/inequality.
7. F-07 document `finally` precedence.
8. F-10 reject lossy Python conversions.
9. F-15 grammar-directed differential property test.

**Safe to defer**
10. F-09 lower `Tuple` to `List` at parse time.
11. F-11 `try` without `catch` (design decision).
12. F-12 control-flow match arms (parser convenience).

**Intentional**
13. F-13 `use` inert.
14. `pub` inert (but see F-08 for consistency).
15. Globally unique enum tags.
16. 256 expression nesting; 512 call frames.
17. Maps string-keyed.

---

## F. Future evolution matrix

Readiness: **GREEN** natural · **YELLOW** targeted refactor · **RED** fights it.

| Category | Readiness | Why |
|---|---|---|
| Modules/imports | **YELLOW** | `use` is inert; `resolve`/module loader absent, but the `Item::Use` slot and a single-namespace model exist. Needs F-14 first. |
| Visibility (`pub`) | **YELLOW** | Parsed; needs a symbol table with visibility, and to fix F-08. |
| Richer type system | **YELLOW** | `Ty` exists and is advisory; function signatures (F-04) and orderability (F-05) are the prerequisites. A real inference engine is a new component, not a rewrite. |
| Generic collections | **RED** | `Ty::List/Map` are monomorphic; there are no type parameters anywhere. Adding `[T]` payload typing is a type-system project. |
| Interfaces/traits | **RED** | No method tables, no nominal method sets; the value model has no dispatch beyond stdlib receiver-type match. |
| Async/concurrency | **RED** | Single-threaded tree-walker with `Rc`/`RefCell`; no scheduler. Not a small change. |
| Bytecode VM | **RED** | Tree-walking by design; `Ctl` propagation assumes recursive evaluation. Feasible only as a separate backend. |
| Optimizer | **RED** | No IR. Depends on a lowering boundary (Phase 3) that does not exist. |
| Formatter | **YELLOW** | AST preserves enough; spans are present but comments are dropped by the lexer, so lossless formatting needs lexer/token retention. |
| LSP / tooling | **YELLOW** | Spans and codes exist; needs an incremental/error-recovering parse and a symbol API. |
| Package manager | **GREEN** | Independent of the language core; filesystem tooling. |
| Native compilation | **RED** | No IR/backend; explicitly out of scope. |

The single biggest architectural lock-in is the **absence of a lowering
boundary** between AST and execution. Every "add an IR/optimizer/VM/formatter"
path runs through it. That is not a reason to build it now, but it should be
a deliberate future milestone, not an accident.

---

## G. Recommended next phase (strictly ordered)

### 1. Mandatory architectural corrections (before any feature)
1. **Fix F-01** (float addition) and add a table-driven arithmetic test that
   exercises every `(operator, type-pair)` combination, so a missing arm
   cannot recur.
2. **Fix F-02 + F-03**: introduce one `BuiltinSig` / `MethodSig` registry
   consumed by both the checker and the runtime. Delete the duplicated
   knowledge.
3. **Fix F-14**: collapse the four execution paths into
   `compile(&str) -> Result<Module>` and `execute(Module, Mode)`. CLI and
   REPL become adapters. Establish a single definition of "valid program".

### 2. Semantic completion (optional refactors, ordered)
4. **F-04** function return type inference.
5. **F-05** one orderability predicate.
6. **F-15** grammar-directed checker/evaluator differential property test.
7. **F-06** decide and document function equality.
8. **F-07 / F-17** documentation: `finally` precedence; refresh
   `docs/grammar.md` for pipeline and `if`/`else`.

### 3. Language-design decisions to freeze now
9. `try` requires `catch` (F-11) — freeze or extend deliberately.
10. `((a,b))` is a list (F-09) — freeze and simplify the AST.
11. Function equality (F-06) — freeze.
12. `pub`/`use` inert (F-08/F-13) — freeze until a module system.
13. Globally unique enum tags — freeze.

### 4. Features that can safely follow
Only after §1–§2: modules/imports (with F-14 in place), then a principled
option type, then richer stdlib. Generics, traits, async, and any VM remain
**NO-GO** until a lowering boundary is designed.

---

## H. Explicit NO-GO list

Do **not** start these now; the architecture is not ready:

* **Generics / type parameters** — no signature representation; would build
  on the monomorphic `Ty`.
* **Traits / interfaces** — no dispatch table and no checker method model
  (F-02).
* **Async / concurrency** — the runtime is single-threaded with `Rc`; a
  concurrency model is a runtime rewrite.
* **Bytecode VM / optimizer / native codegen** — no IR; premature until the
  AST→execution boundary is defined.
* **A module/package *language* system** — wait for F-14; otherwise the
  import rules multiply across the five existing entry points.
* **Any feature that assumes the checker validates calls** — it does not
  (F-02/F-03/F-04). Fix those first.
* **Removing the 256/512 limits** — they are load-bearing for host safety
  until evaluation is iterative. Do not touch them without an iterative
  evaluator.

---

## Appendix — reproduction commands

```bash
# F-01 (P1): float addition returns an internal error
printf 'fn main() { print(1.5 + 2.5) }\n' > /tmp/f.aura
./target/debug/aura run /tmp/f.aura        # E4999 internal error

# F-02: method name not checked
printf 'fn main() { print([1].nope()) }\n' > /tmp/m.aura
./target/debug/aura check /tmp/m.aura      # exit 0
./target/debug/aura run   /tmp/m.aura      # E2003

# F-03: builtin arity not checked
printf 'fn main() { print(len([1], [2])) }\n' > /tmp/a.aura
./target/debug/aura check /tmp/a.aura      # exit 0

# F-04: return type not inferred
printf 'fn f() -> string { return "s" }\nfn main() { let x: int = f() }\n' > /tmp/r.aura
./target/debug/aura check /tmp/r.aura      # exit 0

# F-06: equality not reflexive for functions
printf 'fn g() { return 1 }\nfn main() { print(g == g) }\n' > /tmp/e.aura
./target/debug/aura run /tmp/e.aura        # false

# F-10 (feature py): lossy/overlapping Python conversion
printf 'fn main() { print(py_eval("2**63"))\n print(py_eval("{1: 1, \\"1\\": 2}")) }\n' > /tmp/p.aura
cargo run --quiet --features py -- run /tmp/p.aura
```
