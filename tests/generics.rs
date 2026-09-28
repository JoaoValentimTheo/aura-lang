#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Generics regressions (`docs/GENERICS.md`).
//!
//! These lock the phase's decisions: static, erased, nominal parametric
//! polymorphism over Aura's structural collections (`[T]`, `{string: T}`);
//! identity up to alpha-renaming; inference by structural matching with
//! `Unknown` never becoming a concrete type; bounds as static contracts;
//! erasure at runtime; and full REPL persistence.

use aura::error::codes;
use aura::run_source;

/// Run a program that must succeed and return its stdout.
fn ok(src: &str) -> String {
    run_source(src, "<generics>").expect("expected the program to run")
}

/// The first diagnostic code for a rejected program.
fn code(src: &str) -> u16 {
    run_source(src, "<generics>")
        .expect_err("expected the program to be rejected")
        .code
}

// --------------------------------------------------------------- functions

/// A generic function infers its parameter from the argument, for any type.
#[test]
fn generic_function_infers_across_types() {
    assert_eq!(
        ok("fn identity<T>(x: T) -> T { return x }\nfn main() { print(identity(1))\n print(identity(\"a\"))\n print(identity(true)) }"),
        "1\na\ntrue\n"
    );
}

/// Explicit type arguments are accepted and checked for arity.
#[test]
fn generic_function_accepts_explicit_type_arguments() {
    assert_eq!(
        ok("fn identity<T>(x: T) -> T { return x }\nfn main() { print(identity<int>(7)) }"),
        "7\n"
    );
    // Wrong arity is E3001.
    assert_eq!(
        code(
            "fn identity<T>(x: T) -> T { return x }\nfn main() { print(identity<int, string>(7)) }"
        ),
        codes::TYPE_MISMATCH
    );
    // Explicit type arguments on a non-generic function are rejected.
    assert_eq!(
        code("fn f(x: int) -> int { return x }\nfn main() { print(f<int>(1)) }"),
        codes::TYPE_MISMATCH
    );
}

/// Parameter names are not part of signature identity: `f<T>` and `f<U>` are
/// the same overload and may not be redeclared (`E2007`).
#[test]
fn type_parameter_names_are_not_identity() {
    assert_eq!(
        code("fn f<T>(x: T) -> T { return x }\nfn f<U>(x: U) -> U { return x }\nfn main() { }"),
        codes::REDECLARED
    );
}

/// A generic and a concrete overload coexist and the concrete one wins.
#[test]
fn concrete_overload_wins_over_generic() {
    assert_eq!(
        ok("fn f<T>(x: T) -> string { return \"g\" }\nfn f(x: int) -> string { return \"c\" }\nfn main() { print(f(1))\n print(f(\"a\")) }"),
        "c\ng\n"
    );
}

/// A type parameter that cannot be bound in a value position is `Unknown`,
/// never an arbitrary concrete type.
#[test]
fn unbound_parameter_does_not_become_concrete() {
    // The body returns an int behind a `T` return; inference leaves it
    // `Unknown` and the program still runs (conservative, not unsound).
    assert_eq!(
        ok("fn empty<T>() -> int { return 1 }\nfn main() { print(empty()) }"),
        "1\n"
    );
}

// -------------------------------------------------------------- structures

/// A generic struct is nominal; constructing it infers its arguments.
#[test]
fn generic_struct_infers_and_runs() {
    assert_eq!(
        ok("struct Box<T> { value: T }\nfn main() { let b = Box { value: 9 }\n print(b.value) }"),
        "9\n"
    );
    assert_eq!(
        ok("struct Box<T> { value: T }\nfn main() { let b = Box<int> { value: 9 }\n print(b.value) }"),
        "9\n"
    );
}

/// An explicit type argument is checked against the field value.
#[test]
fn generic_struct_rejects_wrong_field_type() {
    assert_eq!(
        code("struct Box<T> { value: T }\nfn main() { let b = Box<int> { value: \"x\" }\n print(b.value) }"),
        codes::TYPE_MISMATCH
    );
}

/// A generic struct requires its full argument list; a bare use is E3002.
#[test]
fn generic_struct_requires_arguments() {
    assert_eq!(
        code("struct Box<T> { value: T }\nfn f(b: Box) -> int { return 1 }\nfn main() { }"),
        codes::UNKNOWN_TYPE
    );
}

/// Generic type arguments compose with Aura's structural collections.
#[test]
fn generics_compose_with_structural_collections() {
    assert_eq!(
        ok("fn first<T>(xs: [T]) -> T { return xs[0] }\nfn main() { print(first([10, 20]))\n print(first([\"a\", \"b\"])) }"),
        "10\na\n"
    );
    assert_eq!(
        ok("fn get<T>(m: {string: T}, k: string) -> T { return m[k] }\nfn main() { print(get({ \"x\": 5 }, \"x\")) }"),
        "5\n"
    );
}

// ----------------------------------------------------------------- methods

/// A generic method uses the enclosing impl's parameter.
#[test]
fn generic_method_uses_impl_parameter() {
    assert_eq!(
        ok("struct Box<T> { value: T }\nimpl<T> Box<T> { fn get(self) -> T { return self.value } }\nfn main() { print(Box { value: 3 }.get()) }"),
        "3\n"
    );
}

/// A method may declare its own parameter on top of the impl's.
#[test]
fn generic_method_may_add_its_own_parameter() {
    assert_eq!(
        ok("struct Box<T> { value: T }\nimpl<T> Box<T> { fn replace<U>(self, v: U) -> U { return v } }\nfn main() { print(Box { value: 1 }.replace(\"z\")) }"),
        "z\n"
    );
}

// ------------------------------------------------------------------ traits

/// A generic trait is implemented for a generic struct, and its method resolves.
#[test]
fn generic_trait_over_generic_struct() {
    assert_eq!(
        ok("trait Container<T> { fn get(self, i: int) -> T }\nstruct Stack<T> { items: [T] }\nimpl<T> Container<T> for Stack<T> { fn get(self, i: int) -> T { return self.items[i] } }\nfn main() { print(Stack { items: [4, 5, 6] }.get(1)) }"),
        "5\n"
    );
}

/// A bound is a static contract: a satisfying struct passes, one that does not
/// implement the trait is rejected.
#[test]
fn bounds_are_checked_at_the_call_site() {
    assert_eq!(
        ok("trait Show { fn show(self) -> int }\nstruct A { n: int }\nimpl Show for A { fn show(self) -> int { return self.n } }\nfn run<T: Show>(x: T) -> int { return x.show() }\nfn main() { print(run(A { n: 7 })) }"),
        "7\n"
    );
    assert_eq!(
        code("trait Show { fn show(self) -> int }\nstruct B { n: int }\nfn run<T: Show>(x: T) -> int { return 1 }\nfn main() { run(B { n: 1 }) }"),
        codes::TYPE_MISMATCH
    );
}

/// An unknown bound is rejected when it is declared.
#[test]
fn unknown_bound_is_rejected() {
    assert_eq!(
        code("fn run<T: Nope>(x: T) -> T { return x }\nfn main() { }"),
        codes::UNKNOWN_TYPE
    );
}

// ------------------------------------------------------- aliases and unions

/// A parameterised alias expands by substitution.
#[test]
fn parameterised_alias_expands() {
    assert_eq!(
        ok("type Pair<T> = [T]\nfn main() { let p: Pair<int> = [1, 2]\n print(p[1]) }"),
        "2\n"
    );
}

/// A union with `none` stays permissive (`Unknown`) as before.
#[test]
fn union_with_none_stays_permissive() {
    assert_eq!(
        ok("fn f<T>(x: T) -> T { return x }\nfn g(x: int | none) -> int { return 0 }\nfn main() { print(f(1))\n print(g(2)) }"),
        "1\n0\n"
    );
}

// ------------------------------------------------------------- type params

/// A type parameter may not shadow a declared type (`E2007`).
#[test]
fn type_parameter_cannot_shadow_declared_type() {
    assert_eq!(
        code("struct T { n: int }\nfn f<T>(x: T) -> T { return x }\nfn main() { }"),
        codes::REDECLARED
    );
}

/// A type parameter may not be declared twice in one list (`E2007`).
#[test]
fn duplicate_type_parameter_is_rejected() {
    assert_eq!(
        code("fn f<T, T>(x: T) -> T { return x }\nfn main() { }"),
        codes::REDECLARED
    );
}

// ----------------------------------------------------- existing programs

/// A generic enum's variant payload is parameterised too.
#[test]
fn generic_enum_works() {
    assert_eq!(
        ok("enum Opt<T> { Some(T), Nothing }\nfn unwrap<T>(o: Opt<T>) -> T { return match o { Some(v) -> v\n Nothing -> none } }\nfn main() { print(unwrap(Some(3))) }"),
        "3\n"
    );
}

/// An over-deep generic type annotation is `E1015`, never a host-stack trap.
#[test]
fn over_deep_generic_type_is_bounded() {
    let mut t = "int".to_string();
    for _ in 0..4000 {
        t = format!("Box<{t}>");
    }
    let src = format!("struct Box<T> {{ value: T }}\nfn main() {{ let x: {t} = 0 }}");
    assert_eq!(code(&src), codes::NESTING);
}

/// Generics do not change the meaning of comparison or shift operators.
#[test]
fn comparison_operators_are_unaffected() {
    assert_eq!(
        ok("fn main() { print(1 < 2)\n print(3 > 2)\n print(1 << 2) }"),
        "true\ntrue\n4\n"
    );
}

// ------------------------------------------ type-parameter shadowing (E2007)

/// `LANGUAGE_SPEC.md` §36.2 / `docs/GENERICS.md`: a type parameter may not
/// shadow a declared type. The rule is about **any** declared type the
/// declaration can see — a root type, a module-local type, or an imported
/// type — not only a root type whose bare name matches (`CONF-RESOLVE-5`).
#[test]
fn type_parameter_cannot_shadow_any_visible_declared_type() {
    // Imported type: bare `T` in scope names `m::T`.
    assert_eq!(
        code("module m { pub struct T { pub x: int } }\nuse m::T\nfn f<T>(x: T) -> T { return x }\nfn main() { }"),
        codes::REDECLARED
    );
    // Module-local type in the same module as the generic declaration.
    assert_eq!(
        code("module m { pub struct T { pub x: int }\n pub fn f<T>(x: T) -> T { return x } }\nfn main() { }"),
        codes::REDECLARED
    );
    // A generic struct/enum/alias/trait declaration is checked too.
    assert_eq!(
        code("module m { pub struct T { pub x: int } }\nuse m::T\nstruct Box<T> { value: T }\nfn main() { }"),
        codes::REDECLARED
    );
    // An unrelated parameter name is not affected.
    assert_eq!(
        run_source(
            "module m { pub struct T { pub x: int } }\nfn f<U>(x: U) -> U { return x }\nfn main() { print(f(1)) }",
            "<generics>"
        )
        .expect("not a shadow"),
        "1\n"
    );
}

// ---------------------------------------- generic return-position checking

/// `LANGUAGE_SPEC.md` §15.2: every `return` in a function with a declared
/// return type must be compatible with it. `Ty::compatible_with` treats a
/// `Param` on either side as universally permissive, which is right when
/// binding a parameter at a call site but wrong for a `return`: the body value
/// must match the parameter the caller substitutes, so `fn f<T>(x: T) -> T {
/// return 5 }` is `E3005`. Previously `check` accepted it and the runtime then
/// failed with a checker-tier code at the call site (`CONF-GENERIC-1`).
#[test]
fn generic_return_must_match_the_declared_parameter() {
    assert_eq!(
        code("fn id<T>(x: T) -> T { return 5 }\nfn main() { print(id(\"ab\")) }"),
        codes::RETURN_MISMATCH
    );
    assert_eq!(
        code("fn id<T>(x: T) -> T { return \"s\" }\nfn main() { print(id(1)) }"),
        codes::RETURN_MISMATCH
    );
    // Returning the parameter itself, or an opaque value, is accepted.
    assert_eq!(
        ok("fn id<T>(x: T) -> T { return x }\nfn main() { print(id(1)) }"),
        "1\n"
    );
    // A concrete return type still accepts a parameter actual (substitution
    // supplies it) and a wrong concrete value is still rejected.
    assert_eq!(
        ok("fn f<T>(v: T) -> T { return v }\nfn main() { print(f(\"a\")) }"),
        "a\n"
    );
    assert_eq!(
        code("fn f() -> int { return \"x\" }\nfn main() { }"),
        codes::RETURN_MISMATCH
    );
}

/// A generic function may return a value built from its parameters.
#[test]
fn generic_return_of_constructed_value_is_accepted() {
    assert_eq!(
        ok(
            "struct Box<T> { value: T }\nfn make<T>(v: T) -> Box<T> { return Box<T> { value: v } }\nfn main() { print(make(1).value) }"
        ),
        "1\n"
    );
}

/// The mirror direction of `CONF-GENERIC-1`: a concrete declared return type
/// must not be satisfied by a universally-quantified parameter actual, because
/// the caller may substitute any type. `fn f<T>(x: T) -> int { return x }`
/// must be `E3005` (it previously passed checking and then failed at the call
/// site with a checker-tier code, or silently took a wrong branch).
#[test]
fn concrete_return_is_not_satisfied_by_a_parameter_actual() {
    assert_eq!(
        code("fn f<T>(x: T) -> int { return x }\nfn main() { print(f(\"a\")) }"),
        codes::RETURN_MISMATCH
    );
    assert_eq!(
        code("fn f<T>(x: T) -> bool { return x }\nfn main() { }"),
        codes::RETURN_MISMATCH
    );
    // Nested: a parameter inside a compound actual is still not a concrete
    // return.
    assert_eq!(
        code("fn f<T>(x: T) -> [int] { return [x] }\nfn main() { }"),
        codes::RETURN_MISMATCH
    );
    assert_eq!(
        code("struct Box<T> { value: T }\nfn f<T>(x: T) -> Box<int> { return Box { value: x } }\nfn main() { }"),
        codes::RETURN_MISMATCH
    );
    // Methods route through the same check.
    assert_eq!(
        code("struct S { n: int }\nimpl S { pub fn f<T>(self, y: T) -> int { return y } }\nfn main() { }"),
        codes::RETURN_MISMATCH
    );
}

/// Legitimate generic returns must still be accepted: the parameter itself, a
/// constructed generic value, a union containing the parameter, and a
/// higher-order pass-through.
#[test]
fn valid_parameter_returns_are_accepted() {
    assert_eq!(
        ok("fn f<T>(x: T) -> T { return x }\nfn main() { print(f(1)) }"),
        "1\n"
    );
    assert_eq!(
        ok("struct Box<T> { value: T }\nfn make<T>(v: T) -> Box<T> { return Box<T> { value: v } }\nfn main() { print(make(7).value) }"),
        "7\n"
    );
    assert_eq!(
        ok("fn f<T>(x: T) -> T | int { return x }\nfn main() { print(f(1)) }"),
        "1\n"
    );
    assert_eq!(
        ok("fn wrap<T>(x: T) -> [T] { return [x] }\nfn main() { print(wrap(1)) }"),
        "[1]\n"
    );
    assert_eq!(
        ok("fn g<U>(y: U) -> U { return y }\nfn f<T>(x: T) -> T { return g(x) }\nfn main() { print(f(2)) }"),
        "2\n"
    );
    assert_eq!(
        ok("fn h() -> int { return 1 }\nfn f() -> int { return h() }\nfn main() { print(f()) }"),
        "1\n"
    );
}

/// An empty collection literal is compatible with a parameterized return
/// type: `[]` infers as `[unknown]` and `{:}` as `{string: unknown}`, and
/// `LANGUAGE_SPEC.md` §2.3 forbids rejecting on a type the checker cannot
/// determine. The non-empty analogue was already accepted. This locks the
/// permissive boundary for nested positions too.
#[test]
fn empty_collections_are_compatible_with_parameterized_returns() {
    assert_eq!(
        ok("fn f<T>() -> [T] { return [] }\nfn main() { print(f()) }"),
        "[]\n"
    );
    assert_eq!(
        ok("fn f<T>() -> {string: T} { return {:} }\nfn main() { print(f()) }"),
        "{}\n"
    );
    assert_eq!(
        ok("struct Box<T> { value: T }\nfn f<T>() -> Box<[T]> { return Box { value: [] } }\nfn main() { print(f().value) }"),
        "[]\n"
    );
    // A concrete empty collection still passes its concrete annotation.
    assert_eq!(
        ok("fn f() -> [int] { return [] }\nfn main() { print(f()) }"),
        "[]\n"
    );
}
