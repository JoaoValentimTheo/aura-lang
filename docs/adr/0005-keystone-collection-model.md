# ADR-0005 — Keystone collection model (List, Array, Tuple, Set, Map)

- **Status:** Accepted (human Keystone decision, 2026-10-08)
- **Supersedes:** the earlier "no tuple type / tuple is list sugar" Core
  decision (`LANGUAGE_SPEC.md` §21 prior revision) and the open
  `docs/engineering/COLLECTION_MODEL_DECISION_PACKAGE.md` (now implemented).
- **Related:** `LANGUAGE_SPEC.md` §5.1/§5.4/§21, `docs/grammar.md`
  (`array_type`, `tuple_type`, `set_type`), `src/types.rs` (`Ty`),
  `src/ast/mod.rs` (`TypeExpr`, `Expr`, `Pattern`), `src/parse/mod.rs`,
  `src/check/mod.rs` (`check_array_literal`), `src/run/value.rs`,
  `src/run/mod.rs` (`eval_expected`), `src/run/iterative.rs`,
  `src/stdlib/ext.rs`, `src/ais.rs`.

## Context

Aura 0.2 collapsed `Tuple` into `List`: `(a, b)` was list sugar and there was
no distinct `Array` or `Set`. The 0.3 "Keystone" human decision reopens the
collection model: the value algebra must represent distinct identities for
`List`, `Array`, `Tuple`, `Set`, and `Map`, because a shared capability must not
erase identity.

Repository evidence (searched exhaustively) contained **no** historical
`Array`/`Set` syntax and no `[N; T]` design. The decision package therefore
posed the blocking Array question; the human answered it in the campaign brief:

- Array type syntax is `[T; N]` — the `;` separates element type and a
  compile-time length.
- The literal remains comma-separated `[a, b]`; the same spelling is a List by
  default and an Array only under an expected `[T; N]` type.
- `[1; 2; 3]` is **not** an Array literal (no semicolon between elements).

## Decision

Adopt five distinct collection identities with the following rules.

1. **List** — `[T]`, family `Sequence`, value kind `list`; resizable, indexable;
   a bracket literal without an Array expectation is a List.
2. **Array** — `[T; N]`, family `Sequence`, value kind `array`; fixed length,
   indexable, element-mutable, **not** resizable. `N` is a compile-time
   non-negative integer literal, bounded (2^24). Length is part of the type;
   there is no length covariance.
3. **Contextual realization** — a bracket literal under an expected `[T; N]`
   realizes as an Array through **one** seam (checker `check_array_literal`;
   runtime `eval_expected` in each engine). It applies at annotated `let`,
   directly resolved function argument, declared `return`, struct field, and
   nested bracket element. It is not a general conversion.
4. **No implicit conversion** — a binding already inferred as a List never
   satisfies an Array expectation, and vice versa.
5. **Tuple** — `(T, ...)`, family `Sequence`, value kind `tuple`; fixed-length,
   heterogeneous, **immutable**, indexable; literal index yields the exact
   member type. `(a,)` is one-element; `(a)` is grouping. Tuple and list
   patterns are not interchangeable.
6. **Set** — `{T}`, family `SetLike`, value kind `set`; unordered membership
   with deterministic order; **not** indexable; elements are key-capable
   scalars (`int`/`bool`/`string`); duplicate literal members collapse;
   duplicate typed-JSON members are `E4031`.
7. **Map** — `{K: V}`, family `Mapping`, value kind `map`; unchanged.
8. **JSON** — dynamic decode yields List for arrays; typed decode restores the
   requested identity (`[T; N]` Array, `(T, ...)` Tuple, `{T}` Set); encode is a
   serialization, not an identity-preserving round-trip.
9. **Anti-collapse** — `List != Array != Tuple != Set != Map` in equality; a
   struct is never equal to a map; AIS reports distinct `type_name`,
   `families`, and `value_kind`.

## Consequences

- The earlier "tuple is list sugar" absorption and every test asserting it are
  removed or migrated.
- The `Tuple` family is `Sequence`; `Set` adds the `SetLike` family. Families
  remain capability projections, never identities.
- `[a, b]` keeps one spelling for List and Array; identity is carried by the
  type system, equality, AIS, and capabilities — not by distinct display.
- A future RFC may broaden Set membership or add const-expression array
  lengths; both are explicitly out of scope for 0.3.
