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
`E2005`. An ordinary `let`/`let mut` may shadow an existing binding (`let x = 1` then `let x = x + 1`); `const`, functions, parameters, and types do not shadow, and a duplicate is `E2007`.

**Mutation requires `mut`.** Any operation that changes state reached through a
binding needs that binding to be `mut`:

```aura
let xs = [1, 2]
xs.push(3)         # E2001: xs is immutable
xs[0] = 9          # E2001

let mut ys = [1, 2]
ys.push(3)         # ok
ys[0] = 9          # ok
```

The same rule covers map entries (`m["k"] = v`), struct fields (`s.f = v`),
the mutating builtins (`push`, `pop`, `remove`), and any method declared
`mut self`. A pure operation (`sort`, `reverse`, `map`, `len`, …) needs no
`mut`. Capability belongs to each binding: a mutable binding and an immutable
alias of the same value are distinct, so mutating through the mutable one is
allowed while the alias needs its own `mut`.

### Shadowing

An ordinary `let`/`let mut` always creates a **new** binding, and may shadow an
existing one — even in the same scope. The initializer is evaluated against the
bindings visible before the declaration, so it reads the binding being
shadowed:

```aura
let x = 11
let x = x + 10     # the initializer reads the first x → x is now 21

let mut y = 1
let y = 20         # y is now an immutable binding
```

`mut` belongs to the new binding, so `let mut x = 1; let x = 2; x = 3` is
`E2001`. Shadowing changes *names*, not *bindings*: a closure that captured an
earlier binding keeps seeing it:

```aura
let x = 10
let f = () -> x
let x = 20
f()                # 10
x                  # 20
```

A nested scope may **shadow** an outer binding; the outer binding becomes
visible again when the nested scope ends. A loop variable, match binding, and
catch binding are scoped to their construct and never leak out.

`const`, functions, parameters, and types are **not** shadowable: a duplicate
is `E2007`/`E2012`. In particular `const X = 1; const X = 2` is `E2007`.

### Constants

```aura
const PI = 3.14159         # canonical module constant
const LIMIT: int = 100     # with a checked annotation
```

A `const` name is uppercase; a lowercase name is `E1006`. A constant is
immutable and is evaluated in source order after all declarations, so it may
call a function or read an earlier constant but not a later one. A top-level
`let` is the same kind of module constant. Like any immutable binding, a
constant cannot be mutated through: `push(CONST_LIST, x)` is `E2001`, while its
interior is still shared with any mutable alias.

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
