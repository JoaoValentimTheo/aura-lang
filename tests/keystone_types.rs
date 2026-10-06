#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Aura 0.3 Keystone type-system tests: `never`, the `none` type, and
//! optional (`T | none`) narrowing.
//!
//! Authority: `docs/LANGUAGE_SPEC.md` §4.3, §5.2, §15.2; RFC-less because the
//! behavior here is the documented `T | none` boundary made precise, not a new
//! language feature. Every test is written to fail if the invariant is
//! removed.

use aura::check::Checker;
use aura::error::codes;
use aura::parse::parse;
use aura::run_source;

fn ok(src: &str) -> String {
    run_source(src, "<types>").expect("expected the program to run")
}

fn err(src: &str) -> u16 {
    run_source(src, "<types>")
        .expect_err("expected the program to be rejected")
        .code
}

fn check(src: &str) -> Result<(), u16> {
    let module = parse(src).expect("parses");
    Checker::module(&module).map_err(|d| d.code)
}

// ---------------------------------------------------------------------------
// `never`
// ---------------------------------------------------------------------------

#[test]
fn never_is_the_bottom_type() {
    // A `never` value is acceptable where any type is expected.
    assert_eq!(
        ok("fn fail() -> never { throw \"x\" }\nfn f(x: int) -> int { if x < 0 { let v: int = fail()\n return v }\n return x }\nfn main() { print(f(3)) }"),
        "3\n"
    );
    // A `never` return type accepts only `never` (a `return 1` is E3005).
    assert_eq!(
        err("fn f() -> never { return 1 }\nfn main() { f() }"),
        codes::RETURN_MISMATCH
    );
}

#[test]
fn never_is_absorbed_by_unions() {
    // `int | never` is exactly `int`: the bottom member carries no values.
    assert_eq!(
        check("fn f(x: int) -> int | never { return x }\nfn main() { print(f(1)) }"),
        Ok(())
    );
    // A union of only `never` members is `never`.
    assert_eq!(check("fn f() -> never | never { throw 1 }"), Ok(()));
}

#[test]
fn never_runtime_behavior_is_unchanged() {
    // `-> never` does not change execution: the throw propagates and is
    // still reported at the raise site.
    assert_eq!(
        err("fn f() -> never { throw 7 }\nfn main() { f() }"),
        codes::FOREIGN
    );
}

// ---------------------------------------------------------------------------
// `none` precision
// ---------------------------------------------------------------------------

#[test]
fn none_access_is_diagnosed_precisely() {
    // The historical message ("none has no method") is replaced by a precise
    // diagnostic at check time. A bare `none` says so; an optional says the
    // guard that would narrow it.
    assert_eq!(
        check("fn main() { print(none.len()) }"),
        Err(codes::POSSIBLE_NONE)
    );
    assert_eq!(
        check("struct U { name: string }\nfn f(u: U | none) -> string { return u.name }"),
        Err(codes::POSSIBLE_NONE)
    );
    assert_eq!(
        check("struct U { name: string }\nfn f(u: U | none) -> string { return u.name() }"),
        Err(codes::POSSIBLE_NONE)
    );
    assert_eq!(
        check("fn f(xs: [int] | none) -> int { return xs[0] }"),
        Err(codes::POSSIBLE_NONE)
    );
}

#[test]
fn none_permissiveness_is_preserved() {
    // The documented `T | none` behavior is unchanged: a union containing
    // `none` accepts any value, and `none` is accepted anywhere.
    assert_eq!(
        check("type N = int | float | none\nfn main() { let a: N = 1\n let b: N = 2.5\n let c: N = none\n let d: N = \"s\" }"),
        Ok(())
    );
    assert_eq!(check("fn main() { let x: int = none }"), Ok(()));
    assert_eq!(check("fn f() -> {string: int} { return none }"), Ok(()));
}

// ---------------------------------------------------------------------------
// Optional narrowing
// ---------------------------------------------------------------------------

#[test]
fn guard_narrows_in_the_then_branch() {
    assert_eq!(
        ok("struct U { name: string }\nfn f(u: U | none) -> string { if u != none { return u.name }\n return \"missing\" }\nfn main() { print(f(U { name: \"a\" })) }"),
        "a\n"
    );
    // `none != u` is the same guard.
    assert_eq!(
        ok("struct U { name: string }\nfn f(u: U | none) -> string { if none != u { return u.name }\n return \"missing\" }\nfn main() { print(f(U { name: \"a\" })) }"),
        "a\n"
    );
}

#[test]
fn guard_narrows_in_the_else_branch() {
    assert_eq!(
        ok("struct U { name: string }\nfn f(u: U | none) -> string { if u == none { return \"missing\" }\n return u.name }\nfn main() { print(f(none)) }"),
        "missing\n"
    );
    assert_eq!(
        ok("struct U { name: string }\nfn f(u: U | none) -> string { if u == none { return \"missing\" }\n return u.name }\nfn main() { print(f(U { name: \"b\" })) }"),
        "b\n"
    );
}

#[test]
fn diverging_guard_narrows_after_the_if() {
    // `if u == none { return }` proves `u` is not none afterwards, including
    // when a nested member is accessed.
    assert_eq!(
        ok("struct U { name: string }\nfn f(u: U | none) -> string { if u == none { return \"?\" }\n return u.name }\nfn main() { print(f(U { name: \"c\" })) }"),
        "c\n"
    );
    // The narrowing does not survive when the guard branch can fall through.
    assert_eq!(
        check("struct U { name: string }\nfn f(u: U | none) -> string { if u != none { print(u.name) }\n return u.name }"),
        Err(codes::POSSIBLE_NONE)
    );
}

#[test]
fn narrowing_does_not_leak_out_of_the_branch() {
    // Inside the guarded branch the narrow holds; outside, the optional type
    // returns, so a later unguarded access is still E3003.
    assert_eq!(
        check("struct U { name: string }\nfn f(u: U | none) -> string { if u != none { print(u.name) }\n return u.name }"),
        Err(codes::POSSIBLE_NONE)
    );
}

#[test]
fn method_call_narrows_too() {
    assert_eq!(
        ok("struct U { name: string }\nimpl U { fn shout(self) -> string { return self.name } }\nfn f(u: U | none) -> string { if u != none { return u.shout() }\n return \"none\" }\nfn main() { print(f(U { name: \"hi\" })) }"),
        "hi\n"
    );
}

#[test]
fn else_if_chain_still_narrows() {
    // The guard on the `else` expression narrows its block.
    assert_eq!(
        ok("struct U { name: string }\nfn f(u: U | none) -> string { if false { return \"x\" } else if u != none { return u.name } else { return \"none\" } }\nfn main() { print(f(U { name: \"z\" })) }"),
        "z\n"
    );
}

// ---------------------------------------------------------------------------
// `never` divergence enforcement
// ---------------------------------------------------------------------------

#[test]
fn never_body_must_diverge() {
    // A body that can complete normally contradicts `-> never` (`E3006`).
    assert_eq!(
        check("fn bad() -> never { print(\"oops\") }"),
        Err(codes::NEVER_RETURNS)
    );
    // A `return <value>` contradicts `never` at the returned value first:
    // `E3005` names the actual mismatch precisely.
    assert_eq!(
        check("fn bad() -> never { return 1 }"),
        Err(codes::RETURN_MISMATCH)
    );
    // A bare `return` (no value) is a normal completion and is `E3006`.
    assert_eq!(
        check("fn bad() -> never { return }"),
        Err(codes::NEVER_RETURNS)
    );
}

#[test]
fn diverging_shapes_satisfy_never() {
    // `throw`, an infinite `loop`, and a call to another `-> never` function
    // are the recognized diverging shapes.
    assert_eq!(check("fn f() -> never { throw \"x\" }"), Ok(()));
    assert_eq!(check("fn f() -> never { loop { print(1) } }"), Ok(()));
    assert_eq!(
        check("fn a() -> never { throw 1 }\nfn b() -> never { a() }"),
        Ok(())
    );
    // A `loop` with a reachable `break` can complete: not diverging.
    assert_eq!(
        check("fn f() -> never { loop { break } }"),
        Err(codes::NEVER_RETURNS)
    );
}

#[test]
fn never_call_proves_the_path_cannot_continue() {
    // A `-> never` call in a branch makes the branch divergent, so the
    // remainder of a function is not an unreachable-access problem; more
    // importantly the checker does not invent a value for it.
    assert_eq!(
        check("fn fail(m: string) -> never { throw m }\nfn f(x: int) -> int { if x < 0 { fail(\"neg\") }\n return x }"),
        Ok(())
    );
}
