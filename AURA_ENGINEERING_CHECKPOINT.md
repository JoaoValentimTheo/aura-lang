# Aura Engineering Checkpoint

Resumable operational state for the Road-to-1.0 program.

## State

```
PROGRAM HEAD:            f262180 (pushed)
REMOTE HEAD:             f262180
LATEST RELEASE:          v0.2.0
LATEST PRERELEASE:       none
LATEST DEV RUNTIME:      0.2.0-dev.2
CURRENT RELEASE TRAIN:   T1 (patch) closing; T2 (CPython) started
```

## Active work

- T0 organization bootstrap (docs) — done.
- T1 patch train: bridge provenance fix + interop matrix — CI green.
- T2 CPython train: compatibility target, interop matrix, multi-version CI — done.
- T3 security train: threat model, trust boundaries, WASM import policy,
  action SHA-pinning + least-privilege CI (TD-01 closed) — done.
- T4 performance train: shape guards + baseline (`docs/engineering/PERFORMANCE.md`) — done.
- T5 compat: `tests/compat.rs` pins the released 0.2.0 surface — done.
- T5 packaging/release docs: RELEASE_ENGINEERING.md, CHANGELOG.md, README security note — done.
- **BLOCKED (human):** tagging a patch release pending HD-4 (release vs language version).

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
- `docs(org): establish the Aura engineering organization and v1 program`
- `test(interop): add the CPython interoperability matrix`
- `docs(security): strip trailing whitespace in trust boundaries`
- `test(perf): add scaling shape guards and a timing baseline`
- `ci(interop): add a CPython version/platform interop matrix`

## Pushes / CI runs

- Green: CI 36765658391 at `6d25985`; CI 36769088433 at `c12b39f`.
- Pushed `c12b39f..f262180` (perf + CI interop matrix); CI running.

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

1. Commit/close T2/T3/T5 docs; push; verify CI.
2. Human decision on HD-4 (unblocks the first gated patch release `0.2.1`).
3. Write the install/quickstart + artifact-matrix end-user docs (packaging GREEN).
4. Independent review + release audit of the train (release author ≠ approver).
5. Broaden performance workloads (REPL, WASM startup, module graph large-N).
6. Compat upgrade matrix across released artifacts.
