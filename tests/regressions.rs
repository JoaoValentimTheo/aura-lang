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
        out("struct S { a: int }\nfn main() { print(S { a: 1 }.a) }"),
        "1\n"
    );
}

// ---------------------------------------------------------------------------
// FEATURE_001: static user-function argument checking (`LANGUAGE_SPEC.md` §6.5).
// ---------------------------------------------------------------------------

/// A directly resolved top-level function call checks its argument count at
/// check time, not only at runtime.
#[test]
fn static_user_fn_arity_is_checked() {
    let two = "fn add(a, b) { return a + b }\nfn main() { print(add(1, 2)) }";
    assert_eq!(out(two), "3\n");
    for bad in [
        "fn add(a, b) { return a + b }\nfn main() { print(add()) }",
        "fn add(a, b) { return a + b }\nfn main() { print(add(1)) }",
        "fn add(a, b) { return a + b }\nfn main() { print(add(1, 2, 3)) }",
    ] {
        assert_eq!(check(bad), Err(codes::TYPE_MISMATCH), "{bad}");
    }
    // Zero-parameter and one-parameter functions.
    assert_eq!(out("fn z() { return 7 }\nfn main() { print(z()) }"), "7\n");
    assert_eq!(
        check("fn z() { return 7 }\nfn main() { print(z(1)) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("fn o(a) { return a }\nfn main() { print(o()) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// An annotated parameter is checked against the argument's inferred type at a
/// directly resolved call.
#[test]
fn static_user_fn_annotated_argument_type_is_checked() {
    assert_eq!(
        out("fn f(a: int) { return a }\nfn main() { print(f(1)) }"),
        "1\n"
    );
    assert_eq!(
        out("fn f(a: float) { return a }\nfn main() { print(f(1.5)) }"),
        "1.5\n"
    );
    assert_eq!(
        out("fn f(a: bool) { return a }\nfn main() { print(f(true)) }"),
        "true\n"
    );
    for bad in [
        "fn f(a: int) { return a }\nfn main() { f(\"x\") }",
        "fn f(a: int) { return a }\nfn main() { f(1.5) }",
        "fn f(a: string) { return a }\nfn main() { f(1) }",
        "fn f(a: bool) { return a }\nfn main() { f(\"x\") }",
    ] {
        assert_eq!(check(bad), Err(codes::TYPE_MISMATCH), "{bad}");
    }
}

/// Only annotated parameters are constrained; unannotated ones accept any
/// value. Arity still applies.
#[test]
fn static_user_fn_unannotated_parameters_are_unconstrained() {
    let src = "fn f(a: int, b, c: string) { return c }\nfn main() { print(f(1, true, \"ok\"))\n print(f(1, [9], \"ok\")) }";
    assert_eq!(out(src), "ok\nok\n");
    // The unannotated middle parameter is never the reason for a rejection.
    assert_eq!(
        check("fn f(a: int, b, c: string) { return c }\nfn main() { f(\"x\", true, \"ok\") }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("fn f(a: int, b, c: string) { return c }\nfn main() { f(1, true, 4) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // Arity still applies when annotations are partial.
    assert_eq!(
        check("fn f(a: int, b) { return a }\nfn main() { f(1) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// An argument whose inferred type is `Unknown` is never rejected merely
/// because inference is incomplete.
#[test]
fn static_user_fn_unknown_argument_remains_permissive() {
    // `none` infers Unknown.
    assert_eq!(
        check("fn f(a: int) { return a }\nfn main() { f(none) }"),
        Ok(())
    );
    // An if-expression infers Unknown.
    assert_eq!(
        check("fn f(a: int) { return a }\nfn main() { f(if true { none } else { none }) }"),
        Ok(())
    );
    // A call to a function of undetermined return type infers Unknown.
    assert_eq!(
        check("fn g() { return none }\nfn f(a: int) { return a }\nfn main() { f(g()) }"),
        Ok(())
    );
    // `Unknown` does not suppress a proven mismatch on another argument.
    assert_eq!(
        check("fn f(a: int, b: int) { return a }\nfn main() { f(none, \"x\") }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// A local binding that shadows a declared function name is a callable value;
/// the declaration's signature MUST NOT be applied to it.
#[test]
fn shadowed_fn_name_does_not_apply_global_signature() {
    // Local `let` shadowing: the lambda accepts a string even though the
    // global `f` is annotated `int`.
    let local = "fn f(x: int) { return x }\nfn g() { let f = (a) -> a\n return f(\"hello\") }\nfn main() { print(g()) }";
    assert_eq!(check(local), Ok(()));
    assert_eq!(out(local), "hello\n");
    // Mutable local shadowing.
    let mutable = "fn f(x: int) { return x }\nfn g() { let mut f = (a) -> a\n f = (b) -> b\n return f(\"z\") }\nfn main() { print(g()) }";
    assert_eq!(check(mutable), Ok(()));
    assert_eq!(out(mutable), "z\n");
    // Parameter shadowing.
    let param = "fn f(x: int) { return x }\nfn g(f) { return f(\"hello\") }\nfn main() { print(g((a) -> a)) }";
    assert_eq!(check(param), Ok(()));
    assert_eq!(out(param), "hello\n");
    // Nested-block shadowing.
    let nested = "fn f(x: int) { return x }\nfn g() { let mut r = \"init\"\n { let f = (a) -> a\n r = f(\"hi\") }\n return r }\nfn main() { print(g()) }";
    assert_eq!(check(nested), Ok(()));
    assert_eq!(out(nested), "hi\n");
    // Without shadowing, the declaration's signature applies.
    assert_eq!(
        check("fn f(x: int) { return x }\nfn main() { f(\"hello\") }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// Top-level declarations are hoisted, so forward and mutually recursive calls
/// are checked against the callee's signature.
#[test]
fn forward_and_mutual_user_fn_calls_are_checked() {
    // Forward reference to a later declaration.
    assert_eq!(
        check("fn main() { later(\"wrong\") }\nfn later(x: int) { return x }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        out("fn main() { print(later(3)) }\nfn later(x: int) { return x }"),
        "3\n"
    );
    // Mutual recursion.
    assert_eq!(
        check(
            "fn a(x: int) { return b(\"wrong\") }\nfn b(y: int) { return y }\nfn main() { a(1) }"
        ),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        out("fn even(n: int) -> bool { if n == 0 { return true }\n return odd(n - 1) }\nfn odd(n: int) -> bool { if n == 0 { return false }\n return even(n - 1) }\nfn main() { print(even(4)) }"),
        "true\n"
    );
}

/// Calls that the checker cannot resolve to a specific declaration remain
/// runtime-authoritative: function values, closures, and unknown callables.
#[test]
fn dynamic_function_value_remains_runtime_checked() {
    // A closure binding is not checked against any declaration's signature.
    assert_eq!(
        check("fn main() { let f = (x) -> x\n f(\"anything\") }"),
        Ok(())
    );
    // Its arity is still enforced at runtime.
    assert_eq!(
        fail("fn main() { let f = (x) -> x\n f() }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        fail("fn main() { let f = (x) -> x\n f(1, 2, 3) }"),
        codes::TYPE_MISMATCH
    );
    // A function value passed as an argument is likewise dynamic.
    assert_eq!(
        check("fn call(g) { return g(\"x\") }\nfn main() { call((a) -> a) }"),
        Ok(())
    );
}

/// A directly resolved call inside a pipeline is checked with the normal call
/// rules, against the inserted first argument.
#[test]
fn pipeline_user_fn_call_is_checked() {
    let ok = "fn f(x: int, y: string) { return y }\nfn main() { print(42 |> f(\"ok\")) }";
    assert_eq!(check(ok), Ok(()));
    assert_eq!(out(ok), "ok\n");
    let bad = "fn f(x: int, y: string) { return y }\nfn main() { print(\"wrong\" |> f(\"ok\")) }";
    assert_eq!(check(bad), Err(codes::TYPE_MISMATCH));
    // Arity through the pipeline.
    assert_eq!(
        check("fn f(x: int, y: string) { return y }\nfn main() { 42 |> f() }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// A directly resolved call accepts a return value used in an annotated
/// binding, exercising the combined inference and call check.
#[test]
fn direct_call_return_type_flows() {
    assert_eq!(
        check("fn f(a: int) -> string { return \"h\" }\nlet x: int = f(1)"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        out("fn f(a: int) -> string { return \"h\" }\nfn main() { let x: string = f(1)\n print(x) }"),
        "h\n"
    );
}

/// A user function whose name matches a builtin takes precedence, exactly as
/// the runtime dispatches. Its own signature is checked; the builtin signature
/// is not applied to it. This preserves the pre-feature resolution order.
#[test]
fn user_fn_shadowing_a_builtin_name_uses_user_signature() {
    // `fn len` wins over the builtin at runtime and in the checker.
    assert_eq!(
        out("fn len(x) { return 99 }\nfn main() { print(len(5)) }"),
        "99\n"
    );
    assert_eq!(
        out("fn len(a, b) { return a + b }\nfn main() { print(len(1, 2)) }"),
        "3\n"
    );
    // Its annotated parameter is checked against the user function.
    assert_eq!(
        check("fn len(x: string) { return x }\nfn main() { len(5) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // Without a user function, the builtin still applies.
    assert_eq!(out("fn main() { print(len([1, 2, 3])) }"), "3\n");
    assert_eq!(check("fn main() { len(1) }"), Err(codes::TYPE_MISMATCH));
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

/// Structs and enums have no methods, so a method call on a known struct or
/// enum receiver is rejected by the checker, not only at runtime. `range`
/// exposes only `len`. (Red team: `check_method_call` skipped receivers whose
/// `type_class` was `None`, so these were accepted and failed at runtime.)
#[test]
fn method_on_struct_enum_and_range_is_checked() {
    // Struct: any method is invalid, explicit or no-paren.
    assert_eq!(
        check("struct S { len: int }\nfn main() { let s = S { len: 5 }\n s.len() }"),
        Err(codes::UNDEFINED)
    );
    assert_eq!(
        check("struct S { a: int }\nfn main() { S { a: 1 }.get(\"a\") }"),
        Err(codes::UNDEFINED)
    );
    // Enum: any method is invalid.
    assert_eq!(
        check("enum E { A }\nfn main() { A().foo() }"),
        Err(codes::UNDEFINED)
    );
    // Range: only `len` exists.
    assert_eq!(
        check("fn main() { range(0, 3).nope() }"),
        Err(codes::UNDEFINED)
    );
    assert_eq!(
        check("fn main() { range(0, 3).nope }"),
        Err(codes::UNDEFINED)
    );
    // Valid uses are unaffected.
    assert_eq!(out("fn main() { print(range(0, 3).len()) }"), "3\n");
    assert_eq!(out("fn main() { print(range(0, 3).len) }"), "3\n");
    assert_eq!(
        out("struct S { a: int }\nfn main() { print(S { a: 1 }.a) }"),
        "1\n"
    );
}

// ---------------------------------------------------------------------------
// FEATURE_002: named function arguments (`LANGUAGE_SPEC.md` §15.7).
// ---------------------------------------------------------------------------

/// Basic named and reordered calls bind by parameter name.
#[test]
fn named_arguments_bind_by_parameter_name() {
    assert_eq!(
        out("fn f(x: int) -> int { return x }\nfn main() { print(f(x: 1)) }"),
        "1\n"
    );
    // Reordering by name.
    assert_eq!(
        out(
            "fn f(a: int, b: int) -> int { return a * 10 + b }\nfn main() { print(f(b: 2, a: 1)) }"
        ),
        "12\n"
    );
    // Three parameters, fully reversed.
    assert_eq!(
        out("fn f(a, b, c) -> int { return a * 100 + b * 10 + c }\nfn main() { print(f(c: 3, b: 2, a: 1)) }"),
        "123\n"
    );
}

/// A positional argument followed by a named one binds each to the right
/// parameter.
#[test]
fn named_arguments_mixed_with_positional() {
    assert_eq!(
        out("fn f(a: int, b: int) -> int { return a * 10 + b }\nfn main() { print(f(1, b: 2)) }"),
        "12\n"
    );
    assert_eq!(
        out("fn f(a, b, c) -> int { return a * 100 + b * 10 + c }\nfn main() { print(f(1, c: 3, b: 2)) }"),
        "123\n"
    );
}

/// A positional argument after a named one is a parse error.
#[test]
fn named_arguments_positional_after_named_is_syntax_error() {
    assert_eq!(
        fail("fn f(a: int, b: int) -> int { return a }\nfn main() { f(a: 1, 2) }"),
        codes::EXPECTED
    );
}

/// An unknown parameter name is rejected statically.
#[test]
fn named_arguments_unknown_parameter_is_rejected() {
    assert_eq!(
        check("fn f(a: int) -> int { return a }\nfn main() { f(z: 1) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// A parameter given twice — positionally and by name, or twice by name — is
/// rejected statically.
#[test]
fn named_arguments_duplicate_is_rejected() {
    assert_eq!(
        check("fn f(a: int, b: int) -> int { return a }\nfn main() { f(1, a: 2) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("fn f(a: int) -> int { return a }\nfn main() { f(a: 1, a: 2) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// A declared parameter left unsatisfied is rejected statically.
#[test]
fn named_arguments_missing_parameter_is_rejected() {
    assert_eq!(
        check("fn f(a: int, b: int, c: int) -> int { return a }\nfn main() { f(a: 1, c: 3) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("fn f(a: int, b: int) -> int { return a }\nfn main() { f(a: 1) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// Named arguments are type-checked after mapping, and `Unknown` remains
/// permissive.
#[test]
fn named_arguments_type_checking() {
    assert_eq!(
        check("fn f(a: int, b: int) -> int { return a + b }\nfn main() { f(b: 1, a: \"x\") }"),
        Err(codes::TYPE_MISMATCH)
    );
    // Unknown is permissive.
    assert_eq!(
        check("fn f(x: int) -> int { return x }\nfn main() { f(x: none) }"),
        Ok(())
    );
}

/// Named arguments are limited to directly resolved top-level functions.
/// Shadowed locals, dynamic callables, builtins, and methods reject them.
#[test]
fn named_arguments_only_for_direct_user_functions() {
    // Builtin.
    assert_eq!(
        check("fn main() { len(value: [1]) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // Method.
    assert_eq!(
        check("fn main() { \"a,b\".split(sep: \",\") }"),
        Err(codes::TYPE_MISMATCH)
    );
    // Closure value.
    assert_eq!(
        check("fn main() { let f = (a) -> a\n f(x: 1) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // Shadowing: the global signature must not be applied to the local.
    assert_eq!(
        check("fn f(x: int) -> int { return x }\nfn g() { let f = (a) -> a\n return f(x: 1) }\nfn main() { g() }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// Hoisting and mutual recursion work with named arguments.
#[test]
fn named_arguments_hoisting_and_mutual_recursion() {
    assert_eq!(
        out("fn main() { print(later(x: 5)) }\nfn later(x: int) -> int { return x }"),
        "5\n"
    );
    assert_eq!(
        out("fn even(n: int) -> bool { if n == 0 { return true }\n return odd(n: n - 1) }\nfn odd(n: int) -> bool { if n == 0 { return false }\n return even(n: n - 1) }\nfn main() { print(even(n: 4)) }"),
        "true\n"
    );
}

/// The pipeline receiver is the first positional argument; a named argument
/// naming that parameter is a duplicate.
#[test]
fn named_arguments_pipeline() {
    assert_eq!(
        out("fn move(dx: int, dy: int) -> int { return dx * 10 + dy }\nfn main() { print(5 |> move(dy: 2)) }"),
        "52\n"
    );
    assert_eq!(
        check("fn move(dx: int, dy: int) -> int { return dx }\nfn main() { 5 |> move(dx: 2) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// Evaluation order stays source order and is independent of parameter
/// binding; each argument is evaluated exactly once.
#[test]
fn named_arguments_evaluate_in_source_order_once() {
    let src = r#"
fn record(tag, v) {
    print(tag)
    return v
}
fn combine(second: int, first: int) -> int {
    return first * 10 + second
}
fn main() {
    print(combine(second: record("s1", 1), first: record("s2", 2)))
}
"#;
    // s1 then s2 (source order), bound second<-1, first<-2 -> 2*10+1 = 21.
    assert_eq!(out(src), "s1\ns2\n21\n");
}

/// Transparent aliases work as parameter types in named calls.
#[test]
fn named_arguments_alias_parameter_types() {
    assert_eq!(
        out("type Id = int\nfn f(x: Id) -> int { return x }\nfn main() { print(f(x: 5)) }"),
        "5\n"
    );
    assert_eq!(
        check("type Id = int\nfn f(x: Id) -> int { return x }\nfn main() { f(x: \"s\") }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// Positional calls are unchanged.
#[test]
fn named_arguments_positional_calls_unchanged() {
    assert_eq!(
        out("fn f(a: int, b: int) -> int { return a * 10 + b }\nfn main() { print(f(1, 2)) }"),
        "12\n"
    );
}

// ---------------------------------------------------------------------------
// FEATURE_003: static field-type propagation (`LANGUAGE_SPEC.md` §17.5).
// ---------------------------------------------------------------------------

/// A field read on a known struct infers the declared field type, so an
/// annotated binding is checked against it.
#[test]
fn field_read_infers_declared_type_for_primitives() {
    // int, string, bool all propagate.
    assert_eq!(
        out("struct P { x: int }\nfn main() { let p = P { x: 1 }\n let y: int = p.x\n print(y) }"),
        "1\n"
    );
    assert_eq!(
        out("struct P { s: string }\nfn main() { let p = P { s: \"h\" }\n let y: string = p.s\n print(y) }"),
        "h\n"
    );
    assert_eq!(
        out("struct P { b: bool }\nfn main() { let p = P { b: true }\n let y: bool = p.b\n print(y) }"),
        "true\n"
    );
    // Mismatches are `E3001` at check time.
    assert_eq!(
        check("struct P { x: int }\nfn main() { let p = P { x: 1 }\n let y: string = p.x }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("struct P { s: string }\nfn main() { let p = P { s: \"h\" }\n let y: int = p.s }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("struct P { b: bool }\nfn main() { let p = P { b: true }\n let y: int = p.b }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// A field whose annotation is a transparent alias propagates the resolved
/// type.
#[test]
fn field_read_resolves_transparent_aliases() {
    assert_eq!(
        out("type Id = int\nstruct P { id: Id }\nfn main() { let p = P { id: 1 }\n let y: int = p.id\n print(y) }"),
        "1\n"
    );
    assert_eq!(
        check("type Id = int\nstruct P { id: Id }\nfn main() { let p = P { id: 1 }\n let y: string = p.id }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// Nested field reads propagate through repeated application of the same rule.
#[test]
fn field_read_propagates_through_nested_structs() {
    let src = "struct Address { zip: int }\nstruct User { address: Address }\nfn main() { let u = User { address: Address { zip: 7 } }\n let y: int = u.address.zip\n print(y) }";
    assert_eq!(out(src), "7\n");
    let bad = "struct Address { zip: int }\nstruct User { address: Address }\nfn main() { let u = User { address: Address { zip: 7 } }\n let y: string = u.address.zip }";
    assert_eq!(check(bad), Err(codes::TYPE_MISMATCH));
}

/// The propagated field type flows into Feature 001's argument checks.
#[test]
fn field_read_strengthens_function_argument_checking() {
    // Correct field type accepted.
    assert_eq!(
        out("struct P { x: int }\nfn f(a: int) { return a }\nfn main() { let p = P { x: 1 }\n print(f(p.x)) }"),
        "1\n"
    );
    // Incompatible field type rejected through the existing Feature 001 path.
    assert_eq!(
        check("struct P { x: int }\nfn f(a: string) { return a }\nfn main() { let p = P { x: 1 }\n f(p.x) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// The propagated field type flows into Feature 002's named-argument checks.
#[test]
fn field_read_strengthens_named_argument_checking() {
    assert_eq!(
        check("struct P { x: int }\nfn f(a: int, b: string) { return b }\nfn main() { let p = P { x: 1 }\n f(b: p.x, a: 1) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        out("struct P { x: int }\nfn f(a: int, b: int) { return a + b }\nfn main() { let p = P { x: 1 }\n print(f(b: p.x, a: 2)) }"),
        "3\n"
    );
}

/// The propagated field type flows into return, construction, and ordering
/// checks without any feature-specific handling.
#[test]
fn field_read_flows_into_return_construction_and_ordering() {
    // Return type.
    assert_eq!(
        check(
            "struct P { x: int }\nfn g(p: P) -> string { return p.x }\nfn main() { g(P { x: 1 }) }"
        ),
        Err(codes::RETURN_MISMATCH)
    );
    // Struct construction field value.
    assert_eq!(
        check("struct Q { s: string }\nstruct P { x: int }\nfn main() { let p = P { x: 1 }\n Q { s: p.x } }"),
        Err(codes::TYPE_MISMATCH)
    );
    // Ordering still works and is checked.
    assert_eq!(
        out("struct P { x: int }\nfn main() { let p = P { x: 1 }\n print(p.x < 10) }"),
        "true\n"
    );
}

/// Field inference is strictly conservative: a receiver the checker cannot
/// prove to be a struct keeps its field read `Unknown`, so no speculative
/// rejection happens. The runtime remains authoritative.
#[test]
fn field_read_is_conservative_for_unproven_receivers() {
    // An unannotated parameter is `Unknown`; `u.x` stays `Unknown`,
    // so an incompatible annotation is accepted statically.
    assert_eq!(
        check("struct P { x: int }\nfn g(u) { let y: string = u.x }"),
        Ok(())
    );
    // A list element read infers `Unknown`.
    assert_eq!(
        check("struct P { x: int }\nfn main() { let y: string = [P { x: 1 }][0].x }"),
        Ok(())
    );
    // A map lookup infers `Unknown`.
    assert_eq!(
        check("struct P { x: int }\nfn main() { let m = {\"k\": P { x: 1 }}\n let y: string = m[\"k\"].x }"),
        Ok(())
    );
    // A branch result infers `Unknown`.
    assert_eq!(
        check("struct P { x: int }\nfn main() { let p = if true { P { x: 1 } } else { P { x: 2 } }\n let y: string = p.x }"),
        Ok(())
    );
    // A function call whose return type is not declared infers `Unknown`.
    assert_eq!(
        check("struct P { x: int }\nfn mk() { return P { x: 1 } }\nfn main() { let y: string = mk().x }"),
        Ok(())
    );
}

/// Field validity and field type inference are separate; a missing field does
/// not become a valid `Unknown` expression, and the existing field diagnostic
/// is unchanged.
#[test]
fn field_read_missing_field_keeps_existing_behavior() {
    // Reading a missing field on a known struct: the checker does not fabricate
    // a type, and the runtime reports it. (Read-side field errors are runtime,
    // unchanged by Feature 003.)
    assert_eq!(
        fail("struct S { a: int }\nfn main() { let s = S { a: 1 }\n print(s.b) }"),
        codes::UNDEFINED
    );
    // Writing a missing field on a known struct is still `E2003` at check time.
    assert_eq!(
        check("struct S { a: int }\nfn main() { let mut s = S { a: 1 }\n s.b = 2 }"),
        Err(codes::UNDEFINED)
    );
}

/// A struct declared after the read (hoisting) propagates, and a field read
/// inside a recursive function propagates.
#[test]
fn field_read_hoisting_and_recursion() {
    assert_eq!(
        out("fn main() { let p = P { x: 5 }\n let y: int = p.x\n print(y) }\nstruct P { x: int }"),
        "5\n"
    );
    assert_eq!(
        out("struct N { v: int }\nfn sum(n: N) -> int { return n.v }\nfn main() { print(sum(N { v: 3 })) }"),
        "3\n"
    );
}

// ---------------------------------------------------------------------------
// FEATURE_004: destructuring `let` integration.
// ---------------------------------------------------------------------------

/// Destructured names are `Unknown`, so Feature 001 argument checking does not
/// falsely reject them.
#[test]
fn destructuring_names_do_not_trip_feature_001() {
    assert_eq!(
        check("fn f(a: int) { return a }\nfn main() { let [x] = [\"s\"]\n f(x) }"),
        Ok(())
    );
}

/// Feature 002 named arguments are unaffected by destructuring.
#[test]
fn destructuring_does_not_change_feature_002() {
    assert_eq!(
        out("fn f(a: int, b: int) { return a + b }\nfn main() { let [x, y] = [1, 2]\n print(f(b: y, a: x)) }"),
        "3\n"
    );
}

/// Feature 003 field inference is unchanged; a field read on a destructured
/// name stays `Unknown`.
#[test]
fn destructuring_names_do_not_trip_feature_003() {
    assert_eq!(
        check("struct S { x: int }\nfn main() { let [s] = [S { x: 1 }]\n let y: string = s.x }"),
        Ok(())
    );
    // A direct field read on a known struct still propagates (Feature 003).
    assert_eq!(
        check("struct S { x: int }\nfn main() { let s = S { x: 1 }\n let y: string = s.x }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// Pipeline behavior is unchanged when the initializer is a pipeline.
#[test]
fn destructuring_with_pipeline_initializer() {
    assert_eq!(
        out("fn pair(a) { return [a, a] }\nfn main() { let [x, y] = 5 |> pair\n print(x + y) }"),
        "10\n"
    );
}

/// Ordinary `let`, `for`, and `match` are unaffected.
#[test]
fn destructuring_preserves_ordinary_constructs() {
    assert_eq!(
        out("fn main() { let x = 1\n let mut y = 2\n y = y + x\n print(y) }"),
        "3\n"
    );
    assert_eq!(
        out("fn main() { for [a, b] in [[1, 2], [3, 4]] { print(a + b) } }"),
        "3\n7\n"
    );
    assert_eq!(
        out("fn main() { print(match [1, 2] { [a, b] -> a + b }) }"),
        "3\n"
    );
}

// ---------------------------------------------------------------------------
// 0.0.1 post-release hardening audit (H2)
//
// These lock down defects found by the adversarial audit. Each test names the
// subsystem and the smallest reproducer.
// ---------------------------------------------------------------------------

/// H2-01: a `return` (or `throw`/`break`/`continue`) raised while computing a
/// `let` initializer or an assignment RHS propagates out of the statement
/// unchanged, instead of being converted to `E4030`/`E4026`. `LANGUAGE_SPEC.md`
/// §14.4 and the H2 control-flow table require this; before the fix the
/// value-position handler at the `let` and assignment sites dropped the signal.
#[test]
fn h2_01_return_in_initializer_propagates_from_the_function() {
    assert_eq!(
        out("fn f() -> int { let x = if true { return 7 } else { 0 }\n return x }\nfn main() { print(f()) }"),
        "7\n"
    );
    // The same for an assignment RHS.
    assert_eq!(
        out("fn f() -> int { let mut x = 0\n x = if true { return 5 } else { 1 }\n return x }\nfn main() { print(f()) }"),
        "5\n"
    );
}

/// H2-02: a `throw` in a `let` initializer is catchable by an enclosing
/// `try`, exactly as in any other expression position.
#[test]
fn h2_02_throw_in_initializer_is_catchable() {
    assert_eq!(
        out("fn main() { try { let v = if true { throw \"x\" } else { 0 }\n print(\"no\") } catch e -> { print(\"caught \" + e) } }"),
        "caught x\n"
    );
    // A `throw` in a loop header (`while` condition / `for` iterable) is also
    // catchable.
    assert_eq!(
        out("fn main() { try { while if true { throw \"w\" } else { false } {} } catch e -> { print(\"caught \" + e) } }"),
        "caught w\n"
    );
    assert_eq!(
        out("fn main() { try { for x in if true { throw \"fo\" } else { [1] } {} } catch e -> { print(\"caught \" + e) } }"),
        "caught fo\n"
    );
    // And in an assignment target's subexpressions.
    assert_eq!(
        out("fn main() { let mut xs = [1, 2]\n try { xs[if true { throw \"i\" } else { 0 }] = 9 } catch e -> { print(\"caught \" + e) } }"),
        "caught i\n"
    );
}

/// H2-03: `E4030` is still reachable, but only where a `return` genuinely
/// escapes to a value position with no enclosing function (top level).
#[test]
fn h2_03_return_position_is_reachable_at_top_level() {
    assert_eq!(
        fail("let x = if true { return 1 } else { 2 }"),
        codes::RETURN_POSITION
    );
}

/// H2-04: `E4099` is an internal call-boundary signal and MUST NOT reach the
/// user (`LANGUAGE_SPEC.md` §34.3). An uncaught `throw` crossing a call
/// boundary reports `E4026` on every entry point.
#[test]
fn h2_04_internal_throw_signal_never_reaches_the_user() {
    assert_eq!(fail("fn f() { throw \"x\" }\nf()"), codes::FOREIGN);
    assert_eq!(fail("fn f() { throw \"x\" }\nlet v = f()"), codes::FOREIGN);
    // Via the explicit module-mode entry point used by `aura eval`.
    let err = aura::run_toplevel_with("fn f() { throw \"x\" }\nf()", "<t>", Vec::new(), None)
        .expect_err("uncaught throw");
    assert_eq!(err.code, codes::FOREIGN);
}

/// H2-05: a `return` inside a lambda returns from the lambda, not from the
/// enclosing function. The enclosing function's return annotation MUST NOT be
/// applied to the lambda body (`LANGUAGE_SPEC.md` §15.4).
#[test]
fn h2_05_lambda_return_is_not_checked_against_the_enclosing_function() {
    // The lambda's `return "s"` is not an `int`, but it is not the enclosing
    // function's return value, so the program is valid and prints 3.
    assert_eq!(
        out(
            "fn f() -> int { let g = () -> { return \"s\" }\n return 3 }\nfn main() { print(f()) }"
        ),
        "3\n"
    );
    // The lambda still works as a value.
    assert_eq!(
        out("fn f() -> int { let g = () -> { return 9 }\n return g() }\nfn main() { print(f()) }"),
        "9\n"
    );
}

/// H2-06: runtime method arity is validated against the shared registry, so a
/// call the checker could not resolve (an `Unknown` receiver) is still
/// rejected instead of silently ignoring extra arguments (`LANGUAGE_SPEC.md`
/// §24, §15.7).
#[test]
fn h2_06_runtime_method_arity_is_enforced() {
    assert_eq!(
        fail("fn f(x) { return x.len(1, 2, 3) }\nfn main() { print(f([10, 20])) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        fail("fn f(x) { return x.upper(1, 2) }\nfn main() { print(f(\"hi\")) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        fail("fn f(x) { return x.pop(5) }\nfn main() { print(f([1, 2])) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        fail("fn f(x) { return x.get(\"a\", 9, 9) }\nfn main() { print(f({\"a\": 1})) }"),
        codes::TYPE_MISMATCH
    );
    // A correct call still succeeds.
    assert_eq!(
        out("fn f(x) { return x.len() }\nfn main() { print(f([10, 20])) }"),
        "2\n"
    );
}

/// H2-07: the dead `up`/`down` runtime aliases are removed, so they are not
/// reachable through an `Unknown` receiver. `x.up` is an unknown method on a
/// string at runtime (`E2003`), matching the checker (`LANGUAGE_SPEC.md`
/// §34.3, frozen decision "one spelling per construct").
#[test]
fn h2_07_dead_method_aliases_are_unreachable() {
    assert_eq!(
        fail("fn f(x) { return x.up }\nfn main() { print(f(\"hi\")) }"),
        codes::UNDEFINED
    );
    assert_eq!(
        fail("fn f(x) { return x.down }\nfn main() { print(f(\"HI\")) }"),
        codes::UNDEFINED
    );
    assert_eq!(
        out("fn f(x) { return x.upper }\nfn main() { print(f(\"hi\")) }"),
        "HI\n"
    );
}

/// H2-08: every builtin's arity is enforced at runtime, including when the
/// builtin is referenced as a first-class value (`let f = json_encode`) and
/// therefore evades the checker's static builtin-call validation. Before the
/// fix the `json_*`, `regex_*`, and `time_*` natives never consulted the
/// registry, so `f(1, 2, 3)` silently ignored the extra arguments, violating
/// `LANGUAGE_SPEC.md` §25 ("also enforced at runtime").
#[test]
fn h2_08_first_class_builtin_arity_is_enforced() {
    assert_eq!(
        fail("fn main() { let f = json_encode\n print(f(1, 2, 3)) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        fail("fn main() { let f = json_decode\n print(f(\"null\", \"x\")) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        fail("fn main() { let f = regex_match\n print(f(\"a\", \"b\", \"c\")) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        fail("fn main() { let f = time_unix\n print(f(1)) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        fail("fn main() { let f = sleep_ms\n print(f(1, 2)) }"),
        codes::TYPE_MISMATCH
    );
    // The correct call still succeeds through the same path.
    assert_eq!(
        out("fn main() { let f = json_encode\n print(f(1)) }"),
        "1\n"
    );
    assert_eq!(
        out("fn main() { let f = regex_match\n print(f(\"a\", \"abc\")) }"),
        "true\n"
    );
}
