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
fn optional_accepts_exactly_its_members() {
    // `T | none` accepts its own members (`T` and `none`) and nothing else:
    // optionality is a real type relation, not a permissiveness escape
    // hatch (§4.3, §5.2).
    assert_eq!(
        check("type N = int | float | none\nfn main() { let _: N = 1\n let _: N = 2.5\n let _: N = none }"),
        Ok(())
    );
    assert_eq!(
        check("type N = int | float | none\nfn main() { let _: N = \"s\" }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A strict type rejects `none` (no silent optionality), and a `none`
    // return does not satisfy a strict return type.
    assert_eq!(
        check("fn main() { let _: int = none }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("fn f() -> {string: int} { return none }"),
        Err(codes::RETURN_MISMATCH)
    );
    // The variable-bound spelling is the same relation.
    assert_eq!(
        check("fn main() { let n = none\n let _: int = n }"),
        Err(codes::TYPE_MISMATCH)
    );
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

// ---------------------------------------------------------------------------
// Independent-review regression: narrowing must be invalidated by a write
// ---------------------------------------------------------------------------

#[test]
fn assignment_invalidates_narrowing_from_an_inner_scope() {
    // The narrowing override lives in the block scope while the binding is
    // declared in the function scope; a write must drop it, or the checker
    // accepts an access the runtime rejects (review finding 1).
    assert_eq!(
        check("struct U { name: string }\nfn f(mut u: U | none) -> string { if u == none { return \"m\" }\n u = none\n return u.name }"),
        Err(codes::POSSIBLE_NONE)
    );
    assert_eq!(
        check("struct U { name: string }\nfn f(mut u: U | none) -> string { if u != none { u = none\n return u.name }\n return \"x\" }"),
        Err(codes::POSSIBLE_NONE)
    );
}

// ---------------------------------------------------------------------------
// Independent-review regression: `never` divergence rules
// ---------------------------------------------------------------------------

#[test]
fn try_that_completes_normally_does_not_satisfy_never() {
    // Both the body and the catch path must diverge (review finding 2).
    assert_eq!(
        check("fn f() -> never { try { print(\"X\") } catch _ { throw 1 } }"),
        Err(codes::NEVER_RETURNS)
    );
    // A genuinely diverging try still satisfies `never`.
    assert_eq!(
        check("fn f() -> never { try { throw 1 } catch _ { throw 2 } }"),
        Ok(())
    );
}

#[test]
fn nested_loop_break_does_not_escape_the_outer_loop() {
    // A `break` in a nested loop belongs to that loop, so the outer `loop`
    // still diverges (review finding 3).
    assert_eq!(check("fn f() -> never { loop { loop { break } } }"), Ok(()));
    assert_eq!(
        check("fn f() -> never { loop { for _x in [1] { break } } }"),
        Ok(())
    );
    // A reachable break of the measured loop still makes it non-diverging.
    assert_eq!(
        check("fn f() -> never { loop { break } }"),
        Err(codes::NEVER_RETURNS)
    );
}

#[test]
fn a_local_binding_shadows_a_function_name_at_runtime() {
    // `LANGUAGE_SPEC.md` §6.5/§16.3: a shadowed name must not resolve to the
    // top-level declaration (review finding 4).
    assert_eq!(
        ok("fn boom() -> never { throw \"global\" }\nfn main() { let boom = () -> 7\n print(boom()) }"),
        "7\n"
    );
    assert_eq!(
        ok("fn f(x: int) -> int { return x + 100 }\nfn main() { let f = (a: string) -> a\n print(f(\"hello\")) }"),
        "hello\n"
    );
}

// ---------------------------------------------------------------------------
// Optionality is a real relation (TD-21 correction)
//
// `T <: T | none`, `none <: T | none`, but `T | none </: T` without a
// narrowing guard. Every position enforces the same relation.
// ---------------------------------------------------------------------------

#[test]
fn optional_to_strict_is_rejected_in_every_position() {
    // function argument
    assert_eq!(
        check("fn consume(u: int) -> int { return u }\nfn f(m: int | none) -> int { return consume(m) }\nfn main() { print(f(1)) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // return
    assert_eq!(
        check("fn f(m: int | none) -> int { return m }"),
        Err(codes::RETURN_MISMATCH)
    );
    // local initialization
    assert_eq!(
        check("fn main() { let m: int | none = none\n let _: int = m }"),
        Err(codes::TYPE_MISMATCH)
    );
    // reassignment
    assert_eq!(
        check("fn main() { let mut x: int = 0\n let m: int | none = none\n x = m }"),
        Err(codes::TYPE_MISMATCH)
    );
    // struct field
    assert_eq!(
        check("struct S { a: int }\nfn main() { let m: int | none = none\n let _s = S { a: m } }"),
        Err(codes::TYPE_MISMATCH)
    );
    // enum payload
    assert_eq!(
        check("enum E { A(int) }\nfn main() { let m: int | none = none\n let _e = A(m) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // list element
    assert_eq!(
        check("fn main() { let m: int | none = none\n let _xs: [int] = [m] }"),
        Err(codes::TYPE_MISMATCH)
    );
    // builtin argument
    assert_eq!(
        check("fn main() { let m: int | none = none\n print(abs(m)) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

#[test]
fn optional_accepts_its_members_in_every_position() {
    // `T -> T | none`
    assert_eq!(
        check("fn consume(m: int | none) -> int { print(m)\n return 1 }\nfn main() { print(consume(1)) }"),
        Ok(())
    );
    // `none -> T | none`
    assert_eq!(
        check("fn consume(m: int | none) -> int { print(m)\n return 1 }\nfn main() { print(consume(none)) }"),
        Ok(())
    );
    // `T | none -> T | none`
    assert_eq!(
        check("fn consume(m: int | none) -> int { print(m)\n return 1 }\nfn g(m: int | none) -> int { return consume(m) }\nfn main() { print(g(1)) }"),
        Ok(())
    );
    // `T | none -> T | float | none` (widening)
    assert_eq!(
        check("fn consume(m: int | float | none) -> int { print(m)\n return 1 }\nfn g(m: int | none) -> int { return consume(m) }\nfn main() { print(g(1)) }"),
        Ok(())
    );
    // optional struct field accepts all three
    assert_eq!(
        check("struct S { email: string | none }\nfn main() { let _a = S { email: \"x\" }\n let _b = S { email: none } }"),
        Ok(())
    );
}

#[test]
fn narrowings_invalidate_when_the_proof_ends() {
    // A closure captures by reference: calling one that writes a captured
    // name ends the narrowing proven for that name.
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn main() { let mut u: U | none = U { name: \"a\" }\n let clear = () -> { u = none }\n if u != none { clear()\n print(consume(u)) } }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A closure that writes an unrelated name does not disturb the proof.
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn main() { let mut u: U | none = U { name: \"a\" }\n let mut other: int = 0\n let bump = () -> { other = 1 }\n if u != none { bump()\n print(consume(u)) } }"),
        Ok(())
    );
    // A pure closure does not disturb the proof either.
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn main() { let u: U | none = U { name: \"a\" }\n let pure = () -> 1\n if u != none { pure()\n print(consume(u)) } }"),
        Ok(())
    );
    // Shadowing the narrowed name with a fresh `none` ends the proof.
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn f(u: U | none) -> string { if u != none { let u = none\n return consume(u) }\n return \"x\" }\nfn main() { print(f(U { name: \"a\" })) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A loop exit does not prove the guard held.
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn f(mut u: U | none, v: U | none) -> string { while u != none { u = v }\n return consume(u) }\nfn main() { print(f(U { name: \"a\" }, none)) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

#[cfg(feature = "json")]
#[test]
fn unknown_boundary_stays_permissive() {
    // A value the checker genuinely cannot type is not rejected (§2.3).
    assert_eq!(
        check("fn consume(u: int) -> int { return u }\nfn main() { print(consume(json_decode(\"{}\"))) }"),
        Ok(())
    );
    assert_eq!(
        check("fn consume(u: int) -> int { return u }\nfn main() { print(consume(if true { 1 } else { 2 })) }"),
        Ok(())
    );
}

#[test]
fn a_call_through_an_unknown_callable_invalidates_captured_narrowing() {
    // A closure stored in a container and invoked through a higher-order
    // builtin can still write its captures; the narrowing of any name that
    // some lambda in the program assigns is therefore discarded at such a
    // call (Jev-flagged gap in the first closure rule).
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn main() { let mut u: U | none = U { name: \"a\" }\n let hs = [() -> { u = none }]\n if u != none { map(hs, (h) -> h())\n print(consume(u)) } }"),
        Err(codes::TYPE_MISMATCH)
    );
}

#[test]
fn unknown_callable_fallback_does_not_reject_sound_programs() {
    // The fallback only discards narrowings of names that some lambda writes;
    // a program whose lambdas never touch the narrowed name stays accepted.
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn main() { let u: U | none = U { name: \"a\" }\n let xs = [1, 2]\n if u != none { map(xs, (x) -> x + 1)\n print(consume(u)) } }"),
        Ok(())
    );
}

// ---------------------------------------------------------------------------
// Second independent review: narrowing invalidation and branch joins
// ---------------------------------------------------------------------------

#[test]
fn a_closure_passed_to_a_user_function_invalidates_captured_narrowing() {
    // The callee may invoke the callback, so the proof for a captured name
    // cannot survive the call (review C1).
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn apply(cb) -> int { return cb(1) }\nfn main() { let mut u: U | none = U { name: \"a\" }\n let clear = (_x) -> { u = none }\n if u != none { apply(clear)\n print(consume(u)) } }"),
        Err(codes::TYPE_MISMATCH)
    );
}

#[test]
fn a_closure_passed_to_a_higher_order_builtin_invalidates_captured_narrowing() {
    // Free-function and method spellings both invoke the callback (review C2).
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn main() { let mut u: U | none = U { name: \"a\" }\n let clear = (_x) -> { u = none }\n if u != none { map([1], clear)\n print(consume(u)) } }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn main() { let mut u: U | none = U { name: \"a\" }\n let clear = (_x) -> { u = none }\n if u != none { [1, 2].map(clear)\n print(consume(u)) } }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A callback whose captures do not include the narrowed name is fine.
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn main() { let u: U | none = U { name: \"a\" }\n if u != none { map([1], (x) -> x + 1)\n print(consume(u)) } }"),
        Ok(())
    );
}

#[test]
fn a_write_in_the_surviving_branch_cancels_a_divergence_narrowing() {
    // `if u != none { u = none } else { return }` proves nothing after the
    // `if`: the surviving then-branch wrote the name (review C3).
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn f(mut u: U | none) -> string { if u != none { u = none } else { return \"x\" }\n return consume(u) }\nfn main() { print(f(U { name: \"a\" })) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // The symmetric form: the else-branch writes while the then-branch diverges.
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn f(mut u: U | none) -> string { if u == none { return \"x\" } else { u = none }\n return consume(u) }\nfn main() { print(f(U { name: \"a\" })) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A diverging branch that does NOT write still proves the fact.
    assert_eq!(
        check("struct U { name: string }\nfn consume(u: U) -> string { return u.name }\nfn f(u: U | none) -> string { if u == none { return \"x\" }\n return consume(u) }\nfn main() { print(f(U { name: \"a\" })) }"),
        Ok(())
    );
}

#[test]
fn branch_joins_are_not_unknown() {
    // An `if` joins its branches; a possibly-`none` join cannot cross a strict
    // boundary without narrowing (review C4).
    assert_eq!(
        check("fn f(flag: bool) -> int { return if flag { 1 } else { none } }\nfn main() { print(f(false)) }"),
        Err(codes::RETURN_MISMATCH)
    );
    assert_eq!(
        check(
            "fn main() { let flag = true\n let x: int = if flag { 1 } else { none }\n print(x) }"
        ),
        Err(codes::TYPE_MISMATCH)
    );
    // A `match` joins its arms likewise.
    assert_eq!(
        check("fn main() { let x: int = match 1 { 1 -> none, _ -> 5 }\n print(x) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A branch with a genuinely unknown value still infers Unknown (§2.3).
    assert_eq!(
        check("fn u() { return none }\nfn f(flag: bool) -> int { return if flag { 1 } else { u() } }\nfn main() { print(f(true)) }"),
        Ok(())
    );
}

#[test]
fn a_loop_break_in_an_expression_position_is_seen() {
    // A reachable break inside a nested block expression means the loop can
    // complete, so it does not satisfy `never` (review C5).
    assert_eq!(
        check("fn f() -> never { while true { let _x = { break } } }\nfn main() { f() }"),
        Err(codes::NEVER_RETURNS)
    );
    // A break in a nested loop belongs to that loop; the outer still diverges.
    assert_eq!(
        check("fn f() -> never { loop { for _x in [1] { break } } }"),
        Ok(())
    );
}

#[test]
fn map_lookup_is_optional_not_dynamic() {
    // `m.get(k)` returns `V | none`; it cannot launder into a strict binding.
    assert_eq!(
        check("fn main() { let m = {\"a\": 1}\n let v: int = m.get(\"b\")\n print(v) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // The narrowed use is accepted.
    assert_eq!(
        check("fn main() { let m = {\"a\": 1}\n let v = m.get(\"b\")\n if v != none { print(v) } else { print(\"absent\") } }"),
        Ok(())
    );
}

#[test]
fn never_satisfies_class_positions() {
    // The bottom type cannot violate any class, so a diverging call is usable
    // where a class-typed value is expected.
    assert_eq!(
        check("fn f() -> never { throw \"x\" }\nfn main() { print(range(f(), 3)) }"),
        Ok(())
    );
}
