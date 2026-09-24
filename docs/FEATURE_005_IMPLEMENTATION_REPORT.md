# Feature 005 — Empty-Map Literal
## Implementation Report

**Baseline:** `fad8c1ab82a17b71577bf262ec974000a02a6fdf`
(`feat: add destructuring let`, Feature 004).
**Branch:** `rewrite/v3-rust`.
**Status:** Implemented; focused and full validation green.
**Design:** `docs/FEATURE_005_DESIGN.md` (frozen).
**Specification:** `docs/LANGUAGE_SPEC.md` §4.5, §20.3, §33 item 11, §34.1.

---

## Summary

The empty-map literal `{:}` is implemented as a localized, parser-only change.
It constructs the existing `Expr::Map(vec![], span)`. No AST, checker, runtime,
REPL, stdlib, or resource change was required. `{}` remains an empty block
yielding `none`, and non-empty map syntax is unchanged.

---

## Exact Parser Change

**File:** `src/parse/mod.rs` — the `Tok::LBrace` expression-atom arm, inside the
existing `if self.map_ahead()` branch, immediately after `self.bump()` and
`self.skip_newlines()`.

```rust
if self.eat(&Tok::Colon) {
    self.skip_newlines();
    self.expect(&Tok::RBrace)?;
    return Ok(Expr::Map(entries, span));
}
```

* `map_ahead()` is **not** modified. It already returns `true` for `{:}`
  (the token after `{` is `Colon` at depth 0), so `{:}` routes to this branch.
* Block parsing is **not** modified. `{}` still routes to `block()` because
  `map_ahead()` returns `false` when the token after `{` is `}`.
* Non-empty map parsing is **not** restructured; the new guard runs before the
  existing entry loop and only matches when a `Colon` immediately follows the
  skipped newlines.
* `entries` is still empty at this point, so the produced value is the existing
  `Expr::Map(Vec::new(), span)`.

Whitespace handling: the lexer ignores spaces/tabs/`\r`/comments and
`skip_newlines` consumes newlines; therefore `{:}`, `{ : }`, and the
multi-line form all reach the same guard and produce the same empty map.

---

## Why No AST/Checker/Runtime Changes Were Required

* **AST:** `Expr::Map(Vec<(Expr, Expr)>, Span)` already represents a map with
  any number of entries, including zero. No new node is needed.
* **Checker:** `infer(Expr::Map(entries, _))` initializes `val = Ty::Unknown`
  and only updates it while scanning entries; with zero entries it returns
  `Ty::Map(Box::new(Ty::Unknown))`. No new inference rule is required, and
  `Ty::Unknown` conservatism is preserved (an empty map is compatible with any
  map annotation).
* **Runtime:** `Expr::Map` evaluation builds a `BTreeMap`, inserts each entry,
  and returns `Value::Map(Rc<RefCell<BTreeMap::new()>>)`; with zero entries it
  returns an empty map with the existing mutability/reference semantics. No
  new value type or operation is required.
* **REPL/stdlib/bridge:** unchanged; `{:}` is an ordinary expression value.

---

## Tests Added

**`tests/parser.rs`**
* `empty_map_literal_parses_to_empty_map` — `{:}`, `{ : }`, and `{\n:\n}`
  parse to `Expr::Map` with zero entries.
* `empty_braces_still_parse_as_a_block` — `{}` is `Expr::Block`.
* `non_empty_map_is_unchanged` — `{"a": 1}` and `{"a": 1, "b": 2}` keep their
  entry counts.
* `malformed_empty_map_is_e1006` — `{: 1}` yields `E1006`.

**`tests/checker.rs`**
* `empty_map_literal_is_a_map_typed_by_existing_rules` — `let m: {string: int}
  = {:}` is accepted; `{:}.nope()` is `E2003` (map method error), proving it is
  a map value, not a block.

**`tests/run.rs`**
* `empty_map_literal_is_an_empty_map` — `len`/`has`/`keys`/`values`/equality.
* `empty_map_literal_whitespace_variants_are_equal` — `{:} == { : }` and
  `{:} == {\n:\n}`.
* `empty_map_missing_key_uses_existing_e2003` — `{:}["x"]` is `E2003`.
* `empty_braces_remain_a_block_value` — `{}` yields `none`; `{} == none` is
  true and `{} == {:}` is false.
* `non_empty_map_literal_is_unchanged`.

**`tests/property.rs`**
* `empty_map_literal_is_an_empty_map_not_a_block` — for any key `k`,
  `{:}.has(k)` is `false`, `len({:})` is `0`, and `{:} == none` is `false`.

**`tests/boundaries.rs`**
* `empty_map_literal_adds_no_nesting` — a program using `{:}` obeys the same
  depth limits as any other; deep grouping still reports `E1015`.

### Results

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | OK |
| `cargo check --all-features` | OK |
| `cargo test --test parser` | 26 passed |
| `cargo test --test checker` | 25 passed |
| `cargo test --test run` | 36 passed |
| `cargo test --test property` | 19 passed |
| `cargo test --test boundaries` | 20 passed |
| `cargo test --all-features` | all targets green (213 passed, 0 failed) |

---

## Compatibility

* `{}` remains an empty block yielding `none` (verified by tests).
* Non-empty map syntax, checking, and runtime are unchanged.
* `{:}` is **additive syntax**: it previously produced `E1006`, so admitting it
  cannot change the meaning of any previously valid program.
* No new error codes. Malformed variants continue to use existing parser
  diagnostics (`{: 1}` → `E1006`); a missing map key continues to be `E2003`
  (`UNDEFINED`, "map has no key").
* No new runtime semantics: `{:}` supports the existing map operations
  (`len`, `get`, `has`, `keys`, `values`, `remove`, indexing, equality,
  display, iteration).

---

## Resource Semantics

No resource constant changed:

* AST depth limit 256 → `E1015`;
* parser recursion backstop 2048 → `E1015`;
* call-frame limit 512 → `E4011`;
* range materialization limit 10,000,000 → `E4013`.

`{:}` is a fixed three-token sequence parsed without additional nesting, so it
adds no AST/parser depth; the boundary test confirms existing depth limits are
unaffected.

---

## Files Changed

* `src/parse/mod.rs` — the 10-line localized parser insertion.
* `tests/parser.rs`, `tests/checker.rs`, `tests/run.rs`, `tests/property.rs`,
  `tests/boundaries.rs` — focused Feature 005 tests.
* `docs/FEATURE_005_IMPLEMENTATION_REPORT.md` — this report.

**Unchanged:** `src/lex/`, `src/ast/`, `src/check/`, `src/run/`, `src/repl.rs`,
`src/stdlib/`, `src/bridge/`, all resource constants, Feature 003 and Feature
004 code, and `README.md`.

---

## Known Non-Goals

New map key types; new map syntax beyond `{:}`; map type-inference redesign;
empty tuple syntax; any change to `{}`; map iteration semantics; a new runtime
map representation.
