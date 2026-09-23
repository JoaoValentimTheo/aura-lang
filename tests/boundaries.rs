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
