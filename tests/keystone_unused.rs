#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Aura 0.3 Keystone unused-binding analysis (`E2008`).
//!
//! Authority: the Go-like static discipline in the Keystone order §12.
//! Compilation enforces that a *local* named declaration is used, and that a
//! named parameter is used; `_` and `_name` are explicit discards. A
//! module-scope binding is a package declaration and is not enforced, and a
//! REPL submission is a session, not a program, so neither applies there
//! either.

use aura::check::Checker;
use aura::error::codes;
use aura::parse::parse;
use aura::run_source;

fn check(src: &str) -> Result<(), u16> {
    let module = parse(src).expect("parses");
    Checker::module(&module).map_err(|d| d.code)
}

fn ok(src: &str) -> String {
    run_source(src, "<unused>").expect("expected the program to run")
}

#[test]
fn unused_local_is_rejected() {
    assert_eq!(
        check("fn main() { let x = 1\n print(2) }"),
        Err(codes::UNUSED_BINDING)
    );
    // Reads before the end of the scope are enough.
    assert_eq!(check("fn main() { let x = 1\n print(x) }"), Ok(()));
}

#[test]
fn unused_parameter_is_rejected() {
    assert_eq!(
        check("fn f(x: int) -> int { return 1 }\nfn main() { print(f(2)) }"),
        Err(codes::UNUSED_BINDING)
    );
    assert_eq!(
        check("fn f(x: int) -> int { return x }\nfn main() { print(f(2)) }"),
        Ok(())
    );
}

#[test]
fn discard_spellings_are_allowed() {
    // `_` binds nothing; `_name` is an explicit discard.
    assert_eq!(check("fn main() { let _ = 1\n print(2) }"), Ok(()));
    assert_eq!(check("fn main() { let _x = 1\n print(2) }"), Ok(()));
    assert_eq!(
        check("fn f(_x: int) -> int { return 1 }\nfn main() { print(f(2)) }"),
        Ok(())
    );
    // A parameter list still forbids a duplicate name (§16.4): two bare `_`
    // parameters are a same-scope redeclaration (`E2007`), exactly as at
    // HEAD. Distinct discards are spelled `_a`, `_b`, or an annotation-differ.
    assert_eq!(
        check("fn f(_: int, _: string) -> int { return 1 }\nfn main() { print(f(1, \"a\")) }"),
        Err(codes::REDECLARED)
    );
    assert_eq!(
        check("fn f(_a: int, _b: string) -> int { return 1 }\nfn main() { print(f(1, \"a\")) }"),
        Ok(())
    );
}

#[test]
fn underscore_named_binding_is_a_discard_not_a_use_contract() {
    // `_x` is an explicit discard: reading it is legal (the `E2009` reverse
    // contract applies to *parameters*, not locals), and it is never reported
    // as unused.
    assert_eq!(check("fn main() { let _x = 1\n print(_x) }"), Ok(()));
    // The `E2009` parameter contract is unchanged.
    assert_eq!(
        check("fn f(_x: int) -> int { return _x }\nfn main() { print(f(2)) }"),
        Err(codes::UNUSED_PARAM)
    );
}

#[test]
fn unused_pattern_and_catch_bindings_are_rejected() {
    assert_eq!(
        check("fn main() { for x in [1, 2] { print(3) } }"),
        Err(codes::UNUSED_BINDING)
    );
    assert_eq!(check("fn main() { for _x in [1, 2] { print(3) } }"), Ok(()));
    assert_eq!(
        check("fn main() { try { throw 1 } catch e { print(2) } }"),
        Err(codes::UNUSED_BINDING)
    );
    assert_eq!(
        check("fn main() { try { throw 1 } catch _ { print(2) } }"),
        Ok(())
    );
    assert_eq!(
        check("fn main() { let [a, _] = [1, 2]\n print(a) }"),
        Ok(())
    );
    assert_eq!(
        check("fn main() { let [a, b] = [1, 2]\n print(a) }"),
        Err(codes::UNUSED_BINDING)
    );
}

#[test]
fn a_write_counts_as_a_use() {
    assert_eq!(check("fn main() { let mut x = 1\n x = 2 }"), Ok(()));
}

#[test]
fn shadowing_does_not_flag_the_shadowed_binding() {
    // Documented shadowing (§16.3) stays legal: the superseded binding is not
    // reported, and the newest binding is the one that must be used.
    assert_eq!(
        check("fn main() { let x = 1\n let x = x + 1\n print(x) }"),
        Ok(())
    );
    // The newest binding is unused even though an older one was read.
    assert_eq!(
        check("fn main() { let x = 1\n print(x)\n let x = 2 }"),
        Err(codes::UNUSED_BINDING)
    );
}

#[test]
fn module_scope_bindings_are_not_enforced() {
    // A package-level declaration may be used by another module or a later
    // REPL submission, so it is not a dead local.
    assert_eq!(check("let x = 1\nfn main() { print(2) }"), Ok(()));
    assert_eq!(check("const C = 1\nfn main() { print(2) }"), Ok(()));
}

#[test]
fn lambda_parameters_are_enforced() {
    assert_eq!(
        check("fn main() { let f = (x: int) -> 1\n print(f(2)) }"),
        Err(codes::UNUSED_BINDING)
    );
    assert_eq!(
        check("fn main() { let f = (_x: int) -> 1\n print(f(2)) }"),
        Ok(())
    );
}

#[test]
fn unused_analysis_does_not_change_runtime_behavior() {
    // A program that uses everything still runs identically.
    assert_eq!(
        ok("fn f(x: int) -> int { return x * 2 }\nfn main() { print(f(21)) }"),
        "42\n"
    );
}
