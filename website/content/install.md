# Installing Aura

Aura builds with Cargo. The pure-Rust build has no runtime dependency on Python;
the optional Python bridge links CPython only when you ask for it.

## Requirements

* Rust **1.83** or newer.
* A C toolchain for linking (standard on Linux, macOS, and Windows with the
  MSVC toolchain).
* Optional: Python 3.12 if you want the `py` feature.

## Build from a checkout

```bash
git clone https://github.com/JoaoValentimTheo/aura-lang
cd aura-lang

# The shipped pure-Rust binary: no CPython linked.
cargo build --release --no-default-features --features cli,repl,json,regex,time
./target/release/aura version
```

With the default features — which include the Python bridge — a plain
`cargo build --release` links CPython:

```bash
cargo build --release
```

## Install onto your PATH

```bash
cargo install --path . --no-default-features --features cli,repl,json,regex,time
aura run examples/hello.aura
```

## Prebuilt binaries

The release workflow publishes per-platform tarballs built for:

* `x86_64-unknown-linux-gnu`
* `aarch64-apple-darwin`
* `x86_64-pc-windows-msvc`

They are attached to each [GitHub release](https://github.com/JoaoValentimTheo/aura-lang/releases).
The current release is **v0.0.2**, which implements the frozen **0.0.1** language
semantics; **v0.0.1** was the first usable public release.

## Feature flags

| Feature | Default | Provides |
|---|---|---|
| `cli` | yes | the `aura` binary |
| `repl` | yes | `aura repl` |
| `json` | yes | `json_encode`, `json_decode` |
| `regex` | yes | `regex_match`, `regex_find`, `regex_find_all`, `regex_replace` |
| `time` | yes | `time_now`, `time_unix`, `sleep_ms` |
| `py` | yes | `py_eval`, `py_import`, `py_call`, `py_version` |

To confirm a build links no CPython, use the pure-Rust feature set above: the
`pyo3` crate is absent from the dependency graph and every Python builtin
reports `E5002`.

> **Playing without installing.** The [Playground](/playground/) needs no
> toolchain at all — it runs the same interpreter compiled to WebAssembly.
