# Feature 004 — Destructuring `let`
## Implementation Report

**Baseline:** `d9aed732f0d1f47cd83a1853dd918b986d123e4c`
(`feat: add static field-type propagation`, Feature 003).
**Branch:** `rewrite/v3-rust`.
**Status:** Implemented; focused and full validation green.
**Design:** `docs/FEATURE_004_DESIGN.md` (frozen).
**Specification:** `docs/LANGUAGE_SPEC.md` §4.4, §4.7, §16.1.

---

## Summary

`let` now accepts a restricted destructuring pattern:

```
let [a, b] = pair
let Ok(x) = result
let [Some(a), [b, c]] = nested
```

Ordinary `let x = e`, `let mut x = e`, and `let x: T = e` keep the exact
existing `Stmt::Let` path and semantics. Genuine destructuring uses a new
additive `Stmt::LetPattern`. The feature is checker/runtime-local and changes
no grammar token, no pattern variant, no resource limit, and no Feature 003
semantics.

---

## Implementation Scope and Files Changed

### `src/ast/mod.rs`

Added an additive sibling statement variant:

```rust
LetPattern { pattern: Pattern, value: Expr, span: Span }
```

The existing `Stmt::Let` is untouched, so every existing consumer of the
ordinary form is unchanged.

### `src/parse/mod.rs`

* The `let` branch now parses a **`let`-specific pattern** via a new
  `let_pattern` method (separate from the general `pattern` used by `for` and
  `match`). Accepted forms: `Bind`, `List`, `Variant`, and their nesting.
* A lone `Pattern::Bind` is routed through the **existing** ordinary path
  (optional `: type`, `=` required, `Stmt::Let`), preserving `let x`,
  `let mut x`, and `let x: T` byte-for-byte.
* Genuine destructuring rejects `mut` and `: type` with `E1006`, and requires
  `=`.
* Literal and `none` patterns in a `let` pattern are rejected with `E1006`
  (including nested positions), while a reserved word in binding position
  keeps its dedicated `E1009`.
* `check_stmt_depth` gained an arm for the new variant so AST-depth
  enforcement covers its initializer.
* The new `let_pattern` recursion is guarded by the existing `enter()/leave()`
  parser backstop, so an extreme pattern reports `E1015` instead of
  overflowing (see "Resource Safety" below).

The general `pattern` parser is **not modified**.

### `src/check/mod.rs`

New `Stmt::LetPattern` arm:

* checks the initializer expression (`self.expr(value)`);
* reuses `self.check_pattern(pattern)` (duplicate binding `E2014`, unknown
  variant tag `E3002`);
* declares every `pattern.bindings()` name immutably (`E2007` on same-scope
  redeclaration).

No type inference is performed and no `value_types` entry is created, so
destructured names remain `Ty::Unknown`. Feature 003's `Expr::Field` inference
is untouched.

### `src/run/mod.rs`

New `Stmt::LetPattern` arm implements **atomic** destructuring:

1. evaluate the RHS exactly once;
2. bind into a temporary child scope `env.child()` via the existing
   `bind_pattern`;
3. on success, transfer every `pattern.bindings()` name into the current
   environment as an immutable binding.

`bind_pattern`, `match_pattern`, `Env`, and `Value` are **not modified**.

### `src/repl.rs`

The statement path captures whether execution succeeded. On success, a
destructuring `let` persists each bound name individually as
`GlobalDecl::Binding { name, mutable: false, ty: None }`; on failure it
persists nothing. Ordinary `let` persistence (including Feature 003 annotation
`ty`) is unchanged.

---

## Binding and Scope Rules

* Names bound are exactly `Pattern::bindings()` (every `Bind` except `_`).
* All are immutable and use the same scope as an ordinary `let`: visible to
  later statements in the enclosing block, available to nested blocks, and
  discarded when the block ends.
* Same-scope redeclaration is `E2007`; outer-scope shadowing is allowed.
* Binding order is not observable (no user code runs during binding).

## Evaluation Order

The RHS is evaluated once, before any binding, under strict left-to-right
evaluation. A control-flow signal raised during RHS evaluation propagates
before any binding, exactly as for an ordinary `let`.

## Failure Atomicity

Destructuring `let` is atomic. Matching occurs in a temporary child scope; if
it fails, the diagnostic from `bind_pattern` (`E3001`) propagates and the real
environment is untouched. Transfer to the real environment happens only after
a successful match, so no partial binding can leak. This was achieved
**without modifying `bind_pattern` or `Env`**.

---

## Type Behavior

Destructured names have no static type; they are `Ty::Unknown` for the
checker. Consequently Feature 001 argument checking, Feature 002 named
arguments, and Feature 003 field inference treat them conservatively. No
speculative inference is introduced.

---

## Error Model

No new error codes.

| Condition | Code |
|---|---|
| Literal/`none` in `let` pattern, annotation on destructuring, `mut` on destructuring | `E1006` |
| Reserved word in binding position | `E1009` |
| Duplicate name in one pattern | `E2014` |
| Same-scope redeclaration | `E2007` |
| Unknown variant tag | `E3002` |
| List arity / non-list / variant / non-variant at runtime | `E3001` |
| Missing initializer | `E2005` |

---

## REPL Behavior

Each successfully destructured name persists individually and unannotated
(`ty: None`), so it is `Unknown` in later submissions. A failed destructuring
persists nothing and does not alter prior session state. Annotated ordinary
`let` retains Feature 003's type persistence.

---

## Compatibility

Destructuring `let` is an **additive syntax extension**:

* `let x = expr`, `let mut x = expr`, `let x: Type = expr` — unchanged;
* `for` and `match` — unchanged (general pattern parser untouched);
* general pattern parsing — unchanged;
* pipeline, closures, aliases, structs, enums, lists — unchanged;
* Feature 001, 002, 003 — unchanged.

No previously valid program changes meaning.

---

## Tests Added

* `tests/parser.rs`: supported forms; identifier-is-ordinary; literal/`none`
  rejection; annotation/`mut` rejection; reserved-word `E1009`;
  initializer required.
* `tests/checker.rs`: duplicate `E2014`; same-scope redeclaration `E2007`;
  unknown variant `E3002`; all names declared; names stay `Unknown`; shadowing.
* `tests/run.rs`: list/nested-list/variant/nested-variant-list binding;
  `_` binds nothing; mismatch `E3001`; RHS evaluated once; failure is `E3001`;
  scope; nested block; shadowing.
* `tests/repl.rs`: each name persists; failure persists nothing; arity failure
  persists nothing; Feature 003 annotated-`let` persistence unchanged.
* `tests/regressions.rs`: Feature 001/002/003 integration; pipeline
  initializer; ordinary `let`/`for`/`match` preserved.
* `tests/property.rs`: bound names equal `Pattern::bindings()`; failure leaves
  no partial binding; ordinary identifier `let` unchanged.
* `tests/boundaries.rs`: deep destructuring pattern bounded with `E1015`.

### Results

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | OK |
| `cargo check --all-features` | OK |
| `cargo test --all-features` | 201 passed, 0 failed |
| `cargo test --no-default-features` | 195 passed, 0 failed |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| clippy no-py (`cli,repl,json,regex,time`) / cli-only | clean |
| `PROPTEST_CASES=2048 cargo test --test property --all-features` | 18 passed |
| contract + examples | 11 passed |
| release no-py build + smoke (run & REPL) | `42`, `3` |
| Miri (`--lib`, no-default-features, cli) | OK |
| `git diff --check` | clean |

---

## Resource Safety

No resource constant changed: `MAX_AST_DEPTH = 256` (`E1015`),
`MAX_PARSE_DEPTH` backstop, `MAX_CALL_FRAMES = 512` (`E4011`), range cap
`10,000,000` (`E4013`). The new `let_pattern` recursion is guarded by the
existing parser `enter()/leave()` backstop, so a pattern far past the backstop
reports `E1015` and never overflows the host stack.

**Known pre-existing limitation (not introduced by Feature 004):** the general
`pattern()` parser used by `match` and `for` does **not** call
`enter()/leave()`, so an extreme pattern in those constructs can still exhaust
the host stack. This behavior predates Feature 004; the new `let_pattern` path
does not share it. It is recorded here as an out-of-scope observation.

---

## Known Non-Goals

Annotated destructuring; destructuring parameters/`catch` bindings/top-level
constants; `mut` on a pattern; literal/`none`/range patterns in `let`;
destructuring structs; `if let`/`while let`; rest patterns; any type inference
for destructured names; changes to `for`, `match`, `bind_pattern`,
`match_pattern`, `Env`, `Value`, stdlib, Python bridge, or Feature 003.

---

## Unchanged by Design

`src/stdlib/`, `src/bridge/`, `src/lib.rs`, `src/tools/`, `Value`, `Env`,
`bind_pattern`, `match_pattern`, Feature 003 field inference, all resource
constants, and `README.md` are untouched. No new error codes.
