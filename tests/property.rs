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
