//! Grammar-aware *direct* AST generation for the checker fuzz target.
//!
//! Included via `#[path]` from `fuzz_targets/checker.rs`. Where the raw-byte
//! path exercises `bytes -> lexer -> parser -> checker`, this path builds the
//! AST *directly* and hands it to the checker, so the checker is reached on
//! structurally valid inputs that the grammar does not happen to produce from
//! random bytes (deeply mixed expression/statement shapes, unusual but legal
//! nesting, empty bodies, etc.).
//!
//! ## Structural invariants respected
//!
//! The generator only constructs nodes the parser itself can produce, so the
//! checker sees the same representation invariants it always sees:
//!
//! * every `Expr::Call` has a callee and a (possibly empty) argument list;
//! * `Stmt::Let` always has an initializer; `Stmt::Assign` always has a target;
//! * `Param` names are non-empty identifiers;
//! * spans are well-formed (`end >= start`) but otherwise arbitrary — spans are
//!   diagnostics metadata, not semantic input;
//! * `main` is present, so `mode = program` checking (which the CLI uses) is
//!   meaningful; this generator checks in module mode like the checker target.
//!
//! A checker *rejection* of a generated AST is a legitimate result (the AST may
//! be type-incorrect — undefined names, arity mismatches). It is never silently
//! swallowed as if it were ordinary fuzz noise: the target asserts the result is
//! a structured diagnostic and never a panic or an internal error.

#![allow(dead_code)]

use std::sync::Arc;

use aura::ast::*;
use aura::error::Span;

/// The deterministic PRNG shared with the program generator (splitmix64), so a
/// single `u64` seed reproduces the whole AST.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }
}

fn sp(rng: &mut Rng) -> Span {
    let start = rng.below(64);
    Span::new(start, start + rng.below(8))
}

/// Build a structurally valid module for `seed`.
pub fn module(seed: u64) -> Module {
    let mut rng = Rng::new(seed);
    let mut items: Vec<Item> = Vec::new();

    // 0..=3 auxiliary declarations of random kinds.
    let extras = rng.below(4);
    for i in 0..extras {
        items.push(gen_item(&mut rng, i));
    }

    // A `main` is always present so program-mode checking is meaningful.
    let body = gen_block(&mut rng, 0, 3);
    items.push(Item::Fn {
        name: "main".to_string(),
        type_params: Vec::new(),
        params: Vec::new(),
        ret: None,
        ret_span: None,
        body,
        public: false,
        span: sp(&mut rng),
    });

    Module { items }
}

fn gen_item(rng: &mut Rng, i: usize) -> Item {
    match rng.below(5) {
        0 => Item::Fn {
            name: format!("f{i}"),
            type_params: gen_type_params(rng),
            params: gen_params(rng),
            ret: gen_ret(rng),
            ret_span: None,
            body: gen_block(rng, 0, 2),
            public: rng.chance(50),
            span: sp(rng),
        },
        1 => Item::Struct {
            name: format!("S{i}"),
            type_params: gen_type_params(rng),
            fields: (0..rng.below(3))
                .map(|j| FieldDecl {
                    name: format!("field{j}"),
                    ty: gen_type(rng, 0),
                    public: rng.chance(50),
                    span: sp(rng),
                })
                .collect(),
            public: rng.chance(50),
            span: sp(rng),
        },
        2 => Item::Enum {
            name: format!("E{i}"),
            type_params: gen_type_params(rng),
            variants: (0..(1 + rng.below(3)))
                .map(|j| VariantDecl {
                    tag: format!("V{j}"),
                    payload: (0..rng.below(3)).map(|_| gen_type(rng, 0)).collect(),
                    span: sp(rng),
                })
                .collect(),
            public: rng.chance(50),
            span: sp(rng),
        },
        3 => Item::Alias {
            name: format!("A{i}"),
            type_params: gen_type_params(rng),
            target: gen_type(rng, 0),
            public: rng.chance(50),
            span: sp(rng),
        },
        _ => Item::Const {
            name: format!("C{i}"),
            ann: if rng.chance(50) {
                Some(gen_type(rng, 0))
            } else {
                None
            },
            value: gen_expr(rng, 0),
            public: rng.chance(50),
            span: sp(rng),
        },
    }
}

fn gen_type_params(rng: &mut Rng) -> Vec<TypeParam> {
    (0..rng.below(3))
        .map(|j| TypeParam {
            name: format!("T{j}"),
            bounds: Vec::new(),
            span: sp(rng),
        })
        .collect()
}

fn gen_params(rng: &mut Rng) -> Vec<Param> {
    (0..rng.below(3))
        .map(|j| Param {
            name: format!("p{j}"),
            ty: if rng.chance(60) {
                Some(gen_type(rng, 0))
            } else {
                None
            },
            mutable: rng.chance(30),
            span: sp(rng),
        })
        .collect()
}

fn gen_ret(rng: &mut Rng) -> Option<TypeExpr> {
    if rng.chance(50) {
        Some(gen_type(rng, 0))
    } else {
        None
    }
}

/// A type expression, bounded to depth 3 (well within the semantic limit; this
/// generator deliberately does **not** produce TypeExpr-heavy inputs, which are
/// excluded pending AUDIT-3).
fn gen_type(rng: &mut Rng, depth: usize) -> TypeExpr {
    let leaves = [
        TypeExpr::Int,
        TypeExpr::Float,
        TypeExpr::Bool,
        TypeExpr::String,
        TypeExpr::Named("T0".to_string()),
    ];
    if depth >= 3 {
        return rng.pick(&leaves).clone();
    }
    match rng.below(8) {
        0 => TypeExpr::List(Box::new(gen_type(rng, depth + 1))),
        1 => TypeExpr::Map(
            Box::new(TypeExpr::String),
            Box::new(gen_type(rng, depth + 1)),
        ),
        2 => TypeExpr::Union(vec![gen_type(rng, depth + 1), gen_type(rng, depth + 1)]),
        3 => TypeExpr::App("S0".to_string(), vec![gen_type(rng, depth + 1)]),
        _ => rng.pick(&leaves).clone(),
    }
}

fn gen_block(rng: &mut Rng, depth: usize, max: usize) -> Arc<[Stmt]> {
    let n = rng.below(max + 1);
    (0..n)
        .map(|_| gen_stmt(rng, depth))
        .collect::<Vec<_>>()
        .into()
}

fn gen_stmt(rng: &mut Rng, depth: usize) -> Stmt {
    match rng.below(10) {
        0 => Stmt::Let {
            mutable: rng.chance(50),
            name: format!("v{}", rng.below(8)),
            ann: if rng.chance(40) {
                Some(gen_type(rng, 0))
            } else {
                None
            },
            value: gen_expr(rng, depth),
            span: sp(rng),
        },
        1 => Stmt::Assign {
            target: gen_expr(rng, depth),
            value: gen_expr(rng, depth),
            op: None,
            span: sp(rng),
        },
        2 => Stmt::Expr(gen_expr(rng, depth), sp(rng)),
        3 => Stmt::Return(
            if rng.chance(70) {
                Some(gen_expr(rng, depth))
            } else {
                None
            },
            sp(rng),
        ),
        4 => Stmt::Throw(gen_expr(rng, depth), sp(rng)),
        5 => {
            if depth < 3 {
                Stmt::While(gen_expr(rng, depth), gen_block(rng, depth + 1, 2), sp(rng))
            } else {
                Stmt::Break(sp(rng))
            }
        }
        6 => {
            if depth < 3 {
                Stmt::For(
                    Pattern::Bind(format!("it{}", rng.below(4)), sp(rng)),
                    gen_expr(rng, depth),
                    gen_block(rng, depth + 1, 2),
                    sp(rng),
                )
            } else {
                Stmt::Continue(sp(rng))
            }
        }
        7 => {
            if depth < 3 {
                Stmt::Try {
                    body: gen_block(rng, depth + 1, 2),
                    catch: Pattern::Bind("e".to_string(), sp(rng)),
                    catch_body: gen_block(rng, depth + 1, 2),
                    finally: if rng.chance(40) {
                        Some(gen_block(rng, depth + 1, 1))
                    } else {
                        None
                    },
                    span: sp(rng),
                }
            } else {
                Stmt::Expr(gen_expr(rng, depth), sp(rng))
            }
        }
        8 => {
            if depth < 3 {
                Stmt::LetPattern {
                    pattern: Pattern::List(
                        vec![Pattern::Bind(format!("a{}", rng.below(4)), sp(rng))],
                        sp(rng),
                    ),
                    value: gen_expr(rng, depth),
                    span: sp(rng),
                }
            } else {
                Stmt::Expr(gen_expr(rng, depth), sp(rng))
            }
        }
        _ => Stmt::Expr(gen_expr(rng, depth), sp(rng)),
    }
}

fn gen_expr(rng: &mut Rng, depth: usize) -> Expr {
    if depth >= 3 {
        return gen_leaf(rng);
    }
    match rng.below(14) {
        0 => Expr::Unary(
            *rng.pick(&[UnOp::Neg, UnOp::Not, UnOp::BitNot]),
            Arc::new(gen_expr(rng, depth + 1)),
            sp(rng),
        ),
        1 => Expr::Binary(
            *rng.pick(&[
                BinOp::Add,
                BinOp::Sub,
                BinOp::Mul,
                BinOp::Div,
                BinOp::Rem,
                BinOp::Pow,
                BinOp::Eq,
                BinOp::Ne,
                BinOp::Lt,
                BinOp::Le,
                BinOp::Gt,
                BinOp::Ge,
                BinOp::And,
                BinOp::Or,
                BinOp::BitAnd,
                BinOp::BitOr,
                BinOp::Shl,
                BinOp::Shr,
            ]),
            Arc::new(gen_expr(rng, depth + 1)),
            Arc::new(gen_expr(rng, depth + 1)),
            sp(rng),
        ),
        2 => Expr::List(
            (0..rng.below(3))
                .map(|_| gen_expr(rng, depth + 1))
                .collect::<Vec<_>>()
                .into(),
            sp(rng),
        ),
        3 => Expr::Map(
            (0..rng.below(3))
                .map(|_| (gen_expr(rng, depth + 1), gen_expr(rng, depth + 1)))
                .collect::<Vec<_>>()
                .into(),
            sp(rng),
        ),
        4 => Expr::Call(
            Arc::new(Expr::Name(format!("f{}", rng.below(2)), sp(rng))),
            (0..rng.below(3))
                .map(|_| Arg {
                    name: None,
                    value: gen_expr(rng, depth + 1),
                })
                .collect::<Vec<_>>()
                .into(),
            Vec::new(),
            sp(rng),
        ),
        5 => Expr::Method(
            Arc::new(gen_expr(rng, depth + 1)),
            rng.pick(&["len", "push", "map", "unknown_method"])
                .to_string(),
            (0..rng.below(2))
                .map(|_| Arg {
                    name: None,
                    value: gen_expr(rng, depth + 1),
                })
                .collect::<Vec<_>>()
                .into(),
            Vec::new(),
            sp(rng),
        ),
        6 => Expr::Field(
            Arc::new(gen_expr(rng, depth + 1)),
            "field0".to_string(),
            sp(rng),
        ),
        7 => Expr::Index(
            Arc::new(gen_expr(rng, depth + 1)),
            Arc::new(gen_expr(rng, depth + 1)),
            sp(rng),
        ),
        8 => Expr::If(
            Arc::new(gen_expr(rng, depth + 1)),
            gen_block(rng, depth + 1, 1),
            if rng.chance(50) {
                Some(Arc::new(gen_expr(rng, depth + 1)))
            } else {
                None
            },
            sp(rng),
        ),
        9 => Expr::Pipe(
            Arc::new(gen_expr(rng, depth + 1)),
            Arc::new(Expr::Name("len".to_string(), sp(rng))),
            sp(rng),
        ),
        10 => Expr::Range(
            Arc::new(gen_expr(rng, depth + 1)),
            Arc::new(gen_expr(rng, depth + 1)),
            sp(rng),
        ),
        11 => Expr::Construct(
            format!("S{}", rng.below(2)),
            (0..rng.below(2))
                .map(|j| Arg {
                    name: Some(format!("field{j}")),
                    value: gen_expr(rng, depth + 1),
                })
                .collect::<Vec<_>>()
                .into(),
            Vec::new(),
            sp(rng),
        ),
        12 => Expr::Lambda(
            (0..(1 + rng.below(2)))
                .map(|j| Param {
                    name: format!("q{j}"),
                    ty: None,
                    mutable: false,
                    span: sp(rng),
                })
                .collect(),
            Arc::new(gen_expr(rng, depth + 1)),
            sp(rng),
        ),
        _ => gen_leaf(rng),
    }
}

fn gen_leaf(rng: &mut Rng) -> Expr {
    match rng.below(8) {
        0 => Expr::Lit(Lit::Int(rng.below(100) as i64), sp(rng)),
        1 => Expr::Lit(Lit::Float(rng.below(100) as f64 / 4.0), sp(rng)),
        2 => Expr::Lit(Lit::Bool(rng.chance(50)), sp(rng)),
        3 => Expr::Lit(Lit::None, sp(rng)),
        4 => Expr::Lit(Lit::Str(format!("s{}", rng.below(8))), sp(rng)),
        _ => Expr::Name(format!("n{}", rng.below(6)), sp(rng)),
    }
}
