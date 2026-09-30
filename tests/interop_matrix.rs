#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! CPython interoperability matrix (`docs/CPYTHON_COMPATIBILITY_TARGET.md`).
//!
//! One table-driven test per conversion direction, plus exception, lifetime,
//! cleanup, Unicode, and repeated-call coverage. Compiled only with `--features
//! py`. These tests assert the *documented* target: what crosses, what is
//! rejected, and that rejection is a structured diagnostic — never a silent
//! coercion and never a host panic.

#![cfg(feature = "py")]

use aura::error::codes;
use aura::run_source;

fn out(src: &str) -> String {
    run_source(src, "<interop>").expect("program runs")
}

fn code(src: &str) -> u16 {
    run_source(src, "<interop>").unwrap_err().code
}

/// Python scalar literals -> Aura display form.
#[test]
fn python_scalars_to_aura() {
    let cases = [
        ("None", "none"),
        ("True", "true"),
        ("False", "false"),
        ("0", "0"),
        ("-1", "-1"),
        ("9223372036854775807", "9223372036854775807"),
        ("-9223372036854775807 - 1", "-9223372036854775808"),
        ("1.5", "1.5"),
        ("-0.25", "-0.25"),
        ("'aura'", "aura"),
        ("''", ""),
    ];
    for (py, want) in cases {
        let src = format!("fn main() {{ print(py_eval(\"{py}\")) }}");
        assert_eq!(out(&src), format!("{want}\n"), "py_eval({py})");
    }
}

/// Aura scalars -> Python identity: each crosses, is acted on by Python, and
/// returns with the same value.
#[test]
fn aura_scalars_to_python_round_trip() {
    // int
    assert_eq!(
        out("fn main() { print(py_call(\"builtins\", \"int\", 42) == 42) }"),
        "true\n"
    );
    // float
    assert_eq!(
        out("fn main() { print(py_call(\"builtins\", \"float\", 3.5) == 3.5) }"),
        "true\n"
    );
    // string
    assert_eq!(
        out("fn main() { print(py_call(\"builtins\", \"str\", \"hi\") == \"hi\") }"),
        "true\n"
    );
    // bool
    assert_eq!(
        out("fn main() { print(py_call(\"builtins\", \"bool\", true) == true) }"),
        "true\n"
    );
    // none -> Python `None`, whose repr is `None`.
    assert_eq!(
        out("fn main() { print(py_call(\"builtins\", \"repr\", none) == \"None\") }"),
        "true\n"
    );
}

/// Lists and nested structures cross structurally in both directions.
#[test]
fn collections_round_trip() {
    assert_eq!(
        out("fn main() { print(py_eval(\"[1, 2, 3]\")) }"),
        "[1, 2, 3]\n"
    );
    assert_eq!(
        out("fn main() { print(py_eval(\"[[1, 2], [3, [4, 5]]]\")) }"),
        "[[1, 2], [3, [4, 5]]]\n"
    );
    assert_eq!(
        out("fn main() { print(py_eval(\"[1, 'two', 3.0, True, None]\")) }"),
        "[1, \"two\", 3.0, true, none]\n"
    );
    // Aura list -> Python, observed via a Python call that returns a scalar.
    assert_eq!(
        out("fn main() { print(py_call(\"builtins\", \"len\", [1, 2, 3, 4])) }"),
        "4\n"
    );
}

/// Dicts with key-capable keys round-trip; non-capable keys are rejected.
#[test]
fn maps_round_trip_and_reject_bad_keys() {
    assert_eq!(
        out("fn main() { print(py_eval(\"{'a': 1, 'b': 2}\")) }"),
        "{\"a\": 1, \"b\": 2}\n"
    );
    assert_eq!(
        out("fn main() { print(py_eval(\"{1: 'a', '1': 'b'}\")) }"),
        "{1: \"a\", \"1\": \"b\"}\n"
    );
    // tuple key -> rejected, never coerced
    assert_eq!(
        code("fn main() { py_eval(\"{(1, 2): 'x'}\") }"),
        codes::PY_UNSUPPORTED
    );
    // float key -> rejected (not key-capable)
    assert_eq!(
        code("fn main() { py_eval(\"{1.5: 'x'}\") }"),
        codes::PY_UNSUPPORTED
    );
    // Aura map with int key -> Python dict, observed via a Python call.
    assert_eq!(
        out("fn main() { print(py_call(\"builtins\", \"len\", {1: \"a\", 2: \"b\"})) }"),
        "2\n"
    );
}

/// Int overflow at the boundary is `E4013`, never a silent `float`.
#[test]
fn oversized_int_is_overflow_not_float() {
    assert_eq!(code("fn main() { py_eval(\"2**63\") }"), codes::OVERFLOW);
    assert_eq!(
        code("fn main() { py_eval(\"-(2**63) - 1\") }"),
        codes::OVERFLOW
    );
    // The in-range boundaries still cross exactly.
    assert_eq!(
        out("fn main() { print(py_eval(\"2**62\")) }"),
        "4611686018427387904\n"
    );
    assert_eq!(
        out("fn main() { print(py_eval(\"-(2**62)\")) }"),
        "-4611686018427387904\n"
    );
}

/// Python exceptions become `E5001` diagnostics (fatal, not Aura-catchable).
#[test]
fn python_exceptions_are_diagnostics() {
    assert_eq!(code("fn main() { py_eval(\"1 / 0\") }"), codes::PY_ERROR);
    assert_eq!(
        code("fn main() { py_eval(\"undefined_name\") }"),
        codes::PY_ERROR
    );
    assert_eq!(code("fn main() { py_eval(\"[] + 1\") }"), codes::PY_ERROR);
    assert_eq!(
        code("fn main() { py_call(\"math\", \"sqrt\", \"x\") }"),
        codes::PY_ERROR
    );
}

/// Module/attribute resolution failures are `E5001`, not panics.
#[test]
fn import_and_attribute_failures_are_diagnostics() {
    assert_eq!(
        code("fn main() { py_call(\"no_such_module_xyz\", \"f\", 1) }"),
        codes::PY_ERROR
    );
    assert_eq!(
        code("fn main() { py_call(\"math\", \"no_such_attr_xyz\", 1) }"),
        codes::PY_ERROR
    );
    assert_eq!(
        code("fn main() { py_import(\"no_such_module_xyz\") }"),
        codes::PY_ERROR
    );
}

/// Non-representable Aura values (struct, enum, fn, range) are rejected.
#[test]
fn non_representable_aura_values_are_rejected() {
    assert_eq!(
        code("struct S { a: int }\nfn main() { py_call(\"builtins\", \"repr\", S { a: 1 }) }"),
        codes::PY_UNSUPPORTED
    );
    assert_eq!(
        code("enum E { A }\nfn main() { py_call(\"builtins\", \"repr\", A()) }"),
        codes::PY_UNSUPPORTED
    );
    assert_eq!(
        code("fn main() { let f = (x) -> x\n py_call(\"builtins\", \"repr\", f) }"),
        codes::PY_UNSUPPORTED
    );
    assert_eq!(
        code("fn main() { py_call(\"builtins\", \"repr\", range(0, 3)) }"),
        codes::PY_UNSUPPORTED
    );
}

/// Opaque Python objects are printable as their `repr` (never a panic).
#[test]
fn opaque_python_objects_render_as_repr() {
    // A set has no Aura equivalent; it crosses as a string repr.
    let s = out("fn main() { print(len(py_eval(\"{1, 2, 3}\")) > 0) }");
    assert_eq!(s, "true\n");
    // A lambda object likewise.
    let f = out("fn main() { print(len(py_eval(\"lambda x: x\")) > 0) }");
    assert_eq!(f, "true\n");
}

/// A user object implementing `__float__`/`__index__`/`__bool__`/`__str__` must
/// NOT be silently coerced: only genuine `float`/`int`/`bool`/`str` instances
/// convert, and everything else takes the documented `repr` fallback. Before
/// this rule, PyO3's duck-typed `extract::<f64>()` coerced such objects (and
/// `__index__` returning a large value lost precision), contradicting the
/// conversion table.
#[test]
fn duck_typed_objects_are_not_silently_coerced() {
    // `__float__` -> repr, not the float 2.5
    let f = out(
        "fn main() { print(py_eval(\"type('X',(),{'__float__':lambda s:2.5})()\").contains('X')) }",
    );
    assert_eq!(f, "true\n");
    // `__index__` returning 9 -> repr, not 9.0
    let i = out(
        "fn main() { print(py_eval(\"type('X',(),{'__index__':lambda s:9})()\").contains('X')) }",
    );
    assert_eq!(i, "true\n");
    // `__bool__` -> repr, not true
    let b = out(
        "fn main() { print(py_eval(\"type('X',(),{'__bool__':lambda s:True})()\").contains('X')) }",
    );
    assert_eq!(b, "true\n");
    // Genuine instances still convert.
    assert_eq!(out("fn main() { print(py_eval(\"1.5\")) }"), "1.5\n");
    assert_eq!(out("fn main() { print(py_eval(\"True\")) }"), "true\n");
    assert_eq!(out("fn main() { print(py_eval(\"'hi'\")) }"), "hi\n");
}

/// Unicode strings round-trip exactly (no normalization, no lossy encoding).
#[test]
fn unicode_round_trips() {
    assert_eq!(out("fn main() { print(py_eval(\"'café'\")) }"), "café\n");
    assert_eq!(
        out("fn main() { print(py_eval(\"'日本語'\")) }"),
        "日本語\n"
    );
    assert_eq!(
        out("fn main() { print(py_eval(\"'emoji: 🎉'\")) }"),
        "emoji: 🎉\n"
    );
    // Aura string -> Python, observed via a Python length call.
    assert_eq!(
        out("fn main() { print(py_call(\"builtins\", \"len\", \"日本\")) }"),
        "2\n"
    );
}

/// Repeated calls are stable and do not leak interpreter state.
#[test]
fn repeated_calls_are_stable() {
    let src = r#"fn main() {
        let mut total = 0
        for i in range(0, 50) {
            total = total + py_eval("1 + 1")
        }
        print(total)
    }"#;
    assert_eq!(out(src), "100\n");
}

/// Python errors are fatal and uncatchable in v1 (`E5001`); a subsequent
/// crossing still succeeds, so a failed call does not leave the (process-global)
/// CPython interpreter in a state that breaks later conversions.
#[test]
fn failure_then_success_recovers() {
    assert_eq!(code("fn main() { py_eval(\"1 / 0\") }"), codes::PY_ERROR);
    assert_eq!(out("fn main() { print(py_eval(\"21 * 2\")) }"), "42\n");
}

/// `py_call` with multiple positional arguments maps positionally.
#[test]
fn py_call_positional_arguments() {
    assert_eq!(
        out("fn main() { print(py_call(\"builtins\", \"max\", 3, 9, 2)) }"),
        "9\n"
    );
    assert_eq!(
        out("fn main() { print(py_call(\"builtins\", \"len\", [1, 2, 3, 4, 5])) }"),
        "5\n"
    );
    // Keyword-style dispatch is not supported; a wrong call is a diagnostic.
    assert_eq!(
        code("fn main() { py_call(\"builtins\", \"int\", \"notanumber\") }"),
        codes::PY_ERROR
    );
}

/// `py_version` reports a non-empty version string.
#[test]
fn py_version_reports_a_string() {
    assert_eq!(out("fn main() { print(len(py_version()) > 0) }"), "true\n");
}
