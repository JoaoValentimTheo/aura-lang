#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Static-checker tests.
//!
//! These assert that `aura check` (the checker, without execution) rejects
//! what the contract says it must, and that the documented static codes are
//! reachable before any code runs.

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
