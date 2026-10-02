# Aura — Codex Project Instructions

These instructions apply to the entire repository.

## Objective

Develop and maintain Aura from repository evidence. Preserve verified behavior,
historical artifacts, and human-gated decisions. Do not reconstruct current
state from chat memory.

## Startup protocol

Before substantial work:

1. read `AGENT_STATE.md`;
2. inspect the current branch;
3. inspect local `HEAD`;
4. inspect `origin/rewrite/v3-rust`;
5. inspect `git status --short` and staged state;
6. compare Git reality with `AGENT_STATE.md`;
7. if they conflict, Git wins and state must be reconciled before continuing.

`AGENT_STATE.md` is the concise operational checkpoint. Read `STATUS.md` when
detailed chronology or prior validation evidence is needed. Read `PLANS.md`
when future planning is needed.

## Authority

- Git and the working tree are authoritative for repository reality.
- `AGENT_STATE.md` records concise current operational state.
- `AGENTS.md` contains durable agent rules.
- `STATUS.md` is detailed chronology/history.
- `PLANS.md` contains future planning.
- `docs/LANGUAGE_SPEC.md` is normative for Aura language semantics.
- `docs/FILESYSTEM_MODULES_DESIGN.md` is the frozen filesystem-loading design
  contract.

## Phase namespaces

Do not infer one work series from another:

- `CONFORMANCE_PHASE<N>` = historical conformance audit series;
- `FEATURE_<NNN>` = feature development series;
- `FSM-P<N>` = filesystem module system series.

Use `FSM-P<N>` everywhere current filesystem-module work is discussed. Never
use bare `Phase N` for the filesystem-module program.

## Immutable / human-gated state

Historical runtime artifacts are immutable. Never overwrite a versioned
artifact under `playground/runtimes/`.

Frozen release `0.0.2`:

- `playground/runtimes/0.0.2/aura_playground_runtime.wasm`
- size `1,366,621`
- SHA-256 `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`

Stable release `0.2.0`:

- `playground/runtimes/0.2.0/aura_playground_runtime.wasm`
- size `1,654,161`
- SHA-256 `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc`

Current release `0.2.1` (tag `v0.2.1` = commit `3f5f8702`, published
2026-10-01):

- `playground/runtimes/0.2.1/aura_playground_runtime.wasm`
- size `1,768,322`
- SHA-256 `48c456fcda6c50dd6808ccc5f15a0bca4c0b81d7d970172557817decf427cc9e`

AUDIT-3 / TypeExpr nesting is **RESOLVED** by ADR-0004
(`docs/adr/0004-typeexpr-nesting-policy.md`): the human owner delegated routine
language-design decisions to the Architecture Decision Council, which adopted
Option A — structural `TypeExpr` nesting counts toward the `MAX_AST_DEPTH = 256`
semantic limit on every substrate. Implemented, tested, and documented; no
further human token is required. The historical decision package is preserved at
`docs/AUDIT3_TYPE_NESTING_DECISION.md`.

## Reproduce before fixing

For a suspected regression:

1. observe it;
2. reproduce it;
3. classify it;
4. reduce it to the smallest useful reproducer;
5. identify the governing contract;
6. make the smallest justified fix;
7. add regression coverage when appropriate;
8. run focused validation;
9. perform an adversarial recheck when the surface warrants it.

Do not silently resolve a genuine specification choice. Produce a decision
package instead.

## Long-running workflow

For substantial work, inspect Git and the relevant authority files first,
preserve valid dirty-worktree changes, validate focused surfaces immediately,
and run broader validation at milestone closure. Update `AGENT_STATE.md` when
operational state materially changes. Add detailed chronology to `STATUS.md`
when it is useful for future reconstruction.

Before ending a long session or switching models, leave a continuation-grade
checkpoint with exact tests actually run and the next exact action.

## Model handoff protocol

Before switching model:

1. finish the current atomic operation;
2. ensure the current diff is understood;
3. update `AGENT_STATE.md` if state materially changed;
4. run `scripts/agent-state.sh`;
5. set writer ownership to `NONE` or the explicitly named next model.

After switching model:

1. read `AGENT_STATE.md`;
2. run `scripts/agent-state.sh`;
3. inspect the current diff;
4. verify Git reality;
5. assume writer ownership only when no other agent is actively writing.

Synchronization source: Git + working tree + `AGENT_STATE.md`. Chat history and
chain-of-thought synchronization are not required and must not be treated as
repository authority.

## Reasoning / quota policy

Use the configured efficient model for routine search, tests, formatting,
hashing, documentation sync, and straightforward implementation. Escalate only
when concrete evidence leaves an architectural or semantic question unresolved.

Before an escalation that changes working assumptions, record the exact
unresolved question in the current state/history documentation.

## Subagent policy

Use subagents only for independent, bounded work. Prefer at most two concurrent
subagents. Good uses include locating a code path, checking an independent test
surface, comparing docs with implementation, reviewing an existing diff, or
gathering artifact/hash evidence. Avoid parallel edits to the same files and
duplicate broad audits.

## Validation floor

At major closure, when the touched surfaces require it, run:

- `cargo fmt --all -- --check`
- `cargo test --locked --all-targets --all-features`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time`
- `cargo +1.83.0 check --locked --all-features`
- `node playground/tests/node/run-all.mjs`
- `node playground/build.mjs --check`
- `cargo build --locked --manifest-path playground/runtime/Cargo.toml --release --target wasm32-unknown-unknown`
- `node website/tests/run-all.mjs`

Run focused Python/fuzz/property suites when the touched surface requires them.
Do not call local validation “GitHub CI”.

## Git / artifact discipline

- Preserve valid unrelated dirty-worktree changes.
- Do not reset, clean, restore, or discard work merely to simplify a task.
- Do not force-push.
- Do not push unless the human explicitly authorizes it.
- Do not create releases or tags unless explicitly authorized.
- Do not bump a runtime version or publish a runtime implicitly.
- Build and compare bytes before any authorized runtime publication.
- Preserve every historical artifact.
