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

/// B-1R3B.2 — eager-binary cases whose operand remains unsupported. `and_bool`,
/// `or_bool`, and `eager_over_and` were removed in B-1R3B.3 because the
/// short-circuit operators became supported (they are asserted in
/// [`short_circuit_supported_cases`]); the call operand remains R3C work.
#[must_use]
pub fn binary_unsupported_cases() -> Vec<Case> {
    vec![
        // A supported eager operator whose operand remains unsupported (call).
        binary_value("eager_over_call", "1 + len([1, 2])\n"),
    ]
}

fn sc_value(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b3-value",
        name,
        file: "<r3b3>",
        source,
        kind: Kind::Value,
    }
}

fn sc_program(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b3-program",
        name,
        file: "r3b3.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

/// B-1R3B.3 — short-circuit `and`/`or` cases the iterative engine supports.
/// Recursive and iterative must agree exactly on value, type, diagnostics,
/// spans, and stdout. The set pins the four corners of the short-circuit
/// decision, truthiness over every value kind constructible in the subset,
/// real skipping (unsupported constructs and runtime errors on a skipped right
/// operand must never occur), required-operand diagnostics, control-signal
/// propagation, and composition with the R3A/R3B.1/R3B.2 subsets.
#[must_use]
pub fn short_circuit_supported_cases() -> Vec<Case> {
    vec![
        // ----- `and` truthiness: left decides falsy -----------------------
        sc_value("and_none", "none and true\n"),
        sc_value("and_false", "false and true\n"),
        // Moved verbatim from the R3B.2 unsupported set (`and_bool`/`or_bool`):
        // both are now supported and must agree with the recursive engine.
        sc_value("and_true_false", "true and false\n"),
        sc_value("or_false_true", "false or true\n"),
        sc_value("and_int_zero", "0 and true\n"),
        sc_value("and_float_zero", "0.0 and 1\n"),
        sc_value("and_str_empty", "\"\" and 1\n"),
        // ----- `and` truthiness: left truthy, right decides --------------
        sc_value("and_true", "true and true\n"),
        sc_value("and_int_nonzero", "7 and true\n"),
        sc_value("and_float_nonzero", "2.5 and 1\n"),
        sc_value("and_str_nonempty", "\"x\" and 1\n"),
        sc_value("and_fn_value", "len and true\n"),
        sc_value("and_fn_name", "fn f() {}\nf and true\n"),
        sc_value("and_rhs_falsy", "true and 0\n"),
        // ----- `or` truthiness: left decides truthy ----------------------
        sc_value("or_true", "true or false\n"),
        sc_value("or_int_nonzero", "3 or 0\n"),
        sc_value("or_str_nonempty", "\"a\" or \"\"\n"),
        sc_value("or_fn_value", "len or false\n"),
        // ----- `or` truthiness: left falsy, right decides ----------------
        sc_value("or_none", "none or true\n"),
        sc_value("or_false", "false or true\n"),
        sc_value("or_int_zero", "0 or 7\n"),
        sc_value("or_float_zero", "0.0 or 2.5\n"),
        sc_value("or_str_empty", "\"\" or \"x\"\n"),
        sc_value("or_rhs_falsy", "false or none\n"),
        sc_value("or_rhs_truthy", "false or 3\n"),
        // ----- real skipping: unsupported right operands must not run ----
        // If the machine evaluated any of these rights it would fail with the
        // E4999 unsupported sentinel (or, for the call, diverge from the
        // recursive engine); short-circuit equality with the recursive engine
        // proves the right operand never ran.
        sc_value("and_skip_list", "false and [1, 2]\n"),
        sc_value("and_skip_map", "false and {\"a\": 1}\n"),
        // Moved from the R3B.3 unsupported set in B-1R3B.4.1: list literals are
        // supported now, so a required list operand and list-typed left
        // operands agree with the recursive engine (truthiness of a list).
        sc_value("and_required_list", "true and [1, 2]\n"),
        sc_value("or_required_list", "false or [1, 2]\n"),
        sc_value("and_lhs_list_falsy", "[] and true\n"),
        sc_value("and_lhs_list_truthy", "[1] and true\n"),
        sc_value("or_lhs_list_truthy", "[1] or false\n"),
        sc_value("or_lhs_list_falsy", "[] or 7\n"),
        sc_value("and_skip_range", "false and (1..3)\n"),
        sc_value("and_skip_fstring", "false and f\"v={1}\"\n"),
        sc_value("and_skip_call", "false and len([1, 2])\n"),
        sc_value("or_skip_list", "true or [1, 2]\n"),
        sc_value("or_skip_range", "true or (1..3)\n"),
        sc_value("or_skip_fstring", "true or f\"v={1}\"\n"),
        sc_value("or_skip_call", "true or len([1, 2])\n"),
        sc_value("nested_skip_call", "true and (true or len([1, 2]))\n"),
        // ----- real skipping: runtime errors must not occur --------------
        sc_value("and_skip_div", "false and (1 / 0)\n"),
        sc_value("and_skip_overflow", "false and (9223372036854775807 + 1)\n"),
        sc_value("and_skip_shift", "false and (1 << 64)\n"),
        sc_value("and_skip_badop", "false and (\"a\" - \"b\")\n"),
        sc_value("or_skip_div", "true or (1 / 0)\n"),
        sc_value("or_skip_overflow", "true or (9223372036854775807 + 1)\n"),
        sc_value("or_skip_badop", "true or (\"a\" - \"b\")\n"),
        // ----- required right operands: exact recursive diagnostics ------
        sc_value("and_req_div", "true and (1 / 0)\n"),
        sc_value("or_req_div", "false or (1 / 0)\n"),
        sc_value("and_req_badop", "true and (\"a\" - \"b\")\n"),
        sc_value("or_req_badop", "false or (\"a\" - \"b\")\n"),
        sc_value("and_req_overflow", "true and (9223372036854775807 + 1)\n"),
        sc_value("or_req_shift", "false or (1 << 64)\n"),
        // ----- control signals in operands --------------------------------
        // A signal from the left skips the right (no E4007), and a signal from
        // a required right propagates unchanged; the value path maps the
        // residual signal exactly like the recursive engine.
        sc_value("and_lhs_return_skips", "{ return 5 } and (1 / 0)\n"),
        sc_value("or_lhs_return_skips", "{ return 5 } or (1 / 0)\n"),
        sc_value("and_lhs_throw_skips", "{ throw 5 } and (1 / 0)\n"),
        sc_value("and_rhs_return", "true and { return 5 }\n"),
        sc_value("or_rhs_throw", "false or { throw 5 }\n"),
        // A skipped right operand cannot emit its signal.
        sc_value("and_skip_rhs_throw", "false and { throw 5 }\n"),
        sc_value("or_skip_rhs_throw", "true or { throw 5 }\n"),
        // ----- composition with unary and eager operators -----------------
        sc_value("unary_not_and", "not true and false\n"),
        sc_value("and_unary_not", "true and not false\n"),
        sc_value("or_unary_not", "false or not true\n"),
        sc_value("not_paren_or", "not (false or true)\n"),
        sc_value("precedence_or_and", "false or true and false\n"),
        sc_value("paren_or_and", "(false or true) and false\n"),
        sc_value("chain_mixed", "0 or 0 or \"x\" and true\n"),
        sc_value(
            "nested_both",
            "(true and (false or true)) and (true or false)\n",
        ),
        sc_value("eager_lhs", "(1 + 2) and (3 * 4)\n"),
        sc_value("eager_rhs", "true and 1 + 1 == 2\n"),
        // Moved from the R3B.2 unsupported set: the inner `and` is supported
        // now; the outer `+` then applies to `bool` and must produce the same
        // E3001 the recursive engine does.
        sc_value("eager_over_and", "1 + (true and false)\n"),
        sc_value("and_in_if", "if (0 or 1) and (2 and 3) { 7 } else { 8 }\n"),
        sc_value("let_binding", "{ let x = false and true\n x }\n"),
        sc_value("block_value", "{ 1 and 2 }\n"),
        // ----- program (real frame boundary, no calls) --------------------
        sc_program("main_and_short", "fn main() { let x = false and true }\n"),
        sc_program("main_or_short", "fn main() { let x = true or false }\n"),
        sc_program(
            "main_and_skip_unsupported",
            "fn main() { let x = false and [1, 2] }\n",
        ),
        sc_program(
            "main_or_skip_unsupported",
            "fn main() { let x = true or [1, 2] }\n",
        ),
        sc_program("main_if_skip", "fn main() { if false and [1, 2] { } }\n"),
        sc_program(
            "main_skip_rhs_throw",
            "fn main() { let x = false and { throw 7 } }\n",
        ),
        sc_program(
            "main_return_skip_rhs",
            "fn main() { let x = { return } and (1 / 0) }\n",
        ),
        sc_program(
            "main_required_div",
            "fn main() { let x = true and (1 / 0) }\n",
        ),
    ]
}

/// B-1R3B.3 — `and`/`or` whose left operand is unsupported, or whose *required*
/// right operand is unsupported, must fail with the deterministic `E4999`
/// sentinel in iterative mode. This is the anti-fallback guard for the
/// short-circuit extension: a skipped right operand must not be evaluated (it
/// is a supported case above), and a required one must not be executed through
/// the recursive engine.
#[must_use]
pub fn short_circuit_unsupported_cases() -> Vec<Case> {
    vec![
        // Required right operand is a call (R3C).
        sc_value("and_required_call", "true and len([1, 2])\n"),
        sc_value("or_required_call", "false or len([1, 2])\n"),
        // Required right operand is a range/map (R3B.5/R3B.4.2). The list
        // forms were moved to the supported set in B-1R3B.4.1.
        sc_value("and_required_range", "true and (1..3)\n"),
        sc_value("or_required_map", "false or {\"a\": 1}\n"),
        // Left operand itself is an unsupported map literal.
        sc_value("and_lhs_map_unsupported", "{\"a\": 1} and true\n"),
        sc_value("or_lhs_map_unsupported", "{\"a\": 1} or true\n"),
        // A required nested `and` reaching an unsupported call.
        sc_value("nested_required_call", "false or (true and len([1, 2]))\n"),
        // Same shape inside a real user frame.
        sc_program(
            "main_required_call",
            "fn main() { let x = true and len([1, 2]) }\n",
        ),
    ]
}

fn list_value(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b4-value",
        name,
        file: "<r3b4>",
        source,
        kind: Kind::Value,
    }
}

fn list_program(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b4-program",
        name,
        file: "r3b4.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

/// B-1R3B.4.1 — list/tuple construction cases the iterative engine supports.
/// Recursive and iterative must agree exactly on value, type, diagnostics,
/// spans, and stdout. `Expr::Tuple` is list sugar (`LANGUAGE_SPEC.md` §21), so
/// tuple cases assert the same list-valued result.
#[must_use]
pub fn list_supported_cases() -> Vec<Case> {
    vec![
        // ----- shape: empty / one / many ----------------------------------
        list_value("empty", "[]\n"),
        list_value("single", "[7]\n"),
        list_value("multi", "[1, 2, 3]\n"),
        list_value("trailing_comma", "[1, 2,]\n"),
        // ----- element kinds ----------------------------------------------
        list_value("mixed_values", "[1, \"x\", true, none, 2.5]\n"),
        list_value("nested", "[[1, 2], [3], []]\n"),
        list_value("deep_nested_value", "[[[[1]]]]\n"),
        // ----- names and expressions inside ---------------------------------
        list_value("name_element", "{ let x = 3\n [x, x + 1] }\n"),
        list_value("unary_inside", "[-1, not false, ~0]\n"),
        list_value("binary_inside", "[1 + 2, 3 * 4, 1 < 2]\n"),
        list_value("short_circuit_inside", "[false and true, true or false]\n"),
        // A skipped short-circuit right operand inside an element must not run.
        list_value(
            "skipped_rhs_inside",
            "[true or (1 / 0), false and (1 / 0)]\n",
        ),
        // ----- tuple sugar -------------------------------------------------
        list_value("tuple_two", "(1, 2)\n"),
        list_value("tuple_one_trailing", "(1,)\n"),
        list_value("tuple_mixed", "(1, \"a\", true)\n"),
        list_value("tuple_nested", "((1, 2), (3, 4))\n"),
        // ----- composition with R3A constructs -----------------------------
        list_value("in_block", "{ [1, 2] }\n"),
        list_value("in_if_taken", "if true { [1, 2] } else { [] }\n"),
        list_value("in_if_condition", "if [1] { 7 } else { 8 }\n"),
        list_value("empty_in_if_condition", "if [] { 7 } else { 8 }\n"),
        list_value("let_binding", "{ let xs = [1, 2]\n xs }\n"),
        list_value("let_shadow", "{ let xs = [1]\n let ys = [xs, 2]\n ys }\n"),
        // ----- element evaluation: errors and spans ------------------------
        // First element fails: E4007 with the division's own span.
        list_value("first_element_div", "[1 / 0, 2]\n"),
        // Intermediate element fails.
        list_value("middle_element_div", "[1, 2 / 0, 3]\n"),
        // Last element fails.
        list_value("last_element_div", "[1, 2, 3 / 0]\n"),
        // First error wins: the earlier element's E4007 is reported, not the
        // later element's E3001.
        list_value("first_error_wins", "[1 / 0, \"a\" - \"b\"]\n"),
        // A type error inside an element keeps the operator's span.
        list_value("element_type_error", "[1, -true]\n"),
        // ----- element signals ---------------------------------------------
        // A signal from an element aborts the list; later elements must not
        // run (the E4007 must never occur).
        list_value("element_return_aborts", "[{ return 5 }, 1 / 0]\n"),
        list_value("element_throw_aborts", "[{ throw 9 }, 1 / 0]\n"),
        // The abort also skips an *unsupported* later element: if the machine
        // scheduled it, the E4999 sentinel would be reported instead of the
        // signal-derived outcome the recursive engine produces.
        list_value(
            "skipped_unsupported_after_return",
            "[{ return 5 }, len([1, 2])]\n",
        ),
        list_value(
            "skipped_unsupported_after_throw",
            "[{ throw 9 }, len([1, 2])]\n",
        ),
        // A signal from a later element still propagates.
        list_value("later_element_return", "[1, { return 6 }]\n"),
        // ----- program mode (real frame boundary) --------------------------
        list_program("main_list_let", "fn main() { let xs = [1, 2, 3] }\n"),
        list_program("main_tuple_let", "fn main() { let xs = (1, 2) }\n"),
        list_program("main_list_error", "fn main() { let xs = [1, 1 / 0] }\n"),
        list_program("main_list_in_if", "fn main() { if [1] { let xs = [] } }\n"),
        list_program(
            "main_list_signal",
            "fn main() { let xs = [{ return }, 1 / 0] }\n",
        ),
    ]
}

/// B-1R3B.4.1 — list/tuple elements that remain unsupported (calls are R3C,
/// maps are R3B.4.2, ranges R3B.5) must fail with the deterministic `E4999`
/// sentinel in iterative mode, never fall back to recursion, and never yield a
/// partial list. An unsupported construct on a path that is *not* reached
/// cannot be tested with list construction (every element of a list literal is
/// reached); the unreachable case is covered by the short-circuit set above.
#[must_use]
pub fn list_unsupported_cases() -> Vec<Case> {
    vec![
        // An unsupported call element is reached and fails; the supported
        // first element must not let the machine fall back.
        list_value("call_element", "[1, len([1, 2])]\n"),
        // An unsupported map element is reached and fails.
        list_value("map_element", "[1, {\"a\": 1}]\n"),
        // An unsupported range element is reached and fails.
        list_value("range_element", "[1, 1..3]\n"),
        // An unsupported construct *inside* a supported element expression
        // (an f-string operand in a binary) is reached and fails.
        list_value("nested_unsupported_element", "[1 + f\"v={1}\"]\n"),
        // Same shapes inside a real user frame.
        list_program(
            "main_call_element",
            "fn main() { let xs = [1, len([1, 2])] }\n",
        ),
        list_program(
            "main_map_element",
            "fn main() { let xs = [1, {\"a\": 1}] }\n",
        ),
    ]
}
