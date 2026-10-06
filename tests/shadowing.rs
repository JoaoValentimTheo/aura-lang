#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Variable shadowing (`LANGUAGE_SPEC.md` §16.3).
//!
//! `let` and `let mut` create a *new* binding. A later `let`/`let mut` of the
//! same name in the same scope shadows the earlier binding: the initializer
//! resolves against the bindings visible before the declaration, `mut` belongs
//! to the newly created binding, and existing closures keep the binding they
//! captured. `const` is a declaration, not a variable binding, and does **not**
//! shadow: a duplicate constant remains `E2007`.

use aura::error::codes;
use aura::run_source;

/// Run a program that must succeed and return its stdout.
fn ok(src: &str) -> String {
    run_source(src, "<shadow>").expect("expected the program to run")
}

/// The first diagnostic code for a rejected program.
fn code(src: &str) -> u16 {
    run_source(src, "<shadow>")
        .expect_err("expected the program to be rejected")
        .code
}

// ------------------------------------------------ 1. same-scope shadowing

#[test]
fn same_scope_immutable_shadowing() {
    assert_eq!(ok("fn main() { let x = 1\n let x = 2\n print(x) }"), "2\n");
}

#[test]
fn mutable_shadowed_by_immutable() {
    assert_eq!(
        ok("fn main() { let mut x = 11\n let x = 20\n print(x) }"),
        "20\n"
    );
}

#[test]
fn immutable_shadowed_by_mutable() {
    assert_eq!(
        ok("fn main() { let x = 11\n let mut x = 20\n x = 30\n print(x) }"),
        "30\n"
    );
}

#[test]
fn mutable_shadowed_by_mutable() {
    assert_eq!(
        ok("fn main() { let mut x = 11\n let mut x = 20\n x = 30\n print(x) }"),
        "30\n"
    );
}

// ------------------------------------------- 2. initializer resolution

#[test]
fn initializer_reads_the_previous_binding() {
    assert_eq!(
        ok("fn main() { let x = 11\n let x = x + 10\n print(x) }"),
        "21\n"
    );
    assert_eq!(
        ok("fn main() { let mut x = 11\n let x = x + 10\n print(x) }"),
        "21\n"
    );
    assert_eq!(
        ok("fn main() { let mut x = 11\n let mut x = x + 10\n x = x + 1\n print(x) }"),
        "22\n"
    );
}

#[test]
fn initializer_does_not_self_reference() {
    // The new binding must not be visible to its own initializer.
    assert_eq!(
        ok("fn main() { let x = 1\n let x = [x, x + 1, x + 2]\n print(x) }"),
        "[1, 2, 3]\n"
    );
}

// ------------------------------------------------- 3. nested scopes

#[test]
fn nested_scope_shadowing_and_restoration() {
    assert_eq!(
        ok("fn main() { let x = 1\n { let x = 2\n print(x) }\n print(x) }"),
        "2\n1\n"
    );
    assert_eq!(
        ok(
            "fn main() { let x = 1\n { let x = 2\n { let x = 3\n print(x) }\n print(x) }\n print(x) }"
        ),
        "3\n2\n1\n"
    );
}

// ------------------------------------------- 4. shadowing + assignment

#[test]
fn shadow_is_not_assignment() {
    // Assigning to the shadowing immutable binding is `E2001`; the shadow
    // created a new immutable binding, not a mutation of the old one.
    assert_eq!(
        code("fn main() { let mut x = 10\n let x = 20\n x = 30 }"),
        codes::ASSIGN_IMMUTABLE
    );
    // The outer mutable binding is unaffected after the inner scope exits.
    assert_eq!(
        ok("fn main() { let mut x = 10\n { let _ = 20 }\n x = 30\n print(x) }"),
        "30\n"
    );
}

// ------------------------------------- 5. shadowing + mutation capability

#[test]
fn shadowing_shares_the_mutation_capability_rule() {
    // The inner shadow is immutable, so it cannot be mutated.
    assert_eq!(
        code("fn main() { let mut xs = [1]\n { let xs = [2]\n xs.push(3) } }"),
        codes::ASSIGN_IMMUTABLE
    );
    // The outer mutable binding still can be, after the scope exits.
    assert_eq!(
        ok("fn main() { let mut xs = [1]\n { let _ = [2] }\n xs.push(3)\n print(xs) }"),
        "[1, 3]\n"
    );
    // A shadowing `let mut` is itself mutable.
    assert_eq!(
        ok("fn main() { let xs = [1]\n let mut xs = xs\n xs.push(2)\n print(xs) }"),
        "[1, 2]\n"
    );
}

// ---------------------------------------------- 6. shadowing + closures

#[test]
fn closures_keep_the_binding_they_captured() {
    // `f` captured the first `x`; a later shadow does not change what it sees.
    assert_eq!(
        ok("fn main() { let x = 10\n let f = () -> x\n let x = 20\n print(f())\n print(x) }"),
        "10\n20\n"
    );
}

#[test]
fn nested_scope_closures_see_their_own_binding() {
    assert_eq!(
        ok(
            "fn main() { let x = 10\n let f = () -> x\n { let x = 20\n let g = () -> x\n print(f())\n print(g()) } }"
        ),
        "10\n20\n"
    );
}

// ---------------------------------------------- 7. shadowing + patterns

#[test]
fn destructuring_let_shadows_like_a_simple_let() {
    assert_eq!(
        ok("fn main() { let x = 1\n let [x, y] = [10, 20]\n print(x)\n print(y) }"),
        "10\n20\n"
    );
    assert_eq!(
        ok("fn main() { let [a, b] = [1, 2]\n let a = 9\n print(a)\n print(b) }"),
        "9\n2\n"
    );
}

// ---------------------------------- 8. parameters and non-variable scopes

#[test]
fn parameters_are_not_shadowed_by_a_duplicate_parameter() {
    // A repeated parameter name is still a redeclaration (`E2007`).
    assert_eq!(
        code("fn f(a, a) { return a }\nfn main() { }"),
        codes::REDECLARED
    );
    // But a `let` in the body may shadow a parameter (ordinary variable rule).
    assert_eq!(
        ok("fn f(a) { let a = 2\n return a }\nfn main() { print(f(1)) }"),
        "2\n"
    );
}

#[test]
fn loop_variable_shadows_and_restores() {
    assert_eq!(
        ok("fn main() { let i = 99\n for i in range(0, 2) { print(i) }\n print(i) }"),
        "0\n1\n99\n"
    );
}

// ------------------------------------- 9. const is NOT shadowable

#[test]
fn const_is_not_shadowable() {
    // A duplicate constant is a redeclaration, not a shadow.
    assert_eq!(
        code("const X = 10\nconst X = 20\nfn main() { print(X) }"),
        codes::REDECLARED
    );
    // A top-level `let` for an existing constant is also a redeclaration.
    assert_eq!(
        code("const A = 1\nlet A = 2\nfn main() { print(A) }"),
        codes::REDECLARED
    );
    // And a constant for an existing top-level `let`.
    assert_eq!(
        code("let A = 1\nconst A = 2\nfn main() { print(A) }"),
        codes::REDECLARED
    );
}
