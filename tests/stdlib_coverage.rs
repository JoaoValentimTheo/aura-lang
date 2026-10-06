#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Direct behavior coverage for stdlib surfaces the Pre-0.3 audit found were
//! only exercised indirectly (registry iteration) or only from website
//! examples.
//!
//! Every expectation here is derived from `docs/LANGUAGE_SPEC.md` and the
//! method/builtin tables in the spec/reference; none is inferred from an
//! implementation detail. The purpose is to make the public contract of these
//! surfaces discriminating, so a semantic regression cannot hide behind the
//! generic registry property test.

use aura::error::codes;
use aura::run_source;

fn ok(src: &str) -> String {
    run_source(src, "<stdlib>").expect("expected the program to run")
}

fn code(src: &str) -> u16 {
    run_source(src, "<stdlib>")
        .expect_err("expected the program to be rejected")
        .code
}

// ------------------------------------------------------ string methods

/// `lower` folds ASCII (and Unicode where Rust's lowercase is defined),
/// returning a new string; the receiver is unchanged.
#[test]
fn string_lower_is_a_new_folded_string() {
    assert_eq!(ok("fn main() { print(\"ABC\".lower()) }"), "abc\n");
    assert_eq!(ok("fn main() { print(\"MiXeD\".lower()) }"), "mixed\n");
    assert_eq!(
        ok("fn main() {\n let s = \"ABC\"\n print(s.lower())\n print(s)\n}"),
        "abc\nABC\n"
    );
    // `len` counts chars, not bytes (spec §9 string rule).
    assert_eq!(ok("fn main() { print(len(\"é\".lower())) }"), "1\n");
}

/// `starts_with`/`ends_with` are exact prefix/suffix tests returning bool,
/// with the empty string as an identity prefix/suffix.
#[test]
fn string_starts_with_and_ends_with() {
    assert_eq!(
        ok("fn main() {\n print(\"hello\".starts_with(\"he\"))\n print(\"hello\".starts_with(\"lo\"))\n print(\"hello\".ends_with(\"lo\"))\n print(\"hello\".ends_with(\"he\"))\n}"),
        "true\nfalse\ntrue\nfalse\n"
    );
    assert_eq!(
        ok("fn main() {\n print(\"x\".starts_with(\"\"))\n print(\"x\".ends_with(\"\"))\n}"),
        "true\ntrue\n"
    );
    // An over-long needle is false, not an error.
    assert_eq!(
        ok("fn main() { print(\"ab\".starts_with(\"abc\")) }"),
        "false\n"
    );
    // A non-string argument is a type mismatch.
    assert_eq!(
        code("fn main() { print(\"a\".starts_with(1)) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn main() { print(\"a\".ends_with(1)) }"),
        codes::TYPE_MISMATCH
    );
}

/// `contains` is substring containment for strings (not equality).
#[test]
fn string_contains_is_substring_containment() {
    assert_eq!(
        ok("fn main() {\n print(\"hello\".contains(\"ell\"))\n print(\"hello\".contains(\"xyz\"))\n print(\"hello\".contains(\"\"))\n}"),
        "true\nfalse\ntrue\n"
    );
    assert_eq!(
        code("fn main() { print(\"hello\".contains(1)) }"),
        codes::TYPE_MISMATCH
    );
}

/// `chars` yields one single-character string per Unicode scalar, in order.
#[test]
fn string_chars_lists_scalars_in_order() {
    assert_eq!(
        ok("fn main() { print(\"abc\".chars()) }"),
        "[\"a\", \"b\", \"c\"]\n"
    );
    assert_eq!(ok("fn main() { print(len(\"héllo\".chars())) }"), "5\n");
    assert_eq!(ok("fn main() { print(\"\".chars()) }"), "[]\n");
    // chars() and count-of-chars agree with `len`.
    assert_eq!(
        ok("fn main() { print(len(\"abc\".chars()) == len(\"abc\")) }"),
        "true\n"
    );
}

// -------------------------------------------------------- list methods

/// `pop` removes and returns the last element; on the empty list it returns
/// `none` (documented dynamic return).
#[test]
fn list_pop_returns_last_or_none() {
    assert_eq!(
        ok("fn main() {\n let mut xs = [1, 2, 3]\n print(xs.pop())\n print(xs)\n}"),
        "3\n[1, 2]\n"
    );
    assert_eq!(ok("fn main() { print([].pop()) }"), "none\n");
    // The receiver must be mutably bound (spec §16.3).
    assert_eq!(
        code("fn main() { let xs = [1]\n print(xs.pop()) }"),
        codes::ASSIGN_IMMUTABLE
    );
}

/// `first`/`last` read the ends without mutating; empty returns `none`.
#[test]
fn list_first_and_last_read_without_mutating() {
    assert_eq!(
        ok("fn main() {\n let xs = [10, 20, 30]\n print(xs.first())\n print(xs.last())\n print(xs)\n}"),
        "10\n30\n[10, 20, 30]\n"
    );
    assert_eq!(
        ok("fn main() {\n print([].first())\n print([].last())\n}"),
        "none\nnone\n"
    );
    // A single element is both first and last.
    assert_eq!(
        ok("fn main() {\n let xs = [7]\n print(xs.first())\n print(xs.last())\n}"),
        "7\n7\n"
    );
}

/// `join` renders each element with `display()` and inserts the separator;
/// the empty list joins to the empty string.
#[test]
fn list_join_uses_display_and_separator() {
    assert_eq!(ok("fn main() { print([1, 2, 3].join(\",\")) }"), "1,2,3\n");
    assert_eq!(
        ok("fn main() { print([\"a\", \"b\"].join(\"-\")) }"),
        "a-b\n"
    );
    assert_eq!(ok("fn main() { print([].join(\",\")) }"), "\n");
    // One element: no separator.
    assert_eq!(ok("fn main() { print([1].join(\",\")) }"), "1\n");
    // The separator must be a string.
    assert_eq!(
        code("fn main() { print([1, 2].join(1)) }"),
        codes::TYPE_MISMATCH
    );
}

/// `contains` for lists is element equality (`equals`), not substring.
#[test]
fn list_contains_is_element_equality() {
    assert_eq!(
        ok("fn main() {\n print([1, 2, 3].contains(2))\n print([1, 2, 3].contains(9))\n print([\"a\"].contains(\"a\"))\n}"),
        "true\nfalse\ntrue\n"
    );
    // Cross-type numeric equality holds (spec §11): 1 == 1.0.
    assert_eq!(ok("fn main() { print([1, 2].contains(1.0)) }"), "true\n");
}

// ---------------------------------------------------------- to_float

/// `to_float` accepts int, float, bool, and a trimming numeric string
/// (spec §10.9); anything else is `E3001`.
#[test]
fn to_float_accepts_the_documented_kinds_and_rejects_others() {
    assert_eq!(ok("fn main() { print(to_float(1)) }"), "1.0\n");
    assert_eq!(ok("fn main() { print(to_float(1.5)) }"), "1.5\n");
    assert_eq!(ok("fn main() { print(to_float(true)) }"), "1.0\n");
    assert_eq!(ok("fn main() { print(to_float(false)) }"), "0.0\n");
    assert_eq!(ok("fn main() { print(to_float(\" 2.25 \")) }"), "2.25\n");
    assert_eq!(ok("fn main() { print(to_float(\"-3\")) }"), "-3.0\n");
    assert_eq!(
        code("fn main() { print(to_float(\"abc\")) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn main() { print(to_float([1])) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(
        code("fn main() { print(to_float(none)) }"),
        codes::TYPE_MISMATCH
    );
}

// ------------------------------------------------------------- JSON

/// `json_encode` renders the documented value kinds; `json_decode` parses
/// the JSON subset back, and a malformed document is a documented failure.
#[cfg(feature = "json")]
#[test]
fn json_round_trips_the_documented_kinds() {
    assert_eq!(
        ok("fn main() { print(json_encode([1, \"two\", true, none])) }"),
        "[1,\"two\",true,null]\n"
    );
    assert_eq!(
        ok("fn main() { print(json_encode({\"a\": 1, \"b\": [2, 3]})) }"),
        "{\"a\":1,\"b\":[2,3]}\n"
    );
    assert_eq!(
        ok("fn main() {\n let back = json_decode(\"[1, 2, 3]\")\n print(back)\n print(back[1])\n}"),
        "[1, 2, 3]\n2\n"
    );
    // A scalar JSON document decodes to the mapped scalar.
    assert_eq!(ok("fn main() { print(json_decode(\"42\")) }"), "42\n");
    assert_eq!(ok("fn main() { print(json_decode(\"null\")) }"), "none\n");
    // Malformed input is a runtime failure, not a crash.
    assert!(code("fn main() { print(json_decode(\"{oops\")) }") != 0);
}

// ------------------------------------------------------------ regex

/// `regex_find_all` returns every non-overlapping match in order;
/// `regex_replace` replaces every match.
#[cfg(feature = "regex")]
#[test]
fn regex_find_all_and_replace_cover_the_public_contract() {
    assert_eq!(
        ok("fn main() { print(regex_find_all(\"[0-9]+\", \"a1 b22 c333\")) }"),
        "[\"1\", \"22\", \"333\"]\n"
    );
    assert_eq!(
        ok("fn main() { print(regex_find_all(\"x\", \"abc\")) }"),
        "[]\n"
    );
    assert_eq!(
        ok("fn main() { print(regex_replace(\"[0-9]+\", \"a1 b22\", \"#\")) }"),
        "a# b#\n"
    );
    // An invalid pattern is a documented diagnostic, not a panic.
    let bad = code("fn main() { print(regex_find_all(\"[\", \"a\")) }");
    assert_ne!(bad, 0);
    let bad = code("fn main() { print(regex_replace(\"[\", \"a\", \"x\")) }");
    assert_ne!(bad, 0);
}
