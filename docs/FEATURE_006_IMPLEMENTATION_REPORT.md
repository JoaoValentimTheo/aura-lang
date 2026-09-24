# Feature 006 — `else if`
## Implementation Report

**Baseline:** `60d647645dfcdbeefbc6658622ab18a591c14e29`
(`fix: harden pattern validation and recursion`, H1).
**Branch:** `rewrite/v3-rust`.
**Status:** Implemented; focused and full validation green.
**Design:** `docs/FEATURE_006_DESIGN.md` (frozen).
**Specification:** `docs/LANGUAGE_SPEC.md` §4.5 (amended in spec review).

---

## Summary

`else if` is now valid Aura syntax. It is a **parser-level syntax extension**:
`else` already accepts an expression and `if` is an expression, so a chained
`else if` parses into nested existing `Expr::If` nodes. No AST node, checker
rule, or runtime behavior was added. `E1014` is retired.

---

## Exact Parser Change

**File:** `src/parse/mod.rs`, `Parser::atom`, `Tok::If` arm.

Removed only the explicit rejection guard:

```rust
// before
let els = if self.eat(&Tok::Else) {
    if matches!(self.at(), Tok::If) {
        return Err(Diag::new(
            codes::ELSE_IF,
            "`else if` is not part of Aura; use `else { if ... }` or `match`",
            self.span(),
        ));
    }
    Some(Box::new(self.expr()?))
} else {
    None
};

// after
let els = if self.eat(&Tok::Else) {
    // `else` accepts an expression, and `if` is an expression, so
    // `else if B { … }` parses as the nested `Expr::If`
    // `else { if B { … } }` with no special handling.
    Some(Box::new(self.expr()?))
} else {
    None
};
```

No other parser change. The ordinary expression path (`expr_bp → unary →
postfix → atom`) parses the following `if` because `atom` handles `Tok::If`.
Ordinary `if`, `if … else`, and `else <arbitrary expression>` are unchanged.

**Newline behavior is unchanged.** `else` must remain on the closing `}` line
(§3.7) and `expr()` does not skip newlines, so:
* newline before `else` → `E1006`;
* newline between `else` and `if` → `E1006`.

---

## Nested `Expr::If` Representation

No new AST node and no `ElseIf` variant. For example:

```
if A { X } else if B { Y } else if C { Z } else { W }
```

parses to:

```
Expr::If(
    A, [X],
    Some(Expr::If(
        B, [Y],
        Some(Expr::If(C, [Z], Some(Expr::Block([W])))),
    )),
)
```

The chain is the `els` spine of nested `Expr::If` nodes; a final `else { … }`
is the existing `Expr::Block`. Verified by parser tests that inspect the
nesting shape.

---

## E1014 Retirement

`E1014` (`ELSE_IF`) is removed from the active language; it is not repurposed.
The parser producer and the constant are gone, and no active test or document
requires it.

- `src/error.rs`: removed `pub const ELSE_IF: u16 = 1014;` and its doc comment.
- `src/parse/mod.rs`: removed the producer (above).
- `src/check/mod.rs`: reworded the module doc, which used `else if` as the
  example of a forbidden construct; it now cites `break` outside a loop.
- Active docs (`docs/LANGUAGE_SPEC.md`, `docs/contract.md`, `docs/grammar.md`,
  `docs/errors.md`) were already amended in the spec-review phase and remain
  consistent.
- Active tests that asserted `E1014` were removed/replaced during spec review.
- Historical documents remain historical.

Search confirms no active `ELSE_IF` / `codes::ELSE_IF` / `E1014` reference
remains in `src/` or `tests/`.

---

## Checker / Runtime Semantics

**Unchanged.** The checker reuses the existing `Expr::If` case (check the
condition, the `then` block, and the `els` expression); the runtime reuses the
existing `Expr::If` evaluation. Because the AST is the existing nested form,
scoping, shadowing, result values, `none`, `return`, `throw`, `break`,
`continue`, and `try/finally` all use the frozen semantics. Branch-join
inference remains unchanged: `infer(Expr::If) = Ty::Unknown`.

---

## Compatibility

* Existing `if A { X }` and `if A { X } else { Y }` programs are unchanged.
* Existing `else <non-if expression>` (e.g. `else 2 + 3`, `else match …`)
  still works.
* Programs previously rejected with `E1014` (`if … else if …`) are now valid.
* This is **additive syntax** plus removal of the `E1014` diagnostic. No
  previously valid program changes meaning; no runtime semantic change.
* Existing `#` line comments are unaffected and remain the only comment form.

---

## Resource Limits

No resource constant changed: `MAX_AST_DEPTH` (256), `MAX_PARSE_DEPTH`
(2048), `MAX_CALL_FRAMES` (512), range cap (10,000,000). An `else if` chain
nests existing `Expr::If` nodes and is bounded by the existing parser
`enter()/leave()` backstop and the iterative `check_expr_depth` walk, producing
`E1015` past the limit — verified to run for a modest chain and to report
`E1015` (not a host overflow) for a clearly-over chain. The exact boundary was
measured empirically rather than assumed.

---

## Tests Added

**`tests/parser.rs`** (6)
* `else_if_parses_to_nested_if`
* `multiple_else_if_parses_as_deep_nesting`
* `else_if_final_else_is_optional`
* `else_if_newline_rules_are_unchanged`
* `malformed_else_if_is_rejected`
* `ordinary_if_else_is_unchanged`

**`tests/checker.rs`** (2)
* `else_if_conditions_are_checked`
* `else_if_break_continue_loop_context_is_unchanged`

**`tests/run.rs`** (8)
* `else_if_single_and_multiple_clauses`
* `else_if_without_final_else_yields_none`
* `else_if_condition_evaluation_order`
* `else_if_branch_scope_and_shadowing`
* `else_if_return_propagates`
* `else_if_throw_propagates`
* `else_if_break_and_continue`
* `else_if_try_finally_interaction`

**`tests/boundaries.rs`** (1)
* `deep_else_if_chain_is_bounded`

**`tests/property.rs`** (1)
* `else_if_equivalent_to_nested_if` — differential: explicit chain vs nested
  `if` produce identical output.

### Validation

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | OK |
| `cargo check --all-features` | OK |
| `cargo test --test parser` | 31 passed |
| `cargo test --test checker` | 30 passed |
| `cargo test --test run` | 46 passed |
| `cargo test --test boundaries` | 23 passed |
| `cargo test --test property` | 20 passed |
| `cargo test --all-features` | all targets green, 0 failed |
| `git diff --check` | clean |

---

## Subtle Newline Behavior

`else` continued to accept an expression, so removing the guard did not change
newline handling. The mandated observations hold: a newline before `else`, or
between `else` and `if`, is `E1006`.

---

## Comments

Out of scope and unchanged. Existing `#` line comments are the only comment
form; block comments remain future work.

---

## Files Changed

Source: `src/parse/mod.rs`, `src/error.rs`, `src/check/mod.rs` (module doc).
Tests: `tests/parser.rs`, `tests/checker.rs`, `tests/run.rs`,
`tests/boundaries.rs`, `tests/property.rs`.
Docs: `docs/FEATURE_006_IMPLEMENTATION_REPORT.md`.
Docs (from spec review, unchanged here): `docs/LANGUAGE_SPEC.md`,
`docs/contract.md`, `docs/grammar.md`, `docs/errors.md`.

**Unchanged:** `src/ast/mod.rs`, `src/run/mod.rs` (logic), `src/lex/`,
`src/repl.rs`, `src/stdlib/`, `src/bridge/`, resource constants, and
F003/F004/F005/H1 implementation.
