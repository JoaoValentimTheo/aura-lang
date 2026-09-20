---
layout: default
title: "Types — Catalog and Syntax"
parent: Aura Language Reference
nav_order: 13
---

[English](types.md) · [Português](types.pt_BR.md)

# Types — Catalog and Syntax

**Status:** Stable (`0.2.0a5`, except where labeled) · **Evidence:**
`aura/parser/to_ast.py` (`parse_type` §1752, `parse_type_decl` §1653,
`_parse_type_params` §1129), `aura/transpiler/types.py`
(`TypeInference.BUILTIN_NAMES`, `_parse_type_annotation` §1456),
`aura/transpiler/transformers/statements.py` (`transform_TypeDecl`,
`_aura_type_to_python` §770), `types.md`, execution *probes*.

This document lists **which types exist** and **how they are written**. The
*validity* rules — what can be assigned to what, when there is an error, and
which annotations are erased — are in [type-system.md](type-system.md).

Aura is **gradually typed**: annotations are optional and, except for the checks
in [type-system.md](type-system.md) §5, are **erased** before the Python emit.
The parser stores a declaration type as a **string** (`parse_type` returns
`str`); the checker consumes that string with `_parse_type_annotation`.

---

## 1. Primitive types (6)

| Aura type | `Type` instance | Python emit | Notes |
|---|---|---|---|
| `int` | `IntType` | `int` | arbitrary precision (Python) |
| `float` | `FloatType` | `float` | 64-bit (Python) |
| `str` | `StrType` | `str` | Unicode |
| `bool` | `BoolType` | `bool` | `true` / `false` |
| `bytes` | (`AnyType` — see below) | `bytes` | no dedicated `Type` class |
| `none` | `NoneType` | `None` | absence value **and** type |

`TypeInference.BUILTIN_NAMES` (`types.py:293-302`) maps exactly these spellings:
`int`, `float`, `str`, `bool`, `none`, plus the aliases `string` and `null`
(→ `StrType` / `NoneType`) and `any` (→ `AnyType`).

- **`none` is the null literal and the null type.** The literal `null` is
  **not part of Aura**: `let n = null` is rejected by the parser —
  `'null' is not part of Aura; use 'none' instead` (*probe*). Use `none`.
- **`bytes` has no checker type.** `let b: bytes = b"x"` parses and transpiles
  (emit `b'x'`), but `bytes` is absent from `BUILTIN_NAMES`, so
  `_parse_type_annotation("bytes")` falls through to `AnyType` (*probe*). The
  annotation is accepted and erased; it is **not** enforced.
- **`any`** is a real spelling that maps to `AnyType` — the gradual-typing
  escape hatch. `object` / `Never` also **parse** as type names but resolve to
  `AnyType` in the checker (`Never` is only special-cased in the emitter's
  `_SIMPLE_TYPES`, not in the checker).
- There is **no** `uint`, `char`, `void`, `unit`, `Never` (enforced), or
  `Optional`/`Result` type constructor.

### 1.1 `bool` has no numeric widening

`BoolType().is_compatible(IntType())` is `False` and vice-versa (*probe*);
`true + 1` is `E108`. There is no `bool → int` coercion in the checker, unlike
the Kof model.

---

## 2. Collection types

| Spelling | Checker `Type` | Python emit | Status |
|---|---|---|---|
| `[T]` | `ListType(T)` | `list` | Stable |
| `{K: V}` | `DictType(K, V)` | `dict` | Stable |
| `{F1: T1, F2: T2}` | `DictType()` (opaque) | `dict` | Stable (structural) |
| `{T}` | **not parseable** | `set` | **Syntax error** (see §2.3) |
| `(A, B)` | **not parseable** | `tuple` | **Syntax error** (see §2.3) |
| `List[T]` | `ListType(T)` | `list` | alias spelling |
| `Set[T]` | `SetType(T)` | `set` | alias spelling |
| `Dict[K, V]` | `DictType(K, V)` | `dict` | alias spelling |

### 2.1 Lists

`[T]` parses to the string `List[T]` (`to_ast.py:1773-1779`); the checker
re-reads both `[T]` and `List[T]` into `ListType` (`types.py:1472-1476`). A
multi-argument form `[T, U]` is also accepted by the parser and becomes
`List[T, U]`, but the checker only reads the first component — **UNSPECIFIED**
beyond that.

```aura
let xs: [int] = [1, 2, 3]
let nested: [[int]] = [[1], [2]]
let mixed = [1, "two", 3.0, true]   // inferred List[Int] (first element only)
```

Element type inference uses the **first element only** (`types.py:330-333`):
`[1, "two"]` infers `List[Int]`. Empty literal `[]` infers `List[Any]`.

### 2.2 Dicts and structural types

`{K: V}` and `{name: str, age: int}` take the **same parser branch**
(`to_ast.py:1781-1797`): a brace type is a comma-separated `key: type` list and
is stored verbatim as `{K: V}` / `{name: str, age: int}`.

The checker treats **any** braced annotation as `DictType` and does **not**
retain field names (`types.py:1486-1488`, and `StructuralType → DictType()` at
`types.py:1508-1509`). Consequences:

- `{str: int}` → `DictType(StrType, IntType)`.
- `{name: str, age: int}` → `DictType()` (opaque) — the **shape is not
  checked**; any dict is compatible with it.
- A single-entry braces with no colon (e.g. `{int}`) is a **set type**
  (`Set[int]`) — see §2.3.

Optional fields are **not** supported: `{name: str, email?: str}` is a syntax
error — `Expected ':' but got '?'` (*probe*). An optional field is written by
union: `{name: str, email: str | none}`.

A leading visibility word is tolerated inside a brace type
(`{public name: str}` → `{name: str}`, `to_ast.py:1785-1787`).

### 2.3 Set and tuple type syntax

Both parse and map to the matching Python generic:

- `{T}` is a **set type**: `let s: {int} = {1, 2, 3}` → `Set[int]` (*probe*).
  A brace type is a set type when it holds a single type with no `:`, and a
  structural type otherwise (`{name: str}`).
- `(A, B)` is a **tuple type**: `let t: (int, str) = (1, "a")` → `Tuple[int, str]`
  (*probe*). A parenthesised list of types followed by `->` is a function type
  instead (§4).

Set and tuple types are also reachable by their generic spellings `Set[T]` /
`Tuple[T, U]`, and by inference from a literal (`SetLiteral` → `SetType`,
`TupleLiteral` → `TupleType`, `types.py`). The runtime values `{1,2,3}` and
`(1, "x")` transpile to Python `set` and `tuple`.

---

## 3. Optional and union types

### 3.1 Optional `T?`

```ebnf
optional-type = type-ref , "?" ;
```

`T?` parses as a suffix (`to_ast.py:1814-1816`) and the checker lowers it to
`UnionType({T, NoneType()})` (`types.py:1468-1469`).

```aura
let name: str? = get_name()
```

`T?` and `T | none` are the **same type**. There is no dedicated `Optional`
class. A `?` may repeat (`T??`) — the parser appends another `?` and the
checker nests unions; the extra layer is **UNSPECIFIED**.

### 3.2 Union `T | U`

```ebnf
union-type = type-ref , "|" , type-ref , { "|" , type-ref } ;
```

`|` is right-associative in the parser (`parse_type` recurses on the RHS after
matching `|`, `to_ast.py:1817-1820`) and the checker builds a flat
`UnionType` **set** from the split parts (`types.py:1489-1491`).

```aura
let value: int | str = 42
let result: int | float | none = none
```

Union membership is a `set` (`types.py:251`), so order and duplicates are not
observable. A union is compatible with each of its members
(`UnionType.is_compatible` is `any(...)`, `types.py:257-258`); the reverse
(`int` declared, union expected) holds too since the inferred side is the
member. See [type-system.md](type-system.md) §5 for the assignment direction
and the optional-unwrap caveat.

---

## 4. Function types

```ebnf
function-type = "(" , [ type-ref , { "," , type-ref } ] , ")" , "->" , type-ref ;
```

```aura
let handler: (int, int) -> int = add
let predicate: (str) -> bool = is_valid
let callback: () -> none = on_ready
let compose: ((int) -> int) -> (int) -> int = make
```

The parser recognizes a leading `(` as a function type and returns the string
`(A, B) -> C` (`to_ast.py:1760-1771`). The checker maps **any** string
beginning with `(` and containing `->` to a bare `FunctionType()`
(`types.py:1470-1471`) — parameter and return types are **not** preserved in
the annotation. A parenthesized non-arrow type is a syntax error.

There is no `async` or variadic marker in the annotation grammar; the checker's
`FunctionType` carries `is_async`/`variadic`/`required_args`, but those are
populated from **declarations** (`_method_to_function_type`,
`_check_function_decl`), not from function-type annotations.

---

## 5. Generic types

```ebnf
generic-type = name , "[" , type-ref , { "," , type-ref } , "]" ;
```

```aura
class Box[T] { public let value: T = none }
class Pair[A, B] { public let first: A = none }

def first[T](items: [T]) -> T | none { return items[0] }
def map[T, R](items: [T], f: (T) -> R) -> [R] { return [f(i) for i in items] }

let int_box: Box[int] = Box(42)
let p: Pair[str, int] = Pair("age", 30)
```

- **Brackets only.** `Box<T>` is rejected with a pointed error: *"type
  arguments use brackets, not '<...>' (write 'Box[T]')"* (`to_ast.py:1809-1813`).
- Type parameters are introduced as `Name[T, U]` on the declaration
  (`_parse_type_params`, `to_ast.py:1129`) and resolved to `TypeVariable`
  inside the declaration's scope (`types.py:811-815, 871-874`).
- `Box[int]` parses to the string `Box[int]`; the checker does **not** build a
  parameterized `ClassType` — a name with brackets that is not `List`/`Set`/
  `Dict` falls through to `AnyType` (`types.py:1492-1494`). Generic arguments
  are effectively **erased** on both sides.
- `GenericType` AST nodes (programmatic construction) map to
  `ListType`/`SetType` for those two bases and to `AnyType` otherwise
  (`types.py:1503-1507`).

### 5.1 Generic constraints

A parameter may carry a constraint after `:`. The constraint is a builtin type
or a class/trait declared in the same program; a union of such names is
accepted. Anything else is `E110` (`UNKNOWN_TYPE_CONSTRAINT`,
`types.py:653-691`).

```aura
class Comparable { public def compare(other: Comparable) -> int { return 0 } }

def smallest[T: Comparable](items: [T]) -> T { return items[0] }
def display[T: int | str](value: T) -> str { return str(value) }
```

Constraints are **compile-time only**: the emitted Python is unchanged except
for the generic class machinery (`Generic[TypeVar('T')]`); the checker verifies
only that every constraint name **resolves** — it does not enforce that call
sites satisfy it. A parameter that is declared but never used triggers the
warning **`W103`** (`UNUSED_TYPE_PARAMETER`).

Type parameters are parsed for `class`, `trait`, `def`, and `type`. A
constraint on a name that is not one of the owner's parameters is `E110`.

---

## 6. Class and type-alias types

### 6.1 Classes, traits, enums

A `class`/`trait`/`enum` name declared in the program is a type, written by
name (`User`, `Animal`). A dotted name (`pkg.Base`) does **not** parse as a
type annotation — `foo.bar.Baz` → `Unexpected token '.'` (*probe*); dotted
bases are handled only in the `extends` clause, not in `parse_type`. See
[classes.md](classes.md).

### 6.2 Type aliases

```ebnf
type-decl = "type" , name , [ "[" , type-param , { "," , type-param } , "]" ] , "=" , type-ref ;
```

```aura
type UserId = int
type Point = {x: float, y: float}
type Pair[A, B] = (A, B)
```

`TypeDecl` stores the alias as the **string** `type_expr` and is **not**
resolved by the checker (`types.py:719-720` skips `TypeDecl`). Using an alias
name in an annotation therefore resolves via `self.classes`/`BUILTIN_NAMES`
and otherwise to `AnyType` (`types.py:1492-1494`). The alias emits a best-effort
Python name (`UserId = int  # type alias: int`,
`statements.py:751-798`).

Union type aliases parse and emit correctly: `type Maybe = int | none` records
`type_expr = 'int | none'` and emits `Maybe = int | none  # type alias: int | none`
(a union component after `|` continues the type rather than starting a new
statement).

---

## 7. Literals and their inferred types

Inferred by `TypeInference.infer` (`types.py:313-371`):

| Literal | Inferred | Evidence |
|---|---|---|
| `42` | `int` | `IntLiteral` → `IntType` |
| `3.14` | `float` | `FloatLiteral` → `FloatType` |
| `"…"` / `f"…"` | `str` | `StrLiteral`/`FStringLiteral` → `StrType` |
| `b"…"` | **`Any`** | no `bytes` literal branch |
| `true` / `false` | `bool` | `BoolLiteral` → `BoolType` |
| `none` | `none` | `NoneLiteral` → `NoneType` |
| `[1, 2]` | `[int]` (first element) | `types.py:330-333` |
| `{1, 2}` | `{int}` set | `SetLiteral` → `SetType` |
| `(1, "x")` | `(int, str)` | `TupleLiteral` → `TupleType` |
| `{a: 1}` | `{str: int}` (first pair) | `types.py:340-345` |
| `1..10` | `[int]` | `RangeExpr` → `ListType(IntType)` |
| `1 + 2` | `int` | `_infer_binary_op` |
| `3 / 2` | `float` | `/` always yields `float` |
| `"a" + "b"` | `str` | string concatenation |
| `x ?? y` | `strip-none(x)` | `CoalesceExpr` |
| `x?.f` | `Optional(infer x)` | `SafeNavExpr` |

Binary inference (`types.py:444-480`) is a **checker-local** model. Note `3 / 2`
is `float` here even though Python's `/` on ints is also float — consistent;
there is no int-division operator distinction in this table.

---

## 8. What is NOT a type

| Spelling | Result | Evidence |
|---|---|---|
| `null` (literal or type) | Syntax error — use `none` | *probe*: `'null' is not part of Aura` |
| `{T}` (set annotation) | Syntax error (`Expected ':'`) | `parse_type` `{` branch |
| `(A, B)` (tuple annotation) | Syntax error (`Expected '->'`) | `parse_type` `(` branch |
| `email?: str` (optional field) | Syntax error | *probe* |
| `Box<T>` (angle generics) | Syntax error, pointed message | `to_ast.py:1809-1813` |
| `pkg.Type` in an annotation | Syntax error (`Unexpected '.'`) | *probe* |
| `void` / `unit` | not a type name; `none` is the absence type | `BUILTIN_NAMES` |
| `uint` / `char` | absent | `BUILTIN_NAMES` |
| `Optional[T]` / `Result[T, E]` | treated as a generic name → `AnyType`, no checking | `types.py:1492-1494` |
| intersection types, literal/singleton types | absent | — |
| `bytes` **checked** type | parses, but `AnyType` (erased) | `BUILTIN_NAMES` |

---

## 9. Type recursion

A type may refer to itself **by name** in a class field, a method signature, or
a structural brace type: `class Node { public let next: Node? = none }`. The
checker resolves names lazily and erases unknown names to `AnyType`, so
recursion is allowed and finite — resolution is by name, not by expansion.
`List[List[int]]` nests freely. **Recursive types are allowed.**
