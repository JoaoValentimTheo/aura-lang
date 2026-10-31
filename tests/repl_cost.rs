#![allow(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(feature = "repl")]
//! REPL session-cost tests.
//!
//! A REPL session keeps every declaration the user has typed and must see them
//! all when checking each new submission. The cheap way to do that is to
//! rebuild the whole checker from the accumulated declaration list on every
//! submission, which makes a session of `n` submissions cost `O(n²)` work.
//!
//! Wall-clock timing is not usable here: it is noise on shared CI machines and
//! it depends on the allocator, the optimizer, and the machine's load. The
//! deterministic proxy is allocation volume, following `tests/render_cost.rs`.
//! Bytes requested from the allocator are counted per thread, so the numbers do
//! not depend on machine speed, only on how much work the session actually
//! performs.

use aura::repl::SessionEngine;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    /// Bytes requested from the allocator by this thread since the last reset.
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

/// An allocator that records how many bytes this thread requests.
///
/// `realloc` counts the new size rather than the delta because the count is
/// used as a total, not a running balance: a reallocation that grows a buffer
/// to `new` is `new` bytes of live work regardless of what it replaced.
struct Counting;

fn record(n: usize) {
    // A thread-local destructor can run after the fast TLS map is gone, so a
    // panic here would abort the test binary at exit. Accounting failures are
    // not interesting, so they are dropped.
    let _ = BYTES.try_with(|b| b.set(b.get() + n));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        record(l.size());
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        record(new);
        unsafe { System.realloc(p, l, new) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

/// Bytes allocated during `f`, with the counter reset to zero first.
fn cost_of(f: impl FnOnce()) -> usize {
    BYTES.with(|b| b.set(0));
    f();
    BYTES.with(|b| b.get())
}

/// Feed `n` distinct `let` bindings to one session and return the bytes
/// allocated by the last `tail` submissions.
///
/// Only the tail is measured so the number reflects the *steady-state* cost of
/// one submission at the accumulated depth `n`, not the startup cost. Distinct
/// names are used because the session's declaration list grows with each one;
/// repeated names would exercise shadowing instead and never grow the list.
fn steady_state_cost(n: usize, tail: usize) -> usize {
    let mut engine = SessionEngine::new();
    let mut out = Vec::new();
    for i in 0..(n - tail) {
        let _ = engine.feed_line(&format!("let v{i} = {i}"), &mut out);
    }
    cost_of(|| {
        for i in (n - tail)..n {
            let _ = engine.feed_line(&format!("let v{i} = {i}"), &mut out);
        }
    })
}

/// AUDIT-6: a REPL session must not cost quadratic work in the number of
/// accumulated declarations.
///
/// One submission's cost is bounded by a constant that does not grow with the
/// session's depth. Rebuilding the checker from the whole declaration list on
/// every submission makes the steady-state cost grow linearly with `n`, so
/// doubling the depth roughly doubles the cost of the final submissions. The
/// limit below is far wider than the incremental cost, which is flat, and far
/// below the quadratic cost at these depths.
#[test]
fn audit6_repl_submission_cost_does_not_grow_with_session_depth() {
    // Measuring the same number of submissions at two depths isolates the
    // per-submission cost. A linear-in-depth rebuild doubles it; an
    // incremental one leaves it unchanged.
    const TAIL: usize = 200;
    let shallow = steady_state_cost(400, TAIL);
    let deep = steady_state_cost(1600, TAIL);

    // Four times the depth. A quadratic session pays roughly 4x here; the
    // limit of 2x leaves room for allocator and map-growth noise while still
    // separating flat from linear growth.
    let growth = deep as f64 / shallow as f64;
    assert!(
        growth < 2.0,
        "a submission at session depth 1600 allocated {deep} bytes against \
         {shallow} bytes at depth 400 ({growth:.2}x for 4x the depth); each \
         submission should cost a constant amount of work regardless of how \
         many declarations the session already holds, not rebuild the whole \
         checker from the accumulated declarations"
    );
}

/// The session must still see every earlier declaration, whichever way the
/// checker is built. This pins the semantics the cost test is allowed to
/// optimize, so a fix cannot buy flat cost by forgetting earlier bindings.
#[test]
fn audit6_later_submissions_still_see_earlier_declarations() {
    let mut engine = SessionEngine::new();
    let mut out = Vec::new();

    let _ = engine.feed_line("let base = 40", &mut out);
    for i in 0..200 {
        let _ = engine.feed_line(&format!("let v{i} = {i}"), &mut out);
    }
    out.clear();
    let _ = engine.feed_line("base + 2", &mut out);

    let printed = String::from_utf8(out).expect("utf8");
    assert_eq!(printed.trim(), "42", "early bindings must stay visible");
}

/// Shadowing must keep last-write-wins across submissions, and the bound value
/// must be the newest one.
#[test]
fn audit6_shadowing_keeps_the_newest_binding() {
    let mut engine = SessionEngine::new();
    let mut out = Vec::new();

    let _ = engine.feed_line("let x = 1", &mut out);
    for i in 0..100 {
        let _ = engine.feed_line(&format!("let v{i} = {i}"), &mut out);
    }
    let _ = engine.feed_line("let x = 99", &mut out);
    out.clear();
    let _ = engine.feed_line("x", &mut out);

    let printed = String::from_utf8(out).expect("utf8");
    assert_eq!(printed.trim(), "99", "the newest binding must win");
}
