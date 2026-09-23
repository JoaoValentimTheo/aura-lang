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
