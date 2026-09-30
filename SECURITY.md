# Security Policy

Aura is a language, a runtime, a CLI/REPL, a WebAssembly runtime, a browser
Playground, and an optional CPython interoperability layer through PyO3.

This document states what Aura's security posture **is**, in precise terms, and
what it is **not**. It deliberately makes no claim of sandboxing or memory
safety beyond what is verified.

## Reporting a vulnerability

Report suspected vulnerabilities privately to the repository maintainers via
GitHub Security Advisories (Security → Report a vulnerability) rather than a
public issue. Include a minimal reproduction and the affected version/commit.
See `docs/security/INCIDENT_RESPONSE.md` for triage and disclosure handling.

## Supported versions

| Version | Supported |
|---|---|
| `0.2.0` (latest release) | yes (fixes via new patch releases) |
| `0.0.2`, `0.0.1` | no (historical, frozen artifacts retained) |

Pre-1.0, fixes ship as new versions; published artifacts and tags are never
overwritten.

## What is verified

- **No `unsafe` Rust in the core crate.** `unsafe_code = "deny"`; CI runs Miri
  over the pure-Rust library surface.
- **No panic from user-controlled Aura input** on the audited surfaces: the
  parser, checker, resolver, runtime, module system, CLI, and REPL produce
  structured diagnostics rather than aborting the host. Fuzz targets
  (lexer/parser/checker/runtime) run in CI smoke and nightly.
- **Bounded failure.** Resource limits (AST depth, call frames, value
  depth/nodes, range materialization, format width/precision, Python boundary
  node budgets) reject with diagnostics and recover.
- **Frozen artifacts are immutable** and hash-checked by `playground/build.mjs`.
- **Dependency auditing.** `cargo audit` runs in CI.
- **WASM runtime has zero host imports**, verified by the Playground suite and
  the website build (no ambient authority in the browser runtime).

## What Aura does NOT claim

- **Aura is not a sandbox.** The native runtime grants the process's ambient
  filesystem authority through documented builtins (`read_file`, `write_file`)
  and, with the `py` feature, **arbitrary CPython execution** (see below). This
  is intentional host authority, not a vulnerability.
- **`py_eval` / `py_call` / `py_import` execute arbitrary PYTHON with the full
  authority of the host process**, including filesystem, network, environment,
  and subprocess access, loading native extensions, and importing any
  importable module. There is no Python sandbox. Treat a build with the `py`
  feature as "the user can run Python as themselves."
- **The pure-Rust build** (`--no-default-features --features
  cli,repl,json,regex,time`) links no CPython; CI asserts `pyo3` is absent from
  its dependency graph.
- **Reproducible builds** are not yet claimed (see
  `RELEASE_ENGINEERING.md`).

## Trust boundaries

See `docs/security/TRUST_BOUNDARIES.md` and
`docs/security/SECURITY_ARCHITECTURE.md`. The threat model is in
`docs/security/THREAT_MODEL.md`.

## Supply chain

- Rust dependencies are minimal (`serde_json`, `regex`, `chrono`, `rustyline`,
  `pyo3`; proptest dev-only) and audited in CI.
- GitHub Actions are pinned to immutable commit SHAs (standard actions) and CI
  runs with least-privilege `contents: read` permissions. The release workflow
  requests `contents: write` only for publishing.
