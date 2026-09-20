---
title: "00 — Aura Language Overview"
---

# Aura Language Overview

Aura is a gradually-typed programming language that transpiles to Python. It enforces **one spelling per construct** — no synonyms — and targets CPython 3.10+. Every PyPI package is one import away.

## Design Philosophy

| Principle | Meaning |
|-----------|---------|
| One spelling | `def` only (not `fn`/`fun`/`function`); `if` only (not `elif`/`elsif`); `and`/`or`/`not` only (not `&&`/`||`/`!`) |
| Python ecosystem | Full access to any Python package via `py.` prefix or direct import |
| Gradual typing | Type annotations are optional and checked before execution |
| Explicit immutability | `let` is immutable; `let mut` opts into reassignment |

## Relationship to Python

Aura transpiles to Python 3.10+ source, which CPython then executes. This means:

- All Python semantics apply: `/` is true division (never truncates), `%` follows the divisor's sign, `**` is right-associative.
- Python's object model is fully supported: dunder methods, descriptors, context managers, generators.
- There is no floor division operator (`//`); write `int(total / count)` instead.

See `aura/transpiler/transformer.py` for the transpilation pipeline.

## Minimal Program

```aura
def main() {
  print("Hello, Aura!")
}
```

The entry file must declare a top-level `main`. A file imported as a module needs no `main`.

## Key Language Features

### Bindings

```aura
let name = "Aura"         // immutable
let mut count = 0          // mutable
const PI = 3.14159        // compile-time constant
```

### Functions

```aura
def max(a: int, b: int) -> int {
  return a > b ? a : b
}

def square(x: int) -> int = x * x   // expression body
```

### Pattern Matching

```aura
match command {
  case "quit" { return }
  case n if n > 100 { print("big") }
  case _ { print("other") }
}
```

### Classes and OOP

```aura
class User(private name: str, mut age: int = 0) {
  public def greet() -> str {
    return "hi " + self.get_name()
  }
}

let u = User("ana", 30)
print(u.get_name())   // getter auto-generated
```

### Modules

```aura
module MyLib {
  export def greet(name: str) -> str {
    return "hello " + name
  }
}
```

### Error Handling

```aura
try {
  let result = risky_operation()
} catch ValueError as e {
  print(f"caught: {e}")
} finally {
  cleanup()
}
```

## Keywords (Exhaustive)

The following 53 words are reserved and cannot be used as binding names:

```
abstract  and       as        assert    async     await     break
case      catch     class     const     continue  def       else
enum      export    false     finally   fn        for       from
guard     if        import    in        is        let       loop
match     module    new       none      not       or        private
protected public    return    self      spawn     static    super
throw     trait     true      try       type      unless    until
volatile  while     with      yield
```

Foreign spellings (`var`, `val`, `fun`, `function`, `foreach`, `switch`, `repeat`, `lambda`) are rejected with pointed errors directing you to the Aura equivalent.

## What Aura Does NOT Have

| Absent construct | Aura alternative |
|-----------------|-----------------|
| `elif` / `elsif` | `else if` |
| `&&` / `||` / `!` | `and` / `or` / `not` |
| `//` floor division | `int(total / count)` |
| `var` / `val` | `let` / `let mut` |
| `new ClassName()` | `ClassName()` |
| `override def` | `def` (implicit override) |
| `implements` | `extends` |
| `lambda` | `(x) => expr` |
| `switch` | `match` |
| `null` / `None` / `True` / `False` | `none` / `true` / `false` |
| `fn` / `fun` / `func` | `def` |

## CLI Quick Reference

| Command | Purpose |
|---------|---------|
| `aura run <file>` | Transpile and execute |
| `aura check <file>` | Type-check and rule-check |
| `aura format <file>` | Reformat source |
| `aura test [dir]` | Run test files |
| `aura repl` | Interactive REPL |
| `aura init [name]` | Scaffold a project |

## Gotcha

- **`let private x`** is wrong; write `private let x`. Visibility prefixes the declaration.
- **`True`, `False`, `None`** are rejected as literals; write `true`, `false`, `none`.
- **`fn`** is reserved and rejected; use `def`.

## Anti-pattern

Do not mix header fields and a manual `def new` in the same class. Pick one constructor style per class — mixing is a syntax error.
