# Aura Performance

Baselines, methodology, and budgets for the Performance Engineering role.
Companion to `tests/bench.rs`.

## Methodology

- Two layers in `tests/bench.rs`:
  - **Shape guards** (always run in CI): assert that doubling user-controlled N
    scales within a generous multiple of linear (`< 3.2×`), catching accidental
    super-linear behaviour without committing to wall-clock constants.
  - **Timing report** (`#[ignore]`): prints per-stage milliseconds per N for
    humans and release audits. Never asserted.
- Timings are averaged over 3 runs after a warm-up; ratios, not absolutes, are
  asserted. Host-specific numbers are illustrative, not contractual.

## Reproduce

```
cargo test --locked --test bench
cargo test --locked --release --test bench -- --ignored --nocapture
```

## Baseline (Apple Silicon, release, 2026-09)

`lex`/`parse`/`check` on `N` generated top-level functions:

| N | lex (ms) | parse (ms) | check (ms) |
|--:|--:|--:|--:|
| 100 | 0.038 | 0.170 | 0.460 |
| 200 | 0.064 | 0.230 | 0.609 |
| 400 | 0.111 | 0.419 | 1.114 |
| 800 | 0.207 | 0.800 | 2.130 |
| 1600 | 0.383 | 1.489 | 4.101 |

Shape: all three stages are linear in N across the measured range. Module-graph
assembly over N children, deep-expression parsing, and list materialisation are
likewise sub-quadratic (guards pass).

## Budgets

These are guard-rails, not promises. A change that breaches a budget requires a
recorded before/after and an explanation.

| Surface | Budget (release, N=1600) |
|---|---|
| lex | < 5 ms |
| parse | < 15 ms |
| check | < 30 ms |
| module graph, N children | sub-quadratic doubling ratio |
| runtime list materialisation, N=2000→4000 | sub-quadratic doubling ratio |

## Known characteristic: REPL is O(N²) in session size

`SessionEngine` rebuilds the checker environment from all prior session
declarations on every submission (`Checker::with_declarations` over the growing
`decls`). N submissions therefore cost O(N²). Measured (release, single batch of
`let v{i} = i` submissions):

| submissions | ms |
|--:|--:|
| 100 | 16.1 |
| 200 | 48.8 |
| 400 | 174.7 |
| 800 | 661.5 |
| 1600 | 2575.6 |
| 3200 | 10162.4 |

A doubling ratio of ~4 confirms quadratic. This is a **performance
characteristic, not a correctness defect**: sessions of ordinary length are
unaffected. Tracked as TD-13; the guard
`repl_incremental_state_is_not_worse_than_quadratic` fails only on a *cubic*
regression. Incremental checking is a deferred optimisation.

## Regression protection

- `tests/bench.rs` shape guards run in the default CI test job.
- Any future optimisation must preserve semantics (all functional tests green)
  and record before/after numbers here.

## History

- 2026-09: initial baseline recorded; no catastrophic scaling found.
