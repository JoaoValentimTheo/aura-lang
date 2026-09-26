#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Aura OOP V2 — traits (`LANGUAGE_SPEC.md` §17.7).
//!
//! A trait is a behavioral contract: a named set of method signatures that a
//! nominal struct implements with `impl Trait for Struct`. Trait-provided
//! methods merge into the struct's single method namespace and are resolved
//! statically by nominal type — there is no dynamic dispatch, no trait object,
//! and no trait value.

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

/// A trait implementation satisfies the contract and its methods are callable.
#[test]
fn valid_trait_implementation_runs() {
    let src = r#"
trait Printable { fn print(self) }
struct Person { name: string }
impl Printable for Person {
    fn print(self) { print(self.name) }
}
fn main() { Person { name: "Ada" }.print() }
"#;
    assert_eq!(out(src), "Ada\n");
}

/// A trait may declare parameters and a return type; the implementation is
/// checked against them and is callable.
#[test]
fn trait_with_parameters_and_return() {
    let src = r#"
trait Sized {
    fn size(self) -> int
    fn scaled(self, k: int) -> int
}
struct Box { v: int }
impl Sized for Box {
    fn size(self) { return self.v }
    fn scaled(self, k: int) { return self.v * k }
}
fn main() { let b = Box { v: 3 }
    print(b.size())
    print(b.scaled(4)) }
"#;
    assert_eq!(out(src), "3\n12\n");
}

/// A trait method may read fields, mutate them through reference semantics,
/// and call another method of the same struct.
#[test]
fn trait_method_reads_mutates_and_delegates() {
    let src = r#"
trait Counter {
    fn bump(self)
    fn value(self) -> int
}
struct C { n: int }
impl C { fn base(self) -> int { return self.n } }
impl Counter for C {
    fn bump(self) { self.n = self.base() + 1 }
    fn value(self) -> int { return self.base() }
}
fn main() { let c = C { n: 0 }
    c.bump()
    c.bump()
    print(c.value()) }
"#;
    assert_eq!(out(src), "2\n");
}

/// A missing trait method is `E2017`.
#[test]
fn missing_trait_method_is_e2017() {
    let src = r#"
trait T { fn a(self)
    fn b(self) }
struct S { x: int }
impl T for S { fn a(self) { print(self.x) } }
fn main() { print(1) }
"#;
    assert_eq!(check(src), Err(codes::TRAIT_INCOMPLETE));
}

/// A signature that disagrees on arity, a parameter type, or the return type
/// is `E3001`.
#[test]
fn incompatible_trait_signature_is_e3001() {
    let arity = "trait T { fn a(self, x: int) }\nstruct S { x: int }\nimpl T for S { fn a(self) { print(self.x) } }\nfn main() { print(1) }";
    assert_eq!(check(arity), Err(codes::TYPE_MISMATCH));
    let param = "trait T { fn a(self, x: int) }\nstruct S { x: int }\nimpl T for S { fn a(self, y: string) { print(y) } }\nfn main() { print(1) }";
    assert_eq!(check(param), Err(codes::TYPE_MISMATCH));
    let ret = "trait T { fn a(self) -> int }\nstruct S { x: int }\nimpl T for S { fn a(self) -> string { return \"x\" } }\nfn main() { print(1) }";
    assert_eq!(check(ret), Err(codes::TYPE_MISMATCH));
}

/// A trait implementation may not add methods the trait does not declare.
#[test]
fn trait_impl_rejects_extra_methods() {
    let src = r#"
trait T { fn a(self) }
struct S { x: int }
impl T for S {
    fn a(self) { print(self.x) }
    fn extra(self) { print(2) }
}
fn main() { print(1) }
"#;
    assert_eq!(check(src), Err(codes::UNDEFINED));
}

/// Unknown trait, duplicate trait declaration, and duplicate implementation
/// are rejected in the declaration-error family.
#[test]
fn trait_declaration_errors() {
    assert_eq!(
        check("struct S { x: int }\nimpl Nope for S { fn a(self) { print(1) } }\nfn main() { print(1) }"),
        Err(codes::UNDEFINED)
    );
    assert_eq!(
        check("trait T { fn a(self) }\ntrait T { fn b(self) }\nfn main() { print(1) }"),
        Err(codes::REDECLARED)
    );
    assert_eq!(
        check("trait T { fn a(self) }\nstruct S { x: int }\nimpl T for S { fn a(self) { print(1) } }\nimpl T for S { fn a(self) { print(2) } }\nfn main() { print(1) }"),
        Err(codes::REDECLARED)
    );
    // A trait method name declared twice in one trait is a redefinition.
    assert_eq!(
        check("trait T { fn a(self)\n fn a(self) }\nfn main() { print(1) }"),
        Err(codes::REDECLARED)
    );
}

/// A trait implementation target must be a nominal struct; an enum, alias,
/// builtin, or trait name is rejected through the existing non-struct path.
#[test]
fn trait_impl_target_must_be_a_struct() {
    assert_eq!(
        check(
            "trait T { fn a(self) }\nenum E { A }\nimpl T for E { fn a(self) { print(1) } }\nfn main() { print(1) }"
        ),
        Err(codes::UNDEFINED)
    );
    assert_eq!(
        check(
            "trait T { fn a(self) }\ntype E = int\nimpl T for E { fn a(self) { print(1) } }\nfn main() { print(1) }"
        ),
        Err(codes::UNDEFINED)
    );
    assert_eq!(
        check(
            "trait T { fn a(self) }\nimpl T for int { fn a(self) { print(1) } }\nfn main() { print(1) }"
        ),
        Err(codes::UNDEFINED)
    );
    assert_eq!(
        check(
            "trait T { fn a(self) }\nimpl T for T { fn a(self) { print(1) } }\nfn main() { print(1) }"
        ),
        Err(codes::UNDEFINED)
    );
    // A declared struct that is simply absent is still the existing unknown
    // target diagnostic.
    assert_eq!(
        check("trait T { fn a(self) }\nimpl T for Nope { fn a(self) { print(1) } }\nfn main() { print(1) }"),
        Err(codes::UNDEFINED)
    );
}

/// Trait and inherent methods share one namespace: a name provided by both,
/// or by two traits, is rejected.
#[test]
fn one_member_namespace() {
    let inherent = r#"
trait T { fn a(self) }
struct S { x: int }
impl S { fn a(self) { print(1) } }
impl T for S { fn a(self) { print(2) } }
fn main() { print(1) }
"#;
    assert_eq!(check(inherent), Err(codes::REDECLARED));
    let two_traits = r#"
trait T1 { fn a(self) }
trait T2 { fn a(self) }
struct S { x: int }
impl T1 for S { fn a(self) { print(1) } }
impl T2 for S { fn a(self) { print(2) } }
fn main() { print(1) }
"#;
    assert_eq!(check(two_traits), Err(codes::REDECLARED));
    // A field colliding with a trait-provided method is a duplicate member.
    let field = r#"
trait T { fn x(self) }
struct S { x: int }
impl T for S { fn x(self) { print(1) } }
fn main() { print(1) }
"#;
    assert_eq!(check(field), Err(codes::DUPLICATE_FIELD));
}

/// Traits are hoisted: source order of trait, struct, and impl does not
/// matter within a module.
#[test]
fn trait_declarations_are_order_independent() {
    assert_eq!(out("trait T { fn a(self) }\nstruct S { n: int }\nimpl T for S { fn a(self) { print(self.n) } }\nfn main() { S { n: 1 }.a() }"), "1\n");
    assert_eq!(out("trait T { fn a(self) }\nimpl T for S { fn a(self) { print(self.n) } }\nstruct S { n: int }\nfn main() { S { n: 2 }.a() }"), "2\n");
    assert_eq!(out("struct S { n: int }\nimpl T for S { fn a(self) { print(self.n) } }\ntrait T { fn a(self) }\nfn main() { S { n: 3 }.a() }"), "3\n");
}

/// An alias is transparent, so it preserves trait-provided methods.
#[test]
fn alias_preserves_trait_methods() {
    let src = r#"
trait T { fn a(self) }
struct S { n: int }
impl T for S { fn a(self) { print(self.n) } }
type Q = S
fn main() { let q: Q = S { n: 5 }
    q.a() }
"#;
    assert_eq!(out(src), "5\n");
}

/// A union receiver sees a trait-provided method only when every member
/// provides it with a compatible signature.
#[test]
fn union_receiver_trait_method() {
    let both = r#"
trait T { fn a(self) -> int }
struct A { x: int }
struct B { y: int }
impl T for A { fn a(self) -> int { return self.x } }
impl T for B { fn a(self) -> int { return self.y } }
type U = A | B
fn main() { let u: U = A { x: 1 }
    print(u.a()) }
"#;
    assert_eq!(out(both), "1\n");
    let one = r#"
trait T { fn a(self) -> int }
struct A { x: int }
struct B { y: int }
impl T for A { fn a(self) -> int { return self.x } }
type U = A | B
fn main() { let u: U = A { x: 1 }
    print(u.a()) }
"#;
    assert_eq!(check(one), Err(codes::UNDEFINED));
}

/// A trait-declared method counts for the conservative unknown-receiver rule.
#[test]
fn trait_method_on_unknown_receiver() {
    let src = r#"
trait T { fn a(self) }
struct S { n: int }
impl T for S { fn a(self) { print(self.n) } }
fn call(x) { x.a() }
fn main() { call(S { n: 4 }) }
"#;
    assert_eq!(out(src), "4\n");
    // A name that exists nowhere is still `E2003`.
    assert_eq!(
        check("fn f(x) { x.nope() }\nfn main() { }"),
        Err(codes::UNDEFINED)
    );
}

/// Traits are contextual words; `trait` remains an ordinary identifier.
#[test]
fn trait_is_contextual() {
    assert_eq!(out("fn main() { let trait = 1\n print(trait) }"), "1\n");
    assert_eq!(
        out("fn trait(x) { return x }\nfn main() { print(trait(3)) }"),
        "3\n"
    );
}

/// Malformed trait forms are deterministic parse errors, not unrelated
/// expression parses.
#[test]
fn malformed_trait_forms_are_parse_errors() {
    // A trait method with a body is rejected (no default bodies in V1).
    assert_eq!(
        parse("trait T { fn a(self) { print(1) } }\nfn main() { print(1) }")
            .map(|_| ())
            .map_err(|d| d.code),
        Err(codes::EXPECTED)
    );
    // A field (or any non-`fn` member) in a trait is rejected.
    assert_eq!(
        parse("trait T { x: int }\nfn main() { print(1) }")
            .map(|_| ())
            .map_err(|d| d.code),
        Err(codes::EXPECTED)
    );
    // A trait method without the `self` receiver is rejected.
    assert_eq!(
        parse("trait T { fn a() }\nfn main() { print(1) }")
            .map(|_| ())
            .map_err(|d| d.code),
        Err(codes::EXPECTED)
    );
    // `impl Trait P {}` (missing `for`) is a parse error.
    assert_eq!(
        parse("trait T { fn a(self) }\nstruct S { x: int }\nimpl T S { }\nfn main() { print(1) }")
            .map(|_| ())
            .map_err(|d| d.code),
        Err(codes::EXPECTED)
    );
    // `impl T for {}` (missing target) is a parse error.
    assert_eq!(
        parse("trait T { fn a(self) }\nimpl T for { }\nfn main() { print(1) }")
            .map(|_| ())
            .map_err(|d| d.code),
        Err(codes::EXPECTED)
    );
}

/// An empty trait is legal; implementing it requires nothing.
#[test]
fn empty_trait_is_legal() {
    assert_eq!(
        out("trait Empty { }\nstruct S { x: int }\nimpl Empty for S { }\nfn main() { print(1) }"),
        "1\n"
    );
}
