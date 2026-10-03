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
commit SHA. At stabilization intake (2026-10-02) the committed and pushed HEAD
was `3f5f8702` (`fix(release): preflight must not require the tag to be free
on-tag`), which is also the dereferenced `v0.2.1` tag commit. The stabilization
work and the B-1 correction pass sit in local, unpushed commits on top of it.

## Remote HEAD

`origin/rewrite/v3-rust` = `3f5f8702` (the released `v0.2.1` commit). Local
`rewrite/v3-rust` is ahead of it by the unpushed stabilization commits; nothing
has been pushed by this program.

## Released

**Aura `v0.2.1` is a published, non-draft, non-prerelease GitHub release.**

- Annotated tag object `df755340e008faea402ea7bd774afdaafef0d41f`;
- dereferenced commit `3f5f8702bcfad778a3007b792cc8e270a88f97e8`
  (== `origin/rewrite/v3-rust`);
- published `2026-10-01T20:29:36Z`;
- release identity: release = language = runtime = `0.2.1`; Host ABI `1`;
  Playground API `1`;
- release WASM asset `aura-playground-runtime-0.2.1.wasm`: 1,768,322 bytes,
  SHA-256 `48c456fcda6c50dd6808ccc5f15a0bca4c0b81d7d970172557817decf427cc9e`
  (byte-identical to the in-repository artifact);
- release note: the published body omits the filesystem-module and multi-file
  Playground capabilities that are inside the tag; the local
  `docs/release-notes/v0.2.1.md` documents them now. The published release body
  was not edited (releases are immutable).

## Current Track

B-1R — ENGINE-STACK-INDEPENDENT CALL ENGINE (architecture designed; B-1
reclassified as implementation nonconformance; implementation **not started**)

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
- FSM-P5 — CLOSED REMOTELY
- FSM-P6 — COMMITTED, PUSHED, AND INCLUDED IN `v0.2.1`

All of FSM-P1…P6 are ancestors of the `v0.2.1` tag (verified with
`git merge-base --is-ancestor`). FSM-P5 uses caller-supplied virtual sources
through `InMemorySourceProvider`; FSM-P6 exposes that in the Playground as a
project state model plus the additive Host ABI 1 `aura_project_*` transport and
`aura_run_project`. Host ABI remains 1 and Playground API remains 1.

Native filesystem module acquisition is user-visible in the `0.2.1` binaries:
`aura run` / `aura check` compile a selected file together with its reachable
filesystem module tree (`compile_file_with_mode`,
`NativeFilesystemSourceProvider`). Verified end to end: `main.aura` +
`math.aura` prints `42`; a directory with `pkg/mod.aura` prints `42`; a logical
module owned by both `foo.aura` and `foo/mod.aura` is rejected `E2020`.

## Human Decisions

HD-1…HD-4 are all **CURRENT_RESOLVED** by ADR-0001…0004 (see
`docs/adr/`), implemented, tested, committed, pushed, and CI-green. The old
approval-token gate text is historical and must not be reintroduced.

- ADR-0001 — release and language versions are distinct;
  `LANGUAGE_VERSION <= RELEASE_VERSION`; source line `0.2.1`.
- ADR-0002 — module members may reuse builtin spellings; reservation stays
  scoped to the user-visible value namespace.
- ADR-0003 — CPython support tiers; TESTED = Linux 3.10–3.13, macOS 3.12,
  Windows 3.12.
- ADR-0004 — structural `TypeExpr` nesting counts toward `MAX_AST_DEPTH = 256`
  on every substrate; the native/WASM divergence is removed.

## Program State

The `0.2.1` release train is **closed**: the release is published and its
workflow run (ID `36920912666`) succeeded, including validate, the three
platform builds, and publish. Branch CI (`36920833058`) and the GitHub Pages
deployment (`36920833002`) also succeeded on the release SHA; 27/27 check-runs
green.

## Post-v0.2.1 Stabilization

The post-0.2.1 stabilization campaign (STAGE 0 reality reconstruction through
STAGE 8 performance) executed locally on top of `3f5f8702` and is recorded
here. Stages 0–3 verified the release, reconciled repository authority, and
aligned the official website to the released `0.2.1` identity with a
release-version drift guard (`website/tests/release-version.test.mjs`, run in
the website and Pages CI jobs). Stages 4–8 attacked the language, modules, CLI,
REPL, CPython, WASM, and Playground surfaces; ran native/WASM parity,
contract-sync, hardening, and performance checks.

- **B-1 (OPEN — IMPLEMENTATION NONCONFORMANCE; remediation architecture
  designed, implementation not started) — WASM call-frame trap below the
  512-frame language limit.** Corrected
  classification: this is an implementation defect, not a conflict between
  `LANGUAGE_SPEC` §31.3 and §31.5. Those rules reinforce each other: 512 user
  call frames is a language rule "not a host limitation", and §31.5 forbids
  host stack overflow. On WebAssembly, mainstream recursive shapes exhaust the
  JavaScript engine stack before the interpreter's `depth` counter reaches
  `MAX_CALL_FRAMES = 512` and the guest traps instead of reporting `E4011`.
  Fresh-instance first-trap depths (Node 24 cold / Chromium 153 main thread
  cold / Chromium production Worker): else 387 / ~400 cold / 196; match 459 /
  — / 233; closure 356; module 387; if-chain 419; thin shapes reach 510/511
  except the Worker thin shape (360); the Chromium main thread reaches the
  boundary only after warm-up. The engine stack, not the 4 MiB wasm shadow
  stack, is binding (16 MiB control build: identical; `node
  --stack-size=4000`: full boundary). Present in every released WASM artifact
  (`0.0.2` 317, `0.2.0` 389, `0.2.1` 387 for the else shape) and in HEAD; not
  a post-0.2.1 regression. Traps degrade a reused instance cumulatively (a
  second trap without an intervening success leaves deeper execution failing
  with `memory access out of bounds`) while trivial programs still run; the
  Playground's fresh-Worker-per-run model contains this. The
  contract-preserving remedy is an engine-stack-independent evaluator
  (explicit frame/continuation stack) — a runtime-architecture program, not a
  stabilization fix; any substrate-calibrated cap would change the released
  semantics, which this pass must not do automatically. Decision package:
  `docs/WASM_CALL_FRAME_LIMIT_DECISION.md` (OPEN — Options A/B/C, B
  recommended).
  Coverage added: six native boundary shapes exact at 510/511 in
  `tests/corpus/call-frames/` (else, match, method, closure, module, try);
  cross-substrate safe-depth (150) shape matrix in
  `playground/tests/node/differential.test.mjs`; Worker-path structured
  `E4011` assertion in `playground/tests/node/browser.test.mjs`. WASM
  limit−1/limit/limit+1 assertions are deliberately absent until remediation:
  they cannot pass while the defect exists, and encoding it as an expected
  failure is not acceptable coverage.
- **B-1R design pass (2026-10-02, design only; no runtime change):**
  `docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md` specifies the
  contract-preserving remediation architecture for Option B: selected
  Option E1 (explicit continuation machine over the existing AST; no IR, no
  bytecode VM), with the full state inventory, per-variant expression and
  statement suspension matrices, control-flow propagation rules,
  environment/closure/`self` model, exact `try/catch/finally` semantics
  (15 pending combinations), exact `E4011` accounting (`main` counts; frame
  513 is the first over-limit frame), diagnostic/source-provenance
  preservation, a differential oracle design (same `Module`, two fresh
  `Interp`s, feature-gated test-only engine switch), a reuse-first corpus,
  phased migration B-1R2…B-1R8, rollback strategy, and a risk register. The
  minimum viable change is the explicit continuation machine itself; a
  substrate-calibrated cap remains rejected as a contract change. Design
  verification against the code corrected several draft misstatements (a
  throw-code number, match/no-match code, two evaluation-order details, the
  `finally` replacement rule, and statement/pattern nesting bounds). An
  independent read-only design review was performed; its substantiated
  findings were incorporated. Also recorded as an adjacent coverage gap
  (independent of B-1): deeply nested list *patterns* are bounded only by the
  substrate-calibrated parser backstop (native accepts ~2000, WASM ~700),
  and `bind_pattern`/`match_pattern` recurse per pattern level. No runtime
  behavior changed; `v0.2.1` and all frozen artifacts untouched.
- All other stabilization findings classified as NON-ISSUE, EXPECTED
  DOCUMENTED LIMITATION (eager filesystem-module discovery of malformed
  reachable siblings; case-insensitive filesystem artifacts), or HISTORICAL
  behavior; no other unresolved CRITICAL/HIGH defect was reproduced.

This reconciliation pass stages local, **unpushed** repository/doc/website
alignment for the released `0.2.1` identity (authority docs, official website,
a release-version drift guard), the B-1 decision package and guards, and the
B-1R design document (`docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`). It
adds no language semantics, no runtime bytes, and no new release. It is
committed locally and awaits human review before push.

No post-`0.2.1` development line exists yet, and none may be invented here. The
next major engineering direction has not been selected; package management,
browser persistence, LSP, formatter, async, and macros remain deferred.

## Next Exact Action

1. HUMAN REVIEW OF THE ENGINE-STACK-INDEPENDENT EVALUATOR DESIGN
   (`docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`) BEFORE AUTHORIZING
   B-1R2 IMPLEMENTATION.
2. HUMAN REVIEW OF POST-v0.2.1 STABILIZATION STACK BEFORE PUSH.
3. B-1 POLICY DECISION: Option A/B/C in
   `docs/WASM_CALL_FRAME_LIMIT_DECISION.md` (Option B recommended; its
   implementation contract is the design document above).

The `0.2.1` release itself is done and unchanged. What remains is human review
of the local, unpushed stabilization commits (state docs + website `0.2.1`
alignment + release-version drift guard + B-1 correction pass: reclassified
finding, native boundary corpus, cross-substrate shape matrix, Worker boundary
assertion) and of the B-1R evaluator design, then an explicit decision whether
to push them or authorize B-1R2. Do not push, deploy, tag, change runtime
behavior, or start a new development program without authorization.

## Release Immutability

- `0.0.2` — 1,366,621 bytes —
  `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`
- `0.2.0` — 1,654,161 bytes —
  `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc`
- `0.2.0-dev.1` — 1,767,068 bytes — superseded development runtime, NOT a
  release:
  `ba40e89c834896badfb17d5c72aa2dcb227907a7b5ba513c315ef2f2da0adf08`
- `0.2.0-dev.2` — 1,767,723 bytes — superseded development runtime, NOT a
  release:
  `b69f212bf3f1d8df41b66ad249bf9c9829015b06459569fd2b765a5596b66c06`
- `0.2.1-dev.1` — intermediate development build, predates the ADR-0004 parser
  change; preserved on disk, unlisted, NOT a release.
- `0.2.1-dev.2` — 1,767,962 bytes — intermediate development build, predates
  the train-1 parser hardening; preserved on disk, unlisted, NOT a release.
- `0.2.1-dev.3` — 1,768,535 bytes — intermediate development build, predates
  the red-team re-verification checker fixes; preserved on disk, unlisted.
- `0.2.1-dev.4` — 1,768,270 bytes — intermediate development build, predates
  the TD-15 diagnostic-location change; preserved on disk, unlisted.
- `0.2.1-dev.5` — 1,768,369 bytes — development runtime; superseded by the
  `0.2.1` release, preserved on disk:
  `8f8c3e1689fc7fb1472293d610c9392d5ea2b7fe106b50f0c7a5535dfc54bc19`
- `0.2.1` — 1,768,322 bytes — **published release runtime** (tag `v0.2.1`,
  GitHub release published 2026-10-01):
  `48c456fcda6c50dd6808ccc5f15a0bca4c0b81d7d970172557817decf427cc9e`

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
- FSM-P5;
- FSM-P6.

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
- a new release or version line;
- a new FSM phase.

The next major engineering direction is **not selected**. Do not promote a
deferred candidate into a follow-up phase by assumption.

## Handoff

Before a model switch, finish the atomic operation, understand the diff, update
this file when state materially changes, run `scripts/agent-state.sh`, and hand
writer ownership to `NONE` or the explicit next model. After a switch, read
this file, run the preflight, inspect the diff, and verify Git before writing.

Repository synchronization does not rely on chat memory or chain-of-thought.

## Writer

NONE
