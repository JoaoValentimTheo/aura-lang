# Aura — Semantic Closure Report

This report closes the findings B1–B6 from
[`SEMANTIC_FREEZE_AUDIT.md`](SEMANTIC_FREEZE_AUDIT.md). Every finding was
independently reproduced on the stated baseline before any code changed; every
correction has a permanent regression test.

No language feature was added. No resource limit was removed. The parser,
checker, runtime, and entry-point architecture are preserved.

---

## Executive Summary

All six findings were reproduced. Five were defects; one (B6) was a
grammar/AST/runtime misalignment resolved by choosing the rule the language
already implied (variants are positional).

| Finding | Reproduced | Root cause | Status |
|---|---|---|---|
| B1 float `%` by zero | yes → `nan` | float `Rem` arm omitted the zero check | **FIXED + DOCUMENTED** |
| B2 nesting depth | yes → E1006 at 62 parens; flat chains capped at ~128 | two independent frame counters; `postfix` charged the budget for non-wrapping operands; parser guard preempted the semantic limit and used the wrong code | **FIXED + DOCUMENTED** |
| B3 struct field types | yes → accepted `S { a: "x" }` | `Expr::Construct` checked only the type name | **FIXED + DOCUMENTED** |
| B4 extra struct fields | yes → silently dropped | checker validated nothing; runtime ignored unknown names | **FIXED + DOCUMENTED** |
| B5 REPL persistence | yes → E3002/E2003 for struct/enum/type | session `globals` carried only bindings and functions | **FIXED + DOCUMENTED** |
| B6 named variant args | yes → runtime E3001 | shared `ctor_arg` grammar; runtime ignored names | **FIXED** (variants are positional) |

One pre-existing defect was exposed while closing B5 and fixed because it
blocks the B5 session contract: `type Name = T` aliases were parsed and
validated but **never resolved**, so `let x: Id = 5` was rejected even in a
file. This is recorded below as a discovered bug (not one of B1–B6).

The full suite, clippy (three feature sets), fmt, no-Python tests, release
no-Python smoke, extended property tests, and Miri all pass. No user program
can reach `E4999` through the operator/construction surfaces (verified by
sweeps).

---

## Baseline Commit

```
HEAD  = d940fbed54d8afee020e6e7949fe6b2266847422
branch = rewrite/v3-rust
status = clean except the untracked audit report from the previous phase
```

All work below is relative to this commit.

---

## Findings Reproduced

Reproduced before any modification (see each section's *Reproduction*):

* B1: `1.0 % 0.0`, `-1.0 % 0.0`, `1.0 % -0.0`, `-1.0 % -0.0`, `0.0 % 0.0`
  all printed `nan`; `1 % 0`, `1.0 / 0.0`, `1 / 0` printed `E4007`.
* B2: 61 parentheses parsed, 62 failed with `E1006: nesting too deep`; flat
  `+` chains parsed to 128 terms and failed at 129; nested lists failed at 254.
* B3: `S { a: "x" }`, `S { a: [1] }`, `S { a: 1.5 }` all ran.
* B4: `S { a: 1, b: 2 }` ran and printed `S { a: 1 }`; duplicate `a` likewise.
* B5: `struct P { x: int }` then `P { x: 1 }` → `E3002`; `enum E { A }` then
  `A` → `E2003`; `type Id = int` then `let x: Id = 5` → `E3002`.
* B6: `A(x: 1)` parsed, the checker accepted it, and the runtime reported a
  generic arity error.

---

## B1 — Float Remainder by Zero

### Reproduction

```
1.0 % 0.0   -> nan      (contract §7 says E4007)
-1.0 % 0.0  -> nan
1.0 % -0.0  -> nan
0.0 % 0.0   -> nan
1 % 0       -> E4007
1.0 / 0.0   -> E4007
```

### Root cause

`Interp::numeric` (`src/run/mod.rs`) guarded `Div` against a zero divisor but
`Rem` used `a % b` directly. IEEE `.rem` yields `NaN` when `b == 0.0`, so the
zero case was never diagnosed. The `int` path already checked `b == 0` and
raised `E4007`; only the float path was inconsistent.

### Fix

The float `Rem` arm now checks `b == 0.0` (true for `-0.0`) and returns
`E4007`, exactly like `Div` and the int path. One expression, no new code path.

### Regression tests

`tests/regressions.rs::b1_float_remainder_by_zero_is_an_error` covers every
zero spelling for both operand types, both `/` and `%`, and asserts the
non-zero remainder path (including signs) is unchanged:

```
7 % 3 -> 1,  7.5 % 2.0 -> 1.5,  -7.5 % 2.0 -> -1.5,  7.5 % -2.0 -> 1.5
```

### Contract impact

Contract §7 now reads: "Division by zero is `E4007`: this covers `int` and
`float` for both `/` and `%`, and treats `0.0` and `-0.0` alike." The prior
text was already close but is now precise and true.

---

## B2 — Nesting Depth

### Reproduction

```
61  "(" -> 1
62  "(" -> E1006: nesting too deep
128 "+"-terms -> 128
129 "+"-terms -> E1015: expression nests too deeply
253 "[" nesting -> 1
254 "[" nesting -> E1015
```

Three different thresholds, two different codes, none matching the documented
"256 levels, E1015".

### Actual Depth Accounting

Two mechanisms were overlapping unintentionally:

1. **`enforce_depth`** (`src/parse/mod.rs`), `MAX_AST_DEPTH = 256`. Iterative,
   post-parse, counts **AST nodes** from each top-level expression root. This is
   the semantic limit and the one the contract describes. It emits `E1015`. It
   is unaffected by grouping parentheses (they add no AST node).

2. **`Parser::enter`**, previously `MAX_DEPTH = 128`. A recursive-descent frame
   guard used by `block`, `stmt`, `expr`, and `atom`. Each parenthesized level
   costs two frames (`atom` then `expr`), so it fired at ~62 parens with
   `E1006`. It preempted the semantic limit and used the wrong code.

A third bug compounded it: **`Parser::postfix` incremented `expr_nodes` at the
top of its loop before checking whether the token actually started a postfix
operator.** A bare operand therefore consumed a unit of the flat-expression
budget on every iteration. A chain of `N` terms cost `N` (operands) + `N-1`
(operators) = `2N-1`, so the guard `expr_nodes > 256` fired at 129 terms — half
the intended flat-expression budget.

### Root cause

* The parser's frame guard conflated "host recursion" with "language nesting",
  preempted the semantic check, and reported `E1006`.
* `postfix` charged the node budget for non-wrapping iterations.

### Final Rule

> **Expressions and statements may nest at most 256 AST nodes deep. Deeper
> nesting is `E1015`.** The count is on AST nodes (calls, operators,
> collections, blocks, …), starting at 1 for each top-level expression. Grouping
> parentheses add no depth. An extreme chain of grouping parentheses is bounded
> by the parser's own recursion backstop, which also reports `E1015`.

Implementation:

* `postfix` now increments the node budget only when it actually wraps the
  expression (`count_node`), so a flat chain budgets one unit per operator.
* `MAX_PARSE_DEPTH = MAX_AST_DEPTH * 8 = 2048` frames, reported as `E1015`,
  documented as a host-safety backstop that a well-formed program within the
  semantic limit never reaches.
* `parse`, `parse_expr`, and `parse_stmt` now run the recursive parser on a
  dedicated 64 MiB stack (`on_parse_thread`), matching how execution is already
  protected. This is what makes the semantic limit authoritative instead of the
  caller's 8 MiB stack.

### Security Implications

* No limit was removed; a bounded large stack replaced an unbounded reliance on
  the caller's stack.
* The parser frame backstop still caps raw recursion, so pathological grouping
  cannot exhaust the 64 MiB stack.
* Sweeps over parenthesized, list, map, call, block, `if`, lambda, and `match`
  nesting at 200–5000 levels all end in `E1015` (or a parse error for malformed
  input), never a crash (see *Tests Executed*).

### Regression Tests

`tests/boundaries.rs`:

* `regression_nesting_limit_boundary` — 128-term chain accepted, 256-term
  rejected with `E1015`; 200 nested lists accepted, 300 rejected.
* `regression_parenthesis_nesting_is_bounded_not_a_crash` — 1000 grouped
  parentheses accepted, 4000 rejected with `E1015`.

`tests/parser.rs::deep_nesting_is_bounded` was updated: a 500-paren chain now
parses (grouping is not nesting) and a 5000-paren chain is rejected.

### Documentation

Contract §7 rewrites the nesting bullet to state the AST-node rule, the
grouping exception, and the single `E1015` diagnostic for both the semantic
limit and the parser backstop.

---

## B3 — Struct Field Type Validation

### Reproduction

```
struct S { a: int }
S { a: "x" }   -> ran, printed S { a: "x" }
S { a: [1] }   -> ran
S { a: 1.5 }   -> ran
```

### Root cause

`Expr::Construct` in the checker verified only that the constructed name was a
declared struct or variant. Field declarations were stored as
`struct_fields: HashMap<String, HashMap<String, Ty>>`, but construction never
consulted them. The runtime `construct` also did not compare field values to
types, because it has no type table.

### Fix

`Checker::check_struct_construction` now validates a struct literal:

* named form: every supplied name is a declared field, every declared field is
  supplied exactly once, and each value is compatible with its field type;
* positional form: exactly one value per declared field, in declaration order,
  each value compatible with its field type;
* mixed named/positional is rejected.

Only provable mismatches are reported: `Ty::compatible_with` treats `Unknown`
as compatible with everything, so an untyped value is accepted. The ordered
field list (`struct_field_order`) and per-variant payload types
(`variant_payloads`) were added to the checker to support positional checking.

### Unknown-Type Behavior

Preserved exactly. `let x = none; S { a: x }` is accepted (the value is
`Unknown`), and `A(z)` with `z = none` is accepted. This is by design: the
checker never promises more than it can prove.

### Regression Tests

`tests/regressions.rs::b3_struct_field_type_validation` — wrong `string`,
`list`, and `float` values are `E3001`; correct `int`, `float`, and `bool`
values are accepted; an `Unknown` value is accepted. `tests/regressions.rs::
b4_struct_unknown_and_missing_fields` also checks nested struct field types.

---

## B4 — Unknown/Extra Struct Fields

### Reproduction

```
struct S { a: int }
S { a: 1, b: 2 }   -> ran, printed S { a: 1 }   (b silently dropped)
S { a: 1, a: 2 }   -> ran, printed S { a: 1 }   (duplicate silently dropped)
S { b: 1 }         -> runtime E3001: missing field `a` for `S`
S { a: 1 }         -> runtime E3001: missing field `b` for `S`
```

### Root cause

The checker validated nothing. The runtime's named path iterated the *declared*
fields and looked up each by name, so unknown or duplicate supplied names were
never examined; a missing declared field surfaced as a late runtime error.

### Fix

Checker: unknown field → `E2003`; missing, duplicate, extra (positional), or
wrong-typed field → `E3001`; all emitted before execution. Runtime `construct`
was hardened in parallel so the semantics hold even if the checker cannot prove
a case: unknown names, duplicates, and mixed forms are rejected there too.

### Diagnostics

No new codes. The existing `TYPE_MISMATCH` (`E3001`) and `UNDEFINED` (`E2003`)
are reused, matching the runtime's field-access diagnostics (`field_get` /
`field_set` already used `E2003` for an unknown field). Every diagnostic
carries the construction span and names the offending field.

### Regression Tests

`tests/regressions.rs::b4_struct_unknown_and_missing_fields` — extra/unknown
(`E2003`), missing (`E3001`), duplicate (`E3001`), positional arity
(`E3001`), and a no-field struct (`S { }`) that is built and displayed. The
multi-error case `S { a: 1, b: 2, c: 3 }` deterministically reports the first
unknown field (`b`), matching the "first diagnostic" policy.

---

## B5 — REPL Persistent Declarations

### Reproduction

```
> struct P { x: int }
> P { x: 1 }
E3002: `P` is not a declared struct or enum variant
> enum E { A }
> A
E2003: undefined variable `A`
> type Id = int
> let x: Id = 5
E3002: unknown type `Id`
```

Functions, constants, and `let` bindings persisted; structs, enums, and type
aliases did not.

### State Model

The REPL holds:

* a persistent `Interp` (retains functions, structs, enums, and globals), and
* a `globals` list replayed into a fresh `Checker` on each submission.

The interpreter retained types; the checker did not. The session had **one
authority** (the interpreter) that the other (the checker) could not see — a
checker/runtime divergence.

### Root Cause

`globals: Vec<(String, bool, bool)>` encoded only `(name, mutable, is_fn)`.
There was no representation for a struct, enum, or alias, so
`Checker::with_globals` rebuilt a checker that had never heard of them.

### Fix

Replaced the ad-hoc tuple list with a real session-declaration model:

* `check::GlobalDecl` — one enum covering `Binding`, `Function`, `Struct`,
  `Enum`, and `Alias`, carrying their type information.
* `Checker::with_declarations(&[GlobalDecl])` — rebuilds every symbol table,
  registering type names first and then resolving fields/payloads (so
  declaration order and aliases work).
* `repl.rs` — collects `GlobalDecl`s from each successful submission and
  passes them to the checker. The interpreter is unchanged.

This is a single coherent declaration list, not a second parallel metadata
store: it is the session's semantic state, and both the checker and the
interpreter's existing tables derive from the same declarations.

### CLI/REPL Consistency

The REPL still uses the same parser, checker (`check_mode(Module)`), and
interpreter as the file front end. The only intended differences remain the
absence of a `main` requirement and the echo of bare-expression values. A
declaration present in the session now resolves for later submissions, and a
failed submission neither persists a bad declaration nor removes a good one.

### Regression Tests

`tests/repl.rs`:

* `regression_repl_struct_persistence` — struct usable later, including from a
  function defined in another submission.
* `regression_repl_enum_persistence` — variant construction persists.
* `regression_repl_alias_persistence` — `type Id = int` usable in a later
  binding and as a struct field type.
* `session_rejects_unknown_names_without_corruption` — an undefined name still
  errors and the session keeps working.

### Discovered bug fixed here (blocked B5's alias case)

`type Name = T` aliases were never resolved: `Ty::from_expr` turned an alias
name into `Ty::Named(name)`, which is not compatible with the target. So
`let x: Id = 5` was rejected **in files too**. This predates B5 but blocks the
required "type → later usage" session test. The checker now records alias
targets and substitutes them recursively (`resolve_type_expr`), making the
documented "transparent alias" real everywhere. Tests: the alias cases in
`tests/repl.rs` and the alias coverage in `tests/regressions.rs`.

---

## B6 — Enum Variant Named Arguments

### Reproduction

```
enum E { A(int) }
A(x: 1)   -> checker accepted; runtime E3001: variant `A` expects 1 value(s)
```

### Grammar

`ctor_arg = [ IDENT ":" ] expr` is shared by struct construction and variant
construction, so the parser accepts `A(x: 1)` and represents it as
`Expr::Construct("A", [Arg { name: Some("x"), value: 1 }])`.

### AST

`Expr::Construct` with a named `Arg`. The AST can represent named variant
payloads, but nothing downstream gives the name meaning.

### Checker

Previously validated only that `A` was a declared variant; it accepted the
named form.

### Runtime

`construct` collected named and positional payloads separately. The variant path
used only positional values and checked `positional.len() == arity`, so a named
argument produced a confusing arity error.

### Final Semantic Rule

**Enum variants are positional. Named payload arguments are invalid and are
rejected by the checker with `E3001` before execution.**

Rationale from the repository's own design: enum payloads are declared as an
ordered tuple of types (`A(int, string)`); matching, equality, and display all
treat payloads positionally; nothing in the contract gives variant payloads
names; and "one spelling per construct" argues against inventing named payload
semantics here. The shared grammar is a parsing convenience, not a promise.
The runtime also rejects the named form, so the grammar/AST/checker/runtime now
tell the same story.

### Regression Tests

`tests/regressions.rs::b6_enum_named_argument_contract` — named form is
`E3001`; positional form works; wrong arity and wrong payload type are
`E3001`; an untyped payload is accepted. `tests/regressions.rs::
b3_struct_field_type_validation` covers variant payload type checking.

### Documentation

Contract §3 now states: "An enum variant is built positionally (`Ok(x)`); named
payload arguments are rejected (`E3001`). The payload count and, where
statically known, each payload value's type must match the declaration."

---

## Checker/Runtime Conformance After Fixes

A targeted sweep of struct/enum/alias/numeric/nesting programs found:

* **No** `checker accepts → runtime fails with E1xxx/E4999`.
* **No** `checker rejects → runtime would succeed`.

Errors that remain runtime-only are the ones the checker cannot prove, which is
the documented `Ty::Unknown` boundary (for example calling a non-callable
binding, indexing a value the checker cannot type, or supplying an `Unknown`
field value). The static-check matrix after the fix:

| Condition | Checker | Runtime | Result |
|---|---|---|---|
| known field, compatible value | accept | accept | accept |
| known field, provably wrong type | `E3001` | (unreached) | reject |
| unknown field name | `E2003` | `E2003` | reject |
| missing required field | `E3001` | `E3001` | reject |
| duplicate field | `E3001` | `E3001` | reject |
| positional arity mismatch | `E3001` | `E3001` | reject |
| field value `Unknown` | accept | accept | accept |
| named variant argument | `E3001` | `E3001` | reject |
| variant payload arity/type | `E3001` | `E3001` | reject |

---

## Security / Resource Limits

* The 256 AST-nesting limit and the 512 call-frame limit are intact.
* Parsing now runs on a bounded 64 MiB stack, mirroring execution; the parser
  frame backstop (`MAX_PARSE_DEPTH = 2048`) still caps raw recursion.
* `MAX_AST_DEPTH` remains the single semantic limit, shared by the parser's
  iterative check, the checker, and the evaluator.
* Deep-nesting sweeps (200–5000 levels) across every recursive construct end in
  `E1015` or a parse error, never a host failure; 20 malformed adversarial
  inputs produced no panic or abort.

---

## Error Model Verification

* No new diagnostic codes were introduced. `E3001` (type mismatch) and `E2003`
  (undefined/unknown field) are reused.
* Phase is preserved: construction and nesting are rejected at check/parse time
  before execution.
* Spans and messages are preserved or improved (they name the offending field,
  variant, or construct).
* No valid program reaches `E4999`, a panic, or `unreachable!`; the two
  remaining `E4999` sites in `binary`/`numeric` are unreachable and unchanged.

---

## Documentation Verification

Contradictions confirmed or created by this phase were corrected in
`docs/contract.md` only:

* §2 — annotations enforced where provable; aliases are transparent.
* §3 — struct and enum construction rules.
* §7 — division/remainder by zero; the AST nesting rule and its single
  diagnostic.
* §10 already described transparent aliases; it is now accurate.

No other documentation was edited. `LANGUAGE_SPEC.md` was **not** created.

---

## Scope Discipline

All changes map to B1–B6 or to a defect that blocked them:

```
docs/contract.md      B1, B2, B3, B4, B6, alias transparency
src/parse/mod.rs      B2
src/check/mod.rs      B3, B4, B6, B5 session declarations, alias resolution
src/repl.rs           B5
src/run/mod.rs        B1, and runtime defense for B3/B4/B6
tests/boundaries.rs   B2
tests/parser.rs       B2 (updated the assertion that encoded the old limit)
tests/regressions.rs  B1, B3, B4, B6
tests/repl.rs         B5
```

No generics, traits, async, modules, VM, optimizer, new operators, new types,
or new control flow. No unrelated refactor or rename.

---

## Tests Executed

Run on this working tree:

```
cargo fmt --all -- --check                                            clean
cargo clippy --all-targets --all-features -- -D warnings               clean
cargo clippy --no-default-features --features cli,repl,json,regex,time -- -D warnings   clean
cargo clippy --no-default-features --features cli -- -D warnings       clean
cargo test --all-features                                              all pass
cargo test --no-default-features --features cli,repl,json,regex,time   all pass
PROPTEST_CASES=2048 cargo test --test property --all-features          pass
cargo test --test contract --test examples --all-features              pass
cargo build --release --no-default-features --features cli,repl,json,regex,time   ok
cargo tree --no-default-features ...                                   no pyo3
cargo +nightly miri test --lib --no-default-features --features cli    pass
git diff --check                                                       clean
```

Additional targeted probes (temporary, under `/tmp/opencode/sc`, removed):

* B1 zero/negative-zero remainder matrix — all `E4007`; non-zero unchanged.
* B2 nesting at limit−1/limit/limit+1 for flat chains, nested lists,
  parentheses, maps, calls, blocks, `if`, lambdas, `match` — deterministic
  `E1015`/parse error, no crash.
* B3/B4 struct field type, unknown, missing, duplicate, extra, positional,
  nested-struct, struct-in-list, struct-returning-function.
* B6 named/positional/wrong-arity/wrong-type variant construction.
* B5 REPL sessions for struct/enum/alias/function/binding persistence,
  mutation, and error recovery.
* Reverse conformance — no checker false positives.
* 20 malformed adversarial inputs — no host failure.

MSRV note: the 1.83 toolchain is not installed in this environment. The changes
use only long-stable APIs (`std::thread::Builder`, `HashMap`, `Rc::ptr_eq`,
`Option::is_some_and` from 1.70); `cargo check --features cli` on 1.83 is
expected to pass and is exercised by CI's MSRV job.

---

## Remaining Findings

None blocking. Behaviour that remains intentionally dynamic (checker cannot
prove) is unchanged and documented:

* calling a non-callable binding (`E3001` at runtime),
* field access on an untyped receiver,
* an untyped value assigned to an annotated field/payload (accepted).

These are the conservative `Ty::Unknown` boundary, not contradictions.

---

## Deferred Architecture Debt

Recorded, not fixed:

* Runtime method aliases `"up"`/`"down"` in `string_method` are absent from the
  signature registry and unreachable through the pipeline.
* `Tok::As` is lexed but no production consumes it.
* `json_encode`/`json_decode` bypass the shared `arity()` helper.
* `stdlib::arity` retains a min/max fallback beside the registry.
* `Expr::Tuple` lowers to a list at runtime.
* `Expr::Field` without parentheses auto-calls a zero-argument method; this is
  undocumented (out of scope here).

---

## Deferred Language Decisions

* Empty-map literal: `{}` is always a block; there is no empty map expression.
* `%` by zero: now `E4007` for both int and float per the contract; the
  alternative (IEEE `NaN` for float) was rejected because the contract and the
  `int` path already promised `E4007`.
* Static enforcement of user-function call argument types and arity remains
  undone (only builtins and construction are checked); this is a future
  decision, not a contradiction.

---

## Readiness for LANGUAGE_SPEC.md

**Ready.**

All six audit findings are closed or explicitly resolved, the checker and
runtime agree on the corrected surfaces, diagnostics are deterministic and
documented, resource limits are intact, and the full suite is green. The
remaining items are either intentionally dynamic (the `Unknown` boundary),
documented limitations, or deferred architecture debt — not unresolved
contradictions. The language specification phase can proceed against this
tree.
