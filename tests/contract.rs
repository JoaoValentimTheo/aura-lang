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
    // `def`, `function` are not declarations: they are ordinary names. With
    // the separator rule (CONF-PARSE-8) an adjacent name is now a parse error
    // (E1006); without adjacency the name is simply undefined. Either way they
    // do not declare anything.
    assert_eq!(code_of("def f() { }"), codes::EXPECTED);
    assert_eq!(code_of("def"), codes::UNDEFINED);
    assert_eq!(code_of("function f() { }"), codes::EXPECTED);
    assert_eq!(code_of("function"), codes::UNDEFINED);
    // `!` is rejected at the lexer. `&&` and `||` have no operator meaning:
    // they lex as two `&`/`|` tokens and fail to parse, so they never gain an
    // accidental meaning.
    assert_eq!(
        code_of("fn main() { print(true && false) }"),
        codes::EXPECTED
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
        code_of("const X = 1\nconst X = 2\nfn main() { print(X) }"),
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
        codes::RESERVED_NAME,
        codes::ASSIGN_IMMUTABLE,
        codes::UNDEFINED,
        codes::LET_NO_INIT,
        codes::REDECLARED,
        codes::INVALID_ASSIGN,
        codes::MODULE_SOURCE_OWNERSHIP,
        codes::DUPLICATE_LOGICAL_SOURCE,
        codes::MODULE_SOURCE_PATH,
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

/// The release identity is derived from the package version (not a hardcoded
/// duplicate), so the crate version, the exported `aura::VERSION`, and
/// `aura version` always agree. The exact value is not pinned here; it is only
/// required to be valid semver. Historical releases (`0.0.1`, `0.0.2`,
/// `0.2.0`) are immutable artifacts of the past, not the current release.
#[test]
fn release_version_is_valid_semver_and_derived() {
    assert_eq!(aura::VERSION, env!("CARGO_PKG_VERSION"));
    assert!(
        parse_semver(aura::VERSION).is_some(),
        "release version is not valid semver: {}",
        aura::VERSION
    );
}

/// The language semantics version tracks the current public language contract
/// and must be valid semver. It is independent of the release version.
///
/// Invariant (ADR-0001): `LANGUAGE_VERSION <= RELEASE_VERSION`. A release may
/// advance without a language change; a language change never ships ahead of
/// the release that carries it. They are equal when the release *is* a language
/// release.
#[test]
fn language_version_is_valid_semver_and_not_ahead_of_release() {
    let lang = parse_semver(aura::LANGUAGE_VERSION).unwrap_or_else(|| {
        panic!(
            "language version is not valid semver: {}",
            aura::LANGUAGE_VERSION
        )
    });
    let rel = parse_semver(aura::VERSION).expect("release version checked above");
    assert!(
        lang <= rel,
        "LANGUAGE_VERSION ({}) must not exceed RELEASE_VERSION ({})",
        aura::LANGUAGE_VERSION,
        aura::VERSION
    );
}

/// Parse a `MAJOR.MINOR.PATCH` (optionally with a pre-release suffix ignored
/// for ordering here) into a comparable tuple. Returns `None` if not semver.
fn parse_semver(v: &str) -> Option<(u64, u64, u64)> {
    let core = v.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}
