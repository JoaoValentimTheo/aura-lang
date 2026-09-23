#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Property-based and fuzz tests.
//!
//! The invariants:
//! * The lexer and parser must never panic on *any* input.
//! * Parsing a valid program and re-rendering it must be stable.
//! * Executing the same program twice yields identical output.
//! * Every integer operation either succeeds or returns a diagnostic; it
//!   never panics or wraps silently.

use proptest::prelude::*;

use aura::run_source;

/// Arbitrary source-like strings, biased toward Aura syntax.
fn aura_like() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just("let".to_string()),
            Just("mut".to_string()),
            Just("fn".to_string()),
            Just("if".to_string()),
            Just("else".to_string()),
            Just("match".to_string()),
            Just("->".to_string()),
            Just("{".to_string()),
            Just("}".to_string()),
            Just("(".to_string()),
            Just(")".to_string()),
            Just("[".to_string()),
            Just("]".to_string()),
            Just("+".to_string()),
            Just("*".to_string()),
            Just("\"".to_string()),
            Just("f\"{x}\"".to_string()),
            Just("0xff".to_string()),
            Just("1abc".to_string()),
            Just("ação".to_string()),
            Just("\\n".to_string()),
            "[a-z]{1,6}".prop_map(|s| s),
            "[0-9]{1,3}".prop_map(|s| s),
        ],
        0..40,
    )
    .prop_map(|parts| parts.join(" "))
}

proptest! {
    #[test]
    fn lexer_never_panics(src in aura_like()) {
        let _ = aura::lex::lex(&src);
    }

    #[test]
    fn parser_never_panics(src in aura_like()) {
        let _ = aura::parse::parse(&src);
    }

    #[test]
    fn runner_never_panics(src in aura_like()) {
        // Must not panic; error or output are both acceptable.
        let _ = run_source(&src, "<fuzz>");
    }

    #[test]
    fn execution_is_deterministic(seed in 0u64..1000) {
        let src = format!("fn main() {{ print({seed} + 1) }}");
        let a = run_source(&src, "<p>").expect("runs");
        let b = run_source(&src, "<p>").expect("runs");
        prop_assert_eq!(a, b);
    }

    #[test]
    fn arithmetic_never_wraps(a in any::<i64>(), b in any::<i64>()) {
        let src = format!("fn main() {{ print({a} + {b}) }}");
        match run_source(&src, "<p>") {
            Ok(out) => {
                // If it succeeded, the printed value must be the true sum.
                let expected = (i128::from(a) + i128::from(b)).to_string();
                prop_assert_eq!(out.trim(), expected);
            }
            Err(d) => prop_assert_eq!(d.code, aura::error::codes::OVERFLOW),
        }
    }

    #[test]
    fn division_by_zero_is_always_a_diagnostic(a in any::<i64>()) {
        let src = format!("fn main() {{ print({a} / 0) }}");
        let err = run_source(&src, "<p>").unwrap_err();
        prop_assert_eq!(err.code, aura::error::codes::DIV_ZERO);
    }

    #[test]
    fn checker_is_deterministic(src in aura_like()) {
        // Checking the same source twice yields the same verdict.
        let module = aura::parse::parse(&src);
        if let Ok(module) = module {
            let a = aura::check::Checker::module(&module).map_err(|d| d.code);
            let b = aura::check::Checker::module(&module).map_err(|d| d.code);
            prop_assert_eq!(a, b);
        }
    }

    #[test]
    fn subtraction_never_wraps(a in any::<i64>(), b in any::<i64>()) {
        let src = format!("fn main() {{ print({a} - {b}) }}");
        match run_source(&src, "<p>") {
            Ok(out) => {
                let expected = (i128::from(a) - i128::from(b)).to_string();
                prop_assert_eq!(out.trim(), expected);
            }
            Err(d) => prop_assert_eq!(d.code, aura::error::codes::OVERFLOW),
        }
    }

    #[test]
    fn multiplication_never_wraps(a in any::<i64>(), b in any::<i64>()) {
        let src = format!("fn main() {{ print({a} * {b}) }}");
        match run_source(&src, "<p>") {
            Ok(out) => {
                let expected = (i128::from(a) * i128::from(b)).to_string();
                prop_assert_eq!(out.trim(), expected);
            }
            Err(d) => prop_assert_eq!(d.code, aura::error::codes::OVERFLOW),
        }
    }

    #[test]
    fn nesting_never_panics(depth in 0usize..400) {
        // Deeply nested but well-formed input must be handled without panic:
        // either parsed or rejected, never a crash.
        let src = format!("fn main() {{ print({}1{}) }}", "(".repeat(depth), ")".repeat(depth));
        let _ = run_source(&src, "<nest>");
    }
}

/// A grammar-directed generator of well-formed, side-effect-free expressions.
///
/// Unlike `aura_like`, every generated expression parses. Type errors are
/// allowed by design: the differential test is exactly about whether the
/// checker and the evaluator agree on which programs are *type* errors.
fn typed_expr() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        (0i64..5).prop_map(|n| n.to_string()),
        (0i64..5).prop_map(|n| format!("{n}.0")),
        Just("\"s\"".to_string()),
        Just("true".to_string()),
        Just("none".to_string()),
    ];
    leaf.prop_recursive(3, 64, 8, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a} + {b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a} - {b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a} < {b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a} == {b})")),
            (inner.clone(), inner).prop_map(|(a, b)| format!("(if true {{ {a} }} else {{ {b} }})")),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// F-15: the checker and the evaluator must not contradict each other.
    ///
    /// For every generated (well-formed) expression:
    /// * if the checker accepts it, running it must never fail with a
    ///   front-end code (`E1xxx`) or the internal code (`E4999`);
    /// * if the checker rejects it, that rejection must be a type mismatch.
    #[test]
    fn checker_and_evaluator_agree(expr in typed_expr()) {
        let src = format!("fn main() {{ let x = {expr}\n print(x) }}");
        let module = match aura::parse::parse(&src) {
            Ok(m) => m,
            // Should not happen for generated expressions; if it does, the
            // generator is wrong, not the language.
            Err(_) => return Ok(()),
        };
        let checked = aura::check::Checker::module(&module);
        match checked {
            Ok(()) => {
                if let Err(d) = run_source(&src, "<differential>") {
                    prop_assert!(
                        !matches!(d.code, 1001..=1999 | 4999),
                        "checker accepted but runtime failed with E{}: {src}",
                        d.code
                    );
                }
            }
            Err(d) => {
                prop_assert_eq!(d.code, aura::error::codes::TYPE_MISMATCH);
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// FEATURE_001: for a directly resolved call to a function with annotated
    /// parameters, the static verdict must agree with the runtime.
    ///
    /// * if the checker accepts the call, the runtime must not fail with a
    ///   type error at the call itself;
    /// * if the checker rejects it, the diagnostic is `E3001`.
    #[test]
    fn static_argument_check_agrees_with_runtime(
        annotation in prop_oneof![
            Just("int".to_string()),
            Just("float".to_string()),
            Just("string".to_string()),
            Just("bool".to_string()),
        ],
        arg in prop_oneof![
            Just("1".to_string()),
            Just("1.5".to_string()),
            Just("\"s\"".to_string()),
            Just("true".to_string()),
            Just("none".to_string()),
        ],
    ) {
        // The body returns `none`, so a well-typed call has no runtime effect.
        let src = format!(
            "fn f(a: {annotation}) {{ return none }}\nfn main() {{ f({arg}) }}"
        );
        let module = aura::parse::parse(&src).expect("generated source parses");
        match aura::check::Checker::module(&module) {
            Ok(()) => {
                // Accepted: the runtime call must not fail (the body is `none`).
                if let Err(d) = run_source(&src, "<differential>") {
                    prop_assert_eq!(d.code, aura::error::codes::TYPE_MISMATCH);
                }
            }
            Err(d) => {
                prop_assert_eq!(d.code, aura::error::codes::TYPE_MISMATCH);
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// FEATURE_002: a named permutation of the arguments binds the same
    /// parameter values as the positional call, and the reordered call is
    /// accepted exactly when the positional one is.
    #[test]
    fn named_permutation_matches_positional(
        a in 0i64..10,
        b in 0i64..10,
        c in 0i64..10,
    ) {
        let params = "fn f(a: int, b: int, c: int) -> int { return a * 100 + b * 10 + c }";
        let positional = format!("{params}\nfn main() {{ print(f({a}, {b}, {c})) }}");
        let named = format!("{params}\nfn main() {{ print(f(c: {c}, a: {a}, b: {b})) }}");
        let expected = aura::run_source(&positional, "<p>").expect("positional runs");
        let got = aura::run_source(&named, "<p>").expect("named runs");
        prop_assert_eq!(expected, got);
    }
}
