# Aura Engineering Checkpoint

Resumable operational state for the Road-to-1.0 program.

## State

```
PROGRAM HEAD:            6d25985 + T0/T1 commits (unpushed as of writing)
REMOTE HEAD:             6d25985
LATEST RELEASE:          v0.2.0
LATEST PRERELEASE:       none
LATEST DEV RUNTIME:      0.2.0-dev.2
CURRENT RELEASE TRAIN:   T1 (patch) on top of T0 (org bootstrap)
```

## Active work

- T0 organization bootstrap (docs) — in progress.
- T1-1 bridge diagnostic-provenance fix — done (awaiting independent review).
- T1-2 red-team boundary pass — in progress.

## Agent ownership (logical)

- Author T1-1: Runtime/CPython role.
- Reviewer T1-1: pending independent role.
- Security docs: Security role.
- CPython target: CPython Compat role.

## Review queue

- T1-1 bridge fix — needs independent review before release-train closure.

## Security queue

- WASM import policy write-up (T3-1).
- Action SHA-pinning (TD-01).

## Performance queue

- Benchmark suite + budgets (T4 / TD-05).

## Commits

- `fix(bridge): attribute Python boundary errors to the Aura call site`
- `docs(org): establish the Aura engineering organization and v1 program` (pending)

## Pushes / CI runs

- Last known green: CI 36765658391 at `6d25985`.

## V1 readiness snapshot

GREEN: LANGUAGE, TYPE SYSTEM, RUNTIME, STDLIB, MODULES, CLI, REPL, WASM,
PLAYGROUND, RESOURCE LIMITS, DETERMINISM, LINUX, MACOS, WINDOWS, CI.
YELLOW: CPYTHON, SECURITY, PERFORMANCE, DOCS, PACKAGING, RELEASE ENGINEERING,
SUPPLY CHAIN, BACKWARD COMPAT.
RED: v1 GO (blocked by the YELLOW categories).

## Human decisions queued

HD-1 (module-member reservation), HD-2 (AUDIT-3), HD-3 (CPython versions),
HD-4 (pre-v1 version scheme).

## Risks / blockers

No known CRITICAL/HIGH. R-08 (perf regression detection) and R-12/R-13
(compatibility/versioning) open. No frozen artifact mutation.

## Frozen hashes (verified at intake)

- 0.0.2: 1,366,621 / 5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed
- 0.2.0: 1,654,161 / 9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc

## Next tasks

1. Commit T0 scaffolding; push; verify CI.
2. Write the CPython interop matrix test (`tests/interop_matrix.rs`).
3. Add a committed benchmark suite + baseline (T4).
4. Independent review of the bridge fix; red-team pass on the bridge.
5. WASM import policy + action pinning (security train).
6. Compatibility fixtures + install/artifact docs (packaging train).
