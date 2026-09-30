#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Performance and scaling characterisation (Performance Engineering role).
//!
//! Two layers, deliberately separated so CI is stable:
//!
//! * **Shape guards** (always run): assert that the pipeline scales within a
//!   generous multiple of linear for user-controlled N. These catch accidental
//!   super-linear behaviour without depending on wall-clock constants, so they
//!   are safe on shared CI.
//! * **Timing report** (`#[ignore]`, run with `--ignored`): prints actual
//!   milliseconds per stage per N for humans/release audits. Never asserted.
//!
//! Baselines and budgets live in `docs/engineering/PERFORMANCE.md`.

use std::fmt::Write as _;
use std::time::Instant;

use aura::check::Checker;
use aura::lex::lex;
use aura::module_graph::{InMemorySourceProvider, ModuleGraphBuilder, SourceKey};
use aura::parse::parse;
use aura::run_source;
fn gen_functions(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        let _ = writeln!(s, "fn f{i}(x: int) -> int {{ return x + {i} }}");
    }
    s.push_str("fn main() { print(f0(1)) }\n");
    s
}

fn gen_lets(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        let _ = writeln!(s, "let v{i} = {i}");
    }
    s.push_str("fn main() { print(v0) }\n");
    s
}

fn gen_deep_expr(n: usize) -> String {
    let mut s = String::from("fn main() { print(");
    for _ in 0..n {
        s.push('(');
    }
    s.push('1');
    for _ in 0..n {
        s.push(')');
    }
    s.push_str(") }\n");
    s
}

fn gen_list_call(n: usize) -> String {
    let items = (0..n).map(|i| i.to_string()).collect::<Vec<_>>().join(", ");
    format!("fn main() {{ print(len([{items}])) }}\n")
}

/// Measure one workload, repeating it until each sample is comfortably above
/// scheduler noise, then return the **minimum** per-iteration milliseconds
/// (the minimum is the least-noise-contaminated estimate).
///
/// This matters for CI stability: a 5 ms measurement on a shared runner is
/// dominated by jitter, so the ratio of two such measurements can be anything.
/// By accumulating repetitions until a single sample spans `MIN_SAMPLE_MS`, the
/// ratio reflects real work, not noise.
fn time_millis(mut work: impl FnMut()) -> f64 {
    const MIN_SAMPLE_MS: f64 = 40.0;
    const MAX_REPS_PER_SAMPLE: u32 = 1_000_000;
    const SAMPLES: u32 = 3;

    // Warm up (JIT-free, but caches/allocator warm).
    work();

    // Determine a repetition count that makes one sample ~MIN_SAMPLE_MS.
    let probe = Instant::now();
    work();
    let one = probe.elapsed().as_secs_f64() * 1000.0;
    let reps = if one <= 0.0 {
        1
    } else {
        ((MIN_SAMPLE_MS / one).ceil() as u32).clamp(1, MAX_REPS_PER_SAMPLE)
    };

    let mut best = f64::MAX;
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..reps {
            work();
        }
        let per = start.elapsed().as_secs_f64() * 1000.0 / f64::from(reps);
        if per < best {
            best = per;
        }
    }
    best
}

/// Ratio of stage cost at 2N to cost at N, using noise-robust per-iteration
/// timings. Linear work gives ~2; a genuinely quadratic path gives ~4. The
/// guard allows up to 3.2× to absorb constant-factor and cache effects while
/// still failing on real quadratic blow-up.
fn assert_subquadratic(label: &str, stage: impl Fn(usize) + Copy, base: usize) {
    // Take the best ratio of up to two attempts. Each attempt already uses
    // min-of-samples above a noise floor; a second attempt (after a short
    // settle) absorbs a transient CI spike, so only a genuine super-linear
    // shape can fail the release train.
    let mut ratio = ratio_of(stage, base);
    if ratio >= 3.2 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        ratio = ratio.min(ratio_of(stage, base));
    }
    assert!(
        ratio < 3.2,
        "{label}: doubling N scaled {ratio:.2}x, expected sub-quadratic"
    );
}

fn ratio_of(stage: impl Fn(usize) + Copy, base: usize) -> f64 {
    let small = time_millis(|| stage(base));
    let large = time_millis(|| stage(base * 2));
    if small <= 0.0 {
        1.0
    } else {
        large / small
    }
}

#[test]
fn parse_scales_subquadratically() {
    assert_subquadratic(
        "parse(functions)",
        |n| {
            let src = gen_functions(n);
            let _ = parse(&src).unwrap();
        },
        400,
    );
}

#[test]
fn lex_scales_subquadratically() {
    assert_subquadratic(
        "lex(functions)",
        |n| {
            let src = gen_functions(n);
            let _ = lex(&src).unwrap();
        },
        400,
    );
}

#[test]
fn check_scales_subquadratically() {
    assert_subquadratic(
        "check(functions)",
        |n| {
            let src = gen_functions(n);
            let m = parse(&src).unwrap();
            Checker::module(&m).unwrap();
        },
        400,
    );
}

#[test]
fn check_lets_scale_subquadratically() {
    assert_subquadratic(
        "check(lets)",
        |n| {
            let src = gen_lets(n);
            let m = parse(&src).unwrap();
            Checker::module(&m).unwrap();
        },
        400,
    );
}

#[test]
fn deep_expression_scales_subquadratically() {
    assert_subquadratic(
        "parse(deep_expr)",
        |n| {
            let src = gen_deep_expr(n);
            let _ = parse(&src).unwrap();
        },
        200,
    );
}

#[test]
fn runtime_list_materialisation_scales_subquadratically() {
    assert_subquadratic(
        "run(list_call)",
        |n| {
            let src = gen_list_call(n);
            let _ = run_source(&src, "<bench>").unwrap();
        },
        2_000,
    );
}

#[test]
fn module_graph_scales_subquadratically() {
    assert_subquadratic(
        "module_graph(children)",
        |n| {
            let entry = SourceKey::new("main");
            let mut provider = InMemorySourceProvider::new(entry.clone());
            provider
                .insert_source(entry.clone(), "main.aura", "fn main() { print(1) }\n")
                .unwrap();
            for i in 0..n {
                let key = SourceKey::new(format!("child{i}"));
                provider
                    .insert_source(
                        key.clone(),
                        format!("child{i}.aura"),
                        format!("pub let v{i} = {i}\n"),
                    )
                    .unwrap();
                provider
                    .add_child(&entry, format!("child{i}"), &key)
                    .unwrap();
            }
            let _ = ModuleGraphBuilder::new().build(&provider).unwrap();
        },
        // A larger base keeps each build well above scheduler noise; the graph
        // is linear (measured ~5/7.5/15/30/60/121 ms for N=100..3200), so the
        // ratio should be ~2 even under load.
        400,
    );
}

/// REPL submissions rebuild the checker environment from all prior session
/// declarations each time (`Checker::with_declarations`), so N submissions is
/// **O(N²)** by design (recorded as TD-13). This guard therefore asserts only
/// that the cost is *no worse than quadratic*: a doubling ratio near 4 is
/// expected, and a cubic regression (~8) fails. Making the REPL incremental is
/// a tracked optimisation, not a correctness fix.
#[cfg(feature = "repl")]
#[test]
fn repl_incremental_state_is_not_worse_than_quadratic() {
    use aura::repl::SessionEngine;
    let run = |n: usize| {
        time_millis(|| {
            let mut engine = SessionEngine::new();
            let mut sink = Vec::new();
            for i in 0..n {
                let line = format!("let v{i} = {i}");
                let _ = engine.feed_line(&line, &mut sink);
            }
        })
    };
    let base = 150;
    let small = run(base);
    let large = run(base * 2);
    let ratio = if small <= 0.0 { 1.0 } else { large / small };
    // Quadratic ≈ 4; allow headroom for constant-factor/cache effects, but a
    // cubic path is ~8 and must fail.
    assert!(
        ratio < 6.0,
        "repl(batches): doubling N scaled {ratio:.2}x ({small:.3}ms -> {large:.3}ms); \
         expected no worse than quadratic (TD-13)"
    );
}

/// Human-readable timing report. Run with:
/// `cargo test --release --test bench -- --ignored --nocapture`.
#[test]
#[ignore = "timing report, not an assertion"]
fn timing_report() {
    let sizes = [100usize, 200, 400, 800, 1600];
    println!("stage,N,ms");
    for n in sizes {
        let src = gen_functions(n);
        let lex_ms = time_millis(|| {
            let _ = lex(&src).unwrap();
        });
        let parse_ms = time_millis(|| {
            let _ = parse(&src).unwrap();
        });
        let check_ms = time_millis(|| {
            let m = parse(&src).unwrap();
            Checker::module(&m).unwrap();
        });
        println!("lex,{n},{lex_ms:.3}");
        println!("parse,{n},{parse_ms:.3}");
        println!("check,{n},{check_ms:.3}");
    }
}
