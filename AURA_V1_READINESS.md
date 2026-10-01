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
| WASM | GREEN | 0 imports; Host ABI 1; differential 219/219; module/alias depth unified with native |
| PLAYGROUND | GREEN | FSM-P6 suites green; capability detection; dev runtime advanced |
| CPYTHON | YELLOW | bridge works end-to-end (arbitrary authority — documented); normative conversion table + interop matrix; TESTED tiers (ADR-0003): Linux 3.10–3.13, macOS 3.12, Windows 3.12 (Windows probe promoted to a blocking leg after running green); lifetime/GIL review documented (dimension M) |
| SECURITY | YELLOW | threat model / trust boundaries / architecture / incident response written; red-team campaign + re-verification recorded (`docs/security/CAMPAIGN_LOG.md`), all CRITICAL/HIGH fixed and guarded; deep fuzz campaign clean; SBOM + signed SLSA build provenance at publish; cryptographic release signing still pending (TD-03) |
| PERFORMANCE | GREEN | sub-quadratic scaling guards for lex/parse/check, module graph, list/map runtime ops, function calls, string iteration, long-string lexing, and alias chains; budgets set (`docs/engineering/PERFORMANCE.md`); no input-driven catastrophic complexity in the deep fuzz campaign; TD-13 (REPL O(N^2)) root-caused, bounded, and explicitly justified with a recorded fix design |
| RESOURCE LIMITS | GREEN | limits tested limit−1/limit/+1; recovery verified |
| DETERMINISM | GREEN | single-output diagnostics; ordered maps |
| PLATFORM — LINUX | GREEN | CI green |
| PLATFORM — MACOS | GREEN | CI green |
| PLATFORM — WINDOWS | GREEN | CI green |
| DOCS | YELLOW | language/stdlib docs current; security/CPython/ops docs present; CHANGELOG; install guide + website quickstart; 0.2.1 migration guide added (docs + website); broader worked examples and a formal docs-site versioning scheme still pending |
| PACKAGING | GREEN | release workflow builds Linux/macOS/Windows + wasm with per-artifact checksums and a built-binary smoke before release; install guide (`docs/INSTALL.md`, website); end-user installed-binary smoke (`scripts/artifact-smoke.sh`) runs in CI and the release gate; artifact matrix documented; the produced release artifact is exercised post-publish by the release workflow |
| CI | GREEN | comprehensive matrix; all green; now includes a CPython interop version/platform job |
| RELEASE ENGINEERING | YELLOW | gated release workflow + policy doc + CHANGELOG; version scheme decided (ADR-0001); pre-release metadata consistency gate; release-manifest + SBOM generated and attached; signed SLSA provenance at publish; cryptographic signing pending |
| SUPPLY CHAIN | YELLOW | cargo audit in CI; standard actions SHA-pinned; CycloneDX SBOM generated and attached to releases (`scripts/sbom.sh`, tested); release manifest records a dependency-lock SHA-256; signed SLSA build provenance runs at publish (SHA-pinned action); cryptographic release signing still pending (TD-03) |
| BACKWARD COMPAT | YELLOW | `tests/compat.rs` pins the released 0.2.0 surface *and* the 0.2.1 additions (reservation, unified nesting, bounded alias chains) behaviorally, with documented intentional breaks; compatibility policy in RELEASE_ENGINEERING.md; cross-release artifact run-through still pending |
| RELEASE-READY (v1 GO) | RED | CPYTHON/SECURITY/DOCS/RELEASE ENGINEERING/SUPPLY CHAIN/BACKWARD COMPAT still YELLOW; all semantic/version decisions (HD-1…HD-4) closed by ADR-0001…0004, implemented, pushed, and CI-green |

## Current blockers toward v1 GO

1. CPYTHON formal claim, SECURITY (signed provenance), DOCS, RELEASE
   ENGINEERING, SUPPLY CHAIN, and BACKWARD-COMPAT coverage still YELLOW →
   must be GREEN. PERFORMANCE and PACKAGING are now GREEN.
2. The ADR-0001…0004 resolutions plus the train-1 red-team fixes are committed
   and pushed; independent re-verification of the train-1 fixes is in progress.
   All semantic-freeze decisions are resolved (no open HD items).

## Notes

- No unresolved CRITICAL/HIGH correctness or security blocker is currently
  known.
- Frozen `0.0.2`/`0.2.0` artifacts verified byte-identical at intake.
