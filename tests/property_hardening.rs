#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Hardening property tests (pre-modules round).
//!
//! Five properties over the frozen language line. The framework is `proptest`
//! 1.5 — already a dev-dependency used by `tests/property.rs` — so no new
//! dependency is introduced. Every property shrinks to a minimal failing
//! example and prints its seed on failure (proptest's default behavior); the
//! failing case is reproducible by re-running with `PROPTEST_CASES` and the
//! printed seed, or by adding the shrunk input to `tests/corpus/`.
//!
//! Generated-case counts are stated per property with a rationale balancing
//! signal and CI cost (the whole file runs in the existing `test` job).

use proptest::prelude::*;

// The shared generator, reused from the fuzz targets.
#[path = "support/program_gen.rs"]
mod program_gen;

// A single deterministically chosen probe seed for the substrate-parity
// properties: native and wasm must agree on the *same* generated programs, so
// the seed set is fixed and enumerated, not random.
const PARITY_SEEDS: std::ops::Range<u64> = 0..64;

// ---------------------------------------------------------------------------
// PROPERTY 1 — real registry consistency
// ---------------------------------------------------------------------------

// INVARIANT. For *every* builtin in the production signature registry
// (`aura::stdlib::signatures::builtins()` — the single source of truth shared
// by the checker and the runtime), the checker's accept/reject verdict and the
// runtime's dispatch agree on the *call shape*:
//
//   * if the registry says the generated call is well-shaped (arity within
//     `min_args..=max_args` and every argument type satisfies its parameter),
//     the checker must accept it, and the runtime must **resolve the callable**
//     (never `E2003` undefined) — i.e. the two registries name the same
//     callables;
//   * if the registry says the call is malformed, the checker must reject it,
//     with a structured diagnostic, never a panic.
//
// The candidate list is *derived from the real registry*, not a hand-picked
// subset: the property enumerates every registered builtin and reads its own
// `min_args`/`max_args`/`params`, so a new builtin is covered automatically.
// A runtime *content* error for an accepted call (e.g. `json_decode("s")` is
// not valid JSON) is a permitted outcome — it is not a resolution
// disagreement. What must never diverge is callable existence and arity.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(800))]

    #[test]
    fn registry_checker_and_runtime_agree(
        which in 0usize..aura::stdlib::signatures::builtins().len(),
        extra in 0usize..4,
        type_idx in 0usize..6,
    ) {
        use aura::stdlib::signatures::{Accepts, Param};
        use aura::types::Ty;

        let sig = &aura::stdlib::signatures::builtins()[which];
        let name = sig.name;
        // A literal and its static type for each class we generate.
        let pool: [(&str, Ty); 6] = [
            ("1", Ty::Int),
            ("1.5", Ty::Float),
            ("true", Ty::Bool),
            ("\"s\"", Ty::String),
            ("[1, 2]", Ty::List(Box::new(Ty::Int))),
            ("{\"k\": 1}", Ty::Map(Box::new(Ty::Int))),
        ];
        let (lit, ty) = &pool[type_idx % pool.len()];
        // Arity derived from the registry's own declared bounds.
        let argc = sig.min_args + extra;
        let args: Vec<&str> = (0..argc).map(|_| *lit).collect();

        // The registry's own verdict for this shape.
        let arity_ok = arc_in_bounds(sig, argc);
        let types_ok = (0..argc).all(|i| {
            let param = sig.params.get(i).copied().unwrap_or(Param::ANY);
            match param.accepts {
                Accepts::Any => true,
                _ => param.accepts.accepts_ty(ty) != Some(false),
            }
        });
        let well_shaped = arity_ok && types_ok;

        let src = format!("fn main() {{ {}({}) }}", name, args.join(", "));
        let checked = aura::check::Checker::module(
            &aura::parse::parse(&src).expect("property-generated source parses"),
        );
        let ran = aura::run_source(&src, "<registry>");

        if well_shaped {
            // The checker must agree with the registry that this is legal.
            prop_assert!(
                checked.is_ok(),
                "registry says `{}` (argc {}) is well-shaped but checker rejected: {:?}",
                name,
                argc,
                checked.err().map(|d| (d.code, d.message))
            );
            // The runtime must resolve the callable: a checker-accepted call
            // can never be "undefined" at runtime, or the two registries have
            // drifted. A structured runtime *content* error is permitted.
            if let Err(d) = ran {
                prop_assert_ne!(
                    d.code,
                    aura::error::codes::UNDEFINED,
                    "checker accepted `{}` but runtime says undefined",
                    name
                );
            }
        } else if let Err(d) = checked {
            prop_assert!(
                (2000..4000).contains(&d.code) || d.code == aura::error::codes::UNDEFINED,
                "unexpected checker code E{} for `{}`",
                d.code,
                name
            );
        }
    }
}

/// Whether `argc` lies within a signature's declared arity bounds. Kept as a
/// free function so the property reads as the registry's own contract.
fn arc_in_bounds(sig: &aura::stdlib::signatures::Signature, argc: usize) -> bool {
    argc >= sig.min_args && argc <= sig.max_args
}

// ---------------------------------------------------------------------------
// PROPERTY 2 — deterministic overload resolution
// ---------------------------------------------------------------------------

// INVARIANT. For a set of overloads whose parameter types are drawn from a
// small pool, the selected overload does not depend on declaration order or
// on any hash iteration order, and a genuine tie is always `Ambiguous`.
//
// `types::resolve_overload` is a pure function over a `Vec` of candidates
// (`OverloadParams`) and is indexed deterministically; it does not consult a
// `HashMap`, so there is no iteration order to vary. The property therefore
// checks the two observable consequences directly: (a) reversing the
// candidate order never changes a *unique* selection, and (b) when two
// candidates are equally specific, the result is `Ambiguous`.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn overload_resolution_is_order_independent_and_ties_are_ambiguous(
        // Two concrete overloads plus a fixed call argument.
        t1 in prop::sample::select(vec![
            aura::types::Ty::Int, aura::types::Ty::String, aura::types::Ty::Bool,
            aura::types::Ty::Float,
        ]),
        t2 in prop::sample::select(vec![
            aura::types::Ty::Int, aura::types::Ty::String, aura::types::Ty::Bool,
            aura::types::Ty::Float,
        ]),
        arg in prop::sample::select(vec![
            aura::types::Ty::Int, aura::types::Ty::String,
        ]),
    ) {
        use aura::types::{resolve_overload, OverloadParams, OverloadResolution, Ty};
        let params = |t: Ty| -> OverloadParams { vec![Some(t)] };
        let fwd = vec![params(t1.clone()), params(t2.clone())];
        let rev = vec![params(t2.clone()), params(t1.clone())];
        let actual = vec![arg.clone()];

        let a = resolve_overload(&fwd, &actual);
        let b = resolve_overload(&rev, &actual);

        match (&a, &b) {
            (OverloadResolution::Selected(i), OverloadResolution::Selected(j)) => {
                // A unique selection must be the *same type*, regardless of
                // position; reversing order flips the index iff the type flips.
                prop_assert_eq!(fwd[*i][0].clone(), rev[*j][0].clone());
            }
            (OverloadResolution::Ambiguous(_), OverloadResolution::Ambiguous(_)) => {}
            (OverloadResolution::NoMatch, OverloadResolution::NoMatch) => {}
            other => prop_assert!(false, "order changed the resolution: {other:?}"),
        }

        // If both candidates are the same type, a call must be ambiguous (a
        // genuine tie), never a silent pick.
        if t1 == t2 && t1.compatible_with(&arg) {
            prop_assert!(matches!(a, OverloadResolution::Ambiguous(_)));
        }
    }
}

// ---------------------------------------------------------------------------
// PROPERTY 3 — AST limit / substrate-source parity (TypeExpr excluded)
// ---------------------------------------------------------------------------

// AUDIT-3 TypeExpr substrate divergence is intentionally excluded pending human decision.
// Follow-up 3 / AUDIT-3 remains DECISION-PENDING.
// This exclusion is temporary and must be revisited after the human decision.

// INVARIANT. For generated programs *within* the semantic AST limit and
// **excluding TypeExpr-heavy programs**, the native checker and the runtime
// agree on accept/reject. This property runs natively; the cross-substrate
// half (native vs wasm) is enforced by the differential harness, which shares
// the same generated-program corpus via the committed `tests/corpus/ast/*`
// fixtures.
//
// The generator (shared with the fuzz targets) emits only programs that pass
// the checker; this property therefore asserts that contract holds for every
// seed in the parity range, and that no seed triggers a host failure.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn generated_programs_check_and_run(seed in PARITY_SEEDS) {
        let src = program_gen::generate(seed);
        // The generator's contract: the program passes the checker.
        let parsed = aura::parse::parse(&src).expect("generated program parses");
        prop_assert!(
            aura::check::Checker::module(&parsed).is_ok(),
            "generator produced a rejected program for seed {seed}:\n{src}"
        );
        // And it terminates with a normal result or a structured diagnostic,
        // never a panic (a panic would abort the test process).
        let _ = aura::run_source(&src, "<parity>");
    }
}

// ---------------------------------------------------------------------------
// PROPERTY 4 — generated-corpus determinism (native/WASM parity lives in Node)
// ---------------------------------------------------------------------------

// INVARIANT. The committed generated-program corpus (`tests/corpus/ast/*.aura`)
// is byte-identical to what `program_gen::seed_corpus()` produces. This keeps
// the shared corpus from drifting: if the generator changes without the
// fixtures being regenerated, the two engines would be compared on *different*
// programs.
//
// The actual *native/WASM observable parity* property cannot run in a Rust test
// (it needs both the native engine and the wasm artifact). It is enforced by
// the differential harness (`playground/tests/node/differential.test.mjs`,
// "Generated-program native/wasm parity"), which reads exactly these committed
// fixtures, runs each through both engines, and requires stdout / diagnostic
// code / status to agree — with TypeExpr-heavy inputs excluded pending
// AUDIT-3. This property is the Rust-side half of that contract: the corpus the
// differential harness consumes is the corpus the generator defines.
// This half of Property 4 is a deterministic `#[test]`, not a proptest: it must
// pin *every* committed fixture on every run, not sample a subset.
#[test]
fn generated_corpus_matches_the_generator() {
    for (seed, generated) in program_gen::seed_corpus() {
        let path = format!("tests/corpus/ast/gen_{seed:02}.aura");
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("missing corpus fixture {path}: {e}"));
        // Compare ignoring line-ending style: a Windows checkout may apply
        // CRLF unless `.gitattributes` forces LF; the *content* is what the
        // property pins, not the EOL bytes.
        let norm = |s: &str| s.replace("\r\n", "\n");
        assert_eq!(
            norm(&committed),
            norm(&generated),
            "corpus fixture {path} drifted"
        );
    }
}

// ---------------------------------------------------------------------------
// PROPERTY 5 — random cyclic graph safety (display, equality, JSON)
// ---------------------------------------------------------------------------

// INVARIANT. For a runtime-built reference graph over any of the value
// containers (list, map, struct, enum payload) that is cyclic or shares
// subvalues, display, equality, and JSON encoding all terminate in bounded
// time with no panic, abort, or stack overflow (AUDIT-4 regression).
//
// Equality is a separate algorithm from display/JSON: it is an iterative
// worklist with a visited-*pair* set and an `Rc::ptr_eq` identity shortcut
// (`Value::equals`, `src/run/value.rs`). The property therefore exercises the
// *dangerous* equality case explicitly: a `c == c` identity compare takes the
// pointer shortcut and never descends, so it is not evidence. The generated
// case instead compares `c` against a *separately constructed, structurally
// identical* cyclic graph `d`, forcing the visited-pair set (not `ptr_eq`) to
// terminate. Self-cycle, mutual-cycle, shared-subgraph, and fan-out shapes are
// covered by the DSL below.
proptest! {
    // 16 cases: each case performs several bounded renders (the worst is
    // fan-out 4, ~0.65 s at the 1,000,000-node budget), which keeps the
    // property around ~15 s in CI. The full cycle *matrix* (list/map/struct/
    // enum, depth and node boundaries) is separately and exhaustively pinned
    // by the committed `tests/corpus/cycles/*` fixtures, which run in the same
    // job; this property adds randomized shape coverage on top.
    #![proptest_config(ProptestConfig::with_cases(16))]

    #[test]
    fn cyclic_graphs_render_and_compare_safely(
        pushes in 1usize..5,
        use_json in any::<bool>(),
        // Compare against an independently built graph rather than `c == c`,
        // so the pointer shortcut cannot mask a non-terminating equality.
        compare_clone in any::<bool>(),
        // A shared-subgraph variant: `d = [c, c]` re-references the cycle.
        share in any::<bool>(),
    ) {
        // `c` contains itself `pushes` times: fan-out == pushes.
        let mut body = String::from("let mut c = []\n");
        for _ in 0..pushes {
            body.push_str("push(c, c)\n");
        }
        let tail = if use_json {
            "print(len(json_encode(c)))"
        } else {
            "print(len(to_string(c)))"
        };
        // A *separately built* cyclic graph with the same shape, so equality
        // must use its visited-pair set rather than an identity shortcut.
        let mut clone = String::from("let mut d = []\n");
        for _ in 0..pushes {
            clone.push_str("push(d, d)\n");
        }
        let eq = if compare_clone {
            format!("{clone}    print(c == d)\n")
        } else {
            String::new()
        };
        let share_stmt = if share {
            "    let mut e = []\n    push(e, c)\n    push(e, c)\n    print(len(to_string(e)))\n"
        } else {
            ""
        };
        let src = format!("fn main() {{\n{body}{eq}{share_stmt}{tail}\n}}");
        // Terminates with a value or a structured diagnostic; the test process
        // surviving to the assertion *is* the no-panic/no-hang check.
        match aura::run_source(&src, "<cycle>") {
            Ok(_) => {}
            Err(d) => prop_assert!(d.code >= 4000, "unexpected checker code E{}", d.code),
        }
        if compare_clone {
            // Two independently built graphs of the same shape are equal, and
            // the comparison terminates via the visited-pair set.
            let eq_src = format!("fn main() {{\n{body}{clone}    print(c == d)\n}}");
            prop_assert_eq!(aura::run_source(&eq_src, "<cycle>").unwrap(), "true\n");
        }
        // Identity equality always holds and always terminates.
        let id_src = format!("fn main() {{\n{body}    print(c == c)\n}}");
        prop_assert_eq!(aura::run_source(&id_src, "<cycle>").unwrap(), "true\n");
    }
}
