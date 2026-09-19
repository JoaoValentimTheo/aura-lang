---
layout: default
title: "10 — Lambdas and Functional Programming"
parent: Learn Aura
nav_order: 20
---

[English](10-lambdas-and-functional.md) · [Português](10-lambdas-and-functional.pt_BR.md)

# 10 — Lambdas and Functional Programming

> **Chapter goal:** write anonymous functions, capture variables, and build data
> pipelines. Reference:
> [`../language-reference/closures.md`](../language-reference/closures.md).
> Example: `../../examples/functional.aura`.

## Lambda forms

A lambda is an anonymous function value, written with `=>`. There is **no
`lambda` keyword**.

```aura
let double = x => x * 2
let square = (x) => x * x
let add = (a, b) => a + b
let get_answer = () => 42
```

| Form | Example |
|---|---|
| Single parameter, no parens | `x => x * 2` |
| One parameter, parens | `(x) => x * x` |
| Multiple parameters | `(a, b) => a + b` |
| None | `() => 42` |
| Expression body | `(x) => x + 1` |
| Block body | `(x) => { let y = x + 1; return y }` |
| Currying | `(n) => (x) => x + n` |

An expression body becomes a Python `lambda`; a block body is hoisted to a real
named function (`_aura_lambda_N`), because Python's `lambda` cannot hold
statements.

Parameters use the `def` grammar: annotated, defaulted, `*args`, `**kwargs`.
An annotation is accepted and erased; a `->` return type is **not** lambda
syntax.

## Calling lambdas

```aura
def main() {
  let add = (a, b) => a + b
  let blocky = (x) => {
    let y = x + 1
    return y * 2
  }
  print(add(2, 3))        // 5
  print(blocky(4))        // 10
}
```

## Closures

A lambda that reads an enclosing local captures it by reference:

```aura
def main() {
  let base = 10
  let add_base = (x) => x + base
  print(add_base(5))      // 15
}
```

Currying — returning a lambda that captures an argument:

```aura
def main() {
  let make_adder = (n) => (x) => x + n
  let add10 = make_adder(10)
  print(add10(5))         // 15
}
```

**Limitation:** a **block lambda that assigns** to an enclosing local currently
trips `E319` (`'n' is used before it is declared`) in the checker, even though
the reference describes emitting `nonlocal`. Read-only capture and currying are
verified; for a counter, use a mutable object (a one-element list or a class)
instead of mutating a captured local.

**Gotcha:** capture is by **reference**, not a per-iteration snapshot. A lambda
created in a loop sees the loop variable's **final** value. Bind it as a default
parameter (`(x, i = i) => x + i`) for per-iteration capture.

## Higher-order functions

Lambdas are ordinary values, so functions take and return them. This is how the
collection helpers work.

```aura
def apply_twice(f, x) {
  return f(f(x))
}

def main() {
  print(apply_twice((x) => x + 3, 1))    // 7
}
```

## `map` / `filter` / `reduce`

These come from `stdlib.collections` and are auto-imported when used:

```aura
def main() {
  let numbers = [1, 2, 3, 4, 5]
  let doubled = map(numbers, (x) => x * 2)          // [2, 4, 6, 8, 10]
  let evens = filter(numbers, (x) => x % 2 == 0)    // [2, 4]
  let total = reduce(numbers, (a, b) => a + b, 0)   // 15
  print(doubled, evens, total)
}
```

## The pipe operator `|>`

`a |> f` applies `f` to `a`; `a |> f(b)` inserts `a` as the **first** argument.
Pipelines read left to right:

```aura
def main() {
  let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

  let result = numbers
    |> filter((x) => x % 2 == 0)
    |> map((x) => x * x)
    |> reduce((acc, x) => acc + x, 0)

  print(f"Sum of even squares: {result}")    // 220
}
```

When the right side is a stdlib collection function (`map`, `filter`, `reduce`,
`take`, `drop`), the import is injected automatically. Pipe is the **loosest**
expression operator and chains left to right. If the right side is a bare
identifier, it becomes `f(left)`.

## Composing

```aura
def main() {
  let compose = (f, g) => (x) => g(f(x))
  let inc = x => x + 1
  let double = x => x * 2
  print(compose(inc, double)(5))    // 12 — double(inc(5))
}
```

## No trailing-lambda sugar

Aura has **no** `f { ... }` trailing-lambda call syntax. A `{ ... }` after a
call is either a separate block or a struct initialiser, not a final argument.
Put the lambda inside the parentheses:

```aura
apply((x) => x + 1)      // correct
// apply(1) { x => x + 1 }   // NOT a trailing lambda
```

## What you learned

* Lambdas `x => ...`, `(a, b) => ...`, `() => ...`, with expression or block
  bodies.
* Read capture and currying; the mutable-capture limitation.
* Higher-order functions; `map`/`filter`/`reduce`; the pipe operator.
* No trailing-lambda sugar.

## Next step

[Pattern Matching →](11-pattern-matching.md)
