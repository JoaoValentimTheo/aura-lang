# FEATURE_001 — Static User-Function Argument Checking — Implementation Report

**Status:** Implemented and conforming
**Baseline before work:** `904d684` (`docs: finalize static function call semantics`)
**Normative contract:** `docs/LANGUAGE_SPEC.md` §6.5, §6.5.1
**Design:** `docs/FEATURE_001_DESIGN.md`

---

## Executive Summary

Feature 001 was implemented exactly as specified. When the checker resolves a
call to a specific top-level `fn` declaration, it now validates the argument
count and the annotated parameter types before execution, reusing the existing
`Ty::compatible_with` relation and the frozen `Unknown` boundary. Calls that
cannot be resolved to a specific declaration (function values, closures,
unknown callables, shadowed names) remain runtime-authoritative, and runtime
argument validation is retained for every call.

The implementation is a checker-only change plus the REPL declaration plumbing
needed for the check to apply across session submissions. No grammar, AST,
runtime, standard-library, error-code, or resource-limit behavior changed.

Test coverage grew from **161 to 177** passing tests (all-features), with the
new cases split across `tests/regressions.rs` (+10), `tests/repl.rs` (+2), and
`tests/property.rs` (+1). All pre-existing tests pass unchanged; no old test
was weakened or removed.

One subtlety surfaced during implementation and was resolved to preserve
baseline behavior: a user function whose name matches a builtin takes
precedence, exactly as the runtime dispatches. The static check follows that
precedence rather than the builtin, so no previously valid program changed
meaning.

---

## Baseline

```
HEAD      = 904d68413612bce00146b0831c489c7fa3a39a8e
branch    = rewrite/v3-rust
status    = clean
predecessors: 3b83b2a, 0385456, cbd2dc9, aba8866, d940fbe
```

---

## Feature Contract

From `LANGUAGE_SPEC.md` §6.5:

* A call whose callee resolves to a specific top-level `fn` declaration and is
  not shadowed is checked for argument count and, per annotated parameter, for
  the argument's inferred type, before execution (`E3001`).
* An unannotated parameter imposes no static type constraint.
* An `Unknown` argument is never rejected merely for being `Unknown`.
* An unresolved callee — function value, closure, variable, `Unknown` — is not
  statically checked; the runtime remains authoritative.
* A shadowing local binding means the declaration's signature MUST NOT be
  applied.
* Hoisting makes the check apply to forward and mutually recursive calls.
* Runtime argument validation remains in place.

---

## Implementation Strategy

The checker already resolved calls to a top-level function and stored its
return type; it validated parameter annotations during hoisting but discarded
them. Feature 001 therefore required only:

1. retaining each function's parameter annotations alongside its return type;
2. a resolution predicate that distinguishes a directly resolved declaration
   from a shadowed or dynamic callee;
3. a small validation routine reusing `Ty::compatible_with`.

No new type machinery, no global analysis, and no runtime change.

---

## Function Resolution

`Checker::functions` changed from `HashMap<String, Option<Ty>>` (return type
only) to `HashMap<String, FnSig>` where:

```rust
struct FnSig {
    ret: Option<Ty>,
    params: Vec<Option<Ty>>,
}
```

A parameter is `Some(ty)` when annotated, `None` otherwise. Parameter types are
resolved with the same alias-aware `annotation` pass used for every other
annotation.

Resolution is scoped, not textual:

```rust
fn resolves_to_user_function(&self, name: &str) -> bool {
    self.functions.contains_key(name)
        && !self.scopes.iter().skip(1).any(|s| s.declares.contains_key(name))
}
```

`scopes[0]` is the global scope holding hoisted declarations; any inner scope
(`scopes[1..]`) that declares the name shadows it. A user declaration takes
precedence over a builtin of the same name, matching the runtime's dispatch
(user functions before natives).

---

## Shadowing

Because resolution skips the check when any inner scope declares the name, a
local `let`, `let mut`, nested-block binding, or parameter that shadows a
declaration makes the call dynamic. Verified cases: local `let`, mutable
local, parameter, and nested block. In each, a call the global declaration
would reject (e.g. `f("hello")` against `fn f(x: int)`) is accepted and
dispatched as a callable value.

## Hoisting

Hoisting already registers every top-level function before any body is
checked. The annotation pass then fills each `FnSig` with resolved parameter
types, so calls textually preceding a declaration and mutually recursive calls
are checked against the callee's signature.

## Arity Checking

`check_user_call` compares `args.len()` with `sig.params.len()` and emits
`E3001` with `` `name` expects N argument(s), got M ``. This runs before any
argument-type check, so a wrong count reports the count error.

## Annotated Parameter Checking

For each parameter with `Some(expected)`, the argument's inferred type is
compared with `expected.compatible_with(&actual)`. A provable mismatch emits
`E3001` with `` `name` argument N expects `T`, found `U` ``.

## Unknown Handling

`compatible_with` returns `true` when either side is `Unknown`, so an
`Unknown` argument is never rejected. This preserves §2.3 and §6.4. Verified
with `none`, `if`-expressions, and calls to functions of undetermined return
type.

## Dynamic Call Fallback

When the callee is not a directly resolved declaration — a local callable, a
function value, a parameter, or a non-`Name` callee — no signature check is
applied. The existing "ok: a callable binding" path is preserved, and
`infer(Call)` still returns `Unknown` for such calls.

## Runtime Validation Preservation

No runtime code changed. `Interp::call` still rejects wrong arity and
non-callables, so every call retains dynamic validation as the final layer.
The feature moves a provable error earlier; it does not remove the runtime
guard.

---

## REPL

`GlobalDecl::Function` gained `params: Vec<Option<TypeExpr>>`, populated by
`repl.rs::declarations_of`, so a function declared in one submission carries
its parameter types into later submissions' checkers. The session model is
otherwise unchanged.

Verified sequence: `fn f(x: int)` then `f(1)` (accepted), `f("wrong")`
(`E3001`), `f(2)` (accepted) — the failed submission does not corrupt the
declaration.

## Pipeline

The pipeline desugars `x |> f(a)` to `f(x, a)` at parse time, so it reaches the
same `Expr::Call` path and is checked by the same routine with no
pipeline-specific code. `42 |> f("ok")` is checked as `f(42, "ok")`.

## Diagnostics

Reuses `E3001` (`TYPE_MISMATCH`); no new code. Messages follow the existing
builtin-argument style. The span is the call site. The phase is checking.
Determinism is preserved (single first diagnostic).

---

## Test Matrix

New tests, all passing:

| Test | Proves |
|---|---|
| `static_user_fn_arity_is_checked` | 0/1/2/3-argument functions; too few/many → `E3001`; exact → accepted |
| `static_user_fn_annotated_argument_type_is_checked` | int/float/string/bool matches accepted; four mismatches → `E3001` |
| `static_user_fn_unannotated_parameters_are_unconstrained` | only annotated params constrained; arity still applies |
| `static_user_fn_unknown_argument_remains_permissive` | `none`, `if`-expr, undetermined return all accepted; a proven mismatch elsewhere still rejects |
| `shadowed_fn_name_does_not_apply_global_signature` | local `let`/`let mut`/parameter/nested-block shadowing → dynamic; no shadowing → checked |
| `forward_and_mutual_user_fn_calls_are_checked` | forward reference and mutual recursion checked; valid recursive even/odd runs |
| `dynamic_function_value_remains_runtime_checked` | closure call accepted statically, arity enforced at runtime |
| `pipeline_user_fn_call_is_checked` | `x \|> f(a)` checked with inserted first argument |
| `direct_call_return_type_flows` | return-type inference combined with the call check |
| `user_fn_shadowing_a_builtin_name_uses_user_signature` | user `fn len` wins over builtin, as at runtime |
| `static_user_fn_call_is_checked_across_submissions` (repl) | REPL arity/type rejection across submissions |
| `repl_static_check_failure_preserves_declaration` (repl) | a failed check leaves the declaration usable |
| `static_argument_check_agrees_with_runtime` (property) | annotation × argument matrix: checker/runtime agreement |

Existing coverage retained: contract, grammar, adversarial, boundaries,
lexer, parser, checker, run, examples, python, plus the earlier regression
suite.

---

## Property / Differential Verification

Beyond the in-suite property test, targeted sweeps were run:

* **Annotation × argument matrix** (4 annotations × 7 argument kinds): no case
  where the checker accepted and the runtime failed with `E1xxx`/`E4999`.
* **Arity matrix** (0–3 params × 0–3 args): no such case.
* **False-positive sweep** (16 valid programs, including list/map/struct/enum/
  optional/alias parameter types and builtin-name collisions): no valid
  program rejected.
* **Checker-reject ⇒ runtime-invalid**: an `Unknown` argument (`none`) is never
  rejected; only provable mismatches are.

---

## Backward Compatibility

The compatibility classification from §6.5.1 holds:

* **Previously valid programs** — arguments already compatible, or callee not
  resolved — remain valid and behave identically. The 161-test baseline passes
  unchanged; the false-positive sweep is clean.
* **Previously checker-accepted but dynamically invalid programs** — a wrong
  argument count or a provably incompatible argument — are now rejected during
  checking with the same `E3001` code. This is the intended tightening.
* **Runtime validation** remains active for every call.

One resolution detail was deliberately preserved: a user function named like a
builtin takes precedence, matching the runtime, so the check follows the user
signature rather than the builtin.

---

## Files Changed

| File | Change |
|---|---|
| `src/check/mod.rs` | `FnSig`; `functions` map type; `GlobalDecl::Function.params`; hoist/annotation plumbing; `resolves_to_user_function`; `check_user_call`; call arm; `infer` |
| `src/repl.rs` | `declarations_of` carries parameter types |
| `tests/regressions.rs` | 10 Feature 001 tests |
| `tests/repl.rs` | 2 Feature 001 REPL tests |
| `tests/property.rs` | 1 Feature 001 differential property |

No grammar, AST, runtime, stdlib, error-code, resource-limit, or Python file
changed.

---

## Verification

```
cargo fmt --all -- --check                                             clean
cargo test --all-features                                              177 passed
cargo test --no-default-features --features cli,repl,json,regex,time   171 passed
cargo clippy --all-targets --all-features -- -D warnings               clean
cargo clippy --no-default-features --features cli,repl,json,regex,time -- -D warnings  clean
cargo clippy --no-default-features --features cli -- -D warnings       clean
PROPTEST_CASES=2048 cargo test --test property --all-features          12 passed
cargo test --test contract --test examples --all-features              pass
cargo build --release --no-default-features --features cli,repl,json,regex,time   ok + smoke
cargo +nightly miri test --lib --no-default-features --features cli    pass
git diff --check                                                       clean
MSRV (1.83)                                                            NOT verified (toolchain unavailable)
```

---

## Scope Audit

Every source change maps to a Feature 001 rule (`LANGUAGE_SPEC.md` §6.5):

* `FnSig` and the function map — required to have a signature to check.
* hoist/annotation plumbing — required to populate it, including forward and
  mutually recursive calls.
* `resolves_to_user_function` — required by the resolution-first and shadowing
  rules.
* `check_user_call` — required by the arity and annotated-parameter rules.
* call-arm wiring — required to apply the check to directly resolved calls.
* `infer` adjustment — required because the map's value type changed; behavior
  is identical (return type, `Unknown` when unannotated).
* `repl.rs` — required by the REPL rule.
* tests — one per invariant.

No unrelated refactor, cleanup, or dead-code removal. No grammar or AST
change. No new error code. No runtime semantic change. No duplicated signature
system (the check reuses `Ty::compatible_with`).

---

## Conformance Status

```
implementation  =  design  =  specification  =  tests
```

| Case | Static resolution | Static check | Runtime check | Expected | Result |
|---|---|---|---|---|---|
| Direct fn + correct arity | yes | pass | retained | accept | ✅ |
| Direct fn + wrong arity | yes | `E3001` | retained | reject | ✅ |
| Direct fn + wrong annotated type | yes | `E3001` | retained | reject | ✅ |
| Direct fn + `Unknown` argument | yes | permissive | retained | continue | ✅ |
| Unannotated parameter | yes | unconstrained | retained | continue | ✅ |
| Shadowed callee | no declaration | skip | retained | runtime | ✅ |
| Function value / closure | unresolved | skip | retained | runtime | ✅ |
| Forward fn | yes | check | retained | by signature | ✅ |
| Mutual recursion | yes | check | retained | by signature | ✅ |
| Pipeline direct fn | yes | same rules | retained | by signature | ✅ |
| User fn shadowing a builtin | user declaration | user signature | retained | user semantics | ✅ |

---

## Remaining Limitations

Unchanged and explicit:

* Calls through a function value, closure, variable, or unknown callee are not
  statically checked (§34.2).
* No function types, named/default/variadic arguments, or higher-order
  inference (out of scope for Feature 001).
* Field reads and `if`/`match`/lambda results still infer `Unknown`.
* The REPL remains line-oriented for submission boundaries.

---

## Final Status

**FEATURE 001 IMPLEMENTED AND CONFORMING.** No feature work outside the approved
scope; no runtime semantics changed; runtime validation retained; the frozen
specification, design, implementation, and tests agree.
