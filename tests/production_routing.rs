#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! B-1 production-routing tripwire.
//!
//! Engine-selection cannot be discriminated behaviorally on native: every
//! native production entry wraps execution in the 64 MiB execution substrate
//! (`INTERP_STACK`, `src/lib.rs`), which masks a regression from the machine
//! to the recursive evaluator — a reverted seam still passes the full
//! behavior suite. The only behavioral discriminators are the REPL canary
//! (`tests/b1_production_path.rs`, inline on a 1 MiB stack) and the fresh-wasm
//! substrate boundary (`playground/tests/node/b1_boundary.test.mjs`).
//!
//! This file supplies the missing **mechanical** proof for the remaining
//! seams: it pins, at the source level, that each production entry routes to
//! the explicit-continuation machine and that the recursive evaluator is
//! reachable only through the documented rollback API
//! (`Compilation::execute_recursive*`).
//!
//! This is intentionally a routing tripwire, not a style check: when engine
//! routing legitimately changes, update the pinned call sites here as a
//! deliberate act. Silently redirecting a production seam to recursion must
//! fail this test.

use std::path::Path;

fn source(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// The public library facade and `Compilation` methods route to the machine;
/// the recursive entry points live only inside the hidden rollback API.
#[test]
fn library_production_entries_route_to_the_machine() {
    let lib = source("src/lib.rs");

    // `execute_with_host_factory` (both branches), `execute_iterative` (both
    // branches, the oracle's explicit alias), and the free `execute_with` all
    // call the machine. If any production call is redirected to `run`, one of
    // these counts changes and this test fails.
    assert_eq!(
        count(&lib, "interp.run_iterative(&module)"),
        3,
        "expected the machine call in execute_with_host_factory, execute_iterative, \
         and the free aura::execute_with; a production seam was rerouted"
    );
    assert_eq!(
        count(&lib, "interp.run_iterative_sourced("),
        2,
        "expected the sourced machine call in both provider-backed production paths"
    );

    // The recursive engine is reachable only through the documented rollback
    // API. Exactly one @`run` and one `run_sourced` call may exist, and both
    // must sit inside `execute_recursive_host_factory`.
    assert_eq!(
        count(&lib, "interp.run(&module)"),
        1,
        "the recursive run call must exist only in the rollback API"
    );
    assert_eq!(
        count(&lib, "interp.run_sourced("),
        1,
        "the recursive sourced run call must exist only in the rollback API"
    );

    let rollback_at = lib
        .find("pub fn execute_recursive_host_factory")
        .expect("rollback API must exist (documented differential/rollback path)");
    for needle in ["interp.run(&module)", "interp.run_sourced("] {
        let at = lib
            .find(needle)
            .unwrap_or_else(|| panic!("missing {needle}"));
        assert!(
            at > rollback_at,
            "{needle} must only appear inside execute_recursive_host_factory"
        );
    }
}

/// The REPL production paths call the machine spellings and never the
/// recursive ones.
#[test]
fn repl_production_paths_route_to_the_machine() {
    let repl = source("src/repl.rs");

    for needle in [
        "exec_stmt_globals_iterative",
        "eval_globals_iterative",
        "run_item_iterative",
    ] {
        assert!(
            count(&repl, needle) >= 1,
            "REPL is missing the machine call `{needle}`"
        );
    }
    for needle in [
        "interp.run_item(",
        "interp.eval_globals(",
        "interp.exec_stmt_globals(",
        "interp.run(",
        "interp.run_sourced(",
    ] {
        assert_eq!(
            count(&repl, needle),
            0,
            "REPL must not call the recursive spelling `{needle}`"
        );
    }
}

/// The Playground runtime (the B-1 target substrate) routes every execution
/// entry to the machine.
#[test]
fn playground_runtime_routes_to_the_machine() {
    let pg = source("playground/runtime/src/lib.rs");

    assert!(
        count(&pg, "interp.run_iterative(&program)") >= 1,
        "playground `execute` must call the machine"
    );
    for needle in ["run_item_iterative", "eval_globals_iterative"] {
        assert!(
            count(&pg, needle) >= 1,
            "playground module capture is missing the machine call `{needle}`"
        );
    }
    for needle in [
        "interp.run(&program)",
        "interp.run_item(",
        "interp.eval_globals(",
        "interp.exec_stmt_globals(",
    ] {
        assert_eq!(
            count(&pg, needle),
            0,
            "playground must not call the recursive spelling `{needle}`"
        );
    }
}

/// Callback confinement (machine re-entry guard).
///
/// The machine must never re-enter the recursive evaluator through a callback.
/// Today the only producers that invoke an Aura value back are the three list
/// `map`/`filter`/`reduce` bodies, which the machine drives through the
/// resumable protocol; a future callback-taking native that bypassed the
/// protocol would silently reintroduce host-stack recursion proportional to
/// callback nesting on WASM. This tripwire pins both sides: no direct
/// recursive callback call in the machine, and the recursive `call_value_pub`
/// adapter confined to the three registered resumable bodies.
#[test]
fn callbacks_cannot_reenter_the_recursive_evaluator() {
    let machine = source("src/run/iterative.rs");
    assert_eq!(
        count(&machine, "call_value_pub("),
        0,
        "the machine must not call the recursive callback adapter \
         (`Interp::call_value_pub`); route callbacks through `start_call_value`"
    );
    assert_eq!(
        count(&machine, "drive_resumable("),
        0,
        "the machine must not use the recursive `drive_resumable` adapter; \
         schedule `NativeOutcome::InvokeCallback` as machine work"
    );

    let stdlib = source("src/stdlib/mod.rs");
    assert_eq!(
        count(&stdlib, "call_value_pub("),
        3,
        "`call_value_pub` must appear only in the resumable map/filter/reduce \
         bodies; a new callback-taking native must register through \
         `Interp::native_resumable` instead"
    );
}
