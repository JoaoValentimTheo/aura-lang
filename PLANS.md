# Aura Plans

## Completed phase — Aura Core completion and freeze

Status: COMPLETE (local); publication/CI verification in progress.

The Core completion program (see `STATUS.md` for the commit stack and evidence)
closed every Core milestone: collection type coherence, `map.items()`, list and
map comprehensions, the nine Core syntax/resolver SPEC GAPs, CLI/REPL
completion, the Playground editor completion, documentation/website
synchronization, CI-RELIABILITY-1, the final development runtime, adversarial
remediation, and CONF-PLAY-1 (a stale browser scroll fixture).

`docs/CORE_FREEZE.md` is the status/evidence record for the frozen Core;
`docs/LANGUAGE_SPEC.md` remains the semantic authority.

### Non-goals preserved

- AUDIT-3 is DECISION-PENDING and was not modified.
- No historical runtime artifact was overwritten.
- No new language feature was added beyond the approved Core work.
- Filesystem modules were not started.

## Next major phase — AURA FILESYSTEM MODULE SYSTEM

Not started. Begins only after Core publication closes, and includes:
`mod.aura`, physical file discovery, the module graph, cross-file imports,
cycles, source ownership, path diagnostics, native host loading, and a
browser/virtual-filesystem strategy. It must feed the *same* in-source-style
logical module tree, canonical resolver, checker, and runtime — no second
module semantics.

## Preserved decisions

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.
