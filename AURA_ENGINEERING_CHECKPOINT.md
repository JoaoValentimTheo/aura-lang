# Aura Engineering Checkpoint

Resumable operational state for the Road-to-1.0 program.

## State

```
PROGRAM HEAD:            ecfda79 (pushed; T0-T5 final checkpoint)
REMOTE HEAD:             ecfda79
LATEST RELEASE:          v0.2.0
LATEST PRERELEASE:       none
LATEST DEV RUNTIME:      0.2.1-dev.2 (development channel only)
CURRENT RELEASE TRAIN:   T0-T5 complete; ADR council resolved HD-1..HD-4
FINAL STATE:             ADR-0001..0004 IMPLEMENTED IN WORKING TREE, NOT
                         COMMITTED, NOT PUSHED. Validation floor green.
                         Next: commit the ADR work; then independent review.
```

## Completed this session

- **T0 — organization**: `docs/engineering/AURA_ENGINEERING_ORG.md`,
  `AURA_ROAD_TO_V1.md`, `AURA_V1_READINESS.md`, `HUMAN_DECISIONS_QUEUE.md`,
  `SECURITY.md`, `docs/security/{THREAT_MODEL,TRUST_BOUNDARIES,SECURITY_ARCHITECTURE,WASM_IMPORT_POLICY,INCIDENT_RESPONSE}.md`,
  `docs/CPYTHON_COMPATIBILITY_TARGET.md`,
  `docs/engineering/{RELEASE_ENGINEERING,PERFORMANCE,TECHNICAL_DEBT,RISK_REGISTER}.md`,
  `CHANGELOG.md`.
- **T1 — patch**: fixed CPython boundary diagnostic provenance (real bug:
  Python errors reported at `1:1`); regression tests; `tests/python.rs`.
- **T2 — CPython**: 12-dimension compatibility target + normative conversion
  table; `tests/interop_matrix.rs` (14 tests); CI matrix CPython 3.10–3.13
  (Linux) + 3.12 (macOS) green.
- **T3 — security**: threat model, trust boundaries, security architecture,
  incident response, WASM zero-import policy (independently verified), action
  SHA-pinning + least-privilege CI (TD-01 closed).
- **T4 — performance**: `tests/bench.rs` noise-robust scaling guards +
  `docs/engineering/PERFORMANCE.md` baseline (linear through N=1600).
- **T5 — compatibility/packaging**: `tests/compat.rs` pins the released 0.2.0
  surface; README states CPython authority; release policy + changelog.
- **Self-caught**: benchmark guard flakiness on macOS CI; fixed noise-robustly.

## Commits (this session, oldest→newest)

```
95c6f62 fix(bridge): attribute Python boundary errors to the Aura call site
af432ac docs(org): establish the Aura engineering organization and v1 program
c27da05 test(interop): add the CPython interoperability matrix
c12b39f docs(security): strip trailing whitespace in trust boundaries
9ada4b7 test(perf): add scaling shape guards and a timing baseline
f262180 ci(interop): add a CPython version/platform interop matrix
dcfe5c2 docs(security): define the WASM import policy; expand HD-4 evidence
f114e87 ci(security): pin actions to SHAs and enforce least-privilege permissions
bd29fd2 test(compat): pin the released 0.2.0 language surface
e12cc3a docs(program): checkpoint after T0-T5 engineering trains
50d6d78 fix(bench): make scaling guards noise-robust for shared CI
```

## Pushes / CI

- `6d25985..50d6d78` pushed in gated steps; CI run 36773014863 green across
  ubuntu/macos/windows, CPython 3.10–3.13, MSRV, Miri, clippy, audit, fuzz
  smoke, playground, website.
- One transient failure (36771818750) was a **reviewer-introduced flaky bench
  guard**, root-caused (measurement methodology, not a product regression) and
  fixed additively in `50d6d78`.

## V1 readiness snapshot

GREEN: LANGUAGE, TYPE SYSTEM, RUNTIME, STDLIB, MODULES, CLI, REPL, WASM,
PLAYGROUND, RESOURCE LIMITS, DETERMINISM, LINUX, MACOS, WINDOWS, CI.
YELLOW: CPYTHON (target + matrix done; version policy pending HD-3), SECURITY
(program docs done; SBOM/provenance open), PERFORMANCE (guards + baseline done;
broader workloads), DOCS, PACKAGING, RELEASE ENGINEERING, SUPPLY CHAIN,
BACKWARD COMPAT.
RED: v1 GO — blocked by the YELLOW categories and the open human decisions.

## Human decisions — RESOLVED (Architecture Decision Council, 2026-09-30)

All four queued decisions were resolved as ADRs; the implementations are in
the uncommitted working tree.

- **HD-4 → ADR-0001** version scheme: release ≠ language, `LANG <= RELEASE`;
  source line advances to `0.2.1`.
- **HD-1 → ADR-0002** module members may reuse builtin spellings.
- **HD-3 → ADR-0003** CPython tiers (TESTED: Linux 3.10–3.13, macOS 3.12).
- **HD-2 / AUDIT-3 → ADR-0004** structural type nesting counts toward
  `MAX_AST_DEPTH = 256` on every substrate (Option A).

## Risks / blockers

No known CRITICAL/HIGH. R-08 (perf), R-12/R-13 (compat/versioning) remain
tracked. No frozen artifact mutation. The ADR-0004 parser change is a
patch-level language correction; the practical breakage surface is empty (no
real program nests types beyond depth 1).

## Frozen hashes (verified this session)

- 0.0.2: 1,366,621 / 5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed
- 0.2.0: 1,654,161 / 9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc

## Development runtimes

- 0.2.1-dev.1: intermediate (predates ADR-0004), preserved on disk, unlisted.
- 0.2.1-dev.2: current, 1,767,962 /
  6661e1776da4859f367d3f524d7dc21e1d77cdef2992f2551f91af0e8ac2fc21

## Next tasks

1. Commit the ADR-0001…0004 working tree as additive local commits (do not
   push without explicit authorization).
2. Independent adversarial review of the ADR implementations (especially
   ADR-0004's boundary and the alias-expansion budget) and release audit.
3. Add end-user install/quickstart + artifact-matrix docs (packaging GREEN).
4. Broaden performance workloads (REPL, WASM startup, large module graphs).
5. SBOM/provenance + reproducible-build investigation.
6. Then proceed through the RC train (`1.0.0-alpha/beta/rc`) after freeze.

## Independent review (T0–T5)

- Bridge fix: NOT FALSIFIED. Frozen artifacts: all checks PASS.
- One MEDIUM fixture-scope defect found and corrected (`tests/compat.rs`);
  exposed a version-identity divergence now recorded as HD-4 evidence.
- Worktree integrity: 13 accidentally-deleted tracked `.kilo/` files restored.
- Full record: `docs/engineering/REVIEW_RECORD_T0_T5.md`.
