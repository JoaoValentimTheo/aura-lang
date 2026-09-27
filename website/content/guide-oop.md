# The object model

Aura has **one** object model, and it is small on purpose. There is no `class`.
A struct is data, a method is a function bound to a nominal type, and a trait is
a static contract. This page maps the four classical OOP pillars onto Aura and
states what is deliberately absent. The full architecture decision is
`docs/OOP.md`; the normative rules are in the language specification (§15.7,
§17, §26, §27).

## Encapsulation

A struct is a nominal record type with typed fields. Behavior is attached with
an `impl` block. Two rules keep the member surface unambiguous:

* A field and a method of the same struct may not share a name (`E2016`).
* Member lookup is deterministic: `p.x` is a field read, `p.m()` is a method
  call, and an unknown member is `E2003` — there is no fallback to a built-in
  or another struct.

Mutation is a **separate** dimension from member lookup. A writing operation —
a field or index assignment, a mutating built-in, or a `mut self` method —
requires the root binding to be reachable through `mut`:

```aura
struct Point { x: int }

fn main() {
    let p = Point { x: 0 }
    p.x = 1              # E2001: p is immutable (declare it `let mut`)
    let mut q = Point { x: 0 }
    q.x = 1              # ok
    print(q.x)           # 1
}
```

The same rule governs methods: a mutating method declares `mut self`, and its
caller must reach the receiver through a `mut` binding.

On top of that, `module Name { ... }` gives Aura a **real access boundary**:
every item, field, and method is private to its module by default and exported
with `pub`. Modules nest, and `use path [as Alias]` imports a name.

```aura
module counter {
    pub struct Counter { n: int }            # field `n` is private
    pub fn make() -> Counter { return Counter { n: 0 } }
    impl Counter {
        pub fn get(self) -> int { return self.n }
    }
}

fn main() {
    let c = counter::make()
    print(c.get())                           # 0
    print(c.n)                               # E2018: field `n` is private
}
```

A private access is `E2018`; an unknown module or import is `E2019`. The
boundary is in-source — the WebAssembly host has no filesystem — which keeps
Native and Web semantics identical. Visibility is independent of mutation:
`pub` never grants mutable capability, and a `mut` binding never bypasses
visibility.

## Abstraction

A trait names a behavioral contract: a set of method signatures a struct
agrees to implement. A trait has signatures only — no bodies, fields, or
associated items — and introduces no value type.

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

fn main() {
    let u = User { name: "Ada", id: 1 }
    u.print()
}
```

`impl Trait for Struct` must provide exactly the trait's methods, once each,
with a compatible signature (`E2017` if missing, `E3001` if not, and no
extras). Receiver mutability is part of the contract.

## Reuse: composition, not inheritance

**Aura does not implement inheritance.** There are no classes, no struct
inheritance, no subtype relation, no implicit upcasting, and no dynamic
dispatch. Reuse is composition plus traits:

```aura
struct Engine { power: int }
impl Engine { fn describe(self) -> string { return to_string(self.power) } }

struct Car { engine: Engine }
impl Car { fn describe(self) -> string { return self.engine.describe() } }
```

A method reaches a nested struct's method through a field. There is no hidden
base layout, no field copying, and no override. The practical purposes of
inheritance are covered by:

* **Shared state** → composition: `struct Car { engine: Engine }`.
* **Shared behavior** → a free function, a method on the composed type, or a
  trait-provided method.
* **A contract several types satisfy** → a trait: `impl Printable for User`.
* **Substitutability** → a union (`A | B`) with member-wise method resolution,
  or a trait both types implement.

## Polymorphism

Aura exposes exactly two forms:

* **Overloading (ad-hoc).** One name may have several declarations when their
  ordered input types differ. A call resolves by argument types, most specific
  first; the return type never distinguishes overloads and a tie is `E3001`.

  ```aura
  fn f(x: int) { print("int") }
  fn f(x: string) { print("string") }
  fn f(x) { print("any") }
  ```

* **Static union resolution.** A union receiver exposes a member only when
  **every** member provides it with a compatible signature; otherwise the call
  is rejected statically.

Aura deliberately has **no subtype polymorphism and no dynamic dispatch**, and
**no parametric polymorphism** yet — generics are the next phase.

## Rules at a glance

* A struct may have several `impl` blocks; they merge into one method surface.
* Trait and inherent methods share one namespace. Two methods with the same
  overload identity (name plus ordered parameter types after the receiver)
  collide (`E2007`); the same name with different ordered parameter types are
  distinct overloads.
* A trait's methods are single signatures and cannot be overloaded within one
  trait implementation.
* `mut self` is never an overload dimension and never bypasses the
  mutation-capability rule.
* Aliases are transparent: an alias denotes the same nominal struct and keeps
  its methods and trait membership.

## What Aura does not have

Classes; inheritance; subtyping; implicit upcasting; dynamic dispatch; trait
objects; default methods; supertraits; associated items; generics and trait
bounds; filesystem-backed modules and finer visibility levels than
`pub`/private; constructors and destructors;
operator overloading; reflection; metaclasses. Each absence is a decision, not
an oversight — see `docs/OOP.md` and the specification §35.
