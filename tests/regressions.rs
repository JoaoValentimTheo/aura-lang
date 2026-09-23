#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Regression tests for the defects catalogued in
//! `docs/ARCHITECTURE_REVIEW.md`.
//!
//! Each test names the finding it locks down, so a future regression is
//! traceable to the review that demanded the fix.

use aura::check::Checker;
use aura::error::codes;
use aura::parse::parse;
use aura::run_source;

fn check(src: &str) -> Result<(), u16> {
    let module = parse(src).expect("parses");
    Checker::module(&module).map_err(|d| d.code)
}

fn out(src: &str) -> String {
    run_source(src, "<test>").expect("program runs")
}

fn fail(src: &str) -> u16 {
    run_source(src, "<test>")
        .expect_err("program is rejected")
        .code
}

/// F-01: adding two floats (or an int and a float) must not hit the internal
/// "non-arithmetic operator" arm. Before the fix this was a spurious `E4999`.
#[test]
fn f01_float_addition_is_not_an_internal_error() {
    assert_eq!(out("fn main() { print(1.5 + 2.5) }"), "4.0\n");
    assert_eq!(out("fn main() { print(1 + 2.5) }"), "3.5\n");
    assert_eq!(out("fn main() { print(2.5 + 1) }"), "3.5\n");
}

/// A table-driven arithmetic matrix: for each operator and each operand-type
/// pair, the result is either the documented value or the documented
/// diagnostic. This is the test that would have caught F-01.
#[test]
fn f01_arithmetic_matrix() {
    // (source, expected stdout) for valid combinations.
    let valid: &[(&str, &str)] = &[
        ("print(1 + 2)", "3\n"),
        ("print(1.5 + 0.5)", "2.0\n"),
        ("print(1 + 0.5)", "1.5\n"),
        ("print(0.5 + 1)", "1.5\n"),
        ("print(5 - 2)", "3\n"),
        ("print(5.5 - 0.5)", "5.0\n"),
        ("print(5 - 0.5)", "4.5\n"),
        ("print(2 * 3)", "6\n"),
        ("print(2.0 * 3.0)", "6.0\n"),
        ("print(2 * 3.0)", "6.0\n"),
        ("print(7 / 2)", "3\n"),
        ("print(7.0 / 2.0)", "3.5\n"),
        ("print(7 / 2.0)", "3.5\n"),
        ("print(7 % 3)", "1\n"),
        ("print(7.5 % 2.0)", "1.5\n"),
        ("print(2 ^ 10)", "1024\n"),
        ("print(2.0 ^ 3.0)", "8.0\n"),
        ("print(\"a\" + \"b\")", "ab\n"),
        ("print([1] + [2])", "[1, 2]\n"),
    ];
    for (expr, want) in valid {
        let src = format!("fn main() {{ {expr} }}");
        assert_eq!(out(&src), *want, "for `{expr}`");
    }

    // Invalid combinations: overflowing int arithmetic and mixed types that
    // are not numbers.
    let overflow = "fn main() { let mut x = 9223372036854775807\n print(x + 1) }";
    assert_eq!(fail(overflow), codes::OVERFLOW);
    let bad = "fn main() { print(\"a\" + 1) }";
    assert_eq!(fail(bad), codes::TYPE_MISMATCH);
}

/// F-02: a method that exists on no type is rejected by the checker, and a
/// method with the wrong arity is rejected too.
#[test]
fn f02_unknown_method_is_rejected_statically() {
    assert_eq!(check("fn main() { [1].nope() }"), Err(codes::UNDEFINED));
    // Arity is checked when the receiver type is known.
    assert_eq!(
        check("fn main() { let s = \"x\"\n s.upper(1) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A valid method still checks.
    assert_eq!(check("fn main() { let s = \"x\"\n s.upper() }"), Ok(()));
}

/// F-02: builtin arity and argument types are checked against one signature
/// table, so `len([1], [2])` is caught before execution.
#[test]
fn f02_builtin_arity_and_types_are_static() {
    assert_eq!(
        check("fn main() { len([1], [2]) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(check("fn main() { len(1) }"), Err(codes::TYPE_MISMATCH));
}

/// F-04: a function's declared return type flows into the checker, so a
/// provable mismatch is reported before execution.
#[test]
fn f04_return_type_propagates_to_annotated_binding() {
    let src = "fn f() -> string { return \"h\" }\nlet x: int = f()";
    assert_eq!(check(src), Err(codes::TYPE_MISMATCH));
    let ok = "fn f() -> string { return \"h\" }\nlet x: string = f()";
    assert_eq!(check(ok), Ok(()));
}

/// F-05: ordering a list is a type error, caught statically; ordering numbers,
/// strings, and bools is allowed.
#[test]
fn f05_orderability_is_checked_once() {
    assert_eq!(check("fn main() { [1] < [2] }"), Err(codes::TYPE_MISMATCH));
    assert_eq!(check("fn main() { 1 < 2.5 }"), Ok(()));
    assert_eq!(check("fn main() { \"a\" < \"b\" }"), Ok(()));
    assert_eq!(check("fn main() { true < false }"), Ok(()));
}

/// F-06: function equality is reflexive (`g == g`) and distinct functions are
/// not equal.
#[test]
fn f06_function_equality_is_identity() {
    let src = "fn g() -> int { return 1 }\nfn main() { print(g == g)\n print(g != g) }";
    assert_eq!(out(src), "true\nfalse\n");
}

/// F-10 (feature `py`): the Python boundary must reject lossy conversions.
#[cfg(feature = "py")]
#[test]
fn f10_python_boundary_rejects_lossy_values() {
    assert_eq!(fail("fn main() { py_eval(\"2**63\") }"), codes::OVERFLOW);
    assert_eq!(
        fail("fn main() { py_eval(\"{1: 1, '1': 2}\") }"),
        codes::PY_UNSUPPORTED
    );
}
