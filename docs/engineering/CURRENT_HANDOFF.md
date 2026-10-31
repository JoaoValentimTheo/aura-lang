# Current Engineering Handoff

## Current task — CORE FIRST: resolver type-parameter leak — merged

**Merged as PR #22 (`92f5cf18`), CI 21/21 green on round 1.** No red gate, no
retry, no gate weakened. `mergeStateStatus` was `CLEAN` and `reviewDecision`
empty, so no external human approval was required or simulated.

`rewrite/v3-rust` is the **default branch** and HEAD is **`92f5cf18`** (the
merge of PR #22). Twenty-two PRs are merged this campaign: #2, #3, #4, #5, #6, #7,
#8, #9 (B14), #10 (PR template), #11 (TD-24 cost guard), #12 (CHANGELOG
`[Unreleased]` + TD-25), #13 (website suite gate + TD-26), #14 (TD-27 Playwright
gate), #15 (state sync), #16 (B7 project-header contract + HTTP transport docs +
`test:abi`), #17 (validation orchestrator self-tests in CI), #18 (state sync for
#15–#17), #19 (browser consent scoped to the mutation class), #20 (Directive
004: the seven agents), #21 (CORE FIRST: REPL session state incremental, TD-13
closed, session APIs feature-gated), #22 (CORE FIRST: resolver type-parameter
frame popped on the error path).
The Directive 003-R authorization recorded in
`AGENTS.md` permits push, PR creation and merge, workflow administration, and
default-branch changes; force-push, history removal, tag moves, frozen-WASM
replacement, gate weakening, and unapproved stable releases remain forbidden.
Never report a gate as green unless it actually executed.

### The finding — a rejected declaration poisons the session

`Resolver::rewrite_item` and `Resolver::rewrite_member` push a generic-parameter
frame before rewriting a declaration's body and pop it afterwards, with the pop
written as a plain statement at the end of the arm. Every arm has fallible work
between the two: `?` on `rewrite_params`, `rewrite_type`, `rewrite_block`,
`rewrite_field`, `rewrite_variant`, `rewrite_member`, `canonical_type`. When one
of those returns `Err`, the `?` leaves the function before the `pop` runs, so the
frame — and every generic name in it — stays on `type_param_scope` for the rest
of the session.

That stack is load-bearing for path canonicalization. `flatten` calls
`rewrite_type`, which asks `is_type_param(name)` first; a `true` answer means
"compile-time placeholder, do not canonicalize". A leaked name therefore stops a
real, declared type from being canonicalized, and the reference reaches the
checker as a bare unknown type. The user sees an unknown-type error on a line
that is perfectly valid, and the diagnostic for the *actual* mistake is the only
correct one in the output.

Reproducer (`/tmp/v1/min1.txt`, one line per submission; the `impl` targets a
private struct so its rewrite fails with `E2018` after `Q` was pushed):

```aura
module m { pub struct Q { v: int }
struct Priv { v: int } }
impl<Q> m::Priv { fn get(self) -> int { return self.v } }
use m::Q
fn takes(x: Q) -> int { return x.v }
:quit
```

Before the fix the final submission reported `E3002: unknown type `Q``; after it,
the correct `E2018: field `v` of `m::Q` is private; it cannot be read from this
module`. Five controls (no `impl`; `impl` alone; `impl` with a non-colliding
parameter name; a `struct`/`enum`/`alias`/`trait` with a failing body and the
same colliding name) all produced the correct diagnostic both before and after,
which is what pins the cause to the leaked frame rather than to the generic
syntax in general.

### The fix

An RAII guard, `TypeParamFrame<'a>`, holds `&'a Resolver` and pops on `Drop`; each
of the seven push sites binds `let _frame = TypeParamFrame::new(self);`
immediately after `push_type_params`. The pop is then unconditional, including on
the error path, and `let _` keeps the binding alive to end of scope without
triggering `unused_variables`.

The guard borrows the resolver immutably, which is sufficient because
`type_param_scope` is a `RefCell`: `push`, `pop` and `is_type_param` take `&self`
and borrow only for the duration of the call, so no `RefCell` borrow is held
across the rewritten work.

Deliberately **not** the fix: clearing `type_param_scope` in `reset_transient`.
That is what the checker does for its own stack, and it hides a per-declaration
invariant violation behind a per-submission reset — the stack would still be
wrong for the remainder of the submission that leaked it, and any future caller
that resolves more than one declaration per reset would inherit the bug. The
guard fixes the invariant instead of masking it.

### The checker's identical shape — verified safe, do not re-litigate

`src/check/mod.rs` has the same unguarded push/pop pattern at nine sites
(`hoist` struct/enum/alias/fn, trait and impl registration, the body-phase `fn`
and its nested block). It is **not** a bug and is **not** touched here, for three
independent reasons:

1. `Checker::check` (1067–1074) is strictly fail-fast: `for … { self.item(item)?; }`
   returns on the first `Err`, so a leak can never be observed by a later item in
   the same file.
2. There is no multi-diagnostic mode anywhere in the crate — no `Vec<Diag>`, no
   `errors.push`, no `collect_diagnostics` in `src/check/mod.rs`, `src/lib.rs` or
   `src/main.rs`. The three catch sites that look like they continue
   (`block_with` 5565–5581, the `for` loop 5961–5968, `try` 5986–5990) each
   `return Err(e)` immediately after popping.
3. `Checker::reset_transient` clears `current_type_params` (1046–1047) and the
   REPL calls it before every submission (`src/repl.rs:341-342`, `455-456`), so
   the stack is empty when each diagnostic is produced.

`Resolver::reset_transient` (367–373) clears only `applied_imports`,
`item_sources`, `active_source` and `resolved_sources` — that asymmetry, not a
difference in the push/pop code, is why the resolver leak is observable and the
checker one is not. An earlier session concluded the checker leaked too; that was
reproduced in a directory already holding a stray `.aura` fixture and was wrong.
Note the fixture hazard: `compile_file_with_mode` discovers module sources by
scanning the target file's directory, so one stray `.aura` file makes every
`aura check` in that directory report *its* error regardless of the path given.
One fresh directory per probe file.

### Guards

`tests/repl.rs::a_failed_generic_declaration_does_not_leak_its_type_parameters`
pins the behaviour: it asserts the `E2018` for the private struct is present, that
`E3002` is absent, and that the private-field diagnostic is the one reported. It
fails on the pre-fix tree at the `E3002` assertion.

### Validation executed on this tree

Executed locally on `2d7f6356` before the merge; remote CI then re-ran the same
surfaces.

- `cargo test --locked --test repl` → 54 passed, 0 failed (53 before the new test)
- `cargo test --locked --all-targets --all-features` → 66 suites pass, 0 failed
- `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` → 66 suites pass, 0 failed
- `cargo clippy --locked --all-targets --all-features -- -D warnings` → clean
- `cargo fmt --all -- --check` → clean
- `RUSTFLAGS="-D warnings" cargo check --locked --lib --no-default-features --features cli` → clean
- the `/tmp/v1` reproducer and its controls, each in its own fresh directory → correct `E2018` everywhere, no `E3002`
- remote CI on the PR → **21/21 pass**, round 1

### Resume point

HEAD is `92f5cf18`. **Exact next action:** continue CORE FIRST with the next
reproduced defect — the `Stmt::Let` partial-mutation leak test, then
lexer/parser ambiguities, precedence and error recovery; checker
scopes, generics and mutability; evaluator control flow, closures and resource
limits; collection bounds; module cycles; runtime panic and Host/Python/WASM
capability boundaries. One PR per finding, each carrying a reproducible case, a
regression test that fails before the fix, the fix, and a passing test. Keep the
no-`repl` lib check in the floor. Do not begin language features frozen for
stabilization. B-1R8 and any runtime publication remain separately human-gated.

## Previous task — CORE FIRST: REPL session linearization (TD-13) — merged

Historical record of PR #21. HEAD was `2fe944fb` at the time.

### The finding

The ADENDO "CORE FIRST" made the directive-authorized REPL O(N²) defect the
first core priority. It was reproduced, quantified, localized, fixed, and
pinned. Branch `perf/repl-linear-session` off `38f5fed5`, merged as **PR #21
(`2fe944fb`), CI 21/21 green**.

**Symptom.** Release profile, min-of-3, distinct declarations: the `let` path at
N=250/500/1000/2000/4000 cost 0.174/0.337/1.174/4.548/**18.299 s**; the `fn`
path at N=4000 cost **98.270 s**. Cost scaled with the number of **distinct**
declarations — shadowing the same name stayed linear — and a marginal-cost probe
(extra `1+1` submissions over a fixed declaration count) showed a **constant
~658 µs per submission at every depth**, which is a fixed per-submission cost
multiplied by session depth.

**Root cause.** `SessionEngine::eval_line` called `Checker::with_declarations`
on every submission, rebuilding the whole checker from every prior declaration,
so submission *k* did O(k) work.

### The fix — three layers plus two follow-on narrowings

1. **Incremental checker.** `SessionEngine` owns one `Checker` for the session;
   `reset_transient()` clears per-submission frames and
   `extend_declarations(&decls[applied..])` registers only the new declarations.
   The module path needs no `extend_declarations` call — `check_repl_mode` →
   `check` → `hoist` + annotation pass + `item()` already register every table.
2. **`abort_submission` (found by the oracle).** With a cached checker a failing
   submission leaves partial mutations. `fn k(int)` twice then `k(1)` reported
   `E3001 ... 2 overloads accept` instead of `1`, because `hoist` pushed the
   duplicate signature before the duplicate scan rejected it. The helper rebuilds
   the checker from the session on the three paths that can fail *after*
   mutating: stmt-path `check_stmt`, module-path `check_repl_mode`, and
   module-path execution. Resolve failures are deliberately not hooked — nothing
   has mutated yet — and neither are the `executed == false` stmt arms, because
   the persistence block that follows must still record the user's binding.
3. **`session_value_names` symmetry.** `hoist` never wrote the table that
   `extend_declarations` maintains, so the cross-kind clash checks in the
   `Item::Fn` and `Item::Const` arms silently stopped firing across submissions:
   `const X = 1` then `const X = 2` went from `E2007`+`1` to `2`, and `fn X` then
   `const X` from `E2007`+`<fn>` to `2`. Both writes added.
4. **Q1 — duplicate-scan narrowing.** The overload duplicate scan re-examined
   every name in `self.functions` on every submission, but a duplicate can only
   be created by a signature pushed during the *current* call: `self.functions`
   has exactly three write sites and `add_global` pushes only into an empty set.
   The scan now drains a `rescan_functions: HashSet<String>` and scans only those
   names, leaving the comparator, the nested loop, the `active_source` side
   effect and the `E2007` diagnostic byte-identical.
5. **Q2 — persisted resolver.** `Resolver` now lives in `SessionState` instead of
   being rebuilt per submission. `extend_session` seeds items and imports past
   `seeded_items`/`seeded_imports` watermarks held on the resolver, which makes
   the move idempotent by construction; `reset_transient` clears only
   `applied_imports`, `item_sources`, `active_source`, `resolved_sources`.
   Imports must **never** be re-applied — `apply_use`'s `name_visible_locally`
   check would raise a spurious `E2007` — and the watermarks must **not** be
   reset or the quadratic cost returns. Every other resolver table is
   monotone/append-only and safe to persist.

### Measured result

| Build | `let` N=4000 | `fn` N=4000 |
|---|---|---|
| pre-fix | 18.299 s | 98.270 s (and 6.621 s from a mid-campaign baseline) |
| after Q1 | — | 2.734 s |
| **after Q2 (shipped)** | **0.238 s** | **0.411 s** |

`fn` path N=4000: **6.621 → 0.411 s = 16.1×**, doubling ratio 3.88× → 2.47×;
against the pre-fix binary **98.270 → 0.411 s = 239×**. The `let` path at N=4000
is **93.5×** faster than its pre-fix baseline. Shape to N=16000 for `fn`:
0.080/0.148/0.295/0.763/2.008 s (1.86/1.99/2.59/2.63×) — linear modulo cut-off
effects.

### Method notes worth keeping

- **`sample`-based profiling does not work in this harness.** It attaches to the
  shell wrapper, not the `aura` child, and yields `???`/`start`/`__wait4`
  frames. In-source phase instrumentation behind an env var (12 `AtomicU64`
  counters) is the reliable substitute; it attributed the module path at N=2000
  to **check 56% / resolve 43%**, with exec/decl/items ~0%.
- **Ablate data, never control flow.** An early `AURA_SKIP_SCAN` used
  `return Ok(())` inside an inner block, which returns from the entire enclosing
  function and disables everything after it; the measurement was confounded and
  had to be re-run by substituting `Vec::new()` for the block's input instead.
- **The combined ablation is what proved the model.** Neither Q1 nor Q2 alone
  linearizes the `fn` path (each ablation alone left it quadratic); ablating both
  together gave 0.044/0.078/0.149/0.313 s, i.e. linear — the two contributors are
  additive and their sum is necessary *and* sufficient. An earlier
  "no dominant driver" reading was overturned by exactly that experiment.
- **The `aura` binary needs an explicit `repl` subcommand.** Piping stdin to
  `target/release/aura` with no arguments prints the usage banner and exits 0 in
  ~13 ms, so a naive timing probe silently reports meaningless numbers at every
  depth.

### Guards

- `tests/repl_cost.rs` (new, untracked until this commit): a counting global
  allocator asserts a submission's allocation volume does not grow with session
  depth. It measured **4.19× for 4× depth** before the fix and passes now. It
  *cannot* see CPU-only costs, which is precisely why Q1 and Q2 are guarded by
  wall clock in `tests/bench.rs` instead.
- `tests/bench.rs`: `repl_incremental_state_is_linear_in_session_depth` —
  rewritten and renamed from the old weaker "no worse than quadratic" TD-13
  guard (base 800, shared 3.2× limit; the quadratic path measured 3.33–3.51
  there), and `repl_function_declarations_are_linear_in_session_depth` — new,
  covering the module path that Q1 and Q2 actually change (base 800; the
  rescanning path measured 4.18× at that base, the incremental path ~1.9).
- `tests/repl.rs`: two new tests pin the cross-kind `E2007`s
  (`session_const_after_same_named_function_is_rejected`,
  `session_function_after_same_named_const_is_rejected`). All three such tests,
  including the pre-existing `const_persists_and_cannot_be_redefined`, were
  proven to fail without layer 3.

### Equivalence evidence

A 25-case REPL oracle (stdout + diagnostics per submission) and a 6-case
session-names oracle both report **zero divergence** against the pre-fix binary —
checked after each layer, after Q1, after Q2, and again after the `SessionState`
refactor. `SessionState` groups `decls`/`checker`/`applied`/`resolver` so
`abort_submission` fits clippy's 7-argument limit honestly, not via
`#[allow(clippy::too_many_arguments)]`.

### Validation actually executed on the shipped tree

- `cargo fmt --all -- --check` → clean
- `cargo clippy --locked --all-targets --all-features -- -D warnings` → clean
- `cargo test --locked --all-targets --all-features` → 66 suites pass, 0 failed
- `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` → 66 suites pass, 0 failed
- `cargo test --locked --test repl --test repl_cost --test bench --features repl` → 53 / 3 / 15 passed, 2 ignored
- `cargo +1.83.0 check --locked --all-features` → clean
- `RUSTFLAGS="-D warnings" cargo check --locked --lib --no-default-features --features cli` → clean (plus `cli,json,regex,time`, `http-api`, `cli,evaluator-oracle`, `json,regex,time`, `py`: all clean)
- `cargo +nightly check --manifest-path fuzz/Cargo.toml --all-targets` → clean
- `cargo clippy --manifest-path playground/runtime/Cargo.toml --all-targets -- -D warnings` → clean
- `cargo build --manifest-path playground/runtime/Cargo.toml --release --target wasm32-unknown-unknown` → clean
- `node playground/tests/node/run-all.mjs` → all suites 0 failed
- `RUSTDOCFLAGS="-D warnings" cargo test --doc` → 0 tests, clean (both feature sets)
- both oracles → 25/25 and 6/6 identical against `/tmp/p_before.json` and `/tmp/sn_before2.json`

Not run locally: miri (deferred to remote CI, which runs it green), browser
suites. Neither touches this change.

### Full validation floor — permanent lesson from PR #21

The first push of this branch was **5-failed / 16-passed** on remote CI, and every
failure reduced to one real defect in the change: `Checker::session`,
`Checker::reset_transient`, `Resolver::session` and `Resolver::reset_transient`
are `pub(crate)` and reachable only from `src/repl.rs`, which `src/lib.rs:30`
gates with `#[cfg(feature = "repl")]`. In any build **without** `repl` they are
dead code, and CI's `RUSTFLAGS: -D warnings` makes that a hard build error.

**Why the local floor missed it:** every command in it enabled `repl` —
`--all-features` does, and so did the "CI feature set"
`cli,repl,json,regex,time`. Nothing in the matrix ever built the library without
that feature. Jobs that do, and therefore caught it: `miri`
(`--lib --no-default-features --features cli`), `language contract`
(`cargo run --no-default-features --features cli -- version`), `fuzz smoke`,
`playground (wasm runtime)`'s "Lint the runtime crate", and the `website` job's
`playground/tests/node/run-all.mjs` step.

**The rule:** a `pub(crate)` helper reachable only from a feature-gated module is
dead code in every other feature set, and `--all-features` validation cannot see
it. The floor now includes

```
RUSTFLAGS="-D warnings" cargo check --locked --lib --no-default-features --features cli
```

Use `--lib`, **not** `--all-targets`: `tests/cli.rs` uses `serde_json`
unconditionally (lines 795 and 916) and fails with 24 `E0433` errors under
`--all-targets --no-default-features --features cli`. That breakage is
**pre-existing** — verified with `git show --name-only 404980e5`, which does not
list `tests/cli.rs` — and CI never builds that combination. Recorded as a known
gap, not a regression of this PR.

The alternative fixes were rejected deliberately: making the four methods `pub`
widens the public API to silence a lint, and `#[allow(dead_code)]` weakens a
warning to buy green. Both are forbidden by the PR template and Directive 003-R.
Gating with `#[cfg(feature = "repl")]` is the honest statement of the fact that
these methods only exist for the session lifecycle. One consequence: the
intra-doc link `[`Checker::reset_transient`]` inside `Checker::session` had to
become a plain code span, because a link target that is feature-gated away would
otherwise fail `rustdoc::broken-intra-doc-links` under `-D warnings`.

Also worth knowing: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --lib` reports
**8 pre-existing** broken intra-doc links, all in `src/run/` (`RunSession`,
`session_impl::Session`, private `Interp` methods). CI never runs `cargo doc`, and
none of those files are touched here, but do not mistake them for a new failure.

### Resume point

PR #21 is **merged** (`2fe944fb`, CI 21/21 green on round 2). The
`#[cfg(feature = "repl")]` gating commit is `3a42deee`; the performance work is
`404980e5`. Both state documents are reconciled to the merge SHA and the writer
is released (`NONE`).

**Exact next action:** continue CORE FIRST with the next reproduced defect. The
smallest open item is the `Stmt::Let` partial-mutation leak test; then
lexer/parser ambiguities, precedence and error recovery; checker scopes, generics
and mutability; evaluator control flow, closures and resource limits; collection
bounds; module cycles; runtime panic and Host/Python/WASM capability boundaries.
One PR per finding, each carrying a reproducible case, a regression test that
fails before the fix, the fix, and a passing test. Keep the no-`repl` lib check
in the floor. Do not begin language features frozen for stabilization. B-1R8 and
any runtime publication remain separately human-gated.

### Deliberate non-fixes

- The O(Σ set²) → O(Σ set) keyed duplicate-scan rewrite is deferred: `Ty` derives
  `Eq` but not `Hash`, so a key must be `(usize, Vec<Option<Ty>>)` compared with
  `Eq`.

## Directive 004 — Kilo → GitHub Copilot agent migration (merged, `38f5fed5`)

Merged as PR #20 (`dc16b4fe` + `d931f888`), CI 21/21 green including miri, branch
deleted. Kept here because the `400` diagnosis is expensive to rediscover.

### The `400 The requested model is not supported` diagnosis

The directive assumed the error was a single misconfiguration. Controlled
experiment showed **three independent root causes**; the first two are
configuration, the third is a VS Code architecture boundary.

**A — Agent Host hardcoded default.** `agentHostMain.js` declares
`WS="gpt-5.6-luna"`, used as the default for
`chat.agentHost.copilot.executionSubagent.model` and returned by
`_getExecutionSubagentModel()` when unset. `gpt-5.6-luna` is not registered in
this BYOK-only deployment, so an unpinned `explore` dispatch returns `400`.

**B — SDK built-in agent YAMLs.** `@github/copilot/sdk/definitions/
{explore,task}.agent.yaml` declare `model: claude-haiku-4.5`. Pinning exactly
that id reproduces the `400` verbatim, and overriding the
`github.copilot.chat.exploreAgent.model` setting does not change the outcome.

**C — external platform limitation, not a repo defect.** `MEn`/`TEn` in
`workbench.desktop.main.js` split customization payloads by source:

```js
function TEn(s){return(s.type==="skill"||s.type==="agent")&&(s.source===Dr.local||s.source===Dr.user)}
let w=v.filter(k=>!TEn(k));   // → synced.bundle
let C=v.filter(TEn);          // → standalone.bundle
```

`.github/agents/*.agent.md` resolve as `type:"agent", source:local`, so they are
excluded from the synced plugin by construction. The standalone authority is
`<sessionType>-<hash>-standalone`; **no such plugin directory is ever written
to disk** (`ls ~/Library/Application Support/Code/agentPlugins/` shows four
`vscode-synced-customization-*` authorities and no `-standalone`),
`agenthost.log` records exactly one synced sync and one
`appliedCount=0` customization event, `renderer.log` never logs a standalone
push, and the Rust `runtime.node` never scans `.github/agents/` (its only two
occurrences are a docs URL and a system-prompt *prohibition*). The Agent Host's
`customAgents` is therefore structurally empty and `agent_type: aura-scout`
returns the seven builtin names.

**Consequence:** the seven agents are fully valid and fully discovered by the
workbench (`customizationsDebug.log`: `agents 7 0 2 9`, all seven paths, zero
rejections), and an explicit `model:` pin works for builtin agent types. What
cannot work in this VS Code build is dispatching a *custom* agent by
`agent_type` in the Agent Host harness. **Do not re-investigate C.** No
repository change can alter it.

Merged and closed; nothing about Directive 004 remains open.

**PR #13 — merged `19cc4f22`.** It closed the last red suite on the default
branch and repaired the gate that hid it. CI evidence: the `website` job ran
all twelve suites and logged zero `FAIL` lines (`stdlib-consistency` 123/0,
`docs-consistency` 138/0, `ais-doc` 43/1 skipped, `playground-parity` 93/0,
`semver` 52/0, `release-manifest` 12/0, `release-preflight` 32/0, `sbom` 9/0,
browser suites self-skipped per TD-27).

- **The default branch was red through eleven green PRs.**
  `website/tests/stdlib-consistency.test.mjs` asserted
  `/native-only/i.test(stdlibDoc)`, but `11aeb093` (B6/B7) had deliberately
  changed `website/content/reference-stdlib.md` to "host-owned" because B14
  gave the browser Playground HTTP on Host ABI 2. Assertion and doc had
  contradicted each other, undetected, since that commit. Proved pre-existing
  at base `c9f88665` by detached checkout before touching anything.
- **Root cause of the invisibility — TD-26.** The `website` CI job named four
  suites by hand and never invoked `website/tests/run-all.mjs`, so eight
  suites ran only when someone remembered. Of those eight, only
  `stdlib-consistency` was failing (measured, not assumed: `docs-consistency`
  137, `ais-doc` 89, `aurea` 39, `playground-parity` 93 passed; `nav-fit` and
  `aurea-browser` self-skip without Playwright).
- **The fix is bidirectional.** The stale assertion was replaced by two
  direction-guarded assertions — `host-owned` present **and** `native-only`
  absent, plus the browser session path present — so drift is caught in both
  directions. Mutation-verified both ways. An assertion that merely demands
  current wording cannot detect drift.
- **`guide-io.md` was also stale**, not just the test. The browser column
  claimed the filesystem's `native-only` status applied to HTTP and gave
  `E5002` unconditionally. Corrected to distinguish the default `0.3.1`
  runtime (ABI 1, no browser HTTP, `E5002`) from the ABI 2 dev line. The
  filesystem `native-only` claim on line 46 is **correct and untouched**;
  `strings` on the frozen wasm confirmed the ABI-1 virtual project really
  does deny HTTP.
- **Wiring the gate paid for itself immediately.** The first PR carrying
  `run-all.mjs` went red on `docs-consistency`, which verifies each normative
  page linked from the docs index still exists at the release tag. CI checks
  out at `fetch-depth: 1` without tags, so `v0.3.1` was absent and all four
  `git ls-tree` checks failed. Reproduced exactly with
  `git clone --depth=1 --no-tags file://$PWD` (zero tags, 133 passed /
  4 failed); fixed by `fetch-depth: 0` on the job (753 commits, 20 MB `.git`,
  on a job that already builds 46 pages) plus a tag-resolvability preflight
  that reports a missing tag once as itself instead of four times as missing
  documentation. Mutation-verified: tagless 133/5, tags restored 138/0.
- **TD-27 (OPEN, v0.3.2).** No CI job installs Playwright, so browser, a11y,
  browser-HTTP and http-security suites have always skipped — including B14's
  browser HTTP path. The `ci.yml` comment claiming they "run under Chromium
  when Playwright is available" was false and is corrected. Deliberately not
  wired here: adding a browser download changes the job's runtime and failure
  modes and deserves its own change with its own evidence.

**Exact next action:** PR #16 completed the B7 project-header contract, so the
item that used to be listed here is done. Directive 003-R §4 is now fully
closed: B6 is a formalized divergence pinned by `http-security.test.mjs`
("4 redirected call reports E4020", plus "never prints a status"), B7's
single-module and project-session header contracts are both pinned, B14 shipped
as `0.3.2-dev.8`, and TD-23 is behaviourally verified (scheduled run
`41037980226` executed 30 min per front-end target plus a 5 min runtime sweep,
96 min wall time, no crash artifacts). Continue §6 on the remaining
high-yield surfaces — Host/Python/WASM security, benchmark regressions,
fuzz-target coverage, differential/property suites, supply chain, and any
`test:*`-style script or suite that no runner invokes. Two such rot instances
are already fixed (`test:abi` in #16, `scripts/test_validate.py` in #17); an
audit of both test trees found every remaining suite wired, so look next for
*assertions* that cannot fail rather than for unwired files.

**TD-27 — merged `9548e4df`.** Closed the last unproven surface in CI. The
browser suites had never executed: no job installed Playwright, and nine suites
treated a missing dependency as *success*, so a broken `npm ci` or a failed
browser download left CI green. The fix is a shared
`website/tests/playwright-deps.mjs` helper (`detect` / `detectAll` /
`strictRequested` / `skipExitCode`), `AURA_REQUIRE_BROWSER=1` on both jobs, and
real installs with `--with-deps`. Evidence is behavioural: the `playground` job
logged `Cache not found`, downloaded Chrome for Testing 153.0.8010.12 plus
FFmpeg and Headless Shell, ran the apt step on ubuntu-24.04, and executed six
browser suites (199 assertions, zero skipped); the `website` job executed four
(897 assertions). The download path had never run before because a warm local
cache made `playwright install` a no-op locally. No gate was weakened.

Doctrines #1-#33 from across the campaign are now durable in `AGENTS.md`
(`## Hard-won rules`, 28 numbered rules in three groups), so they no longer
depend on session history.

Authoritative for **the active task and the exact next action**. Current
repository state lives in `AGENT_STATE.md`; operating rules in `AGENTS.md`.
Keep this file short — it is read at the start of every session.

## HISTORICAL — FEATURE DEVELOPMENT FROZEN; READ-ONLY AUDIT (2026-10-09)

**AURA FEATURE DEVELOPMENT — FROZEN FOR STABILIZATION.** The 0.3.2 Playground
development preview is published: `rewrite/v3-rust` at `9f8743ee` (= remote),
CI 20/20 green, Pages deployed, live Playground verified (stable 0.3.1 runs;
dev.5 does large-output accounting and real HTTP after consent; PokéAPI →
`200\npikachu\n`).

No new syntax / stdlib / Host capability / Playground / ABI work. Correctness
and security defects reproduced against the frozen surface remain eligible for
narrow fixes.

**Next campaign (this session, read-only first):**
1. A read-only engineering audit — findings in
   `docs/engineering/STABILIZATION_AUDIT.md`, each with file/location,
   governing authority, minimal reproducer, expected vs observed, severity,
   evidence, smallest safe correction, and a regression plan; classified
   CONFIRMED / FALSIFIED / DECISION REQUIRED / INSUFFICIENT EVIDENCE.
2. An **AIS Agent-Native RFC** (design only; no implementation) —
   `docs/engineering/AIS_AGENT_NATIVE_RFC.md`.

No production code is modified during the audit stage.

## HISTORICAL — AURA 0.3.2 PLAYGROUND & BROWSER HTTP (2026-10-09, PREVIEW)

The full campaign record is `docs/engineering/PLAYGROUND_032_CAMPAIGN.md`.

- **D1 + D2 implemented.** `Machine` is a transient view over
  `(&mut Interp, &mut MachineState)`; `PendingEffect` parks and resumes the
  same machine through `NativeOutcome::Suspend`. `src/run/session.rs` owns the
  interpreter (no self-reference). Host ABI 2 (`aura_session_*`, ABI `2`,
  status `3` = pending effect, per-run `effect_id`) is live; `runtime.mjs`
  feature-detects it and never sends it to a frozen ABI-1 artifact.
- **Browser HTTP works end to end.** `http_get`/`http_request` (with
  `headers`/`body`/`timeout_ms`) park on a Host effect; the Worker performs
  `fetch` after the page's per-origin consent. Denial → `E5002`; CORS
  rejection → `E4020`; `credentials:"omit"`, `redirect:"manual"`,
  `referrerPolicy:"no-referrer"`; no proxy.
- **Runtime identity:** `0.3.2-dev.5` (ABI 2) at the time of writing; later
  superseded by `0.3.2-dev.7` (ABI 2), which is the current preview. Neither
  is the public default (`manifest.current = 0.3.1`).
  `0.3.1` and all frozen artifacts are byte-identical; `v0.3.1` unmoved.
- **Evidence:** Rust `session` 14/0, all-features matrix 63/63; Node
  `session-http` 13/0; Chromium `browser-http` 8/0; live PokéAPI →
  `200\npikachu\n`; full website + Playground suites green.
- **Exact next action:** a human reviews the deferred binary-value-model (B)
  decision and the promotion/publishing of the development runtime. Nothing is
  pushed, tagged, released, or deployed.

## HISTORICAL — AURA WEBSITE AUREA MIGRATION (2026-10-09)

A full website redesign onto **Aurea** (Aura's own design system) plus a
reconciliation of all active website documentation against the published Aura
0.3.1 contract. Scope: `website/**`, the relevant `docs/**` content mirrors, and
website tooling/tests. `src/**`, `playground/runtimes/**`, `Cargo.*`, tags, and
frozen artifacts were **not** modified (verified byte-identical).

- Aurea is the **sole** design system. `website/assets/aurea.css` holds the
  canonical `--au-*` tokens + dark-default/light-inversion themes + primitives;
  `website/assets/styles.css` is Aurea-owned layout/component/docs styling with
  no tokens of its own. The Material 3 `website/assets/tokens.css` was deleted
  and removed from `lib/layout.mjs`; no `--md-*` reference remains. The
  standalone Playground stylesheet was rethemed onto the same `--au-*` tokens.
- Documentation reconciled to 0.3.1 (collections, `never`, pattern catch,
  semicolon rule, nested-array-call limitation, canonical typed-JSON, HTTP
  capability); the standard-library inventory is centralized in
  `website/content/stdlib.mjs` and consumed by the landing + reference pages.
- New guards: `tests/aurea.test.mjs` (extended), `tests/aurea-browser.test.mjs`,
  `tests/stdlib-consistency.test.mjs`, `tests/docs-consistency.test.mjs`.
- Follow-up (same session): fixed the reported 768px three-row navigation with a
  tiered, measured-fit app bar (`tests/nav-fit.test.mjs`), and unified the
  integrated + standalone Playground on one shared markup
  (`playground/web/workbench.mjs`) and stylesheet
  (`playground/web/workbench.css`), locked by
  `tests/playground-parity.test.mjs`. Both suites and the full website +
  Playground suites are green locally; nothing is pushed.
- Follow-up: reorganized the documentation manifest into six meaning-based
  groups and added the AIS (`/docs/ais/`) and MCP (`/docs/mcp/`) pages under a
  Semantic Tooling group, grounded in `src/ais.rs`/`src/mcp.rs` with examples
  generated from the released compiler. New guards: `tests/ais-doc.test.mjs`
  (real-binary fixtures + golden JSON) and extended
  `tests/docs-consistency.test.mjs`. Surfaced in CLI/Tools/Architecture. Full
  website suite green (46 pages); nothing pushed. Website publication is the
  next human-authorized gate.

**Exact next action:** the human reviews the committed website work and
authorizes (or declines) a push. A push to this branch triggers `pages.yml` and
deploys the site — confirm acceptance before pushing. **Do not push without
explicit authorization**, and **do not start Aura 0.3.2**. Git reality at this
checkpoint: local = remote = `b909066f`, `v0.3.1` = `b09cbef2`.

## AURA 0.3.1 KEYSTONE — PUBLISHED (2026-10-08)

The 0.3.1 campaign is **closed**. Aura 0.3.1 (codename Keystone) is published:

- Source tag `v0.3.1` → `b09cbef2` (pushed `rewrite/v3-rust`; local = remote).
- GitHub Release: https://github.com/JoaoValentimTheo/aura-lang/releases/tag/v0.3.1
  with platform tarballs (Linux/Windows/macOS), the immutable
  `aura-playground-runtime-0.3.1.wasm` (+ checksum), `release-manifest.json`,
  and `sbom.json`.
- Exact-SHA CI (run 37864894819): **20/20 jobs green**, including the previously
  red `playground (wasm runtime)`.
- Pages deploy (run 37864894705): success; live site
  https://joaovalentimtheo.github.io/aura-lang/ serves `Aura 0.3.1 Keystone`.
- New runtime: `0.3.1`, language `0.3.1`, codename Keystone, ABI 1, sha256
  `96e778ae50bd261f76d49adf04b8650cc3b60678a14c0e0ac555fb54400256ab`,
  1,951,520 bytes. Frozen `0.0.2`/`0.2.0`/`0.2.1` byte-for-byte unchanged;
  tags `v0.0.2`/`v0.2.0`/`v0.2.1` unmoved.
- `0.3.1` is a language release (release = language), per ADR-0001, because
  Keystone changes observable behavior (distinct collection identities,
  contextual Arrays, canonical type-position `json_decode_as`, no general
  semicolon sequencing).

**Exact next action:** none for 0.3.1. The campaign is frozen. The next line,
`0.3.2`, is planned but **not started**; do not begin it (or HTTP expansion, or
local macOS CI migration) without a separate, explicit human instruction.
Publication of any 0.3.2 artifact requires a fresh authorization.

## Validation infrastructure (2026-10-08, local, unpushed)

The pathological macOS validation wall time was **root-caused and fixed**, not
merely made observable.

- **Root cause.** The first execution of a *freshly linked* Rust test binary
  costs 30-65 s wall / ~0.00 s user CPU. It is macOS's security assessment of
  the ad-hoc linker-signed Mach-O image. Reproduced with unique binaries:
  a fresh binary in `target/debug/deps` (758,385 entries) first-executes in
  35-62 s; the *same* binary copied to a small scratch dir first-executes in
  0.4-0.9 s; the second exec anywhere is 0.01 s. Directory size alone is not
  the cause (750k empty files = 4 s); the freshly lined image in the huge deps
  directory is.
- **Fix.** `scripts/validate.py` now discovers the full test surface from
  `cargo metadata` plus the real artifact stream (lib unit tests, all
  integration tests, the bin harness, doctests; the previously skipped lib
  target is now included), compiles each configuration once with live 5 s
  progress and a separate compile timeout, **stages each freshly linked test
  binary into `target/validate-run/` and executes it there**, runs each target
  as its own process group with a per-target deadline (SIGTERM then bounded
  SIGKILL), persists results atomically and incrementally (resumable even
  under `--fresh`), and keys the cache on the artifact digest + runtime-input
  content digest + toolchain + target + feature config. Regression tests:
  `python3 scripts/test_validate.py` (18 tests, ~4 s).
- **Measured result.** All three configurations now run end-to-end in **~369 s
  total** (all-features 97 s, canonical 97 s, bare 44 s; fresh builds):
  `PASS=185 FAIL=0 TIMEOUT=0 SKIP=1 COMPILE_FAILURE=0`. The single SKIP is the
  `aura` bin target under `bare` (`required-features = ["cli"]`), reported as
  SKIP, not FAIL. Previously the matrix exceeded a 600 s shell timeout with no
  output.
- **Real defect surfaced.** `tests/module_graph.rs`'s
  `typed_decode_type_argument_resolves_in_a_child_module` called
  `json_decode_as` with no `#[cfg(feature = "json")]` gate, so it failed under
  `--no-default-features`. Gated; bare `module_graph` is 37/0 and
  all-features 38/0.
- **Historical evidence preserved.** The prior all-features 59/59 PASS cache is
  archived at `target/validate-cache/results.pre-repair.json.bak`; it is not
  presented as a fresh run.

## Program

B-1 — WebAssembly call-frame implementation nonconformance. The released
runtime traps on the JavaScript engine stack below the normative 512-frame
language limit instead of reporting `E4011`. The chosen remediation is an
explicit-continuation (iterative) evaluator over the existing AST
(`docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`, Option E1).

## Current status

- B-1: **REMOTELY CLOSED** at `bb736fc` (inside pushed history; remote tip
  `089dffe` records the closure). The evaluator migration and production
  cutover are pushed; exact-SHA CI green (all 20 jobs) and Pages deployed.
  Released `0.2.1` runtime still contains the defect; the freshly built
  machine-backed runtime holds the language boundary.
- POST-B1 RUNTIME/WASM EDGE CLOSURE: **REMOTELY CLOSED** at `e576238`
  (pushed 2026-10-05; exact-SHA CI 20/20 green and Pages deployed after a
  GitHub Actions runner incident delayed the first attempts). The stdout
  capture-bound contract is pinned in `playground/runtime/tests/execute.rs`,
  `tests/host.rs`, and `playground/tests/node/b1_boundary.test.mjs`, and
  documented in `docs/playground.md` §2.
- AURA 0.3 KEYSTONE SUPER-TRANSACTION: **CLOSURE RANGE PUSHED THROUGH
  `b221e6a9`**; the remote is no longer at `e576238`. Manifest
  `docs/engineering/KEYSTONE_SCOPE_MANIFEST.md`, ledger
  `docs/engineering/KEYSTONE_JEV_LEDGER.md`, RFC
  `docs/rfcs/0001-exception-catch-selection.md`. Landed: exceptions E1-E6
  (pattern catch, raise-site spans, `Aura` namespace reservation), types
  (`never`, precise `none`, optional narrowing), unused analysis `E2008`,
  same-revision Native/WASM differential, HTTP capability, typed JSON
  decoding, structured diagnostics + CLI color, Aurea + overflow invariants,
  codename metadata, tooling reconciliation.
- **REMOTE CI IS RED AT `b221e6a9`** (run `37639795923`, job
  `playground (wasm runtime)`): `b1_boundary.test.mjs` reports 44/19 with
  grouping/f-string `guest trap: Maximum call stack size exceeded` /
  `memory access out of bounds`. The cause is a parser that bounds grouping
  and f-string interpolation recursion. The fix is committed locally as the
  two commits **ahead** of the remote (`04783231`, `e7505408`) and is
  **unpushed**. On the local tip the fresh-wasm boundary suite is **63/0**,
  the full playground suite is green (differential 233/0, browser 98/0,
  worker 12/0), the native matrix is green (all-features 57 suites + 166 lib;
  bare and canonical 57+57; lib 124/166), clippy (both configs) / fmt / MSRV
  1.83 / nightly fuzz check all exit 0, and the website suite is green
  (browser 380/0, a11y 70/0, links 2499 OK, release-version 29/0). Frozen
  `0.0.2`/`0.2.0`/`0.2.1` byte-identical; `v0.2.1` = `3f5f8702…` unmoved;
  `.kilo/**` protected churn untouched; no tag/release/deploy.
- KEYSTONE CLOSURE CAMPAIGN (2026-10-07, pushed through `b221e6a9`): committed
  `2f99a18` (third/fourth/fifth-review checker hardening), `c510215`
  (Value-algebra anti-collapse + type families + AIS delivery + MCP),
  `5abcd0d` (PokéAPI dogfood), `31c77be` (Playground FSM), `6e3ff89` (AIS
  delta fix + properties), `1932047` (Jev discovery + differential harness),
  `b4aabb1` (spec version drift), `ffc701f` (website drift), `a5f182c` +
  `4a36681` (regression seeds, fuzz lock), `6893457` (AIS narrowing proofs).
  Real defects fixed with regression evidence: `json_encode` silently
  collapsing unrepresentable kinds to `null`/payload; the Playground status
  starting as the non-vocabulary `idle`; AIS delta reporting an unrelated
  insertion as a semantic change to every following symbol; the spec header
  claiming language version 0.0.1; the website claiming no development line
  exists. Campaign premise about `;` **falsified with evidence**: the
  canonical grammar, spec §3.7, CONF-PARSE-8 and `tests/syntax_conformance.rs`
  all define `;` as a statement terminator, and `[1; 3]` is `E1006` — no
  grammar change was made.
- Remaining closure work: none required for the publication gate beyond the
  human gates below. Final validation on this exact source: all-features
  1333/0 (176 suites), no-default canonical 1252/0, bare --no-default-features
  1054/0, clippy -D warnings / fmt / MSRV 1.83 / nightly fuzz check all exit 0,
  playground suite green (differential 233/0, browser 98/0, boundary 63/0,
  worker 12/0, multi-file 42/0, cache 7/0), website suite green (browser
  380/0, a11y 70/0, links 2499 OK across 43 pages, release-version 29/0).
  Frozen `0.0.2`/`0.2.0`/`0.2.1` byte-identical; `v0.2.1` = `3f5f8702…`
  unmoved; `.kilo/**` protected churn untouched; no tag/release/deploy.
  **Exact next action:** the branch is at the publication gate. A push must
  be explicitly authorized by the human; if authorized, `git push` normally
  (never force), then verify the exact pushed SHA in CI. Note that CI's
  `pages.yml` triggers on push to these paths and will deploy the website —
  that is the repository's existing behavior, so a push publishes the site;
  confirm the human accepts that consequence before the push.
  Five adversarial review passes total (J1–J12 in the ledger); each finding
  was independently reproduced against the built CLI before being fixed:
  second (committed `45271b2`), then the third/fourth/fifth passes hardening
  `never` reachability, shadowed-callee divergence, transitive closure
  capture, literal conditions, `try`-`finally`, statically-empty iterables,
  eager value-position divergence, and `-> never` method divergence.
  `tests/keystone_types.rs` is 48/48. The full validation floor is green on
  this state (all-features 1247/0, canonical 1172/0, bare 1037/0,
  clippy/fmt/MSRV/fuzz-check clean, playground + website suites green).
  Remaining human gates: shadowing policy, package-manager model, runtime
  publication/codenames.
- PRE-0.3 FOUNDATION SUPER-TRANSACTION: **PUSHED THROUGH `b221e6a9`**.
  Iteration ledger and
  artifacts: `docs/engineering/RUNTIME_ARCHITECTURE.md`,
  `EXCEPTION_ARCHITECTURE.md`, `CAPABILITY_MODEL.md`,
  `EMBEDDED_PYTHON_ARCHITECTURE.md`, `CRITICAL_SYSTEM_PROFILE.md`,
  `MODULE_ARCHITECTURE.md`, `FUTURE_EXTENSION_BOUNDARIES.md`,
  `DIAGNOSTIC_TAXONOMY.md`, plus `tests/stdlib_coverage.rs` and the extended
  transport-limit, poison-recovery, and benchmark coverage.
- B-1R1 (design): COMPLETE.
- B-1R2 (differential oracle): COMPLETE and mutation-validated.
- B-1R3A-ARCH-1 (Arc AST sharing): RESOLVED.
- B-1R3A (machine skeleton + first executable subset): **COMPLETE AND REMOTELY
  CLOSED** at `c9ade0b` (exact-SHA CI green; website deployed).
- B-1R3B.1 (unary `-`/`not`/`~`): **COMPLETE AND PUSHED** at `1a12b84`.
- B-1R3B.2 (eager binary operators): **COMPLETE AND PUSHED** at `1a12b84`.
- B-1R3B.3 (short-circuit `and`/`or`): **COMPLETE AND PUSHED** at `2f34b9c` via
  the intermediate commits `58a1b16`/`31e313a`/`2f34b9c`.
- B-1R3B.4.1 (list/tuple construction): **COMPLETE AND PUSHED** at `efc66bd`
  (remote-closed).
- B-1R3B.4.2 (map construction): **COMPLETE AND REMOTELY CLOSED** at `5e70677`.
- B-1R3B.5 (range construction): **COMPLETE AND REMOTELY CLOSED** at `cf17689`.
- B-1R3B.6 (index / field reads): **COMPLETE AND REMOTELY CLOSED** at `cf17689`.
- B-1R3B.7 (f-strings): **COMPLETE AND REMOTELY CLOSED** at `52124a0` (pushed
  after a passing adversarial push gate; exact-SHA CI and Pages green; local =
  tracking = server at `52124a0`, ahead/behind 0/0).
- B-1R3B.8 (milestone completion audit + dependency graph): **COMPLETE AND
  REMOTELY CLOSED** at `9cb5e28` — `docs/engineering/B1R3B8_COMPLETION_AUDIT.md`.
- B-1R3C–B-1R3F (Call, Method, Construct, Lambda, Pipe, Assign, LetPattern,
  While, Loop, For, ListComp, MapComp, Match, Try): **COMPLETE AND PUSHED** in
  the B-1 closure range (tip `bb736fc`).
- Production cutover `62dd592`: **COMPLETE AND PUSHED** in the closure range.
  Every production entry point runs the machine; the residual recursive seams
  were closed at `11abad2` (free `aura::execute_with`, REPL `Item::Const`,
  Playground `execute`/`run_module_capture`).
- B-1R4 (full differential): DONE (whole-corpus `engines_agree`; differential
  228/228; syntax conformance 43/43).
- B-1R5 (substrate boundary): PARTIALLY DONE — fresh machine-backed wasm pinned
  by `playground/tests/node/b1_boundary.test.mjs` (63 checks: the frame/pattern
  boundary matrix, the pattern-depth calibration, and the post-B1 stdout
  capture-bound checks) and native CLI/REPL canaries. The Chromium main-thread
  and production-Worker boundary matrix needs the fresh artifact published to
  close (human-gated; frozen `playground/runtimes/**`).
- B-1R6 (red team): DONE — independent read-only review of the full range; all
  fifteen claims confirmed with no falsification; six minor findings, A/B/C/D/F
  fixed in `39caf6f` (field-receiver and tuple order differentials,
  `#[doc(hidden)]` on `Interp::run`, callback-confinement tripwire, wasm
  pattern-depth calibration pin); E did not reproduce.
- B-1R7 (full validation gate): DONE — including the exact-SHA CI run on
  `bb736fc` (all 20 jobs green) after the additive miri fix.
- B-1R8 (recursive-engine removal + oracle switch removal): NOT STARTED —
  human-gated; requires a release decision.

See `AGENT_STATE.md` for the exact SHAs and ahead/behind.

## Production vs experimental engine

- Production/default/CLI/REPL/Playground/library: **explicit-continuation
  machine** since the local cutover `62dd592`. The recursive evaluator is
  retained only as the differential reference (`Compilation::execute_recursive*`)
  and the rollback path.
- The machine handles **every** `Expr` (21) and `Stmt` (12) variant with no
  recursive fallback and no unsupported sentinel: literals, name lookup,
  expression statements, blocks, `let` shadowing and `let` patterns, assignment
  (simple/compound, name/index/field targets), `if`/`while`/`loop`/`for` (lazy
  ranges), `break`/`continue` boundaries, `return`/`throw`, unary/binary/
  short-circuit operators, list/tuple/map/range construction, index/field
  reads, f-strings, user/native/closure/method calls (including the resumable
  `map`/`filter`/`reduce` callback protocol), struct/enum construction,
  lambdas, pipes, list/map comprehensions, `match`, and `try`/`catch`/`finally`.
  The whole corpus is required to agree between engines
  (`tests/evaluator_oracle.rs::engines_agree`).

## Completed in B-1R3B.7 (remote-closed at `52124a0`)

- `src/run/iterative.rs` — added `Cont::FStrNext { parts, index, spec, out, env }`,
  `Machine::start_fstring`/`advance_fstring`, and the `Expr::FStr` arm in
  `start_expr` plus the `Cont::FStrNext` `resume` arm. Parts are consumed left
  to right, exactly once each, mirroring `Interp::eval_inner`'s `Expr::FStr`
  arm: each `FPart::Lit` is appended verbatim (raw text; `{{`/`}}` already
  resolved by the parser), each `FPart::Expr` is evaluated exactly once and
  appended with `v.display()` (no spec) or `Interp::format_value` (with a
  spec). The accumulator `String` lives in the continuation, so nesting does
  not grow the Rust stack. A control signal or diagnostic from an interpolation
  aborts the whole f-string and propagates unchanged, so later parts never run
  and no partial string is observable. No recursive AST evaluation and no
  fallback.
- Oracle: new `tests/oracle/r3b7_golden.tsv` (LF-pinned; 112 supported cases)
  and differential tests `r3b7_fstring_supported_subset_agrees`,
  `iterative_fstring_unsupported_fails_explicitly`, `r3b7_iterative_golden_matches`,
  plus `r3b7_golden_has_lf_pin` and 11 crate-internal f-string unit tests.
  Boundary movements: `fstring` removed from the R3A unsupported set
  (`r3a_golden.tsv` −1 row); `and_skip_fstring`/`or_skip_fstring` moved from the
  R3B.3 supported set into the R3B.7 set (`r3b3_golden.tsv` −2 rows);
  `nested_unsupported_element` (R3B.4.1), `fstring_value` (R3B.4.2), and
  `fstring_index` (R3B.6) moved out of their unsupported sets into supported
  R3B.7 cases. The main `golden.tsv` is byte-unchanged.
- Verified: empty/text-only/escaped-brace shapes; raw (undecoded) literal text;
  the stringification matrix (none/bool/int/float/string/list/tuple-sugar/map/
  range/`<fn>`); the format mini-language (`d b o x X f F e E %`, sign, width,
  fill/align, zero-pad, precision) and its `E3001` type and `E4013`
  precision/width bounds; Unicode; one-level nested f-strings; interpolation
  source order with first-error-wins; control-signal abort; `and`/`or` skipping
  of a whole f-string; composition with List/Map/Range/Index/Field/block/let/
  if; program-mode frame boundary; `E4999` no-fallback unsupported surface;
  host-stack/AST-depth safety and `expr_depth` restoration. A deliberate
  `display`→`debug_repr` stringification mutation was detected by seven R3B.7
  cases and reverted byte-exactly.
- Honest gaps: exactly-once is structural, not differentially falsifiable in the
  current subset (the only side-effecting interpolation constructs — calls,
  assignment, `print` — are still unsupported). Two-level f-string nesting is
  not expressible in the grammar (the innermost level would need the outer
  delimiter, which terminates it). Struct-instance/variant stringification is
  unreachable (`Expr::Construct` unsupported).

## Completed in B-1R3B.6 (remote-closed at `cf17689`)

- `src/run/iterative.rs` — added `Cont::IndexTarget { idx, env, span }`,
  `Cont::IndexApply { base, span }`, and `Cont::FieldReceiver { name, span }`,
  the `Expr::Index` / `Expr::Field` arms in `start_expr`, and the three `resume`
  arms. Index evaluates the target first and exactly once, then the index
  exactly once, then applies the lookup through `Interp::index_get`; a control
  signal from either operand is redelivered and the lookup never runs. Field
  evaluates the receiver once and resolves it exactly like `eval_inner`'s
  `Expr::Field` arm (struct field read with the method-not-a-value `E2003`
  guard, or zero-argument builtin method dispatch). Because struct construction
  is `Expr::Construct` (still unsupported), `Value::Instance` field reads are
  unreachable in this subset; the reachable field surface is the builtin method
  registry and its `E2003` unknown-member diagnostic. No recursive AST
  evaluation and no fallback.
- Oracle: new `tests/oracle/r3b6_golden.tsv` (LF-pinned; 55 index + 23 field =
  78 supported cases) and differential tests
  `r3b6_index_supported_subset_agrees`, `r3b6_field_supported_subset_agrees`,
  `iterative_index_field_unsupported_fails_explicitly`,
  `r3b6_iterative_golden_matches`, plus `r3b6_golden_has_lf_pin`. Boundary
  movement: `index` removed from the R3A unsupported set (now supported;
  `r3a_golden.tsv` −1 row). `field_access` stays in R3A unsupported: it is a
  struct literal via `Expr::Construct`. The main `golden.tsv` is byte-unchanged.
- Verified: target-before-index order and exactly-once; list/tuple/string/map
  lookup; negative-index normalization; `E4019` out-of-range; `E2003` missing
  map key and unknown member; `E3001` non-key-capable map key, unsupported
  base/index combination, and check-time map key-kind mismatch; receiver error
  wins over member resolution; control-signal propagation from target, index,
  and receiver; nested Index/Field and Range composition; program-mode frame
  boundary (Index and Field); unsupported→`E4999` with no fallback; and
  host-stack/AST-depth safety. A deliberate base/index-swap mutation was
  detected by three R3B.6 tests and reverted byte-exactly.
- Historical note (now resolved): struct-instance field reads and
  struct-instance indexing could not be differentially exercised until
  `Expr::Construct` landed in B-1R3C.3, and exactly-once was structural until
  calls/assignment/`print` landed in B-1R3C.1/B-1R3D.1; both gaps are closed by
  the R3C–R3F migration and the `print`-observable order cases in
  `tests/oracle/r3c1.rs`.

## Completed in B-1R3B.5 (remote-closed at `cf17689`)

- `src/run/iterative.rs` — added `Cont::RangeStart { end, env, span }` and
  `Cont::RangeEnd { start, span }`, the `Expr::Range` arm in `start_expr`, and
  the two `resume` arms. The start operand is evaluated first and exactly once;
  its completion retains the value (unvalidated) and schedules the end exactly
  once. **Both** operands complete before either bound is validated, and
  validation is start-first, exactly like `Interp::eval_inner`'s `Expr::Range`
  arm (`src/run/mod.rs:1583`): a non-int bound is `E3001` with the recursive
  message (`range start/end expects an int, found {type}`) at the range
  expression's span, and a start error beats a runtime end error, while an end
  signal/error preempts a runtime-invalid start (the end is evaluated first).
  A valid pair becomes the same `Value::Range(RangeVal { start, end })` that
  `range(a, b)` builds (half-open, end-exclusive). A control signal from either
  operand aborts construction and propagates unchanged; no partial Range
  escapes. No recursive AST evaluation and no fallback.
- Oracle: new `tests/oracle/r3b5_golden.tsv` (LF-pinned; 55 supported cases)
  and differential tests `r3b5_range_supported_subset_agrees`,
  `iterative_range_unsupported_fails_explicitly`, `r3b5_iterative_golden_matches`,
  plus `r3b5_golden_has_lf_pin`. Boundary movements: `range_literal` removed
  from the R3A unsupported set (`r3a_golden.tsv` −1 row); `and_required_range`
  removed from the R3B.3 unsupported set and replaced by six supported range
  short-circuit cases (`r3b3_golden.tsv` +5 rows); `range_element` moved from
  the R3B.4.1 unsupported set into the supported set plus a new program-mode
  `main_list_range` (`r3b4_golden.tsv` +2 rows); `range_value` moved from the
  R3B.4.2 unsupported set into the supported set plus a new program-mode
  `main_map_range` (`r3b42_golden.tsv` +2 rows). The main `golden.tsv` is
  byte-unchanged.
- Verified: start-before-end order and exactly-once scheduling; both-bounds
  evaluated then start-first validation; `E3001` runtime/check-time split;
  `E3001` at the exact range span; control propagation from either bound;
  `i64` extremes (`len()` saturates), descending/equal/empty, negative bounds;
  list/map/if/let/block/program composition; no fallback on unsupported bounds
  (`E4999`); AST-depth `E1015` below/above the limit; deep nesting host-stack
  safe. A deliberate start/end endpoint-swap mutation was detected by the R3B.5
  differential + golden tests and the composite R3B.4/R3B.4.2 tests, then
  reverted byte-exactly.
- Honest gap: exactly-once is structural but not differentially falsifiable in
  the current subset, because the only side-effecting bound constructs
  (calls/assignment/print) are still unsupported (recorded in
  `src/run/iterative.rs`).

## Completed in B-1R3B.4.2 (remote-closed at `5e70677`)

- `src/run/iterative.rs` — added `Cont::MapKeyNext { entries, index, out, env }`
  and `Cont::MapValueNext { key, entries, index, out, env }`, `Machine::start_map`,
  and the `Expr::Map` arm in `start_expr`. Entries run in source order; for each
  entry the key is evaluated and validated with `MapKey::from_value` **before**
  its value is scheduled, exactly like `Interp::eval_inner`'s `Expr::Map` arm
  (`src/run/mod.rs:1464`). An invalid runtime key is `E3001` with the exact
  recursive message at the key expression's span; the value never runs. A
  control signal or diagnostic from a key or value aborts the map and propagates
  unchanged, so later entries never run and no partial map is observable.
  Duplicate keys keep the last value (`BTreeMap::insert`), matching recursion.
  Empty `{:}` completes immediately. The accumulated `BTreeMap` lives in the
  continuations; no `Map`-sized or nested Rust recursion. No recursive AST
  evaluation and no fallback.
- Oracle: new `tests/oracle/r3b42_golden.tsv` (LF-pinned; 57 supported cases:
  49 value-mode and 8 program-mode; 57 golden rows) and differential tests
  `r3b42_map_supported_subset_agrees`,
  `iterative_map_unsupported_fails_explicitly`,
  `r3b42_iterative_golden_matches`. Boundary movements: `map_literal` removed
  from the R3A unsupported set (now supported); `or_required_map`/`and_lhs_map`/
  `or_lhs_map` moved from the R3B.3 unsupported set into the supported set;
  `map_element` moved from the R3B.4.1 unsupported set into the supported set
  (new rows in `r3b3_golden.tsv`/`r3b4_golden.tsv`; `r3a_golden.tsv` −1 row).
  The main `golden.tsv` is byte-unchanged.
- Push-blocker remediation: the R3B.4.1 `main_map_element` program-mode case
  (list containing a Map through a real frame boundary) was dropped without
  replacement when map elements moved to supported coverage; it is restored in
  the R3B.4.2 supported set (`r3b42-program/main_map_element`) with a guard test,
  and `tests/oracle/r3b42_golden.tsv` is now pinned `text eol=lf` in
  `.gitattributes` like every sibling golden.
- Verified: key-before-value and entry-by-entry order; exactly-once key/value;
  runtime invalid-key `E3001` at the key span with value suppression;
  first-error-wins; key/value control-signal abort with later-entry and
  later-unsupported skipping; last-wins duplicate keys; nested maps,
  Map-in-List, List-in-Map; `{:}`; unsupported keys/values → `E4999` with no
  fallback; AST-depth `E1015` below/above limit; deep nesting through a real
  frame boundary and mixed Map/List nesting host-stack safe; and a deliberate
  key/value-order mutation detected by the map oracle tests and reverted.

## Completed in B-1R3B.4.1 (pushed at `efc66bd`)

- `src/run/iterative.rs` — added `Cont::ListNext { items, index, out, env }`,
  `Machine::start_list`, the `Expr::List`/`Expr::Tuple` arm in `start_expr`, and
  the `Cont::ListNext` arm in `resume`. Elements are evaluated left to right,
  exactly once each, with the same environment; a non-`Val` completion from an
  element aborts the list and is redelivered unchanged (later elements are
  never scheduled); completed elements become `Value::list`. `Expr::Tuple`
  shares the list path because the recursive arm is byte-identical and
  `LANGUAGE_SPEC.md` §21 froze `(a, b)` as list sugar **at that time**. No
  recursive AST evaluation and no fallback.
  > **SUPERSEDED (2026-10-08):** the Keystone collection decision (ADR-0005)
  > makes `Expr::Tuple` a distinct `Value::tuple`; `start_expr` now dispatches
  > `Expr::List`/`Expr::Tuple`/`Expr::Set` to `start_seq`/`start_set`, and an
  > `[a, b]` literal under an `[T; N]` expectation realizes a `Value::array`.
- Oracle: new `tests/oracle/r3b4_golden.tsv` (LF-pinned; 37 supported cases)
  and differential tests `r3b4_list_supported_subset_agrees`,
  `iterative_list_unsupported_fails_explicitly`,
  `r3b4_iterative_golden_matches`. `list_literal`/`tuple_literal` moved out of
  the R3A unsupported set (and `r3a_golden.tsv` regenerated accordingly);
  `and`/`or` list-operand cases moved from the R3B.3 unsupported set into the
  supported set (6 new rows in `r3b3_golden.tsv`). The main `golden.tsv` is
  byte-unchanged.
- Verified: order/exactly-once (first error wins; later-error cases), element
  control-signal abort with unsupported-later-element skipping, exact
  diagnostics/spans, nested lists, AST-depth `E1015` below/above the limit,
  deep nesting through a real frame boundary host-stack safe, and a
  deliberate element-order mutation detected by four oracle tests and reverted
  byte-for-byte.

## Completed in B-1R3B.3 (pushed at `2f34b9c`)

- `src/run/iterative.rs` — added `Cont::ShortCircuitLeft { op, rhs, env }` and
  `Cont::ShortCircuitRight`; the machine evaluates the left operand exactly
  once, and only on completion decides from `Value::truthy()` whether the right
  operand is required (`and`: truthy left; `or`: falsy left). A skipped right
  operand is never scheduled (no stdout, mutation, diagnostic, or signal); a
  required right is evaluated exactly once. The result always mirrors the
  recursive arms: `Bool(false)` / `Bool(true)` on the deciding left, otherwise
  `Bool(rhs.truthy())`; control signals propagate via the `val!`-equivalent
  redelivery path. No recursive AST evaluation and no fallback.
- Oracle: new `tests/oracle/r3b3_golden.tsv` (LF-pinned) and differential tests
  `r3b3_short_circuit_supported_subset_agrees`,
  `iterative_short_circuit_unsupported_fails_explicitly`, and
  `r3b3_iterative_golden_matches`. `and_bool`, `or_bool`, and `eager_over_and`
  moved out of the R3B.2 unsupported set (now supported); the call-operand case
  stays. The main `golden.tsv` and all earlier iteratives goldens are unchanged.
- Verified: skipped-RHS error suppression (E4007/E4013/E3001/E4999 must not
  occur), required-RHS exact diagnostics, control-signal propagation, deep
  chains host-stack safe, `and`/`or` decision mutations rejected by the oracle.

## Completed in B-1R3B.2 (pushed at `1a12b84`)

- `src/run/iterative.rs` — added `Cont::BinaryLeft { op, rhs, env, span }` and
  `Cont::BinRight { op, lv, span }`; the machine evaluates the left operand,
  retains it, evaluates the right exactly once, then applies the operator only
  after both complete. Operator semantics are delegated to `Interp::binary`
  (the recursive engine's value-level helper, which never evaluates an AST
  node), so arithmetic, overflow (`E4013`), divide-by-zero (`E4007`), bitwise,
  shift-range, comparison, and equality behavior are identical by construction.
  `and`/`or` (B-1R3B.3) remain unsupported and never fall back.
- Oracle: new `tests/oracle/r3b2_golden.tsv` (LF-pinned) and differential tests
  `r3b2_binary_supported_subset_agrees`, `iterative_binary_unsupported_fails_explicitly`,
  `r3b2_iterative_golden_matches`. The R3A unsupported case `binary_add` was
  removed (now supported) and `r3a_golden.tsv` regenerated accordingly; the main
  `golden.tsv` is unchanged.
- Deliberate operand-swap mutation was detected by both R3B.2 tests and reverted.

## Completed in B-1R3B.1 (local)

- `src/run/iterative.rs` — added `Cont::UnaryApply { op, span }`; the operand is
  evaluated once and the operator is applied at its completion. `apply_unary`
  mirrors `Interp::eval`'s `Expr::Unary` arm exactly (checked `int` negation with
  `E4013`, `not` over `truthy()`, `~` on `int` only, both type errors `E3001`
  with the same messages). Operand control signals (`return`/`throw`/…) propagate
  without applying the operator; a deep unary chain is stack-safe.
- Oracle: `tests/oracle/r3b.rs` + `tests/oracle/r3b_golden.tsv` (LF-pinned) and
  differential tests `r3b_unary_supported_subset_agrees`,
  `iterative_unary_unsupported_fails_explicitly`, `r3b_iterative_golden_matches`.

## Completed in B-1R3A (remotely closed at `c9ade0b`)

- `src/run/iterative.rs` — explicit `Machine`/`Ctrl`/`Cont`/`UserFrame`/
  `FrameBoundary`; one `loop` over `Ctrl` with a `Vec<Cont>` stack; 512/513
  frame accounting and the `E1015` AST-depth guard, unit-tested.
- Oracle extended with an identified R3A subset (`tests/oracle/r3a.rs`),
  strict supported-subset equality, an explicit no-fallback test, and a
  full-field iterative golden (`tests/oracle/r3a_golden.tsv`, LF-pinned).

## Current invariants — do not break

- AST sharing uses `std::sync::Arc` (not `Rc`); runtime `Env`/`Value`/`Closure`
  deliberately stay `Rc`. Do not mechanically convert runtime `Rc` to `Arc`.
- The main oracle golden `tests/oracle/golden.tsv` is byte-exact and must not
  change unless an intended observable semantic change is being made.
- Frozen release runtimes (`0.0.2`, `0.2.0`, `0.2.1`) are immutable, byte for
  byte.
- No `unsafe`; no manual `Send`/`Sync`.
- B-1 is REMOTELY CLOSED: the full migration and the 512-frame boundary hold
  on every substrate (fresh wasm proven; frozen `0.2.1` remains the historical
  artifact). Do not reopen B-1 without a new human gate.

## Do not touch

- `playground/runtimes/**` (frozen artifacts).
- The `v0.2.1` tag or any release.
- Untracked/pre-existing dirty state (see `AGENT_STATE.md` protected local
  state), including `.kilo/**`.
- The website, unless a current public statement is demonstrably false.

## Relevant authority

- Design: `docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`
- Oracle: `docs/engineering/B1R2_DIFFERENTIAL_ORACLE.md`
- Arc decision: `docs/B1R3A_AST_SHARING_DECISION.md`
- B-1 decision package: `docs/WASM_CALL_FRAME_LIMIT_DECISION.md`
- Normative semantics: `docs/LANGUAGE_SPEC.md`

## Required tests for B-1R work

- `cargo test --locked --features evaluator-oracle --test evaluator_oracle`
- `cargo test --locked --features evaluator-oracle --lib iterative`
- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`

At milestone closure, run the full validation floor in `AGENTS.md`.

## Phase ledger — B-1R3B (values and operators)

R3B is **COMPLETE**: every planned microphase (R3B.1—R3B.7) is implemented,
differentially oracled, and remotely closed; R3B.8 audited the milestone. The
B-1R3C–B-1R3F evaluator migration and the production cutover `62dd592` are
implemented, oracle-covered, validated, **pushed, and remote-closed** as part
of the B-1 closure range (tip `bb736fc`).

- R3B.1 unary operators — **COMPLETE AND PUSHED**
- R3B.2 binary operators — **COMPLETE AND PUSHED**
- R3B.3 short-circuit / evaluation order — **COMPLETE AND PUSHED**
- R3B.4.1 list / tuple construction — **COMPLETE AND PUSHED**
- R3B.4.2 map construction — **COMPLETE AND REMOTELY CLOSED**
- R3B.4.3 (reserved: map-key admissibility / nesting if the code shows a
  distinct boundary)
- R3B.5 range — **COMPLETE AND REMOTELY CLOSED**
- R3B.6 index / field reads — **COMPLETE AND REMOTELY CLOSED**
- R3B.7 f-strings — **COMPLETE AND REMOTELY CLOSED**
- R3B.8 milestone completion audit — **COMPLETE (audit only)**; see
  `docs/engineering/B1R3B8_COMPLETION_AUDIT.md`

Use multi-reviewer analysis mainly at milestone closure, not after each
microphase.

## PRE-0.3 iteration ledger (2026-10-05)

Compact record of the Pre-0.3 Foundation super-transaction. Every iteration
was run under the mandatory Jev PRE/POST loop; evidence and dispositions only
(no chain-of-thought).

| # | Workstream | Question / action | Evidence before | Jev PRE | Verification | Jev POST | Commit |
|---|---|---|---|---|---|---|---|
| I1 | A/J/K/H/P/O reconnaissance | Inventory repo order, module identity, stdlib, limits, unsafe/panic, supply chain | two read-only subagent inventories + lead verification (falsified the `[0]`-index concern: test-only) | proceed 0.99 | − (read-only) | accept 0.99 | — |
| I2 | P/Host | Poisoned locks silently drop bytes? | host.rs:568-578 / lib.rs:195-199 confirm silent drop | proceed 0.92 (hidden-corruption p=0.07) | host 17 tests incl. new poison regression; playground execute 19 | commit 0.99 (risk 0.15) | `5edab88` |
| I3 | H | Triplicated `MAX_AST_DEPTH`, magic range/sleep limits | parse:163, run:34, check:152; run:1223 local const; host:370 magic | proceed 0.97 | boundaries 34; oracle 48; fmt/clippy clean | commit 1.0 | `2fd1433` |
| I4 | O | Unused `serde`/`arbitrary` deps | `cargo tree -i serde` = self-only; no derives | proceed 0.92 | all feature configs + 51 suites + fuzz nightly + locks | commit 0.94 | `3186340` |
| I5 | A/25 | Stale "production recursive" claims; divergent errors reference | 7 docs wrong; website duplicate | proceed 0.91 (spec-rewording risk flagged 0.51, mitigated by wording) | syntax_docs 3 (new sync test), grammar 6, website build/tests | commit 0.98 | `c1ffb18` |
| I6 | G | No evaluator performance baseline | bench.rs had shape guards only | proceed 1.0 | 8 stages measured linear; guards 14 pass; fmt/clippy | commit 0.99 | `9151df1` |
| I7 | C | Exception family/custom-exception foundation | throw/catch reality; TryResult snapshots | proceed 0.99 (preemption risk 0.4 → design-only markers strengthened) | doc cross-checked | commit 0.65→ (low, revised wording) | `34c5878` |
| I8 | D/E/F | Capability model, embedded-Python boundary, critical profile | Host trait, pyo3 inventory, resource table | proceed 0.96 | all claims cross-checked to source | accept 0.95 | `8ac9b93` |
| I9 | K/J/L/M/N | Stdlib coverage gaps; module map; AI/Dart/quantum boundaries | inventory found untested surfaces; cycle-coverage suspicion falsified | proceed 0.92 | stdlib_coverage 11 (2 expectation fixes to match documented surface) | commit 0.94 | `a1a5aa4` |
| I10 | WASM | Remaining transport limits untested | args/project/key/name limits had no direct tests | commit scope approved | execute 22, virtual_project 24 (1 expectation corrected to E4020) | commit 0.99 | `eb1e339` |
| I11 | I | Diagnostic taxonomy; E4099/E5003 undocumented | 46 constants / 45 codes; doc gap | proceed 0.99 | sync test green; grammar 6 | (folded) | `d4becc3` |
| I12 | B | Runtime architecture record; state reconciliation | entry points verified with file:line | commit scope approved | production_routing exists; agent-state preflight OK | commit 0.96 | `5b74ebb` |
| I13 | Residuals/§21 | Dispose 5 residuals; deliberate divergence | fresh-wasm pattern + f-string sweeps: no traps | commit scope approved | 3 mutations detected and reverted byte-exact (sha256); wasm 63 checks | commit 0.93 | `0e34fed`, `ef920a8` |
| I14 | Validation/review | Full matrix + independent adversarial review | — | — | 52+52 suites, clippy×2, MSRV, fuzz, wasm 63, node, website, python, miri 117; review: claim not falsified, 7 minor doc corrections + 2 test-precision nits + 1 mirror-drift surface | accept 0.98 (corrections) | `1d38b49` |

Jev totals: PRE consultations: 13; POST classifications: 12; disagreements:
1 (I7: Jev 0.65 commit vs deterministic evidence that the doc was
design-only and non-normative — proceeded, strengthened the non-normative
markers first); Jev-driven extra investigations: 3 (I2 hidden-corruption
question, I5 spec-rewording caution, I7 preemption caution).

## PRE-0.3 adversarial re-audit + human syntax gate (2026-10-06)

Bootstrap (verified, not assumed): branch `rewrite/v3-rust`; local HEAD
`1102b23` (then `150befe` after the repair below); remote
`e576238f7adb52f6ab6d18431602e3fbc2bd3636`; ahead 15 / behind 0; range
`e576238..1102b23` intact; staged none; protected `.kilo/**` churn matches the
documented state; `.codex/**` not tracked; root `s` absent; frozen runtime
hashes byte-identical; tag `v0.2.1` = `3f5f8702`; declared version `0.2.1`.

| # | Workstream | Question / action | Evidence before | Jev PRE | Verification | Jev POST | Commit |
|---|---|---|---|---|---|---|---|
| A1 | Bootstrap | Is the checkpoint true in Git? | local/remote SHAs, reflog linear, frozen hashes, tags | proceed 0.85 (divergence 0.21, defect 0.26) | all 18 audited claims checked; 0 production defects | accept | — |
| A2 | F1 in-range defect | Merged doc-comment delimiter `tests/host.rs:383` | introduced by `2fd1433`; single occurrence | proceed 0.97 | fmt clean; host 19/19 | accept | `150befe` |
| A3 | F2 bare-config hygiene | 13 tests in 8 suites fail bare `--no-default-features` | every failing file unchanged since remote `e576238`; canonical configs 52/52 | future_work 0.99 | reproduced (exit 101); recorded as TD-20 | accept | (doc only, below) |
| A4 | Routing mutation | Does the tripwire catch a rerouted seam? | `src/lib.rs:443` temporarily → `interp.run` | reuse A1 gate | `production_routing` FAILED (2≠3); reverted, sha256 byte-identical | accept | (reverted) |
| A5 | Decision package | Verify candidate claims by execution | 21 probes on the built CLI | A/B `needs_more` 0.60, span 0.65 → revised claims | A and B already constructible; nominal cross-module identity verified; C needs new grammar; catch-selection/raise-site-span are the real gaps | accept (revised) | `docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md` |

Validation executed in this pass: all-features 52/52 suites; canonical
`--no-default-features --features cli,repl,json,regex,time` 52/52; evaluator
oracle 228/228 differential; fresh-WASM boundary 63/63; syntax conformance
43/43 zero imports; browser 66; worker 12; multi-file 42; cache 7; `fmt`
clean; clippy `--all-features` and bare both `-D warnings` clean; MSRV
`+1.83.0 check --all-features` green; nightly fuzz `check --all-targets`
green; Miri 117/117; four fuzz targets ~890k runs clean; website build + tests
green.

**Outcome A stands: no public exception syntax was chosen or implemented;
E1–E6 remain the human gate.** The verified, refined decision package is
`docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md`; it supersedes the
prior session's untracked `plans/EXCEPTION_SYNTAX_DECISION_PACKAGE.md`
(sha256 `647e0e24…`; its content is preserved as a strict subset). TD-20
records the pre-existing bare-config test-hygiene gap (not repaired: outside
the audited range and every affected configuration is unsupported/
documented-against).

Independent adversarial review (fresh read-only session, 2026-10-05):
completion claim **not falsified** across 13 attack areas (frozen artifacts,
protected state, commit coherence, uncommitted state, no-push, doc claims,
resource contracts, poison soundness, semantic safety, test quality, oracle
golden, dual authority, validation honesty). Findings: 7 minor
documentation-accuracy corrections, all applied in this iteration (TryResult
variant names, FrameBoundary location, entry-point citation, taxonomy
sampling wording, PERFORMANCE min-vs-average wording, stale "53" counts,
state-ledger pointer). One pre-existing non-blocking note retained: the
Playground `project.js` mirrors four transport limits without a mechanical
sync test (documented as non-authoritative; runtime refusal is the decision).



| # | Residual | Disposition |
|---|---|---|
| 1 | Real browser/Worker validation of a fresh machine-backed runtime | **Publication-gated** (unchanged). Frozen `playground/runtimes/**` is immutable; the fresh-wasm Node boundary is pinned at 63 checks; the Chromium/Worker matrix closes only when a new runtime is published (human-gated). |
| 2 | `bind_pattern`/`match_pattern` bounded host recursion | **Verified safe and pinned.** Fresh-wasm calibration: deep pattern @700 binds without trap; @766 is structured `E1015`. AST-bounded (`MAX_AST_DEPTH` + substrate parser budget). Recorded in `docs/engineering/RUNTIME_ARCHITECTURE.md` §3. |
| 3 | f-string sub-parser budget composition | **Investigated empirically; no trap found.** Additive frontier sweep (outer/inner grouping, two-level nested f-strings) on fresh wasm is ok-or-`E1015`; grammar bounds nesting depth. Now pinned by 10 fresh-wasm checks in `playground/tests/node/b1_boundary.test.mjs` so a future parser-frame change cannot reintroduce a trap silently. |
| 4 | Poisoned-mutex silent ignore in `BrowserHost` | **FIXED** in `5edab88`: all output/result locks recover via `PoisonError::into_inner`; regression test deliberately poisons the buffer and asserts bytes survive. |
| 5 | Recursive-engine removal | **Human-gated** (B-1R8). The recursive evaluator remains the differential reference (`execution_recursive*`, oracle `engines_agree`); no production entry reaches it (`tests/production_routing.rs`). |

## Exact next action

0. **Aura 0.3 Keystone final collection model + typed-JSON API (2026-10-08,
   LOCAL, UNPUSHED).** Local tip is **25 ahead / 0 behind** remote `b221e6a9`.
   The campaign commits: `d5bb2fbd` remove general semicolon statement/item
   sequencing (human override; `;` reserved for the Array type); `49ef5298`
   project dynamic ranges as `range_like`/`range`; `36c3d49f` archive historical
   docs; `d0ff3076` implement distinct `List`/`Array`/`Tuple`/`Set`/`Map`
   identities end-to-end (`[T; N]` array type, contextual bracket-literal
   realization, no implicit List↔Array conversion, real Tuple, Set with
   `SetLike`); `86eb6925` finalize the docs (ADR-0005, spec §21, grammar +
   website mirrors) and add the observable sharded validator
   `scripts/validate.py`; `ce8deb0a` implement the final
   `json_decode_as(text, Type)` direct-type API (new `Expr::TypeRef`, canonical
   type grammar, static result type, string form kept as a normalized
   compatibility spelling). **The collection model and typed-JSON API are now
   implemented**, superseding the earlier "no tuple / no array / no set" and
   string-type decisions. Frozen runtimes verified byte-identical; `v0.2.1`
   unmoved. **Nothing pushed.** **Exact next action:** the human decides whether
   to authorize a push of the 25-commit range (a push deploys the site —
   confirm) and reviews the implemented collection model and typed-JSON API for
   any change of decision. Do not push without explicit authorization.

1. B-1 and the post-B1 runtime/WASM edge closure are **remotely closed**. The
   Pre-0.3 Foundation super-transaction and the 2026-10-06 adversarial re-audit
   are **complete and pushed** through `b221e6a9` (the former local range
   `e576238..150befe` is now on the remote). The exception feature is
   **decided and implemented**: RFC 0001 (`docs/rfcs/0001-exception-catch-selection.md`)
   is **Accepted** — the record of the Keystone human authorization — and
   pattern-based catch selection, raise-site spans, and the reserved `Aura`
   namespace are implemented and regression-tested (`tests/catch_syntax.rs`);
   `KEYSTONE_SCOPE_MANIFEST.md` §G2 records the same disposition.
   `docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md` is retained as the
   **superseded historical** decision package (it records Outcome A, the state
   that existed *before* the authorization); it is not current authority.
2. The Keystone closure range is pushed through `b221e6a9`. Only the 2-commit
   parser wasm-safety repair (`04783231`, `e7505408`) is unpushed; pushing it
   requires explicit human authorization.
3. B-1R8 (remove the recursive engine and the oracle switch) and any runtime
   publication remain separately human-gated.
4. Keep frozen runtimes, `v0.2.1`, and `.kilo/**` untouched.
5. Publication-gated validation debt: the Chromium/Worker boundary against a
   machine-backed runtime awaits a published runtime (release-gated); the
   fresh-wasm boundary is already pinned in Node (63 checks).
6. Pre-0.3 decisions awaiting the human: exception syntax/catching (E1–E6 in
   `docs/engineering/EXCEPTION_ARCHITECTURE.md` §8) and embedded-CPython
   packaging (P1–P5 in `docs/engineering/EMBEDDED_PYTHON_ARCHITECTURE.md` §9).
   Bare-config test-feature gating is tracked as TD-20 (low, v1.1).

## Stop conditions

Stop and report instead of improvising if:

- Git reality contradicts `AGENT_STATE.md`;
- production behavior would change;
- the language contract and current behavior disagree in a way that needs a
  semantic decision (produce a decision package instead);
- a frozen artifact would be touched;
- a genuine oracle blind spot is found that cannot be closed within the slice.
