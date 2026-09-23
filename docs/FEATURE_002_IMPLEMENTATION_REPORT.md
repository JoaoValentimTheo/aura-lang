# FEATURE_002 — Named Function Arguments — Implementation Report

**Status:** Implemented
**Baseline:** `c1eb185` (Feature 002 design finalized)
**Normative contract:** `docs/LANGUAGE_SPEC.md` §4.5, §13, §15.7, §23, §29, §33, §34
**Design:** `docs/FEATURE_002_DESIGN.md`

---

## Repository State

```
HEAD (before this phase) = c1eb185
branch                   = rewrite/v3-rust
working tree             = clean
```

## Implemented Semantics

Named arguments (`name: value`) are supported for **directly resolved
top-level user-defined functions**. Positional arguments fill the next unfilled
parameter; named arguments fill the parameter with that exact, case-sensitive
name; positional arguments must precede named arguments. A parameter supplied
twice, a named argument naming no parameter, and a declared parameter left
unfilled are each `E3001` at check time. Type checking reuses Feature 001's
`Ty::compatible_with` after mapping; `Unknown` stays permissive. Built-ins,
methods, and dynamic/unknown callables reject named arguments with `E3001`.

**Evaluation order is source order and independent of parameter binding.** The
implementation evaluates every argument expression left-to-right, once, and
only then associates values with parameters.

## Architecture: evaluation → mapping → invocation

This is the mandatory architecture section.

```
source:  f(second: e1(), first: e2())         # parameters: [second, first]

1. evaluate argument expressions in source order, once
       vals = [ e1(), e2() ]                   # e1 before e2

2. map arguments to parameters (pure, no evaluation)
       positional -> next unfilled parameter
       named      -> exact parameter by name
       bound = { second: vals[0], first: vals[1] }

3. invoke the existing runtime calling convention with parameter-ordered values
       call(closure, [ vals[0], vals[1] ])     # closure.params = [second, first]
```

* The evaluator (`run::Interp::eval_call`) evaluates `arg.value` in source
  order into `vals`, then calls the free helper `bind_arguments(args, vals,
  params, span)` to reorder `vals` into parameter order.
* `bind_arguments` never evaluates anything; it only permutes already-evaluated
  values.
* The existing positional call machinery (`Interp::call`) is unchanged: it
  receives parameter-ordered values and binds them positionally.
* The checker computes the same mapping independently for validation, using
  the AST (no source of runtime values). The mapping is pure and
  order-independent, so checker and runtime agree by construction.

No expression is duplicated or re-executed. Canonical positional normalization
happens **after** evaluation, on the evaluated values.

## Parser Changes

`src/parse/mod.rs`:

* New `Parser::call_args()` parses a parenthesized argument list using the
  existing `cons_arg` form (`[ IDENT ":" ] expr`), enforcing positional-then-
  named. A positional argument after a named one is `E1006`.
* `postfix` now uses `call_args()` for `Expr::Call` and `Expr::Method`
  arguments.
* `desugar_pipe` inserts the piped value as a positional `Arg { name: None, .. }`
  at the front of the argument list.
* `check_expr_depth` traverses `Arg.value` for call/method arguments.
* The now-unused `arg_expr` helper was removed.

## AST Changes

`src/ast/mod.rs`: `Expr::Call` and `Expr::Method` now carry `Vec<Arg>` instead
of `Vec<Expr>`, reusing the existing `Arg { name: Option<String>, value: Expr }`
already used by `Expr::Construct`. No parallel AST hierarchy was introduced.

## Checker Changes

`src/check/mod.rs`:

* `FnSig.params` changed from `Vec<Option<Ty>>` to `Vec<(String, Option<Ty>)>`,
  carrying parameter names and annotations.
* `GlobalDecl::Function.params` carries `(name, annotation)` for REPL sessions.
* `hoist` records parameter names immediately; the annotation pass fills the
  types.
* `check_user_call` performs parameter satisfaction (positional then named,
  duplicate/unknown/missing detection) and then the Feature 001 type check
  against the mapped arguments.
* New `reject_named_args` rejects named arguments on built-ins, methods,
  callable bindings, and non-`Name` callees.
* `check_builtin_call`/`check_method_call` now take `&[Arg]` and read
  `.value`.
* `resolves_to_user_function` is unchanged (Feature 001 resolution).

## Runtime Changes

`src/run/mod.rs`:

* `eval_call` tests against `&[Arg]`, evaluates `arg.value` in source order
  once, and calls `bind_arguments` for a resolved user function before the
  existing `Interp::call`.
* New free `bind_arguments` maps evaluated values to parameter order.
* The `Expr::Method` eval arm reads `arg.value`.
* Runtime validation remains: `Interp::call` still checks arity and
  non-callables as the final layer.

## REPL Changes

`src/repl.rs`: `declarations_of` carries `(parameter name, annotation)` pairs
so a session function's parameter names persist across submissions.

## Pipeline Changes

None beyond the parser desugaring. `x |> f(y: 1)` becomes `f(x, y: 1)`; the
ordinary call path validates and binds it. `x |> f(x: 1)` is a duplicate.

## Evaluation-Order / Stack-Hardening Fix

Feature 002 enlarged the `Expr` node (arguments became `Arg`), which reduced
the drop/checker-recursion headroom. The checker previously ran on the caller
thread; a valid program near the nesting limit could then overflow a
small-stack caller. This was fixed by running the checker on the large
interpreter stack in `lib::compile_with_mode` (`check_on_big_stack`), so
session:

```
submission 1:  fn greet(name: string, punctuation: string) { ... }
submission 2:  greet(punctuation: "!", name: "João")
```

resolves against the persisted parameter names. A failed named call does not
mutate or remove the stored signature.

## Error Behavior

All new cases reuse `E3001`; positional-after-named is the parser's `E1006`.

| Case | Code | Phase |
|---|---|---|
| unknown parameter name | `E3001` | check |
| duplicate parameter | `E3001` | check |
| missing parameter | `E3001` | check |
| named on built-in | `E3001` | check |
| named on method | `E3001` | check |
| named on dynamic/unknown callee | `E3001` | check |
| positional after named | `E1006` | parse |
| type mismatch | `E3001` | check (Feature 001) |

## Specification Changes

`docs/LANGUAGE_SPEC.md` amended: §4.5 (`call_args = arg { "," arg }`, the
positional-then-named rule), §13 (binding independent of evaluation order),
§15.7 (rewritten for named arguments), §23 (pipeline receiver as first
positional argument), §29 (REPL persists parameter names), §33 (frozen
decision 18), §34 (limitations updated). `docs/grammar.md` and
`docs/contract.md` updated to match.

## Tests

15 new tests: 12 in `tests/regressions.rs` (binding, reordering, mixed,
ordering error, unknown, duplicate, missing, type check, scope boundary,
hoisting/mutual recursion, pipeline, evaluation order, aliases, positional
regression), 1 in `tests/repl.rs`, 1 property in `tests/property.rs`
(named permutation equals positional), plus the earlier Feature 002 grammar
coverage via the sample runner.

## Property Tests

`named_permutation_matches_positional`: for random `a`,`b`,`c`, the call
`f(c: c, a: a, b: b)` produces the same output as `f(a, b, c)`. Run at 2048
cases (pass).

## Regression Results

```
all-features       192 passed (baseline 177; +15)
no-default-features 186 passed (baseline 171)
```

All pre-existing tests pass unchanged; none were weakened or removed.

## Conformance Audit

* E4999/panic sweep over named calls: 0 internal failures.
* False-positive sweep (11 valid named/positional programs): 0.
* False-negative sweep (9 invalid programs): 0.
* Evaluation order verified by side effects: source order preserved.
* Single evaluation verified: each argument expression prints once.
* Shadowing, dynamic, built-in, and method boundaries verified.
* REPL persistence and failure isolation verified.

## Known Limitations

* Named arguments are not supported for built-ins or methods (no parameter
  names in the registry).
* No default or variadic parameters.
* A dynamic/unknown callee rejects named arguments rather than resolving them
  at runtime.

## Future Extensions

Add canonical parameter names to the standard-library registry to extend named
arguments to built-ins and methods; add default parameters; define variadic
semantics. None is part of this feature.
