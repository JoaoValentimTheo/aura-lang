# ADR-0003 — Supported CPython versions

- **Status:** Accepted (Architecture Decision Council, 2026-09-30)
- **Supersedes:** queued decision HD-3
- **Related:** `docs/CPYTHON_COMPATIBILITY_TARGET.md`,
  `.github/workflows/ci.yml` (`cpython-interop`), `src/bridge/mod.rs`

## Context

The `py` feature links CPython via PyO3 (0.29, `auto-initialize`). A compatibility
claim must be exactly what CI verifies, not broader.

## Evidence

- CI (`cpython-interop`) tests **Linux × 3.10, 3.11, 3.12, 3.13** and
  **macOS × 3.12**, all green.
- The local development interpreter (3.9.25) also passes the full interop
  battery, but is outside any CI leg.
- The `test (windows-latest)` job deliberately skips `--all-features`: linking
  CPython through MSVC is not reliably configured by `actions/setup-python`
  without extra `LIB`/`PATH` wiring. The pure-Rust build is exercised on
  Windows.
- CPython 3.9 is at/near end of upstream maintenance; the PyO3 0.29 line
  supports modern CPython.

## Decision

Adopt explicit support tiers. **A version is "tested" only if a CI leg runs the
interop matrix on it.**

| Tier | Meaning | Versions / platforms |
|---|---|---|
| **TESTED** | CI runs `interop_matrix` + `python` suites; failures block | Linux: CPython **3.10, 3.11, 3.12, 3.13**. macOS: CPython **3.12** |
| **SUPPORTED (best effort)** | Expected to work; not CI-gated | macOS: 3.10, 3.11, 3.13; Linux/macOS other patch releases of the 3.10–3.13 lines |
| **UNSUPPORTED** | Not claimed | CPython ≤ 3.9; ≥ 3.14 until tested; any Windows `py` build until a CI leg exists |

- The primary v1 CPython claim: **"Aura supports CPython 3.10–3.13 through the
  `py` interoperability feature on Linux (fully tested) and macOS (best
  effort), with the documented value-conversion table, exception propagation,
  and import behavior."** Windows `py` support is explicitly **not claimed**
  until a CI leg exists.
- 3.9 is dropped from the claim: it is past upstream maintenance and would add
  a CI leg for little value.

## Consequences

- `docs/CPYTHON_COMPATIBILITY_TARGET.md` is updated to state these tiers
  verbatim; its evidence plan matches the actual matrix.
- Follow-up (tracked, not blocking): the optional Windows × 3.12
  `continue-on-error` interop leg is implemented as the
  `cpython-interop-windows-probe` job in `.github/workflows/ci.yml`: it installs
  CPython 3.12 via `setup-python` and wires `PYO3_PYTHON` plus the interpreter's
  `libs`/`Include` directories into the MSVC environment. It measures
  feasibility without gating the build; it is promoted to TESTED (and this ADR
  amended to claim Windows) only once it is reliably green.
- Expanding macOS to additional versions is low-value; left as best-effort.
