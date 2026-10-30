// AUDIT-5: the cost of rendering a deeply nested value.
//
// The value encoder used to build a fresh `String` for every node's subtree
// and then copy that string into each enclosing level, so total byte work was
// quadratic in the rendered size. The output length was bounded — the node and
// depth budgets worked — but a nested value legitimately approaching the
// `MAX_VALUE_DEPTH` ceiling copied its inner levels once per level above it.
// The fuzz `runtime` target reported this as a libFuzzer *timeout* rather than
// a crash, which is why it went unnoticed: nothing was wrong with the guards,
// only with the cost of reaching them.
//
// The encoder now appends into a single output buffer, making byte work linear
// in the emitted text. This test pins that cost contract. The neighbouring
// AUDIT-4 tests pin termination and elision, and `regressions.rs` pins the
// emitted length of a budget-exhausting value; none of them can distinguish a
// linear encoder from a quadratic one that still terminates, because the two
// produce byte-identical output.
//
// Wall clock cannot express this contract portably — the pre-fix encoder took
// ~0.9s in release and ~3.4s in debug on one machine and over 2s on a shared
// Windows CI runner, so any fixed millisecond bound is either flaky on slow
// hardware or too loose to catch a regression on fast hardware. Allocation
// volume is the honest proxy: it measures the same byte-copying work that
// makes the wall clock blow up, and it is deterministic for a given depth.
//
// Measured bytes allocated per byte emitted, for this nesting shape:
//
//     depth   linear encoder   pre-fix encoder
//     100     2.51             214.42
//     200     2.53             414.46
//     400     2.55             814.48
//
// The linear figure is flat in depth (it is `String` growth geometry, not
// value traversal); the pre-fix figure doubles whenever depth doubles, which
// is the quadratic signature. The gaps asserted below are wider than the
// measurements by more than two orders of magnitude, so the thresholds are not
// tuned to the current allocator's behaviour — they separate linear from
// quadratic, not this build from an adjacent one.

#![allow(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use aura::run::value::Value;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

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

/// A single-element chain `n` levels deep: `[[[…1…]]]`.
///
/// This is the cheapest shape that reproduces the defect. Each level holds one
/// value, so the rendered size is `2n + 1` bytes and a linear encoder touches
/// each level once. A quadratic encoder copies the whole inner subtree into
/// every enclosing level, so its cost grows with the square of the depth while
/// the output stays the same size.
fn nest(n: usize) -> Value {
    let mut v = Value::Int(1);
    for _ in 0..n {
        let l = Value::List(Rc::new(RefCell::new(vec![v])));
        v = l;
    }
    v
}

/// Bytes allocated while rendering `v`, and the length of what it rendered.
fn render_cost(v: &Value) -> (usize, usize) {
    BYTES.with(|b| b.set(0));
    let out = v.display();
    let allocated = BYTES.with(|b| b.get());
    // Keep the optimizer from discarding a render whose only visible effect is
    // the allocation it performs.
    std::hint::black_box(&out);
    (allocated, out.len())
}

/// AUDIT-5: rendering cost must be linear in the emitted text, not quadratic in
/// the nesting depth.
///
/// The bound is 64 bytes per emitted byte. The linear encoder sits two orders
/// of magnitude below that; the quadratic encoder sits one to two orders of
/// magnitude above it at these depths, and the gap widens as the depth grows.
#[test]
fn audit5_render_cost_is_linear_in_output_not_quadratic_in_depth() {
    const MAX_BYTES_PER_OUTPUT_BYTE: f64 = 64.0;

    for depth in [200usize, 400] {
        let (allocated, emitted) = render_cost(&nest(depth));

        // Pinning the shape keeps the ratio meaningful: if a future rendering
        // change alters the spelling of a nested list, the byte-per-byte
        // comparison silently starts measuring a different value.
        assert_eq!(
            emitted,
            2 * depth + 1,
            "depth {depth} renders as {emitted} bytes, expected {} (one pair of \
             brackets per level plus one digit)",
            2 * depth + 1
        );

        let per_byte = allocated as f64 / emitted as f64;
        assert!(
            per_byte < MAX_BYTES_PER_OUTPUT_BYTE,
            "rendering a depth-{depth} value allocated {allocated} bytes for \
             {emitted} bytes of output ({per_byte:.2} bytes per output byte, \
             limit {MAX_BYTES_PER_OUTPUT_BYTE}); the encoder should append into \
             one buffer and cost bytes linear in the emitted text, not copy each \
             subtree once per enclosing level"
        );
    }
}

/// AUDIT-5: the per-byte cost must not grow with depth.
///
/// This is the property that actually distinguishes the two encoders, because
/// a quadratic encoder can still be cheap at one fixed shallow depth while
/// being quadratic. Quadrupling the depth leaves a linear encoder's per-byte
/// cost flat and multiplies a quadratic encoder's by four, so the two are
/// separated by the *shape* of the curve rather than by its height.
///
/// Measured over a 4x depth increase: linear 2.51 to 2.55 (1.02x), pre-fix
/// 214.42 to 814.48 (3.80x). The 1.5x limit sits between them with room on
/// both sides.
#[test]
fn audit5_render_cost_does_not_grow_with_depth() {
    let (shallow_alloc, shallow_out) = render_cost(&nest(100));
    let (deep_alloc, deep_out) = render_cost(&nest(400));
    let shallow = shallow_alloc as f64 / shallow_out as f64;
    let deep = deep_alloc as f64 / deep_out as f64;

    assert!(
        deep < shallow * 1.5,
        "per-byte render cost grew {:.2}x (from {shallow:.2} at depth 100 to \
         {deep:.2} at depth 400) while the depth grew 4x; a linear encoder holds \
         this flat, whereas copying each subtree once per enclosing level makes \
         it grow with the depth",
        deep / shallow
    );
}
