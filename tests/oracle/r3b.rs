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
        // The operand is a call (R3C): the unary machine must not apply
        // `-`/`not` to it, and must not fall back to recursion.
        value("neg_of_unsupported_call", "-len([1, 2])\n"),
        value("not_of_unsupported_call", "not len([1, 2])\n"),
        // The operand is a list literal (R3B.4).
        value("neg_of_unsupported_list", "-[1, 2]\n"),
        // A list literal is still unsupported on its own.
        value("list_literal", "[1, 2]\n"),
    ]
}

fn binary_value(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b2-value",
        name,
        file: "<r3b2>",
        source,
        kind: Kind::Value,
    }
}

fn binary_program(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b2-program",
        name,
        file: "r3b2.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

/// B-1R3B.2 — eager binary operators the iterative engine supports. Recursive
/// and iterative must agree exactly on value, type, diagnostics, and span.
#[must_use]
pub fn binary_supported_cases() -> Vec<Case> {
    vec![
        // ----- arithmetic: int -------------------------------------------
        binary_value("add_int", "1 + 2\n"),
        binary_value("sub_int", "5 - 3\n"),
        binary_value("mul_int", "4 * 6\n"),
        binary_value("div_int", "20 / 6\n"),
        binary_value("rem_int", "20 % 6\n"),
        binary_value("pow_int", "2 ^ 10\n"),
        // ----- arithmetic: float and mixed -------------------------------
        binary_value("add_float", "1.5 + 2.25\n"),
        binary_value("mixed_int_float", "1 + 2.5\n"),
        binary_value("mixed_float_int", "2.5 + 1\n"),
        binary_value("div_float", "10.0 / 4.0\n"),
        binary_value("rem_float", "10.0 % 3.0\n"),
        binary_value("pow_float", "2.0 ^ 3.0\n"),
        // ----- string concatenation --------------------------------------
        binary_value("add_str", "\"a\" + \"b\"\n"),
        // ----- comparison ------------------------------------------------
        binary_value("eq_int_true", "1 == 1\n"),
        binary_value("eq_int_false", "1 == 2\n"),
        binary_value("ne_int", "1 != 2\n"),
        binary_value("lt_int", "1 < 2\n"),
        binary_value("le_equal", "2 <= 2\n"),
        binary_value("gt_int", "3 > 2\n"),
        binary_value("ge_equal", "3 >= 3\n"),
        binary_value("eq_int_float", "1 == 1.0\n"),
        binary_value("lt_mixed", "1 < 1.5\n"),
        binary_value("eq_str", "\"a\" == \"a\"\n"),
        binary_value("lt_str", "\"a\" < \"b\"\n"),
        binary_value("eq_bool", "true == true\n"),
        binary_value("eq_none", "none == none\n"),
        // ----- bitwise / shifts ------------------------------------------
        binary_value("bitand", "6 & 3\n"),
        binary_value("bitor", "6 | 3\n"),
        binary_value("shl", "1 << 4\n"),
        binary_value("shr", "16 >> 2\n"),
        // ----- overflow / divide-zero / shift-range ----------------------
        binary_value("add_overflow", "9223372036854775807 + 1\n"),
        binary_value("sub_overflow", "-9223372036854775808 - 1\n"),
        binary_value("mul_overflow", "9223372036854775807 * 2\n"),
        binary_value("div_min_by_neg_one", "-9223372036854775808 / -1\n"),
        binary_value("rem_min_by_neg_one", "-9223372036854775808 % -1\n"),
        binary_value("div_zero_int", "1 / 0\n"),
        binary_value("rem_zero_int", "1 % 0\n"),
        binary_value("div_zero_float", "1.0 / 0.0\n"),
        binary_value("neg_pow_exponent", "2 ^ -1\n"),
        binary_value("shift_too_large", "1 << 64\n"),
        binary_value("shift_negative", "1 >> -1\n"),
        // ----- invalid operand types -------------------------------------
        binary_value("add_bool", "true + false\n"),
        binary_value("sub_str", "\"a\" - \"b\"\n"),
        binary_value("bitand_float", "1.5 & 2\n"),
        binary_value("shl_float", "1 << 1.5\n"),
        binary_value("lt_bool", "true < false\n"),
        // ----- composition: precedence, associativity, nesting -----------
        binary_value("precedence_mul_add", "1 + 2 * 3\n"),
        binary_value("assoc_sub_left", "10 - 3 - 2\n"),
        binary_value("assoc_pow_right", "2 ^ 3 ^ 2\n"),
        binary_value("nested_binary", "(1 + 2) * (3 - 1)\n"),
        binary_value("binary_with_unary", "-2 + -3\n"),
        binary_value("unary_of_binary", "-(1 + 2)\n"),
        binary_value("bitwise_precedence", "1 | 2 == 3\n"),
        // ----- binary inside R3A constructs ------------------------------
        binary_value("binary_in_block", "{ let x = 3\n x + 4 }\n"),
        binary_value("binary_in_if_cond", "if 1 + 1 == 2 { 1 } else { 2 }\n"),
        binary_value(
            "binary_let_chain",
            "{ let a = 1 + 1\n let b = a * 2\n b }\n",
        ),
        // ----- program (real frame boundary) -----------------------------
        binary_program("main_add", "fn main() { return 1 + 2 }\n"),
        binary_program("main_cmp", "fn main() { if 1 < 2 { return } }\n"),
        binary_program(
            "main_nested_binary",
            "fn main() { let x = (2 + 3) * 4\n if x == 20 { return } }\n",
        ),
    ]
}

/// B-1R3B.2 — short-circuit operators (`and`/`or`) are B-1R3B.3 and must remain
/// explicitly unsupported: the iterative engine must produce the `E4999`
/// sentinel and never conditionally evaluate the right operand.
#[must_use]
pub fn binary_unsupported_cases() -> Vec<Case> {
    vec![
        binary_value("and_bool", "true and false\n"),
        binary_value("or_bool", "false or true\n"),
        // A supported eager operator whose operand is a short-circuit operator
        // must not make the `and`/`or` work through another path.
        binary_value("eager_over_and", "1 + (true and false)\n"),
        // A supported eager operator whose operand remains unsupported (call).
        binary_value("eager_over_call", "1 + len([1, 2])\n"),
    ]
}
