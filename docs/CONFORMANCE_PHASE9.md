# AURA LANGUAGE CONFORMANCE PASS — PHASE 9

## 1. Scope

Aura's diagnostic surface as a language API: every error code, its category,
producer, message, span, native/WASM availability, documentation, and tests.

Starting HEAD: `73c7b63`.

## 2. Inventory

`src/error.rs` defines 43 `pub const` codes. `docs/errors.md` documents the
user-facing set; `docs/LANGUAGE_SPEC.md` §30 documents the full table.

| Band | Meaning | Examples |
|---|---|---|
| `E1xxx` | lexer/parser | `E1001` invalid char, `E1004` unterminated string, `E1006` expected, `E1015` nesting |
| `E2xxx` | checker (declarations) | `E2003` undefined, `E2007` redeclared, `E2011` bad `main`, `E2012` duplicate type, `E2013` duplicate variant, `E2018` private, `E2019` unknown module/`use` item |
| `E3xxx` | checker (types) | `E3001` type mismatch, `E3002` unknown type, `E3005` return mismatch |
| `E4xxx` | runtime | `E4007` div-by-zero, `E4011` call depth, `E4013` overflow, `E4018` not iterable, `E4019` index, `E4020` IO, `E4026` uncaught throw, `E4027` no `main`, `E4028` assert, `E4029` no match, `E4030` return position |
| `E5xxx` | host/bridge | `E5001` Python error, `E5002` capability unavailable, `E5003` feature unavailable |

## 3. Code Integrity

| Check | Result |
|---|---|
| Duplicate numeric codes | Only `E5002`, shared intentionally by `PY_UNSUPPORTED` and `CAPABILITY_UNAVAILABLE` (both mean "this build/host lacks the capability") — documented in `src/error.rs`. |
| `docs/errors.md` vs `error.rs` | No documented code is absent from the implementation. |
| Codes in the implementation but not `errors.md` | `E4099` (`THROWN`) and `E5003` — both explicitly documented in `LANGUAGE_SPEC.md` §30 as internal/unused and not part of the normative user surface. |
| Wrong-phase code | None found: every probed condition reports the band its phase produces. |
| Wrong span | None found after the Phase-2 `CONF-RESOLVE-4` fix (type-position spans). |
| Runtime error reported as checker error | None; the reverse (checker-accepted program failing at runtime) is exactly the documented §9.1 arithmetic case. |

## 4. Message and Span Stability

* Repeated identical inputs produce byte-identical diagnostics (determinism
  verified 30×).
* Spans are 1-based line/column pairs derived from byte offsets; type-position
  diagnostics now point at the declaring construct.
* Invalid UTF-8 yields `E1001` at the offending byte sequence.

## 5. Wording Notes (not defects)

* An invalid-regex `E3001` message embeds the regex crate's multi-line parse
  error. The code and span are correct; the embedded detail is third-party.
* `E5002` intentionally serves two names because both denote a missing
  capability.

No message was rewritten: no code/category was wrong, no wording contradicted
the contract, no span was objectively wrong, and determinism holds.

## 6. Findings

**No implementation defects.** One shared-code case (`E5002`) and two
internal/unused codes (`E4099`, `E5003`) are documented and intentional.

## 7. Tests

The registry oracle and `tests/contract.rs`/`tests/contract_sync.rs` assert the
documented codes are reachable and stable. All pass. No new permanent
diagnostic-inventory test was required because the existing coverage already
maps documented codes to reproductions.
