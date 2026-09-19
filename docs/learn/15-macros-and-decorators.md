---
layout: default
title: "15 — Macros and Decorators"
parent: Learn Aura
nav_order: 25
---

[English](15-macros-and-decorators.md) · [Português](15-macros-and-decorators.pt_BR.md)

# 15 — Macros and Decorators

> **Chapter goal:** attach reusable behaviour to functions with decorators, and
> use Aura's compile-time macros. Examples: `../../examples/macros.aura`,
> `../../examples/compile_time_macros.aura`.

## Decorators

A decorator is `@name` or `@name(args)` above a `def` (or a class). Decorators
on a **field** are rejected (`E320`). Member modifiers and decorators may appear
in any order.

```aura
@staticmethod
public def max(a, b) -> int { return a }

class Rect {
  public @property
  def area() -> int { return 4 }

  public @classmethod
  def create(cls) { return cls() }
}
```

The class-member decorators are `@property`, `@staticmethod` and
`@classmethod` (chapter 07). A decorator on a plain function can be any
callable, including one you write:

```aura
def trace(f) {
  def wrapper(x) {
    print(f"calling with {x}")
    return f(x)
  }
  return wrapper
}

@trace
def square(x) -> int { return x * x }

def main() {
  print(square(4))
}
```

```text
calling with 4
16
```

## Built-in runtime macros

Aura ships a small prelude of decorators. The prelude is injected automatically
only when a macro is used.

```aura
@debug
def multiply(a, b) -> int {
  return a * b
}

@timeit
def sum_to(n) -> int {
  let mut total = 0
  for i in range(n) { total += i }
  return total
}

@memoize
def fib(n) -> int {
  if n < 2 { return n }
  return fib(n - 1) + fib(n - 2)
}

@cache(maxsize=256)
def expensive(n) -> int { return n * n }

def main() {
  print(multiply(6, 7))     // DEBUG: enter/exit lines, then 42
  print(sum_to(100))        // a timing line, then 4950
  print(fib(20))            // 6765
  print(expensive(12))      // 144
}
```

| Decorator | Effect |
|---|---|
| `@debug` | prints the call's arguments and result |
| `@timeit` | prints how long the call took |
| `@memoize` | unbounded result cache |
| `@cache(maxsize=n)` | bounded LRU cache |

These are **runtime** decorators: they wrap the emitted Python function.

## Compile-time macros

A second tier expands **before** any Python is emitted. The macro receives its
operands as quoted AST and returns replacement AST, so nothing survives to
runtime unless the expansion chooses to emit it. Import them from `macros`:

```aura
import macros

def main() {
  assert_eq(2 + 2, 4)        // evaluate both once, then assert
  assert_ne("a", "b")
  static_assert(true)        // checked while compiling

  let mut a = 1
  let mut b = 2
  swap(a, b)                 // hygienic temporary; a,b change
  print(a, b)                // 2 1

  print(identity(41) + 1)    // identity disappears; prints 42
  print(stringify(42))       // folds the literal at compile time
  print(debug_value(a))      // prints "a = 2", yields the value
}
```

| Macro | Expansion |
|---|---|
| `assert_eq(a, b)` | evaluate both once, assert equality |
| `assert_ne(a, b)` | evaluate both once, assert inequality |
| `static_assert(x)` | checked at compile time; a non-literal is an error |
| `identity(x)` | expands to `x` |
| `stringify(x)` | folds a literal to a string at compile time |
| `swap(a, b)` | exchange two lvalues via a hygienic temporary |
| `debug_value(x)` | print `x = <value>` once, then yield the value |

A macro that introduces bindings uses **hygienic** names, so it can never capture
a local at the call site.

## Runtime vs compile-time

| | Runtime decorators | Compile-time macros |
|---|---|---|
| When they run | when the function is called | while transpiling |
| Form | `@decorator` on a `def` | call-like expressions in the body |
| Examples | `@debug`, `@timeit`, `@memoize`, `@cache` | `assert_eq`, `swap`, `stringify` |
| Use when | you want a wrapper around a call | you want the code rewritten before emit |

## Decorator order

When several decorators are stacked they apply bottom-up: the one closest to the
`def` wraps first, the outermost runs first. Order changes behaviour. Using a
local `trace` wrapper:

```aura
def trace(f) {
  def wrapper(x) {
    print(f"calling with {x}")
    return f(x)
  }
  return wrapper
}

@memoize
@trace
def fib(n) -> int {
  if n < 2 { return n }
  return fib(n - 1) + fib(n - 2)
}

def main() {
  print(fib(5))
}
```

```text
calling with 5
calling with 4
calling with 3
calling with 2
calling with 1
calling with 0
5
```

With `@memoize` outermost, the cache is checked before the trace, so each value
of `n` is traced exactly once. Swap the two lines and the trace runs on every
call, because recursion reaches the traced function before the cache.

## What you learned

* Decorators attach to `def`/class (never a field); `@property`,
  `@staticmethod`, `@classmethod`, and custom decorators.
* Built-in runtime decorators `@debug`, `@timeit`, `@memoize`, `@cache`.
* Compile-time macros from `macros`, including `assert_eq`, `swap`,
  `stringify`, `static_assert`.

## Next step

[CLI and Tooling →](16-cli-and-tooling.md)
