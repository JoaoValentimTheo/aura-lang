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
        // The operand is a lambda (R3C.4): the unary machine must not apply
        // `-`/`not` to it, and must not fall back to recursion. (These probes
        // used a call operand until B-1R3C.1 made calls supported; the
        // call-operand shapes are now supported cases in
        // `r3c1::supported_cases`.)
        value("neg_of_unsupported_lambda", "-(() -> 1)\n"),
        value("not_of_unsupported_lambda", "not (() -> 1)\n"),
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
        // A supported eager operator whose operand remains unsupported (a
        // lambda). The original `eager_over_call` (`1 + len([1, 2])`) became
        // supported in B-1R3C.1 and is asserted by `r3c1::supported_cases`.
        binary_value("eager_over_lambda", "1 + (() -> 1)\n"),
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
        // Moved from the R3B.3 unsupported set in B-1R3B.4.2: map construction
        // is supported now, so a required map operand and map-typed left
        // operands agree with the recursive engine (truthiness of a map).
        sc_value("or_required_map", "false or {\"a\": 1}\n"),
        sc_value("and_lhs_map", "{\"a\": 1} and true\n"),
        sc_value("or_lhs_map", "{\"a\": 1} or true\n"),
        // Moved from the R3B.3 unsupported set in B-1R3B.5: range construction
        // is supported now, so a required range operand and range-typed left
        // operands agree with the recursive engine (a range is truthy).
        sc_value("and_required_range", "true and (1..3)\n"),
        sc_value("or_required_range", "false or (1..3)\n"),
        sc_value("and_lhs_range", "(1..3) and true\n"),
        sc_value("or_lhs_range", "(1..3) or false\n"),
        sc_value("or_lhs_empty_range", "(3..1) or 7\n"),
        sc_value("and_skip_range", "false and (1..3)\n"),
        // The f-string skip cases (`and_skip_fstring`/`or_skip_fstring`) moved
        // to `r3b::fstring_supported_cases` in B-1R3B.7, where they additionally
        // assert that a skipped f-string is never evaluated.
        sc_value("and_skip_call", "false and len([1, 2])\n"),
        sc_value("or_skip_list", "true or [1, 2]\n"),
        sc_value("or_skip_range", "true or (1..3)\n"),
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
        // Required right operand is a lambda (R3C.4). The original call-shaped
        // probes (`and_required_call`, `or_required_call`, `nested_required_call`,
        // `main_required_call`) became supported in B-1R3C.1 and are asserted by
        // `r3c1::supported_cases`.
        sc_value("and_required_lambda", "true and (() -> 1)\n"),
        sc_value("or_required_lambda", "false or (() -> 1)\n"),
        // A required nested `and` reaching an unsupported lambda.
        sc_value("nested_required_lambda", "false or (true and (() -> 1))\n"),
        // Same shape inside a real user frame.
        sc_program(
            "main_required_lambda",
            "fn main() { let x = true and (() -> 1) }\n",
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
        // Moved from the R3B.4.1 unsupported set in B-1R3B.4.2: a map element
        // is supported now, so list/map composition agrees with recursion.
        list_value("map_element", "[1, {\"a\": 1}]\n"),
        // Moved from the R3B.4.1 unsupported set in B-1R3B.5: a range element
        // is supported now, so a list containing a Range agrees with recursion
        // (`LANGUAGE_SPEC.md` §22.1: `[1..3]` is a one-element list).
        list_value("range_element", "[1, 1..3]\n"),
        // ----- program mode (real frame boundary) --------------------------
        list_program("main_list_let", "fn main() { let xs = [1, 2, 3] }\n"),
        list_program("main_tuple_let", "fn main() { let xs = (1, 2) }\n"),
        list_program("main_list_error", "fn main() { let xs = [1, 1 / 0] }\n"),
        list_program("main_list_in_if", "fn main() { if [1] { let xs = [] } }\n"),
        list_program(
            "main_list_signal",
            "fn main() { let xs = [{ return }, 1 / 0] }\n",
        ),
        // Moved from the R3B.4.1 unsupported set in B-1R3B.5: a list containing
        // a Range through a real frame boundary.
        list_program("main_list_range", "fn main() { let xs = [1..3] }\n"),
    ]
}

/// B-1R3B.4.1 — list/tuple elements that remain unsupported (calls are R3C,
/// f-strings R3B.7; ranges moved to supported in R3B.5) must fail with the
/// deterministic `E4999` sentinel in iterative mode, never fall back to
/// recursion, and never yield a partial list. An unsupported construct on a
/// path that is *not* reached cannot be tested with list construction (every
/// element of a list literal is reached); the unreachable case is covered by
/// the short-circuit set above.
#[must_use]
pub fn list_unsupported_cases() -> Vec<Case> {
    vec![
        // An unsupported lambda element is reached and fails; the supported
        // first element must not let the machine fall back. The original
        // `call_element` (`[1, len([1, 2])]`) became supported in B-1R3C.1 and
        // is asserted by `r3c1::supported_cases`.
        list_value("lambda_element", "[1, (() -> 1)]\n"),
        // `nested_unsupported_element` (`[1 + f"v={1}"]`) moved to
        // `r3b::fstring_supported_cases` in B-1R3B.7: the f-string is supported
        // now and the element's `int + string` is a real E3001 both engines
        // agree on.
        // Same shapes inside a real user frame.
        list_program(
            "main_lambda_element",
            "fn main() { let xs = [1, (() -> 1)] }\n",
        ),
    ]
}

fn map_value(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b42-value",
        name,
        file: "<r3b42>",
        source,
        kind: Kind::Value,
    }
}

fn map_program(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b42-program",
        name,
        file: "r3b42.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

/// B-1R3B.4.2 — map-construction cases the iterative engine supports.
/// Recursive and iterative must agree exactly on value, type, diagnostics,
/// spans, and stdout. The set pins construction shape, valid key kinds, the
/// exact key→value→next-entry order, invalid-key diagnostics and spans,
/// value suppression after a failed key, later-entry suppression after a
/// failed value, control-signal abort, duplicate-key last-wins, nesting and
/// Map/List composition, and unsupported-surface non-reachability.
#[must_use]
pub fn map_supported_cases() -> Vec<Case> {
    vec![
        // ----- shape: empty / one / many ----------------------------------
        map_value("empty", "{:}\n"),
        map_value("empty_spaced", "{ : }\n"),
        map_value("single_int_key", "{1: \"a\"}\n"),
        map_value("single_str_key", "{\"a\": 1}\n"),
        map_value("single_bool_key", "{true: 1}\n"),
        map_value("multi_str", "{\"a\": 1, \"b\": 2, \"c\": 3}\n"),
        // ----- display/iteration order is by key, not source order --------
        map_value("int_keys_order", "{3: \"c\", 1: \"a\", 2: \"b\"}\n"),
        map_value("bool_keys_order", "{true: 1, false: 0}\n"),
        map_value("mixed_key_kinds", "{true: 1, \"a\": 2, 1: 3}\n"),
        // ----- valid value kinds ------------------------------------------
        map_value("mixed_values", "{1: \"x\", 2: true, 3: none, 4: 2.5}\n"),
        // ----- expressions as keys and values -----------------------------
        map_value("expr_key", "{1 + 1: \"two\"}\n"),
        map_value("expr_value", "{1: 2 * 3}\n"),
        map_value("name_key", "{ let k = 7\n {k: k} }\n"),
        map_value("name_value", "{ let v = 9\n {1: v} }\n"),
        map_value("unary_key", "{-1: \"neg\"}\n"),
        map_value("unary_value", "{1: -2}\n"),
        map_value("binary_value", "{1: 1 + 2, 2: 3 * 4}\n"),
        map_value(
            "short_circuit_value",
            "{1: false and true, 2: true or false}\n",
        ),
        // A skipped short-circuit right operand inside a value must not run.
        map_value(
            "skipped_rhs_value",
            "{1: true or (1 / 0), 2: false and (1 / 0)}\n",
        ),
        // ----- duplicates: last write wins --------------------------------
        map_value("duplicate_int", "{1: \"first\", 1: \"second\"}\n"),
        map_value("duplicate_with_other", "{1: \"a\", 2: \"b\", 1: \"c\"}\n"),
        map_value("duplicate_bool", "{true: 1, true: 2}\n"),
        map_value("duplicate_str", "{\"k\": 1, \"k\": 2}\n"),
        // ----- nesting / composition --------------------------------------
        map_value("nested_map", "{1: {2: 3}}\n"),
        map_value("map_of_lists", "{1: [1, 2], 2: [3]}\n"),
        map_value("list_of_maps", "[{\"a\": 1}, {\"b\": 2}]\n"),
        // Moved from the R3B.4.2 unsupported set in B-1R3B.5: a range value is
        // supported now, so a map containing a Range value agrees with
        // recursion.
        map_value("range_value", "{1: (1..3)}\n"),
        map_value("deep_nested_value", "{1: {2: {3: 4}}}\n"),
        // ----- evaluation order: key before value, entry by entry ---------
        // If entries were reordered, the first error would differ.
        map_value("first_key_error_wins", "{1 / 0: 2, \"a\" - \"b\": 3}\n"),
        map_value("first_value_error_wins", "{1: 1 / 0, \"a\" - \"b\": 3}\n"),
        // The first value must complete before the second key runs: the
        // division is in the *second* entry's key.
        map_value("later_key_error", "{1: 2, 1 / 0: 3}\n"),
        map_value("later_value_error", "{1: 2, 3: 1 / 0}\n"),
        // Key before value on the same entry: the key's error wins.
        map_value("key_before_value", "{\"a\" - \"b\": 1 / 0}\n"),
        // ----- invalid keys: E3001 at the key expression's span -----------
        // `float`, `none`, `[int]`, and `{int: int}` keys are rejected by the
        // checker for literal keys; the runtime-invalid paths are reached via
        // an untyped binding whose static type is `Unknown`.
        map_value("invalid_float_key_check", "{1.0: \"x\"}\n"),
        map_value("invalid_none_key_check", "{none: \"x\"}\n"),
        map_value("invalid_list_key_check", "{[1]: \"x\"}\n"),
        map_value("invalid_range_key_check", "{(1..3): 1}\n"),
        map_value("invalid_runtime_none_key", "{ let x = none\n {x: 1} }\n"),
        // A runtime-invalid key must suppress its value: the diagnostic is the
        // key's E3001, never the value's E4007.
        map_value(
            "invalid_key_suppresses_value",
            "{ let x = none\n {x: 1 / 0} }\n",
        ),
        // ----- control signals in keys and values -------------------------
        // A signal from a key aborts the map and skips the value and every
        // later entry (the E4007 must never occur).
        map_value("key_return_aborts", "{ {return 5}: 1 / 0 }\n"),
        map_value("key_throw_aborts", "{ {throw 5}: 1 / 0 }\n"),
        // A signal from a value aborts the map and skips later entries.
        map_value("value_return_aborts", "{1: {return 5}, 2: 1 / 0}\n"),
        map_value("value_throw_aborts", "{1: {throw 9}, 2: 1 / 0}\n"),
        // A signal from a later entry's key still propagates once reached.
        map_value("later_key_return", "{1: 2, {return 6}: 3}\n"),
        // ----- composition with R3A/R3B constructs ------------------------
        map_value("in_block", "{ {1: 2} }\n"),
        map_value("in_if_taken", "if true { {1: 2} } else { {:} }\n"),
        map_value("in_if_condition", "if {1: 2} { 7 } else { 8 }\n"),
        map_value("empty_in_if_condition", "if {:} { 7 } else { 8 }\n"),
        map_value("let_binding", "{ let m = {\"a\": 1}\n m }\n"),
        map_value("let_shadow", "{ let k = 1\n let m = {k: 2}\n m }\n"),
        // ----- program mode (real frame boundary) --------------------------
        map_program(
            "main_map_let",
            "fn main() { let m = {1: \"a\", 2: \"b\"} }\n",
        ),
        map_program(
            "main_map_in_if",
            "fn main() { if {3: \"c\", 1: \"a\"} { let m = {:} } }\n",
        ),
        map_program("main_map_empty", "fn main() { let m = {:} }\n"),
        map_program("main_map_error", "fn main() { let m = {1: 1 / 0} }\n"),
        map_program(
            "main_map_dup",
            "fn main() { let m = {1: \"a\", 1: \"b\"} }\n",
        ),
        map_program(
            "main_map_signal",
            "fn main() { let m = {1: {return}, 2: 1 / 0} }\n",
        ),
        map_program(
            "main_map_invalid_key",
            "fn main() { let x = none\n let m = {x: 1} }\n",
        ),
        // Restored by the R3B.4.2 push-blocker remediation: a list whose
        // element is a Map, executed through a real frame boundary. It is the
        // only R3B.4.2 case crossing the frame boundary with a Map nested in a
        // List; no value-mode case substitutes for it.
        map_program(
            "main_map_element",
            "fn main() { let xs = [1, {\"a\": 1}] }\n",
        ),
        // Moved from the R3B.4.2 unsupported set in B-1R3B.5: a Map whose value
        // is a Range through a real frame boundary.
        map_program("main_map_range", "fn main() { let m = {1: (2..5)} }\n"),
    ]
}

/// B-1R3B.4.2 — map keys/values that remain unsupported (calls are R3C,
/// f-strings R3B.7; ranges moved to supported in R3B.5) must fail with the
/// deterministic `E4999` sentinel in iterative mode, never fall back to
/// recursion, and never yield a partial map. A *skipped* unsupported construct
/// cannot be tested with map construction (every key and value of a literal is
/// reached); the unreachable case is covered by the short-circuit set above.
#[must_use]
pub fn map_unsupported_cases() -> Vec<Case> {
    vec![
        // An unsupported lambda value is reached and fails. The original
        // `call_key`/`call_value` probes became supported in B-1R3C.1 and are
        // asserted by `r3c1::supported_cases`. (A lambda cannot be a map key:
        // the checker rejects it as a key type, so the value position is the
        // reachable shape.)
        map_value("lambda_value", "{1: (() -> 1)}\n"),
        // `fstring_value` (`{1: f"v={1}"}`) moved to
        // `r3b::fstring_supported_cases` in B-1R3B.7: the f-string is a
        // supported value now, and a string map value agrees with recursion.
        // A supported first entry must not let the machine fall back on a
        // later unsupported one.
        map_value("later_lambda_value", "{1: 2, 2: (() -> 1)}\n"),
        // Same shapes inside a real user frame.
        map_program(
            "main_lambda_value",
            "fn main() { let m = {1: (() -> 1)} }\n",
        ),
    ]
}

fn range_value(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b5-value",
        name,
        file: "<r3b5>",
        source,
        kind: Kind::Value,
    }
}

fn range_program(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b5-program",
        name,
        file: "r3b5.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

/// B-1R3B.5 — range-construction cases the iterative engine supports.
/// Recursive and iterative must agree exactly on value, type, diagnostics,
/// spans, and stdout. The set pins the value representation (start/end), the
/// half-open endpoint semantics (`b <= a` empty; no materialization), the
/// start-then-end evaluation order and exactly-once behavior, the E3001
/// diagnostics and span, control-signal propagation, extreme `i64` bounds, and
/// composition with R3A/R3B.1–R3B.4.2 and program mode. This is the current
/// step-1, half-open Range only; the planned Aura 0.3 `step`/inclusive
/// extensions are deliberately absent.
#[must_use]
pub fn range_supported_cases() -> Vec<Case> {
    vec![
        // ----- endpoints and shape ----------------------------------------
        range_value("ascending", "1..4\n"),
        range_value("single", "3..4\n"),
        range_value("one_before", "0..1\n"),
        range_value("equal_bounds_empty", "4..4\n"),
        range_value("descending_empty", "5..1\n"),
        range_value("zero_start", "0..3\n"),
        range_value("negative_ascending", "-3..1\n"),
        range_value("negative_to_zero", "-3..0\n"),
        range_value("negative_descending", "1..-3\n"),
        range_value("both_negative", "-5..-1\n"),
        // ----- extreme i64 bounds (len saturates; representation exact) ----
        range_value("min_to_max", "-9223372036854775808..9223372036854775807\n"),
        range_value("max_bound", "0..9223372036854775807\n"),
        range_value("min_bound", "-9223372036854775808..0\n"),
        range_value(
            "max_to_min_empty",
            "9223372036854775807..-9223372036854775808\n",
        ),
        // ----- expressions as bounds --------------------------------------
        range_value("arithmetic_bounds", "1 + 1..2 * 3\n"),
        range_value("unary_bound", "-2..3\n"),
        range_value("bitnot_bound", "~0..2\n"),
        range_value("cmp_bound", "1..2 + 1\n"),
        range_value("name_bounds", "{ let a = 1\n let b = 5\n a..b }\n"),
        range_value("computed_end_from_name", "{ let n = 4\n 0..n - 1 }\n"),
        // ----- precedence: `..` binds looser than additive, tighter than cmp -
        range_value("precedence_add", "{ let n = 4\n 1 + 2..n - 1 }\n"),
        // ----- composition with R3A/R3B constructs ------------------------
        range_value("in_block", "{ 1..3 }\n"),
        range_value("in_if_taken", "if true { 1..3 } else { 4..5 }\n"),
        range_value("in_if_condition", "if 1..3 { 7 } else { 8 }\n"),
        range_value("empty_in_if_condition", "if 4..4 { 7 } else { 8 }\n"),
        range_value("let_binding", "{ let r = 1..3\n r }\n"),
        range_value("in_list", "[1..3, 4..6]\n"),
        range_value("in_tuple", "(1..3, 4..6)\n"),
        range_value("in_map_value", "{1: 0..2, 2: 3..5}\n"),
        // ----- equality/identity by start/end -----------------------------
        range_value("equal_literals", "(1..3) == (1..3)\n"),
        range_value("unequal_end", "(1..3) == (1..4)\n"),
        range_value("descending_eq", "(5..1) == (5..1)\n"),
        // ----- diagnostics: bounds must be int, E3001 at the range span ----
        range_value("float_start_check", "1.5..3\n"),
        range_value("float_end_check", "1..2.0\n"),
        range_value("bool_start_check", "true..3\n"),
        range_value("str_end_check", "1..\"x\"\n"),
        range_value("none_start_check", "none..3\n"),
        // Both bounds bad: start is validated first, so its message is
        // reported (matching `eval_inner`'s sequential match arms).
        range_value("both_bad_start_wins", "true..\"x\"\n"),
        // Runtime-invalid bound reached through an untyped binding: `none`
        // infers `Unknown`, so the checker cannot prove the type and the
        // runtime E3001 is exercised. A statically-typed binding (`1.5`, `true`)
        // is instead rejected at check time, so those spellings appear above.
        range_value("runtime_start_none", "{ let x = none\n x..3 }\n"),
        range_value("runtime_end_none", "{ let x = none\n 1..x }\n"),
        // Both bounds runtime-invalid and untyped: the start is validated
        // first, so the start's message is reported.
        range_value(
            "runtime_both_bad_start_wins",
            "{ let a = none\n let b = none\n a..b }\n",
        ),
        // A statically-known non-int binding is a check-time E3001.
        range_value("typed_binding_start_check", "{ let x = 1.5\n x..3 }\n"),
        range_value("typed_binding_end_check", "{ let x = true\n 1..x }\n"),
        // ----- first-error-wins and evaluation order ----------------------
        // A bad start still evaluates the end: the end's error wins only when
        // the start is a *valid* int. Here both are evaluated and the start is
        // validated first.
        range_value("first_error_wins_start", "1 / 0..3\n"),
        range_value("first_error_wins_end", "1..1 / 0\n"),
        // Left-to-right, exactly-once order proven by composition with an
        // *unsupported* end: if the end ran before the start (or at all, for a
        // signal from the start) the E4999 sentinel would surface instead of
        // the start's own E4007 / error.
        range_value("start_error_before_call_end", "1 / 0..len([1, 2])\n"),
        // ----- control signals in bounds ----------------------------------
        range_value("start_return_skips_end", "{ return 5 }..(1 / 0)\n"),
        range_value("end_return", "1..{ return 6 }\n"),
        range_value("end_throw", "1..{ throw 9 }\n"),
        // A signal from the end wins over a *runtime*-invalid start: the end
        // still evaluates even though the start is `none` (untyped), so it
        // propagates and no E3001 is produced. A `true` start would be a
        // check-time rejection, not this runtime ordering.
        range_value(
            "end_signal_over_bad_start",
            "{ let x = none\n x..{ return 5 } }\n",
        ),
        // ----- program mode (real frame boundary) -------------------------
        range_program("main_range_let", "fn main() { let r = 1..3 }\n"),
        range_program(
            "main_range_in_if",
            "fn main() { if 1..3 { let r = 9..9 } }\n",
        ),
        range_program("main_range_error", "fn main() { let r = 1..2.0 }\n"),
        range_program(
            "main_range_signal",
            "fn main() { let r = { return }..(1 / 0) }\n",
        ),
        range_program(
            "main_range_in_list",
            "fn main() { let xs = [1..3, 4..6] }\n",
        ),
        range_program("main_range_in_map", "fn main() { let m = {\"a\": 1..3} }\n"),
    ]
}

/// B-1R3B.5 — range bounds that remain unsupported (calls are R3C) must fail
/// with the deterministic `E4999` sentinel in iterative mode, never fall back
/// to recursion, and never yield a partial Range. The unsupported construct
/// must be on the *evaluated* path (a range always evaluates both bounds), so
/// every case here is reached. An f-string bound is not usable here: the
/// checker types an f-string as `string`, so the recursive engine rejects it
/// statically (`E3001`) and the runtime path is unreachable.
#[must_use]
pub fn range_unsupported_cases() -> Vec<Case> {
    vec![
        // An unsupported construct nested one level under a supported bound.
        // The original call-bound probes (`call_start`, `call_end`,
        // `call_end_after_int`, `nested_call_start`, `main_range_call`) became
        // supported in B-1R3C.1 and are asserted by `r3c1::supported_cases`. A
        // lambda is not an int, so the checker rejects it as a range bound
        // before execution; the reachable shape is a nested unsupported
        // construct under a bound, probed through an unsupported index instead.
        range_value("nested_lambda_start", "(1 + (() -> 1))..3\n"),
    ]
}

fn index_value(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b6-index-value",
        name,
        file: "<r3b6>",
        source,
        kind: Kind::Value,
    }
}

fn index_program(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b6-index-program",
        name,
        file: "r3b6.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

fn field_value(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b6-field-value",
        name,
        file: "<r3b6>",
        source,
        kind: Kind::Value,
    }
}

fn field_program(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b6-field-program",
        name,
        file: "r3b6.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

/// B-1R3B.6 — index-read cases the iterative engine supports. Recursive and
/// iterative must agree exactly on value, type, diagnostics, spans, and stdout.
/// The set pins the current `Expr::Index` surface: list (including current
/// tuple sugar), string (code-point indexing), and map (int/string/bool keys)
/// lookup; negative-index normalization; out-of-range `E4019`; missing-key
/// `E2003`; non-key-capable key `E3001`; unsupported base/index combinations
/// `E3001`; target-before-index evaluation order; control-signal propagation;
/// nested index and Index/Field/Range composition; and a real frame boundary.
/// Struct-instance indexing is deliberately absent: struct construction is
/// `Expr::Construct`, still unsupported, so no instance is reachable in this
/// subset.
#[must_use]
pub fn index_supported_cases() -> Vec<Case> {
    vec![
        // ----- list: shape and negative normalization ---------------------
        index_value("list_first", "[10, 20, 30][0]\n"),
        index_value("list_middle", "[10, 20, 30][1]\n"),
        index_value("list_last", "[10, 20, 30][2]\n"),
        index_value("list_neg_last", "[10, 20, 30][-1]\n"),
        index_value("list_neg_first", "[10, 20, 30][-3]\n"),
        index_value("list_single", "[7][0]\n"),
        index_value("list_element_is_list", "[[10, 20]][0]\n"),
        index_value("tuple_sugar", "(10, 20)[1]\n"),
        // Extreme `i64` indices fail with the exact recursive message; the
        // negative normalization is sign-safe by construction (`normalize`
        // computes in `i128`).
        index_value("list_max_index", "[1][9223372036854775807]\n"),
        index_value("list_min_index", "[1][-9223372036854775808]\n"),
        // ----- string: code-point indexing --------------------------------
        index_value("string_first", "\"abc\"[0]\n"),
        index_value("string_neg", "\"abc\"[-1]\n"),
        index_value("string_unicode", "\"h\u{e9}llo\"[1]\n"),
        // ----- map key families -------------------------------------------
        index_value("map_int_key", "{1: \"a\", 2: \"b\"}[2]\n"),
        index_value("map_str_key", "{\"a\": 1, \"b\": 2}[\"b\"]\n"),
        index_value("map_bool_key", "{true: 1, false: 2}[false]\n"),
        index_value("map_nested_value", "{1: {2: 3}}[1][2]\n"),
        index_value("map_list_value", "{1: [10, 20]}[1][0]\n"),
        index_value("map_range_value", "{1: (1..4)}[1].len\n"),
        // ----- expressions as target and index ----------------------------
        index_value("base_expr", "([1] + [2, 3])[2]\n"),
        index_value("index_expr", "[10, 20, 30][0 - 1]\n"),
        index_value("expr_key", "{1 + 1: \"two\"}[2]\n"),
        index_value("nested_index", "[[1, 2], [3, 4]][1][0]\n"),
        index_value("range_in_list", "[1..3, 4..5][1].len\n"),
        // ----- composition with R3A/R3B constructs ------------------------
        index_value("in_block", "{ [1, 2][1] }\n"),
        index_value("in_if", "if true { [1, 2][1] } else { 0 }\n"),
        index_value("in_let", "{ let xs = [1, 2, 3]\n xs[2] }\n"),
        index_value("in_range_bound", "{ [1, 2][1]..[3][0] }\n"),
        // ----- diagnostics: out of range, E4019 ---------------------------
        index_value("list_oob", "[10, 20][5]\n"),
        index_value("list_neg_oob", "[10, 20][-3]\n"),
        index_value("list_empty_oob", "[][0]\n"),
        index_value("string_oob", "\"ab\"[5]\n"),
        index_value("nested_oob", "[[1]][1][0]\n"),
        // ----- diagnostics: missing map key, E2003 ------------------------
        index_value("map_missing_int", "{1: \"a\"}[9]\n"),
        index_value("map_missing_str", "{\"a\": 1}[\"z\"]\n"),
        index_value("map_missing_bool", "{true: 1}[false]\n"),
        // ----- diagnostics: unsupported base/index combinations, E3001 ----
        index_value("bool_base", "true[0]\n"),
        index_value("float_base", "2.5[0]\n"),
        index_value("range_base", "(1..3)[0]\n"),
        index_value("list_bool_index", "[1, 2][true]\n"),
        index_value("list_float_index", "[1, 2][1.5]\n"),
        // Runtime-invalid index through an untyped binding (`none` infers
        // `Unknown`, so the checker leaves it to the runtime).
        index_value("list_none_index", "{ let x = none\n [1, 2][x] }\n"),
        index_value("map_none_key", "{ let x = none\n {1: 2}[x] }\n"),
        // A statically known map with an incompatible index kind is a
        // check-time E3001, identical in both engines.
        index_value("map_key_kind_mismatch", "{\"a\": 1}[1]\n"),
        // ----- evaluation order: target before index ----------------------
        // Both operands fail: the target's E4007 must win (if the index ran
        // first, its column would be reported instead).
        index_value("target_error_before_index", "{ 1 / 0 }[1 / 0]\n"),
        // The index is evaluated only after a *successful* target.
        index_value("index_error_after_target", "[1, 2][1 / 0]\n"),
        // ----- control signals in target and index ------------------------
        // A target signal must skip the failing index (the E4007 must never
        // surface).
        index_value("target_signal_skips_index", "{ return 5 }[1 / 0]\n"),
        index_value("target_throw_skips_index", "{ throw 5 }[1 / 0]\n"),
        // An index signal propagates (the target already succeeded).
        index_value("index_signal_return", "[1, 2][{ return 6 }]\n"),
        index_value("index_signal_throw", "[1, 2][{ throw 9 }]\n"),
        // ----- program mode (real frame boundary) -------------------------
        index_program(
            "main_index_let",
            "fn main() { let xs = [1, 2, 3]\n let y = xs[1] }\n",
        ),
        index_program(
            "main_index_error",
            "fn main() { let xs = [1]\n let y = xs[5] }\n",
        ),
        index_program(
            "main_index_map_nested",
            "fn main() { let m = {1: [10, 20]}\n let y = m[1][0] }\n",
        ),
        index_program(
            "main_index_range",
            "fn main() { let xs = [1..3, 4..5]\n let y = xs[0] }\n",
        ),
        index_program("main_index_signal", "fn main() { let y = { return }[0] }\n"),
    ]
}

/// B-1R3B.6 — field-read cases the iterative engine supports. The field name is
/// syntactic; the receiver is evaluated exactly once and resolution mirrors
/// `Interp::eval_inner`'s `Expr::Field` arm. Struct instances are unreachable
/// (construction is still unsupported), so the reachable surface is the
/// zero-argument builtin registry and its `E2003` unknown-member diagnostic.
#[must_use]
pub fn field_supported_cases() -> Vec<Case> {
    vec![
        // ----- builtin zero-argument members ------------------------------
        field_value("list_len", "[1, 2, 3].len\n"),
        field_value("string_len", "\"abcd\".len\n"),
        field_value("map_len", "{1: 2, 3: 4}.len\n"),
        field_value("range_len", "(1..4).len\n"),
        field_value("chained_sort_len", "[3, 1, 2].sort.len\n"),
        // ----- composition with R3A/R3B constructs ------------------------
        field_value("in_block", "{ [1, 2].len }\n"),
        field_value("in_if_condition", "if [1].len { 7 } else { 8 }\n"),
        field_value("in_let", "{ let xs = [1, 2]\n xs.len }\n"),
        field_value("index_then_field", "[[1, 2], [3, 4]][0].len\n"),
        field_value("field_then_index", "[3, 1, 2].sort[0]\n"),
        // ----- diagnostics: receiver error wins over member resolution ----
        field_value("receiver_error", "{ 1 / 0 }.len\n"),
        field_value("receiver_throw", "{ throw 5 }.len\n"),
        field_value("receiver_return", "{ return 5 }.len\n"),
        // ----- unknown members: E2003, exact recursive message -----------
        field_value("unknown_on_list", "[1, 2].foo\n"),
        field_value("unknown_on_int", "42.foo\n"),
        field_value("len_on_bool", "true.len\n"),
        field_value("len_on_none", "none.len\n"),
        field_value("len_on_indexed_int", "[1, 2][0].len\n"),
        // An untyped binding (`none` infers `Unknown`) defers member
        // resolution to the runtime, so this exercises the iterative builtin
        // dispatch's runtime `E2003` differentially.
        field_value("unknown_on_untyped_none", "{ let x = none\n x.len }\n"),
        field_value("unknown_member_untyped_none", "{ let x = none\n x.foo }\n"),
        // ----- program mode (real frame boundary) -------------------------
        field_program(
            "main_field_let",
            "fn main() { let xs = [1, 2]\n let n = xs.len }\n",
        ),
        field_program(
            "main_field_map",
            "fn main() { let m = {1: 2}\n let n = m.len }\n",
        ),
        field_program("main_field_error", "fn main() { let n = [1].foo }\n"),
    ]
}

/// B-1R3B.6 — index reads whose target or index remains unsupported (calls are
/// R3C, f-strings R3B.7) must fail with the deterministic `E4999` sentinel in
/// iterative mode, never fall back to recursion, and never produce a partial
/// read. The unsupported construct is always on the evaluated path.
#[must_use]
pub fn index_unsupported_cases() -> Vec<Case> {
    vec![
        // An unsupported lambda target/index is not reachable as written (the
        // checker requires an indexable/keyable type), so the reachable shapes
        // are nested under a supported target/index. The original call-shaped
        // probes (`call_base`, `call_index`, `nested_call_base`,
        // `main_index_call`) became supported in B-1R3C.1 and are asserted by
        // `r3c1::supported_cases`.
        index_value("lambda_in_base", "([1] + (() -> 1))[0]\n"),
        // `fstring_index` (`[1, 2][f"v={1}"]`) moved to
        // `r3b::fstring_supported_cases` in B-1R3B.7: the f-string is a
        // supported index now, and the runtime `E3001` non-key-capable index
        // agrees with recursion.
        // Same shape inside a real user frame.
        index_program(
            "main_lambda_in_base",
            "fn main() { let y = ([1] + (() -> 1))[0] }\n",
        ),
    ]
}

/// B-1R3B.6 — field reads whose receiver remains unsupported must fail with the
/// deterministic `E4999` sentinel, never fall back to recursion.
#[must_use]
pub fn field_unsupported_cases() -> Vec<Case> {
    vec![
        // `struct_construct_receiver` became supported in B-1R3C.3 (struct
        // construction landed) and is asserted by `r3c1::supported_cases` as
        // `field_instance_read`; the field unsupported set now probes a lambda
        // receiver only.
        // An unsupported lambda receiver is reached and fails before the member
        // is resolved (the recursive engine reports the receiver's own runtime
        // type error, so this program is valid for it). The original
        // call-receiver probes became supported in B-1R3C.1 and are asserted
        // by `r3c1::supported_cases`.
        field_value("lambda_receiver", "{ (() -> 1) }.len\n"),
        // An unsupported construct nested one level under the receiver.
        field_value("nested_lambda_receiver", "{ (1 + (() -> 1)) }.len\n"),
    ]
}

fn fstring_value(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b7-value",
        name,
        file: "<r3b7>",
        source,
        kind: Kind::Value,
    }
}

fn fstring_program(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3b7-program",
        name,
        file: "r3b7.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

/// B-1R3B.7 — f-string cases the iterative engine supports. Recursive and
/// iterative must agree exactly on value, type, diagnostics, spans, and stdout.
/// The set pins the part model (empty/text-only/literal/interpolation), source
/// order, exactly-once scheduling, the stringification matrix, the format-spec
/// mini-language and its bounds, real skipping inside `and`/`or`, Unicode, raw
/// (undecoded) literal text, and the malformed-syntax boundary that is rejected
/// by the parser before the evaluator ever runs.
#[must_use]
pub fn fstring_supported_cases() -> Vec<Case> {
    vec![
        // ----- shape: empty / text-only / escaped braces -------------------
        fstring_value("empty", "f\"\"\n"),
        fstring_value("text_only", "f\"hello\"\n"),
        fstring_value("text_only_spaces", "f\"a b c\"\n"),
        fstring_value("escaped_open", "f\"{{}}\"\n"),
        fstring_value("escaped_close", "f\"a}}b\"\n"),
        fstring_value("escaped_both", "f\"{{literal}}\"\n"),
        // ----- raw literal text: no escape decoding in an f-string --------
        fstring_value("raw_backslash_n", "f\"a\\nb\"\n"),
        fstring_value("raw_backslash_t", "f\"a\\tb\"\n"),
        fstring_value("raw_trailing_backslash", "f\"a\\\\\"\n"),
        // ----- single interpolation: stringification matrix -----------------
        fstring_value("int", "f\"{1}\"\n"),
        fstring_value("int_negative", "f\"{-5}\"\n"),
        fstring_value("float", "f\"{1.5}\"\n"),
        fstring_value("float_integral", "f\"{1.0}\"\n"),
        fstring_value("bool_true", "f\"{true}\"\n"),
        fstring_value("bool_false", "f\"{false}\"\n"),
        fstring_value("none", "f\"{none}\"\n"),
        // An inner string/interpolation uses the other quote: a double-quoted
        // f-string ends at the first unescaped `"` in its body, so a nested
        // string or nested f-string must be single-quoted.
        fstring_value("string", "f\"{'hi'}\"\n"),
        fstring_value("list", "f\"{[1, 2, 3]}\"\n"),
        fstring_value("tuple_sugar", "f\"{(1, 2)}\"\n"),
        fstring_value("nested_list", "f\"{[[1], [2]]}\"\n"),
        fstring_value("map", "f\"{ {1: 'a', 2: 'b'} }\"\n"),
        fstring_value("range", "f\"{(1..3)}\"\n"),
        // A native and a closure value display as `<fn>` (reachable without a
        // call; this is the `Value::Native`/`Value::Closure` display path).
        fstring_value("native_value", "f\"{len}\"\n"),
        fstring_value("closure_value", "fn f() { }\nf\"{f}\"\n"),
        // ----- interpolation positions ------------------------------------
        fstring_value("at_start", "f\"{1} end\"\n"),
        fstring_value("at_end", "f\"start {1}\"\n"),
        fstring_value("at_middle", "f\"a{1}b\"\n"),
        fstring_value("adjacent", "f\"{1}{2}\"\n"),
        fstring_value("empty_text_boundaries", "f\"{1}{2}{3}\"\n"),
        // ----- multiple interpolations: source order ----------------------
        fstring_value("two", "f\"{1}-{2}\"\n"),
        fstring_value("three", "f\"{1}{2}{3}\"\n"),
        fstring_value("mixed_kinds", "f\"{1}{1.5}{true}{none}{'s'}\"\n"),
        // ----- evaluation order: first error wins --------------------------
        // The first interpolation fails; a later one would also fail.
        fstring_value("first_error_wins", "f\"{1 / 0}{2 / 0}\"\n"),
        // A middle interpolation fails after the first succeeded.
        fstring_value("middle_error", "f\"{1}{2 / 0}{3}\"\n"),
        // A later interpolation fails; the first is fine.
        fstring_value("later_error", "f\"{1}{2}{3 / 0}\"\n"),
        // An earlier expression error versus a later unsupported expression:
        // the E4007 must win over the E4999 (shown by the unsupported set for
        // the converse), proving source order.
        fstring_value("error_before_unsupported", "f\"{1 / 0}{len([1])}\"\n"),
        // ----- format specification: presentation types --------------------
        fstring_value("spec_dec", "f\"{42:d}\"\n"),
        fstring_value("spec_binary", "f\"{10:b}\"\n"),
        fstring_value("spec_octal", "f\"{8:o}\"\n"),
        fstring_value("spec_hex", "f\"{255:x}\"\n"),
        fstring_value("spec_hex_upper", "f\"{255:X}\"\n"),
        fstring_value("spec_fixed", "f\"{3.14159:.2f}\"\n"),
        fstring_value("spec_fixed_upper", "f\"{3.5:F}\"\n"),
        fstring_value("spec_exp", "f\"{1234.0:.2e}\"\n"),
        fstring_value("spec_percent", "f\"{0.5:.1%}\"\n"),
        // `int` coerces to float for `f`.
        fstring_value("spec_int_as_fixed", "f\"{3:f}\"\n"),
        fstring_value("spec_precision_plain", "f\"{3.14159:.3}\"\n"),
        // ----- format specification: sign / width / fill / align -----------
        fstring_value("spec_sign_plus", "f\"{42:+d}\"\n"),
        fstring_value("spec_sign_plus_negative", "f\"{-42:+d}\"\n"),
        fstring_value("spec_sign_space", "f\"{42: d}\"\n"),
        fstring_value("spec_width", "f\"[{7:6}]\"\n"),
        fstring_value("spec_width_left", "f\"[{7:<6}]\"\n"),
        fstring_value("spec_width_right", "f\"[{7:>6}]\"\n"),
        fstring_value("spec_width_center", "f\"[{7:^6}]\"\n"),
        fstring_value("spec_fill_align", "f\"[{7:*>6}]\"\n"),
        fstring_value("spec_zero_pad", "f\"{42:06d}\"\n"),
        fstring_value("spec_string_width", "f\"[{'ab':>5}]\"\n"),
        // ----- format specification: type applied to an incompatible value -
        fstring_value("spec_type_mismatch_bool", "f\"{true:d}\"\n"),
        fstring_value("spec_type_mismatch_float_dec", "f\"{1.5:d}\"\n"),
        fstring_value("spec_type_mismatch_string", "f\"{'s':f}\"\n"),
        fstring_value("spec_type_mismatch_list", "f\"{[1]:x}\"\n"),
        // ----- format specification: bounds (E4013) ------------------------
        fstring_value("spec_precision_over_bound", "f\"{1.0:.70000f}\"\n"),
        fstring_value("spec_width_over_bound", "f\"{1:10000001}\"\n"),
        // ----- composition with arithmetic / unary -------------------------
        fstring_value("expr_add", "f\"{1 + 2}\"\n"),
        fstring_value("expr_unary_neg", "f\"{-5}\"\n"),
        fstring_value("expr_not", "f\"{not false}\"\n"),
        fstring_value("expr_bitnot", "f\"{~0}\"\n"),
        fstring_value("expr_nested_binary", "f\"{(1 + 2) * 3}\"\n"),
        // ----- composition with short-circuit (`and` / `or`) ---------------
        fstring_value("and_expr", "f\"{true and false}\"\n"),
        fstring_value("or_expr", "f\"{false or true}\"\n"),
        // A skipped short-circuit right operand inside an interpolation must
        // not run: the E4007 must never occur.
        fstring_value("skipped_rhs_in_interp", "f\"{true or (1 / 0)}\"\n"),
        fstring_value("skipped_rhs_and_in_interp", "f\"{false and (1 / 0)}\"\n"),
        // Real skipping of a whole f-string operand (moved from the R3B.3
        // supported set to keep the R3B.7 golden self-contained). Short-circuit
        // precedence is weaker than `and`/`or`, so the parse is
        // `false and (f"...")` and the f-string is never reached.
        fstring_value("and_skip", "false and f\"{1}\"\n"),
        fstring_value("or_skip", "true or f\"{1}\"\n"),
        // ----- composition with List / current-Tuple -----------------------
        fstring_value("list_element", "[f\"v={1}\"]\n"),
        fstring_value("list_element_after_int", "[1, f\"v={2}\"]\n"),
        fstring_value("nested_list_fstring", "[[f\"{1}\"]]\n"),
        // The element's `int + string` is a real E3001 both engines agree on
        // (moved from the R3B.4.1 unsupported set).
        fstring_value("list_element_type_error", "[1 + f\"v={1}\"]\n"),
        // ----- composition with Map ---------------------------------------
        fstring_value("map_value", "{1: f\"v={1}\"}\n"),
        fstring_value("map_key_string", "{f\"k{1}\": 2}\n"),
        fstring_value("map_value_nested", "{1: {2: f\"{3}\"}}\n"),
        // ----- composition with Range -------------------------------------
        fstring_value("range_end", "1..f\"3\"\n"),
        // ----- composition with Index / Field (R3B.6 boundary) -------------
        fstring_value("index_string_target", "f\"abc\"[0]\n"),
        fstring_value("index_fstring_index", "[1, 2][f\"{1}\"]\n"),
        fstring_value("field_len_of_fstring", "f\"abcd\".len\n"),
        fstring_value("fstring_in_index_base", "[f\"{1}\", 2][0]\n"),
        // ----- composition with block / let / if ---------------------------
        // A `throw` from an interpolation aborts the f-string; the value path
        // maps the residual signal exactly like recursion (`E4026`).
        fstring_value("throw_interp", "f\"{ {throw 5} }\"\n"),
        // Two f-strings as the operands of an eager binary: each is evaluated
        // exactly once, in source order.
        fstring_value("two_fstrings_binary", "f\"{1}\" + f\"{2}\"\n"),
        // An f-string used as an index target (the target is the `string`).
        fstring_value("fstring_index_target", "f\"{1}\"[0]\n"),
        fstring_value("in_block", "{ f\"{1}\" }\n"),
        fstring_value("let_binding", "{ let x = f\"v={1}\"\n x }\n"),
        fstring_value("let_shadow_in_interp", "{ let x = 2\n f\"{x}\" }\n"),
        fstring_value("if_taken", "if true { f\"{1}\" } else { f\"{2}\" }\n"),
        fstring_value("if_condition", "if f\"\" { 1 } else { 2 }\n"),
        // ----- Unicode ----------------------------------------------------
        fstring_value("unicode_accent", "f\"caf\u{e9} {1}\"\n"),
        fstring_value("unicode_cjk", "f\"\u{65e5}\u{672c} {1}\"\n"),
        fstring_value("unicode_emoji", "f\"\u{1f389} {1}\"\n"),
        fstring_value("unicode_combining", "f\"e\u{301} {1}\"\n"),
        fstring_value("unicode_interpolated_string", "f\"{'caf\u{e9}'}\"\n"),
        // Outer single quotes so the inner string uses the other delimiter; the
        // CJK string is width-padded by character count.
        fstring_value("unicode_width", "f'[{\"\u{65e5}\":^5}]'\n"),
        // ----- nested f-string --------------------------------------------
        // The inner f-string is single-quoted because the outer one uses `"`.
        // Two levels of nesting are not expressible: the innermost level would
        // need the same delimiter as the outer one, which terminates it, so the
        // grammar admits at most one nested f-string level.
        fstring_value("nested_fstring", "f\"a{f'{1}'}b\"\n"),
        fstring_value("nested_fstring_around_expr", "f\"{f'{1 + 1}'}\"\n"),
        fstring_value("nested_fstring_outer_inner_text", "f\"[{f'x={2}'}]\"\n"),
        // ----- program mode (real frame boundary) --------------------------
        fstring_program("main_fstring", "fn main() { let s = f\"v={1}\" }\n"),
        fstring_program(
            "main_fstring_multi",
            "fn main() { let s = f\"{1}{2}{3}\" }\n",
        ),
        fstring_program(
            "main_fstring_format",
            "fn main() { let s = f\"{42:06d}\" }\n",
        ),
        fstring_program("main_fstring_error", "fn main() { let s = f\"{1 / 0}\" }\n"),
        fstring_program(
            "main_fstring_signal",
            "fn main() { let s = f\"{ {return} }\" }\n",
        ),
        fstring_program(
            "main_fstring_in_list",
            "fn main() { let xs = [f\"{1}\", f\"{2}\"] }\n",
        ),
        fstring_program(
            "main_fstring_in_map",
            "fn main() { let m = {1: f\"{2}\"} }\n",
        ),
        fstring_program(
            "main_fstring_unicode",
            "fn main() { let s = f\"caf\u{e9} {1}\" }\n",
        ),
        fstring_program(
            "main_fstring_nested",
            "fn main() { let s = f\"{f'{1}'}\" }\n",
        ),
    ]
}

/// B-1R3B.7 — f-string interpolations that remain unsupported (calls are R3C)
/// must fail with the deterministic `E4999` sentinel in iterative mode, never
/// fall back to recursion, and never produce a partial string. A *skipped*
/// f-string (short-circuit) is a supported case above; every f-string here is
/// on the evaluated path.
#[must_use]
pub fn fstring_unsupported_cases() -> Vec<Case> {
    vec![
        // An unsupported lambda interpolation is reached and fails. The
        // original call-interpolation probes (`call_interp`,
        // `nested_call_interp`, `later_call_interp`, `list_call_interp`,
        // `spec_call_interp`, `main_fstring_call`, `main_fstring_later_call`)
        // became supported in B-1R3C.1 and are asserted by
        // `r3c1::supported_cases`.
        fstring_value("lambda_interp", "f\"{(() -> 1)}\"\n"),
        // An unsupported lambda nested one level under a supported expression.
        fstring_value("nested_lambda_interp", "f\"{1 + (() -> 1)}\"\n"),
        // A supported first interpolation must not let the machine fall back on
        // a later unsupported one.
        fstring_value("later_lambda_interp", "f\"{1}{(() -> 1)}\"\n"),
        // An unsupported construct in a format-spec interpolation.
        fstring_value("spec_lambda_interp", "f\"{(() -> 1):d}\"\n"),
        // Same shape inside a real user frame.
        fstring_program(
            "main_fstring_lambda",
            "fn main() { let s = f\"{(() -> 1)}\" }\n",
        ),
    ]
}
