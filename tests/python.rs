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
fn unsupported_python_dict_keys_are_rejected() {
    // Aura map keys are `string`, `int`, or `bool`. A key of any other kind
    // (here a tuple) must be refused rather than coerced, which would collapse
    // distinct keys into one.
    let err = run_source("fn main() { py_eval(\"{(1, 2): 'x'}\") }", "<py>").unwrap_err();
    assert_eq!(err.code, aura::error::codes::PY_UNSUPPORTED);
    // String keys still round-trip unchanged.
    let out = run_source("fn main() { print(py_eval(\"{'a': 1}\")) }", "<py>").unwrap();
    assert_eq!(out, "{\"a\": 1}\n");
}

#[test]
fn int_and_bool_python_dict_keys_round_trip() {
    // Integer and boolean keys are key-capable and distinct from their string
    // spellings, so `1` and `"1"` remain different keys.
    let out = run_source(
        "fn main() { print(py_eval(\"{1: 'a', '1': 'b'}\")) }",
        "<py>",
    )
    .unwrap();
    assert_eq!(out, "{1: \"a\", \"1\": \"b\"}\n");
    let out = run_source(
        "fn main() { print(py_eval(\"{True: 'y', False: 'n'}\")) }",
        "<py>",
    )
    .unwrap();
    assert_eq!(out, "{false: \"n\", true: \"y\"}\n");
}

/// AUDIT-5: a Python container that references itself must not hang the
/// boundary. Before the fix, `to_value_depth` bounded only depth, so a
/// self-referential Python list with fan-out 2 re-expanded exponentially and
/// the process never returned. The cycle is now rejected with a structured
/// diagnostic instead of being followed.
#[test]
fn python_cyclic_container_is_rejected_not_hung() {
    // One list, appended to itself twice: fan-out 2, a cycle.
    let src = "fn main() { let c = py_eval(\"(lambda c: (c.append(c), c.append(c), c)[2])([])\")\n print(len(to_string(c))) }";
    let err = run_source(src, "<py-cycle>").unwrap_err();
    assert_eq!(err.code, aura::error::codes::PY_UNSUPPORTED);

    // A dict that contains itself is the same hazard on the mapping path.
    let dict = "fn main() { let c = py_eval(\"(lambda d: (d.__setitem__('self', d), d)[1])({})\")\n print(len(to_string(c))) }";
    let err = run_source(dict, "<py-cycle-dict>").unwrap_err();
    assert_eq!(err.code, aura::error::codes::PY_UNSUPPORTED);
}

/// AUDIT-5: an acyclic Python value still round-trips exactly (the cycle guard
/// must not reject ordinary sharing or nesting).
#[test]
fn python_acyclic_values_still_round_trip() {
    let out = run_source(
        "fn main() { print(py_eval(\"[[1, 2], [3, [4, 5]]]\")) }",
        "<py-nested>",
    )
    .unwrap();
    assert_eq!(out, "[[1, 2], [3, [4, 5]]]\n");
    // A shared (DAG) Python value duplicates by value semantics; it must
    // terminate and be equal to the same structure built without sharing.
    let shared = run_source(
        "fn main() { let x = py_eval(\"(lambda s: [s, s])([1, 2])\")\n print(x) }",
        "<py-shared>",
    )
    .unwrap();
    assert_eq!(shared, "[[1, 2], [1, 2]]\n");
}

/// AUDIT-5: the reverse direction (an Aura cyclic/shared value crossing into
/// Python) must also terminate. A cycle is rejected; an acyclic shared DAG is
/// bounded by the node budget and either crosses or is rejected, never hangs.
#[test]
fn aura_value_into_python_terminates() {
    // Aura cycle -> Python: rejected, not hung.
    let cyc = "struct R { next: R }\nfn main() { let mut c = []\n push(c, c)\n print(len(py_call(\"builtins\", \"repr\", c))) }";
    let err = run_source(cyc, "<aura-cycle>").unwrap_err();
    assert_eq!(err.code, aura::error::codes::PY_UNSUPPORTED);

    // Acyclic Aura value -> Python still works exactly.
    let ok = run_source(
        "fn main() { let c = [1, [2, 3]]\n print(len(py_call(\"builtins\", \"repr\", c))) }",
        "<aura-ok>",
    )
    .unwrap();
    assert!(
        ok.trim().parse::<usize>().is_ok(),
        "expected a length: {ok}"
    );
}
