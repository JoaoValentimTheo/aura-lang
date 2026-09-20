---
title: "02 — Type System"
---

# Type System

Aura is gradually typed: annotations are optional and checked before execution. Unknown values become `AnyType` and never produce false positives.

See `aura/transpiler/types.py:1-512` (TypeInference), `aura/transpiler/types.py:514-1334` (TypeChecker).

## Primitive Types

| Aura type | Python equivalent | Description |
|-----------|-------------------|-------------|
| `int` / `Int` | `int` | Arbitrary-precision integer |
| `float` / `Float` | `float` | IEEE 754 double |
| `str` / `string` / `String` | `str` | Unicode string |
| `bool` / `Bool` | `bool` | `true` or `false` |
| `none` / `None` | `None` | Null value |

```aura
let x: int = 42
let pi: float = 3.14
let name: str = "Aura"
let flag: bool = true
let empty: none = none
```

Type annotations use lowercase names in the canonical form. PascalCase (`Int`, `Float`, `String`, `Bool`, `None`) is also accepted for backwards compatibility.

## Collection Types

| Syntax | Description |
|--------|-------------|
| `[T]` | List of `T` |
| `{K: V}` | Dict with keys `K`, values `V` |
| `{T}` | Set of `T` |
| `(T1, T2, ...)` | Tuple of fixed types |

```aura
let nums: [int] = [1, 2, 3]
let scores: {str: int} = {"alice": 95}
let unique: {int} = {1, 2, 3}
let pair: (str, int) = ("alice", 30)
```

Lists, sets, dicts, and tuples infer their element types from literals. An empty collection infers `AnyType` for its elements.

## Type Inference

The compiler infers types from literals and expressions without annotations:

```aura
let x = 42           // inferred as int
let y = 3.14         // inferred as float
let z = x + y        // inferred as float
let items = [1, 2]   // inferred as [int]
```

Inferred types are used for all internal checks. When a type annotation is provided, the checker verifies the annotated type matches the inferred type.

## Optional Types

`none` is a first-class value. An optional type is a union with `none`:

```aura
let name: str | none = none
if name is not none {
  print(name)    // type is narrowed to str here
}
```

The type checker narrows types in conditional branches — `x is not none` in an `if` body narrows `x` to its non-`none` variant. See `aura/transpiler/types.py:1030-1074` (`_narrowings`).

## Union Types

```aura
def process(value: int | str) {
  if value is int {
    print("integer: " + str(value))
  } else {
    print("string: " + value)
  }
}
```

Union types use the `|` separator. The type checker accepts a value if it is compatible with **any** member of the union.

## Function Types

Function types are expressed as `(ParamType1, ParamType2) -> ReturnType`:

```aura
let fn: (int, str) -> bool
```

Function types appear in variable annotations and generic constraints. They are erased at transpile time.

## Generic Types

Generics use **square brackets**:

```aura
class Box[T] {
  public let value: T = none
  public def new(value: T) { self.value = value }
  public def get() -> T { return self.value }
}

let b = Box(42)       // Box[int]
let s = Box("hello")  // Box[str]
```

Constraints follow `:` on the parameter:

```aura
def smallest[T: Comparable](items: [T]) -> T {
  return items[0]
}
```

Generics are erased at transpile time — the emitted Python function/class is untyped. `id(7)` works without explicit `[int]`.

## Type Annotations

Annotations are optional at every position:

```aura
def add(a: int, b: int) -> int {
  return a + b
}

let x = 5                    // no annotation, inferred as int
let y: float = 5.0           // annotation, verified against value
```

Missing annotations infer `AnyType` and pass silently. The checker is deliberately conservative.

## `as` Casts

`x as T` compiles to `T(x)`:

```aura
let x: int = "42" as int       // int("42")
let s: str = 3.14 as str       // str(3.14)
```

`as` sits at precedence 13 (same as `*`, `/`, `%`): `1 + x as int` → `(1 + int(x))`.

## Gotcha

- `let x: int = "hello"` is a type error (E101): expected `Int`, got `String`.
- `fn` is not a type keyword; it is reserved and rejected.
- Generic parameters use brackets `[T]`, not angle brackets `<T>` — `Box<T>` is a parse error.
- `(1)` is not a tuple — it is the parenthesized expression `1`. A one-element tuple needs a trailing comma: `(42,)`.
- Empty `{}` in expression position is an empty dict, not an empty block.

## Anti-pattern

Do not annotate every line with explicit types. Aura infers types well — annotations should communicate intent, not duplicate the compiler's work. Reserve annotations for function signatures and ambiguous cases.
