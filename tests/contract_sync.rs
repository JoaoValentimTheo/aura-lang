#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Language Strong Contract Synchronize (LSCS) regressions.
//!
//! These lock the corrections made while synchronizing the core contract:
//! the `const` declaration, f-string interpolation diagnostic spans, and the
//! value/type namespace split of session declarations.

use aura::check::Checker;
use aura::error::codes;
use aura::parse::parse;
use aura::run_source;

/// Run a program that must succeed and return its stdout.
fn ok(src: &str) -> String {
    run_source(src, "<lscs>").expect("expected the program to run")
}

/// The first diagnostic code for a rejected program.
fn code(src: &str) -> u16 {
    run_source(src, "<lscs>")
        .expect_err("expected the program to be rejected")
        .code
}

// ------------------------------------------------------------------ `const`

/// `const NAME = expr` declares an immutable module constant, usable from a
/// function and by later constants.
#[test]
fn const_declares_a_module_constant() {
    assert_eq!(ok("const PI = 3\nfn main() { print(PI) }"), "3\n");
    assert_eq!(
        ok("const A = 1\nconst B = A + 1\nfn main() { print(B) }"),
        "2\n"
    );
}

/// A constant accepts a type annotation, which is checked.
#[test]
fn const_accepts_a_checked_annotation() {
    assert_eq!(ok("const N: int = 5\nfn main() { print(N) }"), "5\n");
    assert_eq!(
        code("const N: int = \"x\"\nfn main() { print(N) }"),
        codes::TYPE_MISMATCH
    );
}

/// A constant name MUST begin with an uppercase letter, so a constant
/// declaration does not read like an ordinary binding.
#[test]
fn const_name_must_be_uppercase() {
    assert_eq!(
        code("const pi = 3\nfn main() { print(pi) }"),
        codes::EXPECTED
    );
}

/// A constant is immutable: assignment is the ordinary immutable-binding
/// diagnostic.
#[test]
fn const_cannot_be_reassigned() {
    assert_eq!(
        code("const X = 1\nfn main() { X = 2 }"),
        codes::ASSIGN_IMMUTABLE
    );
}

/// A constant without an initializer is the `let`-without-initializer error.
#[test]
fn const_requires_an_initializer() {
    assert_eq!(code("const X\nfn main() { print(1) }"), codes::LET_NO_INIT);
}

/// Constants are evaluated in source order, so a forward reference is
/// undefined and a duplicate is a redeclaration.
#[test]
fn const_source_order_and_duplicates() {
    assert_eq!(
        code("const A = B\nconst B = 1\nfn main() { print(A) }"),
        codes::UNDEFINED
    );
    assert_eq!(
        code("const A = 1\nconst A = 2\nfn main() { print(A) }"),
        codes::REDECLARED
    );
}

/// `const` is contextual: it remains an ordinary identifier, and the existing
/// top-level `let` (also a module constant) is unchanged.
#[test]
fn const_is_contextual_and_let_is_unchanged() {
    assert_eq!(ok("fn main() { let const = 1\n print(const) }"), "1\n");
    assert_eq!(ok("let X = 7\nfn main() { print(X) }"), "7\n");
    assert_eq!(
        code("const A = 1\nlet A = 2\nfn main() { print(A) }"),
        codes::REDECLARED
    );
}

// ------------------------------------------------------- f-string locations

/// A name used inside an f-string interpolation reports its real source
/// location, not byte zero.
#[test]
fn fstring_interpolation_diagnostics_point_at_the_name() {
    let src = "fn main() {\n  print(f\"value {nope} end\")\n}\n";
    // Parsing succeeds; the checker rejects. The diagnostic span must sit
    // inside the interpolation, not at the start of the file.
    let d = run_source(src, "f.aura").expect_err("undefined name");
    assert_eq!(d.code, codes::UNDEFINED);
    assert!(
        d.span.start >= src.find("nope").expect("name present"),
        "span {:?} should point at the interpolation",
        d.span
    );
}

/// An empty interpolation reports a span at the braces.
#[test]
fn fstring_empty_interpolation_has_a_local_span() {
    let src = "fn main() {\n  print(f\"x{ }y\")\n}\n";
    let d = run_source(src, "f.aura").expect_err("empty interpolation");
    assert_eq!(d.code, codes::EXPECTED);
    assert!(d.span.start > 0, "span should not be the start of file");
}

// ---------------------------------------------------------- namespace split

/// A function and a struct may share a name (separate value/type namespaces),
/// exactly as they do in a module: neither declaration is a redeclaration, and
/// the struct remains constructible.
#[test]
fn function_and_struct_names_are_separate_namespaces() {
    // The struct is still usable after the name is also a function.
    assert_eq!(
        ok("fn S() { return 1 }\nstruct S { a: int }\nfn main() { print(S { a: 5 }.a) }"),
        "5\n"
    );
    // Declaring both in either order is accepted (no E2007/E2012).
    let m =
        parse("struct S { a: int }\nfn S() { return 1 }\nfn main() { print(1) }").expect("parse");
    assert_eq!(Checker::module_with_main(&m).map_err(|d| d.code), Ok(()));
}

/// A constant and a struct may share a name too; two constants may not.
#[test]
fn constant_and_struct_names_are_separate_namespaces() {
    assert_eq!(
        ok("const S = 1\nstruct S { a: int }\nfn main() { print(S) }"),
        "1\n"
    );
    assert_eq!(
        code("const A = 1\nfn A() { return 1 }\nfn main() { print(1) }"),
        codes::REDECLARED
    );
}

/// Checker-level: `const` and top-level `let` occupy the same value namespace.
#[test]
fn const_and_let_share_the_value_namespace() {
    let m = parse("const A = 1\nlet A = 2\nfn main() { print(A) }").expect("parse");
    assert_eq!(
        Checker::module(&m).map_err(|d| d.code),
        Err(codes::REDECLARED)
    );
}
