---
layout: default
title: "03 — Language Basics"
parent: Learn Aura
nav_order: 13
---

[English](03-language-basics.md) · [Português](03-language-basics.pt_BR.md)

# 03 — Language Basics

> **Chapter goal:** learn how bindings, functions and the entry point fit
> together. Reference:
> [`../language-reference/statements.md`](../language-reference/statements.md),
> [`../language-reference/functions.md`](../language-reference/functions.md).

## `let`, `let mut`, `const`

Aura makes mutability explicit at the declaration.

```aura
let name = "Alice"          // immutable binding
let mut counter = 0         // mutable binding
const LIMIT = 3             // constant; must be initialised

counter += 1                // ok — counter is mutable
```

Rules:

* `let` binds a name that cannot be **reassigned**.
* `let mut` binds a name that can be reassigned.
* `const` must be initialised and is never reassignable.
* Reassigning a `let` or `const` is **E303**, and it is an error (not a
  warning) in `aura run`, `aura check` and the REPL alike.

```aura
let x = 1
x = 2              // E303: reassign an immutable binding

let mut total = 0
total += 1         // ok
```

An annotation is optional and comes after `:`:

```aura
let age: int = 30
let items: [int] = [1, 2, 3]
const MAX: int = 100
```

There is **no `var`**. `var x = 1` is rejected with a message pointing at
`let mut`/`let`.

### Destructuring

```aura
let (a, b) = (1, 2)
let [first, ...rest] = [1, 2, 3, 4]
print(a, b, first, rest)     // 1 2 1 [2, 3, 4]
```

The `...name` element collects the remaining items.

## `def` — functions

Functions are declared with `def`. There is no `fn`, `fun` or `function`.

```aura
def greet(name) -> str {
  return "Hello, " + name
}

def square(x: int) -> int = x * x     // expression body
```

The body is either a brace block or a single `= expr`. A return type after `->`
is optional; parameters may be annotated, unannotated, defaulted, or variadic.
These are all chapter 06.

## `main` — the entry point

A file run with `aura run` must have a top-level `main`. The runtime calls it;
never call it yourself.

```aura
def main() {
  print("entry point")
}
```

`main` may:

* take no parameters, or a single `args` parameter;
* be `async`;
* return an `int`, which becomes the process exit code.

```aura
def main() -> int {
  return 3          // process exit code 3
}
```

A missing `main` is **E310**; a wrong signature is **E311**; a `main` inside a
`module` body is **E312**.

## `print`

`print` writes its arguments to standard output, space-separated, with a
trailing newline:

```aura
def main() {
  print("value:", 42, true)
}
```

```text
value: 42 True
```

Boolean literals are `true` and `false`; the null value is `none`. Note how
`true` prints as Python's `True`: booleans are CPython booleans at runtime.

## Putting it together

```aura
const GREETING = "Hello"

def greet(name, punctuation = "!") -> str {
  return GREETING + ", " + name + punctuation
}

def main() {
  let mut count = 0
  print(greet("world"))
  count += 1
  print(f"greeted {count} time(s)")
}
```

```text
Hello, world!
greeted 1 time(s)
```

## Modifier position

Class and module modifiers appear **before** the declaration keyword:
`private let x = 1`, never `let private x`. This matters from chapter 07 on.

## What you learned

* `let` is immutable, `let mut` is mutable, `const` is constant.
* Reassigning an immutable binding is E303.
* `def` declares functions; the body is a block or `= expr`.
* `main` is the entry point and is invoked by the runtime.
* `print`, `true`/`false`, `none`, and f-strings.

## Next step

[Variables and Types →](04-variables-and-types.md)
