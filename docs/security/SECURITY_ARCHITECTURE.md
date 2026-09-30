# Aura Security Architecture

How Aura is structured to keep its security claims honest and its failure modes
bounded. Pair with `THREAT_MODEL.md` and `TRUST_BOUNDARIES.md`.

## Layering

```
source ──▶ lexer ──▶ parser ──▶ resolver ──▶ checker ──▶ runtime ──▶ host
              │          │           │            │           │
            E1xxx      E1xxx       E2xxx        E3xxx       E4xxx
                     (diagnostics are values; the host is not aborted)
```

- **Failures are values.** Lexing through execution return `
  Result<_, Diag>`; user input produces diagnostics, not panics. `unwrap_used`,
  `expect_used`, and `panic` are denied lints; `unsafe_code` is denied.
- **Authority is centralized at the host boundary** (`src/host.rs`). Every
  outside-world interaction (stdin/stdout, args, filesystem, clock, sleep)
  routes through one `Host` trait with native and browser implementations.
  A capability a host lacks is `E5002`, not a silent no-op.

## Feature gating as a security boundary

| Feature set | CPython linked | Intended use |
|---|---|---|
| `default` (includes `py`) | yes | full scripting + Python interop |
| `cli,repl,json,regex,time` | **no** | pure-Rust; no arbitrary Python |

CI asserts `pyo3` is absent from the pure-Rust dependency graph and smoke-tests
the resulting binary. This is the only supported way to obtain "no arbitrary
code execution beyond Aura itself."

## Resource-bounded failure

| Limit | Where | Failure |
|---|---|---|
| AST depth | parse/check/run | `E1015` |
| Call frames | runtime | `E4011` |
| Value depth / nodes | repr/equality/json/conversion | bounded output / diagnostic |
| Range materialization | runtime | diagnostic above 10M |
| Format width/precision | f-strings | `E4013` |
| Python boundary nodes | bridge | `E5002` |

Rejection recovers: a failed submission leaves prior REPL state intact.

## WASM authority

The wasm runtime is built from the same interpreter with zero host imports. The
browser host provides no filesystem/clock/args; those builtins report `E5002`.
The runtime is self-contained; the Playground verifies this.

## CPython boundary

The bridge (PyO3) maps Aura values structurally and bounds conversion work
(node budget + cycle detection) so a malicious/aliased/cyclic object graph
cannot exhaust the host *during conversion*. It does **not** sandbox Python —
that is out of scope and documented as such. Bridge diagnostics are attributed
to the Aura call site.

## Release integrity

- Frozen runtime artifacts are pinned by SHA-256 in `playground/build.mjs`; the
  build refuses to overwrite a differing artifact and `--check` verifies disk
  bytes against the manifest.
- The release workflow validates, builds per-target, smoke-tests each built
  executable, records SHA-256 per archive, and refuses `overwrite_files`.

## What is not yet architected

- Sandboxed execution of untrusted Aura or Python.
- ABI-level stability guarantees (Host ABI 1 / Playground API 1 are versioned
  but not frozen for 1.0).
- Reproducible builds, SBOM, and signed provenance (tracked debt).
