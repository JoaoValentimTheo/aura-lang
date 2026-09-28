# AURA LANGUAGE CONFORMANCE PASS — PHASE 5

## 1. Scope

The runtime value model: representation, collections, structs/enums, equality,
rendering, patterns, and ranges — verified against `docs/LANGUAGE_SPEC.md`
§§10–11, 16–22, 31.

Starting HEAD: `73c7b63`.

## 2. Value Inventory

| `Value` variant | Checker `Ty` | Runtime backing | Equality | Rendering |
|---|---|---|---|---|
| `Int(i64)` | `Int` | inline | numeric | decimal |
| `Float(f64)` | `Float` | inline | IEEE (`NaN != NaN`, `0.0 == -0.0`) | shortest round-trip |
| `Bool` | `Bool` | inline | value | `true`/`false` |
| `Str(Rc<str>)` | `String` | `Rc` | value | raw |
| `None` | `Unknown` | inline | all `none` equal | `none` |
| `List(Rc<RefCell<Vec>>)` | `List` | `Rc<RefCell>` | structural, cycle-safe | `[a, b]` |
| `Map(Rc<RefCell<BTreeMap>>)` | `Map` | `Rc<RefCell<BTreeMap>>` | structural, key order-independent | `{"k": v}` |
| `Instance(Rc)` (struct) | `Named` | `Rc` | nominal + field-wise | `Name { f: v }` |
| `Variant(Rc)` (enum) | `Enum` | `Rc` | tag + payload-wise | `Tag(p)` |
| `Range` | — | start/end `i64` | by start/end | `a..b` |
| function/closure | — | `Rc` | **identity** | `<fn>` |

`BTreeMap` backing makes map iteration and rendering **deterministic** (keys in
sorted order), independent of insertion order.

## 3. Lists and Maps

* Negative list indexes are allowed (Python-style from the end): `[1,2,3][-1]`
  is `3`; out-of-range is `E4019`.
* A missing map key is `E2003`; `map.get` returns `none`.
* Nested lists/maps, aliasing, and in-place mutation work; aliased subvalues are
  shared (`Rc`), not copied.
* Map key equality is by string value; duplicate literal keys follow the
  last-write rule.
* `len`, `contains`, `first`/`last` (empty → `none`), `join`, `keys`, `values`,
  `sort` (stable), `reverse`, `map`/`filter`/`reduce`, `enumerate`, `zip`
  (shortest), `sum` (overflow `E4013`) all behave per §22.

## 4. Structs and Enums

* Field order is declaration order; `S(x)` positional construction and
  `S { x: 1 }` named construction both validate field count and types.
* Struct equality is nominal (name + field order/count/values); enum equality is
  tag + payload.
* Enum payloads are positional; named payloads are `E3001`.
* Generic runtime identity carries the instantiated type name.

## 5. Equality

Re-falsified the iterative, cycle-safe equality:

| Case | Result |
|---|---|
| self-cycle `l == l` | `true` |
| two independently-built equal cycles | `true` |
| unequal cycles | `false` |
| mutual cycles | `true` for a value against itself |
| shared DAG subvalues | correct |
| map key-order difference | equal |
| `NaN == NaN` | `false` |
| `0.0 == -0.0` | `true` |
| cross-kind (`"1" == 1`) | `false` |
| function identity | closure equals only itself |

No false positives from the visited-pair optimization were found.

## 6. Rendering and Budgets

* `MAX_VALUE_DEPTH = 512`, `MAX_VALUE_NODES = 1_000_000`; rendering is
  per-operation and cycle-safe (`[…, …]` for a cycle, `…` at depth).
* A 600-deep nested list renders truncated, never overflows.
* A cyclic map renders with the truncation marker, not an infinite loop.

## 7. Patterns

Supported and verified: wildcard `_`, name binding, int/string/bool/none
literals, exact-length list patterns, variant patterns (bare and enum-path),
qualified variants, and `if` guards. A no-match is `E4029`. Exact-length list
matching means `[1]` does not match `[a, b]` (falls to `_`). Not supported (and
not added): list-rest, struct, range, negative-literal, and float-literal
patterns — matching the documented surface.

## 8. Ranges

* `a..b` is half-open; `a..b..c` is right-associative `a..(b..c)` (Phase 1).
* Reversed/empty ranges have `len() == 0` and iterate zero times.
* `len` uses saturating arithmetic: `len(-i64::MAX..i64::MAX)` saturates to
  `i64::MAX` rather than overflowing.
* Materializing a range (outside lazy `for`) is capped at 10,000,000 elements
  (`E4013` beyond), per §31.4.
* Range equality is by start/end; a range is not indexable (`E3001`).

## 9. Findings

**No new implementation defects.** All value-model invariants, cycle safety,
budgets, patterns, and ranges match the contract. No change was made to
equality, rendering, the Python bridge (Phase 7), or the range model.

## 10. Tests

`tests/run.rs`, `tests/contract.rs`, `tests/adversarial.rs`,
`tests/boundaries.rs` cover this layer; all pass. No permanent test was added
because no defect was found.

## 11. Phase-6 Handoff

The value model is verified cycle-safe and bounded. Phase 6 (stdlib) can rely
on: string-keyed deterministic maps, exact-length list patterns, identity-based
function equality, and the documented collection builtins.
