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

// -------------------------------------------------------------- bitwise ops

/// The bitwise operators (`&`, `|`, `~`, `<<`, `>>`) operate on integers.
#[test]
fn bitwise_operators() {
    assert_eq!(ok("fn main() { print(6 & 3) }"), "2\n");
    assert_eq!(ok("fn main() { print(6 | 1) }"), "7\n");
    assert_eq!(ok("fn main() { print(~0) }"), "-1\n");
    assert_eq!(ok("fn main() { print(1 << 4) }"), "16\n");
    assert_eq!(ok("fn main() { print(256 >> 4) }"), "16\n");
}

/// `&` binds tighter than `|`, and shift binds tighter than additive.
#[test]
fn bitwise_precedence() {
    // `1 | (2 & 3)` == 3
    assert_eq!(ok("fn main() { print(1 | 2 & 3) }"), "3\n");
    // `1 << (2 + 1)` == 8, since shift is *looser* than additive here: the
    // additive runs first. This mirrors Python's `<<`/`+` ordering.
    assert_eq!(ok("fn main() { print(1 << 2 + 1) }"), "8\n");
}

/// `^` remains exponentiation, not XOR: Aura has no XOR operator.
#[test]
fn caret_is_power_not_xor() {
    assert_eq!(ok("fn main() { print(2 ^ 10) }"), "1024\n");
}

/// Bitwise operators require integers.
#[test]
fn bitwise_requires_integers() {
    assert_eq!(code("fn main() { print(1.0 & 2) }"), codes::TYPE_MISMATCH);
    assert_eq!(code("fn main() { print(~true) }"), codes::TYPE_MISMATCH);
}

/// A shift count that is negative or too large is a runtime error, never a
/// host panic.
#[test]
fn shift_count_is_checked() {
    assert_eq!(code("fn main() { print(1 << -1) }"), codes::OVERFLOW);
    assert_eq!(code("fn main() { print(1 << 64) }"), codes::OVERFLOW);
}

/// The bitwise and shift compound assignments exist alongside the arithmetic
/// ones.
#[test]
fn compound_assignments() {
    assert_eq!(ok("fn main() { let mut x = 6\n x |= 1\n print(x) }"), "7\n");
    assert_eq!(ok("fn main() { let mut x = 6\n x &= 3\n print(x) }"), "2\n");
    assert_eq!(
        ok("fn main() { let mut x = 1\n x <<= 3\n print(x) }"),
        "8\n"
    );
    assert_eq!(
        ok("fn main() { let mut x = 16\n x >>= 2\n print(x) }"),
        "4\n"
    );
    assert_eq!(ok("fn main() { let mut x = 7\n x %= 3\n print(x) }"), "1\n");
    assert_eq!(ok("fn main() { let mut x = 2\n x ^= 3\n print(x) }"), "8\n");
}

/// `|` is contextual: a type-union separator in type position, bitwise OR in
/// expression position.
#[test]
fn bar_is_union_in_types_and_or_in_expressions() {
    assert_eq!(
        ok("struct A { x: int }\nstruct B { y: int }\ntype U = A | B\nfn main() { let u: U = A { x: 1 }\n print(u.x) }"),
        "1\n"
    );
    assert_eq!(ok("fn main() { print(1 | 2) }"), "3\n");
}

// ------------------------------------------------------- f-string formatting

/// f-strings interpolate, escape braces, and support a small, orthogonal
/// format mini-language.
#[test]
fn fstring_format_mini_language() {
    // Interpolation and escapes (unchanged).
    assert_eq!(ok("fn main() { let x = 5\n print(f\"v={x}\") }"), "v=5\n");
    assert_eq!(
        ok("fn main() { print(f\"{{literal}} {1 + 1}\") }"),
        "{literal} 2\n"
    );
    // Precision.
    assert_eq!(ok("fn main() { print(f\"{3.14159:.2f}\") }"), "3.14\n");
    // Width, alignment, and fill.
    assert_eq!(ok("fn main() { print(f\"[{42:>6}]\") }"), "[    42]\n");
    assert_eq!(ok("fn main() { print(f\"[{'hi':<6}]\") }"), "[hi    ]\n");
    assert_eq!(ok("fn main() { print(f\"[{'hi':^6}]\") }"), "[  hi  ]\n");
    assert_eq!(ok("fn main() { print(f\"[{'hi':*>7}]\") }"), "[*****hi]\n");
    // Zero padding.
    assert_eq!(ok("fn main() { print(f\"{42:06d}\") }"), "000042\n");
    // Sign.
    assert_eq!(ok("fn main() { print(f\"{5:+}\") }"), "+5\n");
    // Presentation types.
    assert_eq!(ok("fn main() { print(f\"{255:x} {255:X}\") }"), "ff FF\n");
    assert_eq!(ok("fn main() { print(f\"{10:b} {10:o}\") }"), "1010 12\n");
    assert_eq!(ok("fn main() { print(f\"{0.5:.1%}\") }"), "50.0%\n");
    assert_eq!(ok("fn main() { print(f\"{1234.5:.2e}\") }"), "1.23e3\n");
    // Width and precision together.
    assert_eq!(
        ok("fn main() { print(f\"[{3.14159:10.2f}]\") }"),
        "[      3.14]\n"
    );
}

/// A format type that does not apply to the value is a type error, and an
/// unknown format type is a parse error.
#[test]
fn fstring_format_diagnostics() {
    assert_eq!(
        code("fn main() { print(f\"{'s':d}\") }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(code("fn main() { print(f\"{5:z}\") }"), codes::EXPECTED);
}

// --------------------------------------------- deliberately absent features

/// Aura has no `++`/`--`: increment is an explicit assignment.
#[test]
fn increment_operators_are_absent() {
    assert_eq!(code("fn main() { let mut x = 1\n x++ }"), codes::EXPECTED);
    assert_eq!(code("fn main() { let mut x = 1\n ++x }"), codes::EXPECTED);
}

// ------------------------------------------------------------ trailing comma

/// A trailing comma is accepted uniformly before every closing delimiter, so
/// lists of any kind may be written and extended line by line.
#[test]
fn trailing_comma_is_uniform() {
    assert_eq!(ok("fn main() { print([1, 2,]) }"), "[1, 2]\n");
    assert_eq!(ok("fn main() { print({\"a\": 1,}) }"), "{\"a\": 1}\n");
    assert_eq!(
        ok("fn f(a, b,) { return a + b }\nfn main() { print(f(1, 2,)) }"),
        "3\n"
    );
    assert_eq!(
        ok("struct S { a: int, }\nfn main() { print(S { a: 1, }.a) }"),
        "1\n"
    );
    assert_eq!(
        ok("enum E { A(int, int), }\nfn main() { match A(1, 2,) { A(x, y) -> print(x + y) } }"),
        "3\n"
    );
    // Layout across lines is allowed at a trailing comma.
    assert_eq!(
        ok("fn f(a, b) { return a + b }\nfn main() { print(f(\n 1,\n 2,\n)) }"),
        "3\n"
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
