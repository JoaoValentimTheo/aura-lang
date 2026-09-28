# Aura — Codex Project Instructions

These instructions apply to the entire repository.

## Objective

Develop and maintain Aura using evidence-driven changes while preserving the verified conformance baseline. Do not restart completed audits unless a concrete regression requires it.

## Current verified baseline

- Branch: `rewrite/v3-rust`
- Verified conformance/hardening baseline: `34eac815b6a56df7bc27f23ed4fb51565df4ff77`
- Full all-features Rust suite at that baseline: 650 passed, 0 failed.
- Selected no-default suite at that baseline: 641 passed, 0 failed.
- CI baseline: run `36390575872`, 14 jobs, success.
- Current development runtime at that baseline: `0.0.2-dev.29`.

Always derive current HEAD/status/artifact state from the repository before relying on these historical values.

## Immutable / human-gated state

Frozen release:
- `playground/runtimes/0.0.2/aura_playground_runtime.wasm`
- size `1,366,621`
- SHA-256 `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`

Historical dev.23:
- size `1,604,958`
- SHA-256 `71072150e67384120c63e22d6176f3683110735b84f74723bea315f79778a528`

Never overwrite historical runtime artifacts.

AUDIT-3 / TypeExpr nesting remains DECISION-PENDING. Never implement Option A/B unless the human provides exactly:
- `DECISION APPROVED: OPTION A`
- `DECISION APPROVED: OPTION B`

Use this exact status sentence when relevant:

`Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.`

## Open specification gaps

Do not silently resolve these as bugs:
- general separator policy / CONF-PARSE-8;
- numeric underscore placement;
- f-string outer edge rules;
- generic-head `::` continuation / CONF-GRAM-4;
- variant-tag uniqueness scope / CONF-RESOLVE-6;
- nested-module visibility / CONF-RESOLVE-7;
- `pub use` re-exports / CONF-RESOLVE-8;
- module aliases / CONF-RESOLVE-9;
- type/variant canonical-name collision semantics / CONF-RESOLVE-10.

## Long-running workflow

For a substantial task:
1. Read `STATUS.md` and the relevant section of `PLANS.md`.
2. Inspect branch, HEAD, git status, diff, and relevant files.
3. Reproduce before fixing.
4. Classify the issue.
5. Make the smallest justified change.
6. Run focused validation immediately.
7. Add permanent regression/corpus/property coverage when appropriate.
8. Update `STATUS.md` at each milestone.
9. Run broader validation only at milestone boundaries.
10. Before ending or compacting, write a continuation-grade checkpoint to `STATUS.md`.

Do not repeatedly reread the whole repository when `STATUS.md`, `PLANS.md`, and git history already establish the state.

## Reasoning / quota policy

Default work uses the configured efficient model and low reasoning.

Escalate only when evidence shows the current configuration is insufficient.

Use GPT-6 Astra only for narrowly scoped difficult work such as:
- ambiguous architecture;
- difficult root-cause analysis;
- complex semantic contradictions;
- final adjudication between well-supported competing hypotheses.

Do not use high/xhigh for file search, test execution, formatting, obvious implementation, documentation sync, hashing, or routine review.

Before escalation, write the exact unresolved question into `STATUS.md`.

## Subagent policy

Subagents are not free parallelism. Use them only for independent, bounded tasks. Prefer at most two concurrent subagents.

Good uses:
- locate a code path;
- inspect an independent test surface;
- compare docs vs implementation;
- review an existing diff;
- gather artifact/hash evidence.

Avoid multiple agents editing the same files or duplicating broad audits.

## Plan mode

For risky or multi-file work, use `/plan` first. Milestones must have acceptance criteria and exact validations.

## Validation floor

At major closure:
- `cargo fmt --all -- --check`
- `cargo test --locked --all-targets --all-features`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time`
- `node playground/tests/node/run-all.mjs`
- `node playground/build.mjs --check`
- `cargo build --locked --manifest-path playground/runtime/Cargo.toml --release --target wasm32-unknown-unknown`
- `node website/tests/run-all.mjs`

Run Python/fuzz/property suites when the touched surface requires them.

Do not call local validation "GitHub CI".

## Git / artifact discipline

- Preserve valid dirty-worktree changes.
- Do not reset/clean/discard work merely to simplify a task.
- Do not force-push.
- Do not create releases/tags unless explicitly authorized.
- Build and compare bytes before any dev-runtime bump.
- Preserve every historical artifact.

## Checkpoint format

Before `/compact`, before a model switch, at a milestone boundary, and before ending a long session, update `STATUS.md` with:
- HEAD / branch / git status;
- current milestone;
- files changed;
- confirmed findings;
- fixes completed;
- tests actually run and exact results;
- artifact state if relevant;
- open spec gaps / decisions;
- next exact action;
- items that must not be reopened.
