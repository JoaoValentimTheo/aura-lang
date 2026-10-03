# Aura — Repository Agent Instructions

These instructions apply to the entire repository and to every agent or model
that works on it, regardless of provider. They are durable operating rules, not
a project report: current state and active work live in `AGENT_STATE.md` and
`docs/engineering/CURRENT_HANDOFF.md`.

## Objective

Develop and maintain Aura from repository evidence. Preserve verified behavior,
historical artifacts, and human-gated decisions. Do not reconstruct current
state from chat memory.

## The repository is the memory

Conversation history is non-authoritative and may be unavailable. An agent must
be able to resume correctly from the repository alone, by reading, in order:

1. `AGENTS.md` (this file) — operating rules;
2. `AGENT_STATE.md` — compact current engineering state;
3. `docs/engineering/CURRENT_HANDOFF.md` — the active task and exact next
   action;
4. the specific design/decision document the task references.

Nothing operational should depend on a particular chat implementation.

## Startup protocol

Before substantial work:

1. read `AGENT_STATE.md` and `docs/engineering/CURRENT_HANDOFF.md`;
2. inspect the current branch;
3. inspect local `HEAD`;
4. inspect `origin/rewrite/v3-rust`;
5. inspect `git status --short`, the staged state, and the protected local state;
6. compare Git reality with `AGENT_STATE.md`;
7. if they conflict, Git wins and state must be reconciled before continuing.

`STATUS.md` is detailed chronology; `PLANS.md` is future planning. Read them
only when their content is needed.

## Authority

One authoritative answer for each question, nothing duplicated across docs:

| Question | Authority |
|---|---|
| Operating rules every agent obeys | `AGENTS.md` |
| Normative language & architecture behavior | `docs/LANGUAGE_SPEC.md`, accepted `docs/adr/*` |
| Frozen filesystem-loading design contract | `docs/FILESYSTEM_MODULES_DESIGN.md` |
| Current repository state | `AGENT_STATE.md` |
| Active task + exact next action | `docs/engineering/CURRENT_HANDOFF.md` |
| Higher-level project/release status | `STATUS.md` |
| Future authorized work | `PLANS.md` |
| Decisions & historical rationale | `docs/adr/*`, decision packages, historical checkpoints |

No two current-authority documents may disagree. Git and the working tree are
authoritative for repository reality.

## Single writer

The lead model is the only writer of the worktree in a given session. Delegated
or self-spawned reviewers are **read-only**: they must not edit files, stage,
commit, push, reset, restore, clean, delete, or otherwise mutate Git state.
A future human authorization may change this, but it must be explicit.

## Evidence

Reviewer consensus is not evidence. Repository state, Git history, executable
tests, specifications, and reproduced commands are authoritative. Any reviewer
finding that triggers a change must be independently reproduced by the writer.

## Scope discipline

Do the task. Do not opportunistically refactor unrelated production code, and do
not convert a bounded task into a broad campaign. If a genuine blocker appears,
stop and report it rather than improvising.

## Context safety

When context pressure or degraded reasoning is detected: do not rush; do not
silently reduce validation; stop at a coherent boundary; update `AGENT_STATE.md`
and `CURRENT_HANDOFF.md`; and report the exact continuation action.

## Phase namespaces

Do not infer one work series from another:

- `CONFORMANCE_PHASE<N>` = historical conformance audit series;
- `FEATURE_<NNN>` = feature development series;
- `FSM-P<N>` = filesystem module system series;
- `B-1R<N>` = WASM call-frame evaluator remediation series.

Never use bare “Phase N” for the filesystem-module program.

## Immutable / protected artifacts

Historical runtime artifacts are immutable. Never overwrite a versioned artifact
under `playground/runtimes/`, never move a release tag, and never create a
release or bump a version without explicit human authorization. Current frozen
identities and hashes are listed in `AGENT_STATE.md`.

`.kilo/**`, `.codex/**`, and other tool configuration are not engineering
authority. User-local or ambiguous tool state is preserved, not deleted.

## Reproduce before fixing

For a suspected regression:

1. observe it; 2. reproduce it; 3. classify it; 4. reduce it to the smallest
   useful reproducer; 5. identify the governing contract; 6. make the smallest
   justified fix; 7. add regression coverage when appropriate; 8. run focused
   validation; 9. perform an adversarial recheck when the surface warrants it.

Do not silently resolve a genuine specification choice. Produce a decision
package instead.

## Long-running workflow

For substantial work, inspect Git and the relevant authority files first,
preserve valid dirty-worktree changes, validate focused surfaces immediately,
and run broader validation at milestone closure. Update `AGENT_STATE.md` and
`CURRENT_HANDOFF.md` when operational state materially changes. Add detailed
chronology to `STATUS.md` when it is useful for future reconstruction.

Before ending a long session or switching models, leave a continuation-grade
checkpoint with the exact tests actually run and the next exact action.

## Model handoff protocol

Before switching model: finish the current atomic operation; ensure the diff is
understood; update `AGENT_STATE.md`/`CURRENT_HANDOFF.md` if state changed; run
`scripts/agent-state.sh`; set writer ownership to `NONE` or the named next model.

After switching model: read `AGENT_STATE.md` and `CURRENT_HANDOFF.md`; run
`scripts/agent-state.sh`; inspect the diff; verify Git reality; assume writer
ownership only when no other agent is actively writing.

Synchronization source: Git + working tree + `AGENT_STATE.md`. Chat history and
chain-of-thought are not repository authority.

## Reasoning / quota policy

Use the configured efficient model for routine search, tests, formatting,
hashing, documentation sync, and straightforward implementation. Escalate only
when concrete evidence leaves an architectural or semantic question unresolved.
Before an escalation that changes working assumptions, record the exact
unresolved question in the current state/history documentation.

## Review / orchestration protocol

The lead agent is the **sole writer** of the worktree. Fresh reviewer sessions —
whether spawned in parallel or run sequentially — are **read-only
investigators**: they may read, search, and run read-only commands, but must not
edit, stage, commit, push, reset, restore, clean, or delete anything.

- Reviewer roles may cover architecture, semantics/spec, tests/oracle,
  performance, adversarial falsification, and a final gate.
- The lead independently reproduces every finding before acting on it.
- Give each reviewer only the minimum context its task needs; do not send the
  whole project history to every reviewer.
- Do not use multiple reviewers for trivial deterministic checks.
- Spend expensive parallel review at milestone boundaries, not after every small
  edit. If parallel subagents are unavailable, run fresh sequential review
  sessions instead — the protocol degrades cleanly and must not be faked.
- Prefer at most two concurrent subagents. Avoid parallel edits to the same
  files and duplicate broad audits.

## Context-budget policy for long programs

Decompose a milestone into small microphases, each completed as: implement →
targeted tests → oracle/differential check → checkpoint (`AGENT_STATE.md` +
`CURRENT_HANDOFF.md`). Do not create a large planning document per microphase;
a compact handoff is enough. Escalate to multi-reviewer analysis at milestone
closure or when an architectural/semantic question is genuinely unresolved.

## Validation floor

At major closure, when the touched surfaces require it, run:

- `cargo fmt --all -- --check`
- `cargo test --locked --all-targets --all-features`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time`
- `cargo +1.83.0 check --locked --all-features`
- `cargo +nightly check --manifest-path fuzz/Cargo.toml --all-targets`
- `node playground/tests/node/run-all.mjs`
- `node playground/build.mjs --check`
- `node website/build.mjs` and `node website/tests/run-all.mjs`

Run focused Python/fuzz/property suites when the touched surface requires them.
Do not call local validation “GitHub CI”.

## Git / artifact discipline

- Preserve valid unrelated dirty-worktree changes.
- Do not reset, clean, restore, or discard work merely to simplify a task.
- Do not force-push, and do not rewrite published history.
- Do not push unless the human explicitly authorizes it.
- Do not create releases or tags unless explicitly authorized.
- Do not bump a runtime version or publish a runtime implicitly.
- Build and compare bytes before any authorized runtime publication.
- Preserve every historical artifact.
