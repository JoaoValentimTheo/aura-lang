# Aura Agent State

Compact current engineering state. Rules: `AGENTS.md`. Active task and exact
next action: `docs/engineering/CURRENT_HANDOFF.md`. Detailed chronology:
`STATUS.md`. If Git and this file disagree, **Git wins** — reconcile this file
before continuing substantial work.

## Repository

- Branch: `rewrite/v3-rust`
- **Git reality (verify with `git rev-parse`, do not trust prose):** local HEAD
  and `origin/rewrite/v3-rust` are both **`b909066f`** (ahead/behind `0/0`);
  tag **`v0.3.1` = `b09cbef2`**. Aura 0.3.1 (Keystone) is the current release
  (release = language = runtime = `0.3.1`) and is **PUBLISHED**. The earlier
  `b221e6a9`/`v0.2.1` notes below are **historical**; Git is authoritative.
- Local/remote relationship: authoritative value is `git rev-list
  --left-right --count origin/rewrite/v3-rust...HEAD`; a tracked file cannot
  safely hardcode its own position.
- **Historical (superseded by `b909066f`):** the intermediate remote tip
  `b221e6a9` once carried a red `playground (wasm runtime)` CI job; that was
  repaired by the parser wasm-safety commits and the range subsequently
  advanced. Do not act on the old `b221e6a9` state.
- Historical release: **`v0.2.1`** (2026-10-01, immutable), tag `v0.2.1` =
  `3f5f8702`. Superseded by `v0.3.1`.

## Aura website — Aurea design-system migration (2026-10-09, LOCAL, UNPUSHED)

A full website redesign + documentation synchronization against the published
0.3.1 contract, entirely on branch `rewrite/v3-rust` at local HEAD `b909066f`
(= remote; nothing pushed). Scope was `website/**`, the relevant `docs/**`
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

## Aura 0.3.2 — Playground execution & browser HTTP campaign (2026-10-09)

**Gate 2 (output architecture) and Gate 3 (native HTTP foundations) are
complete locally; Gate 4 (browser HTTP) is blocked on a human ABI decision.**
See `docs/engineering/PLAYGROUND_032_CAMPAIGN.md` for the full inventory,
design, benchmarks and the decision package. Nothing pushed.

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

See `docs/engineering/CURRENT_HANDOFF.md`. In short: Aura 0.3.1 (Keystone) is
published; local = remote = `b909066f`, `v0.3.1` = `b09cbef2`. The **Aura
website Aurea migration + 0.3.1 documentation synchronization** is complete
locally and **unpushed** (see the section above). **Exact next action:** the
human reviews the locally committed website work and authorizes (or declines) a
push; a push to this branch triggers `pages.yml` and deploys the site, so the
human must accept that consequence before any push. Do not push without explicit
authorization, and do not begin Aura 0.3.2 (including HTTP expansion or local
macOS CI migration). B-1R8 (remove the recursive engine and the oracle switch)
and any runtime publication remain separately human-gated. **Validation
(TD-22):** use the observable sharded `scripts/validate.py` (not a monolithic
watch) for the Rust matrix.

## Writer

NONE (single-writer discipline; the 2026-10-07 recovery session held exclusive
worktree ownership — a stale external `opencode` writer had already exited on
its own).
