#![cfg(feature = "json")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Aura 0.3 Keystone typed JSON decoding (`json_decode_as`, `E4031`).
//!
//! Authority: Keystone §22 (HTTP response → JSON → target Aura type →
//! validated Struct/value). The decode is type-directed: the document is
//! validated against the declared Aura type, and malformed input, a missing
//! required field, a wrong field or element type, and a nested mismatch are
//! each distinguishable.

use aura::error::codes;
use aura::run_source;

fn ok(src: &str) -> String {
    run_source(src, "<json>").expect("expected the program to run")
}

fn err(src: &str) -> u16 {
    run_source(src, "<json>")
        .expect_err("expected the program to be rejected")
        .code
}

#[test]
fn decodes_a_struct_with_all_fields() {
    assert_eq!(
        ok("struct U { name: string, age: int }\nfn main() { let u = json_decode_as('{\"name\": \"a\", \"age\": 3}', \"U\")\n print(u.name)\n print(u.age) }"),
        "a\n3\n"
    );
}

#[test]
fn optional_field_accepts_absence_and_null() {
    // A `T | none` field decodes to `none` when absent or null; a required
    // field does not.
    assert_eq!(
        ok("struct U { name: string, email: string | none }\nfn main() { let u = json_decode_as('{\"name\": \"a\"}', \"U\")\n print(u.email) }"),
        "none\n"
    );
    assert_eq!(
        ok("struct U { name: string, email: string | none }\nfn main() { let u = json_decode_as('{\"name\": \"a\", \"email\": null}', \"U\")\n print(u.email) }"),
        "none\n"
    );
}

#[test]
fn decodes_a_list_of_structs() {
    assert_eq!(
        ok("struct U { name: string }\nfn main() { let us = json_decode_as('[{\"name\": \"a\"}, {\"name\": \"b\"}]', \"[U]\")\n print(len(us))\n print(us[1].name) }"),
        "2\nb\n"
    );
}

#[test]
fn decodes_nested_structs() {
    assert_eq!(
        ok("struct Inner { v: int }\nstruct Outer { name: string, inner: Inner }\nfn main() { let o = json_decode_as('{\"name\": \"x\", \"inner\": {\"v\": 7}}', \"Outer\")\n print(o.inner.v) }"),
        "7\n"
    );
}

#[test]
fn decode_domains_are_distinguishable() {
    // Malformed JSON.
    assert_eq!(
        err("struct U { name: string }\nfn main() { let u = json_decode_as('nope', \"U\")\n print(u) }"),
        codes::DECODE_MISMATCH
    );
    // Missing required field.
    assert_eq!(
        err("struct U { name: string, age: int }\nfn main() { let u = json_decode_as('{\"name\": \"a\"}', \"U\")\n print(u) }"),
        codes::DECODE_MISMATCH
    );
    // Wrong field type.
    assert_eq!(
        err("struct U { name: string, age: int }\nfn main() { let u = json_decode_as('{\"name\": \"a\", \"age\": \"x\"}', \"U\")\n print(u) }"),
        codes::DECODE_MISMATCH
    );
    // Element mismatch in a collection.
    assert_eq!(
        err("struct U { tags: [string] }\nfn main() { let u = json_decode_as('{\"tags\": [\"a\", 3]}', \"U\")\n print(u) }"),
        codes::DECODE_MISMATCH
    );
    // Unknown field in the document.
    assert_eq!(
        err("struct U { name: string }\nfn main() { let u = json_decode_as('{\"name\": \"a\", \"x\": 1}', \"U\")\n print(u) }"),
        codes::DECODE_MISMATCH
    );
    // Unknown target type.
    assert_eq!(
        err("fn main() { let u = json_decode_as('{}', \"Nope\")\n print(u) }"),
        codes::DECODE_MISMATCH
    );
}

#[test]
fn decode_is_not_catchable_as_a_user_exception() {
    // A decode mismatch is an ordinary diagnostic (level separation): a `try`
    // does not intercept it.
    assert_eq!(
        err("struct U { name: string }\nfn main() { try { let u = json_decode_as('nope', \"U\")\n print(u) } catch _ { print(\"caught\") } }"),
        codes::DECODE_MISMATCH
    );
}

#[test]
fn plain_json_decode_stays_permissive() {
    // The existing dynamic decode is unchanged: it is not type-directed.
    assert_eq!(
        ok("fn main() { let m = json_decode('{\"a\": 1}')\n print(m[\"a\"]) }"),
        "1\n"
    );
}

// ---------------------------------------------------------------------------
// Anti-collapse at the JSON boundary (Keystone Value algebra)
// ---------------------------------------------------------------------------

#[test]
fn json_encode_rejects_a_range_rather_than_emitting_null() {
    // `null` is the encoding of `none` (and of the documented over-depth
    // truncation), so a `range` must not become `null`: that would make
    // `json_decode(json_encode(0..3))` collapse `0..3` and `none` into one
    // value. The rejection is deterministic and matches the existing
    // non-string-map-key rule.
    assert_eq!(
        err("fn main() { print(json_encode(0..3)) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("fn main() { print(json_encode(1..1)) }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn json_encode_rejects_a_function_rather_than_emitting_null() {
    // A closure and a native function are distinct kinds with no JSON
    // representation; neither is silently `none`.
    assert_eq!(
        err("fn main() { print(json_encode(len)) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("fn main() { let g = () -> 1\n print(json_encode(g)) }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn json_encode_rejects_a_variant_rather_than_flattening_it() {
    // Flattening a variant erases enum identity: with the old encoding
    // `A(1)` and `B(1)` (distinct values of one enum) both produced `1`, and
    // a unit variant became a bare string. JSON has no enum kind and
    // `json_decode` cannot reconstruct one, so the collapse is refused.
    assert_eq!(
        err("enum E { A(int), B }\nfn main() { print(json_encode(A(1))) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("enum E { A(int), B }\nfn main() { print(json_encode(B())) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("enum E { A(int), B(int) }\nfn main() { print(json_encode(B(1, 2))) }"),
        codes::TYPE_MISMATCH
    );
    // The same rule applies nested inside a representable container.
    assert_eq!(
        err("enum E { A(int) }\nfn main() { print(json_encode([A(1)])) }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn json_encode_rejects_a_non_finite_float_rather_than_emitting_null() {
    // JSON numbers are finite. `nan` and `inf` must not become `null`
    // (indistinguishable from `none`); they are a deterministic error.
    assert_eq!(
        err("fn main() { print(json_encode(to_float(\"nan\"))) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("fn main() { print(json_encode(to_float(\"inf\"))) }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn json_encode_still_encodes_representable_values_unchanged() {
    // The anti-collapse rule must not change the encoding of a value that
    // *has* a JSON form — including finite floats and `none`.
    assert_eq!(
        ok("fn main() { print(json_encode([1, \"two\", true, none])) }"),
        "[1,\"two\",true,null]\n"
    );
    assert_eq!(ok("fn main() { print(json_encode(2.5)) }"), "2.5\n");
    assert_eq!(ok("fn main() { print(json_encode(none)) }"), "null\n");
}

#[test]
fn json_encode_rejects_a_nested_unrepresentable_value() {
    // The rule applies at any depth, not only at the top level.
    assert_eq!(
        err("fn main() { print(json_encode([1, 0..3])) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("fn main() { print(json_encode({\"r\": 0..3})) }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn json_encode_rejection_is_an_ordinary_diagnostic() {
    // Like every other conversion failure, this is not a catchable user
    // exception (level separation): `try` does not intercept it.
    assert_eq!(
        err("fn main() { try { print(json_encode(0..3)) } catch _ { print(\"caught\") } }"),
        codes::TYPE_MISMATCH
    );
}
