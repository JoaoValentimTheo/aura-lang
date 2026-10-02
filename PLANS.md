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
Playground and website were updated to present `0.2.0` as the then-current
stable release (later superseded by `0.2.1`; see below).

`docs/CORE_FREEZE.md` is the status/evidence record for the frozen Core;
`docs/LANGUAGE_SPEC.md` remains the semantic authority.

### Non-goals preserved

- AUDIT-3 is DECISION-PENDING and was not modified.
- No historical runtime artifact was overwritten.
- No new language feature was added beyond the approved Core work.
- Filesystem modules were not part of the Core release itself; the subsequent
  filesystem-module program has since completed its native foundation and
  virtual/WASM source foundation through FSM-P5 locally.

## Completed program — AURA FILESYSTEM MODULE SYSTEM (released in v0.2.1)

The filesystem-module program is complete and shipped:

- FSM-P1 — design — complete;
- FSM-P2 — multi-source provenance — complete;
- FSM-P3 — provider-neutral graph — complete;
- FSM-P4 — native filesystem provider — complete and remotely closed;
- FSM-P5 — virtual/WASM VFS foundation — complete and remotely closed;
- FSM-P6 — multi-file Playground UX — **committed, pushed, and included in
  `v0.2.1`**.

FSM-P5 lets browser/WASM callers supply virtual multi-source projects through
`InMemorySourceProvider` and the same `ModuleGraphBuilder`, resolver, checker,
and runtime used by native modules.

FSM-P6 exposes that capability in the browser: the Playground holds a project
(a flat set of files, an active file, an entry file, and each file's declared
provider child links) and executes it through the same virtual-project
transport. It is a UI/state/transport phase — no language semantics were added,
and no second module graph or resolver exists for browser sources.

Native filesystem module acquisition is also user-visible in `v0.2.1`:
`aura run`/`aura check` compile a selected file together with its reachable
filesystem module tree. The release runtime `0.2.1` carries the additive Host
ABI 1 virtual-project exports; superseded development runtimes
(`0.2.0-dev.1`/`0.2.0-dev.2`, `0.2.1-dev.1`..`.5`) remain preserved historical
artifacts.

## Next major program — NOT SELECTED

`v0.2.1` is published. No successor engineering program has been chosen, and
nothing below is authorized. Explicitly deferred until a human selects a
direction:

- browser persistence (localStorage/URL project sharing);
- package manifests / package management / remote dependency resolution / URL
  imports;
- LSP or a formatter product;
- async;
- macros;
- a new release or version line.

Do not begin any of them by assumption.

## Preserved decisions

AUDIT-3 / HD-1…HD-4 are resolved (ADR-0001…0004) and must not be reopened or
re-queued. The TypeExpr property-test exclusion was lifted once ADR-0004
landed.
