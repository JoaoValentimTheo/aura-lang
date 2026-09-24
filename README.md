<div align="center">

# Aura

**Aura** is a small, expression-oriented scripting language with a native
Rust runtime and optional Python interop through PyO3.

One spelling per construct. Immutable by default. No `null` — only `none`.
No Python required to run.

</div>

---

## Status

Aura v3 is a from-scratch Rust rewrite. The previous Python transpiler is
gone; the last Python release is preserved as tag `v0.2.0a8`. Version
`3.0.0-alpha.1` is an honest alpha: the language core, the runtime, the
standard library, the CLI, the REPL, and the Python bridge are implemented
and covered by an executable test suite, and the CI runs on Linux, macOS, and
Windows with and without CPython.

| Area | State |
|------|-------|
| Lexer, parser, AST | Implemented |
| Static checks (names, mutability, reserved words) | Implemented |
| Tree-walking interpreter | Implemented |
| Core stdlib + `json`, `regex`, `time` | Implemented |
| Structured concurrency / async | Not in this alpha |
| Compilation to native machine code | Not planned |
| Optional Python bridge (`py` feature) | Implemented |

## Requirements

* Rust **1.83+** to build. No runtime dependency on Python; the `py`
  feature (on by default) links CPython, and `--no-default-features
  --features cli,repl,json,regex,time` produces a pure-Rust binary.

## Install

From a checkout (requires Rust 1.83+):

```bash
cargo build --release
./target/release/aura run examples/tour.aura
./target/release/aura repl
```

Or install the binary onto your `PATH`:

```bash
cargo install --path . --no-default-features --features cli,repl,json,regex,time
aura run examples/tour.aura
```

Prebuilt binaries for Linux, macOS, and Windows are attached to GitHub
releases. Without Python at all:

```bash
cargo build --release --no-default-features --features cli,repl,json,regex,time
```

## The language

Aura has exactly **one spelling per construct**. The full grammar is in
[docs/grammar.md](docs/grammar.md), and it is enforced by `tests/grammar.rs`.

```aura
# Functions, immutability, and types.
fn fib(n) -> int {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}

fn main() {
    let name = "Aura"
    let mut total = 0
    for i in range(1, 6) { total = total + i }
    print(f"{name}: {total}, fib(10) = {fib(10)}")
}
```

### One spelling per construct

* `fn` declares functions — not `def`, `function`, or `func`.
* `and` / `or` / `not` are the logic operators — not `&&`, `||`, `!`.
* `none` is the only absence — not `null`, `nil`, or `undefined`.
* `else if` is supported and is the nested form `else { if ... }`; `match` is
  available for multi-way branching.
* `let` is immutable; `let mut` opts into reassignment.
* Function calls accept positional arguments, or named arguments for
  top-level functions (`f(x: 1)`); positional arguments come first. Named
  arguments are for `struct` construction too; enum variant payloads are
  positional.
* `use` and `pub` are reserved and inert in this version (see
  [docs/contract.md](docs/contract.md) §10).

### Values and types

```aura
int          float        bool         string
[T]          list of T
{string: V}  map (keys are strings)
T | none     optional
```

Types are optional annotations and are checked before execution. There is no
implicit coercion: `1 + "1"` is a compile error, and `let x: int = "a"` is
rejected before the program runs. The checker is conservative: it rejects only
mismatches it can prove and leaves the rest to the runtime (see
[docs/LANGUAGE_SPEC.md](docs/LANGUAGE_SPEC.md) §2.3, §6).

### Data and pattern matching

```aura
struct Point { x: int, y: int }

enum Shape {
    Circle(int),
    Rectangle(int, int),
}

fn area(s) -> int {
    return match s {
        Circle(r) -> 3 * r * r
        Rectangle(w, h) -> w * h
    }
}
```

### Functional core

```aura
let xs = [1, 2, 3, 4, 5, 6]
let result = xs |> filter((x) -> x % 2 == 0) |> map((x) -> x * x)

print(result)                  # [4, 16, 36]
print(xs.reduce((a, b) -> a + b, 0))
```

A pipeline continues on the same line: a newline ends the statement, so the
`|>` operator cannot start a continuation line.

### Errors

```aura
fn safe_div(a, b) -> int {
    if b == 0 { throw "division by zero" }
    return a / b
}

fn main() {
    try {
        print(safe_div(10, 2))
    } catch e -> {
        print(f"error: {e}")
    } finally {
        print("done")
    }
}
```

`try/catch` catches only explicit `throw` values. Runtime errors such as
division by zero, integer overflow, or an out-of-range index are fatal and
are not catchable.

## Python interop

Python lives behind an explicit boundary. With the default `py` feature:

```aura
fn main() {
    print(py_eval("1 + 2"))
    print(py_eval("'aura'.upper()"))
    print(py_eval("[1, 2, 3]"))
}
```

Values cross structurally: ints, floats, strings, bools, `none`, lists, and
dicts map both ways. Without the feature the same names exist but every call
is `E5002`, and the binary links no CPython — programs still run.

## CLI

| Command | Purpose |
|---------|---------|
| `aura run <file>` | Parse, check, and execute (or stdin with `-`) |
| `aura check <file>` | Parse and check without running |
| `aura eval <code>` | Run a one-liner |
| `aura repl` | Interactive session with persistent state |
| `aura version` | Print the version |

`aura run script.aura foo bar` passes `foo` and `bar` to the program, readable
via `args()`. `aura run -` reads the program from standard input. When the
program is read from a file, standard input is available to `read_line()`; when
it is read from stdin (`-`), `read_line()` has no remaining input and returns
`none`. `aura eval` exposes standard input to `read_line()` but `args()` is
empty. The REPL and the library expose neither.

### Scripting I/O

```aura
fn main() {
    # Program arguments (exclude the command, subcommand, and script path).
    for a in args() { print(a) }

    # Read a file; a missing path is `none`, not an error.
    let text = read_file("input.txt")
    if text == none {
        print("no input.txt")
    } else {
        write_file("copy.txt", text)
    }

    # Filter standard input line by line.
    let mut line = read_line()
    while line != none {
        print(line.upper())
        line = read_line()
    }
}
```

A genuine file or standard-input failure is `E4020`, which is fatal and is not
catchable (like other runtime diagnostics); a missing `read_file` path is
`none`. Filesystem access is local and unsandboxed.

## Architecture

Every entry point (library, CLI, REPL) runs the same `parse → check → execute`
pipeline. `aura::compile` is the single front end: it takes a compile mode
(`module` requires nothing; `program` requires `fn main`), so "what is a valid
program" is defined in exactly one place. The checker and the runtime share one
signature registry (`src/stdlib/signatures.rs`) for builtin arity, argument
types, method existence, and return types, so the two layers cannot disagree
about the standard library.

## Standard library

Core (always available): `print`, `len`, `to_string`, `to_int`, `to_float`,
`range`, `abs`, `min`, `max`, `push`, `pop`, `keys`, `values`, `sort`,
`reverse`, `map`, `filter`, `reduce`, `sum`, `assert`, `enumerate`, `zip`,
and the scripting I/O functions `read_line`, `read_file`, `write_file`,
`args`.

Methods: strings (`upper`, `lower`, `trim`, `split`, `replace`, `contains`,
`starts_with`, `ends_with`, `chars`), lists (`len`, `push`, `pop`, `first`,
`last`, `join`, `contains`, `sort`, `reverse`, `map`, `filter`, `reduce`),
maps (`get`, `has`, `keys`, `values`, `remove`).

Feature-gated modules: `json_encode` / `json_decode`, `regex_match` /
`regex_find` / `regex_find_all` / `regex_replace`, `time_now` / `time_unix` /
`sleep_ms`.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features --features cli,repl,json,regex,time
```

See [CONTRIBUTING.md](CONTRIBUTING.md). The language contract lives in
[docs/contract.md](docs/contract.md); diagnostic codes in
[docs/errors.md](docs/errors.md).

## License

MIT — see [LICENSE](LICENSE).
