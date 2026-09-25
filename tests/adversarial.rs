#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Adversarial regression tests.
//!
//! Each test here corresponds to a real defect found during the hardening
//! audit. They are written to generalise the original finding, not merely
//! replay the exact input that first exposed it.

use aura::check::Checker;
use aura::error::codes;
use aura::run_source;

fn out(src: &str) -> String {
    run_source(src, "<adv>").expect("program runs")
}

fn code(src: &str) -> u16 {
    run_source(src, "<adv>").expect_err("must be rejected").code
}

fn check_code(src: &str) -> u16 {
    let module = aura::parse::parse(src).expect("parses");
    Checker::module(&module).map_or_else(|d| d.code, |()| 0)
}

// ---------------------------------------------------- P0: host crashes

#[test]
fn huge_flat_expression_is_bounded_in_every_phase() {
    let src = format!("fn main() {{ print(1{}) }}", "+1".repeat(200_000));
    // Parser, checker, and evaluator must all terminate with a diagnostic.
    assert!(matches!(
        code(&src),
        codes::NESTING | codes::EXPECTED | codes::OVERFLOW
    ));
}

#[test]
fn checker_bounds_depth_on_the_calling_thread() {
    // Deep expressions are rejected at parse time, before any phase can walk
    // a deep tree on a small stack.
    let src = format!("fn main() {{ print(1{}) }}", "+1".repeat(20_000));
    let err = aura::parse::parse(&src).expect_err("rejected before checking");
    assert_eq!(err.code, codes::NESTING);
}

// ---------------------------------------------------- P1: execution semantics

#[test]
fn block_lambda_supports_return_and_implicit_value() {
    assert_eq!(
        out("fn main() { let f = (x) -> { return x * 2 }\n print(f(21)) }"),
        "42\n"
    );
    assert_eq!(
        out("fn main() { let f = (x) -> { x * 2 }\n print(f(21)) }"),
        "42\n"
    );
    assert_eq!(
        out("fn main() { let f = () -> { let g = () -> { return 7 }\n return g() }\n print(f()) }"),
        "7\n"
    );
}

#[test]
fn return_through_match_and_if_propagates() {
    assert_eq!(
        out("fn f(x) -> int { return match x { 0 -> { return 5 }\n _ -> 2 } }\nfn main() { print(f(0)) }"),
        "5\n"
    );
    assert_eq!(
        out("fn f(b) -> int { return if b { 10 } else { 20 } }\nfn main() { print(f(true)) }"),
        "10\n"
    );
}

#[test]
fn nan_ordering_is_false_not_an_error() {
    // NaN is reachable through explicit float parsing.
    assert_eq!(
        out("fn main() { print(to_float(\"nan\") < 1.0) }"),
        "false\n"
    );
    assert_eq!(
        out("fn main() { print(to_float(\"nan\") == to_float(\"nan\")) }"),
        "false\n"
    );
}

#[test]
fn pipeline_passes_left_operand_as_first_argument() {
    assert_eq!(
        out("fn main() { print([1, 2, 3] |> filter((x) -> x > 1)) }"),
        "[2, 3]\n"
    );
    assert_eq!(
        out("fn inc(x) { return x + 1 }\nfn main() { print(41 |> inc) }"),
        "42\n"
    );
    assert_eq!(
        out("fn add(a, b) { return a + b }\nfn main() { print(10 |> add(5)) }"),
        "15\n"
    );
}

// ---------------------------------------------- P1: checker/runtime agreement

#[test]
fn const_forward_reference_is_rejected_by_checker_and_runtime() {
    // Source-order semantics: a constant cannot reference a later constant.
    assert_eq!(
        check_code("let a = b\nlet b = 3\nfn main() { print(a) }"),
        codes::UNDEFINED
    );
    assert_eq!(
        code("let a = b\nlet b = 3\nfn main() { print(a) }"),
        codes::UNDEFINED
    );
}

#[test]
fn forward_function_reference_is_still_allowed() {
    assert_eq!(
        out("fn main() { print(later()) }\nfn later() { return 1 }"),
        "1\n"
    );
}

#[test]
fn forward_const_reference_inside_a_function_body_is_allowed() {
    // Functions run after initialization, so this is valid.
    assert_eq!(
        out("fn main() { print(f()) }\nfn f() { return x }\nlet x = 7"),
        "7\n"
    );
}

// ---------------------------------------------- P2: checker enforcement

#[test]
fn annotated_binding_type_is_enforced_on_reassignment() {
    assert_eq!(
        check_code("fn main() { let mut x: int = 1\n x = \"s\" }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(check_code("fn main() { let mut x: int = 1\n x = 2 }"), 0);
}

#[test]
fn statically_non_iterable_for_is_rejected_at_check_time() {
    assert_eq!(
        check_code("fn main() { for x in 5 { } }"),
        codes::NOT_ITERABLE
    );
    assert_eq!(check_code("fn main() { for x in \"ab\" { } }"), 0);
    assert_eq!(check_code("fn main() { for x in [1] { } }"), 0);
}

#[test]
fn break_and_continue_outside_a_loop_are_rejected() {
    assert_eq!(check_code("fn main() { break }"), codes::LOOP_CONTROL);
    assert_eq!(check_code("fn main() { continue }"), codes::LOOP_CONTROL);
    assert_eq!(
        check_code("fn main() { for i in range(0, 1) { break } }"),
        0
    );
    // A loop in a function does not license `break` in another function.
    assert_eq!(
        check_code("fn g() { break }\nfn main() { for i in range(0,1) { g() } }"),
        codes::LOOP_CONTROL
    );
}

#[test]
fn duplicate_variant_in_the_same_enum_is_rejected() {
    assert_eq!(check_code("enum A { X, X }"), codes::DUPLICATE_VARIANT);
    assert_eq!(check_code("enum A { X, Y }"), 0);
}

// ---------------------------------------------- P2: runtime boundaries

#[test]
fn i64_min_literal_is_accepted() {
    assert_eq!(
        out("fn main() { print(-9223372036854775808) }"),
        "-9223372036854775808\n"
    );
    // The bare magnitude is still out of range.
    assert_eq!(
        code("fn main() { print(9223372036854775808) }"),
        codes::INVALID_NUMBER
    );
}

#[test]
fn huge_range_loop_can_break_immediately() {
    // Lazily iterated ranges must not be materialised, so this terminates.
    assert_eq!(
        out("fn main() { let mut n = 0\n for i in range(0, 100000000) { n = n + 1\n if i == 4 { break } }\n print(n) }"),
        "5\n"
    );
}

#[test]
fn to_int_rejects_unrepresentable_floats() {
    assert_eq!(
        code("fn main() { print(to_int(to_float(\"inf\"))) }"),
        codes::OVERFLOW
    );
    assert_eq!(
        code("fn main() { print(to_int(to_float(\"nan\"))) }"),
        codes::OVERFLOW
    );
    assert_eq!(out("fn main() { print(to_int(2.9)) }"), "2\n");
}

#[test]
fn range_requires_integers() {
    assert_eq!(
        code("fn main() { print(len(range(1.0, 3.0))) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(out("fn main() { print(len(range(1, 3))) }"), "2\n");
}

#[test]
fn builtins_reject_extra_arguments() {
    assert_eq!(
        code("fn main() { print(len([1], [2])) }"),
        codes::TYPE_MISMATCH
    );
    assert_eq!(code("fn main() { print(abs(1, 2)) }"), codes::TYPE_MISMATCH);
    assert_eq!(
        code("fn main() { print(reduce([1], (a, b) -> a, 0, 9)) }"),
        codes::TYPE_MISMATCH
    );
}

#[test]
fn non_exhaustive_match_has_its_own_code() {
    assert_eq!(
        code("fn main() { print(match 5 { 1 -> \"a\" }) }"),
        codes::NO_MATCH
    );
}

// ------------------------------------------- language evolution: unions/ranges

/// A very long union chain is accepted and bounded by the existing parser
/// nesting budget, never by unbounded recursion.
#[test]
fn long_union_chain_is_bounded() {
    // A 200-member union parses and checks without blowing the stack.
    let members = (0..200)
        .map(|i| format!("type T{i} = int"))
        .collect::<Vec<_>>()
        .join("\n");
    let union = (0..200)
        .map(|i| format!("T{i}"))
        .collect::<Vec<_>>()
        .join(" | ");
    let src = format!("{members}\ntype All = {union}\nfn main() {{ let x: All = 1 }}");
    let module = aura::parse::parse(&src).expect("parses");
    assert_eq!(Checker::module(&module).map_or_else(|d| d.code, |()| 0), 0);

    // A very long *flat* union is bounded by the parser's iterative member
    // loop (not recursion), so it checks cleanly rather than overflowing.
    let flat = format!("type D = {}", vec!["int"; 5000].join(" | "));
    let module = aura::parse::parse(&flat).expect("parses");
    assert_eq!(Checker::module(&module).map_or_else(|d| d.code, |()| 0), 0);

    // A deeply nested type expression on the language's own large-stack
    // substrate does not crash; it is a deterministic diagnostic.
    let deep = format!(
        "fn main() {{ let x: {}int{} = 1 }}",
        "[".repeat(5000),
        "]".repeat(5000)
    );
    assert!(run_source(&deep, "<adv>").is_err());
}

/// A deep alias chain resolves iteratively through unions without unbounded
/// recursion, and a cyclic union alias is `E3002` rather than a crash.
#[test]
fn deep_and_cyclic_union_aliases_are_safe() {
    let chain = (0..100)
        .map(|i| {
            if i == 0 {
                "type T0 = int | float".to_string()
            } else {
                format!("type T{i} = T{}", i - 1)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let src = format!("{chain}\nfn main() {{ let x: T99 = 1 }}");
    let module = aura::parse::parse(&src).expect("parses");
    assert_eq!(Checker::module(&module).map_or_else(|d| d.code, |()| 0), 0);

    assert_eq!(check_code("type A = A | int"), codes::UNKNOWN_TYPE);
    assert_eq!(
        check_code("type A = B | int\ntype B = A"),
        codes::UNKNOWN_TYPE
    );
    assert_eq!(
        check_code("type A = B | int\ntype B = C\ntype C = A"),
        codes::UNKNOWN_TYPE
    );
    assert_eq!(
        check_code("type A = [B] | int\ntype B = A"),
        codes::UNKNOWN_TYPE
    );
}

/// Range literals around token boundaries and numeric ambiguity behave
/// deterministically and never consume a float.
#[test]
fn range_and_numeric_ambiguity_is_deterministic() {
    // `1..2` is a range of two ints, not a float and a dot.
    assert_eq!(out("fn main() { print(len(1..2)) }"), "1\n");
    // Floats still parse normally alongside int range bounds.
    assert_eq!(out("fn main() { print([1.5, 2..4]) }"), "[1.5, 2..4]\n");
    // A float bound anywhere is rejected; no implicit coercion.
    assert_eq!(code("fn main() { print(2.5..4) }"), codes::TYPE_MISMATCH);
    assert_eq!(code("fn main() { print(1.0..3) }"), codes::TYPE_MISMATCH);
    // `1..1.5` has a float end bound: rejected.
    assert_eq!(code("fn main() { print(1..1.5) }"), codes::TYPE_MISMATCH);
    // A malformed `...` is a syntax error, not a range.
    let src = "fn main() { print(1...2) }";
    let d = aura::parse::parse(src).expect_err("malformed range");
    assert_eq!(d.code, codes::EXPECTED);
}

/// Multiline comments around token boundaries and before EOF never corrupt
/// the lexer, and an unterminated one is a bounded `E1005`.
#[test]
fn comments_at_boundaries_are_safe() {
    assert_eq!(
        out("fn main() { let x = 1 <!-- c --!> + 2\n print(x) }"),
        "3\n"
    );
    // Comment between a receiver and its method.
    assert_eq!(out("fn main() { print([1]<!-- c --!>.len()) }"), "1\n");
    // Deeply many comments do not overflow.
    let many = "<!-- c --!>".repeat(2000);
    assert_eq!(out(&format!("fn main() {{ {many} print(1) }}")), "1\n");
    // Unterminated is E1005.
    let src = "fn main() { print(1) } <!-- nope";
    let d = aura::run_source(src, "<adv>").expect_err("unterminated");
    assert_eq!(d.code, codes::UNTERMINATED_COMMENT);
}

/// P1: an alias chain where each alias names the previous one twice used to
/// expand exponentially during resolution (`type T_i = T_{i-1} | T_{i-1}`),
/// hanging the checker and bypassing the AST limits. Resolution must be
/// bounded: a 200-level doubling chain completes promptly and still resolves
/// to the correct member set.
#[test]
fn doubling_alias_union_resolves_in_linear_time() {
    use std::fmt::Write as _;

    let n = 200usize;
    // A doubling chain `type T_i = T_{i-1} | T_{i-1}`.
    let chain: String = (1..=n).fold(String::new(), |mut acc, i| {
        let _ = writeln!(acc, "type T{i} = T{} | T{}", i - 1, i - 1);
        acc
    });

    let src = format!("type T0 = int | float\n{chain}fn main() {{ print(1) }}");
    let module = aura::parse::parse(&src).expect("parses");
    assert_eq!(
        Checker::module(&module).map_or_else(|d| d.code, |()| 0),
        0,
        "doubling alias chain must check cleanly"
    );

    // The resolved union is still correct after deduplication.
    let ok = format!(
        "type T0 = int | float\n{chain}type U = T{n} | string\nfn main() {{ let a: U = 1\n let b: U = 2.5\n let c: U = \"s\" }}"
    );
    let module = aura::parse::parse(&ok).expect("parses");
    assert_eq!(Checker::module(&module).map_or_else(|d| d.code, |()| 0), 0);
    // ...and a member outside the union is still rejected.
    let bad = format!(
        "type T0 = int | float\n{chain}type U = T{n} | string\nfn main() {{ let x: U = true }}"
    );
    let module = aura::parse::parse(&bad).expect("parses");
    assert_eq!(
        Checker::module(&module).map_or_else(|d| d.code, |()| 0),
        codes::TYPE_MISMATCH
    );
}
