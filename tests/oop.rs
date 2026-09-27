#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! OOP-completion regressions: the four-pillar model, and the defects the
//! audit corrected.
//!
//! These lock the phase's decisions (`docs/OOP.md`): encapsulation is struct
//! data plus deterministic member lookup and mutation capability; abstraction
//! is traits; reuse is composition plus traits, never inheritance; and
//! polymorphism is overloading plus static union resolution. They also cover
//! the corrections: multiple `impl` blocks merge into one overload set, a
//! return-annotation diagnostic points at the annotation, and a trait method
//! cannot be overloaded inside one trait.

use aura::error::codes;
use aura::run_source;

/// Run a program that must succeed and return its stdout.
fn ok(src: &str) -> String {
    run_source(src, "<oop>").expect("expected the program to run")
}

/// The first diagnostic for a rejected program.
fn diag(src: &str) -> aura::error::Diag {
    run_source(src, "<oop>").expect_err("expected the program to be rejected")
}

/// The first diagnostic code for a rejected program.
fn code(src: &str) -> u16 {
    diag(src).code
}

// ------------------------------------------------------- multiple impl blocks

/// Several `impl` blocks for one struct merge into one method surface; a later
/// block may add an overload of the same name.
#[test]
fn impl_blocks_merge_and_may_add_overloads() {
    assert_eq!(
        ok("struct P { n: int }\nimpl P { fn f(self, x: int) { print(\"i\") } }\nimpl P { fn f(self, x: string) { print(\"s\") } }\nfn main() { P { n: 0 }.f(1)\n P { n: 0 }.f(\"a\") }"),
        "i\ns\n"
    );
    // Distinct method names across blocks likewise merge.
    assert_eq!(
        ok("struct P { n: int }\nimpl P { fn a(self) -> int { return 1 } }\nimpl P { fn b(self) -> int { return 2 } }\nfn main() { print(P { n: 0 }.a() + P { n: 0 }.b()) }"),
        "3\n"
    );
}

/// The same overload identity across two blocks is a duplicate, not a second
/// overload.
#[test]
fn duplicate_identity_across_impl_blocks_is_rejected() {
    assert_eq!(
        code("struct P { n: int }\nimpl P { fn f(self, x: int) { print(1) } }\nimpl P { fn f(self, x: int) { print(2) } }\nfn main() { P { n: 0 }.f(1) }"),
        codes::REDECLARED
    );
}

// ------------------------------------------------ trait + inherent overloads

/// An inherent overload with a different identity may coexist with a
/// trait-provided method of the same name; only an identical identity collides.
#[test]
fn inherent_and_trait_methods_share_one_namespace_by_identity() {
    assert_eq!(
        ok("struct S { n: int }\ntrait T { fn m(self) -> int }\nimpl T for S { fn m(self) -> int { return 1 } }\nimpl S { fn m(self, x: int) -> int { return x } }\nfn main() { print(S { n: 0 }.m())\n print(S { n: 0 }.m(5)) }"),
        "1\n5\n"
    );
    // Same identity from a trait and an inherent block is a collision.
    assert_eq!(
        code("struct S { n: int }\ntrait T { fn m(self) -> int }\nimpl T for S { fn m(self) -> int { return 1 } }\nimpl S { fn m(self) -> int { return 2 } }\nfn main() { print(1) }"),
        codes::REDECLARED
    );
}

/// A trait's own methods are single signatures: a trait implementation may not
/// add a second overload of a trait method.
#[test]
fn trait_method_cannot_be_overloaded_in_its_implementation() {
    assert_eq!(
        code("struct S { n: int }\ntrait T { fn m(self) -> int }\nimpl T for S { fn m(self) -> int { return 1 }\n fn m(self, x: int) -> int { return x } }\nfn main() { print(1) }"),
        codes::REDECLARED
    );
}

// -------------------------------------------------- return-annotation spans

/// An unknown return type reports at the annotation, not at the item or the
/// start of the file.
#[test]
fn unknown_return_type_points_at_the_annotation() {
    let src = "fn f() -> Zed { return 1 }\nfn main() { print(1) }";
    let d = diag(src);
    assert_eq!(d.code, codes::UNKNOWN_TYPE);
    assert!(
        d.span.start >= src.find("->").expect("annotation present"),
        "span {:?} should point at the return annotation",
        d.span
    );
    assert_ne!(d.span.start, 0, "span should not be the start of file");
}

/// The same precision holds for a method's and a trait method's return
/// annotation.
#[test]
fn method_and_trait_return_annotations_point_at_the_type() {
    let method =
        "struct S { n: int }\nimpl S { fn m(self) -> Zed { return 1 } }\nfn main() { print(1) }";
    let d = diag(method);
    assert_eq!(d.code, codes::UNKNOWN_TYPE);
    assert!(
        d.span.start >= method.find("->").expect("annotation present"),
        "method span {:?}",
        d.span
    );

    let trait_decl = "struct S { n: int }\ntrait T { fn m(self) -> Zed }\nfn main() { print(1) }";
    let d = diag(trait_decl);
    assert_eq!(d.code, codes::UNKNOWN_TYPE);
    assert!(
        d.span.start >= trait_decl.find("->").expect("annotation present"),
        "trait span {:?}",
        d.span
    );
}

// ------------------------------------------------- encapsulation + mutation

/// A method that mutates its receiver must declare `mut self`; otherwise the
/// body is rejected, and a mutating method needs a `mut`-reachable caller.
#[test]
fn encapsulation_keeps_mutation_capability_separate() {
    assert_eq!(
        code("struct S { n: int }\nimpl S { fn bump(self) { self.n = self.n + 1 } }\nfn main() { let mut s = S { n: 0 }\n s.bump() }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        code("struct S { n: int }\nimpl S { fn bump(mut self) { self.n = self.n + 1 } }\nfn main() { let s = S { n: 0 }\n s.bump() }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        ok("struct S { n: int }\nimpl S { fn bump(mut self) { self.n = self.n + 1 } }\nfn main() { let mut s = S { n: 0 }\n s.bump()\n print(s.n) }"),
        "1\n"
    );
}

/// A field and a method of one struct may not share a name; both namespaces
/// stay unambiguous.
#[test]
fn field_and_method_names_cannot_collide() {
    assert_eq!(
        code("struct S { m: int }\ntrait T { fn m(self) -> int }\nimpl T for S { fn m(self) -> int { return 1 } }\nfn main() { print(1) }"),
        codes::DUPLICATE_FIELD
    );
}

// ------------------------------------------------ reuse without inheritance

/// Reuse is composition: a method reaches a nested struct's method through a
/// field, and no struct is ever a subtype of another.
#[test]
fn reuse_is_composition_not_inheritance() {
    assert_eq!(
        ok("struct Engine { power: int }\nimpl Engine { fn describe(self) -> string { return to_string(self.power) } }\nstruct Car { engine: Engine }\nimpl Car { fn describe(self) -> string { return self.engine.describe() } }\nfn main() { print(Car { engine: Engine { power: 120 } }.describe()) }"),
        "120\n"
    );
}

// -------------------------------------------------------- polymorphism forms

/// Ad-hoc polymorphism: overloading resolves by argument type, most specific
/// first; an unannotated overload is the fallback.
#[test]
fn overloading_is_ad_hoc_polymorphism() {
    assert_eq!(
        ok("fn f(x: int) { print(\"i\") }\nfn f(x: string) { print(\"s\") }\nfn f(x) { print(\"any\") }\nfn main() { f(1)\n f(\"a\")\n f(true) }"),
        "i\ns\nany\n"
    );
}

/// Static polymorphism: a union receiver exposes a method only when every
/// member provides it compatibly.
#[test]
fn union_receiver_requires_every_member() {
    assert_eq!(
        ok("struct A { x: int }\nstruct B { y: int }\nimpl A { fn m(self) -> int { return self.x } }\nimpl B { fn m(self) -> int { return self.y } }\nfn main() { let u: A | B = A { x: 7 }\n print(u.m()) }"),
        "7\n"
    );
    assert_eq!(
        code("struct A { x: int }\nstruct B { y: int }\nimpl A { fn m(self) -> int { return self.x } }\nfn main() { let u: A | B = A { x: 7 }\n print(u.m()) }"),
        codes::UNDEFINED
    );
}
