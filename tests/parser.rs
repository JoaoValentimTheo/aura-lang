#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Tests for the parser.

use aura::ast::*;
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
    let src = format!("{}1{}", "(".repeat(1000), ")".repeat(1000));
    assert!(parse_expr(&src).is_err());
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
