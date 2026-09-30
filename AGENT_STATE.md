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

Authoritative current value is `git rev-parse HEAD`, also printed by
`scripts/agent-state.sh`; a tracked state file cannot safely hardcode its own
commit SHA. Final blocker-remediation starting HEAD:
`b55a43473379ecc04c209882345defb70801640d`.

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
- FSM-P6 — multi-file Playground UX (UI + state + transport; no language
  semantics)

Never use bare “Phase N” for filesystem-module work. `CONFORMANCE_PHASE<N>` is
the historical conformance series and `FEATURE_<NNN>` is the feature series.

## Verified Closed

- FSM-P1 — CLOSED
- FSM-P2 — CLOSED
- FSM-P3 — CLOSED
- FSM-P4 — CLOSED REMOTELY
- FSM-P5 — CLOSED REMOTELY (green on
  `bf95d101dd18f3ad16eaa7bfb99c8304d44bf49d`)
- FSM-P6 — IMPLEMENTED LOCALLY — NOT COMMITTED, NOT PUSHED

FSM-P5 uses caller-supplied virtual sources through `InMemorySourceProvider`,
the existing `SourceProvider`, `ModuleGraphBuilder`, canonical resolver,
checker, runtime, and WASM boundary. Playground API remains 1 and Host ABI
remains 1.

## Current Work

FSM-P6 — Multi-file Playground UX. The Playground now exposes the virtual
multi-source capability FSM-P5 delivered, through a file-tab UI and a project
state model, without adding any language semantics:

- `playground/web/project.js` — the project state model (files, opaque
  `SourceKey`s, unique display names, active file, entry file, declared
  provider child links). Pure, DOM-free, unit-testable.
- `playground/web/app.js` — file tabs, add/rename/set-entry/delete/reset, an
  example loader that supports multi-file examples, source-aware diagnostics
  (a diagnostic activates the file that produced it), and a `run()` that sends
  `source` for a one-file project (historical path) or `project` for a real
  multi-file project.
- `playground/web/worker.js` — additive project transport: `runProject` when a
  project is supplied, `run` otherwise; a runtime without `aura_project_*`
  reports a structured capability error instead of failing obscurely.
- `playground/runtimes/0.2.0-dev.2/` — a development runtime carrying the
  additive Host ABI 1 virtual-project exports and the builtin-name reservation.
  Development channel only; not a release, not tagged, never a replacement for
  `0.2.0`.

`playground/build.mjs` now pins the `0.2.0` release identity permanently, so
the build can never regenerate or overwrite it. The manifest lists 5 entries
(`0.0.1`, `0.0.2`, `0.0.2-dev.30`, `0.2.0`, `0.2.0-dev.2`).

Host ABI remains 1. Playground API remains 1. No `src/**` change. No frozen
artifact change.

The earlier FSM-P5 blockers remain remediated: invalid virtual/provider source
keys use the normative `E2022` diagnostic partition, and case-only child
collision detection no longer performs the quadratic pairwise scan.

Latest validation completed after both production fixes:

- `cargo fmt --all -- --check`;
- `cargo test --locked --all-targets --all-features`;
- `cargo clippy --locked --all-targets --all-features -- -D warnings`;
- `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time`;
- `cargo +1.83.0 check --locked --all-features`;
- `cargo build --locked --manifest-path playground/runtime/Cargo.toml --release --target wasm32-unknown-unknown`;
- `node playground/tests/node/run-all.mjs`;
- `node playground/build.mjs --check`;
- `node website/tests/run-all.mjs`.

The 64,000-claim VFS adversarial reproduction is within the 2 MiB request
limit and returns deterministic `E2021` without a host failure. Frozen runtime
artifacts remain byte-identical.

## Next Exact Action

INDEPENDENT REVIEW OF THE LOCAL FSM-P6 STACK, THEN HUMAN AUTHORIZATION TO PUSH.

FSM-P6 is implemented and locally validated but deliberately uncommitted and
unpushed. Do not push without new explicit human authorization, and do not
begin a follow-up phase (package management, persistence, LSP, formatter) until
FSM-P6 is reviewed.

## Human Gates

AUDIT-3 / TypeExpr nesting = DECISION-PENDING.

Valid approval tokens remain exactly:

- `DECISION APPROVED: OPTION A`
- `DECISION APPROVED: OPTION B`

## Repository-local Codex Policy

Project `.codex/config.toml` contains project-local safety/context settings
only: approval policy, sandbox mode, documentation/output limits, and history
persistence. Model selection, provider routing, the ChatGPT Web bridge,
Compatibility mode, and global agent concurrency belong in
`~/.codex/config.toml`.

The stale project-local `[models.new_thread]` and `[agents]` routing overrides
were removed during the post-FSM-P5 authority-alignment pass. The project file
must not reintroduce model/provider/bridge/global-agent routing.

## Protected Local State

- `s` — pre-existing untracked file; verified size 1 byte; do not edit, stage,
  commit, delete, or rename it.

## Release Immutability

- `0.0.2` — 1,366,621 bytes —
  `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`
- `0.2.0` — 1,654,161 bytes —
  `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc`
- `0.2.0-dev.1` — 1,767,068 bytes — superseded development runtime, NOT a
  release:
  `ba40e89c834896badfb17d5c72aa2dcb227907a7b5ba513c315ef2f2da0adf08`
- `0.2.0-dev.2` — 1,767,723 bytes — current development runtime, NOT a release:
  `b69f212bf3f1d8df41b66ad249bf9c9829015b06459569fd2b765a5596b66c06`

Historical runtime directories are immutable. `playground/build.mjs` pins each
frozen release identity and refuses to regenerate or overwrite it; advancing a
runtime means adding a new version, never replacing one.

## Do Not Reopen

- Core freeze;
- resolved Core SPEC gaps;
- FSM-P1;
- FSM-P2;
- FSM-P3;
- FSM-P4;
- FSM-P5.

Reopen only for a concrete reproducible regression.

## Do Not Start

Without new human authorization:

- package manager;
- package manifest;
- browser/localStorage project persistence;
- LSP;
- formatter product;
- async;
- macros;
- new release;
- AUDIT-3 implementation.

FSM-P6 is defined as *Multi-file Playground UX* and is implemented locally. It
authorizes no language-semantics change: the deferred candidates above (package
management, persistence, LSP, formatter) remain deferred and must not be
promoted into a follow-up phase by assumption.

## Handoff

Before a model switch, finish the atomic operation, understand the diff, update
this file when state materially changes, run `scripts/agent-state.sh`, and hand
writer ownership to `NONE` or the explicit next model. After a switch, read
this file, run the preflight, inspect the diff, and verify Git before writing.

Repository synchronization does not rely on chat memory or chain-of-thought.

## Writer

NONE
