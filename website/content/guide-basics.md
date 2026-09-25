# Values and bindings

## The value universe

Aura's runtime values are exactly:

| Kind | Description |
|---|---|
| `int` | signed 64-bit integer |
| `float` | IEEE-754 binary64 |
| `bool` | `true` / `false` |
| `string` | immutable UTF-8 text, indexed by Unicode scalar value |
| `none` | absence — there is no `null` |
| `list` | ordered, mutable, reference semantics |
| `map` | string-keyed, ordered by key, mutable, reference semantics |
| `struct` | named fields in declaration order |
| `enum` | a tag with a positional payload |
| `fn` | a closure or native function |
| `range` | `start`..`end`, step 1 |

## Bindings

```aura
let x = 1          # immutable
let mut y = 2      # mutable
y = y + 1
```

Assigning to an immutable binding is `E2001`. A `let` without an initializer is
`E2005`. Redeclaring a name in the same scope is `E2007`.

### Destructuring bindings

```aura
let [a, b] = [1, 2]
let Some(x) = maybe
```

Aura's `let` patterns are names, list patterns, and enum-variant patterns.
Destructuring is atomic: if the pattern does not match, nothing is bound and the
statement fails with `E3001`.

## Truthiness

These values are **falsy**: `false`, `none`, integer `0`, float `0.0`, the empty
string, the empty list, and the empty map. Every other value is truthy.

```aura
if [] { print("never") } else { print("empty is falsy") }
```

## Equality and ordering

Equality is structural for lists and maps, nominal-and-structural for structs,
and tag-plus-payload for enum variants. `==` always yields a `bool`. Ordering
(`<`, `<=`, `>`, `>=`) is defined for numbers, strings, and booleans; comparing
incomparable types is `E3001`. A `NaN` operand makes every ordering comparison
`false`.

## Type annotations

Annotations are optional and are enforced only where the checker can prove a
mismatch. `int` and `float` do not coerce in annotations, and there is no
`null`: absence is the value `none`.

```aura
let n: int = 41
let maybe: int | none = none
```

## Comments

A line comment runs from `#` to the end of the line. A multiline comment is
written `<!-- ... --!>` and may span any number of lines; both kinds of comment
are discarded before parsing. An unterminated multiline comment is `E1005`.
