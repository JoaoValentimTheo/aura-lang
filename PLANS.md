# Aura Final Implementation / Release-Readiness Plan

## Active Objective

Transition Aura from conformance/hardening into final implementation and
release readiness: derive what is genuinely unfinished, separate implementation
from design, implement only evidence-backed unfinished work, validate it, and
preserve every existing invariant. Do not create a release/tag/deploy.

## Non-goals

- Do not reopen completed audits without a reproduced regression.
- Do not resolve AUDIT-3 or the nine documented SPEC GAPs.
- Do not implement RFC-candidate features whose current behavior is already
  normatively specified.
- Do not overwrite any historical runtime artifact.
- Do not create releases/tags or deploy.

## Milestones

### M1 — Forensic resume
Status: COMPLETE

Acceptance met: `pwd`, `git status --short`, branch, HEAD, log, `git diff --check`,
`git diff`, and untracked inventory derived first. Branch `rewrite/v3-rust`,
HEAD `34eac815b6a56df7bc27f23ed4fb51565df4ff77`, no tracked changes; pre-existing
untracked workflow files preserved.

Validation: `git status --short`, `git rev-parse HEAD`, `git log --oneline --decorate -20`.

### M2 — Parallel reconnaissance
Status: COMPLETE

Acceptance met: four independent read-only investigations (implementation
completeness, spec-vs-gap classification, artifact/version verification,
unfinished-work scouting). Findings: no production implementation defect; all
unfinished constructs are documented SPEC GAPs or deliberate limitations. The
only non-design-gated work is documentation drift / release-readiness surface
inconsistency.

Validation: focused CLI reproductions of the SPEC-GAP behaviors; artifact hash
recomputation; spec/doc cross-reads.

### M3 — Final implementation backlog
Status: COMPLETE

Category A (implement now): documentation-drift corrections in
`docs/LANGUAGE_SPEC.md`, `docs/contract.md`, `website/content/reference-errors.md`,
`README.md`, `CONTRIBUTING.md`; CLI unknown-command hint. Category B (human
design required): the nine SPEC GAPs, AUDIT-3, and the four RFC candidates.
Category C (already complete / do not reopen): all conformance surfaces,
error-code reachability, version reporting, artifact manifest.

### M4 — Implement Category A
Status: COMPLETE

Acceptance met: two coherent commits, `81e7410` (docs) and `bacba57` (cli +
regression). No language policy defined; no normative rule invented beyond
removing a self-contradiction and adding already-normative codes to tables.

Validation: focused doc-coupled tests (`syntax_docs`, `grammar`, `contract`,
`contract_sync`, `syntax_conformance`) all pass.

### M5 — Validation and adversarial review
Status: COMPLETE

Acceptance met: full validation floor run; independent adversarial review
performed. The review falsified one claim and raised two minors, all fixed and
re-validated.

Validation: see `STATUS.md` "Validation Actually Run". WASM rebuild byte-identical
to dev.29, so no artifact bump.

### M6 — Durable checkpoint
Status: COMPLETE

Acceptance met: `STATUS.md` records the release-readiness state, exact commits,
validation results, artifact state, preserved decisions, and next action.

## Preserved Decisions

Nine documented specification gaps remain unresolved. Completed audits stay
closed without a reproduced regression.

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

## Next Action

Await a concrete user-authorized development objective or explicit
release/tag/deploy authorization; this plan is complete.
