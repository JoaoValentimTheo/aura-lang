# Aura Agent State

Compact current engineering state. Rules: `AGENTS.md`. Active task and exact
next action: `docs/engineering/CURRENT_HANDOFF.md`. Detailed chronology:
`STATUS.md`. If Git and this file disagree, **Git wins** — reconcile this file
before continuing substantial work.

## Repository

- Branch: `rewrite/v3-rust`
- Remote `origin/rewrite/v3-rust`: `9cb5e28ce17cba652b48faa4ebbf682ec6029519`
  (B-1R3B.8 checkpoint; pushed and remote-closed; exact-SHA CI and Pages
  green). Local `HEAD` is the tip of the unpushed B-1 cutover range
  (`62dd592` production cutover, `11abad2` residual-seam closures, plus
  state-doc reconciliation); the local range is **not pushed**. Do not
  hardcode the ahead count: read it from `git rev-list --left-right --count
  origin/rewrite/v3-rust...HEAD`.
- Local/remote relationship: authoritative value is `git rev-list
  --left-right --count origin/rewrite/v3-rust...HEAD`; a tracked file cannot
  safely hardcode its own position.
- Current release: **`v0.2.1`**, published 2026-10-01, immutable. Tag
  `v0.2.1` = commit `3f5f8702`. Release = language = runtime = `0.2.1`;
  Host ABI 1; Playground API 1.
- No successor program or version is selected; there is **no `0.2.2`**.

## Frozen release runtimes (immutable, byte-for-byte)

| Version | Bytes | SHA-256 |
|---|---|---|
| `0.0.2` | 1,366,621 | `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed` |
| `0.2.0` | 1,654,161 | `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc` |
| `0.2.1` | 1,768,322 | `48c456fcda6c50dd6808ccc5f15a0bca4c0b81d7d970172557817decf427cc9e` |

Never overwrite a versioned artifact under `playground/runtimes/`; advancing a
runtime means adding a version, never replacing one.

## Current Track

B-1 — ENGINE-STACK-INDEPENDENT CALL ENGINE. B-1 is OPEN. The R3A baseline
through B-1R3B.8 are pushed and remote-closed (`52124a0` for R3B.7, `9cb5e28`
for the R3B.8 checkpoint). The full B-1R3C–B-1R3F evaluator migration plus the
**local production cutover** (`62dd592`) are implemented and validated locally
but **NOT pushed** (see the local commit range and the super-transaction
report). Production now runs the explicit-continuation machine on every entry
point; the recursive evaluator is retained only as the differential reference
and rollback path (B-1R8 removes it after the release decision).

B-1R phase state:

- **B-1:** OPEN — evaluator migration complete and cut over locally
  (`62dd592` + seam closure `11abad2`); adversarial review and validation
  matrix done; the human-authorized push gate is pending, and B-1R8
  (recursive-engine removal) follows a release decision. The released
  WASM runtime still traps on the engine stack below the 512-frame language
  limit; `v0.2.1` is immutable and contains the defect. The freshly built
  machine-backed WASM holds the boundary (proven by
  `playground/tests/node/b1_boundary.test.mjs`). Native conforms.
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
- **Production cutover (`62dd592`):** COMPLETE LOCALLY, NOT PUSHED. Every
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

## Exact Next Action

See `docs/engineering/CURRENT_HANDOFF.md`. In short: the complete B-1R3C–B-1R3F
evaluator migration and the local production cutover `62dd592` are implemented,
oracle-covered, and validated locally on top of the remotely closed `9cb5e28`
checkpoint, but are **not pushed**. Remaining before B-1 can close: finish the
adversarial review and the residual-seam worktree commit, then present the
commit range for one final human-authorized push gate. Do not push
implementation commits and do not start unrelated Aura 0.3 work.

## Writer

NONE
