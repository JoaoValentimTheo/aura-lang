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
| Boundary | `module Name { ... }`, `pub`, `use path [as A]`, `path::item` | An in-source visibility boundary: private by default, `pub` to export. |

There is no `class`. A struct is data; a method is a function bound to a
nominal type; a trait is a static contract. Inherent methods (`impl S`) and
trait-provided methods (`impl T for S`) share **one member namespace** per
struct. `impl` and `self` and `trait` are **contextual** words, not reserved
ones.

---

## 2. Pillar 1 — Encapsulation

**Classical concept.** Bundle state with the operations on it and restrict
external access to implementation details.

**Aura interpretation.** Encapsulation has two layers, and Aura now provides
both:

1. **Structural coherence.** A struct's state is its typed fields; its
   operations are its `impl` methods. The two are kept unambiguous by one rule
   — **a field and a method of the same struct may not share a name**
   (`E2016`). Member lookup on a statically known struct is total and
   deterministic: `r.name` is a field read, `r.name(args)` is a method call,
   and an unknown member is `E2003` with no fallback.

2. **A real access boundary — modules.** `module Name { items }` declares an
   in-source module; modules nest, and are reached by `::`-separated paths and
   `use path [as Alias]` imports. Every declaration, field, and method is
   **private to its module by default**; `pub` exports it. A name is reachable
   from module `M` when it is `pub`, or when `M` is the declaring module or a
   descendant. A private access is `E2018`; an unknown module or import target
   is `E2019`.

   An `impl` block has no name, so `pub` on it is rejected (`E1006`). A
   trait-provided method follows its **trait's** visibility. Overload
   visibility is **per overload**.

**Why in-source.** The WebAssembly/Playground host has **no filesystem**. A
boundary that depended on files the guest cannot see could not keep Native and
WebAssembly semantics identical, so Aura's module is declared in source and has
no file of its own. This is the smallest model that is a *real* boundary and
survives every substrate.

**Mutation is a separate dimension.** Reading through a binding is free;
writing — a field assignment, an index assignment, a mutating built-in, or a
`mut self` method — requires the root binding to be reachable through `mut`
(`E2001`). This is **mutation capability**, not visibility.

**Implemented.** Nominal structs; typed fields validated at construction;
deterministic member lookup; field/method name-collision rejection;
mutation-capability enforcement; in-source modules; `pub`; `use` with `as`;
private/public fields, methods, functions, constants, and traits; qualified
`::` paths; and their REPL persistence.

**Deliberately absent.** Filesystem-backed modules; package management;
cross-file imports; `pub(crate)`-style granularity. The module model has one
visibility level (`pub` or private) and one boundary kind (a module), which is
what encapsulation requires and nothing more.

**Invariant.** Visibility, mutability, and method resolution are **separate
semantic dimensions**. No visibility rule may grant mutation capability, and a
mutable binding may not bypass a visibility rule.

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

Generics have since been implemented on this model; see
[`GENERICS.md`](GENERICS.md). The finalized model left a clean path, which the
implementation followed:

* A **struct** is a nominal type constructor with ordered typed fields;
  parameterisation extended it without changing its identity rules.
* A **trait** was already a named set of method signatures on `self`; trait
  bounds (`fn f<T: Trait>(x: T)`) layered on without a second contract
  mechanism.
* **Overload resolution** is keyed on argument types through one shared
  resolver (`types::resolve_overload`); generic instantiation refined the types
  it compares, up to alpha-renaming.
* **Aliases** are transparent and context-free, so generic aliases were a
  natural extension.
* A **module** is a pure visibility boundary over a flat canonical namespace,
  so a generic declaration is exported, imported, and reached by the same
  rules as any other; visibility composes with type parameters without a new
  mechanism.
* No **subtype** relation exists to reconcile, and no dynamic dispatch must be
  preserved, so parametric polymorphism was added statically and erased at
  runtime.

### 7.1 Internal architecture (Rust core)

The object model is expressed through a few explicit internal concepts, each
tied to one semantic invariant:

| Concept | Where | Invariant it makes explicit |
|---|---|---|
| `Resolver` + canonical names | `src/resolve.rs` | A name denotes exactly one declaration; visibility is checked at the boundary. |
| `FnSig.owner` / `FnSig.public` | `src/check/mod.rs` | A function, method, or overload is reachable only where its module allows. |
| `struct_public_fields` | `src/check/mod.rs` | Field privacy is a property of the struct's module, not of a call site. |
| `trait_public` / `trait_owner` | `src/check/mod.rs` | A trait method's reachability follows its trait. |
| `current_module` | `src/check/mod.rs` | A body's access rights are fixed by where it was declared, never by how it is called. |
| `FnSig` overload sets | `src/check/mod.rs` | One method/function namespace, keyed by ordered parameter types. |
| `GlobalDecl` session | `src/repl.rs` | REPL persistence carries the same declarations, not a parallel model. |

These were added because each encodes a semantic invariant the checker and the
resolver must both honour. No abstraction was added for its own sake: the
runtime consumes the same flat, canonical tree and needs no visibility
machinery.

### 7.2 Why the resolution pass is not "a second namespace system"

The resolver does **not** duplicate the checker's name resolution. It computes
one thing the checker cannot: the **canonical name** of each reference, and
whether it crosses a boundary. After that, every existing checker rule
(overload identity, trait matching, member lookup, mutation capability) runs
unchanged on the flat program. There is one overload resolver, one trait
model, and one member table.

---

## 8. What Aura deliberately does not have

* classes, struct inheritance, subtype inheritance, implicit upcasting;
* dynamic dispatch, trait objects, vtables, runtime type inspection;
* default/variadic parameters, constructors, destructors, lifecycle hooks;
* filesystem-backed modules, package management, cross-file imports,
  finer visibility levels than `pub`/private;
* generics, trait bounds, associated types/constants, operator overloading;
* reflection, metaclasses, multiple inheritance.

Each absence is a decision, recorded here and in
[`LANGUAGE_SPEC.md`](LANGUAGE_SPEC.md) §35.
