# The Aura Playground

**Status:** Gaiola 7 implementation. The Playground executes the **real Aura
WebAssembly runtime** produced from this repository. It contains no second
interpreter and no JavaScript/TypeScript reimplementation of Aura semantics.

---

## 1. Architecture

```text
Browser UI (playground/index.html, web/app.js)
    ↓  one fresh Worker per run
Web Worker (web/worker.js)
    ↓  loads the selected immutable artifact
Aura WASM runtime (playground/runtime → runtimes/<version>/*.wasm)
    ↓  links the real aura-lang crate
Aura Runtime (src/)
    ↓  Interp owns Box<dyn Host>
BrowserHost (src/host.rs)
    ↓
ExecutionResult (structured JSON)
```

The browser is an **orchestration layer only**. It selects a versioned
artifact, spawns a Worker, forwards source/args/stdin, and renders the
structured result. Cancellation is Worker termination, never an Aura `throw`.

### Why a Worker

Aura programs can run forever (`while true {}`) or recurse until `E4011`. A
synchronous wasm call cannot be interrupted cooperatively, so the Playground
runs every execution in its own Worker and hard-terminates it on **Stop**,
completion, or restart. The UI thread never executes user code.

### Generation ids

`app.js` tags every run with a monotonically increasing `runId` and ignores any
message whose id is not the current run, so a terminated run can never
overwrite a newer result.

---

## 2. BrowserHost

`aura::host::BrowserHost` (`src/host.rs`) is the single browser capability
boundary. It provides:

| Capability | Behavior |
|---|---|
| `write_stdout` | collected in an in-memory buffer (optionally bounded) |
| `read_line` | consumed from a caller-supplied stdin string |
| `args` | caller-supplied plain string list |
| `read_file` / `write_file` | `E5002` (unavailable) |
| `now_unix` / `now_local` | `E5002` (unavailable) |
| `sleep_ms` | `E5002` (unavailable; never busy-waited) |

It holds only plain data and never exposes the DOM, JS, the network,
localStorage, IndexedDB, process state, or handles. Its optional
stdout limit is an **application** policy, not a language rule: exceeding it is
`E4020`, so a runaway `print` loop is bounded.

Error semantics are unchanged: unavailable capability → `E5002`; genuine I/O
failure → `E4020`; a missing file where the filesystem exists → `none`.

---

## 3. The wasm ABI

`playground/runtime` is a tiny crate with a **zero-import** wasm module. The
ABI is pointer-free (so the wrapper stays `#![deny(unsafe_code)]`) and
byte-oriented:

| Export | Purpose |
|---|---|
| `aura_abi_version() -> u32` | Host ABI version (`1`) |
| `aura_version_len()` / `aura_version_byte(i)` | language version (`0.0.1`) |
| `aura_runtime_version_len()` / `aura_runtime_version_byte(i)` | runtime artifact version |
| `aura_source_reset()` / `aura_source_push(word, nbytes)` | feed source bytes |
| `aura_options_reset()` / `aura_options_push(word, nbytes)` | feed options bytes |
| `aura_run() -> u32` | `0` ok, `1` diagnostic, `2` internal |
| `aura_output_len()` / `aura_output_byte(i)` | read the JSON result |

The options mini-protocol is `arg <text>\n` lines followed by an optional
`stdin-bytes <n>\n` header and exactly `n` raw bytes.

The result document is structured, built from Aura's own `Diag` and
`line_col` — never scraped from rendered text:

```json
{
  "status": "ok",
  "stdout": "…",
  "result": null,
  "diagnostics": [
    { "code": 4026, "code_text": "E4026", "message": "…", "line": 1, "column": 1 }
  ]
}
```

The loader (`web/runtime.mjs`) refuses any artifact that declares imports, so a
runtime can never gain host authority.

---

## 4. Versioning

`playground/runtimes/manifest.json` is the version registry. Each entry records
the language version, runtime artifact, artifact SHA-256, Playground API
version, and Host ABI version. `current` is only an alias to an immutable
entry. `playground/build.mjs`:

* builds the wasm artifact,
* copies it to `runtimes/<version>/`,
* hashes it,
* writes the manifest,
* **refuses to overwrite** an existing version directory whose hash differs
  (historical artifacts are immutable; bump the crate version instead).

`node playground/build.mjs --check` re-derives hashes from disk and fails on
any drift; it is exercised by the test suite and CI.

### 0.0.1 vs 0.0.2

Three identities are deliberately distinct and are recorded separately in the
manifest: the **release** (`release_version`), the **language semantics**
(`language_version`), and the **runtime artifact** (`runtime_version`).

The published **0.0.1** predates the WebAssembly execution substrate. Its
frozen source spawns OS threads for parsing and execution, which
`wasm32-unknown-unknown` does not provide; building it for wasm yields a module
that returns `E4026` for every program. It therefore has **no browser
runtime**, and the manifest records it honestly as `available: false` with a
reason. The first wasm-executable runtime is the **0.0.2** release
(`runtimes/0.0.2/`). No historical artifact is fabricated or overwritten.

Aura **0.0.2** is a release of the runtime and tooling, not a language change:
it implements the frozen **0.0.1** language semantics. The manifest therefore
records `release_version: "0.0.2"`, `runtime_version: "0.0.2"`, and
`language_version: "0.0.1"`.

A version entry is *real*, not decorative: the selected entry's immutable
artifact URL is exactly what the Worker fetches and executes, and the loader
checks the artifact's ABI against the manifest's `host_abi_version`.

---

## 5. Running the Playground

```bash
# build the immutable runtime + manifest
node playground/build.mjs

# serve the UI (root is playground/)
node playground/tests/node/serve.mjs 8080
# open http://127.0.0.1:8080/
```

## 6. Tests

```bash
# Rust: engine + host
cargo test --manifest-path playground/runtime/Cargo.toml
cargo test --no-default-features --features cli,repl,json,regex,time

# JS: manifest/immutability, ABI, native/wasm differential, browser, worker
node playground/tests/node/run-all.mjs
```

Browser and Worker suites require Playwright's Chromium; when it is not
installed they report **skipped**, never a silent pass.
