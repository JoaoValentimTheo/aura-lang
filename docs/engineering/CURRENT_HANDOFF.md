# Current Engineering Handoff

Authoritative for **the active task and the exact next action**. Current
repository state lives in `AGENT_STATE.md`; operating rules in `AGENTS.md`.
Keep this file short — it is read at the start of every session.

## Program

B-1 — WebAssembly call-frame implementation nonconformance. The released
runtime traps on the JavaScript engine stack below the normative 512-frame
language limit instead of reporting `E4011`. The chosen remediation is an
explicit-continuation (iterative) evaluator over the existing AST
(`docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`, Option E1).

## Current status

- B-1: **OPEN** — full iterative evaluator not implemented; production is
  still recursive.
- B-1R1 (design): COMPLETE.
- B-1R2 (differential oracle): COMPLETE and mutation-validated.
- B-1R3A-ARCH-1 (Arc AST sharing): RESOLVED.
- B-1R3A (machine skeleton + first executable subset): **COMPLETE AND REMOTELY
  CLOSED** at `c9ade0b` (exact-SHA CI green; website deployed).
- B-1R3B.1 (unary `-`/`not`/`~`): **COMPLETE LOCALLY**, unpushed (local
  commits on top of `origin/rewrite/v3-rust`).
- B-1R3B.2…R3G: NOT STARTED.

See `AGENT_STATE.md` for the exact SHAs and ahead/behind.

## Production vs experimental engine

- Production/default/production path: **recursive** engine, unchanged.
- Iterative engine: exists only under the non-default `evaluator-oracle`
  Cargo feature; not reachable from the CLI, REPL, Playground, or library
  production path. It currently supports literals, name lookup, expression
  statements, blocks, `let` shadowing, `if`/`else`, and unary `-`/`not`/`~`;
  every other construct returns the deterministic `E4999` sentinel and never
  falls back to recursion.

## Completed in B-1R3B.1 (local)

- `src/run/iterative.rs` — added `Cont::UnaryApply { op, span }`; the operand is
  evaluated once and the operator is applied at its completion. `apply_unary`
  mirrors `Interp::eval`'s `Expr::Unary` arm exactly (checked `int` negation with
  `E4013`, `not` over `truthy()`, `~` on `int` only, both type errors `E3001`
  with the same messages). Operand control signals (`return`/`throw`/…) propagate
  without applying the operator; a deep unary chain is stack-safe.
- Oracle: new `tests/oracle/r3b.rs` + `tests/oracle/r3b_golden.tsv` (LF-pinned)
  and differential tests `r3b_unary_supported_subset_agrees`,
  `iterative_unary_unsupported_fails_explicitly`, `r3b_iterative_golden_matches`.
  The R3A unsupported case `unary_neg` was removed (now supported) and
  `r3a_golden.tsv` regenerated accordingly; the main `golden.tsv` is unchanged.
- Deliberate mutation of `Not` was detected by both R3B tests and reverted.

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
- B-1 must stay OPEN until the full migration and the 512-frame boundary hold
  on every substrate.

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

## Next phase — B-1R3B (values and operators)

Implement the next semantic slice on the same machine, microphase by
microphase, keeping the oracle green and production unchanged. R3B.1 (unary)
is complete; remaining ordered microphases; each is implement → targeted
tests → oracle → checkpoint. Do **not** start the next one before human review
of B-1R3B.1.

- R3B.1 unary operators — **COMPLETE LOCALLY**
- R3B.2 binary operators
- R3B.3 short-circuit / evaluation order
- R3B.4 list / tuple / map
- R3B.5 range
- R3B.6 index / field reads
- R3B.7 f-strings
- R3B.8 milestone adversarial closure

Use multi-reviewer analysis mainly at milestone closure, not after each
microphase.

## Exact next action

1. Human review of the B-1R3B.1 unary extension and its oracle coverage.
2. If accepted, begin **R3B.2 (binary operators)** on `src/run/iterative.rs`,
   extending the oracle and keeping `tests/oracle/golden.tsv` byte-unchanged.
3. Then proceed through R3B.3…R3B.8 with a checkpoint at each.

## Stop conditions

Stop and report instead of improvising if:

- Git reality contradicts `AGENT_STATE.md`;
- production behavior would change;
- the language contract and current behavior disagree in a way that needs a
  semantic decision (produce a decision package instead);
- a frozen artifact would be touched;
- a genuine oracle blind spot is found that cannot be closed within the slice.
