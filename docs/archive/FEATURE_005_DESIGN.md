# Feature 005 — Empty-Map Literal

## Status

Design (pre-implementation), **normative and implementation-ready**.

**Baseline:** `fad8c1ab82a17b71577bf262ec974000a02a6fdf`
(`feat: add destructuring let`, Feature 004).
**Branch:** `rewrite/v3-rust`.
**Semantic authority:** `docs/LANGUAGE_SPEC.md` (frozen).
**Frozen syntax decision:** `{:}`.

No open questions remain in this document. Every decision below is resolved
from the existing implementation.

---

## Motivation

`docs/LANGUAGE_SPEC.md` §20.3 and §34.1 document a real language limitation:
`{}` is an empty **block** (yielding `none`), and a map literal requires at
least one entry, so an **empty map cannot be written**. This blocks the
common patterns of building a map from zero elements and spelling the
identity value for map accumulation:

```aura
let m: {string: int} = {:}      # not expressible today
```

Feature 005 adds the spelling `{:}` for an empty string-keyed ordered map. It
introduces no new AST node, no new runtime value type, no new checker rule, and
no change to `{}` or to non-empty map syntax.

---

## Current Language Behavior (verified against source)

* `Expr::Map(Vec<(Expr, Expr)>, Span)` (`src/ast/mod.rs:190`) is the map
  literal AST. There is no dedicated empty-map node.
* Map parsing (`src/parse/mod.rs:1257`, `Tok::LBrace` arm):
  ```rust
  if self.map_ahead() {
      self.bump();
      let mut entries = Vec::new();
      self.skip_newlines();
      while !self.eat(&Tok::RBrace) {
          let k = self.expr()?;          // fails when the token is `:`
          self.expect(&Tok::Colon)?;
          let v = self.expr()?;
          entries.push((k, v));
          ...
      }
      Expr::Map(entries, span)
  } else {
      let body = self.block()?;
      Expr::Block(body, span)
  }
  ```
* `map_ahead()` (`src/parse/mod.rs:1421`) scans from the token after `{` and
  returns `true` on the first `Tok::Colon` at bracket depth 0, skipping
  newlines. For `{:}` the token after `{` is `Colon`, so `map_ahead()` already
  returns `true`; the existing map branch is selected.
* `infer(Expr::Map(entries, _))` (`src/check/mod.rs:899`) initializes
  `val = Ty::Unknown`, scans entries, and returns `Ty::Map(Box::new(val))`.
  With **zero** entries the scan does not run and the result is
  `Ty::Map(Box::new(Ty::Unknown))`.
* Runtime `Expr::Map` (`src/run/mod.rs:939`) builds a `BTreeMap`, inserts each
  entry, and returns `Value::Map(Rc<RefCell<BTreeMap::new()>>)`. With zero
  entries it returns an empty `Value::Map`.
* The lexer already emits `Tok::LBrace`, `Tok::Colon`, `Tok::RBrace`
  (`src/lex/mod.rs`); no new token is required.
* Maps are string-keyed and ordered by key (`docs/LANGUAGE_SPEC.md` §20,
  §33 item 16). The `Ljava`-style `BTreeMap<String, Value>` already encodes
  this.

---

## 1. Syntax

**Normative rule.** The empty-map literal is exactly the three-token sequence
`{`, `:`, `}`, written `{:}`.

**Normative rule (whitespace).** Lexing is whitespace-insensitive, and
`map_ahead()` skips `Tok::Newline`. Therefore `{:}`, `{ : }`, and the
multi-line form

```
{
:
}
```

all produce the same token stream and are all **accepted** as the empty-map
literal. There is no whitespace-sensitivity rule specific to `{:}`.

**Normative rule.** `{}` remains an empty **block** yielding `none` (§20.3).
It is not affected by this feature.

**Normative rule.** Non-empty map syntax `{k: v, ...}` is unchanged.

---

## 2. Parsing

**Normative rule.** `{:}` MUST be parsed directly to
`Expr::Map(vec![], span)` — the existing map AST representation with zero
entries. No new AST node is introduced.

**Normative rule.** `map_ahead()` MUST NOT be changed. Source inspection
confirms it already returns `true` for `{:}` (the token after `{` is
`Colon` at depth 0), so the existing map branch is selected.

**Normative rule.** The only parser change is inside the existing map branch:
the entry loop MUST accept the zero-entry case (a `}` immediately following
`{`, or a `:` immediately following `{`). The general map/block
disambiguation and `block()` are untouched.

**Parser behavior after the change:**

```
{          -> map_ahead() == true
  :        -> empty-map marker
}          -> Expr::Map(vec![], span)
```

**Normative rule.** `{}` continues to route to `block()` because
`map_ahead()` returns `false` (the token after `{` is `}`, which returns false
at depth 0).

**Normative rule.** `{ : }` with surrounding whitespace and the multi-line
form are accepted, as established in §1.

---

## 3. Semantics

**Normative rule.** `{:}` evaluates to a valid, empty, string-keyed, ordered,
mutable, reference-semantics map value, indistinguishable from any other map
with zero entries.

**Normative rule.** The string-keyed ordered-map semantics are exactly those
already specified for maps (§20); Feature 005 adds no new map semantics.

**Normative rule.** With no entries, no key type or value type can be inferred
from the contents. This is consistent with the existing map inference
behavior: the inferred type is `Ty::Map(Unknown)` (see §4).

---

## 4. Type Checking

**Normative rule.** The checker uses the existing `Ty::Map` semantics. The
empty-map literal has the type `Ty::Map(Box::new(Ty::Unknown))`, produced by
the existing `infer(Expr::Map)` code path with zero entries.

**Normative rule.** No new inference rule is introduced. In particular,
`{:}` does not acquire a special type, and the checker does not attempt to
infer key/value types from context.

**Normative rule.** `Ty::Unknown` conservatism is preserved: an empty map's
value type is `Unknown`, so it is compatible with any map annotation, exactly
as a non-empty map whose value types are all `Unknown` already is.

**Normative rule.** An empty map annotated with a concrete map type is
accepted because `Ty::Map(Unknown)` is compatible with it under the existing
compatibility rule. For example `let m: {string: int} = {:}` is accepted.

---

## 5. Runtime

**Normative rule.** `{:}` creates the existing empty map representation:
`Value::Map(Rc::new(RefCell::new(BTreeMap::new())))`.

**Normative rule.** No new runtime value type is introduced, and no
evaluation or mutation side effect occurs. Evaluating `{:}` has no observable
effect beyond producing an empty map value.

**Normative rule.** The `Expr::Map` runtime implementation is unchanged; it
already handles zero entries.

---

## 6. Compatibility

**Normative rule.** `{}` remains a block yielding `none`.

**Normative rule.** Existing non-empty maps are unchanged in syntax,
checking, and runtime behavior.

**Normative rule.** No previously valid program changes meaning. `{:}` is
**additive syntax**: it was previously a parse error (`E1006`), so admitting it
cannot alter any valid program.

**Normative rule.** This is a syntax extension (per the `FEATURE_ROADMAP.md`
versioning policy), not a semantic tightening or relaxation.

---

## 7. Resource and Safety Invariants

**Normative rule.** No resource limit changes. Preserved exactly:

* AST depth limit 256 → `E1015`;
* parser recursion backstop 2048 → `E1015`;
* call-frame limit 512 → `E4011`;
* range materialization limit 10,000,000 → `E4013`.

**Normative rule.** `{:}` adds no recursion: it is a fixed three-token
sequence parsed without additional nesting, so parser/AST depth protections
are unaffected.

**Normative rule.** `{:}` allocates a single empty `BTreeMap` and cannot panic
or overflow.

---

## 8. Interaction

* **Functions.** An empty map is an ordinary value and may be passed,
  returned, and stored exactly like any map.
* **Named arguments.** No interaction; `{:}` is an expression argument like
  any other.
* **Pipeline.** No interaction; `{:}` may appear as a pipeline input or as an
  argument, using the ordinary call/pipeline semantics.
* **Structs / enums.** No interaction; map fields and payloads accept an empty
  map as an ordinary map value.
* **REPL.** `{:}` is an expression; the REPL echoes its value/display. No new
  persistence model (only top-level `let`/`fn`/`struct`/`enum`/`alias`
  declarations persist).
* **Map indexing / lookups.** An empty map supports the existing map
  operations: `len` is `0`, `has(k)` is `false`, `keys()`/`values()` are
  empty, and an indexing lookup of any key is the existing missing-key
  diagnostic `E2003` (`UNDEFINED`, "map has no key"). No new behavior.
* **Existing map operations.** `get`, `remove`, `keys`, `values`, equality,
  and display all use the existing map machinery and are unchanged.

The display of an empty map is the existing map display of zero entries
(`{}`), as produced by the existing `Value::Map` display path; this is a
*value* display and must not be confused with the *source* spelling, which is
`{:}`.

---

## 9. Error Behavior

**Normative rule.** No new error code is introduced.

**Normative rule.** Malformed variants continue to use the existing parser
diagnostics:

* `{` followed by a non-entry, non-`}` token (other than the empty-map `:`):
  existing `E1006` (`EXPECTED`);
* an unterminated map: existing `E1006`;
* a non-string key: existing `E3001` (`TYPE_MISMATCH`) at runtime.

**Normative rule.** `{:}` is not an error; it is the empty-map literal.

---

## 10. Testing Plan

Focused tests, placed with the existing test suites:

**Parser (`tests/parser.rs`)**
* `{:}` parses and yields `Expr::Map` with zero entries.
* `{ : }` parses identically (whitespace-insensitive).
* Multi-line `{\n:\n}` parses identically.
* `{}` still parses as `Expr::Block`.
* Non-empty `{"a": 1}` still parses as `Expr::Map` with one entry.
* A malformed `{: 1}` / `{:,}` continues to produce `E1006`.

**Checker (`tests/checker.rs`)**
* `let m: {string: int} = {:}` is accepted.
* `{:}` used where a map is expected is accepted; used as a non-map (for
  example ordered with `<`) follows existing behavior.

**Runtime (`tests/run.rs`)**
* `len({:})` prints `0`.
* `{:}.has("x")` prints `false`.
* `{:}.keys()` displays as `[]`.
* `{:} == {:}` is `true`; `{:} == {"a": 1}` is `false`.
* `{:}["x"]` produces the existing missing-key/index diagnostic.
* An empty map is mutable and reference-valued like any map.

**REPL (`tests/repl.rs`)**
* Evaluating `{:}` echoes/prints the empty map value.
* `{:}` behaves as an empty map across submissions.

**Boundary (`tests/boundaries.rs`)**
* A deeply nested expression containing `{:}` obeys the existing AST/parse
  depth diagnostics; `{:}` itself adds no depth.

**Regression**
* `{}` remains a block (`tests/run.rs`, existing behavior).
* General map/block disambiguation tests unchanged.
* Non-empty map tests unchanged.

---

## 11. Property / Differential Testing

Minimal properties using the existing `proptest` infrastructure
(`tests/property.rs`):

* **Empty-map property.** For a generated key string `k`,
  `{:}.has(k)` is always `false` and `len({:})` is always `0`.
* **Non-block property.** `{:}` never yields `none` (block semantics);
  explicitly, `{:} == none` is `false` while `{} == none` is `true`.
* **Equivalence property.** `{:}` is equal to every map constructed with zero
  entries and unequal to any non-empty map.
* **Regression property.** Whitespace variants (`{:}`, `{ : }`, multi-line)
  all produce equal empty-map values.

No reference implementation is invented; the oracle is the existing map
semantics.

---

## 12. Out of Scope

* New map key types (maps remain string-keyed).
* New map syntax beyond `{:}` (no trailing-comma/spread/dict-comprehension).
* Map type-inference redesign (no contextual key/value inference).
* Empty tuple syntax.
* Any change to `{}` (it remains a block).
* Map iteration semantics.
* A new runtime map representation.
* Any change to non-empty map syntax, checking, or runtime.

---

## Implementation Plan (for the later implementation phase)

* **Parser (`src/parse/mod.rs`)** — in the `Tok::LBrace` atom arm, inside the
  `map_ahead()` branch, accept the zero-entry empty-map case and produce
  `Expr::Map(vec![], span)`. Do not modify `map_ahead()`, `block()`, or the
  non-empty entry parsing.
* **AST, checker, runtime, REPL, stdlib, Python bridge** — unchanged.
* **Tests** — per §10 and §11.
* **Docs** — at spec-review/implementation time only: amend
  `docs/LANGUAGE_SPEC.md` §4.5 grammar and §20.3 (and §34.1), add a
  `docs/grammar.md` production, and add a `docs/contract.md` statement.

**Must remain untouched:** `Expr::Map`, `Ty::Map`, `Value::Map`, the map
methods and indexing, `map_ahead()`, block parsing, resource constants, and
all other constructs.

---

## Resolved Design Decisions

1. **Spelling:** `{:}`, three tokens `LBrace Colon RBrace`.
2. **Whitespace:** `{ : }` and multi-line forms are accepted
   (whitespace-insensitive lexing; `map_ahead` skips newlines).
3. **`{}`:** remains a block; unaffected.
4. **Parser:** no `map_ahead()` change; only the existing map branch gains the
   zero-entry case; directly yields `Expr::Map(vec![])`.
5. **AST:** no new node.
6. **Type:** `Ty::Map(Unknown)` via the existing `infer(Expr::Map)` path; no
   new inference; `Ty::Unknown` conservatism preserved.
7. **Runtime:** existing empty `Value::Map`; no new value type; no side
   effects.
8. **Errors:** no new codes; malformed variants use existing `E1006`/
   `E3001`.
9. **Compatibility:** additive syntax; no existing program changes meaning.
10. **Resources:** unchanged; no new recursion; no host-failure risk.
