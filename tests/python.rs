#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Integration tests for the Python bridge.
//!
//! Compiled only with `--features py`, so the default test run exercises the
//! pure-Rust runtime.

#![cfg(feature = "py")]

use aura::run_source;

#[test]
fn evaluates_python_expressions() {
    assert_eq!(
        run_source("fn main() { print(py_eval(\"1 + 2\")) }", "<py>").unwrap(),
        "3\n"
    );
    assert_eq!(
        run_source("fn main() { print(py_eval(\"'aura'.upper()\")) }", "<py>").unwrap(),
        "AURA\n"
    );
}

#[test]
fn round_trips_collections() {
    let out = run_source("fn main() { print(py_eval(\"[1, 2, 3]\")) }", "<py>").unwrap();
    assert_eq!(out, "[1, 2, 3]\n");
}

#[test]
fn reports_python_errors_with_a_code() {
    let err = run_source("fn main() { py_eval(\"1 / 0\") }", "<py>").unwrap_err();
    assert_eq!(err.code, aura::error::codes::PY_ERROR);
}

#[test]
fn py_version_is_available() {
    let out = run_source("fn main() { print(len(py_version()) > 0) }", "<py>").unwrap();
    assert_eq!(out, "true\n");
}

#[test]
fn oversized_python_int_is_rejected_not_truncated() {
    // `2**63` overflows `i64`; demoting it to `f64` would silently change the
    // value, so crossing must fail with an explicit diagnostic.
    let err = run_source("fn main() { py_eval(\"2**63\") }", "<py>").unwrap_err();
    assert_eq!(err.code, aura::error::codes::OVERFLOW);
    // The largest representable positive `i64` still round-trips exactly.
    let out = run_source("fn main() { print(py_eval(\"2**62\")) }", "<py>").unwrap();
    assert_eq!(out, "4611686018427387904\n");
}

#[test]
fn non_string_python_dict_keys_are_rejected() {
    // Stringifying keys would collapse distinct keys (e.g. `1` and `"1"`).
    let err = run_source("fn main() { py_eval(\"{1: 1, '1': 2}\") }", "<py>").unwrap_err();
    assert_eq!(err.code, aura::error::codes::PY_UNSUPPORTED);
    let out = run_source("fn main() { print(py_eval(\"{'a': 1}\")) }", "<py>").unwrap();
    assert_eq!(out, "{\"a\": 1}\n");
}
