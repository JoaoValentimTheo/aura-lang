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
    /// `none` (only meaningful as a union member)
    None,
    /// `[T]`
    List(Box<TypeExpr>),
    /// `{K: V}`
    Map(Box<TypeExpr>, Box<TypeExpr>),
    /// `T1 | T2 | ...` (two or more members; a single member is its own type)
    Union(Vec<TypeExpr>),
    /// A named user type, possibly a generic type parameter. The checker
    /// decides which by the parameters in scope; the parser cannot know.
    Named(String),
    /// `Name<T1, T2, ...>` — a parameterised type application. The name is a
    /// declared generic type (struct, enum, or alias); the arguments are the
    /// type parameters of its declaration.
    App(String, Vec<TypeExpr>),
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
            TypeExpr::None => "none".into(),
            TypeExpr::List(t) => format!("[{}]", t.name()),
            TypeExpr::Map(k, v) => format!("{{{}: {}}}", k.name(), v.name()),
            TypeExpr::Union(ms) => ms
                .iter()
                .map(TypeExpr::name)
                .collect::<Vec<_>>()
                .join(" | "),
            TypeExpr::Named(n) => n.clone(),
            TypeExpr::App(n, args) => format!(
                "{}<{}>",
                n,
                args.iter()
                    .map(TypeExpr::name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// A generic type parameter declaration: `T` or `T: Trait` / `T: A + B`.
///
/// A type parameter is a compile-time placeholder in the type namespace. It is
/// never a runtime value. The declaration order defines each parameter's
/// identity, so `f<T>` and `f<U>` are the same signature (alpha-equivalence):
/// the spelling is not part of the signature.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    /// Parameter name.
    pub name: String,
    /// Trait bounds, in declaration order. An empty list means unbounded.
    pub bounds: Vec<String>,
    /// Source span of the parameter declaration.
    pub span: Span,
}

/// A source span for a whole generic parameter list, used by diagnostics that
/// name the list rather than one parameter.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TypeParams {
    /// The declared parameters, in order.
    pub params: Vec<TypeParam>,
}

impl TypeParams {
    /// Whether the list is empty (a non-generic declaration).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// The parameter names, in declaration order.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.params.iter().map(|p| p.name.clone()).collect()
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
    /// `&`
    BitAnd,
    /// `|`
    BitOr,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
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
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
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
    /// `~x`
    BitNot,
}

/// A struct field declaration: a name, a type, and its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    /// Field name.
    pub name: String,
    /// Field type.
    pub ty: TypeExpr,
    /// Whether the field is visible outside its module (`pub`). A field with
    /// no `pub` is private to its struct's module: it cannot be named in a
    /// construction or read or written from outside that module
    /// (`LANGUAGE_SPEC.md` §28).
    pub public: bool,
    /// Source span of the field declaration.
    pub span: Span,
}

/// An enum variant declaration: a tag, its payload types, and its span.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantDecl {
    /// Variant tag.
    pub tag: String,
    /// Payload types.
    pub payload: Vec<TypeExpr>,
    /// Source span of the variant declaration.
    pub span: Span,
}

/// Match patterns.
///
/// Every pattern carries the source span of the text that produced it, so a
/// diagnostic about a pattern (an unknown variant tag, a duplicate binding, an
/// arity mismatch) points at the real location rather than the file start
/// (`LANGUAGE_SPEC.md` §30.3).
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// An integer literal.
    Int(i64, Span),
    /// A string literal.
    Str(String, Span),
    /// A boolean literal.
    Bool(bool, Span),
    /// `none`.
    None(Span),
    /// A binding name (lowercase) or `_`.
    Bind(String, Span),
    /// `[p, p, ...]`.
    List(Vec<Pattern>, Span),
    /// `Variant(p, ...)`.
    Variant(String, Vec<Pattern>, Span),
}

impl Pattern {
    /// The source span of this pattern.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Pattern::Int(_, s)
            | Pattern::Str(_, s)
            | Pattern::Bool(_, s)
            | Pattern::None(s)
            | Pattern::Bind(_, s)
            | Pattern::List(_, s)
            | Pattern::Variant(_, _, s) => *s,
        }
    }

    /// Names this pattern binds.
    #[must_use]
    pub fn bindings(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect(&self, out: &mut Vec<String>) {
        match self {
            Pattern::Bind(n, _) if n != "_" => out.push(n.clone()),
            Pattern::List(ps, _) | Pattern::Variant(_, ps, _) => {
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
    ///
    /// `ty_args` holds explicit generic type arguments (`f<int>(x)`), empty
    /// when none are written. They are checked against the callee's declared
    /// parameters and seed inference before the argument types are matched.
    Call(Box<Expr>, Vec<Arg>, Vec<TypeExpr>, Span),
    /// `recv.method(args)`, with optional explicit type arguments
    /// (`recv.map<int>(f)`).
    Method(Box<Expr>, String, Vec<Arg>, Vec<TypeExpr>, Span),
    /// `recv.field`.
    Field(Box<Expr>, String, Span),
    /// `base[index]`.
    Index(Box<Expr>, Box<Expr>, Span),
    /// `[a, b, c]`.
    List(Vec<Expr>, Span),
    /// `{k: v, ...}`.
    Map(Vec<(Expr, Expr)>, Span),
    /// `Variant(args)` or `Struct(field: value)`. Field names optional; the
    /// parser records named arguments as `Arg`. `ty_args` holds explicit
    /// generic type arguments on a parameterised construction
    /// (`Box<int> { value: 1 }`), empty when none are written.
    Construct(String, Vec<Arg>, Vec<TypeExpr>, Span),
    /// `(a, b)` tuple (kept minimal; single element is a group).
    Tuple(Vec<Expr>, Span),
    /// `(x: int, mut y) -> body` or `x -> body`. A lambda shares the function
    /// parameter model (`Param`: name, optional annotation, `mut`), so
    /// functions and lambdas are one semantic model (`LANGUAGE_SPEC.md` §15.4).
    Lambda(Vec<Param>, Box<Expr>, Span),
    /// `x |> f`.
    Pipe(Box<Expr>, Box<Expr>, Span),
    /// `start..end` — a Rust-style half-open range expression. Equivalent to
    /// `range(start, end)` and evaluating to the same Range value.
    Range(Box<Expr>, Box<Expr>, Span),
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
    /// An interpolation: the expression and its optional format specification.
    Expr(Expr, Option<FormatSpec>),
}

/// A parsed f-string format specification: `{value:spec}`.
///
/// The mini-language is deliberately small and orthogonal: an optional fill
/// and alignment, an optional sign, then width, precision, and type, in that
/// order. This is the whole language; anything else is a diagnostic.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormatSpec {
    /// Fill character, default a space. Only meaningful with an alignment.
    pub fill: Option<char>,
    /// Alignment: `<`, `>`, or `^`.
    pub align: Option<Align>,
    /// Sign for numbers: `+` (always), `-` (only negatives), or ` ` (space).
    pub sign: Option<Sign>,
    /// Minimum field width.
    pub width: Option<usize>,
    /// Precision: digits after the decimal point.
    pub precision: Option<usize>,
    /// Presentation type.
    pub ty: Option<FormatType>,
    /// Source span of the whole spec (for diagnostics).
    pub span: Span,
}

/// Alignment in a format spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// `<` — left.
    Left,
    /// `>` — right.
    Right,
    /// `^` — center.
    Center,
}

/// Sign in a format spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// ` ` (space)
    Space,
}

/// Presentation type in a format spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatType {
    /// `d` — decimal integer.
    Dec,
    /// `b` — binary.
    Binary,
    /// `o` — octal.
    Octal,
    /// `x` / `X` — hexadecimal (case of the digits).
    Hex {
        /// Uppercase digits.
        upper: bool,
    },
    /// `f` / `F` — fixed-point float.
    Fixed,
    /// `e` / `E` — scientific notation.
    Exp,
    /// `%` — percentage.
    Percent,
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
    /// `let pattern = expr` (destructuring), where the pattern is a list or
    /// variant pattern (possibly nested) as defined by §4.7 of the
    /// specification. All names it binds are immutable.
    LetPattern {
        /// The destructuring pattern.
        pattern: Pattern,
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
    /// `try { } catch e { } finally { }`.
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
    /// Whether the parameter is declared `mut`, granting mutable capability
    /// over the bound name inside the body (and, for a receiver, over the
    /// caller's value at the call site).
    pub mutable: bool,
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
        /// Generic type parameters, in declaration order (empty when none).
        type_params: Vec<TypeParam>,
        /// Parameters.
        params: Vec<Param>,
        /// Return type.
        ret: Option<TypeExpr>,
        /// Span of the return type annotation, when one is written. Used so an
        /// unknown return type is reported at the annotation, not the item
        /// (`LANGUAGE_SPEC.md` §17.6, §17.7).
        ret_span: Option<Span>,
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
        /// Generic type parameters, in declaration order (empty when none).
        type_params: Vec<TypeParam>,
        /// Fields, each with its source span.
        fields: Vec<FieldDecl>,
        /// Whether the type is visible outside its module (`pub`).
        public: bool,
        /// Span.
        span: Span,
    },
    /// `enum Name { Variant(T), ... }`.
    Enum {
        /// Name.
        name: String,
        /// Generic type parameters, in declaration order (empty when none).
        type_params: Vec<TypeParam>,
        /// Variants, each with its source span.
        variants: Vec<VariantDecl>,
        /// Whether the type is visible outside its module (`pub`).
        public: bool,
        /// Span.
        span: Span,
    },
    /// `type Name = T`.
    Alias {
        /// Name.
        name: String,
        /// Generic type parameters, in declaration order (empty when none).
        type_params: Vec<TypeParam>,
        /// Target.
        target: TypeExpr,
        /// Whether the type is visible outside its module (`pub`).
        public: bool,
        /// Span.
        span: Span,
    },
    /// `use path [as Alias]` — an import (`pub use` also re-exports the
    /// imported name).
    Use {
        /// Path segments.
        path: Vec<String>,
        /// Local name to bind the import to, when `as Alias` is written.
        alias: Option<String>,
        /// Whether the imported name is re-exported (`pub use`).
        public: bool,
        /// Span.
        span: Span,
    },
    /// `module Name { items }` — an in-source module (`LANGUAGE_SPEC.md` §28).
    /// A module is a real visibility boundary: its items are private by default
    /// and reachable from outside only when marked `pub`. Modules nest, and
    /// each has a canonical dotted path from the file root. The resolver
    /// flattens a parsed module tree into top-level items with path-qualified
    /// names before checking, so no `Item::Module` reaches the checker.
    Module {
        /// Module name (an ordinary identifier; capitalized by convention).
        name: String,
        /// Items declared inside the module.
        items: Vec<Item>,
        /// Whether the module is visible outside its parent (`pub module`).
        public: bool,
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
        /// Whether the constant is visible outside its module (`pub`).
        public: bool,
        /// Span.
        span: Span,
    },
    /// A top-level expression such as `main()`.
    Expr(Expr, Span),
    /// `impl Target { fn method(self, ...) { ... } }` — the behavior block
    /// attached to an already-declared nominal struct (`LANGUAGE_SPEC.md`
    /// §17.6), or `impl Trait for Target { ... }` — a trait implementation
    /// (§17.7). `methods` holds only [`Item::Fn`] values whose first parameter
    /// is the explicit receiver `self`; the parser guarantees this shape, so
    /// no second function type is introduced. `trait_name` is `Some` for the
    /// trait-implementation form.
    Impl {
        /// The nominal struct this behavior block belongs to.
        target: String,
        /// Type arguments applied to the target, when this is
        /// `impl<T> Trait<T> for Name<T>`. Empty for a plain `impl Name`.
        target_args: Vec<TypeExpr>,
        /// The trait being implemented, when this is `impl Trait for Target`.
        trait_name: Option<String>,
        /// Type arguments applied to the trait, when this is
        /// `impl<T> Container<T> for Stack<T>`. Empty otherwise.
        trait_args: Vec<TypeExpr>,
        /// Generic type parameters declared on the block (`impl<T> ...`),
        /// in declaration order (empty when none).
        type_params: Vec<TypeParam>,
        /// Methods, each an [`Item::Fn`] with `self` as its first parameter.
        methods: Vec<Item>,
        /// The canonical module path the block was declared in, filled by the
        /// resolver (empty until then). A method's visibility is relative to
        /// this module (`LANGUAGE_SPEC.md` §28).
        owner: Vec<String>,
        /// Span.
        span: Span,
    },
    /// `trait Name { fn method(self, ...) -> T ... }` — a behavioral contract
    /// (`LANGUAGE_SPEC.md` §17.7). Declarations only: each entry is an
    /// [`Item::Fn`] with a `self` receiver and no body. A trait introduces no
    /// value type and no dispatch mechanism.
    Trait {
        /// Name.
        name: String,
        /// Generic type parameters, in declaration order (empty when none).
        type_params: Vec<TypeParam>,
        /// Declared method signatures, each an [`Item::Fn`] with an empty body.
        methods: Vec<Item>,
        /// Whether the trait is visible outside its module (`pub`).
        public: bool,
        /// The canonical module path the trait was declared in, filled by the
        /// resolver (empty until then).
        owner: Vec<String>,
        /// Span.
        span: Span,
    },
}

/// A whole file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    /// Items in order.
    pub items: Vec<Item>,
}
