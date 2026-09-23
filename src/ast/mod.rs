//! The abstract syntax tree.

use crate::error::Span;

/// Type expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// `int`
    Int,
    /// `float`
    Float,
    /// `bool`
    Bool,
    /// `string`
    String,
    /// `[T]`
    List(Box<TypeExpr>),
    /// `{K: V}`
    Map(Box<TypeExpr>, Box<TypeExpr>),
    /// `T | none`
    Optional(Box<TypeExpr>),
    /// A named user type.
    Named(String),
}

impl TypeExpr {
    /// The source spelling of this type.
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            TypeExpr::Int => "int".into(),
            TypeExpr::Float => "float".into(),
            TypeExpr::Bool => "bool".into(),
            TypeExpr::String => "string".into(),
            TypeExpr::List(t) => format!("[{}]", t.name()),
            TypeExpr::Map(k, v) => format!("{{{}: {}}}", k.name(), v.name()),
            TypeExpr::Optional(t) => format!("{} | none", t.name()),
            TypeExpr::Named(n) => n.clone(),
        }
    }
}

/// Literal values.
#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    /// Integer.
    Int(i64),
    /// Float.
    Float(f64),
    /// String.
    Str(String),
    /// Boolean.
    Bool(bool),
    /// `none`.
    None,
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Rem,
    /// `^`
    Pow,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `and`
    And,
    /// `or`
    Or,
}

impl BinOp {
    /// Source spelling.
    #[must_use]
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Pow => "^",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "and",
            BinOp::Or => "or",
        }
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// `-x`
    Neg,
    /// `not x`
    Not,
}

/// Match patterns.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// An integer literal.
    Int(i64),
    /// A string literal.
    Str(String),
    /// A boolean literal.
    Bool(bool),
    /// `none`.
    None,
    /// A binding name (lowercase) or `_`.
    Bind(String),
    /// `[p, p, ...]`.
    List(Vec<Pattern>),
    /// `Variant(p, ...)`.
    Variant(String, Vec<Pattern>),
}

impl Pattern {
    /// Names this pattern binds.
    #[must_use]
    pub fn bindings(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect(&self, out: &mut Vec<String>) {
        match self {
            Pattern::Bind(n) if n != "_" => out.push(n.clone()),
            Pattern::List(ps) | Pattern::Variant(_, ps) => {
                for p in ps {
                    p.collect(out);
                }
            }
            _ => {}
        }
    }
}

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal.
    Lit(Lit, Span),
    /// A name.
    Name(String, Span),
    /// An f-string: literal and interpolated parts.
    FStr(Vec<FPart>, Span),
    /// `-x` / `not x`.
    Unary(UnOp, Box<Expr>, Span),
    /// `a op b`.
    Binary(BinOp, Box<Expr>, Box<Expr>, Span),
    /// `callee(args)`. Arguments may be positional (`Arg { name: None, .. }`)
    /// or named (`Arg { name: Some(..), .. }`); named arguments are supported
    /// only for directly resolved user functions.
    Call(Box<Expr>, Vec<Arg>, Span),
    /// `recv.method(args)`.
    Method(Box<Expr>, String, Vec<Arg>, Span),
    /// `recv.field`.
    Field(Box<Expr>, String, Span),
    /// `base[index]`.
    Index(Box<Expr>, Box<Expr>, Span),
    /// `[a, b, c]`.
    List(Vec<Expr>, Span),
    /// `{k: v, ...}`.
    Map(Vec<(Expr, Expr)>, Span),
    /// `Variant(args)` or `Struct(field: value)`. Field names optional; the
    /// parser records named arguments as `Arg`.
    Construct(String, Vec<Arg>, Span),
    /// `(a, b)` tuple (kept minimal; single element is a group).
    Tuple(Vec<Expr>, Span),
    /// `(x, y) -> body` or `x -> body`.
    Lambda(Vec<String>, Box<Expr>, Span),
    /// `x |> f`.
    Pipe(Box<Expr>, Box<Expr>, Span),
    /// `if c { a } else { b }` as an expression.
    If(Box<Expr>, Vec<Stmt>, Option<Box<Expr>>, Span),
    /// `match value { pat -> block ... }`.
    Match(Box<Expr>, Vec<Arm>, Span),
    /// A block expression.
    Block(Vec<Stmt>, Span),
}

/// An argument, possibly named (`field: value`).
#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    /// The parameter/field name, if named.
    pub name: Option<String>,
    /// The value.
    pub value: Expr,
}

/// Part of an f-string.
#[derive(Debug, Clone, PartialEq)]
pub enum FPart {
    /// Static text.
    Lit(String),
    /// An interpolation, already parsed.
    Expr(Expr),
}

/// One `match` arm.
#[derive(Debug, Clone, PartialEq)]
pub struct Arm {
    /// The pattern.
    pub pattern: Pattern,
    /// Optional guard `if cond`.
    pub guard: Option<Expr>,
    /// The body.
    pub body: Vec<Stmt>,
}

/// A statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `let [mut] name [: T] = expr`.
    Let {
        /// Mutable.
        mutable: bool,
        /// Name.
        name: String,
        /// Annotation.
        ann: Option<TypeExpr>,
        /// Initializer.
        value: Expr,
        /// Span.
        span: Span,
    },
    /// `target = value` / `target op= value`.
    Assign {
        /// Target.
        target: Expr,
        /// Value.
        value: Expr,
        /// Compound operator.
        op: Option<BinOp>,
        /// Span.
        span: Span,
    },
    /// `expr`.
    Expr(Expr, Span),
    /// `return [expr]`.
    Return(Option<Expr>, Span),
    /// `throw expr`.
    Throw(Expr, Span),
    /// `break`.
    Break(Span),
    /// `continue`.
    Continue(Span),
    /// `while cond { body }`.
    While(Expr, Vec<Stmt>, Span),
    /// `loop { body }`.
    Loop(Vec<Stmt>, Span),
    /// `for pat in iter { body }`.
    For(Pattern, Expr, Vec<Stmt>, Span),
    /// `try { } catch e -> { } finally { }`.
    Try {
        /// Try body.
        body: Vec<Stmt>,
        /// Catch binding.
        catch: String,
        /// Catch body.
        catch_body: Vec<Stmt>,
        /// Optional finally body.
        finally: Option<Vec<Stmt>>,
        /// Span.
        span: Span,
    },
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// Name.
    pub name: String,
    /// Optional type.
    pub ty: Option<TypeExpr>,
    /// Span.
    pub span: Span,
}

/// A top-level item.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// `fn name(params) -> T { body }`.
    Fn {
        /// Name.
        name: String,
        /// Parameters.
        params: Vec<Param>,
        /// Return type.
        ret: Option<TypeExpr>,
        /// Body.
        body: Vec<Stmt>,
        /// Public.
        public: bool,
        /// Span.
        span: Span,
    },
    /// `struct Name { field: T, ... }`.
    Struct {
        /// Name.
        name: String,
        /// Fields.
        fields: Vec<(String, TypeExpr)>,
        /// Span.
        span: Span,
    },
    /// `enum Name { Variant(T), ... }`.
    Enum {
        /// Name.
        name: String,
        /// Variants.
        variants: Vec<(String, Vec<TypeExpr>)>,
        /// Span.
        span: Span,
    },
    /// `type Name = T`.
    Alias {
        /// Name.
        name: String,
        /// Target.
        target: TypeExpr,
        /// Span.
        span: Span,
    },
    /// `use path`.
    Use {
        /// Path segments.
        path: Vec<String>,
        /// Span.
        span: Span,
    },
    /// A top-level `let` (module constant).
    Const {
        /// Name.
        name: String,
        /// Annotation.
        ann: Option<TypeExpr>,
        /// Value.
        value: Expr,
        /// Span.
        span: Span,
    },
    /// A top-level expression such as `main()`.
    Expr(Expr, Span),
}

/// A whole file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    /// Items in order.
    pub items: Vec<Item>,
}
