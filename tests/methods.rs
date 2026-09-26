#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Aqua OOP V1 — struct methods (`LANGUAGE_SPEC.md` §17.6).
//!
//! A method is behavior attached to a nominal struct through an `impl` block,
//! with an explicit `self` receiver. These tests cover the vertical slice:
//! declaration, receiver, field access, mutation, arguments, return typing,
//! method-to-method calls, composition, the field/method distinction, and the
//! collision rule.

use aura::check::Checker;
use aura::error::codes;
use aura::parse::parse;
use aura::run_source;

fn out(src: &str) -> String {
    run_source(src, "<test>").expect("program runs")
}

fn check(src: &str) -> Result<(), u16> {
    let module = parse(src).expect("parses");
    Checker::module(&module).map_err(|d| d.code)
}

/// A basic method reads the receiver's fields and returns a value.
#[test]
fn method_reads_fields_and_returns() {
    let src = r#"
struct Point { x: int, y: int }
impl Point {
    fn sum(self) { return self.x + self.y }
}
fn main() { let p = Point { x: 1, y: 2 }
    print(p.sum()) }
"#;
    assert_eq!(out(src), "3\n");
}

/// A method mutates a field; the mutation is observable through the existing
/// reference semantics.
#[test]
fn method_mutation_is_observable() {
    let src = r#"
struct Counter { n: int }
impl Counter {
    fn bump(self, by: int) { self.n = self.n + by }
}
fn main() { let c = Counter { n: 0 }
    c.bump(3)
    c.bump(4)
    print(c.n) }
"#;
    assert_eq!(out(src), "7\n");
}

/// A method takes ordinary parameters after `self`.
#[test]
fn method_arguments() {
    let src = r#"
struct Box { v: int }
impl Box {
    fn scaled(self, k: int) -> int { return self.v * k }
}
fn main() { print(Box { v: 6 }.scaled(7)) }
"#;
    assert_eq!(out(src), "42\n");
}

/// Return typing is checked exactly like a function.
#[test]
fn method_return_typing() {
    let ok = r#"
struct P { x: int }
impl P {
    fn get(self) -> int { return self.x }
}
fn main() { print(P { x: 1 }.get()) }
"#;
    assert_eq!(out(ok), "1\n");
    let bad = r#"
struct P { x: int }
impl P {
    fn get(self) -> int { return "s" }
}
fn main() { print(1) }
"#;
    assert_eq!(check(bad), Err(codes::RETURN_MISMATCH));
}

/// One method calls another method on `self`.
#[test]
fn method_to_method_call() {
    let src = r#"
struct P { x: int }
impl P {
    fn base(self) { return self.x }
    fn twice(self) { return self.base() * 2 }
}
fn main() { print(P { x: 5 }.twice()) }
"#;
    assert_eq!(out(src), "10\n");
}

/// A method on a nested struct field is reachable through composition.
#[test]
fn composition_calls_nested_method() {
    let src = r#"
struct Inner { v: int }
struct Outer { inner: Inner }
impl Inner {
    fn get(self) { return self.v }
}
fn main() { let o = Outer { inner: Inner { v: 7 } }
    print(o.inner.get()) }
"#;
    assert_eq!(out(src), "7\n");
}

/// A field and a method of the same struct may not share a name.
#[test]
fn field_method_collision_is_rejected() {
    let src = r#"
struct P { x: int }
impl P {
    fn x(self) { return 1 }
}
fn main() { print(1) }
"#;
    assert_eq!(check(src), Err(codes::DUPLICATE_FIELD));
}

/// A method requires parentheses; `r.m` without them is not a bound method.
#[test]
fn method_without_parens_is_a_field_read() {
    let src = r#"
struct P { x: int }
impl P {
    fn m(self) { return 1 }
}
fn main() { let p = P { x: 1 }
    print(p.m) }
"#;
    assert_eq!(check(src), Err(codes::UNDEFINED));
}

/// A nonexistent method on a known struct is `E2003`, with no fallback to a
/// built-in or another struct.
#[test]
fn unknown_method_on_struct_is_e2003() {
    let src = "struct P { x: int }\nfn main() { let p = P { x: 1 }\n print(p.nope()) }";
    assert_eq!(check(src), Err(codes::UNDEFINED));
    // A method name that exists only on a built-in must not resolve on a struct.
    assert_eq!(
        check("struct S { a: int }\nfn main() { S { a: 1 }.upper() }"),
        Err(codes::UNDEFINED)
    );
}

/// Wrong argument count or a provably wrong argument type is `E3001`.
#[test]
fn method_argument_checking() {
    let arity = r#"
struct P { x: int }
impl P {
    fn add(self, n: int) { return self.x + n }
}
fn main() { print(P { x: 1 }.add()) }
"#;
    assert_eq!(check(arity), Err(codes::TYPE_MISMATCH));
    let wrong_type = r#"
struct P { x: int }
impl P {
    fn add(self, n: int) { return self.x + n }
}
fn main() { print(P { x: 1 }.add("s")) }
"#;
    assert_eq!(check(wrong_type), Err(codes::TYPE_MISMATCH));
}

/// A transparent type alias preserves method availability.
#[test]
fn alias_receiver_preserves_methods() {
    let src = r#"
struct P { x: int }
impl P {
    fn get(self) { return self.x }
}
type Q = P
fn main() { let q: Q = P { x: 3 }
    print(q.get()) }
"#;
    assert_eq!(out(src), "3\n");
}

/// Adding methods does not change structural equality.
#[test]
fn methods_do_not_change_equality() {
    let src = r#"
struct P { x: int }
impl P {
    fn get(self) { return self.x }
}
fn main() { print(P { x: 1 } == P { x: 1 }) }
"#;
    assert_eq!(out(src), "true\n");
}

/// Method mutation is visible through a shared struct reference.
#[test]
fn mutation_visible_through_shared_reference() {
    let src = r#"
struct C { n: int }
impl C {
    fn set(self, v: int) { self.n = v }
}
fn main() { let a = C { n: 0 }
    let b = a
    a.set(9)
    print(b.n) }
"#;
    assert_eq!(out(src), "9\n");
}

/// Two structs may declare a method with the same name: methods are scoped to
/// the nominal type, never a global namespace.
#[test]
fn method_names_are_type_scoped() {
    let src = r#"
struct A { x: int }
struct B { y: int }
impl A {
    fn get(self) { return self.x }
}
impl B {
    fn get(self) { return self.y }
}
fn main() { print(A { x: 1 }.get())
    print(B { y: 2 }.get()) }
"#;
    assert_eq!(out(src), "1\n2\n");
}

/// `impl` requires an already-declared nominal struct, and permits one block.
#[test]
fn impl_target_rules() {
    assert_eq!(
        check("impl Nope {\n fn f(self) { return 1 }\n}\nfn main() { print(1) }"),
        Err(codes::UNDEFINED)
    );
    assert_eq!(
        check("enum E { A }\nimpl E {\n fn f(self) { return 1 }\n}\nfn main() { print(1) }"),
        Err(codes::UNDEFINED)
    );
    let two = r#"
struct P { x: int }
impl P {
    fn a(self) { return 1 }
}
impl P {
    fn b(self) { return 2 }
}
fn main() { print(1) }
"#;
    assert_eq!(check(two), Err(codes::REDECLARED));
    let dup = r#"
struct P { x: int }
impl P {
    fn a(self) { return 1 }
    fn a(self) { return 2 }
}
fn main() { print(1) }
"#;
    assert_eq!(check(dup), Err(codes::REDECLARED));
}

/// A method must declare `self` as its first parameter (a parser rule).
#[test]
fn method_requires_self_receiver() {
    assert_eq!(
        parse("struct P { x: int }\nimpl P {\n fn a() { return 1 }\n}\nfn main() { print(1) }")
            .map(|_| ())
            .map_err(|d| d.code),
        Err(codes::EXPECTED)
    );
    // A non-`self` first parameter is rejected too.
    assert_eq!(
        parse(
            "struct P { x: int }\nimpl P {\n fn a(other) { return 1 }\n}\nfn main() { print(1) }"
        )
        .map(|_| ())
        .map_err(|d| d.code),
        Err(codes::EXPECTED)
    );
}

/// `self` and `impl` are NOT globally reserved: `self` has receiver semantics
/// only as a method's first parameter, and `impl` is a behavior declaration
/// only in item position. Everywhere else they remain ordinary identifiers, so
/// pre-Aqua programs keep their meaning (`LANGUAGE_SPEC.md` §3.3, §17.6).
#[test]
fn self_and_impl_remain_ordinary_identifiers() {
    assert_eq!(out("fn main() { let self = 1\n print(self) }"), "1\n");
    assert_eq!(out("fn main() { let impl = 1\n print(impl) }"), "1\n");
    assert_eq!(
        out("fn f(self) { return self }\nfn main() { print(f(6)) }"),
        "6\n"
    );
    assert_eq!(
        out("struct S { impl: int }\nfn main() { print(S { impl: 9 }.impl) }"),
        "9\n"
    );
    assert_eq!(
        out("struct S { self: int }\nfn main() { print(S { self: 9 }.self) }"),
        "9\n"
    );
    assert_eq!(out("fn main() { let [self] = [8]\n print(self) }"), "8\n");
    assert_eq!(out("fn main() { let [impl] = [8]\n print(impl) }"), "8\n");
    // `impl Struct {` at item position is still a behavior block; `impl`
    // followed by a lowercase identifier or an expression is not.
    assert_eq!(
        check(
            "struct P { x: int }\nimpl P { fn g(self) { return self.x } }\nfn main() { print(1) }"
        ),
        Ok(())
    );
}

/// An `Unknown` receiver (an unannotated parameter) may call a user struct
/// method. The checker must not gate the call by the built-in registry: a
/// user method is a valid runtime target (`LANGUAGE_SPEC.md` §2.3, §17.6).
#[test]
fn unknown_receiver_user_method_resolves() {
    let src = r#"
struct P { x: int }
impl P {
    fn set(self, v: int) { self.x = v }
    fn get(self) { return self.x }
}
fn f(p) { p.set(7)
    print(p.get()) }
fn main() { f(P { x: 0 }) }
"#;
    assert_eq!(out(src), "7\n");
}

/// A name that is neither a built-in method nor a method of any struct stays
/// `E2003` on an `Unknown` receiver, preserving the conservative contract.
#[test]
fn unknown_receiver_unknown_method_is_e2003() {
    assert_eq!(
        check("fn f(p) { p.nonexistent() }\nfn main() { }"),
        Err(codes::UNDEFINED)
    );
    // A user method whose name matches a built-in name is still accepted.
    assert_eq!(
        check("struct P { x: int }\nimpl P { fn get(self) { return self.x } }\nfn f(p) { p.get() }\nfn main() { }"),
        Ok(())
    );
}

/// For a union receiver, a method is available only when every member provides
/// it with a compatible signature (`LANGUAGE_SPEC.md` §17.6).
#[test]
fn union_method_requires_compatible_signatures() {
    let same = r#"
struct A { x: int }
struct B { y: int }
impl A { fn m(self) { return self.x } }
impl B { fn m(self) { return self.y } }
type U = A | B
fn main() { let u: U = A { x: 1 }
    print(u.m()) }
"#;
    assert_eq!(out(same), "1\n");

    let same_args = r#"
struct A { x: int }
struct B { y: int }
impl A { fn m(self, n: int) { return n } }
impl B { fn m(self, n: int) { return n } }
type U = A | B
fn main() { let u: U = A { x: 1 }
    print(u.m(5)) }
"#;
    assert_eq!(out(same_args), "5\n");

    let arity = r#"
struct A { x: int }
struct B { y: int }
impl A { fn m(self, n: int) { return n } }
impl B { fn m(self) { return self.y } }
type U = A | B
fn main() { let u: U = A { x: 1 }
    print(u.m()) }
"#;
    assert_eq!(check(arity), Err(codes::TYPE_MISMATCH));

    let param_ty = r#"
struct A { x: int }
struct B { y: int }
impl A { fn m(self, n: int) { return n } }
impl B { fn m(self, n: float) { return n } }
type U = A | B
fn main() { let u: U = A { x: 1 }
    print(u.m(1)) }
"#;
    assert_eq!(check(param_ty), Err(codes::TYPE_MISMATCH));

    let ret_ty = r#"
struct A { x: int }
struct B { y: int }
impl A { fn m(self) -> int { return self.x } }
impl B { fn m(self) -> string { return "s" } }
type U = A | B
fn main() { let u: U = A { x: 1 }
    print(u.m()) }
"#;
    assert_eq!(check(ret_ty), Err(codes::TYPE_MISMATCH));

    // A method missing on one member remains E2003.
    let missing = r#"
struct A { x: int }
struct B { y: int }
impl A { fn m(self) { return self.x } }
type U = A | B
fn main() { let u: U = A { x: 1 }
    print(u.m()) }
"#;
    assert_eq!(check(missing), Err(codes::UNDEFINED));
}

/// The contextual rule recognizes `impl Struct {` only at item position.
/// `impl` used as an identifier is not a behavior block, and a behavior block
/// still parses when `impl` names an ordinary binding elsewhere in the file.
#[test]
fn impl_is_contextual() {
    assert_eq!(out("fn main() { let impl = 1\n print(impl) }"), "1\n");
    assert_eq!(
        out("struct P { x: int }\nimpl P { fn g(self) { return self.x } }\nlet impl = 2\nfn main() { print(P { x: 5 }.g())\n print(impl) }"),
        "5\n2\n"
    );
    // `impl` as a function name is unaffected.
    assert_eq!(
        out("fn impl(x) { return x }\nfn main() { print(impl(3)) }"),
        "3\n"
    );
}
