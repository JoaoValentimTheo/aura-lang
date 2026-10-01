# Aura Engineering Checkpoint

Resumable operational state for the Road-to-1.0 program.

## State

```
PROGRAM HEAD:            45b5f9d (pushed)
REMOTE HEAD:             45b5f9d
CURRENT RELEASE:         v0.2.0 (tagged; immutable)
CURRENT TRAIN:           TRAIN 2 — supply chain, perf breadth, compat, docs
LATEST DEV RUNTIME:      0.2.1-dev.3 (development channel only)
TEAM STATUS:             language/runtime/security GREEN; CPython, performance,
                         release engineering, docs advancing
DECISIONS CLOSED:        HD-1, HD-2 (AUDIT-3), HD-3, HD-4 — all four, via ADR-0001…0004
ACTIVE FINDINGS:         train-1 red-team F1(FIXED), F2/F3/F4(FIXED),
                         F5(LOW, tracked TD-15), F6(INFO, by design)
BUGS FIXED:              alias-chain stack overflow (F1) + Θ(k²) memory (F2);
                         Θ(n²) string lexing (F3); module-nesting substrate
                         divergence (F4); CPython dict-key identity + depth
                         symmetry; alias expansion amplification (TD-14)
PERFORMANCE RESULTS:     parser fuzz 200k + checker 150k + runtime 80k clean;
                         13 sub-quadratic scaling guards incl. map/call/string
SECURITY RESULTS:        campaign recorded (docs/security/CAMPAIGN_LOG.md);
                         no known CRITICAL/HIGH open
COMMITS:                 ADR-0001…0004; security F1–F4; CPython fixes; runtime
                         0.2.1-dev.3; SBOM + release manifest; perf breadth;
                         compat fixtures; decision-doc resolutions
PUSHES:                  …→45b5f9d (all CI-green)
CI:                      GREEN through 45b5f9d
RELEASES:                none this train (v0.2.0 remains latest)
V1 GREEN:                LANGUAGE, TYPE SYSTEM, RUNTIME, STDLIB, MODULES, CLI,
                         REPL, RESOURCE LIMITS, DETERMINISM, WASM, PLAYGROUND,
                         PLATFORMS, CI
V1 YELLOW:               CPYTHON, SECURITY, PERFORMANCE, DOCS, PACKAGING,
                         RELEASE ENGINEERING, SUPPLY CHAIN, BACKWARD COMPAT
V1 RED:                  v1 GO — blocked by the YELLOW categories
FROZEN HASHES:           0.0.2 = 1,366,621 / 5a4ad3f7…; 0.2.0 = 1,654,161 / 9937fd80…
NEXT PARALLEL PHASE:     TRAIN 3 — CPython formalization (Windows probe result,
                         lifetime review), SECURITY provenance/signing,
                         PERFORMANCE report + TD-13 incremental REPL design,
                         DOCS install/quickstart + migration
```

## Train 1 — completed work

- **HD-4 → ADR-0001**: release distinct from language version;
  `LANGUAGE_VERSION <= RELEASE_VERSION`; line advances to 0.2.1.
- **HD-1 → ADR-0002**: module members may reuse builtin spellings; reservation
  scoped to the user-visible value namespace.
- **HD-3 → ADR-0003**: CPython support tiers (TESTED Linux 3.10–3.13, macOS
  3.12; best-effort other 3.10–3.13; Windows not claimed).
- **HD-2 / AUDIT-3 → ADR-0004**: structural type nesting counts toward the
  semantic AST limit (256) on every substrate; flat unions exempt.
- **TD-14 (new)**: checker type-alias expansion budget bounds exponentially
  duplicating alias chains (`E1015`).
- **CPython bridge**: exact-type identity for dict keys (was duck-typed
  `__index__`); depth bounded symmetrically (Python→Aura rejects, no silent
  `repr`); doc wording reconciled to the implementation.
- Runtime `0.2.1-dev.3`; playbook/website/docs synced to the new version line.

## Train 2 — completed work

- **SBOM** (`scripts/sbom.sh`, tested): CycloneDX 1.5 over both workspaces,
  attached to releases; the SBOM half of TD-03 is closed.
- **Release manifest** (`scripts/release-manifest.sh`, tested): machine-readable
  identities (ADR-0001 invariant, runtimes, ADR-0003 matrix) attached to
  releases.
- **End-user artifact smoke** (`scripts/artifact-smoke.sh`): 12 checks against
  the installed binary, run in CI and the release gate.
- **Performance breadth**: sub-quadratic guards for map ops, function calls,
  string iteration, long-string lexing, and alias-chain resolution.
- **Compatibility**: `tests/compat.rs` now pins the `0.2.1` additions with
  documented intentional breaks; stale AUDIT-3/HD-1 "pending" markers removed
  across `AGENTS.md`, decision packages, corpus, and property tests.
- **WASM runtime** `0.2.1-dev.3` carries the train-1 parser hardening.

## Root-caused, deferred (not defects)

- **TD-13** (REPL O(N²)): `Checker::with_declarations` rebuilds the environment
  each submission and is *also* the rollback mechanism; its registrations are
  synthetic (`Span::default()`, `source: None`) unlike the live path. A safe
  fix needs an exactly-equivalent incremental `absorb_decl` + a differential
  oracle. Design recorded in `docs/engineering/TECHNICAL_DEBT.md` (v1.1).

## Next tasks

1. TRAIN 3: CPython formalization (Windows probe outcome, lifetime/GIL review),
   provenance/attestation + signing, PERFORMANCE report + TD-13 design,
   docs install/quickstart/migration, TD-06 cross-release artifact run-through.
2. Then the pre-1.0 progression (`0.2.1` dev → alpha/beta/rc) under ADR-0001.
