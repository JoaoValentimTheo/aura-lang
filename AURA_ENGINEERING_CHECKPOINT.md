# Aura Engineering Checkpoint

Resumable operational state for the Road-to-1.0 program.

## State

```
PROGRAM HEAD:            f80c5be (pushed) + uncommitted docs/fuzz batch
REMOTE HEAD:             f80c5be
CURRENT RELEASE:         v0.2.0 (tagged; immutable)
CURRENT TRAIN:           TRAIN 1 — semantic-decision closure + first hardening pass
LATEST DEV RUNTIME:      0.2.1-dev.2 (development channel only)
TEAM STATUS:             language/runtime GREEN; CPython, security, performance,
                         docs advancing
DECISIONS CLOSED:        HD-1, HD-2 (AUDIT-3), HD-3, HD-4 — all four, via ADR-0001…0004
ACTIVE FINDINGS:         CPython dict-key duck-typing (HIGH) — FIXED; CPython
                         depth asymmetry (MEDIUM) — FIXED; TD-13 REPL O(N²) —
                         root-caused, design recorded, deferred (not a defect)
BUGS FIXED:              CPython map-key identity; CPython depth symmetry;
                         type-alias expansion amplification (TD-14)
PERFORMANCE RESULTS:     parser fuzz 200k runs clean; checker fuzz 150k clean
SECURITY RESULTS:        red-team campaign vs 92ea6bc in progress
COMMITS:                 ADR-0001…0004; alias bound; PyO3 exact-type; runtime
                         0.2.1-dev.2; program state; website install; CPython
                         key/depth fixes; errors doc; fuzz pin
PUSHES:                  ecfda79..92ea6bc; 92ea6bc..f80c5be
CI:                      ecfda79→92ea6bc GREEN; f80c5be in progress
RELEASES:                none this train (v0.2.0 remains latest)
V1 GREEN:                LANGUAGE, TYPE SYSTEM, RUNTIME, STDLIB, MODULES, CLI,
                         REPL, RESOURCE LIMITS, DETERMINISM, LINUX/MACOS/WINDOWS, CI
V1 YELLOW:               CPYTHON, SECURITY, PERFORMANCE, DOCS, PACKAGING,
                         RELEASE ENGINEERING, SUPPLY CHAIN, BACKWARD COMPAT
V1 RED:                  v1 GO — blocked by the YELLOW categories
FROZEN HASHES:           0.0.2 = 1,366,621 / 5a4ad3f7…; 0.2.0 = 1,654,161 / 9937fd80…
NEXT PARALLEL PHASE:     finish TRAIN 1 (red team + CI) → TRAIN 2 from the
                         readiness board (TD-13 incremental REPL, SBOM, perf
                         breadth, packaging/artifact smoke)
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
- Runtime `0.2.1-dev.2`; playbook/website/docs synced to the new version line.

## Root-caused, deferred (not defects)

- **TD-13** (REPL O(N²)): `Checker::with_declarations` rebuilds the environment
  each submission and is *also* the rollback mechanism; its registrations are
  synthetic (`Span::default()`, `source: None`) unlike the live path. A safe
  fix needs an exactly-equivalent incremental `absorb_decl` + a differential
  oracle. Design recorded in `docs/engineering/TECHNICAL_DEBT.md`.

## Next tasks

1. Close TRAIN 1: red-team findings, CI green on the batch.
2. TRAIN 2: TD-13 incremental REPL; SBOM/provenance; broader perf workloads;
   packaging/end-user artifact smoke; CPython Windows feasibility leg.
3. Then the pre-1.0 progression (`0.2.1` dev → alpha/beta/rc) under ADR-0001.
