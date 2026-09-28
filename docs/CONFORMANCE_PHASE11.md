# AURA LANGUAGE CONFORMANCE PASS — PHASE 11

## 1. Scope

Reconciliation of user-facing and normative documentation with verified
implementation behavior, after the semantic and runtime investigation of
Phases 2–10.

Documentation authority hierarchy (unchanged from Phase 1):

1. `docs/LANGUAGE_SPEC.md` — normative.
2. `docs/grammar.md` — formal/canonical syntax.
3. `docs/contract.md` — compatibility promises, defers to the spec.
4. guides and website references.
5. roadmap and historical reports.
6. RFCs.

## 2. Reviewed and Found Accurate

| Document | Check | Result |
|---|---|---|
| `docs/LANGUAGE_SPEC.md` §9.1 | arithmetic result types and the static/runtime split | matches `CONF-TYPE-1` fix (mixed → float; arithmetic type errors are runtime) |
| `docs/LANGUAGE_SPEC.md` §12 | ordering is statically checked when provable | matches implementation |
| `docs/LANGUAGE_SPEC.md` §8 | assignability matrix | every cell reproduced |
| `docs/LANGUAGE_SPEC.md` §18.1/§27 | variant tag uniqueness vs canonical `path::tag` | tension recorded as a SPEC GAP (`CONF-RESOLVE-6`) |
| `docs/LANGUAGE_SPEC.md` §27 | variant named by enum path + tag; `use … as Alias` | matches the fixed resolver |
| `docs/GENERICS.md` | type parameter may not shadow a declared type | matches the `CONF-RESOLVE-5` fix |
| `docs/errors.md` | documented codes are reachable; no undocumented user code | matches (E4099/E5003 internal/unused) |
| `docs/grammar.md` | `let` destructuring rejects qualified variant paths | matches |
| `docs/contract.md` | permissive `Unknown` boundary, no implicit int↔float | matches |

## 3. Corrections

No new documentation drift was found beyond Phase 1's reconciliation. The
Phase 2/3 fixes were changes *toward* the already-normative text (mixed numeric
promotion, the type-parameter shadow rule, §27 variant naming), so no spec edit
was required — the documents already stated the intended behavior and the code
now matches them.

The only documentation additions are the new `docs/CONFORMANCE_PHASE*.md`
reports, which state current behavior and explicitly mark unresolved items.

## 4. Overclaim Check

* No report calls a SPEC GAP fixed.
* No report converts observed behavior into normative syntax.
* Executive summaries distinguish CONFIRMED FIXED BUG, SPEC GAP, DESIGN
  LIMITATION, PLATFORM DIVERGENCE, DECISION-PENDING, RFC CANDIDATE, CI
  RELIABILITY, and NON-ISSUE.
* Historical audit/release documents were not rewritten to pretend they always
  contained current behavior.

## 5. Code Examples

Every example in the new reports was executed (the reports' "Observed" columns
are real `aura run`/`aura check` output), so no report example is unverified.

## 6. Findings

**No new DOC DRIFT.** The Phase-1 authority model holds; the Phase 2/3 fixes
align code to the existing normative text.

## 7. Phase-12 Handoff

Documentation states verified behavior. The only unresolved items are the
Phase-1 SPEC GAPs, the Phase-2 `CONF-RESOLVE-6…10` resolution questions, and
the AUDIT-3 decision — all explicitly marked.
