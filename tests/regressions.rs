#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Regression tests for the defects catalogued in
//! `docs/archive/ARCHITECTURE_REVIEW.md`.
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
    let src = "fn f() -> string { return \"h\" }\nlet _: int = f()";
    assert_eq!(check(src), Err(codes::TYPE_MISMATCH));
    let ok = "fn f() -> string { return \"h\" }\nlet _: string = f()";
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
    // A tuple key is not key-capable (`string`/`int`/`bool` are) and must be
    // rejected rather than coerced. Since generic map keys, `1` and `"1"` are
    // distinct keys and are both accepted, so they are no longer a lossy case.
    assert_eq!(
        fail("fn main() { py_eval(\"{(1, 2): 'x'}\") }"),
        codes::PY_UNSUPPORTED
    );
}

// ---------------------------------------------------------------------------
// B1–B6: findings from `docs/archive/SEMANTIC_FREEZE_AUDIT.md`.
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
    // A definite `none` in a strict field is rejected (optionality is a real
    // relation); a value the checker genuinely cannot type stays conservative.
    assert_eq!(
        check("struct S { a: int }\nfn main() { let x = none\n S { a: x } }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("struct S { a: int }\nfn main() { let x = f_unknown()\n S { a: x } }\nfn f_unknown() { return none }"),
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
    // A definite `none` payload is rejected; a value the checker cannot type
    // stays accepted (the §2.3 unknown boundary).
    assert_eq!(
        check("enum E { A(int) }\nfn main() { let z = none\n A(z) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("enum E { A(int) }\nfn main() { let z = f_unknown()\n A(z) }\nfn f_unknown() { return none }"),
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
    let src = "fn f(_a: int, _b, c: string) { return c }\nfn main() { print(f(1, true, \"ok\"))\n print(f(1, [9], \"ok\")) }";
    assert_eq!(out(src), "ok\nok\n");
    // The unannotated middle parameter is never the reason for a rejection.
    assert_eq!(
        check("fn f(_a: int, _b, c: string) { return c }\nfn main() { f(\"x\", true, \"ok\") }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("fn f(_a: int, _b, c: string) { return c }\nfn main() { f(1, true, 4) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // Arity still applies when annotations are partial.
    assert_eq!(
        check("fn f(a: int, _a) { return a }\nfn main() { f(1) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// An argument whose inferred type is `Unknown` is never rejected merely
/// because inference is incomplete. A definite `none` is no longer `Unknown`
/// (Keystone optionality, §4.3/§5.2), so it is checked as `none`.
#[test]
fn static_user_fn_unknown_argument_remains_permissive() {
    // A definite `none` argument to a strict parameter is rejected.
    assert_eq!(
        check("fn f(a: int) { return a }\nfn main() { f(none) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // An if-expression joins its branches: both are `none`, so the argument is
    // `none` and a strict parameter rejects it. A join involving a genuinely
    // undecidable branch still infers `Unknown` (see the dynamic cases below).
    assert_eq!(
        check("fn f(a: int) { return a }\nfn main() { f(if true { none } else { none }) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("fn f(a: int) { return a }\nfn main() { f(if true { 1 } else { 2 }) }"),
        Ok(())
    );
    // A call to a function of undetermined return type infers Unknown.
    assert_eq!(
        check("fn g() { return none }\nfn f(a: int) { return a }\nfn main() { f(g()) }"),
        Ok(())
    );
    // `Unknown` does not suppress a proven mismatch on another argument.
    assert_eq!(
        check("fn f(a: int, _a: int) { return a }\nfn main() { f(none, \"x\") }"),
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
    let ok = "fn f(_a: int, y: string) { return y }\nfn main() { print(42 |> f(\"ok\")) }";
    assert_eq!(check(ok), Ok(()));
    assert_eq!(out(ok), "ok\n");
    let bad = "fn f(_a: int, y: string) { return y }\nfn main() { print(\"wrong\" |> f(\"ok\")) }";
    assert_eq!(check(bad), Err(codes::TYPE_MISMATCH));
    // Arity through the pipeline.
    assert_eq!(
        check("fn f(_a: int, y: string) { return y }\nfn main() { 42 |> f() }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// A directly resolved call accepts a return value used in an annotated
/// binding, exercising the combined inference and call check.
#[test]
fn direct_call_return_type_flows() {
    assert_eq!(
        check("fn f(_a: int) -> string { return \"h\" }\nlet x: int = f(1)"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        out("fn f(_a: int) -> string { return \"h\" }\nfn main() { let x: string = f(1)\n print(x) }"),
        "h\n"
    );
}

/// A user function must not use a builtin name (human law): `fn len` is
/// `E1009` (`RESERVED_NAME`), not a shadowing user signature. This supersedes
/// the pre-hardening shadowing behavior.
#[test]
fn user_fn_shadowing_a_builtin_name_uses_user_signature() {
    // `fn len` is reserved and cannot be declared as a user function.
    assert_eq!(
        fail("fn len(x) { return 99 }\nfn main() { print(len(5)) }"),
        codes::RESERVED_NAME
    );
    assert_eq!(
        fail("fn len(a, b) { return a + b }\nfn main() { print(len(1, 2)) }"),
        codes::RESERVED_NAME
    );
    // Its annotated parameter is never reached: the declaration itself is
    // rejected.
    assert_eq!(
        check("fn len(x: string) { return x }\nfn main() { len(5) }"),
        Err(codes::RESERVED_NAME)
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
        check("fn f(a: int, _a: int) -> int { return a }\nfn main() { f(1, a: 2) }"),
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
        check("fn f(a: int, _a: int, _b: int) -> int { return a }\nfn main() { f(a: 1, c: 3) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("fn f(a: int, _a: int) -> int { return a }\nfn main() { f(a: 1) }"),
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
    // A definite `none` is rejected; an unknown value stays permissive.
    assert_eq!(
        check("fn f(x: int) -> int { return x }\nfn main() { f(x: none) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("fn f(x: int) -> int { return x }\nfn g() { return none }\nfn main() { f(x: g()) }"),
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
        check("fn move(dx: int, _a: int) -> int { return dx }\nfn main() { 5 |> move(dx: 2) }"),
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
        check("struct P { x: int }\nfn f(_a: int, b: string) { return b }\nfn main() { let p = P { x: 1 }\n f(b: p.x, a: 1) }"),
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

/// Field inference stays conservative for genuinely unproven receivers, while
/// a proven collection element type is now propagated (LANGUAGE_SPEC §17): an
/// `Unknown` receiver keeps its field read `Unknown` (no speculative
/// rejection), but a known `[P]`/`{K: P}` element is `P`, so a provable field
/// mismatch is rejected. The runtime remains authoritative.
#[test]
fn field_read_is_conservative_for_unproven_receivers() {
    // An unannotated parameter is `Unknown`; `u.x` stays `Unknown`,
    // so an incompatible annotation is accepted statically.
    assert_eq!(
        check("struct P { x: int }\nfn g(u) { let _: string = u.x }"),
        Ok(())
    );
    // A list element read of a known element type now infers that type
    // (LANGUAGE_SPEC §17), so a provable mismatch is rejected rather than
    // hidden. `[P { x: 1 }][0]` is `P`, whose field `x` is `int`.
    assert_eq!(
        check("struct P { x: int }\nfn main() { let y: string = [P { x: 1 }][0].x }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A map lookup of a known value type infers that type likewise.
    assert_eq!(
        check("struct P { x: int }\nfn main() { let m = {\"k\": P { x: 1 }}\n let y: string = m[\"k\"].x }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A branch result joins its branches (§6.4, corrected): both arms are `P`,
    // so the join is `P` and the annotation mismatch is caught.
    assert_eq!(
        check("struct P { x: int }\nfn main() { let p = if true { P { x: 1 } } else { P { x: 2 } }\n let _: string = p.x }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A function call whose return type is not declared infers `Unknown`.
    assert_eq!(
        check("struct P { x: int }\nfn mk() { return P { x: 1 } }\nfn main() { let _: string = mk().x }"),
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
        out("struct N { v: int }\nfn total(n: N) -> int { return n.v }\nfn main() { print(total(N { v: 3 })) }"),
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
        check("struct S { x: int }\nfn main() { let [s] = [S { x: 1 }]\n let _: string = s.x }"),
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
        out("fn main() { try { let _ = if true { throw \"x\" } else { 0 }\n print(\"no\") } catch e { print(\"caught \" + e) } }"),
        "caught x\n"
    );
    // A `throw` in a loop header (`while` condition / `for` iterable) is also
    // catchable.
    assert_eq!(
        out("fn main() { try { while if true { throw \"w\" } else { false } {} } catch e { print(\"caught \" + e) } }"),
        "caught w\n"
    );
    assert_eq!(
        out("fn main() { try { for _ in if true { throw \"fo\" } else { [1] } {} } catch e { print(\"caught \" + e) } }"),
        "caught fo\n"
    );
    // And in an assignment target's subexpressions.
    assert_eq!(
        out("fn main() { let mut xs = [1, 2]\n try { xs[if true { throw \"i\" } else { 0 }] = 9 } catch e { print(\"caught \" + e) } }"),
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
            "fn f() -> int { let _ = () -> { return \"s\" }\n return 3 }\nfn main() { print(f()) }"
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
#[cfg(feature = "json")]
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

// ---------------------------------------------------------------------------
// 0.0.2 pre-infrastructure bug hunt (BH1)
//
// A cyclic or pathologically deep runtime value must not crash the host.
// `LANGUAGE_SPEC.md` §31.5: "No syntactically valid, well-formed program may
// cause a host panic, stack overflow, or undefined behavior." Before the fix,
// display, structural equality, JSON encoding, and value teardown all
// recursed over the structure and aborted the process.
// ---------------------------------------------------------------------------

/// BH1-01: a cyclic list is constructible (`a.push(a)`) and must display,
/// compare, and encode without overflowing the native stack.
#[cfg(feature = "json")]
#[test]
fn bh1_01_cyclic_list_operations_do_not_crash() {
    // Display terminates (truncated) instead of aborting.
    let shown = out("fn main() { let mut a = []\n a.push(a)\n print(a) }");
    assert!(shown.starts_with('['), "unexpected display: {shown}");
    // Reflexive equality holds; two structurally identical cycles are equal
    // under the coinductive reading (`LANGUAGE_SPEC.md` §31.6).
    assert_eq!(
        out("fn main() { let mut a = []\n a.push(a)\n print(a == a) }"),
        "true\n"
    );
    assert_eq!(
        out(
            "fn main() { let mut a = []\n a.push(a)\n let mut b = []\n b.push(b)\n print(a == b) }"
        ),
        "true\n"
    );
    // A differing element makes distinct cycles unequal.
    assert_eq!(
        out("fn main() { let mut a = []\n a.push(a)\n a.push(1)\n let mut b = []\n b.push(b)\n b.push(2)\n print(a == b) }"),
        "false\n"
    );
    // JSON encoding terminates.
    let json = out("fn main() { let mut a = []\n a.push(a)\n print(json_encode(a)) }");
    assert!(json.starts_with('['), "unexpected json: {json}");
}

/// BH1-02: a cyclic map and a cycle crossing list<->map must not crash.
#[test]
fn bh1_02_cyclic_map_operations_do_not_crash() {
    let shown = out("fn main() { let mut m = {:}\n m[\"s\"] = m\n print(m) }");
    assert!(shown.starts_with('{'), "unexpected display: {shown}");
    assert_eq!(
        out("fn main() { let mut m = {:}\n m[\"s\"] = m\n print(m == m) }"),
        "true\n"
    );
    assert_eq!(
        out("fn main() { let mut a = []\n let b = [a]\n a.push(b)\n print(a == a) }"),
        "true\n"
    );
}

/// BH1-03: a deeply nested value built at runtime must not crash display,
/// equality, JSON encoding, or teardown — even well past the depth at which
/// naive recursion overflowed the 64 MiB interpreter stack.
#[cfg(feature = "json")]
#[test]
fn bh1_03_deep_runtime_value_operations_do_not_crash() {
    let build =
        "let a = []\n let mut c = a\n for _i in range(0, 40000) { let n = []\n c.push(n)\n c = n }";
    // Display (truncated) and equality both terminate.
    let shown = out(&format!("fn main() {{ {build}\n print(a) }}"));
    assert!(shown.starts_with('['), "unexpected display");
    assert_eq!(
        out(&format!("fn main() {{ {build}\n print(a == a) }}")),
        "true\n"
    );
    // JSON encoding terminates.
    let json = out(&format!("fn main() {{ {build}\n print(json_encode(a)) }}"));
    assert!(json.starts_with('['), "unexpected json");
    // Teardown of the deep value does not overflow (this is the statement
    // whose mere execution is the test).
    assert_eq!(
        out(&format!("fn main() {{ {build}\n print(\"ok\") }}")),
        "ok\n"
    );
}

/// BH1-04: a recursive enum built to depth must not crash on teardown.
#[test]
fn bh1_04_deep_recursive_enum_does_not_crash() {
    let src = "enum E { N(int), S(E) }\nfn main() { let mut a = N(1)\n for _ in range(0, 40000) { a = S(a) }\n print(\"ok\") }";
    assert_eq!(out(src), "ok\n");
}

/// BH1-05: shallow values keep their exact prior semantics — the depth guard
/// is a host-safety bound, not a value-model change.
#[cfg(feature = "json")]
#[test]
fn bh1_05_shallow_value_semantics_are_unchanged() {
    assert_eq!(
        out("fn main() { print([1, [2, 3], {\"k\": true}]) }"),
        "[1, [2, 3], {\"k\": true}]\n"
    );
    assert_eq!(out("fn main() { print([1, 2] == [1, 2]) }"), "true\n");
    assert_eq!(out("fn main() { print([1, 2] == [1, 3]) }"), "false\n");
    assert_eq!(
        out("struct P { x: int }\nfn main() { print(P(1) == P(1)) }"),
        "true\n"
    );
    assert_eq!(
        out("fn main() { print(json_encode({\"a\": [1, 2], \"b\": true})) }"),
        "{\"a\":[1,2],\"b\":true}\n"
    );
    assert_eq!(
        out("fn main() { print(json_encode(json_decode(\"{\\\"a\\\": 1}\"))) }"),
        "{\"a\":1}\n"
    );
}

/// BH1-06: `receiver.name` without parentheses on a known enum is a
/// zero-argument method call; since enums have no methods, it must be
/// `E2003` at check time exactly as the parenthesized form is
/// (`LANGUAGE_SPEC.md` §24). Before the fix the checker accepted it and the
/// runtime rejected it, so the diagnostic phase diverged.
#[test]
fn bh1_06_no_paren_member_on_known_enum_is_checked() {
    assert_eq!(
        check("enum E { A }\nfn f(e: E) { return e.foo }\nfn main() { print(f(A())) }"),
        Err(codes::UNDEFINED)
    );
    assert_eq!(
        check("enum E { A }\nfn f(e: E) { return e.len }\nfn main() { print(f(A())) }"),
        Err(codes::UNDEFINED)
    );
    // A struct receiver stays a field read, so a missing field is still
    // runtime-authoritative (§17.5) and a present field still works.
    assert_eq!(
        check("struct P { x: int }\nfn f(p: P) { return p.nope }\nfn main() { f(P(1)) }"),
        Ok(())
    );
    assert_eq!(
        out("struct P { x: int }\nfn f(p: P) { return p.x }\nfn main() { print(f(P(1))) }"),
        "1\n"
    );
}

// ---------------------------------------------------------------------------
//
// AUDIT-GEN-01: an f-string format precision above `u16::MAX` used to reach
// Rust's `format!` and panic ("Formatting argument out of range") on native,
// which became `E4999` (INTERNAL), and trap (`unreachable`) on WebAssembly.
// `LANGUAGE_SPEC.md` §31.5 forbids a host panic on a valid program. The
// precision is now bounded by the language and reported as a stable `E4013`.
// ---------------------------------------------------------------------------

/// A huge f-string precision is a stable `E4013` on every substrate, never a
/// panic. `65535` (`u16::MAX`) is the largest the formatter accepts and must
/// still succeed; `65536` and beyond must be rejected.
#[test]
fn audit_precision_above_u16_max_is_bounded() {
    // At the boundary: accepted.
    let ok = out("fn main() { print(len(f\"{1.5:.65535}\")) }");
    assert_eq!(ok.trim(), "65537");
    // Just past it: a diagnostic, not a panic.
    assert_eq!(
        fail("fn main() { print(f\"{1.5:.65536}\") }"),
        codes::OVERFLOW
    );
    assert_eq!(
        fail("fn main() { print(f\"{1.5:.100000}\") }"),
        codes::OVERFLOW
    );
    // The bound applies to every float presentation type, not only the plain
    // form.
    assert_eq!(
        fail("fn main() { print(f\"{1.5:.70000f}\") }"),
        codes::OVERFLOW
    );
    assert_eq!(
        fail("fn main() { print(f\"{1:.70000e}\") }"),
        codes::OVERFLOW
    );
}

// ---------------------------------------------------------------------------
//
// AUDIT-GEN-02: an f-string format *width* was unbounded. A very large width
// padded with `fill`, so native attempted a multi-gigabyte allocation while
// WebAssembly trapped on memory exhaustion — a native/WASM divergence on a
// valid program. The width is now bounded by the language and reported as
// `E4013` beyond the bound, matching the range-materialization cap.
// ---------------------------------------------------------------------------

/// A huge f-string width is a stable `E4013` rather than an allocation or a
/// trap, and the bound still admits a large-but-bounded field.
#[test]
fn audit_huge_format_width_is_bounded() {
    // At the bound: accepted.
    let size = out("fn main() { print(len(f\"{1:10000000}\")) }");
    assert_eq!(size.trim(), "10000000");
    // Beyond it: a diagnostic.
    assert_eq!(
        fail("fn main() { print(f\"{1:10000001}\") }"),
        codes::OVERFLOW
    );
    assert_eq!(
        fail("fn main() { print(f\"{1:2000000000}\") }"),
        codes::OVERFLOW
    );
}

// ---------------------------------------------------------------------------
//
// AUDIT-GEN-03: a type annotation is NOT counted by the semantic AST-node
// walker (`enforce_depth` descends expressions and statements only), so type
// nesting is governed solely by the substrate-calibrated parser backstop
// (§31.2). The invariant that MUST hold is §31.5: no depth causes a host
// failure. The *acceptance threshold* is substrate-dependent (native ~2047,
// WebAssembly ~767) and is recorded as a divergence finding for a spec
// decision rather than silently changed here.
// ---------------------------------------------------------------------------

/// A type annotation nested past the semantic limit is either accepted or
/// reported as `E1015`; it never panics, aborts, or overflows the host stack,
/// at any depth. This is the §31.5 invariant that type nesting must satisfy.
#[test]
fn audit_deep_type_annotation_never_host_fails() {
    fn nested(n: usize) -> String {
        let mut t = "int".to_string();
        for _ in 0..n {
            t = format!("Box<{t}>");
        }
        t
    }
    for depth in [300usize, 1000, 2047, 2048, 5000, 20000] {
        let src = format!(
            "struct Box<T> {{ value: T }}\nfn f(x: {}) -> int {{ return 1 }}\nfn main() {{ print(1) }}",
            nested(depth)
        );
        match run_source(&src, "<test>") {
            // Accepted (below this substrate's backstop), or a deterministic
            // nesting diagnostic. Either is safe; a panic would surface as
            // `E4999` and is what this test forbids.
            Ok(_) => {}
            Err(d) => assert_eq!(
                d.code,
                codes::NESTING,
                "deep type depth {depth} produced an unexpected code"
            ),
        }
    }
}

// ---------------------------------------------------------------------------
//
// AUDIT-4: the display and JSON encoders bounded recursion *depth* but not the
// *total number of nodes visited*. A value with reference fan-out of two or
// more — a cycle reachable from more than one position, or a shared subvalue —
// is re-traversed at every occurrence, so the render expands exponentially and
// does not terminate. `push(c, c)` twice on an empty list, or `x = [x, x]`
// repeated, hangs BOTH substrates at the host level (no diagnostic, no trap,
// no return), violating LANGUAGE_SPEC.md §31.5 (no host failure) and §31.6
// (display/JSON terminate). Fixed with a shared total-node render budget
// (`MAX_VALUE_NODES`) alongside the existing depth bound.
// ---------------------------------------------------------------------------

/// AUDIT-4: a list that contains itself twice must display and JSON-encode in
/// bounded time, eliding the remainder exactly like the depth bound.
#[cfg(feature = "json")]
#[test]
fn audit4_multi_reference_cycle_terminates() {
    // The minimal reproducer: one list, pushed onto itself twice.
    let shown = out("fn main() { let mut c = []\n push(c, c)\n push(c, c)\n print(c) }");
    assert!(shown.starts_with('['), "unexpected display: {shown}");
    assert!(shown.contains('…'), "expected elision, got: {shown}");
    // `to_string` takes the same path.
    let s =
        out("fn main() { let mut c = []\n push(c, c)\n push(c, c)\n print(len(to_string(c))) }");
    assert!(
        s.trim().parse::<usize>().is_ok(),
        "expected a length, got: {s}"
    );
    // JSON encoding terminates too.
    let json =
        out("fn main() { let mut c = []\n push(c, c)\n push(c, c)\n print(len(json_encode(c))) }");
    assert!(
        json.trim().parse::<usize>().is_ok(),
        "expected a json length, got: {json}"
    );
}

/// AUDIT-4: a shared (DAG) value with exponential unfolding must render in
/// bounded time even without a cycle.
#[test]
fn audit4_shared_subvalue_render_is_bounded() {
    // `grow([1], n)` builds `[x, x]` n times; the display would be 2^n nodes if
    // every occurrence were traversed. It must terminate with elision.
    let src = "fn grow(xs, n) {\n let mut cur = xs\n let mut i = 0\n while i < n { cur = [cur, cur]\n i = i + 1 }\n return cur\n}\nfn main() { print(len(to_string(grow([1], 40)))) }";
    let n = out(src);
    assert!(
        n.trim().parse::<usize>().is_ok(),
        "expected a bounded length, got: {n}"
    );
}

/// AUDIT-4: the node bound is a rendering guard only — shallow values keep
/// their exact prior semantics.
#[test]
fn audit4_shallow_semantics_unchanged() {
    assert_eq!(
        out("fn main() { print([1, [2, 3], {\"k\": true}]) }"),
        "[1, [2, 3], {\"k\": true}]\n"
    );
    assert_eq!(
        out("fn main() { let mut a = []\n push(a, a)\n print(a == a) }"),
        "true\n"
    );
}
