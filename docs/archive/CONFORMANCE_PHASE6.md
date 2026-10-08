# AURA LANGUAGE CONFORMANCE PASS — PHASE 6

## 1. Scope

Every shipped builtin and stdlib-facing feature, inventoried from the
implementation (`src/stdlib/signatures.rs` — the single source of truth shared
by checker and runtime) and verified against `docs/LANGUAGE_SPEC.md` §22,
§§29–30.

Starting HEAD: `73c7b63`.

## 2. Builtin Inventory

`stdlib::signatures::builtins()` declares the free functions; method forms are
declared separately and share the same registry. Core categories:

| Category | Builtins |
|---|---|
| Output / diagnostics | `print`, `assert` |
| Collections | `len`, `push`, `pop`, `remove`, `keys`, `values`, `has`, `get`, `contains`, `first`, `last`, `sort`, `reverse`, `map`, `filter`, `reduce`, `sum`, `enumerate`, `zip`, `range` |
| Strings | `split`, `join`, `upper`, `lower`, `trim`, `replace`, `starts_with`, `ends_with`, `chars` |
| Conversions | `to_int`, `to_float`, `to_string` |
| Numeric | `abs`, `min`, `max` |
| IO | `read_line`, `read_file`, `write_file`, `args` |
| JSON (`json`) | `json_encode`, `json_decode` |
| Regex (`regex`) | `regex_match`, `regex_find`, `regex_find_all`, `regex_replace` |
| Time (`time`) | `time_now`, `time_unix`, `sleep_ms` |
| Python (`py`) | `py_eval`, `py_import`, `py_call`, `py_version` |

Each was checked for existence, arity, accepted types, checker signature,
runtime dispatch, feature gate, and diagnostics.

## 3. Conversions

| Call | Result |
|---|---|
| `to_int("42")` | `42` |
| `to_int("abc")` | `E3001` |
| `to_int(3.9)` | `3` (truncation) |
| `to_int("9223372036854775807")` | `i64::MAX` exactly |
| `to_int("-9223372036854775808")` | `i64::MIN` exactly |
| `to_int("99999999999999999999")` | `E3001` (overflow, not truncation) |
| `to_float("3.5")` / `to_float("x")` | `3.5` / `E3001` |
| `abs(i64::MIN)` | `E4013` |
| `sum([i64::MAX, 1])` | `E4013` |

## 4. JSON

`json_decode` / `json_encode` (names confirmed; `json_parse`/`json_stringify`
do not exist). Scalars, arrays, objects, Unicode (`\u00e9`), and `i64`
boundaries round-trip. Malformed content is a deterministic `E3001`. A
non-`i64` JSON number decodes as `float` rather than truncating. Encoding a
cyclic value is bounded by the value-render budget (§5.6) — it terminates with
the documented truncation, never hangs.

## 5. Regex

`(pattern, text[, replacement])` argument order. Valid patterns match, find,
find-all, and replace; Unicode is honoured (character semantics); an empty
pattern matches. An **invalid** pattern is a deterministic `E3001`. No ReDoS was
claimed or reproduced; the crate is compiled with `perf` and `unicode`.

*Diagnostic note (Phase 9):* an invalid-regex message embeds the regex crate's
multi-line parse error, so the diagnostic text spans several lines. The code and
span are correct; the wording is third-party detail.

## 6. Time

`time_unix()` (deterministic integer seconds) and `time_now()` are separated
from the clock; `sleep_ms(n)` rejects negative input (`E3001`). Native provides
these; a host without the capability reports `E5002`.

## 7. IO

* `read_line()` — one line, `none` at EOF, `E4020` on invalid UTF-8.
* `read_file(path)` — whole UTF-8 file or `none` if absent; a directory or
  permission error is `E4020`.
* `write_file(path, content)` — create/truncate, UTF-8, returns `none`;
  failures are `E4020`.
* `args()` — program arguments excluding the program path.
* Filesystem capability is not expanded; the browser/worker host reports
  `E5002` for absent capabilities.

## 8. Findings

**No implementation defects.** Names and signatures are consistent between the
checker and runtime; the registry oracle (Phase 3 §5) proves no well-shaped
accepted call yields a runtime `E2003`. One documentation/naming clarification
was recorded: the JSON builtins are `json_encode`/`json_decode`.

*Diagnostic wording note* (regex multi-line message) recorded for Phase 9; not
a defect.

## 9. Tests

`tests/checker.rs`, `tests/run.rs`, `tests/contract.rs`, `tests/boundaries.rs`,
`tests/property_hardening.rs` (registry oracle), `tests/python.rs` cover this
layer; all pass. No new permanent test was required because no defect was
found; the existing registry oracle already derives every builtin dynamically.

## 10. Phase-7 Handoff

Stdlib signatures and runtime implementations agree. The Python bridge is its
own subsystem (Phase 7) and was separately audited.
