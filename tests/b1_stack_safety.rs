#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! B-1 stack-safety and resource-boundary campaign.
//!
//! Pins the properties the iterative engine exists for: user-frame recursion
//! and loop iteration must not consume host stack proportional to depth or
//! iteration count, the 512-frame boundary stays exact (E4011 at 513, from
//! `main` frame 1), deep expressions agree with the recursive engine, and the
//! shared resource caps (range materialization, format width/precision) are
//! unchanged. These run on deliberately small thread stacks so a regression to
//! host-stack recursion fails loudly.

#[path = "oracle/mod.rs"]
mod harness;

use harness::{observe, Case, Completion, Kind};

fn p(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "campaign",
        name,
        file: "campaign.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

fn run_on(
    engine: harness::Engine,
    case: Case,
    stack: usize,
) -> Result<harness::Observable, String> {
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || observe(&case, engine).map_err(|e| e.to_string()))
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "thread aborted (stack overflow)".to_string())?
}

#[test]
fn stack_and_resource_campaign() {
    // Deep recursion at the legal limit through a real frame boundary.
    let deep_ok = "fn main() { f(510) }\nfn f(n) { if n { f(n - 1) } else { 0 } }\n";
    // One past the limit must be E4011.
    let deep_over = "fn main() { f(511) }\nfn f(n) { if n { f(n - 1) } else { 0 } }\n";
    // A million-iteration loop must not consume host stack (iterative only).
    let big_loop = "fn main() { let mut i = 0\n let mut s = 0\n while i < 1000000 { s = s + 1\n i = i + 1 } }\n";
    // Deep expression nesting near the AST limit.
    let deep_expr = format!(
        "fn main() {{ let x = {}1{} }}\n",
        "(".repeat(250),
        ")".repeat(250)
    );
    let deep_expr_over = format!(
        "fn main() {{ let x = {}1{} }}\n",
        "(".repeat(300),
        ")".repeat(300)
    );

    for (name, src, want_e4011) in [
        ("deep_ok_510", deep_ok, false),
        ("deep_over_511", deep_over, true),
        ("big_loop_1e6", big_loop, false),
    ] {
        let case = p(name, src);
        // Iterative on a small (1 MiB) stack: the machine must not rely on
        // host stack proportional to frames or iterations.
        match run_on(harness::Engine::iterative(), case, 1 << 20) {
            Ok(o) => {
                let e4011 = matches!(&o.completion, Completion::Runtime(d) if d.code == 4011);
                assert_eq!(e4011, want_e4011, "{name}: {o:?}");
                eprintln!("iterative {name} @1MB: {:?}", o.completion);
            }
            Err(e) => panic!("iterative {name} @1MB aborted: {e}"),
        }
    }

    // Deep-expression cases compare both engines for identical E1015 behavior.
    // Parentheses are not AST nodes, so both 250 and 300 nested parens are
    // legal; the meaningful property is engine agreement at each depth (and
    // machine success on a 1 MiB stack).
    for (name, src, want_e1015) in [
        ("deep_expr_250", deep_expr.clone(), false),
        ("deep_expr_300", deep_expr_over.clone(), false),
        (
            "deep_expr_600",
            format!(
                "fn main() {{ let x = {}1{} }}\n",
                "(".repeat(600),
                ")".repeat(600)
            ),
            false,
        ),
    ] {
        let mk = || {
            p(
                Box::leak(name.to_string().into_boxed_str()),
                Box::leak(src.clone().into_boxed_str()),
            )
        };
        let rec = run_on(harness::Engine::recursive(), mk(), 64 << 20).expect("recursive");
        let it = run_on(harness::Engine::iterative(), mk(), 1 << 20).expect("iterative");
        assert_eq!(rec.completion, it.completion, "{name} diagnostic parity");
        let e1015 = matches!(&it.completion, Completion::Runtime(d) if d.code == 1015);
        assert_eq!(e1015, want_e1015, "{name}: {it:?}");
        eprintln!("{name} agreement: {:?}", it.completion);
    }

    // Resource boundaries are shared constants: range materialization cap,
    // format width/precision caps.
    // `for` over a Range is lazy (no materialization), so it must succeed;
    // a comprehension over a Range materializes through `iterate` and hits
    // the 10,000,000-element cap.
    let range_cap = "fn main() { let xs = [i for i in 0..10000001] }\n";
    let range_lazy = "fn main() { for i in 0..999999999 { break } }\n";
    let fmt_prec = "fn main() { let s = f\"{1.0:.70000f}\" }\n";
    let fmt_width = "fn main() { let s = f\"{1:10000001}\" }\n";
    for (name, src, code) in [
        ("range_cap_comprehension", range_cap, 4013u16),
        ("range_lazy_break", range_lazy, 0),
        ("fmt_prec_cap", fmt_prec, 4013),
        ("fmt_width_cap", fmt_width, 4013),
    ] {
        let case = p(name, src);
        let it = run_on(harness::Engine::iterative(), case, 8 << 20).expect("iterative");
        if code == 0 {
            assert!(matches!(it.completion, Completion::Ok), "{name}: {it:?}");
        } else {
            let got = match &it.completion {
                Completion::Runtime(d) => d.code,
                other => panic!("{name}: expected E{code}, got {other:?}"),
            };
            assert_eq!(got, code, "{name}");
        }
        eprintln!("{name}: ok");
    }
}
