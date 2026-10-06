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
