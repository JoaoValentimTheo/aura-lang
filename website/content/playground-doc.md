# Playground

The [Playground](/playground/) runs the **real Aura runtime**, compiled to
WebAssembly, inside a Web Worker in your browser. No second interpreter, no
JavaScript reimplementation of Aura — the same interpreter the CLI uses.

## How execution works

```text
Browser UI
    ↓  one fresh Web Worker per run
Web Worker
    ↓  loads the selected immutable artifact
Aura WASM runtime
    ↓
Aura Runtime  →  BrowserHost
    ↓
Structured execution result
```

## Isolation

* Every run executes in its own Worker. User code never touches the UI thread.
* **Stop** terminates the Worker. This is the hard-cancellation mechanism: a
  `while true {}` loop is stopped by terminating the running instance, not by an
  Aura-level signal. Cancellation is not catchable by Aura `try/catch`.
* The runtime module has **zero imports**: it can reach no DOM, network,
  storage, filesystem, or JS handle. Its only effect is its own linear memory,
  which the page reads back.

## Capabilities

The browser host provides standard output, standard input (the text you supply),
and arguments. Filesystem, clock, and sleep are **unavailable** and report
`E5002`; the Playground never fakes them.

| Capability | In the Playground |
|---|---|
| `print` | shown in the output panel |
| `read_line` | reads the Standard input box |
| `args` | reads the Arguments box (one per line) |
| `read_file` / `write_file` | `E5002` |
| `time_now` / `time_unix` / `sleep_ms` | `E5002` |
| `http_get` / `http_request` | not registered on the released `0.3.1` runtime (a call is `E2003`, undefined name); **available on the `0.3.2` development runtime** behind a per-origin permission prompt |

### Browser HTTP (development runtime)

The published `0.3.1` runtime predates the HTTP builtin surface, so
`http_get`/`http_request` are not registered there and a call is a compile-time
`E2003` (undefined name) — no request is possible. The development runtime
(`0.3.2-dev.5`, Host ABI 2) adds them through an **experimental permission
model**:

* Network is **off by default**. Each request asks the page to authorize a
  **specific origin**, and nothing is dispatched until you allow it.
* A grant is scoped to one origin for the browser session: allowing one site
  never authorizes another, and a redirect that changes origin is not followed.
* Requests carry **no cookies and no stored credentials**
  (`credentials: "omit"`), and use a restrictive referrer policy.
* A denial is `E5002`; a browser CORS rejection or transport failure is
  `E4020`. There is no proxy and no CORS bypass — the browser's own rules
  apply.
* Methods and options follow the language: `http_get(url)`,
  `http_request(method, url)`, `http_request(method, url, options)` with
  `headers`, `body`, and `timeout_ms`.

This runtime is a **development preview**: it is not a release, it may change,
and it never becomes the default on its own. The Playground labels it
`development runtime` in the selector.

## Multi-file projects

The browser has **no host filesystem authority** — `read_file` / `write_file`
and the filesystem module provider are `E5002`. That is not the same as being
single-file: the Playground executes a *project*, a flat set of Aura sources
with one entry file, supplied by the page itself (virtual sources, held in the
browser session). The file tabs add, rename, set the entry file, delete, and
reset sources; each file's declared provider child links (`name → file`) define
the module layout, and Aura module identity still comes from the source text,
never from a filename.

Execution goes through the additive Host ABI 1 `aura_project_reset` /
`aura_project_push` / `aura_run_project` transport into the same
`InMemorySourceProvider → ModuleGraphBuilder → resolver → checker → runtime`
path used by native modules. A one-file project keeps the historical
single-source path. A runtime artifact that predates the project exports
reports a structured capability error instead of silently running one file.

## Version selection

The runtime selector chooses a **versioned, immutable artifact**. Each version
entry records its artifact and SHA-256 hash. Selecting a version selects exactly
the artifact that executes — see [Runtime & host](/docs/runtime-doc/) and the
[releases page](/releases/) for the available entries.

## Diagnostics

The Playground shows Aura's structured diagnostics with their stable codes and
source positions, not scraped terminal text. Codes such as `E1015`, `E3001`,
`E4011`, `E4026`, and `E5002` appear as they would from the CLI.
