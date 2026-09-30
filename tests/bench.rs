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

fn time_millis(mut work: impl FnMut()) -> f64 {
    // Warm once, then average a few runs to damp scheduler noise.
    work();
    let runs = 3;
    let start = Instant::now();
    for _ in 0..runs {
        work();
    }
    start.elapsed().as_secs_f64() * 1000.0 / f64::from(runs)
}

/// Ratio of stage cost at 2N to cost at N. Linear work gives ~2; a quadratic
/// path gives ~4. The guard allows up to 3.2 to absorb constant-factor noise
/// while still failing on genuine quadratic blow-up.
fn assert_subquadratic(label: &str, stage: impl Fn(usize) + Copy, base: usize) {
    let small = time_millis(|| stage(base));
    let large = time_millis(|| stage(base * 2));
    // Guard against a zero/near-zero denominator masking a blow-up.
    let ratio = if small < 0.5 { 1.0 } else { large / small };
    assert!(
        ratio < 3.2,
        "{label}: doubling N scaled {ratio:.2}x ({small:.3}ms -> {large:.3}ms), \
         expected sub-quadratic"
    );
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
        100,
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
