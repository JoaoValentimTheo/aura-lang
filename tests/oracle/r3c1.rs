#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! B-1R3C.1/R3C.3 — the iterative-engine oracle subset for user and native
//! calls and for struct/enum construction (`Expr::Construct`).
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
        // ----- R3C.3 construct: struct and enum construction ------------
        value("construct_positional", "struct P { x: int }\nP { x: 1 }\n"),
        value(
            "construct_named",
            "struct P { x: int, y: int }\nP { x: 1, y: 2 }\n",
        ),
        value("construct_zero_fields", "struct U {}\nU {}\n"),
        value("construct_wrong_count", "struct P { x: int }\nP { x: 1, y: 2 }\n"),
        value("construct_missing_field", "struct P { x: int, y: int }\nP { x: 1 }\n"),
        value(
            "construct_unknown_field",
            "struct P { x: int }\nP { z: 1 }\n",
        ),
        value("construct_duplicate_field", "struct P { x: int }\nP { x: 1, x: 2 }\n"),
        value(
            "construct_mixed_forms",
            "struct P { x: int, y: int }\nP { x: 1, 2 }\n",
        ),
        value("construct_unknown_type", "Nope { x: 1 }\n"),
        value(
            "construct_used_as_call_argument",
            "struct P { x: int }\nfn f(x) { 0 }\nf(P { x: 1 })\n",
        ),
        program(
            "main_construct_argument",
            "struct P { x: int }\nfn main() { let x = f(P { x: 1 }) }\nfn f(x) { 0 }\n",
        ),
        value("construct_enum_variant", "enum E { A(int), B }\nA(5)\n"),
        value("construct_enum_nullary", "enum E { A(int), B }\nB\n"),
        value("construct_variant_arity", "enum E { A(int) }\nA(1, 2)\n"),
        value("construct_arg_order", "struct P { x: int }\nP { x: print(\'c\') }\n"),
        value(
            "construct_arg_error_first",
            "struct P { x: int, y: int }\nP { x: 1 / 0, y: print(\'z\') }\n",
        ),
        // ----- R3C.3 field reads (instance arm was unreachable before) ---
        value("field_instance_read", "struct P { x: int }\nP { x: 1 }.x\n"),
        value(
            "field_instance_read_named",
            "struct P { x: int, y: int }\nP { x: 1, y: 2 }.y\n",
        ),
        value(
            "field_instance_missing",
            "struct P { x: int }\nP { x: 1 }.z\n",
        ),
        value(
            "field_method_not_value",
            "struct P { x: int }\nimpl P { fn get(self) { self.x } }\nP { x: 1 }.get\n",
        ),
        value(
            "field_instance_read_after_new",
            "struct P { x: int }\n{ let a = P { x: 1 }\n let b = P { x: 2 }\n a.x + b.x }\n",
        ),

        // ----- R3C.2 methods -------------------------------------------------
        value("method_instance_call", "struct P { x: int }\nimpl P { fn get(self) { self.x } }\nP { x: 7 }.get()\n"),
        value("method_builtin_len", "[1, 2, 3].len()\n"),
        value("method_builtin_upper", "\'abc\'.upper()\n"),
        value("method_call_migrated", "[1, 2].len()\n"),
        value("method_builtin_map_call", "fn d(x) { x * 2 }\n[1, 2, 3].map(d)\n"),
        value("method_builtin_filter_call", "fn odd(x) { x % 2 == 1 }\n[1, 2, 3, 4].filter(odd)\n"),
        value("method_builtin_reduce_call", "fn add(a, b) { a + b }\n[1, 2, 3].reduce(add, 0)\n"),
        value("method_unknown", "[1].no_such_method()\n"),
        value("method_struct_unknown", "struct P { x: int }\nimpl P { fn get(self) { self.x } }\nP { x: 1 }.nope()\n"),
        value("method_arg_count", "struct P { x: int }\nimpl P { fn add(self, n) { self.x + n } }\nP { x: 1 }.add()\n"),
        value("method_arg_order", "struct P { x: int }\nimpl P { fn f(self, a, b) { a - b } }\nP { x: 0 }.f(print(\'1\'), print(\'2\'))\n"),
        value("method_arity_mismatch", "\'a\'.replace()\n"),
        // ----- R3C.4 lambdas and pipes ---------------------------------------
        value("lambda_expression_body", "(() -> 1)()\n"),
        value("lambda_called_direct", "(() -> 21 * 2)()\n"),
        value("lambda_block_body", "(() -> { let x = 3\n x + 1 })()\n"),
        value("lambda_return_inside", "(() -> { return 9\n 0 })()\n"),
        value("lambda_param", "((x) -> x + 1)(41)\n"),
        value("lambda_two_params", "((a, b) -> a * b)(6, 7)\n"),
        value("lambda_capture", "{ let n = 5\n ((x) -> x + n)(1) }\n"),
        value("lambda_capture_after", "{ let n = 5\n let f = (x) -> x + n\n f(1) }\n"),
        value("lambda_as_display", "(() -> 1)\n"),
        value("lambda_call_as_argument", "fn apply(f, x) { f(x) }\napply((y) -> y + 1, 4)\n"),
        value("lambda_map", "map([1, 2, 3], (x) -> x * 2)\n"),
        value("lambda_filter", "filter([1, 2, 3, 4], (x) -> x % 2 == 0)\n"),
        value("lambda_reduce", "reduce([1, 2, 3], (a, b) -> a + b, 0)\n"),
        value("lambda_throw", "((() -> throw 3))()\n"),
        value(
            "lambda_arg_order",
            "((a, b) -> a)(print(\'l\'), print(\'r\'))\n",
        ),
        value("pipe_native", "[3, 1, 2] |> len\n"),
        value("pipe_named_fn", "fn d(xs) { len(xs) }\n[1, 2] |> d\n"),
        value("pipe_lambda", "[1, 2, 3] |> ((xs) -> len(xs))\n"),
        value("pipe_lambda_map", "[1, 2] |> ((xs) -> len(xs))\n"),
        value("pipe_chained", "[1, 2, 3] |> len |> ((n) -> n + 1)\n"),
        value("pipe_left_once", "fn d(x) { 0 }\nprint(\'p\') |> d\n"),
        value("pipe_left_error_first", "1 / 0 |> ((x) -> x)\n"),
        value("pipe_right_error", "1 |> ((x) -> x / 0)\n"),
        value("pipe_non_callable", "1 |> 2\n"),
        // ----- R3D.1 assignment and destructuring ----------------------------
        value("assign_simple", "{ let mut x = 1\n x = 2\n x }\n"),
        value("assign_immutable", "{ let x = 1\n x = 2\n x }\n"),
        value("assign_undefined", "{ missing = 1 }\n"),
        value("assign_compound_add", "{ let mut x = 1\n x += 2\n x }\n"),
        value("assign_compound_mul", "{ let mut x = 3\n x *= 4\n x }\n"),
        value("assign_compound_sub", "{ let mut x = 3\n x -= 4\n x }\n"),
        value("assign_compound_div", "{ let mut x = 8\n x /= 2\n x }\n"),
        value("assign_compound_rem", "{ let mut x = 7\n x %= 4\n x }\n"),
        value("assign_index_simple", "{ let mut xs = [1]\n xs[0] = 9\n xs }\n"),
        value(
            "assign_index_compound",
            "{ let mut xs = [1]\n xs[0] += 9\n xs }\n",
        ),
        value(
            "assign_field_simple",
            "struct P { x: int }\n{ let mut p = P { x: 1 }\n p.x = 5\n p.x }\n",
        ),
        value(
            "assign_invalid_target",
            "{ let mut x = 1\n (x + 1) = 2 }\n",
        ),
        value(
            "assign_rhs_signal",
            "{ let mut x = 1\n x = { throw 3 } }\n",
        ),
        // Compound assignment deliberately evaluates the target's index
        // expression twice (read then write), matching the recursive engine's
        // documented behavior.
        value(
            "assign_compound_double_index",
            "fn idx() { print(\'i\')\n 0 }\n{ let mut xs = [1]\n xs[idx()] += 10\n xs }\n",
        ),
        value(
            "assign_compound_single_index_name",
            "fn d() { print(\'d\') }\n{ let mut x = 1\n x += { d()\n 2 }\n x }\n",
        ),
        value("let_pattern_list", "{ let [a, b] = [1, 2]\n a + b }\n"),
        value("let_pattern_nested", "{ let [a, [b]] = [1, [2]]\n a + b }\n"),
        value("let_pattern_wildcard", "{ let [_, b] = [1, 2]\n b }\n"),
        value("let_pattern_arity_mismatch", "{ let [a, b] = [1]\n a }\n"),
        value("let_pattern_type_mismatch", "{ let [a] = 1\n a }\n"),
        value(
            "let_pattern_variant",
            "enum E { A(int, int) }\n{ let A(a, b) = A(1, 2)\n a + b }\n",
        ),
        value(
            "let_pattern_variant_mismatch",
            "enum E { A(int), B }\n{ let A(a) = B\n 0 }\n",
        ),
        value(
            "let_pattern_shadow",
            "{ let a = 1\n let [a] = [2]\n a }\n",
        ),
        value(
            "let_pattern_rhs_signal",
            "{ let [a] = { throw 1 }\n a }\n",
        ),
        value(
            "let_pattern_atomic",
            "{ let mut a = 1\n let [a, b] = [2]\n a }\n",
        ),
        value(
            "assign_then_read",
            "{ let mut x = 1\n x = x + 1\n x = x * 3\n x }\n",
        ),
        // ----- R3D.2 while / loop --------------------------------------------
        value("while_zero_iterations", "{ while false { print(\'x\') }\n 1 }\n"),
        value("while_counts", "{ let mut i = 0\n while i < 3 { i = i + 1 }\n i }\n"),
        value("while_break", "{ let mut i = 0\n while true { i = i + 1\n if i == 2 { break } }\n i }\n"),
        value("while_continue", "{ let mut i = 0\n let mut s = 0\n while i < 4 { i = i + 1\n if i == 2 { continue }\n s = s + i }\n s }\n"),
        value("while_nested", "{ let mut n = 0\n let mut i = 0\n while i < 2 { let mut j = 0\n while j < 2 { n = n + 1\n j = j + 1 }\n i = i + 1 }\n n }\n"),
        value("while_shadow_scope", "{ let x = 1\n let mut i = 0\n while i < 1 { let x = 2\n i = i + 1 }\n x }\n"),
        value("while_return_propagates", "fn f() { while true { return 5 } }\nf()\n"),
        value("while_throw_propagates", "fn f() { while true { throw 7 } }\nf()\n"),
        value("while_condition_error", "{ while 1 / 0 { } }\n"),
        value("loop_zero", "{ loop { break }\n 1 }\n"),
        value("loop_counts", "{ let mut i = 0\n loop { i = i + 1\n if i == 3 { break } }\n i }\n"),
        value("loop_continue", "{ let mut i = 0\n let mut n = 0\n loop { i = i + 1\n if i > 3 { break }\n if i == 2 { continue }\n n = n + i }\n n }\n"),
        value("loop_return", "fn f() { loop { return 3 } }\nf()\n"),
        value("loop_print_trace", "{ let mut i = 0\n while i < 2 { print(\'w\')\n i = i + 1 } }\n"),
        // Constant continuation growth: a long loop must not accumulate
        // continuations or host stack. 100k iterations exercise the
        // scheduling without a memory/stack blow-up.
        value(
            "while_large_iteration_count",
            "{ let mut i = 0\n let mut s = 0\n while i < 100000 { s = s + 1\n i = i + 1 }\n s }\n",
        ),
        value(
            "loop_large_iteration_count",
            "{ let mut i = 0\n loop { i = i + 1\n if i == 100000 { break } }\n i }\n",
        ),
        // ----- R3D.4 for loops -----------------------------------------------
        value("for_range", "{ let mut s = 0\n for i in 0..4 { s = s + i }\n s }\n"),
        value("for_list", "{ let mut s = 0\n for x in [1, 2, 3] { s = s + x }\n s }\n"),
        value("for_string", "{ let mut s = \'\'\n for c in \'ab\' { s = s + c }\n s }\n"),
        value("for_map_keys", "{ let mut s = 0\n for k in {1: \'a\', 2: \'b\'} { s = s + k }\n s }\n"),
        value("for_zero_items", "{ let mut n = 0\n for x in [] { n = n + 1 }\n n }\n"),
        value("for_break", "{ let mut n = 0\n for x in 0..10 { n = n + 1\n if n == 2 { break } }\n n }\n"),
        value("for_continue", "{ let mut s = 0\n for x in 0..4 { if x == 2 { continue }\n s = s + x }\n s }\n"),
        value("for_nested", "{ let mut n = 0\n for x in 0..2 { for y in 0..2 { n = n + 1 } }\n n }\n"),
        value("for_pattern_list", "{ let mut s = 0\n for [a, b] in [[1, 2], [3, 4]] { s = s + a + b }\n s }\n"),
        value("for_pattern_wildcard", "{ let mut s = 0\n for [_, b] in [[1, 2], [3, 4]] { s = s + b }\n s }\n"),
        value("for_scope_not_leak", "{ for x in [1] { }\n x }\n"),
        value("for_range_lazy_break", "{ let mut n = 0\n for i in 0..10000000 { n = n + 1\n if n == 3 { break } }\n n }\n"),
        value("for_return", "fn f() { for x in 0..3 { return x } }\nf()\n"),
        value("for_throw", "fn f() { for x in 0..3 { throw x } }\nf()\n"),
        value("for_iterable_error", "{ for x in 1 / 0 { } }\n"),
        value("for_not_iterable", "{ for x in 1 { } }\n"),
        value("for_mutation_accumulator", "{ let mut acc = []\n for x in 0..3 { acc = acc + [x] }\n acc }\n"),
        value("for_range_cap_break_early", "{ for i in 0..20000000 { break }\n 1 }\n"),
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
        // A comprehension argument (R3E.1) remains unsupported; lambdas
        // (R3C.4) and constructors (R3C.3) are supported now, so their former
        // probes moved to the supported set.
        value("listcomp_argument", "fn f(x) { 0 }\nf([x for x in [1]])\n"),
        // A `match` argument (R3E.2) remains unsupported.
        value(
            "match_argument",
            "fn f(x) { 0 }\nf(match 1 { 1 -> { 2 } })\n",
        ),
        // Same shapes inside a real user frame.
        program(
            "main_listcomp_argument",
            "fn main() { let x = f([y for y in [1]]) }\nfn f(x) { 0 }\n",
        ),
    ]
}
