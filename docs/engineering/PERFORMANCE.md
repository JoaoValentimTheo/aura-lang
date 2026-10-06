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
- Timings use the minimum of 3 samples after a warm-up (least-noise
  estimate); ratios, not absolutes, are asserted. Host-specific numbers are
  illustrative, not contractual.

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
| runtime map ops, N=2000→4000 | sub-quadratic doubling ratio |
| runtime function calls, N=4000→8000 | sub-quadratic doubling ratio |
| runtime string walk, N=20000→40000 | sub-quadratic doubling ratio |
| lex long string literal, N=100k→200k | sub-quadratic doubling ratio |
| check alias chain (below depth limit), N=60→120 | sub-quadratic doubling ratio |

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

## Pre-0.3 evaluator baseline (2026-10-05)

The production engine is the explicit-continuation machine. These numbers are
measured with the `eval_timing_report` (`tests/bench.rs`, `#[ignore]`), every
workload executed end-to-end through `run_source`, on Apple M2 / arm64, macOS,
release profile, min-of-3 samples after warm-up. Absolute values are
host-specific and illustrative; the shape (linear in N) is the contract.

| stage,N | 50 | 100 | 200 | 400 | 800 |
|---|--:|--:|--:|--:|--:|
| `eval_loop` (while + arithmetic) | 0.158 | 0.197 | 0.299 | 0.496 | 0.894 |
| `eval_recursion` (self-call, ≤400: 512-frame limit) | 0.160 | 0.212 | 0.318 | 0.526 | — |
| `eval_closures` (lambda called via a function) | 0.200 | 0.316 | 0.518 | 0.920 | 1.730 |
| `eval_index` (list index in a loop) | 0.217 | 0.324 | 0.520 | 0.941 | 1.759 |
| `eval_methods` (string method in a loop) | 0.155 | 0.204 | 0.309 | 0.500 | 0.912 |
| `eval_fstring` (interpolated format per iteration) | 0.153 | 0.200 | 0.295 | 0.491 | 0.867 |
| `eval_try` (try/catch/finally per iteration) | 0.182 | 0.255 | 0.400 | 0.698 | 1.275 |
| `eval_map` (map lookup per iteration) | 0.168 | 0.228 | 0.347 | 0.586 | 1.062 |

Doubling N scales every stage within 1.75–2.0×: linear, no evaluator-stage
super-linearity. Cheapest stages are loops, methods, and f-strings; closures
and indexing carry roughly 2× their per-unit cost because each iteration is an
extra call/index dispatch. The whole-pipeline cost on N generated functions
(lex+parse+check+execute) is 0.434 / 1.363 / 5.149 ms at N=100/400/1600, also
linear. Re-run:

```
cargo test --locked --release --test bench -- --ignored --nocapture eval_timing
cargo test --locked --release --test bench -- --ignored --nocapture timing_report
```

The 2026-09 lex/parse/check baseline reproduces within jitter (lex 0.442 ms,
parse 1.465 ms, check 4.116 ms at N=1600 on the same class of machine).

## Regression protection

- `tests/bench.rs` shape guards run in the default CI test job.
- Any future optimisation must preserve semantics (all functional tests green)
  and record before/after numbers here.

## History

- 2026-09: initial baseline recorded; no catastrophic scaling found.
- 2026-10-05: Pre-0.3 evaluator-stage baseline recorded (machine-backed
  production engine); lex/parse/check baseline re-confirmed. No super-linear
  evaluator shape found.
