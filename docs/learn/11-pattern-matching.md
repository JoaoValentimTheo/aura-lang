---
layout: default
title: "11 — Pattern Matching"
parent: Learn Aura
nav_order: 21
---

[English](11-pattern-matching.md) · [Português](11-pattern-matching.pt_BR.md)

# 11 — Pattern Matching

> **Chapter goal:** dispatch on shape and value with `match`. Reference:
> [`../language-reference/statements.md`](../language-reference/statements.md)
> §5. Example: `../../examples/pattern_matching.aura`.

## `match` as a statement

```aura
def classify(value) -> str {
  match value {
    case 0 { return "zero" }
    case 1 { return "one" }
    case n if n > 100 { return "big" }
    case n if n > 0 { return "positive" }
    case n if n < 0 { return "negative" }
    case _ { return "other" }
  }
  return "unreachable"
}
```

```text
0 -> zero
1 -> one
42 -> positive
200 -> big
-7 -> negative
```

A case body is a **block** `{ ... }` or an **arrow** `-> expression`. There is
no fallthrough. `:` and `=>` case syntax are rejected with a pointed message.

## Patterns

| Pattern | Example | Binds |
|---|---|---|
| Wildcard | `case _` | nothing |
| Literal | `case 0`, `case "quit"` | nothing |
| Binding | `case n` | `n` |
| Guard | `case n if n > 0` | `n`, only if the guard holds |
| Or-pattern | `case 1 &#124; 2` | — |
| Enum member | `case Color.RED` | — |
| List/tuple destructuring | `case [a, b]`, `case [first, *rest]` | `a`, `b`, ... |

A guarded case does **not** count as a catch-all: a `match` over a scalar,
`bool` or enum domain with no `case _` (or bare binding) warns `E109`.

## Matching strings

```aura
def command(name) -> str {
  match name {
    case "quit" { return "exiting" }
    case "help" { return "showing help" }
    case _ { return "unknown command" }
  }
  return ""
}
```

## Destructuring

```aura
def sum_pair(pair) -> int {
  match pair {
    case [a, b] { return a + b }
    case _ { return 0 }
  }
  return 0
}
```

`*rest` collects the remainder: `case [first, *rest]`.

## `match` as an expression

With arrow bodies, `match` produces a value:

```aura
def main() {
  let n = 2
  let text = match n {
    case 1 -> "one"
    case 2 -> "two"
    case _ -> "other"
  }
  print(text)              // two
}
```

The expression form is hoisted into a helper function; the tail expression of
each case becomes the returned value.

## Enums

```aura
enum Color { RED, GREEN, BLUE }

def main() {
  let c = Color.RED
  match c {
    case Color.RED { print("red") }
    case _ { print("other color") }
  }
}
```

Members are comma-separated. `Color.RED` is a distinct value, never the string
`"Red"`.

## Gotchas

* **`break` in an arrow case swallows the next `case` as a label.** Because
  newlines are insignificant, `case x -> break` followed by `case y` parses as
  `break case`. Use a block body or a trailing `;`.
* `match` needs a catch-all for a finite domain, or `E109` warns (it never fails
  a build). A guarded `case _ if cond` does **not** suppress the warning.
* There is no `when` keyword; guards use `if`.

## Choosing `match` over `if`

Use `if`/`else if` for a couple of boolean conditions. Use `match` when you are
dispatching on **shape** (destructuring), on an **enum**, or on several literal
values at once — it keeps the value being inspected in one place.

## Patterns, precisely

A pattern is not an expression: it never *computes*, it only **tests** and
**binds**. A bare lowercase name always binds (it does not compare against a
variable of the same name); capitalized dotted members such as `Color.RED`
compare against an enum member. That distinction is why `case n` is a catch-all
binding while `case Color.RED` is a value test. To compare against a computed
value, use a guard: `case x if x == threshold`.

An or-pattern `case 1 | 2` matches when **any** alternative matches, and every
alternative must bind the same names. Destructuring `case [a, b]` matches only a
list/tuple of exactly two elements; add `*rest` to allow more.

## A fuller example

```aura
enum Color { RED, GREEN, BLUE }

def describe(value) -> str {
  match value {
    case 0 { return "zero" }
    case Color.RED { return "red" }
    case [a, b] { return f"pair summing to {a + b}" }
    case [first, *rest] { return f"list starting with {first}" }
    case "hello" { return "the greeting" }
    case _ { return "something else" }
  }
  return ""
}
```

```text
zero
pair summing to 3
list starting with 1
the greeting
red
something else
```

Order matters: patterns are tried top to bottom and the first match wins. Put
the most specific patterns first, and end with `case _` when the domain is not
exhaustive.

**Gotcha:** a bare binding pattern (`case s`) matches **any** value, so a guard
on it such as `case s if s.length() > 3` will also be reached by values that
have no `length` — order the concrete cases first, or check the type in the
guard.

## Exercises

1. Write `def sign(n) -> str` using literal and guard patterns.
2. Match on a list and return its head, or `none` when empty.
3. Build a two-value enum and dispatch on it, then remove `case _` and observe
   the `E109` warning from `aura check`.

## What you learned

* `match` as a statement and as an expression.
* Wildcard, literal, binding, guard, or, enum and destructuring patterns.
* No fallthrough; `->` and `{ }` bodies; the `break` gotcha; `E109`.

## Next step

[Error Handling →](12-error-handling.md)
