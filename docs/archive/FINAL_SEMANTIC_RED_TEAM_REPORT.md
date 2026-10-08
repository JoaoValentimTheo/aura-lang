# Final Semantic Red Team Report

An adversarial investigation of the Aura implementation against the frozen
[`docs/LANGUAGE_SPEC.md`](LANGUAGE_SPEC.md). The goal was to falsify the
semantic contract, not to confirm it.

---

## Executive Summary

The red team executed attacks across twenty-five categories. It found **one
new implementation bug** (a checker gap that let method calls on known struct,
enum, and range receivers pass the checker and fail — or misbehave — at
runtime), fixed it, and added a regression test. It confirmed the two
previously fixed defects (recursive aliases, no-parentheses methods) remain
fixed. It found **no host-crash path**, no `E4999`, no checker false-positive,
and no CLI/REPL/library semantic divergence beyond the explicitly permitted
ones.

All other attack categories were **CONFORMING** or **NO CONTRADICTION FOUND**;
negative results are reported with the inputs used.

One behavior that looks surprising but is conforming was identified and is now
documented: a type's value can be mutated through an immutable binding
(`let s = S{...}; s.x = 5`), because mutability is binding-level while field
and element mutation is value-level.

The full verification suite passes. The SHA changed with one new commit.

---

## Baseline

```
HEAD     = 0385456a6fff879f0f2a42f0c8278f395901dc54
branch   = rewrite/v3-rust
status   = clean at the start
predecessors: cbd2dc9 (spec freeze), aba8866 (semantic closure),
              d940fbe (review corrections)
```

---

## Attack Methodology

Every attack was executed against the freshly built binary at the baseline
commit, independently of the prior conformance report and the existing test
suite. Each attack recorded the programs tested, the expected behavior from
the specification, the actual behavior, and a classification. The red team
also deliberately searched for checker false positives and for any path to a
host failure.

Result vocabulary: `CONFORMING`, `NO CONTRADICTION FOUND`,
`IMPLEMENTATION BUG`, `SPECIFICATION ERROR`, `DOCUMENTATION DRIFT`,
`TEST GAP`, `ARCHITECTURE DEBT`, `KNOWN LIMITATION`,
`DESIGN DECISION REQUIRED`.

---

## Semantic Invariants Tested

I1 specification authority; I2 checker/runtime agreement; I3 parser/runtime
reachability; I4 type consistency; I5 deterministic evaluation; I6
control-flow consistency; I7 reference/state consistency; I8 REPL session
consistency; I9 resource safety; I10 Python boundary integrity; I11 error
determinism; I12 no accidental internal failures. See the invariant table at
the end.

---

## Compound Assignment

**Attack.** Vary the target: `a[i] += x`, `a[f()] += x`, `a[i].field += x`,
`obj[f()].x += x`, `matrix[i][j] += x`; side-effecting indices; out-of-range
writes; writes through immutable bindings.

| Program | Expected (§13) | Actual | Result |
|---|---|---|---|
| `a[e("idx",1)] += e("rhs",5)` | RHS once, target twice | `rhs idx idx` | CONFORMING |
| `a[f()] += 5` | `f` twice | `f f` | CONFORMING |
| `a[f()].x += 5` | `f` twice | `f f`, field written | CONFORMING |
| `a[5] += 1` | `E4019` | `E4019` | CONFORMING |
| `a[nxt()] += 100` (index changes) | read index 1, write index 2 | `[10,20,120]`, `n=2` | CONFORMING (documented hazard) |
| `let a = [1]; a[0] = 9` | value mutation allowed | `[9]` | CONFORMING (see Mutability note) |

No contradiction. The double evaluation is documented normatively in §13.

---

## Field / Method Resolution

**Attack.** Struct fields named like methods; explicit vs no-parenthesized
calls; unknown methods on known receivers; dead `up`/`down` aliases; range
methods; enums.

| Program | Expected (§24) | Actual (before fix) | Result |
|---|---|---|---|
| `struct S{len:int}; S{len:5}.len` | field read | `5` | CONFORMING |
| `S{len:5}.len()` | `E2003` (structs have no methods) | checker **accepted**, runtime `E2003` | **IMPLEMENTATION BUG → FIXED** |
| `S{a:1}.get("a")` | `E2003` | checker **accepted**, runtime `E2003` | **IMPLEMENTATION BUG → FIXED** |
| `A().foo()` (enum) | `E2003` | checker accepted, runtime `E2003` | **IMPLEMENTATION BUG → FIXED** |
| `range(0,3).nope` | `E2003` | checker **accepted**, runtime `E2003` | **IMPLEMENTATION BUG → FIXED** |
| `"abc".does_not_exist` | `E2003` | `E2003` (from F2) | CONFORMING |
| `"x".up` / `"X".down` | `E2003` (not language) | `E2003` (from F2) | CONFORMING |
| `s.len` where `s` is a struct field | field read | field value | CONFORMING |

The gap was that `check_method_call` bailed out when `Ty::type_class()`
returned `None`, which is the case for `Named` (struct, `range`) and `Enum`.
The fix resolves the receiver: `Unknown` stays permissive; struct/enum
reject any method; `range` uses the `Other` table; everything else uses its
class. A helper `method_class_for` is shared by the explicit and
no-parentheses paths. Regression test
`method_on_struct_enum_and_range_is_checked`.

---

## Unknown Boundary

**Attack.** Feed `Unknown` values into arithmetic, ordering, calls, methods,
indexing, struct fields, enum payloads, assignment, builtins, and equality.

| Program | Expected (§2.3) | Actual | Result |
|---|---|---|---|
| `f(1) + f(2)` | accept, compute | `3` | CONFORMING |
| `f(1) < f(2)` | accept, compute | `true` | CONFORMING |
| `f("ab").upper()` | accept, compute | `AB` | CONFORMING |
| `S{a: x}` with `x = none` | accept | `S { a: none }` | CONFORMING |
| `A(x)` with `x = none` | accept | `A(none)` | CONFORMING |
| `len(f(1))` | accept, runtime may fail | runtime `E3001` | CONFORMING |
| `f(1) == f(2)` | accept | `false` | CONFORMING |
| `x.foo` with `x = none` | accept, runtime may fail | runtime `E2003` | CONFORMING |

`Unknown` is never treated as known-valid or known-invalid. No contradiction.

---

## Closures

**Attack.** Captured mutable variables; nested closures; multiple closures
sharing a variable; shadowing; returned closures; closures in collections;
closure mutation of lists/maps/structs.

| Program | Expected (§15.5) | Actual | Result |
|---|---|---|---|
| `let mut x=1; let f=()->{x=x+1;x}; f(); x` | `2`,`2` | `2 2` | CONFORMING |
| closure returned from `make()` | observes its env | `1` then `2` | CONFORMING |
| two closures sharing `n` | shared mutation | `2` | CONFORMING |
| closure mutating a struct field | visible | `3` | CONFORMING |
| closure mutating a list | visible | `[1,2,3]` | CONFORMING |
| lambda self-reference in its own initializer | `f` not yet in scope | `E2003` | CONFORMING (§16.3) |

No ownership/lifetime semantics were invented; only observable behavior was
tested. No contradiction.

---

## Reference Semantics

**Attack.** The full aliasing matrix (list↔list, list↔struct, struct↔list,
map↔list, list↔map, struct↔map), then equality vs aliasing.

All eight combinations aliased correctly (mutation through one alias visible
through the other), and equality remained structural and reflected mutation
(`a == b` true, then false after `b.push(2)`). CONFORMING with §16.6 and §11.

---

## Function Equality

**Attack.** `g == g`, `g != g`, `g == h`, closure self-equality, aliased
closure, distinct identical closures, natives.

| Program | Expected (§11) | Actual | Result |
|---|---|---|---|
| `g == g` / `g != g` | `true` / `false` | `true false` | CONFORMING |
| `g == h` (distinct fns) | `false` | `false` | CONFORMING |
| `a == a`, `a == b`, `a == c (=a)` | `true false true` | `true false true` | CONFORMING |
| `g == len` (fn vs native) | `false` | `false` | CONFORMING |

Identity semantics hold. No contradiction.

---

## Recursive Alias Regression

**Attack.** `type A = A`, `A = B; B = A`, `A = [A]`, `A = {string: A}`,
`A = A | none`, plus a valid chain.

All recursive forms return `E3002: recursive type alias ... has no concrete
target`; the valid chain resolves. No crash, deterministic diagnostic.
**Regression passed** (F1 remains fixed).

---

## Struct Construction

**Attack.** Correct/missing/extra/unknown/duplicate/wrong-typed fields;
`Unknown` values; nested structs; aliases; functions; lists; maps; multiple
simultaneous errors.

All behave per §17: unknown field `E2003`; missing/duplicate/extra/wrong-type
`E3001`; `Unknown` accepted; no silent field loss. Multi-error input
(`S{a:"x",b:1,c:2,d:3}`) reports the first error deterministically across
runs. No contradiction.

---

## Enum Construction

**Attack.** Zero/one/multi payload; wrong arity; named and mixed named/
positional arguments; nested enum values; enums in collections.

All conform to §18: positional payloads only; named/mixed `E3001`;
arity/type `E3001`; zero-payload requires `A()`. No path where grammar and
checker accept while the runtime rejects strangely.

---

## Enum Tag Uniqueness

**Attack.** Two enums declaring the same tag; construction; matching;
equality; REPL.

`enum A { X } enum B { X }` is `E2013`. A single enum's tag constructs,
matches, and compares globally (by tag). No cross-enum accidental resolution.
CONFORMING with §18.1/§14.

---

## Evaluation Order

**Attack.** Side-effecting probes for binary operands, call arguments, list
elements, map entries, struct construction, index base/index, pipeline
operands, `if` branches, `match` subject, and short-circuit `and`/`or`.

Every construct evaluated left-to-right; `and`/`or` short-circuit; `if`
evaluated only the selected branch; `match` evaluated the subject once and
arms in order. **One under-documented behavior** (compound-assignment target
double evaluation) was already documented in §13 during the prior phase.
No contradiction.

---

## Control Flow / Finally

**Attack.** The full matrix of `return`/`break`/`continue`/`throw` in `try`,
`catch`, and `finally`, plus nested `finally` and fatal errors in `finally`.

| Case | Expected (§14.6) | Actual | Result |
|---|---|---|---|
| return in try, return in finally | finally wins | `2` | CONFORMING |
| throw caught, return in finally | finally wins | `2` | CONFORMING |
| throw in finally over caught | replaces | `caught a` then `outer b` | CONFORMING |
| break in try + finally | finally runs | `fin done` | CONFORMING |
| continue + break in finally | break replaces | `1` | CONFORMING |
| return + break in finally | break replaces | `after` | CONFORMING |
| nested finally | both run | `inner outer` | CONFORMING |
| fatal error in finally | propagates | `E4007` | CONFORMING |

No contradiction.

---

## Pipeline

**Attack.** `x |> f`, `x |> f(a)`, chaining, methods, closures, side effects,
invalid targets, named-argument-looking forms.

`10 |> add(5)` = `15`; chaining left-associative; `e("L",1) |> add(e("R",2))`
evaluates `L` then `R`; invalid targets `E3001`. The amount of side effects
matches left-to-right insertion. No contradiction.

---

## Collections

**Attack.** List indexing (positive/negative/out-of-range), mutation,
aliasing, equality, ordering, nesting; map key restrictions, insertion,
lookup, mutation, iteration, ordering, equality, nesting; empty-map limitation.

Maps iterate and display in ascending key order; equality is order-independent
(`{"x":1,"y":2} == {"y":2,"x":1}`); non-string keys `E3001`; missing key
`E2003`; `get` → `none`. Lists index by Unicode scalar and reject
out-of-range with `E4019`. `{}` is a block (`none`). CONFORMING.

---

## Tuple Sugar

**Attack.** `(a,b)`, `(a,b,c)`, nested, indexing, iteration, equality, function
arguments/returns, nested containers.

`(1,2,3)` displays `[1, 2, 3]`, equals `[1, 2, 3]`, indexes, has `len`
3, nests as lists, and passes/returns as a list. `(1)` is a grouping, not a
tuple. No path where tuple syntax becomes a distinct type. CONFORMING.

---

## REPL State Integrity

**Attack.** Persistence of let/let mut/fn/struct/enum/type across
submissions; invalid submissions interleaved; runtime errors; forward
references.

A failed submission leaves the session intact (`let x = 6` after a failed
`S{a:"bad"}` reports `E2007`, and `x` remains `5`); structs/enums/aliases
persist; runtime errors do not corrupt state. CONFORMING.

**Finding (KNOWN LIMITATION, not a bug).** A multi-item submission is bounded
by the REPL's line-oriented input: two complete `fn` items on **separate
lines** are two submissions, so a function cannot forward-reference a
function defined on a later line. Two items on **one line** are one
submission and forward-reference correctly, exactly like a file. §29 defines
per-submission checking and does not promise cross-submission forward
references, so this is consistent; it is recorded as a REPL input-model
limitation.

---

## CLI / REPL / API Parity

**Attack.** The same source string through `aura run`, `aura check`,
`aura eval`, and `aura repl`, for struct construction, enum named arguments,
and division by zero.

Struct wrong-type, enum named-argument, and `E4007` produce the **same code
and phase** on every surface (only the file label differs). The only
differences are the permitted ones: `main` requirement, output presentation,
and session persistence. **No semantic divergence found.**

---

## Resource Limits

**Attack.** The AST-node limit and call-frame limit independently, below/at/
above; combined deep AST + recursion + `finally`; closures; recursive aliases.

| Boundary | Result |
|---|---|
| nested lists 250 / 254 / 255 / 300 | accept / `E1015` / `E1015` / `E1015` |
| recursive `fn` | `E4011` |
| recursion through `finally` | `E4011` |
| deep parentheses (backstop) | `E1015` |
| recursive alias | `E3002`, no crash |

No panic, overflow, or abort observed. CONFORMING with §31.

---

## Python Boundary

**Attack.** `2**63 − 1`, `2**63`, `−2**63`, `−(2**63) − 1`, `2**62`; nested
maps/lists; non-string keys; large floats; structs; bool-vs-int.

`2**63 − 1` and `−2**63` round-trip; `2**63` and `−(2**63) − 1` are `E4013`;
nested containers convert; non-string keys `E5002`; `1e308*10` → `inf`;
structs `E5002`; `True` is `bool` and `py_eval("1") == true` is `false`.
No silent loss. CONFORMING.

---

## Built-in / Method Registry

**Attack.** Wrong arity, wrong types, too few/too many, `Unknown`, explicit and
no-paren invocation; the `json_*` bypass and the `stdlib::arity` fallback.

Every tested builtin and method produced the documented `E3001`/`E2003` at
the check phase (and at runtime). `json_encode(1,2,3)` is caught by the
checker. The `json_*` bypass and the `arity` fallback are **not observable
through the pipeline** — they are internal debt only. No semantic divergence.

---

## Declaration / Hoisting

**Attack.** Forward functions/structs/enums/aliases; mutual functions; mutual
aliases; shadowing; duplicates; nested declarations; REPL declarations.

Forward references work in a module; mutual functions work; duplicate `fn`
`E2007`; function shadowing by `let` is allowed. CONFORMING with §26.

---

## Numeric Extremes

**Attack.** `i64::MAX+1`, `i64::MIN`, `i64::MIN / −1`, `i64::MIN % −1`,
`i64::MAX*2`, `2^63`, `2.0/0.0`, `−0.0`, `0.0 == −0.0`, `to_int(±1e19)`.

All produce the documented `E4013`/`E4007`/values, with `−0.0 == 0.0` true
and `i64::MIN` representable. No wraparound. CONFORMING.

---

## Diagnostic Determinism

**Attack.** Repeated runs of programs with multiple simultaneous errors;
checking code, phase, and span stability.

Ten runs of a three-undefined-name program produced the same first
diagnostic; two-field struct mismatch consistently reported the first field;
comparison error stable. CONFORMING with §30.3.

---

## Internal Failure Surface

**Attack.** Static search for `panic!`, `unwrap()`, `expect()`,
`unreachable!`, `todo!`, `unimplemented!`, and `E4999`; adversarial input.

* No panicking macros in production code.
* `unwrap`/`expect` occurrences are all `Parser::expect` (a method) or
  `unwrap_or`/`unwrap_or_default`, none fallible on user input.
* `E4999` sites: thread spawn/join failures (host-level), and two
  unreachable operator-dispatch arms.
* 768 operator combinations and 20 malformed inputs produced no panic,
  overflow, abort, or `E4999`.

---

## Cross-Feature Attacks

**Attack.** alias+struct+map+list; closure+mutable struct; closure+list;
pipeline+closure; pipeline+method; enum+match+finally; deep AST+closure;
REPL+error+alias; alias-to-struct/enum in method calls; map of closures.

All combinations behaved per the specification. `"a,b,c" |> split(",")` is
correctly `E2003` (a bare `split` is not a builtin; the method form is
`"a,b,c".split(",")`), which follows §23's desugaring rule. No contradiction.

---

## Specification Attack

The specification was read again for internal contradiction. No
specification-vs-implementation contradiction remains. One specification
wording was tightened (the method rule for struct/enum/range receivers) to
match the strengthened implementation.

---

## Documentation Cross-Check

`README.md`, `docs/contract.md`, `docs/errors.md`, and `docs/grammar.md` were
re-checked. No new drift was found beyond what the prior phase already
corrected. `README` examples were executed and are consistent.

---

## Findings

| ID | Area | Severity | Finding | Reproduction | Classification | Fixed | Regression |
|---|---|---|---|---|---|---|---|
| RT1 | Field/Method | P2 | Method calls on known struct/enum/range receivers passed the checker and failed at runtime | `struct S{len:int}; S{len:5}.len()`, `range(0,3).nope`, `A().foo()` | IMPLEMENTATION BUG | yes | `method_on_struct_enum_and_range_is_checked` |
| RT2 | Mutability | P3 | Value mutation through an immutable binding is allowed but only implicit in the spec | `let s=S{a:1}; s.x=5` | DOCUMENTATION (clarified) | spec | n/a |
| RT3 | REPL | P3 | Cross-submission forward references differ from a single module | two `fn` items on separate REPL lines | KNOWN LIMITATION (recorded) | no | n/a |
| RT4 | Compound assign | P3 | Target subexpressions evaluated twice | `a[f()] += x` prints `f` twice | KNOWN BEHAVIOR (documented §13) | spec (prior phase) | n/a |

No P0 or P1 findings.

---

## Fixes

**RT1** (source): `src/check/mod.rs` — added `method_class_for`, which
distinguishes `Unknown` (permissive), struct/enum (no methods → `E2003`),
`range` (the `Other` table), and all other types (their class). Both
`check_method_call` and the no-parentheses `Expr::Field` path use it.

**RT2** (specification): §16 now states explicitly that mutation of a value
(through a field, element, or entry) is permitted regardless of the binding's
mutability, because mutability is binding-level and containers are shared.

No other source changes. No feature added. No limit removed.

---

## Regression Tests

* `tests/regressions.rs::method_on_struct_enum_and_range_is_checked` — RT1.

The recursive-alias (`recursive_type_alias_is_rejected_not_a_crash`) and
no-paren (`no_paren_method_exists_is_checked`) regressions from the prior
phase were re-run and pass.

---

## Verification

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
no-Python                                                              pass
```

Attack probes (temporary, removed): ~120 programs across the twenty-five
categories, including 768 operator combinations, the aliasing matrix, the
`finally` matrix, the Python integer boundary set, and the false-positive
sweep.

---

## Remaining Limitations

Inherited and re-confirmed: user-function call arguments are not statically
checked (§34.2); field reads infer `Unknown` (§17.5); `if`/`match`/lambda
infer `Unknown`; no empty-map literal; no tuple type; no `try` without
`catch`; `match` control-flow needs a block; no range step; no lexicographic
ordering of compound values; the REPL's line-oriented submission boundary
(RT3).

---

## Remaining Architecture Debt

`up`/`down` runtime aliases (now unreachable); `json_*` bypassing `arity()`;
the `stdlib::arity` fallback; unused `Tok::As`; unused `E5003`; `E4099` as an
internal signal; `Expr::Tuple` lowering; `Expr::Field` conflating field and
method. None causes semantic divergence.

---

## Remaining Design Decisions

None blocking. Closure lifetime/ownership and the empty-map spelling remain
deliberately unspecified, as documented.

---

## Final Invariant Table

| Invariant | Attack Performed | Result | Evidence |
|---|---|---|---|
| I1 Specification authority | re-read spec; cross-checked every area | PASS | no unresolved contradiction |
| I2 Checker/runtime agreement | differential sweep + method/struct/enum attacks | PASS (after RT1) | 768-combo sweep; RT1 regression |
| I3 Parser/runtime reachability | every AST variant constructed and executed | PASS | AST enumeration + probes |
| I4 Type consistency | compatibility matrix, aliases, Unknown | PASS | alias/method probes |
| I5 Deterministic evaluation | side-effect ordering probes | PASS | eval-order probes |
| I6 Control-flow consistency | full `finally` matrix | PASS | 10-case matrix |
| I7 Reference/state consistency | aliasing matrix + equality | PASS | 8-combination matrix |
| I8 REPL session consistency | valid/invalid interleaving | PASS | REPL probes |
| I9 Resource safety | AST/call/parser boundaries | PASS | boundary probes |
| I10 Python boundary integrity | integer extremes + nested | PASS | Python probes |
| I11 Error determinism | repeated multi-error runs | PASS | 10-run stability |
| I12 No accidental internal failures | static search + 768 combos | PASS | no panic/E4999 |

---

## Final Semantic Status

**SEMANTICALLY READY FOR FEATURE DEVELOPMENT.**

One P2 implementation bug was found and fixed; no P0/P1 finding remains; the
two prior fixes hold; no user-reachable host-crash path exists; checker and
runtime agree across the audited surface; CLI/REPL/library semantics agree
within the permitted differences; resource limits are effective; Python
conversion is lossless where promised; and specification contradictions are
resolved.

This is not a claim that Aura is bug-free. It is a claim that the semantic
contract survives the attacks executed here, with the documented limitations
above.
