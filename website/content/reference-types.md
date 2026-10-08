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
| `[T; N]` | fixed-length array of `N` elements of type `T` |
| `(T, U, ...)` | tuple of the given member types |
| `{T}` | set of `T` |
| `{K: V}` | map from key type `K` to value type `V` |
| `Named(name)` | a user struct or alias-resolved type |
| `Name<T, ...>` | a generic type applied to type arguments |
| `Enum(name)` | an enum type |
| `Unknown` | not determined |

List, Array, Tuple, Set, and Map are five distinct identities. The array length
is part of the type, so `[int; 2]` and `[int; 3]` are different types, and the
bracket literal `[a, b]` is a list unless the expected type is `[T; N]`. See the
[Collections](/docs/guide-collections/) guide and specification §21.

A map key must be **key-capable**: `string`, `int`, or `bool`, or a union every
member of which is key-capable. A set element has the same requirement. A map
annotation with any other key (for
example `float`, `none`, `[int]`, or another map) is rejected (`E3001`). A
generic parameter is a valid key in a declaration and must be key-capable when
instantiated, so `type Map<K, V> = {K: V}` is well-formed while `Map<float,
int>` is rejected.

## Generic types

A declaration may take **type parameters** (`fn identity<T>(x: T) -> T`,
`struct Box<T> { value: T }`). A type parameter is a static placeholder
substituted before execution; it is never a runtime value. Type arguments are
written `Name<T>` and inferred at construction or call sites when omitted. A
parameter may carry a **bound** (`T: Trait`), which is a static contract
checked where `T` is instantiated. See the [Generics](/docs/guide-generics/) guide
and specification §36.

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
same primitive, both are lists with compatible element types, both are arrays
with the same length and compatible element types, both are tuples with the
same arity and element-wise compatible members, both are sets with a compatible
element type, both are maps
with compatible key **and** value types, both are the same nominal type, or a
union member matches. A list and an array are never compatible, even when the
contents would fit; there is no implicit conversion. `int` and `float` are
**not**
compatible in annotations — there is no implicit numeric coercion in
annotations, even though mixed arithmetic promotes at runtime. (A union such as
`int | float` accepts either, because each member is checked separately.)

| Expected ↓ / Actual → | int | float | bool | string | list | array | tuple | set | map | Named | Enum | Union | Unknown |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `int` | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓† | ✓ |
| `float` | ✗ | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓† | ✓ |
| `bool` | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓ |
| `string` | ✗ | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓ |
| `[T]` | ✗ | ✗ | ✗ | ✗ | ✓* | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓ |
| `[T; N]` | ✗ | ✗ | ✗ | ✗ | ✗ | ✓* | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓ |
| `(T, ...)` | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓* | ✗ | ✗ | ✗ | ✗ | ✗ | ✓ |
| `{T}` | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓* | ✗ | ✗ | ✗ | ✗ | ✓ |
| `{K: V}` | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓* | ✗ | ✗ | ✗ | ✓ |
| `Named(n)` | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓** | ✗ | ✗ | ✓ |
| `Enum(n)` | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓** | ✗ | ✓ |
| `Union` | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓‡ | ✓ |
| `Unknown` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

\* element/value/member types must themselves be compatible; an array also
requires an equal length and a tuple an equal arity.
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
