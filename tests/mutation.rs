#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Mutation capability (`LANGUAGE_SPEC.md` §16.6).
//!
//! Aura is immutable by default: `let` creates an immutable binding and
//! `let mut` a mutable one. Any operation that can mutate state reached
//! through a binding requires that binding to be `mut`. This is a single
//! general rule, not a list of forbidden operations.

use aura::error::codes;
use aura::run_source;

/// Run a program that must succeed and return its stdout.
fn ok(src: &str) -> String {
    run_source(src, "<mut>").expect("expected the program to run")
}

/// The first diagnostic code for a rejected program.
fn code(src: &str) -> u16 {
    run_source(src, "<mut>")
        .expect_err("expected the program to be rejected")
        .code
}

#[test]
fn push_requires_a_mutable_binding() {
    assert_eq!(
        code("fn main() { let xs = [1, 2]\n xs.push(3) }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        ok("fn main() { let mut xs = [1, 2]\n xs.push(3)\n print(xs) }"),
        "[1, 2, 3]\n"
    );
}

#[test]
fn free_mutating_builtin_requires_a_mutable_binding() {
    assert_eq!(
        code("fn main() { let xs = [1]\n push(xs, 2) }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        ok("fn main() { let mut xs = [1]\n push(xs, 2)\n print(xs) }"),
        "[1, 2]\n"
    );
}

#[test]
fn index_assignment_requires_a_mutable_binding() {
    assert_eq!(
        code("fn main() { let xs = [1, 2]\n xs[0] = 9 }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        ok("fn main() { let mut xs = [1, 2]\n xs[0] = 9\n print(xs[0]) }"),
        "9\n"
    );
}

#[test]
fn field_assignment_requires_a_mutable_binding() {
    assert_eq!(
        code("struct S { a: int }\nfn main() { let s = S { a: 1 }\n s.a = 2 }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        ok("struct S { a: int }\nfn main() { let mut s = S { a: 1 }\n s.a = 2\n print(s.a) }"),
        "2\n"
    );
}

#[test]
fn map_entry_assignment_requires_a_mutable_binding() {
    assert_eq!(
        code("fn main() { let m = {\"a\": 1}\n m[\"b\"] = 2 }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        ok("fn main() { let mut m = {\"a\": 1}\n m[\"b\"] = 2\n print(m.get(\"b\")) }"),
        "2\n"
    );
}

#[test]
fn map_remove_requires_a_mutable_binding() {
    assert_eq!(
        code("fn main() { let m = {\"a\": 1}\n m.remove(\"a\") }"),
        codes::ASSIGN_IMMUTABLE
    );
}

#[test]
fn mutating_receiver_must_declare_mut_self() {
    // A method that mutates its receiver and does not declare `mut self` is
    // rejected inside its own body.
    assert_eq!(
        code(
            "struct S { n: int }\nimpl S { fn bump(self) { self.n = self.n + 1 } }\nfn main() { let mut s = S { n: 0 }\n s.bump() }"
        ),
        codes::ASSIGN_IMMUTABLE
    );
    // With `mut self` the body is allowed, but the caller also needs `mut`.
    assert_eq!(
        code(
            "struct S { n: int }\nimpl S { fn bump(mut self) { self.n = self.n + 1 } }\nfn main() { let s = S { n: 0 }\n s.bump() }"
        ),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        ok(
            "struct S { n: int }\nimpl S { fn bump(mut self) { self.n = self.n + 1 } }\nfn main() { let mut s = S { n: 0 }\n s.bump()\n print(s.n) }"
        ),
        "1\n"
    );
}

#[test]
fn reading_methods_need_no_mutability() {
    assert_eq!(
        ok("struct S { n: int }\nimpl S { fn get(self) -> int { return self.n } }\nfn main() { let s = S { n: 3 }\n print(s.get()) }"),
        "3\n"
    );
}

#[test]
fn capability_is_per_binding_not_per_value() {
    // Each binding carries its own capability. A mutable binding can be
    // mutated even after a shared alias exists; the alias is a separate
    // binding and needs its own `mut` to be mutated through.
    assert_eq!(
        ok("fn main() { let mut a = [1]\n let b = a\n a.push(2)\n print(b) }"),
        "[1, 2]\n"
    );
    assert_eq!(
        code("fn main() { let mut a = [1]\n let b = a\n b.push(2) }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        ok("fn main() { let mut a = [1]\n let mut b = a\n b.push(2)\n print(a) }"),
        "[1, 2]\n"
    );
}

#[test]
fn closure_capture_respects_the_captured_binding() {
    assert_eq!(
        code("fn main() { let xs = [1]\n let f = () -> xs.push(2)\n f() }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        ok("fn main() { let mut xs = [1]\n let f = () -> xs.push(2)\n f()\n print(xs) }"),
        "[1, 2]\n"
    );
}

#[test]
fn mut_parameter_grants_body_capability() {
    assert_eq!(
        code("fn f(x) { x = 1\n return x }\nfn main() { print(f(0)) }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        ok("fn f(mut x) { x = 1\n return x }\nfn main() { print(f(0)) }"),
        "1\n"
    );
}

#[test]
fn loop_and_catch_bindings_are_immutable() {
    assert_eq!(
        code("fn main() { for i in range(0, 2) { i = 5 } }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        code("fn main() { try { throw 1 } catch e -> { e = 2 } }"),
        codes::ASSIGN_IMMUTABLE
    );
}

#[test]
fn nested_places_use_the_root_binding() {
    assert_eq!(
        code(
            "struct Inner { n: int }\nstruct Outer { inner: Inner }\nfn main() { let o = Outer { inner: Inner { n: 0 } }\n o.inner.n = 1 }"
        ),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        ok(
            "struct Inner { n: int }\nstruct Outer { inner: Inner }\nfn main() { let mut o = Outer { inner: Inner { n: 0 } }\n o.inner.n = 1\n print(o.inner.n) }"
        ),
        "1\n"
    );
}

#[test]
fn trait_receiver_mutability_is_part_of_the_contract() {
    // A trait declaring `self` cannot be implemented with `mut self`.
    assert_eq!(
        code(
            "trait T { fn a(self) }\nstruct S { n: int }\nimpl T for S { fn a(mut self) { self.n = 1 } }\nfn main() {}"
        ),
        codes::TYPE_MISMATCH
    );
    // A trait declaring `mut self` requires `mut self` in the implementation.
    assert_eq!(
        code(
            "trait T { fn a(mut self) }\nstruct S { n: int }\nimpl T for S { fn a(self) { print(1) } }\nfn main() {}"
        ),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn pure_methods_that_return_new_values_do_not_mutate() {
    // `sort`/`reverse` return a new list and leave the receiver unchanged, so
    // they need no mutability and are rejected only on an immutable binding
    // when used as a mutating method (they are not).
    assert_eq!(
        ok("fn main() { let xs = [3, 1, 2]\n print(sort(xs))\n print(xs) }"),
        "[1, 2, 3]\n[3, 1, 2]\n"
    );
}

#[test]
fn module_constants_are_immutable_places() {
    // A module constant is immutable, so mutating through it is rejected,
    // exactly like an immutable `let`.
    assert_eq!(
        code("const XS = [1]\nfn main() { push(XS, 2) }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        code("const XS = [1]\nfn main() { XS[0] = 2 }"),
        codes::ASSIGN_IMMUTABLE
    );
}
