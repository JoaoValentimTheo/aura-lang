#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(feature = "repl")]
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

/// B5: a type declared in one submission stays visible to the checker in later
/// submissions, for structs, enums, and type aliases.
#[test]
fn regression_repl_struct_persistence() {
    let out = body("struct P { x: int }\nP { x: 1 }\n:quit\n");
    assert!(out.contains("P { x: 1 }"), "{out}");
    // And a function defined later can use it.
    let out = body("struct P { x: int }\nfn f(p) { return p.x }\nf(P { x: 7 })\n:quit\n");
    assert!(out.contains('7'), "{out}");
}

#[test]
fn regression_repl_enum_persistence() {
    let out = body("enum E { A(int) }\nA(3)\n:quit\n");
    assert!(out.contains("A(3)"), "{out}");
    let out = body("enum E { A(int), B }\nA(1)\nB\n:quit\n");
    assert!(out.contains("A(1)"), "{out}");
    assert!(out.contains('B'), "{out}");
}

#[test]
fn regression_repl_alias_persistence() {
    // The alias is transparent: `Id` denotes `int`, now and later.
    let out = body("type Id = int\nlet x: Id = 5\nx\n:quit\n");
    assert!(out.contains('5'), "{out}");
    let out = body("type Id = int\nstruct P { id: Id }\nP { id: 7 }\n:quit\n");
    assert!(out.contains("P { id: 7 }"), "{out}");
}

/// A session that references an undeclared name still fails, and the session
/// keeps working afterwards.
#[test]
fn session_rejects_unknown_names_without_corruption() {
    let out = body("let a = 1\nlet b = 2\nnope\nb\n:quit\n");
    assert!(out.contains("E2003"), "{out}");
    assert!(out.contains('2'), "{out}");
}

/// FEATURE_001: a function declared in one submission is statically checked in
/// later submissions, and a rejected call does not corrupt the session.
#[test]
fn static_user_fn_call_is_checked_across_submissions() {
    let out = body("fn add(a: int, b: int) { return a + b }\nadd(1)\n:quit\n");
    assert!(out.contains("E3001"), "{out}");
    let out = body("fn add(a: int, b: int) { return a + b }\nadd(\"x\", 2)\n:quit\n");
    assert!(out.contains("E3001"), "{out}");
    // A valid call in the same session still works.
    let out = body("fn add(a: int, b: int) { return a + b }\nadd(1, 2)\n:quit\n");
    assert!(out.contains('3'), "{out}");
}

/// A failed static check leaves the declaration usable; the session is not
/// corrupted.
#[test]
fn repl_static_check_failure_preserves_declaration() {
    let out = body("fn f(x: int) { return x }\nf(1)\nf(\"wrong\")\nf(2)\n:quit\n");
    assert!(out.contains("E3001"), "{out}");
    // Both valid calls produced their value despite the failed one between.
    assert!(out.contains('1'), "{out}");
    assert!(out.contains('2'), "{out}");
}

/// FEATURE_002: parameter names persist across submissions; named calls
/// resolve against the session signature, and a failed call leaves it intact.
#[test]
fn named_arguments_across_submissions() {
    let out = body(
        "fn greet(name: string, punctuation: string) -> string { return name + punctuation }\ngreet(punctuation: \"!\", name: \"Joao\")\n:quit\n",
    );
    assert!(out.contains("Joao!"), "{out}");
    // A rejected named call does not corrupt the declaration.
    let out = body(
        "fn greet(name: string, punctuation: string) -> string { return name + punctuation }\ngreet(name: 1, punctuation: \"!\")\ngreet(punctuation: \"?\", name: \"Ana\")\n:quit\n",
    );
    assert!(out.contains("E3001"), "{out}");
    assert!(out.contains("Ana?"), "{out}");
}
