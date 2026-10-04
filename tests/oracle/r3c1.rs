#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! B-1R3C.1 — the iterative-engine oracle subset for user and native calls.
//!
//! These cases exercise **only** `Expr::Call` layered on the constructs the
//! explicit-continuation machine already supports (R3A, R3B.1–R3B.7). Every
//! case runs through the real `Engine::iterative` path in the differential
//! test in `tests/evaluator_oracle.rs`; none may fall back to recursion.
//!
//! Calls are the first construct that makes **side effects observable** through
//! `print`, so this set also *consumes the B-1R3B deferred-strengthening
//! ledger*: exactly-once and left-to-right evaluation for unary, eager binary,
//! short-circuit, list/tuple, map, range, index/field, and f-string operands are
//! pinned by stdout sequences, not merely structurally.
//!
//! The unsupported set asserts the anti-fallback guard for constructs that
//! remain unsupported *inside* a call (a lambda argument, a comprehension
//! argument, a construct receiver).

use crate::harness::{Case, Kind};

fn value(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3c1-value",
        name,
        file: "<r3c1>",
        source,
        kind: Kind::Value,
    }
}

fn program(name: &'static str, source: &'static str) -> Case {
    Case {
        group: "r3c1-program",
        name,
        file: "r3c1.aura",
        source,
        kind: Kind::ExecuteProgram,
    }
}

/// Calls the iterative engine supports, including exact agreement on stdout,
/// diagnostics, spans, values, and control outcomes.
///
/// Every argument order/once case uses `print` because `print` returns `none`,
/// so a printed trace is the only observable witness of evaluation order and
/// exactly-once.
#[must_use]
pub fn supported_cases() -> Vec<Case> {
    vec![
        // ----- native calls: arity and arguments -------------------------
        value("native_zero_args", "abs(-3)\n"),
        value("native_one_arg", "len([1, 2, 3])\n"),
        value("native_two_args", "min(3, 5)\n"),
        value("native_arity_too_few", "len()\n"),
        value("native_arity_too_many", "len([1], [2])\n"),
        value("native_undefined", "no_such_fn(1)\n"),
        value("native_unknown_as_value", "let f = no_such_fn\n"),
        // ----- user function calls ---------------------------------------
        value("user_zero_args", "fn f() { 7 }\nf()\n"),
        value("user_one_arg", "fn f(x) { x + 1 }\nf(41)\n"),
        value("user_two_args", "fn f(a, b) { a * b }\nf(6, 7)\n"),
        value("user_arg_count_low", "fn f(a, b) { a }\nf(1)\n"),
        value("user_arg_count_high", "fn f(a) { a }\nf(1, 2)\n"),
        value("user_forward_reference", "fn g() { f() }\nfn f() { 3 }\ng()\n"),
        value("user_named_argument", "fn f(a, b) { a - b }\nf(b: 1, a: 5)\n"),
        value("user_named_unknown", "fn f(a) { a }\nf(c: 1)\n"),
        value("user_named_duplicate", "fn f(a) { a }\nf(1, a: 2)\n"),
        value("user_return", "fn f() { return 9 }\nf()\n"),
        value("user_implicit_none", "fn f() { }\nf()\n"),
        // ----- closures as values and first-class callees ----------------
        value("closure_value_call", "fn f(x) { x }\nlet g = f\ng(4)\n"),
        value("native_as_value_call", "let f = len\nf([1, 2])\n"),
        value("callee_expression", "fn f() { 1 }\n(true and f)()\n"),
        value("callee_result_non_callable", "let x = 3\nx()\n"),
        value("call_inside_array_index_expr", "[10, 20][min(1, 1)]\n"),
        // ----- diagnostics from inside the callee ------------------------
        value("user_overflow", "fn f() { 9223372036854775807 + 1 }\nf()\n"),
        value("user_undefined_name", "fn f() { missing }\nf()\n"),
        value("user_divide_by_zero", "fn f(a) { a / 0 }\nf(1)\n"),
        // ----- signals through a call boundary ---------------------------
        value("user_throw", "fn f() { throw 5 }\nf()\n"),
        value("user_throw_value", "fn f() { throw \"boom\" }\nf()\n"),
        value("user_return_ignored_after", "fn f() { return 1\n 2 }\nf()\n"),
        // ----- historically conserved cases (migrated from earlier phases'
        // unsupported sets, same programs and value/program modes) ----------
        // From R3A (`call`), R3B unary, binary, and short-circuit sentinels.
        value("call", "len([1, 2])\n"),
        value("pipe_native_call", "[1, 2] |> len\n"),
        value("neg_of_unsupported_call", "-len([1, 2])\n"),
        value("not_of_unsupported_call", "not len([1, 2])\n"),
        value("eager_over_call", "1 + len([1, 2])\n"),
        value("and_required_call", "true and len([1, 2])\n"),
        value("or_required_call", "false or len([1, 2])\n"),
        value("nested_required_call", "false or (true and len([1, 2]))\n"),
        program(
            "main_required_call",
            "fn main() { let x = true and len([1, 2]) }\n",
        ),
        // From the list unsupported set.
        value("call_element", "[1, len([1, 2])]\n"),
        program(
            "main_call_element",
            "fn main() { let xs = [1, len([1, 2])] }\n",
        ),
        // From the map unsupported set.
        value("call_key", "{len([1, 2]): 1}\n"),
        value("call_value", "{1: len([1, 2])}\n"),
        value("later_call_value", "{1: 2, 2: len([1, 2])}\n"),
        program(
            "main_call_value",
            "fn main() { let m = {1: len([1, 2])} }\n",
        ),
        program("main_call_key", "fn main() { let m = {len([1, 2]): 1} }\n"),
        // From the range unsupported set.
        value("call_start", "len([1, 2])..3\n"),
        value("call_end", "1..len([1, 2])\n"),
        value("call_end_after_int", "0..len([1, 2])\n"),
        value("nested_call_start", "(1 + len([1, 2]))..3\n"),
        program("main_range_call", "fn main() { let r = 0..len([1, 2]) }\n"),
        // From the index unsupported set.
        value("call_base", "len([1, 2])[0]\n"),
        value("call_index", "[1, 2][len([1])]\n"),
        value("nested_call_base", "([1] + len([2]))[0]\n"),
        program("main_index_call", "fn main() { let y = len([1])[0] }\n"),
        // From the field unsupported set.
        value("call_receiver", "{ len([1, 2]) }.len\n"),
        value("nested_call_receiver", "{ (1 + len([2])) }.len\n"),
        // From the f-string unsupported set.
        value("call_interp", "f\"{len([1, 2])}\"\n"),
        value("nested_call_interp", "f\"{1 + len([1, 2])}\"\n"),
        value("later_call_interp", "f\"{1}{len([1])}\"\n"),
        value("list_call_interp", "f\"{[len([1])]}\"\n"),
        value("spec_call_interp", "f\"{len([1]):d}\"\n"),
        program(
            "main_fstring_call",
            "fn main() { let s = f\"{len([1])}\" }\n",
        ),
        program(
            "main_fstring_later_call",
            "fn main() { let s = f\"{1}{len([1])}\" }\n",
        ),
        // ----- higher-order callbacks through the resumable protocol -----
        // `map`/`filter`/`reduce` invoke Aura callbacks; the machine drives
        // them as machine work (no recursive fallback, no nested host frame
        // per element). Named functions are closure values, so these cases
        // need no lambda syntax (R3C.4).
        value("map_named_callback", "fn d(x) { x * 2 }\nmap([1, 2, 3], d)\n"),
        value("map_native_callback", "map([-1, -2], abs)\n"),
        value("filter_named_callback", "fn odd(x) { x % 2 == 1 }\nfilter([1, 2, 3, 4], odd)\n"),
        value("reduce_named_callback", "fn add(a, b) { a + b }\nreduce([1, 2, 3, 4], add, 0)\n"),
        value("map_callback_order", "fn d(x) { x * 2 }\nmap([print(\'a\'), 1], d)\n"),
        value("higher_order_apply", "fn twice(f, x) { f(f(x)) }\nfn inc(n) { n + 1 }\ntwice(inc, 1)\n"),
        value(
            "callback_throw_propagates",
            "fn boom(x) { throw x }\nmap([1], boom)\n",
        ),
        value(
            "reduce_callback_error",
            "fn div(a, b) { a / b }\nreduce([1, 0], div, 8)\n",
        ),
        // ----- composition with R3B constructs ---------------------------
        value("call_in_unary", "-(abs(-2))\n"),
        value("call_in_binary", "1 + len([9, 9])\n"),
        value("call_in_if", "if len([1]) { \"y\" } else { \"n\" }\n"),
        value("call_in_list", "[len([1]), len([1, 2])]\n"),
        value("call_in_map_key", "{len([1]): \"a\"}\n"),
        value("call_in_map_value", "{1: len([1, 2])}\n"),
        value("call_in_range", "0..len([1, 2])\n"),
        value("call_in_index_base", "[1, 2][len([1]) - 1]\n"),
        value("call_in_fstring", "f\"v={len([1])}\"\n"),
        value("call_in_fstring_spec", "f\"{len([1, 2]):d}\"\n"),
        value("call_in_let", "{ let x = len([1])\n x }\n"),
        value("call_in_block_last", "{ abs(-5) }\n"),
        // ----- nested and recursive calls --------------------------------
        value("call_nested", "fn f(x) { x + 1 }\nf(f(f(0)))\n"),
        value("call_recursive_factorial", "fn f(n) { if n { n * f(n - 1) } else { 1 } }\nf(5)\n"),
        value("call_mutual_recursion", "fn even(n) { if n { odd(n - 1) } else { true } }\nfn odd(n) { if n { even(n - 1) } else { false } }\neven(10)\n"),
        // ----- short-circuit skipping a whole call -----------------------
        value("call_skipped_and", "false and len([1, 2])\n"),
        value("call_skipped_or", "true or len([1, 2])\n"),
        // ----- side effects: exactly-once and order (R3B ledger) ---------
        value("print_side_effect", "print(\"a\")\n"),
        value("arg_order_two", "fn f(a, b) { 0 }\nf(print(\"a\"), print(\"b\"))\n"),
        value("arg_order_three", "fn f(a, b, c) { 0 }\nf(print(\"1\"), print(\"2\"), print(\"3\"))\n"),
        value("arg_evaluated_once", "fn f(a) { 0 }\nlet n = [0]\nf(print(\"x\"))\n"),
        value("binary_order_over_call", "print(\"l\") + 0\n"),
        value("binary_order_left_then_right", "fn f(a, b) { 0 }\nf(1 + print(\"r\") - 1, 2)\n"),
        value("list_elements_once", "[print(\"a\"), print(\"b\"), print(\"c\")]\n"),
        value("map_key_before_value", "{print(\"k\"): print(\"v\")}\n"),
        value("range_bounds_order", "print(\"s\")..print(\"e\")\n"),
        value("index_target_then_index", "[print(\"t\")][print(\"i\")]\n"),
        value(
            "fstring_interp_order",
            "f\"{print(\'1\')}{print(\'2\')}\"\n",
        ),
        value("unary_operand_once", "-print(\"u\")\n"),
        value("short_circuit_skips_print", "false and print(\"skipped\")\n"),
        value("short_circuit_prints_when_required", "true and print(\"done\")\n"),
        // ----- call depth boundary ---------------------------------------
        // Program mode so both engines run on the production execution stack
        // (the recursive engine's 512-frame chain needs the 64 MiB substrate;
        // the oracle value path would otherwise exhaust the test thread).
        program(
            "call_depth_509_ok",
            "fn main() { f(507) }\nfn f(n) { if n { f(n - 1) } else { 0 } }\n",
        ),
        program(
            "call_depth_510_ok",
            "fn main() { f(508) }\nfn f(n) { if n { f(n - 1) } else { 0 } }\n",
        ),
        program(
            "call_depth_511_ok",
            "fn main() { f(509) }\nfn f(n) { if n { f(n - 1) } else { 0 } }\n",
        ),
        program(
            "call_depth_512_e4011",
            "fn main() { f(510) }\nfn f(n) { if n { f(n - 1) } else { 0 } }\n",
        ),
        program(
            "call_depth_513_e4011",
            "fn main() { f(511) }\nfn f(n) { if n { f(n - 1) } else { 0 } }\n",
        ),
        program(
            "call_deep_1000_e4011",
            "fn main() { f(1000) }\nfn f(n) { if n { f(n - 1) } else { 0 } }\n",
        ),
        // ----- program (real frame boundary) -----------------------------
        program("main_native_call", "fn main() { let x = len([1, 2]) }\n"),
        program("main_user_call", "fn main() { let x = f() }\nfn f() { 3 }\n"),
        program("main_nested_call", "fn main() { let x = f(g()) }\nfn f(a) { a }\nfn g() { 1 }\n"),
        program("main_recursive", "fn main() { let x = f(4) }\nfn f(n) { if n { n + f(n - 1) } else { 0 } }\n"),
        program("main_throw", "fn main() { f() }\nfn f() { throw 1 }\n"),
        program("main_call_after_return", "fn main() { return\n f() }\nfn f() { print(\"unreached\") }\n"),
        program("main_print_order", "fn main() { print(\"one\")\n print(\"two\") }\n"),
    ]
}

/// Calls whose argument, callee, or nested expression is a construct that is
/// still unsupported (`lambda`, comprehensions, `construct`, ...) must fail
/// with the deterministic `E4999` sentinel. In particular the machine must not
/// apply the call after the failing subexpression, and must never fall back to
/// recursion.
#[must_use]
pub fn unsupported_cases() -> Vec<Case> {
    vec![
        // A lambda argument: the lambda itself is R3C.4 work.
        value("lambda_argument", "fn f(x) { 0 }\nf(() -> 1)\n"),
        // A comprehension argument.
        value("listcomp_argument", "fn f(x) { 0 }\nf([x for x in [1]])\n"),
        // A construct argument.
        value(
            "construct_argument",
            "struct P { x: int }\nfn f(x) { 0 }\nf(P { x: 1 })\n",
        ),
        // A lambda callee.
        value("lambda_callee", "(() -> 1)()\n"),
        // A construct receiver (method call on a struct literal).
        value(
            "construct_method_receiver",
            "struct P { x: int }\nimpl P { fn get(self) { self.x } }\nP { x: 1 }.get()\n",
        ),
        // Same shapes inside a real user frame.
        program(
            "main_lambda_argument",
            "fn main() { let x = f(() -> 1) }\nfn f(x) { 0 }\n",
        ),
        program(
            "main_construct_argument",
            "struct P { x: int }\nfn main() { let x = f(P { x: 1 }) }\nfn f(x) { 0 }\n",
        ),
    ]
}
