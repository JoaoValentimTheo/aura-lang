# AURA TOTAL HARDENING — INDEPENDENT ADVERSARIAL REVIEW

Reviewer role: independent adversarial reviewer (did not author the hardening).
Mode: read-only until a defect was independently reproduced and root-caused;
one additive local remediation commit was then made (no push).

---

## 1. Executive verdict

**FINAL: AURA TOTAL HARDENING INDEPENDENTLY VERIFIED — READY FOR PUSH REVIEW**
(after one reviewer remediation; see §41).

The three hardening commits after FSM-P6 (`c8ded06`, `2b7bd5e`, `d7105ac`) are
additive descendants of `4b9d16b`, semantically coherent, and survive
independent falsification. Frozen artifacts are byte-identical. The builtin
reservation fix targets the true root cause and is complete for the global and
local value namespace.

One HIGH integration regression was independently found and fixed:

* `playground/tests/node/differential.test.mjs` still used a `let values`
  fixture (`values` is a builtin) after the same fixture in `tests/maps.rs` was
  renamed. This made `node playground/tests/node/run-all.mjs` and
  `node website/tests/run-all.mjs` fail at the differential gate against the
  current native engine.

One MEDIUM documentation inconsistency was independently found and fixed:
the in-file `LANGUAGE_SPEC.md` §30.2 E1009 row contradicted the new normative
builtin-reservation rule in the same file.

One cross-namespace observation (INFORMATIONAL / possible future human
decision) remains: a **module member** named after a builtin legitimately
shadows the builtin only inside its own module (`module M { pub fn sum }`),
while the same spelling is rejected in the global value namespace. This is
consistent with the module-member namespace being separate; it does not
contradict enforced law, but it is worth a human note.

No CRITICAL finding. No unremediated in-scope correctness blocker remains.

---

## 2. Repository state

* Branch: `rewrite/v3-rust`
* BASE (FSM-P6 locally complete): `4b9d16b`
* CURRENT HEAD at review start: `d7105ac`
* CURRENT HEAD at review end: `f416201` (reviewer remediation)
* Worktree: clean except protected untracked `?? s` (1 byte, SHA-256
  `043a718774c572bd8a25adbeb1bfcd5c0256ae11cecf9f9c3f925d0e52beaf89`);
  never staged, deleted, or modified.
* `git diff --check 4b9d16b..HEAD`: clean.
* `git merge-base --is-ancestor 4b9d16b HEAD`: YES (additive descendant).

## 3. Baseline and reviewed HEAD

`git log --oneline 4b9d16b..HEAD`:

```
f416201 fix(review): align E1009 fixtures and internal code table ...  (reviewer)
d7105ac chore(fuzz): sync aura-lang version pin 0.0.2 -> 0.2.0
2b7bd5e docs(spec): E1009 row covers builtin-name reservation
c8ded06 fix(checker): reject builtin names in user value bindings
```

FSM-P6 commits (`3abf433`, `2f84704`, `c07c1b3`, `9155716`, `4b9d16b`) intact;
none amended, rebased, squashed, or reset.

## 4. Frozen artifact verification

| Artifact | Size | SHA-256 | Match |
|---|---|---|---|
| `0.0.2/aura_playground_runtime.wasm` | 1,366,621 | `5a4ad3f7…b334ed` | YES |
| `0.2.0/aura_playground_runtime.wasm` | 1,654,161 | `9937fd80…295c5bc` | YES |

`git diff --name-status 4b9d16b..HEAD -- playground/runtimes/` is empty: no
hardening commit touched any runtime artifact. Both re-hashed again after the
reviewer commit: unchanged. No BLOCKER.

Development runtime `0.2.0-dev.1`: size 1,767,068; SHA-256
`ba40e89c834896badfb17d5c72aa2dcb227907a7b5ba513c315ef2f2da0adf08` — matches
the recorded identity, and is **not** byte-overwritten by the hardening.

## 5. Authority-map verification

Verified against the repo, not the prior report:

* Normative semantics: `docs/LANGUAGE_SPEC.md` (self-declared normative).
* `docs/contract.md` explicitly defers: “If the two disagree,
  `LANGUAGE_SPEC.md` is authoritative for semantics and this file is out of
  date” (`docs/contract.md:3-7`).
* Filesystem loading design contract: `docs/FILESYSTEM_MODULES_DESIGN.md`.
* Authority ordering matches `AGENT_STATE.md` and `AGENTS.md`.

Discrepancy found and fixed: the `LANGUAGE_SPEC.md` §30.2 code table lagged the
normative rule added to the same file (see §32 / §41). Historical
`CONFORMANCE_*`, `FEATURE_*_REPORT`, `AUDIT3_TYPE_NESTING_DECISION.md` remain
historical; none is cited as authority by the hardening.

## 6. Commit-by-commit review

### `c8ded06` fix(checker): reject builtin names in user value bindings — JUSTIFIED
Enforces E1009 at declaration in `src/check/mod.rs` (`hoist` for user `fn` and
top-level `let`; `declare_inner` for local/param/loop/catch/pattern/lambda) and
in `src/resolve.rs` for value-namespace import aliases; relocates the
`lookup()` builtin fallback after lexical scopes. 14 new tests; two existing
tests updated. Diff and behavior match the message.

### `2b7bd5e` docs(spec): E1009 row covers builtin-name reservation — DOCUMENTATION-ONLY CORRECTION
`docs/errors.md` E1009 row corrected. Consistent with the normative rule.

### `d7105ac` chore(fuzz): sync aura-lang version pin 0.0.2 -> 0.2.0 — JUSTIFIED
See §10.

## 7. Builtin reservation independent review

Authoritative registry: `src/stdlib/signatures.rs::builtins()` (38 entries;
`src/stdlib/mod.rs:20` `builtin_names()` derives from it). The 68 `name:`
strings in the file are 38 builtins + method signatures; only the 38 table
entries are reservation-governed — correct.

Independent CLI sweep (release oracle, explicit path), 38 builtins × 10 forms
(`let`, `let mut`, fn param, lambda param, `for`, `catch`, list pattern,
user `fn`, top-level `let`, import alias): **380/380 → E1009, 0 misses**.

Negative controls (isolated directories to avoid sibling discovery): `sum2`,
`summary`, `total`, `len2`, `printable`, `range_value`, `summ`, `len_`, `lenn`,
`prnt`, `toString`, `too_string`, `maxx`, `min2`, `keys1`, `values_`, `sort2`,
`reverse2`, `mapper`, `filtered`, `reduce2`, `assert2`, `enumerate2`, `zip2`,
`read_line2`, `readFile`, `my_sum`, `keys2`, `chars2`, `split2`, `join2` — all
**accepted**. (`Sum`/`LEN`/`Print` are rejected by the pre-existing
destructuring parse rule, not by reservation.)

Separate namespaces accepted: `struct sum`, `enum sum`, `type sum = int`,
field `sum`, variant `sum`, method `fn sum(self)`, `module sum`. Non-builtin
call forms still resolve (`sum([...])`, `len([...])`, first-class `let f = len`).

## 8. `sum` root-cause reconstruction

Independently reproduced from the parent revision `c8ded06^` (built in an
isolated temp tree; no repo mutation):

* Pre-fix `fn main() { let mut sum = 0; sum = sum + 1 }` → **E2001**
  `cannot assign to sum: it is immutable`.
* Pre-fix `fn len(x) { return 99 } ... print(len(5))` → **99** (user fn shadows
  builtin).

Root cause: `Checker::lookup` (`src/check/mod.rs`, pre-fix lines ~1697)
returned `Some(false)` for any builtin name **before** scanning lexical scopes.
A `let mut sum` binding therefore resolved as an immutable builtin, so the later
assignment was misdiagnosed E2001. The declaration was accepted; the failure
was a lookup-precedence defect, not an assignment defect.

`c8ded06` fixes the root cause, not merely the reproducer: it removes the
declaration-time escape by rejecting builtin declarations, **and** moves the
builtin fallback after lexical scopes. Current behavior is E1009 at the
declaration — the earliest correct phase.

## 9. E1009 review

* `src/error.rs:272` `RESERVED_NAME = 1009`.
* `docs/errors.md:21` now: “Reserved word or builtin name as a value-namespace
  name”, examples `let if = 1`; `let sum = 1`.
* `docs/LANGUAGE_SPEC.md:195` normative builtin-reservation rule, same code.
* `docs/LANGUAGE_SPEC.md` §30.2 table was stale; fixed by reviewer (§41).

Sharing E1009 between reserved words and builtin names is **already authoritative
Aura law** (the hardening’s own normative rule and `docs/errors.md`); no new
code was allocated. Both conditions share the observable effect: a
value-namespace name that the language will not let a user bind. No conflation
of semantically distinct runtime behavior is introduced.

## 10. Fuzz-version-pin review (`d7105ac`)

* Exact file: `fuzz/Cargo.lock`; the `aura-lang` entry `0.0.2 -> 0.2.0`.
* It is a generated lockfile (tracked in git), not hand-authored API.
* Root `Cargo.toml` package version is `0.2.0`; root `Cargo.lock` already
  records `aura-lang 0.2.0`. The stale `0.0.2` in the fuzz lock predated the
  release-line bump (`61d6a1f`), so it was **stale, not release-specific**.
* `cargo fuzz` builds the `aura-lang` path dependency and regenerates the lock,
  so the commit is a deterministic consequence of running the canonical fuzz
  command.
* No MSRV effect (crate code unchanged); no frozen-runtime effect; lockfile
  itself is the only change.
* Classification: **JUSTIFIED** (reproducibility/hygiene). It would have been
  more precisely scoped in the release-line commit, but it is not scope creep
  and reversing it would leave the fuzz lock inconsistent with the workspace.

## 11. CLI oracle review

Executable tested: `target/release/aura`, built at HEAD via
`cargo build --locked --release` (rustc 1.98.1). Commands used explicit
absolute paths; no PATH-ambiguous binary. Pre-fix oracle built separately from
`c8ded06^` in `/tmp` (isolated), never installed over the canonical binary.
Build command and binary path recorded above.

## 12. Wrong-file rejection verification

Fresh isolated fixtures:

* two valid siblings unique outputs `AAA`/`ZZZ` → correct entry each;
* absolute entry, relative entry, nested directory entry → correct;
* same filename in two directories → correct entry each;
* malformed sibling → parser failure attributed to the sibling (not entry
  misselection).

**ENTRY SELECTION is correct.** The malformed-sibling failure is a separate
behavior (eager compilation-unit discovery), not wrong-entry execution.

## 13. Sibling-discovery policy review

Observed: `aura run good.aura` fails when an unused sibling `*.aura` is
malformed.

Authority: `docs/FILESYSTEM_MODULES_DESIGN.md:399-423` (“eager structural
discovery … a physical child that conceptually becomes an in-source module is
also part of the compilation unit”) and the adversarial table at line 774
(“unused malformed reachable child source → parser failure”). Discovery is
bounded by ownership (line 408-414); unrelated unowned directories are not
scanned.

Classification: **DEFINED AND CORRECT** (an implementation of an explicit
design decision). It is arguably **DEFINED BUT POORLY EXPLAINED** to end users
(no CLI-level prose; only design docs). No semantic change made; not
“improved”, per instruction.

## 14. Language-law adversarial findings

* builtin × closure capture / nested scope / loop / lambda / pattern: enforced.
* builtin call in closure (`(xs) -> sum(xs)`) accepted; builtin call in loop
  accepted; builtin as first-class value accepted.
* builtin × import alias: rejected (E1009); module alias (`use M as sum`)
  accepted (module namespace).
* named args × unknown / duplicate / missing: all E3001, before runtime.
* mut param × capture: closure sees the mutation (5) — consistent.
* failed assignment × subsequent state: no state corruption.
* shadowing: pre-existing `tests/shadowing.rs` 15/15.

No new semantic divergence discovered.

## 15. Checker/runtime differential

Cases with potential static/runtime disagreement were probed: assignments,
calls, structs/enums, operators, ranges, closures, modules. Checker accept →
runtime consistent; checker reject → earliest correct phase. No case found
where the checker accepts and the runtime contradicts for the touched surface.
Dynamic boundaries remain deferred to runtime as documented.

## 16. Numeric safety

`i64::MIN`, `i64::MAX`, `-1`, `0`, `1` across `+ - * / %` negation, range,
index, length: all produce structured diagnostics or correct values; **no
host panic**. Confirmed: `i64::MAX + 1` → E4013; `i64::MIN / -1` → E4013;
`i64::MIN % -1` → E4013; `x / 0`, `x % 0` → E4007; `-i64::MIN` → E4013;
`range(0, 10000001)` and `range(0, 10000000)` both materialize correctly
(the guard is `n > 10_000_000`, i.e. `abs(end-start)` on the realistic axis;
`range(-10000000, 10000000)` stays lazy/cheap as verified).

## 17. Runtime panic surface

`src/` production paths: `unwrap_used`/`expect_used`/`panic` are `deny` in
`Cargo.toml`. Textual and structural search found no production `unwrap()`,
`expect(`, `panic!`, `todo!`, `unimplemented!`. Two `unreachable!()` in
`src/resolve.rs`:

* line 1011 — inside a `match item { Item::Module { .. } => { let name = match item { Item::Module { name } => name, _ => unreachable!() } } }`; the outer arm already proved it is a `Module`.
* line 1210 — `rewrite_item` receives only non-`Module`/non-`Use` items because `flatten` (line 1005-1018) handles those first.

Both are control-flow-defensive; user input cannot reach them. No finding.

## 18. Resource boundaries

Present limits verified in source: `MAX_AST_DEPTH = 256` (parse/check/run),
`MAX_CALL_FRAMES = 512`, `MAX_VALUE_DEPTH = 512`, `MAX_VALUE_NODES = 1_000_000`,
`MAX_RANGE_MATERIALIZE = 10_000_000`, `MAX_FORMAT_WIDTH = 10_000_000`,
`MAX_FORMAT_PRECISION = u16::MAX`. Boundary probes (`limit-1/limit/limit+1`
where practical) returned diagnostics or normal results, never crashes; state
recovers after rejection. Deep container debug/repr is bounded. No finding.

## 19. Cycle safety

`let mut xs = []; push(xs, xs)` then `==` (true), `repr`, `json_encode`: all
terminate without panic (repr/json bounded by depth + node budget; the inner
self-reference renders as a bounded sentinel). Identity equality for a
self-referential value is stable. Equality semantics for acyclic values are
unchanged (`tests/adversarial.rs` cycle tests pass). No finding.

## 20. Module/provider parity

Native, InMemory, and WASM providers share `SourceProvider → ModuleGraphBuilder
→ resolver → checker → runtime`. The playground `differential` and `syntax`
suites compare native vs the real wasm artifact across 214 + 43 programs with
matching status/stdout/codes. `module_graph`/`native_source`/`modules` tests
pass. No divergence beyond the language-version gap described in §27.

## 21. Determinism

Repeated diagnostics for the same input: 1 distinct output over 5 runs.
Sibling-heavy module assembly: identical stdout across repeated runs.
No dependence on `HashMap` iteration, filesystem enumeration, or insertion
order observed on the touched surfaces. `syntax_conformance` includes a
deterministic-diagnostics test (pass).

## 22. Performance

200-sibling project check reproduced in ~0.21 s (previous ~0.06 s claim was a
different host/run; both are small). Measured growth for N siblings
(25/50/100/200/400): 0.19/0.25/0.20/0.21/0.34 s — near-linear, dominated by
per-source parse/check. The prior FSM-P5 case-collision quadratic scan was
already removed (`e17e37c`). No performance finding requiring action.

## 23. Fuzzing

Independent passes on the canonical targets:

* `parser`: 698,733 runs / 61 s, zero crashes.
* `checker`: 325,803 runs / 61 s, zero crashes.

No crash artifacts. Targets present: `lexer`, `parser`, `checker`, `runtime`.
No missing high-value target identified for the hardening surface.

## 24. Property testing

`tests/property.rs` and `tests/property_hardening.rs` (5 properties) pass.
`registry_checker_and_runtime_agree` enumerates the real registry (not a
hand-picked subset). Property 3/4 deliberately exclude TypeExpr-heavy inputs
with an explicit, narrow, documented exclusion tied to AUDIT-3
(`tests/property_hardening.rs:206-219`, `tests/corpus.rs:13`). The exclusion is
narrow and explicit. No finding.

## 25. REPL

Builtin collision rejected with E1009 and prior state preserved (`let mut
total = 5` still worked after a rejected `let mut sum`). Rejected `fn len` did
not break the builtin `len`. Valid `fn total` persisted. Failed submission did
not corrupt valid state. REPL does not create a second language surface.

## 26. CLI

`run`/`check`/`eval`/`repl`/`version`: correct responsibilities. `run -`
(stdin) enforces reservation. Missing file, Unicode/relative/absolute/nested
paths handled without raw panics. Exit codes 0/1 as expected; diagnostics on
stderr. No finding.

## 27. FSM-P6 regression review

`playground/tests/node/multifile.test.mjs` 42/42; `project.test.mjs` 36/36;
`cache.test.mjs` 7/7. Full `run-all.mjs` green after the §41 fixture fix:
manifest 42, project 36, completion 7, ABI 80, integrity 31, differential 214,
syntax 43, browser 62, worker 12, multi-file 42, cache 7. FSM-P6 intact.

## 28. Native / InMemory / WASM parity

The differential now passes 214/214 native↔wasm after the reviewer fixture
fix. Before the fix it failed exactly on the stale `let values` fixture because
the native engine (current source) enforces reservation while the committed
wasm predates it — a fixture/version artifact, not a semantic divergence.
`typeexpr sweep`: 37 depths, native ceiling 2048, wasm ceiling 768, host
failures 0 (the known AUDIT-3 substrate gap, left untouched).

## 29. Development-runtime integrity

`0.2.0-dev.1` unchanged (hash/size as recorded). It legitimately predates the
hardening and is a development artifact. No silent overwrite; no promotion to
`0.2.0`.

## 30. `build.mjs` protection

`FROZEN_0_0_2` and `FROZEN_0_2_0` are pinned in source; `--check` verifies
manifest↔disk hashes and refuses frozen drift; the build refuses to overwrite
an existing immutable runtime with a different hash. `node
playground/build.mjs --check` → “manifest matches 5 version(s)”, exit 0. Frozen
`0.2.0` cannot be regenerated by the dev line. No finding.

## 31. Error-code law

Spot-checked E1009, E1015, E2001, E2007, E2016, E3001, E4007, E4011, E4013,
E4020, E4027, E4999 and hardening-touched codes. E1009 now consistently means
“reserved word or builtin name as a value-namespace name” across code, tests,
`errors.md`, and (after §41) the spec table. No same-condition/conflicting-code
or misleading-shared-code finding on the touched surface. No new code
allocated.

## 32. Documentation consistency

`docs/errors.md`, the `LANGUAGE_SPEC.md` normative rule, and tests agree after
the §41 fix. Historical reports remain historical and are not cited as
authority. No normative contradiction remains on the hardened surface.

## 33. Test-quality review

New `tests/builtin_reservation.rs` (14 tests) is table-driven over the real
registry, with negative controls and separate-namespace acceptance tests — it
reaches the claimed paths and fails on pre-fix code (verified: the `sum`
minimal test fails on `c8ded06^`). `tests/regressions.rs` was updated to assert
the new law (E1009) rather than the removed shadowing behavior; not a
self-justifying change, because the law is independently normative.

Gap that the prior tests did **not** catch: the JS differential fixture. Root
cause: the fixture corpus is duplicated across `tests/maps.rs` and
`differential.test.mjs`, and only the Rust copy was updated. This is exactly
the class of “fixture does not match the fix” the reviewer is meant to find.

## 34. Security review

Trust boundaries reviewed: CLI filesystem, `SourceProvider`, WASM ABI,
playground worker/JSON transport, manifest hash verification, source keys.
Filesystem builtins (`read_file`/`write_file`) are documented host authority,
not sandbox escapes. No user-controlled input reached a panic or an unbounded
allocation; no project/source-boundary crossing observed. No finding.

## 35. Cross-subsystem adversarial round

Constructed interactions (builtin × closure/nested scope/import alias/
module; alias × visibility; closure × mutation; module × malformed sibling;
module × reserved member; failed run × valid run; multi-file × nested
diagnostic; legacy runtime × multi-file) were exercised. No new failure shape
beyond §41 was found. The module-member observation (§7/§14, INFORMATIONAL)
was surfaced rather than “fixed”.

## 36. Canonical validation matrix

All commands run at HEAD (explicit paths; host: darwin, rustc 1.98.1):

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo test --locked --all-targets --all-features` | PASS (EXIT=0) |
| `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` | PASS (EXIT=0) |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | PASS (EXIT=0) |
| `cargo +1.83.0 check --locked --all-features` | PASS |
| runtime `cargo fmt --all -- --check` | PASS |
| runtime `cargo test --locked` | PASS |
| `node playground/tests/node/run-all.mjs` | PASS (all suites) |
| `node playground/build.mjs --check` | PASS |
| `node website/tests/run-all.mjs` | PASS |
| `cargo +nightly miri test --locked --lib --no-default-features --features cli` | PASS (EXIT=0) |
| parser fuzz / checker fuzz | 0 crashes |

Note: `cargo audit` is not installed in this environment and was not run.
The canonical aggregate commands did complete here; they were not split.

## 37. Claims independently verified

* Builtin reservation enforced for all 38 builtins × 10 value-namespace forms.
* Pre-fix `sum` E2001 reproduced from source; root cause is `lookup()`
  precedence.
* `c8ded06` fixes the root cause and is complete at declaration.
* Frozen runtimes byte-identical.
* Fuzz pin `d7105ac` is a deterministic lockfile synchronization, not scope
  creep.
* Sibling eager discovery is defined in `FILESYSTEM_MODULES_DESIGN.md`.
* Entry selection is correct for abs/rel/nested/duplicate-name entries.
* Rust, playground, and website validation suites are green.
* FSM-P6 multi-file E2E 42/42.

## 38. Claims partially verified

* “200-sibling ~0.06 s”: measured ~0.21 s here; near-linear growth confirmed,
  exact figure is host-dependent (informational only).
* “Zero panics across the whole repo”: confirmed for `src/` production paths
  and interpreted for the two `unreachable!()` defensive arms; not proven
  exhaustively for every indirect macro alias, though the `deny` lints and
  Miri support the claim strongly.

## 39. Claims rejected

* The prior “wrong-file execution” candidate was already reclassified in
  `c8ded06`; independently confirmed REJECTED (entry selection is correct).
* Any characterization of the fuzz-pin commit as unjustified: rejected; it is
  a justified lockfile synchronization.

No claim by the hardening that survived review was found false.

## 40. New defects discovered

### F1 — Stale differential fixture breaks the Playground/website suites (HIGH)
* Severity: HIGH (valid CI/validation surface red; native/wasm parity gate
  fails on a legitimate program shape).
* Confidence: REPRODUCED + ROOT-CAUSED + REGRESSION-TESTED + FIXED.
* Minimal repro:
  `node playground/tests/node/differential.test.mjs playground/runtimes/0.2.0-dev.1/aura_playground_runtime.wasm playground/runtime/target/release/aura-playground-native`
  → `FAIL generic map alias` (wasm `ok "3.3"`, native `diagnostic E1009`).
* Authority: builtin reservation law (`LANGUAGE_SPEC.md` §…; `errors.md`);
  the identical fixture in `tests/maps.rs` was renamed by `c8ded06`.
* Root cause: `c8ded06` renamed `let values`→`let vals` in `tests/maps.rs` but
  the same fixture text persisted in `differential.test.mjs:599`; `values` is a
  registered builtin, so the current native engine rejects it.
* Affected layers: playground JS test corpus, website test wrapper.
* Should prior tests have caught it? Yes — `run-all.mjs` is the canonical gate
  and was reported green; it is red at `d7105ac`.
* Fix: reviewer commit `f416201` (fixture rename only).
* Regression protection: the differential harness now passes 214/214; the
  canonical `run-all.mjs` executes it on every run.

### F2 — Internal spec code table contradicted the new rule (MEDIUM)
* Severity: MEDIUM (documentation contradiction users read).
* Confidence: REPRODUCED + ROOT-CAUSED + FIXED.
* Minimal repro: `docs/LANGUAGE_SPEC.md:3012` said “E1009 | reserved word used
  as a name” while the same file’s §… normative rule and `docs/errors.md` say
  builtin names are also E1009.
* Fix: reviewer commit `f416201`.

### F3 — Module-member builtin shadowing (INFORMATIONAL)
* Severity: LOW/INFORMATIONAL.
* Confidence: REPRODUCED.
* Minimal repro: `module M { pub fn sum(xs: [int]) -> int { return 42 }
  pub fn use_it() -> int { return sum([1,2,3]) } } fn main() {
  print(M::use_it()) }` → `42` (builtin shadowed inside the module).
* Authority: module-member namespace is separate from the global value
  namespace; no explicit law forbids this. The hardening’s declared scope is
  user-visible global/local value bindings.
* Decision: **HUMAN SEMANTIC DECISION REQUIRED** if module members are meant to
  be covered too. Not changed; not invented.

## 41. Remediations made by reviewer

Single additive local commit `f416201`:

* `playground/tests/node/differential.test.mjs` — renamed `values`→`vals` in
  the `generic map alias` fixture (test fixture; no semantics).
* `docs/LANGUAGE_SPEC.md` §30.2 — E1009 row now covers builtin names.

No production code changed. No frozen artifact changed. Not pushed.

## 42. Human semantic decisions required

* HD-1 (optional): Should the builtin reservation extend to **module members**
  (`module M { pub fn sum }` / `pub let sum`), which currently shadow the
  builtin only within their module? Current behavior is consistent with the
  separate module-member namespace; leave as-is unless the human extends the
  law.
* HD-2: AUDIT-3 / TypeExpr nesting remains DECISION-PENDING (unchanged; §43).

## 43. AUDIT-3 confirmation

No hardening commit resolved or encoded AUDIT-3. No test encodes a TypeExpr
nesting policy; the property/corpus exclusions explicitly cite
DECISION-PENDING. No doc converted the decision package into normative law.
Status sentence unchanged: `Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no
code or doc changes beyond the existing decision package; property test
AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.`

## 44. Remaining risks

* HD-1 semantics (module members) if the human intends broader reservation.
* The native/wasm differential is sensitive to fixture drift; the F1 class can
  recur whenever the builtin registry grows and a fixture binds a new builtin
  name. Recommendation: gate fixture corpora against `builtin_names()` in CI.
* `cargo audit` was not run (tool absent); dependency advisories unverified.
* Miri was run on the `cli` lib surface only.

## 45. Final Git state

```
HEAD: f416201674bfde352b8d20b964f1c73fe66ddbb6
git status --short: ?? s
git diff --check 4b9d16b..HEAD: clean
FSM-P6 history: intact
hardening: additive
frozen runtimes: byte-identical
```

WORKTREE: review.md (this report) is untracked unless you choose to add it.
`s` untracked and untouched. No push, tag, or release.

## 46. Recommended next action

Proceed to **push review** of the local stack (`4b9d16b..f416201`) by the
human. Before pushing, optionally decide HD-1 and drop/commit this report.
Do **not** push, tag, or release without explicit human authorization.
