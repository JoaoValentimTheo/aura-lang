#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Static-checker tests.
//!
//! These assert that `aura check` (the checker, without execution) rejects
//! what the contract says it must, and that the documented static codes are
//! reachable before any code runs.

use std::fmt::Write as _;

use aura::check::Checker;
use aura::error::codes;
use aura::parse::parse;

/// Check a module without running it, returning the diagnostic code.
fn check(src: &str) -> Result<(), u16> {
    let module = parse(src).expect("parses");
    Checker::module(&module).map_err(|d| d.code)
}

/// Check and require `main`.
fn check_main(src: &str) -> Result<(), u16> {
    let module = parse(src).expect("parses");
    Checker::module_with_main(&module).map_err(|d| d.code)
}

#[test]
fn undefined_function_is_rejected_statically() {
    assert_eq!(check("fn main() { nope() }"), Err(codes::UNDEFINED));
}

#[test]
fn forward_reference_is_allowed() {
    assert_eq!(
        check("fn main() { return later() }\nfn later() { return 1 }"),
        Ok(())
    );
}

#[test]
fn undefined_variable_is_rejected() {
    assert_eq!(check("fn main() { print(x) }"), Err(codes::UNDEFINED));
}

#[test]
fn missing_main_is_rejected_only_when_required() {
    assert_eq!(check("fn f() { }"), Ok(()));
    assert_eq!(check_main("fn f() { }"), Err(codes::NO_MAIN));
    assert_eq!(check_main("fn main() { }"), Ok(()));
}

#[test]
fn main_with_parameters_is_rejected() {
    assert_eq!(check_main("fn main(x) { }"), Err(codes::INVALID_MAIN));
}

#[test]
fn duplicate_main_is_rejected() {
    assert_eq!(
        check_main("fn main() { }\nfn main() { }"),
        Err(codes::REDECLARED)
    );
}

#[test]
fn annotation_mismatch_is_rejected() {
    assert_eq!(
        check("fn main() { let x: int = \"text\" }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("fn main() { let x: string = 3 }"),
        Err(codes::TYPE_MISMATCH)
    );
}

#[test]
fn matching_annotation_is_accepted() {
    assert_eq!(check("fn main() { let x: int = 3 }"), Ok(()));
    assert_eq!(check("fn main() { let xs: [int] = [1, 2] }"), Ok(()));
    assert_eq!(
        check("fn main() { let m: {string: int} = {\"a\": 1} }"),
        Ok(())
    );
}

#[test]
fn return_mismatch_is_rejected() {
    assert_eq!(
        check("fn f() -> int { return \"no\" }"),
        Err(codes::RETURN_MISMATCH)
    );
    assert_eq!(
        check("fn f() -> string { return 1 }"),
        Err(codes::RETURN_MISMATCH)
    );
    assert_eq!(check("fn f() -> int { return 1 }"), Ok(()));
}

#[test]
fn unknown_type_is_rejected() {
    assert_eq!(
        check("fn f() -> Widget { return 1 }"),
        Err(codes::UNKNOWN_TYPE)
    );
    assert_eq!(check("struct S { x: Widget }"), Err(codes::UNKNOWN_TYPE));
}

#[test]
fn map_key_type_must_be_string() {
    assert_eq!(
        check("fn f() -> {int: string} { return none }"),
        Err(codes::TYPE_MISMATCH)
    );
}

#[test]
fn underscore_parameter_must_stay_unused() {
    assert_eq!(check("fn f(_x) { return _x }"), Err(codes::UNUSED_PARAM));
    assert_eq!(check("fn f(_x) { return 1 }"), Ok(()));
}

#[test]
fn duplicate_enum_variant_tags_are_rejected() {
    assert_eq!(
        check("enum A { X }\nenum B { X }"),
        Err(codes::DUPLICATE_VARIANT)
    );
    assert_eq!(check("enum A { X, Y }"), Ok(()));
}

#[test]
fn duplicate_type_is_rejected() {
    assert_eq!(
        check("struct S { }\nstruct S { }"),
        Err(codes::DUPLICATE_TYPE)
    );
    assert_eq!(
        check("struct S { }\nenum S { }"),
        Err(codes::DUPLICATE_TYPE)
    );
}

#[test]
fn duplicate_pattern_binding_is_rejected() {
    assert_eq!(
        check("fn main() { match [1, 2] { [a, a] -> print(a)\n _ -> print(0) } }"),
        Err(codes::DUPLICATE_BINDING)
    );
}

#[test]
fn unknown_constructor_is_rejected() {
    assert_eq!(
        check("fn main() { let p = Ghost { x: 1 } }"),
        Err(codes::UNKNOWN_TYPE)
    );
}

#[test]
fn unknown_variant_in_pattern_is_rejected() {
    assert_eq!(
        check("fn main() { match 1 { Nope -> print(0)\n _ -> print(1) } }"),
        Err(codes::UNKNOWN_TYPE)
    );
}

#[test]
fn assignment_to_immutable_is_static() {
    assert_eq!(
        check("fn main() { let x = 1\n x = 2 }"),
        Err(codes::ASSIGN_IMMUTABLE)
    );
}

// ---------------------------------------------------------------------------
// FEATURE_004: destructuring `let` (`LANGUAGE_SPEC.md` §4.7).
// ---------------------------------------------------------------------------

/// A duplicate name in one destructuring pattern is `E2014`.
#[test]
fn destructuring_duplicate_binding_is_static() {
    assert_eq!(
        check("fn main() { let [a, a] = [1, 2] }"),
        Err(codes::DUPLICATE_BINDING)
    );
    assert_eq!(
        check("enum E { A(int) }\nfn main() { let [A(x), A(x)] = [A(1), A(2)] }"),
        Err(codes::DUPLICATE_BINDING)
    );
}

/// Declaring a name that already exists in the same scope is `E2007`.
#[test]
fn destructuring_same_scope_redeclaration_is_static() {
    assert_eq!(
        check("fn main() { let x = 1\n let [x, y] = [1, 2] }"),
        Err(codes::REDECLARED)
    );
}

/// An unknown variant tag in a `let` pattern is `E3002`.
#[test]
fn destructuring_unknown_variant_is_static() {
    assert_eq!(
        check("fn main() { let Nope(x) = [1] }"),
        Err(codes::UNKNOWN_TYPE)
    );
}

/// Destructuring validly declares every bound name, so later use type-checks
/// and a same-scope reuse of a nested name is caught.
#[test]
fn destructuring_declares_all_names() {
    // Outer/intact: using both names is fine.
    assert_eq!(
        check("fn main() { let [a, b] = [1, 2]\n print(a + b) }"),
        Ok(())
    );
    // A nested binding is also declared and collides in the same scope.
    assert_eq!(
        check("fn main() { let [a, [b, c]] = [1, [2, 3]]\n a = 1 }"),
        Err(codes::ASSIGN_IMMUTABLE)
    );
}

/// Destructured names have no static type (they stay `Unknown`), so an
/// incompatible annotation is never rejected on their account.
#[test]
fn destructuring_names_stay_unknown() {
    assert_eq!(
        check("struct S { x: int }\nfn main() { let [s] = [S { x: 1 }]\n let y: string = s.x }"),
        Ok(())
    );
}

/// Shadowing an outer-scope binding is allowed.
#[test]
fn destructuring_shadows_outer_scope() {
    assert_eq!(
        check("fn main() { let x = 1\n if true { let [x, y] = [1, 2]\n print(x + y) } }"),
        Ok(())
    );
}

// ---------------------------------------------------------------------------
// FEATURE_005: empty-map literal typing (`LANGUAGE_SPEC.md` §20.3).
// ---------------------------------------------------------------------------

/// An empty map is compatible with any map annotation (its value type is
/// `Unknown`), and is a map, not a block.
#[test]
fn empty_map_literal_is_a_map_typed_by_existing_rules() {
    assert_eq!(check("fn main() { let m: {string: int} = {:} }"), Ok(()));
    // It is a map value: a method that a map lacks is rejected as a method
    // error, not accepted as a block.
    assert_eq!(check("fn main() { {:}.nope() }"), Err(codes::UNDEFINED));
}

// ---------------------------------------------------------------------------
// H1a: pattern correctness (checker/runtime agreement, `LANGUAGE_SPEC.md` §19.3).
// ---------------------------------------------------------------------------

/// A variant pattern must name a declared runtime variant tag. A struct name,
/// enum type name, or alias name is not a variant and is `E3002`.
#[test]
fn non_variant_type_names_are_rejected_as_patterns() {
    assert_eq!(
        check("struct P { x: int }\nfn main() { match P(1) { P(v) -> 1\n _ -> 2 } }"),
        Err(codes::UNKNOWN_TYPE)
    );
    assert_eq!(
        check("enum E { A }\nfn main() { match A() { E -> 1\n _ -> 2 } }"),
        Err(codes::UNKNOWN_TYPE)
    );
    assert_eq!(
        check("type Id = int\nfn main() { match 5 { Id -> 1\n _ -> 2 } }"),
        Err(codes::UNKNOWN_TYPE)
    );
}

/// `for` uses the same pattern validation as `match` and destructuring `let`.
#[test]
fn for_pattern_uses_the_same_validation_as_match() {
    assert_eq!(
        check("struct P { x: int }\nfn main() { for P(v) in [P(1)] { } }"),
        Err(codes::UNKNOWN_TYPE)
    );
    assert_eq!(
        check("fn main() { for Nope(v) in [1] { } }"),
        Err(codes::UNKNOWN_TYPE)
    );
    assert_eq!(
        check("enum E { A(int) }\nfn main() { for A(v) in [A(1)] { } }"),
        Ok(())
    );
}

/// A variant pattern that names a real variant tag remains accepted, including
/// the case where a variant tag equals its enum type name.
#[test]
fn real_variant_tags_remain_accepted() {
    assert_eq!(
        check("enum E { A(int), B }\nfn main() { match A(1) { A(v) -> v\n B -> 0 } }"),
        Ok(())
    );
    assert_eq!(
        check("enum E { E(int) }\nfn main() { match E(1) { E(v) -> v } }"),
        Ok(())
    );
}

// ---------------------------------------------------------------------------
// FEATURE_006: `else if` static checking (`LANGUAGE_SPEC.md` §4.5).
// ---------------------------------------------------------------------------

/// Conditions in `else if` clauses are checked by the ordinary `Expr::If`
/// path: an unknown name in a later clause is `E2003`.
#[test]
fn else_if_conditions_are_checked() {
    assert_eq!(
        check("fn main() { if true { } else if missing { } }"),
        Err(codes::UNDEFINED)
    );
}

/// `break`/`continue` legality is governed by the existing loop context, even
/// when they appear inside an `else if` branch.
#[test]
fn else_if_break_continue_loop_context_is_unchanged() {
    assert_eq!(
        check("fn main() { if true { } else if true { break } }"),
        Err(codes::LOOP_CONTROL)
    );
    assert_eq!(
        check("fn main() { for x in [1] { if false { } else if true { break } } }"),
        Ok(())
    );
}

// ---------------------------------------------------------------------------
// Aura 0.0.1 scripting I/O builtins (`docs/RELEASE_0_0_X_DESIGN.md`).
// ---------------------------------------------------------------------------

/// The four release builtins are registered and arity/type-checked by the
/// shared signature registry, exactly like other builtins.
#[test]
fn scripting_io_builtins_are_checked() {
    // Valid arities/types.
    assert_eq!(
        check("fn main() { read_line()\n let t = read_file(\"a\")\n write_file(\"a\", \"b\")\n let a = args() }"),
        Ok(())
    );
    // Arity mismatches are `E3001`, consistent with the rest of the library.
    assert_eq!(
        check("fn main() { read_line(1) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("fn main() { read_file() }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("fn main() { write_file(\"a\") }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(check("fn main() { args(1) }"), Err(codes::TYPE_MISMATCH));
    // Wrong argument type is a provable mismatch.
    assert_eq!(
        check("fn main() { read_file(1) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// A valid alias chain resolves transitively (`LANGUAGE_SPEC.md` §7): `A`
/// denotes `int` through `B`, in both the accepting and the rejecting
/// direction.
#[test]
fn chained_aliases_resolve_transitively() {
    assert_eq!(
        check("type B = int\ntype A = B\nfn main() { let x: A = 5 }"),
        Ok(())
    );
    assert_eq!(
        check("type B = int\ntype A = B\nfn main() { let x: A = \"s\" }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// An alias is transparent in every annotated position: return types, struct
/// fields, and enum payloads (§7), never creating a nominal type.
#[test]
fn alias_is_transparent_in_return_field_and_payload_positions() {
    assert_eq!(check("type Id = int\nfn f() -> Id { return 5 }"), Ok(()));
    assert_eq!(
        check("type Id = int\nfn f() -> Id { return \"s\" }"),
        Err(codes::RETURN_MISMATCH)
    );
    assert_eq!(
        check("type Id = int\nstruct S { id: Id }\nfn main() { let s = S(id: 3) }"),
        Ok(())
    );
    assert_eq!(
        check("type Id = int\nstruct S { id: Id }\nfn main() { let s = S(id: \"x\") }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        check("type Id = int\nenum E { A(Id) }\nfn main() { let e = A(5) }"),
        Ok(())
    );
    assert_eq!(
        check("type Id = int\nenum E { A(Id) }\nfn main() { let e = A(\"s\") }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// An alias chain through `T | none` resolves, and the optional annotation
/// stays at the conservative `Unknown` boundary (§2.3, §6.4): it proves
/// nothing, so it rejects nothing.
#[test]
fn alias_chain_through_optional_stays_conservative() {
    assert_eq!(
        check("type M = int | none\ntype N = M\nfn main() { let x: N = 5 }"),
        Ok(())
    );
    assert_eq!(
        check("type M = int | none\ntype N = M\nfn main() { let x: N = none }"),
        Ok(())
    );
    assert_eq!(
        check("type M = int | none\ntype N = M\nfn main() { let x: N = \"s\" }"),
        Ok(())
    );
    // The inner position of `T | none` is still resolved and validated.
    assert_eq!(check("type M = Nope | none"), Err(codes::UNKNOWN_TYPE));
}

/// An alias whose target is an unknown name is a declaration-time `E3002`,
/// even when the alias is never used.
#[test]
fn alias_with_unknown_target_is_declaration_error() {
    assert_eq!(check("type A = Nope"), Err(codes::UNKNOWN_TYPE));
    assert_eq!(
        check("type A = Nope\nfn main() { }"),
        Err(codes::UNKNOWN_TYPE)
    );
}

/// A long but acyclic alias chain resolves in bounded time and stack, in
/// both the accepting and the rejecting direction.
#[test]
fn deep_alias_chain_resolves() {
    let chain = |target: &str| {
        let mut aliases = String::new();
        for i in 1..200 {
            writeln!(aliases, "type T{i} = T{}", i - 1).unwrap();
        }
        format!("type T0 = {target}\n{aliases}fn main() {{ let x: T199 = 5 }}\n")
    };
    assert_eq!(check(&chain("int")), Ok(()));
    assert_eq!(check(&chain("string")), Err(codes::TYPE_MISMATCH));
}

/// A multi-level alias chain resolves in declaration order and forward order
/// alike (`LANGUAGE_SPEC.md` §7): aliases are hoisted, so a target may be
/// declared after the alias that names it, and a three-link chain through
/// primitive and optional targets stays transparent.
#[test]
fn alias_chain_resolves_in_any_declaration_order() {
    // Declaration order: target first.
    assert_eq!(
        check("type C = int\ntype B = C\ntype A = B\nfn main() { let x: A = 9 }"),
        Ok(())
    );
    assert_eq!(
        check("type C = int\ntype B = C\ntype A = B\nfn main() { let x: A = \"s\" }"),
        Err(codes::TYPE_MISMATCH)
    );
    // Forward order: target last.
    assert_eq!(
        check("type A = B\ntype B = C\ntype C = int\nfn main() { let x: A = 9 }"),
        Ok(())
    );
    assert_eq!(
        check("type A = B\ntype B = C\ntype C = int\nfn main() { let x: A = \"s\" }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// An alias resolves through collection and map compound positions, including
/// a list/map of aliased element types, and a two-level chain inside a
/// compound (`LANGUAGE_SPEC.md` §7).
#[test]
fn alias_resolves_through_collection_positions() {
    // Alias as a list element type.
    assert_eq!(
        check("type Id = int\nfn main() { let xs: [Id] = [1, 2] }"),
        Ok(())
    );
    assert_eq!(
        check("type Id = int\nfn main() { let xs: [Id] = [\"a\"] }"),
        Err(codes::TYPE_MISMATCH)
    );
    // Alias as a map value type.
    assert_eq!(
        check("type Id = int\nfn main() { let m: {string: Id} = {\"a\": 1} }"),
        Ok(())
    );
    assert_eq!(
        check("type Id = int\nfn main() { let m: {string: Id} = {\"a\": \"b\"} }"),
        Err(codes::TYPE_MISMATCH)
    );
    // Alias chain resolving to a list, used as a value type.
    assert_eq!(
        check("type Id = int\ntype Ids = [Id]\nfn main() { let xs: Ids = [1] }"),
        Ok(())
    );
    // Alias in both sides of a chain nested through `T | none`.
    assert_eq!(
        check("type Id = int\ntype MaybeIds = [Id] | none\nfn main() { let xs: MaybeIds = [1] }"),
        Ok(())
    );
}

/// An alias may name a user struct or enum type and stay transparent: aliases
/// never create nominal identity, so the aliased struct is still the same
/// nominal type (`LANGUAGE_SPEC.md` §7).
#[test]
fn alias_to_user_type_is_transparent() {
    assert_eq!(
        check("struct P { x: int }\ntype Alias = P\nfn main() { let s: Alias = P { x: 1 } }"),
        Ok(())
    );
    assert_eq!(
        check("struct P { x: int }\ntype Alias = P\nfn main() { let s: Alias = P { x: \"s\" } }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A forward-declared struct target also resolves.
    assert_eq!(
        check("type Alias = P\nstruct P { x: int }\nfn main() { let s: Alias = P { x: 1 } }"),
        Ok(())
    );
    // An aliased enum is usable in an annotated binding.
    assert_eq!(
        check("enum E { A(int) }\ntype AliasE = E\nfn main() { let e: AliasE = A(3) }"),
        Ok(())
    );
}

/// An alias chain that ends at an unknown name is `E3002`, whether the unknown
/// name is the direct target or reached through another alias
/// (`LANGUAGE_SPEC.md` §7).
#[test]
fn alias_chain_to_unknown_target_is_declaration_error() {
    assert_eq!(check("type A = B\ntype B = Nope"), Err(codes::UNKNOWN_TYPE));
    assert_eq!(
        check("type A = B\ntype B = C\ntype C = Nope"),
        Err(codes::UNKNOWN_TYPE)
    );
}
