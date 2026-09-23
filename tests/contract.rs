#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Contract tests.
//!
//! `docs/contract.md` is normative. Every rule asserted here is a promise
//! the language makes to its users. If a test here fails, either the
//! implementation or the contract is wrong — never neither.

use aura::error::codes;
use aura::run_source;

/// Assert a program is rejected with a specific code.
fn code_of(src: &str) -> u16 {
    run_source(src, "<contract>")
        .expect_err("expected the program to be rejected")
        .code
}

/// Assert a program runs and produces the given output.
fn output_of(src: &str) -> String {
    run_source(src, "<contract>").expect("expected the program to run")
}

// ---------------------------------------------------------------- R#1

#[test]
fn r1_mutable_means_mutable() {
    assert_eq!(
        code_of("fn main() { let x = 1\n x = 2 }"),
        codes::ASSIGN_IMMUTABLE
    );
    assert_eq!(
        output_of("fn main() { let mut x = 1\n x = 2\n print(x) }"),
        "2\n"
    );
}

#[test]
fn r1_let_requires_initializer() {
    assert_eq!(code_of("fn main() { let x }"), codes::LET_NO_INIT);
}

// ---------------------------------------------------------------- R#2

#[test]
fn r2_none_is_the_only_absence() {
    assert_eq!(output_of("fn main() { print(none) }"), "none\n");
    // `null`, `nil`, `undefined` are ordinary names -> undefined variable.
    assert_eq!(code_of("fn main() { print(null) }"), codes::UNDEFINED);
    assert_eq!(code_of("fn main() { print(nil) }"), codes::UNDEFINED);
    assert_eq!(code_of("fn main() { print(undefined) }"), codes::UNDEFINED);
}

// ---------------------------------------------------------------- R#3

#[test]
fn r3_no_synonyms() {
    // `def`, `function` are not declarations: they are ordinary names, so a
    // program using them fails name resolution rather than parsing as a
    // function. Either way, they do not declare anything.
    assert_eq!(code_of("def f() { }"), codes::UNDEFINED);
    assert_eq!(code_of("function f() { }"), codes::UNDEFINED);
    // `&&` and `!` are rejected at the lexer. `||` is rejected because `|`
    // is reserved for type unions, so it fails to parse as an operator.
    assert_eq!(
        code_of("fn main() { print(true && false) }"),
        codes::INVALID_CHAR
    );
    assert_eq!(
        code_of("fn main() { print(true || false) }"),
        codes::EXPECTED
    );
    assert_eq!(code_of("fn main() { print(!true) }"), codes::INVALID_CHAR);
    // `and`/`or`/`not` are the accepted spellings.
    assert_eq!(
        output_of("fn main() { print(true and not false) }"),
        "true\n"
    );
    assert_eq!(output_of("fn main() { print(false or true) }"), "true\n");
}

#[test]
fn r3_else_if_is_rejected() {
    assert_eq!(
        code_of("fn main() { if true { } else if false { } }"),
        codes::ELSE_IF
    );
}

// ---------------------------------------------------------------- R#4

#[test]
fn r4_no_silent_coercion() {
    // String + int is a type error.
    assert_eq!(
        code_of("fn main() { print(\"a\" + 1) }"),
        codes::TYPE_MISMATCH
    );
    // Explicit conversion works.
    assert_eq!(
        output_of("fn main() { print(\"a\" + to_string(1)) }"),
        "a1\n"
    );
}

// ---------------------------------------------------------------- R#6

#[test]
fn r6_every_rejection_has_a_stable_code() {
    // The contract table maps constructs to codes; spot-check the mapping.
    assert_eq!(code_of("fn main() { @ }"), codes::INVALID_CHAR);
    assert_eq!(code_of("fn main() { print(1abc) }"), codes::INVALID_NUMBER);
    assert_eq!(
        code_of("fn main() { print(\"unterminated) }"),
        codes::UNTERMINATED_STRING
    );
    assert_eq!(
        code_of("fn main() { print(\"\\q\") }"),
        codes::INVALID_ESCAPE
    );
    assert_eq!(
        code_of("fn main() { let x = 1\n let x = 2 }"),
        codes::REDECLARED
    );
    assert_eq!(code_of("fn main() { let = 1 }"), codes::EXPECTED);
}

// ---------------------------------------------------------------- R#7

#[test]
fn r7_structural_equality() {
    assert_eq!(output_of("fn main() { print([1, 2] == [1, 2]) }"), "true\n");
    assert_eq!(
        output_of("fn main() { print([1, 2] == [1, 3]) }"),
        "false\n"
    );
    assert_eq!(
        output_of("fn main() { print({\"a\": 1} == {\"a\": 1}) }"),
        "true\n"
    );
    assert_eq!(output_of("fn main() { print(1 == 1.0) }"), "true\n");
}

// ---------------------------------------------------------------- R#8

#[test]
fn r8_recursion_is_capped_not_a_crash() {
    let src = "fn f(n) { return f(n + 1) }\nfn main() { f(0) }";
    let err = run_source(src, "<contract>").expect_err("must be rejected");
    assert_eq!(err.code, codes::RECURSION);
}

// ---------------------------------------------------------------- R#10

#[test]
fn r10_defined_errors_documented() {
    // Every code referenced in docs/contract.md must be non-zero and
    // distinct; this guards against typos in the code table.
    let all = [
        codes::INVALID_CHAR,
        codes::INVALID_NUMBER,
        codes::INVALID_ESCAPE,
        codes::UNTERMINATED_STRING,
        codes::EXPECTED,
        codes::ELSE_IF,
        codes::RESERVED_NAME,
        codes::ASSIGN_IMMUTABLE,
        codes::UNDEFINED,
        codes::LET_NO_INIT,
        codes::REDECLARED,
        codes::INVALID_ASSIGN,
        codes::TYPE_MISMATCH,
        codes::NOT_ITERABLE,
        codes::OVERFLOW,
        codes::DIV_ZERO,
        codes::RECURSION,
        codes::NO_MAIN,
    ];
    assert!(all.iter().all(|c| *c >= 1001));
    let mut sorted = all.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    // Codes are allowed to share numeric ranges but the ones we use are
    // distinct enough that duplicates would signal a copy/paste bug.
    assert_eq!(sorted.len(), all.len());
}
