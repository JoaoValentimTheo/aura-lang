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

// ---------------------------------------------------------------------------
// B1–B6: findings from `docs/SEMANTIC_FREEZE_AUDIT.md`.
// ---------------------------------------------------------------------------

/// B1: `%` by zero is `E4007` on both int and float, including `-0.0`; valid
/// non-zero remainder is unchanged. Previously float `%` yielded `nan`.
#[test]
fn b1_float_remainder_by_zero_is_an_error() {
    // Every zero spelling is rejected for both operand types.
    assert_eq!(fail("fn main() { print(1 % 0) }"), codes::DIV_ZERO);
    assert_eq!(fail("fn main() { print(1.0 % 0.0) }"), codes::DIV_ZERO);
    assert_eq!(fail("fn main() { print(-1.0 % 0.0) }"), codes::DIV_ZERO);
    assert_eq!(fail("fn main() { print(1.0 % -0.0) }"), codes::DIV_ZERO);
    assert_eq!(fail("fn main() { print(-1.0 % -0.0) }"), codes::DIV_ZERO);
    assert_eq!(fail("fn main() { print(0.0 % 0.0) }"), codes::DIV_ZERO);
    // Division behaves the same way.
    assert_eq!(fail("fn main() { print(1 / 0) }"), codes::DIV_ZERO);
    assert_eq!(fail("fn main() { print(1.0 / 0.0) }"), codes::DIV_ZERO);
    // The non-zero remainder path is untouched, including sign semantics.
    assert_eq!(out("fn main() { print(7 % 3) }"), "1\n");
    assert_eq!(out("fn main() { print(7.5 % 2.0) }"), "1.5\n");
    assert_eq!(out("fn main() { print(-7.5 % 2.0) }"), "-1.5\n");
    assert_eq!(out("fn main() { print(7.5 % -2.0) }"), "1.5\n");
}

/// B3: struct field annotations are checked at construction.
#[test]
fn b3_struct_field_type_validation() {
    let bad_string = "struct S { a: int }\nfn main() { print(S { a: \"x\" }) }";
    assert_eq!(check(bad_string), Err(codes::TYPE_MISMATCH));
    assert_eq!(fail(bad_string), codes::TYPE_MISMATCH);
    let bad_list = "struct S { a: int }\nfn main() { print(S { a: [1] }) }";
    assert_eq!(check(bad_list), Err(codes::TYPE_MISMATCH));
    let bad_float = "struct S { a: int }\nfn main() { print(S { a: 1.5 }) }";
    assert_eq!(check(bad_float), Err(codes::TYPE_MISMATCH));
    // Correct types are accepted.
    assert_eq!(
        check("struct S { a: int }\nfn main() { S { a: 1 } }"),
        Ok(())
    );
    assert_eq!(
        out("struct S { a: float }\nfn main() { print(S { a: 1.5 }) }"),
        "S { a: 1.5 }\n"
    );
    assert_eq!(
        out("struct S { a: bool }\nfn main() { print(S { a: true }) }"),
        "S { a: true }\n"
    );
    // A value the checker cannot type is not rejected (conservative).
    assert_eq!(
        check("struct S { a: int }\nfn main() { let x = none\n S { a: x } }"),
        Ok(())
    );
}

/// B4: struct construction rejects unknown, missing, duplicate, and extra
/// fields instead of silently dropping them.
#[test]
fn b4_struct_unknown_and_missing_fields() {
    // Extra / unknown field: undefined field, before execution.
    let extra = "struct S { a: int }\nfn main() { print(S { a: 1, b: 2 }) }";
    assert_eq!(check(extra), Err(codes::UNDEFINED));
    assert_eq!(fail(extra), codes::UNDEFINED);
    // Missing required field.
    let missing = "struct S { a: int, b: int }\nfn main() { print(S { a: 1 }) }";
    assert_eq!(check(missing), Err(codes::TYPE_MISMATCH));
    assert_eq!(fail(missing), codes::TYPE_MISMATCH);
    // Duplicate field.
    let dup = "struct S { a: int }\nfn main() { print(S { a: 1, a: 2 }) }";
    assert_eq!(check(dup), Err(codes::TYPE_MISMATCH));
    // Positional arity.
    assert_eq!(
        check("struct S { a: int }\nfn main() { S(1, 2) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("struct S { a: int, b: int }\nfn main() { S(1) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A no-field struct literal is exact.
    assert_eq!(out("struct S { }\nfn main() { print(S { }) }"), "S {  }\n");
}

/// B6: enum variants are positional; named payload arguments are rejected
/// explicitly rather than surfacing as a confusing arity error.
#[test]
fn b6_enum_named_argument_contract() {
    let named = "enum E { A(int) }\nfn main() { print(match A(x: 1) { A(n) -> n }) }";
    assert_eq!(check(named), Err(codes::TYPE_MISMATCH));
    assert_eq!(fail(named), codes::TYPE_MISMATCH);
    // Positional construction works; arity and payload types are checked.
    assert_eq!(
        out("enum E { A(int) }\nfn main() { print(match A(1) { A(n) -> n }) }"),
        "1\n"
    );
    assert_eq!(
        check("enum E { A(int) }\nfn main() { A(1, 2) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("enum E { A(int) }\nfn main() { A(\"x\") }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A payload the checker cannot type is accepted.
    assert_eq!(
        check("enum E { A(int) }\nfn main() { let z = none\n A(z) }"),
        Ok(())
    );
}

/// A recursive type alias has no concrete target and must be rejected before
/// execution with `E3002`, never by recursing without bound. (Conformance
/// audit: the checker previously overflowed the host stack.)
#[test]
fn recursive_type_alias_is_rejected_not_a_crash() {
    assert_eq!(check("type A = A"), Err(codes::UNKNOWN_TYPE));
    assert_eq!(check("type A = B\ntype B = A"), Err(codes::UNKNOWN_TYPE));
    assert_eq!(
        check("type A = B\ntype B = C\ntype C = A"),
        Err(codes::UNKNOWN_TYPE)
    );
    // Cycles through compound positions are cycles too.
    assert_eq!(check("type A = [A]"), Err(codes::UNKNOWN_TYPE));
    assert_eq!(check("type A = {string: A}"), Err(codes::UNKNOWN_TYPE));
    assert_eq!(check("type A = A | none"), Err(codes::UNKNOWN_TYPE));
    // A non-cyclic chain still resolves.
    assert_eq!(
        out("type A = B\ntype B = int\nfn main() { let x: A = 1\n print(x) }"),
        "1\n"
    );
}

/// `receiver.name` without parentheses is a zero-argument method call, so an
/// unknown method on a known receiver must be rejected by the checker — not
/// only at runtime. (Conformance audit: the no-paren path bypassed the method
/// registry, making the dead `up`/`down` aliases reachable.)
#[test]
fn no_paren_method_exists_is_checked() {
    // Known receiver, unknown method: rejected before execution.
    assert_eq!(
        check("fn main() { let s = \"x\"\n s.nope }"),
        Err(codes::UNDEFINED)
    );
    assert_eq!(fail("fn main() { print(\"x\".nope) }"), codes::UNDEFINED);
    // The runtime-only aliases are not part of the language.
    assert_eq!(
        check("fn main() { let s = \"x\"\n s.up }"),
        Err(codes::UNDEFINED)
    );
    assert_eq!(
        check("fn main() { let s = \"X\"\n s.down }"),
        Err(codes::UNDEFINED)
    );
    // A valid no-paren zero-argument method still works.
    assert_eq!(out("fn main() { print(\"x\".upper) }"), "X\n");
    assert_eq!(out("fn main() { print([3, 1].sort) }"), "[1, 3]\n");
    // A struct receiver is a field read, not a method call.
    assert_eq!(
        out("struct S { a: int }\nfn main() { print(S { a: 1 }.a) }"),
        "1\n"
    );
}
