# Aura WebAssembly Import Policy

The Playground WebAssembly runtime is a distinct trust boundary: it executes
Aura source supplied by a web page inside a browser. This policy governs what
that module may import and export, and how the Playground selects runtimes.

## Import policy

**The Aura wasm runtime MUST have zero host imports.**

Independently verified with `WebAssembly.Module.imports` (Node's WebAssembly
API), not by inspecting Rust source:

| Artifact | Imports |
|---|---|
| `0.0.2/aura_playground_runtime.wasm` | 0 |
| `0.2.0/aura_playground_runtime.wasm` | 0 |
| `0.2.0-dev.2/aura_playground_runtime.wasm` | 0 |
| `0.2.1-dev.2/aura_playground_runtime.wasm` | 0 |
| `0.2.1-dev.3/aura_playground_runtime.wasm` | 0 |

Consequences:

- The runtime cannot read the filesystem, network, environment, clock, or
  process state from the guest side. Aura builtins that require host authority
  (`read_file`, `write_file`, `read_line`, `args`, `time_now`, `sleep_ms`,
  `py_*`) are **unavailable** in the browser and report the documented
  capability diagnostic (`E5002`) rather than acting.
- There is no ambient authority to escape; the module is self-contained.
- A new import is an **immediate blocker**: it requires purpose, security
  analysis, browser implications, documentation, and tests, and would change
  the runtime's authority story.

## Export surface

The module exports only the C-ABI handshake and data-transfer functions; the
host never calls in except through these.

- **Host ABI 1, Playground API 1.**
- Base exports: `memory`, `aura_abi_version`, `aura_run`, `aura_source_push`,
  `aura_source_reset`, `aura_options_push`, `aura_options_reset`,
  `aura_output_byte`, `aura_output_len`, `aura_version_byte/len`,
  `aura_runtime_version_byte/len`.
- Additive (development runtime carrying virtual projects): `aura_run_project`,
  `aura_project_push`, `aura_project_reset`.

The host ABI is **additive-only**; a breaking change needs a new ABI version.

## Runtime identity and capability detection

- Each runtime identity is immutable and pinned by SHA-256 in the manifest and
  in `playground/build.mjs`.
- The Playground feature-detects capabilities (e.g. `aura_run_project`) and
  reports a structured incompatibility when an older runtime is selected for a
  multi-file project (`runtime artifact <id> does not support virtual
  projects`). It never silently executes unsupported syntax.
- A language-semantics change requires a **new** development runtime identity;
  historical identities are never overwritten.

## Verification

- `playground/tests/node/abi.test.mjs` asserts zero imports and the export set
  per version.
- The website build re-verifies the manifest and that the wasm is served from
  the same origin under the versioned path.
- This document's import table is reproduced with a `WebAssembly.Module`
  inspection (see the review evidence) so the claim is externally checkable.

## Out of scope

This policy does not claim the wasm runtime sandboxes *resource* use (CPU/memory
within the module) — that is bounded by the same interpreter limits as native,
and by the browser's own controls. It claims **no ambient host authority**,
which is what zero imports provides.
