# Aura Agent State

## Directive 003 — stabilization and hardening campaign (current)

- Active branch: `rewrite/v3-rust`, remote HEAD **`02c3c897`** — the merge of
  **PR #24** (CORE FIRST: the set key-method surface is now type-checked).
  **Twenty-four PRs are merged** this campaign:
  #2 `f96536b6` (Directive 002 preflight),
  #3 `07845932` (rustls advisories), #4 `a562bacf` (batches 1–5),
  #5 `eaa3b9ed` (TD-23 + default-branch flip), #6 `4a20fa84` (post-merge
  records), #7 `22e73cce` (B6/B7 divergences + website link),
  #8 `1aa301c6` (session provenance + encoder cost), #9 `9654e15d` (B14
  project sessions), #10 `a3b7c377` (PR template + Windows flake),
  #11 `c9f88665` (TD-24 cost guard), #12 `1498cd3c` (CHANGELOG + guards),
  #13 `19cc4f22` (website gate + stale HTTP docs),
  #14 `9548e4df` (TD-27 Playwright gate),
  #15 `6458c2e6` (state sync after #14),
  #16 `757eee48` (B7 project-header contract + HTTP transport docs +
  `test:abi` no longer pinned to the frozen `0.0.2` runtime),
  #17 `da9ab375` (`scripts/test_validate.py` wired into CI),
  #18 `6cb8c601` (state sync for PRs #15–#17 and the Directive 003-R §4
  blockers), #19 `47232a79` (browser network consent scoped to the mutation
  class; `npm run test:site` rebuilds a stale site instead of testing old
  code), #20 `38f5fed5` (Directive 004: the seven agents), #21 `2fe944fb`
  (CORE FIRST: REPL session state incremental, TD-13 closed, session APIs
  feature-gated), #22 `92f5cf18` (CORE FIRST: resolver type-parameter frame
  popped on the error path), #23 `db9b511a` (state sync after #22), #24
  `02c3c897` (CORE FIRST: Set `add`/`has`/`contains`/`remove` element typing).
  The
  `/private/tmp/aura-directive-002-release-preflight` worktree and the
  `repair/directive-002-release-preflight` branch are **historical**.
- **The default branch is `rewrite/v3-rust`** (TD-23): flipped from `master`,
  which is a strict ancestor (`0 587`), after verifying no rulesets, no branch
  protection, no open PRs to `master`, five tags, and a tag-triggered
  `release.yml`. `gh workflow list` now reports `Fuzz (nightly)` where it
  previously 404'd, and `gh workflow run fuzz-nightly.yml` queued a run — the
  workflow is *registered*. **The timer fires:** `schedule`-event run
  `41037980226` completed successfully on 2026-10-30T10:04Z with all 14 steps
  green, and the earlier `workflow_dispatch` run `40984609515` also passed.
  Exactly one `schedule`-event run exists in the workflow's history
  (`gh run list --workflow fuzz-nightly.yml`): `event=schedule`,
  `branch=rewrite/v3-rust`, created 2026-10-30T10:04Z. The declared cron is
  `17 3 * * *` UTC, so the observed firing does not sit at the literal 03:17
  slot and GitHub may delay scheduled runs. Treat the *firing mechanism* as
  proven by the event type; treat the *slot* as still to be observed.
- **Directive 003-R authorization is in force** (`AGENTS.md`, "Standing
  authorization"): push, PR, merge, workflow administration, and fixes for B6,
  B7, B14, TD-23 are permitted. Force-push, history removal, tag moves,
  frozen-WASM replacement, gate weakening, and unapproved stable releases
  remain forbidden.
- Scope: stabilize, harden, and measure — core language fixes, adversarial
  security, performance, testing/CI, documentation, and technical debt.
  Findings ledger: `docs/engineering/STABILIZATION_AUDIT.md`.
- Constraints in force: still forbidden are force-push, history removal, tag
  changes, frozen-WASM replacement, gate weakening, and a stable release
  without an approved candidate. Push, PR, merge, and workflow administration
  are authorized by Directive 003-R.
- Completed batches: (1) SemVer precedence centralized in `scripts/semver.py`
  (+ `aura::version` in-crate), replacing three `sort -V` call sites;
  (2) documentation drift reconciled (LANGUAGE_SPEC tuple rule, SECURITY,
  README, dev.7); (3) Playground HTTP hardening — B10, B11, B13′, B9 fixed and
  pinned by `transport-bounds.test.mjs` and `abi.test.mjs`, B13 falsified;
  (4) REPL O(N²) re-confirmed in release profile and the dark nightly fuzz
  workflow recorded as TD-23; (5) two RustSec advisories in the TLS trust
  chain patched (`rustls-webpki` 0.103.16, `webpki-roots` 1.0.10); (6) B14
  project sessions shipped as `0.3.2-dev.8` (PR #9); (7) PR template replaced
  the stale Python-era version, and the Windows-only wall-clock test was
  root-caused as pre-existing and retired (PR #10); (8) TD-24's encoder cost
  guard documented accurately (PR #11); (9) `CHANGELOG.md` `[Unreleased]`
  reconciled with what actually shipped, plus two mutation-verified
  contract-sync guards (PR #12); (10) the `website` CI job was wiring four of
  the twelve suites by hand, so the default branch stayed red through eleven
  green PRs — `run-all.mjs` is now the gate, the stale `native-only`
  assertion and `guide-io.md` HTTP claims are corrected, and the newly wired
  `docs-consistency` immediately exposed a shallow-checkout tag defect that
  was fixed with `fetch-depth: 0` (PR #13). Batches 1–5 are **merged** via
  PR #4 with all 20 remote checks green; #9–#13 each merged at 20/20 as well.
- **B6, B7, B14 and TD-23 are all closed.** B6 and B7 are formalized
  divergences (browser `opaqueredirect` exposes no status; `Headers` is a
  combining collection), stated normatively in `docs/LANGUAGE_SPEC.md` §25 and
  pinned by tests. B14 is **implemented**: `aura_session_start_project`
  extends Host ABI 2 *in place* (`ABI_VERSION` stays `2`, no frozen runtime
  touched) and ships as the feature-detected `0.3.2-dev.8` artifact, proved by
  `playground/tests/node/project-http.test.mjs` with a negative control.
  TD-23's default-branch flip is executed and behaviourally verified.
- **TD-27 closed (PR #14).** The browser suites had never run in CI: no job
  installed Playwright, and nine suites treated its absence as *success*, so a
  broken `npm ci` or a failed browser download kept CI green. Fixed with a
  shared `detect`/`skipExitCode` helper, `AURA_REQUIRE_BROWSER=1` on both
  jobs, and real installs. Proof is behavioural, not structural: the
  playground job ran **4m7s**, logged `Cache not found`, downloaded Chrome for
  Testing 153.0.8010.12 and FFmpeg/Headless Shell, ran `--with-deps` apt on
  ubuntu-24.04, and executed all six browser suites (199 assertions, zero
  skipped); the website job executed all four (897 assertions). The path had
  never been exercised before because a warm local cache hid the download.
  `TECHNICAL_DEBT.md` now has **zero `OPEN` rows**.
- Exact next action: `docs/engineering/CURRENT_HANDOFF.md`.

## Directive 004 — Kilo → GitHub Copilot agent migration (current P0)

- Seven Kilo roles under `.kilo/agent/` converted to native Copilot custom
  agents under `.github/agents/`, committed as `dc16b4fe` on
  `chore/copilot-agent-migration`, all pinned to
  `customendpoint/Robin/deepseek-ai/deepseek-v4.1-flash-1017`.
- `.kilo/` is **preserved untouched** as migration reference.
- **The `400 The requested model is not supported` error is diagnosed to three
  independent root causes, all proven by controlled experiment:**
  1. **A — host hardcoded default.** `agentHostMain.js` declares
     `WS="gpt-5.6-luna"` and uses it as the default for
     `chat.agentHost.copilot.executionSubagent.model`. `gpt-5.6-luna` is not
     registered in this BYOK-only deployment, so dispatching an `explore`
     subagent with no pin reproduces the `400`.
  2. **B — SDK built-in agent YAMLs.** `@github/copilot/sdk/definitions/
     {explore,task}.agent.yaml` declare `model: claude-haiku-4.5`, likewise
     unregistered. Pinning `claude-haiku-4.5` reproduces the `400` exactly.
  3. **C — platform, not repo (external, unfixable here).**
     `MEn`/`TEn` in `workbench.desktop.main.js` route `local`/`user`-sourced
     `agent` and `skill` entries to the **standalone** bundler
     (`<authority>-standalone`) while plugin/extension/builtin entries go to
     the **synced** bundler. The Agent Host only ever receives the synced
     plugin; no `-standalone` plugin directory is ever written to disk, and
     `agenthost.log` shows exactly one synced-plugin sync and one
     `appliedCount=0` customization event. Consequently `customAgents` in the
     Agent Host is structurally empty and `agent_type: aura-scout` returns the
     7-builtin enumeration.
- **The available-model set is exactly two entries per surface** (`auto` and
  the DeepSeek id), from `window3/renderer.log`. Neither `gpt-5.6-luna` nor
  `claude-haiku-4.5` is present, which is why both `400`s are *correct* server
  rejections rather than registration bugs.
- **Working:** the workbench discovers all seven agents
  (`customizationsDebug.log`: `agents 7 0 2 9`, all seven paths, no
  rejections) and an explicit `model:` pin makes builtin agent types execute
  correctly.
- **Not achievable from this repository:** dispatching a *custom* agent by
  `agent_type` in the Agent Host harness. Root cause C is a VS Code
  architecture decision, documented here so no future session re-investigates
  it.
- Scope: infrastructure recovery. CORE FIRST work resumes after this migration
  lands.

## CORE FIRST — set key-method element typing — merged, `02c3c897` (PR #24)

`TypeClass::Set` had **no element branch at all** in `Checker::check_method_call`.
The entire set key-method surface (`add`, `has`, `contains`, `remove`) was
therefore statically unchecked, while the structurally identical map key methods
were correctly rejected with `E3001`. `List.push` was checked correctly, so the
asymmetry — not a missing feature — was the defect.

```aura
let mut s = set{1}
s.add("x")      // accepted; ran to completion producing {1, "x"}
s.has("x")      // accepted; silently returned false, no diagnostic
s.remove(1.5)   // accepted; failed only at runtime
```

Not caught with an explicit `{int}` annotation, a declared return type, or an
`{int}` parameter, and still not caught when the receiver arrived through a
`{int}`-returning function. `Set.add` and `Set.remove` are registered with
`Param::ANY` / `Ty::Unknown` (`src/stdlib/signatures.rs:1205`, `:1214`), so the
coarse signature table cannot express the constraint and the checker must.

- **Fix:** `check_element_assignable` gained a `container: &str` noun so the
  diagnostic can name list, array, or set; its four existing call sites pass
  `"list"`/`"list"`/`"list"`/`"array"` and their messages are unchanged byte for
  byte. Two branches were added to `check_method_call`: `Set` + `add`, and
  `Set` + `has`/`contains`/`remove`. Both reject a non-key-capable argument
  first (so the diagnostic is right even when the element type is `Unknown`),
  then check agreement with the receiver's proven element type.
- **Set-specific wording is deliberate.** `Ty::key_type_error()` says "map key",
  so it is left untouched for maps rather than made ambiguous; the new branches
  reuse the set-literal path's existing message.
- **Guards:** `tests/collections.rs::
  typed_set_mutation_obeys_the_element_type` and
  `::typed_set_key_methods_obey_the_element_type`. Both failed pre-fix with
  `expected the program to be rejected: ""`. They cover annotated, inferred and
  declared-return receivers; the accept path; union receivers; the untyped
  `set{}` receiver (no false rejection); all three query methods; and
  non-key-capable arguments.
- **Docs:** `docs/LANGUAGE_SPEC.md` had **no `set` method table at all**, so the
  roster existed only in code. §24.1 now has one beside the string/list/map
  tables, and §21.4 states the check-time typing contract normatively.
- **Merged as PR #24 (`02c3c897`), CI 21/21 green on round 1.** All six local
  template gates were run first and are recorded in the PR body with their
  results; `mergeStateStatus` was `CLEAN` and `reviewDecision` empty, so no
  external human approval was required or simulated.
- **Considered and rejected as defects:** `list.contains`, `array.contains`,
  `tuple.contains` and `range.contains` also take `Param::ANY` and return
  `false` for a foreign argument, but that is the documented contract (spec
  §24.1 "1 any") and it cannot corrupt state. `reduce`'s `Param::ANY`
  accumulator *is* checked, because overload resolution rejects `"s"` against
  the numeric `+` in the body.

## CORE FIRST — resolver type-parameter leak — merged, `92f5cf18` (PR #22)

`Resolver::rewrite_item` and `Resolver::rewrite_member` pushed a generic-parameter
frame before rewriting a declaration and popped it with a plain statement at the
end of the arm. Every arm performs fallible work in between (`rewrite_params`,
`rewrite_type`, `rewrite_block`, `rewrite_field`, `rewrite_variant`,
`rewrite_member`, `canonical_type`), so any `?` returned before the pop and left
the frame — and its generic names — on `type_param_scope` for the rest of the
session. `flatten` → `rewrite_type` → `is_type_param` then refused to
canonicalize a real declared type whose name matched a leaked parameter, and the
checker reported a bogus unknown type on an unrelated, valid line.

- **Reproducer:** a declaration whose rewrite fails after the push (an `impl<Q>`
  on a private struct → `E2018`), followed by a submission declaring a real type
  also named `Q`. Pre-fix the second submission reported
  ``E3002: unknown type `Q``` instead of the true private-field `E2018`.
  Five controls (no `impl`; `impl` alone; non-colliding parameter name;
  `struct`/`enum`/`alias`/`trait` failing bodies with the same name) were correct
  before and after, which pins the cause to the leaked frame.
- **Fix:** module-level RAII guard `TypeParamFrame<'a>` holding `&'a Resolver`
  and popping on `Drop`, bound as `let _frame = TypeParamFrame::new(self);` at
  all seven push sites. The guard borrows immutably, which is enough because
  `type_param_scope` is a `RefCell` borrowed only inside `push`/`pop`/
  `is_type_param`, so no borrow is held across the rewritten work.
- **Deliberately not the fix:** a blanket `type_param_scope.clear()` in
  `reset_transient`. That masks a per-declaration invariant violation behind a
  per-submission reset; the guard repairs the invariant.
- **Guard:** `tests/repl.rs::
  a_failed_generic_declaration_does_not_leak_its_type_parameters` — asserts the
  `E2018` is present, `E3002` is absent, and the private-field diagnostic is the
  one reported. Fails pre-fix at the `E3002` assertion.
- **The checker's identical shape is NOT a bug and must not be "fixed".**
  `Checker::check` (1067–1074) is strictly fail-fast (`self.item(item)?` inside
  the loop), there is no multi-diagnostic collection anywhere in the crate, the
  three catch-like sites each `return Err(e)` right after popping, and
  `Checker::reset_transient` clears `current_type_params` (1046–1047) which the
  REPL calls before every submission (`src/repl.rs:341-342`, `455-456`). The
  observable asymmetry comes from `Resolver::reset_transient` (367–373) not
  clearing `type_param_scope`, not from a difference in the push/pop code. An
  earlier session's "checker leak reproduced" was `/tmp` fixture pollution:
  `compile_file_with_mode` scans the target file's directory for module sources,
  so one stray `.aura` file makes every `aura check` there report *its* error.
  **One fresh directory per probe file.**
- **Pre-existing, unrelated:** `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
  --lib` still reports 8 broken intra-doc links, all in `src/run/`. CI never runs
  `cargo doc`.
- **Merged as PR #22 (`92f5cf18`), CI 21/21 green on round 1.** `language
  contract`, `fuzz smoke`, `miri`, `playground (wasm runtime)` and `website` all
  passed on the first push — no red gate, no retry, no gate weakened.
  `mergeStateStatus` was `CLEAN` and `reviewDecision` empty, so no external human
  approval was required or simulated.

## CORE FIRST — REPL session linearization (TD-13) — merged, `2fe944fb` (PR #21)

Directive 003-R ADENDO "CORE FIRST" makes the language core the campaign's top
priority. The first CORE FIRST finding is the directive-authorized REPL
O(N²) defect, reproduced, root-caused, fixed in three layers plus two follow-on
narrowings, and pinned by five new/rewritten regression guards. **Merged as PR
#21 (`2fe944fb`), CI 21/21 green on round 2** after one real feature-gating
defect found by round 1 and fixed by `3a42deee`.

- **Symptom (pre-fix, release, min-of-3, distinct names):** `let v{i} = {i}` at
  N=250/500/1000/2000/4000 cost 0.174/0.337/1.174/4.548/**18.299 s** (~4.0×
  per doubling). The `fn` path was far worse: N=4000 cost **98.270 s**.
  Cost scaled with the count of **distinct** declarations; shadowing one name
  stayed linear. A marginal-cost probe (`1+1` submissions on top of a fixed
  declaration count) showed **~658 µs per submission at every depth** — i.e. a
  constant per-submission cost multiplied by session depth, which is the
  signature of the defect.
- **Root cause:** `SessionEngine::eval_line` rebuilt the whole `Checker` from
  every prior declaration via `Checker::with_declarations` on each submission,
  so submission *k* did O(k) work. Localized to `with_declarations`
  (`src/check/mod.rs`) called from `src/repl.rs`.
- **Layer 1 — incremental checker.** `SessionEngine` now owns one `Checker` for
  the session; `reset_transient()` clears per-submission frames and
  `extend_declarations(&decls[applied..])` registers only new declarations. The
  module path needs no `extend_declarations` call: `check_repl_mode` → `check`
  → `hoist` + annotation pass + `item()` already register every table.
- **Layer 2 — `abort_submission` (found by the oracle).** A cached checker means
  a failing submission leaves partial mutations. A 25-case REPL oracle caught
  it: `fn k(int)` twice then `k(1)` reported `E3001 ... 2 overloads accept`
  instead of `1`, because `hoist` pushed the duplicate signature before the
  duplicate scan rejected it. `abort_submission` rebuilds the checker from the
  session on the three paths that can fail *after* mutating (stmt-path
  `check_stmt`, module-path `check_repl_mode`, module-path execution). Resolve
  failures are deliberately not hooked — nothing has mutated yet.
- **Layer 3 — `session_value_names` symmetry (found by a second oracle).**
  `hoist` never wrote `session_value_names` while `extend_declarations` does, so
  the cross-kind clash checks (`Item::Fn`, `Item::Const`) silently stopped
  firing across submissions: `const X = 1` then `const X = 2` went from
  `E2007`+`1` to `2`. Both `hoist` writes added.
- **Q1 — duplicate-scan narrowing.** The overload duplicate scan re-examined
  every name in `self.functions` on every submission, but a duplicate can only
  be created by a signature pushed during the *current* call. `self.functions`
  has exactly three write sites and `add_global` pushes only into an empty set,
  so the scan now drains a `rescan_functions: HashSet<String>` and scans only
  those names — with the comparator, nested loop, side effect and `E2007`
  diagnostic byte-identical.
- **Q2 — persisted resolver.** `Resolver` is now kept in `SessionState` instead
  of being rebuilt per submission. `extend_session` seeds items/imports past
  `seeded_items`/`seeded_imports` watermarks (stored on the resolver, so the
  move is idempotent by construction); `reset_transient` clears only
  `applied_imports`, `item_sources`, `active_source`, `resolved_sources`.
  Imports must **never** be re-applied (`apply_use`'s `name_visible_locally`
  check would raise a spurious `E2007` on a second application), and the
  watermarks must **not** be reset or the quadratic cost returns. Every other
  resolver table is monotone/append-only and safe to persist.
- **Measured result.** `let` path: N=4000 from **18.299 s → 0.238 s** (93.5×
  faster at N=4000 pre-Q1; the shipped tree measures 0.045/0.082/0.153/0.238 s
  at N=500/1000/2000/4000, ratios 1.85/1.86/1.55 — linear). `fn` path: N=4000
  from **6.621 s → 2.734 s (Q1) → 0.411 s (Q2)**, **16.1×**, ratio 3.88× →
  2.47×; shape to N=16000 is 0.080/0.148/0.295/0.763/2.008 s
  (1.86/1.99/2.59/2.63×), linear modulo cut-off effects. Against the pre-fix
  binary at N=4000 the `fn` path improved **98.270 → 0.411 s (239×)**.
- **Diagnosis method (reusable).** `sample`-based profiling does not work in
  this harness — it attaches to the shell wrapper, not the `aura` child. In-source
  phase instrumentation behind an env var (12 `AtomicU64` counters) was used
  instead and attributed the module path at N=2000 to **check 56% / resolve
  43%**, exec/decl/items ~0%. A surgical ablation (`AURA_SKIP_SEED` alone, a
  scan-only skip alone, then both) proved each contributor independently and
  that their **sum** is necessary and sufficient: ablating either alone left the
  path quadratic, ablating both made it linear. An earlier "no dominant driver"
  conclusion was overturned by that combined experiment.
- **Ablation discipline.** Skipping an inner block with `return` returns from
  the whole enclosing function and silently disables everything after it; the
  first `AURA_SKIP_SCAN` measurement was confounded this way and was re-run by
  replacing the block's *data* (`Vec::new()`) rather than its control flow.
  Always replace data, never control flow.
- **Guards (five).** `tests/repl_cost.rs` (new): a counting global allocator
  proves a submission's allocation volume does not grow with session depth —
  it measured **4.19× for 4× depth** before the fix and passes now. It cannot
  see CPU-only costs, which is exactly why Q1 and Q2 are guarded in
  `tests/bench.rs` instead: `repl_incremental_state_is_linear_in_session_depth`
  (rewritten and renamed from the old "no worse than quadratic" TD-13 guard;
  base 800, limit 3.2×, quadratic path measured 3.33–3.51) and
  `repl_function_declarations_are_linear_in_session_depth` (new; base 800, the
  pre-fix path measured 4.18× at that base). Two new `tests/repl.rs` tests pin
  the cross-kind name-collision `E2007`s; all three such tests were proven to
  fail without the Layer 3 fix.
- **Equivalence evidence:** a 25-case REPL oracle (`/tmp/probe_matrix.py`) and a
  6-case session-names oracle (`/tmp/probe_session_names.py`) both report
  **zero divergence** against the pre-fix binary, after every layer, after Q1,
  after Q2, and after the `SessionState` refactor.
- **`SessionState`** groups `decls`/`checker`/`applied`/`resolver` so
  `abort_submission` fits clippy's 7-argument limit honestly rather than via
  `#[allow(clippy::too_many_arguments)]`.
- **Deliberate non-fixes (recorded so they are not re-litigated):**
  `type_param_scope` can leak a frame when the `Item::Impl` arm hits an early
  `Err` (four `?` between push and pop). Harmless with a per-submission
  resolver, latent with a persisted one. It is **not** cleared in
  `reset_transient`; no oracle case diverges. The O(Σ set²) → O(Σ set) keyed
  duplicate scan is also deferred (`Ty` has `Eq` but not `Hash`).
- **CI round 1 — 5 failed / 16 passed, one real defect.** Every failure was the
  same thing: `Checker::session`, `Checker::reset_transient`, `Resolver::session`
  and `Resolver::reset_transient` are `pub(crate)` and reachable only from
  `src/repl.rs`, which `src/lib.rs:30` gates with `#[cfg(feature = "repl")]`.
  In any build without `repl` they are dead code, and CI's `RUSTFLAGS: -D
  warnings` turns that into a build error. Failing jobs: `language contract`,
  `fuzz smoke`, `miri`, `playground (wasm runtime)` ("Lint the runtime crate"),
  and `website` (its `playground/tests/node/run-all.mjs` step) — all compile the
  lib without `repl`.
- **Why the local floor missed it — permanent lesson.** Every command in it
  enabled `repl`: `--all-features` does, and so did the "CI feature set"
  `cli,repl,json,regex,time`. **A `pub(crate)` helper reachable only from a
  feature-gated module is dead code in every other feature set, and
  `--all-features` validation cannot see it.** The floor now includes
  `RUSTFLAGS="-D warnings" cargo check --locked --lib --no-default-features
  --features cli`, with `--lib` rather than `--all-targets` because
  `tests/cli.rs` uses `serde_json` unconditionally (795, 916) and fails with 24
  `E0433` under `--all-targets` in that configuration. That breakage is
  **pre-existing** (`git show --name-only 404980e5` does not list
  `tests/cli.rs`) and CI never builds the combination.
- **The fix, and why the alternatives were rejected.** `#[cfg(feature =
  "repl")]` on all four methods. Making them `pub` widens the public API to
  silence a lint; `#[allow(dead_code)]` weakens a warning to buy green. Both are
  forbidden by the PR template and Directive 003-R. One consequence: the
  intra-doc link ``[`Checker::reset_transient`]`` inside `Checker::session`
  became a plain code span, since a feature-gated link target would otherwise
  fail `rustdoc::broken-intra-doc-links` under `-D warnings`.
  `RUSTDOCFLAGS="-D warnings" cargo doc --lib` still reports **8 pre-existing**
  broken links, all in `src/run/` (untouched here; CI never runs `cargo doc`).
- **Verification of the fix.** Reproduced the dead-code error on unmodified
  `HEAD` (stash/restore) and cleared it in the same configuration, plus negative
  controls on the fuzz workspace and the playground runtime clippy — the three
  distinct job classes. Six no-`repl` lib feature sets clean under `-D
  warnings`: `cli`, `cli,json,regex,time`, `http-api`, `cli,evaluator-oracle`,
  `json,regex,time`, `py`. Both oracles still identical (25/25, 6/6); full floor
  re-run green; playground fmt/clippy/tests, wasm build, `build.mjs --check` and
  `run-all.mjs` all clean.
- **CI round 2 — 21/21 green.** `language contract`, `fuzz smoke`, `miri`,
  `playground (wasm runtime)` and `website` all pass on `3a42deee`, confirming
  the gating commit is the complete fix. Merged with `gh pr merge --merge` — no
  force, no protection bypass; `mergeStateStatus` was `CLEAN` and
  `reviewDecision` empty, so no external human approval was required or
  simulated.

## Repository

- Branch: `rewrite/v3-rust`
- **Git reality (verify with `git rev-parse`, do not trust prose):**
  `origin/rewrite/v3-rust` is at **`02c3c897`**, the merge commit of PR #24.
  Tag **`v0.3.1` = `b09cbef2`**. Aura 0.3.1 (Keystone) is the current release
  (release = language = `0.3.1`, runtime `0.3.1` is the Playground default) and
  is **PUBLISHED**. The earlier `b221e6a9`/`v0.2.1` notes below are
  **historical**; Git is authoritative.
- Local/remote relationship: authoritative value is `git rev-list
  --left-right --count origin/rewrite/v3-rust...HEAD`; a tracked file cannot
  safely hardcode its own position.
- **Historical (superseded):** the intermediate remote tip `b221e6a9` once
  carried a red `playground (wasm runtime)` CI job; that was repaired by the
  parser wasm-safety commits and the range subsequently advanced. The
  `b909066f` tip was superseded first by the Directive 002 release-preflight
  repair (PR #2) and the rustls security remediation (PR #3, merge
  `07845932`). Do not act on those old states.
- Historical release: **`v0.2.1`** (2026-10-01, immutable), tag `v0.2.1` =
  `3f5f8702`. Superseded by `v0.3.1`.

## Aura website — Aurea design-system migration (2026-10-09, historical)

A full website redesign + documentation synchronization against the published
0.3.1 contract, entirely on branch `rewrite/v3-rust` at local HEAD `b909066f`
(that state has since been merged forward to `07845932`). Scope was
`website/**`, the relevant `docs/**`
content mirrors, and website tooling/tests only; `src/**`,
`playground/runtimes/**`, `Cargo.*`, tags, and frozen artifacts were untouched
(verified byte-identical).

- **Aurea is the sole design system.** `website/assets/aurea.css` defines the
  canonical `--au-*` tokens, dark-default + light-inversion themes, base
  element styling, primitives, and accessibility rules.
  `website/assets/styles.css` is Aurea-owned layout/component/docs styling and
  defines no tokens. The Material 3 `website/assets/tokens.css` was **deleted**
  and dropped from `lib/layout.mjs`; no `--md-*` reference survives in the
  active chain. The standalone Playground stylesheet was rethemed onto the same
  `--au-*` vocabulary.
- **Documentation truth** reconciled to 0.3.1: dotted enum construction fixed;
  five collection identities documented; `never`, pattern-based `catch`, the
  semicolon rule, the nested-array-call-argument limitation, Set scalar-only
  membership and the Array length bound recorded; the typed-JSON canonical
  type-position form and strict `E4031`; HTTP (`http_get`/`http_request`,
  feature `http`, native-only) documented for the first time with `E5002`/
  `E4020`/`E4031`; stdlib inventory centralized in `website/content/stdlib.mjs`
  and rendered by both the landing and reference pages.
- **New drift guards:** `tests/aurea.test.mjs` (extended),
  `tests/aurea-browser.test.mjs`, `tests/stdlib-consistency.test.mjs`
  (parses `src/stdlib/signatures.rs`), `tests/docs-consistency.test.mjs`
  (route/version/link parity, release-pinned normative links).

## AURA FEATURE DEVELOPMENT — FROZEN FOR STABILIZATION (2026-10-09)

The 0.3.2 Playground development preview is **published** (branch
`rewrite/v3-rust`, HEAD `9f8743ee`, = remote; CI 20/20; Pages deployed). From
this checkpoint:

- **No new language syntax, standard-library functions, Host capabilities,
  Playground functionality, or ABI changes.**
- **No unapproved release.** `0.3.1` remains the release; `0.3.2-dev.7`
  (SHA-256 `4d0ae55d…e7ecd`) is an explicitly selected experimental preview,
  never the default (`manifest.current = 0.3.1`).
- Existing features may be corrected when a real defect is reproduced; security
  and correctness defects remain eligible for narrow fixes.
- The deferred global **binary value model (B)** remains deferred.
- **Next campaign:** a read-only engineering audit (see
  `docs/engineering/STABILIZATION_AUDIT.md`) and an **AIS Agent-Native RFC**
  (design only; no AIS/0.2 implementation until the human reviews it).

## Aura 0.3.2 — Playground execution & browser HTTP campaign (2026-10-09)

**D1 + D2 approved and implemented; browser HTTP works end to end.** See
`docs/engineering/PLAYGROUND_032_CAMPAIGN.md` for the full record. Nothing
pushed. Summary of the resumed campaign:

- **D1.** `Machine` is now a transient view over `(&mut Interp, &mut
  MachineState)`; `Control::Park`/`Resume::Park`/`RunOutcome::Parked` and
  `NativeOutcome::Suspend` carry a `PendingEffect` out of the loop with the
  native's resume token on the continuation stack. `src/run/session.rs` adds
  the public `RunSession`, which **owns** the `Interp` (no self-reference) and
  resumes the same machine exactly. Stale/duplicate resumes are rejected.
- **D2.** Host ABI 2: `aura_session_start`/`resume`/`reset`/`resume_push`
  (ABI version `2`), newline-delimited JSON, Rust-validated response values,
  status `3` = pending effect with a per-run `effect_id`. `runtime.mjs`
  feature-detects `supportsSessions`; frozen ABI-1 runtimes are never sent the
  new commands (verified: `0.0.2`/`0.2.1`/`0.3.1` load and run unchanged).
- **Browser HTTP.** HTTP builtins are resumable natives behind the new
  `http-api` feature (substrate-neutral; `http` implies it). The Worker does
  `fetch` with `credentials:"omit"`, `redirect:"manual"`,
  `referrerPolicy:"no-referrer"` only after the page's per-origin consent;
  denial is `E5002`, CORS rejection is `E4020`, no proxy, no fabricated
  statuses. `http_get`/`http_request` + `headers`/`body`/`timeout_ms` kept.
- **Runtime identity.** `0.3.2-dev.7` (ABI 2), SHA-256
  `4d0ae55dd33c0e3520dcf6b7123c127c1722030a20a2b55a2cd0701405fe7ecd`;
  `0.3.2-dev.5`, `0.3.2-dev.1`, and all frozen artifacts preserved; published
  `0.3.1` stays the default.
- **Evidence.** Rust `session` 14/0 and the all-features matrix 63/63;
  Node `session-http` 13/0; Chromium `browser-http` 8/0; live PokéAPI →
  `200\npikachu\n`; website suite green (browser 404, a11y 82, playground
  suite incl. browser 107/session-http 13).

**Remaining human gate:** the deferred global **binary value model (B)** for
arbitrary binary HTTP bodies beyond the additive `body_bytes` view, and the
promotion/publishing of the development runtime. Nothing is pushed.

- **Output.** `src/host.rs` gains a bounded-memory `OutputSink`
  (`preview`/`complete`, exact retained/written/omitted accounting, UTF-8-safe
  truncation). `BrowserHost::with_output_sink` makes a full preview
  non-fatal; `with_stdout_limit` (the frozen-release fatal `E4020`) is
  preserved. The reproduced defect — `for i in 0..600000 { print(i) }` →
  `E4020` — is fixed: the run finishes `ok`, retaining 262 KiB and omitting
  the rest (421 ms measured).
- **Development runtime.** `0.3.2-dev.1` is built, hashed
  (`a90ad31b…bbc4f63`), manifested as `development`, and selectable; the
  published `0.3.1` remains the public default (`PROMOTE_DEV_TO_DEFAULT` is
  `false`). `0.0.2`/`0.2.0`/`0.2.1`/`0.3.1` are byte-identical and `v0.3.1`
  is unmoved.
- **Native HTTP.** Four reproduced defects fixed: outgoing headers now reach
  the wire (loopback-captured), the body is binary-safe (`body_bytes`,
  additive), repeated headers are preserved (lists + `header_lines`, no
  comma-join), and a supplied body on GET/HEAD is reported, not dropped.
  `E5002`/`E4020`/`E4031` keep their meanings.
- **Host effects.** `HostError::Pending`/`PendingEffect` are in place as the
  transport foundation; the language-observable behavior is unchanged.
- **Validation.** Rust: all-features sharded matrix 62/62 PASS; clippy `-D
  warnings` clean on default, `http`, bare, and wasm; fmt clean; host 19/0,
  http 16/0. Playground: manifest 88/0, cross-release 69/0 (8 runtimes),
  differential 233/0, syntax conformance 77, b1 59/0, browser 107/0, worker
  12/0, multi-file 42/0, cache 7/0. Website suite re-run below.
- **Blocked (Gate 4).** Browser HTTP needs the cross-boundary parking driver
  (D1: `Machine` owns `Interp`) and an additive ABI (D2). Presenting them
  rather than guessing keeps ABI compatibility and the binary value model at
  the human gate. The browser host still reports `E5002`; no faked HTTP.

### Follow-up: documentation IA + AIS/MCP section (2026-10-09, LOCAL, UNPUSHED)

- Reorganized `website/content/docs.mjs` into six meaning-based groups (Getting
  Started, Language Guide, Language Reference, Runtime & Tooling, Semantic
  Tooling, Releases & Migration); every public slug preserved.
- Added two first-class pages: `website/content/ais.md` (`/docs/ais/`) and
  `website/content/mcp.md` (`/docs/mcp/`), grounded in `src/ais.rs`,
  `src/mcp.rs`, and the engineering records, with every example generated from
  the released compiler. Surfaced AIS/MCP in `cli.md`, `tools.mjs`,
  `architecture.mjs`, and the docs index.
- Added `tests/ais-doc.test.mjs` (real-binary fixtures + golden JSON, 89
  checks) and extended `tests/docs-consistency.test.mjs` (135 checks) with
  route/group/CLI-dispatch/protocol-version/tool-inventory guards. Fixtures at
  `website/tests/fixtures/ais/`.
- Documented contradiction: the engineering record's claim that a body-only
  edit yields an *empty* delta holds only for a same-length edit; a
  length-changing body edit reports later declarations `moved` (position
  refresh, ignored by `is_empty`). The website states this accurately.
- Validation: full `website/tests/run-all.mjs` green (46 pages; ais-doc 89/0,
  docs-consistency 135/0, browser 404/0, a11y 82/0, links 3365 OK both bases,
  base 3364/0; embedded Playground suite unchanged). Frozen runtimes and tag
  `v0.3.1` untouched. **Nothing pushed.**

### Follow-up: navigation fix + Playground workbench unification (2026-10-09)

- **Navigation.** The 768px three-row defect is fixed by a tiered architecture
  from one destination source (`site.config.mjs`): a restrained inline set plus
  a grouped `More` menu (overflow by set difference), collapsing at the
  measured-fit width (900px) into one compact disclosure; brand/GitHub/theme
  stay in the closed bar. `tests/nav-fit.test.mjs` proves single-row geometry and
  full reachability across 20 widths in both bases.
- **Playground.** The integrated and standalone Playgrounds now share one
  interface: `playground/web/workbench.mjs` (`workbenchMarkup()`) is the single
  markup string and `playground/web/workbench.css` the single stylesheet, both on
  the `--au-*` tokens. The standalone `style.css` is removed.
  `tests/playground-parity.test.mjs` locks id/stylesheet/token parity. The
  Playground page's oversized intro became a concise header + About disclosure.
- **Validation:** full `website/tests/run-all.mjs` green (nav-fit 380,
  playground-parity 93, aurea 39, browser 390, a11y 70; the embedded Playground
  suite 79/60/233/63/98/12/42/7 unchanged) plus the standalone
  `playground/tests/node/run-all.mjs`. Zero horizontal overflow across 13 widths
  × 17 routes × both themes, and at 200% zoom; zero console errors; frozen
  `0.0.2`/`0.2.0`/`0.2.1`/`0.3.1` and tag `v0.3.1` untouched. Nothing pushed.
- **Validation (local, not GitHub CI):** full `website/tests/run-all.mjs`
  green — 44 pages, links 2554 OK, base 2553 OK, browser 388/0, a11y 70/0,
  examples 22/0, aurea 35/0, aurea-browser 31/0, stdlib-consistency 122/0,
  docs-consistency 77/0, plus the reused Playground engine suite; zero
  horizontal overflow across 12 widths × 16 routes; `--base=/` variant green;
  `node playground/build.mjs --check` matches 8 runtimes. Frozen
  `0.0.2`/`0.2.0`/`0.2.1` and the `0.3.1` runtime byte-identical; `v0.3.1`
  unmoved. **Nothing pushed, tagged, or deployed.**

- No successor program or version is selected; there is **no `0.2.2`**, and
  `0.3.2` is planned but **not started**.
- Active program: **AURA 0.3 "KEYSTONE" CLOSURE** — the closure range is
  **pushed through `b221e6a9`**; only the 2-commit parser wasm-safety repair
  (`04783231`, `e7505408`) remains unpushed and requires a new human
  authorization. See
  `docs/engineering/CURRENT_HANDOFF.md` and
  `docs/engineering/KEYSTONE_SCOPE_MANIFEST.md`. The publication gate is the
  objective: no release, tag, version bump, or deployment is created.

## Frozen release runtimes (immutable, byte-for-byte)

| Version | Bytes | SHA-256 |
|---|---|---|
| `0.0.2` | 1,366,621 | `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed` |
| `0.2.0` | 1,654,161 | `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc` |
| `0.2.1` | 1,768,322 | `48c456fcda6c50dd6808ccc5f15a0bca4c0b81d7d970172557817decf427cc9e` |

Never overwrite a versioned artifact under `playground/runtimes/`; advancing a
runtime means adding a version, never replacing one.

## Current Track

B-1 — ENGINE-STACK-INDEPENDENT CALL ENGINE. **B-1 IS REMOTELY CLOSED** at
`bb736fc` (inside the pushed history at remote `089dffe`). The complete
B-1R3C–B-1R3F evaluator migration plus the production cutover (`62dd592`),
the residual seam closures (`11abad2`), the mechanical routing tripwire
(`a45d1ec`), the review-driven fixes (`39caf6f`), the review-outcome
checkpoint (`9fa70c6`), and the additive miri fix (`bb736fc`) are pushed.
Production runs the explicit-continuation machine on every entry point; the
recursive evaluator is retained only as the differential reference and
rollback path (B-1R8 removes it after the release decision).

POST-B1 RUNTIME/WASM EDGE CLOSURE — **REMOTELY CLOSED** at `e576238`
(remote-closed 2026-10-05 after the GitHub Actions runner incident cleared;
exact-SHA CI 20/20 green, Pages deployed). A narrow audit of the production
machine and its host boundaries (triggered by the Playground `E4020` at 1 MiB
stdout on the frozen `0.2.1` artifact). Key results: the 1 MiB
`MAX_STDOUT_BYTES` bound is a `BrowserHost` application resource policy since
the first wasm runtime (identical on `0.0.2`/`0.2.0`/`0.2.1`), enforced
atomically at the host accept step, with `E4020` fatal but `finally`-running
and a per-execution budget; native/REPL/library stdout stays unbounded process
stdout; long computation is independent of capture; stdin/args/virtual-project/
clock boundaries verified; machine endurance, unwinding, recovery, and
deep-value edges verified on fresh wasm; no recursive production seam. The
capture contract is documented in `docs/playground.md` §2.

PRE-0.3 FOUNDATION SUPER-TRANSACTION — **PUSHED THROUGH `b221e6a9`** (the
former local range is now on the remote). Commits establish:
poisoned-lock byte preservation (`5edab88`); single-source resource limits +
named sleep cap (`2fd1433`); unused-dependency removal (`3186340`); stale
recursive-engine documentation reconciliation + errors-reference sync
(`c1ffb18`); evaluator performance baseline (`9151df1`); exception-family
foundation, design-only (`34c5878`); capability/embedded-Python/
critical-profile architecture (`8ac9b93`); stdlib contract tests + module/
future-boundary maps (`a1a5aa4`); WASM transport-limit tests (`eb1e339`);
diagnostic taxonomy + internal-code documentation (`d4becc3`); runtime
architecture record + these state updates. Jev PRE/POST consulted for every
engineering iteration; the full iteration ledger (questions, evidence, Jev
outcomes, dispositions, commits) is recorded in the Pre-0.3 section of
`docs/engineering/CURRENT_HANDOFF.md`.

PRE-0.3 ADVERSARIAL RE-AUDIT + HUMAN SYNTAX GATE — **COMPLETE LOCALLY
2026-10-06, WAITING FOR HUMAN.** Bootstrap verified against the checkpoint
(branch, HEAD `1102b23`, remote `e576238`, ahead 15 / behind 0, range intact,
frozen hashes byte-identical, tag `v0.2.1` = `3f5f8702`, protected state
unchanged). The `e576238..1102b23` range was adversarially re-audited: routing
to the iterative machine mechanically re-verified (`production_routing` 4/4),
non-catchability of resource/host/internal/Python failures re-executed,
single-sourced limits confirmed, native/WASM parity re-measured, a deliberate
production-seam mutation was caught and reverted byte-exact. One in-range
documentation defect was repaired separately (`150befe`, `tests/host.rs`
merged doc-comment delimiter; behavior identical). The exception decision
package was verified by execution, refined, and committed as
`docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md`; pre-existing bare
`--no-default-features` test-hygiene gap recorded as TD-20 (not repaired;
outside the audited range). **No public exception syntax was implemented or
chosen: E1–E6 remain the human gate. Outcome A stands.**

B-1R phase state:

- **B-1:** **REMOTELY CLOSED** at `bb736fc`. Current-language iterative
  surface zero (21/21 `Expr`, 12/12 `Stmt`, no sentinel, no fallback);
  production cutover mechanically proven; unintended recursive production
  seams zero; whole-language differential campaign green; deferred
  exactly-once obligations closed; stack/resource/control/environment/
  diagnostic semantics verified; frozen historical artifacts unchanged. The
  released `0.2.1` WASM runtime still traps on the engine stack below the
  512-frame language limit — that defect is fixed in source and proven on a
  fresh build; it ships only when a new runtime is published (human-gated).
- **B-1R1:** DESIGN COMPLETE (`docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`).
- **B-1R2:** DIFFERENTIAL ORACLE COMPLETE AND MUTATION-VALIDATED
  (`docs/engineering/B1R2_DIFFERENTIAL_ORACLE.md`; 104-case corpus; isolated
  engines; `tests/oracle/golden.tsv`).
- **B-1R3A-ARCH-1:** RESOLVED — AST sharing uses `Arc`
  (`docs/B1R3A_AST_SHARING_DECISION.md`). Runtime `Env`/`Value`/`Closure` stay
  `Rc` intentionally.
- **B-1R3A:** COMPLETE AND REMOTELY CLOSED at `c9ade0b`. `src/run/iterative.rs`
  is a real explicit-continuation machine; the oracle has an identified R3A
  subset; all deliberate mutations detected and reverted.
- **B-1R3B.1–B-1R3B.7:** COMPLETE AND REMOTELY CLOSED (R3B.1/B.2 at `1a12b84`,
  B.3 at `2f34b9c`, B.4.1 at `efc66bd`, B.4.2 at `5e70677`, B.5/B.6 at
  `cf17689`, B.7 at `52124a0`).
- **B-1R3B.8:** COMPLETE AND REMOTELY CLOSED at `9cb5e28` (completion audit +
  dependency graph; `docs/engineering/B1R3B8_COMPLETION_AUDIT.md`).
- **B-1R3C.1 (Call):** COMPLETE LOCALLY — `61180e7`/`2064022`. User and native
  calls, argument source order and exactly-once, overloads/named args, the
  512-frame accounting, resumable `map`/`filter`/`reduce` callback protocol
  (machine work; no nested recursion), side-effect-observable differentials.
- **B-1R3C.2/C.3 (Method, Construct):** COMPLETE LOCALLY — `ecfd522`/`85e64fc`.
  `Value::Instance`/`Variant`, instance field reads, builtin/instance methods,
  receiver-before-arguments order.
- **B-1R3C.4 (Lambda, Pipe):** COMPLETE LOCALLY — `cf7c53f`/`9f415cd`.
- **B-1R3D.1 (Assign, LetPattern):** COMPLETE LOCALLY — `00f126c`/`fd53da7`.
  Includes the documented compound-target double evaluation.
- **B-1R3D.2 (While, Loop):** COMPLETE LOCALLY — `07caba0`/`81a745f`.
- **B-1R3D.4 (For):** COMPLETE LOCALLY — `e983a0d`/`32def41`/`95c90c0`.
- **B-1R3E.1 (ListComp, MapComp):** COMPLETE LOCALLY — `675ac76`/`f750c53`.
- **B-1R3E.2 (Match):** COMPLETE LOCALLY — `bab56bc`/`f982c53`.
- **B-1R3F.1 (Try):** COMPLETE LOCALLY — `a58c84f`/`71d0780`. Fatal-vs-catch,
  finally override, cross-frame `THROWN`/`pending_throw`, source attribution.
- **B-1R3F.1 unsupported=zero:** all 21 `Expr` + 12 `Stmt` variants handled;
  both wildcard fall-throughs removed. Whole-corpus engine agreement required
  (`tests/evaluator_oracle.rs::engines_agree`).
- **B-1R3G.1 (callback protocol):** COMPLETE LOCALLY (folded into B-1R3C.1).
- **Adversarial hardening (R3C–R3F):** three genuine bug families found by
  independent read-only reviews and fixed with mutation-tested regression
  coverage: (1) a false `match` guard retried later arms in the failed arm's
  scope instead of the match environment; (2) a fatal crossing an inner `try`
  without `finally` skipped outer regions (and their `finally`s); (3) an error
  raised in a catch/finally body through a callee frame leaked the frame and
  corrupted `interp.depth`/`expr_depth` (fixed with `frames_len`/`depth`/
  `saved_expr_depth` snapshots on `Cont::TryCatchEnd`/`Cont::TryFinally`).
  Eleven vacuous compile-error oracle cases were also replaced with
  runtime-exercising shapes.
- **Production cutover (`62dd592`):** COMPLETE AND PUSHED (inside the B-1
  closure range, remote-closed at `bb736fc`). Every
  production entry point now runs the machine: `Compilation::execute_with*`
  (with the sourced `run_iterative_sourced` branch for provider-backed
  compilations), the REPL statement/expression/const paths, the free
  `aura::execute_with`, and the Playground WASM wrapper (`execute`,
  `run_module_capture`). The recursive engine is retained as the differential
  reference and rollback path.
- **B-1R4 (full differential):** DONE — whole-corpus engine agreement
  (`engines_agree`), differential 228/228, syntax conformance 43/43.
- **B-1R5 (substrate boundary):** PARTIALLY DONE — fresh machine-backed WASM
  pinned by `playground/tests/node/b1_boundary.test.mjs` (Node cold path: 510
  legal / 511 E4011 for all mainstream shapes; module mode 511 legal / 512
  E4011; instance recovery); native CLI/REPL boundary canaries in
  `tests/cli.rs` and `tests/b1_production_path.rs`. The Chromium main-thread
  and production-Worker boundary at limit−1/limit/limit+1 cannot be exercised
  against the fresh artifact until a new runtime is published (human-gated,
  `playground/runtimes/**` immutable); the browser/Worker suites currently
  exercise the frozen `0.2.1` artifact only at depths it supports.
- **B-1R6 (red team):** DONE — independent read-only adversarial review of the
  cutover range. Findings: no production path reaches the recursive engine;
  `run_item_iterative` parity and playground ordering confirmed; host-factory
  mirroring exact. One genuine finding was independently reproduced and fixed:
  the CLI boundary test cannot discriminate an engine revert (the 64 MiB
  execution substrate masks recursion) — its comment now states that honestly
  and discrimination lives in the REPL canary and fresh-wasm boundary. Stale
  "feature-gated/experimental" docs on the machine and retained recursive
  APIs were corrected and the retained recursive REPL methods marked
  `#[doc(hidden)]`; a registry tripwire
  (`tests/builtins.rs::only_the_resumable_builtins_accept_callbacks`) now
  fails if a callback-taking builtin/method is added without extending the
  resumable protocol. Pre-existing, already-disclosed pattern-helper
  recursion remains; accepted depths bind without trap (E1015 above).
- **Final push gate (deliberate-falsification campaign):** six mutations were
  applied to the final architecture and reverted byte-exact
  (shasum-verified). Detected: REPL const seam→recursion (stack abort),
  eager-binary operand-order break and duplicated operand (oracle failures),
  short-circuit signal swallow (oracle failures), Playground `execute`
  →recursion (39/41 fresh-wasm boundary checks trap). Not behaviorally
  discriminable on native: the free library seam (64 MiB substrate masks it,
  by design), so a new mechanical routing tripwire
  (`tests/production_routing.rs`) pins every production entry's machine call
  spelling and was proven to fail when that seam is rerouted to recursion.
- **Independent adversarial review (fresh reviewer, full range):** all
  fifteen claims CONFIRMED with no falsification (unsupported-zero, cutover
  completeness, callback registry, call frames, closures/environments,
  loops/control, try, exactly-once ledger, stack safety, resource limits,
  diagnostics/spans, oracle conservation, frozen state, test
  discrimination, pattern-residual classification). Six minor findings;
  A/B/C/D/F fixed in `39caf6f` (field-receiver and tuple order
  differentials, `#[doc(hidden)]` on `Interp::run`, callback-confinement
  tripwire, wasm pattern-depth calibration pin); E did not reproduce
  (the recursive `run_item`/`eval_globals`/`exec_stmt_globals` methods
  exist, so the negative routing assertions are meaningful).
- **B-1R7 (full validation gate):** DONE LOCALLY — fmt, clippy (both feature
  configurations), full test matrix (50 suites each), MSRV 1.83, nightly fuzz
  check, playground suite, website suite, artifact smoke; frozen artifacts
  byte-unchanged.
- **B-1R8:** NOT STARTED — remove the recursive engine and the oracle switch
  only after cutover validation and a human release decision.

## Production vs experimental engine

- Production / default: **explicit-continuation machine** since the local
  cutover `62dd592`. The recursive evaluator is retained only as the
  differential reference (`Compilation::execute_recursive*`) and the rollback
  path; no production entry point reaches it.
- Iterative engine: compiled always (no longer feature-gated); it handles
  **every** `Expr` (21) and `Stmt` (12) variant with no recursive fallback; the
  whole corpus is required to agree between engines
  (`tests/evaluator_oracle.rs::engines_agree`). It supports literals, names,
  blocks, `let`/shadowing, `let` patterns, assignment (simple/compound,
  name/index/field targets), `if`/`while`/`loop`/`for` (lazy ranges),
  `break`/`continue` boundaries, `return`/`throw`, unary/binary/short-circuit
  operators, list/tuple/map/range construction, index/field reads, f-strings,
  user/native/closure/method calls (including the resumable
  `map`/`filter`/`reduce` callback protocol), struct/enum construction,
  lambdas, pipes, list/map comprehensions, `match`, and `try`/`catch`/`finally`
  (fatal diagnostics propagate but always run `finally`; only explicit
  `throw` is caught; cross-frame throws recover the value via
  `pending_throw`). `tests/b1_stack_safety.rs` pins host-stack independence for
  deep call frames, long loops, and deep expressions.

## Known blockers

- B-1 only. All HD/AUDIT decisions are resolved (ADR-0001…0004) and released
  in `v0.2.1`. No open CRITICAL/HIGH security blocker.

## Protected local state

- `.kilo/**`: pre-existing, **unstaged** tool-config churn unrelated to any
  engineering task. As of this checkpoint `git status --short -- .kilo/` shows
  **12 tracked deletions** (`.kilo/agent/*` × 7, `.kilo/command/*` × 3,
  `.kilo/run-script.sh`, `.kilo/setup-script.sh`) and **1 tracked modification**
  (`.kilo/kilo.jsonc`). Preserve exactly as Git reports it: do not stage,
  restore, modify, or commit any of it. Tool configuration is not engineering
  authority (`AGENTS.md`). Always trust `git status` over this summary.
- Root-level `s`: previously traced to a non-hermetic property test and fixed.
  It is **absent** and must remain absent; if it reappears, investigate rather
  than delete blindly.

## Human Decisions

Resolved by ADR-0001…0004 (see `docs/adr/`); do not reopen or re-queue.
- ADR-0001 — release and language versions are distinct.
- ADR-0002 — module members may reuse builtin spellings.
- ADR-0003 — CPython support tiers.
- ADR-0004 — AUDIT-3: structural `TypeExpr` nesting counts toward
  `MAX_AST_DEPTH = 256` on every substrate (RESOLVED).

## Verified Closed

- FSM-P1 — CLOSED
- FSM-P2 — CLOSED
- FSM-P3 — CLOSED
- FSM-P4 — CLOSED REMOTELY
- FSM-P5 — CLOSED REMOTELY
- FSM-P6 — CLOSED (included in `v0.2.1`)

## Do Not Start

Without new human authorization: package manager/manifests, browser
persistence, LSP, formatter product, async, macros, a new release or version
line, a new FSM phase, or any successor engineering program.

## Handoff

Before a model switch: finish the atomic operation, understand the diff, update
`AGENT_STATE.md` and `docs/engineering/CURRENT_HANDOFF.md` if state changed, run
`scripts/agent-state.sh`, and set writer ownership below. Synchronization is
Git + working tree + these documents; chat history is not authority.

## Aura 0.3 "Keystone" super-transaction (PUSHED THROUGH `b221e6a9`)

Started 2026-10-06 from local `8e4a59a` (then-remote `e576238`, 18 ahead / 0
behind); the whole closure range is now on the remote through `b221e6a9`. Scope manifest: `docs/engineering/KEYSTONE_SCOPE_MANIFEST.md`.
Adversarial-review ledger: `docs/engineering/KEYSTONE_JEV_LEDGER.md`.
Decisions RFC: `docs/rfcs/0001-exception-catch-selection.md`.

Landed locally (each with focused evidence, full-suite runs, and mutation
checks where applicable):

- `66ed6dc` exceptions E1-E6: pattern-based catch selection (a full pattern
  after `catch`), raise-site spans for uncaught `E4026`, reserved namespace
  root `Aura` (`E2023`).
- `3113fce` types: `never` bottom type (`E3006` for a normal-completion
  `-> never` body), `Ty::None` retention with precise `E3003` possible-none
  diagnostics, flow narrowing through `!= none`/`== none` guards including
  divergent-branch narrowing.
- `7dd8576` checker: Go-like unused analysis `E2008` (locals, parameters,
  pattern and catch bindings; `_`/`_name` discard; module-scope and REPL
  exempt; generator-aware corpus migration).
- `97c45fc` playground gates: the Native/WASM differential compares the two
  substrates *of the same revision* (fresh-source wasm), frozen runtimes stay
  covered by `cross-release`.
- `37715d3` Jev ledger records J1-J6.

Validated locally: `cargo test --locked --all-targets --all-features`
1171 passed / 0 failed; clippy `-D warnings` clean; fmt clean; playground
suite green (differential 228/0, syntax conformance 43/43, boundary 63/63,
browser/worker/multi-file/cache green). Frozen `0.0.2`/`0.2.0`/`0.2.1`
byte-identical; `v0.2.1` unmoved; nothing pushed.

All Keystone workstreams landed locally (29 commits, `8e4a59a`..`45271b2`):
exceptions E1-E6 (RFC 0001), `never`/`none`/narrowing, unused analysis
`E2008`, typed JSON `E4031`, HTTP capability (feature `http`), AIS/0.1,
structured diagnostics + CLI color, Aurea + overflow invariants, Playground
state machine + recovery, codename metadata, tooling reconciliation. Final
validation: all-features tests 1217/0; no-default 1142/0; clippy/fmt/MSRV
green; fuzz campaigns clean; Miri 124/0; playground + website suites green.
An independent adversarial review found 7 defects, all fixed with
regressions (`39b4935`).

Keystone went through four further adversarial review passes after the
initial closure. Each finding was independently reproduced by the writer
against the built CLI before it was fixed, never dismissed on reviewer
authority:

- Second review (`45271b2`): narrowing invalidation across closure calls,
  branch joins, and a loop-break in expression position; `E3008`→precise
  `E3003`.
- Third review: `never` reachability, shadowing, and
  `none`-narrowing precision (C1–C5).
- Fourth review: shadowed-callee divergence (F1/F1b),
  false-divergence narrowing (F2b), transitive closure capture (F3),
  literal-condition divergence (F4), `try`-`finally` divergence (F5),
  statically-empty iterable (F6).
- Fifth review: value-position divergence (`let x = boom()`,
  `print(boom())`, operands, `let x = { throw }`, doubly-diverging `if`) and
  method `-> never` divergence on a statically-known receiver — two
  wrong-reject classes, both fixed in `expr_diverges` (now modeling eager
  sub-expression divergence symmetric with the reachable-return scan) with
  regressions in `tests/keystone_types.rs` (48/48). The full validation floor
  was re-run on this state: all-features 1247/0 (56 suites), no-default
  canonical 1172/0, bare `--no-default-features` 1037/0 (TD-20 stays closed),
  clippy/fmt/MSRV 1.83/nightly-fuzz-check clean, playground suite green
  (browser 78/0, differential 228/0, boundary 63/0), website suite green
  (browser 380/0, a11y 70/0, links 2499 OK).

Pushed through `b221e6a9` (the whole Keystone range is now on the remote).
At the 2026-10-07 recovery checkpoint the local tip was `e7505408`, **2 ahead**
of remote `b221e6a9`: the unpushed parser wasm-safety fix that repairs the red
`playground (wasm runtime)` CI job. Remaining human gates: shadowing policy
change, package-manager product/security model, runtime publication/codenames,
and the authorization to push the 2-commit CI repair.

## Aura 0.3 "Keystone" canonicalization campaign (2026-10-08, LOCAL, UNPUSHED)

Human-authorized broad cleanup/reconciliation campaign on top of `8696f8ed`.
**No push, tag, release, or deploy was performed; the remote remains `b221e6a9`.**

Commits so far:

- `d5bb2fbd` **fix(parser): remove general semicolon statement/item sequencing.**
  Human override of the earlier decision: `;` is no longer a general statement
  or item separator. `item_separator`/`end_stmt`/`block_vec`/match-arm
  termination no longer accept `Semi`; the token is reserved (now for the Array
  type grammar). Grammar, spec (§3.4/§3.7/§19.1/CONF-PARSE-8), website mirrors,
  corpus, oracle fixtures and goldens updated. Negative regressions:
  `let a = 1; let b = 2`, `fn a() {}; fn b() {}`, `{;;}` are `E1006`.
- `49ef5298` **fix(types): project dynamic ranges as range_like, not object.**
  §5.4 is normative: a range's family is `RangeLike` and value kind `range`.
- `36c3d49f` **chore(docs): archive historical artifacts + collection decision
  package.** Root-level `AURA_*.md`/`IMPLEMENT.md` and the historical
  `CONFORMANCE_PHASE<N>`/`FEATURE_<NNN>`/audit/report series moved to
  `docs/archive/`.
- `d0ff3076` **feat(types,runtime): distinct List/Array/Tuple/Set/Map
  collection identities.** The human collection decision, implemented
  end-to-end atomically: `[T; N]` Array type (semicolon-separated, compile-time
  bounded length), contextual bracket-literal realization under an Array
  expectation (one seam per engine; no implicit List↔Array conversion), real
  Tuple identity (superseding list sugar), Set `{a, b}`/`set{}` with the
  `SetLike` family and key-capable members, plus `Ty`/`Value`/`TypeFamily`/
  equality/indexing/iteration/mutation/patterns/repr/JSON-typed-decode/AIS
  updates. Also added the observable sharded validator `scripts/validate.py`.
- `86eb6925` **docs,collections: finalize the Keystone collection algebra.**
  ADR-0005, LANGUAGE_SPEC §5.1/§5.4/§21 rewrite, grammar + website mirror,
  guide-collections, one-element-tuple repr `(1,)`, resumable callback methods
  on Array/Tuple, migrated superseded tests, regenerated four oracle goldens.
- `c640fc64` **refactor(stdlib): route array/tuple callbacks through one
  resumable body.** The `production_routing` tripwire correctly rejected the
  first cut (it had introduced a second set of recursive callback sites); the
  three sequence kinds now share one `sequence_callback_method` body, so
  exactly three `call_value_pub` sites remain. Also added Set `add` to the
  front-end language metadata (`language_metadata` tripwire).
- `ce8deb0a` **feat(json): direct-type `json_decode_as(text, Type)` API.** The
  final Keystone typed-JSON contract closes the last explicit human API
  requirement: the second argument is a **type argument** in the canonical
  grammar (new `Expr::TypeRef`), resolved by the parser and checker, never a
  runtime string. A string literal in that position is a compatibility spelling
  normalized to the same node (one type path). The checker resolves unknown/
  malformed types statically and `infer` returns the decoded type; both engines
  decode via the existing recursive decoder. Regression file
  `tests/keystone_typed_json_api.rs`; spec §22/grammar/website mirrors updated.
- `f512913b` **fix(stdlib): end the receiver borrow before running a sequence
  callback.** The shared `sequence_callback_method` was invoked with
  `l.borrow().clone()` as an argument, keeping the `RefCell` borrow across the
  callback; a callback that mutated its own receiver panicked. The snapshot is
  now bound to a local first (matching the established snapshot semantics).
  Found by `evaluator_oracle` (`callback/callback_list_snapshot`).

Frozen `0.0.2`/`0.2.0`/`0.2.1` verified byte-identical (hashes match the table
above); `v0.2.1` tag unmoved at `3f5f8702`.

**Closure validation (2026-10-08, local green — not GitHub CI).** all-features
`scripts/validate.py` **59/59 PASS**, 0 FAIL, 0 TIMEOUT (observable/sharded;
no opaque multi-hour wait). Also green: lib all-features 167/0 and bare 124/0;
canonical `cli,repl,json,regex,time`; MSRV `+1.83.0 check --all-features`;
nightly `fuzz` check; playground (differential 233/0, syntax conformance
77/77 incl. array/tuple/set and the direct-type typed-JSON API, boundary 63/0,
browser 98/0); website (examples 22/0, links 2500 OK, a11y 70/0, browser
380/0, cross-release 51/0 ×6 runtimes); fmt + clippy `-D warnings` clean in
both all-features and bare configurations. Real defects found by this
validation and fixed with regressions: callback-routing tripwire,
language-metadata inventory, receiver-borrow snapshot, bare-config
feature-gating of the typed-decode path.

## Exact Next Action

See `docs/engineering/CURRENT_HANDOFF.md`. In short: HEAD is `02c3c897` (PR #24
merged, twenty-four PRs merged this campaign); Aura 0.3.1 (Keystone) is published
and `v0.3.1` = `b09cbef2`. Batches 1–5, B6/B7/B14, TD-23, TD-24, TD-27,
Directive 004, TD-13, the resolver type-parameter leak and the set key-method
element typing are **merged and closed**. Directive 003-R §4's blockers are
closed.
**Exact next action:** begin the next CORE FIRST work item — a real, reproduced
defect in `src/`, `tests/`, or `fuzz/`, shipped as one PR carrying a reproducible
case, a regression test that fails before the fix, the fix, and a passing test.
Highest-value unfinished leads already recorded here: (a) lexer/parser
ambiguities, precedence, adversarial input and error recovery,
(b) checker scopes, generics, mutability, (c) evaluator control flow, closures,
resource limits, (d) collections and bounds, (e) module cycles,
(f) Host/Python/WASM capability and panic boundaries.
Probed and found conformant (do not re-investigate): HOF arity and ordering
(`reduce` is `(list, fn, init)`), `zip` truncating to the shortest input,
`map`/`filter`/`enumerate`, closure capture by reference with per-call cells,
`regex_*` on malformed patterns (`E3001` with the parse error) and linear-time
behavior on adversarial input, and `contains` on a foreign argument returning
`false` (documented "1 any", cannot corrupt state).
Do not begin language features frozen for stabilization. B-1R8 (remove the
recursive engine and the oracle switch) and any runtime publication remain
separately human-gated.
**Lesson already paid for:** `--all-features` validation structurally cannot see
a `pub(crate)` item reachable only from a feature-gated module. Keep the
no-`repl` lib check in the floor.
**Validation (TD-22):** use the observable sharded `scripts/validate.py` (not a
monolithic watch) for the Rust matrix, and include the no-`repl` lib check
above — the sharded runner alone will not catch feature-gated dead code.

## Writer

NONE (single-writer discipline; this session held exclusive worktree ownership
and released it at session end).
