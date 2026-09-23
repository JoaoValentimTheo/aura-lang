# Feature 003 — Static Field-Type Propagation

## Status

Design (pre-implementation). No code, tests, grammar, or specification have
been changed. This document is the deliverable.

**Baseline:** `39aacab` (`rewrite/v3-rust`).
**Semantic authority:** `docs/LANGUAGE_SPEC.md` (frozen).

---

## Motivation

Aura's contract says that type annotations, when present, are checked. Since
Feature 001, a function's declared return type flows into the checker, and
since Feature 002, arguments are mapped and checked against annotated
parameters. But a **field read** on a struct value still infers `Unknown`, so
annotations that depend on a field are not checked:

```aura
struct Point { x: int, y: int }

let p = Point { x: 3, y: 4 }
let bad: string = p.x        # accepted today; p.x is int
```

This is the most visible remaining conservatism in the checker, and the
roadmap records it as a documented limitation (`LANGUAGE_SPEC.md` §34.2,
"Field reads infer `Unknown`"). It also weakens Feature 001 and Feature 002:
an argument or annotation whose value is a field read cannot be validated.

Feature 003 makes the checker propagate a struct field's **declared** type out
of a field read when the receiver's type is statically known. It extends the
existing conservative rule; it does not build a type system or an inference
engine.

---

## Current Language Behavior

* Structs are nominal: `struct S { f: T, ... }` records each field's declared
  type (`Checker::struct_fields: HashMap<String, HashMap<String, Ty>>`).
* `let` records an inferred type for the binding: an annotated `let` stores the
  annotation; an unannotated `let` stores the inferred type when it is not
  `Unknown` (`Checker::value_types`). So `let p = Point { .. }` records
  `p : Named("Point")`.
* `Checker::infer(Expr::Name)` returns that recorded type.
* `Checker::infer(Expr::Field(..))` returns `Unknown` unconditionally, and its
  `Expr::Field` validation only checks the receiver.
* Consequently `let bad: string = p.x` and `f(p.x)` (with `f(a: string)`) are
  accepted; at runtime `p.x` is an `int`.

**Observed today** (`39aacab`), all accepted by the checker:

```aura
struct P { x: int }
fn main() { let p = P { x: 1 }
            let y: string = p.x }        # accepted, runs; y is 1
```

```aura
struct P { x: int }
fn f(a: string) { return a }
fn main() { let p = P { x: 1 }
            f(p.x) }                     # accepted, runs
```

---

## Proposed Syntax

**None.** Feature 003 adds no syntax. Field access `receiver.field` is
unchanged, and no new token, operator, or production is introduced.

This is a deliberate property: the feature is a checker precision improvement,
not a language-surface change.

---

## Semantic Model

**Normative intent.** When a field access `receiver.name` is checked on a
receiver whose inferred type is a **known nominal struct type** `Ty::Named(n)`
and `n` declares a field `name`, the inferred type of the field access is the
**declared type of that field**.

The rule is conservative and total:

* If the receiver's inferred type is `Ty::Named(n)` and `n` is a declared
  struct with a field `name`, infer that field's declared `Ty`.
* If the receiver's inferred type is `Ty::Named(n)` but `n` is an enum, or has
  no field `name`, the receiver is not a struct with that field; inference
  yields `Unknown` (the existing `Expr::Field` validation, unchanged, reports
  unknown fields on a known struct at check time — see Error Model).
* If the receiver's inferred type is anything else (a primitive, list, map,
  `Unknown`, `Enum`, or a non-struct), inference yields `Unknown`, exactly as
  today.

Nothing else changes. In particular, a field read never *reads* anything: it
only consults the declared field table, exactly as struct construction already
does.

### What this composes with

Because the field's declared type becomes the inferred type of the field read,
it flows into everything that already uses `Ty::compatible_with` or the
`Unknown` boundary:

* annotated `let` bindings (`let y: int = p.x`);
* reassignment to an annotated binding (`y = p.x`);
* `return` against a declared return type;
* struct field values at construction (`Q { s: p.x }`);
* enum payload values at construction;
* function-call arguments (Feature 001/002, including named and pipeline
  arguments);
* ordering checks (`p.x < 3`), which already use `orderable_with`.

This is the whole point: one local inference change reaches every existing
check.

---

## Scope

### IN SCOPE

* Infer a struct field's declared type for `receiver.field` when the receiver's
  inferred type is a known struct with that field.
* Let that inferred type flow through the existing compatibility, ordering,
  call, construction, and return checks.
* A regression and property-test surface for these cases.
* A normative specification amendment and a documentation update of the
  §34.2 limitation.

## Out of Scope

* **Field reads on `Unknown` receivers.** If the receiver is `Unknown`, the
  result stays `Unknown`. No new inference source is invented.
* **Field reads on non-struct receivers** (primitive/list/map/enum). Unchanged;
  result stays `Unknown` (the runtime still reports invalid field access).
* **Indexing** (`receiver[index]`) type propagation. A list index yields the
  element type only if the list type carries it, which the checker does not
  track with precision; explicitly out of scope.
* **Map value-type propagation** (`m[k]` / `m.get(k)`). Out of scope.
* **`if`/`match`/block/lambda inference.** These still infer `Unknown`; making
  them infer a joined type is the separate "branch-join inference" item.
* **A static `none` type.** `none` still infers `Unknown`.
* **Method return-type changes.** Methods already declare return types; no
  change.
* **User-defined methods, modules, generics, traits, async.** Not related.
* **Any grammar, AST, runtime, stdlib, Python, or error-code change.**

---

## Resolution / Typing

Enums, structs, aliases, and `Unknown` interact as follows.

* **Struct receiver.** `receiver : Named(S)` and `S` has field `f : T` ⇒ the
  field access infers `T`, where `T` is the **resolved** declared type (an
  alias in the field's annotation is already resolved by `struct_fields`, which
  stores the alias-resolved `Ty`).
* **Enum receiver.** A value whose inferred type is `Ty::Enum(_)` has no
  fields; the field read infers `Unknown`. (The checker does not, and will not,
  report "enums have no fields" at check time here; the runtime does.)
* **Alias receiver.** A transparent alias to a struct resolves to `Named(S)`
  through annotation resolution, so a value annotated with the alias is a
  struct to this rule.
* **`Unknown` receiver.** Stays `Unknown`.
* **`Ty::Named(n)` where `n` is not a declared struct** (e.g. the builtin
  `range`): no `struct_fields` entry for the field ⇒ `Unknown`.
* **Existing unknown-field diagnostic.** The `Expr::Field` check already
  reports `E2003` when the receiver is a known struct and the field does not
  exist (assignment path) — and the runtime reports it for reads. Feature 003
  does not change this; it only supplies a type when the field *does* exist.

**Nested access.** `p.q.s` works by composition: `infer(p)` is `Named("P")`;
`p.q` infers the declared type of `q` (say `Named("Q")`); `p.q.s` infers the
declared type of `s`. No special handling is needed.

---

## Evaluation Order

**No change.** Field access has no side effects in Aura; `infer` is a pure
compile-time function. The feature does not evaluate anything and does not
reorder anything.

* Evaluation order remains strictly source order (`LANGUAGE_SPEC.md` §13).
* No expression is evaluated by the checker.
* The runtime never changes: `field_get` still reads the value; the checker
  only *predicts* its type.

---

## Runtime Semantics

**No change.** The runtime is unaffected in every respect:

* `Value::Instance` still stores `(name, value)` fields and `field_get`
  returns the stored value.
* Field assignment and mutation are unchanged.
* Runtime validation remains as the final layer (a field read on a non-struct
  or a missing field is still a runtime `E2003`).

The feature is **checker-only**. The runtime cannot become stricter or more
lenient because of it.

---

## Error Model

**No new error code.** The feature does not introduce a new rejection
category; it makes an existing check (`E3001` via `compatible_with`) apply to a
value it previously could not see.

| Situation | Code | Phase | Change |
|---|---|---|---|
| field read's declared type is incompatible with an annotation | `E3001` | check | **newly diagnosed** (was accepted) |
| field read's declared type is incompatible with a parameter | `E3001` | check | **newly diagnosed** (Feature 001 path) |
| field read's declared type is incompatible at construction | `E3001` | check | newly diagnosed |
| field read compatible | — | check | unchanged (accepted) |
| field read on `Unknown` receiver | — | check | unchanged (`Unknown`) |
| field read on a non-struct / missing field | `E2003`/`E3001` | run (and check for known structs) | unchanged |
| alias-typed field value | — | check | unchanged |

Messages reuse the existing shapes, for example
`` `y` is annotated as `string` but its value is `int` `` and
`` `f` argument 1 expects `string`, found `int` ``. Spans point at the
annotated binding or the call, as today.

Because this is a **semantic tightening**, the compatibility note mirrors
Feature 001's: previously valid programs remain valid; a program that relies on
a field read being `Unknown` at a provably incompatible site may now be
rejected at check time with the same `E3001` it would have produced (or
evaded) at runtime.

---

## REPL

**No new session state is required.** The REPL's checker already records
`value_types` per submission and rebuilds struct tables from the persistent
declarations; a field read on a struct declared in an earlier submission
benefits automatically.

* A struct declared in submission 1 exposes its field types to a later
  submission's field reads, exactly as it already exposes them to
  construction.
* No `GlobalDecl` change is needed: struct field types already persist
  (`GlobalDecl::Struct { fields }`).
* A failed submission does not corrupt the session (unchanged).

If a value's type is not known in the session, the field read stays `Unknown`
and the REPL accepts, preserving the conservative rule.

---

## Pipeline

**No change, but it composes.** The pipeline desugars `x |> f(a)` to
`f(x, a)`, an ordinary call, so a field read as the piped value or as an
argument is checked by the same call path:

```aura
struct P { x: int }
fn f(a: string) { return a }
let p = P { x: 1 }
p |> f            # now rejected: p.x is not involved; p itself is Named(P)
(f(p.x))          # rejected: p.x is int, f wants string
```

No pipeline-specific inference is introduced; the existing call check simply
now has a type to work with. (Note: `p |> f` passes the *struct* `p` to `f`,
which is itself a `Named(P)` vs `string` mismatch once Feature 001/002 checks
the argument — that behavior already exists and is not caused by Feature 003.)

---

## Resource Limits

**No new resource semantics. No limit changes.**

* **AST node limit (`E1015`, 256):** unchanged. The feature adds no AST nodes
  and no new recursion; `infer` still recurses over the existing tree bounded
  by the parser's `MAX_AST_DEPTH` enforcement.
* **Nesting limit (`E1015`):** unchanged.
* **Call-frame limit (`E4011`, 512):** unchanged; the runtime is untouched.
* **Range materialization (10,000,000) and other limits:** unchanged.

The only added work per field read is a hash lookup; it introduces no
unbounded recursion and no new allocation visible to users.

---

## Backward Compatibility

**Class: semantic tightening with source-compatibility impact** (the same
class as Feature 001).

* **Previously valid programs** — a field read used where its declared type is
  compatible, or where the expected type is `Unknown` — remain valid and
  behave identically.
* **Previously checker-accepted but dynamically invalid programs** — a field
  read used at a provably incompatible annotated site — may now be rejected
  during checking with the same `E3001` code. This is the intended effect.
* **No syntax change, no AST change, no runtime change, no new error code.**
* Every program that ran and was type-correct under its annotations is
  unaffected.

---

## Interaction with Features 001 and 002

* **Feature 001 (static argument checking).** Field reads previously supplied
  `Unknown` as call arguments, so argument checks were skipped for them.
  Feature 003 gives the checker a concrete argument type, so those checks now
  apply. This strengthens Feature 001 without changing its rule.
* **Feature 002 (named arguments).** A named argument whose value is a field
  read is type-checked the same way after parameter mapping; `bind_arguments`
  is unaffected (it binds already-evaluated values and does not use types).
  Evaluation order and single-evaluation are unaffected.
* **No conflict.** The feature is a pure addition to `infer(Expr::Field)`; it
  changes no existing rule and no existing diagnostic meaning.

---

## Proposed LANGUAGE_SPEC Amendment

The following is the precise normative text to insert at implementation time.
It is **not** applied in this phase.

### §17.5 (Structs — Display and mutation) — replace the limitation sentence

**Current:**

> **Normative rule.** Struct field types are validated at construction; the
> checker does not track the static type of a field read (`s.field` infers
> `Unknown`).

**Proposed:**

> **Normative rule.** Struct field types are validated at construction. A field
> read `s.field` on a receiver whose inferred type is a declared struct `S`
> that has a field `field` infers the declared type of that field. A field read
> on any other receiver (an `Unknown` type, a primitive, a list, a map, an
> enum, or a struct without that field) infers `Unknown`. This inferred type
> participates in every existing check that uses compatibility or ordering
> (§6.3, §6.5, §9, §15.7). The runtime is unchanged and remains authoritative
> for field access.

### §34.2 (Static-checking limitations) — narrow the field bullet

**Current:**

> * **Field reads infer `Unknown`**; the checker does not propagate a field's
>   declared type out of `s.field` (§17.5).

**Proposed:**

> * **Field reads infer a declared type only when the receiver's type is a
>   known struct.** A field read on a value whose type the checker cannot
>   determine (for example a parameter without an annotation, an `if`
>   expression, a call whose return type is not declared, or an element of a
>   list) still infers `Unknown` (§17.5), as do indexed reads and map lookups.

### §33 (Frozen Design Decisions) — add

> **Struct field reads propagate declared types.** When the receiver's
> inferred type is a known struct, `s.field` infers the field's declared type;
> otherwise it infers `Unknown`. This never evaluates anything and never
> changes evaluation order.

No new diagnostic code.

---

## Implementation Plan

The change is localized to the checker.

* **`src/check/mod.rs` — `infer`'s `Expr::Field` arm.** Replace the
  `Unknown` result with a lookup:

  ```
  Expr::Field(recv, name, _) => {
      if let Ty::Named(sname) = self.infer(recv) {
          if let Some(fty) = self.struct_fields.get(&sname).and_then(|m| m.get(name)) {
              return fty.clone();
          }
      }
      Ty::Unknown
  }
  ```

  `struct_fields` already stores alias-resolved declared types, so no
  resolution work is needed here. The existing `Expr::Field` validation (the
  unknown-field diagnostic for known structs) is unchanged.

* **`docs/LANGUAGE_SPEC.md`** — apply the amendment above.
* **`docs/contract.md`** — add a bullet under "Beyond annotations …" noting
  that a field read on a known struct carries the field's declared type, and
  that this is now checked at call sites and annotated bindings.
* **`tests/regressions.rs`** — the test matrix below.
* **`tests/property.rs`** — the property below.
* **`docs/FEATURE_003_IMPLEMENTATION_REPORT.md`** — written at implementation
  time.

**Explicitly not changed:** `src/ast`, `src/parse`, `src/run`, `src/stdlib`,
`src/repl`, `src/bridge`, `Cargo.toml`. No grammar, AST, runtime, stdlib,
Python, or resource-limit change.

**Performance.** `infer` is already called throughout checking; the added
work is at most three hash lookups per field read (`struct_fields` outer, inner,
then a clone). No whole-program analysis, no fixpoint, no growth.

---

## Test Matrix

Each row is source → expected. "Accepted" = the checker accepts and the program
runs; "E3001" = rejected at check time.

### Basic valid propagation

| Source | Expected |
|---|---|
| `struct P { x: int }` + `let p = P { x: 1 }` + `let y: int = p.x` | accepted, prints `1` |
| `let y: float = p.fx` where `fx: float` | accepted |
| `let y: string = p.s` where `s: string` | accepted |
| `let y: bool = p.b` where `b: bool` | accepted |
| `let y: [int] = p.xs` where `xs: [int]` | accepted |
| `let y: {string: int} = p.m` where `m: {string: int}` | accepted |

### Newly diagnosed mismatches

| Source | Expected |
|---|---|
| `let y: string = p.x` where `x: int` | `E3001` at check |
| `let y: int = p.s` where `s: string` | `E3001` at check |
| `p.x` passed to `fn f(a: string)` | `E3001` at check (Feature 001 path) |
| `p.x` passed to named arg `f(a: string)` (Feature 002) | `E3001` at check |
| `Q { s: p.x }` where `Q.s: string`, `p.x: int` | `E3001` at check |
| `A(p.x)` variant payload where `A(string)` | `E3001` at check |
| `fn g() -> string { return p.x }` | `E3005` at check (return path) |
| `if p.x < "s" { }` (ordering with wrong types) | `E3001` at check via `orderable_with` |

### Nested and aliased

| Source | Expected |
|---|---|
| `p.q.s` with `p.q: Q`, `Q.s: int`, `let y: int = p.q.s` | accepted |
| `let y: string = p.q.s` (`s: int`) | `E3001` |
| alias to struct used as receiver | declared field type propagates |
| field whose annotation is an alias (`type Id = int`, `struct S { id: Id }`) | `p.id` infers `int` |

### Conservatism (must stay `Unknown` / accepted)

| Source | Expected |
|---|---|
| `let p = unknown_param` (unannotated parameter) + `let y: int = p.x` | accepted (receiver `Unknown`) |
| field read on a list element: `let y: int = [P { x: 1 }][0].x` | accepted (index infers `Unknown`) |
| field read on a map value: `let y: int = m["k"].x` | accepted |
| field read on an `if` result: `let p = if c { P { x: 1 } } else { P { x: 2 } }` + `let y: string = p.x` | accepted (receiver `Unknown`) |
| field read on a function return without a declared type | accepted |
| field read on a runtime struct accessed through a closure/global of unknown type | accepted |

### Existing behavior unchanged

| Source | Expected |
|---|---|
| unknown field read `S { a: 1 }.b` | runtime `E2003` (unchanged) |
| unknown field assignment `s.b = 1` on known struct | `E2003` at check (unchanged) |
| positional/named construction validation | unchanged |
| primitive/list/map field access | runtime error (unchanged) |

### Shadowing, hoisting, recursion

| Source | Expected |
|---|---|
| struct declared after the field read (hoisted) | propagates (types are collected in the pre-pass) |
| field read inside a recursive function | propagates |
| locally shadowed variable of a different struct type | uses the local type |

### REPL

| Session | Expected |
|---|---|
| submission 1: `struct P { x: int }`; submission 2: `let p = P { x: 1 }`; submission 3: `let y: string = p.x` | `E3001` at check of submission 3 |
| submission 2 (valid): `let y: int = p.x` | prints the value |
| field read on a struct declared in an earlier submission | propagates |

### Pipeline

| Source | Expected |
|---|---|
| `fn f(a: int) { }` + `p.x \|> f` | accepted (`p.x: int`) |
| `fn f(a: string) { }` + `p.x \|> f` | `E3001` |

---

## Property / Differential Testing

Invariants to encode as property/differential tests:

1. **Compatibility of the inferred type.** For a struct `S` with field `f: T`,
   for every `S` value bound to a name, `infer(name.f)` equals the resolved
   `T` — i.e. a field read infers exactly the declared field type, never
   anything else.
2. **No false positives.** A field read used where its declared type is
   compatible (including `Unknown` expected types and `Unknown` receivers) is
   never rejected.
3. **Checker/runtime agreement.** If the checker accepts a program that reads a
   known field, the runtime read does not fail; if the checker infers `T` and
   `T` is used compatibly, the run does not produce a type error at that use.
4. **Additivity.** For programs with no field reads at an annotated site, the
   static and runtime verdicts are identical to the pre-feature baseline
   (generator-driven differential test over the existing expression grammar,
   extended with struct field reads).

These compose with the existing `checker_and_evaluator_agree` property.

---

## Regression Risks

Localized, with mitigation:

* **`infer` is used by many checks.** A wrong field type could cause false
  rejections. Mitigation: the rule returns the *declared* type only; the
  declared type is authoritative (construction validates values against it),
  so a correctly constructed struct's field genuinely has that type. If a
  mismatch is reported, the program was already incorrect under its own
  declarations.
* **Alias resolution.** `struct_fields` stores alias-resolved `Ty`, so no
  extra resolution is needed; verify with an alias-typed field test.
* **Enums and non-struct `Named` (e.g. `range`).** The lookup is keyed by
  `struct_fields`, which contains only structs, so enums and `range` yield
  `Unknown`. Verified by conservatism tests.
* **Existing tests.** Full suite must stay green; no existing test weakens.
* **`Expr::Field` conflates field read and no-paren method call.** On a
  non-struct receiver the evaluator treats `.name` as a zero-arg method call;
  the new inference only fires for `Ty::Named(struct)`, so method-style field
  access is unaffected (its receiver is not `Named`).

---

## Future Extensions

* **Indexed reads** (`list[i]`, `map[k]`) type propagation — needs element/
  value type tracking through `infer`.
* **Branch-join inference** for `if`/`match`/block — would give many more
  field reads a known receiver type.
* **A static `none` type** — would let `T | none` be modelled precisely.
* **Method return types on user types** — belongs with user-defined methods.

None is part of Feature 003.

---

## Design Review

* **Is every semantic decision explicit?** Yes: the rule, its conservatism
  cases (non-struct, enum, `Unknown`, missing field), and its composition are
  stated.
* **Contradicts `LANGUAGE_SPEC.md`?** No. It replaces the documented
  "field reads infer `Unknown`" limitation with a narrower, precise rule, and
  the amendment makes the specification match.
* **Requires an architectural refactor?** No. One arm of `infer`.
* **Incrementally implementable?** Yes: one arm plus tests.
* **Hidden interactions with Feature 001/002?** None beyond strengthening
  their existing checks; both are unaffected structurally.
* **Ambiguous example?** `p.q.s`, aliases, `Unknown` receivers, enum receivers,
  and indexed reads are all resolved above.
* **Errors deterministic?** Yes: `infer` is deterministic; the first
  diagnostic policy is unchanged.
* **Evaluation order explicit?** Yes: unaffected; no evaluation added.
* **Resource limits preserved?** Yes: no new limits, no growth, bounded
  recursion.
* **REPL behavior explicit?** Yes: no new state; existing struct persistence
  suffices.

---

## Open Questions

None. The feature is a single, conservative extension of an existing rule,
resolved entirely from the frozen specification, the existing `struct_fields`
and `value_types` infrastructure, and the established `compatible_with`/
`Unknown` semantics. There is no syntax decision, no ambiguity, and no product
choice to make.

---

## Final Design Decision

**Feature 003 is Static Field-Type Propagation**, a checker-only semantic
tightening with source-compatibility impact, localized to `infer`'s field
arm, reusing the existing struct-field table and compatibility relation, with
no grammar, AST, runtime, stdlib, REPL, Python, resource-limit, or error-code
change.

**READY FOR IMPLEMENTATION.**
