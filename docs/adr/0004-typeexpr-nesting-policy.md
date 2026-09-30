# ADR-0004 — Type-expression nesting policy (AUDIT-3)

- **Status:** Accepted (Architecture Decision Council, 2026-09-30)
- **Supersedes:** queued decision HD-2 / AUDIT-3 (DECISION-PENDING)
- **Related:** `docs/AUDIT3_TYPE_NESTING_DECISION.md` (the decision package),
  `LANGUAGE_SPEC.md` §31.1/§31.2, `src/parse/mod.rs`
  (`enforce_depth`, `check_expr_depth`, `parse_recursion_budget`),
  `playground/tests/node/differential.test.mjs`

## Context

The semantic AST-depth limit (`MAX_AST_DEPTH = 256`, `E1015`) is enforced by
`enforce_depth` over expressions and statements, but **not** over `TypeExpr`
nodes. Type nesting is bounded only by the substrate-calibrated parser backstop
`parse_recursion_budget()` (native 2048, wasm 768). This is the one place where
acceptance differs between native and WASM for the same source, contradicting
the architecture's "valid under the semantic limit ⇒ accepted on every
substrate" invariant.

The decision package (`docs/AUDIT3_TYPE_NESTING_DECISION.md`) offered:
- **Option A** — count `TypeExpr` nodes toward the 256-node semantic budget
  (eliminate the divergence).
- **Option B** — document the substrate-dependent bound, change no code.

Measured (reconfirmed): native last-accepted 2047 / first-rejected 2048; WASM
767 / 768; both `E1015`, no host failure. No real Aura program in the repo
nests a type more than one level (deepest executable usage 0; deepest anywhere
3, in prose).

## Decision

**Adopt Option A:** extend the semantic depth accounting to descend `TypeExpr`
nodes so type nesting counts toward the same `MAX_AST_DEPTH = 256` budget as
every other node kind.

Rationale, in the project's stated priority order:

1. **Correctness/coherence** — restores the substrate-independence invariant:
   "valid under 256 ⇒ accepted everywhere". Removing the last documented
   native/WASM acceptance asymmetry is worth a bounded, well-understood change.
2. **Determinism/platform consistency** — one bound on all substrates.
3. **Compatibility** — the practical breakage surface is empty: no program in
   the repository nests types beyond depth 1, and 256 is far beyond any
   realistic annotation. The change rejects only synthetic/adversarial input;
   it does **not** change the meaning of any well-defined program. It is
   therefore a patch-level language correction (see ADR-0001), shipping with
   `LANGUAGE_VERSION` at the same line.
4. **Safety** — both before and after, over-limit input is a structured
   `E1015`, never a trap. Option A makes the bound tighter and uniform, which
   is strictly safer.

A union (`A | B | ...`) is a flat list, not nesting; it must **not** be
penalized per member. Only true structural nesting (generic application
`Box<…>`, list `[T]`, map `{K: V}`) counts.

## Consequences

- `src/parse/mod.rs`: `enforce_depth`/`check_expr_depth` gain a `TypeExpr`
  descent; type annotations in `fn` params, returns, `let`, struct fields, enum
  payloads, aliases, `impl`/`trait` signatures, and generic args are covered.
- `LANGUAGE_SPEC.md` §31.1/§31.2 updated: type nesting counts toward
  `MAX_AST_DEPTH`; the substrate note is removed.
- `website/content/reference-limits.md` updated.
- `playground/tests/node/differential.test.mjs` TypeExpr sweep: the two
  ceilings collapse to one; the sweep asserts native == WASM.
- Regression tests at N-1/N/N+1 (255/256/257) on **both** substrates, plus a
  flat-union acceptance test (a long union stays accepted).
- A WASM runtime carrying this parser change needs a **new** development
  runtime identity (per the runtime-identity policy); the `0.2.1-dev.1` build,
  which predated this change, is superseded by `0.2.1-dev.2` (the manifested
  current development runtime) and preserved on disk unlisted.

## Alternatives considered

- **Option B (document only).** Rejected: it permanently enshrines a
  native/WASM acceptance divergence in a system built to eliminate exactly
  that, for no compatibility benefit (no real program is affected).
- **A separate, larger type-only limit (e.g. 512).** Rejected as needless: it
  would preserve an asymmetry and add a special case; one uniform budget is
  simpler and sufficient.
