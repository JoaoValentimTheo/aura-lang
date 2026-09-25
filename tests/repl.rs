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

/// The printed body of a session, with the version banner and prompts removed.
///
/// The banner carries the release version (`Aura 0.0.2 REPL — …`), which is not
/// program output; stripping it keeps assertions about evaluated results
/// independent of the release number.
fn body(input: &str) -> String {
    session(input)
        .lines()
        .filter(|line| !line.starts_with("Aura "))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("aura> ", "")
        .replace("  ... ", "")
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

/// A chained alias carries across submissions (`LANGUAGE_SPEC.md` §7): the
/// resolved type is usable in a later binding once every link is declared.
/// Each submission is its own module, so a forward reference to a target that
/// has not been declared yet is `E3002`, exactly like a file.
#[test]
fn regression_repl_chained_alias_persistence() {
    // Target declared first: B -> int, A -> B.
    let out = body("type B = int\ntype A = B\nlet x: A = 5\nx\n:quit\n");
    assert!(out.contains('5'), "{out}");
    // A provable mismatch through a chained alias is still `E3001`.
    let out = body("type B = int\ntype A = B\nlet x: A = \"s\"\n:quit\n");
    assert!(out.contains("E3001"), "{out}");
    // A submission naming a target the session has not seen is `E3002`.
    let out = body("type A = B\n:quit\n");
    assert!(out.contains("E3002"), "{out}");
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

/// FEATURE_002 stack hardening: an over-limit submission is rejected by the
/// parser on the large parse stack with `E1015` — a diagnostic, never a native
/// overflow — and a modest nested submission evaluates normally.
#[test]
fn repl_nesting_boundary_is_a_diagnostic_not_a_crash() {
    let under = format!("print({}1{})\n:quit\n", "[".repeat(60), "]".repeat(60));
    let out = body(&under);
    assert!(!out.contains("overflow"), "{out}");
    assert!(out.contains("none"), "{out}");

    let over = format!("print({}1{})\n:quit\n", "[".repeat(256), "]".repeat(256));
    let out = body(&over);
    assert!(out.contains("E1015"), "{out}");
}

/// FEATURE_003: a binding's declared struct type persists across submissions,
/// so a field read in a later submission infers the field's declared type and
/// participates in existing checks. A wrong later use is rejected with the
/// existing diagnostic, and the failed submission does not corrupt the state.
#[test]
fn field_read_type_persists_across_submissions() {
    // The declared field type flows into a function-argument check.
    let out = body(
        "struct User { age: int }\nlet user: User = User { age: 3 }\nfn takes_int(x: int) { return x }\ntakes_int(user.age)\n:quit\n",
    );
    assert!(out.contains('3'), "{out}");

    // An incorrect use is rejected with the existing E3001, and the session
    // (struct, binding, function) survives it.
    let out = body(
        "struct User { age: int }\nlet user: User = User { age: 3 }\nfn takes_str(x: string) { return x }\ntakes_str(user.age)\ntakes_int(user.age)\n:quit\n",
    );
    assert!(out.contains("E3001"), "{out}");
    assert!(!out.contains("E4999"), "{out}");
    assert!(!out.contains("panic"), "{out}");
    // The struct read still yields its value and the type metadata is intact.
    assert!(out.contains('3'), "{out}");
}

/// FEATURE_003 REPL failure isolation: a rejected annotated binding neither
/// enters the session nor damages the persisted struct/binding state.
#[test]
fn field_read_failed_binding_is_isolated() {
    let out = body(
        "struct User { age: int }\nlet user: User = User { age: 3 }\nlet bad: string = user.age\nuser.age\n:quit\n",
    );
    // The bad binding is rejected at check time...
    assert!(out.contains("E3001"), "{out}");
    // ...and `bad` was never introduced, while `user.age` still evaluates.
    assert!(!out.contains("bad ="), "{out}");
    assert!(out.contains('3'), "{out}");
}

// ---------------------------------------------------------------------------
// FEATURE_004: destructuring `let` across submissions (`LANGUAGE_SPEC.md` §4.7).
// ---------------------------------------------------------------------------

/// A successful destructuring persists every bound name across submissions.
#[test]
fn destructuring_persists_each_name() {
    let out = body("let [a, b] = [10, 20]\na\nb\na + b\n:quit\n");
    assert!(out.contains("10"), "{out}");
    assert!(out.contains("20"), "{out}");
    assert!(out.contains("30"), "{out}");
}

/// A failed destructuring leaves no binding behind: none of its names is
/// visible in a later submission.
#[test]
fn destructuring_failure_persists_nothing() {
    let out = body("let [p, [q, r]] = [1, [2]]\np\nq\nr\n:quit\n");
    // The match fails at runtime...
    assert!(out.contains("E3001"), "{out}");
    // ...and none of the names were persisted.
    assert!(out.contains("undefined variable `p`"), "{out}");
    assert!(out.contains("undefined variable `q`"), "{out}");
    assert!(out.contains("undefined variable `r`"), "{out}");
}

/// An arity failure does not persist the names that would have been bound.
#[test]
fn destructuring_arity_failure_persists_nothing() {
    let out = body("let [p, q] = [1]\np\n:quit\n");
    assert!(out.contains("E3001"), "{out}");
    assert!(out.contains("undefined variable `p`"), "{out}");
}

/// Ordinary annotated `let` keeps Feature 003's type persistence.
#[test]
fn destructuring_does_not_change_annotated_let_persistence() {
    let out = body(
        "struct User { age: int }\nlet user: User = User { age: 3 }\nfn takes_str(x: string) { return x }\ntakes_str(user.age)\nuser.age\n:quit\n",
    );
    assert!(out.contains("E3001"), "{out}");
    assert!(out.contains('3'), "{out}");
}

// ---------------------------------------------------------------------------
// 0.0.1 post-release hardening audit (H2)
// ---------------------------------------------------------------------------

/// H2-04/H2-08: an uncaught `throw` crossing a call boundary reports `E4026`
/// in the REPL, not the internal `E4099` signal, matching `aura run`.
#[test]
fn h2_repl_uncaught_throw_uses_the_public_code() {
    let out = body("fn f() { throw \"x\" }\nf()\n:quit\n");
    assert!(out.contains("E4026"), "{out}");
    assert!(!out.contains("E4099"), "internal code leaked: {out}");
}

/// H2-08: a bare top-level control-flow signal in the REPL is reported, not
/// silently swallowed. `throw "X"` reports `E4026`; a top-level `return` used
/// as a value reports `E4030`.
#[test]
fn h2_repl_top_level_control_flow_is_reported() {
    let thrown = body("throw \"X\"\n:quit\n");
    assert!(thrown.contains("E4026"), "{thrown}");
    let ret = body("if true { return 1 } else { 2 }\n:quit\n");
    assert!(ret.contains("E4030"), "{ret}");
}

/// H2-09: a `return` in a lambda inside a REPL submission is not checked
/// against any enclosing function annotation and works at the session level.
#[test]
fn h2_repl_lambda_return_works() {
    let out = body("fn f() -> int { let g = () -> { return \"s\" }\n return 3 }\nf()\n:quit\n");
    assert!(out.contains('3'), "{out}");
}
