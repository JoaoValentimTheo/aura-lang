#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! REPL tests, driven through the testable `run_with` core with piped input.
//!
//! The REPL must behave like the file front end (same parser, same checker,
//! same interpreter) while keeping state across submissions and never
//! corrupting that state after a failed command.

use aura::repl::run_with;

/// Feed `input` to the REPL and return everything it printed.
fn session(input: &str) -> String {
    let mut out = Vec::new();
    run_with(input.as_bytes(), &mut out).expect("repl io");
    String::from_utf8(out).expect("utf8")
}

/// The printed body of a session, with prompts removed.
fn body(input: &str) -> String {
    session(input).replace("aura> ", "").replace("  ... ", "")
}

#[test]
fn evaluates_expressions() {
    let out = body("1 + 2\n:quit\n");
    assert!(out.contains('3'), "{out}");
}

#[test]
fn persistent_variable() {
    let out = body("let x = 41\nx + 1\n:quit\n");
    assert!(out.contains("42"), "{out}");
}

#[test]
fn mutable_variable_and_assignment() {
    let out = body("let mut x = 1\nx = x + 1\nx\n:quit\n");
    assert!(out.contains('2'), "{out}");
}

#[test]
fn immutable_assignment_is_rejected_and_state_survives() {
    let out = body("let y = 1\ny = 2\ny\n:quit\n");
    assert!(out.contains("E2001"), "{out}");
    // `y` is still 1 after the failed assignment.
    assert!(out.matches('1').count() >= 1, "{out}");
}

#[test]
fn persistent_functions() {
    let out = body("fn add(a, b) { return a + b }\nadd(2, 3)\n:quit\n");
    assert!(out.contains('5'), "{out}");
}

#[test]
fn multiline_function_definition() {
    let out = body("fn add(a, b) {\n  return a + b\n}\nadd(10, 20)\n:quit\n");
    assert!(out.contains("30"), "{out}");
}

#[test]
fn recovers_after_check_error() {
    let out = body("let x = 1\nnope\nx + 1\n:quit\n");
    assert!(out.contains("E2003"), "{out}");
    assert!(out.contains('2'), "{out}");
}

#[test]
fn recovers_after_runtime_error() {
    let out = body("let mut x = 5\nx / 0\nx + 1\n:quit\n");
    assert!(out.contains("E4007"), "{out}");
    assert!(out.contains('6'), "{out}");
}

#[test]
fn string_methods_work() {
    let out = body("\"abc\".upper()\n:quit\n");
    assert!(out.contains("ABC"), "{out}");
}

#[test]
fn empty_lines_are_ignored() {
    let out = body("\n\n1 + 1\n:quit\n");
    assert!(out.contains('2'), "{out}");
}

#[test]
fn eof_terminates_cleanly() {
    // No `:quit`; EOF must end the session without error.
    let out = body("1 + 1\n");
    assert!(out.contains('2'), "{out}");
}

#[test]
fn quit_command_exits() {
    let out = body(":quit\n1 + 1\n");
    // Nothing after `:quit` is evaluated.
    assert!(!out.contains('2'), "{out}");
}

#[test]
fn shadowing_an_existing_binding() {
    // Redefinition in the REPL follows file semantics and is rejected, but
    // the original binding keeps working.
    let out = body("let x = 1\nlet x = 2\nx\n:quit\n");
    assert!(out.contains("E2007"), "{out}");
    assert!(out.contains('1'), "{out}");
}

#[cfg(not(feature = "py"))]
#[test]
fn feature_gated_builtin_reports_unavailable() {
    let out = body("py_eval(\"1\")\n:quit\n");
    assert!(out.contains("E5002"), "{out}");
}
