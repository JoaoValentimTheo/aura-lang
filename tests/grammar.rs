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
    "let mut y = 2",
    "let z: int = 3",
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
    "fn e6() { print(1 < 2 <= 3 > 4 >= 5) }",
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
const ERROR_SAMPLES: &[(u16, &str)] = &[
    (codes::INVALID_CHAR, "fn main() { a && b }"),
    (codes::INVALID_NUMBER, "fn main() { print(1abc) }"),
    (codes::INVALID_ESCAPE, "fn main() { print(\"\\q\") }"),
    (codes::UNTERMINATED_STRING, "fn main() { print(\"abc) }"),
    (codes::EXPECTED, "fn () { }"),
    (
        codes::ELSE_IF,
        "fn main() { if true { } else if false { } }",
    ),
    (codes::RESERVED_NAME, "fn main() { let if = 1 }"),
    (codes::ASSIGN_IMMUTABLE, "fn main() { let x = 1\n x = 2 }"),
    (codes::UNDEFINED, "fn main() { print(nope) }"),
    (codes::LET_NO_INIT, "fn main() { let x }"),
    (codes::REDECLARED, "fn main() { let x = 1\n let x = 2 }"),
    (codes::INVALID_ASSIGN, "fn main() { 1 = 2 }"),
    (codes::TYPE_MISMATCH, "fn main() { print(\"a\" + 1) }"),
    (codes::NOT_ITERABLE, "fn main() { for x in 1 { } }"),
    (
        codes::OVERFLOW,
        "fn main() { print(9223372036854775807 + 1) }",
    ),
    (codes::DIV_ZERO, "fn main() { print(1 / 0) }"),
    (codes::RECURSION, "fn f(n) { f(n + 1) }\nfn main() { f(0) }"),
    (codes::FOREIGN, "fn main() { throw 1 }"),
];

#[test]
fn every_documented_error_code_is_reachable() {
    let mut failures = Vec::new();
    for (code, src) in ERROR_SAMPLES {
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
fn errors_doc_lists_every_code() {
    let doc = include_str!("../docs/errors.md");
    for (code, _) in ERROR_SAMPLES {
        let needle = format!("E{code:04}");
        assert!(doc.contains(&needle), "docs/errors.md is missing {needle}");
    }
}
