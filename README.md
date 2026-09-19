<div align="center">

<img src="https://raw.githubusercontent.com/JoaoValentimTheo/aura-lang/master/aura_logo.png" alt="Aura" width="220">

# Aura

**Aura** is a gradually-typed programming language that transpiles to Python.
One spelling per construct, no synonyms. The whole Python ecosystem — any PyPI
package — is one import away.

[![CI](https://github.com/JoaoValentimTheo/aura-lang/actions/workflows/ci.yml/badge.svg)](https://github.com/JoaoValentimTheo/aura-lang/actions/workflows/ci.yml)
[![Release](https://github.com/JoaoValentimTheo/aura-lang/actions/workflows/release.yml/badge.svg)](https://github.com/JoaoValentimTheo/aura-lang/releases)
[![PyPI](https://img.shields.io/pypi/v/aura-language.svg)](https://pypi.org/project/aura-language/)
[![Python](https://img.shields.io/pypi/pyversions/aura-language.svg)](https://pypi.org/project/aura-language/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

</div>

---

## Status

Aura is **alpha** (`0.2.0a3`). The compiler, type checker, rule checker, REPL,
language server, formatter, linter, project tooling, and standard library are
implemented and covered by a test suite of **11,000+ tests**.

> **Syntax freeze.** As of `0.2.0a1`, Aura's syntax is officially frozen.
> No syntax changes will be made before the stable 1.0 release. The
> canonical grammar is [docs/GRAMMAR.md](docs/GRAMMAR.md) and every change is
> recorded in [CHANGELOG.md](CHANGELOG.md).

What exists today:

| Area | State |
|------|-------|
| Language, parser, type/rule/mutability checkers | Implemented |
| Transpiler to Python (Python 3.10+) | Implemented |
| CLI (17 subcommands) | Implemented |
| Project tooling (init, venv, add, remove, install, deps, doctor) | Implemented |
| Language server (diagnostics, completion, hover, go-to-definition) | Implemented |
| Standard library (18 modules) | Implemented |
| Compile-time macros | Implemented |
| Security hardening (SSRF guards, input limits, no traceback leaks) | Implemented |
| Compilation to native machine code | Not planned for 1.0 |

## Requirements

- Python **3.10+** (the generated code targets 3.10+).
- No mandatory third-party runtime dependencies. On Python 3.10, `tomli` is
  installed automatically to read `aura.toml`.
- Optional: `cryptography>=44` (the `[pqc]` extra) enables the production
  post-quantum backend for `stdlib.crypto`.

## Installation

From PyPI:

```bash
pip install aura-language
```

From a checkout:

```bash
git clone https://github.com/JoaoValentimTheo/aura-lang.git
cd aura-lang
pip install -e .
aura run examples/hello.aura
```

Optional post-quantum backend:

```bash
pip install "aura-language[pqc]"
```

## Quick start

```bash
aura init myapp          # scaffolds aura.toml and src/main.aura
cd myapp
aura run src/main.aura
```

The smallest Aura program:

```aura
def main() {
  print("Hello, Aura!")
}
```

From a source checkout without installing, the `main.py` shim works too:

```bash
python3 main.py run examples/hello.aura
```

## The language

Aura has exactly **one spelling per construct** — no synonyms. The full grammar
is in [docs/GRAMMAR.md](docs/GRAMMAR.md).

```aura
// Bindings: `let` is immutable, `let mut` opts into reassignment.
let name = "Aura"
let mut count = 0

// Functions; types are optional and checked before execution.
def max(a: int, b: int) -> int {
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

- `let`/`const` are immutable; `let mut` is required to reassign.
- `none` is the null literal; `not`, `and`, `or` are the logical operators.
- Only `def` declares functions; only `new` declares constructors; only
  `extends` declares inheritance (`class X(Y)` and `implements` are rejected).
- Every class/trait member declares its visibility (`public`/`private`/
  `protected`).
- `*args` and `**kwargs` work in every context the grammar permits: function
  declarations, calls, decorators, and lambdas.

## Classes

Fields may be declared in the class header, which builds the constructor and
generates accessors:

```aura
class User(private name: str, mut age: int = 0, public id: int = 0) {
  public def greet() -> str {
    return "hi " + self.get_name()
  }
}

let u = User("ana", 30)
print(u.get_name())    // ana  -- getter is always generated
u.set_age(31)          // setter exists because `age` is `mut`
print(u.id)            // 0    -- public field, direct access
```

A class has **one** constructor style: header fields *or* body fields with a
manual `def new` — never both. Mixing them is a syntax error.

Inheritance uses `extends`; a subclass header declares only its own fields:

```aura
class Admin extends User(email: str) { }

let a = Admin(email: "a@x.com", name: "bob")
print(a.get_email())
```

Overriding is implicit: declare a method with the same name in the subclass.
There is no `override` keyword.

### Traits and abstract classes

A `trait` is a pure contract: methods without a body are abstract, and a
concrete class must implement every one it inherits.

```aura
trait Drawable {
  public def draw() -> void
}

class Circle extends Drawable {
  public let radius: float = 1.0
  public def draw() -> void { print("circle") }
}
```

An `abstract class` is a real base — fields, concrete methods and a constructor
— that cannot be instantiated and may defer methods with `abstract def`:

```aura
abstract class Shape {
  public abstract def area() -> float
}

class Square extends Shape {
  public let side: float = 2.0
  public def area() -> float { return self.side * self.side }
}

// let s = Shape()   // E316: abstract, cannot be instantiated
```

Aura honours Python's object protocols: dunder methods, descriptors, context
managers (`with` / `async with`), generators, and the MRO are all supported when
extending Python classes.

## Concurrency

```aura
import stdlib.threading as threading

def main() {
  let results = threading.map_concurrent((n) => n * n, [1, 2, 3, 4])
  print(results)
}
```

Async is native, including file and HTTP helpers that do not block the loop:

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
(ML-KEM / ML-DSA).

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

> The bundled pure-Python reference backend reports `production = false` and is
> **not** cryptographically secure. Install the `[pqc]` extra for real secrets.

## Python interop

Any PyPI package is one import away; the `python` bridge reaches anything else.

```aura
import os
import math
import json
import python

def main() {
  print(math.sqrt(144.0))
  let payload = json.dumps({"name": "aura", "ok": true})
  print(payload)
  print(python.is_instance(payload, str))
}
```

Aura apps can drive real frameworks directly — for example a Flask app with
dotted `@app.route(...)` decorators:

```aura
import flask

let app = flask.Flask(__name__)

@app.route("/hello/<name>")
def hello(name) {
  return f"Hello, {name}!"
}
```

## Compile-time macros

Beyond runtime decorators such as `@debug`, `@memoize`, and `@cache`, Aura has
compile-time macros expanded by the transpiler *before* any Python is emitted.
A macro receives its operands as quoted AST, returns replacement AST, and
leaves no trace at runtime unless it chooses to emit one.

```aura
import macros

def main() {
  assert_eq(2 + 2, 4)       // evaluate both once, assert equality
  static_assert(true)       // checked at compile time
  print(stringify(42))      // folds the literal to "42" during compilation
  let mut a = 1
  let mut b = 2
  swap(a, b)                // a binding plus two assignments
}
```

Built-ins: `assert_eq`, `assert_ne`, `static_assert`, `identity`, `discard`,
`stringify`, `swap`, `debug_value`, `todo`, `unreachable`. Macro expansion is
hygienic — introduced bindings can never capture a call-site name — and a
program's own declaration always shadows a built-in macro of the same name.

## Aura Patterns (AUP)

[AUP](docs/AUP.md) is a catalog of idiomatic solutions, each with a runnable
example under [`examples/aup/`](examples/aup):

| Pattern | Example |
|---------|---------|
| Optional results (`T \| none`) | [`option.aura`](examples/aup/option.aura) |
| Typed error handling | [`error_handling.aura`](examples/aup/error_handling.aura) |
| Builder | [`builder.aura`](examples/aup/builder.aura) |
| Strategy | [`strategy.aura`](examples/aup/strategy.aura) |
| Pipeline (`\|>`) | [`pipeline.aura`](examples/aup/pipeline.aura) |
| Memoize / cache | [`memoize.aura`](examples/aup/memoize.aura) |
| Observer | [`observer.aura`](examples/aup/observer.aura) |
| Resource management (`with`) | [`resource.aura`](examples/aup/resource.aura) |
| Worker pool | [`worker_pool.aura`](examples/aup/worker_pool.aura) |
| Hybrid post-quantum crypto | [`hybrid_crypto.aura`](examples/aup/hybrid_crypto.aura) |

## CLI

| Command | Purpose |
|---------|---------|
| `aura run <file>` | Transpile and execute an Aura file (`-v` prints the generated Python) |
| `aura check <file>` | Type-check and rule-check without running |
| `aura transpile <file>` | Print the generated Python (`-o <file>` writes it) |
| `aura format <file>` | Reformat source (`-i` in place, `-o <file>` to a file) |
| `aura lint <file>` | Style warnings (`--allow-warnings` to exit 0) |
| `aura test [dir]` | Run `.aura` test files (`-v` verbose) |
| `aura repl` | Interactive REPL |
| `aura init [name]` | Scaffold a project (`--venv` to also create an environment) |
| `aura venv [action]` | Manage `.venv`: `init`, `info`, `shell`, `remove` |
| `aura add <pkg>` | Add a dependency (`-D` dev, `--no-install`, `-V <spec>`) |
| `aura remove <pkg>` | Remove a declared dependency |
| `aura install` | Install everything declared in `aura.toml` |
| `aura deps` | List dependencies (`--lock` writes `aura.lock`) |
| `aura doctor` | Check Python, venv, and installed dependencies |
| `aura debug <file>` | Run under the trace debugger |
| `aura lsp` | Language server over stdio |
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

Dependencies install into the project's `.venv` when one exists, and into the
current interpreter otherwise. `aura venv shell` prints the activation command.

## REPL

`aura repl` shares the real parser and every checker (types, structural rules,
and mutability), so each line is validated the way `aura check` validates it.
State persists across lines:

```text
$ aura repl
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

## Standard library

| Module | Purpose |
|--------|---------|
| `stdlib.threading` | Thread pool, `map_concurrent` |
| `stdlib.asyncio` | Async event-loop helpers |
| `stdlib.io` | File I/O (sync + async) |
| `stdlib.http` | HTTP client (sync + async) |
| `stdlib.crypto` | Hashing, HMAC, HKDF, post-quantum (ML-KEM/ML-DSA) |
| `stdlib.json` | JSON encode/decode |
| `stdlib.regex` | Regular expressions |
| `stdlib.string` | String manipulation |
| `stdlib.math` | Mathematical functions |
| `stdlib.collections` | Collection utilities |
| `stdlib.itertools` | Iterator combinators |
| `stdlib.os` | OS-level utilities |
| `stdlib.python` | Python bridge (`python.is_instance`, ...) |
| `stdlib.time` | Time functions |
| `stdlib.testing` | Test framework support |
| `stdlib.macros` | Compile-time macro surface |

## Documentation

| Document | Covers |
|----------|--------|
| [docs/README.md](docs/README.md) | Documentation index and reading order |
| [docs/GRAMMAR.md](docs/GRAMMAR.md) | Canonical EBNF grammar (source of truth) |
| [docs/LANGUAGE.md](docs/LANGUAGE.md) | Complete language reference (English) |
| [docs/LANGUAGE_PT.md](docs/LANGUAGE_PT.md) | Referencia completa da linguagem (Portugues) |
| [docs/TYPES.md](docs/TYPES.md) | Type system (English) |
| [docs/TYPES_PT.md](docs/TYPES_PT.md) | Sistema de tipos (Portugues) |
| [docs/ERRORS.md](docs/ERRORS.md) | Every diagnostic code (`E##` / `W##`) |
| [docs/AUP.md](docs/AUP.md) | Aura Patterns catalog |
| [docs/DESIGN.md](docs/DESIGN.md) | Transpiler architecture |
| [docs/COMPLETENESS.md](docs/COMPLETENESS.md) | Language coverage and remaining gaps |
| [CHANGELOG.md](CHANGELOG.md) | Release history |
| [examples/](examples/) | Runnable example programs |

## Development

```bash
git clone https://github.com/JoaoValentimTheo/aura-lang.git
cd aura-lang
pip install -e ".[dev]"

pytest                    # full test suite (3,000+ tests)
ruff check aura/          # lint
mypy aura/                # type-check the compiler
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow and
[SECURITY.md](SECURITY.md) to report a vulnerability.

## License

MIT — see [LICENSE](LICENSE).
