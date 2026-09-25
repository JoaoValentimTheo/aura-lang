# Runtime & host

Aura's runtime talks to the outside world only through a **host**. The standard
library never calls the operating system directly. This single boundary is what
lets the same interpreter run natively and in a browser.

## The host contract

A host provides three mandatory capabilities and four optional ones:

| Capability | Required | Meaning |
|---|---|---|
| `write_stdout` | yes | write bytes to standard output |
| `read_line` | yes | read one line, or `none` at end of input |
| `args` | yes | the program arguments |
| `read_file` | optional | read a file, or `none` if absent |
| `write_file` | optional | create or truncate a file |
| `now_unix` / `now_local` | optional | the clock |
| `sleep_ms` | optional | sleep |

A capability the host does not provide reports `E5002` (capability
unavailable); a genuine failure of a provided capability reports `E4020`. The
two are never collapsed. A missing file, where the filesystem capability exists,
is the value `none`.

## Two hosts

### Native — `StdHost`

Used by the CLI, the REPL, and the library. It provides real process stdout,
real filesystem, a real clock, and real sleep. Native execution runs the
interpreter on a dedicated 64 MiB execution stack so the language's own limits
are reached before any host-stack limit.

### Browser — `BrowserHost`

Used by the Playground. It collects standard output in memory, reads standard
input from a supplied string, and holds arguments as plain data. It has **no**
filesystem, clock, or sleep, and it exposes no DOM, network, storage, or JS
handle. Sleep is never busy-waited.

## Execution substrates

```text
Native:   on a dedicated large-stack thread
WASM:     inline on the engine stack
```

The pipeline is identical on both:

```text
lex → parse → check → execute
```

The browser runtime module has zero imports. The Playground can pass it
source and options and read back a structured JSON result, and nothing else.

## Versioned runtime artifacts

Each runtime is an immutable artifact identified by version and hash. A version
entry records the language version, the runtime artifact, its SHA-256, the
Playground API version, and the Host ABI version. Selecting a version selects
the artifact that executes; historical artifacts are never silently replaced.

## The WASM ABI

The Playground wrapper exposes a small, pointer-free, byte-oriented ABI so it
needs no `unsafe` and can never gain host authority:

| Export | Purpose |
|---|---|
| `aura_abi_version` | Host ABI version |
| `aura_version_len` / `aura_version_byte` | language version string |
| `aura_runtime_version_len` / `aura_runtime_version_byte` | runtime version string |
| `aura_source_reset` / `aura_source_push` | feed source bytes |
| `aura_options_reset` / `aura_options_push` | feed options bytes |
| `aura_run` | execute; returns a status code |
| `aura_output_len` / `aura_output_byte` | read the JSON result |

The result is assembled from Aura's own structured types, so diagnostics are
real data, never scraped text.
