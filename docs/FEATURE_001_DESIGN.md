# FEATURE_001 — Static User-Function Argument Checking

**Status:** Design (pre-implementation)
**Target specification:** `docs/LANGUAGE_SPEC.md` (frozen at `3b83b2a`)
**This document is not an implementation and does not change the language.**

---

## Motivation

Aura's contract says that type annotations, when present, are checked. Today
that is true for bindings, function return types, struct fields, and enum
payloads — but **not** for the arguments of a call to a user-defined
function. A parameter annotation such as `fn f(a: int)` is used to type the
function body but is not enforced against the value passed at the call site.

This is the single most concrete correctness gap in the frozen language: a
program that the checker could prove wrong runs and produces a value that
violates its own declarations.

## User Problem

Given:

```aura
fn area(w: int, h: int) -> int {
    return w * h
}

fn main() {
    print(area("3", 4))     # compiles and runs today
}
```

Aura today prints `"3333"` (string repetition via `w * h` on `"3"` and `4`
would in fact be a runtime error; the point stands for any mismatch the
runtime happens to tolerate or reports late). The user annotated `w: int`,
so passing `"3"` should be rejected. Similarly:

```aura
fn f(a, b) -> int { return a + b }
fn main() { print(f(1)) }      # E3001 only at runtime
```

## Current Workaround

None. The error, when it occurs, surfaces at runtime as `E3001`, or is
silently tolerated when the runtime operation happens to succeed on the
unexpected type.

---

## Proposed Syntax

**No new syntax.** Call syntax is unchanged. Only the *checking* of an
existing form is strengthened.

```aura
f(a, b)          # unchanged
```

---

## Proposed Semantics

For a call whose callee is resolved statically to a top-level `fn`:

1. **Arity is checked.** The number of positional arguments MUST equal the
   number of declared parameters. A mismatch is `E3001` **before execution**.
2. **Annotated argument types are checked.** For each parameter that has a
   type annotation, the corresponding argument's inferred type MUST be
   compatible with it under `Ty::compatible_with` (§6.3). A provable mismatch
   is `E3001` **before execution**.
3. **Only provable mismatches are rejected.** If an argument's inferred type is
   `Unknown`, no check is performed for that argument (the frozen §2.3 rule).
4. **Unannotated parameters impose no type constraint.** Only arity applies to
   them.
5. **Calls that are not statically resolvable remain runtime-checked.** A call
   through a local binding holding a lambda, or through any non-`Name` callee,
   is unchanged. (Checking those would require function types, which do not
   exist; see *Out of Scope*.)

This is a **semantic extension**: it does not add behavior at runtime and does
not change evaluation; it moves an already-defined error class earlier for
calls the checker can resolve.

---

## Interaction with Existing Types

* Uses `Ty` and `Ty::compatible_with` unchanged.
* `int` and `float` remain **not** annotation-compatible, exactly as for
  bindings and fields. `area(3.0, 4)` where `w: int` is rejected.
* `Unknown` remains compatible with everything, preserving soundness.
* Struct/enum/list/map annotations on parameters are checked with the same
  `compatible_with` recursion already used elsewhere.

## Interaction with Evaluation Order

None. Argument expressions are still evaluated left-to-right at runtime; the
checker only inspects types, which has no runtime effect.

## Interaction with Control Flow

None.

## Interaction with Functions

* Applies to top-level `fn` calls by name.
* Recursion and mutual recursion are unaffected: both the caller and callee
  signatures are known during hoisting.
* `main` keeps its existing rule (`E2011` for parameters).

## Interaction with Closures

Not covered. A call through a variable (`let f = (x) -> ...; f("x")`) is not
statically checked because the checker has no function type for the binding.
This is unchanged and documented as a limitation.

## Interaction with Structs / Enums

None, except that the constructor paths already perform analogous checks. This
feature makes function calls consistent with construction.

## Interaction with Collections

None. Argument types that are lists/maps are checked recursively via
`compatible_with`.

## Interaction with Pipeline

The pipeline desugars `x |> f(a)` to `f(x, a)` at parse time. Because the
desugared call is an ordinary `Expr::Call` with a `Name` callee, the new
checks apply automatically: `x` becomes the first argument and is checked
against the first parameter. This is desirable and consistent with §23.

## Interaction with REPL

A function defined in an earlier submission is present in the session, so its
parameter types are available and calls in later submissions are checked. No
new REPL state is required. A call before the function is defined in the same
submission is still rejected as `E2003` (undefined), unchanged.

## Interaction with Errors

* Reuses `E3001` (type mismatch). No new code.
* Phase moves from runtime to check time for the affected calls.
* Message form, consistent with existing diagnostics:
  * arity: `` `f` expects 2 argument(s), got 1 ``
  * type: `` `f` argument 1 expects `int`, found `string` ``

## Resource / Security Implications

None. The checker gains a small amount of work per call (a map lookup and a
compatibility check); it adds no recursion beyond what `compatible_with`
already does, and it runs inside the existing bounded checker traversal.

## Backward Compatibility

* **Valid programs** (arguments already compatible): unchanged.
* **Programs that ran and were type-correct under their annotations**:
  unchanged.
* **Programs that passed an incompatible argument**: previously either failed
  at runtime or silently ran; now rejected at check time with the same `E3001`
  code. This is the intended effect and must be recorded as a compatibility
  note.
* **Programs relying on a runtime-only failure that is caught earlier** now
  fail at the check phase; the code is unchanged.
* No existing syntax changes meaning. No new ambiguity.

---

## Grammar Impact

**None.** `call_args` is unchanged.

## AST Impact

**None.** `Expr::Call`, `Param`, and `Arg` already exist with what is needed.

## Checker Impact

**MEDIUM.** Two contained changes:

1. **Record parameter annotations per function.** Extend the checker's
   function table so that, alongside the return type, each top-level function
   stores its parameter list as `Vec<Option<Ty>>` (or an equivalent small
   struct). This is populated during the existing annotation-validation pass
   in `hoist`, which already visits every parameter.
2. **Validate calls.** In the `Expr::Call` handling, when the callee is a
   `Name` that resolves to a stored function:
   * check arity (`args.len()` vs parameters);
   * for each parameter with `Some(expected)`, infer the argument and reject
     only if `compatible_with` returns false (i.e. the argument is not
     `Unknown`).

The existing `infer` already computes argument types and already handles the
`Unknown` rule; the builtin path already validates calls the same way, so the
two paths become symmetric.

**Not required:** changes to `Ty`, `compatible_with`, the inference of
expressions, or the `Unknown` boundary.

## Runtime Impact

**None.** The runtime already reports arity and type errors for user
functions; those paths remain as defense in depth. No runtime behavior
changes.

## Standard Library Impact

**None.** Builtins already validate arity and argument classes through the
shared registry, and remain unchanged.

## Documentation Impact

* `docs/LANGUAGE_SPEC.md` §6.2 and §15.7 must change from "call arguments are
  not checked" to the new rule, with the `Unknown` caveat.
* `docs/LANGUAGE_SPEC.md` §26/§34.2 limitations narrow accordingly.
* `docs/contract.md` §6 gains a bullet for user-function call checking.
* `README.md` needs no change beyond what already points to the spec.
* No example changes are required (all existing examples are type-correct).

## Test Plan

**Regression tests** (`tests/regressions.rs`, named for this feature):

* arity too few / too many → `E3001` at check time;
* annotated parameter, wrong type → `E3001` at check time;
* annotated parameter, correct type → runs;
* mixed annotated/unannotated parameters: only annotated ones are checked;
* `Unknown` argument to an annotated parameter → accepted (runtime decides);
* recursive and mutually recursive calls with correct arguments → run;
* pipeline desugaring: `x |> f(a)` checks `x` against parameter 1;
* a call through a lambda binding is **not** statically rejected;
* a call to an undefined name is still `E2003`.

**Property / differential tests** (`tests/property.rs`):

* for generated well-typed calls, checker-accepts implies runtime does not
  fail with a *type* error;
* extend the grammar-directed generator to include calls with annotated
  parameters and assert the cross-layer invariant.

**Boundary tests:**

* zero-parameter function called with an argument;
* a function whose parameters are all unannotated (arity only);
* deeply nested calls remain within the AST limit.

**Full suite:** the existing 152+ tests must continue to pass; a conformance
audit must confirm spec ↔ implementation ↔ tests agreement.

---

## Out of Scope

* Named, default, and variadic arguments.
* Checking calls through lambdas or callable bindings.
* Function types or higher-order signatures in `Ty`.
* Inferring parameter types (annotations only).
* Field-type propagation or branch-join inference (a prerequisite for checking
  *more* call sites, but separable).
* Any change to the grammar, AST, runtime, stdlib, Python bridge, resource
  limits, or error codes.
* A migration tool. The compatibility note is documentation only.

---

## Open Questions

### Q1 — Should arity checking and type checking ship together?

* **Why it matters.** Arity is always knowable for a resolved function and has
  no `Unknown` caveat; type checking depends on annotations.
* **Possible answers.** (a) Together, in one feature. (b) Arity first, types
  second.
* **Recommendation.** Together. They are the same call-site rule, they share
  the signature record, and splitting them would touch the same code twice.
  Both are small and testable.

### Q2 — What should the diagnostic for an argument type mismatch be?

* **Why it matters.** The existing struct-field message (`field 'a' of 'S' is
  'int' but the value is 'string'`) and the builtin message (`'len' argument 1
  expects string, list, map, value, found 'int'`) differ in shape.
* **Possible answers.** (a) Reuse the builtin shape for user functions. (b) Use
  a new shape.
* **Recommendation.** Reuse the builtin shape (a), because it is the closest
  analogue (a named callable with positional parameters) and keeps the
  diagnostic vocabulary consistent. **Not a semantic decision** — wording is
  non-normative — but worth confirming during implementation.

### Q3 — Does checking a call through a variable require a function type?

* **Why it matters.** `let f = (x: ...) -> ...` cannot annotate a lambda's
  parameters, and the checker has no `Ty` for a function.
* **Possible answers.** (a) Leave such calls runtime-only (this design).
  (b) Add a function type (out of scope).
* **Recommendation.** (a). This is a documented limitation, not a defect, and
  adding a function type is a separate, larger feature.

### Q4 — Should any currently-runtime error be removed?

* **Why it matters.** Defense in depth is valuable.
* **Possible answers.** (a) Keep the runtime checks. (b) Remove them.
* **Recommendation.** (a). The runtime checks protect against any path the
  checker does not cover (e.g. calls through bindings) and cost nothing
  observable. Do not remove them.

### Q5 — Is this a breaking change?  **RESOLVED**

* **Resolution.** Static user-function argument checking is classified as a
  **SEMANTIC TIGHTENING WITH SOURCE-COMPATIBILITY IMPACT**.
* It preserves all previously *valid* Aura programs. It may reject at check
  time programs that were previously accepted by the checker but could only
  fail dynamically at runtime (or evaded the failure). Runtime checks remain in
  place, and the change is documented as a compatibility change.
* It is **not** a breaking redesign and does not extend to unrelated type-system
  changes.
* The full classification is in **Compatibility Classification** below and in
  `LANGUAGE_SPEC.md` §6.5.1.

---

## Compatibility Classification

**SEMANTIC TIGHTENING WITH SOURCE-COMPATIBILITY IMPACT.**

This feature does not redesign the language. It tightens an existing rule
("annotations are checked") so that it also applies at directly resolved
function calls, and it moves one class of error from runtime to check time.

| Property | Status |
|---|---|
| Semantic tightening | Yes — an existing rule applies in one more position |
| Source-compatibility impact | Yes — some previously accepted programs are rejected earlier |
| Preserves previously valid programs | Yes — compatible/valid programs are unaffected |
| Earlier diagnostics | Yes — provable mismatches are `E3001` during checking |
| Retained runtime checks | Yes — runtime argument validation is never removed |
| New syntax | No |
| New AST | No |
| New diagnostic codes | No |
| Runtime behavior change | No |
| Evaluation-order change | No |
| Type-system redesign | No |

### What stays valid

A program is **previously valid** when, for every directly resolved call, the
arguments were already compatible with the declared parameters (or the callee
was not directly resolved). Every such program remains valid and produces the
same result. No correctly-annotated program is invalidated.

### What changes acceptance

A program is **previously checker-accepted but dynamically invalid** when it
passed an argument the declaration did not allow, or called a resolved function
with the wrong argument count. Such a program may now be rejected during
checking. This is the intended effect: it was already incorrect under its own
annotations. The diagnostic code is the same `E3001`; only the phase changes.

### Runtime fallback is preserved

Calls that the checker cannot resolve to a specific top-level function — a
function value, a closure, a variable holding a callable, or an `Unknown`
callee — are still validated at runtime. Runtime argument validation is
retained for every call as defense in depth.

### Migration

No source rewrite is required for valid programs. A program that is rejected
under the new rule must have been passing an incompatible argument; the fix is
to correct the argument or the annotation. This is documented in the
specification compatibility note (§6.5.1) and in release notes.

---

## Required Implementation Test Matrix

These tests are **specified here, not implemented in this phase**. They define
the evidence required when Feature 001 is implemented. Each row is a
source → expected-outcome pair; the outcome is the diagnostic code and phase.

### Arity

| Source | Expected |
|---|---|
| `fn f(a, b) { }` + `f()` | `E3001` at **check** |
| `fn f(a, b) { }` + `f(1)` | `E3001` at **check** |
| `fn f(a, b) { }` + `f(1, 2)` | runs |
| `fn f(a, b) { }` + `f(1, 2, 3)` | `E3001` at **check** |

### Annotated parameters

| Source | Expected |
|---|---|
| `fn f(a: int) { }` + `f(1)` | runs |
| `fn f(a: int) { }` + `f("x")` | `E3001` at **check** |
| `fn f(a: float) { }` + `f(1)` | `E3001` at **check** (`int`/`float` not interchangeable) |
| `fn f(a: int) { }` + `f(1.0)` | `E3001` at **check** |

### Mixed annotations

| Source | Expected |
|---|---|
| `fn f(a: int, b) { }` + `f(1, anything)` | runs |
| `fn f(a: int, b) { }` + `f("x", 1)` | `E3001` at **check** (only `a` checked) |
| `fn f(a: int, b) { }` + `f(1)` | `E3001` at **check** (arity) |

### Unknown

| Source | Expected |
|---|---|
| `fn f(a: int) { }` + `f(none)` | checker accepts; runtime decides |
| `fn f(a: int) { }` + `let x = make()` + `f(x)` | checker accepts if `make()` is `Unknown` |
| `fn f(a: [int]) { }` + `f([1, 2])` | runs |
| `fn f(a: [int]) { }` + `f(["x"])` | `E3001` at **check** (element type) |

### Shadowing

| Source | Expected |
|---|---|
| `fn f(x: int) { }` + inside `g`: `let f = (y) -> y` + `f("x")` | **not** statically rejected as a function call; `f` is a callable value |
| `fn f(x: int) { }` + `f("x")` at top level | `E3001` at **check** (resolved to the declaration) |
| A local binding named `f` that is not callable | existing undefined/callable runtime behavior, not the declaration's signature |

### Mutual recursion

| Source | Expected |
|---|---|
| `fn a(x: int) { b("wrong") }` + `fn b(y: string) { }` | `E3001` at **check** (`b` is hoisted) |
| `fn a(x: int) { b(x) }` + `fn b(y: string) { }` | `E3001` at **check** (int vs string) |
| mutually recursive functions with compatible arguments | run |

### Runtime fallback

| Source | Expected |
|---|---|
| `let f = (x) -> x` + `f("x")` | runs (no static signature) |
| `let f = (x) -> x` + `f()` | runtime `E3001` |
| call through `Unknown` | runtime decides |
| call to a non-function value | runtime `E3001` |

### REPL

| Source | Expected |
|---|---|
| submission 1: `fn f(a: int) { return a }`; submission 2: `f("x")` | `E3001` at check of submission 2 |
| submission 1: `fn f(a: int) { return a }`; submission 2: `f(1)` | prints the result |
| a shadowing `let f` in a later submission makes subsequent `f(...)` a callable-value call | not checked as the declaration |

### Pipeline

| Source | Expected |
|---|---|
| `fn add(a, b) -> int { return a + b }` + `1 \|> add(2)` | runs (`x` checked as argument 1) |
| `fn add(a: int, b: int) -> int { return a + b }` + `"x" \|> add(2)` | `E3001` at **check** (first arg is `"x"`) |

### Property / differential

* Extend the grammar-directed generator to emit calls with annotated
  parameters, and assert the cross-layer invariant: if the checker accepts a
  generated call, the runtime does not fail with a *type* error.
* Assert that the set of programs rejected only by the new rule is exactly the
  set whose arguments were provably incompatible.

---

## Implementation Contract

The future implementation must satisfy exactly this invariant:

```
If the checker resolves a call to a specific user-defined function:
    validate arity;
    validate annotated parameter types when provable.

Otherwise:
    preserve existing dynamic/runtime behavior.

Runtime validation remains active in all cases.
```

Concretely:

* **Resolve first.** A call is "resolved" only when its callee is a `Name`
  that denotes a top-level `fn` declaration and that name is not shadowed by a
  local binding in scope.
* **Arity always applies** to a resolved call.
* **Type checks apply only when provable:** for a parameter with an
  annotation `T`, reject only if the argument's inferred `Ty` is known and not
  `compatible_with(T)`. `Unknown` arguments are always accepted.
* **Never apply a declaration's signature to a shadowed name.**
* **Do not touch:** `Ty`, `compatible_with`, evaluation order, the runtime,
  the stdlib, the grammar, the AST, resource limits, or error codes.
* **Keep runtime validation** for every call.

Non-conforming implementations include: naive global-name lookup that ignores
shadowing; rejecting `Unknown` arguments; removing runtime checks; or extending
the check to function values/closures.

---

## Proposed Specification Changes

The specification amendment has been **drafted in this phase** (spec-first) and
is already present in `LANGUAGE_SPEC.md`. The implementation phase adds the
code and the tests; the normative text is frozen now.

Specification edits made:

1. **§6.2 "When annotations are enforced"** — the sentence that said call
   arguments are not checked now states that parameter annotations are also
   enforced at directly resolved calls, pointing to §6.5.
2. **§6.5 "Argument checking at directly resolved calls"** (new) — states the
   arity rule, the annotated-parameter rule, the `Unknown` skip, the
   unresolved-callee carve-out, the shadowing rule, the hoisting/mutual-
   recursion rule, and the retained runtime checks. Includes §6.5.1, the
   compatibility note.
3. **§15.7 "Arity and arguments"** — now maps arity/type errors to `E3001` at
   check time for directly resolved calls and at runtime otherwise.
4. **§26 "Declarations and Symbol Visibility"** — notes that hoisting makes
   parameter signatures available regardless of declaration order.
5. **§34.2 "Static-checking limitations"** — the first bullet now scopes the
   limitation to calls through a function value/closure/variable/unknown callee.

Still to edit at implementation time (documentation, not semantics):

6. **`docs/contract.md` §6** — add a bullet: "A call to a top-level function is
   checked against its declared parameter count and annotated parameter types
   (`E3001`), subject to the `Unknown` rule."
7. **`docs/grammar.md`** — no change (grammar is unchanged).

No new diagnostic codes. No change to the value model, operators, evaluation
order, or resource limits.

---

## Definition of Done (for this feature)

* The specification amendment is present normatively (drafted in this phase).
* `docs/contract.md` carries the matching bullet.
* Parameter annotations are recorded and call sites validated per the
  Implementation Contract.
* The Required Implementation Test Matrix passes at check time where expected.
* Regression, boundary, property, and differential tests pass.
* The full verification suite passes.
* A conformance audit confirms `LANGUAGE_SPEC` = implementation = tested
  behavior for the new rule.
* The compatibility classification (§6.5.1) is recorded in the release notes.
