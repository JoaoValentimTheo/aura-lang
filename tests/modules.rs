#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Modules and visibility (`LANGUAGE_SPEC.md` §28).
//!
//! Aura modules are in-source (`module Name { ... }`), the only module model
//! that preserves Native↔WASM parity because the WebAssembly host has no
//! filesystem. These tests lock the resolved semantics: a real boundary
//! (private by default, `pub` to export), qualified access, `use` imports,
//! nesting, and the strict separation of visibility from mutation.

use aura::error::codes;
use aura::run_source;

fn ok(src: &str) -> String {
    run_source(src, "<modules>").expect("expected the program to run")
}

fn code(src: &str) -> u16 {
    run_source(src, "<modules>")
        .expect_err("expected the program to be rejected")
        .code
}

// ----------------------------------------------------------- the boundary

/// A `pub` item is reachable by path; a private one is `E2018`.
#[test]
fn module_is_a_real_visibility_boundary() {
    assert_eq!(
        ok("module m { pub fn f() -> int { return 7 } }\nfn main() { print(m::f()) }"),
        "7\n"
    );
    assert_eq!(
        code("module m { fn f() -> int { return 7 } }\nfn main() { print(m::f()) }"),
        codes::PRIVATE_ACCESS
    );
    // A private item is usable from within its own module.
    assert_eq!(
        ok("module m { fn g() -> int { return 9 }\n pub fn f() -> int { return g() } }\nfn main() { print(m::f()) }"),
        "9\n"
    );
    // ... and from a descendant module.
    assert_eq!(
        ok("module a { fn hidden() -> int { return 5 }\n module b { pub fn f() -> int { return hidden() } } }\nfn main() { print(a::b::f()) }"),
        "5\n"
    );
}

/// A module may be nested; a path walks the hierarchy.
#[test]
fn modules_nest() {
    assert_eq!(
        ok("module a { module b { module c { pub fn f() -> int { return 42 } } } }\nfn main() { print(a::b::c::f()) }"),
        "42\n"
    );
}

/// Two modules may declare the same name without collision.
#[test]
fn sibling_modules_do_not_collide() {
    assert_eq!(
        ok("module a { pub fn f() -> int { return 1 } }\nmodule b { pub fn f() -> int { return 2 } }\nfn main() { print(a::f() + b::f()) }"),
        "3\n"
    );
    assert_eq!(
        ok("module a { pub struct S { pub x: int } }\nmodule b { pub struct S { pub x: int } }\nfn main() { print(a::S { x: 1 }.x + b::S { x: 2 }.x) }"),
        "3\n"
    );
}

// ------------------------------------------------------------- imports

/// `use` binds an imported name in the current module, with `as` renaming.
#[test]
fn use_binds_and_aliases_imports() {
    assert_eq!(
        ok("module m { pub fn f() -> int { return 2 } }\nuse m::f\nfn main() { print(f()) }"),
        "2\n"
    );
    assert_eq!(
        ok("module m { pub fn f() -> int { return 2 } }\nuse m::f as g\nfn main() { print(g()) }"),
        "2\n"
    );
    // A forward reference works regardless of source order.
    assert_eq!(
        ok("use m::f\nmodule m { pub fn f() -> int { return 3 } }\nfn main() { print(f()) }"),
        "3\n"
    );
    // Importing a private item is a visibility error.
    assert_eq!(
        code("module m { fn f() -> int { return 2 } }\nuse m::f\nfn main() { print(f()) }"),
        codes::PRIVATE_ACCESS
    );
    // Importing something that does not exist is E2019.
    assert_eq!(
        code("use nope\nfn main() { print(1) }"),
        codes::UNKNOWN_MODULE
    );
}

/// An import may not silently shadow a name already declared in the module.
#[test]
fn import_collision_is_rejected() {
    assert_eq!(
        code("module a { pub fn f() -> int { return 1 } }\nmodule b { pub fn f() -> int { return 2 } }\nuse a::f\nuse b::f\nfn main() { print(f()) }"),
        codes::REDECLARED
    );
    assert_eq!(
        code("module a { pub fn f() -> int { return 1 } }\nuse a::f\nfn f() -> int { return 2 }\nfn main() { print(f()) }"),
        codes::REDECLARED
    );
}

// -------------------------------------------------- field and method visibility

/// A struct field is private unless marked `pub`; it cannot be constructed,
/// read, or written from another module.
#[test]
fn fields_are_private_by_default() {
    assert_eq!(
        code("module m { pub struct S { x: int } }\nfn main() { let s = m::S { x: 1 }\n print(s.x) }"),
        codes::PRIVATE_ACCESS
    );
    assert_eq!(
        ok("module m { pub struct S { pub x: int } }\nfn main() { let s = m::S { x: 1 }\n print(s.x) }"),
        "1\n"
    );
    // A private field is reachable through a method or factory declared in the
    // module; construction itself can only happen inside the module.
    assert_eq!(
        ok("module m { pub struct S { x: int }\n pub fn make() -> S { return S { x: 9 } }\n impl S { pub fn get(self) -> int { return self.x } } }\nfn main() { let s = m::make()\n print(s.get()) }"),
        "9\n"
    );
    // A private field cannot be written from outside even through `mut`.
    assert_eq!(
        code("module m { pub struct S { pub x: int, y: int } }\nfn main() { let mut s = m::S { x: 1, y: 2 }\n s.y = 3 }"),
        codes::PRIVATE_ACCESS
    );
}

/// A method is private unless marked `pub`.
#[test]
fn methods_are_private_by_default() {
    assert_eq!(
        code("module m { pub struct S { x: int }\n impl S { fn get(self) -> int { return self.x } } }\nfn main() { let s = m::S { x: 1 }\n print(s.get()) }"),
        codes::PRIVATE_ACCESS
    );
    assert_eq!(
        ok("module m { pub struct S { x: int }\n pub fn make() -> S { return S { x: 1 } }\n impl S { pub fn get(self) -> int { return self.x } } }\nfn main() { let s = m::make()\n print(s.get()) }"),
        "1\n"
    );
    // A root-level struct's methods stay reachable as before.
    assert_eq!(
        ok("struct S { x: int }\nimpl S { fn get(self) -> int { return self.x } }\nfn main() { print(S { x: 5 }.get()) }"),
        "5\n"
    );
}

/// Overload visibility is per-overload: an inaccessible overload is never a
/// candidate, so it cannot be selected, and a matching private overload is
/// `E2018` rather than an undefined name.
#[test]
fn overload_visibility_is_per_overload() {
    assert_eq!(
        ok("module m { pub fn f(x: int) -> int { return 1 }\n pub fn f(x: string) -> int { return 2 } }\nfn main() { print(m::f(\"a\")) }"),
        "2\n"
    );
    assert_eq!(
        code("module m { pub fn f(x: int) -> int { return 1 }\n fn f(x: string) -> int { return 2 } }\nfn main() { print(m::f(\"a\")) }"),
        codes::PRIVATE_ACCESS
    );
}

// -------------------------------------------- visibility vs mutation

/// Visibility and mutation capability are independent: a public mutating
/// method still needs a mutable receiver, and a mutable binding cannot reach a
/// private field.
#[test]
fn visibility_never_grants_mutation() {
    assert_eq!(
        code("module m { pub struct S { pub n: int }\n impl S { pub fn inc(mut self) { self.n = self.n + 1 } } }\nfn main() { let s = m::S { n: 0 }\n s.inc() }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        ok("module m { pub struct S { pub n: int }\n impl S { pub fn inc(mut self) { self.n = self.n + 1 } } }\nfn main() { let mut s = m::S { n: 0 }\n s.inc()\n print(s.n) }"),
        "1\n"
    );
    assert_eq!(
        code("module m { pub struct S { n: int } }\nfn main() { let mut s = m::S { n: 0 }\n s.n = 1 }"),
        codes::PRIVATE_ACCESS
    );
}

/// A closure inherits the access of the lexical environment where its
/// references were resolved; it cannot escalate privilege.
#[test]
fn closures_do_not_escalate_visibility() {
    assert_eq!(
        ok("module m { fn hidden() -> int { return 5 }\n pub fn run() -> int { let f = () -> hidden()\n return f() } }\nfn main() { print(m::run()) }"),
        "5\n"
    );
    assert_eq!(
        code("module m { fn hidden() -> int { return 5 } }\nfn main() { let f = () -> m::hidden()\n print(f()) }"),
        codes::PRIVATE_ACCESS
    );
}

// --------------------------------------------------------- REPL-style errors

/// A `pub` on an `impl` block is meaningless and rejected.
#[test]
fn pub_impl_is_rejected() {
    assert_eq!(
        code("struct S { x: int }\npub impl S { fn f(self) -> int { return 1 } }\nfn main() { print(1) }"),
        codes::EXPECTED
    );
}

// ------------------------------------------------------------ determinism

/// When several modules carry duplicate declarations, the reported diagnostic
/// is always the same one, never dependent on hash iteration order. This locks
/// the fix for a real nondeterminism the module fuzzer found: iteration over
/// the function table (a `HashMap`) previously decided which duplicate won.
#[test]
fn duplicate_diagnostics_are_deterministic() {
    let src = "module m0 {\n fn f() -> int { return 1 }\n fn f() -> int { print(1) }\n}\nmodule m1 {\n fn f() -> int { print(1) }\n fn f() -> int { return 1 }\n}\nfn main() { print(1) }";
    let first = run_source(src, "<modules>").expect_err("duplicate must be rejected");
    assert_eq!(first.code, codes::REDECLARED);
    // The same span every time, across many runs (HashMap order varies per
    // process, so a stable result is the determinism guarantee).
    for _ in 0..25 {
        let again = run_source(src, "<modules>").expect_err("duplicate must be rejected");
        assert_eq!(again.span, first.span, "duplicate diagnostic span drifted");
    }
}

// ------------------------------------------- qualified enum-variant paths

/// An enum variant is named by its enum's path plus the tag
/// (`shapes::Color::Red`) or by the module path plus the tag
/// (`shapes::Red`) — both denote the same variant (`LANGUAGE_SPEC.md` §27).
/// The enum-path form must resolve for a root enum (`E::A`), a module enum
/// (`m::E::A`), a nested module enum, and an imported enum, in both
/// expression construction and pattern position, including zero-payload
/// variants. A 2-segment `Enum::Tag` path previously fell through to `E3002`.
#[test]
fn enum_path_variants_resolve_in_every_position() {
    // Root enum: expression and pattern, payload and zero-payload.
    assert_eq!(
        ok("enum E { A(int) }\nfn main() { print(E::A(1)) }"),
        "A(1)\n"
    );
    assert_eq!(
        ok("enum E { Red, Green }\nfn main() { print(match E::Red() { E::Red -> 1\n E::Green -> 2 }) }"),
        "1\n"
    );
    assert_eq!(
        ok("enum E { A(int) }\nfn main() { print(match A(1) { E::A(x) -> x }) }"),
        "1\n"
    );
    // Module enum: the enum path may be module-qualified or two segments.
    assert_eq!(
        ok("module m { pub enum E { A(int) } }\nfn main() { print(m::E::A(1)) }"),
        "m::A(1)\n"
    );
    assert_eq!(
        ok("module m { pub enum E { Red } }\nfn main() { print(match m::Red() { m::E::Red -> 1 }) }"),
        "1\n"
    );
    // Nested module enum.
    assert_eq!(
        ok("module a { pub module b { pub enum E { A(int) } } }\nfn main() { print(a::b::E::A(1)) }"),
        "a::b::A(1)\n"
    );
    // Imported enum: the enum's bare name names the type.
    assert_eq!(
        ok("module m { pub enum E { A(int) } }\nuse m::E\nfn main() { print(E::A(1)) }"),
        "m::A(1)\n"
    );
    // An unknown enum path is still undefined, never silently accepted.
    assert_eq!(code("fn main() { print(E::A(1)) }"), codes::UNKNOWN_TYPE);
}

/// `use Enum::Tag` imports the variant directly (`LANGUAGE_SPEC.md` §27 names
/// the variant by its enum's path plus the tag). This is the same item as
/// `use module::Tag`; importing a private enum is `E2018`.
#[test]
fn use_enum_variant_path_resolves() {
    assert_eq!(
        ok("module shapes { pub enum Color { Red(int) } }\nuse shapes::Color::Red\nfn main() { print(Red(1)) }"),
        "shapes::Red(1)\n"
    );
    assert_eq!(
        ok("module m { pub enum E { A(int) } }\nuse m::E::A\nfn main() { print(A(1)) }"),
        "m::A(1)\n"
    );
    assert_eq!(
        code("module m { enum E { A(int) } }\nuse m::E::A\nfn main() {}"),
        codes::PRIVATE_ACCESS
    );
}

/// A type and a variant may share a local name (they are separate
/// namespaces, §26). Resolution must be **deterministic and order-
/// independent**: previously `m::E::A(1)` built the struct `m::A` when the
/// enum was declared first, but failed with `E3002` when the struct was
/// declared first. Now the collision is rejected identically in both orders,
/// because the flat canonical name `m::A` cannot denote both a struct and a
/// variant downstream. (The namespace question itself is a SPEC GAP recorded
/// in `docs/CONFORMANCE_PHASE2.md`; this test locks only the determinism and
/// the absence of a silent wrong construction.)
#[test]
fn struct_and_variant_same_name_resolve_deterministically() {
    let enum_first =
        "module m { pub enum E { A(int) }\n pub struct A { pub x: int } }\nfn main() { print(m::E::A(1)) }";
    let struct_first =
        "module m { pub struct A { pub x: int }\n pub enum E { A(int) } }\nfn main() { print(m::E::A(1)) }";
    // Both orders are rejected with the same code, never a silent struct.
    assert_eq!(code(enum_first), codes::UNKNOWN_TYPE);
    assert_eq!(code(struct_first), codes::UNKNOWN_TYPE);
    for _ in 0..25 {
        assert_eq!(code(enum_first), code(struct_first));
    }
    // With no collision the enum-path variant resolves in expression and
    // pattern position.
    assert_eq!(
        ok("module m { pub enum E { A(int) } }\nfn main() { print(m::E::A(1)) }"),
        "m::A(1)\n"
    );
    // The unqualified module-path spelling names the type, so with a struct of
    // that name it constructs the struct — order-independent, as before.
    assert_eq!(
        ok("module m { pub enum E { A(int) }\n pub struct A { pub x: int } }\nfn main() { print(m::A(1)) }"),
        "m::A { x: 1 }\n"
    );
}

// --------------------------------------------------- diagnostics carry spans

/// A private type named in a *type position* is reported at the declaration
/// that mentions it, not at the file start. The resolver previously passed a
/// default span for every `TypeExpr` (which carries no span of its own), so
/// `fn g() -> a::S` reported `E2018` at `1:1` (`CONF-RESOLVE-4`).
#[test]
fn private_type_in_type_position_reports_its_own_span() {
    let cases = [
        "module a { struct S { pub x: int } }\nmodule b { fn g() -> a::S { return a::S { x: 1 } } }\nfn main() { }",
        "module a { struct S { pub x: int } }\nmodule b { fn g(x: a::S) -> int { return 1 } }\nfn main() { }",
        "module a { struct S { pub x: int } }\nmodule b { struct T { f: a::S } }\nfn main() { }",
        "module a { struct S { pub x: int } }\nmodule b { pub fn g() -> int { let v: a::S = a::S { x: 1 }\n return 1 } }\nfn main() { }",
    ];
    for src in cases {
        let d = run_source(src, "<modules>").expect_err("private type must be rejected");
        assert_eq!(d.code, codes::PRIVATE_ACCESS, "{src}");
        assert_ne!(
            d.span,
            aura::error::Span::default(),
            "{src}: span fell back to the file start"
        );
        // The reported span is inside the source, never past its end.
        assert!(
            d.span.start < src.len(),
            "{src}: span {:?} past EOF",
            d.span
        );
    }
}
