# Language Specification — Conformance Report

This report records the **Conformance Audit** of the Aura implementation
against [`docs/LANGUAGE_SPEC.md`](LANGUAGE_SPEC.md). It is not the
specification; it is the audit trail and the classification of every
meaningful discrepancy found.

The audit question was: *Does the actual Aura implementation conform to the
language specified in `LANGUAGE_SPEC.md`?*

---

## Executive Summary

The implementation conforms to the specification across the great majority of
its normative surface. Each major subsystem was reconstructed from the
implementation and compared against the specification's normative rules, and
every documented error code was traced to a producing path.

Three **implementation defects** were found and fixed, each restoring a rule
the specification already stated:

1. A **host stack overflow** from a recursive type alias (`type A = A`). The
   specification claimed recursive aliases were not expressible; in fact the
   checker recursed without bound and crashed. Recursive aliases are now
   rejected with `E3002` before execution.
2. **No-parentheses method calls bypassed the method registry.** `"x".nope`
   and the dead runtime aliases `"x".up` / `"X".down` were accepted by the
   checker and failed (or succeeded) only at runtime, violating §24. The
   checker now validates the no-paren form against the shared registry.
3. **`E4030` was not covered by the reachability test**, although §30.2 lists
   it as a normative code reachable from valid syntax.

Documentation drift was found and corrected in one place: the README's
headline pipeline example did not compile (a `|>`-led continuation line is not
valid), and the README claimed named arguments for enum construction, which
§18.2 rejects.

The specification itself contained **dangling/incorrect internal
cross-references** (sixteen of them) and one stale statement describing a
documentation drift that had already been fixed; these were corrected as
specification errors.

No language feature was added. No resource limit was removed or weakened. No
behavior was changed except the three conformance fixes above. The full
verification suite passes.

---

## Repository Baseline

```
HEAD     = cbd2dc91d8e03b3dbd3132e75f43ba120d649f6a
branch   = rewrite/v3-rust
status   = clean at the start of the audit
previous = aba88668856173337b68cd4fb8e046f0467bf561
```

Verified with `git rev-parse HEAD`, `git branch --show-current`,
`git status --short`, `git log -10 --oneline`.

---

## Conformance Method

For every normative statement in `LANGUAGE_SPEC.md` the audit established:

```
Specification rule
      ↓
Implementation location
      ↓
Reproduction (built binary, or existing test)
      ↓
Status: CONFORMING | IMPLEMENTATION BUG | SPECIFICATION ERROR |
        DOCUMENTATION DRIFT | TEST GAP | IMPLEMENTATION LIMITATION |
        DESIGN DECISION REQUIRED
```

The audit did not rely on the previous conformance report or on test status as
proof. Each claim was independently reproduced against the built binary at the
baseline commit, and only then classified. Implementation behavior without
supporting evidence was not promoted to normativity.

---

## Specification Evidence Hierarchy

Evidence was ranked:

1. explicit design decisions already frozen in the repository;
2. executable tests that intentionally assert semantics;
3. parser / checker / runtime implementation;
4. `docs/LANGUAGE_SPEC.md`;
5. `docs/contract.md`, README, examples;
6. accidental behavior with no supporting evidence.

Two strong sources never conflicted in a way that required stopping the audit.
Where the specification made a claim the implementation contradicted
(recursive aliases; no-paren method validation), the specification expressed
the *intended* semantics and the implementation was the defect.

---

## Specification Coverage

Every normative section was audited: §2 pipeline and `Unknown`; §3 lexical;
§4 grammar; §5 value/type universe; §6 annotations; §7 aliases; §8
compatibility; §9 operators; §10 numeric; §11 equality; §12 ordering; §13
evaluation order; §14 control flow and `finally`; §15 functions and closures;
§16 mutability and scope; §17 structs; §18 enums; §19 match; §20 collections;
§21 tuples; §22 loops and ranges; §23 pipeline; §24 methods; §25 built-ins;
§26 declarations and hoisting; §27 `pub`/`use`; §28 entry points; §29 REPL;
§30 error model; §31 resource safety; §32 Python boundary; §33 frozen
decisions; §34 known limitations.

---

## Grammar Conformance

The grammar in §4 was checked production-by-production against
`src/parse/mod.rs` and the executable EBNF in `docs/grammar.md`.

| Construct | Spec | grammar.md | Parser | Status |
|---|---|---|---|---|
| `fn` declaration | §4.2 | yes | `fn_item` | CONFORMING |
| `struct` / `enum` / `type` / `use` / `let` | §4.1–4.2 | yes | `struct_item`, `enum_item`, `alias_item`, `use_item`, `const_item` | CONFORMING |
| statement grammar | §4.4 | yes | `stmt_inner` | CONFORMING |
| precedence and associativity | §4.5 | yes | `infix`, `expr_bp`, `unary`, `postfix` | CONFORMING |
| `(a, b)` list sugar | §4.5 | yes | `atom` → `Expr::Tuple` | CONFORMING |
| `else if` → `E1014` | §4.5 | yes | `atom` (`if`) | CONFORMING |
| patterns | §4.6 | yes | `pattern` | CONFORMING |
| newline before `else`/`catch`/`finally` → `E1006` | §3.7 | yes | `block`/`stmt_inner` | CONFORMING |
| `use a as b` → parse error | §26 | note | `ident` sees `as` | CONFORMING |

**No grammar drift found.** `docs/grammar.md` and the parser agree on every
production, including the pipeline note.

`Tok::As` is lexed but consumed by no production (recorded in §34.3).

---

## AST Conformance

Every AST variant was checked for a parser source, a checker path, and a
runtime path.

| AST node | Parser | Checker | Runtime | Spec | Status |
|---|---|---|---|---|---|
| `Lit` | yes | `infer` | value | §3.6, §5 | CONFORMING |
| `Name` | yes | resolution | env/fn/native | §26 | CONFORMING |
| `FStr` | yes | inner exprs | display | §3.6.4 | CONFORMING |
| `Unary` | yes | `infer` | neg / truthiness | §9.2 | CONFORMING |
| `Binary` | yes | orderability | total dispatch | §9 | CONFORMING |
| `Call` | yes | builtin sig + name | call | §15, §25 | CONFORMING |
| `Method` | yes | receiver registry | dispatch | §24 | CONFORMING |
| `Field` | yes | receiver registry (fixed) | field / 0-arg method | §24 | FIXED |
| `Index` | yes | operands | get/set | §9.5 | CONFORMING |
| `List` / `Map` | yes | elements/entries | values | §20 | CONFORMING |
| `Construct` | yes | full validation | struct/variant | §17, §18 | CONFORMING |
| `Tuple` | yes | elements | lowers to list | §21 | CONFORMING |
| `Lambda` | yes | scope reset | closure | §15.4 | CONFORMING |
| `Pipe` | yes (non-call RHS) | operands | `call_value` | §23 | CONFORMING |
| `If` / `Match` / `Block` | yes | cond/branches/patterns | branch value | §14, §19 | CONFORMING |

No dead AST variant: every variant is constructed by the parser (verified by
static enumeration) and has a checker and runtime path.

---

## Type-System Conformance

The runtime value universe (`Value`) and the checker type universe (`Ty`) match
§5.1 and §5.2 exactly: `int`, `float`, `bool`, `string`, `none`, list, map,
struct, enum, function, range (runtime); `Int`, `Float`, `Bool`, `String`,
`List`, `Map`, `Named`, `Enum`, `Unknown` (checker).

Property claims in §5.3 (equality, ordering, indexing, iteration, callability,
mutability) were each reproduced and hold. There is no static `none` type;
`none` infers `Unknown` (§2.3, §5.2) — confirmed.

---

## Type Compatibility

The compatibility matrix in §8 was reconstructed from `Ty::compatible_with`
and reproduced at the checker: primitive types match only themselves;
`int`/`float` are not annotation-compatible; `List`/`Map` recurse; `Named` and
`Enum` are nominal; `Unknown` is compatible with everything. All cells
conform.

---

## Alias Conformance

Aliases are transparent and resolve in every type position (binding,
parameter, return, struct field, enum payload, and nested inside `[T]`,
`{string: V}`, `T | none`), chain transitively, and are visible across REPL
submissions. All conforming **except** recursive aliases, which crashed the
host — see Findings.

| Case | Status |
|---|---|
| `type Id = int` in a binding | CONFORMING |
| alias in parameter / return | CONFORMING |
| alias in struct field / enum payload | CONFORMING |
| chained aliases (`A = B`, `B = int`) | CONFORMING |
| alias nested in `[T]` / map / `T | none` | CONFORMING |
| alias unknown target → `E3002` | CONFORMING |
| **recursive alias** | **IMPLEMENTATION BUG → FIXED** |

---

## Operator Conformance

The full operator matrix in §9 was reproduced over all operand kinds. All
arithmetic (`+ - * / % ^`), unary (`-`, `not`), logical (`and`, `or`),
comparison, equality, and assignment operators conform. `+` is defined for
`int`, `float`, `string`, `list`; other arithmetic is numeric-only; comparisons
are numeric/string/bool only.

An exhaustive sweep of 384 `(value, op, value)` arithmetic combinations and a
comparison/equality sweep produced **zero `E4999`** — §30.3 holds.

---

## Numeric Conformance

§10 conforms in full: `i64` checked arithmetic with `E4013` on overflow;
`i64::MIN`/`i64::MAX` literal handling; truncating integer division;
sign-of-dividend remainder; integer power with negative exponent `E4013`;
`E4007` for `/` and `%` by zero on both `int` and `float`, treating `0.0` and
`-0.0` alike; `NaN`/`inf` reachable only through arithmetic; NaN comparisons
false; `-0.0` distinct; `to_int`/`to_float` conversion and error rules.

---

## Equality Conformance

§11 conforms: structural equality for lists/maps/structs/enums, numeric
cross-type equality (`1 == 1.0`), `none` only equal to `none`, ranges by
`start`/`end`, **functions by identity**, `NaN == NaN` false, `0.0 == -0.0`
true, cross-kind `false` without error.

---

## Ordering Conformance

§12 conforms: orderable pairs are numeric pairs, `string`/`string`, and
`bool`/`bool`; everything else is `E3001` (statically when provable, otherwise
at runtime); NaN is unordered (`false`, not an error); no lexicographic
ordering for compound values; `sort`/`min`/`max` fall back to source order
without error. `Ty::orderable_with` matches `Value::comparable_with`.

---

## Evaluation-Order Conformance

§13 conforms for every listed construct, verified with side-effecting probes:
binary operands, call arguments, list/map elements, constructor arguments,
index base/index, pipeline operands, `if` selection, `match` arms, and
short-circuit `and`/`or`.

One behavior was **underdocumented** and is now stated normatively: a compound
assignment evaluates the target's subexpressions **twice** (once to read, once
to write). This is a **SPECIFICATION GAP** that was closed; no code changed.

---

## Control-Flow Conformance

§14 conforms: truthiness; `if`/`else` values; `while`/`loop`/`for`; `break`
and `continue` scoped to loops and reset by lambdas (`E2015`); `return`
semantics and `E4030` for a control-flow signal escaping to value position;
`throw` and `catch` (only explicit `throw` is catchable; runtime diagnostics
are fatal); uncaught throw `E4026`; and the `finally` precedence rule (a
control-flow signal in `finally` replaces the pending outcome). Nested
`try`/`finally` and `finally` with `break`/`continue`/`return`/`throw` all
match the normative wording.

---

## Function Conformance

§15 conforms: declaration, positional calls, no defaults or variadics,
first-class values, function display `<fn>`, identity equality, recursion and
mutual recursion bounded by 512 frames (`E4011`), top-level-only named
functions, and lambda bodies with `return`. The documented limitation that
user-function call arity and argument types are **not** checked statically is
accurate: `f()` and `f("x")` against `fn f(a: int)` are accepted by the
checker and fail at runtime with `E3001`.

---

## Closure Conformance

§15.5 conforms: capture by reference; mutation of a captured `let mut` visible
outside; closures returned from their defining function still observe their
environment; closures stored in lists share that environment; nested closures
capture transitively; lambda parameters are immutable (`E2001`). The
specification's explicit statement that no ownership/lifetime model is
provided is accurate.

---

## Scope and Mutability

§16 conforms: `let`/`let mut`; `E2001` on immutable reassignment (static and
runtime); `E2005` without initializer; lexical block scoping; loop/catch/match
bindings scoped to their construct; nested shadowing permitted and same-scope
redeclaration `E2007`; parameters immutable and `_`-prefixed parameters
`E2009`; in-place mutation of struct fields, list elements, and map entries;
and reference semantics for lists, maps, and structs (verified through
aliases).

---

## Struct Conformance

§17 conforms in full. Named construction requires every declared field exactly
once; an unknown field is `E2003`; a missing, duplicate, or wrong-typed field
is `E3001`; positional construction requires an exact count in declaration
order; a value whose type is `Unknown` is accepted; no supplied field is
silently dropped. Field access, field assignment, equality (nominal +
structural), display, and the documented absence of field-type propagation on
reads were all reproduced.

---

## Enum Conformance

§18 conforms: globally unique tags (`E2013`); zero-payload variants
constructed with `A()` (bare `A` is `E2003`); positional payloads; named
payload arguments rejected (`E3001`) by both checker and runtime; payload
arity and type checked; equality by tag and payload; display by tag or
`Tag(payload)`. No path exists where the grammar accepts, the checker accepts,
and the runtime rejects strangely.

---

## Match Conformance

§19 conforms: arm order; guards; literal/binding/list/variant patterns;
duplicate binding `E2014`; unknown variant `E3002`; no exhaustiveness with
`E4029` on no match; subject evaluated once; control flow (`return`/`break`/
`throw`/`continue`) propagating from arms; and the documented requirement that
a bare control-flow keyword as an arm body needs a block.

---

## Collection Conformance

§20 conforms. Lists: construction, integer and negative indexing, `E4019` out
of range, snapshot iteration, mutation, element-wise equality, no ordering,
display. Maps: string keys (non-string literal key `E3001`), `m[k]` lookup with
`E2003` when absent, `get` → `none`, insert-on-assign, key-wise equality, no
ordering, **ascending key iteration and display**, removal. The empty-map
limitation (`{}` is a block, `none`) is reproduced and correctly documented.

---

## Tuple Conformance

§21 conforms: `(a, b)` constructs a list; nested comma-lists nest lists;
indexing, `len`, equality with a list, mutation, and display are list
semantics. There is no distinct tuple runtime type.

---

## Loop and Range Conformance

§22 conforms: `range(n)` = `range(0, n)`; start-inclusive/end-exclusive; step
`+1`; integer bounds only (`E3001` otherwise); empty and descending ranges
empty; negative bounds allowed; saturating `len`; lazy `for` iteration with
immediate `break`; materialization cap 10,000,000 (`E4013`); non-iterable
`E4018`; and snapshot iteration.

---

## Pipeline Conformance

§23 conforms: `x |> f` → `f(x)`; `x |> f(a)` → `f(x, a)`; `x |> r.m(a)` →
`r.m(x, a)`; non-call callable `x |> c` → `c(x)`; parse-time desugaring;
left-associativity and lowest precedence; left-to-right evaluation; and
`E3001` when the right operand is not callable.

---

## Method Conformance

§24 conforms **after a fix**: explicit method calls on a known receiver are
validated against the shared registry; no-parentheses member access on a
non-struct receiver is a zero-argument method call and is now validated
against the same registry; struct receivers are field reads. The dead runtime
aliases `up`/`down` are now unreachable through the pipeline, so §34.3's claim
is accurate.

---

## Built-in / Standard Library Conformance

The §24.1 method inventory and §25 built-in inventory were generated from
`src/stdlib/signatures.rs` and match the registry exactly: 34 built-ins and 30
method entries. Names, arity bounds, argument classes, and declared return
types agree with the checker and the runtime. The registry/implementation
drift items (`json_*` bypassing the shared `arity()` helper; the
`stdlib::arity` fallback) do **not** cause semantic divergence through the
public pipeline because the checker validates first; they remain internal
cleanup debt.

---

## Declaration and Hoisting Conformance

§26 conforms: forward references to functions, structs, enums, and aliases;
source-order evaluation of constants and top-level expressions; a constant
cannot read a later constant (`E2003`) while functions may; `E2007`/`E2012`/
`E2013` for duplicate declarations; and `as` reserved but unconsumed.

---

## REPL Conformance

§29 conforms: persistent bindings, functions, structs, enums, aliases, and
mutation across submissions; checking against all prior declarations; a failed
submission leaving the session unchanged (verified for a redeclaration and for
runtime errors); bare-expression display and silent declarations; `:quit`/
`:help`/EOF; module mode per submission; and `E2007` on redefinition. The REPL
shares the parser, checker, and interpreter with the file front end.

---

## Error-Model Conformance

§30 conforms. All thirty-two codes in §30.2 are produced by a reachable path;
the code set in `src/error.rs` matches the specification except `E5003`, which
§30.2/§34.3 correctly exclude as unused. `E4099` is used only as an internal
throw-crossing signal and never surfaces. Diagnostics carry a code, a span, and
a message; they are deterministic; runtime diagnostics are terminal and not
catchable; and `E4999` is unreachable (zero occurrences across the arithmetic
and comparison sweeps).

Three error-model issues were found:
* `E4030` was not covered by the reachability test → **TEST GAP, fixed**.
* The specification's §30.4 and §34.4 described the `E4030` `errors.md`
  example as still inaccurate, although it had been corrected in the previous
  phase → **STALE SPECIFICATION STATEMENT, fixed**.
* Recursive aliases crashed before producing a diagnostic → **IMPLEMENTATION
  BUG, fixed** (now `E3002`).

---

## Resource-Safety Conformance

§31 conforms. The semantic AST-node limit and the parser recursion backstop
are distinct mechanisms with a single `E1015` diagnostic; the call-frame limit
is `E4011`; range materialization is capped at 10,000,000. Boundary probes
(deep flat chains, nested lists, nested maps, nested calls, nested `if`,
parentheses) all terminate with `E1015` or `E4013`, never a host crash. The
one host-crash path found was the recursive alias (a checker defect), now
fixed.

---

## Python Boundary Conformance

§32 conforms (verified with the `py` feature): `None`/`bool`/`int`/`float`/
`str`/`list`/`dict` convert; `bool` is checked before `int`; an out-of-range
Python integer is `E4013`; `2**63 - 1` round-trips; a non-string dict key is
`E5002`; `nan`/`inf` cross as floats; structs and ranges cannot cross
(`E5002`); opaque objects become repr strings. No silent data loss was found.

---

## CLI / Library / REPL Conformance

§28 conforms. `aura run` (program mode), `aura check` / `aura eval` (module
mode), the library entry points, and the REPL share one parser, checker, and
interpreter. The only differences are the `main` requirement, output
presentation, and REPL persistence. `main` with parameters is `E2011`; a
missing `main` in program mode is `E4027`; `main`'s value is discarded;
top-level expressions and constants run in source order before `main`.

---

## Documentation Drift

| Location | Claim | Reality | Classification |
|---|---|---|---|
| README "Functional core" | multiline `\|>` pipeline | a `\|>`-led continuation line is `E1006` | DOCUMENTATION DRIFT → fixed |
| README "One spelling" | named arguments for `struct` **and enum** | enum payloads are positional; named rejected | DOCUMENTATION DRIFT → fixed |
| README checker note | linked `§11` | should be `§2.3`, `§6` | DOCUMENTATION DRIFT → fixed |
| README `E4030`/`errors.md` | (spec claimed drift) | already fixed in prior phase | SPECIFICATION ERROR → fixed |
| contract.md §6 | table omitted `E2015` | checker emits `E2015` | DOCUMENTATION DRIFT → fixed |

No further contradictions were found in `docs/contract.md`, `docs/errors.md`,
or `docs/grammar.md`.

---

## Example Conformance

All four `examples/*.aura` files parse, check, and run (`tests/examples.rs`).
They are consistent with the specification. The one invalid example found was
in the README (fixed above), not in `examples/`.

---

## Differential Testing

Targeted differential checks were run around operators, types, struct and enum
construction, aliases, collections, control flow, and methods:

* **checker accepts → runtime `E1xxx`/`E4999`**: none.
* **checker rejects → runtime would succeed**: none.
* **checker accepts → runtime dynamic error** (`E3001`, `E4007`): expected and
  documented under the `Ty::Unknown` boundary (§2.3), not a contradiction.
* The property/differential suite (`tests/property.rs`, 2048 cases) passes.

---

## Findings

### F1 — IMPLEMENTATION BUG (P1, host crash): recursive type alias

* **Location:** `src/check/mod.rs`, `resolve_type_expr`.
* **Reproduction:** `type A = A` (or `type A = B; type B = A`, or a cycle
  through `[A]`, `{string: A}`, `A | none`).
* **Evidence:** `thread 'main' has overflowed its stack` on both `check` and
  `run`.
* **Impact:** any valid-looking program could crash the host, violating §31.5
  and §7.
* **Resolution:** cycle detection with a visited set; rejected with `E3002`
  ("recursive type alias `X` has no concrete target"). Regression test
  `recursive_type_alias_is_rejected_not_a_crash`.

### F2 — IMPLEMENTATION BUG: no-paren method not validated

* **Location:** `src/check/mod.rs`, `Expr::Field` handling.
* **Reproduction:** `"x".nope` (checker accepts, runtime `E2003`); `"x".up`
  and `"X".down` (accepted and executed via dead runtime aliases).
* **Evidence:** §24 states unknown methods on known receivers are `E2003` at
  check time, and that `receiver.name` without parens is a method call.
* **Impact:** checker/runtime divergence; dead aliases reachable; violates
  "one spelling per construct".
* **Resolution:** the checker now validates no-paren member access against the
  shared method registry when the receiver type is known. Regression test
  `no_paren_method_exists_is_checked`.

### F3 — TEST GAP: `E4030` not in the reachability test

* **Location:** `tests/grammar.rs`, `error_samples`.
* **Evidence:** §30.2 lists `E4030` as normative; the reachability test that
  proves documented codes are producible omitted it.
* **Resolution:** added `E4030` with the reachable form
  `let x = if true { return 1 } else { 2 }`.

### F4 — SPECIFICATION ERROR: dangling internal cross-references

* **Location:** `docs/LANGUAGE_SPEC.md`.
* **Evidence:** sixteen `§n` references pointed at non-existent or wrong
  sections (e.g. `§22.8`, `§24.6`, `§34.7`, `§36`, `§38`, `§40`, `§44`,
  `§32`, `§10.2`, `§11`, `§13`, `§29`, `§34`, `§31`, `§21`).
* **Resolution:** all references corrected; a reference-integrity check now
  shows no dangling section references.

### F5 — SPECIFICATION GAP: compound-assignment target evaluated twice

* **Location:** §13 evaluation order.
* **Evidence:** `l[e("idx", i)] += e("rhs", 5)` prints `rhs` then `idx` twice.
* **Resolution:** §13 now states normatively that the target subexpressions are
  evaluated twice. No code changed.

### F6 — DOCUMENTATION DRIFT: README pipeline example

* **Location:** `README.md` "Functional core".
* **Evidence:** the multiline `|>` example is `E1006`; the named-enum-argument
  sentence contradicts §18.2; a section reference pointed at §11.
* **Resolution:** example rewritten on one line; enum claim corrected; the
  reference fixed to §2.3/§6.

### F7 — DOCUMENTATION DRIFT: contract table omitted `E2015`

* **Resolution:** added `E2015` to the contract's check-time table.

### F8 — ARCHITECTURE DEBT (recorded, not fixed)

`up`/`down` runtime aliases; `json_*` bypassing `arity()`; the
`stdlib::arity` fallback; unused `Tok::As`; unused `E5003`; `E4099` internal
signal; `Expr::Tuple` lowering; `Expr::Field` conflating field and method. None
causes semantic divergence through the public pipeline.

---

## Fixes Applied

| Finding | Change | File |
|---|---|---|
| F1 | alias-cycle detection → `E3002` | `src/check/mod.rs` |
| F2 | validate no-paren member access against the registry | `src/check/mod.rs` |
| F3 | add `E4030` reachability sample | `tests/grammar.rs` |
| F4 | correct sixteen dangling section references; remove stale §34.4 | `docs/LANGUAGE_SPEC.md` |
| F5 | document compound-assignment double evaluation | `docs/LANGUAGE_SPEC.md` |
| F6 | fix invalid pipeline example, enum claim, section link | `README.md` |
| F7 | add `E2015` to the check-time table | `docs/contract.md` |

No feature was added; no limit was removed.

---

## Tests Added

* `tests/regressions.rs::recursive_type_alias_is_rejected_not_a_crash`
* `tests/regressions.rs::no_paren_method_exists_is_checked`
* `tests/grammar.rs`: `E4030` added to the reachability sample list.

The 151-test baseline is retained and expanded.

---

## Remaining Architecture Debt

Unchanged from the specification phase and recorded here: the `up`/`down`
aliases (now truly unreachable), `json_*`/`arity()` bypass, `stdlib::arity`
fallback, `Tok::As`, `E5003`, `E4099`, `Expr::Tuple`, and the `Expr::Field`
conflation. None violates the specification.

---

## Remaining Specification Gaps

None known after this audit, beyond the documented limitations in §34. The
`Ty::Unknown` boundary, the static function-argument limitation, the
empty-map limitation, and the no-tuple and no-step-on-range limitations are all
explicitly documented.

---

## Remaining Design Decisions

None blocking. The specification does not define closure lifetime/ownership
(deliberately) or a spelling for the empty map (recorded as a limitation).
Neither is required for conformance.

---

## Verification Commands

```
cargo fmt --all -- --check                                             clean
cargo test --all-features                                              all pass
cargo test --no-default-features --features cli,repl,json,regex,time   all pass
cargo clippy --all-targets --all-features -- -D warnings               clean
cargo clippy --no-default-features --features cli,repl,json,regex,time -- -D warnings  clean
cargo clippy --no-default-features --features cli -- -D warnings       clean
PROPTEST_CASES=2048 cargo test --test property --all-features          pass
cargo test --test contract --test examples --all-features              pass
cargo build --release --no-default-features --features cli,repl,json,regex,time   ok
cargo +nightly miri test --lib --no-default-features --features cli    pass
git diff --check                                                       clean
MSRV (1.83)                                                            NOT verified (toolchain unavailable)
no-Python build / tests                                                pass
```

Additional probes (temporary, removed): operator/type matrix (384 combos),
comparison/equality sweep, differential checker/runtime sweep, recursive-alias
and no-paren-method cases, REPL session probes, Python-boundary cases,
resource-limit boundaries, evaluation-order side-effect probes.

---

## Final Semantic Status

**FROZEN WITH DOCUMENTED LIMITATIONS.**

Across all audited subsystems the implementation now conforms to
`LANGUAGE_SPEC.md`, with two genuine implementation defects fixed, one test gap
closed, and specification/documentation drift corrected. The remaining items
are documented limitations (static function-argument checking, empty-map
limitation, no tuple/step/lexicographic-ordering) and architecture debt that
does not affect semantics. The repository is ready for the final semantic
red-team before feature development.
