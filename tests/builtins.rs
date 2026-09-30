#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Standard-library value semantics (`LANGUAGE_SPEC.md` §25).
//!
//! `tests/property_hardening.rs` proves the *shape* contract (callable
//! existence, arity) for every registry entry. This file pins the *value*
//! behavior of the builtins whose documented semantics were previously
//! untested — most notably `min`/`max`, whose spec contract is deliberately
//! type-independent ("unordered ⇒ source order", §25) and therefore cannot be
//! covered by a generated same-type call.

use aura::error::codes;
use aura::run_source;

fn out(src: &str) -> String {
    run_source(src, "<builtins>").expect("program runs")
}

fn code(src: &str) -> Result<String, u16> {
    run_source(src, "<builtins>").map_err(|d| d.code)
}

/// `min`/`max` on an ordered pair return the smaller/larger element.
#[test]
fn min_max_ordered_pairs() {
    assert_eq!(
        out("fn main() { print(min(3, 7))\n print(min(7, 3)) }"),
        "3\n3\n"
    );
    assert_eq!(
        out("fn main() { print(max(3, 7))\n print(max(7, 3)) }"),
        "7\n7\n"
    );
    assert_eq!(out("fn main() { print(min(-1, 1)) }"), "-1\n");
    assert_eq!(out("fn main() { print(min(2.5, 1.5)) }"), "1.5\n");
    assert_eq!(out("fn main() { print(max(2.5, 1.5)) }"), "2.5\n");
    assert_eq!(out("fn main() { print(min(1, 2.5)) }"), "1\n");
    assert_eq!(out("fn main() { print(max(1, 2.5)) }"), "2.5\n");
    assert_eq!(out("fn main() { print(min(\"b\", \"a\")) }"), "a\n");
    assert_eq!(out("fn main() { print(max(\"b\", \"a\")) }"), "b\n");
    assert_eq!(out("fn main() { print(min(false, true)) }"), "false\n");
    assert_eq!(out("fn main() { print(max(false, true)) }"), "true\n");
}

/// Equal elements: the result is that element for both `min` and `max`.
#[test]
fn min_max_equal_pairs() {
    assert_eq!(
        out("fn main() { print(min(5, 5))\n print(max(5, 5)) }"),
        "5\n5\n"
    );
}

/// The documented unordered rule: when the pair has no ordering (a list, or a
/// mixed/unsupported type pair), source order wins for **both** `min` and
/// `max` — `a` is returned. `cmp_val` returns `None` for these pairs rather
/// than trapping.
#[test]
fn min_max_unordered_returns_source_order() {
    // Two lists are not orderable (§12), so `a` is returned by both.
    assert_eq!(
        out("fn main() { print(min([2], [1]))\n print(max([2], [1])) }"),
        "[2]\n[2]\n"
    );
    // A mixed number/string pair has no ordering; source order wins.
    assert_eq!(
        out("fn main() { print(min(1, \"a\"))\n print(max(1, \"a\")) }"),
        "1\n1\n"
    );
}

/// Arity is a static `E3001` before execution.
#[test]
fn min_max_arity_is_checked() {
    assert_eq!(
        code("fn main() { print(min(1)) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        code("fn main() { print(max(1, 2, 3)) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// `abs` returns the magnitude for both signs and both numeric types, and
/// `i64::MIN` is the documented `E4013` overflow.
#[test]
fn abs_value_and_overflow() {
    assert_eq!(
        out("fn main() { print(abs(-5))\n print(abs(5))\n print(abs(-2.5)) }"),
        "5\n5\n2.5\n"
    );
    assert_eq!(
        code("fn main() { print(abs(0 - 9223372036854775807 - 1)) }"),
        Err(codes::OVERFLOW)
    );
}

/// `to_int` truncates toward zero for floats, parses strings, and maps
/// `true`/`false` to `1`/`0` (§10.9).
#[test]
fn to_int_documented_conversions() {
    assert_eq!(
        out("fn main() { print(to_int(3.9))\n print(to_int(-3.9))\n print(to_int(\"42\"))\n print(to_int(true))\n print(to_int(false)) }"),
        "3\n-3\n42\n1\n0\n"
    );
    // An unparseable string is a structured `E3001`, never a host panic.
    assert_eq!(
        code("fn main() { print(to_int(\"nope\")) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // An out-of-i64-range decimal string is likewise a diagnostic.
    assert_eq!(
        code("fn main() { print(to_int(\"99999999999999999999\")) }"),
        Err(codes::TYPE_MISMATCH)
    );
}

/// `to_float` converts ints and parseable strings.
#[test]
fn to_float_documented_conversions() {
    assert_eq!(
        out("fn main() { print(to_float(3))\n print(to_float(\"2.5\")) }"),
        "3.0\n2.5\n"
    );
}

/// `keys`/`values` follow ascending map-key order (§25), independent of the
/// source insertion order.
#[test]
fn keys_values_use_ascending_key_order() {
    assert_eq!(
        out("fn main() { print(keys({3:\"c\",1:\"a\",2:\"b\"})) }"),
        "[1, 2, 3]\n"
    );
    assert_eq!(
        out("fn main() { print(values({3:\"c\",1:\"a\",2:\"b\"})) }"),
        "[\"a\", \"b\", \"c\"]\n"
    );
}

/// `zip` stops at the shortest list; `enumerate` pairs each element with its
/// index (§25).
#[test]
fn zip_and_enumerate_values() {
    assert_eq!(
        out("fn main() { print(zip([1,2],[\"a\"])) }"),
        "[[1, \"a\"]]\n"
    );
    assert_eq!(
        out("fn main() { print(enumerate([\"a\",\"b\"])) }"),
        "[[0, \"a\"], [1, \"b\"]]\n"
    );
}

/// `sum` folds numerically; int overflow is `E4013` (§25).
#[test]
fn sum_folds_and_overflows() {
    assert_eq!(out("fn main() { print(sum([1,2,3])) }"), "6\n");
    assert_eq!(out("fn main() { print(sum([1.5, 2.0])) }"), "3.5\n");
    assert_eq!(
        code("fn main() { print(sum([9223372036854775807, 1])) }"),
        Err(codes::OVERFLOW)
    );
}

/// `assert` is a no-op on truthy values and `E4028` otherwise, with the
/// optional message rendered by display (§25).
#[test]
fn assert_truthiness_and_message() {
    assert_eq!(
        out("fn main() { assert(true)\n assert(1, \"unused\")\n print(\"ok\") }"),
        "ok\n"
    );
    assert_eq!(code("fn main() { assert(false) }"), Err(codes::ASSERT));
    assert_eq!(
        code("fn main() { assert(0, \"boom\") }"),
        Err(codes::ASSERT)
    );
}

/// The higher-order builtins statically reject a non-callable and a
/// wrong-arity callable (`E3001`), and never panic.
#[test]
fn higher_order_builtins_reject_bad_callables() {
    assert_eq!(
        code("fn main() { print(map([1,2], 5)) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        code("fn main() { print(filter([1,2], 5)) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        code("fn main() { print(reduce([1], 5, 0)) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        code("fn main() { print(map([1,2], (a, b) -> a)) }"),
        Err(codes::TYPE_MISMATCH)
    );
    assert_eq!(
        code("fn main() { print(reduce([1,2], (a) -> a, 0)) }"),
        Err(codes::TYPE_MISMATCH)
    );
    // A valid call still runs.
    assert_eq!(
        out("fn main() { print(map([1,2,3], (x) -> x * 2)) }"),
        "[2, 4, 6]\n"
    );
}

/// Registry-shaped errors: each builtin rejects a provably wrong receiver
/// type rather than panicking.
#[test]
fn wrong_receiver_types_are_diagnostics() {
    for src in [
        "fn main() { print(sort(5)) }",
        "fn main() { print(reverse(5)) }",
        "fn main() { print(keys([1])) }",
        "fn main() { print(values(5)) }",
        "fn main() { print(sum(5)) }",
        "fn main() { print(enumerate(5)) }",
        "fn main() { print(zip(1, 2)) }",
        "fn main() { print(len(5)) }",
    ] {
        assert_eq!(code(src), Err(codes::TYPE_MISMATCH), "{src}");
    }
}

/// `regex_*` use `(pattern, text)` argument order.
#[test]
fn regex_argument_order_is_pattern_then_text() {
    assert_eq!(
        out("fn main() { print(regex_match(\"a.c\", \"abc\")) }"),
        "true\n"
    );
    assert_eq!(
        out("fn main() { print(regex_match(\"a.c\", \"xabcx\")) }"),
        "true\n"
    );
    assert_eq!(
        out("fn main() { print(regex_match(\"z\", \"abc\")) }"),
        "false\n"
    );
    assert_eq!(
        out("fn main() { print(regex_find(\"abc\", \"xabcx\")) }"),
        "abc\n"
    );
}
