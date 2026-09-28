# AURA LANGUAGE CONFORMANCE PASS — PHASE 7

## 1. Scope

The Python interop bridge (`src/bridge/mod.rs`), re-falsifying AUDIT-5 without
changing language semantics: Python → Aura and Aura → Python for scalars,
strings, lists, dicts, nested/shared/cyclic structures, exceptions, and
unsupported objects.

Starting HEAD: `73c7b63`. Feature `py` (PyO3 0.29, auto-initialize).

## 2. Boundary Model

* **Python → Aura (`to_value`).** Python identity is the object address
  (`PyAny::as_ptr`); containers on the current descent path form the cycle set.
  A reference to a container **on the path** is a cycle and is rejected
  (`E5002`); a reference **off** the path is ordinary sharing, converted again
  and bounded by `MAX_PY_SOURCE_NODES = 1_000_000`. Depth ≥ `MAX_VALUE_DEPTH`
  becomes the object's `repr` (string), so a deep object degrades, never
  overflows.
* **Aura → Python (`to_py`).** `Rc` value identity on the descent path detects
  cycles (`E5002`); sharing is re-converted, bounded by `MAX_PY_NODES =
  1_000_000` and `MAX_VALUE_DEPTH`.
* Both directions are **total**: they reject with a structured diagnostic
  rather than truncating, so no partial object crosses.

## 3. Verified Behaviors

| Case | Result |
|---|---|
| ints, floats, bools, strings, `none` | round-trip |
| `i64` boundary (`2**62`) | exact; `2**63` → `E4013` (never demoted to float) |
| lists / nested lists | round-trip |
| dicts / nested dicts | round-trip; string-keyed |
| non-string dict key | `E5002` (never stringified, which would collapse keys) |
| Python `set` | not a list/dict → degrades to its `repr` string |
| cycle at the boundary | `E5002` |
| shared DAG | re-converted, bounded |
| Python exception | `E5001` (`PY_ERROR`) with a code |
| unsupported object | rendered `repr` string (printable) |

## 4. Falsification Attempts

* Exponential traversal via aliasing: bounded by the node budget, rejects
  `E5002` — no hang.
* Self-cycle and mutual cycles: detected by address identity, `E5002`.
* Very deep nesting: falls back to `repr`, no native stack overflow.
* Repeated conversions across many calls: deterministic and bounded.
* Oversized `int`: explicit overflow, never silent precision loss.
* Python exception leakage: mapped to `E5001`; no panic crosses the boundary.

No identity loss, accidental mutation, GIL misuse, or crash was reproduced.

## 5. Findings

**No implementation defects.** The bridge is bounded and cycle-safe in both
directions, matching AUDIT-5's conclusions. The `py` feature is default-on;
without it the same names exist as stubs returning `E5002`, so checker and
runtime agree.

## 6. Tests

`tests/python.rs` (9 tests, `--features py`): expression evaluation, collection
round-trip, error codes, `py_version`, oversized-int rejection, non-string-key
rejection. All pass. No new permanent test was required; no defect was found.

## 7. Boundary Integrity

The bridge is not reachable from the frozen `0.0.2` runtime (no `py` in WASM),
so no artifact impact. Native `--all-features` exercises it.
