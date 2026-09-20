---
title: "08 — Pattern Matching"
---

# Pattern Matching

`match` dispatches on shape and value. Patterns test and bind — they never compute.

See `aura/parser/to_ast.py:parse_match_stmt`, `aura/transpiler/transformers/statements.py:transform_MatchStmt`, `aura/transpiler/types.py:1099-1265` (exhaustiveness).

## `match` as a Statement

```aura
match status {
  case 0 { print("inactive") }
  case n if n > 100 { print("overflow") }
  case n if n > 0 { print("positive") }
  case _ { print("other") }
}
```

A case body is a **block** `{ ... }` or an **arrow** `-> expression`. There is no fallthrough. Patterns are tried top to bottom; the first match wins.

## `match` as an Expression

With arrow bodies, `match` produces a value:

```aura
let n = 2
let text = match n {
  case 1 -> "one"
  case 2 -> "two"
  case _ -> "other"
}
print(text)              // two
```

The expression form is hoisted into a helper function.

## Pattern Types

| Pattern | Example | Binds |
|---------|---------|-------|
| Wildcard | `case _` | nothing |
| Literal | `case 0`, `case "quit"` | nothing |
| Binding | `case n` | `n` |
| Guard | `case n if n > 0` | `n`, only if guard holds |
| Or-pattern | `case 1 \| 2` | — |
| Enum member | `case Color.RED` | — |
| List destructuring | `case [a, b]` | `a`, `b` |
| Tuple destructuring | `case (a, b)` | `a`, `b` |
| Spread | `case [first, *rest]` | `first`, `rest` |

### Wildcard

```aura
match x {
  case _ { print("anything") }
}
```

### Literal

```aura
match command {
  case "quit" { return }
  case "help" { print("showing help") }
  case _ { print("unknown") }
}
```

### Binding

A bare lowercase name always **binds** (it does not compare against a variable):

```aura
match x {
  case n { print(n) }   // n is bound to the value of x
}
```

### Guard

```aura
match n {
  case x if x > 100 { print("big") }
  case x if x > 0 { print("positive") }
  case _ { print("non-positive") }
}
```

A guarded case does **not** count as a catch-all for exhaustiveness.

### Or-Patterns

```aura
match x {
  case 1 | 2 { print("one or two") }
  case _ { print("other") }
}
```

Every alternative must bind the same names.

### Enum Members

```aura
enum Color { RED, GREEN, BLUE }

match color {
  case Color.RED { print("red") }
  case _ { print("other color") }
}
```

`Color.RED` is a distinct member value, never the string `"Red"`.

### Destructuring

```aura
match pair {
  case [a, b] { return a + b }
  case _ { return 0 }
}

match point {
  case [x, y] { print(f"({x}, {y})") }
  case [x, y, z] { print(f"({x}, {y}, {z})") }
}
```

`*rest` collects the remainder: `case [first, *rest]`.

### Constructor Patterns

```aura
match shape {
  case Circle(r) { return 3.14 * r * r }
  case Square(s) { return s * s }
  case _ { return 0 }
}
```

## Exhaustiveness

The type checker warns (**E109**) when a `match` over a known finite domain (`bool`, enum) has no catch-all. This is a warning, never a build failure:

```aura
enum Color { RED, GREEN, BLUE }

match color {
  case Color.RED { print("red") }
  case Color.GREEN { print("green") }
  // E109: no case handles Blue
}
```

A guarded `case _ if cond` does **not** suppress the warning.

## Ordering

Order matters. Patterns are tried top to bottom and the first match wins. Put the most specific patterns first:

```aura
match value {
  case 0 { print("zero") }              // specific literal first
  case n if n > 100 { print("big") }    // guard next
  case n if n > 0 { print("positive") } // broader guard
  case _ { print("other") }             // catch-all last
}
```

## Gotcha

- A bare binding (`case s`) matches **any** value. A guard like `case s if s.length() > 3` can fail at runtime if `s` has no `length` — check the type in the guard.
- `match` needs a catch-all for finite domains, or E109 warns. A guarded `case _` does not suppress it.
- There is no `when` keyword; guards use `if`.
- `:` and `=>` case syntax are rejected; use `->` or `{ ... }`.

## Anti-pattern

Do not use `match` for a couple of boolean conditions. Use `if`/`else if` instead. Use `match` when dispatching on **shape** (destructuring), an **enum**, or several literal values at once.
