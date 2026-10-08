#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Aura 0.3 Keystone Value-algebra anti-collapse matrix.
//!
//! Authority: `docs/LANGUAGE_SPEC.md` §6.6 (type families), §11 (equality),
//! §21 (tuples are list sugar), §24 (JSON builtins) and the executable model in
//! `src/types.rs` (`Ty`), `src/run/value.rs` (`Value`), and
//! `src/stdlib/signatures.rs` (`TypeClass`).
//!
//! The Keystone invariant is that a shared capability must never erase
//! identity. Aura has no `Array` and no `Set`; its distinct runtime value kinds
//! are `int`, `float`, `string`, `bool`, `none`, `list`, `map`, `range`,
//! struct, enum variant, closure, and native function. `(a, b)`/`Tuple` is
//! deliberately list sugar (§21), so "tuple equals list" is the one *normative*
//! absorption, not a collapse bug.
//!
//! Every assertion here is an identity claim between kinds that share a
//! capability (iteration, indexing, sizing, JSON object form) but must remain
//! distinguishable in equality, display, JSON, and the static checker.

use aura::error::codes;
use aura::run_source;

fn ok(src: &str) -> String {
    run_source(src, "<value-algebra>").expect("expected the program to run")
}

fn err(src: &str) -> u16 {
    run_source(src, "<value-algebra>")
        .expect_err("expected the program to be rejected")
        .code
}

// ---------------------------------------------------------------------------
// 1. Cross-kind equality never collapses
// ---------------------------------------------------------------------------

#[test]
fn empty_collections_of_different_kinds_are_not_equal() {
    // The three empty containers are pairwise distinct even though each is
    // falsy and each is `len == 0`.
    assert_eq!(
        ok("fn main() { print([] == {:})\n print([] == 0..0)\n print({:} == 0..0) }"),
        "false\nfalse\nfalse\n"
    );
    // Emptiness does not make them interchangeable in any equality position.
    assert_eq!(
        ok("fn main() { print([] == \"\")\n print({:} == \"\")\n print(0..0 == \"\") }"),
        "false\nfalse\nfalse\n"
    );
}

#[test]
fn equal_length_collections_of_different_kinds_are_not_equal() {
    // A one-element list, a one-entry map, and a one-element range share
    // `len == 1` and iteration arity; they are still three distinct values.
    assert_eq!(
        ok(
            "fn main() { print(len([0]) == len({0: 0}) )\n print(len([0]) == len(0..1))\n print([0] == {0: 0})\n print([0] == 0..1)\n print({0: 0} == 0..1) }"
        ),
        "true\ntrue\nfalse\nfalse\nfalse\n"
    );
}

#[test]
fn none_equals_only_none() {
    // `none` is absence, not zero, not false, not the empty string, not an
    // empty collection, and not a function.
    assert_eq!(
        ok(
            "fn main() { print(none == none)\n print(none == false)\n print(none == 0)\n print(none == 0.0)\n print(none == \"\")\n print(none == [])\n print(none == {:})\n print(none == 0..0)\n print(none == len) }"
        ),
        "true\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\n"
    );
}

#[test]
fn bool_never_equals_a_number() {
    // `false`/`true` are not `0`/`1`, unlike many dynamically typed
    // languages. This is a kind boundary, not a coercion rule.
    assert_eq!(
        ok("fn main() { print(false == 0)\n print(true == 1)\n print(false == 0.0)\n print(true == 1.0) }"),
        "false\nfalse\nfalse\nfalse\n"
    );
    // And the reverse direction through arithmetic types is the same boundary.
    assert_eq!(err("fn main() { print(false + 1) }"), codes::TYPE_MISMATCH);
}

#[test]
fn struct_never_equals_a_map_even_with_equal_fields() {
    // A struct and a `string`-keyed map can carry the same field data and may
    // both encode as a JSON object, but they are different kinds: equality and
    // display keep them apart.
    assert_eq!(
        ok(
            "struct P { x: int }\nfn main() {\n let p = P { x: 1 }\n let m = {\"x\": 1}\n print(p == m)\n print(m == p)\n print(p)\n print(m)\n}"
        ),
        "false\nfalse\nP { x: 1 }\n{\"x\": 1}\n"
    );
}

#[test]
fn variants_never_equal_their_payload_or_a_sibling_variant() {
    // Enum identity is nominal and tagged: a variant is not its payload, and
    // two distinct tags with equal payloads are different values.
    assert_eq!(
        ok(
            "enum E { A(int), B(int), Nil }\nfn main() {\n print(A(1) == B(1))\n print(A(1) == 1)\n print(B(1) == 1)\n print(A(1) == A(1))\n print(Nil() == Nil())\n print(Nil() == none) }"
        ),
        "false\nfalse\nfalse\ntrue\ntrue\nfalse\n"
    );
}

#[test]
fn function_identity_is_not_structural_and_never_cross_kind() {
    // A closure equals only itself; a native builtin equals the same builtin;
    // no function equals a non-function.
    assert_eq!(
        ok(
            "fn g() -> int { return 1 }\nfn main() {\n let a = g\n let b = g\n print(a == b)\n print(a == len)\n print(len == len)\n print(g == 0)\n print(g == \"g\") }"
        ),
        "true\nfalse\ntrue\nfalse\nfalse\n"
    );
}

// ---------------------------------------------------------------------------
// 2. The Keystone collection model: Tuple is a distinct identity
// ---------------------------------------------------------------------------

#[test]
fn tuple_is_a_distinct_identity_not_list_sugar() {
    // Keystone §26 supersedes the earlier list-sugar absorption: a
    // parenthesized comma-list is now a genuine Tuple with its own runtime
    // identity, and it is never equal to a list of the same contents.
    assert_eq!(
        ok("fn main() { let t = (1, 2)\n print(t)\n print(t == [1, 2])\n print(len(t))\n print(t[0]) }"),
        "(1, 2)\nfalse\n2\n1\n"
    );
    // A tuple pattern is distinct from a list pattern: a list does not match a
    // tuple pattern and vice versa (the runtime rejects the mismatch).
    assert_eq!(
        err("fn main() { let (a, b) = [1, 2]\n print(a)\n print(b) }"),
        codes::TYPE_MISMATCH
    );
}

// ---------------------------------------------------------------------------
// 3. Numeric equality is the documented cross-*type* (not cross-kind) rule
// ---------------------------------------------------------------------------

#[test]
fn numeric_equality_crosses_int_and_float_only() {
    // §11: `int` and `float` compare numerically and across types. This is the
    // single documented cross-type equality, and it does not extend to any
    // other kind, nor does it make container kinds interchangeable.
    assert_eq!(
        ok("fn main() { print(1 == 1.0)\n print(1.5 == 1.5)\n print(1 == 1.0000001) }"),
        "true\ntrue\nfalse\n"
    );
    // Element-wise: a list of ints and a list of floats compare numerically
    // element-wise, but a list is still never equal to a map of the same
    // numbers.
    assert_eq!(
        ok("fn main() { print([1] == [1.0])\n print({\"a\": 1} == {\"a\": 1.0})\n print([1] == {0: 1}) }"),
        "true\ntrue\nfalse\n"
    );
}

// ---------------------------------------------------------------------------
// 4. Display and JSON preserve kind identity
// ---------------------------------------------------------------------------

#[test]
fn display_form_distinguishes_every_kind() {
    // Each kind renders with its own form; none is silently rendered as
    // another. (Struct-vs-map and list-vs-range are the pairs with the
    // closest-looking reprs; both stay distinct.)
    assert_eq!(
        ok(
            "fn main() {\n print(1)\n print(1.5)\n print(true)\n print(\"s\")\n print(none)\n print([])\n print({:})\n print(0..0)\n print(to_string(len)) }"
        ),
        "1\n1.5\ntrue\ns\nnone\n[]\n{}\n0..0\n<fn>\n"
    );
}

#[test]
#[cfg(feature = "json")]
fn json_preserves_kind_identity_or_rejects() {
    // JSON encodes exactly the representable kinds: `none` is `null`, a list
    // is an array, a map and a struct are objects, and each is distinct.
    assert_eq!(
        ok(
            "fn main() { print(json_encode([]))\n print(json_encode({:}))\n print(json_encode(none))\n print(json_encode([1]))\n print(json_encode({\"a\": 1})) }"
        ),
        "[]\n{}\nnull\n[1]\n{\"a\":1}\n"
    );
    // Kinds with no exact JSON form are never approximated (they used to
    // collapse into `null`/payload; the rule is now uniform).
    assert_eq!(
        err("fn main() { print(json_encode(0..0)) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("fn main() { print(json_encode(len)) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("enum E { A(int) }\nfn main() { print(json_encode(A(1))) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("fn main() { print(json_encode(to_float(\"nan\"))) }"),
        codes::TYPE_MISMATCH
    );
}

// ---------------------------------------------------------------------------
// 5. Shared capabilities, distinct identities (the capability matrix)
// ---------------------------------------------------------------------------

#[test]
fn indexing_is_per_kind_and_does_not_transfer() {
    // list/string/map index by their own key type; a struct indexes by field
    // name; a range is sized but NOT indexable. Indexability is a per-kind
    // capability, and a wrong key kind is a deterministic error.
    assert_eq!(
        ok("fn main() { print([1, 2][1])\n print(\"ab\"[1])\n print({\"k\": 1}[\"k\"])\n print(len(0..3)) }"),
        "2\nb\n1\n3\n"
    );
    // A range has `len` but cannot be indexed.
    assert_eq!(err("fn main() { print((0..3)[1]) }"), codes::TYPE_MISMATCH);
    // A struct is not an int-indexable sequence.
    assert_eq!(
        err("struct P { x: int }\nfn main() { print(P { x: 1 }[0]) }"),
        codes::TYPE_MISMATCH
    );
    // A list cannot be indexed by a string or a float.
    assert_eq!(err("fn main() { print([1][\"0\"]) }"), codes::TYPE_MISMATCH);
    assert_eq!(err("fn main() { print([1][0.5]) }"), codes::TYPE_MISMATCH);
    // A scalar is not indexable.
    assert_eq!(err("fn main() { print(1[0]) }"), codes::TYPE_MISMATCH);
}

#[test]
fn iteration_yields_the_kind_specific_element() {
    // A list and a range iterate their elements, a map iterates its keys, and
    // a string iterates its characters. A struct is not iterable at all.
    assert_eq!(
        ok(
            "fn main() {\n for v in [7] { print(v) }\n for v in 0..2 { print(v) }\n for k in {\"a\": 1} { print(k) }\n for ch in \"hi\" { print(ch) } }"
        ),
        "7\n0\n1\na\nh\ni\n"
    );
    assert_eq!(
        err("struct P { x: int }\nfn main() { for v in P { x: 1 } { print(v) } }"),
        codes::NOT_ITERABLE
    );
    assert_eq!(
        err("fn main() { for v in 5 { print(v) } }"),
        codes::NOT_ITERABLE
    );
    assert_eq!(
        err("fn main() { for v in true { print(v) } }"),
        codes::NOT_ITERABLE
    );
    assert_eq!(
        err("fn main() { for v in none { print(v) } }"),
        codes::NOT_ITERABLE
    );
}

#[test]
fn patterns_are_assertive_per_kind_not_coercive() {
    // §14.3/§19: a list pattern requires a list of the same length, a variant
    // pattern requires that variant, and a mismatch is an error, never a
    // silent "no match" that would make cross-kind values interchangeable.
    assert_eq!(
        err("fn main() { let [a] = [1, 2]\n print(a) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("enum E { A(int) }\nfn main() { let A(x) = 5\n print(x) }"),
        codes::TYPE_MISMATCH
    );
    // Cross-kind pattern uses are rejected rather than skipping.
    assert_eq!(
        err("fn main() { let [a] = {0: 1}\n print(a) }"),
        codes::TYPE_MISMATCH
    );
    // A matching pattern still binds normally, preserving the positive path.
    assert_eq!(
        ok("fn main() { let [a, b] = [1, 2]\n print(a + b) }"),
        "3\n"
    );
}

// ---------------------------------------------------------------------------
// 6. The static checker refuses cross-kind interchange
// ---------------------------------------------------------------------------

#[test]
fn annotations_reject_a_provably_different_kind() {
    // Each annotation names one kind family; a value of a different kind is
    // E3001 with the two kinds named. This is what makes the *static* model
    // distinguish the same kinds the runtime does.
    assert_eq!(
        err("fn main() { let xs: [int] = 0..3\n print(xs) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("fn main() { let m: {string: int} = [1]\n print(m) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(err("fn main() { let n: int = none }"), codes::TYPE_MISMATCH);
    assert_eq!(err("fn main() { let b: bool = 1 }"), codes::TYPE_MISMATCH);
    // A list of the wrong *element* kind is also refused, including when a
    // later element would reveal it (no first-element rule).
    assert_eq!(
        err("fn main() { let xs: [int] = [1, \"a\"] }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn a_union_type_records_the_mixed_element_kinds_rather_than_collapsing() {
    // The inferred element type of a mixed literal is the union of the
    // distinct member types (§21.1): string-ness is not lost.
    assert_eq!(
        ok("fn main() { let xs = [1, \"a\"]\n print(xs) }"),
        "[1, \"a\"]\n"
    );
    assert_eq!(
        err("fn main() { let xs: [int] = [1, \"a\"] }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        err("fn main() { let m: {string: int} = {\"a\": 1, \"b\": \"x\"} }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn a_range_projects_range_like_and_never_object() {
    // §5.4: a range's static carrier is the dynamic `Ty::Named("range")`
    // marker; its family is `range_like` and its value kind is `range`. It must
    // never project as a nominal `object`/`struct` (a pure projection defect
    // with no runtime symptom).
    use aura::types::{Ty, TypeFamily};
    let range = Ty::Named("range".to_string());
    assert_eq!(range.families(), &[TypeFamily::RangeLike]);
    assert_eq!(range.family(), TypeFamily::RangeLike);
    assert_eq!(range.value_kind(), "range");
    // A real struct name still projects `object`/`struct`.
    let user = Ty::Named("User".to_string());
    assert_eq!(user.families(), &[TypeFamily::Object]);
    assert_eq!(user.value_kind(), "struct");
}

#[test]
fn ordering_is_defined_only_for_scalars_and_never_across_kinds() {
    // §12: no lexicographic ordering for lists/maps/structs/enums/ranges.
    // Equality across kinds is `false` rather than an error, but *ordering*
    // across kinds is a type error — the two operators do not share a
    // permissive fallback.
    assert_eq!(err("fn main() { print([1] < [2]) }"), codes::TYPE_MISMATCH);
    assert_eq!(err("fn main() { print({} < {}) }"), codes::TYPE_MISMATCH);
    assert_eq!(
        err("fn main() { print(0..1 < 0..2) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        ok("fn main() { print(1 < 2)\n print(\"a\" < \"b\")\n print(false < true) }"),
        "true\ntrue\ntrue\n"
    );
}
