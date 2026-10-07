# Keystone Jev Ledger (Aura 0.3)

Mandatory adversarial-reviewer ledger (order §3). Jev is an adversarial
engineering reviewer, not the authority. Verdicts are recorded, not obeyed
blindly; deterministic repository evidence outranks a Jev judgment, and any
such disagreement is recorded here.

Bootstrap state (verified 2026-10-06): branch `rewrite/v3-rust`, HEAD
`8e4a59a1bb08372bd5f628236d275d0c7eb17a6b`, remote
`e576238f7adb52f6ab6d18431602e3fbc2bd3636`, ahead 18 / behind 0, staged empty,
frozen hashes byte-identical, tags `v0.2.1` = `3f5f8702…`, version `0.2.1`,
`.kilo/**` documented churn, `.codex/config.toml` tracked-clean, root `s`
absent.

| # | Phase | Question | Verdict | Disposition |
|---|---|---|---|---|
| J1 | PRE | Is treating most of the Keystone feature list as ALREADY IMPLEMENTED (verify + close narrow gaps) materially wrong — i.e. is at least one assumed-existing feature absent/broken at HEAD? | **0.52** (substantive risk) | Investigated by execution: probes confirmed unions, generics, traits, `impl`, modules, patterns, named args all work; confirmed genuinely absent: `never`, catch selection, unused analysis, structured diag fields, color policy, Aurea, HTTP, AIS, codename metadata. Premise corrections recorded (local `const` in `fn` bodies is NOT valid; eager sibling-module discovery is frozen design, not a defect). |
| J2 | PRE | Is the scope-manifest categorization materially wrong in a way that would misdirect implementation? | **0.47** | Kept the categorization but strengthened it: every "already implemented" row now cites executed probes and/or passing suites; every gap cites a probe or file:line; premises contradicted by evidence are recorded under FALSIFIED. |

| J3 | PRE | Is reusing the existing `Pattern` grammar for the catch clause the right E2 choice, or does it create a semantic dead-end? | **choice=reuse, p=0.71** | Supported the pattern-reuse design; kept the clause count at one pattern (generalizes to multiple clauses later without breaking syntax) and used existing `match_pattern` selection semantics. |
| J4 | POST | (a) How likely is a material semantic defect in the pattern-catch implementation not covered by 23 tests + oracle? (b) E1009 vs a dedicated code for the reserved module name? | **(a) 0.21 low**; **(b) choice=use_new_code, confidence 0.4 (p=0.60)** | (a) accepted; oracle + differential coverage judged sufficient. (b) deterministic repository evidence agreed with Jev: `E1009` is documented as a *value-namespace* reservation, while modules are a separate namespace (ADR-0002); added dedicated `E2023 RESERVED_NAMESPACE`, documented in `docs/errors.md` + website copy, sampled in `tests/grammar.rs`. Disagreement: none (evidence and Jev agreed). |

| J5 | POST | Does the type-system slice have a material soundness defect or an unjustified breaking change: (a) unrequired accept->reject, (b) unsound narrowing, (c) retained `none` union member breaking an invariant the collapse provided? | **0.12 low** | Accepted. Evidence: only earlier-detection transitions at the same codes in regenerated goldens; mutation checks discriminated; spec updated. |

| J6 | POST | Does the Go-like unused analysis (E2008) risk false positives that reject valid documented programs, or break REPL/session semantics, given the evidence: 10 new tests, 108 fixture migrations, generator post-pass, full suite 1171/0, mutation discriminated? | **0.18 low** | Accepted. Two false positives found and fixed during implementation (called-local bindings; shadowed bindings), REPL/module-scope exemption added, duplicate-`_`-parameter behavior verified against a HEAD worktree (E2007 at both revisions, not a regression). |

| J7 | POST | (a) How likely is a material soundness/security/contract defect in AIS + typed JSON decode + HTTP not covered by tests? (b) Is the capability/protocol boundary the right structure to build on? | **(a) 0.43 elevated**; **(b) choice=sound, p=0.76 (fix_first 0.21)** | Investigated the two named structural concerns. (1) AIS advertised `flow`/`completion` capabilities in `Capabilities::default()` while `document()` populates neither — a real overclaim; fixed to `false` for both, with the schema bits reserved for a future producer that actually carries them. (2) HTTP redirect behavior verified with a live 302 server: the layer returns `302` as an ordinary response and does not follow it; added as a permanent regression test. Both findings were Jev-driven; neither required a design change. |

| J8 | POST | Final gate: what concrete evidence could falsify the completion claim, and is the `take(u: U)` acceptance a Keystone regression, a should-fix-now, or a recorded pre-existing limitation? | **(falsifier question) 0.47; (disposition) choice=recorded_limitation, p=0.94** | The falsifier question is open-ended and was resolved by evidence rather than the number: the strongest candidate (`take(u: U)` accepting a possible-`none`) was verified against a pre-Keystone worktree binary to predate Keystone, so it is recorded as TD-21 rather than fixed (tightening it would falsify the documented `T \| none` permissiveness). Independent review found 7 defects, all fixed. No undispositioned material remainder identified. |
| J9 | PRE | Is rejecting statically-unreachable `return` under `-> never` (e.g. `if false { return }`, `throw "x"; return`, `while false { return }`) a material contract defect that should be fixed now? | **0.74** (substantive) | Reproduced by execution against the built CLI: all three shapes were `E3006`, contradicting the spec's *"any reachable `return`"*. Second fresh review's C5 flag (`function_saw_return`) was lexical, not reachability-aware. Fixed with `body_can_complete_normally`; added a six-case regression test. |
| J10 | POST | Is the reachability-aware `-> never` body check sound (never wrongly accepts a normal-completing body, never wrongly rejects a divergent one)? | **0.73** (residual risk) | Investigated the elevated reading by executing six adversarial wrongly-accept shapes (block return, match-arm return, `if`-expression return, try/catch that completes, call to a `-> returning` function, finally + body return): all still `E3006`. Also tightened the `if`-expression scan to treat a diverging condition as making neither branch reachable. No wrong-accept found; three wrong-reject classes fixed. |
| J11 | POST | Does the fourth-review fix set (shadowed-callee divergence, false-divergence narrowing, transitive closure capture, literal conditions, try-finally, empty iterable) leave a material soundness/contract defect in the `-> never` divergence + `none`-narrowing analysis? | **0.30** (elevated residual risk) | The elevated reading drove a fifth adversarial sweep by execution. It found three further **wrong-reject** classes (not wrong-accept): G1, a value position that cannot yield (`let _x = boom()`, `print(boom())`, `r(boom())`, `let _x = { throw }`, a doubly-diverging `if`) was not recognized as divergence; G2, a method declared `-> never` on a statically-known receiver did not prove divergence; G3, a `while` condition or `for` iterable that diverges never enters the body but was not recognized. All were reproduced against the built CLI, fixed (`expr_diverges` now models eager sub-expression divergence and the statically-known method `-> never` case; the loop-header divergence and the corresponding unreachable-body-return are recognized; `block_diverges` is a whole-list scan), and regression-tested. Narrowing soundness controls re-run: short-circuited operands do not prove divergence, value positions that can yield still reject, and a body-level `break` keeps a `for` non-diverging. |
| J12 | POST | After the fifth-review fixes, how likely is a material soundness/contract defect still undiscovered on this surface? | **0.23** (residual) | Accepted with residual risk. A **self-caught soundness hole** was found during this sweep: the first draft of the eager-divergence rule treated a comprehension element/key/filter as guaranteed to evaluate, so `fn f(xs: [int]) -> never { let _y = [boom() for _x in xs] }` was accepted though `xs == []` completes normally (reproduced by execution: `run` printed `none`). Fixed by recognizing only the *iterable* as eager — the element/key/filter run only for yielded, pattern-matching elements and must never prove divergence — with a four-case regression. The remaining conservative cases (dynamically-dispatched method `-> never` on an `Unknown`/union receiver; a diverging statement in an *earlier* `try`-body statement) reject code that is legal rather than admit unsound code, and are pre-existing, not introduced by Keystone. No wrong-accept remains; the narrowing direction stayed sound throughout. |

Jev totals: PRE consultations 4; POST classifications 8; disagreements 0;
Jev-driven extra investigations 8 (all resolved with deterministic evidence;
J4(b) and J7 both changed the implementation; J6-driven checks confirmed two
false-positive fixes and the REPL exemption; J9 changed the `-> never`
reachability analysis; J10 drove the adversarial wrongly-accept sweep; J11
drove the fifth sweep and changed `expr_diverges` and the method arm; J12
accepted the bounded residual after that sweep).

## Closure-campaign consultations (2026-10-07)

During the final closure campaign Jev was consulted twice by the CLI and
**timed out** both times (`jev noul` / `jev choice` produced no result within
60 s in this container; the evaluator's own network path is throttled here,
the same environment fact recorded in `HTTP_ARCHITECTURE.md` §7). Neither
consultation is recorded as a verdict, because a timeout is not a
measurement. The campaign therefore proceeded on deterministic repository
evidence, exactly as AGENTS.md requires ("Reviewer consensus is not
evidence"), and no decision below rests on an unrecorded Jev judgment.

The findings that a Jev consultation had been prepared to triage were decided
by evidence instead:

- **The campaign's `;` premise.** Jev was not needed: `docs/grammar.md`
  (`terminator = NEWLINE | ";"`), `docs/LANGUAGE_SPEC.md` §2.4/§3.7,
  CONF-PARSE-8 (CLOSED) and `tests/syntax_conformance.rs` (`{;;}` accepted,
  `1 2` rejected) all agree, and `[1; 3]` was reproduced as `E1006`. Verdict:
  the premise was wrong; no change made.
- **`json_encode` of unrepresentable kinds.** Reproduced identity collapses
  (`A(1)` and `B(1)` both encoded `1`; `0..3` encoded `null`), and the
  released non-string-map-key rule already established the project's
  anti-collapse stance. Fixed uniformly; decision record
  `JSON_VALUE_ALGEBRA_DECISION.md`.
- **AIS delta on unrelated edits.** Found by a metamorphic property test
  (comment insertion reported every following symbol as `changed`), not by
  Jev. Fixed (`moved` vs `changed`).

Jev availability is exercised as an *optional capability*: `aura mcp` reports
`jev.present` by PATH discovery, and `src/mcp.rs` pins that its absence
changes no semantic output. The differential harness
(`tests/ais_differential.mjs`) is the reproducible structure for a real Jev
(or any evaluator) run once an evaluator is reachable.
