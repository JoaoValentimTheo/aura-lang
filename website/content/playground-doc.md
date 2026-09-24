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

## Version selection

The runtime selector chooses a **versioned, immutable artifact**. Each version
entry records its artifact and SHA-256 hash. Selecting a version selects exactly
the artifact that executes — see [Runtime & host](/docs/runtime-doc/) and the
[releases page](/releases/) for the available entries.

## Diagnostics

The Playground shows Aura's structured diagnostics with their stable codes and
source positions, not scraped terminal text. Codes such as `E1015`, `E3001`,
`E4011`, `E4026`, and `E5002` appear as they would from the CLI.
