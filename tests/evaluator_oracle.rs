#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! B-1R2 — differential evaluator oracle.
//!
//! Test-only measuring instrument. It is **not** an evaluator and changes **no**
//! production behavior. Read `tests/oracle/mod.rs` for the isolation and
//! normalization contract.
//!
//! Tests in this file:
//!
//! 1. `oracle_corpus_matches_golden` — every corpus case is observed and must
//!    equal the committed golden manifest (pins current behavior, including the
//!    deliberately-preserved compound-assignment double evaluation and the B-1
//!    contract-boundary corpus).
//! 2. `engines_agree` — every engine on this build must agree on every case
//!    (baseline self-comparison today; recursive vs iterative under the
//!    `evaluator-oracle` feature in B-1R3).
//! 3. `execution_is_isolated` — a mutating program observed twice yields
//!    identical observables (no cross-run contamination).
//! 4. `golden_manifest_is_reproducible` — regenerating the manifest from the
//!    current tree yields exactly the committed file.
//! 5. `regenerate_golden` — ignored by default; rewrites the manifest.
//! 6. `closure_source_lifetime_probe` — Step 14 probe (diagnostic only).
//! 7. `reentrancy_classification` — Step 15 contract evidence.
//!
//! Run: `cargo test --locked --test evaluator_oracle`.

#[path = "oracle/cases.rs"]
mod cases;
#[path = "oracle/mod.rs"]
mod harness;

use std::collections::BTreeMap;

use harness::{
    encode_golden, engines, observe, parse_golden, Case, Completion, Observable, GOLDEN_PATH,
};

fn corpus() -> Vec<Case> {
    cases::cases()
}

fn observe_all() -> BTreeMap<String, Observable> {
    let mut out = BTreeMap::new();
    for case in corpus() {
        let obs = observe(&case, harness::Engine::recursive())
            .unwrap_or_else(|e| panic!("oracle harness failure for {}: {e}", case.key()));
        out.insert(case.key(), obs);
    }
    out
}

#[test]
fn oracle_corpus_matches_golden() {
    let golden_text = std::fs::read_to_string(GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing golden manifest {GOLDEN_PATH}: {e}"));
    let golden = parse_golden(&golden_text, "golden");
    let observed = observe_all();
    let corpus_keys: std::collections::BTreeSet<String> = corpus().iter().map(Case::key).collect();

    let mut failures = Vec::new();
    for case in corpus() {
        let key = case.key();
        let Some(expected) = golden.get(&key) else {
            failures.push(format!("{key}: present in corpus but absent from golden"));
            continue;
        };
        let Some(actual) = observed.get(&key) else {
            failures.push(format!("{key}: not observed"));
            continue;
        };
        if expected != actual {
            failures.push(format!(
                "{key}:\n  expected {expected:?}\n  actual   {actual:?}"
            ));
        }
    }
    for key in golden.keys() {
        if !corpus_keys.contains(key) {
            failures.push(format!("{key}: golden entry has no corpus case"));
        }
    }
    assert!(
        failures.is_empty(),
        "oracle corpus/golden divergence:\n{}",
        failures.join("\n")
    );
}

#[test]
fn engines_agree() {
    let engines = engines();
    let mut failures = Vec::new();
    for case in corpus() {
        let mut observed: Vec<(&str, Observable)> = Vec::new();
        for engine in &engines {
            let obs = observe(&case, *engine)
                .unwrap_or_else(|e| panic!("harness failure for {}: {e}", case.key()));
            observed.push((engine.name(), obs));
        }
        let (first_name, first) = &observed[0];
        for (name, obs) in observed.iter().skip(1) {
            if obs != first {
                failures.push(format!(
                    "{}: {first_name} != {name}\n  {first_name}: {first:?}\n  {name}: {obs:?}",
                    case.key()
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "engine disagreement:\n{}",
        failures.join("\n")
    );
}

#[test]
fn execution_is_isolated() {
    // A program that mutates a captured binding and a list.
    let mutating = Case {
        group: "isolation",
        name: "mutating",
        file: "isolation.aura",
        source: r#"fn main() {
  let mut x = 0
  let mut xs = []
  let f = () -> { x = x + 1
    xs.push(x) }
  f()
  f()
  f()
  print(x)
  print(xs)
}
"#,
        kind: harness::Kind::ExecuteProgram,
    };
    let other = Case {
        group: "isolation",
        name: "other",
        file: "other.aura",
        source: "fn main() { print(\"other\") }\n",
        kind: harness::Kind::ExecuteProgram,
    };

    // Interleave two different programs. If any interpreter, environment,
    // host, or stdout buffer were shared, the mutating program's state (or its
    // output) would bleed into or out of the other program.
    for _ in 0..5 {
        let a = observe(&mutating, harness::Engine::recursive()).unwrap();
        let b = observe(&other, harness::Engine::recursive()).unwrap();
        let c = observe(&mutating, harness::Engine::recursive()).unwrap();
        assert_eq!(a, c, "isolation violation: repeated observation diverged");
        assert_eq!(a.stdout, b"3\n[1, 2, 3]\n");
        assert_eq!(b.stdout, b"other\n", "stdout buffer leaked between runs");
    }

    // A returned observable must own its bytes: mutating a captured result must
    // not affect a fresh observation.
    let first = observe(&mutating, harness::Engine::recursive()).unwrap();
    let mut tampered = first.clone();
    tampered.stdout.extend_from_slice(b"tampered");
    let second = observe(&mutating, harness::Engine::recursive()).unwrap();
    assert_eq!(first, second, "observable bytes are aliased across runs");
}

#[test]
fn golden_manifest_is_reproducible() {
    let committed = std::fs::read_to_string(GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing golden manifest {GOLDEN_PATH}: {e}"));
    let regenerated = encode_golden(&corpus(), &observe_all());
    assert_eq!(
        committed, regenerated,
        "golden manifest is stale; run `cargo test --locked --test evaluator_oracle regenerate_golden -- --ignored`"
    );
}

#[test]
#[ignore = "regenerates the committed golden manifest"]
fn regenerate_golden() {
    let text = encode_golden(&corpus(), &observe_all());
    std::fs::write(GOLDEN_PATH, text).expect("write golden");
}

/// The corpus must cover every surface the design enumerates. This asserts
/// breadth structurally (groups and kind tags), not through brittle counts.
#[test]
fn corpus_covers_required_surfaces() {
    use std::collections::BTreeSet;

    let cs = corpus();
    let groups: BTreeSet<&str> = cs.iter().map(|c| c.group).collect();
    for g in [
        "core",
        "diag",
        "float",
        "provenance",
        "call-position",
        "callback",
        "compound",
        "finally",
        "b1-boundary",
        "module",
        "value",
    ] {
        assert!(groups.contains(g), "corpus is missing group `{g}`");
    }
    let mut tags: BTreeSet<&str> = BTreeSet::new();
    for c in &cs {
        tags.insert(harness::kind_tag(c.kind));
    }
    assert!(tags.contains("compile-module"));
    assert!(tags.contains("compile-program"));
    assert!(tags.contains("exec-module"));
    assert!(tags.contains("exec-program"));
    assert!(tags.contains("value"));
    assert!(tags.contains("multi"));

    // The call-position matrix must exercise at least these named positions.
    let names: BTreeSet<&str> = cs
        .iter()
        .filter(|c| c.group == "call-position")
        .map(|c| c.name)
        .collect();
    // The corpus must actually produce all three completion categories, so a
    // regression that turns every case into `ok` is caught structurally.
    assert_eq!(
        harness::distinct_completion_kinds(&cs),
        3,
        "corpus must exercise ok, compile-time, and runtime completions"
    );

    for n in [
        "function_argument",
        "binary_left",
        "binary_right_vs_left",
        "short_circuit",
        "index_base_and_key",
        "method_receiver_and_arg",
        "list_items",
        "map_key_and_value",
        "fstring_interpolation",
        "comprehension_value",
        "comprehension_filter",
        "match_guard",
        "return_expression",
        "assign_rhs",
        "range_bounds",
        "construct_args",
    ] {
        assert!(names.contains(n), "call-position matrix missing `{n}`");
    }
}

/// Anti-tautology guard: a small set of hand-verified expected outcomes,
/// independent of the generated golden file. This ensures the oracle is not
/// merely blessing whatever the engine happens to do (a bug committed before
/// golden generation would otherwise be invisible), and that the golden
/// manifest cannot be regenerated over a regression in these core semantics.
#[test]
fn hand_verified_anchors() {
    use harness::Completion;

    let expect_ok = |group: &'static str,
                     name: &'static str,
                     file: &'static str,
                     src: &'static str,
                     stdout: &[u8]| {
        let case = Case {
            group,
            name,
            file,
            source: src,
            kind: harness::Kind::ExecuteProgram,
        };
        let obs = observe(&case, harness::Engine::recursive()).unwrap();
        assert_eq!(obs.completion, Completion::Ok, "{group}/{name}");
        assert_eq!(obs.stdout, stdout, "{group}/{name}");
    };

    expect_ok(
        "anchor",
        "arithmetic",
        "anchor.aura",
        "fn main() { print(1 + 2 * 3) }\n",
        b"7\n",
    );
    expect_ok(
        "anchor",
        "eval_order",
        "anchor_order.aura",
        "fn a() -> int { print(\"a\"); return 1 }\nfn b() -> int { print(\"b\"); return 2 }\nfn add(x: int, y: int) -> int { return x + y }\nfn main() { print(add(a(), b())) }\n",
        b"a\nb\n3\n",
    );
    expect_ok(
        "anchor",
        "short_circuit",
        "anchor_sc.aura",
        "fn t() -> bool { print(\"t\"); return true }\nfn main() { print(false and t()) }\n",
        b"false\n",
    );
    expect_ok(
        "anchor",
        "captured_mutation",
        "anchor_cap.aura",
        "fn main() { let mut x = 1\n let f = () -> { x = x + 5 }\n f()\n print(x) }\n",
        b"6\n",
    );
    // A diagnostic anchor with hand-checked code, source, and span.
    let case = Case {
        group: "anchor",
        name: "div_zero_span",
        file: "anchor_div.aura",
        source: "fn main() { print(1 / 0) }\n",
        kind: harness::Kind::ExecuteProgram,
    };
    let obs = observe(&case, harness::Engine::recursive()).unwrap();
    match obs.completion {
        Completion::Runtime(d) => {
            assert_eq!(d.code, 4007);
            assert_eq!(d.source_name.as_deref(), Some("anchor_div.aura"));
            assert_eq!((d.line, d.column), (1, 21));
        }
        other => panic!("expected E4007, got {other:?}"),
    }
}

/// Step 14 — closure-source lifetime probe.
///
/// The B-1R1 review identified that `closure_sources` is keyed by the raw `Rc`
/// address of a closure and is never pruned. This probe determines whether
/// repeated closure creation/destruction causes unbounded metadata growth or
/// address-reuse misattribution. It is diagnostic only; it does **not** fix the
/// pre-existing behavior and does not fold it into B-1.
#[test]
fn closure_source_lifetime_probe() {
    // Many lambda creations in a loop, then a diagnostic from a lambda in a
    // separate source, to check attribution stability.
    let case = Case {
        group: "probe",
        name: "closure_churn",
        file: "churn.aura",
        source: r#"fn make(n: int) { return () -> n }
fn main() {
  let mut last = make(0)
  for i in range(2000) { last = make(i) }
  print(last())
}
"#,
        kind: harness::Kind::ExecuteProgram,
    };
    let a = observe(&case, harness::Engine::recursive()).unwrap();
    let b = observe(&case, harness::Engine::recursive()).unwrap();
    assert_eq!(a, b, "closure churn made observation nondeterministic");

    // Address-reuse misattribution: a lambda defined in `child.aura` that
    // faults must attribute to `child.aura` even after heavy churn in the
    // entry source before it.
    let case = Case {
        group: "probe",
        name: "closure_churn_attribution",
        file: "main.aura",
        source: "",
        kind: harness::Kind::Multi(&cases::MULTI_CHURN),
    };
    let obs = observe(&case, harness::Engine::recursive()).unwrap();
    match &obs.completion {
        Completion::Runtime(d) => {
            eprintln!(
                "closure_source_lifetime_probe: runtime diagnostic {} at {:?}:{}:{}",
                d.code, d.source_name, d.line, d.column
            );
            assert_eq!(
                d.source_name.as_deref(),
                Some("child.aura"),
                "lambda fault misattributed its source after churn: {d:?}"
            );
        }
        other => panic!("expected a runtime diagnostic in child.aura, got {other:?}"),
    }
}

/// Step 15 — host/native reentrancy contract evidence.
///
/// This test itself exercises the documented behavior: an embedder-registered
/// native that calls back into `Interp::call`/`call_value` nests a Rust call
/// inside the interpreter frame that invoked it. The probe records the
/// classification it observes rather than asserting a language decision.
#[test]
fn reentrancy_classification() {
    use aura::error::Span;
    use aura::run::value::Value;
    use aura::run::{Ctl, Interp};

    // Build a first-class Aura closure value by evaluating a lambda expression.
    let module = aura::compile_with_mode("() -> 41\n", aura::CompileMode::Module)
        .expect("lambda module compiles");
    let mut interp = Interp::new();
    let mut closure = Value::None;
    for item in &module.items {
        if let aura::ast::Item::Expr(e, _) = item {
            if let Ctl::Val(v) = interp.eval_globals(e).expect("lambda evaluates") {
                closure = v;
            }
        }
    }
    assert!(
        matches!(closure, Value::Closure(_)),
        "expected a closure value"
    );

    // A host-registered native that synchronously re-enters Aura to call the
    // closure it is handed. This is exactly the embedder pattern from
    // `Interp::native`.
    interp.native("reenter", |it, args, span| {
        let f = args.first().cloned().unwrap_or(Value::None);
        it.call_value(f, Vec::new(), span)
    });

    // Evidence of SUPPORTED reentry: invoking the native from Aura's own call
    // path returns the nested call's value.
    let result = interp
        .call_value(
            Value::Native("reenter".to_string()),
            vec![closure],
            Span::default(),
        )
        .expect("host-registered native synchronously re-entered Aura");
    assert_eq!(result.display(), "41");
}

/// The oracle must be able to run under the non-default feature in B-1R3;
/// this test documents the feature gate without requiring it.
#[test]
fn oracle_feature_gate_is_documented() {
    assert_eq!(harness::ORACLE_FEATURE, "evaluator-oracle");
    // Default builds have exactly one engine; the feature adds the second.
    let n = engines().len();
    if cfg!(feature = "evaluator-oracle") {
        assert_eq!(n, 2, "feature build must expose two engines");
    } else {
        assert_eq!(n, 1, "default build must expose only the recursive engine");
    }
}

/// Guard against accidental use of the oracle in production: the production
/// library must not depend on the test-only feature. This is a static check on
/// `Cargo.toml` (documentation of Step 22), kept cheap.
#[test]
fn feature_is_not_default() {
    let cargo = std::fs::read_to_string("Cargo.toml").unwrap();
    let default_line = cargo
        .lines()
        .find(|l| l.trim_start().starts_with("default ="))
        .unwrap_or("");
    assert!(
        !default_line.contains("evaluator-oracle"),
        "evaluator-oracle must never be a default feature"
    );
}
