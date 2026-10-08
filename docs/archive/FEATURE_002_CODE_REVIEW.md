# FEATURE_002 — Named Function Arguments — Code Review

**Reviewed artifact:** the Feature 002 implementation commit
**Normative contract:** `docs/LANGUAGE_SPEC.md` §4.5, §13, §15.7, §23, §29
**Design:** `docs/FEATURE_002_DESIGN.md`

---

## Resolution Correctness

`resolves_to_user_function` is unchanged from Feature 001: a callee is a
directly resolved top-level function only if it is in the hoisted function
table and no inner scope shadows it. Named-argument validation begins only
after this resolution. Verified: a local `let f` shadowing a global `fn f`
causes `f(x: 1)` to be rejected (`E3001`), and the global parameter list is
never applied to the local callable. **PASS.**

## Shadowing

Four shadowing forms (local `let`, `let mut`, parameter, nested block) all
suppress the global signature and reject named arguments on the local callable.
**PASS.**

## Argument Mapping

`check_user_call` (checker) and `bind_arguments` (runtime) both map positional
arguments to the next unfilled parameter and named arguments to the exact
parameter name. Both compute the same pure mapping. Verified by
`named_permutation_matches_positional` and by direct probes: `f(b: 9, a: 8)`
returns the value for `a=8, b=9`, not a positional misbinding. **PASS.**

## Duplicate Detection

A parameter supplied positionally and by name, or twice by name, is `E3001` at
check time. Verified: `f(1, a: 2)` and `f(a: 1, a: 2)`. **PASS.**

## Missing Detection

A declared parameter with no argument is `E3001` at check time, reported by
name. Verified: `f(a: 1, c: 3)` against `fn f(a, b, c)` reports the missing
`b`. **PASS.**

## Unknown-Name Detection

A named argument whose name matches no parameter is `E3001`. Verified:
`f(z: 1)`. **PASS.**

## Type Checking

After mapping, the existing Feature 001 check runs: an annotated parameter's
type must be `compatible_with` the mapped argument's inferred type. Verified
with reordered arguments (`f(b: 1, a: "x")` against `fn f(a: int, b: int)`
rejects the `a` argument). **PASS.**

## Unknown Handling

`Unknown` argument types remain permissive: `f(x: none)` against `fn f(x: int)`
is accepted. An unknown *name* is still rejected, because the parameter list is
known regardless of type inference — this is the intended distinction. **PASS.**

## Evaluation Order

This is the central invariant. `eval_call` evaluates every `arg.value` in
source order into `vals` **before** calling `bind_arguments`; binding only
permutes evaluated values. Verified by side-effecting probes:
`combine(second: record("s1", 1), first: record("s2", 2))` prints `s1` then
`s2` and returns `2*10+1 = 21`. **PASS.**

## Single Evaluation

Each argument expression is evaluated exactly once. Verified: a nested named
call with a side-effecting argument prints its tag once; no expression is
re-evaluated during binding. **PASS.**

## Runtime Binding

The runtime receives parameter-ordered values and the existing `Interp::call`
binds them positionally; runtime arity and callable checks remain as the final
safety layer. No runtime calling convention was redesigned. **PASS.**

## Pipeline

The parser inserts the piped value as a positional `Arg` at the front, so
`x |> f(y: 1)` is `f(x, y: 1)` through the ordinary call path.
`x |> f(x: 1)` is a duplicate (`E3001`). No pipeline-specific binding exists.
**PASS.**

## REPL

`GlobalDecl::Function` carries parameter names; a function declared in one
submission is checkable by name in later submissions. A failed named call
leaves the declaration intact. **PASS.**

## Built-in / Method Boundary

Named arguments are rejected for built-ins and methods with `E3001`. The
standard-library registry is unchanged. **PASS.**

## Diagnostics

All new cases use `E3001`; positional-after-named uses `E1006`. Codes, spans,
and phases are deterministic; CLI, REPL, and library share the checker. No new
error code. **PASS.**

## Regression Risk

* The full pre-existing suite passes unchanged (192 all-features, 186 no-py).
* False-positive sweep (valid named/positional programs): 0 rejections.
* False-negative sweep (invalid programs): 0 acceptances.
* E4999/panic sweep over named calls: 0 internal failures.
* The `Vec<Arg>` AST change was audited for ignored `Arg.name`: every
  consumer that binds values uses the name (`check_user_call`,
  `bind_arguments`); builtin/method/dynamic paths reject named arguments
  before use, so no name is silently ignored. **PASS.**

## Stack-Hardening Note

Feature 002 enlarged the `Expr` node, which reduced the headroom of the
checker's recursive walk on small caller stacks. The checker now runs on the
large interpreter stack (`lib::check_on_big_stack`), preserving the "no crash,
`E1015` only" invariant. The nesting-limit and other boundary tests pass.

## Conclusion

The runtime binding architecture is unambiguous: **evaluate in source order,
then map by name, then invoke positionally.** Source-order evaluation is never
altered by parameter binding. The feature matches the specification, stays in
scope, and introduces no new error code or callable abstraction.

**Approved.**
