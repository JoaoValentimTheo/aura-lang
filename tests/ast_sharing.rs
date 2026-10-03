#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! B-1R3A-ARCH-1: `Arc`-based AST sharing is `Send`-compatible and
//! semantically neutral.
//!
//! The AST subexpressions and bodies are `Arc`-shared so a future iterative
//! evaluator can retain pending nodes in continuations (design §16.1). `Arc`
//! (not `Rc`) is required because the parser and the `Compilation` execution
//! path move the `Module` across a native worker-thread boundary
//! (`on_parse_stack`/`on_execution_stack`, `T: Send + 'static`). See
//! `docs/B1R3A_AST_SHARING_DECISION.md`.
//!
//! These tests prove two things independently of the oracle:
//!
//! 1. the boundary-critical AST graph is `Send` (a compile-time property); and
//! 2. the parser produces the same semantic tree it did before the ownership
//!    swap — same node kinds, spans, and ordering — and clones share nodes
//!    rather than deep-copying them.

use std::sync::Arc;

use aura::ast::*;
use aura::parse::{parse, parse_expr};

/// Compile-time: the whole AST graph is `Send + Sync`, so `Module` still
/// crosses the native execution-thread boundary.
#[test]
fn ast_graph_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Module>();
    assert_send_sync::<Item>();
    assert_send_sync::<Stmt>();
    assert_send_sync::<Expr>();
    assert_send_sync::<TypeExpr>();
    assert_send_sync::<Pattern>();
    assert_send_sync::<Arm>();
    assert_send_sync::<Arg>();
    assert_send_sync::<FPart>();
    assert_send_sync::<Param>();
}

/// `Arc` sharing: cloning an AST node bumps a refcount, it does not deep-copy.
#[test]
fn ast_clone_shares_nodes() {
    let module = parse("fn f() { g(h(1), i(2)) }").expect("parse");
    let clone = module.clone();
    // The clone shares the same backing allocations as the original.
    let Item::Fn { body, .. } = &module.items[0] else {
        panic!("expected fn");
    };
    let Item::Fn {
        body: clone_body, ..
    } = &clone.items[0]
    else {
        panic!("expected fn");
    };
    assert!(
        std::ptr::eq(Arc::as_ptr(body), Arc::as_ptr(clone_body)),
        "cloning a Module must share the Arc-backed body, not deep-copy it",
    );
}

/// Parser semantic equivalence: ordering, spans, and node shape are unchanged.
#[test]
fn parse_ordering_and_spans_are_preserved() {
    // Function-argument order.
    let e = parse_expr("f(a(), b(), c())").expect("parse");
    let Expr::Call(callee, args, ty_args, _) = &e else {
        panic!("expected call, got {e:?}");
    };
    assert!(matches!(callee.as_ref(), Expr::Name(n, _) if n == "f"));
    assert!(ty_args.is_empty());
    let names: Vec<&str> = args
        .iter()
        .map(|a| match &a.value {
            Expr::Call(c, _, _, _) => match c.as_ref() {
                Expr::Name(n, _) => n.as_str(),
                _ => "?",
            },
            _ => "?",
        })
        .collect();
    assert_eq!(names, ["a", "b", "c"], "argument order must be preserved");

    // Statement order in a block.
    let module = parse("fn f() { let x = 1\n let y = 2\n print(x + y) }").expect("parse");
    let Item::Fn { body, .. } = &module.items[0] else {
        panic!("expected fn");
    };
    assert_eq!(body.len(), 3, "statement order/count must be preserved");

    // Match-arm order and body.
    let e = parse_expr("match x { A() -> 1\n B() -> 2\n C() -> 3 }").expect("parse");
    let Expr::Match(_, arms, _) = &e else {
        panic!("expected match, got {e:?}");
    };
    assert_eq!(arms.len(), 3, "match-arm order/count must be preserved");
    for arm in arms.iter() {
        assert_eq!(arm.body.len(), 1);
    }

    // Spans still point at real source bytes (`"1 + 2"`): operands cover their
    // literals, the `Binary` span is the operator.
    let e = parse_expr("1 + 2").expect("parse");
    let Expr::Binary(_, l, r, span) = &e else {
        panic!("expected binary, got {e:?}");
    };
    assert_eq!(l.span(), aura::error::Span { start: 0, end: 1 });
    assert_eq!(r.span(), aura::error::Span { start: 4, end: 5 });
    assert_eq!(*span, aura::error::Span { start: 2, end: 3 });
}

/// Collections keep their element identity through the shared representation.
#[test]
fn parse_collections_and_fstrings_are_preserved() {
    let e = parse_expr("[1, 2, 3]").expect("parse");
    let Expr::List(items, _) = &e else {
        panic!("expected list, got {e:?}");
    };
    assert_eq!(items.len(), 3);

    let e = parse_expr("{\"a\": 1, \"b\": 2}").expect("parse");
    let Expr::Map(entries, _) = &e else {
        panic!("expected map, got {e:?}");
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries[0].0,
        Expr::Lit(Lit::Str("a".to_string()), entries[0].0.span())
    );

    let e = parse_expr("f\"{a} and {b}\"").expect("parse");
    let Expr::FStr(parts, _) = &e else {
        panic!("expected fstring, got {e:?}");
    };
    assert_eq!(parts.len(), 3, "lit, expr, lit");

    // PartialEq compares contents even through Arc: two independent parses of
    // the same source are equal.
    let a = parse("[1, 2, 3]").expect("parse");
    let b = parse("[1, 2, 3]").expect("parse");
    assert_eq!(a, b, "content equality must be preserved through Arc");
}

/// The parser runs on its own thread and returns the `Module` across the
/// boundary; this exercises that end to end (the property the `Arc` choice
/// exists to preserve).
#[test]
fn module_crosses_the_execution_thread_boundary() {
    let module = parse("fn main() { print(1) }").expect("parse");
    let handle = std::thread::spawn(move || module.items.len());
    assert_eq!(handle.join().unwrap(), 1);
}
