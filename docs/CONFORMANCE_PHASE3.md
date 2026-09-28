# AURA LANGUAGE CONFORMANCE PASS — PHASE 3

## 1. Scope

Type representation, assignability, inference, unions, `none`, collections,
functions, constructors, operators, calls, generics, bounds, traits, methods,
overloads, and the builtin registry — reconstructing Aura's semantic type
system from the implementation and verifying it against `docs/LANGUAGE_SPEC.md`
§§5–15, 18, 36, `docs/GENERICS.md`, and `docs/contract.md`.

Starting HEAD: `2462d3f` (Phase 2 closed).

## 2. Type Model

`src/types.rs` `Ty`: `Int`, `Float`, `Bool`, `String`, `List(T)`, `Map(V)`
(string-keyed, §5), `Named(String)`, `Enum(String)`, `Union(Vec<Ty>)`,
`Param(String)`, `App(String, Vec<Ty>)`, `Unknown`.

* **Canonicalization (`Ty::union`).** Flattens nested unions, collapses to
  `Unknown` if any member is `Unknown` (so `T | none` is permissive — `none`
  is `Unknown`), deduplicates, and sorts by a fixed rank + spelling. Total and
  deterministic. `int | int` → `int`; `A | A` → `A`.
* **`Unknown` boundary (§2.3).** Permissive: a rule is not applied where the
  type is unknown; failures become runtime `E4xxx`. `infer` knows a type only
  for literals, annotated names, names bound to known-typed expressions,
  concrete builtin/method returns, and declared function returns.
* **Assignability (§8 matrix).** Nominal for structs/enums; `int`/`float` are
  **not** implicitly convertible (`let x: float = 1` is `E3001`); `none` and
  any union containing `none` collapse to `Unknown` and are accepted anywhere.

All §8 matrix cells were reproduced and match, including `float`←`int` = R,
union-expected `†‡` rules, and `Unknown`-actual = A.

## 3. Confirmed Findings

| ID | Classification | Reproduction / root cause | Fix | Regression |
|---|---|---|---|---|
| CONF-TYPE-1 | IMPLEMENTATION BUG | `infer` returned the left operand type for every non-comparison binary operator, so `1 + 1.5` inferred `int`. `let x: int = 1 + 1.5` was **accepted** and `f(1 + 1.5)` for `f(float)` was **wrongly rejected** (`E3001`), while `run` evaluated it to `2.5`. Contradicts §9.1 (mixed numeric promotes to float). | `infer` now computes the real result type: mixed `int`/`float` arithmetic → `float`; `+` on two strings → `string`; `+` on two equal lists → list of that element; bitwise/shift → `int` only for two `int`s; otherwise `Unknown` (permissive boundary preserved). | `tests/checker.rs::arithmetic_result_type_is_inferred_from_both_operands` |
| CONF-GENERIC-1 | IMPLEMENTATION BUG | `Ty::compatible_with` treats a generic parameter as universally permissive. Both directions are unsound in a `return`: `fn f<T>(x: T) -> T { return 5 }` was accepted and then failed at the call site with a checker-tier `E2003`/`E3001`, and the mirror `fn f<T>(x: T) -> int { return x }` was accepted although `T` may be any type (runtime `E3001` or a silently wrong branch). Violates §15.2 (every `return` must be compatible with the declared return type) and `docs/GENERICS.md`. | Return-position compatibility requires the declared type to be an exact match or a matching union member; any unresolved parameter on either side that is not identical is rejected. `Unknown` stays permissive. Covers functions, methods, `impl<T>` methods, and trait-impl method bodies, including parameters nested in lists, maps, and applications. Valid `-> T`, `-> Box<T>`, `-> T \| int`, and pass-through returns still pass. | `tests/generics.rs::generic_return_must_match_the_declared_parameter`, `concrete_return_is_not_satisfied_by_a_parameter_actual`, `valid_parameter_returns_are_accepted` |

No other checker/runtime **code** disagreement was found: undefined names,
arity, named-argument rules, method dispatch, union-receiver methods, trait
bounds, and every builtin arity/type are enforced identically before and during
execution.

### Checker/runtime discrepancy table

| Construct | Checker | Runtime | Assessment |
|---|---|---|---|
| `1 + "a"`, `true + true`, `"a" * 3` | accept | `E3001` | §9.1 documents arithmetic type errors as **runtime**; permissive boundary is intentional (a value the checker cannot type is not rejected, `contract.md`) |
| `1 << "a"`, `1 & 1.0`, `~1.5` | accept | `E3001` | same |
| `1 / 0`, `5 % 0` | accept | `E4007` | content-dependent, runtime |
| `1 < "a"` | `E3001` | `E3001` | §12: ordering is statically checked when provable — agrees |
| `f(1 + 1.5)` vs `f(float)` | was reject | accept | **fixed** (CONF-TYPE-1) |
| undefined var/function, arity, unknown named arg | `E2xxx`/`E3001` | same | agrees |
| struct `S` has no method `m` | `E2003` | `E2003` | agrees |

A generated sweep over arithmetic/bitwise/comparison/unary/collection
expressions found **zero** divergences after the fix.

## 4. Operator Checking

Every operator was probed for accepted operand pairs against §9.1/§12/§9.2:

* `+` accepts int/float/string/list (with per-operand rules); `- * / % ^`
  numeric only; `& | << >>` int only; `< <= > >=` int/float/string/bool only.
* `not` accepts any value → `bool`; unary `-` numeric; `~` int only.
* Mixed numeric static checking is deliberately **runtime** except where both
  operand types are known (ordering), matching §9.1's `static`/`E3001` split.
* Unicode-aware string indexing/`chars` iterate scalars, not bytes.

## 5. Calls and the Builtin Registry

* Arity (min/max), positional-before-named, duplicate/unknown named arguments,
  and constructors are enforced (`E1006`/`E3001`).
* Named **method** arguments parse but are rejected (`E3001`) — a DESIGN
  LIMITATION (RFC candidate), confirmed unchanged.
* `json_encode`/`json_decode` are the real names (`json_parse`/`json_stringify`
  do not exist — a documentation/naming point, not a defect).
* `regex_*` take `(pattern, text[, replacement])`; an invalid pattern is a
  deterministic `E3001`.
* The strengthened registry oracle (`tests/property_hardening.rs` PROPERTY 1)
  derives every builtin from `stdlib::signatures::builtins()` and asserts
  checker-accepted well-shaped calls never produce a runtime `E2003`. All
  builtins pass.

## 6. Generics, Traits, Overloads

* Generic inference, explicit `f<int>(x)`, nested `Box<Box<int>>`, generic
  structs/enums/aliases/functions/impls all resolve; arity and bounds are
  enforced (`E3001`/`E3002`).
* Traits: missing/extra/mismatched methods (`E2017`/`E2003`/`E3001`), unmet
  bounds, and duplicate method definitions are deterministic.
* Overload selection is by ordered argument types, **independent of
  declaration order** (verified with reversed declarations); a union argument
  matching multiple overloads is rejected, not silently chosen.
* Orphan-rule mechanics are absent; implementing a trait for a local type is
  allowed (matching the spec, which defines no orphan restriction).

## 7. Inference

* Unannotated `let`, branch results, list/map literals, empty collections,
  lambda params, calls, and generic calls were probed. Empty collections infer
  `Unknown` (permissive), so `let mut l = []` plus `push` works.
* Branch/collection inference is first-known-element based (documented
  behavior); no order-dependence was observed in overload selection.
* No flow-sensitive narrowing exists (§13.10): `x == none` does not narrow a
  union. Documented current behavior, not a bug.

## 8. Tests Added

`tests/checker.rs::arithmetic_result_type_is_inferred_from_both_operands` —
locks result-type inference for mixed numeric promotion, string/list `+`, and
the rejection of a provably wrong annotation.

## 9. Validation

`cargo fmt --all -- --check` clean; `cargo clippy --locked --all-targets
--all-features -- -D warnings` clean; `cargo test --locked --all-features` =
**641 passed, 0 failed** (Phase-1 635, Phase-2 640, +1). `node
playground/tests/node/run-all.mjs` green (differential 195, syntax 43, browser
53, worker 12, cache 7; TypeExpr ceilings native 2048 / WASM 768 unchanged).

## 10. Phase-3 Gate

- Confirmed bugs have regressions: yes (CONF-TYPE-1).
- Focused and full checker suites pass: yes.
- Adversarial review: the Phase-2 reviewer's counterexample (a resolver
  regression) was fixed and re-verified; no unresolved type-system
  counterexample was produced.

## 11. Phase-4 Handoff

Verified starting point for execution semantics: a checker whose inferred
types match the runtime's result types, whose permissiveness is exactly the
documented `Unknown` boundary, and whose only intended static/runtime split is
the §9.1 arithmetic-vs-ordering one. No type-system decision is pending except
the Phase-2 `CONF-RESOLVE-6/7/8/9/10` questions and the AUDIT-3 TypeExpr
decision, which Phase 4 must not touch.
