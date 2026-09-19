---
layout: default
title: "06 — Functions"
parent: Learn Aura
nav_order: 16
---

[English](06-functions.md) · [Português](06-functions.pt_BR.md)

# 06 — Functions

> **Chapter goal:** declare, call and compose functions. Reference:
> [`../language-reference/functions.md`](../language-reference/functions.md).
> Example: `fibonacci.aura`, `prime_checker.aura`.

## Declaration

`def` is the only function keyword.

```aura
def add(a: int, b: int) -> int {
  return a + b
}

def square(x: int) -> int = x * x     // expression body
def double(x) = x * 2                 // no return type
def noop() { }                        // no return type, no return
```

The body is either a brace block or a single `= expr`; the expression body is
desugared to `return <expr>`. Parameters may be annotated or bare. The return
type after `->` is optional, and is **erased** at emit — a mismatch is not
enforced by the transpiler.

## Calling

```aura
def add(a, b) -> int {
  return a + b
}

def main() {
  print(add(2, 3))            // positional
  print(add(a: 2, b: 3))      // keyword
}
```

Keyword arguments are written `name: expr` (or `name = expr`) at the call site
and become Python keyword arguments. A positional argument after a keyword is a
parse error.

## Default parameters

```aura
def greet(name, greeting = "Hello") -> str {
  return greeting + ", " + name
}

def main() {
  print(greet("Ana"))           // Hello, Ana
  print(greet("Bob", "Hi"))     // Hi, Bob
}
```

Any trailing defaulted argument may be omitted.

## Variadics and keyword-only

```aura
def sum_all(*numbers) -> int {
  let mut total = 0
  for n in numbers { total += n }
  return total
}

def log(level, **context) {
  print(level, context)
}

def f(a, *, b) { return a + b }     // b must be passed by keyword
```

* `*args` — positional variadic.
* `**kwargs` — keyword mapping.
* A bare `*` makes the following parameters keyword-only.

At the call site, `*xs` spreads a sequence, `**m` spreads a mapping, and
`...v` spreads adaptively (dict → keywords, otherwise positional):

```aura
def add_all(*nums) -> int {
  let mut total = 0
  for n in nums { total += n }
  return total
}

def main() {
  let nums = [1, 2, 3]
  print(add_all(*nums))       // 6
}
```

**Limitation:** `aura check` under-counts spread arguments, so a spread into a
fixed-arity function can report `E105` even though `aura run` succeeds (the
emitted Python honours the spread). Prefer spreading into a variadic function,
or expect the check diagnostic.

`UNSPECIFIED`: duplicate parameter names are not rejected by the grammar; the
target Python rejects them.

## Return

`return` with a value, or bare. Multiple values are a comma list and become a
tuple:

```aura
def swap(a, b) {
  return b, a
}

def main() {
  let x, y = swap(1, 2)       // x = 2, y = 1
  print(x, y)
}
```

A bare `return` in a value position yields `none`. In a `void`-like function,
`return` alone ends the flow early.

## Recursion

```aura
def factorial(n) -> int {
  if n <= 1 { return 1 }
  return n * factorial(n - 1)
}

def main() {
  print(factorial(5))         // 120
}
```

Direct recursion is supported. There is no guaranteed tail-call optimisation;
deep recursion is bounded by the CPython stack.

## Nested functions

A function declared inside another is emitted at the point of declaration:

```aura
def main() {
  def helper(x: int) -> int { return x + 1 }
  print(helper(2))            // 3
}
```

## Generics

Type parameters use **square brackets** and precede the parameter list; a
constraint may follow `:`. They are erased at emit.

```aura
class Comparable {
  public def compare(other: Comparable) -> int { return 0 }
}

def id[T](x: T) -> T { return x }
def smallest[T: Comparable](items: [T]) -> T { return items[0] }

def main() {
  print(id(7))                // 7
  print(smallest([3, 1, 2]))  // 3
}
```

A constraint must name a builtin type or a class/trait declared in the program
(`E110` otherwise). A parameter declared but never used warns `W103`.

## No overloading

There is no language-level overloading: a repeated name in the same scope is
`E301`. Top-level functions with the same name would silently overwrite in
Python, which is why Aura rejects them.

## `main` — the entry point

```aura
def main() {
  print("hello")
}
```

To receive command-line arguments, give `main` a single `args` parameter:

```aura
def main(args: [string]) {
  print(f"{args.length()} argument(s)")
}
```

```bash
aura run app.aura one two
```

```text
2 argument(s)
```

`main` takes no parameters or a single `args`; it may return an `int` exit code
and may be `async`.

## What you learned

* `def`, expression bodies, optional return types.
* Positional, keyword, default and variadic parameters; spread calls.
* Multiple returns as tuples; recursion; nested functions; generics.
* `main` signature rules.

## Next step

[Classes and Objects →](07-classes-and-objects.md)
