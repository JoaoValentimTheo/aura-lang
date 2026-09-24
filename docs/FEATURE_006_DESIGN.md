# Feature 006 — `else if`

## Status

Design (pre-implementation), **normative and implementation-ready**.

**Baseline:** `60d647645dfcdbeefbc6658622ab18a591c14e29`
(`fix: harden pattern validation and recursion`, H1).
**Branch:** `rewrite/v3-rust`.
**Semantic authority:** `docs/LANGUAGE_SPEC.md` (frozen).
**Scope:** `else if` only. Block comments are out of scope (see §11).

No open questions remain in this document. Every decision is resolved from the
existing implementation.

---

## 1. Exact Syntax

**Normative rule.** An `if` expression MAY be followed by zero or more
`else if` clauses and an optional final `else` clause:

```
if A { X }
else if B { Y }
else if C { Z }
else { W }
```

**Normative rule.** Each clause condition is followed by a block. The final
`else` (when present) is followed by a block or any expression, exactly as the
current `else` production permits.

**Normative rule.** `else if` uses two existing keywords, `else` and `if`,
with no new token, operator, or keyword. There is no `elif` or `elseif`.

**Normative rule (same-line requirement).** The `else` keyword MUST appear on
the same line as the closing `}` of the block it follows. This is the existing
rule for `else` (§3.7) and is unchanged. Consequently `else` and the following
`if` are on the same line: `} else if B {`.

**Normative rule (newlines).** `else if` does not alter newline significance.
The parser does not skip newlines before `else` or after it (other than the
single line on which `else if` appears). Therefore:

* `} else if B {` — accepted (same line).
* `}\nelse if B {` — `E1006` (newline before `else`), exactly as for a plain
  `else` today.
* `} else\nif B {` — `E1006` (newline between `else` and `if`), because the
  `else` operand is parsed as an expression and newlines are not skipped in
  expression position.

**Normative rule (optional final else).** If no condition matches and there is
no final `else`, the `if` expression yields `none`, exactly as an `if` without
`else` does today.

---

## 2. AST Representation

**Normative rule.** `else if` introduces **no new AST node**. The existing
`Expr::If(Box<Expr>, Vec<Stmt>, Option<Box<Expr>>, Span)`
(`src/ast/mod.rs`) represents chained `else if` by nesting in the `els` slot.

The statement

```
if A { X } else if B { Y } else if C { Z } else { W }
```

is represented exactly as the parser's existing `else` path would build:

```
Expr::If(
    A,
    [X],
    Some(Expr::If(
        B,
        [Y],
        Some(Expr::If(
            C,
            [Z],
            Some(Expr::Block([W])),
        )),
    )),
)
```

That is, `if A { X } else if B { Y } else { Z }` is represented as
`Expr::If(A, [X], Some(Expr::If(B, [Y], Some(Expr::Block([Z])))))` — the
`else` branch holds the next `if` expression.

**Normative rule.** A final `else { … }` is the existing `Expr::Block`
(produced because `else` parses an expression and `{ … }` parses as a block
expression). A final `else expr` (non-block) is the same expression the
current grammar already permits. `else if` does not change how an `else`
operand is represented.

**Normative rule.** The `Span` of each nested `Expr::If` is the span of its
`if` keyword, as today.

---

## 3. Semantics

**Normative rule.** An `else if` chain is **definitionally** the nested `if`
shown in §2. Because the parser constructs exactly that nested `Expr::If`
shape, the semantics are the frozen semantics of the existing `Expr::If` and
require no separate specification.

Consequently, for `if A { X } else if B { Y } else { Z }` versus
`if A { X } else { if B { Y } else { Z } }`, all of the following are
identical:

* **Evaluation order.** Conditions are evaluated in source order, each at
  most once, and each later condition only when all earlier conditions were
  falsy. Runtime `Expr::If` (`src/run/mod.rs`) evaluates `cond` once; the
  `els` operand (the nested `if`) is evaluated only on the falsy branch.
* **Branch selection.** The first truthy condition selects its block; if none
  is truthy the final `else` (or `none`) is used. Identical AST ⇒ identical
  selection.
* **Scopes.** A selected block runs via `exec_block(then, env, true)`, i.e. a
  child scope, exactly as today. Each clause's block is independently scoped.
* **Shadowing.** Same block-scope rules; a `let` inside a clause shadows an
  outer binding for that block only.
* **Result value.** The value of the selected block (its last statement) is
  the value of the `if`, as today.
* **`none`.** A missing final `else` on a false chain yields `Value::None`,
  exactly as `if` without `else`.
* **`return`.** A `return` in a selected block propagates via `Ctl::Return`
  through the nested `Expr::If` unchanged.
* **`throw`.** Propagates via `Ctl::Throw`; not caught by `if`.
* **`break` / `continue`.** Propagate via `Ctl::Break` / `Ctl::Continue`;
  loop-context legality is still enforced by the checker (`E2015`).
* **`try` / `finally`.** Orthogonal; `finally` runs on every exit path
  through either form identically, because the control-flow signals are the
  same.
* **Nested conditionals.** Arbitrary nesting of `if` inside clause blocks, and
  of `else if` within an `else if` body, is the ordinary existing recursion.

**Normative rule (no special evaluation).** No condition is evaluated twice,
no clause is reordered, and no new control-flow semantics are introduced.

---

## 4. Parser

**Current behavior.** In `Parser::atom`'s `Tok::If` arm (`src/parse/mod.rs`),
after parsing `if cond block`, the parser eats `else` and then explicitly
rejects a following `if` with `E1014`:

```rust
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
```

**Minimum change.** Delete the `if matches!(self.at(), Tok::If) { … }` guard
only. The existing `Some(Box::new(self.expr()?))` then parses the following
`if` as an ordinary expression, producing the nested `Expr::If` of §2. No
other parser change is made.

**Normative rule.** Ordinary `if`/`else`/`else expr` behavior is unchanged;
`else` still parses an arbitrary expression, and `if` is an expression.
`else if` is therefore not a special grammar construct but the natural
composition of two existing constructs.

**Normative rule.** The same-line `else` rule and the newline-in-expression
behavior are unchanged (see §1).

**Grammar production.** The grammar already admits this: `if_expr = "if" expr
block [ "else" expr ]` and `if_expr` is an `expr`, so `else <if_expr>` is
already grammatical. The explicit rejection was parser policy, not the
grammar. No grammar production changes; only the accompanying notes (§13).

---

## 5. E1014 Retirement

**Normative rule.** `E1014` (`ELSE_IF`) is **removed** from the language. It
is not repurposed. Because `else if` becomes valid, `E1014` loses its only
producer, and the repository requires every documented error code to remain
reachable (`docs/LANGUAGE_SPEC.md` §30.4 and
`tests/grammar.rs::every_documented_error_code_is_reachable`, which iterates
`error_samples()` and also asserts each code appears in `docs/errors.md`).
Leaving `E1014` defined but unreachable would violate that rule.

**Verified references to update at implementation time (not now):**

| Location | Current content | Action |
|---|---|---|
| `src/error.rs:99-100` | `pub const ELSE_IF: u16 = 1014;` + doc comment | remove the constant |
| `src/parse/mod.rs:1305-1306` | guard emitting `codes::ELSE_IF` | remove the guard |
| `src/check/mod.rs:5` | module doc: "every forbidden construct (e.g. `else if`) is rejected by the parser" | reword to a currently-valid example (e.g. `else if` no longer an example) |
| `tests/parser.rs:67-70` | test `else_if_is_rejected` asserting `ELSE_IF` | replace with an `else if` acceptance test |
| `tests/grammar.rs:110-113` | `error_samples()` entry `(codes::ELSE_IF, …)` | remove the entry (and its doc assertion dependency) |
| `tests/contract.rs:82-86` | `r3_else_if_is_rejected` asserting `ELSE_IF` | replace with an acceptance test |
| `tests/contract.rs:164` | `codes::ELSE_IF` in the `r10` distinct-code list | remove the entry |
| `docs/LANGUAGE_SPEC.md:105` | Parse-phase row mentions `else if` | remove `else if` from the diagnostic list |
| `docs/LANGUAGE_SPEC.md:466-467` | normative rule rejecting `else if` as `E1014` | replace with the `else if` rule |
| `docs/LANGUAGE_SPEC.md:1966` | §30.2 code table row `E1014` | remove the row |
| `docs/LANGUAGE_SPEC.md:2176` | §34.1 "`else if` is not part of the language" | remove the bullet |
| `docs/contract.md:143` | "`else if` is **not** allowed" | replace with the rule |
| `docs/contract.md:158` | check-time table row `E1014` | remove the row |
| `docs/grammar.md:125-126` | note "`else if` does not exist … `E1014`" | replace with a note that `else if` is supported |
| `docs/errors.md:21` | `E1014` row | remove the row |

**Normative rule.** After removal, the documented/implemented error-code set
is 1001, 1002, 1003, 1004, 1006, 1009, 1015, and the E2xxx–E5xxx codes; no
`E1014` remains defined, produced, or documented.

**Non-normative/historical notes.** `docs/SEMANTIC_FREEZE_AUDIT.md`,
`docs/LANGUAGE_SPEC_CONFORMANCE_REPORT.md`, and `docs/FEATURE_ROADMAP.md`
reference `E1014`/`else if`; these are historical/planning documents and are
updated only if and when the project updates them (the roadmap is explicitly
out of scope for this step). They do not affect reachability tests.

---

## 6. Static Checking

**Normative rule.** The checker reuses the existing `Expr::If` handling
(`src/check/mod.rs`): check the condition, check the `then` block, and check
the `els` expression if present. Because `else if` is nested `Expr::If`, it is
checked by the same recursion with no new code.

**Normative rule.** Branch-join inference is unchanged: `infer(Expr::If)` and
`infer(Expr::Match)` remain `Ty::Unknown` (§34.2). `else if` does not introduce
or require any type inference.

**Normative rule.** No new static diagnostic is introduced.

---

## 7. Runtime

**Normative rule.** No runtime change. The existing `Expr::If` evaluation
(`src/run/mod.rs`) is authoritative: evaluate the condition once; if truthy,
execute the `then` block in a child scope; otherwise evaluate the `els`
expression (the next `if`), or yield `none` when absent.

---

## 8. Resource Safety

**Normative rule.** Multiple `else if` clauses increase AST nesting naturally
as nested `Expr::If` nodes. The existing depth accounting already covers this:

* The parser's `Parser::enter()/leave()` backstop (`MAX_PARSE_DEPTH`, 2048) and
  the post-parse `check_expr_depth` walk (`Expr::If` pushes `cond` and `els` at
  the same depth, and `then` via `check_stmt_depth`) bound the nesting.
* A chain of N clauses is rejected with the existing `E1015` (`NESTING`) once
  it exceeds the AST/parse depth limit, exactly as an equivalent nested `if`.

**Normative rule.** No resource constant changes. `MAX_AST_DEPTH` (256),
`MAX_PARSE_DEPTH` (2048), `MAX_CALL_FRAMES` (512), and the range cap
(10,000,000) are unchanged.

---

## 9. Compatibility

**Normative rule.** Existing `if A { X }` and `if A { X } else { Y }`
programs are unchanged in syntax, checking, and behavior.

**Normative rule.** Programs previously rejected with `E1014`
(`if … else if …`) become valid, using the semantics of §3.

**Classification.** This is:

* **additive syntax** (a new accepted source form), plus
* **removal of the `E1014` diagnostic** (a documented-code removal required by
  the reachability rule).

It is **not** a semantic change to any previously valid program, and **not** a
breaking change. No existing valid program changes meaning. `else if` is
equivalent to a form (`else { if … }`) that was already valid and specified, so
no new runtime semantics are introduced.

---

## 10. Comments

**Normative rule.** Existing line comments (`#`, §3.5) are unaffected.
Comments are removed in the lexer before parsing and therefore have no
interaction with `else if`. Comments MAY appear before a statement, after a
statement, inside any block, and at end of line.

**Normative rule.** `else # comment` followed by a newline then `if` is
**not** valid, because the `#` comment is stripped but the newline remains
significant in expression position; this is the ordinary `else` newline rule
and is not a comment-specific restriction.

**Normative rule.** Block comments are **not** part of Feature 006 and remain
future work; the spec sentence "There are no block comments" (§3.5) is
unchanged by this feature.

---

## 11. Testing

Focused tests (to be added at implementation time):

**Parser (`tests/parser.rs`)**
* one `else if` parses to nested `Expr::If`;
* multiple `else if` clauses parse to the expected nested shape;
* final `else` and no-final-`else` both parse;
* newline before `else` → `E1006`; newline between `else` and `if` → `E1006`;
* `else if` with a missing block → `E1006`;
* existing `if`/`else` regression;
* the former `else_if_is_rejected` test replaced by acceptance.

**Checker (`tests/checker.rs`)**
* `else if` conditions are checked; unknown names in later clauses → `E2003`;
* `break`/`continue` legality through `else if` unchanged (`E2015`).

**Runtime (`tests/run.rs`)**
* one `else if`; multiple `else if`; final `else`; no final `else` (`none`);
* condition evaluation order with side effects (each evaluated at most once,
  later only if earlier falsy);
* branch scope and shadowing;
* expression value;
* nested conditionals;
* `return` / `throw` through `else if`;
* `break` / `continue` within a loop through `else if`;
* `try/finally` interaction;
* malformed chains rejected.

**Contract (`tests/contract.rs`)**
* replace `r3_else_if_is_rejected` with an acceptance assertion;
* remove `codes::ELSE_IF` from the distinct-code list.

**Grammar reachability (`tests/grammar.rs`)**
* remove the `E1014` sample from `error_samples()`; keep the reachability and
  `errors_doc_lists_every_code` tests passing with the reduced set.

**Boundary (`tests/boundaries.rs`)**
* a deep `else if` chain near the AST depth boundary accepts, and one past it
  reports `E1015`, never a host overflow — matching the equivalent nested `if`.

**Comments (`tests/lexer.rs`)**
* existing line-comment coverage continues to pass; no new comment tests are
  required for Feature 006.

---

## 12. Property / Differential Testing

**Normative rule.** A lightweight differential property is warranted and cheap:
for generated branch conditions with side-effecting conditions, assert that an
explicit `else if` chain and its equivalent nested `if` construction produce
the **same stdout and the same diagnostic code**. Because the parser builds the
identical AST, this property is expected to hold structurally; it guards
against any future divergence in parsing or desugaring.

Implemented with the existing `proptest` infrastructure (`tests/property.rs`),
generating small condition/effect programs. An N-clause depth property may
assert that a chain reports `E1015` at the same threshold as the equivalent
nested construction.

No property is added for comments (line comments are already covered by the
lexer test and are unaffected).

---

## 13. Documentation Migration (at implementation time)

Normative/authoritative documents to update (not modified during this design
phase):

* `docs/LANGUAGE_SPEC.md`: §4.5 (`else if` rule), §30.2 (remove `E1014` row),
  §34.1 (remove the "`else if` is not part of the language" bullet), and the
  Parse-phase diagnostic list at §105.
* `docs/grammar.md`: the `else if` note; the `if_expr` production itself is
  unchanged (it already admits `else expr`).
* `docs/contract.md`: §5 statements bullet and the check-time `E1014` row.
* `docs/errors.md`: remove the `E1014` row.
* `src/error.rs`, `src/parse/mod.rs`, `src/check/mod.rs` (module doc): code and
  comment changes.
* `tests/parser.rs`, `tests/contract.rs`, `tests/grammar.rs`: test updates.

Historical documents (`docs/SEMANTIC_FREEZE_AUDIT.md`,
`docs/LANGUAGE_SPEC_CONFORMANCE_REPORT.md`, `docs/FEATURE_ROADMAP.md`) are not
part of the implementation deliverable; the roadmap is explicitly out of scope.

---

## 14. Out of Scope

* Block comments (and any change to comment syntax).
* Branch-join inference or any new static typing.
* New conditional semantics beyond the nested-`if` equivalence.
* New AST nodes.
* Loop changes.
* Pattern changes.
* New expression forms.
* Changes to `match`.
* Resource-limit changes.
* Repurposing `E1014`; it is removed.
* Roadmap/documentation-history rewrites.

---

## Resolved Design Decisions

1. **Syntax:** `if A { X } else if B { Y } … [else { Z }]`; no new keyword.
2. **Same-line rule:** `else` on the closing `}` line; `else if` on one line;
   newline before `else` or between `else` and `if` is `E1006`.
3. **Optional final else:** absent ⇒ false chain yields `none`.
4. **AST:** nested existing `Expr::If`; no new node.
5. **Semantics:** definitionally the nested `if`; identical evaluation,
   selection, scope, shadowing, value, `none`, and control-flow behavior.
6. **Parser change:** delete only the `E1014` guard; `else` continues to parse
   an expression.
7. **Grammar:** production unchanged; notes updated.
8. **E1014:** removed entirely (not repurposed), because documented codes must
   remain reachable.
9. **Checker:** reuse existing `Expr::If`; no new inference.
10. **Runtime:** unchanged.
11. **Resources:** unchanged; nesting bounded by existing limits (`E1015`).
12. **Compatibility:** additive syntax + `E1014` removal; no valid program
    changes meaning.
13. **Comments:** only existing `#` line comments; unaffected; block comments
    excluded.
14. **Testing:** parser/checker/runtime/contract/grammar/boundary updates and
    one differential property.
