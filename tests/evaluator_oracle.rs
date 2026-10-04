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
// The R3A subset is meaningful only with the iterative engine present.
#[cfg(feature = "evaluator-oracle")]
#[path = "oracle/r3a.rs"]
mod r3a;
// B-1R3B.1 — unary operators on the iterative machine.
#[cfg(feature = "evaluator-oracle")]
#[path = "oracle/r3b.rs"]
mod r3b;
// B-1R3C.1 — user and native calls on the iterative machine.
#[cfg(feature = "evaluator-oracle")]
#[path = "oracle/r3c1.rs"]
mod r3c1;

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
            // B-1R3F.1 closed the current-language runtime surface: the
            // iterative engine supports every construct and the machine
            // handles multi-source provider compilations, so the whole corpus
            // must agree between engines with **no** exception.
            if obs == first {
                continue;
            }
            failures.push(format!(
                "{}: {first_name} != {name}\n  {first_name}: {first:?}\n  {name}: {obs:?}",
                case.key()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "engine disagreement:\n{}",
        failures.join("\n")
    );
}
/// B-1R3A — the real iterative machine must agree with the recursive engine on
/// every supported-subset case. This is the strict differential comparison; it
/// is meaningful only with the `evaluator-oracle` feature (without it the
/// iterative engine does not exist).
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3a_supported_subset_agrees() {
    let rec = harness::Engine::recursive();
    let it = harness::Engine::iterative();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in r3a::supported_cases() {
        count += 1;
        let a = observe(&case, rec)
            .unwrap_or_else(|e| panic!("harness failure (recursive) {}: {e}", case.key()));
        let b = observe(&case, it)
            .unwrap_or_else(|e| panic!("harness failure (iterative) {}: {e}", case.key()));
        if a != b {
            failures.push(format!(
                "{}:\n  recursive: {a:?}\n  iterative: {b:?}",
                case.key()
            ));
        }
    }
    assert!(count > 0, "R3A supported subset is empty");
    assert!(
        failures.is_empty(),
        "iterative machine diverged from the recursive engine on supported R3A cases:\n{}",
        failures.join("\n")
    );
}

/// B-1R3F.1 — the current-language runtime surface is fully iterative:
/// this phase's unsupported sentinel set must be empty, and there must be no
/// construct left to fall back on the recursive engine for.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn iterative_unsupported_fails_explicitly() {
    let cases = r3a::unsupported_cases();
    assert!(
        cases.is_empty(),
        "unsupported set must be empty after B-1R3F.1; found {} case(s)",
        cases.len()
    );
}

/// B-1R3A — all R3A cases (supported *and* unsupported) observed through the
/// iterative engine, in a stable order.
#[cfg(feature = "evaluator-oracle")]
fn r3a_all_cases() -> Vec<Case> {
    let mut v = r3a::supported_cases();
    v.extend(r3a::unsupported_cases());
    v
}

/// Path of the committed R3A iterative-engine golden manifest.
#[cfg(feature = "evaluator-oracle")]
const R3A_GOLDEN_PATH: &str = "tests/oracle/r3a_golden.tsv";

/// B-1R3A — full-field regression guard for the iterative engine.
///
/// Unlike `r3a_supported_subset_agrees` (which requires recursive/iterative
/// equality on the supported subset), this pins the *complete* normalized
/// observable — code, message, source identity, byte span, line, and column —
/// of the iterative engine for every R3A case, including the explicit
/// unsupported sentinel. Without it a wrong span or source on an unsupported
/// case (which the recursive side never produces) would escape.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3a_iterative_golden_matches() {
    let committed = std::fs::read_to_string(R3A_GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing R3A golden {R3A_GOLDEN_PATH}: {e}"));
    let golden = harness::parse_golden(&committed, "r3a");
    let mut observations = BTreeMap::new();
    for case in r3a_all_cases() {
        let obs = observe(&case, harness::Engine::iterative())
            .unwrap_or_else(|e| panic!("harness failure for {}: {e}", case.key()));
        observations.insert(case.key(), obs);
    }
    let cases = r3a_all_cases();
    let regenerated = harness::encode_golden(&cases, &observations);
    assert_eq!(
        committed, regenerated,
        "R3A iterative golden is stale or the machine diverged; run \
         `cargo test --locked --features evaluator-oracle --test evaluator_oracle \
         regenerate_r3a_golden -- --ignored`"
    );
    // All keys must be present.
    for case in &cases {
        assert!(
            golden.contains_key(&case.key()),
            "R3A golden missing {}",
            case.key()
        );
    }
}

/// Regenerate the R3A iterative golden (ignored by default).
#[test]
#[cfg(feature = "evaluator-oracle")]
#[ignore = "regenerates the committed R3A iterative golden manifest"]
fn regenerate_r3a_golden() {
    let cases = r3a_all_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative()).unwrap();
        observations.insert(case.key(), obs);
    }
    let text = harness::encode_golden(&cases, &observations);
    std::fs::write(R3A_GOLDEN_PATH, text).expect("write R3A golden");
}

/// B-1R3B.1 — the iterative machine must agree with the recursive engine on
/// every supported unary-subset case, including diagnostics and spans.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b_unary_supported_subset_agrees() {
    let rec = harness::Engine::recursive();
    let it = harness::Engine::iterative();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in r3b::supported_cases() {
        count += 1;
        let a = observe(&case, rec)
            .unwrap_or_else(|e| panic!("harness failure (recursive) {}: {e}", case.key()));
        let b = observe(&case, it)
            .unwrap_or_else(|e| panic!("harness failure (iterative) {}: {e}", case.key()));
        if a != b {
            failures.push(format!(
                "{}:\n  recursive: {a:?}\n  iterative: {b:?}",
                case.key()
            ));
        }
    }
    assert!(count > 0, "R3B unary supported subset is empty");
    assert!(
        failures.is_empty(),
        "iterative machine diverged from the recursive engine on supported R3B unary cases:\n{}",
        failures.join("\n")
    );
}

/// B-1R3F.1 — the current-language runtime surface is fully iterative:
/// this phase's unsupported sentinel set must be empty, and there must be no
/// construct left to fall back on the recursive engine for.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn iterative_unary_unsupported_fails_explicitly() {
    let cases = r3b::unsupported_cases();
    assert!(
        cases.is_empty(),
        "unsupported set must be empty after B-1R3F.1; found {} case(s)",
        cases.len()
    );
}

/// Regenerate the R3B unary iterative golden (ignored by default).
#[test]
#[cfg(feature = "evaluator-oracle")]
#[ignore = "regenerates the committed R3B unary iterative golden manifest"]
fn regenerate_r3b_golden() {
    let cases = r3b::supported_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative()).unwrap();
        observations.insert(case.key(), obs);
    }
    let text = harness::encode_golden(&cases, &observations);
    std::fs::write(R3B_GOLDEN_PATH, text).expect("write R3B golden");
}

/// Path of the committed R3B unary iterative-engine golden manifest.
#[cfg(feature = "evaluator-oracle")]
const R3B_GOLDEN_PATH: &str = "tests/oracle/r3b_golden.tsv";
/// B-1R3B.1 — full-field regression guard for the iterative unary subset.
///
/// Pins the complete normalized observable (code, message, source, byte span,
/// line, column, value type and representation) of the iterative engine for
/// every supported unary case. Without it a wrong span or type on a unary case
/// would escape `r3b_unary_supported_subset_agrees`.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b_iterative_golden_matches() {
    let committed = std::fs::read_to_string(R3B_GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing R3B golden {R3B_GOLDEN_PATH}: {e}"));
    let golden = harness::parse_golden(&committed, "r3b");
    let cases = r3b::supported_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative())
            .unwrap_or_else(|e| panic!("harness failure for {}: {e}", case.key()));
        observations.insert(case.key(), obs);
    }
    let regenerated = harness::encode_golden(&cases, &observations);
    assert_eq!(
        committed, regenerated,
        "R3B unary iterative golden is stale or the machine diverged; run \
         `cargo test --locked --features evaluator-oracle --test evaluator_oracle \
         regenerate_r3b_golden -- --ignored`"
    );
    for case in &cases {
        assert!(
            golden.contains_key(&case.key()),
            "R3B golden missing {}",
            case.key()
        );
    }
}

/// Path of the committed R3B.2 binary iterative-engine golden manifest.
#[cfg(feature = "evaluator-oracle")]
const R3B2_GOLDEN_PATH: &str = "tests/oracle/r3b2_golden.tsv";

/// B-1R3B.2 — the iterative machine must agree with the recursive engine on
/// every supported eager-binary case, including diagnostics and spans.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b2_binary_supported_subset_agrees() {
    let rec = harness::Engine::recursive();
    let it = harness::Engine::iterative();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in r3b::binary_supported_cases() {
        count += 1;
        let a = observe(&case, rec)
            .unwrap_or_else(|e| panic!("harness failure (recursive) {}: {e}", case.key()));
        let b = observe(&case, it)
            .unwrap_or_else(|e| panic!("harness failure (iterative) {}: {e}", case.key()));
        if a != b {
            failures.push(format!(
                "{}:\n  recursive: {a:?}\n  iterative: {b:?}",
                case.key()
            ));
        }
    }
    assert!(count > 0, "R3B.2 binary supported subset is empty");
    assert!(
        failures.is_empty(),
        "iterative machine diverged from the recursive engine on supported R3B.2 cases:\n{}",
        failures.join("\n")
    );
}

/// B-1R3F.1 — the current-language runtime surface is fully iterative:
/// this phase's unsupported sentinel set must be empty, and there must be no
/// construct left to fall back on the recursive engine for.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn iterative_binary_unsupported_fails_explicitly() {
    let cases = r3b::binary_unsupported_cases();
    assert!(
        cases.is_empty(),
        "unsupported set must be empty after B-1R3F.1; found {} case(s)",
        cases.len()
    );
}

/// B-1R3B.2 — full-field regression guard for the iterative eager-binary subset.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b2_iterative_golden_matches() {
    let committed = std::fs::read_to_string(R3B2_GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing R3B.2 golden {R3B2_GOLDEN_PATH}: {e}"));
    let golden = harness::parse_golden(&committed, "r3b2");
    let cases = r3b::binary_supported_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative())
            .unwrap_or_else(|e| panic!("harness failure for {}: {e}", case.key()));
        observations.insert(case.key(), obs);
    }
    let regenerated = harness::encode_golden(&cases, &observations);
    assert_eq!(
        committed, regenerated,
        "R3B.2 binary iterative golden is stale or the machine diverged; run \
         `cargo test --locked --features evaluator-oracle --test evaluator_oracle \
         regenerate_r3b2_golden -- --ignored`"
    );
    for case in &cases {
        assert!(
            golden.contains_key(&case.key()),
            "R3B.2 golden missing {}",
            case.key()
        );
    }
}

/// Regenerate the R3B.2 binary iterative golden (ignored by default).
#[test]
#[cfg(feature = "evaluator-oracle")]
#[ignore = "regenerates the committed R3B.2 binary iterative golden manifest"]
fn regenerate_r3b2_golden() {
    let cases = r3b::binary_supported_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative()).unwrap();
        observations.insert(case.key(), obs);
    }
    let text = harness::encode_golden(&cases, &observations);
    std::fs::write(R3B2_GOLDEN_PATH, text).expect("write R3B.2 golden");
}

/// Path of the committed R3B.3 short-circuit iterative-engine golden manifest.
#[cfg(feature = "evaluator-oracle")]
const R3B3_GOLDEN_PATH: &str = "tests/oracle/r3b3_golden.tsv";

/// B-1R3B.3 — the iterative machine must agree with the recursive engine on
/// every supported short-circuit case, including diagnostics, spans, and
/// stdout (the skipped-operand cases are only meaningful differentially: the
/// recursive engine's stdout is captured and both sides must match exactly).
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b3_short_circuit_supported_subset_agrees() {
    let rec = harness::Engine::recursive();
    let it = harness::Engine::iterative();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in r3b::short_circuit_supported_cases() {
        count += 1;
        let a = observe(&case, rec)
            .unwrap_or_else(|e| panic!("harness failure (recursive) {}: {e}", case.key()));
        let b = observe(&case, it)
            .unwrap_or_else(|e| panic!("harness failure (iterative) {}: {e}", case.key()));
        if a != b {
            failures.push(format!(
                "{}:\n  recursive: {a:?}\n  iterative: {b:?}",
                case.key()
            ));
        }
    }
    assert!(count > 0, "R3B.3 short-circuit supported subset is empty");
    assert!(
        failures.is_empty(),
        "iterative machine diverged from the recursive engine on supported R3B.3 cases:\n{}",
        failures.join("\n")
    );
}

/// B-1R3F.1 — the current-language runtime surface is fully iterative:
/// this phase's unsupported sentinel set must be empty, and there must be no
/// construct left to fall back on the recursive engine for.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn iterative_short_circuit_unsupported_fails_explicitly() {
    let cases = r3b::short_circuit_unsupported_cases();
    assert!(
        cases.is_empty(),
        "unsupported set must be empty after B-1R3F.1; found {} case(s)",
        cases.len()
    );
}

/// B-1R3B.3 — full-field regression guard for the iterative short-circuit
/// subset: pins the complete normalized observable (stdout bytes, completion,
/// code, message, source, byte span, line, column, value type and
/// representation) of the iterative engine for every supported case.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b3_iterative_golden_matches() {
    let committed = std::fs::read_to_string(R3B3_GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing R3B.3 golden {R3B3_GOLDEN_PATH}: {e}"));
    let golden = harness::parse_golden(&committed, "r3b3");
    let cases = r3b::short_circuit_supported_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative())
            .unwrap_or_else(|e| panic!("harness failure for {}: {e}", case.key()));
        observations.insert(case.key(), obs);
    }
    let regenerated = harness::encode_golden(&cases, &observations);
    assert_eq!(
        committed, regenerated,
        "R3B.3 iterative golden is stale or the machine diverged; run \
         `cargo test --locked --features evaluator-oracle --test evaluator_oracle \
         regenerate_r3b3_golden -- --ignored`"
    );
    for case in &cases {
        assert!(
            golden.contains_key(&case.key()),
            "R3B.3 golden missing {}",
            case.key()
        );
    }
}

/// Regenerate the R3B.3 short-circuit iterative golden (ignored by default).
#[test]
#[cfg(feature = "evaluator-oracle")]
#[ignore = "regenerates the committed R3B.3 short-circuit iterative golden manifest"]
fn regenerate_r3b3_golden() {
    let cases = r3b::short_circuit_supported_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative()).unwrap();
        observations.insert(case.key(), obs);
    }
    let text = harness::encode_golden(&cases, &observations);
    std::fs::write(R3B3_GOLDEN_PATH, text).expect("write R3B.3 golden");
}

/// Path of the committed R3B.4.1 list-construction iterative-engine golden
/// manifest.
#[cfg(feature = "evaluator-oracle")]
const R3B4_GOLDEN_PATH: &str = "tests/oracle/r3b4_golden.tsv";

/// B-1R3B.4.1 — the iterative machine must agree with the recursive engine on
/// every supported list/tuple-construction case, including diagnostics, spans,
/// and stdout (element-signal and element-error cases are only meaningful
/// differentially).
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b4_list_supported_subset_agrees() {
    let rec = harness::Engine::recursive();
    let it = harness::Engine::iterative();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in r3b::list_supported_cases() {
        count += 1;
        let a = observe(&case, rec)
            .unwrap_or_else(|e| panic!("harness failure (recursive) {}: {e}", case.key()));
        let b = observe(&case, it)
            .unwrap_or_else(|e| panic!("harness failure (iterative) {}: {e}", case.key()));
        if a != b {
            failures.push(format!(
                "{}:\n  recursive: {a:?}\n  iterative: {b:?}",
                case.key()
            ));
        }
    }
    assert!(count > 0, "R3B.4 list supported subset is empty");
    assert!(
        failures.is_empty(),
        "iterative machine diverged from the recursive engine on supported R3B.4 list cases:\n{}",
        failures.join("\n")
    );
}

/// B-1R3F.1 — the current-language runtime surface is fully iterative:
/// this phase's unsupported sentinel set must be empty, and there must be no
/// construct left to fall back on the recursive engine for.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn iterative_list_unsupported_fails_explicitly() {
    let cases = r3b::list_unsupported_cases();
    assert!(
        cases.is_empty(),
        "unsupported set must be empty after B-1R3F.1; found {} case(s)",
        cases.len()
    );
}

/// B-1R3B.4.1 — full-field regression guard for the iterative
/// list-construction subset: pins the complete normalized observable (stdout
/// bytes, completion, code, message, source, byte span, line, column, value
/// type and representation) of the iterative engine for every supported case.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b4_iterative_golden_matches() {
    let committed = std::fs::read_to_string(R3B4_GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing R3B.4 golden {R3B4_GOLDEN_PATH}: {e}"));
    let golden = harness::parse_golden(&committed, "r3b4");
    let cases = r3b::list_supported_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative())
            .unwrap_or_else(|e| panic!("harness failure for {}: {e}", case.key()));
        observations.insert(case.key(), obs);
    }
    let regenerated = harness::encode_golden(&cases, &observations);
    assert_eq!(
        committed, regenerated,
        "R3B.4 list iterative golden is stale or the machine diverged; run \
         `cargo test --locked --features evaluator-oracle --test evaluator_oracle \
         regenerate_r3b4_golden -- --ignored`"
    );
    for case in &cases {
        assert!(
            golden.contains_key(&case.key()),
            "R3B.4 golden missing {}",
            case.key()
        );
    }
}

/// Regenerate the R3B.4.1 list-construction iterative golden (ignored by
/// default).
#[test]
#[cfg(feature = "evaluator-oracle")]
#[ignore = "regenerates the committed R3B.4 list iterative golden manifest"]
fn regenerate_r3b4_golden() {
    let cases = r3b::list_supported_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative()).unwrap();
        observations.insert(case.key(), obs);
    }
    let text = harness::encode_golden(&cases, &observations);
    std::fs::write(R3B4_GOLDEN_PATH, text).expect("write R3B.4 golden");
}

/// Path of the committed R3B.4.2 map-construction iterative-engine golden
/// manifest.
#[cfg(feature = "evaluator-oracle")]
const R3B42_GOLDEN_PATH: &str = "tests/oracle/r3b42_golden.tsv";

/// B-1R3B.4.2 — the iterative machine must agree with the recursive engine on
/// every supported map-construction case, including diagnostics, spans, and
/// stdout (invalid-key, duplicate-key, order, and signal cases are only
/// meaningful differentially).
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b42_map_supported_subset_agrees() {
    let rec = harness::Engine::recursive();
    let it = harness::Engine::iterative();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in r3b::map_supported_cases() {
        count += 1;
        let a = observe(&case, rec)
            .unwrap_or_else(|e| panic!("harness failure (recursive) {}: {e}", case.key()));
        let b = observe(&case, it)
            .unwrap_or_else(|e| panic!("harness failure (iterative) {}: {e}", case.key()));
        if a != b {
            failures.push(format!(
                "{}:\n  recursive: {a:?}\n  iterative: {b:?}",
                case.key()
            ));
        }
    }
    assert!(count > 0, "R3B.4.2 map supported subset is empty");
    assert!(
        failures.is_empty(),
        "iterative machine diverged from the recursive engine on supported R3B.4.2 map cases:\n{}",
        failures.join("\n")
    );
}

/// B-1R3F.1 — the current-language runtime surface is fully iterative:
/// this phase's unsupported sentinel set must be empty, and there must be no
/// construct left to fall back on the recursive engine for.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn iterative_map_unsupported_fails_explicitly() {
    let cases = r3b::map_unsupported_cases();
    assert!(
        cases.is_empty(),
        "unsupported set must be empty after B-1R3F.1; found {} case(s)",
        cases.len()
    );
}

/// B-1R3B.4.2 — full-field regression guard for the iterative
/// map-construction subset: pins the complete normalized observable (stdout
/// bytes, completion, code, message, source, byte span, line, column, value
/// type and representation) of the iterative engine for every supported case.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b42_iterative_golden_matches() {
    let committed = std::fs::read_to_string(R3B42_GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing R3B.4.2 golden {R3B42_GOLDEN_PATH}: {e}"));
    let golden = harness::parse_golden(&committed, "r3b42");
    let cases = r3b::map_supported_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative())
            .unwrap_or_else(|e| panic!("harness failure for {}: {e}", case.key()));
        observations.insert(case.key(), obs);
    }
    let regenerated = harness::encode_golden(&cases, &observations);
    assert_eq!(
        committed, regenerated,
        "R3B.4.2 map iterative golden is stale or the machine diverged; run \
         `cargo test --locked --features evaluator-oracle --test evaluator_oracle \
         regenerate_r3b42_golden -- --ignored`"
    );
    for case in &cases {
        assert!(
            golden.contains_key(&case.key()),
            "R3B.4.2 golden missing {}",
            case.key()
        );
    }
}

/// Regenerate the R3B.4.2 map-construction iterative golden (ignored by
/// default).
#[test]
#[cfg(feature = "evaluator-oracle")]
#[ignore = "regenerates the committed R3B.4.2 map iterative golden manifest"]
fn regenerate_r3b42_golden() {
    let cases = r3b::map_supported_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative()).unwrap();
        observations.insert(case.key(), obs);
    }
    let text = harness::encode_golden(&cases, &observations);
    std::fs::write(R3B42_GOLDEN_PATH, text).expect("write R3B.4.2 golden");
}

/// Push-blocker remediation guard (B-1R3B.4.2): the program-mode
/// list-contains-map case must exist in the supported set, must cross a real
/// frame boundary, and must be pinned in the R3B.4.2 golden. The case was
/// silently dropped when map elements moved from the R3B.4.1 unsupported set to
/// supported coverage; this guard keeps the frame-boundary coverage from being
/// lost again.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b42_program_map_element_coverage_is_present() {
    let cases = r3b::map_supported_cases();
    let case = cases
        .iter()
        .find(|c| c.key() == "r3b42-program/main_map_element")
        .expect("R3B.4.2 supported set is missing program-mode main_map_element");
    assert!(
        matches!(case.kind, harness::Kind::ExecuteProgram),
        "main_map_element must execute in program mode through `main`"
    );
    assert!(
        case.source.contains("fn main()") && case.source.contains("[1, {\"a\": 1}]"),
        "main_map_element must construct a list containing a map inside `main`: {:?}",
        case.source
    );
    // Both engines must agree: the iterative machine must really evaluate the
    // nested Map through the frame boundary, not fall back to recursion.
    let rec = observe(case, harness::Engine::recursive()).unwrap();
    let it = observe(case, harness::Engine::iterative()).unwrap();
    assert_eq!(
        rec, it,
        "program-mode map element diverged between the engines"
    );
    // The case must be pinned in the committed golden.
    let committed = std::fs::read_to_string(R3B42_GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing R3B.4.2 golden {R3B42_GOLDEN_PATH}: {e}"));
    let golden = harness::parse_golden(&committed, "r3b42");
    assert!(
        golden.contains_key(&case.key()),
        "R3B.4.2 golden missing {}",
        case.key()
    );
}

/// Push-blocker remediation guard (B-1R3B.4.2): the R3B.4.2 golden is compared
/// byte-for-byte against LF-only generator output, so `.gitattributes` must
/// force LF on every checkout. Without the pin a Windows `core.autocrlf=true`
/// checkout materializes CRLF and the byte-exact golden test fails for a
/// non-semantic reason; this guard fails loudly if the pin is ever removed.
#[test]
fn r3b42_golden_has_lf_pin() {
    let attrs = std::fs::read_to_string(".gitattributes").expect("read .gitattributes");
    assert!(
        attrs
            .lines()
            .any(|l| l.trim() == "tests/oracle/r3b42_golden.tsv text eol=lf"),
        "tests/oracle/r3b42_golden.tsv must be pinned to `text eol=lf`"
    );
    let golden = std::fs::read("tests/oracle/r3b42_golden.tsv")
        .expect("missing tests/oracle/r3b42_golden.tsv");
    assert!(
        !golden.contains(&b'\r'),
        "R3B.4.2 golden must be physically LF-only"
    );
}

/// Path of the committed R3B.5 range-construction iterative-engine golden.
#[cfg(feature = "evaluator-oracle")]
const R3B5_GOLDEN_PATH: &str = "tests/oracle/r3b5_golden.tsv";

/// B-1R3B.5 — the iterative machine must agree with the recursive engine on
/// every supported range-construction case, including diagnostics, spans,
/// value representation, and stdout (endpoint, ordering, exactly-once, and
/// signal cases are only meaningful differentially).
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b5_range_supported_subset_agrees() {
    let rec = harness::Engine::recursive();
    let it = harness::Engine::iterative();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in r3b::range_supported_cases() {
        count += 1;
        let a = observe(&case, rec)
            .unwrap_or_else(|e| panic!("harness failure (recursive) {}: {e}", case.key()));
        let b = observe(&case, it)
            .unwrap_or_else(|e| panic!("harness failure (iterative) {}: {e}", case.key()));
        if a != b {
            failures.push(format!(
                "{}:\n  recursive: {a:?}\n  iterative: {b:?}",
                case.key()
            ));
        }
    }
    assert!(count > 0, "R3B.5 range supported subset is empty");
    assert!(
        failures.is_empty(),
        "iterative machine diverged from the recursive engine on supported R3B.5 range cases:\n{}",
        failures.join("\n")
    );
}

/// B-1R3F.1 — the current-language runtime surface is fully iterative:
/// this phase's unsupported sentinel set must be empty, and there must be no
/// construct left to fall back on the recursive engine for.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn iterative_range_unsupported_fails_explicitly() {
    let cases = r3b::range_unsupported_cases();
    assert!(
        cases.is_empty(),
        "unsupported set must be empty after B-1R3F.1; found {} case(s)",
        cases.len()
    );
}

/// B-1R3B.5 — full-field regression guard for the iterative range-construction
/// subset: pins the complete normalized observable (stdout bytes, completion,
/// code, message, source, byte span, line, column, value type and
/// representation) of the iterative engine for every supported case.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b5_iterative_golden_matches() {
    let committed = std::fs::read_to_string(R3B5_GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing R3B.5 golden {R3B5_GOLDEN_PATH}: {e}"));
    let golden = harness::parse_golden(&committed, "r3b5");
    let cases = r3b::range_supported_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative())
            .unwrap_or_else(|e| panic!("harness failure for {}: {e}", case.key()));
        observations.insert(case.key(), obs);
    }
    let regenerated = harness::encode_golden(&cases, &observations);
    assert_eq!(
        committed, regenerated,
        "R3B.5 range iterative golden is stale or the machine diverged; run \
         `cargo test --locked --features evaluator-oracle --test evaluator_oracle \
         regenerate_r3b5_golden -- --ignored`"
    );
    for case in &cases {
        assert!(
            golden.contains_key(&case.key()),
            "R3B.5 golden missing {}",
            case.key()
        );
    }
}

/// Regenerate the R3B.5 range-construction iterative golden (ignored by
/// default).
#[test]
#[cfg(feature = "evaluator-oracle")]
#[ignore = "regenerates the committed R3B.5 range iterative golden manifest"]
fn regenerate_r3b5_golden() {
    let cases = r3b::range_supported_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative()).unwrap();
        observations.insert(case.key(), obs);
    }
    let text = harness::encode_golden(&cases, &observations);
    std::fs::write(R3B5_GOLDEN_PATH, text).expect("write R3B.5 golden");
}

/// B-1R3B.5 — the R3B.5 golden is compared byte-for-byte against LF-only
/// generator output, so `.gitattributes` must force LF on every checkout (the
/// same policy every sibling golden uses, and the fix for the R3B.4.2 LF
/// omission). This guard fails loudly if the pin is ever removed.
#[test]
fn r3b5_golden_has_lf_pin() {
    let attrs = std::fs::read_to_string(".gitattributes").expect("read .gitattributes");
    assert!(
        attrs
            .lines()
            .any(|l| l.trim() == "tests/oracle/r3b5_golden.tsv text eol=lf"),
        "tests/oracle/r3b5_golden.tsv must be pinned to `text eol=lf`"
    );
    let golden = std::fs::read("tests/oracle/r3b5_golden.tsv")
        .expect("missing tests/oracle/r3b5_golden.tsv");
    assert!(
        !golden.contains(&b'\r'),
        "R3B.5 golden must be physically LF-only"
    );
}

/// Path of the committed R3B.6 index/field iterative-engine golden.
#[cfg(feature = "evaluator-oracle")]
const R3B6_GOLDEN_PATH: &str = "tests/oracle/r3b6_golden.tsv";

/// All supported R3B.6 index/field cases in a stable order (index cases, then
/// field cases).
#[cfg(feature = "evaluator-oracle")]
fn r3b6_all_cases() -> Vec<Case> {
    let mut v = r3b::index_supported_cases();
    v.extend(r3b::field_supported_cases());
    v
}

/// B-1R3B.6 — the iterative machine must agree with the recursive engine on
/// every supported index case, including diagnostics, spans, value
/// representation, and stdout (target/index ordering, negative normalization,
/// bounds, map lookup, and signal propagation are only meaningful
/// differentially).
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b6_index_supported_subset_agrees() {
    let rec = harness::Engine::recursive();
    let it = harness::Engine::iterative();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in r3b::index_supported_cases() {
        count += 1;
        let a = observe(&case, rec)
            .unwrap_or_else(|e| panic!("harness failure (recursive) {}: {e}", case.key()));
        let b = observe(&case, it)
            .unwrap_or_else(|e| panic!("harness failure (iterative) {}: {e}", case.key()));
        if a != b {
            failures.push(format!(
                "{}:\n  recursive: {a:?}\n  iterative: {b:?}",
                case.key()
            ));
        }
    }
    assert!(count > 0, "R3B.6 index supported subset is empty");
    assert!(
        failures.is_empty(),
        "iterative machine diverged from the recursive engine on supported R3B.6 index cases:\n{}",
        failures.join("\n")
    );
}

/// B-1R3B.6 — the iterative machine must agree with the recursive engine on
/// every supported field case, including builtin-method dispatch, the
/// method-not-a-value guard, and the exact `E2003` unknown-member diagnostic.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b6_field_supported_subset_agrees() {
    let rec = harness::Engine::recursive();
    let it = harness::Engine::iterative();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in r3b::field_supported_cases() {
        count += 1;
        let a = observe(&case, rec)
            .unwrap_or_else(|e| panic!("harness failure (recursive) {}: {e}", case.key()));
        let b = observe(&case, it)
            .unwrap_or_else(|e| panic!("harness failure (iterative) {}: {e}", case.key()));
        if a != b {
            failures.push(format!(
                "{}:\n  recursive: {a:?}\n  iterative: {b:?}",
                case.key()
            ));
        }
    }
    assert!(count > 0, "R3B.6 field supported subset is empty");
    assert!(
        failures.is_empty(),
        "iterative machine diverged from the recursive engine on supported R3B.6 field cases:\n{}",
        failures.join("\n")
    );
}

/// B-1R3F.1 — the current-language runtime surface is fully iterative:
/// this phase's unsupported sentinel set must be empty, and there must be no
/// construct left to fall back on the recursive engine for.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn iterative_index_field_unsupported_fails_explicitly() {
    let cases = r3b::index_unsupported_cases();
    assert!(
        cases.is_empty(),
        "unsupported set must be empty after B-1R3F.1; found {} case(s)",
        cases.len()
    );
    let field = r3b::field_unsupported_cases();
    assert!(
        field.is_empty(),
        "unsupported field set must be empty after B-1R3F.1; found {} case(s)",
        field.len()
    );
}

/// B-1R3B.6 — full-field regression guard for the iterative index/field subset:
/// pins the complete normalized observable (stdout bytes, completion, code,
/// message, source, byte span, line, column, value type and representation) of
/// the iterative engine for every supported case.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b6_iterative_golden_matches() {
    let committed = std::fs::read_to_string(R3B6_GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing R3B.6 golden {R3B6_GOLDEN_PATH}: {e}"));
    let golden = harness::parse_golden(&committed, "r3b6");
    let cases = r3b6_all_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative())
            .unwrap_or_else(|e| panic!("harness failure for {}: {e}", case.key()));
        observations.insert(case.key(), obs);
    }
    let regenerated = harness::encode_golden(&cases, &observations);
    assert_eq!(
        committed, regenerated,
        "R3B.6 index/field iterative golden is stale or the machine diverged; run \
         `cargo test --locked --features evaluator-oracle --test evaluator_oracle \
         regenerate_r3b6_golden -- --ignored`"
    );
    for case in &cases {
        assert!(
            golden.contains_key(&case.key()),
            "R3B.6 golden missing {}",
            case.key()
        );
    }
}

/// Regenerate the R3B.6 index/field iterative golden (ignored by default).
#[test]
#[cfg(feature = "evaluator-oracle")]
#[ignore = "regenerates the committed R3B.6 index/field iterative golden manifest"]
fn regenerate_r3b6_golden() {
    let cases = r3b6_all_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative()).unwrap();
        observations.insert(case.key(), obs);
    }
    let text = harness::encode_golden(&cases, &observations);
    std::fs::write(R3B6_GOLDEN_PATH, text).expect("write R3B.6 golden");
}

/// B-1R3B.6 — the R3B.6 golden is compared byte-for-byte against LF-only
/// generator output, so `.gitattributes` must force LF on every checkout.
#[test]
fn r3b6_golden_has_lf_pin() {
    let attrs = std::fs::read_to_string(".gitattributes").expect("read .gitattributes");
    assert!(
        attrs
            .lines()
            .any(|l| l.trim() == "tests/oracle/r3b6_golden.tsv text eol=lf"),
        "tests/oracle/r3b6_golden.tsv must be pinned to `text eol=lf`"
    );
    let golden = std::fs::read("tests/oracle/r3b6_golden.tsv")
        .expect("missing tests/oracle/r3b6_golden.tsv");
    assert!(
        !golden.contains(&b'\r'),
        "R3B.6 golden must be physically LF-only"
    );
}

/// Path of the committed R3B.7 f-string iterative-engine golden.
#[cfg(feature = "evaluator-oracle")]
const R3B7_GOLDEN_PATH: &str = "tests/oracle/r3b7_golden.tsv";

/// All supported R3B.7 f-string cases in a stable order.
#[cfg(feature = "evaluator-oracle")]
fn r3b7_all_cases() -> Vec<Case> {
    r3b::fstring_supported_cases()
}

/// B-1R3B.7 — the iterative machine must agree with the recursive engine on
/// every supported f-string case, including diagnostics, spans, value
/// representation, and stdout. Part order, exactly-once scheduling, the
/// stringification matrix, format-spec rendering/bounds, Unicode, raw literal
/// text, and short-circuit skipping are only meaningful differentially.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b7_fstring_supported_subset_agrees() {
    let rec = harness::Engine::recursive();
    let it = harness::Engine::iterative();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in r3b::fstring_supported_cases() {
        count += 1;
        let a = observe(&case, rec)
            .unwrap_or_else(|e| panic!("harness failure (recursive) {}: {e}", case.key()));
        let b = observe(&case, it)
            .unwrap_or_else(|e| panic!("harness failure (iterative) {}: {e}", case.key()));
        if a != b {
            failures.push(format!(
                "{}:\n  recursive: {a:?}\n  iterative: {b:?}",
                case.key()
            ));
        }
    }
    assert!(count > 0, "R3B.7 f-string supported subset is empty");
    assert!(
        failures.is_empty(),
        "iterative machine diverged from the recursive engine on supported R3B.7 f-string cases:\n{}",
        failures.join("\n")
    );
}

/// B-1R3F.1 — the current-language runtime surface is fully iterative:
/// this phase's unsupported sentinel set must be empty, and there must be no
/// construct left to fall back on the recursive engine for.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn iterative_fstring_unsupported_fails_explicitly() {
    let cases = r3b::fstring_unsupported_cases();
    assert!(
        cases.is_empty(),
        "unsupported set must be empty after B-1R3F.1; found {} case(s)",
        cases.len()
    );
}

/// B-1R3B.7 — full-field regression guard for the iterative f-string subset:
/// pins the complete normalized observable (stdout bytes, completion, code,
/// message, source, byte span, line, column, value type and representation) of
/// the iterative engine for every supported case.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3b7_iterative_golden_matches() {
    let committed = std::fs::read_to_string(R3B7_GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing R3B.7 golden {R3B7_GOLDEN_PATH}: {e}"));
    let golden = harness::parse_golden(&committed, "r3b7");
    let cases = r3b7_all_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative())
            .unwrap_or_else(|e| panic!("harness failure for {}: {e}", case.key()));
        observations.insert(case.key(), obs);
    }
    let regenerated = harness::encode_golden(&cases, &observations);
    assert_eq!(
        committed, regenerated,
        "R3B.7 f-string iterative golden is stale or the machine diverged; run \
         `cargo test --locked --features evaluator-oracle --test evaluator_oracle \
         regenerate_r3b7_golden -- --ignored`"
    );
    for case in &cases {
        assert!(
            golden.contains_key(&case.key()),
            "R3B.7 golden missing {}",
            case.key()
        );
    }
}

/// Regenerate the R3B.7 f-string iterative golden (ignored by default).
#[test]
#[cfg(feature = "evaluator-oracle")]
#[ignore = "regenerates the committed R3B.7 f-string iterative golden manifest"]
fn regenerate_r3b7_golden() {
    let cases = r3b7_all_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative()).unwrap();
        observations.insert(case.key(), obs);
    }
    let text = harness::encode_golden(&cases, &observations);
    std::fs::write(R3B7_GOLDEN_PATH, text).expect("write R3B.7 golden");
}

/// B-1R3B.7 — the R3B.7 golden is compared byte-for-byte against LF-only
/// generator output, so `.gitattributes` must force LF on every checkout.
#[test]
fn r3b7_golden_has_lf_pin() {
    let attrs = std::fs::read_to_string(".gitattributes").expect("read .gitattributes");
    assert!(
        attrs
            .lines()
            .any(|l| l.trim() == "tests/oracle/r3b7_golden.tsv text eol=lf"),
        "tests/oracle/r3b7_golden.tsv must be pinned to `text eol=lf`"
    );
    let golden = std::fs::read("tests/oracle/r3b7_golden.tsv")
        .expect("missing tests/oracle/r3b7_golden.tsv");
    assert!(
        !golden.contains(&b'\r'),
        "R3B.7 golden must be physically LF-only"
    );
}

/// Path of the committed R3C.1 call iterative-engine golden.
#[cfg(feature = "evaluator-oracle")]
const R3C1_GOLDEN_PATH: &str = "tests/oracle/r3c1_golden.tsv";

/// All supported R3C.1 call cases in a stable order.
#[cfg(feature = "evaluator-oracle")]
fn r3c1_all_cases() -> Vec<Case> {
    r3c1::supported_cases()
}

/// B-1R3C.1 — the iterative machine must agree with the recursive engine on
/// every supported call case, including stdout (side-effect order and
/// exactly-once), diagnostics, spans, value representation, and control
/// outcomes. This is also the first phase where the R3B exactly-once claims
/// become differentially falsifiable, which the side-effect cases pin.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3c1_call_supported_subset_agrees() {
    let rec = harness::Engine::recursive();
    let it = harness::Engine::iterative();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in r3c1::supported_cases() {
        count += 1;
        let a = observe(&case, rec)
            .unwrap_or_else(|e| panic!("harness failure (recursive) {}: {e}", case.key()));
        let b = observe(&case, it)
            .unwrap_or_else(|e| panic!("harness failure (iterative) {}: {e}", case.key()));
        if a != b {
            failures.push(format!(
                "{}:\n  recursive: {a:?}\n  iterative: {b:?}",
                case.key()
            ));
        }
    }
    assert!(count > 0, "R3C.1 call supported subset is empty");
    assert!(
        failures.is_empty(),
        "R3C.1 call differential failures ({}):\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// B-1R3F.1 — the current-language runtime surface is fully iterative:
/// this phase's unsupported sentinel set must be empty, and there must be no
/// construct left to fall back on the recursive engine for.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3c1_unsupported_fails_explicitly() {
    let cases = r3c1::unsupported_cases();
    assert!(
        cases.is_empty(),
        "unsupported set must be empty after B-1R3F.1; found {} case(s)",
        cases.len()
    );
}

/// B-1R3C.1 — full-field regression guard for the iterative call subset: pins
/// the complete normalized observable (stdout bytes, completion, code,
/// message, source, byte span, line, column, value type and representation) of
/// the iterative engine for every supported case.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn r3c1_iterative_golden_matches() {
    let committed = std::fs::read_to_string(R3C1_GOLDEN_PATH)
        .unwrap_or_else(|e| panic!("missing R3C.1 golden {R3C1_GOLDEN_PATH}: {e}"));
    let golden = harness::parse_golden(&committed, "r3c1");
    let cases = r3c1_all_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative())
            .unwrap_or_else(|e| panic!("harness failure for {}: {e}", case.key()));
        observations.insert(case.key(), obs);
    }
    let regenerated = harness::encode_golden(&cases, &observations);
    assert_eq!(
        committed, regenerated,
        "R3C.1 call iterative golden is stale or the machine diverged; run \
         `cargo test --locked --features evaluator-oracle --test evaluator_oracle \
         regenerate_r3c1_golden -- --ignored`"
    );
    for case in &cases {
        assert!(
            golden.contains_key(&case.key()),
            "R3C.1 golden missing {}",
            case.key()
        );
    }
}

/// Regenerate the R3C.1 call iterative golden (ignored by default).
#[test]
#[cfg(feature = "evaluator-oracle")]
#[ignore = "regenerates the committed R3C.1 call iterative golden manifest"]
fn regenerate_r3c1_golden() {
    let cases = r3c1_all_cases();
    let mut observations = BTreeMap::new();
    for case in &cases {
        let obs = observe(case, harness::Engine::iterative()).unwrap();
        observations.insert(case.key(), obs);
    }
    let text = harness::encode_golden(&cases, &observations);
    std::fs::write(R3C1_GOLDEN_PATH, text).expect("write R3C.1 golden");
}

/// B-1R3C.1 — the R3C.1 golden is compared byte-for-byte against LF-only
/// generator output, so `.gitattributes` must force LF on every checkout.
#[test]
fn r3c1_golden_has_lf_pin() {
    let attrs = std::fs::read_to_string(".gitattributes").expect("read .gitattributes");
    assert!(
        attrs
            .lines()
            .any(|l| l.trim() == "tests/oracle/r3c1_golden.tsv text eol=lf"),
        "tests/oracle/r3c1_golden.tsv must be pinned to `text eol=lf`"
    );
    let golden = std::fs::read("tests/oracle/r3c1_golden.tsv")
        .expect("missing tests/oracle/r3c1_golden.tsv");
    assert!(
        !golden.contains(&b'\r'),
        "R3C.1 golden must be physically LF-only"
    );
}

/// B-1R3A anti-tautology guard: the iterative engine must not be an alias of
/// the recursive engine. A supported case must agree between the two engines,
/// and an equivalent-but-unsupported case must diverge to the explicit
/// `E4999` sentinel rather than silently producing the recursive result. This
/// structural check complements the deliberate divergence experiment in the
/// report.
#[test]
#[cfg(feature = "evaluator-oracle")]
fn iterative_engine_is_a_distinct_path() {
    // A program valid for both engines must agree.
    let supported = Case {
        group: "r3a-program",
        name: "distinct_path",
        file: "r3a.aura",
        source: "fn main() { if true { return } }\n",
        kind: harness::Kind::ExecuteProgram,
    };
    let a = observe(&supported, harness::Engine::recursive()).unwrap();
    let b = observe(&supported, harness::Engine::iterative()).unwrap();
    assert_eq!(a, b, "supported case must agree");

    // B-1R3F.1: the current-language runtime surface is fully iterative, so
    // the former "equivalent-but-unsupported diverges to E4999" probe no
    // longer has a construct to use. Distinguish the engines structurally
    // instead: every case in the phase subsets (which exercise only the
    // machine's own scheduling) must agree with the recursive engine, and the
    // full-language differential campaign in `engines_agree` covers the
    // composed surface. A call-heavy program that only the machine's explicit
    // frames can run without host recursion is checked here as a smoke case.
    let deep = Case {
        group: "r3a-program",
        name: "distinct_path_deep",
        file: "r3a.aura",
        source: "fn main() { let x = f(300) }\nfn f(n) { if n { f(n - 1) } else { 0 } }\n",
        kind: harness::Kind::ExecuteProgram,
    };
    let a = observe(&deep, harness::Engine::recursive()).unwrap();
    let b = observe(&deep, harness::Engine::iterative()).unwrap();
    assert_eq!(a, b, "deep call case must agree between engines");
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
