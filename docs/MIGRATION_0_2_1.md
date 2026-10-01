# Migrating to Aura `0.2.1`

`0.2.1` is the development line after the released `0.2.0` Core language. This
guide lists every change in observable language behavior and what to do about
it. Most programs need no changes.

The version-identity model is recorded in
[`adr/0001-release-vs-language-version.md`](adr/0001-release-vs-language-version.md):
the *release* version identifies a published artifact, the *language* version
identifies the observable language contract, and `language <= release`.

## Summary

| Change | Kind | Who is affected |
|---|---|---|
| Builtin names reserved as value bindings (`E1009`) | intentional break | programs that bound a builtin name |
| Structural type nesting bounded at 256 on every substrate (`E1015`) | tightening | programs with type annotations nested >256 levels |
| Type-alias chains bounded in depth and breadth (`E1015`) | tightening | programs with pathological alias chains |
| Module nesting bounded at 256 on every substrate (`E1015`) | tightening | programs with >256 nested modules (physical or in-source) |
| String lexing is O(n) instead of O(n²) | performance | long string literals |
| Python dict keys require exact types | tightening | Python objects with `__index__` used as keys |

## Builtin names are reserved as value bindings (`E1009`)

A value binding — `let`/`let mut`, a parameter, a loop/catch/pattern binding, a
top-level `let`, a top-level `fn`, or a `use ... as` alias — may no longer reuse
a registered builtin name (for example `len`, `sum`, `print`).

```aura
# 0.2.0: accepted. 0.2.1: E1009.
fn main() { let sum = 1 }
```

**Fix:** rename the binding. Near-misses are fine (`summary` is allowed).

Type, module, field, variant, method, and **module-member** names are separate
namespaces and are unaffected (ADR-0002):

```aura
# Allowed: the member `sum` lives in module `M`'s namespace.
module M { pub fn sum(n: int) -> int { return n + 1 } }
fn main() { print(M::sum(1)) }
```

An import alias is a value binding, so `use M::sum as len` is still `E1009`.

## Type nesting is bounded at 256 on every substrate

Structural nesting of a type annotation — generic application (`Box<…>`), a list
(`[T]`), or a map (`{K: V}`) — now counts toward the semantic AST limit, so a
type nested past 256 levels is `E1015` identically on native and WebAssembly.
A flat union (`A | B | …`) lists alternatives and is not penalized per member.
See [`adr/0004-typeexpr-nesting-policy.md`](adr/0004-typeexpr-nesting-policy.md).

**Fix:** no real program nests types this deeply; this only affects generated or
adversarial input. If you generate types programmatically, cap nesting at 256.

## Type-alias chains are bounded

A chain of aliases that nests deeper than 256 levels, or that expands to more
than a bounded number of nodes, is `E1015` rather than an unbounded
computation. This covers both plain chains (`type A{i} = [A{i-1}]`) and
parameterized ones (`type A{i}<T> = A{i-1}<T>` and grow-the-argument shapes).
A flat, wide set of aliases (thousands of independent aliases), or a flat union
with thousands of members, stays accepted.

**Fix:** none for real programs; this bounds only pathological generation.

## Module nesting is bounded at 256

Any structural `module` nesting — in-source `module A { module B { … } }` or a
physical directory chain of nested modules — is bounded at 256 levels on every
substrate.

## Python dict keys require exact types

At the Python boundary, a dict key converts only when it is a genuine builtin
scalar (`bool`/`int`/`str`). A user object implementing `__index__` is **not**
coerced to an integer key; it is rejected `E5002`. Seed-map code that relied on
duck-typed keys must convert explicitly. See
[`CPYTHON_COMPATIBILITY_TARGET.md`](CPYTHON_COMPATIBILITY_TARGET.md).

## What did *not* change

- Core syntax, modules (`pub`/`pub use`/aliases), generics, the four OOP
  pillars, collections, comprehensions, and the standard library are unchanged.
- Diagnostics keep their codes; wording may have improved.

## Checking compatibility

`tests/compat.rs` pins both the `0.2.0` surface and the `0.2.1` additions
behaviorally. Run it, or the full suite, after upgrading:

```
cargo test --locked --test compat
```
