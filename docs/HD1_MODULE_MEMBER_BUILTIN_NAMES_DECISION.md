# HD-1 — Module-Member Names and Builtin Reservation

Status: **HUMAN SEMANTIC DECISION REQUIRED**

This is a decision package, not a resolved rule. No behavior was changed.

## Question

Does the builtin-name value-namespace reservation (`E1009`,
`LANGUAGE_SPEC.md` §3.3) extend to **module members** — a `fn`/`let` declared
inside `module M { ... }` whose name matches a registered builtin?

## Current behavior

Module members are resolved to canonical, qualified names (`M::sum`) by the
resolver and live in the module's own namespace. The reservation is enforced
only where a binding is installed into the user-visible *global/local* value
namespace. As a result:

```
module M {
    pub fn sum(xs: [int]) -> int { return 42 }
    pub fn use_it() -> int { return sum([1, 2, 3]) }   # 42 — the builtin is shadowed
}
fn main() { print(M::use_it()) }        # prints 42, accepted

use M::sum
fn main() { print(sum([1,2,3])) }       # E1009 (checked at the import alias)
```

A module member named after a builtin is accepted at declaration; it shadows
the builtin **only inside its own module** (or through a qualified path), and
never leaks into the caller's value namespace.

## Specification evidence

- `LANGUAGE_SPEC.md` §3.3 (builtin reservation, added by hardening `c8ded06`):
  "Registered builtin function names ... MUST NOT be used as user-defined names
  in the value namespace: `let`/`let mut`, function parameters, lambda
  parameters, loop bindings, catch bindings, pattern bindings, **user
  functions**, top-level `let`, and import aliases."
- `LANGUAGE_SPEC.md` §26: a module's declarations are "visible throughout the
  module"; types and value names occupy separate namespaces.
- `LANGUAGE_SPEC.md` §27: `use path as Alias` binds a name into the importing
  module; module member names are reached through the module path.
- `docs/LANGUAGE_SPEC.md` §25: builtins are global callables dispatched by
  name.
- The hardening commit message: "Type, module, field, variant, and method
  namespaces remain separate and may reuse spellings per namespace law."

## Implementation evidence

- `src/check/mod.rs` `hoist` applies the reservation to top-level `Item::Fn`
  and `Item::Const` only.
- `src/resolve.rs` `rewrite_item` rewrites a module `Item::Fn`/`Item::Const`
  name to `join(prefix, name)` (e.g. `M::sum`) before the checker sees it, so
  the checker's `hoist` never matches the bare builtin spelling for a module
  member.
- `src/resolve.rs` import handling rejects a value-namespace alias that equals
  a builtin (`E1009`).

## Why it is ambiguous

The normative text says "user functions" without qualifying "global". Read
literally, `module M { fn sum }` is a user function and should be `E1009`. Read
in context ("value namespace" contrasted with "Type, module, field, variant,
and method namespaces remain separate"), a module member occupies the module
namespace, not the user-visible value namespace, so it is out of scope.

## Alternatives

- **Option A — keep current behavior.** Module members may reuse builtin
  spellings; they shadow the builtin only within their module. Consistent with
  a distinct module-member namespace and with type/field/variant/method reuse.
  Lowest compatibility risk; module code that names a helper `sum` keeps
  working.
- **Option B — extend the reservation to module members.** Reject `module M {
  fn sum }` and `module M { let sum }` with `E1009`. More uniform with the
  global rule's stated rationale ("must not silently shadow a builtin"), but it
  makes a *module-local* helper name that cannot affect any caller illegal and
  is a breaking change for any such program.

## Compatibility consequences

- Option A: none.
- Option B: rejects previously valid module member names matching any of the 38
  builtins; requires updating any such fixtures/examples.

## Recommendation

**Option A (keep current behavior)** unless the human wants the reservation to
be namespace-uniform. The module-member namespace is genuinely separate (a
member is never reachable unqualified from another module), and the hardening
commit itself lists "module ... namespaces remain separate". The literal
phrase "user functions" in §3.3 should be clarified to "top-level (module
value-namespace) functions" if Option A is chosen.

## Impact if unresolved

None. The ambiguity does not block any current-core work; it is a policy
boundary only.
