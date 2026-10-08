#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Aura 0.3 Keystone collection-identity matrix.
//!
//! Authority: `docs/LANGUAGE_SPEC.md` §21 and ADR-0005. This file pins the
//! Array/List/Tuple/Set contract, in particular the contextual-typing rule that
//! is the most architecturally sensitive part of the collection model.

use aura::error::codes;
use aura::run_source;

fn ok(src: &str) -> String {
    run_source(src, "<collections>").expect("expected the program to run")
}

fn err(src: &str) -> u16 {
    run_source(src, "<collections>")
        .expect_err("expected the program to be rejected")
        .code
}

// --------------------------------------------------------------- Array type

#[test]
fn array_type_uses_semicolon_between_element_and_length() {
    // `[int; 3]` is an Array type; a `;` between elements is not a literal.
    assert_eq!(
        ok("fn main() { let a: [int; 3] = [1, 2, 3]\n print(a[2]) }"),
        "3\n"
    );
    // `[1; 3]` is not an Array literal; the `;` is not a value separator.
    assert_eq!(err("fn main() { let x = [1; 3] }"), codes::EXPECTED);
}

#[test]
fn array_length_must_be_a_compile_time_integer_literal() {
    assert_eq!(
        err("fn main() { let a: [int; x] = [1, 2] }"),
        codes::EXPECTED
    );
    assert_eq!(err("fn main() { let a: [int; -1] = [1] }"), codes::EXPECTED);
}

// ------------------------------------------------- contextual realization

#[test]
fn untyped_bracket_literal_is_a_list() {
    assert_eq!(ok("fn main() { print([1, 2]) }"), "[1, 2]\n");
    // A list and an array of the same contents are unequal.
    assert_eq!(
        ok("fn main() { let l = [1, 2]\n let a: [int; 2] = [1, 2]\n print(l == a) }"),
        "false\n"
    );
}

#[test]
fn annotated_array_binding_realizes_an_array() {
    assert_eq!(
        ok("fn main() { let a: [int; 2] = [1, 2]\n print(a == [1, 2]) }"),
        "false\n"
    );
    // Two arrays with the same contents are equal.
    assert_eq!(
        ok("fn main() { let a: [int; 2] = [1, 2]\n let b: [int; 2] = [1, 2]\n print(a == b) }"),
        "true\n"
    );
}

#[test]
fn array_length_mismatch_is_static() {
    assert_eq!(
        err("fn main() { let a: [int; 2] = [1, 2, 3] }"),
        codes::TYPE_MISMATCH
    );
    // Fewer elements than declared is also a mismatch.
    assert_eq!(
        err("fn main() { let a: [int; 3] = [1, 2] }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn array_element_mismatch_is_static() {
    assert_eq!(
        err("fn main() { let a: [int; 2] = [1, \"x\"] }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn no_implicit_list_to_array_conversion() {
    // A binding already inferred as a List is not a bracket literal, so it can
    // never satisfy an Array expectation, even when the contents fit.
    assert_eq!(
        err("fn main() { let list = [1, 2]\n let a: [int; 2] = list }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("fn take(xs: [int; 2]) { print(xs) }\nfn main() { let list = [1, 2]\n take(list) }"),
        codes::TYPE_MISMATCH
    );
    // The same spelling passed *directly* is a valid contextual literal.
    assert_eq!(
        ok("fn take(xs: [int; 2]) { print(xs) }\nfn main() { take([1, 2]) }"),
        "[1, 2]\n"
    );
}

#[test]
fn array_realizes_in_every_typed_context() {
    // Function argument.
    assert_eq!(
        ok("fn f(xs: [int; 2]) -> int { return xs[0] + xs[1] }\nfn main() { print(f([3, 4])) }"),
        "7\n"
    );
    // Return under a declared return type.
    assert_eq!(
        ok("fn g() -> [int; 2] { return [8, 9] }\nfn main() { print(g()[1]) }"),
        "9\n"
    );
    // Struct field.
    assert_eq!(
        ok(
            "struct P { rgb: [int; 3], }\nfn main() { let p = P { rgb: [1, 2, 3] }\n print(p.rgb == [1, 2, 3]) }"
        ),
        "false\n"
    );
    // Nested: expected type propagates to inner literals.
    assert_eq!(
        ok("fn main() { let m: [[int; 2]; 2] = [[1, 2], [3, 4]]\n print(m[1][0]) }"),
        "3\n"
    );
}

#[test]
fn array_cannot_be_resized_but_elements_are_mutable() {
    assert_eq!(
        err("fn main() { let mut a: [int; 2] = [1, 2]\n a.push(3) }"),
        codes::UNDEFINED
    );
    assert_eq!(
        ok("fn main() { let mut a: [int; 2] = [1, 2]\n a[0] = 9\n print(a[0]) }"),
        "9\n"
    );
}

// ------------------------------------------------------------------- Tuple

#[test]
fn tuple_is_distinct_from_list() {
    assert_eq!(ok("fn main() { print((1, 2) == [1, 2]) }"), "false\n");
    assert_eq!(ok("fn main() { print([1, 2] == (1, 2)) }"), "false\n");
    assert_eq!(ok("fn main() { print((1, \"x\")) }"), "(1, \"x\")\n");
}

#[test]
fn tuple_is_a_fixed_length_immutable_sequence() {
    assert_eq!(ok("fn main() { let t = (1, \"x\")\n print(t[0]) }"), "1\n");
    assert_eq!(
        err("fn main() { let mut t = (1, 2)\n t[0] = 9 }"),
        codes::ASSIGN_IMMUTABLE
    );
}

#[test]
fn tuple_and_list_patterns_are_not_interchangeable() {
    assert_eq!(
        err("fn main() { let (a, b) = [1, 2]\n print(a)\n print(b) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("fn main() { let [a, b] = (1, 2)\n print(a)\n print(b) }"),
        codes::TYPE_MISMATCH
    );
}

// --------------------------------------------------------------------- Set

#[test]
fn set_is_distinct_and_has_membership_semantics() {
    assert_eq!(ok("fn main() { print({1, 2, 2, 3}) }"), "{1, 2, 3}\n");
    assert_eq!(ok("fn main() { print(set{}) }"), "{}\n");
    assert_eq!(ok("fn main() { print({1, 2}.has(2)) }"), "true\n");
    // A set is not equal to a list of the same members.
    assert_eq!(ok("fn main() { print({1, 2} == [1, 2]) }"), "false\n");
    // A set is not indexable.
    assert_eq!(err("fn main() { print({1, 2}[0]) }"), codes::TYPE_MISMATCH);
}

#[test]
fn set_elements_must_be_key_capable() {
    assert_eq!(
        err("fn main() { let s = {[1], [2]} }"),
        codes::TYPE_MISMATCH
    );
}

// -------------------------------------------------------------------- JSON

// `json_decode`/`json_decode_as` are feature-gated, so this test runs only when
// the `json` feature is enabled (it is in the default and canonical configs).
#[cfg(feature = "json")]
#[test]
fn typed_json_preserves_requested_collection_identity() {
    // Typed decode with `[T; N]` produces an Array; dynamic decode a List.
    assert_eq!(
        ok(
            "fn main() { let a = json_decode_as(\"[1,2]\", [int; 2])\n print(a == json_decode(\"[1,2]\")) }"
        ),
        "false\n"
    );
    // A length mismatch is E4031.
    assert_eq!(
        err("fn main() { let _a = json_decode_as(\"[1,2,3]\", [int; 2]) }"),
        codes::DECODE_MISMATCH
    );
    // Typed tuple.
    assert_eq!(
        ok("fn main() { print(json_decode_as(\"[1,\\\"x\\\"]\", (int, string))) }"),
        "(1, \"x\")\n"
    );
    // Typed set rejects duplicate JSON members as a shape mismatch.
    assert_eq!(
        err("fn main() { let _s = json_decode_as(\"[1,1]\", {int}) }"),
        codes::DECODE_MISMATCH
    );
}
