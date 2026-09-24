#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Tests for the parser.

use aura::ast::*;
use aura::error::codes;
use aura::parse::{parse, parse_expr};

#[test]
fn empty_module() {
    let m = parse("").expect("parse");
    assert!(m.items.is_empty());
}

#[test]
fn simple_fn() {
    let m = parse("fn add(a: int, b: int) -> int { return a + b }").expect("parse");
    assert_eq!(m.items.len(), 1);
    match &m.items[0] {
        Item::Fn {
            name,
            params,
            ret,
            body,
            ..
        } => {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "a");
            assert_eq!(ret.as_ref().map(TypeExpr::name), Some("int".to_string()));
            assert_eq!(body.len(), 1);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn precedence() {
    // 1 + 2 * 3 parses as 1 + (2 * 3)
    let e = parse_expr("1 + 2 * 3").expect("parse");
    match e {
        Expr::Binary(BinOp::Add, _, rhs, _) => {
            assert!(matches!(*rhs, Expr::Binary(BinOp::Mul, _, _, _)));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn pow_is_right_associative() {
    // 2 ^ 3 ^ 2 parses as 2 ^ (3 ^ 2)
    let e = parse_expr("2 ^ 3 ^ 2").expect("parse");
    match e {
        Expr::Binary(BinOp::Pow, _, rhs, _) => {
            assert!(matches!(*rhs, Expr::Binary(BinOp::Pow, _, _, _)));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn if_else_expression() {
    let e = parse_expr("if a > b { a } else { b }").expect("parse");
    assert!(matches!(e, Expr::If(_, _, Some(_), _)));
}

#[test]
fn else_if_is_rejected() {
    let err = parse("fn f() { if a { 1 } else if b { 2 } else { 3 } }").unwrap_err();
    assert_eq!(err.code, aura::error::codes::ELSE_IF);
}

#[test]
fn match_arms() {
    let e = parse_expr("match x {\n  1 -> \"one\"\n  n -> \"other\"\n}").expect("parse");
    match e {
        Expr::Match(_, arms, _) => assert_eq!(arms.len(), 2),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn lambda_with_parens() {
    let e = parse_expr("(x, y) -> x + y").expect("parse");
    match e {
        Expr::Lambda(ps, _, _) => assert_eq!(ps, vec!["x", "y"]),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn lists_maps_and_index() {
    assert!(matches!(
        parse_expr("[1, 2, 3]").expect("parse"),
        Expr::List(_, _)
    ));
    assert!(matches!(
        parse_expr("{1: \"a\"}").expect("parse"),
        Expr::Map(_, _)
    ));
    assert!(matches!(
        parse_expr("xs[0]").expect("parse"),
        Expr::Index(_, _, _)
    ));
}

#[test]
fn method_and_field() {
    assert!(matches!(
        parse_expr("s.trim()").expect("parse"),
        Expr::Method(_, _, _, _)
    ));
    assert!(matches!(
        parse_expr("p.x").expect("parse"),
        Expr::Field(_, _, _)
    ));
}

#[test]
fn pipe() {
    let e = parse_expr("xs |> len").expect("parse");
    assert!(matches!(e, Expr::Pipe(_, _, _)));
}

#[test]
fn let_requires_initializer() {
    let err = parse("fn f() { let x }").unwrap_err();
    assert_eq!(err.code, aura::error::codes::LET_NO_INIT);
}

#[test]
fn fstring_parses_parts() {
    let e = parse_expr("f\"a{x}b\"").expect("parse");
    match e {
        Expr::FStr(parts, _) => {
            assert_eq!(parts.len(), 3);
            assert!(matches!(parts[1], FPart::Expr(_)));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn deep_nesting_is_bounded() {
    // Grouping parentheses do not add AST depth, so a very long paren chain is
    // accepted; the parser still bounds raw recursion so an extreme chain is
    // rejected rather than overflowing the host stack.
    let ok = format!("{}1{}", "(".repeat(500), ")".repeat(500));
    assert!(parse_expr(&ok).is_ok());
    let extreme = format!("{}1{}", "(".repeat(5000), ")".repeat(5000));
    assert!(parse_expr(&extreme).is_err());
}

#[test]
fn struct_and_enum() {
    let m = parse("struct P { x: float, y: float }\nenum E { A(int), B }").expect("parse");
    assert_eq!(m.items.len(), 2);
}

#[test]
fn for_and_while() {
    let m = parse("fn f() { for x in xs { print(x) } while c { c = false } }").expect("parse");
    assert_eq!(m.items.len(), 1);
}

// ---------------------------------------------------------------------------
// FEATURE_004: destructuring `let` (`LANGUAGE_SPEC.md` §4.7).
// ---------------------------------------------------------------------------

/// The supported destructuring forms parse.
#[test]
fn destructuring_let_parses_supported_forms() {
    for src in [
        "fn main() { let [a, b] = e }",
        "fn main() { let [a, [b, c]] = e }",
        "fn main() { let Ok(x) = e }",
        "fn main() { let Some([a, b]) = e }",
        "fn main() { let [Ok(a), Err(b)] = e }",
        "fn main() { let _ = e }",
        "fn main() { let [a, _] = e }",
    ] {
        assert!(parse(src).is_ok(), "should parse: {src}");
    }
}

/// A lone identifier keeps the ordinary `let` path (same AST shape).
#[test]
fn destructuring_let_identifier_is_ordinary() {
    let m = parse("fn main() { let x = 1 }").expect("parse");
    match &m.items[0] {
        Item::Fn { body, .. } => match &body[0] {
            Stmt::Let {
                name, ann, mutable, ..
            } => {
                assert_eq!(name, "x");
                assert!(ann.is_none());
                assert!(!mutable);
            }
            other => panic!("expected ordinary let, got {other:?}"),
        },
        _ => panic!("expected fn"),
    }
}

/// Literal and `none` patterns are rejected specifically in `let`, but remain
/// legal in `match`/`for`.
#[test]
fn destructuring_let_rejects_literals() {
    for src in [
        "fn main() { let 5 = e }",
        "fn main() { let \"s\" = e }",
        "fn main() { let true = e }",
        "fn main() { let none = e }",
        "fn main() { let [a, 5] = e }",
        "fn main() { let Ok(5) = e }",
    ] {
        let err = parse(src).expect_err("must be rejected");
        assert_eq!(err.code, codes::EXPECTED, "src: {src}");
    }
    // The general pattern parser still accepts literals.
    assert!(parse("fn main() { match x { 5 -> 1\n _ -> 0 } }").is_ok());
    assert!(parse("fn main() { for 5 in xs { } }").is_ok());
}

/// Annotations and `mut` are rejected on a destructuring `let`.
#[test]
fn destructuring_let_rejects_annotation_and_mut() {
    assert_eq!(
        parse("fn main() { let [a, b]: [int] = e }")
            .expect_err("must be rejected")
            .code,
        codes::EXPECTED
    );
    assert_eq!(
        parse("fn main() { let mut [a, b] = e }")
            .expect_err("must be rejected")
            .code,
        codes::EXPECTED
    );
    // The ordinary annotated/mutable forms still parse.
    assert!(parse("fn main() { let x: int = 1 }").is_ok());
    assert!(parse("fn main() { let mut x = 1 }").is_ok());
}

/// A reserved word in binding position keeps its dedicated diagnostic.
#[test]
fn destructuring_let_reserved_word_is_e1009() {
    assert_eq!(
        parse("fn main() { let if = 1 }")
            .expect_err("rejected")
            .code,
        codes::RESERVED_NAME
    );
}

/// Destructuring requires an initializer.
#[test]
fn destructuring_let_requires_initializer() {
    assert_eq!(
        parse("fn main() { let [a, b] }")
            .expect_err("rejected")
            .code,
        codes::LET_NO_INIT
    );
}
