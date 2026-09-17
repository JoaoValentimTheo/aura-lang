# Aura

**Aura** is a gradually-typed programming language that transpiles to Python.
It combines a clean, unambiguous syntax with the entire Python ecosystem:
first-class PyPI interop, a standard library, native threads and coroutines,
and post-quantum cryptography.

> **Status:** alpha (`0.1.0a7`). The syntax is standardized and frozen for the
> alpha series; see the [grammar](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/GRAMMAR.md)
> and the [changelog](https://github.com/JoaoValentimTheo/aura-lang/blob/master/CHANGELOG.md).

## Installation

```bash
pip install aura-language            # from PyPI
# or from a checkout:
pip install .
aura run examples/hello.aura
```

Optional post-quantum backend (recommended for real secrets):

```bash
pip install "aura-language[pqc]"     # adds cryptography>=44 (ML-KEM/ML-DSA)
```

Requires Python 3.10+. The runtime has no mandatory third-party dependencies
(on 3.10, `tomli` is installed automatically for reading `aura.toml`).

## Quick Start

```bash
aura init myapp
aura run myapp/src/main.aura
```

From a source checkout without installing:

```bash
git clone https://github.com/JoaoValentimTheo/aura-lang.git
cd aura-lang
python3 main.py run examples/hello.aura
```

## The Language

Aura has exactly **one spelling per construct** — no synonyms. The full
grammar lives in [`docs/GRAMMAR.md`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/GRAMMAR.md). Highlights:

```aura
// Functions, generics and traits.
def max[T](a: T, b: T) -> T {
  return a > b ? a : b
}

trait Shape {
  public def area() -> float
}

class Circle implements Shape {
  private let r: float = 1.0
  public def new(r: float) { self.r = r }
  public def area() -> float { return 3.14159 * self.r * self.r }
}

def parse(text) -> int | none {
  guard text.length() > 0 else { return none }
  return try { int(text) } catch e { none }
}

// The entry point: `aura run` invokes `main` for you.
def main() {
  // Immutable by default; opt into mutation.
  let name = "Aura"
  let mut count = 0
  count += 1

  // Pattern matching, pipes, guard clauses, error handling.
  let label = match count {
    case 0 -> "zero"
    case n if n > 0 -> "positive"
    case _ -> "other"
  }

  let sum = [1, 2, 3, 4] |> filter((x) => x % 2 == 0) |> reduce((a, x) => a + x, 0)
}
```

Canonical spellings include `def` (not `fn`), `new` (not `init`), `none` (not
`null`), `not`/`and`/`or` (not `!`/`&&`/`||`), and `[T]` generics (not `<T>`).
Removed spellings raise a clear `SyntaxError`.

## Concurrency

Aura supports native OS threads and `async def`/`await`.

Threads (globals are shared; guard them with a lock):

```aura
import stdlib.threading as threading

let mut total = 0
let lock = threading.lock()

def worker(n) {
  let mut i = 0
  while i < n {
    lock.acquire()
    total += 1
    lock.release()
    i += 1
  }
}

def main() {
  let t = threading.spawn((x) => worker(x), 100)
  t.join()
  print(total)
}
```

Coroutines:

```aura
import stdlib.asyncio as aio

async def work(n) {
  await aio.sleep(0.01)
  return n * 2
}

async def main() {
  let results = await aio.gather(work(1), work(2), work(3))
  print(results)
}
```

See [`aura/stdlib/README.md`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/aura/stdlib/README.md) for the full API.

## Cryptography

`stdlib.crypto` provides real SHA-2/SHA-3/SHAKE hashing, HMAC, HKDF and secure
randomness, plus post-quantum **ML-KEM** (FIPS 203) and **ML-DSA** (FIPS 204)
behind a pluggable backend. Install the `pqc` extra for a vetted
implementation; otherwise a clearly-labelled reference backend is used and
`require_production_backend()` fails loudly.

```aura
import stdlib.crypto as crypto

def main() {
  print(crypto.sha3_256("hello"))

  let kp = crypto.kem_keypair()
  let enc = crypto.kem_encapsulate(kp.public_key)
  let shared = crypto.kem_decapsulate(kp.secret_key, enc.ciphertext)

  let signer = crypto.dsa_keypair()
  let sig = crypto.dsa_sign(signer.secret_key, "payload")
  print(crypto.dsa_verify(signer.public_key, "payload", sig))
}
```

## Python Interop

Aura transpiles to Python, so community PyPI packages are first-class. Install
one with `aura add`, declare it in `aura.toml`, then import it directly, or use
the explicit bridge for dynamic access:

```aura
import python

def main() {
  let requests = python.import_module("requests")
  let text = requests.get("https://example.com").text

  let re = python.load("re")
  print(re.findall("[0-9]+", "a1b22c333"))

  print(python.eval("sum(range(10))"))
}
```

The bridge exposes `import_module`, `load`, `eval`, `exec_code`, `call`,
`getattr`/`setattr`/`hasattr`, `is_available`, `to_aura`/`to_python`, and more.
See [`aura/stdlib/python.py`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/aura/stdlib/python.py).

## Aura Patterns (AUP)

[AUP](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/AUP.md) is a catalog of idiomatic solutions — option, builder,
strategy, pipelines, error handling, memoization, observer, resource
management, worker pools and hybrid crypto. Every pattern is a runnable
program under [`examples/aup/`](https://github.com/JoaoValentimTheo/aura-lang/tree/master/examples/aup):

| Pattern | File |
|---------|------|
| Option (result-or-null) | `examples/aup/option.aura` |
| Builder | `examples/aup/builder.aura` |
| Strategy (traits) | `examples/aup/strategy.aura` |
| Pipeline | `examples/aup/pipeline.aura` |
| Typed error handling | `examples/aup/error_handling.aura` |
| Memoization / caching | `examples/aup/memoize.aura` |
| Observer | `examples/aup/observer.aura` |
| Resource management | `examples/aup/resource.aura` |
| Worker pool | `examples/aup/worker_pool.aura` |
| Hybrid secure message | `examples/aup/hybrid_crypto.aura` |

## CLI

Installed as `aura <command>`; the same commands work via `python3 main.py <cmd>`.

| Command | Description |
|---------|-------------|
| `aura run <file> [-v]` | Transpile and execute (optionally show Python) |
| `aura transpile <file> [-o out.py]` | Convert Aura to Python |
| `aura check <file>` | Type, mutability and rule checks |
| `aura format <file>` | Format source |
| `aura lint <file>` | Style warnings (non-zero exit on warnings) |
| `aura test <dir>` | Run `.aura` files |
| `aura repl` | Interactive REPL |
| `aura init [name]` | Create `aura.toml` and `src/main.aura` |
| `aura add <pkg>` | Add and install a dependency |
| `aura install` | Install dependencies from `aura.toml` |
| `aura deps` | List declared dependencies |
| `aura version [bump]` | Show or bump the version |
| `aura debug <file> [-t]` | Run under the trace debugger |
| `aura lsp` | Start the language server (stdio) |

## REPL

```bash
aura repl
```

```
aura> let mut total = 0
aura> for i in 1..5 { total += i }
aura> total
10
aura> :type total
int  (value: 10)
aura> import python
aura> python.eval("2 ** 8")
256
aura> :q
```

The REPL enforces the same rules as `aura check`, including mutability across
chunks, and supports top-level `await`.

| Command | Description |
|---------|-------------|
| `:help` | Show help |
| `:vars` | List session bindings |
| `:type <expr>` | Evaluate and show the value/type |
| `:ast <code>` | Print the syntax tree |
| `:py <code>` | Run raw Python in the session |
| `:load <file>` | Execute an Aura file into the session |
| `:run <file>` | Run an Aura file as a program |
| `:history` | Show entered chunks |
| `:reset` | Clear all bindings |
| `:q` | Quit |

A bare expression prints its value and stores it in `_`. Multi-line input
continues automatically while brackets are open or a line ends with a
continuation token.

## Documentation

- [Documentation index](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/README.md)
- [Grammar (canonical source of truth)](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/GRAMMAR.md)
- [Diagnostics reference (E##/W##)](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/ERRORS.md)
- [Language Reference](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/LANGUAGE.md) · [Referência da Linguagem](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/LANGUAGE_PT.md)
- [Aura Patterns (AUP)](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/AUP.md)
- [Standard Library](https://github.com/JoaoValentimTheo/aura-lang/blob/master/aura/stdlib/README.md)
- [Type System](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/TYPES.md) · [Sistema de Tipos](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/TYPES_PT.md)
- [Architecture](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/DESIGN.md)
- [Contributing](https://github.com/JoaoValentimTheo/aura-lang/blob/master/CONTRIBUTING.md) · [Security](https://github.com/JoaoValentimTheo/aura-lang/blob/master/SECURITY.md) · [Changelog](https://github.com/JoaoValentimTheo/aura-lang/blob/master/CHANGELOG.md)

## Development

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"            # pytest, coverage, hypothesis, ruff, mypy

python -m pytest tests/ -q         # full suite
python -m pytest tests/ --cov=aura # with coverage (floor: 65%)
ruff check aura/                   # lint
mypy aura/                         # types
```

The suite covers every syntax construct, the full object system, concurrency,
cryptography and hardening regressions, and runs the generated `.aura` corpora.
It also includes property-based tests (Hypothesis) for the lexer, parser and
transpiler, a differential suite that compares Aura against equivalent Python,
and tests that enforce the diagnostics catalogue and the Aura Pattern standard.
See [CONTRIBUTING.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/CONTRIBUTING.md) for the architecture and how to add a
language feature.

## License

MIT — see [LICENSE](https://github.com/JoaoValentimTheo/aura-lang/blob/master/LICENSE).