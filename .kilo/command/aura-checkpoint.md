---
description: Produce a handoff checkpoint for another model or session.
agent: aura-orchestrator
---

Produce a continuation-grade CHECKPOINT for the current Aura task. Include:
HEAD, branch, git status, changed/untracked files, task scope, confirmed
findings and their IDs, fixes already applied, tests actually run with exact
results, artifact/version state, AUDIT-3 state, unresolved SPEC GAPs and
decisions, CI state, and the exact next action.

Keep test categories separate (Rust `#[test]`, property iterations, generated
differential cases, corpus fixtures, fuzz inputs, Node tests, browser tests,
documentation examples). Do not perform new edits.
