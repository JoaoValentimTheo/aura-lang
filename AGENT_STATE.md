# Aura Agent State

## Authority

1. Actual Git/worktree state is authoritative for current repository reality.
2. `AGENT_STATE.md` is the concise operational checkpoint.
3. `AGENTS.md` contains durable agent rules.
4. `STATUS.md` is detailed chronology/history.
5. `PLANS.md` contains future planning.
6. `docs/LANGUAGE_SPEC.md` is normative for language semantics.
7. `docs/FILESYSTEM_MODULES_DESIGN.md` is the filesystem loading architecture
   design contract.

If Git and `AGENT_STATE.md` disagree, Git wins. Reconcile this file before
continuing substantial work.

## Branch

`rewrite/v3-rust`

## Local HEAD

Authoritative current value: `git rev-parse HEAD` (also printed by
`scripts/agent-state.sh`). A tracked state file cannot safely hardcode the SHA
of the commit that contains itself without becoming stale as that commit is
created.

Correction-pass starting HEAD:
`e17f6b58b839d002904ca8afa0a3ad6cd2fe45f9`.

## Remote HEAD

`origin/rewrite/v3-rust` =
`75c57428ca70c53a3d592fdbb93ec3c5d7cb46f8`

FSM-P5 is local/unpushed relative to that remote HEAD.

## Current Track

AURA FILESYSTEM MODULE SYSTEM

## Phase Namespace

- FSM-P0 — architecture reconstruction
- FSM-P1 — filesystem module design
- FSM-P2 — multi-source provenance
- FSM-P3 — provider-neutral module graph
- FSM-P4 — native filesystem provider
- FSM-P5 — virtual/WASM VFS foundation

Never use bare “Phase N” for filesystem-module work. `CONFORMANCE_PHASE<N>` is
the historical conformance series and `FEATURE_<NNN>` is the feature series.

## Verified Closed

- FSM-P1 — CLOSED
- FSM-P2 — CLOSED
- FSM-P3 — CLOSED
- FSM-P4 — CLOSED REMOTELY
- FSM-P5 — LOCALLY CLOSED — NOT PUSHED

FSM-P5 uses caller-supplied virtual sources through `InMemorySourceProvider`,
the existing `SourceProvider`, `ModuleGraphBuilder`, canonical resolver,
checker, runtime, and WASM boundary. Playground API remains 1 and Host ABI
remains 1.

## Current Work

Post-FSM-P5 correction pass is locally complete: FSM-P5 was independently
audited, repository authority was reconciled, the source-provider boundary was
clarified in current documentation, and validation is green. Human review is
the next action. No FSM-P6 work is authorized.

## Next Exact Action

HUMAN REVIEW OF FSM-P5 + AUTHORITY RECONCILIATION BEFORE PUSH

## Human Gates

AUDIT-3 / TypeExpr nesting = DECISION-PENDING.

Valid approval tokens remain exactly:

- `DECISION APPROVED: OPTION A`
- `DECISION APPROVED: OPTION B`

## Protected Local State

- `.codex/config.toml` — pre-existing modified unrelated local state; do not
  edit, stage, commit, restore, or normalize it in Aura work.
- `s` — pre-existing untracked file; verified size 1 byte; do not edit, stage,
  commit, delete, or rename it.
- The ignored project-level `.codex/config.toml` `models` warning is outside
  this task and must not be “fixed” as incidental cleanup.

## Release Immutability

- `0.0.2` — 1,366,621 bytes —
  `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`
- `0.2.0` — 1,654,161 bytes —
  `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc`

Historical runtime directories are immutable. A development build may remain
in target/output only until separately authorized for publication.

## Do Not Reopen

- Core freeze;
- resolved Core SPEC gaps;
- FSM-P1;
- FSM-P2;
- FSM-P3;
- FSM-P4;

Reopen only for a concrete reproducible regression.

## Do Not Start

Without new human authorization:

- FSM-P6;
- multi-file Playground UI;
- package manager;
- package manifest;
- LSP;
- formatter product;
- async;
- macros;
- new release;
- AUDIT-3 implementation.

FSM-P6 is not yet defined. Deferred candidates must not be promoted into it by
assumption.

## Handoff

Before a model switch, finish the atomic operation, understand the diff, update
this file when state materially changes, run `scripts/agent-state.sh`, and hand
writer ownership to `NONE` or the explicit next model. After a switch, read
this file, run the preflight, inspect the diff, and verify Git before writing.

Repository synchronization does not rely on chat memory or chain-of-thought.

## Writer

NONE
