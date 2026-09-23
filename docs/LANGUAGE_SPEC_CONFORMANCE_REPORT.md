# Language Specification — Conformance Report

This report explains how [`docs/LANGUAGE_SPEC.md`](LANGUAGE_SPEC.md) was
derived and verified. It is **not** the specification; it is the audit trail
behind it.

---

## Executive Summary

The Aura language was inspected end to end — lexer, parser, AST, checker,
runtime, standard library, Python bridge, CLI, REPL, grammar, contract, tests,
and examples — and a single normative specification was produced at
`docs/LANGUAGE_SPEC.md`.

The specification was derived from evidence in this order: explicit recent
design decisions (the Semantic Closure commit), executable tests, parser /
checker / runtime implementation, `docs/contract.md`, README/examples, and
finally accidental behavior (which was **not** promoted to normativity without
supporting evidence).

Every major language construct now has a complete semantic path. Two
implementation artifacts required a documented status rather than a silent
freeze: the absence of a tuple type (§21) and no-parentheses method calls
(§24), both of which are now normatively documented because the repository
provides strong, consistent evidence for them. Two genuine
documentation/implementation discrepancies were found and are recorded (§30.4
and Known Limitations 34.3); one (`E4030`'s example) was corrected in
`docs/errors.md`.

No language behavior was changed to make the specification easier to write.
The only non-specification edits are documentation alignment: one README
sentence, the contract header, and the `E4030` example. No source-code change
was needed; the implementation already conformed to the frozen semantics.

The full verification suite passes.

---

## Repository Baseline

```
HEAD     = aba88668856173337b68cd4fb8e046f0467bf561
branch   = rewrite/v3-rust
status   = clean at the start of the phase
previous = d940fbed54d8afee020e6e7949fe6b2266847422
```

The Semantic Closure commit (`aba8866`) is present. Baseline was verified with
`git rev-parse HEAD`, `git branch --show-current`, `git status --short`,
`git log -5 --oneline`.

---

## Specification Authority Model

Three layers are distinguished throughout the specification:

```
IMPLEMENTED BEHAVIOR   what the code does
INTENDED CONTRACT      what contract/tests/decisions indicate
FROZEN SPECIFICATION   docs/LANGUAGE_SPEC.md (normative)
```

`docs/LANGUAGE_SPEC.md` is the **normative semantic authority**. `docs/contract.md`
is the **compatibility contract** and defers to the specification on semantics
(updated header). `docs/errors.md` remains the diagnostic-code reference.

Each specification section labels its content as a normative rule, an
implementation note, or a non-normative example.

---

## Sources of Semantic Evidence

| Source | Role | Problems found |
|---|---|---|
| `src/lex/mod.rs`, `src/lex/token.rs` | lexical authority | none |
| `src/parse/mod.rs` | grammar + parsing authority | none |
| `src/ast/mod.rs` | AST authority | `Expr::Tuple` carries no semantics beyond list sugar |
| `src/check/mod.rs` | static-check authority | none |
| `src/types.rs` | `Ty` and compatibility | none |
| `src/run/mod.rs`, `src/run/value.rs` | runtime authority | none |
| `src/stdlib/signatures.rs` | builtin/method registry | runtime `up`/`down` aliases absent from registry |
| `src/stdlib/mod.rs`, `src/stdlib/ext.rs` | builtin implementations | `json_*` bypass shared arity helper |
| `src/bridge/mod.rs` | Python boundary | none |
| `src/error.rs` | diagnostic codes | `E5003` unused; `E4099` internal |
| `src/lib.rs`, `src/main.rs`, `src/repl.rs` | entry points | none |
| `docs/contract.md` | compatibility contract | now defers to spec |
| `docs/grammar.md` | EBNF | consistent with parser |
| `docs/errors.md` | codes | `E4030` example was wrong → fixed |
| `tests/*` (152 `#[test]` across 13 files) | executable semantics | gaps noted below |
| `examples/*.aura` | executable docs | none |
| `README.md` | overview | one imprecise sentence → clarified |

---

## Language Features Covered

The specification covers every construct the parser can produce:

* lexical: identifiers, keywords, comments, newlines, semicolons, integer
  (decimal/hex/binary/octal), float, string, f-string, escape sequences;
* types: `int`, `float`, `bool`, `string`, `[T]`, `{string: V}`, `T | none`,
  named types;
* items: `fn`, `struct`, `enum`, `type`, `use`, top-level `let`,
  top-level expression, `pub`;
* statements: `let`/`let mut`, assignment, compound assignment, `return`,
  `throw`, `break`, `continue`, `while`, `loop`, `for`, `try`/`catch`/`finally`,
  expression statements, blocks;
* expressions: literals, names, f-strings, unary, binary, call, method, field,
  index, list, map, constructor, tuple/list sugar, lambda, pipeline, `if`,
  `match`, block;
* patterns: literal, binding, list, variant, wildcard, guards.

---

## Grammar Coverage

The grammar in §4 was checked against `Parser` for every production that
creates an AST node:

| Production | Parser function | AST | Verified |
|---|---|---|---|
| `fn_decl` | `fn_item` | `Item::Fn` | yes |
| `struct_decl` | `struct_item` | `Item::Struct` | yes |
| `enum_decl` | `enum_item` | `Item::Enum` | yes |
| `type_alias` | `alias_item` | `Item::Alias` | yes |
| `use_decl` | `use_item` | `Item::Use` | yes |
| `const_decl` | `const_item` | `Item::Const` | yes |
| `expr_stmt` | `item` default | `Item::Expr` | yes |
| `let_stmt` | `stmt_inner` | `Stmt::Let` | yes |
| `assign_or_expr` | `stmt_inner` | `Stmt::Assign`/`Expr` | yes |
| `return/throw/break/continue` | `stmt_inner` | `Stmt::*` | yes |
| `while/loop/for` | `stmt_inner` | `Stmt::*` | yes |
| `try_stmt` | `stmt_inner` | `Stmt::Try` | yes |
| `expr` / operators | `expr_bp`, `unary`, `postfix`, `atom` | `Expr::*` | yes |
| patterns | `pattern` | `Pattern::*` | yes |
| f-string | `fstring` | `Expr::FStr` | yes |

**Grammar drift found: none.** `docs/grammar.md` and the parser agree on all
productions, including the corrected pipeline note.

One reserved token is not reachable through any production: `Tok::As`
(recorded as implementation debt; `use a as b` fails to parse).

---

## AST Coverage

| AST construct | Parser source | Checker | Runtime | Spec section | Tests |
|---|---|---|---|---|---|
| `Lit` | literals | `infer` | value | §3.6, §5 | lexer/run |
| `Name` | identifiers | resolution | env/fn/native | §26 | run/checker |
| `FStr` | f-string | inner exprs | display concat | §3.6.4 | run |
| `Unary` | `-x`/`not x` | infer | neg/truthiness | §9.2 | run |
| `Binary` | operators | orderability | total dispatch | §9 | run/regressions |
| `Call` | `f(...)` | builtin sig / resolve | call | §15, §25 | run/regressions |
| `Method` | `r.m(...)` | method sig | dispatch | §24 | run/regressions |
| `Field` | `r.f` | receiver | field / 0-arg method | §24 | run |
| `Index` | `b[i]` | operands | get/set | §9.5 | run/boundaries |
| `List` | `[...]` | elements | `Value::List` | §20.1 | run |
| `Map` | `{k:v}` | entries | string keys enforced | §20.2 | run |
| `Construct` | `C(...)`/`C{...}` | full validation | struct/variant | §17, §18 | regressions |
| `Tuple` | `(a,b)` | elements | lowers to list | §21 | run |
| `Lambda` | lambdas | scope reset | closure | §15.4 | run/adversarial |
| `Pipe` | `x \|> y` (non-call) | operands | `call_value` | §23 | grammar |
| `If` | `if/else` | cond+branches | branch value | §14.2 | run |
| `Match` | `match` | patterns | first match | §19 | run |
| `Block` | `{...}` | scope | last value | §14 | run |

**No dead AST variants.** Every variant is constructed by the parser and has a
checker and runtime path. `Expr::Pipe` is produced only for a pipeline whose
right operand is not a call/method (for example `5 |> 3`), and the runtime
handles it.

---

## Type-System Coverage

The type universe was taken from `Ty` (`src/types.rs`) and `Value`
(`src/run/value.rs`). Properties (representation, equality, ordering, indexing,
iteration, callability, mutability) are tabulated in §5.3 and §13. The
compatibility matrix in §8 is grounded in `Ty::compatible_with` and the
regression tests.

Key facts frozen: `int`/`float` are not annotation-compatible; `Unknown` is
compatible with everything; struct types are nominal; maps are string-keyed;
`none` infers `Unknown`.

---

## Operator Matrix Verification

Every operator was exercised against every relevant operand kind on the built
binary; the §9 matrix records the observed result or diagnostic. Coverage
includes:

* `+ - * / % ^` across `int`, `float`, mixed, `string`, `list`, and
  incompatible pairs;
* `/` and `%` by zero for `int` and `float`, including `-0.0`;
* `^` with negative and huge exponents;
* `== !=` across all value kinds, including cross-type and `NaN`;
* `< <= > >=` across ordered kinds and the rejection of non-ordered kinds;
* unary `-` and `not`;
* short-circuit `and`/`or`.

No operator/type combination fell into `E4999`.

---

## Evaluation-Order Verification

Evaluation order was confirmed by side-effecting probes: argument order,
binary operand order, list/map element order, assignment order, `if`/`match`
selection, and short-circuit. All are strictly left-to-right deterministic,
as specified in §13. Determinism across runs is covered by
`tests/property.rs`.

---

## Control-Flow Verification

`return`, `break`, `continue`, `throw` propagation through blocks, loops,
`if`, `match`, lambdas, and function calls was exercised. `finally` precedence
was re-confirmed for `return`, `throw`, `break`, and `continue`, including the
rule that a control-flow signal in `finally` replaces the pending outcome.
`catch`-only-`throw` (runtime diagnostics are not catchable) was confirmed.

---

## Function and Closure Verification

Declaration, first-class values, arity, recursion, and mutual recursion were
confirmed. Closure capture by reference and mutation visibility were confirmed
by probe (`f()` and the outer binding both observe the mutation). The
specification records this as normative to the extent observed and does not
invent a lifetime model.

Static gaps (call-argument checking, field-type propagation) are documented as
limitations rather than guarantees.

---

## Struct Verification

Construction was verified for named and positional forms, correct types,
wrong types, unknown fields, missing fields, duplicate fields, extra
(positional) fields, nested structs, structs in lists, and structs returned
from functions. Checker and runtime agree on every case. Field assignment and
mutation through aliases were confirmed. Equality and display were confirmed.

---

## Enum Verification

Declaration, zero- and multi-payload variants, positional construction, named
rejection, payload arity and type checking, payload extraction by `match`,
equality, and display were confirmed. The zero-payload construction detail
(`A()` required in expression position; bare `A` is a name error) is frozen in
§18.2. Global tag uniqueness (`E2013`) was confirmed.

---

## Match Verification

Arm ordering, guards, bindings, list and variant patterns, duplicate-binding
rejection, unknown-variant rejection, non-exhaustiveness (`E4029`), and
control-flow propagation from arms were verified. The block requirement for
bare control-flow arm bodies is documented.

---

## Loop and Range Verification

`while`, `loop`, `for` over lists/strings/maps/ranges, `break`, `continue`,
`return` from loops, laziness of `range` iteration, empty and descending
ranges, negative bounds, and the materialization cap were verified. Maps
iterate in ascending key order.

---

## Pipeline Verification

`x |> f`, `x |> f(a)`, `x |> r.m(a)`, and `x |> c` were verified with
observable results. Left-associativity and precedence relative to binary
operators were verified. The desugaring is at parse time, and the surviving
`Expr::Pipe` path (non-call target) was verified to produce `E3001` when the
target is not callable.

---

## Built-in / Method Verification

The builtin and method inventories in §24 and §25 were generated from
`src/stdlib/signatures.rs` (the shared registry), not from stale prose.
Arity/type checking agreement between checker and runtime was verified. The
registry/implementation drift items (unreachable `up`/`down` aliases;
`json_*` bypassing the shared arity helper) are recorded as architecture debt,
not language surface.

---

## REPL Verification

A multi-submission session confirmed persistence of bindings, functions,
structs, enums, and aliases, and confirmed that a failed submission does not
corrupt the session. Output behavior (bare-expression echo; declarations
silent) and `:quit`/`:help` were confirmed. This is now normative (§29).

---

## Error Model Verification

Every documented code was cross-referenced with `src/error.rs`; the
reachability test (`tests/grammar.rs::every_documented_error_code_is_reachable`)
passes for all listed codes. Two discrepancies were found:

* `E4030`'s documented trigger (`let x = return 1`) is a parse error; the code
  is reachable via `let x = if true { return 1 } else { 2 }`. Fixed in
  `docs/errors.md`.
* `E5003` is defined but never produced; recorded as implementation debt and
  excluded from the normative surface.

`E4099` is internal (a throw crossing a call boundary) and never user-visible;
recorded as such.

---

## Resource-Safety Verification

The semantic AST-node limit (256), the parser recursion backstop, the call
frame limit (512), and the range materialization cap (10,000,000) were
confirmed by boundary probes at limit−1 / limit / limit+1 across flat chains,
nested lists, parentheses, maps, calls, blocks, `if`, lambdas, and `match`. No
input produced a host crash; all over-limit inputs produced `E1015`/`E1013`-
family diagnostics. Twenty malformed adversarial inputs produced no panic; a
deep-input sweep terminated deterministically.

The conceptual distinction between the semantic AST-node limit and the parser
recursion backstop is explicitly resolved in §31.

---

## Python Boundary Verification

Conversion in both directions was reviewed. Out-of-range Python integers are
rejected (`E4013`); non-string dict keys are rejected (`E5002`); opaque
objects become repr strings. `nan`/`inf` cross as floats. The `no-py` build
exposes the same names with `E5002`. Covered by `tests/python.rs` and
`tests/regressions.rs`.

---

## CLI / Library / REPL Consistency

`aura run` (program mode), `aura check`/`aura eval` (module mode), the library
functions, and the REPL share one parser, checker, and interpreter. The only
intended differences are the `main` requirement, output presentation, and REPL
persistence; these are documented in §28.5 and §29.

---

## Documentation Cross-Check

| Topic | LANGUAGE_SPEC | contract.md | implementation | tests | Status |
|---|---|---|---|---|---|
| primitives & compounds | §5 | §2 | `Ty`/`Value` | yes | RESOLVED |
| annotations enforced where provable | §6, §11 | §2 | `check` | yes | RESOLVED |
| transparent aliases | §7 | §10 | `resolve_type_expr` | yes | RESOLVED |
| operator semantics | §9 | §4/§7 | `binary`/`numeric` | yes | RESOLVED |
| `%` by zero | §10.6 | §7 | `numeric` | regressions | RESOLVED |
| nesting limit | §31 | §7 | parser/check/run | boundaries | RESOLVED |
| equality incl. functions | §11 | §7 | `equals` | contract/regressions | RESOLVED |
| ordering | §12 | §7 | `cmp_val` | regressions | RESOLVED |
| `finally` precedence | §14.6 | §7 | `Stmt::Try` | run | RESOLVED |
| struct construction | §17 | §3 | checker/runtime | regressions | RESOLVED |
| enum positional payloads | §18 | §3 | checker/runtime | regressions | RESOLVED |
| pipeline insertion | §23 | §4 | `desugar_pipe` | grammar | RESOLVED |
| no-paren method call | §24 | (was silent) | `Expr::Field` | run | RESOLVED (now specified) |
| tuple is list | §21 | grammar note | `eval_inner` | run | RESOLVED (now specified) |
| empty-map limitation | §20.3 | (silent) | parser | boundaries | RESOLVED (now specified) |
| REPL persistence | §29 | (silent) | `repl.rs` | repl | RESOLVED (now specified) |
| `pub`/`use` inert | §27 | §10 | parser | run | RESOLVED |
| `E4030` example | §30.4 | — | runtime | grammar | DOCUMENTATION DRIFT → fixed in errors.md |
| `E5003` unused | §34.3 | — | `error.rs` | — | IMPLEMENTATION DEBT (recorded) |
| `else`/`catch`/`finally` same line | §3.7 | (was silent) | parser | run | RESOLVED (now specified) |

---

## Specification Contradictions Found

During drafting, potential self-contradictions were actively searched:

* "optional annotations" vs "annotations statically enforced" — resolved by
  §6.2 (enforcement positions) and §6.4 (`Unknown`).
* "nesting depth" vs "AST-node budget" — resolved by §31 (two mechanisms,
  one diagnostic).
* "enum arguments may be named" vs "positional" — resolved by §18.2
  (positional; named rejected).
* "maps orderable" vs "not orderable" — resolved by §12 (not orderable).
* "functions by value" vs "by identity" — resolved by §11 (identity).
* "REPL persistent" vs "isolated" — resolved by §29 (persistent).
* "all errors at check time" vs runtime-only `Unknown` — resolved by §2.3 and
  §6.4.

No unresolved contradictions remain in the specification.

---

## Implementation Contradictions Found

None that required a code change. The implementation already matched the
intended semantics on every audited surface after the Semantic Closure commit.
The only discrepancies were documentation-level:

1. `docs/errors.md` `E4030` example was wrong → corrected.
2. `README.md` overstated static checking → clarified with a pointer to the
   specification.
3. `docs/contract.md` claimed sole normativity → now defers to
   `LANGUAGE_SPEC.md`.

One further **specification gap** was found while drafting and is now
normative: `else`, `catch`, and `finally` must appear on the same line as the
closing `}` of the block they follow (a newline before them is `E1006`). This
was implemented behavior that no document stated; it is now frozen in §3.7 and
noted in `docs/grammar.md`.

---

## Resolved Decisions

Frozen into §33: function identity equality; positional enum payloads;
transparent aliases; REPL persistence; `finally` precedence; pipeline receiver
insertion; struct validation and unknown-field rejection; float remainder by
zero; nesting/resource limits; list-sugar tuples; `{}` as a block; inert
`pub`/`use`; `try` requiring `catch`; global enum tags; no-paren method calls;
string-keyed ordered maps; reference semantics for lists/maps/structs.

---

## Deferred Decisions

No language decision is left unresolved in a way that blocks the
specification. Two areas are explicitly **not** generalized:

* **Closure lifetime/ownership.** Aura exposes no ownership concept; capture is
  by reference and environments are kept alive by reference counting. The
  specification freezes only what is observable and does not invent a lifetime
  model.
* **Empty-map spelling.** The limitation is recorded; choosing a new spelling
  would be a language-design change, which this phase does not make.

---

## Known Limitations

Recorded in §34: no empty-map literal; no tuple type; no `try` without
`catch`; `match` control-flow needs a block; no named/default/variadic
parameters; no nested named functions; no `else if`; no range step or
`for…else`; no lexicographic ordering for compound values; user-function call
arguments not statically checked; field reads infer `Unknown`; `if`/`match`/
lambda infer `Unknown`; `E5003` unused; `E4099` internal; `Tok::As` unused;
unreachable `up`/`down` runtime aliases; stale `E4030` example (now fixed).

---

## Remaining Architecture Debt

* Runtime method aliases `up`/`down` absent from the signature registry.
* `Tok::As` lexed but no production consumes it.
* `json_encode`/`json_decode` bypass the shared `arity()` helper.
* `stdlib::arity` retains a min/max fallback beside the registry.
* `Expr::Tuple` lowers to a list (semantically documented; AST node retained).
* `Expr::Field` conflates field access and zero-arg method call.
* `E5003` is a dead constant.
* `E4099` is a user-invisible signal multiplexed through the public code
  namespace.

None of these change language semantics; they are recorded for future work.

---

## Test Coverage

152 `#[test]` functions across 13 files; 16 `test result: ok` lines on a full
run (some files have multiple integration binaries / feature-gated suites).
Semantic-Closure regressions are all present and passing:

```
b1_float_remainder_by_zero_is_an_error
regression_nesting_limit_boundary
regression_parenthesis_nesting_is_bounded_not_a_crash
b3_struct_field_type_validation
b4_struct_unknown_and_missing_fields
regression_repl_struct_persistence
regression_repl_enum_persistence
regression_repl_alias_persistence
b6_enum_named_argument_contract
f01_arithmetic_matrix  (the earlier 151-test baseline is subsumed)
```

The 151-test baseline from the previous phase is fully retained and expanded
by the Semantic Closure tests.

---

## Verification Commands

Executed against this working tree:

```
cargo fmt --all -- --check                                            clean
cargo test --all-features                                             all pass
cargo test --no-default-features --features cli,repl,json,regex,time  all pass
cargo clippy --all-targets --all-features -- -D warnings               clean
cargo clippy --no-default-features --features cli,repl,json,regex,time -- -D warnings   clean
cargo clippy --no-default-features --features cli -- -D warnings       clean
PROPTEST_CASES=2048 cargo test --test property --all-features          pass
cargo test --test contract --test examples --all-features              pass
cargo build --release --no-default-features --features cli,repl,json,regex,time   ok
cargo +nightly miri test --lib --no-default-features --features cli    pass
git diff --check                                                       clean
```

MSRV (1.83) was **not** verified in this environment: the toolchain is not
installed. No new API was introduced by this phase (documentation only), so
the existing MSRV posture is unchanged and CI's MSRV job remains the
authority.

Additional semantic probes (temporary, removed afterward) confirmed: tuple
lowering, empty map vs block, no-paren method calls, short-circuit,
closure capture, evaluation order, map ordering, NaN behavior, integer
division/remainder signs, range edges, negative indexing, reference semantics,
zero-payload enum construction, alias chaining, and struct field diagnostics.

---

## Semantic Readiness Assessment

| Area | Status |
|---|---|
| Grammar | FROZEN |
| AST | FROZEN |
| Types | FROZEN |
| Operators | FROZEN |
| Evaluation order | FROZEN |
| Control flow / `finally` | FROZEN |
| Functions | FROZEN |
| Closures | FROZEN WITH DOCUMENTED LIMITATION (no lifetime model) |
| Structs | FROZEN |
| Enums | FROZEN |
| Match | FROZEN |
| Loops / ranges | FROZEN |
| Pipeline | FROZEN |
| Built-ins / methods | FROZEN |
| REPL | FROZEN |
| Errors | FROZEN (one doc example corrected; `E5003` excluded) |
| Resource safety | FROZEN |
| Python boundary | FROZEN |
| CLI / library / REPL | FROZEN |
| Empty-map spelling | FROZEN WITH DOCUMENTED LIMITATION |
| User-function static call checking | FROZEN WITH DOCUMENTED LIMITATION |

**Overall:** the language behavior is sufficiently specified and stable to
serve as the contract for future feature development. The specification is
coherent, evidence-backed, and contains no unresolved contradictions. The
remaining items are documented limitations and architecture debt, not
semantic uncertainty.
