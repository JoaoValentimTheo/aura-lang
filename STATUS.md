# Aura Current Status

Authoritative continuation checkpoint for the FINAL IMPLEMENTATION /
release-readiness continuation.

## Repository

- Verified on 2026-09-28.
- Branch: `rewrite/v3-rust`.
- Baseline HEAD at task start: `34eac815b6a56df7bc27f23ed4fb51565df4ff77`.
- HEAD after this task: `bacba57` (`fix(cli): list repl in the unknown-command hint`),
  on top of `81e7410` (`docs: reconcile release-readiness documentation surfaces`).
- Untracked (pre-existing workflow infrastructure, preserved): `.codex/`,
  `.kilo/`, `AGENTS.md`, `IMPLEMENT.md`, `PLANS.md`, `STATUS.md`.
- Pre-existing stray untracked file `s` (1 byte, mtime 2026-09-28 11:59,
  before this session). Not produced by this work; left untouched.

## Last Verified Global State

Historical baseline evidence, not rerun during the prior state-verification task:
- Rust all-features at baseline: 650 passed, 0 failed.
- Selected no-default at baseline: 641 passed, 0 failed.
- GitHub CI: `36390575872`, 14 jobs, success.
- Deploy: `36390575875`, success.

## Active Task / Current Milestone

FINAL IMPLEMENTATION → release-readiness. Milestones M1 (forensic resume),
M2 (parallel reconnaissance), M3 (backlog classification), M4 (Category-A
implementation), M5 (validation + adversarial review), and M6 (durable
checkpoint) are complete.

## Confirmed Findings

Reconnaissance (scout + spec auditor + artifact verifier + deep auditor) found
**no production implementation defect**. Every unfinished construct is either a
documented SPEC GAP (design-gated) or a deliberate DESIGN LIMITATION. The only
objectively incomplete, non-design-gated work was **documentation drift /
release-readiness surface inconsistency**.

- Unimplemented/inert surfaces are exactly the nine documented SPEC GAPs, not
  defects: `pub use` re-exports, `pub module` visibility boundary, module
  aliases, variant-tag scope, canonical-name collision, separator policy,
  numeric underscores, f-string outer edges, generic-head `::` continuation.
- Named method arguments, list-rest patterns, range step, and multiline
  pipelines are RFC candidates; the current rejection behavior is itself
  normatively specified, so implementing them would be new work.
- `if`/`match`/lambda inferring `Unknown` is an intentional §34.2 boundary.
- No `TODO`/`FIXME`/`todo!()`/`unimplemented!()` markers in `src/`.
- The WASM rebuild from current source is byte-identical to dev.29.

## Changes Made

Two coherent commits:

- `81e7410 docs: reconcile release-readiness documentation surfaces`
  - `docs/LANGUAGE_SPEC.md` §26: removed the stale "`as` is reserved but
    unconsumed; `use a as b` is a parse error" sentence, which contradicted
    §27 (normative `use path as Alias`).
  - `docs/LANGUAGE_SPEC.md` §30.2: added the E2017/E2018/E2019 rows that were
    normatively used in §17.7/§27/§36 and present in `docs/errors.md` but
    absent from the error table.
  - `docs/LANGUAGE_SPEC.md`: corrected §15.7→§15.8 and §15.8→§15.9
    cross-references (Arguments is §15.8; Overloading is §15.7).
  - `docs/contract.md` §3: replaced the never-valid `use stdlib.math` example
    with a real in-source module import.
  - `docs/contract.md` §6: added E2016–E2019 to the checker table.
  - `website/content/reference-errors.md`: added the missing
    E1005/E2017/E2018/E2019 rows.
  - `README.md`: replaced the stale "`use` and `pub` are reserved and inert"
    claim with the real module/visibility semantics.
  - `CONTRIBUTING.md`: completed the `tests/` inventory.
- `bacba57 fix(cli): list repl in the unknown-command hint`
  - `src/main.rs`: the unknown-command message now lists `repl`; the module
    doc block also lists all subcommands including `repl`.
  - `tests/syntax_conformance.rs`: new regression
    `cli_unknown_command_hint_lists_every_implemented_command`.

## Validation Actually Run

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --locked --all-targets --all-features -- -D warnings` — pass.
- `cargo test --locked --all-targets --all-features` — **651 passed, 0 failed**
  (baseline 650 + 1 new CLI regression).
- `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time`
  — all pass (selected no-default surface).
- `node playground/tests/node/run-all.mjs` — manifest 26, ABI 67, integrity 27,
  differential 195, syntax 43, browser 53, worker 12, cache 7 — 0 failed.
- `node playground/build.mjs --check` — `manifest matches 3 version(s)`.
- WASM release build (clean `CARGO_TARGET_DIR`, wasm32-unknown-unknown) —
  1,614,239 bytes, `aa832ba72578897f6b99650574939011dda25e0d390fdb5efb6fae825816bdd3`,
  byte-identical to dev.29.
- `node website/tests/run-all.mjs` — examples 19, links 2086, browser 343,
  a11y 70, manifest 26, ABI 67, integrity 27, differential 195, syntax 43,
  browser 53, worker 12, cache 7 — 0 failed.
- Independent adversarial review of the diff: 8/9 claims survived; the one
  falsification (`contract.md` still showed an invalid `stdlib::math` example)
  and two minor findings (main.rs doc block; CONTRIBUTING label) were fixed and
  re-validated.

## Artifact State

All paths are under `playground/runtimes/`, ending in `/aura_playground_runtime.wasm`.

| Runtime | Bytes | SHA-256 |
| --- | ---: | --- |
| Frozen `0.0.2` | 1,366,621 | `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed` |
| Historical `0.0.2-dev.23` | 1,604,958 | `71072150e67384120c63e22d6176f3683110735b84f74723bea315f79778a528` |
| Manifest-selected `0.0.2-dev.29` | 1,614,239 | `aa832ba72578897f6b99650574939011dda25e0d390fdb5efb6fae825816bdd3` |

No runtime bump: the only source change is the native-only CLI message;
the WASM rebuild is byte-identical to dev.29. Historical artifacts untouched.

## Open Decisions / SPEC GAPs

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

Nine gaps remain unresolved and were NOT implemented: separator policy /
CONF-PARSE-8; numeric underscore placement; f-string outer edge rules;
generic-head `::` continuation / CONF-GRAM-4; variant-tag uniqueness /
CONF-RESOLVE-6; nested-module visibility / CONF-RESOLVE-7; `pub use`
re-exports / CONF-RESOLVE-8; module aliases / CONF-RESOLVE-9; type/variant
canonical-name collisions / CONF-RESOLVE-10.

## Next Exact Action

Await user authorization. Category-A implementation is complete; no
evidence-backed unfinished implementation work remains. Any next step is a
human design decision (a SPEC GAP or AUDIT-3) or an explicitly authorized
release/tag/deploy.

## Do Not Reopen

- Completed whole-language conformance/hardening without a reproduced regression.
- Closed rendering/equality/Python-bridge/integrity audits without new evidence.
- AUDIT-3 without the exact human approval token.
- The nine SPEC GAPs as "bugs"; they require a human design decision.
- Historical runtime artifacts; never overwrite them.
