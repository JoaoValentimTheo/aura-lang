# Installing Aura

Aura runs on Linux, macOS, and Windows (native), and in the browser (WebAssembly
Playground). This guide covers the supported installation paths for **0.2.1**.

## Requirements

| Path | Requirement |
|---|---|
| Prebuilt binary | none (self-contained) |
| Build from source | Rust **1.83+** |
| Python interop | a CPython the build can link (optional; see below) |

## Option 1 — Prebuilt release binaries

Attached to each [GitHub release](https://github.com/JoaoValentimTheo/aura-lang/releases)
are archives per target, each with a `.sha256`:

| Target | Archive |
|---|---|
| Linux x86-64 | `aura-x86_64-unknown-linux-gnu.tar.gz` |
| macOS Apple Silicon | `aura-aarch64-apple-darwin.tar.gz` |
| Windows x86-64 (MSVC) | `aura-x86_64-pc-windows-msvc.tar.gz` |

Released binaries are built **pure-Rust** (no CPython). Verify the checksum,
then extract and run:

```bash
sha256sum -c aura-<target>.sha256
tar xzf aura-<target>.tar.gz
./aura-<target>/aura version
```

## Option 2 — Build from a checkout

```bash
git clone https://github.com/JoaoValentimTheo/aura-lang
cd aura-lang
cargo build --release
./target/release/aura run examples/tour.aura
```

Install onto your `PATH`, without CPython:

```bash
cargo install --path . --no-default-features --features cli,repl,json,regex,time
```

or with Python interop (links the CPython found by the build):

```bash
cargo install --path .
```

## Feature sets

| Features | CPython linked | Use when |
|---|---|---|
| `default` (incl. `py`) | yes | you want `py_eval`/`py_import`/`py_call` |
| `cli,repl,json,regex,time` | no | you must not allow arbitrary Python execution |

> **Security:** the `py` feature runs **arbitrary Python with full host
> authority** and is **not a sandbox**. Choose the pure-Rust feature set when
> that is not acceptable. See [SECURITY.md](../SECURITY.md).

## Quickstart

`hello.aura`:

```aura
fn main() {
    print("hello, world")
}
```

```bash
aura run hello.aura        # compile and run
aura check hello.aura      # type-check only
aura eval 'print(1 + 2)'   # one-liner
aura repl                  # interactive session
echo 'print(1 + 2)' | aura run -   # from stdin
```

## Python interop (optional)

```bash
cargo build --release   # default features include py
./target/release/aura run - <<'EOF'
fn main() {
    print(py_version())
    print(py_eval("1 + 2"))
    print(py_call("math", "sqrt", 16.0))
}
EOF
```

The exact supported surface and conversion rules are in
[CPYTHON_COMPATIBILITY_TARGET.md](CPYTHON_COMPATIBILITY_TARGET.md).

## Verifying a release

```bash
tar xzf aura-<target>.tar.gz
sha256sum -c aura-<target>.sha256
./aura-<target>/aura version    # prints the release version
```

## Troubleshooting

- **`py_*` reports `E5002`** — the binary was built without the `py` feature.
- **`E2022` on a path** — the path is not a regular file (directory/symlink).
- **`E4027`** — the program has no `main` (use `aura eval` for a bare script).
