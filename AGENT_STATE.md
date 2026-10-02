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
commit SHA. At reconciliation intake (2026-10-02) the committed and pushed HEAD
is `3f5f8702` (`fix(release): preflight must not require the tag to be free
on-tag`), which is also the dereferenced `v0.2.1` tag commit.

## Remote HEAD

`origin/rewrite/v3-rust` = `3f5f8702` (equal to local committed HEAD).

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

POST-v0.2.1 RELEASE RECONCILIATION

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

This reconciliation pass stages local, **unpushed** repository/doc/website
alignment for the released `0.2.1` identity (authority docs, official website,
and a release-version drift guard). It adds no language semantics, no runtime
bytes, and no new release. It is committed locally and awaits human review
before push.

No post-`0.2.1` development line exists yet, and none may be invented here. The
next major engineering direction has not been selected; package management,
browser persistence, LSP, formatter, async, and macros remain deferred.

## Next Exact Action

HUMAN REVIEW OF POST-v0.2.1 STATE + WEBSITE ALIGNMENT BEFORE PUSH.

The `0.2.1` release itself is done. What remains is human review of the local
reconciliation commits (state docs + website `0.2.1` alignment + drift guard),
then an explicit decision whether to push them. Do not push, deploy, tag, or
start a new development program without that authorization.

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
