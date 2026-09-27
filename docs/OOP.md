# Aura's Object Model — the Four Pillars

This document is the **architecture decision and reference** for Aura's object
model. It states how the four classical OOP concepts map onto Aura, which
capabilities are implemented, and which are deliberately absent. The normative
semantic rules live in [`LANGUAGE_SPEC.md`](LANGUAGE_SPEC.md) (§15.7, §17,
§26, §27); this document explains the design and the reasoning. If the two
disagree, `LANGUAGE_SPEC.md` is authoritative.

The governing question is not *“how do we reproduce Java/C++/C#/Python?”* but
*“which capability does each pillar actually provide, and what is the smallest
coherent Aura construct that provides it?”* — **Aura → Zen; the Rust compiler →
Poder.**

---

## 1. The unified model

Aura has one object model, not several overlapping ones:

| Concept | Aura construct | Semantics |
|---|---|---|
| Data | `struct Name { field: T, ... }` | Nominal product type with typed fields in declaration order. |
| Behavior | `impl Name { fn m(self, ...) { ... } }` | A method is an ordinary function whose first parameter is the explicit receiver `self`. |
| Contract | `trait Name { fn m(self) ... }` | A named set of method signatures; no bodies, fields, or associated items. |
| Implementation | `impl Trait for Name { ... }` | Provides exactly the trait's methods, once each, with compatible signatures. |
| Reuse | Composition (`struct A { b: B }`) + traits + free functions | A method reaches a nested struct's method through `self.field.method()`. |
| Selection | Method/function **overloading** | One name, several declarations; the call resolves by argument types. |

There is no `class`. A struct is data; a method is a function bound to a
nominal type; a trait is a static contract. Inherent methods (`impl S`) and
trait-provided methods (`impl T for S`) share **one member namespace** per
struct. `impl` and `self` and `trait` are **contextual** words, not reserved
ones.

---

## 2. Pillar 1 — Encapsulation

**Classical concept.** Bundle state with the operations on it and restrict
external access to implementation details.

**Aura interpretation.** Aura's encapsulation is *coherence*, not
*access control*. A struct's state is its typed fields; its operations are its
`impl` methods; the two are kept unambiguous by one rule — **a field and a
method of the same struct may not share a name** (`E2016`). Member lookup on a
statically known struct is total and deterministic: `r.name` is a field read,
`r.name(args)` is a method call, and an unknown member is `E2003` with no
fallback to a built-in or another struct.

Mutation is a separate, orthogonal dimension. Reading through a binding is
free; writing — a field assignment, an index assignment, a mutating built-in,
or a `mut self` method — requires the root binding to be reachable through
`mut` (`E2001`). This is **mutation capability**, not visibility.

**Implemented.** Nominal structs; typed fields validated at construction;
deterministic member lookup; field/method name-collision rejection;
mutation-capability enforcement.

**Deliberately absent.** Modules; `pub`; `use`; private/default visibility;
private fields; private methods; module boundaries. `pub` and `use` are
**parsed, reserved, and semantically inert** (§27): they exist so a future
module system can be added without a syntax break, and they carry no visibility
meaning today. There is no scope in which a member is hidden, because there is
no module boundary that would define "inside" and "outside".

**Reason.** Visibility is meaningful only relative to a module boundary, and
Aura has deliberately deferred modules to a separate design phase (they are a
large surface: file boundaries, name resolution across modules, cyclic imports,
and their REPL semantics). Adding field-level privacy *without* modules would
introduce a second, weaker notion of "scope" that conflicts with the deferred
decision and touches construction, field access, the checker, the runtime, the
REPL, and every existing example. The smallest coherent choice is to keep
encapsulation structural and defer visibility to the module phase — formalized
here as a decision, not left ambiguous.

**Invariant.** Visibility, mutability, and method resolution are **separate
semantic dimensions**. No visibility rule may grant mutation capability, and a
mutable binding may not bypass a visibility rule. (With visibility deferred,
only the mutability dimension is live; the separation is preserved for when it
is added.)

---

## 3. Pillar 2 — Abstraction

**Classical concept.** Describe *what* an operation does, not *how*, so clients
depend on a contract rather than a concrete implementation.

**Aura interpretation.** Traits are Aura's static behavioral abstraction:

* A `trait` declares method signatures only — no bodies (no defaults), no
  fields, no associated types or constants, and no `self`-less associated
  functions. Each declared method has `self` as its first parameter.
* A trait introduces **no value type** and **no dispatch mechanism**; it is not
  a `Ty`.
* `impl Trait for Struct` must implement every declared method exactly once
  with a compatible signature, and no extras. Missing is `E2017`; an
  incompatible signature is `E3001`; an extra method is `E2003`.
* Receiver mutability is part of the contract: a trait declaring `mut self`
  requires `mut self` in the implementation, and vice versa (`E3001`).
* Trait and inherent methods share one namespace: a name provided twice with
  the same identity is `E2007`.

**Implemented.** The full static trait model above, including multiple
implementations of one trait across different structs, composition through
traits, and interaction with overloading (§6).

**Deliberately absent.** Abstract classes; default methods; supertraits;
associated types/constants; trait objects; trait bounds; generic abstraction
`fn show<T: Printable>(x: T)`. Bounds and generic abstraction require the
Generics phase.

**Reason.** Traits already provide the reusable capability — a named contract
several concrete types can satisfy — without introducing a second object model
or a value type that needs runtime representation. Generic abstraction over a
trait is the one capability that genuinely needs Generics and is deferred to
that phase rather than half-built here.

---

## 4. Pillar 3 — Inheritance / Reuse

### Architecture decision (mandatory statement)

> **Aura does not implement inheritance.** There are no classes, no struct
> inheritance, no subtype inheritance, no implicit upcasting, and no dynamic
> dispatch. Code and state reuse are expressed with **composition + traits +
> method/function overloading + free functions**.

This is a deliberate decision, re-evaluated for the OOP-completion phase, not
an omission. The study below is why it stands.

**What inheritance is usually used for, and the Aura replacement.**

| Purpose of inheritance | Aura replacement |
|---|---|
| Share *fields/state* between related types | **Composition**: `struct Car { engine: Engine }`; reach it with `self.engine.…`. State is where it is declared; there is no hidden base layout. |
| Share *behavior* | A free function, a method on the composed type, or a trait-provided method. |
| Define a *contract* several types satisfy | **Traits**: `impl Printable for User`. |
| Provide *substitutability* (use a subtype where a supertype is expected) | A **union** type (`A | B`) with member-wise method resolution, or a **trait** that both types implement. |
| Select behavior by *runtime type* | Not provided. Aura resolves statically by the receiver's nominal type; a union call requires **every** member to provide the method compatibly. |
| Override a base method | Not provided. Give the new type its own method, or compose. |

**Why not inheritance.** Inheritance entangles three things Aura keeps
separate: **state layout**, **type identity**, and **behavior lookup**. It
introduces field inheritance, override semantics, constructor/lifecycle
questions, diamonds and cycles, and implicit upcasting — a large surface whose
useful part is composition + traits. Adding it would also force decisions the
core has not made (Aura has no constructors, no ownership, no dynamic
dispatch), and would conflict with the future Generics model, which must build
on nominal structs and static traits, not on subtyping.

**Composition + traits is not a silent inheritance substitute.** There is no
mechanism that copies a base's fields into a derived type, no implicit widening
of one struct to another, and no dispatch that prefers a "derived" method.
A union receiver requires *every* member to provide the member with a
compatible signature; it never upcasts. Two distinct nominal structs are always
distinct types, even when structurally identical.

**Consequences checked for this phase.** Type identity (nominal, unchanged),
unions (member-wise, unchanged), aliases (transparent to the same nominal
struct, unchanged), traits (static contracts, unchanged), overload resolution
(by argument types, unchanged), mutation capability (unchanged), runtime
representation (`Rc`-shared tagged values, unchanged), WASM (identical
semantics), and future Generics (no subtype relation to reconcile).

---

## 5. Pillar 4 — Polymorphism

Aura supports **exactly two** forms, and deliberately no others:

1. **Ad-hoc polymorphism — method and function overloading.** One name may
   have several declarations when their **ordered input types** differ. A call
   selects the most specific viable declaration by argument types. This works
   identically for free functions and for methods (after the receiver).

2. **Static (nominal) polymorphism — traits and union receivers.**
   * Traits let the *same call syntax* be checked against different nominal
     types, each of which implements the contract statically.
   * A union receiver `A | B` exposes a member method only when **every**
     member provides it with a compatible signature; the call is accepted or
     rejected statically.

**Deliberately absent.**
* **Subtype polymorphism / dynamic dispatch** — there is no vtable, no trait
  object, no runtime type inspection, no base/derived relationship.
* **Parametric polymorphism** — generics and trait bounds are the next phase.

**Reason.** Ad-hoc and static polymorphism are exactly the expressive power the
current model needs, and both are fully determined before execution, so the
checker and the runtime cannot disagree and the result is deterministic across
native and WASM. Dynamic dispatch would add a runtime lookup, a second
resolution mechanism, and a representation decision, to satisfy terminology the
language does not need.

---

## 6. Overloading + OOP (compatibility)

Overloading is **one mechanism**, shared by free functions and methods, and it
integrates with the OOP model as follows:

* **Identity** = declaration name + **ordered input types** (for a method, the
  parameter types after the receiver, within the nominal struct). The **return
  type**, **parameter names**, the **method body**, and **`mut self`** are
  never part of identity.
* Multiple `impl` blocks for one struct **merge** into one overload set;
  a later block may add an overload, and a duplicate identity is `E2007`.
* **Trait interaction.** A trait method is a *single signature*, not an
  overload set. A trait implementation implements exactly the trait's methods,
  once each, with a compatible signature — it may not add a second overload of
  a trait method (`E2007`) or an extra method (`E2003`). An **inherent**
  overload of the same name with a *different* ordered-parameter-type identity
  may coexist, because inherent and trait methods share one namespace but only
  identical identities collide (`E2007`).
* **Resolution** is deterministic: viability by arity + compatible annotated
  parameters, then the most specific (exact beats union/`Unknown` beats
  unannotated). A tie is ambiguous (`E3001`); no viable candidate is `E3001`.
  An `Unknown` argument is never guessed into an exact match.
* **Aliases** are transparent: an alias denotes the same nominal struct and
  preserves its methods and trait membership.
* **Unions** resolve member-wise on the receiver and require compatible
  signatures across members.
* **`mut self`** never distinguishes overloads and never bypasses mutation
  capability; a selected `mut self` overload still requires a `mut`-reachable
  receiver.
* **REPL** grows overload sets across submissions with signature-aware
  identity; a duplicate is `E2007` and cannot corrupt session state.

---

## 7. Generics readiness

This phase does **not** implement generics. The finalized model leaves a clean
path for them:

* A **struct** is a nominal type constructor with ordered typed fields;
  parameterisation can extend it without changing its identity rules.
* A **trait** is already a named set of method signatures on `self`;
  trait bounds (`fn f<T: Trait>(x: T)`) can be layered on without a second
  contract mechanism.
* **Overload resolution** is keyed on argument types through one shared
  resolver (`types::resolve_overload`); generic instantiation can refine the
  types it compares.
* **Aliases** are transparent and context-free, so generic aliases are a
  natural extension.
* No **subtype** relation exists to reconcile, and no dynamic dispatch must be
  preserved, so parametric polymorphism can be added statically.

Nothing in the OOP model contradicts a future generic type model; no generic
machinery is introduced early.

---

## 8. What Aura deliberately does not have

* classes, struct inheritance, subtype inheritance, implicit upcasting;
* dynamic dispatch, trait objects, vtables, runtime type inspection;
* default/variadic parameters, constructors, destructors, lifecycle hooks;
* modules, visibility, private members (`pub`/`use` inert);
* generics, trait bounds, associated types/constants, operator overloading;
* reflection, metaclasses, multiple inheritance.

Each absence is a decision, recorded here and in
[`LANGUAGE_SPEC.md`](LANGUAGE_SPEC.md) §35.
