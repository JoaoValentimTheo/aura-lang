---
layout: default
title: "05 — Control Flow"
parent: Learn Aura
nav_order: 15
---

[English](05-control-flow.md) · [Português](05-control-flow.pt_BR.md)

# 05 — Control Flow

> **Chapter goal:** branch and repeat. Reference:
> [`../language-reference/statements.md`](../language-reference/statements.md)
> (control flow) and [`../language-reference/expressions.md`](../language-reference/expressions.md)
> (the ternary operator). Examples: `fibonacci.aura`, `prime_checker.aura`.

## `if` / `else if` / `else`

```aura
def main() {
  let n = 7
  if n % 2 == 0 {
    print("even")
  } else if n % 3 == 0 {
    print("divisible by 3")
  } else {
    print("odd")
  }
}
```

```text
odd
```

The braces are **mandatory**. `if c print(1)` is a parse error. `else if`
chains to any depth, but `elif` and `elsif` are rejected with a pointed message
telling you to write `else if`.

## `unless` — inverted `if`

`unless cond { ... }` runs the block when `cond` is falsy:

```aura
def main() {
  let authenticated = false
  unless authenticated {
    print("please log in")
  }
}
```

```text
please log in
```

`unless` also accepts an `else`.

## `guard ... else`

`guard cond else { ... }` runs the `else` block when `cond` is **falsy**, then
continues. It is the idiom for early validation — the happy path stays flat:

```aura
def process(value) {
  guard value > 0 else {
    print(f"invalid: {value}")
    return
  }
  print(f"processing {value}")
}
```

At the top level, a bare `return` in the guard body exits the program. This is
Aura's idiom for "stop here":

```aura
guard ready else { return }
```

## Loops

Aura has four loop forms plus `for ... in`.

```aura
let mut i = 0
while i < 3 {
  print(i)
  i += 1
}

until i >= 5 {
  i += 1
}

loop {
  if i >= 6 { break }
  i += 1
}
```

| Form | Runs |
|---|---|
| `while cond { }` | while `cond` is truthy |
| `until cond { }` | while `cond` is **falsy** (inverted `while`) |
| `loop { }` | forever, until `break`/`return`/`throw` |

There is no `repeat` or `do`; use `loop { }` or `until`.

### `for ... in` and ranges

```aura
for item in [10, 20, 30] { print(item) }

for i in 0..<3 { print(i) }          // 0 1 2   (exclusive)
for i in 0..5 step 2 { print(i) }    // 0 2 4
for i in range(0, 6, 2) { print(i) } // 0 2 4
```

`1..10` is **inclusive** (1 through 10); `0..<10` is **exclusive**.
`for x in items step 2` slices any iterable with `[::2]`. A `for` target is a
pattern, so `for (k, v) in pairs` destructures each element.

### Labels

A label before a loop names it; `break label` / `continue label` target that
loop:

```aura
outer: for i in range(3) {
  inner: for j in range(3) {
    if j == 1 { continue outer }
    if i == 2 { break outer }
    print(i, j)
  }
}
```

A `break`/`continue` outside a loop is rejected (`E004`); a label that names no
enclosing loop is `E318`.

**Gotcha:** `break` at the end of an arrow-bodied `match` case swallows the
following `case` as a label (newlines are insignificant). Use a `;` or a block
body to disambiguate.

## `match` — a first look

`match` compares a value against patterns. It is statement and expression, and
is covered in depth in chapter 11:

```aura
def classify(value) -> str {
  match value {
    case 0 { return "zero" }
    case n if n > 100 { return "big" }
    case n if n > 0 { return "positive" }
    case _ { return "other" }
  }
  return "unreachable"
}
```

Case bodies are blocks `{ ... }` or arrows `-> expression`. Guards use `if`.
`:` and `=>` are rejected.

## Ternary (conditional expression)

```aura
let label = condition ? "yes" : "no"
```

`c ? t : f` emits `t if c else f`. It binds looser than everything down to `or`.

## Statement vs expression

Most control-flow constructs are statements. `if`, `match` and `try` also have
an **expression** form that produces a value:

```aura
let x = if ready { 1 } else { 0 }
let kind = match n { case 1 -> "one"; case _ -> "other" }
```

Assignment itself is **not** an expression.

## What you learned

* `if`/`else if`/`else`, `unless`, `guard ... else`.
* `while`, `until`, `loop`, `for ... in`, ranges and `step`, labels.
* A first look at `match`, the ternary `? :`, and `if`/`match` as expressions.

## Next step

[Functions →](06-functions.md)
