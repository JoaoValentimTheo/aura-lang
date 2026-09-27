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
