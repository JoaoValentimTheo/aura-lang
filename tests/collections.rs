#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Collection type coherence.
//!
//! List and map literal inference fold every statically known element into a
//! union (LANGUAGE_SPEC §5.2, §13, §20.2), so a provable mismatch is rejected
//! rather than hidden by a first-element rule. Typed mutation and known-receiver
//! indexing carry the container's element/key/value contract. `Unknown`
//! elements impose no constraint (the §2.3 boundary).

use aura::error::codes;
use aura::run_source;

fn ok(src: &str) -> String {
    run_source(src, "<collections>").expect("expected the program to run")
}

fn code(src: &str) -> u16 {
    run_source(src, "<collections>")
        .expect_err("expected the program to be rejected")
        .code
}

// ----------------------------------------------------- list inference

#[test]
fn list_literal_infers_a_union_of_known_elements() {
    assert_eq!(ok("fn main() { print([1, 2]) }"), "[1, 2]\n");
    assert_eq!(ok("fn main() { print([1, \"x\"]) }"), "[1, \"x\"]\n");
    // A parenthesized comma-list is now a genuine Tuple (Keystone §26), not
    // list sugar, so it prints with its tuple identity.
    assert_eq!(ok("fn main() { print((1, \"x\")) }"), "(1, \"x\")\n");
}

#[test]
fn annotated_list_rejects_a_provable_element_mismatch() {
    assert_eq!(
        code("fn main() { let xs: [int] = [1, \"x\"] }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn f() -> [int] { return [1, \"x\"] }\nfn main() { }"),
        codes::RETURN_MISMATCH
    );
}

#[test]
fn an_unknown_element_cannot_mask_a_known_mismatch() {
    // The first element is `Unknown` (a bare `py_eval` result); the later known
    // string must still be rejected against `[int]`.
    assert_eq!(
        code("fn main() { let xs: [int] = [py_eval(\"1\"), \"bad\"] }"),
        codes::TYPE_MISMATCH
    );
}

// ------------------------------------------------ whole-literal checking

#[test]
fn nested_literals_are_checked_against_the_expected_type() {
    assert_eq!(
        code("fn main() { let xs: [[int]] = [[1], [\"x\"]] }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn main() { let m: {int: [int]} = {1: [1], 2: [\"x\"]} }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn f(_: [int]) { }\nfn main() { f([1, \"x\"]) }"),
        codes::TYPE_MISMATCH
    );
}

// ------------------------------------------------------ indexing

#[test]
fn indexing_a_known_container_yields_the_element_type() {
    // A successful index is the element type; `f` requires `int`.
    assert_eq!(
        ok("fn f(x: int) { print(x) }\nfn main() { let xs: [int] = [7]\n f(xs[0]) }"),
        "7\n"
    );
    assert_eq!(
        ok("fn f(v: string) { print(v) }\nfn main() { let m: {int: string} = {1: \"x\"}\n f(m[1]) }"),
        "x\n"
    );
    // A list index that yields `int` is not assignable to a `string` parameter.
    assert_eq!(
        code("fn f(_: string) { }\nfn main() { let xs: [int] = [1]\n f(xs[0]) }"),
        codes::TYPE_MISMATCH
    );
}

// ---------------------------------------------- typed mutation

#[test]
fn typed_list_mutation_obeys_the_element_type() {
    assert_eq!(
        code("fn main() { let mut xs: [int] = []\n xs.push(\"x\") }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn main() { let mut xs: [int] = []\n push(xs, \"x\") }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn main() { let mut xs: [int] = [1]\n xs[0] = \"x\" }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        ok("fn main() { let mut xs: [int] = []\n xs.push(1)\n xs[0] = 9\n print(xs) }"),
        "[9]\n"
    );
}

// ------------------------------------------------ get/remove honesty

#[test]
fn map_get_and_remove_are_dynamic_not_lying_as_v() {
    // `get` can return `none`; a successful lookup still yields the value.
    assert_eq!(
        ok("fn main() { let m: {int: string} = {1: \"one\"}\n print(m.get(1))\n print(m.get(9)) }"),
        "one\nnone\n"
    );
    assert_eq!(
        ok("fn main() { let mut m: {int: string} = {1: \"one\"}\n print(m.remove(1))\n print(m.remove(9)) }"),
        "one\nnone\n"
    );
}

// -------------------------------------------------- list methods

#[test]
fn list_sort_and_reverse_preserve_the_element_type() {
    assert_eq!(
        ok("fn main() { let xs: [int] = [3, 1, 2]\n print(xs.sort())\n print(xs.reverse()) }"),
        "[1, 2, 3]\n[2, 1, 3]\n"
    );
}
