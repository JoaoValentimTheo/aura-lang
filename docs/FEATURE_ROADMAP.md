# Aura Post-Freeze Feature Roadmap

This document plans the evolution of Aura after the semantic freeze. It is a
**design** document, not a specification change and not an implementation.

The semantic foundation is frozen at commit `3b83b2a`:

```
Architecture Hardening   → d940fbe
Semantic Closure         → aba8866
Specification Freeze     → cbd2dc9
Conformance Audit        → 0385456
Final Semantic Red Team  → 3b83b2a
```

`docs/LANGUAGE_SPEC.md` is the single semantic authority. Every proposal here
must be designed against it before any code is written.

---

## Post-Freeze Development Philosophy

1. **Spec first.** A feature is designed, specified, and reviewed before it is
   implemented. The specification is updated in the same change that changes
   observable behavior.
2. **Preserve the character.** Aura is small, expression-oriented,
   dynamically typed, conservatively checked, and interpreted. A feature that
   requires abandoning one of those traits is a language-version redesign, not
   a feature.
3. **One spelling per construct.** A synonym is a defect. New syntax must earn
   its place.
4. **No silent divergence.** The checker and runtime must never contradict each
   other. `Ty::Unknown` stays permissive; a feature must not turn "cannot
   prove" into a false rejection.
5. **Preserve the frozen invariants.** One semantic authority, checker/runtime
   agreement, deterministic evaluation, explicit control flow, resource
   safety, REPL consistency, boundary integrity, stable diagnostics.
6. **Smallest coherent step.** Prefer a feature that closes one real gap
   cleanly over a cluster that touches every subsystem.
7. **Architecture debt is not a feature.** Unrelated cleanup does not ride
   along with a feature; it is scheduled on its own.

---

## Current Language State

Aura today is a tree-walking interpreted language with a conservative checker.

| Area | State |
|---|---|
| Lexing | ASCII identifiers; line comments; newline/semicolon statements; decimal/hex/binary/octal ints, floats, string/char quotes, f-strings |
| Grammar | Recursive-descent + Pratt; right-associative `^`; left-associative everything else; explicit precedence table |
| Value model | `int`, `float`, `string`, `bool`, `none`, `list`, `map` (string-keyed, ordered), struct instance, enum variant, function, range |
| Type model | Checker `Ty`: `Int`, `Float`, `Bool`, `String`, `List`, `Map`, `Named`, `Enum`, `Unknown` |
| Unknown | Exact conservative boundary: the checker rejects only what it can prove |
| Operators | `+ - * / % ^ == != < <= > >= and or`, unary `-`/`not`, pipeline `\|>`, assignment `= += -= *= /=` |
| Evaluation order | Strict left-to-right; `and`/`or` short-circuit; documented |
| Control flow | `if`/`else if`/`else`, `while`, `loop`, `for`, `break`/`continue`, `return`, `throw`, `try`/`catch`/`finally` |
| Functions | Top-level `fn` with hoisting and mutual recursion; positional and named calls; static argument checking |
| Closures | Lambdas capture by reference; no ownership/lifetime model |
| Mutability | Binding-level (`let`/`let mut`); value- and field-level mutation through shared references |
| Structs | Nominal; named and positional construction; validated fields |
| Enums | Global tags; positional payloads; named payload args rejected |
| Match | Expression; literal/binding/list/variant patterns; guards; no exhaustiveness |
| Collections | Lists (index/mutate/iterate) and string-keyed ordered maps; empty map `{:}` |
| Destructuring | `let [a, b] = e` and `let Ok(x) = e`, reusing the pattern system |
| Field types | A field read on a known struct infers the declared type (Feature 003) |
| Scripting I/O | `read_line`, `read_file`, `write_file`, `args` (0.0.1) |
| Comments | Line comments (`#`); no block comments |
| Tuples | `(a, b)` is list sugar; no distinct tuple type |
| Loops/ranges | `range(a, b)` step 1, end-exclusive, lazy in `for` |
| Pipeline | `x \|> f(a)` is `f(x, a)` |
| Methods | Built-in receiver kinds only (string/list/map/range); structs/enums have none |
| Built-ins | Core builtins (incl. `read_line`, `read_file`, `write_file`, `args`) + method entries in one shared registry |
| Aliases | Transparent; chained; recursive aliases rejected (`E3002`) |
| REPL | Persistent session; bindings/functions/structs/enums/aliases; line-oriented submissions |
| CLI/API | `run`, `check`, `eval`, `repl` over one `compile`+`execute` pipeline |
| Python | Optional `py` bridge; lossless integer/dict-key rules |
| Resource limits | 256 AST nodes (`E1015`), 512 call frames (`E4011`), 10M range cap |
| Error model | Stable `E####` codes by phase; deterministic first diagnostic |

### Core strengths to preserve

* One pipeline and one signature registry: the checker and runtime cannot
  drift on the standard library.
* A precise `Unknown` boundary with no false positives.
* Deterministic left-to-right evaluation.
* A complete, executable, machine-checked specification.

### Current limitations (documented in `LANGUAGE_SPEC.md` §34)

* `if`/`match`/block expressions and lambdas infer `Unknown`.
* No default or variadic function arguments.
* No tuple type (list sugar).
* `try` requires `catch`.
* `match` arm bodies that are bare control flow keywords need a block.
* No nested named function declarations.
* No range step or `for…else`.
* No block comments.
* No tuple type or lexicographic ordering for compound values.
* No closure lifetime/ownership model (deliberate).

### Completed milestones

Feature 001 (static argument checking), Feature 002 (named arguments),
Feature 003 (field-type propagation), Feature 004 (destructuring `let`),
Feature 005 (empty-map literal), Feature 006 (`else if`), and the H1 pattern
correctness/safety hardening are implemented. The 0.0.1 release hardening adds
the scripting I/O and argument surface.

---

## Architecture Debt

Internal cleanup that does **not** change observable semantics. None of these
is a prerequisite for the first feature.

| Debt | Kind | Causes observable divergence? |
|---|---|---|
| `json_*` bypass the shared `arity()` helper | internal inconsistency | **fixed (0.0.1 hardening)** — arity now enforced centrally at native dispatch, including first-class builtin values |
| `stdlib::arity` retains a min/max fallback | duplicate logic | No |
| `Tok::As` lexed but unconsumed | dead token | No |
| `E5003` defined but never produced | dead constant | No |
| `E4099` internal throw signal in the public code space | naming | No |
| `Expr::Tuple` lowers to a list | AST surface | No (documented) |
| `Expr::Field` conflates field read and no-paren method call | structural | No (checker covers known receivers) |

These are recorded so they are not silently entangled with feature work. A
feature may be scheduled before or after them independently.

---

## Current Limitations Classification

Classification keys:

* **A** — likely worth addressing soon
* **B** — useful but not urgent
* **C** — intentional language limitation
* **D** — internal cleanup only
* **E** — requires fundamental language redesign

| Limitation / Debt | Class | Rationale |
|---|---|---|
| No static user-function argument checking | **A** | Concrete correctness gap; infrastructure already half-present; no new syntax |
| Field reads infer `Unknown` | **B** | Blocks some static checks downstream; needs field-type propagation, medium scope |
| `if`/`match`/lambda infer `Unknown` | **B** | Limits static checking; branch-join inference is a real design step |
| No named function arguments | **B** | Ergonomics; reuses `Arg` AST; no correctness gap |
| No default arguments | **B** | Ergonomics; interacts with arity and calls |
| No variadic arguments | **C/B** | Rarely needed; complicates arity and the registry |
| No empty-map literal | **B** | Real expressiveness gap; needs a syntax decision |
| No tuple type | **C** | Deliberate; list sugar is coherent for this language |
| No range step | **B** | Expressible with `for` + arithmetic or a stdlib helper |
| No lexicographic ordering | **C** | Deliberate; ordering is a scalar relation here |
| No nested named functions | **C** | Lambdas cover the use case |
| No `else if` | **C** | Deliberate "one spelling" decision; `match`/nesting cover it |
| `match` bare control-flow needs a block | **C** | Deliberate grammar decision |
| No closure lifetime/ownership model | **C** | Deliberate; reference-counted environments are the model |
| Inert `pub`/`use` | **C (until modules)** | Intentional placeholder; becomes real only with a module system |
| `up`/`down` aliases | **done (0.0.1 hardening)** | Removed |
| `json_*` arity bypass | **D** | Internal |
| `stdlib::arity` fallback | **D** | Internal |
| `Tok::As` | **D** | Dead token |
| `E5003` | **D** | Dead constant |
| `E4099` | **D** | Internal signal |
| `Expr::Tuple` lowering | **D** | Documented |
| `Field`/`Unknown` architecture | **D** | Covered by the checker |
| Modules / imports | **E** | Requires a package model, visibility, initialization order, and a REPL story |
| Generics / traits | **E** | Outside the current monomorphic `Ty`; a redesign |
| Async / concurrency | **E** | Requires a different runtime |
| Bytecode VM / codegen | **E** | Out of scope by design |

No class **E** item is proposed in this roadmap. They are recorded so that
"missing feature" is not confused with "next feature".

---

## Candidate Feature Families

1. **Function ergonomics** — static argument checking; named arguments;
   default arguments; variadic arguments.
2. **Collection ergonomics** — empty-map literal; list/map helpers; iteration
   conveniences.
3. **Pattern / destructuring** — destructuring `let`; destructuring
   parameters; richer slice patterns.
4. **Type-system improvements** — field-type propagation; branch-join
   inference; a static `none` type; function types.
5. **Method / abstraction** — user-defined methods on structs/enums.
6. **Modules** — real `pub`/`use`.
7. **Control-flow ergonomics** — `else if`; expression conditionals; match
   improvements.
8. **Range ergonomics** — step; reverse; range helpers.
9. **Error-handling ergonomics** — result helpers; catch-all ergonomics.
10. **Interoperability / stdlib** — more builtins; Python conveniences.
11. **Tooling** — formatter, LSP, package manager.

---

## Detailed Candidate Analysis

Each candidate answers the same questions: user value; new semantics; what it
could disturb; whether it forces type-system or evaluation-order change;
checker/runtime divergence risk; resource-safety risk; REPL and Python
effects; and whether it redesigns a frozen concept.

### 1. Static user-function argument checking — **Class A**

* **User value.** High. Closes the most concrete correctness gap: `f("x")`
  against `fn f(a: int)` currently runs silently; `f()`/`f(1,2)` against a
  one-parameter function fail only at runtime. This makes "annotations are
  checked" true for calls, consistent with struct and enum construction.
* **New semantics.** None at the value level. It extends *where* an existing
  annotation is enforced, under the existing `compatible_with` and `Unknown`
  rules.
* **Disturbances.** It could reject call sites previously accepted. Only
  provable mismatches are rejected, so programs that were actually type-safe
  are unaffected; programs that would have failed at runtime now fail earlier.
* **Type system.** No change to `Ty`. It requires recording parameter
  annotations per function (currently only the return type is recorded).
* **Evaluation order.** None.
* **Divergence risk.** Low if `Unknown` handling mirrors the existing
  conservative rules (unannotated parameter ⇒ no check on that argument).
* **Resource safety.** None.
* **REPL.** Improves static feedback per submission; a function defined in an
  earlier submission is already in the session, so its signature is available.
* **Python.** None.
* **Redesign?** No.
* **Multiplicity.** Arity checking and argument-type checking are separable.
  Named/default/variadic arguments are a *different* family and are not
  required.

### 2. Empty-map literal — **Class B**

* **User value.** Medium. A real expressiveness gap: an empty map cannot be
  written; `{}` is a block.
* **New semantics.** A new spelling whose parse depends on context.
* **Disturbances.** `{}` currently means an empty block (`none`). Any change
  risks silently changing existing programs.
* **Type system.** None.
* **Divergence risk.** Low, but the parser disambiguation (`map_ahead`) must
  not misclassify a block.
* **REPL / Python.** Minor.
* **Redesign?** No, but it needs a syntax decision (see open questions).

### 3. Function argument ergonomics (named / default / variadic) — **Class B/C**

* **User value.** Medium for named and default; low for variadic.
* **New semantics.** Named arguments change call binding; defaults change
  arity; variadic changes arity and the registry.
* **Disturbances.** Named arguments interact with the pipeline (which inserts
  a positional first argument) and with the existing positional contract.
* **Type system.** Defaults require argument-type inference for the default;
  named arguments interact with parameter names.
* **Divergence risk.** Medium; binding rules must be specified precisely.
* **Redesign?** No, but it is a cluster and should be staged.

### 4. User-defined methods — **Class E/B**

* **User value.** Medium–high (ergonomics and composition).
* **New semantics.** Method definitions on structs/enums; receiver binding.
* **Disturbances.** The method registry is currently keyed by `TypeClass` for
  built-in kinds only; user methods would require a per-nominal-type method
  table and a lookup order (user method vs field vs built-in).
* **Type system.** Method signatures and receiver typing; likely a `Named`
  method table.
* **Divergence risk.** High; lookup precedence between fields, methods, and
  no-paren calls must be exact.
* **Redesign?** It touches the method model but not the value model. It is a
  large, careful feature, not a version change.

### 5. Modules / imports — **Class E**

* **User value.** High for larger programs.
* **New semantics.** Symbol visibility, file boundaries, initialization order,
  a dependency graph.
* **Disturbances.** Turns inert syntax into semantics; changes the meaning of
  existing `pub`/`use`.
* **Type system.** Cross-module type identity.
* **REPL / build.** A package/build model.
* **Redesign?** This is the largest candidate; it requires a deliberate
  design phase of its own and is explicitly out of scope now.

### 6. Destructuring — **Class B**

* **User value.** Medium; ergonomics.
* **New semantics.** Binding patterns in `let` and parameters; reuse of
  `match` pattern machinery.
* **Disturbances.** Parameter destructuring interacts with arity and types.
* **Divergence risk.** Low if patterns are non-exhaustive-checked the same way
  as `match`.
* **Redesign?** No; incremental.

### 7. Control-flow ergonomics (`else if`, match improvements) — **Class C**

* `else if` is a deliberate "one spelling" decision. Adding it weakens the
  language's stated philosophy for marginal convenience. **Not recommended.**
* Match improvements (exhaustiveness, or-patterns) are larger and can wait.

### 8. Range ergonomics (step, reverse) — **Class B**

* A `step` argument or a stdlib helper covers most uses without new syntax.
* No correctness gap. Keep in the queue.

### 9. Type-system improvements — **Class B**

* Field-type propagation and branch-join inference would extend the checker's
  reach. They are valuable but larger and introduce real inference questions;
  they are prerequisites for *some* future checks, not for the first feature.

### 10. Tooling — **Class B (independent)**

* A formatter, LSP, or package manager is largely orthogonal to the language
  core. The lexer drops comments, so lossless formatting needs lexer changes;
  this is a separate track.

---

## Semantic Cost Analysis

Cost per layer: **LOW / MEDIUM / HIGH**. No overall ranking is implied.

| Candidate | Grammar | AST | Checker | Runtime | Stdlib | REPL | Spec | Tests |
|---|---|---|---|---|---|---|---|---|
| Static arg checking | LOW | LOW | MEDIUM | LOW | LOW | LOW | MEDIUM | MEDIUM |
| Empty-map literal | MEDIUM | LOW | LOW | LOW | LOW | LOW | MEDIUM | MEDIUM |
| Named args | MEDIUM | LOW | MEDIUM | MEDIUM | LOW | LOW | HIGH | HIGH |
| Default args | LOW | MEDIUM | MEDIUM | MEDIUM | LOW | LOW | HIGH | HIGH |
| Variadic args | MEDIUM | MEDIUM | HIGH | MEDIUM | MEDIUM | LOW | HIGH | HIGH |
| User methods | MEDIUM | MEDIUM | HIGH | HIGH | MEDIUM | MEDIUM | HIGH | HIGH |
| Modules | HIGH | HIGH | HIGH | HIGH | HIGH | HIGH | HIGH | HIGH |
| Destructuring | MEDIUM | MEDIUM | MEDIUM | LOW | LOW | LOW | MEDIUM | MEDIUM |
| Field-type propagation | LOW | LOW | MEDIUM | LOW | LOW | LOW | MEDIUM | MEDIUM |
| Branch-join inference | LOW | LOW | HIGH | LOW | LOW | LOW | HIGH | MEDIUM |
| Range step | MEDIUM | LOW | LOW | MEDIUM | LOW | LOW | MEDIUM | MEDIUM |

---

## Compatibility Analysis

A feature that merely *adds* syntax has a different risk profile from one that
changes the meaning of existing source.

| Candidate | Existing valid code stays valid? | Existing invalid code becomes valid? | Existing behavior changes? | New ambiguity? | Source-compat risk |
|---|---|---|---|---|---|
| Static arg checking | Yes | No | Yes: some runtime `E3001` becomes check-time `E3001` | No | Low (phase changes only) |
| Empty-map literal | Only if `{}` keeps meaning block | Maybe | Possibly, if `{}` changes | Yes | **Medium–High** |
| Named args | Yes | Yes (new call forms) | No | Low | Low |
| Default args | Yes | Yes | No | Low | Low–Medium |
| Variadic args | Yes | Yes | No, but arity rules relax | Low | Medium |
| User methods | Yes | Yes | Yes (new resolution over user types) | Field-vs-method | Medium |
| Modules | Maybe | N/A | Yes (`pub`/`use` gain meaning) | Yes | **High** |
| Destructuring | Yes | Yes | No | Pattern-vs-expression in `let` | Low–Medium |

**The strongest compatibility property** belongs to static argument checking:
it changes only the *phase* of an error for programs that were already
incorrect, and it adds no syntax. Every valid program keeps running; every
program that ran *and was type-correct under its annotations* is unaffected.

---

## Specification Impact

A feature that does not add syntax can often be specified by tightening an
existing normative rule:

* Static argument checking: amend §6.2 and §15.7 (currently they say call
  arguments are *not* checked) and add a normative rule for arity/type at call
  sites. Add §34.2 entries removed or narrowed.
* Empty-map literal: amend §4.5 (grammar), §20.3 (the limitation), and §34.1.
* Named args: amend §4.5, §15.7, §23 (pipeline), and §24 (constructors).
* Any feature: update `docs/grammar.md`, `docs/contract.md`, and the examples
  that demonstrate the changed rule.

Every feature updates `LANGUAGE_SPEC.md` in the same change that changes
observable behavior.

---

## Recommended First Feature

**Static user-function argument checking** (arity plus annotated argument
types at call sites).

This is a *recommendation with rationale*, not an objective ranking.

**Why it fits the current architecture.**
* The checker already resolves calls to top-level functions and already knows
  each function's return type (`Checker::functions`). It already records and
  validates parameter annotations during hoisting; it simply does not retain
  them per function.
* The compatibility machinery (`Ty::compatible_with`) and the `Unknown`
  boundary already exist and are used for bindings, returns, struct fields,
  and enum payloads.
* It adds **no syntax, no AST nodes, no runtime behavior, and no stdlib
  surface**. It changes only *when* an already-defined error is reported.
* It is precisely testable: every claim is a source → expected-diagnostic pair.

**Why it provides user value.** It closes the exact gap the specification
documents as a limitation: annotations on parameters are currently advisory at
call sites, so `f("x")` against `fn f(a: int)` silently runs. That undermines
the language's promise that "when present, annotations are checked". Aligning
calls with construction makes the type story coherent.

**What it does not require.** No type-system redesign, no inference engine, no
evaluation-order change, no REPL model change, no Python change, and no
redesign of any frozen concept. `Unknown` stays permissive.

**Risks.** The only real risk is a false rejection if the checker over-reaches.
The mitigation is the frozen rule: reject only when both the annotation and the
argument type are known and incompatible. Arity is always known for a
statically resolved function. Because this can reject call sites that
previously ran, the change must be explicit in the specification and in the
release notes.

**Subsystems it would touch.** `src/check/mod.rs` (record parameter types;
validate calls), `docs/LANGUAGE_SPEC.md`, `docs/contract.md`, `tests/`.

### Status update — Q5 resolved

The one open design question (Q5: how to classify the change) has been
resolved:

**Static user-function argument checking is a SEMANTIC TIGHTENING WITH
SOURCE-COMPATIBILITY IMPACT.** It preserves all previously valid programs; it
may reject at check time programs that were previously accepted but could only
fail dynamically at runtime; runtime checks remain in place; and the change is
documented as a compatibility change. It is not a breaking redesign and does
not extend to unrelated type-system changes.

The normative specification amendment has been drafted in
`docs/LANGUAGE_SPEC.md` (§6.5 and §6.5.1, plus §6.2, §15.7, §26, §34.2). No
code has been changed. See `docs/FEATURE_001_DESIGN.md` for the full design,
the Compatibility Classification, the Required Implementation Test Matrix, and
the Implementation Contract.

See `docs/FEATURE_001_DESIGN.md` for the full design.

---

## First Feature Scope

### IN SCOPE

* Static **arity** checking for calls to top-level `fn` by name.
* Static **argument-type** checking for annotated parameters, using
  `compatible_with`, skipping any argument whose inferred type is `Unknown`.
* Diagnostics consistent with existing codes (`E3001`) and clear messages.
* Applies on every surface (CLI, library, REPL) because it lives in the
  checker.

### OUT OF SCOPE

* Named arguments, default arguments, variadic arguments.
* Checking calls through a lambda or a callable binding (still runtime).
* Inferring parameter types or propagating field/branch types.
* Any change to `Ty`, the runtime, the stdlib, the grammar, or the AST.
* Cross-submission forward references in the REPL.

### FUTURE RELATED WORK

* Named/default arguments (a separate cluster).
* Field-type propagation and branch-join inference (prerequisites for
  checking more call sites).
* Destructuring parameters.

---

## Future Feature Queue

Ordered by a qualitative sense of value against risk, not by a score. Items
marked **done** have shipped; the release-hardening surface (0.0.1) is listed
under "Release hardening" below.

| Order | Feature | Class | Notes |
|---|---|---|---|
| 1 | Static argument checking | A | **done** (Feature 001) |
| 2 | Empty-map literal | B | **done** (Feature 005) |
| 3 | Destructuring `let` | B | **done** (Feature 004) |
| 4 | Field-type propagation | B | **done** (Feature 003) |
| 5 | Named function arguments | B | **done** (Feature 002) |
| 6 | `else if` | C | **done** (Feature 006) |
| 7 | Range step / helpers | B | Possibly stdlib-only |
| 8 | Default arguments | B | Interacts with arity |
| 9 | Branch-join inference | B | Larger checker change |
| 10 | Block comments | B | Lexical feature; needs a syntax decision |
| 11 | User-defined methods | E/B | Large; resolution design required |
| 12 | Modules | E | Separate design phase |
| — | Generics / traits / async / VM | E | Not planned |

### Release hardening (0.0.1)

The first usable public scripting surface adds standard input, text-file I/O,
and program arguments: `read_line`, `read_file`, `write_file`, `args`, and the
`E4020` I/O diagnostic. See `docs/RELEASE_0_0_X_DESIGN.md`.

---

## Feature Development Workflow

This becomes the standard post-freeze discipline.

```
1. Feature proposal        (motivation, scope, non-goals)
2. Semantic design         (docs/FEATURE_<NNN>_DESIGN.md)
3. Specification update    (LANGUAGE_SPEC.md, contract.md, grammar.md)
4. Grammar / AST design    (only if the feature adds syntax or nodes)
5. Checker / runtime design
6. Implementation
7. Regression tests        (source → result / source → diagnostic)
8. Property / differential tests
9. Documentation           (README, examples)
10. Conformance audit      (spec ↔ implementation ↔ tests)
```

A feature is not "done" until step 10 confirms that the specification,
implementation, and tests agree.

---

## Versioning Policy

Classify every change before making it:

* **Compatible feature** — adds behavior without changing existing valid
  programs' meaning (e.g. a new stdlib function). Requires: spec update,
  regression tests, docs.
* **Syntax extension** — adds new syntax. Requires: grammar update, spec
  update, parser/AST/checker/runtime tests, docs.
* **Semantic extension** — changes where or when an existing rule applies
  (e.g. static argument checking). Requires: spec update, a compatibility
  note, regression tests, and a note that some previously runtime-only errors
  are now static.
* **Breaking change** — changes the meaning of existing valid source.
  Requires: a migration note, a version change, and explicit human approval.
* **Language redesign** — changes the character of the language (modules,
  generics, async, VM). Requires a dedicated design phase and a version change.

No version numbers are changed in this phase.

---

## Definition of Done (per feature)

A feature is done when:

* its design document exists and its open questions are resolved;
* `LANGUAGE_SPEC.md` states the new behavior normatively;
* the grammar/contract/examples that the feature touches are updated;
* the implementation is complete and clippy-clean;
* regression tests cover the new behavior and its boundary cases;
* property or differential tests cover the new invariants;
* the full verification suite passes;
* a conformance audit finds no spec/implementation/test contradiction.
