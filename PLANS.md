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
- FSM-P5 — virtual/WASM VFS foundation — locally closed, not pushed;
- FSM-P6 — not authorized and not yet defined.

FSM-P5 lets browser/WASM callers supply virtual multi-source projects through
`InMemorySourceProvider` and the same `ModuleGraphBuilder`, resolver, checker,
and runtime used by native modules.

The current single-file Playground UI remains unchanged. Multi-file editor UI,
browser persistence, package manifests/package management, remote dependencies,
and URL imports remain deferred candidates. None of them defines FSM-P6 unless
the human explicitly chooses that scope. No second module graph or resolver is
planned for browser sources.

## Preserved decisions

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.
