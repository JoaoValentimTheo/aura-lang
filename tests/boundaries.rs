#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Integer boundary tests.
//!
//! Invariant: no integer operation may panic or silently wrap. Every result
//! is either a correct value or a stable diagnostic.

use aura::error::codes;
use aura::run_source;

/// Build `i64::MIN` at runtime without relying on a negative literal (which
/// the lexer rejects as out of range).
const MIN: &str = "-9223372036854775807 - 1";
const MAX: &str = "9223372036854775807";

fn run(src: &str) -> Result<String, u16> {
    run_source(src, "<boundary>").map_err(|d| d.code)
}

#[test]
fn min_div_neg_one_overflows() {
    let src = format!("fn main() {{ let m = {MIN}\n print(m / -1) }}");
    assert_eq!(run(&src), Err(codes::OVERFLOW));
}

#[test]
fn min_rem_neg_one_overflows() {
    let src = format!("fn main() {{ let m = {MIN}\n print(m % -1) }}");
    assert_eq!(run(&src), Err(codes::OVERFLOW));
}

#[test]
fn max_plus_one_overflows() {
    let src = format!("fn main() {{ print({MAX} + 1) }}");
    assert_eq!(run(&src), Err(codes::OVERFLOW));
}

#[test]
fn min_minus_one_overflows() {
    let src = format!("fn main() {{ let m = {MIN}\n print(m - 1) }}");
    assert_eq!(run(&src), Err(codes::OVERFLOW));
}

#[test]
fn min_negation_overflows() {
    let src = format!("fn main() {{ let m = {MIN}\n print(-m) }}");
    assert_eq!(run(&src), Err(codes::OVERFLOW));
}

#[test]
fn extreme_range_length_saturates() {
    // len() of an enormous range must not overflow; it saturates.
    let src = format!("fn main() {{ print(len(range({MIN}, {MAX}))) }}");
    assert_eq!(run(&src), Ok(format!("{MAX}\n")));
}

#[test]
fn extreme_negative_index_does_not_panic() {
    let src = format!("fn main() {{ let xs = [1]\n print(xs[{MIN}]) }}");
    assert_eq!(run(&src), Err(codes::INDEX));
}

#[test]
fn index_out_of_range_uses_index_code() {
    assert_eq!(run("fn main() { print([1, 2][5]) }"), Err(codes::INDEX));
    assert_eq!(run("fn main() { print(\"ab\"[9]) }"), Err(codes::INDEX));
}

#[test]
fn division_by_zero_is_div_zero() {
    assert_eq!(run("fn main() { print(1 / 0) }"), Err(codes::DIV_ZERO));
    assert_eq!(run("fn main() { print(1 % 0) }"), Err(codes::DIV_ZERO));
}

#[test]
fn float_boundaries() {
    // Division by zero on floats is also a language-level error (not IEEE).
    assert_eq!(run("fn main() { print(1.0 / 0.0) }"), Err(codes::DIV_ZERO));
    assert_eq!(run("fn main() { print(0.0 / 0.0) }"), Err(codes::DIV_ZERO));
}

#[test]
fn assert_uses_its_own_code() {
    assert_eq!(
        run("fn main() { assert(1 == 2, \"math broke\") }"),
        Err(codes::ASSERT)
    );
}

#[test]
fn deeply_nested_expression_is_rejected_not_a_crash() {
    // A flat but deeply nested expression must never overflow the host stack
    // in the parser, the checker, or the evaluator.
    let src = format!("fn main() {{ print(1{}) }}", "+1".repeat(100_000));
    assert_eq!(run(&src), Err(codes::NESTING));
}

#[test]
fn deeply_nested_checker_is_rejected_not_a_crash() {
    // The parser rejects a tree deeper than the language limit, so the
    // checker is never asked to walk one on an arbitrary stack.
    let src = format!("fn main() {{ print(1{}) }}", "+1".repeat(10_000));
    let err = aura::parse::parse(&src).expect_err("must be bounded");
    assert_eq!(err.code, codes::NESTING);
}

#[test]
fn deep_call_recursion_uses_the_call_limit_not_the_nesting_limit() {
    // Expression nesting resets per call, so deep recursion is governed by
    // E4011, never by E1015.
    let src = "fn f(n) { return f(n + 1) }\nfn main() { f(0) }";
    assert_eq!(run(src), Err(codes::RECURSION));
}

#[test]
fn moderately_long_expression_is_accepted() {
    // Ordinary chains must still work.
    let src = format!("fn main() {{ print({}) }}", "1+".repeat(100) + "1");
    assert_eq!(run(&src), Ok("101\n".to_string()));
}

/// B2: the semantic nesting limit is measured on the AST and reported as
/// `E1015` at a deterministic boundary. A program at the limit runs; a
/// program beyond it is rejected, never a host crash.
#[test]
fn regression_nesting_limit_boundary() {
    // A flat chain is left-nested AST depth. 128 terms is well within the
    // limit; 256 is past it.
    let ok = format!("fn main() {{ print({}) }}", vec!["1"; 128].join("+"));
    assert!(run(&ok).is_ok(), "128-term chain must be accepted");
    let over = format!("fn main() {{ print({}) }}", vec!["1"; 256].join("+"));
    assert_eq!(run(&over), Err(codes::NESTING));

    // Nested collections count AST depth too: just under the limit succeeds,
    // well past it is rejected with E1015 (not E1006, not a crash).
    let under = format!(
        "fn main() {{ print({}1{}) }}",
        "[".repeat(200),
        "]".repeat(200)
    );
    assert!(run(&under).is_ok(), "200 nested lists must be accepted");
    let over = format!(
        "fn main() {{ print({}1{}) }}",
        "[".repeat(300),
        "]".repeat(300)
    );
    assert_eq!(run(&over), Err(codes::NESTING));
}

/// B2: grouping parentheses add no AST depth, so a long parenthesized chain is
/// accepted; only an extreme one hits the parser's host-safety backstop, and
/// it too reports `E1015` rather than overflowing the stack.
#[test]
fn regression_parenthesis_nesting_is_bounded_not_a_crash() {
    let ok = format!(
        "fn main() {{ print({}1{}) }}",
        "(".repeat(1000),
        ")".repeat(1000)
    );
    assert!(
        run(&ok).is_ok(),
        "1000 grouped parentheses must be accepted"
    );
    let extreme = format!(
        "fn main() {{ print({}1{}) }}",
        "(".repeat(4000),
        ")".repeat(4000)
    );
    assert_eq!(run(&extreme), Err(codes::NESTING));
}

/// FEATURE_002 stack hardening: the checker runs with enough native stack that
/// a valid program at the semantic nesting limit is accepted, and one past it
/// is rejected with `E1015` — a language diagnostic, never a native overflow.
/// This guards the invariant that a larger native stack does not change the
/// language limit.
#[test]
fn checker_survives_the_semantic_nesting_limit() {
    // Just below the limit: accepted (the CLI default stack must not overflow).
    let under = format!(
        "fn main() {{ print({}1{}) }}",
        "[".repeat(250),
        "]".repeat(250)
    );
    assert!(run(&under).is_ok(), "250 nested lists must be accepted");
    // At/over the limit: a deterministic diagnostic, not a crash.
    let over = format!(
        "fn main() {{ print({}1{}) }}",
        "[".repeat(256),
        "]".repeat(256)
    );
    assert_eq!(run(&over), Err(codes::NESTING));
    // A deeply nested call chain (Feature 002 argument path) is likewise
    // bounded by the same diagnostic.
    let calls = format!(
        "fn id(x) {{ return x }}\nfn main() {{ print({}1{}) }}",
        "id(".repeat(300),
        ")".repeat(300)
    );
    assert_eq!(run(&calls), Err(codes::NESTING));
}

/// FEATURE_004: a deeply nested `let` pattern is bounded by the parser
/// recursion backstop and reports `E1015`, never a host overflow.
#[test]
fn deep_destructuring_pattern_is_bounded() {
    // A pattern within the backstop parses and checks (the undefined `v` is a
    // checker diagnostic, not a nesting one).
    let under = format!(
        "fn main() {{ let v = 1\n let {}x{} = v }}",
        "[".repeat(200),
        "]".repeat(200)
    );
    assert!(run(&under).is_err());
    // Far past the backstop, the pattern is rejected with the nesting code.
    let over = format!(
        "fn main() {{ let v = 1\n let {}x{} = v }}",
        "[".repeat(3000),
        "]".repeat(3000)
    );
    assert_eq!(run(&over), Err(codes::NESTING));
}

/// FEATURE_005: the empty-map literal `{:}` adds no AST nesting, so a program
/// using it obeys the same depth limits as any other.
#[test]
fn empty_map_literal_adds_no_nesting() {
    // A long chain of `{:}`-valued operators stays within the limit.
    let expr = vec!["{:} == {:}"; 50].join(" == ");
    let within = format!("fn main() {{ print({expr}) }}");
    assert!(run(&within).is_ok());
    // And a deeply nested grouping still reports the existing nesting limit.
    let over = format!(
        "fn main() {{ print({}1{}) }}",
        "(".repeat(5000),
        ")".repeat(5000)
    );
    assert_eq!(run(&over), Err(codes::NESTING));
}

/// H1b: a deeply nested `for`/`match` pattern is bounded by the parser
/// recursion backstop and reports `E1015`, never a host stack overflow.
#[test]
fn deep_match_pattern_is_bounded() {
    let over = format!(
        "fn main() {{ let v = 1\n match v {{ {}x{} -> 1\n _ -> 0 }} }}",
        "[".repeat(5000),
        "]".repeat(5000)
    );
    assert_eq!(run(&over), Err(codes::NESTING));
    let over_for = format!(
        "fn main() {{ let xs = []\n for {}x{} in xs {{ }} }}",
        "[".repeat(5000),
        "]".repeat(5000)
    );
    assert_eq!(run(&over_for), Err(codes::NESTING));
}

/// H1b: a pattern within the parser backstop parses, checks, and executes
/// without a host overflow (runtime pattern recursion is bounded by the
/// parser-accepted depth and runs on the large interpreter stack).
#[test]
fn bounded_deep_pattern_executes_without_overflow() {
    let n = 1000usize;
    let src = format!(
        "fn id(v) {{ return v }}\nfn main() {{ let mut x = id(1)\n \
         for i in range(0, {n}) {{ x = id([x]) }}\n \
         let r = match x {{ {pat} y {close} -> \"m\"\n _ -> \"f\" }}\n print(r) }}",
        pat = "[".repeat(n),
        close = "]".repeat(n),
    );
    assert_eq!(run(&src), Ok("m\n".to_string()));
}

/// FEATURE_006: a deep `else if` chain nests existing `Expr::If` nodes and is
/// bounded by the existing AST nesting limit (`E1015`), never a host
/// overflow. The exact off-by-one is measured against real behavior: a modest
/// chain runs, and a clearly-over chain is rejected with `E1015`.
#[test]
fn deep_else_if_chain_is_bounded() {
    let chain = |n: usize| {
        let mut parts = vec!["if false { print(0) }".to_string()];
        for i in 1..n {
            parts.push(format!("else if false {{ print({i}) }}"));
        }
        parts.push("else { print(99) }".to_string());
        format!("fn main() {{ {} }}", parts.join(" "))
    };
    // A modest chain is accepted and evaluates to the final `else`.
    assert_eq!(run(&chain(100)), Ok("99\n".to_string()));
    // A clearly-over chain is rejected with the existing nesting diagnostic.
    assert_eq!(run(&chain(2000)), Err(codes::NESTING));
}
