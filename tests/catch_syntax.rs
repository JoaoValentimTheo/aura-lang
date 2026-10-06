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
            "fn f() -> int { try { return 1 } catch _ { return 0 } finally { return 2 } }\nfn main() { print(f()) }"
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

// ---------------------------------------------------------------------------
// RFC 0001 — catch selection reuses the pattern grammar
// ---------------------------------------------------------------------------

#[test]
fn catch_selects_a_nominal_variant() {
    // A variant pattern catches only that variant; the payload binds.
    assert_eq!(
        ok(
            "enum MyErr { Bad(string), Worse(int) }\nfn main() { try { throw MyErr::Bad(\"boom\") } catch MyErr::Bad(m) { print(m) } }"
        ),
        "boom\n"
    );
}

#[test]
fn catch_non_matching_variant_propagates() {
    // A non-selected value keeps propagating: the outer catch sees it.
    assert_eq!(
        ok(
            "enum MyErr { Bad(string), Worse(int) }\nfn main() { try { try { throw MyErr::Worse(3) } catch MyErr::Bad(_) { print(\"inner\") } } catch _ { print(\"outer\") } }"
        ),
        "outer\n"
    );
}

#[test]
fn catch_wildcard_binds_nothing() {
    assert_eq!(
        ok("fn main() { try { throw 5 } catch _ { print(\"any\") } }"),
        "any\n"
    );
}

#[test]
fn catch_none_pattern() {
    assert_eq!(
        ok("fn main() { try { throw none } catch none { print(\"absent\") } }"),
        "absent\n"
    );
}

#[test]
fn catch_literal_pattern() {
    assert_eq!(
        ok("fn main() { try { throw 7 } catch 7 { print(\"seven\") } }"),
        "seven\n"
    );
}

#[test]
fn catch_unknown_variant_is_rejected() {
    // The catch pattern is validated exactly like a match arm's.
    assert_eq!(
        code("fn main() { try { throw 1 } catch Nope::X(m) { print(m) } }"),
        codes::UNKNOWN_TYPE
    );
}

#[test]
fn catch_pattern_duplicate_binding_is_rejected() {
    assert_eq!(
        code("fn main() { try { throw [1, 2] } catch [a, a] { print(a) } }"),
        codes::DUPLICATE_BINDING
    );
}

#[test]
fn catch_variant_payload_binding_is_immutable() {
    assert_eq!(
        code("enum E { V(int) }\nfn main() { try { throw E::V(1) } catch E::V(x) { x = 2 } }"),
        codes::ASSIGN_IMMUTABLE
    );
}

#[test]
fn catch_non_match_still_runs_finally() {
    assert_eq!(
        ok(
            "enum E { V(int) }\nfn main() { try { try { throw E::V(1) } catch 3 { print(\"no\") } finally { print(\"fin\") } } catch _ { print(\"outer\") } }"
        ),
        "fin\nouter\n"
    );
}

#[test]
fn catch_module_qualified_variant_is_nominal() {
    // A cross-module variant pattern uses the module-qualified path and
    // selects by nominal identity, not by spelling.
    assert_eq!(
        ok(
            "module M { pub enum E { Boom(string) } }\nfn main() { try { throw M::E::Boom(\"x\") } catch M::E::Boom(m) { print(m) } }"
        ),
        "x\n"
    );
}

#[test]
fn builtin_exception_namespace_is_reserved() {
    // E6 (RFC 0001): a user module cannot claim the builtin family root.
    assert_eq!(
        code("module Aura { pub struct Foo { x: int } }\nfn main() { print(1) }"),
        codes::RESERVED_NAMESPACE
    );
    // The reservation is case-sensitive nominal identity: a differently
    // spelled module is a different name.
    assert_eq!(
        ok("module aura { pub struct Foo { x: int } }\nfn main() { print(1) }"),
        "1\n"
    );
    // A nested module cannot counterfeit the root namespace.
    assert_eq!(
        ok("module Outer { module Aura { pub struct Foo { x: int } } }\nfn main() { print(1) }"),
        "1\n"
    );
}

#[test]
fn uncaught_throw_reports_the_raise_site() {
    // RFC 0001: an uncaught throw reports the `throw` statement, not the
    // frame-crossing call site.
    let err = run_source(
        "fn f() -> int {\n    throw 7\n}\nfn main() {\n    f()\n}\n",
        "<raise>",
    )
    .expect_err("expected an uncaught throw");
    assert_eq!(err.code, codes::FOREIGN);
    let (line, _col) = aura::error::line_col(
        "fn f() -> int {\n    throw 7\n}\nfn main() {\n    f()\n}\n",
        err.span.start,
    );
    assert_eq!(line, 2, "raise-site line");
}

#[test]
fn caught_throw_reports_no_uncaught_error() {
    assert_eq!(
        ok("fn f() -> int {\n    throw 7\n}\nfn main() {\n    try { f() } catch e { print(e) }\n}"),
        "7\n"
    );
}
