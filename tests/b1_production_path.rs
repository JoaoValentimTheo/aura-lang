#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(feature = "repl")]
//! B-1 production-path canaries: the production REPL must run the
//! explicit-continuation machine, not host-stack recursion.
//!
//! The other entry points (library `execute_with*`, CLI, Playground) wrap
//! execution in [`aura::on_execution_stack`]'s dedicated 64 MiB thread on
//! native, which masks a regression to the recursive evaluator. The REPL core
//! [`aura::repl::run_with`] is the one production entry that evaluates inline
//! on its caller's stack, so a deliberately small thread stack turns a
//! regression into a loud host-stack failure (the same "fails loudly"
//! technique `tests/b1_stack_safety.rs` uses).
//!
//! Calibration (measured on this tree): the machine completes the depth-510
//! shapes below on a 256 KiB caller stack, while the retained recursive engine
//! aborts on a legal depth-510 program at 256 KiB–1 MiB. The tests use 1 MiB:
//! 4× headroom for the machine, and still well under what recursion needs.

/// Feed `input` to the REPL core and return everything it printed.
fn session(input: &str) -> String {
    let mut out = Vec::new();
    aura::repl::run_with(input.as_bytes(), &mut out).expect("repl io");
    String::from_utf8(out).expect("utf8")
}

/// The printed body of a session, with the banner and prompts removed.
fn body(input: &str) -> String {
    session(input)
        .lines()
        .filter(|line| !line.starts_with("Aura "))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("aura> ", "")
        .replace("  ... ", "")
}

/// Run `f` on a 1 MiB thread; a host-stack overflow aborts the process (a
/// failure), so a successful join means the machine, not recursion, ran.
fn on_small_stack<F: FnOnce() -> String + Send + 'static>(f: F) -> String {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(f)
        .expect("spawn test thread")
        .join()
        .expect("production path overflowed a 1 MiB stack")
}

#[test]
fn deep_recursion_and_const_items_run_inline_without_host_growth() {
    // One session exercises all three production REPL seams: the statement
    // path (`f(510)`), the `const` item path (`const X = f(500)`), and the
    // expression echo (`X`). Legal depth-510 recursion must complete.
    let out = on_small_stack(|| {
        body("fn f(n) { if n <= 0 { return 0 }\n return 1 + f(n - 1) }\nf(510)\nconst X = f(500)\nX\n:quit\n")
    });
    assert!(out.contains("510"), "statement path: {out}");
    assert!(out.contains("500"), "const item path: {out}");
}

#[test]
fn the_repl_reports_e4011_at_the_frame_limit() {
    // The REPL has no `main` frame, so 512 simultaneous user calls are legal
    // and frame 513 is over the language limit: `f(512)` must produce the
    // structured diagnostic, never a host failure, on the production path.
    let out = on_small_stack(|| {
        body("fn f(n) { if n <= 0 { return 0 }\n return 1 + f(n - 1) }\nf(512)\n:quit\n")
    });
    assert!(out.contains("E4011"), "{out}");
}
