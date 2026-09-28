# AURA LANGUAGE CONFORMANCE PASS — PHASE 10

## 1. Scope

Adversarial robustness, resource safety, property testing, fuzzing, and CI
reliability.

Starting HEAD: `73c7b63`.

## 2. Panic / Abort Surface

Production-path search across `src/`:

| Construct | Count in production | Assessment |
|---|---|---|
| `unsafe` | 0 | none |
| `.unwrap()` / `.expect()` | 0 outside tests | none |
| `panic!` | 0 | none |
| `unreachable!` | 2 (`src/resolve.rs:710, 904`) | true invariants guarded by the enclosing match: a `Module` arm re-matching `Item::Module`, and the `flatten` arm reached only for non-module/non-use items. Not reachable from user input. |
| `todo!` / `unimplemented!` | 0 | none |

## 3. Arithmetic Boundaries

All operators were exercised at `i64::MIN`, `i64::MAX`, `-1`, `0`, `1`:

| Case | Result |
|---|---|
| `i64::MAX + 1`, `i64::MIN - 1`, `i64::MIN * -1`, `-i64::MIN` | `E4013` |
| `i64::MIN / -1`, `i64::MIN % -1` | `E4013` |
| `x / 0`, `x % 0` | `E4007` |
| `1 << 63` | `i64::MIN` (= `1<<63` as two's complement, documented) |
| `1 << 64`, `1 << -1` | `E4013` (shift amount out of range) |
| `2 ^ 64` | `E4013` |
| `sum([i64::MAX, 1])`, `abs(i64::MIN)` | `E4013` |
| `len(-i64::MAX..i64::MAX)` | saturates to `i64::MAX` |

The MIN / `-1` protections remain intact.

## 4. Depth / Width / Node Budgets

| Budget | Constant | Boundary verified |
|---|---|---|
| Parser AST depth | `MAX_AST_DEPTH = 256` | nesting `E1015` |
| Call frames | `MAX_CALL_FRAMES = 512` | 510 ok / 511 `E4011` |
| Value render depth | `MAX_VALUE_DEPTH = 512` | 600-deep truncates |
| Value nodes | `MAX_VALUE_NODES = 1_000_000` | bounded render/JSON |
| Range materialization | 10,000,000 | `E4013` beyond |
| Python nodes | `MAX_PY_NODES`/`MAX_PY_SOURCE_NODES = 1_000_000` | reject, never truncate |
| TypeExpr nesting | native 2048 / WASM 768 | **DECISION-PENDING (AUDIT-3), untouched** |

Deeply nested scopes (150 levels), thousands of declarations, and hundreds of
modules were resolved in milliseconds with identical repeated output.

## 5. Property Testing

Existing properties have independent oracles, not self-referential ones:

* **Registry oracle** (`tests/property_hardening.rs` PROPERTY 1) derives every
  builtin from the single production registry and asserts checker-accepted
  well-shaped calls never produce a runtime `E2003` — an external relationship,
  not "implementation says X".
* Native result == WASM result (generated differential, Phase 8).
* Normalization idempotence for unions and rendering.

The AST-limit property explicitly **excludes TypeExpr-heavy inputs** pending the
AUDIT-3 decision.

## 6. Fuzzing

Targets: `lexer`, `parser`, `checker`, `runtime` (`arbitrary`-driven; `ast_gen`
feeds direct-AST checking). Pinned `cargo-fuzz 0.13.2`, nightly, ASan always on;
LSan scoped off only for `runtime` (documented `Rc`-cycle retention).

Local smoke (60 s budget, 25 s unit timeout):

| Target | Runs | Crash/hang artifacts |
|---|---|---|
| lexer | 1,340,812 | 0 |
| parser | 518,573 | 0 |
| checker | 273,964 | 0 |
| runtime | (1.8M+) | 0 |

No crash, abort, or timeout was produced by the lexer/parser/checker targets.

## 7. CI Reliability (`CI-RELIABILITY-1`)

The `runtime` fuzz-smoke step can nondeterministically produce a **`slow-unit`**
artifact (a unit whose execution exceeds the 25 s per-unit timeout under ASan),
and the workflow's "fail if any artifact was produced" step then fails the job.

Evidence gathered across both Phase 1 and this phase:

* The `runtime` fuzz target and its generator (`tests/support/program_gen.rs`)
  are **untouched** by any Phase 1–10 change.
* The same production code passed the fuzz job on one commit and failed on a
  later commit that changed **only** `tests/syntax_docs.rs`.
* Historically the job failed identically on pre-Phase-1 commits.
* Locally the target can generate a genuinely slow program (~10 s under ASan),
  confirming the artifact is a real long-running generated unit, not a
  crash/nontermination bug.

Classification: **CI RELIABILITY**, not a production defect. The workflow's
blanket artifact check treats a `slow-unit` (a performance signal) the same as
a crash. Recommended (future, out of Phase scope): either distinguish
`slow-unit-*` from crash artifacts in the failure step, or raise the per-unit
timeout for `runtime`. No timeout was raised and no production code changed
merely to hide the signal.

## 8. Findings

**No implementation defects.** No production panic path, no arithmetic-boundary
violation, no budget escape, and no fuzz crash. The only actionable item is the
pre-existing CI reliability issue above, classified and documented rather than
"fixed" by weakening a check.

## 9. Tests

The front-end fuzz targets produced zero artifacts locally; corpus/property
suites all pass. No permanent regression fixture was required because no crash
was found.
