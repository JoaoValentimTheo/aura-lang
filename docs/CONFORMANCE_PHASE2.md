# AURA LANGUAGE CONFORMANCE PASS — PHASE 2

## 1. Scope

AST invariants, symbol identity, name resolution, module namespaces, imports,
aliases, visibility, lexical scopes, shadowing, forward references, duplicate
detection, resolution diagnostics, and the checker input boundary.

Starting HEAD: `573ec8bc2811b0a4cf1db43e02bf57479132fd6b` (Phase 1 closed).
Layer: `src/parse/mod.rs` → `src/ast/mod.rs` → `src/resolve.rs` →
`src/check/mod.rs`.

## 2. Method

The resolver was reconstructed from `src/resolve.rs` (1,537 lines) and verified
against `docs/LANGUAGE_SPEC.md` §§16, 18, 26–27, 36, `docs/grammar.md`,
`docs/GENERICS.md`, the module design commit `c7551df`, and
`tests/{modules,shadowing,generics,checker,grammar,adversarial}.rs`. Every
candidate was reproduced with the built CLI over stdin and classified before
any code change. Confirmed defects received a minimal fix, a focused
regression, and a full-suite check.

## 3. Resolver Architecture

```text
parse → Module (nested Item tree)
  resolve(module):
    collect_module(items, [])        Pass A: declare every module/type/value/variant
                                     into per-module scopes + flat `items`
    apply_imports(items, [])         Pass B: apply every `use` (whole program known)
    flatten(items, [])               rewrite every free reference to its
                                     canonical name; enforce visibility
  → flat Module (canonical names)
      → check::module / run::Interp   module-agnostic consumers
```

Two global passes mean **source order never matters** for declaration
resolution. The resolver rewrites every free reference to one canonical name,
so the checker and runtime cannot disagree about which declaration a name
denotes. `resolve_with_session`/`resolve_stmt` reuse the same algorithm for the
REPL, seeding prior submissions as canonical declarations and root imports.

## 4. AST Invariant Matrix

`src/ast/mod.rs` node inventory (18 public types; `Span` is a byte range):

| Node | Span? | Mandatory children | Impossible-states / notes | Constructed by |
|---|---|---|---|---|
| `Module` | no (file) | `items` | — | parser |
| `Item` (enum) | per-variant `span` | name, body/fields | `impl` has no name; `pub` is `E1006` on `impl` | `item`/`module_item` |
| `Expr` (enum) | every variant carries `Span` | operands | `Construct` names a struct or variant | `expr`/`postfix` |
| `Stmt` (enum) | `Let`/`LetPattern`/`Assign`/`While`/`Loop`/`For`/`Try` carry `Span` | value/body | `Let` name is `BIND_NAME` | statements |
| `Pattern` (enum) | `Variant`/`List` carry `Span` | bindings | exact-length list; no rest/struct/sign/float | `pattern`/`let_pattern` |
| `TypeExpr` (enum) | **none** | members | no span of its own — a known limitation (§9) | type parser |
| `Param` | yes | name | `mut` grants capability | params |
| `Arg` | no | value, optional name | named args exist for calls | call args |
| `VariantDecl` | yes | tag, payload | tags canonicalized per module | enum parser |
| `FieldDecl` | yes | name, `ty` | `public` for `pub` | struct parser |
| `TypeParam` | yes | name, bounds | bounds static §36.4 | generic list |
| `FPart`/`FormatSpec` | f-string parts | literal/expr | format type in `FormatSpec` | lexer/parser |
| `Arm` | pattern + guard + body | — | guard is `if` | match |

Structural invariant: every `Expr` variant that can be a diagnostic subject
carries a `Span`; `TypeExpr` is the sole span-less node, so all type-position
diagnostics borrow the enclosing declaration's span (§9, `CONF-RESOLVE-4`).

## 5. Namespace Model

Three namespaces, matching `LANGUAGE_SPEC.md` §26 and §18.1:

| Namespace | Members | Collision rule |
|---|---|---|
| Value | functions, constants, top-level `let`, parameters, locals | duplicate in one scope is `E2007` |
| Type | `struct`, `enum`, `type` alias, `trait`, type parameters | duplicate is `E2012` (`E2007` for a shadowing type parameter) |
| Variant | enum tags, per module | duplicate tag within a module is `E2013` |

A value name and a type name **may coincide** (`fn S` and `struct S` coexist,
§26, verified). Constructors/references disambiguate by position
(`S()`/`S{...}` is the type; a bare value name is the value). Enum tags are
canonicalized to their module path plus the tag (`§27`), so two modules may
each declare a tag of the same local spelling — see `CONF-RESOLVE-6` for the
resulting tension with `§18.1`'s "tags are global".

## 6. Scope and Shadowing Matrix

| Case | Actual behavior | Classification |
|---|---|---|
| Parameter shadows outer local | inner wins, outer restored on exit | correct |
| Local shadows outer local/global | inner wins (block scoping) | correct (§16.3) |
| Nested block shadows and restores | correct | correct |
| Loop binding scoped to the loop | correct | correct |
| Match-arm binding scoped to the arm | no leak | correct |
| Catch binding scoped to the handler | no leak | correct |
| Lambda parameter shadows outer | inner wins | correct |
| `let x = x` | reads the **previous** binding | correct |
| Import vs later declaration | `E2007` either order | correct |
| Module name vs local | local wins inside scope | correct |
| Generic parameter vs declared type | `E2007` (see `CONF-RESOLVE-5`) | fixed |
| `const` not a variable binding | `const N` + `let N` in one scope is `E2007`; a **local** `let N` shadows the module constant | correct |
| Wildcard `_` reused | allowed, binds nothing | correct |
| Capitalized local (`let X = 1`) | parses as a **variant pattern**, not a binding (`let_stmt = "let" BIND_NAME`, §4.2) | documented, not a bug |

Determinism: duplicate and collision diagnostics are chosen by source order, not
hash iteration (`merge_diags`/`const_order`; locked by
`tests/modules.rs::duplicate_diagnostics_are_deterministic`).

## 7. Hoisting / Forward References

Verified hoisted (usable before declaration): functions and mutual recursion,
structs/enums/aliases/traits in annotations, `impl` blocks, constants **inside
function bodies** (functions run after initialization), modules, `use`
declarations, and generic calls. A constant initializer that reads a later
constant is `E2003` (§26). Type parameters are bound per declaration and do not
leak across sibling declarations.

## 8. Module / Import / Visibility Matrix

| Feature | Behavior | Status |
|---|---|---|
| In-source `module`, nesting | resolved by canonical path | verified |
| `::` paths, historical `.` separators | both accepted in `use` | verified |
| `use path`, `use path as Alias` (item) | binds last segment / alias | verified |
| `use m::E::A` (enum-path variant) | binds the variant (`§27`) | fixed `CONF-RESOLVE-2` |
| `use module` | allowed, binds no local name | verified |
| `pub use` re-export | **not** implemented; `n::f` is `E2003` | DESIGN LIMITATION (`CONF-RESOLVE-8`) |
| module alias `use m as mm` | accepted, then ignored (`mm::f` is `E2003`) | `CONF-RESOLVE-9` |
| Private item across boundary | `E2018` | verified |
| Descendant reaches ancestor private | allowed (§27) | verified |
| `pub` on module | parsed, **inert**: a nested module is reachable unqualified from outside | `CONF-RESOLVE-7` |
| Field/method visibility | enforced (`E2018`), per-overload | verified |
| Imports do not leak to siblings | verified | correct |
| Unknown module/item in `use` | `E2019` | verified |
| Unknown module in an expression | `E2003`/`E3002` (undefined name) | consistent with `docs/errors.md`; NON-ISSUE |

## 9. Confirmed Findings

| ID | Classification | Reproduction / root cause | Fix | Regression |
|---|---|---|---|---|
| CONF-RESOLVE-1 | IMPLEMENTATION BUG | `E::A(1)`, `E::Red()`, `use m::E` then `E::A(1)` and the pattern `E::A(x)` failed with `E3002`: `canonical_construct` only rewrote 3+ segment enum paths, and `resolve_use_path` could not pass an enum segment as a module. §27 names a variant by "its enum's path plus the tag". | Enum-path resolution against the enum's own variant set, in both construct and pattern positions and in `use`. | `tests/modules.rs::enum_path_variants_resolve_in_every_position`, `use_enum_variant_path_resolves` |
| CONF-RESOLVE-2 | IMPLEMENTATION BUG | Same root cause in `use`: `use shapes::Color::Red` reported `E2019: Color is not a module`. | `resolve_use_path` resolves the enum path before the module walk. | `use_enum_variant_path_resolves` |
| CONF-RESOLVE-3 | IMPLEMENTATION BUG | With both a struct `A` and a variant `E::A` sharing the canonical name `m::A`, `m::E::A(1)` built the struct when the enum was declared first and failed `E3002` when the struct was declared first — order-dependent and silently wrong. | The enum-path branch is tried first, membership-tested against the enum's variant set, so the result is identical in both orders. (Which namespace an *ambiguous* unqualified `m::A` names remains open — §10.) | `struct_and_variant_same_name_resolve_deterministically` |
| CONF-RESOLVE-4 | IMPLEMENTATION BUG (wrong span) | `fn g() -> a::S` for a private `a::S` reported `E2018` at `1:1`: `rewrite_type` passed `Span::default()` because `TypeExpr` carries no span. | The enclosing declaration's span is threaded through `rewrite_type`/`rewrite_type_args` (return, parameter, field, alias, annotation, type argument). | `private_type_in_type_position_reports_its_own_span` |
| CONF-RESOLVE-5 | IMPLEMENTATION BUG | A type parameter could shadow an **imported** or **module-local** declared type with no diagnostic: `check_type_params` tested only canonical root names. §36.2 forbids shadowing any declared type (`E2007`). | The resolver, which has the full module/import visibility model, rejects a type parameter that shadows any visible declared type. | `tests/generics.rs::type_parameter_cannot_shadow_any_visible_declared_type` |
| CONF-RESOLVE-6 | SPEC GAP | `module a { enum E { A } } module b { enum F { A } }` is accepted, but §18.1 says variant tags are unique **across the whole program**. §27's canonical per-module tags make them coexist. | Not changed. Contradiction recorded for a human decision. | — |
| CONF-RESOLVE-7 | DISCOVERED, PRESERVED | A nested `module secret { ... }` is reachable from outside its parent (`a::secret::f()`), and `pub module` is inert, though §27 says "every declaration is private to its module by default". | Not changed: locked by `tests/modules.rs::modules_nest` (`module a { module b { module c { ... } } }` reached as `a::b::c::f`). Recorded as a spec/implementation tension. | existing `modules_nest` |
| CONF-RESOLVE-8 | DESIGN LIMITATION | `pub use m::f` does not re-export: `n::f` is `E2003`. Re-exports are not implemented. | Not changed; RFC/roadmap candidate. | — |
| CONF-RESOLVE-9 | IMPLEMENTATION BUG | `use m as mm` is accepted and discarded; `mm::f` is `E2003`. §27 says `use path as Alias` binds under `Alias`, but a module import "binds no local name". | Not changed: aliasing a module has no defined semantics in §27. Recorded; safe minimal fix would be to reject a module alias rather than accept-and-ignore. | — (documented) |

No defect in AST invariants, hoisting, shadowing, or duplicate detection was
found beyond the items above.

## 10. Unresolved Questions

- `CONF-RESOLVE-6` — variant-tag uniqueness: program-global (§18.1) or
  module-scoped (§27). The implementation follows §27.
- `CONF-RESOLVE-7` — is a nested module a visibility boundary? The
  implementation says no; §27 says every declaration is private by default.
- `CONF-RESOLVE-8` — `pub use` re-exports: unsupported, unspecified.
- `CONF-RESOLVE-9` — module aliases: accepted but inert. Either implement or
  reject.

## 11. Tests Added

| Test | File | Locks |
|---|---|---|
| `enum_path_variants_resolve_in_every_position` | `tests/modules.rs` | CONF-RESOLVE-1 |
| `use_enum_variant_path_resolves` | `tests/modules.rs` | CONF-RESOLVE-2 |
| `struct_and_variant_same_name_resolve_deterministically` | `tests/modules.rs` | CONF-RESOLVE-3 |
| `private_type_in_type_position_reports_its_own_span` | `tests/modules.rs` | CONF-RESOLVE-4 |
| `type_parameter_cannot_shadow_any_visible_declared_type` | `tests/generics.rs` | CONF-RESOLVE-5 |

## 12. Validation

`cargo fmt --all -- --check` clean; `cargo clippy --locked --all-targets
--all-features -- -D warnings` clean; `cargo test --locked --all-features` =
**640 passed, 0 failed** (Phase-1 baseline 635; +5 new tests). Focused suites
`modules`, `generics`, `checker`, `grammar`, `adversarial`, `corpus` all pass.
No test asserts any current (pre-fix) wrong behavior.

## 13. Phase-2 Gate

- Confirmed Phase-2 bugs have regressions: **yes** (CONF-RESOLVE-1…5).
- Focused tests pass: **yes**.
- Full checker suite passes: **yes**.
- Adversarial reviewer counterexamples: recorded in the continuation result.

## 14. Phase-3 Handoff

Verified starting point for the type/checker layer: a resolver that
rewrites every free reference to one canonical name under a documented,
deterministic visibility model; three namespaces with per-module variant tags;
hoisting that covers functions/types/traits/constants; and four explicitly
open resolution questions (§10). Phase 3 must not assume a variant-tag
uniqueness rule or a nested-module visibility rule; both are decisions.
