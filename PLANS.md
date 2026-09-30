# Aura Plans

## Completed phase — Aura Core completion, freeze, and 0.2.0 release

Status: COMPLETE. The Core is released as the public **0.2.0**.

The Core completion program (see `STATUS.md` for the commit stack and evidence)
closed every Core milestone: collection type coherence, `map.items()`, list and
map comprehensions, the nine Core syntax/resolver SPEC GAPs, CLI/REPL
completion, the Playground editor completion, documentation/website
synchronization, CI-RELIABILITY-1, the final development runtime, adversarial
remediation, and CONF-PLAY-1 (a stale browser scroll fixture). The version
authority was then migrated from the `0.0.x` line to the public `0.2.0`
release, a new `0.2.0` runtime artifact was built and verified, and the
Playground and website now present `0.2.0` as the current stable release.

`docs/CORE_FREEZE.md` is the status/evidence record for the frozen Core;
`docs/LANGUAGE_SPEC.md` remains the semantic authority.

### Non-goals preserved

- AUDIT-3 is DECISION-PENDING and was not modified.
- No historical runtime artifact was overwritten.
- No new language feature was added beyond the approved Core work.
- Filesystem modules were not part of the Core release itself; the subsequent
  filesystem-module program has since completed its native foundation and
  virtual/WASM source foundation through FSM-P5 locally.

## Current major program — AURA FILESYSTEM MODULE SYSTEM

Current filesystem-track status:

- FSM-P1 — design — complete;
- FSM-P2 — multi-source provenance — complete;
- FSM-P3 — provider-neutral graph — complete;
- FSM-P4 — native filesystem provider — complete and remotely closed;
- FSM-P5 — virtual/WASM VFS foundation — complete and remotely closed;
- FSM-P6 — multi-file Playground UX — implemented locally, under review.

FSM-P5 lets browser/WASM callers supply virtual multi-source projects through
`InMemorySourceProvider` and the same `ModuleGraphBuilder`, resolver, checker,
and runtime used by native modules.

FSM-P6 exposes that capability in the browser: the Playground holds a project
(a flat set of files, an active file, an entry file, and each file's declared
provider child links) and executes it through the same virtual-project
transport. It is a UI/state/transport phase — no language semantics were added,
and no second module graph or resolver exists for browser sources.

A development runtime, `0.2.0-dev.2`, carries the additive Host ABI 1
virtual-project exports so the surface can be exercised end to end. It is a
development artifact: not a release, not tagged, and never a replacement for
`0.2.0`.

Still deferred, and not part of FSM-P6: browser persistence, package
manifests/package management, remote dependency resolution, URL imports, a
visual module-ownership tree, and interactive child-link editing.

## Preserved decisions

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.
