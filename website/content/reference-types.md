# Types

Aura is dynamically-typed with an **optional, conservative** static checker.
Annotations are never required, and the checker rejects only what it can prove
is wrong. A value whose type cannot be determined has the checker type
`Unknown`, and no type rule is applied to it.

## Primitive types

| Type | Description |
|---|---|
| `int` | signed 64-bit integer |
| `float` | IEEE-754 binary64 |
| `bool` | `true` or `false` |
| `string` | immutable UTF-8 text |
| `none` | absence — the only absence value, with no static `none` type |

## Compound types

| Type | Description |
|---|---|
| `[T]` | list of `T` |
| `{string: V}` | map with string keys and values of `V` |
| `Named(name)` | a user struct or alias-resolved type |
| `Enum(name)` | an enum type |
| `Unknown` | not determined |

A map annotation with a non-`string` key is rejected (`E3001`).

## Union types

A type may union several members with `|`. The only historical form,
`T | none`, is now the one-member-plus-`none` case of general unions:

```aura
type Number = int | float
type ID = string | int
type UserID = ID            # aliases compose transparently into unions
let found: int | none = none
```

A union accepts a value when **one** of its members does. `int | float` accepts
an `int` and a `float`; `int | float | none` also accepts `none`. Union member
order and duplicates do not matter (`int | float` and `float | int` are the
same type). Because `none` has no static type, a union containing `none` is
permissive — `T | none` accepts any value the checker cannot rule out, exactly
as before.

## Compatibility

"`A` is compatible with `B`" holds when either side is `Unknown`, both are the
same primitive, both are lists/maps with compatible element types, both are the
same nominal type, or a union member matches. `int` and `float` are **not**
compatible in annotations — there is no implicit numeric coercion in
annotations, even though mixed arithmetic promotes at runtime. (A union such as
`int | float` accepts either, because each member is checked separately.)

| Expected ↓ / Actual → | int | float | bool | string | list | map | Named | Enum | Union | Unknown |
|---|---|---|---|---|---|---|---|---|---|---|
| `int` | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓† | ✓ |
| `float` | ✗ | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓† | ✓ |
| `bool` | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓ |
| `string` | ✗ | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓ |
| `[T]` | ✗ | ✗ | ✗ | ✗ | ✓* | ✗ | ✗ | ✗ | ✗ | ✓ |
| `{string: V}` | ✗ | ✗ | ✗ | ✗ | ✗ | ✓* | ✗ | ✗ | ✗ | ✓ |
| `Named(n)` | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓** | ✗ | ✗ | ✓ |
| `Enum(n)` | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓** | ✗ | ✓ |
| `Union` | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓ |
| `Unknown` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

\* element/value types must themselves be compatible.
\** same nominal name only.
† a union expected type accepts the value when some member does.
‡ a union is accepted when every member matches the expected type, or when any
member matches a union member. A union containing `none` is `Unknown`, not
`Union`.

## Where annotations are enforced

* the initializer of an annotated `let` and top-level constant;
* a `return` expression against the function's declared return type;
* a struct field value at construction;
* an enum payload value at construction;
* a plain reassignment to an annotated binding;
* arguments of a call the checker can resolve to a specific top-level function.

In every case, `Unknown` disables the rule.

## Type aliases

`type Name = T` is a transparent alias — it creates no distinct nominal type and
is resolved transitively through compound positions. A recursive alias is
`E3002`.
