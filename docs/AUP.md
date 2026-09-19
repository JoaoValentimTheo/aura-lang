---
layout: default
title: "AUP — Aura Patterns"
nav_order: 5
---

# AUP — Aura Patterns

**AUP** (Aura Uniform Patterns, or simply *Aura Patterns*) is a catalog of
idiomatic solutions to recurring problems, written in canonical Aura. Each
pattern is a runnable program under [`examples/aup/`](https://github.com/JoaoValentimTheo/aura-lang/tree/master/examples/aup) and is
exercised by `tests/test_aup.py`.

Patterns are **conventions**, not a library: they use only the language and the
standard library. Every example passes `aura check` and runs.

| # | Pattern | File | Uses |
|---|---------|------|------|
| 01 | Option (result-or-null) | `examples/aup/option.aura` | `none`, `guard`, `??`, `try`-as-expression |
| 02 | Builder | `examples/aup/builder.aura` | `let mut`, methods returning `self` |
| 03 | Strategy | `examples/aup/strategy.aura` | traits → ABCs, polymorphism |
| 04 | Pipeline | `examples/aup/pipeline.aura` | `\|>`, `map`/`filter`/`reduce`/`take` |
| 05 | Typed error handling | `examples/aup/error_handling.aura` | `throw`, `catch Type as e`, `finally`, `try` expression |
| 06 | Memoization / caching | `examples/aup/memoize.aura` | `@memoize`, `@cache(maxsize)` |
| 07 | Observer | `examples/aup/observer.aura` | lambdas/closures, dict of callback lists |
| 08 | Resource management | `examples/aup/resource.aura` | `with`, `enter`/`exit` |
| 09 | Worker pool | `examples/aup/worker_pool.aura` | `stdlib.threading`, locks, `map_concurrent` |
| 10 | Hybrid secure message | `examples/aup/hybrid_crypto.aura` | `stdlib.crypto` (ML-KEM, HKDF, HMAC-SHA3) |

---

## 01 — Option

Aura has no built-in `Option`; `T | none` plus early-exit `guard` and the
coalescing operator `??` cover the same ground without a wrapper type.

```aura
def parse_positive(text) -> int | none {
  guard text.length() > 0 else { return none }
  let value = try { int(text) } catch Error as e { -1 }
  guard value > 0 else { return none }
  return value
}

let n = parse_positive("42") ?? -1     // fall back on absence
```

Callers either `guard ... != none else { return }` or coalesce. Prefer this to
returning sentinel values like `-1`, which are indistinguishable from real data.

## 02 — Builder

Return `self` from each configuration method to chain calls. Chainability
requires the object to be mutable, so the fields are set with `let mut` where
they change.

```aura
let sql = Query("users").select("id", "name").limit(10).build()
```

## 03 — Strategy

A `trait` is an interface that compiles to an abstract base class. Concrete
classes `extends` it, and call sites accept the trait as a parameter type.

```aura
trait Discount { public def apply(price: float) -> float }

class PercentOff extends Discount {
  private let percent: float = 0.0
  public def new(percent: float) { self.percent = percent }
  public def apply(price: float) -> float { return price * (1.0 - self.percent / 100.0) }
}
```

The compiler enforces the contract: a class that omits an abstract method
cannot be instantiated.

## 04 — Pipeline

`|>` passes the left value as the first argument of the right function. The
standard library's `map`, `filter`, `reduce`, `take` and `drop` are data-first,
so pipelines read top-to-bottom.

```aura
let result = numbers
  |> filter((x) => x % 2 == 0)
  |> map((x) => x * x)
  |> reduce((acc, x) => acc + x, 0)
```

## 05 — Typed error handling

`throw` raises an exception object; `catch Type as e` handles a specific class.
`try` also works as an expression producing a value.

```aura
try {
  risky()
} catch ValueError as e {
  print(f"bad value: {e}")
} finally {
  print("cleanup")
}

let label = try { f"ok: {parse('42')}" } catch Error as e { "failed" }
```

Rules: `try` needs at least one `catch` or a `finally`; don't use exceptions
for ordinary control flow — use Option (#01) for expected absence.

## 06 — Memoization

`@memoize` gives an unbounded cache; `@cache(maxsize=N)` gives an LRU cache.

```aura
@memoize
def fib(n) -> int {
  if n < 2 { return n }
  return fib(n - 1) + fib(n - 2)
}
```

Use `@memoize` for pure functions with a small input domain; use `@cache(N)`
when the domain is large or unbounded.

## 07 — Observer

Model events as a dictionary from event name to a list of callbacks. Lambdas
and named functions are interchangeable as handlers.

```aura
bus.on("order", (payload) => print(f"email: {payload}"))
bus.emit("order", 42)
```

Keep handlers small and side-effect-only; return values are ignored.

## 08 — Resource management

`with` guarantees cleanup. A class provides `enter()` (returns the bound value)
and `exit(exc_type, exc, tb)`.

```aura
with Timer("load") as t {
  print("working...")
}
```

Nested `with` blocks run their `exit` in reverse order (LIFO). Use this for
files, connections, locks and timers.

## 09 — Worker pool

For I/O-bound work, bound concurrency with a thread pool. `map_concurrent`
preserves input order; for manual fan-out, guard shared state with a lock.

```aura
let results = threading.map_concurrent((x) => fetch(x), ids, workers: 3)
```

Warn: Aura globals are shared across threads. Any assignment to shared state
from a worker must be protected by a lock (`lock.acquire()/release()` or `with`).
For CPU-bound work, prefer processes; the GIL limits threading speedups.

## 10 — Hybrid secure message

Establish a shared secret with a post-quantum KEM (ML-KEM), derive a symmetric
key with HKDF, and authenticate with HMAC-SHA3. This is the shape of a hybrid
handshake.

```aura
let enc = crypto.kem_encapsulate(recipient_public_key)
let key = crypto.hkdf_sha256(enc.shared_secret, 32, info: b"message")
let tag = crypto.hmac_sha3_256(key, message)
```

Install `aura-language[pqc]` for a vetted KEM/signature backend. The bundled
reference backend is for tests and education only — call
`crypto.require_production_backend()` before protecting real secrets. The XOR
encryption in the example is illustrative; use a real AEAD (AES-GCM,
ChaCha20-Poly1305) in production.

---

## Conventions

- **One spelling.** Use canonical syntax (`def`, `new`, `none`, `and`, `not`).
  See [grammar.md](language-reference/grammar.md).
- **Immutability by default.** `let` binds an immutable name; use `let mut`
  only when reassignment is genuinely needed.
- **Early exit with `guard`.** Validate preconditions at the top of a function.
- **Traits for contracts, `raises` for the unexpected.**
- **Prefer the pipeline** for data transformations; prefer loops when the body
  has side effects or early exits.

## Adding a pattern

1. Add `examples/aup/<name>.aura` — a self-contained, runnable program whose
   **first line** is `// AUP-NN: Title.` (two-digit, sequential, no gaps) and
   which declares `def main()`.
2. Add a row to the table and a `## NN — Title` section here explaining *when*
   to use it.
3. `tests/test_aup.py` runs every example and checks the header, numbering,
   table row and section stay in sync; no test edit is needed.