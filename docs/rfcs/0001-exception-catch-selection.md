# RFC 0001 — Exception Catch Selection

- **Status:** Accepted (record of the Aura 0.3 “Keystone” human authorization,
  2026-10-06)
- **Scope:** language grammar and semantics; additive to `try`/`catch`/`finally`
- **Authorities:** `docs/LANGUAGE_SPEC.md` §14.5–14.6 (current `try`),
  `docs/engineering/EXCEPTION_ARCHITECTURE.md` §2 (level separation),
  `docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md` (E1–E6 evidence),
  ADR-0002 (builtin-name namespace reservation)
- **Human decisions:** E1–E6 resolved by the Keystone engineering order
  (§20). This RFC records that authorization; it opens no new human gate
  because the chosen grammar reuses an existing, unambiguous production.

## 1. Motivation

Today the catch clause binds exactly one untyped name:

```aura
try { throw MyErr::Bad("boom") } catch e {
    match e {
        MyErr::Bad(m) -> { print(m) }
        _ -> { print("other") }
    }
}
```

Family selection is possible only through the verbose nested `match` inside a
single all-catching clause. A program cannot select a nominal variant at the
catch site, which is the single remaining gap identified by the Pre-0.3
decision package (E2). Every other exception requirement — raise mechanism
(E1), identity (E3), non-catchable builtin diagnostics (E4), Python boundary
(E5), reserved builtin identity (E6) — is already satisfied or is a
non-grammar policy decision.

## 2. Decision (E1–E6)

| # | Decision | Implementation |
|---|---|---|
| E1 | Keep `throw <expr>`; no `raise`/`new`/class constructors/inheritance. | unchanged |
| E2 | Catch selection reuses Aura’s **existing pattern grammar**: the clause after `catch` is a full `pattern` (§4.7), matched against the thrown value with the same first-match semantics as `match`. | this RFC |
| E3 | Exception identity is **nominal and module-qualified**, exactly the canonical variant path the resolver already produces (`M::Err::Bad`), never a string tag. | `rewrite_pattern` canonicalization |
| E4 | Builtin runtime/resource/Host/internal diagnostics (`E4xxx`/`E5xxx`/`E4999`) stay **non-catchable**; only explicit `throw` remains catchable. | unchanged level separation |
| E5 | Python failure (`E5001`) remains behind the foreign/provider boundary; it is not converted into a catchable exception. | unchanged |
| E6 | Builtin exception-family identity is reserved by a **reserved module root**: the resolver rejects a user module that would claim the builtin namespace root. | `resolve.rs` guard |

## 3. Grammar delta

```
try_stmt        = "try" block "catch" pattern block [ "finally" block ] ;
```

Formerly the clause after `catch` was `IDENT`; it is now `pattern`, which
already parses a bare identifier as `Pattern::Bind`. The production is a
strict widening: every program that parsed before parses identically and keeps
its meaning.

## 4. Semantics

- The thrown value is matched against the catch pattern with the ordinary
  pattern-match rules (`match_pattern`/`bind_pattern`):
  - `catch e` binds any thrown value (unchanged behavior);
  - `catch MyErr::Bad(m)` selects one nominal variant and binds its payload;
  - `catch _` selects any thrown value and binds nothing;
  - `catch none` selects a thrown `none`;
  - list/literal patterns select by value shape, exactly as in `match`.
- A pattern that does **not** match does not run the catch body; the throw
  continues to the next enclosing `try` (or becomes `E4026`).
- A catch pattern MUST name declared variants; an unknown tag is checked by
  the same `check_pattern` used for `match` (`E3002`).
- Bindings introduced by the pattern are ordinary immutable bindings
  (§16.3), exactly as a match arm’s bindings.
- `finally` and all error-path behavior are unchanged.
- Only explicit `throw` is catchable; a pattern never makes a fatal
  diagnostic catchable (E4/E5).

## 5. Migration

None. The change is additive: the old spelling is a special case of the new
grammar. No existing test or program changes meaning.

## 6. Enforcement

- `tests/catch_syntax.rs`: bare binding, variant selection, wildcard, `none`,
  non-matching propagation, unknown tag `E3002`, duplicate-binding `E2014`,
  catch binding is immutable, closures capture it, `finally` interplay.
- `tests/oracle/*` differential: both engines produce identical results for
  every catch-pattern shape.
- Runtime raise-site span preservation: an uncaught `throw` reports the
  throwing site, not the frame-crossing call site.
- Reserved namespace: a user `module Aura { … }` is rejected (`E2023`).
- Mutation check: restoring the old bare-ident parser makes the variant tests
  fail for the expected reason, then the source is restored byte-exact.
