# Aura Trust Boundaries

Each boundary below states: the input crossing it, the authority on the other
side, and whether that authority is intentional.

## 1. Aura source → compiler front end

- **Input:** arbitrary bytes (Aura source).
- **Authority:** CPU/memory only, bounded by resource limits.
- **Intentional:** yes. Invalid input must yield a diagnostic, never a panic.
- **Guarded by:** lexer/parser/checker fuzzing, property tests, limit tests.

## 2. Aura program → native runtime

- **Input:** a checked Aura program.
- **Authority:** full process authority through documented host builtins:
  `read_file`, `write_file`, `args`, standard I/O, clock, `sleep_ms`.
- **Intentional:** yes. This is ordinary local scripting authority, comparable
  to any scripting language run as the invoking user. **Not a sandbox.**
- **Note:** filesystem builtins are documented language surface; they are not a
  vulnerability.

## 3. Aura program → CPython (`py` feature)

- **Input:** Aura strings passed to `py_eval` / `py_import` / `py_call`.
- **Authority:** **arbitrary Python execution with the full authority of the
  host process**, including:
  - reading/writing/deleting any file the user can;
  - importing any importable module, including loading **native extensions**;
  - network access, environment variables, subprocess spawning;
  - reading process state and the current working directory.
- **Intentional:** yes, and explicitly documented. Aura provides **no Python
  sandbox and no capability restriction**. Red-team evidence:
  `py_eval("__import__('os').getcwd()")` returns the host CWD; `py_import("os")`
  enumerates `os`.
- **Consequence:** a deployment that must not allow arbitrary code execution
  must build without the `py` feature. The pure-Rust feature set links no
  CPython (CI verifies `pyo3` absent).
- **Guarded by:** `SECURITY.md` warning; phase separation of the `py` feature;
  boundary node budgets (DoS bounding, not sandboxing).

## 4. Aura program → WebAssembly runtime

- **Input:** Aura source in the browser/Playground.
- **Authority:** **none beyond the wasm sandbox.** The runtime exposes **zero
  host imports** (verified by the Playground ABI suite and website build).
  Filesystem/clock/args builtins resolve to structured `E5002` capability
  errors in hosts that do not provide them.
- **Intentional:** yes. The browser runtime is self-contained.

## 5. Playground web → worker → wasm

- **Input:** Aura source text and virtual-project JSON.
- **Authority:** compute + memory in the worker; runtime artifacts are loaded
  from the same origin under a versioned manifest with SHA-256 pinning.
- **Intentional:** yes.

## 6. Runtime manifest → browser loader

- **Input:** `manifest.json` version selection.
- **Authority:** chooses which wasm artifact executes; the loader verifies the
  artifact hash and, for virtual projects, feature-detects `aura_project_*`.
- **Intentional:** yes. A runtime lacking a capability reports a structured
  incompatibility rather than executing unsupported syntax.

## 7. Filesystem → module graph (native provider)

- **Input:** paths and sibling `.aura` files.
- **Authority:** reads sources owned by the entry/module tree; resolves
  physical children as logical submodules (eager, bounded discovery).
- **Intentional:** yes. Symlink/path policy enforced; traversal bounded to
  owned children; diagnostics attribute malformed children to their own source.

## 8. Tag push → release pipeline

- **Input:** a `v*` tag matching the crate version.
- **Authority:** builds native artifacts for 3 targets + the versioned wasm
  runtime, records SHA-256, publishes a GitHub release; historical assets are
  never overwritten.
- **Intentional:** yes. `publish` depends on `validate` and `build`.

## 9. Dependencies / actions → build

- **Input:** crate and GitHub Action code.
- **Authority:** arbitrary code at build time.
- **Intentional:** yes, under a trusted-supply-chain assumption; minimized
  dependencies; `cargo audit` in CI; action SHA-pinning is tracked debt.
