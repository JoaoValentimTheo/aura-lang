#![cfg(feature = "json")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The final Keystone typed-JSON API contract (`LANGUAGE_SPEC.md` §22).
//!
//! `json_decode_as(text, Type)` takes its target type as a **type argument** in
//! the canonical Aura type grammar, resolved by the parser into a `TypeExpr`
//! (never a runtime string or value), so the checker knows the result type and
//! an unknown/malformed type is a compile-time diagnostic. This file pins the
//! required forms and failure domains.

use aura::error::codes;
use aura::run_source;

fn ok(src: &str) -> String {
    run_source(src, "<typed-json>").expect("expected the program to run")
}

fn err(src: &str) -> u16 {
    run_source(src, "<typed-json>")
        .expect_err("expected the program to be rejected")
        .code
}

// 1. Direct nominal type reference succeeds.
#[test]
fn direct_nominal_type_reference_succeeds() {
    assert_eq!(
        ok("struct Pokemon { id: int, name: string }\nfn main() { let p = json_decode_as('{\"id\":25,\"name\":\"pikachu\"}', Pokemon)\n print(p.name)\n print(p.id) }"),
        "pikachu\n25\n"
    );
}

// 2. List of nominal Structs succeeds.
#[test]
fn list_of_structs_succeeds() {
    assert_eq!(
        ok("struct U { n: string }\nfn main() { let us = json_decode_as('[{\"n\":\"a\"},{\"n\":\"b\"}]', [U])\n print(len(us))\n print(us[1].n) }"),
        "2\nb\n"
    );
}

// 3. Contextual fixed-length Array decoding succeeds.
#[test]
fn array_target_restores_array_identity() {
    assert_eq!(
        ok("fn main() { let a = json_decode_as('[1,2]', [int; 2])\n print(a)\n print(a == json_decode('[1,2]')) }"),
        "[1, 2]\nfalse\n"
    );
}

// 4. Incorrect Array length produces E4031.
#[test]
fn wrong_array_length_is_e4031() {
    assert_eq!(
        err("fn main() { let _a = json_decode_as('[1,2,3]', [int; 2]) }"),
        codes::DECODE_MISMATCH
    );
}

// 5. Tuple target restores Tuple identity.
#[test]
fn tuple_target_restores_tuple_identity() {
    assert_eq!(
        ok("fn main() { let t = json_decode_as('[1,\"x\"]', (int, string))\n print(t)\n print(t == [1, \"x\"]) }"),
        "(1, \"x\")\nfalse\n"
    );
}

// 6. Set target restores Set identity.
#[test]
fn set_target_restores_set_identity() {
    assert_eq!(
        ok("fn main() { print(json_decode_as('[3,1,2]', {int})) }"),
        "{1, 2, 3}\n"
    );
    // A duplicate JSON member is a shape mismatch for strict validation.
    assert_eq!(
        err("fn main() { let _s = json_decode_as('[1,1]', {int}) }"),
        codes::DECODE_MISMATCH
    );
}

// 7. Map target restores Map identity.
#[test]
fn map_target_restores_map_identity() {
    assert_eq!(
        ok("fn main() { let m = json_decode_as('{\"a\":1}', {string: int})\n print(m)\n print(m[\"a\"]) }"),
        "{\"a\": 1}\n1\n"
    );
}

// 8. Unknown target type fails statically.
#[test]
fn unknown_target_type_fails_statically() {
    assert_eq!(
        err("fn main() { let u = json_decode_as('{}', Nope)\n print(u) }"),
        codes::UNKNOWN_TYPE
    );
    // The compatibility string spelling resolves through the same path.
    assert_eq!(
        err("fn main() { let u = json_decode_as('{}', \"Nope\")\n print(u) }"),
        codes::UNKNOWN_TYPE
    );
}

// 9. A malformed type expression fails safely.
#[test]
fn malformed_type_expression_fails_safely() {
    assert_eq!(
        err("fn main() { let u = json_decode_as('{}', [int)\n print(u) }"),
        codes::EXPECTED
    );
    assert_eq!(
        err("fn main() { let u = json_decode_as('{}', 5)\n print(u) }"),
        codes::EXPECTED
    );
}

// 10. Runtime variable/type-name confusion is rejected.
#[test]
fn a_value_cannot_masquerade_as_a_type() {
    assert_eq!(
        err("fn main() { let Pokemon = 1\n let u = json_decode_as('{}', Pokemon)\n print(u) }"),
        codes::UNKNOWN_TYPE
    );
}

// 11. Nested optional Struct decoding remains correct.
#[test]
fn nested_optional_struct_decoding_remains_correct() {
    assert_eq!(
        ok("struct Inner { v: int, note: string | none }\nstruct Outer { name: string, inner: Inner }\nfn main() { let o = json_decode_as('{\"name\":\"x\",\"inner\":{\"v\":7}}', Outer)\n print(o.inner.v)\n print(o.inner.note) }"),
        "7\nnone\n"
    );
}

// 12. Existing strict unknown-field behavior remains correct.
#[test]
fn strict_unknown_field_behavior_remains() {
    assert_eq!(
        err("struct U { n: string }\nfn main() { let u = json_decode_as('{\"n\":\"a\",\"x\":1}', U)\n print(u) }"),
        codes::DECODE_MISMATCH
    );
}

// The canonical example from the campaign brief, exactly:
#[test]
fn the_canonical_brief_example_executes() {
    assert_eq!(
        ok("struct Pokemon {\n    id: int,\n    name: string,\n}\n\nfn main() {\n    let pokemon = json_decode_as(\n        '{\"id\":25,\"name\":\"pikachu\"}',\n        Pokemon\n    )\n    print(pokemon.name)\n}"),
        "pikachu\n"
    );
}
