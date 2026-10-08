# Architecture Decision Records

Every material language, runtime, compatibility, security, or release-engineering
decision is recorded here as a numbered ADR. An ADR states the context, the
decision, the consequences, and the evidence; the code and tests implement and
enforce it.

The Architecture Decision Council (the principal language/compiler/runtime/
compatibility/security/performance/release engineers plus an independent
reviewer) has delegated authority to resolve routine technical and
language-design decisions, so these no longer require the human owner's prior
approval. A genuine specification choice with unresolved evidence still
produces a decision package rather than an improvised answer.

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-release-vs-language-version.md) | Release version vs. language version | Accepted |
| [0002](0002-module-member-builtin-names.md) | Module members vs. builtin-name reservation | Accepted |
| [0003](0003-supported-cpython-versions.md) | Supported CPython versions | Accepted |
| [0004](0004-typeexpr-nesting-policy.md) | TypeExpr nesting policy | Accepted |

## Adding an ADR

1. Copy the structure of an existing ADR (Context, Decision, Consequences,
   Evidence).
2. Number it sequentially; never renumber or rewrite a merged ADR — supersede
   it with a new one and mark the old one `Superseded by ADR-N`.
3. Implement and test the decision in the same train.
4. Reference the ADR from the code, the specification, and the relevant
   operational file (`AGENT_STATE.md`, `docs/archive/AURA_V1_READINESS.md`).
