#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Function/method overloading and the unified lambda parameter model
//! (`LANGUAGE_SPEC.md` §15.4, §15.7, §17.7).
//!
//! Overload identity is the name plus the ordered input types. The return
//! type is never part of identity, and neither is `mut self`. Resolution is
//! deterministic: the most specific viable overload wins, an exact match beats
//! a merely compatible one, an annotated parameter beats an unannotated one,
//! and a genuine tie is reported as ambiguous rather than guessed.

use aura::error::codes;
use aura::run_source;

/// Run a program that must succeed and return its stdout.
fn ok(src: &str) -> String {
    run_source(src, "<overload>").expect("expected the program to run")
}

/// The first diagnostic code for a rejected program.
fn code(src: &str) -> u16 {
    run_source(src, "<overload>")
        .expect_err("expected the program to be rejected")
        .code
}

// -------------------------------------------------------------- functions

#[test]
fn two_function_overloads() {
    assert_eq!(
        ok("fn show(x: int) { print(\"i\") }\nfn show(x: string) { print(\"s\") }\nfn main() { show(1)\n show(\"a\") }"),
        "i\ns\n"
    );
}

#[test]
fn three_function_overloads() {
    assert_eq!(
        ok("fn f(x: int) { print(\"i\") }\nfn f(x: float) { print(\"f\") }\nfn f(x: string) { print(\"s\") }\nfn main() { f(1)\n f(1.5)\n f(\"a\") }"),
        "i\nf\ns\n"
    );
}

#[test]
fn overload_resolves_nested_calls() {
    assert_eq!(
        ok("fn f(x: int) -> int { return x + 1 }\nfn f(x: string) -> string { return x + \"!\" }\nfn g(x: int) -> int { return f(f(x)) }\nfn main() { print(g(1)) }"),
        "3\n"
    );
}

#[test]
fn overload_with_recursion() {
    assert_eq!(
        ok("fn f(n: int) -> int { if n <= 1 { return 1 }\n return n * f(n - 1) }\nfn f(n: string) -> string { return n }\nfn main() { print(f(5)) }"),
        "120\n"
    );
}

#[test]
fn annotated_overload_beats_unannotated() {
    // `fn f(x: int)` is more specific than `fn f(x)`; an `int` argument picks
    // the annotated overload, any other type falls to the unannotated one.
    assert_eq!(
        ok("fn f(x: int) { print(\"int\") }\nfn f(x) { print(\"any\") }\nfn main() { f(1)\n f(true) }"),
        "int\nany\n"
    );
}

// ------------------------------------------------------ overload identity

#[test]
fn duplicate_signature_is_rejected() {
    assert_eq!(
        code("fn f(x: int) { print(1) }\nfn f(x: int) { print(2) }\nfn main() { f(1) }"),
        codes::REDECLARED
    );
}

#[test]
fn return_type_does_not_distinguish_overloads() {
    assert_eq!(
        code("fn f(x: int) -> int { return 1 }\nfn f(x: int) -> string { return \"a\" }\nfn main() { }"),
        codes::REDECLARED
    );
}

#[test]
fn different_arity_is_a_different_overload() {
    assert_eq!(
        ok("fn f(x: int) { print(\"one\") }\nfn f(x: int, y: int) { print(\"two\") }\nfn main() { f(1)\n f(1, 2) }"),
        "one\ntwo\n"
    );
}

// ------------------------------------------------------------- resolution

#[test]
fn no_matching_overload_is_deterministic() {
    assert_eq!(
        code("fn f(x: int) { print(1) }\nfn main() { f(\"a\") }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn f(x: int) { print(1) }\nfn main() { f(1, 2) }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn ambiguous_overload_is_reported() {
    // Two unions that both accept an `int` are equally specific, so neither is
    // preferred: the call is reported ambiguous rather than picked by order.
    assert_eq!(
        code("fn f(x: int | string) { print(1) }\nfn f(x: int | bool) { print(2) }\nfn main() { f(1) }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn duplicate_unannotated_overload_is_a_duplicate_not_ambiguous() {
    // Two unannotated parameters share one identity (`None`), so this is a
    // duplicate declaration, checked before resolution.
    assert_eq!(
        code("fn f(x) { print(1) }\nfn f(y) { print(2) }\nfn main() { f(1) }"),
        codes::REDECLARED
    );
}

#[test]
fn union_overload_matches_a_member() {
    assert_eq!(
        ok("fn f(x: int | string) { print(\"u\") }\nfn f(x: bool) { print(\"b\") }\nfn main() { f(1)\n f(true) }"),
        "u\nb\n"
    );
}

#[test]
fn alias_overload_resolves_to_the_underlying_type() {
    assert_eq!(
        ok("struct S { n: int }\ntype A = S\nfn f(x: S) { print(\"s\") }\nfn f(x: int) { print(\"i\") }\nfn main() { let a: A = S { n: 0 }\n f(a)\n f(1) }"),
        "s\ni\n"
    );
}

// --------------------------------------------------------------- methods

#[test]
fn method_overloads_by_argument_type() {
    assert_eq!(
        ok("struct P { n: int }\nimpl P {\n fn greet(self, name: string) { print(\"hi \" + name) }\n fn greet(self, times: int) { print(self.n * times) }\n}\nfn main() { let p = P { n: 3 }\n p.greet(\"Ada\")\n p.greet(2) }"),
        "hi Ada\n6\n"
    );
}

#[test]
fn method_duplicate_signature_is_rejected() {
    assert_eq!(
        code("struct P { n: int }\nimpl P {\n fn f(self, x: int) { print(1) }\n fn f(self, x: int) { print(2) }\n}\nfn main() { P { n: 0 }.f(1) }"),
        codes::REDECLARED
    );
}

#[test]
fn method_return_type_does_not_distinguish_overloads() {
    assert_eq!(
        code("struct P { n: int }\nimpl P {\n fn f(self, x: int) -> int { return 1 }\n fn f(self, x: int) -> string { return \"a\" }\n}\nfn main() { }"),
        codes::REDECLARED
    );
}

#[test]
fn method_mut_receiver_is_not_an_overload_dimension() {
    // `mut self` difference alone must not create two overloads.
    assert_eq!(
        code("struct P { n: int }\nimpl P {\n fn f(self, x: int) { print(1) }\n fn f(mut self, x: int) { print(2) }\n}\nfn main() { }"),
        codes::REDECLARED
    );
}

#[test]
fn method_no_match_is_deterministic() {
    assert_eq!(
        code("struct P { n: int }\nimpl P {\n fn f(self, x: int) { print(1) }\n}\nfn main() { P { n: 0 }.f(\"a\") }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn separate_impl_blocks_extend_the_same_overload_set() {
    assert_eq!(
        ok("struct P { n: int }\nimpl P {\n fn f(self, x: int) { print(\"i\") }\n}\nimpl P {\n fn f(self, x: string) { print(\"s\") }\n}\nfn main() { P { n: 0 }.f(1)\n P { n: 0 }.f(\"a\") }"),
        "i\ns\n"
    );
}

#[test]
fn field_method_collision_still_applies() {
    assert_eq!(
        code("struct P { n: int }\nimpl P { fn n(self, x: int) { print(x) } }\nfn main() { }"),
        codes::DUPLICATE_FIELD
    );
}

// --------------------------------------------------------------- traits

#[test]
fn trait_method_is_not_overloadable_in_the_impl() {
    // A trait implementation must implement exactly the trait's methods: it
    // may not add a second overload of a trait method.
    assert_eq!(
        code("trait T { fn show(self, x: int) }\nstruct S { n: int }\nimpl T for S { fn show(self, x: int) { print(1) }\n fn show(self, x: string) { print(2) } }\nfn main() { }"),
        codes::REDECLARED
    );
}

#[test]
fn trait_and_inherent_overloads_share_one_namespace() {
    assert_eq!(
        ok("trait T { fn show(self, x: int) }\nstruct S { n: int }\nimpl T for S { fn show(self, x: int) { print(\"t\") } }\nimpl S { fn show(self, x: string) { print(\"i\") } }\nfn main() { S { n: 0 }.show(1)\n S { n: 0 }.show(\"a\") }"),
        "t\ni\n"
    );
}

// --------------------------------------------------------------- lambdas

#[test]
fn lambda_accepts_annotations_and_mut() {
    assert_eq!(
        ok("fn main() { let f = (x: int) -> x + 1\n print(f(1)) }"),
        "2\n"
    );
    assert_eq!(
        ok("fn main() { let f = (mut x) -> { x = x + 1\n return x }\n print(f(1)) }"),
        "2\n"
    );
    assert_eq!(ok("fn main() { let f = () -> 7\n print(f()) }"), "7\n");
}

#[test]
fn lambda_parameter_matches_the_function_model() {
    // The same `Param` model: name, annotation, `mut`.
    let e = aura::parse::parse_expr("(x: int, mut y) -> x").expect("parse");
    match e {
        aura::ast::Expr::Lambda(ps, _, _) => {
            assert_eq!(ps.len(), 2);
            assert!(ps[0].ty.is_some() && !ps[0].mutable);
            assert!(ps[1].ty.is_none() && ps[1].mutable);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn lambda_nested_and_curried() {
    assert_eq!(
        ok("fn main() { let f = (x: int) -> (y: int) -> x + y\n print(f(1)(2)) }"),
        "3\n"
    );
}

#[test]
fn lambda_captures_and_shadowing_are_preserved() {
    assert_eq!(
        ok("fn main() { let x = 10\n let f = () -> x\n let x = 20\n print(f())\n print(x) }"),
        "10\n20\n"
    );
}
