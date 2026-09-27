# Structs and enums

## Structs

A `struct` declares a nominal record type with typed fields.

```aura
struct Point { x: int, y: int }

fn main() {
    let p = Point { x: 0, y: 0 }
    print(p.x)          # 0
    let mut q = Point { x: 0, y: 0 }
    q.x = 1             # fields are mutable through a `mut` binding
    print(q.x)          # 1
}
```

Construction uses `Name { field: value }`. Fields may also be supplied by name
in any order. A missing field, an unknown field, or a wrongly typed field is
`E3001`. Struct equality is nominal (same type name) and then structural.

## Methods

Behavior is attached to a struct with an `impl` block. A method's first
parameter is the explicit receiver `self`; it is an ordinary binding, immutable
unless declared `mut self`. A method that assigns `self` or a field of `self`
MUST declare `mut self`, and the caller must reach the receiver through a `mut`
binding — the same mutation-capability rule as everywhere else.

```aura
struct Counter { n: int }

impl Counter {
    fn bump(mut self, by: int) {
        self.n = self.n + by
    }
    fn get(self) -> int { return self.n }
}

fn main() {
    let mut c = Counter { n: 0 }
    c.bump(3)          # the receiver must be reachable through a `mut` binding
    print(c.get())     # 3
}
```

Rules:

* Method names are scoped to the struct, so two structs may each declare
  `fn get(self)`. Multiple `impl` blocks for one struct merge into one method
  surface and may add overloads.
* `s.field` reads a field; `s.method(args)` calls a method. A method requires
  parentheses — `s.method` without them is not a bound method (`E2003`).
* A field and a method of the same struct may not share a name (`E2016`).
* An unknown member on a known struct is `E2003`; there is no fallback to a
  built-in method or another struct.
* There is no inheritance, no constructor, and no visibility: reuse is
  composition plus methods and free functions.
* `impl` and `self` are **contextual**, not reserved: `impl Struct { … }` is a
  behavior block only at item position, and `self` is the receiver only as a
  method's first parameter. Elsewhere both are ordinary identifiers
  (`let impl = 1`, `fn self(x)`, a field named `impl`) and behave as before.

## Traits

A `trait` names a behavioral contract: a set of method signatures a struct
agrees to provide. A trait has signatures only — no bodies, fields, or
associated items — and introduces no value type. `impl Trait for Struct` must
implement exactly the trait's methods.

```aura
trait Printable {
    fn print(self)
    fn label(self) -> string
}

struct User { name: string, id: int }

impl Printable for User {
    fn print(self) { print(self.name) }
    fn label(self) -> string { return self.name + "#" + to_string(self.id) }
}

let u = User { name: "Ada", id: 1 }
u.print()
```

Rules:

* A trait implementation must provide every declared method (`E2017` if one is
  missing) with a compatible signature (`E3001` if not), and no extras.
* Traits add no dispatch: calls resolve statically by the receiver's nominal
  type. There are no trait objects, vtables, or bounds.
* Trait and inherent methods share one namespace: a name provided twice is
  `E2007`.
* `trait` is contextual (`let trait = 1` is valid), like `impl` and `self`.

## Enums

An `enum` declares a tagged sum type. Each variant may carry a positional
payload.

```aura
enum Shape {
    Circle(int),
    Rectangle(int, int),
    Empty,
}
```

Construction is by variant name:

```aura
let circle = Circle(2)
let empty = Empty()
```

Enum variants are positional; named payload arguments are not allowed. An
unknown constructor is `E3002`.

## Pattern matching over data

```aura
fn area(s) -> int {
    return match s {
        Circle(r) -> 3 * r * r
        Rectangle(w, h) -> w * h
        Empty -> 0
    }
}
```

See [Pattern matching](/docs/guide-matching/) for the full pattern grammar.

## Type aliases

`type Name = T` declares a transparent alias. It is validated (the target must
exist) but creates no distinct nominal type: `type Id = int` and `int` are the
same type.

```aura
type Id = int
type Maybe = int | none

fn f(id: Id) -> Id { return id }
```

Aliases may be chained and are resolved transitively. A recursive alias is
rejected with `E3002`.

## `use` and `pub`

`use` and `pub` are **parsed and reserved but inert** in this version. They
exist so that future module and visibility semantics can be introduced without a
syntax break. Using them is not an error; they do nothing.
