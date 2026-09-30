# ADR-0002 — Module-member names and builtin reservation

- **Status:** Accepted (Architecture Decision Council, 2026-09-30)
- **Supersedes:** queued decision HD-1
- **Related:** `LANGUAGE_SPEC.md` §3.3 (builtin reservation), §26/§27 (modules),
  `src/check/mod.rs` (`hoist`/`declare_inner`), `src/resolve.rs` (`collect_module`)

## Context

The builtin-name value-namespace reservation (`E1009`) forbids a *user-visible
value binding* (`let`/`let mut`, parameters, loop/catch/pattern bindings,
top-level `let`, import aliases, top-level `fn`) from using a registered
builtin name. The question: does it also forbid a **module member** declared as
`module M { pub fn sum(...) }`?

## Evidence

- Module members are resolved to canonical, **qualified** names
  (`M::sum`) by `resolve.rs` (`collect_module`/`join`), before the checker sees
  them. They are never reachable unqualified from another module.
- The reservation is enforced in `checker.hoist` (top-level `fn`/`const`) and
  `declare_inner` (local value bindings) — i.e. in the *user-visible value
  namespace*, and in `resolve.rs` for value-namespace import aliases.
- `LANGUAGE_SPEC.md` §26/§27 establish that a module's declarations are visible
  within the module; §3.3's rationale for reservation is to prevent a user
  binding from *silently shadowing* a builtin in the same value namespace.
- Type, module, field, variant, and method namespaces already permit reusing a
  builtin spelling (`struct sum`, `enum { sum }`, `module sum`, `impl { fn sum }`),
  because they are distinct namespaces.

## Decision

**Module members may reuse builtin spellings.** A member named `sum` in
`module M` occupies `M`'s member namespace and is reachable only as `M::sum`
(or through an explicit `use`). It shadows the builtin **only inside its own
module**, where qualified resolution makes the intent unambiguous, and never
leaks into a caller's value namespace unqualified.

The reservation remains exactly scoped to the **user-visible value namespace**
(global and local), which is where a collision can silently break binding
lookup — the actual `sum` defect. Extending it to module members would forbid
harmless module-local helper names and contradict the established separation of
semantic namespaces.

Clarification recorded in `LANGUAGE_SPEC.md`: the §3.3 phrase "user functions"
means **top-level functions in the value namespace**, not module members.

## Consequences

- No behavior change (current behavior confirmed correct and now normative).
- Importing a module value-binding by alias into the value namespace
  (`use M::sum`) remains `E1009` where it would collide — the boundary at which
  unqualified shadowing could actually occur is still enforced. (Aura has no
  glob/wildcard import; the only way to bring a module member into a value
  namespace is an explicit `use … [as …]`.)
- New tests assert: module member reuse accepted; qualified and intra-module
  unqualified calls resolve to the member; an aliasing import that collides is
  rejected.
