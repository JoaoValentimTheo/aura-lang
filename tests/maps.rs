#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Generic map key types (`{K: V}`).
//!
//! Maps are keyed by a *key-capable* scalar: `string`, `int`, or `bool`.
//! `float`, `none`, containers, structs, enums, ranges, and functions are not
//! key-capable because they have no total, stable, equality-consistent key
//! identity. A generic key parameter must become key-capable when it is
//! instantiated; a union key type is valid only when every member is.
//!
//! These lock the feature's contract and its boundaries.

use aura::error::codes;
use aura::run_source;

/// Run a program that must succeed and return its stdout.
fn ok(src: &str) -> String {
    run_source(src, "<maps>").expect("expected the program to run")
}

/// The first diagnostic code for a rejected program.
fn code(src: &str) -> u16 {
    run_source(src, "<maps>")
        .expect_err("expected the program to be rejected")
        .code
}

// ------------------------------------------------------------- A. concrete

#[test]
fn string_keyed_maps_are_unchanged() {
    assert_eq!(
        ok("fn main() { let m: {string: int} = {\"a\": 1, \"b\": 2}\n print(m)\n print(m[\"a\"]) }"),
        "{\"a\": 1, \"b\": 2}\n1\n"
    );
}

#[test]
fn int_keyed_maps_work_end_to_end() {
    assert_eq!(
        ok("fn main() { let m: {int: string} = {1: \"one\", 2: \"two\"}\n print(m)\n print(m[1]) }"),
        "{1: \"one\", 2: \"two\"}\none\n"
    );
}

#[test]
fn bool_keyed_maps_work_end_to_end() {
    assert_eq!(
        ok("fn main() { let m: {bool: int} = {true: 1, false: 0}\n print(m)\n print(m[true]) }"),
        "{false: 0, true: 1}\n1\n"
    );
}

// ------------------------------------------------------------ B. alias

#[test]
fn generic_map_alias_works() {
    assert_eq!(
        ok("type Map<K, V> = {K: V}\nfn main() { let vals: Map<int, float> = {1: 1.2, 2: 3.3}\n print(vals[2]) }"),
        "3.3\n"
    );
}

#[test]
fn union_key_alias_works() {
    assert_eq!(
        ok(
            "type Key = string | int\ntype Cache<T> = {Key: T}\nfn main() { let c: Cache<float> = {\"a\": 1.0, 1: 2.0}\n print(c[\"a\"])\n print(c[1]) }"
        ),
        "1.0\n2.0\n"
    );
}

// ------------------------------------------- C/D. generic + union keys

#[test]
fn generic_function_over_a_key_parameter() {
    assert_eq!(
        ok("fn get<K, V>(m: {K: V}, k: K) -> V { return m[k] }\nfn main() { print(get({1: \"x\", 2: \"y\"}, 2)) }"),
        "y\n"
    );
}

// --------------------------------------------------------- E. invalid keys

#[test]
fn float_key_is_rejected() {
    assert_eq!(
        code("fn main() { let m: {float: int} = {1.0: 1} }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn main() { let m: {float: int} = {:} }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn container_and_none_keys_are_rejected() {
    assert_eq!(
        code("fn main() { let m: {[int]: string} = {[1]: \"x\"} }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn main() { let m: {{int: int}: string} = {:} }"),
        codes::TYPE_MISMATCH
    );
    // `none` has no static type: it is the permissive `Unknown` (§2.3), so a
    // `{none: T}` annotation is accepted by the existing `Unknown` boundary.
    // It is not a key-capable *value*: a `none` key in a literal is rejected.
    assert_eq!(code("fn main() { print({none: 1}) }"), codes::TYPE_MISMATCH);
}

#[test]
fn invalid_concrete_generic_instantiation_is_rejected() {
    assert_eq!(
        code("type Map<K, V> = {K: V}\nfn main() { let m: Map<[int], int> = {:} }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn union_key_rejects_a_non_key_capable_member() {
    assert_eq!(
        code("fn main() { let m: {string | float: int} = {:} }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        ok("fn main() { let m: {string | int: bool} = {\"a\": true, 1: false}\n print(m[\"a\"]) }"),
        "true\n"
    );
}

#[test]
fn non_key_capable_literal_key_is_rejected() {
    assert_eq!(
        code("fn main() { print({1.0: \"x\"}) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn main() { print({[1]: \"x\"}) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn main() { print({none: \"x\"}) }"),
        codes::TYPE_MISMATCH
    );
}

// --------------------------------------------------------- F. inference

#[test]
fn integer_keyed_literal_infers_the_key_type() {
    assert_eq!(
        ok("fn main() { let m = {1: \"a\", 2: \"b\"}\n print(m[2]) }"),
        "b\n"
    );
}

#[test]
fn mixed_valid_key_literal_infers_a_union_key_type() {
    // Two distinct valid key kinds in one literal infer the union key type, so
    // the map's own type (and therefore `keys`) is honest. A first-entry-wins
    // rule would have typed this `{int: string}` while it held a string key.
    assert_eq!(
        ok("fn main() { let m = {1: \"a\", \"1\": \"b\"}\n print(m)\n print(m.keys()) }"),
        "{1: \"a\", \"1\": \"b\"}\n[1, \"1\"]\n"
    );
    assert_eq!(
        ok("fn main() { let m = {true: 1, \"a\": 2}\n print(m.keys()) }"),
        "[true, \"a\"]\n"
    );
}

#[test]
fn annotated_map_checks_every_literal_entry() {
    // A later entry that disagrees with the annotation is rejected, not masked
    // by the first entry's type.
    assert_eq!(
        code("fn main() { let m: {int: string} = {1: \"a\", true: \"b\"} }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        ok("fn main() { let m: {int | bool: string} = {1: \"a\", true: \"b\"}\n print(m) }"),
        "{1: \"a\", true: \"b\"}\n"
    );
}

#[test]
fn heterogeneous_value_literal_infers_a_union_value_type() {
    // The value dimension follows the same rule, so `values` is honest.
    assert_eq!(
        ok("fn main() { let m = {\"a\": 1, \"b\": \"x\"}\n print(m.values()) }"),
        "[1, \"x\"]\n"
    );
}

#[test]
fn empty_map_stays_permissive_under_an_annotation() {
    assert_eq!(
        ok("fn main() { let m: {int: string} = {:}\n print(m) }"),
        "{}\n"
    );
}

// ------------------------------------------------------- G. indexing

#[test]
fn indexing_uses_the_key_type() {
    assert_eq!(
        code("fn main() { let m: {int: string} = {1: \"one\"}\n print(m[\"1\"]) }"),
        codes::TYPE_MISMATCH
    );
}

// ------------------------------------------------------- H. mutation

#[test]
fn indexed_assignment_obeys_the_key_and_value_contract() {
    assert_eq!(
        ok("fn main() { let mut m: {int: string} = {:}\n m[1] = \"one\"\n m[1] = \"ONE\"\n print(m) }"),
        "{1: \"ONE\"}\n"
    );
    assert_eq!(
        code("fn main() { let mut m: {int: string} = {:}\n m[\"1\"] = \"one\" }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn main() { let mut m: {int: string} = {:}\n m[1] = 2 }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn map_methods_use_the_key_type() {
    assert_eq!(
        ok("fn main() { let mut m: {int: string} = {1: \"one\"}\n print(m.get(1))\n print(m.has(1))\n print(m.remove(1))\n print(m) }"),
        "one\ntrue\none\n{}\n"
    );
    assert_eq!(
        code("fn main() { let m: {int: string} = {1: \"one\"}\n print(m.get(\"1\")) }"),
        codes::TYPE_MISMATCH
    );
}

// ------------------------------------------------------- I. duplicate keys

#[test]
fn duplicate_keys_are_last_wins_and_deterministic() {
    assert_eq!(
        ok("fn main() { print({1: \"a\", 1: \"b\"}) }"),
        "{1: \"b\"}\n"
    );
    assert_eq!(
        ok("fn main() { print({\"a\": 1, \"a\": 2}) }"),
        "{\"a\": 2}\n"
    );
}

// ------------------------------------------------------- J. equality

#[test]
fn map_equality_is_order_independent() {
    assert_eq!(
        ok("fn main() { print({1: \"a\", 2: \"b\"} == {2: \"b\", 1: \"a\"}) }"),
        "true\n"
    );
    assert_eq!(ok("fn main() { print({1: 1} == {1: 2}) }"), "false\n");
    // Keys of different kinds are never equal, even with equal value types.
    assert_eq!(
        ok("fn main() { print({1: \"a\"} == {\"1\": \"a\"}) }"),
        "false\n"
    );
    // Nested maps.
    assert_eq!(
        ok("fn main() { print({1: {2: 3}} == {1: {2: 3}}) }"),
        "true\n"
    );
}

// ------------------------------------------------------- K. rendering

#[test]
fn generic_keys_render_deterministically() {
    assert_eq!(
        ok("fn main() { print({2: \"b\", 1: \"a\"}) }"),
        "{1: \"a\", 2: \"b\"}\n"
    );
    assert_eq!(
        ok("fn main() { print({true: 1, false: 2}) }"),
        "{false: 2, true: 1}\n"
    );
    assert_eq!(ok("fn main() { print({1: {2: 3}}) }"), "{1: {2: 3}}\n");
}

// ------------------------------------------------------- L. JSON

#[cfg(feature = "json")]
#[test]
fn json_does_not_stringify_non_string_keys() {
    // A JSON object's keys are strings; a non-string-keyed map is a
    // deterministic conversion error, never a silent stringification.
    assert_eq!(
        code("fn main() { print(json_encode({1: \"x\"})) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        ok("fn main() { print(json_encode({\"a\": 1})) }"),
        "{\"a\":1}\n"
    );
    // Decoding always yields string keys, and they round-trip.
    assert_eq!(
        ok("fn main() { print(json_decode(\"{\\\"a\\\": 1}\")) }"),
        "{\"a\": 1}\n"
    );
}

// ------------------------------------------------------- O. nested

#[test]
fn nested_generic_maps_work() {
    assert_eq!(
        ok("fn main() { let m: {string: {int: float}} = {\"a\": {1: 1.5, 2: 2.5}}\n print(m[\"a\"][2]) }"),
        "2.5\n"
    );
}

// ------------------------------------------------- P. iteration snapshot

#[test]
fn iteration_yields_keys_in_ascending_order() {
    assert_eq!(
        ok("fn main() { for k in {2: \"b\", 1: \"a\"} { print(k) } }"),
        "1\n2\n"
    );
}

// ------------------------------------------------------- items()

#[test]
fn items_returns_eager_pair_lists_in_key_order() {
    assert_eq!(
        ok("fn main() { let m: {int: string} = {2: \"b\", 1: \"a\"}\n print(m.items()) }"),
        "[[1, \"a\"], [2, \"b\"]]\n"
    );
    // Destructuring works because each pair is a two-element list.
    assert_eq!(
        ok("fn main() { let m: {int: string} = {1: \"a\", 2: \"b\"}\n for [k, v] in m.items() { print(k)\n print(v) } }"),
        "1\na\n2\nb\n"
    );
    // The result is a snapshot: later map mutation does not change it.
    assert_eq!(
        ok("fn main() { let mut m: {int: string} = {1: \"a\"}\n let snap = m.items()\n m[2] = \"b\"\n print(snap)\n print(m.items()) }"),
        "[[1, \"a\"]]\n[[1, \"a\"], [2, \"b\"]]\n"
    );
}

#[test]
fn items_pairs_have_the_union_element_type() {
    // A pair is a homogeneous `[K | V]` list (no tuple type). Iterating and
    // printing each pair renders the two-element list.
    assert_eq!(
        ok("fn main() { let m: {int: string} = {1: \"a\"}\n for p in m.items() { print(p) } }"),
        "[1, \"a\"]\n"
    );
    // The static type is a list *of* pairs: `[[K | V]]`, not `[K | V]`.
    assert_eq!(
        ok("fn main() { let m: {int: string} = {1: \"a\"}\n let x: [[int | string]] = m.items()\n print(x) }"),
        "[[1, \"a\"]]\n"
    );
    // Indexing the result once yields the pair, twice yields a component.
    assert_eq!(
        ok("fn main() { let m: {int: int} = {1: 2}\n let x: [int] = m.items()[0]\n print(x) }"),
        "[1, 2]\n"
    );
    // Annotating a pair as a scalar is a provable mismatch, not silent.
    assert_eq!(
        code("fn main() { let m: {int: int} = {1: 2}\n let x: int = m.items()[0] }"),
        codes::TYPE_MISMATCH
    );
}
