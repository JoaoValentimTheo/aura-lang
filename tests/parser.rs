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

// ---------------------------------------------------------------------------
// FEATURE_005: empty-map literal `{:}` (`LANGUAGE_SPEC.md` §20.3).
// ---------------------------------------------------------------------------

fn empty_map_entries(e: &Expr) -> usize {
    match e {
        Expr::Map(entries, _) => entries.len(),
        other => panic!("expected a map, got {other:?}"),
    }
}

#[test]
fn empty_map_literal_parses_to_empty_map() {
    assert_eq!(empty_map_entries(&parse_expr("{:}").expect("parse")), 0);
    // Whitespace and newlines around the colon do not change the meaning.
    assert_eq!(empty_map_entries(&parse_expr("{ : }").expect("parse")), 0);
    assert_eq!(empty_map_entries(&parse_expr("{\n:\n}").expect("parse")), 0);
}

#[test]
fn empty_braces_still_parse_as_a_block() {
    assert!(matches!(
        parse_expr("{}").expect("parse"),
        Expr::Block(_, _)
    ));
}

#[test]
fn non_empty_map_is_unchanged() {
    assert_eq!(
        empty_map_entries(&parse_expr("{\"a\": 1}").expect("parse")),
        1
    );
    assert_eq!(
        empty_map_entries(&parse_expr("{\"a\": 1, \"b\": 2}").expect("parse")),
        2
    );
}

#[test]
fn malformed_empty_map_is_e1006() {
    assert_eq!(
        parse_expr("{: 1}").expect_err("rejected").code,
        codes::EXPECTED
    );
}

// ---------------------------------------------------------------------------
// FEATURE_006: `else if` (`LANGUAGE_SPEC.md` §4.5).
// ---------------------------------------------------------------------------

/// `else if` parses to nested existing `Expr::If` nodes — no new AST node.
#[test]
fn else_if_parses_to_nested_if() {
    let e = parse_expr("if a { 1 } else if b { 2 } else { 3 }").expect("parse");
    // Outer is `if a { 1 } else <inner>`; the `else` operand is itself an `if`.
    match e {
        Expr::If(_, _, Some(els), _) => {
            assert!(matches!(*els, Expr::If(_, _, Some(_), _)));
        }
        other => panic!("expected nested if, got {other:?}"),
    }
}

/// Multiple `else if` clauses nest one `Expr::If` per clause.
#[test]
fn multiple_else_if_parses_as_deep_nesting() {
    let e = parse_expr("if a { 1 } else if b { 2 } else if c { 3 } else { 4 }").expect("parse");
    let mut depth = 0;
    let mut cur = &e;
    while let Expr::If(_, _, Some(els), _) = cur {
        depth += 1;
        cur = els;
    }
    // Three conditions => three nested `Expr::If` nodes along the `els` spine.
    assert_eq!(depth, 3);
}

/// A final `else` and no final `else` both parse; the latter leaves the
/// innermost `els` as `None`.
#[test]
fn else_if_final_else_is_optional() {
    let with_else = parse_expr("if a { 1 } else if b { 2 } else { 3 }").expect("parse");
    let inner = match with_else {
        Expr::If(_, _, Some(els), _) => *els,
        other => panic!("expected if, got {other:?}"),
    };
    assert!(matches!(inner, Expr::If(_, _, Some(_), _)));

    let no_else = parse_expr("if a { 1 } else if b { 2 }").expect("parse");
    let inner = match no_else {
        Expr::If(_, _, Some(els), _) => *els,
        other => panic!("expected if, got {other:?}"),
    };
    assert!(matches!(inner, Expr::If(_, _, None, _)));
}

/// Newline rules are unchanged: a newline before `else`, or between `else`
/// and `if`, is `E1006`.
#[test]
fn else_if_newline_rules_are_unchanged() {
    assert_eq!(
        parse("fn f() { if a { 1 }\nelse { 2 } }")
            .expect_err("rejected")
            .code,
        codes::EXPECTED
    );
    assert_eq!(
        parse("fn f() { if a { 1 } else\nif b { 2 } }")
            .expect_err("rejected")
            .code,
        codes::EXPECTED
    );
}

/// A malformed `else if` (missing condition) is a parse error.
#[test]
fn malformed_else_if_is_rejected() {
    assert_eq!(
        parse("fn f() { if a { 1 } else if { 2 } }")
            .expect_err("rejected")
            .code,
        codes::EXPECTED
    );
}

/// Ordinary `if`/`else` and `else <non-if expression>` remain unchanged.
#[test]
fn ordinary_if_else_is_unchanged() {
    let e = parse_expr("if a > b { a } else { b }").expect("parse");
    assert!(matches!(e, Expr::If(_, _, Some(_), _)));
    // `else` still accepts an arbitrary expression.
    assert!(parse_expr("if a { 1 } else 2 + 3").is_ok());
}

/// General unions are now current syntax (`LANGUAGE_SPEC.md` §4.3): `T1 | T2`
/// and longer chains parse in every annotation position, and `T | none` is
/// just the one-member-plus-`none` case of the same grammar.
#[test]
fn general_union_parses_in_every_type_position() {
    let accepted = [
        "type Number = int | float",
        "type ID = string | int",
        "type N = int | float | none",
        "fn main() { let x: int | float = 1 }",
        "let x: int | bool = true",
        "fn f(x: string | int) { }",
        "fn f() -> int | float { return 1 }",
        "struct S { x: int | float }",
        "enum E { A(string | int) }",
        "fn main() { let xs: [int | float] = [1] }",
        "fn main() { let m: {string: int | float} = {\"a\": 1} }",
        // `T | none` remains valid as a union with `none`.
        "type N2 = int | none",
        "fn f2(x: int | none) -> bool | none { return none }",
    ];
    for src in accepted {
        assert!(parse(src).is_ok(), "expected {src:?} to parse");
    }
}

/// A union member must be a type, and the chain must not dangle: `int |`,
/// `| int`, and `int | 5` are deterministic `E1006`.
#[test]
fn malformed_union_is_syntax_error() {
    for src in [
        "type N = int |",
        "type N = | int",
        "type N = int | 5",
        "type N = int || string",
        "fn f(x: int | ) { }",
    ] {
        assert_eq!(
            parse(src).map_err(|d| d.code),
            Err(codes::EXPECTED),
            "expected E1006 for {src:?}"
        );
    }
}

/// The parser records a union as `TypeExpr::Union` with one node per member,
/// in source order, so diagnostics and `name()` spell it back verbatim.
#[test]
fn union_is_parsed_into_a_multi_member_node() {
    let m = parse("type N = int | float | string").expect("parse");
    match &m.items[0] {
        Item::Alias { target, .. } => {
            assert_eq!(
                target,
                &TypeExpr::Union(vec![TypeExpr::Int, TypeExpr::Float, TypeExpr::String])
            );
            assert_eq!(target.name(), "int | float | string");
        }
        other => panic!("expected alias, got {other:?}"),
    }
    // Two members form a two-member union, not a nested pair.
    let m = parse("type N = int | none").expect("parse");
    match &m.items[0] {
        Item::Alias { target, .. } => {
            assert_eq!(
                target,
                &TypeExpr::Union(vec![TypeExpr::Int, TypeExpr::None])
            );
        }
        other => panic!("expected alias, got {other:?}"),
    }
}

/// `a..b` is now current range syntax (`LANGUAGE_SPEC.md` §22.1): it parses in
/// expression, `for`, list, and binding positions, and preserves the semantic
/// distinction `Range ≠ List`.
#[test]
fn dot_dot_range_syntax_parses() {
    let e = parse_expr("1..2").expect("parse");
    assert!(matches!(e, Expr::Range(_, _, _)));
    for src in [
        "fn main() { print(1..2) }",
        "fn main() { for i in 1..10 { } }",
        "fn main() { let xs = [1..3] }",
        "fn main() { let x = 0..0 }",
        "fn main() { let r = 0..10 }",
        // `..` binds looser than arithmetic, so both bounds are expressions.
        "fn main() { let r = 1 + 2..n - 1 }",
    ] {
        assert!(parse(src).is_ok(), "expected {src:?} to parse");
    }
    // The builtin form and the sugar coexist.
    assert!(parse("fn main() { for i in range(1, 10) { } }").is_ok());
}

/// A range needs both bounds: a dangling `..` or a trailing operator is a
/// deterministic `E1006`, and `..` is never a float.
#[test]
fn malformed_range_is_syntax_error() {
    for src in [
        "fn main() { print(1..) }",
        "fn main() { print(..2) }",
        "fn main() { let x = 1...2 }",
        "fn main() { let x = 1.. }",
    ] {
        assert_eq!(
            parse(src).map_err(|d| d.code),
            Err(codes::EXPECTED),
            "expected E1006 for {src:?}"
        );
    }
    // A single dot still means field access, not a range.
    assert!(parse_expr("a.b").is_ok());
}

/// Module items are declarations and expression statements only
/// (`docs/grammar.md`): a top-level loop is `E1006`, not a silent no-op.
#[test]
fn top_level_control_flow_is_not_an_item() {
    assert_eq!(
        parse("for i in range(3) { print(i) }").map_err(|d| d.code),
        Err(codes::EXPECTED)
    );
    assert_eq!(
        parse("while true { }").map_err(|d| d.code),
        Err(codes::EXPECTED)
    );
    assert!(parse("print(1)").is_ok());
}
