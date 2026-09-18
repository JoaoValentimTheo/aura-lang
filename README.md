<div align="center">

<img src="aura_logo.png" alt="Aura" width="220">

# Aura

**Aura** is a gradually-typed programming language that transpiles to Python. It
pairs a clean, unambiguous syntax with the entire Python ecosystem: first-class
PyPI interop, a standard library, native threads and coroutines, and
post-quantum cryptography.

[![CI](https://github.com/JoaoValentimTheo/aura-lang/actions/workflows/ci.yml/badge.svg)](https://github.com/JoaoValentimTheo/aura-lang/actions/workflows/ci.yml)
[![Release](https://github.com/JoaoValentimTheo/aura-lang/actions/workflows/release.yml/badge.svg)](https://github.com/JoaoValentimTheo/aura-lang/releases)
[![PyPI](https://img.shields.io/pypi/v/aura-language.svg)](https://pypi.org/project/aura-language/)
[![Python](https://img.shields.io/pypi/pyversions/aura-language.svg)](https://pypi.org/project/aura-language/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/JoaoValentimTheo/aura-lang/blob/master/LICENSE)

> **Status:** alpha (`0.1.0a13`). The syntax is standardized and frozen for the
> alpha series; see the [grammar](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/GRAMMAR.md)
> and the [changelog](https://github.com/JoaoValentimTheo/aura-lang/blob/master/CHANGELOG.md).

</div>

---

## Table of contents

- [Installation](#installation)
- [Quick start](#quick-start)
- [The language](#the-language)
- [Classes](#classes)
- [Concurrency](#concurrency)
- [Cryptography](#cryptography)
- [Python interop](#python-interop)
- [Aura Patterns (AUP)](#aura-patterns-aup)
- [CLI](#cli)
- [Projects and dependencies](#projects-and-dependencies)
- [REPL](#repl)
- [Documentation](#documentation)
- [Development](#development)
- [License](#license)

## Installation

```bash
pip install aura-language            # from PyPI
```

Or from a checkout:

```bash
git clone https://github.com/JoaoValentimTheo/aura-lang.git
cd aura-lang
pip install .
aura run examples/hello.aura
```

Optional post-quantum backend (recommended for real secrets):

```bash
pip install "aura-language[pqc]"     # adds cryptography>=44 (ML-KEM/ML-DSA)
```

Requires Python 3.10+. The runtime has no mandatory third-party dependencies
(on 3.10, `tomli` is installed automatically for reading `aura.toml`).

## Quick start

```bash
aura init myapp
aura run myapp/src/main.aura
```

From a source checkout without installing:

```bash
python3 main.py run examples/hello.aura
```

The smallest Aura program:

```aura
def main() {
  print("Hello, Aura!")
}
```

## The language

Aura has exactly **one spelling per construct** — no synonyms. The full grammar
lives in [docs/GRAMMAR.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/GRAMMAR.md).

```aura
// Bindings. `let` is immutable; `mut` opts in.
let name = "Aura"
let mut count = 0

// Functions and generics.
def max[T](a: T, b: T) -> T {
  return a > b ? a : b
}

// Pattern matching with guards.
match command {
  case "quit" { return }
  case n if n > 100 { print("big") }
  case _ { print("other") }
}
```

Key rules:

- `let`/`const` are immutable; `let mut` (or `mut`) is required to reassign.
- Types are optional and checked before execution by `aura check`.
- `none` is the null literal, `not`/`and`/`or` are the logical operators.
- Only `def` declares functions, only `new` declares constructors, and only
  `extends` declares inheritance.

## Classes

Fields can be declared in the class header, which builds the constructor and
generates accessors:

```aura
class User(private name: str, mut age: int = 0, public id: int = 0) {
  public def greet() -> str {
    return "hi " + self.get_name()
  }
}

let u = User("ana", 30)
print(u.get_name())    // ana
u.set_age(31)          // setter exists because `age` is `mut`
print(u.id)            // 0      — public field, direct access
```

Inheritance uses `extends`, and a subclass header declares only its own fields:

```aura
class Admin extends User(email: str) { }

let a = Admin(email: "a@x.com", name: "bob")
print(a.get_email())
```

## Concurrency

```aura
import stdlib.threading as threading

def main() {
  let results = threading.map_concurrent(
    (n) => n * n,
    [1, 2, 3, 4],
  )
  print(results)
}
```

Async is native too, including file and HTTP helpers that do not block the loop:

```aura
import stdlib.io as io
import stdlib.http as http

async def main() {
  await io.write_async("out.txt", "hello\n")
  let text = await io.read_async("out.txt")
  let response = await http.aget("https://example.com")
  print(text.trim())
  print(response.status)
}
```

## Cryptography

`stdlib.crypto` provides hashing, HMAC, HKDF, and post-quantum primitives
(ML-KEM / ML-DSA). Install the `[pqc]` extra for the production backend.

```aura
import stdlib.crypto as crypto

def main() {
  let key = crypto.random_bytes(32)
  let tag = crypto.hmac_sha3_256(key, "authenticated")

  let kp = crypto.kem_keypair()
  let envelope = crypto.kem_encapsulate(kp.public_key)
  let shared = crypto.kem_decapsulate(kp.secret_key, envelope.ciphertext)
  print(shared == envelope.shared_secret)
}
```

> The pure-Python reference backend is **not** cryptographically secure and
> reports `production = false`. Use the `[pqc]` extra for real secrets.

See [examples/crypto.aura](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/crypto.aura).

## Python interop

Any PyPI package is one import away, and the `python` bridge reaches anything
else.

```aura
import os
import math
import json as json

import python

def main() {
  print(math.sqrt(144.0))
  let payload = json.dumps({"name": "aura", "ok": true})
  print(payload)
  print(python.is_instance(payload, str))
}
```

## Aura Patterns (AUP)

[AUP](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/AUP.md) is a
catalog of idiomatic solutions, each with a runnable example in
[`examples/aup/`](https://github.com/JoaoValentimTheo/aura-lang/tree/master/examples/aup):

| Pattern | Example |
|---------|---------|
| Optional results (`T \| none`) | [`option.aura`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/aup/option.aura) |
| Typed error handling | [`error_handling.aura`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/aup/error_handling.aura) |
| Builder | [`builder.aura`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/aup/builder.aura) |
| Strategy | [`strategy.aura`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/aup/strategy.aura) |
| Pipeline | [`pipeline.aura`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/aup/pipeline.aura) |
| Memoize / cache | [`memoize.aura`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/aup/memoize.aura) |
| Observer | [`observer.aura`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/aup/observer.aura) |
| Resource management | [`resource.aura`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/aup/resource.aura) |
| Worker pool | [`worker_pool.aura`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/aup/worker_pool.aura) |
| Hybrid post-quantum crypto | [`hybrid_crypto.aura`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/aup/hybrid_crypto.aura) |

## CLI

| Command | Purpose |
|---------|---------|
| `aura run <file>` | Transpile and execute an Aura file |
| `aura check <file>` | Type-check and rule-check without running |
| `aura transpile <file>` | Print the generated Python |
| `aura format <file>` | Reformat source; `-i` writes in place, `-o <file>` writes to a file |
| `aura lint <file>` | Style warnings (`--allow-warnings` to exit 0) |
| `aura test [dir]` | Run `.aura` test files |
| `aura repl` | Interactive REPL |
| `aura init [name]` | Scaffold a project (`--venv` to set up `.venv`) |
| `aura venv [action]` | Manage `.venv`: `init`, `info`, `shell`, `remove` |
| `aura add <pkg>` | Add a dependency (`-D` for dev, `--no-install`) |
| `aura remove <pkg>` | Remove a dependency (`--uninstall` too) |
| `aura install` | Install everything declared in `aura.toml` |
| `aura deps` | List dependencies (`--lock` writes `aura.lock`) |
| `aura doctor` | Check Python, venv, and installed dependencies |
| `aura debug <file>` | Trace execution / inspect a crash |
| `aura lsp` | Language server (stdio) |
| `aura version` | Print or bump the version |

Run `aura --help` or `aura <command> --help` for details.

## Projects and dependencies

Aura projects are self-contained: `aura init` writes an `aura.toml`, `aura venv`
creates the environment, and `aura add` records and installs dependencies.

```bash
aura init myapp --venv          # aura.toml + src/main.aura + .venv
cd myapp
aura add "requests>=2.28"       # runtime dependency
aura add -D pytest              # development dependency
aura deps --lock                # write aura.lock with exact versions
aura doctor                     # verify the environment
```

`aura.toml`:

```toml
[project]
name = "myapp"
version = "0.1.0"

[dependencies]
requests = ">=2.28"

[dependencies.dev]
pytest = ">=8"
```

Dependencies install into the project's `.venv` when it exists, and into the
current interpreter otherwise. `aura venv shell` prints the activation command.

## REPL

`aura repl` shares the real parser and every checker (types, structural rules,
and mutability), so what you type is validated the way `aura check` validates
it. State persists across lines:

```text
$ aura repl
Aura REPL v0.4 (type ':help' for help, ':q' to quit)
aura> let mut x = 1
aura> x = x + 1
aura> x
2
aura> let y: int = "text"
[E101] Variable 'y': expected Int, got String
aura> :type x
int
```

Commands: `:help`, `:vars`, `:type <expr>`, `:ast <expr>`, `:load <file>`,
`:run <file>`, `:py <code>`, `:history`, `:reset`, `:q`.

## Documentation

Everything lives under [`docs/`](https://github.com/JoaoValentimTheo/aura-lang/tree/master/docs).
Start with the index or jump straight in:

| Document | What it covers |
|----------|----------------|
| [docs/README.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/README.md) | Documentation index and recommended reading order |
| [GRAMMAR.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/GRAMMAR.md) | Canonical EBNF grammar (the source of truth for syntax) |
| [LANGUAGE.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/LANGUAGE.md) | Complete language reference (English) |
| [LANGUAGE_PT.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/LANGUAGE_PT.md) | Referência completa da linguagem (Português) |
| [TYPES.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/TYPES.md) | Type system (English) |
| [TYPES_PT.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/TYPES_PT.md) | Sistema de tipos (Português) |
| [ERRORS.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/ERRORS.md) | Every diagnostic code (`E##`/`W##`) |
| [AUP.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/AUP.md) | Aura Patterns catalog |
| [DESIGN.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/DESIGN.md) | Compiler architecture |
| [COMPLETENESS.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/docs/COMPLETENESS.md) | Language coverage and remaining gaps |
| [CHANGELOG.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/CHANGELOG.md) | Release history |
| [examples/](https://github.com/JoaoValentimTheo/aura-lang/tree/master/examples) | Runnable example programs |

## Development

```bash
git clone https://github.com/JoaoValentimTheo/aura-lang.git
cd aura-lang
pip install -e ".[dev]"

pytest                    # full test suite (coverage floor: 90%)
ruff check aura/          # lint
mypy aura/                # type-check the compiler
```

See [CONTRIBUTING.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/CONTRIBUTING.md)
for the contribution workflow and [SECURITY.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/SECURITY.md)
to report a vulnerability.

## License

MIT — see [LICENSE](https://github.com/JoaoValentimTheo/aura-lang/blob/master/LICENSE).