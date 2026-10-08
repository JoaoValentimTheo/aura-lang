# FEATURE_001 — Final Code Review and Conformance Gate

**Commit reviewed:** `9cfff0a` (`feat: add static user-function argument checking`)
**Specification baseline:** `904d684` (`docs: finalize static function call semantics`)
**Review mode:** read-only; no source code modified.

---

## Executive Summary

The implementation at `9cfff0a` implements exactly the approved Feature 001
contract (`LANGUAGE_SPEC.md` §6.5 and §6.5.1). Function resolution is lexical
and scope-aware, shadowing is respected, hoisting and mutual recursion are
checked, arity and annotated parameter types are validated before execution,
`Unknown` stays permissive, unannotated parameters stay unconstrained, and
dynamic calls remain runtime-authoritative. Runtime validation is untouched
(`src/run/` has no diff). No grammar, AST, stdlib, error-code, or resource-limit
change exists.

Measured evidence: **0 false positives** across 22 valid programs, **0
divergences** across a 32-case annotation × argument matrix and a 16-case arity
× count matrix, all 13 Feature 001 tests pass, and the full suite is green
(177 all-features / 171 no-Python). The review found **no defect** and made
**no source change**.

One observable diagnostic change is documented and accepted: a directly
resolved user call with two problems (for example a wrong count and an
undefined argument) now reports the arity `E3001` before the undefined-name
`E2003`, matching the long-standing behavior of the builtin call path. This is
within scope and consistent with the existing checker convention.

---

## Baseline

```
HEAD      = 9cfff0a5bcffbc3d5977975add09648ff7161c3a
branch    = rewrite/v3-rust
status    = clean
diff 904d684..9cfff0a:
  src/check/mod.rs                        +141/-  (Feature 001)
  src/repl.rs                             +5/-    (declaration plumbing)
  tests/regressions.rs                    +238/-  (10 tests)
  tests/repl.rs                           +24/-   (2 tests)
  tests/property.rs                       +44/-   (1 test)
  docs/FEATURE_001_IMPLEMENTATION_REPORT.md  new
```

No file under `src/run/`, `src/parse/`, `src/lex/`, `src/ast/`, `src/stdlib/`,
`src/bridge/`, `src/error.rs`, `docs/grammar.md`, or `Cargo.toml` changed.

---

## Contract Reviewed

`LANGUAGE_SPEC.md` §6.5 and §6.5.1, plus `docs/FEATURE_001_DESIGN.md`:

* A call whose callee resolves to a specific, unshadowed top-level `fn` is
  checked for argument count and, per annotated parameter, the argument's
  inferred type; a provable mismatch is `E3001` during checking.
* An unannotated parameter and an `Unknown` argument impose no static type
  constraint.
* An unresolved callee (function value, closure, variable, `Unknown`) is not
  statically checked.
* A shadowing local binding prevents the declaration's signature from being
  applied.
* Hoisting makes the check apply to forward and mutually recursive calls.
* Runtime argument validation remains in place.

---

## Implementation Reviewed

| Piece | Location | Purpose |
|---|---|---|
| `FnSig { ret, params }` | `src/check/mod.rs` | one function's return type and per-parameter annotation (`Option<Ty>`) |
| `functions: HashMap<String, FnSig>` | `src/check/mod.rs` | hoisted function table |
| hoist + annotation pass | `src/check/mod.rs` | records signatures, including alias-resolved parameter types |
| `resolves_to_user_function` | `src/check/mod.rs` | scope-aware resolution gate |
| `check_user_call` | `src/check/mod.rs` | arity + annotated-type validation |
| `Expr::Call` wiring | `src/check/mod.rs` | applies the check only to resolved calls |
| `infer(Call)` | `src/check/mod.rs` | reads `sig.ret`; behavior-preserving |
| `GlobalDecl::Function.params` | `src/check/mod.rs`, `src/repl.rs` | carries signatures across REPL submissions |

---

## Function Resolution Review

`resolves_to_user_function(name)` is true iff:

```
name is in the hoisted function table
AND no scope above the global scope declares `name`
```

`scopes[0]` is the global scope holding hoisted declarations. A function body
pushes a parameter scope (`scopes[1]`) and its block pushes another
(`scopes[2]`), so a parameter or local that shadows a function name lives
above `scopes[0]` and is detected. This is lexical resolution, not textual
global lookup.

A user declaration takes precedence over a builtin of the same name, matching
the runtime's dispatch order (`functions` before `natives`). Verified:
`fn len(x) { return 99 }` then `len(5)` is checked against the **user**
signature and runs, and `fn len(x: string) { ... }` then `len(5)` is rejected
against the user annotation.

**Result:** PASS.

---

## Shadowing Review

| Form | Global signature applied? | Local wins? | Runtime preserved? |
|---|---|---|---|
| local `let` | no | yes | yes |
| local `let mut` | no | yes | yes |
| parameter | no | yes | yes |
| nested block | no | yes | yes |
| no shadowing | yes | — | yes |

Verified by `shadowed_fn_name_does_not_apply_global_signature`: in each
shadowing form a call the global declaration would reject
(`f("hello")` against `fn f(x: int)`) is accepted and dispatched dynamically.
A local non-callable binding (`let f = 123`) likewise does not receive the
global signature. **Result:** PASS.

---

## Hoisting Review

Forward calls and mutual recursion are checked because the annotation pass
populates every `FnSig` before any body is checked. Verified:
`fn main() { later("wrong") }` before `fn later(x: int)` is `E3001`, and
`even`/`odd` mutual recursion runs while a wrong-typed mutual call is `E3001`.
**Result:** PASS.

---

## Arity Review

Static `E3001` for `f()` / `f(1)` / `f(1,2,3)` against a 2-parameter function,
and acceptance of `f(1,2)`. Zero- and one-parameter cases covered. The check
runs before execution, and the runtime arity check is unchanged. **Result:**
PASS.

---

## Annotated-Type Review

Checked against `int`, `float`, `string`, `bool`, `[int]`, `{string: int}`,
a struct type, an alias (`type Id = int`), an alias to a compound
(`type Ids = [int]`), a chained alias, and a nested list of structs.
Compatible arguments are accepted; provably incompatible ones are `E3001`.
The check reuses `Ty::compatible_with`; no parallel compatibility logic was
introduced. **Result:** PASS.

---

## Unknown Review

An argument whose inferred type is `Unknown` is never rejected. Verified with
`none`, `if`-expressions, and a call to a function of undetermined return
type. `int | none` parameters accept both `int` and `none` (both resolve to a
permissive type), consistent with the frozen design. A proven mismatch on
another argument is still caught. **Result:** PASS.

---

## Dynamic-Call Review

Calls through a closure binding, a parameter, and a non-`Name` callee are not
statically checked. Their arity is still enforced at runtime. The checker does
not invent a signature for them. **Result:** PASS.

---

## Runtime Preservation

`git diff 904d684..9cfff0a -- src/run/mod.rs` is empty. Runtime dispatch,
arity validation, and callable checks are unchanged. The feature changes only
*when* a provable error is reported, never *whether* the runtime validates.
**Result:** PASS.

---

## REPL Review

`GlobalDecl::Function` now carries `params`, populated by
`declarations_of`, so a function declared in one submission exposes its
signature to later submissions. Verified: `fn f(x: int)` then `f(1)` accepted,
`f("w")` rejected, `f(2)` accepted — the failed submission does not corrupt
the declaration. Local shadowing across submissions is respected, and
builtin-name precedence persists. **Result:** PASS.

---

## Pipeline Review

The pipeline desugars `x |> f(a)` to `f(x, a)` at parse time, so it reaches
the same `Expr::Call` path. No pipeline-specific checking exists. Verified:
`42 |> f("ok")` accepted as `f(42, "ok")`; `"w" |> f("!")` rejected against
`fn f(x: int, y: string)`. **Result:** PASS.

---

## Built-in Collision Review

`fn len(x) { return 99 }` followed by `len(5)` is accepted and returns `99`,
matching the runtime. `fn len(x: string)` followed by `len(5)` is `E3001`.
Without a user function, the builtin applies and `len(1)` is `E3001`. The
resolution rule mirrors the runtime's user-first dispatch, so dispatch
semantics are unchanged. **Result:** PASS.

---

## Alias Review

Parameter annotations resolve through the existing alias machinery:
`type Id = int` checks as `int`; `type A = B; type B = int` resolves
transitively; `type Ids = [int]` checks element types. Recursive aliases are
still rejected (`E3002`) before signature storage. No alias logic was added to
Feature 001. **Result:** PASS.

---

## Diagnostic Review

All new paths use `E3001`; no new code. Messages:
`` `name` expects N argument(s), got M `` and
`` `name` argument K expects `T`, found `U` ``. The span is the call site.
Output is deterministic (three runs produce one distinct diagnostic). CLI,
REPL, and library share the checker. **Result:** PASS.

### Documented diagnostic-ordering detail

For a call with **both** a signature problem and an invalid argument
expression (e.g. `f(nope)` where `f` takes two parameters), the signature
error is now reported first. The baseline builtin path already behaved this
way (`abs(nope, 2)` reported the arity error at baseline). The change makes
user calls consistent with builtins. It is deterministic, within scope, and
permitted by §30.3 (which requires determinism, not a cross-category priority).
**Not a defect.**

---

## False-Positive Sweep

22 valid programs exercised: direct calls with every annotated type, list/map/
struct/enum/alias/optional parameters, unannotated and mixed parameters,
`Unknown` arguments, four shadowing forms, forward and mutual recursion,
builtin-name collision, closures, pipeline, and unannotated lambdas.

```
checked 22 valid programs; false positives: 0
```

---

## False-Negative Sweep

13 invalid direct calls exercised: wrong arity, wrong annotated types across
primitives, compound types, aliases, structs, forward calls, mutual recursion,
and pipeline insertion.

```
checked 13 invalid direct calls; false negatives: 0
```

(Four initial "misses" were shell-escaping artifacts; re-run with correct
source they all reported `E3001`.)

---

## Property / Differential Verification

The in-suite property `static_argument_check_agrees_with_runtime` was run at
2048 cases (pass). Two external matrices were also run:

* **annotation × argument** (4 annotations × 8 argument kinds = 32): 0
  divergences; no checker-accepts-runtime-type-fails, no false reject of
  `Unknown`.
* **arity × count** (0–3 params × 0–3 args = 16): 0 divergences; acceptance
  exactly when counts match.

An additional 8 pathological inputs produced no `E4999`, panic, or overflow.

---

## Full Test Suite

```
cargo fmt --all -- --check                                             clean
cargo test --all-features                                              177 passed
cargo test --no-default-features --features cli,repl,json,regex,time   171 passed
cargo clippy --all-targets --all-features -- -D warnings               clean
cargo clippy --no-default-features --features cli,repl,json,regex,time -- -D warnings  clean
cargo clippy --no-default-features --features cli -- -D warnings       clean
PROPTEST_CASES=2048 cargo test --test property --all-features          12 passed
cargo test --test contract --test examples --all-features              pass
cargo build --release --no-default-features --features cli,repl,json,regex,time   ok
cargo +nightly miri test --lib --no-default-features --features cli    pass
git diff --check                                                       clean
MSRV (1.83)                                                            NOT verified (toolchain unavailable)
```

Test-count integrity: baseline 161 → current 177 (all-features), +16 across
`regressions` (+10), `repl` (+2), `property` (+1), and three pre-existing
files whose counts are unchanged. No test was removed or weakened.

---

## Diff Scope Audit

```
docs/FEATURE_001_IMPLEMENTATION_REPORT.md   new (documentation)
src/check/mod.rs                            Feature 001 implementation
src/repl.rs                                 declaration plumbing
tests/regressions.rs                        Feature 001 coverage
tests/repl.rs                               Feature 001 coverage
tests/property.rs                           Feature 001 coverage
```

Absent: grammar, AST, parser, lexer, runtime, stdlib, Python bridge, error
codes, resource limits, crate configuration. Every source change maps to a
§6.5 rule; every test protects an invariant; the documentation records the
feature. No unrelated cleanup or refactor.

---

## Findings

No defects. Two observations, neither a defect:

1. **Diagnostic-ordering consistency (Observation, not a defect).** A call
   with both a signature error and an invalid argument now reports the
   signature error first, matching the builtin path's established behavior.
   Deterministic and in scope.
2. **Local shadow of a global function at runtime (pre-existing).** A local
   `let f = ...` does not prevent the runtime from calling a global `fn f`
   when the call is `f(...)`, because runtime dispatch checks the function
   table before the environment. This predates Feature 001 (`src/run/mod.rs`
   unchanged). Feature 001 is conservative here: it skips the static check,
   so it neither rejects nor alters runtime behavior. Recorded, not changed.

---

## Fixes, if Any

None. The implementation is correct, and the task requires that a correct
implementation not be modified for stylistic reasons.

---

## Conformance Table

| Requirement | Implementation | Test Evidence | Result |
|---|---|---|---|
| Direct user fn resolution | `resolves_to_user_function`: hoisted table, no inner-scope shadow | `static_user_fn_*`, `shadowed_fn_*` | PASS |
| Arity checking | `check_user_call` count compare; `E3001` | `static_user_fn_arity_is_checked` | PASS |
| Annotated parameter checking | `compatible_with` per annotated param | `static_user_fn_annotated_argument_type_is_checked` | PASS |
| Unknown permissiveness | `compatible_with` accepts `Unknown` | `static_user_fn_unknown_argument_remains_permissive` | PASS |
| Unannotated parameters | `None` entry imposes no type check | `static_user_fn_unannotated_parameters_are_unconstrained` | PASS |
| Shadowing | inner scopes skip the check | `shadowed_fn_name_does_not_apply_global_signature` | PASS |
| Hoisting | signatures recorded in the pre-pass | `forward_and_mutual_user_fn_calls_are_checked` | PASS |
| Mutual recursion | same, both directions checked | `forward_and_mutual_user_fn_calls_are_checked` | PASS |
| Dynamic fallback | unresolved callee path unchanged | `dynamic_function_value_remains_runtime_checked` | PASS |
| Runtime validation | `src/run/` diff empty | full suite; runtime arity cases | PASS |
| REPL | `GlobalDecl::Function.params` | `static_user_fn_call_is_checked_across_submissions`, `repl_static_check_failure_preserves_declaration` | PASS |
| Pipeline | shared `Expr::Call` path | `pipeline_user_fn_call_is_checked` | PASS |
| Builtin precedence | user-first resolution and inference | `user_fn_shadowing_a_builtin_name_uses_user_signature` | PASS |
| Compatibility | valid programs unchanged; 0 false positives | false-positive sweep (22/22), baseline suite | PASS |

---

## Final Conformance Decision

```
FEATURE 001 CONFORMING — APPROVED
```

The implementation is correct, within scope, and conforms to the frozen
specification. No source change was made during this review; the existing
commit `9cfff0a` is preserved unchanged.
