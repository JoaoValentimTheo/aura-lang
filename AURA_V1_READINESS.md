# Aura V1 Readiness

Evidence-based readiness for a justified Aura v1.0. Colour is set by evidence,
never by schedule. Status legend: **GREEN** (ready), **YELLOW** (gaps remain),
**RED** (blocking).

| Category | State | Evidence / gap |
|---|---|---|
| LANGUAGE | GREEN | Core COMPLETE; `AURA_COMPLETENESS_MATRIX.md`; no accidental PARTIAL/MISSING; spec/grammar/token contracts tested |
| TYPE SYSTEM | GREEN | checker↔runtime agreement invariant; typed rejection paths exercised |
| RUNTIME | GREEN | no host panic from input; bounded failure; cycles/bounds tested |
| STDLIB | GREEN | 38 builtins + methods; registry-drift property test; value-semantics tests |
| MODULES | GREEN | providers parity; eager discovery defined; visibility/collision tested |
| CLI | GREEN | `tests/cli.rs`; exit codes; paths; stdin |
| REPL | GREEN | persistence + rollback; `tests/repl.rs` |
| WASM | GREEN | 0 imports; Host ABI 1; differential 218/218 |
| PLAYGROUND | GREEN | FSM-P6 suites green; capability detection; dev runtime advanced |
| CPYTHON | YELLOW | bridge works end-to-end (arbitrary authority — documented); compatibility target + normative conversion table + interop matrix (14 tests) + multi-version CI added; version policy proposed (HD-3), pending human confirmation |
| SECURITY | YELLOW | threat model / trust boundaries / architecture / incident response written; no known CRITICAL/HIGH; action SHA-pinning + SBOM open |
| PERFORMANCE | YELLOW | scaling shape guards + baseline committed (`tests/bench.rs`, `docs/engineering/PERFORMANCE.md`); budgets set; needs broader workload coverage before GREEN |
| RESOURCE LIMITS | GREEN | limits tested limit−1/limit/+1; recovery verified |
| DETERMINISM | GREEN | single-output diagnostics; ordered maps |
| PLATFORM — LINUX | GREEN | CI green |
| PLATFORM — MACOS | GREEN | CI green |
| PLATFORM — WINDOWS | GREEN | CI green |
| DOCS | YELLOW | language/stdlib docs current; security/CPython/ops docs now present; CHANGELOG created; install/quickstart and migration still thin |
| PACKAGING | YELLOW | release workflow builds 3 targets + wasm; install guide thin; artifact matrix documented in RELEASE_ENGINEERING.md; no end-user install test yet |
| CI | GREEN | comprehensive matrix; all green; now includes a CPython interop version/platform job |
| RELEASE ENGINEERING | YELLOW | gated release workflow + policy doc + CHANGELOG; no SBOM/provenance; version scheme decision pending (HD-4) |
| SUPPLY CHAIN | YELLOW | cargo audit in CI; actions not SHA-pinned; no SBOM |
| BACKWARD COMPAT | YELLOW | no versioned compatibility fixtures/upgrade tests yet |
| RELEASE-READY (v1 GO) | RED | CPYTHON/SECURITY/PERF/DOCS/PACKAGING/COMPAT not yet GREEN; HD-1 + AUDIT-3 open |

## Current blockers toward v1 GO

1. CPYTHON target and version policy undefined (YELLOW → must be GREEN).
2. SECURITY program docs absent (YELLOW → must be GREEN).
3. PERFORMANCE benchmark suite absent (YELLOW).
4. DOCS/PACKAGING/RELEASE ENGINEERING policy incomplete (YELLOW).
5. HD-1 and AUDIT-3 must be decided or explicitly excluded before semantic freeze.

## Notes

- No unresolved CRITICAL/HIGH correctness or security blocker is currently
  known.
- Frozen `0.0.2`/`0.2.0` artifacts verified byte-identical at intake.
