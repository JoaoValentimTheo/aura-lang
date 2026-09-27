# Aura Generics — the parametric model

This document is the **architecture decision and reference** for Aura's generic
type system. It states the syntax, the semantic rules, and the runtime model,
and records the reasoning. The normative rules are mirrored in
[`LANGUAGE_SPEC.md`](LANGUAGE_SPEC.md); the grammar is in
[`grammar.md`](grammar.md). If this document and `LANGUAGE_SPEC.md` disagree,
`LANGUAGE_SPEC.md` is authoritative.

The governing question is *"what is the smallest, clearest, most powerful
generic type system that fits Aura's existing language model?"* — not *"how do
we reproduce Rust's generics?"* **Aura → Zen; the Rust compiler → Poder.**

---

## 1. Shape of the model

Aura's generics are **static, erased, and nominal**.

* **Static** — a type parameter is a compile-time placeholder. It never exists
  as a runtime value; it is substituted before execution.
* **Erased** — the runtime is already dynamically typed (`Value::ty()` is
  element-agnostic), so a generic function is *the same* closure as a
  monomorphic one. There is no monomorphization pass and no runtime dictionary.
* **Nominal** — a generic struct is still a nominal constructor. `Box<int>` and
  `Box<string>` are the same nominal `Box`, differing only in their type
  argument. There is no structural subtyping.

Aura has no subtype relation and no dynamic dispatch, so parametric
polymorphism composes with overloading, traits, modules, aliases, and unions
without reconciling any of them against a second dispatch mechanism.

### 1.1 Collections are structural, not nominal

Aura's collection types are:

```aura
[T]              (* list  *)
{string: T}      (* map   *)
```

There is **no** `List<T>` or `Map<K, V>`. Generic programming composes with the
existing structural collection syntax:

```aura
fn first<T>(xs: [T]) -> T {
    return xs[0]
}

fn get<T>(m: {string: T}, key: string) -> T {
    return m[key]
}
```

The map key stays `string`, exactly as the non-generic model already requires.
A future `{K: V}` map is a separate language decision and is *not* introduced
by generics.

---

## 2. Syntax

### 2.1 Declarations

```text
type_params     = "<" type_param { "," type_param } ">" ;
type_param      = IDENT [ ":" IDENT { "+" IDENT } ] ;

fn_decl         = "fn" IDENT [ type_params ] "(" [ params ] ")" [ "->" type ] block ;
struct_decl     = "struct" IDENT [ type_params ] "{" [ field { "," field } [ "," ] ] "}" ;
trait_decl      = "trait" IDENT [ type_params ] "{" { NEWLINE | trait_method } "}" ;
impl_decl       = "impl" [ type_params ] type_name "{" ... "}"
                | "impl" [ type_params ] trait_name "for" type_name "{" ... "}" ;
```

A type parameter is written in angle brackets directly after the declared name.
A bound is written `T: Trait`, or `T: A + B` for several (`+` binds a
parameter to an intersection of static contracts). `fn identity<T>(x: T) -> T`
is the canonical form.

`<` and `>` are **not** new tokens: generic brackets are recognized
**contextually** in declaration and type positions only, exactly as `impl`,
`trait`, `module`, and `const` are contextual. This preserves every existing
program: `a < b` and `a << b` in expression position keep their meaning.

### 2.2 Type positions and applications

```text
type_member     = ... | IDENT [ "<" type { "," type } ">" ] ;
```

A parameterised type is written `Name<T1, T2>`. In a type position, `<` after a
capitalized (or `::`-qualified) name is a generic application. `[T]` and
`{string: T}` take the inner type as before.

### 2.3 Explicit type arguments

At a call site, type arguments may be given explicitly:

```aura
identity<int>(1)
Box<int> { value: 1 }
xs.map<int>(f)
```

They are **inferred when omitted** and **optional** when inference succeeds.
They are required only when nothing in the argument types determines the
parameter (e.g. `empty<T>()`). The parser recognizes `IDENT "<" ... ">" "("`
and `IDENT "<" ... ">" "{"` as a call/construct with explicit type arguments;
every other use of `<` stays comparison.

### 2.4 Generic parameters are identified by position, not name

```aura
fn f<T>(x: T) -> T
fn f<U>(x: U) -> U
```

These have the **same** overload identity. Parameter names are not part of a
generic signature. Overload identity is computed **up to alpha-renaming** of
type parameters.

---

## 3. Type parameters

* **Where declared** — on a function, struct, trait, or `impl` block, in the
  `[]`-analogue bracketed list immediately after the name.
* **Scope** — the declaration's whole signature and body: parameter and return
  annotations, field types, method receivers, and the body of a function or
  method. A type parameter is visible in nested `impl` methods whose enclosing
  `impl` declares it.
* **Shadowing** — a type parameter may **not** shadow a type already declared
  in the program, and may **not** be declared twice in the same list
  (`E2007`). Shadowing an *outer* type parameter is likewise rejected, so a
  generic `impl<T> Box<T>` method cannot introduce a second `T`; it reuses the
  enclosing one.
* **Namespace** — type parameters live in the type namespace, alongside
  structs and enums, and never in the value namespace. A parameter and a value
  binding may share a spelling.
* **Identity** — a parameter is a placeholder identified by its declaration
  position, not by its spelling. Two occurrences of `T` in one declaration are
  the same parameter; the parameter is distinct from a program type that
  happens to be spelled `T` (which is why shadowing is rejected).

---

## 4. Inference and substitution

Call checking builds a **substitution** `σ : param → Ty` by structurally
matching each argument's inferred type against the parameter's declared type.
Matching is one-way (declared pattern against actual type) and deterministic:

* `T` matches any actual type, binding `σ(T)` to it; a second occurrence must
  bind it to a compatible type or the call is rejected.
* `[T]` matches `[A]` by matching `T` against `A`; `{string: T}` matches
  `{string: A}` likewise.
* `Name<T>` matches `Name<A>` (same nominal name, same arity) by matching
  element-wise.
* A concrete declared type matches only a compatible actual type.
* An unannotated parameter contributes no binding.

The inferred return type is the declared return type with `σ` applied. An
unbound parameter in the return type is `Unknown` (never an arbitrary concrete
type). Explicit type arguments, when written, seed `σ` before matching and are
checked for arity and bounds.

`Unknown` is compatible with everything and **never** silently becomes a
concrete type: a call with `Unknown` arguments leaves parameters unbound rather
than guessing.

---

## 5. Bounds

A bound `T: Trait` requires that every type bound to `T` at an instantiation
implements `Trait`. Bounds are checked **at the call site** (and inside a
generic body, against the bounds in scope). A bound is a static contract: it
grants the body the right to call the trait's methods on a value of type `T`.
There are no trait objects and no dynamic dispatch; a bound never changes the
runtime representation.

`T: A + B` requires both. Bounds are checked against the `impl Trait for
Struct` declarations already in the program; a concrete struct satisfies a
bound exactly when it has that implementation. A parameter bound by a trait
may call that trait's methods on `self`-typed values inside the body.

---

## 6. Generic structs

A generic struct is a nominal constructor with ordered typed fields:

```aura
struct Box<T> { value: T }

fn unbox<T>(b: Box<T>) -> T {
    return b.value
}
```

* Construction — `Box<int> { value: 1 }` (explicit) or `Box { value: 1 }`
  (inferred from the field values).
* Field access — `b.value` has the field type with the struct's substitution
  applied. The checker records the receiver's `Ty::App` arguments and
  substitutes them into the field type.
* The struct remains nominal: `Box<int>` and `Box<string>` are the same `Box`.

---

## 7. Generic methods

Methods reuse the existing method model. A method may declare its own
parameters, or use the enclosing `impl`'s:

```aura
impl Box {
    fn map<U>(self, f: fn(T) -> U) -> Box<U> { ... }
}

impl<T> Box<T> {
    fn get(self) -> T { return self.value }
}
```

* A method's type parameters extend the enclosing `impl`'s (they may not
  redeclare them).
* Overload identity for methods is computed up to alpha-renaming of the
  method's own and the receiver's parameters.
* `mut self` is **not** an overload dimension (unchanged).

---

## 8. Generic traits

A trait may be parameterised:

```aura
trait Container<T> {
    fn get(self, i: int) -> T
}

struct Stack<T> { items: [T] }
impl<T> Container<T> for Stack<T> {
    fn get(self, i: int) -> T { return self.items[i] }
}
```

A trait stays a static contract: no trait objects, no dynamic dispatch, no
associated types or constants, no default or supertrait methods. A bound
`U: Container<T>` is satisfied by the matching `impl`.

---

## 9. Aliases, unions, modules, visibility, closures

* **Aliases** stay transparent. `type IntBox = Box<int>` and
  `type Pair<T> = [T]` (a parameterised alias) both expand at use, preserving
  Aura's context-free alias model. A generic alias is expanded by substituting
  its arguments into its target.
* **Unions** compose: `T | none` is `Unknown` exactly as today, and a union
  member may be a parameterised type. No exponential expansion is introduced;
  the existing canonicalisation and alias memoisation are preserved.
* **Modules / visibility** are unchanged. A generic declaration is exported,
  imported, and reached by the same rules as any other; substitution happens
  after resolution and never bypasses visibility.
* **Closures** capture the type parameters of their enclosing generic
  declaration lexically; a lambda inside a generic function sees the same
  substitution the body does.

---

## 10. Overloading integration

* A generic signature is a normal candidate in the one shared resolver
  (`types::resolve_overload`). No second resolution path exists.
* **Identity** is the name plus the ordered *parameter types*, computed up to
  alpha-renaming of type parameters. Two declarations that differ only in the
  spelling of their parameters are the same overload (`E2007`).
* A generic parameter is *less specific* than a concrete type, so a concrete
  overload wins over a generic one that also matches. Specificity never falls
  back to declaration order.
* **The return type never selects an overload**, generic or not.
* A tie between equally specific viable candidates is `Ambiguous` (an error),
  never a silent pick.
* `Unknown` arguments leave candidates tied and are reported ambiguous rather
  than guessed.

---

## 11. Runtime model — erasure

Type parameters are erased before execution.

* A generic function is stored as an ordinary closure; its `param_tys` maps a
  `Ty::Param` to `None` (the existing "unannotated accepts anything" tier), so
  the runtime's `resolve_overload` stays permissive and never inspects a
  parameter as a value.
* No runtime dictionary, box, or monomorphized copy exists. `Value` is
  unchanged; `Instance` still carries only its nominal type name.
* Because the runtime is already dynamically typed, erasure is sound and
  deterministic. It stays Native/WASM compatible and resource-bounded.

---

## 12. Errors

| Code | Situation |
|---|---|
| `E2007` | duplicate or shadowing type parameter; alpha-equivalent redeclaration |
| `E3001` | type mismatch (bad explicit argument, unsolved bound, arity) |
| `E3002` | unknown type, or wrong generic arity on a name |
| `E1015` | nesting limit (unchanged) |
| `E2009` | an unused type parameter is **not** an error |

Diagnostics name the parameter, the expected arity, and the offending
position, and are deterministic.

---

## 13. REPL

A generic declaration is persisted with its type parameters and bounds and
rebuilt into later checkers exactly as a non-generic one. A failed generic
declaration does not mutate session state: declarations are persisted only
after the checker accepts them, exactly as today.

---

## 14. Deliberately absent

Not introduced by generics, and not planned here: higher-kinded types,
associated types or constants, trait objects / dynamic dispatch, default or
supertrait methods, const generics, variance annotations, specialization,
GADTs, and higher-rank polymorphism. Each would be a separate decision with
its own justification.
