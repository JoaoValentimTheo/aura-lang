# Aura Post-Freeze Feature Roadmap

This document plans the evolution of Aura after the semantic freeze. It is a
**roadmap and historical design** document, not a specification. Current
status is summarized below; historical recommendations are retained with their
original context and do not describe today's missing features.

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
| Lexing | ASCII identifiers; line and multiline comments; newline/semicolon statements; decimal/hex/binary/octal ints, floats, string/char quotes, f-strings |
| Grammar | Recursive-descent + Pratt; right-associative `^` and `..`; left-associative everything else; explicit precedence table |
| Value model | `int`, `float`, `string`, `bool`, `none`, `list`, `map` (keyed by a key-capable scalar, ordered), struct instance, enum variant, function, range |
| Type model | Primitives, structural collections, nominal types, unions and static generic parameters/applications |
| Unknown | Exact conservative boundary: the checker rejects only what it can prove |
| Operators | `+ - * / % ^ == != < <= > >= and or`, bitwise `& \| ~ << >>`, unary `-`/`not`/`~`, pipeline `\|>`, assignment `= += -= *= /= %= ^= &= \|= <<= >>=` |
| Evaluation order | Strict left-to-right; `and`/`or` short-circuit; documented |
| Control flow | `if`/`else if`/`else`, `while`, `loop`, `for`, `break`/`continue`, `return`, `throw`, `try`/`catch`/`finally` |
| Functions | Top-level `fn` with hoisting and mutual recursion; positional and named calls; static argument checking; function/method **overloading** by ordered input types (return type does not distinguish overloads) |
| Lambdas | `(x: int, mut y) -> …` — one parameter model shared with functions; the same signature/closure model |
| Closures | Lambdas capture by reference; no ownership/lifetime model |
| Mutability | Binding capability: `let` immutable, `let mut` mutable; place writes and mutating calls require a `mut` root; `mut self` marks a mutating method |
| Structs | Nominal; named and positional construction; validated fields |
| Enums | Tags unique per module; positional payloads; named payload args rejected |
| Match | Expression; literal/binding/list/variant patterns; guards; no exhaustiveness |
| Collections | Lists (index/mutate/iterate) and ordered maps keyed by `string`/`int`/`bool` (or a union); `map.items()`; list and map comprehensions; empty map `{:}` |
| Destructuring | `let [a, b] = e` and `let Ok(x) = e`, reusing the pattern system |
| Field types | A field read on a known struct infers the declared type (Feature 003) |
| Scripting I/O | `read_line`, `read_file`, `write_file`, `args` (0.0.1) |
| Comments | Line (`#`) and multiline (`<!-- ... --!>`) comments |
| Tuples | `(a, b)` is list sugar; no distinct tuple type |
| Loops/ranges | `range(a, b)` and `a..b`, step 1, end-exclusive, lazy in `for` |
| Pipeline | `x \|> f(a)` is `f(x, a)` |
| Methods | Built-in receiver kinds (string/list/map/range) plus user methods on nominal structs via `impl` with an explicit receiver (`self`/`mut self`) (OOP V1); enum methods and bound-method values are not implemented |
| Traits | Static behavioral contracts (`trait`/`impl Trait for Struct`), merged into the struct's method table; no defaults, objects, or dynamic dispatch (OOP V2) |
| Constants | `const NAME = e` module constants; a top-level `let` is the same; source-ordered, immutable bindings |
| f-strings | `{expr}` interpolation, `{{`/`}}` escapes, and a small format spec (`{x:.2f}`, `{n:>6}`, `{n:06d}`, `{n:x}`); no `{x=}`/`{x!r}` |
| Built-ins | Core builtins (incl. `read_line`, `read_file`, `write_file`, `args`) + method entries in one shared registry |
| Aliases | Transparent; chained; recursive aliases rejected (`E3002`) |
| REPL | Persistent session; bindings/functions/structs/enums/aliases/traits/constants; line-oriented submissions |
| CLI/API | `run`, `check`, `eval`, `repl` over one `compile`+`execute` pipeline |
| Python | Optional `py` bridge; lossless integer/dict-key rules |
| Resource limits | 256 AST nodes (`E1015`), 512 call frames (`E4011`), 10M range cap |
| Error model | Stable `E####` codes by phase; deterministic first diagnostic |

### Core strengths to preserve

* One pipeline and one signature registry: the checker and runtime cannot
  drift on the standard library.
* A precise `Unknown` boundary with no false positives.
* Deterministic left-to-right evaluation.
* A specification backed by conformance tests; open syntax gaps are tracked in
  `CONFORMANCE_PHASE1.md`.

### Current limitations (documented in `LANGUAGE_SPEC.md` §34)

* `if`/`match`/block expressions and lambdas infer `Unknown`.
* No default or variadic function arguments.
* `try` requires `catch`.
* `match` arm bodies that are bare control flow keywords need a block.
* No nested named function declarations.
* No range step or `for…else`.
* No `a..=` inclusive range literal (the half-open `a..b` form exists).
* No nested block comments; `<!-- ... --!>` multiline comments are current.
* No distinct tuple type (`(a, b)` is list sugar) or lexicographic ordering for compound values.
* No closure lifetime/ownership model (deliberate).
* No bitwise XOR (`^` is exponentiation) and no `++`/`--` (increment is an
  explicit assignment); `%=`/`^=`/`&=`/`|=`/`<<=`/`>>=` do exist.
* f-strings have no self-documenting `{x=}`, conversions `{x!r}`, or grouping
  flags (`{x:,}`); the format mini-language is the small subset in §3.6.4.

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
| `Tok::As` | delivered | Consumed by `use path as Alias`; no longer debt |
| `E5003` defined but never produced | dead constant | No |
| `E4099` internal throw signal in the public code space | naming | No |
| `Expr::Tuple` lowers to a list | AST surface | No (documented) |
| `Expr::Field` conflates field read and no-paren method call | structural | No (checker covers known receivers) |

These are recorded so they are not silently entangled with feature work. A
feature may be scheduled before or after them independently.

---

## Historical freeze-era limitations classification

This table records the original assessment at the freeze, before Features
001–006, modules and generics. Rows worded “No …” below are historical, not
current limitations. The current status ledger is authoritative within this
roadmap:

| State | Capability | Evidence |
|---|---|---|
| DELIVERED | Named function arguments; static call checks | Spec §15.7; tests/contract_sync.rs |
| DELIVERED | Empty map `{:}`; destructuring `let`; `else if` | Spec §§4,20; tests/parser.rs |
| DELIVERED | In-source modules, visibility, imports and aliases | Spec §§27–28; tests/modules.rs |
| DELIVERED | Struct methods, traits, generics and bounds | Spec §§17,36; tests/generics.rs, tests/traits.rs |
| DELIVERED | Half-open ranges; general unions; multiline comments | Spec §§3–4,22; tests/lexer.rs, tests/parser.rs |
| DEVELOPMENT | Provider-backed cross-source modules (native complete; virtual/WASM foundation local/unreleased) | `docs/FILESYSTEM_MODULES_DESIGN.md`; `STATUS.md` |
| CURRENT LIMITATION | Named method arguments; range step; rest patterns; package/project tooling | Separate proposals required |
| DELIBERATE DESIGN DECISION | No tuple value type, XOR or increment operator | Spec §§4,21 |
| RFC CANDIDATE | Multiline pipelines, list-rest patterns, range step, named method arguments | CONFORMANCE_PHASE1.md |
| DECISION-PENDING | TypeExpr nesting | Existing AUDIT3_TYPE_NESTING_DECISION.md only |

Historical classification keys:

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
| No `a..=` inclusive range literal | **B** | `a..b` half-open exists; `..=` is a later ergonomic addition |
| General union types | **DONE** | Shipped: `T1 \| T2`, transparent through aliases (`LANGUAGE_SPEC.md` §4.3) |
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
| Modules / imports | **E** | Filesystem/project module system; the next major phase after Generics |
| Generics / trait bounds | **done** | Static, erased, nominal parametric polymorphism; `docs/GENERICS.md` |
| Async / concurrency | **E** | Requires a different runtime |
| Bytecode VM / codegen | **E** | Out of scope by design |

No class **E** item is proposed in this roadmap. They are recorded so that
"missing feature" is not confused with "next feature".

---

## Historical candidate feature families

1. **Function ergonomics** — static argument checking; named arguments;
   default arguments; variadic arguments.
2. **Collection ergonomics** — empty-map literal; list/map helpers; iteration
   conveniences.
3. **Pattern / destructuring** — destructuring `let`; destructuring
   parameters; richer slice patterns.
4. **Type-system improvements** — field-type propagation; branch-join
   inference; a static `none` type; function types.
5. **Methods** — struct methods are done (Feature: Aqua OOP V1); enum methods
   and bound-method values remain.
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

### 4. User-defined methods — **done (partial: struct methods; enum methods remain)**

* **User value.** Medium–high (ergonomics and composition).
* **New semantics.** Struct methods via `impl`, with an explicit `self`
  receiver (`LANGUAGE_SPEC.md` §17.6); several `impl` blocks per struct merge
  into one method surface; field/method collisions rejected; static
  per-nominal-type lookup with no fallback.
* **Disturbances.** A per-nominal-type method table now sits beside the
  built-in `TypeClass` registry; the lookup order is fixed (field read without
  parentheses, method with parentheses, no cross-category fallback).
* **Remaining.** Enum methods and bound-method values. In-source modules and
  visibility have since shipped (§27). See `docs/OOP.md` for the finalized four-pillar model.

### 5. Modules / imports — **DELIVERED SEMANTICS; PROVIDER LOADING IN DEVELOPMENT**

* **User value.** High for larger programs.
* **Delivered semantics.** `module Name { ... }` (nestable), private-by-default items,
  `pub` to export, `use path [as Alias]` imports, and `::`-qualified paths,
  enforced by `src/resolve.rs` and the checker (`LANGUAGE_SPEC.md` §27).
* **Provider loading.** Native filesystem sources and caller-supplied virtual
  sources feed the same provider-neutral graph and resolver. FSM-P6 exposes the
  virtual path in the browser: the Playground holds a multi-file project and
  executes it through the same transport. The runtime that carries the additive
  virtual-project exports is currently a development artifact
  (`0.2.0-dev.1`), not a published release.
* **Remaining.** Package/build manifests, package management, remote dependency
  resolution, browser project persistence, and a visual module-ownership tree
  remain deferred.

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

## Implemented Language Evolution (development after frozen 0.0.2)

The following were previously recorded here as *future design directions*. They
are now **current, implemented, and tested** features; the boundary note that
used to live here is superseded. The single authority remains
`docs/LANGUAGE_SPEC.md`.

### General union types + transparent alias composition — DONE

**Current (normative).** A type expression is a `|`-separated union of one or
more members (`LANGUAGE_SPEC.md` §4.3). `T | none` is the one-member-plus-`none`
case of the same grammar. Unions flatten, deduplicate, and canonicalize
member order (`int | float` == `float | int`); aliases compose transparently
into and through unions.

```aura
type Number = int | float
type ID = string | int
type UserID = ID            # transparent alias of the union `string | int`
type Nullable = Number | none
```

Implementation: `TypeExpr::Union` (AST), `Ty::Union` + `Ty::union`
(normalization) + generalized `Ty::compatible_with` (`src/types.rs`),
`resolve_type_expr`/`resolve_type_expr_lenient` (checker). Cycle protection is
unchanged (`E3002`). Tests: `tests/checker.rs`, `tests/parser.rs`,
`tests/run.rs`, `tests/boundaries.rs`.

### Rust-style `a..b` range literal — DONE

**Current (normative).** `a..b` parses to the same lazy, half-open,
start-inclusive Range value as `range(a, b)` (`LANGUAGE_SPEC.md` §22.1). `..`
is a distinct lexer token, never part of a float; `range(a, b)` is retained and
the two are equivalent (`0..3 == range(0, 3)`). A Range is a Range, not a list:
`[1..3]` holds one Range.

```aura
for i in 0..10 { }
let r = 0..3
```

Implementation: `Tok::DotDot` (lexer), `Expr::Range` (AST + parser precedence
`range = additive [ ".." range ]`), `Ty::Named("range")` inference, and
`eval` building the existing `RangeVal` — no second range subsystem. Tests:
`tests/lexer.rs`, `tests/parser.rs`, `tests/run.rs`, `tests/repl.rs`.

### Multiline comments — DONE

**Current (normative).** `<!-- ... --!>` is discarded by the lexer, may span
lines, produces no token/AST/runtime effect, and does not act as a statement
separator (`LANGUAGE_SPEC.md` §3.5). An unterminated comment is `E1005`.

```aura
<!--
  a comment
--!>
```

---

## Historical semantic cost analysis

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

## Historical compatibility analysis

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

## Historical specification impact

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

## Historical recommendation for Feature 001 (delivered)

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

## Historical Feature 001 scope (delivered)

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

## Delivery ledger and future queue

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
| 10 | Multiline comments | B | **done** (`<!-- ... --!>`); nesting remains absent |
| 11 | User-defined methods | E/B | **done** (struct methods, §17.6; enum methods remain) |
| 11b | Traits | E | **done** (static behavioral contracts, §17.7; no dynamic dispatch; generic bounds have since shipped) |
| 11c | Mutation capability | E | **done** (BFR-II: `let mut`/`mut self`, §16.6) |
| 11d | Bitwise operators + compound assignments | C | **done** (BFR-II: `& \| ~ << >>`, `%= ^= &= \|= <<= >>=`) |
| 11e | f-string format mini-language | C | **done** (BFR-II, §3.6.4) |
| 11f | Variable shadowing | E | **done** (Shadowing: `let`/`let mut` shadow, `const` does not, §16.3) |
| 11g | Foundation stability + CI/clean-room gate | E | **done** (Break-the-Aura II, PC ↔ Web symmetry, reproducibility) |
| 12 | Method overloading | E | **done** (Function/method overloading by ordered input types, §15.7) |
| 13 | Modules | E | **done** (logical semantics); provider-backed cross-source loading implemented through current filesystem track; package/project tooling remains future work |
| — | Async / VM / `++`/`--` | E | Not planned |
| 14 | Generics | E | **done** (Generic functions, structs, methods, traits, bounds, aliases; `docs/GENERICS.md`) |

### Release hardening (0.0.1)

The first usable public scripting surface adds standard input, text-file I/O,
and program arguments: `read_line`, `read_file`, `write_file`, `args`, and the
`E4020` I/O diagnostic. See `docs/RELEASE_0_0_X_DESIGN.md`.

### Infrastructure release (0.0.2)

No language-semantic change. `0.0.2` adds portable execution and tooling on top
of the frozen `0.0.1` language:

* a WebAssembly runtime (`wasm32-unknown-unknown`) built from the same
  interpreter, with no operating-system authority;
* an explicit host boundary (`src/host.rs`) so every outside-world interaction
  routes through one contract, with native and browser hosts;
* a versioned, immutable browser Playground with hashed runtime artifacts;
* the official website and GitHub Pages deployment;
* cross-platform release infrastructure with checksummed artifacts.

The release, language, and runtime versions are deliberately distinct:
release `0.0.2`, language `0.0.1`, runtime `0.0.2`. OOP V1 (struct methods via
`impl` with an explicit `self` receiver) is implemented and available in the
development runtime. Traits (static behavioral contracts, §17.7) followed. The
core contract was then synchronized (LSCS): `const NAME = e` is the canonical
module constant, `let`/`mut`/shadowing/scope have one explicit rule with a
published scope matrix, and the operator and f-string contracts were closed.
The foundation revision (BFR-II) then added the mutation-capability model
(`let mut` / `mut self`, §16.6), the bitwise operator family with compound
assignments, a small f-string format mini-language, and uniform trailing
commas. Variable shadowing followed (`let`/`let mut` shadow, `const` does not,
§16.3), and a foundation-stability gate verified the core through adversarial
destruction, a semantic-consistency audit, a Native/WASM differential, and a
PC ↔ Web symmetry corpus. The language foundation is now **frozen** for feature
development. **Function and method overloading** is implemented on top of the
frozen foundation: a name may have several definitions when their ordered input
types differ, the return type never distinguishes overloads, and resolution is
deterministic (most specific wins; a tie is an error). **Generics** followed:
static, erased, nominal type parameters on functions, structs, methods, traits,
and aliases, with bounds and inference composed into the same overload resolver
(`docs/GENERICS.md`). The filesystem module system is the active development
track; FSM-P1 through FSM-P5 are closed, and FSM-P6 (the multi-file Playground
UX) is implemented locally and under review.

### Increment/decrement decision (`++` / `--`)

Aura deliberately has **no** `++` or `--` operator. This is a design decision,
not an omission:

* **Mutation is explicit.** `x = x + 1` and `x += 1` already express increment,
  and they read the same as every other mutation. A dedicated operator would be
  a second spelling for one construct, against the one-spelling principle.
* **Value semantics would be a trap.** Prefix vs postfix differ only in the
  *value* of the expression, an easy source of off-by-one bugs; Aura's
  expression-oriented design would have to define, teach, and test both forms.
* **It composes badly with the capability model.** A postfix `x++` hides a
  write inside an expression; the mutation-capability rule (§16.6) is easiest to
  state and reason about when writes appear as assignments.
* **No test showed a need.** No existing corpus is clearer with `++`.

If it were ever admitted, the coherent form would be postfix-only or
prefix-only (never both), statement-position-first, requiring a `mut` place,
and defined as sugar for the corresponding assignment returning `none`; that is
recorded here so a future proposal starts from a defined baseline rather than
inventing two forms.

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
