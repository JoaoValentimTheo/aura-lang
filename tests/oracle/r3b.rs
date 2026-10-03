#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! B-1R3B.1 — the iterative-engine oracle subset for unary operators.
//!
//! These cases exercise **only** the unary constructs the explicit-continuation
//! machine now supports (`-`, `not`, `~`) layered on the existing R3A subset.
//! Every case is run through the real `Engine::iterative` path by the
//! differential test in `tests/evaluator_oracle.rs`; none may fall back to
//! recursion.
//!
//! The unsupported set is asserted independently: an operator whose operand is
//! itself an unsupported construct must fail with the deterministic `E4999`
//! sentinel *without* the operator being applied — the continuation-safety
//! anti-fallback guard.

use crate::harness::{Case, Kind};

fn value(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b-value",
        name,
        file: "<r3b>",
        source,
        kind: Kind::Value,
    }
}

fn program(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b-program",
        name,
        file: "r3b.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

/// Unary constructs the iterative engine supports. Recursive and iterative must
/// agree exactly, including diagnostics, spans, and value types.
#[must_use]
pub fn supported_cases() -> Vec<Case> {
    vec![
        // ----- unary minus: int ------------------------------------------
        value("neg_int", "-5\n"),
        value("neg_int_zero", "-0\n"),
        value("neg_int_min", "-9223372036854775808\n"),
        // ----- unary minus: float ----------------------------------------
        value("neg_float", "-1.5\n"),
        value("neg_float_zero", "-0.0\n"),
        // ----- unary minus: operand forms --------------------------------
        value("neg_nested", "- -5\n"),
        value("neg_nested_float", "- -1.5\n"),
        // i64::MIN is a literal; negating it overflows (E4013), exactly as the
        // recursive `checked_neg` path does.
        value("neg_min_overflow", "-(-9223372036854775808)\n"),
        // ----- unary not: truthiness -------------------------------------
        value("not_true", "not true\n"),
        value("not_false", "not false\n"),
        value("not_int_zero", "not 0\n"),
        value("not_int_nonzero", "not 7\n"),
        value("not_float_zero", "not 0.0\n"),
        value("not_float_nonzero", "not 2.5\n"),
        value("not_str_empty", "not \"\"\n"),
        value("not_str_nonempty", "not \"x\"\n"),
        value("not_none", "not none\n"),
        value("not_nested", "not not true\n"),
        // A function/native value is truthy, so `not` yields false.
        value("not_function_value", "not len\n"),
        value("not_function_name", "fn f() {}\nnot f\n"),
        // ----- bitwise not: int ------------------------------------------
        value("bitnot_zero", "~0\n"),
        value("bitnot_positive", "~5\n"),
        value("bitnot_negative", "~-1\n"),
        value("bitnot_nested", "~~5\n"),
        // ----- composed with R3A constructs ------------------------------
        value("neg_inside_block", "{ let x = -3\n -x }\n"),
        value("not_in_if_cond", "if not false { 1 } else { 2 }\n"),
        value("neg_in_if_cond", "if -1 { 1 } else { 2 }\n"),
        value("unary_let_shadow", "{ let x = true\n let y = not x\n y }\n"),
        // ----- diagnostics reachable in the subset -----------------------
        value("neg_bool", "-true\n"),
        value("neg_str", "-\"x\"\n"),
        value("neg_none", "-none\n"),
        value("neg_function_value", "-len\n"),
        value("bitnot_float", "~1.5\n"),
        value("bitnot_str", "~\"x\"\n"),
        // ----- program (real frame boundary) -----------------------------
        program("main_neg", "fn main() { return -5 }\n"),
        program("main_not", "fn main() { return not false }\n"),
        program("main_bitnot", "fn main() { return ~0 }\n"),
        program(
            "main_unary_in_if",
            "fn main() { if not false { let x = -1\n if x { } } }\n",
        ),
    ]
}

/// Unary operators whose operand is an unsupported construct (or an unsupported
/// expression that merely *contains* the operator) must still fail with the
/// deterministic `E4999` sentinel. In particular the operator must not be
/// applied after an unsupported operand fails: the machine propagates the
/// failure and never applies the operator or recurses.
#[must_use]
pub fn unsupported_cases() -> Vec<Case> {
    vec![
        // The operand is a binary expression (R3B.2): the unary machine must
        // not apply `-`/`not` to it, and must not fall back to recursion.
        value("neg_of_unsupported_binary", "-(1 + 1)\n"),
        value("not_of_unsupported_binary", "not (1 + 1)\n"),
        // The operand is a call (R3C).
        value("neg_of_unsupported_call", "-len([1, 2])\n"),
        // A binary expression is still unsupported on its own.
        value("binary_add", "1 + 1\n"),
    ]
}
