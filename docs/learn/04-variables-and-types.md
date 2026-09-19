---
layout: default
title: "04 — Variables and Types"
parent: Learn Aura
nav_order: 14
---

[English](04-variables-and-types.md) · [Português](04-variables-and-types.pt_BR.md)

# 04 — Variables and Types

> **Chapter goal:** know which types exist, how to write them, and what the
> checker actually enforces. Reference:
> [`../language-reference/types.md`](../language-reference/types.md) and
> [`../language-reference/type-system.md`](../language-reference/type-system.md).

## Aura is gradually typed

Annotations are **optional**. A binding without an annotation takes the type
inferred from its initialiser; a binding the checker cannot prove is treated as
`any` and accepted. There is no "must annotate" rule.

```aura
let inferred = 42          // int, from the literal
let explicit: int = 42     // written out
```

Crucially, annotations are **erased** before the Python emit. They document and
(partly) check; they do not change the runtime. `aura check` runs the type
checker; `aura run` does not — so a program can run with a type error that
`check` would report.

## Primitive types

| Aura type | Meaning |
|---|---|
| `int` | integer (arbitrary precision, Python `int`) |
| `float` | 64-bit floating point |
| `str` | Unicode string |
| `bool` | `true` / `false` |
| `bytes` | byte string (annotation accepted, not checked) |
| `none` | absence value **and** its type |

```aura
let age: int = 30
let price: float = 3.14
let title: str = "Aura"
let active: bool = true
let nothing: none = none
```

`null` is **not** Aura — `let n = null` is a pointed parse error telling you to
use `none`. There is no `char`, `uint`, `void` or `unit` type.

### `bool` does not widen to `int`

Unlike Kof/JVM, `true + 1` is rejected (`E108`); there is no `bool → int`
coercion. Convert explicitly with `int(b)` if you need a number.

## Collection types

```aura
let items: [int] = [1, 2, 3]              // list
let scores: {str: int} = {math: 95}       // dict
let structured: {x: float, y: float} = {x: 1.0, y: 2.0}   // structural
let boxed: Box[int] = Box(42)             // generic (brackets only)
```

| Spelling | Meaning |
|---|---|
| `[T]` | list of `T` (also `List[T]`) |
| `{K: V}` | dict from `K` to `V` (also `Dict[K, V]`) |
| `{name: T, ...}` | structural shape; checked as a dict, field names not retained |
| `Set[T]` | set of `T` |
| `Box[T]` | generic type application — **brackets**, never `<...>` |

`Box<T>` is rejected: *"type arguments use brackets, not '<...>'"*. Set and
tuple **type** spellings are limited: `{T}` and `(A, B)` do not parse as types
(see [`types.md`](../language-reference/types.md) §2.3), though set and tuple
**values** `{1, 2}` and `(1, "x")` exist and are inferred.

## Optional and union types

```aura
let maybe: str? = none            // optional — same as `str | none`
let value: int | str = 42         // union
```

`T?` lowers to `T | none`. A union is compatible with each of its members.

## Function types

```aura
let handler: (int, int) -> int = add
let predicate: (str) -> bool = is_valid
```

A leading `(` with `->` is a function type. Parameter and return types are not
retained by the checker — any such annotation becomes a plain function type.

## Type aliases

```aura
type UserId = int
type Point = {x: float, y: float}
```

Aliases are documentation: the checker does not expand them, so an alias name in
an annotation resolves to `any` unless it is also a declared class.

## Casts: `as`

```aura
let q = 3.0 as int        // int(3.0)
let s = 42 as str         // str(42)
print(7 / 2)              // 3.5
print(int(7 / 2))         // 3 — integer quotient
```

`x as T` emits `T(x)`. Aura has **no floor-division operator**; `//` is a line
comment. Use `int(a / b)`.

## `none` and null checks

```aura
let missing = none
print(missing is none)          // true
print(missing ?? "fallback")    // fallback
print((0) ?: "zero-ish")        // zero-ish
```

* `x is none` / `x is not none` — identity null check.
* `x ?? y` — yields `y` only when `x` is `none`.
* `x ?: y` — yields `y` when `x` is **falsy** (`none`, `false`, `0`, `""`, `[]`,
  `{}`).

Comparing any other literal with `is` is a parse error: write `x == 5`, not
`x is 5`.

## What the checker guarantees

* Same type, or either side `any` — accepted.
* `int → float` widening is **not** implicit; annotate or cast.
* Unannotated functions have no inferred return type; the annotation is
  documentation.

The guarantee boundary is spelled out in
[`type-system.md`](../language-reference/type-system.md) §4.

## What you learned

* Primitives: `int`, `float`, `str`, `bool`, `bytes`, `none`.
* Collections: `[T]`, `{K: V}`, `{shape}`, `Set[T]`, `Box[T]`.
* Optional `T?`, union `T | U`, function types `(T) -> R`.
* `none`, `is none`, `??`, `?:`, and the `as` cast.
* Annotations are optional, checked partially, and erased at runtime.

## Next step

[Control Flow →](05-control-flow.md)
