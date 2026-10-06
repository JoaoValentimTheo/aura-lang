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

/// `n` keyed map insertions then one lookup: exercises map construction and
/// hashing, a runtime workload distinct from list materialisation.
fn gen_map_ops(n: usize) -> String {
    let mut s = String::from("fn main() {\n let mut m = {\"seed\": 0}\n");
    for i in 0..n {
        let _ = writeln!(s, " m[\"k{i}\"] = {i}");
    }
    s.push_str(" print(len(m))\n}\n");
    s
}

/// A chain of `n` top-level function calls: exercises call dispatch, frames,
/// and closures through the interpreter.
fn gen_calls(n: usize) -> String {
    let mut s =
        String::from("fn inc(x: int) -> int { return x + 1 }\nfn main() {\n let mut x = 0\n");
    for _ in 0..n {
        s.push_str(" x = inc(x)\n");
    }
    s.push_str(" print(x)\n}\n");
    s
}

/// Evaluator-shaped workloads for the Pre-0.3 baseline. Each returns a
/// program whose `main` performs `n` units of the named operation; they are
/// only used by the `#[ignore]` timing report.
fn gen_eval_loop(n: usize) -> String {
    format!(
        "fn main() {{\n let mut i = 0\n let mut acc = 0\n while i < {n} {{\n acc = acc + i\n i = i + 1\n }}\n print(acc)\n}}\n"
    )
}

fn gen_eval_recursion(n: usize) -> String {
    format!(
        "fn rec(k: int) -> int {{\n if k <= 0 {{\n return 0\n }} else {{\n return k + rec(k - 1)\n }}\n}}\nfn main() {{ print(rec({n})) }}\n"
    )
}

fn gen_eval_closures(n: usize) -> String {
    format!(
        "fn apply(f, x: int) -> int {{ return f(x) }}\nfn main() {{\n let add = (x: int) -> {{ return x + 1 }}\n let mut acc = 0\n let mut i = 0\n while i < {n} {{\n acc = apply(add, acc)\n i = i + 1\n }}\n print(acc)\n}}\n"
    )
}

fn gen_eval_index(n: usize) -> String {
    let mut s = String::from("fn main() {\n let xs = [0");
    for i in 1..n {
        let _ = write!(s, ", {i}");
    }
    let _ = write!(
        s,
        "]\n let mut i = 0\n let mut acc = 0\n let mut k = 0\n while k < {n} {{\n acc = acc + xs[i]\n i = i + 1\n if i >= {n} {{ i = 0 }}\n k = k + 1\n }}\n print(acc)\n}}\n"
    );
    s
}

fn gen_eval_methods(n: usize) -> String {
    format!(
        "fn main() {{\n let mut s = \"seed\"\n let mut i = 0\n while i < {n} {{\n s = s.trim()\n i = i + 1\n }}\n print(s)\n}}\n"
    )
}

fn gen_eval_fstring(n: usize) -> String {
    format!(
        "fn main() {{\n let mut acc = \"\"\n let mut i = 0\n while i < {n} {{\n acc = f\"{{i}}\"\n i = i + 1\n }}\n print(acc)\n}}\n"
    )
}

fn gen_eval_try(n: usize) -> String {
    format!(
        "fn main() {{\n let mut i = 0\n let mut acc = 0\n while i < {n} {{\n try {{ acc = acc + 1 }} catch e {{ acc = 0 }} finally {{ acc = acc + 0 }}\n i = i + 1\n }}\n print(acc)\n}}\n"
    )
}

fn gen_eval_map(n: usize) -> String {
    format!(
        "fn main() {{\n let m = {{\"a\": 1, \"b\": 2, \"c\": 3}}\n let mut i = 0\n let mut acc = 0\n while i < {n} {{\n acc = acc + m.get(\"a\")\n i = i + 1\n }}\n print(acc)\n}}\n"
    )
}

/// A fold over `n` characters of a string: exercises string iteration and
/// byte handling.
fn gen_string_walk(n: usize) -> String {
    format!(
        "fn main() {{\n let s = \"{}\"\n let mut i = 0\n let mut acc = 0\n while i < len(s) {{\n acc = acc + 1\n i = i + 1\n }}\n print(acc)\n}}\n",
        "a".repeat(n)
    )
}

/// A single string literal of `n` characters (red-team F3 regression shape).
fn gen_long_string(n: usize) -> String {
    format!("let s = \"{}\"\n", "a".repeat(n))
}

/// A linear chain of wrapping aliases `type A{i} = [A{i-1}]` of length `n` (the
/// red-team F1/F2 shape). Kept below the semantic nesting limit in the scaling
/// guard, which exercises the accepted path.
fn gen_alias_chain(n: usize) -> String {
    use std::fmt::Write as _;
    let mut s = String::from("type A0 = int\n");
    for i in 1..=n {
        let _ = writeln!(s, "type A{i} = [A{}]", i - 1);
    }
    let _ = writeln!(
        s,
        "fn f(x: A{n}) -> int {{ return 1 }}\nfn main() {{ print(1) }}"
    );
    s
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

/// Red-team F3: lexing a long string literal must be linear. The previous
/// scanner re-validated the entire remaining source per character (Θ(n²)), so a
/// 5 MB literal took ~40s. The guard doubles N; quadratic gives ~4x.
#[test]
fn lex_long_string_scales_subquadratically() {
    assert_subquadratic(
        "lex(long_string)",
        |n| {
            let src = gen_long_string(n);
            let _ = lex(&src).unwrap();
        },
        100_000,
    );
}

/// Red-team F1/F2: a *linear* alias chain (`type A{i} = [A{i-1}]`) was Θ(k²)
/// because resolution is memoized in declaration order and each cached level
/// nested one deeper (16k aliases took ~40s and 3.4 GB), and above the semantic
/// limit it then overflowed the stack. The depth guard now caps the chain at
/// 256 levels, so the meaningful contract is a bounded absolute cost within the
/// accepted range plus `E1015` past it — not a ratio (the accepted range cannot
/// be doubled). This mirrors the parameterized-chain guard.
#[test]
fn alias_chain_resolution_is_bounded_by_the_depth_cap() {
    // The maximal accepted chain must check quickly.
    let at_limit = gen_alias_chain(255);
    let m = parse(&at_limit).unwrap();
    let start = Instant::now();
    Checker::module(&m).unwrap();
    let elapsed = start.elapsed().as_millis();
    assert!(
        elapsed < 2_000,
        "a depth-capped alias chain took {elapsed}ms; the depth bound should keep it fast"
    );
    // Past the limit is the stable diagnostic, never a hang or overflow.
    let over = gen_alias_chain(20_000);
    let m = parse(&over).unwrap();
    assert!(Checker::module(&m).is_err());
}

/// A *parameterised* alias chain (`type A{i}<T> = A{i-1}<T>`) is expanded by
/// `substitute_alias`, which is not memoised, so its cost is quadratic in the
/// chain length. The red-team re-verification N1 found that with **no depth
/// bound** this was unbounded (k=2000 ≈ 11.5s, k=4000 timed out). The
/// expansion-path depth guard now caps the chain at the semantic limit (256),
/// so the quadratic factor applies only to a bounded length: the absolute worst
/// case is small. This test pins the bound — a maximal accepted chain completes
/// quickly, and one past the limit is a diagnostic — rather than a scaling shape
/// a depth cap makes moot.
#[test]
fn parameterized_alias_chain_is_bounded_by_the_depth_cap() {
    use std::fmt::Write as _;
    let gen = |n: usize| {
        let mut s = String::from("type A0<T> = [T]\n");
        for i in 1..=n {
            let _ = writeln!(s, "type A{i}<T> = A{}<T>", i - 1);
        }
        let _ = write!(
            s,
            "fn f(x: A{n}<int>) -> int {{ return 1 }}\nfn main() {{ print(1) }}"
        );
        s
    };
    // The maximal accepted chain (255 wraps) must check quickly.
    let at_limit = gen(255);
    let m = parse(&at_limit).unwrap();
    let start = Instant::now();
    Checker::module(&m).unwrap();
    let elapsed = start.elapsed().as_millis();
    assert!(
        elapsed < 2_000,
        "a depth-capped parameterized chain took {elapsed}ms; the depth bound should keep it fast"
    );
    // One past the limit is the stable diagnostic, never a hang.
    let over = gen(20_000);
    let m = parse(&over).unwrap();
    assert!(Checker::module(&m).is_err());
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
fn runtime_map_ops_scale_subquadratically() {
    assert_subquadratic(
        "run(map_ops)",
        |n| {
            let src = gen_map_ops(n);
            let _ = run_source(&src, "<bench>").unwrap();
        },
        2_000,
    );
}

#[test]
fn runtime_function_calls_scale_subquadratically() {
    assert_subquadratic(
        "run(calls)",
        |n| {
            let src = gen_calls(n);
            let _ = run_source(&src, "<bench>").unwrap();
        },
        4_000,
    );
}

#[test]
fn runtime_string_walk_scales_subquadratically() {
    assert_subquadratic(
        "run(string_walk)",
        |n| {
            let src = gen_string_walk(n);
            let _ = run_source(&src, "<bench>").unwrap();
        },
        20_000,
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

/// A named evaluator workload: a report label, a program generator, and the
/// sizes to measure (some stages have a lower useful maximum, like the
/// 512-call-frame recursion cap).
type EvalStage = (&'static str, fn(usize) -> String, &'static [usize]);

/// The standard measurement ladder.
const EVAL_SIZES: &[usize] = &[50, 100, 200, 400, 800];

/// Recursion is capped at 400: 512 is the language call-frame limit, so
/// larger N would be a legitimate `E4011`, not a measurement.
const RECURSION_SIZES: &[usize] = &[50, 100, 200, 400];

/// Evaluator-stage timing report for the Pre-0.3 baseline. Run with:
/// `cargo test --release --test bench -- --ignored --nocapture eval_timing`.
///
/// Every workload executes through the production explicit-continuation
/// machine (`run_source`), so the numbers are the baseline the 0.3 work must
/// not regress. Absolute values are host-specific; the recorded baseline in
/// `docs/engineering/PERFORMANCE.md` names the machine it was measured on.
#[test]
#[ignore = "timing report, not an assertion"]
fn eval_timing_report() {
    println!("stage,N,ms");
    let stages: [EvalStage; 8] = [
        ("eval_loop", gen_eval_loop, EVAL_SIZES),
        ("eval_recursion", gen_eval_recursion, RECURSION_SIZES),
        ("eval_closures", gen_eval_closures, EVAL_SIZES),
        ("eval_index", gen_eval_index, EVAL_SIZES),
        ("eval_methods", gen_eval_methods, EVAL_SIZES),
        ("eval_fstring", gen_eval_fstring, EVAL_SIZES),
        ("eval_try", gen_eval_try, EVAL_SIZES),
        ("eval_map", gen_eval_map, EVAL_SIZES),
    ];
    for (name, gen, stage_sizes) in stages {
        for &n in stage_sizes {
            let src = gen(n);
            let ms = time_millis(|| {
                let _ = run_source(&src, "<bench>").unwrap();
            });
            println!("{name},{n},{ms:.4}");
        }
    }

    // Whole-pipeline cost on the generated function corpus, for scale.
    for n in [100usize, 400, 1600] {
        let src = gen_functions(n);
        let ms = time_millis(|| {
            let _ = run_source(&src, "<bench>").unwrap();
        });
        println!("eval_pipeline,{n},{ms:.3}");
    }
}
