# Aura Examples

Runnable Aura programs demonstrating the language. Every file here passes
`aura check` and runs with `aura run`.

```bash
aura run examples/tour.aura
```

## Learning path

Start here and follow the order; each file introduces one idea.

| # | File | What it shows |
|---|------|---------------|
| 1 | `hello.aura` | The smallest program: `main` and `print` |
| 2 | `fibonacci.aura` | Loops and mutable bindings (`let mut`) |
| 3 | `prime_checker.aura` | Functions, conditionals, and comprehensions |
| 4 | `functional.aura` | Lambdas, the pipe operator `\|>`, comprehensions, closures |
| 5 | `pattern_matching.aura` | `match` with guards, literals, and list destructuring |
| 6 | `error_handling.aura` | `try`/`catch`/`finally`, typed catches, `guard`, `throw`, `assert` |
| 7 | `classes.aura` | Classes, constructors, `extends`, `@property`, visibility |
| 8 | `macros.aura` | Built-in decorators (`@debug`, `@timeit`, `@memoize`, `@cache`) |
| 9 | `compile_time_macros.aura` | Compile-time macros (`assert_eq`, `static_assert`, `swap`, `stringify`, ...) |
| 10 | `crypto.aura` | Hashing, HMAC, HKDF, and post-quantum KEM/signatures via `stdlib.crypto` |
| 11 | `python_interop.aura` | Calling Python from Aura (`import os`, `math`, `json`, the `python` bridge) |
| 12 | `tour.aura` | A single-file tour of the whole language |
| 13 | `abstract_classes.aura` | `abstract class` + `abstract def`, and polymorphism over the abstract base |

## Framework integration

Real Python frameworks driven from Aura. Each installs its dependency and
verifies the app end to end without blocking on a server or a window.

| File | What it shows |
|------|---------------|
| `flask_app.aura` | A Flask app with dotted `@app.route(...)` decorators, exercised through Flask's test client |
| `django_views.aura` | Django views and `urlpatterns` built with `django.http` / `django.urls` |
| `flet_app.aura` | A Flet view builder over real controls, run headless against a page stand-in |

## Patterns (AUP)

`aup/` contains the Aura Patterns — idiomatic solutions to recurring problems,
each explained in [docs/AUP.md](../docs/AUP.md).

| File | Pattern |
|------|---------|
| `aup/option.aura` | Representing an optional result with `T \| none` |
| `aup/error_handling.aura` | Typed catches with try-as-an-expression |
| `aup/builder.aura` | Builder with a chainable API |
| `aup/strategy.aura` | Strategy selected from a trait |
| `aup/pipeline.aura` | Data pipeline with `\|>` |
| `aup/memoize.aura` | `@memoize` (unbounded) vs `@cache(64)` (bounded LRU) |
| `aup/observer.aura` | Observer using a dict of callback lists |
| `aup/resource.aura` | Resource management with `with` |
| `aup/worker_pool.aura` | Concurrency with `threading.map_concurrent` |
| `aup/hybrid_crypto.aura` | Hybrid post-quantum handshake |

## Style notes

Aura follows a single, explicit syntax (see [docs/GRAMMAR.md](../docs/GRAMMAR.md)):

- Functions use `def`; `fn` does not exist.
- Constructors use `new`; `init` does not exist.
- Inheritance uses `extends`: `class Dog extends Animal { ... }`. The
  parenthesised form (`class Dog(Animal)`) and `implements` are not Aura.
- Class fields may be declared in the header:
  `class User(private name: str, mut age: int = 0) { ... }`.
- A `let` field is immutable; use `let mut` when a field must change.
- Generics use square brackets: `class Box[T] { ... }`, `List[Int]`; `<T>` does
  not exist.
- Logical negation is `not`; `!` does not exist.
- `and`/`or` are the boolean operators; `&&`/`||` do not exist.
- The null literal is `none`; `null` does not exist.
- Visibility modifiers come before the declaration: `private let x`, not
  `let private x`. Every class/trait member needs one.
- `unless`, `until`, `loop` and `guard` are first-class statements.

## Verifying the examples

The whole `examples/` tree is checked by the test suite
(`tests/test_examples.py`), which parses, type-checks, transpiles, and runs each
file so a stale example cannot slip through.