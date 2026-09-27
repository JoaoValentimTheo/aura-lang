#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The `catch` surface syntax (`LANGUAGE_SPEC.md` §14.5).
//!
//! A catch binding is followed directly by its block: `catch e { … }`. The
//! older `catch e -> { … }` form is a parse error. This is a surface-syntax
//! change only: propagation, binding, scope, mutability, and diagnostics are
//! unchanged.

use aura::error::codes;
use aura::run_source;

/// Run a program that must succeed and return its stdout.
fn ok(src: &str) -> String {
    run_source(src, "<catch>").expect("expected the program to run")
}

/// The first diagnostic code for a rejected program.
fn code(src: &str) -> u16 {
    run_source(src, "<catch>")
        .expect_err("expected the program to be rejected")
        .code
}

#[test]
fn basic_catch_without_arrow() {
    assert_eq!(
        ok("fn main() { try { throw 7 } catch e { print(e) } }"),
        "7\n"
    );
}

#[test]
fn catch_binding_is_visible_in_the_catch_body() {
    assert_eq!(
        ok("fn main() { try { throw \"boom\" } catch e { print(e) } }"),
        "boom\n"
    );
}

#[test]
fn catch_binding_does_not_leak_out_of_scope() {
    assert_eq!(
        code("fn main() { try { throw 1 } catch e { print(e) }\n print(e) }"),
        codes::UNDEFINED
    );
}

#[test]
fn nested_catch() {
    assert_eq!(
        ok(
            "fn main() { try { throw 1 } catch e { try { throw 2 } catch f { print(e)\n print(f) } } }"
        ),
        "1\n2\n"
    );
}

#[test]
fn catch_binding_can_be_shadowed() {
    // The catch binding is an ordinary immutable binding (§16.3): a `let`
    // inside the body shadows it.
    assert_eq!(
        ok("fn main() { try { throw 5 } catch e { let e = e + 1\n print(e) } }"),
        "6\n"
    );
    // Reassigning the catch binding itself is still `E2001`.
    assert_eq!(
        code("fn main() { try { throw 5 } catch e { e = 6 } }"),
        codes::ASSIGN_IMMUTABLE
    );
}

#[test]
fn catch_binding_can_be_captured_by_a_closure() {
    assert_eq!(
        ok("fn main() { try { throw 9 } catch e { let f = () -> e\n print(f()) } }"),
        "9\n"
    );
}

#[test]
fn catch_catches_a_value_thrown_in_a_called_function() {
    assert_eq!(
        ok("fn f() { throw 3 }\nfn main() { try { f() } catch e { print(e) } }"),
        "3\n"
    );
}

#[test]
fn catch_with_finally_and_return() {
    assert_eq!(
        ok(
            "fn f() -> int { try { return 1 } catch e { return 0 } finally { return 2 } }\nfn main() { print(f()) }"
        ),
        "2\n"
    );
}

#[test]
fn the_old_arrow_form_is_rejected() {
    // A `->` after the catch binding is no longer accepted; the block must
    // follow directly. The diagnostic is the ordinary parser diagnostic.
    assert_eq!(
        code("fn main() { try { throw 1 } catch e -> { print(e) } }"),
        codes::EXPECTED
    );
}

#[test]
fn try_requires_catch() {
    // `catch` remains mandatory; this is unchanged by the syntax cleanup.
    assert_eq!(code("fn main() { try { throw 1 } }"), codes::EXPECTED);
}
