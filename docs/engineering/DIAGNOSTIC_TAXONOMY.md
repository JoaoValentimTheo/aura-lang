# Aura Diagnostic Taxonomy (Pre-0.3 Foundation)

Status: audit record of the code taxonomy as implemented. This file adds no
codes and renumbers nothing. Authority for the public table: `docs/errors.md`
(and its byte-synced website copy). Authority for declarations:
`src/error.rs::codes`.

---

## 1. Inventory

- **46 declared constants**, **45 distinct numeric codes**: `PY_UNSUPPORTED`
  and `CAPABILITY_UNAVAILABLE` intentionally share `5002` (both mean "this
  build or host cannot provide the requested capability"), documented at
  `src/error.rs:326-330`.
- One code is mechanically asserted reachable: `tests/grammar.rs` samples one
  program per public code and compares the produced code.
- `docs/errors.md` documents every code; two non-public codes (`E4099`
  internal, `E5003` reserved) have an explicit "Internal and reserved"
  section so no declared code is undocumented.

## 2. Categories (phase = first digit)

| Range | Category | Phase | Catchable | Owner |
|---|---|---|---|---|
| `E1xxx` | lexical / syntactic | compile | no | lexer, parser |
| `E2xxx` | name and rule checking | static check | no | checker, module graph |
| `E3xxx` | type-level | static check | no | checker |
| `E4xxx` | runtime | execute | no (fatal) | evaluator, host |
| `E5xxx` | capability / feature | execute | no (fatal) | host, bridge |
| `E4099` | internal signal | execute | never visible | evaluator |
| `E4999` | internal error | any | never | any invariant |

## 3. Uniqueness and stability

- Codes are unique per meaning; the single intentional numeric sharing is
  documented above and does not create ambiguity (both are "unavailable"
  conditions with the same disposition).
- **No renumbering without compelling compatibility evidence.** The codes are
  a public contract (`docs/errors.md` Stability). A future semantic change
  gets a new code, never a reused one.
- `E4099` and `E5003` are declared-but-not-public; they may be removed only
  with the same care as a public removal (they appear in `src/error.rs` and
  in the spec's internal table).

## 4. Severity and disposition

There is one severity in the current model: a diagnostic terminates the
program (or compilation). "Warning" severity does not exist in the emitted
language contract (the checker's unused-parameter signal is complete
rejection, not a warning). A future warning channel is a language-design
decision, not a taxonomy cleanup.

## 5. Span / attribution

- Diagnostics carry `Span` and, when crossing source boundaries, a
  `SourceDiagnostic { diagnostic, source }` pair.
- Provider-level failures (module ownership, invalid keys, duplicate logical
  sources) are **locationless**: no Aura source text is at fault
  (`E2020`/`E2021`/`E2022`).
- Depth diagnostics are attributed to the offending node (TD-15 closed).
- Fatal diagnostics produced while unwinding preserve the original throw
  location; a `finally`-raised diagnostic supersedes with its own span.

## 6. Documentation coverage

- Every public code appears in `docs/errors.md` with a meaning and an example
  trigger; the website reference is byte-synced (test-pinned).
- `tests/grammar.rs::errors_doc_lists_every_code` checks that each sampled
  code appears in the doc; `every_documented_error_code_is_reachable` checks
  producibility.

## 7. Separation from the exception model

Per `docs/engineering/EXCEPTION_ARCHITECTURE.md` §2:

- compile/check diagnostics (no program running) — never catchable;
- runtime operation diagnostics — fatal, never catchable;
- host/resource/capability — fatal, never catchable;
- internal — never visible;
- **catchable** language exceptions are a separate mechanism (explicit
  `throw` today) with their own value path, and a future family taxonomy must
  map onto *that* mechanism, not onto diagnostics.

A family name must never collide with a diagnostic code number space; codes
stay `E<n>`, exception families are names/tags.

## 8. Findings from this audit

| Finding | Disposition |
|---|---|
| `MAX_AST_DEPTH` was triplicated | Fixed in `2fd1433` (single source, re-export) |
| `E4099`/`E5003` undocumented | Fixed: explicit internal/reserved section in `docs/errors.md` |
| `E5002` shared by Python and capability | Intentional and documented; kept |
| `E5003` unused | Tracked TD-07; reserved, kept |
| duplicate website error reference | Fixed in `c1ffb18` (sync test in `tests/syntax_docs.rs`) |
| no diagnostic renumbering pressure found | No action |
