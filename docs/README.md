# Documentation index and authority map

This file removes ambiguity about which document defines current Aura
semantics. `AGENTS.md` states the authority rule: one authoritative answer per
question, and no two current-authority documents may disagree.

## Normative (defines what Aura programs mean)

| Question | Authority |
|---|---|
| Operating rules every agent obeys | `/AGENTS.md` |
| The language: syntax and semantics | `/docs/LANGUAGE_SPEC.md` |
| The grammar (enforced by `tests/grammar.rs`) | `/docs/grammar.md` |
| The frozen filesystem-loading design contract | `/docs/FILESYSTEM_MODULES_DESIGN.md` |
| Accepted architecture decisions | `/docs/adr/*` |
| The embedded-Python boundary | `/docs/CPYTHON_COMPATIBILITY_TARGET.md` |
| Diagnostic codes | `/docs/errors.md` |
| The object model (four pillars, no inheritance) | `/docs/OOP.md` |
| Generics | `/docs/GENERICS.md` |
| The language contract summary | `/docs/contract.md` |

When a normative document and the implementation disagree, the implementation
is the fact and the document is the defect: reconcile them in the same change.
`tests/contract_sync.rs` pins the specification's declared language version
against `src/lib.rs::LANGUAGE_VERSION`.

## Current engineering state

| Question | Authority |
|---|---|
| Current repository state | `/AGENT_STATE.md` |
| Active task and exact next action | `/docs/engineering/CURRENT_HANDOFF.md` |
| Higher-level project and release status | `/STATUS.md` |
| Future authorized work | `/PLANS.md` |
| Human decisions awaiting an owner | `/HUMAN_DECISIONS_QUEUE.md` |
| The Keystone program scope and dispositions | `/docs/engineering/KEYSTONE_SCOPE_MANIFEST.md` |

## Engineering design records (`docs/engineering/`)

These explain a subsystem; they are authoritative for their design decisions
but do not override the language specification.

Runtime and execution: `RUNTIME_ARCHITECTURE.md`,
`ITERATIVE_EVALUATOR_DESIGN.md`, `CRITICAL_SYSTEM_PROFILE.md`,
`PERFORMANCE.md`, `MODULE_ARCHITECTURE.md`.

Semantics and safety: `EXCEPTION_ARCHITECTURE.md`,
`EXCEPTION_SYNTAX_DECISION_PACKAGE.md`, `CAPABILITY_MODEL.md`,
`DIAGNOSTIC_TAXONOMY.md`, `RISK_REGISTER.md`, `TECHNICAL_DEBT.md`,
`FUTURE_EXTENSION_BOUNDARIES.md`.

Boundaries and interfaces: `HTTP_ARCHITECTURE.md`, `EMBEDDED_PYTHON_ARCHITECTURE.md`,
`AIS.md`, `MCP.md`, `DX_AIX.md`, `JSON_VALUE_ALGEBRA_DECISION.md`.

Process: `AURA_ENGINEERING_ORG.md`, `RELEASE_ENGINEERING.md`,
`TOOLING_RECONCILIATION.md`, `REVIEW_RECORD_T0_T5.md`,
`B1R2_DIFFERENTIAL_ORACLE.md`, `B1R3B8_COMPLETION_AUDIT.md`.

## Historical (evidence, not current authority)

These record decisions, audits, and reports from earlier programs. They are
preserved because AGENTS.md requires historical artifacts to be retained and
because a future reconstruction may need them. They MUST NOT be treated as
current semantics:

- `docs/CONFORMANCE_PHASE1..12.md`, `docs/CONFORMANCE_FINAL.md` — the
  historical conformance audit series (`CONFORMANCE_PHASE<N>`).
- `docs/FEATURE_001..006_*`, `docs/FEATURE_H1_*`, `docs/FEATURE_ROADMAP.md` —
  the `FEATURE_<NNN>` development series.
- `docs/AUDIT3_TYPE_NESTING_DECISION.md`, `docs/B1R3A_AST_SHARING_DECISION.md`,
  `docs/HD1_MODULE_MEMBER_BUILTIN_NAMES_DECISION.md`,
  `docs/WASM_CALL_FRAME_LIMIT_DECISION.md` — decision packages; superseded by
  their accepted ADRs where one exists.
- `docs/SEMANTIC_CLOSURE_REPORT.md`, `docs/SEMANTIC_FREEZE_AUDIT.md`,
  `docs/LANGUAGE_SPEC_CONFORMANCE_REPORT.md`,
  `docs/FINAL_SEMANTIC_RED_TEAM_REPORT.md`, `docs/ARCHITECTURE_REVIEW.md`,
  `docs/AURA_TOTAL_HARDENING_INDEPENDENT_REVIEW.md`,
  `docs/POST_FEATURE_002_STACK_AUDIT.md`, `docs/PRE_MODULES_AUDIT.md`,
  `docs/CORE_FREEZE.md`, `docs/CORRECTIONS.md` — completed audit and freeze
  reports.
- Root-level `AURA_*.md` (`AURA_COMPLETENESS_MATRIX.md`,
  `AURA_COMPLETION_*.md`, `AURA_ENGINEERING_*.md`, `AURA_ROAD_TO_V1.md`,
  `AURA_V1_READINESS.md`, `IMPLEMENT.md`) — completion and readiness snapshots
  from earlier checkpoints.
- `docs/RELEASE_0_0_X_DESIGN.md`, `docs/MIGRATION_0_2_1.md` — release history
  and migration guidance for shipped versions.

## Superseded

None currently. A document is listed here only when a normative document
replaces it and no historical value remains; deleting or relocating history
requires a human gate (AGENTS.md: never discard historical artifacts).

## Generated

- `website/dist/**` — built output; never edited by hand.
- `/docs/_site/` — generated documentation output (gitignored).

## User-facing (`website/content/`)

The website content is a *projection* of the specification and the
engineering records, never a second source of semantic truth. The
release-version guard (`website/tests/release-version.test.mjs`) fails when
the site's version claims, `Cargo.toml`, `LANGUAGE_VERSION`, and the runtime
manifest disagree.
