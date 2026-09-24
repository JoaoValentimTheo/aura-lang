//! Documentation-as-tests.
//!
//! These tests keep `docs/grammar.md` and `docs/errors.md` honest. Each
//! grammar production is exercised, and every documented diagnostic code is
//! produced by at least one program.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use aura::error::codes;
use aura::run_source;

/// Every documented grammar production, as a runnable snippet.
const GRAMMAR_SAMPLES: &[&str] = &[
    // use / declarations
    "use stdlib",
    "use stdlib.math",
    "fn f() { }",
    "fn f(a) { }",
    "fn f(a: int, b: string) -> bool { return true }",
    "pub fn g() { }",
    "struct S { }",
    "struct S { a: int, b: string }",
    "enum E { }",
    "enum E { A, B(int), C(int, string) }",
    "type Id = int",
    "type Opt = int | none",
    "let x = 1",
    "fn m() { let mut y = 2 }",
    "let z: int = 3",
    // reserved syntax is accepted and inert
    "pub fn g2() { }",
    "use stdlib.math",
    "use a.b.c",
    // types
    "fn a() -> int { return 1 }",
    "fn b() -> float { return 1.0 }",
    "fn c() -> bool { return true }",
    "fn d() -> string { return \"s\" }",
    "fn e() -> [int] { return [1] }",
    "fn h() -> {string: int} { return {\"a\": 1} }",
    "fn i() -> int | none { return none }",
    // statements
    "fn s1() { let mut a = 1\n a = a\n }",
    "fn s2(x) { let mut a = x\n a += 1\n a -= 1\n a *= 2\n a /= 2 }",
    "fn s3() { return }",
    "fn s4() { throw \"e\" }",
    "fn s5() { loop { break } }",
    "fn s6() { loop { continue } }",
    "fn s7(c) { while c { break } }",
    "fn s8(xs) { for x in xs { print(x) } }",
    "fn s9() { try { throw 1 } catch e -> { print(e) } finally { print(\"f\") } }",
    // expressions
    "fn e1() { print(1 + 2 * 3 - 4 / 5 % 6) }",
    "fn e2() { print(2 ^ 3 ^ 2) }",
    "fn e3() { print(-1) }",
    "fn e4() { print(not true) }",
    "fn e5() { print(1 == 1 != 2) }",
    "fn e6() { print(1 < 2) }",
    "fn e7() { print(true and false or true) }",
    "fn e8() { print([1, 2] |> len) }",
    "fn e9() { print((1, 2)) }",
    "fn e10() { print([1, 2, 3]) }",
    "fn e11() { print({\"a\": 1}) }",
    "fn e12() { print((x) -> x + 1) }",
    "fn e13() { print(((x, y) -> x + y)) }",
    "fn e14() { print(if true { 1 } else { 2 }) }",
    "fn e15() { print(match 1 { 1 -> \"a\"\n _ -> \"b\" }) }",
    "fn e16() { print(f\"x = {1 + 1}\") }",
    "fn e17() { let t = true\n print(t) }",
    "fn e18() { print({ 1 + 1 }) }",
    "fn e19() { print(\"a\".upper()) }",
    "fn e20() { print(\"a\".split(\",\")) }",
];

#[test]
fn grammar_samples_parse_and_run() {
    let mut failures = Vec::new();
    for src in GRAMMAR_SAMPLES {
        if let Err(d) = run_source(src, "<grammar>") {
            failures.push(format!("{src}\n  -> {d}"));
        }
    }
    assert!(
        failures.is_empty(),
        "grammar samples failed:\n{}",
        failures.join("\n")
    );
}

/// Documented codes and a program that triggers each.
///
/// Codes that can only be produced by the run path (`E4027`) or by a
/// feature-disabled build (`E5002`) are covered by dedicated tests below.
fn error_samples() -> Vec<(u16, String)> {
    let mut v: Vec<(u16, String)> = vec![
        (codes::INVALID_CHAR, "fn main() { a && b }".to_string()),
        (
            codes::INVALID_NUMBER,
            "fn main() { print(1abc) }".to_string(),
        ),
        (
            codes::INVALID_ESCAPE,
            "fn main() { print(\"\\q\") }".to_string(),
        ),
        (
            codes::UNTERMINATED_STRING,
            "fn main() { print(\"abc) }".to_string(),
        ),
        (codes::EXPECTED, "fn () { }".to_string()),
        (codes::RESERVED_NAME, "fn main() { let if = 1 }".to_string()),
        (
            codes::ASSIGN_IMMUTABLE,
            "fn main() { let x = 1\n x = 2 }".to_string(),
        ),
        (codes::UNDEFINED, "fn main() { print(nope) }".to_string()),
        (codes::LET_NO_INIT, "fn main() { let x }".to_string()),
        (
            codes::REDECLARED,
            "fn main() { let x = 1\n let x = 2 }".to_string(),
        ),
        (codes::UNUSED_PARAM, "fn f(_x) { return _x }".to_string()),
        (codes::INVALID_ASSIGN, "fn main() { 1 = 2 }".to_string()),
        (codes::INVALID_MAIN, "fn main(x) { }".to_string()),
        (
            codes::DUPLICATE_TYPE,
            "struct S { }\nstruct S { }".to_string(),
        ),
        (
            codes::DUPLICATE_VARIANT,
            "enum A { X }\nenum B { X }".to_string(),
        ),
        (
            codes::DUPLICATE_BINDING,
            "fn main() { match [1, 2] { [a, a] -> print(a)\n _ -> print(0) } }".to_string(),
        ),
        (
            codes::TYPE_MISMATCH,
            "fn main() { print(\"a\" + 1) }".to_string(),
        ),
        (
            codes::UNKNOWN_TYPE,
            "fn f() -> Widget { return 1 }".to_string(),
        ),
        (
            codes::RETURN_MISMATCH,
            "fn f() -> int { return \"no\" }".to_string(),
        ),
        (
            codes::NOT_ITERABLE,
            "fn main() { for x in 1 { } }".to_string(),
        ),
        (
            codes::OVERFLOW,
            "fn main() { print(9223372036854775807 + 1) }".to_string(),
        ),
        (codes::DIV_ZERO, "fn main() { print(1 / 0) }".to_string()),
        (codes::INDEX, "fn main() { print([1, 2][5]) }".to_string()),
        (
            codes::RECURSION,
            "fn f(n) { f(n + 1) }\nfn main() { f(0) }".to_string(),
        ),
        (codes::FOREIGN, "fn main() { throw 1 }".to_string()),
        (
            codes::ASSERT,
            "fn main() { assert(false, \"nope\") }".to_string(),
        ),
        (codes::LOOP_CONTROL, "fn main() { break }".to_string()),
        (
            codes::NO_MATCH,
            "fn main() { print(match 5 { 1 -> \"a\" }) }".to_string(),
        ),
        // A `return`, `break`, `continue`, or `throw` produced while computing
        // an expression escapes to value position.
        (
            codes::RETURN_POSITION,
            "fn main() { let x = if true { return 1 } else { 2 } }".to_string(),
        ),
    ];
    v.push((
        codes::NESTING,
        format!("fn main() {{ print(1{}) }}", "+1".repeat(5000)),
    ));
    v
}

#[test]
fn every_documented_error_code_is_reachable() {
    let mut failures = Vec::new();
    for (code, src) in &error_samples() {
        match run_source(src, "<errors>") {
            Ok(_) => failures.push(format!("expected E{code:04} from: {src}")),
            Err(d) if d.code == *code => {}
            Err(d) => failures.push(format!(
                "expected E{code:04}, got E{:04} from: {src}",
                d.code
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "error table mismatch:\n{}",
        failures.join("\n")
    );
}

#[test]
fn missing_main_is_reachable_through_the_program_entry_point() {
    // E4027 is produced by the `aura run` path, which requires an entry point.
    let err = aura::run_program("fn helper() { }", "<program>")
        .expect_err("a program without main must be rejected");
    assert_eq!(err.code, codes::NO_MAIN);
}

#[test]
fn feature_unavailable_is_reachable_without_py() {
    // When the `py` feature is disabled, `py_eval` reports E5002 rather than
    // pretending to exist or reporting a generic undefined-name error.
    #[cfg(not(feature = "py"))]
    {
        let err = run_source("fn main() { py_eval(\"1\") }", "<errors>")
            .expect_err("py_eval without the feature must be rejected");
        assert_eq!(err.code, codes::PY_UNSUPPORTED);
    }
}

#[test]
fn errors_doc_lists_every_code() {
    let doc = include_str!("../docs/errors.md");
    for (code, _) in &error_samples() {
        let needle = format!("E{code:04}");
        assert!(doc.contains(&needle), "docs/errors.md is missing {needle}");
    }
}

/// F-17: the documented pipeline desugaring must match the parser. `x |> f(a)`
/// passes `x` as the *first* argument of `f`.
#[test]
fn pipeline_passes_the_left_operand_as_the_first_argument() {
    // `f(x, a)`.
    let src = "fn f(a, b) -> string { return a + b }\nfn main() { print(1 |> f(2)) }";
    assert_eq!(run_source(src, "<pipe>").unwrap(), "3\n");
    // Method form: `x |> r.m(a)` is `r.m(x, a)` (the left operand becomes the
    // receiver's first argument). Here `"x" |> "xZ".replace("y")` desugars to
    // `"xZ".replace("x", "y")`.
    let src = "fn main() { print(\"x\" |> \"xZ\".replace(\"y\")) }";
    assert_eq!(run_source(src, "<pipe>").unwrap(), "yZ\n");
    // Bare callable: `x |> f` is `f(x)`.
    let src = "fn f(x) -> int { return x + 1 }\nfn main() { print(1 |> f) }";
    assert_eq!(run_source(src, "<pipe>").unwrap(), "2\n");
}
