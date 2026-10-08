# Feature H1 — Pattern Correctness and Safety Hardening
## Implementation Report

**Baseline:** `c5de1862580945e10b87f534ae5a2d7947caba2d`
(`feat: add empty-map literal`, Feature 005).
**Branch:** `rewrite/v3-rust`.
**Status:** Implemented; focused and full validation green.
**Scope:** Fix the two pre-release blockers found by the deep core-language
audit. No feature work; no resource-constant change; F003/F004/F005 semantics
unchanged.

---

## H1a — Pattern Checker/Runtime Contradiction

### Defect

`Checker::check_pattern` (`src/check/mod.rs`) accepted a `Pattern::Variant(tag,
ps)` if `tag` was a declared *type* (`self.types`) **or** a declared *variant*
(`self.variants`). The runtime (`match_pattern`/`bind_pattern`,
`src/run/mod.rs`) matches only `Value::Variant` by tag. Therefore a struct
name, enum type name, or alias name used as a variant pattern passed checking
but could never match at runtime.

Evidence (before the fix):

```
struct P { x: int }
fn main() { print(match P(1) { P(v) -> "m"\n _ -> "f" }) }   # check: ok; run: "f"
enum E { A }
fn main() { print(match A() { E -> "m"\n _ -> "f" }) }        # check: ok; run: "f"
type Id = int
fn main() { print(match 5 { Id -> "m"\n _ -> "f" }) }         # check: ok; run: "f"
```

Additionally, the `Stmt::For` checker arm did **not** call `check_pattern` at
all, so `for P(v) in …` and `for Nope(v) in …` were not validated statically,
while `match` was (incorrectly) permissive and the runtime rejected them.

This contradicts `docs/LANGUAGE_SPEC.md` §19.3:

> A variant pattern MUST name a declared variant (`E3002` otherwise).

### Correction

Minimal, localized:

1. `src/check/mod.rs`, `check_pattern`: a `Pattern::Variant` is accepted only
   when `self.variants.contains_key(tag)`. The `self.types` clause was removed.
   Payload validation and duplicate-binding validation are unchanged.
2. `src/check/mod.rs`, `Stmt::For`: call `self.check_pattern(pat)` before
   declaring the loop bindings, so `for` uses exactly the same pattern
   validation as `match` and destructuring `let`.

No new error code: the existing `E3002` (`UNKNOWN_TYPE`) is reused with the
existing message. Feature 004's destructuring `let` already called
`check_pattern`, so it inherits the correction automatically.

### Post-fix behavior

* struct / enum type / alias name as a `match` or `for` pattern → `E3002`.
* real variant tags (including nullary, and a tag equal to its enum's name)
  remain accepted.
* payload validation unchanged (`E3001` at runtime for arity/tag mismatch).
* `for` and `match` now agree with the runtime.
* `for [a, a] in …` now reports the pattern-specific `E2014`
  (`DUPLICATE_BINDING`), consistent with `match`/`let` and §19.3, instead of
  surfacing later as `E2007`.

---

## H1b — Unguarded Pattern Recursion (Host Stack Overflow)

### Defect

The general `pattern()` parser used by `for`/`match` did **not** call
`Parser::enter()/leave()`, so pattern recursion was unbounded. A syntactically
valid, deeply nested pattern overflowed the parser thread's host stack:

```
for [[[…x…]]] in xs { }      # ~30,000 levels
thread 'aura-parse' has overflowed its stack
fatal runtime error: stack overflow, aborting
```

This violates `docs/LANGUAGE_SPEC.md` §31.5 (“No syntactically valid,
well-formed program may cause a host panic, stack overflow, or undefined
behavior.”). The path was pre-existing (it predates Feature 004); Feature 004's
new `let_pattern` was already guarded.

### Recursion-path analysis

| Path | Recurses over | Guarded before | Guarded after |
|---|---|---|---|
| Parser `pattern()` (`for`/`match`) | pattern depth | **no** | **yes** (H1b) |
| Parser `let_pattern()` (F004) | pattern depth | yes | yes (unchanged) |
| Checker `check_pattern` | pattern depth | (bounded by parser input) | same |
| Runtime `match_pattern` | pattern depth | no explicit guard | no explicit guard needed |
| Runtime `bind_pattern` | pattern depth | no explicit guard | no explicit guard needed |

The checker and runtime recurse over **pattern structure**, not value depth.
Both run on the large interpreter stack (`INTERP_STACK = 64 MiB`,
`src/lib.rs`). Once the parser bounds pattern depth, the maximum pattern that
can reach checking/runtime is bounded.

### Correction

`src/parse/mod.rs`: wrap the general `pattern()` with the existing parser depth
mechanism, exactly as `let_pattern()` already is:

```rust
fn pattern(&mut self) -> Result<Pattern> {
    self.enter()?;
    let r = self.pattern_inner();
    self.leave();
    r
}
```

`pattern_inner` is the former body. All recursive calls inside it still call
`self.pattern()`, so the guard applies at every level. No resource constant
changed; `enter()` uses the existing `MAX_PARSE_DEPTH` (2048) and reports the
existing `E1015` (`NESTING`).

### Was a runtime guard necessary?

No. Verified empirically:

* The parser now rejects patterns beyond ~2045 levels with `E1015` before
  checking/runtime (both `for` and `match`).
* A 2044-level (maximum accepted) list pattern executed correctly through
  `match_pattern` **and** `bind_pattern` on the 64 MiB interpreter stack,
  producing the correct result with no overflow.
* Value depth does not drive runtime recursion: a 50,000-deep value matched by
  a shallow pattern executes fine (runtime descends per pattern node, not per
  value).

Therefore the parser boundary is sufficient, the runtime is proven safe within
the parseable range, and no runtime guard was added.

### Safety boundary (exact)

| Quantity | Value |
|---|---|
| `MAX_AST_DEPTH` (semantic, expressions) | 256 → `E1015` |
| `MAX_PARSE_DEPTH` (parser backstop) | 2048 → `E1015` |
| Max list-pattern depth accepted | ~2045 (just under the backstop) |
| First rejected list-pattern depth (observed) | 2046 → `E1015` |
| Runtime pattern depth exercised safely | 2044 (max accepted) |
| Runtime guard added | none (not required) |

---

## Files Changed

* `src/check/mod.rs` — `check_pattern` narrowed to declared variants; `Stmt::For`
  now calls `check_pattern`.
* `src/parse/mod.rs` — `pattern()` bounded by `enter()/leave()`.
* `tests/checker.rs` — H1a correctness tests.
* `tests/run.rs` — H1a runtime-consistency tests.
* `tests/boundaries.rs` — H1b safety tests.
* `docs/FEATURE_H1_PATTERN_HARDENING_REPORT.md` — this report.

**Unchanged:** `MAX_AST_DEPTH`, `MAX_PARSE_DEPTH`, `MAX_CALL_FRAMES`, range
limit, `match_pattern`, `bind_pattern`, `Env`, `Value`, Feature 003 inference,
Feature 004 `let_pattern`, Feature 005 parser, stdlib, Python bridge, REPL,
and all general control-flow semantics.

---

## Tests Added

**`tests/checker.rs`**
* `non_variant_type_names_are_rejected_as_patterns` — struct / enum type /
  alias names as `match` patterns → `E3002`.
* `for_pattern_uses_the_same_validation_as_match` — struct name and unknown tag
  in `for` → `E3002`; real variant in `for` accepted.
* `real_variant_tags_remain_accepted` — payload and nullary variants, including
  a tag equal to its enum name.

**`tests/run.rs`**
* `variant_pattern_matching_is_unchanged` — real variant matching (payload and
  nullary) still works.
* `variant_tag_equal_to_type_name_matches` — `enum E { E(int) }` matches `E`.

**`tests/boundaries.rs`**
* `deep_match_pattern_is_bounded` — 5000-deep `match` and `for` patterns →
  `E1015`, no abort.
* `bounded_deep_pattern_executes_without_overflow` — a 1000-deep pattern with a
  matching 1000-deep value executes correctly.

### Focused validation results

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | OK |
| `cargo check --all-features` | OK |
| `cargo test --test checker` | 28 passed |
| `cargo test --test parser` | 26 passed |
| `cargo test --test run` | 38 passed |
| `cargo test --test boundaries` | 22 passed |
| `cargo test --test grammar` | 6 passed |
| `cargo test --all-features` | all targets green; 262 executed, 0 failed |
| Feature 004 destructuring tests | 11 + 4 passed |
| Feature 004 `let_pattern` guard | intact |

---

## Compatibility

* Valid programs are unaffected: real variant patterns and literal/bind/list
  patterns in `match`/`for` behave as before.
* Programs that relied on a non-variant type name as a pattern (previously
  accepted by the checker but unable to match at runtime) are now rejected with
  the already-documented `E3002`. This is the intended spec behavior.
* `for` with a duplicate binding now reports `E2014` (pattern duplicate)
  instead of `E2007`; both are static rejections and the new code matches
  `match`/`let`.
* No change to Feature 003, 004, or 005 semantics.

---

## Resource Semantics

No resource constant changed:

* 256 AST depth → `E1015` (unchanged);
* 2048 parser recursion backstop → `E1015` (now also applied to `pattern()`);
* 512 call frames → `E4011` (unchanged);
* 10,000,000 range materialization → `E4013` (unchanged).

The only change is that the general pattern parser now respects the existing
backstop, and the checker now validates `for` patterns.

---

## Remaining Blockers

None for H1. Both audit blockers are resolved and covered by tests.
