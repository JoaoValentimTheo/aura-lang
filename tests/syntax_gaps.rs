#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The closed Core syntax SPEC GAPs (LANGUAGE_SPEC §3.6, §3.7, §4.9, §27, §34).
//!
//! Every previously open Core syntax gap has a decided rule and a test here:
//! separator policy (CONF-PARSE-8), numeric underscore placement, f-string
//! brace edges, generic-head `::` continuation (CONF-GRAM-4), and the removal
//! of the legacy dotted module path.

use aura::error::codes;
use aura::parse::parse;
use aura::run_source;

fn ok(src: &str) -> String {
    run_source(src, "<syntax-gaps>").expect("expected the program to run")
}

fn code(src: &str) -> u16 {
    run_source(src, "<syntax-gaps>")
        .expect_err("expected the program to be rejected")
        .code
}

// ------------------------------------------------------- CONF-PARSE-8

#[test]
fn statements_require_a_real_separator() {
    assert_eq!(
        code("fn main() { let x = 1 let y = 2\n print(x) }"),
        codes::EXPECTED
    );
    assert_eq!(code("fn main() { print(1) print(2) }"), codes::EXPECTED);
    // A newline and the boundary before `}` are the only separators. `;` is a
    // reserved token, not a statement separator (§3.7).
    assert_eq!(
        ok("fn main() { let x = 1\n let y = 2\n print(x + y) }"),
        "3\n"
    );
    assert_eq!(
        code("fn main() { let x = 1; let y = 2; print(x + y) }"),
        codes::EXPECTED
    );
    assert_eq!(ok("fn main() { print(1) }"), "1\n");
    // Items too: `fn a() {} fn b() {}` on one line is rejected, in every
    // item-list body (file root, module, impl, trait).
    assert_eq!(code("fn a() {} fn b() {}"), codes::EXPECTED);
    assert_eq!(ok("fn a() { }\nfn b() { }\nfn main() { print(1) }"), "1\n");
    assert_eq!(
        code("module M { fn a() { } fn b() { } }\nfn main() { print(1) }"),
        codes::EXPECTED
    );
    assert_eq!(
        ok("module M {\n fn a() { }\n fn b() { }\n}\nfn main() { print(1) }"),
        "1\n"
    );
    assert_eq!(
        code("struct S { x: int }\nimpl S { fn f(self) -> int { return 1 } fn g(self) -> int { return 2 } }\nfn main() { }"),
        codes::EXPECTED
    );
    assert_eq!(
        code("trait T { fn f(self) fn g(self) }\nfn main() { }"),
        codes::EXPECTED
    );
}

// ------------------------------------------------- numeric underscores

#[test]
fn underscore_may_sit_between_digits_only() {
    for src in [
        "fn main() { print(1_000) }",
        "fn main() { print(0xff_ff) }",
        "fn main() { print(0b1010_0001) }",
        "fn main() { print(0o755_123) }",
        "fn main() { print(1_000.25) }",
        "fn main() { print(1.25_00) }",
        "fn main() { print(1e1_0) }",
        "fn main() { print(1.5e1_0) }",
    ] {
        assert!(run_source(src, "<n>").is_ok(), "{src}");
    }
    for src in [
        "fn main() { print(1_) }",
        "fn main() { print(1__0) }",
        "fn main() { print(0x_ff) }",
        "fn main() { print(0b_101) }",
        "fn main() { print(1_.0) }",
        "fn main() { print(1e_10) }",
        "fn main() { print(1e10_) }",
    ] {
        assert_eq!(code(src), codes::INVALID_NUMBER, "{src}");
    }
}

// ------------------------------------------------------- f-string edges

#[test]
fn a_lone_closing_brace_in_an_fstring_is_rejected() {
    assert_eq!(code("fn main() { print(f\"a}b\") }"), codes::EXPECTED);
    assert_eq!(code("fn main() { print(f\"}\") }"), codes::EXPECTED);
    // `}}` is the only literal `}`.
    assert_eq!(ok("fn main() { print(f\"a{{b}}c\") }"), "a{b}c\n");
    assert_eq!(ok("fn main() { print(f\"{{}}\") }"), "{}\n");
    // A `:` inside a nested map is part of the expression, not a format spec.
    assert_eq!(ok("fn main() { print(f\"{ {1: 2}[1] }\") }"), "2\n");
}

// ------------------------------------------- CONF-GRAM-4 generic `::`

#[test]
fn generic_head_may_continue_through_qualified_variant() {
    assert_eq!(
        ok("enum Result<T, E> { Ok(T), Err(E) }\nfn main() { let r: Result<int, string> = Result<int, string>::Ok(1)\n print(r) }"),
        "Ok(1)\n"
    );
    // The type arguments belong to the enum, so a wrong arity is rejected.
    assert_eq!(
        code("enum Result<T, E> { Ok(T), Err(E) }\nfn main() { let r = Result<int>::Ok(1) }"),
        codes::UNKNOWN_TYPE
    );
    // Module-qualified generic head.
    assert_eq!(
        ok("module m { pub enum Result<T> { Ok(T) } }\nfn main() { let r = m::Result<int>::Ok(1)\n print(r) }"),
        "m::Ok(1)\n"
    );
}

// ------------------------------------------------- module path spelling

#[test]
fn module_paths_use_double_colon_only() {
    assert_eq!(
        code("module a { pub fn f() { } }\nuse a.f\nfn main() { f() }"),
        codes::EXPECTED
    );
    let module = parse("module a { pub fn f() { } }\nuse a::f\nfn main() { f() }")
        .expect("canonical `::` import parses");
    assert!(!module.items.is_empty());
}

// --------------------------------------------- container nesting is bounded

#[test]
fn nested_containers_report_e1015_before_the_host_stack() {
    // Container nesting is counted *during* parsing, so over-deep input reports
    // the semantic E1015 rather than exhausting the substrate stack (which
    // traps on wasm). The accepted/rejected boundary matches the post-parse
    // AST-depth rule.
    let deep = |d: usize| format!("fn main() {{ print({}1{}) }}", "[".repeat(d), "]".repeat(d));
    assert!(run_source(&deep(252), "<n>").is_ok());
    assert!(run_source(&deep(253), "<n>").is_ok());
    assert_eq!(code(&deep(254)), codes::NESTING);
    assert_eq!(code(&deep(400)), codes::NESTING);
    // Pure grouping parentheses add no AST level, so many are fine; only
    // container literals are bounded.
    let parens = format!(
        "fn main() {{ print({}1{}) }}",
        "(".repeat(300),
        ")".repeat(300)
    );
    assert!(run_source(&parens, "<n>").is_ok());
}
