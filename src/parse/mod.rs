//! Recursive-descent + Pratt parser.

use crate::ast::*;
use crate::error::{codes, Diag, Result, Span};
use crate::lex::lex;
use crate::lex::token::{Tok, Token};

/// Parse a full file.
///
/// # Errors
/// Returns the first lexer, parser, or nesting-limit diagnostic.
pub fn parse(src: &str) -> Result<Module> {
    on_parse_stack(src, parse_inner)
}

/// Parse one expression (used by the REPL and tests).
///
/// # Errors
/// Returns the first lexer, parser, or nesting-limit diagnostic.
pub fn parse_expr(src: &str) -> Result<Expr> {
    on_parse_stack(src, parse_expr_inner)
}

/// Parse exactly one statement. Unlike module items, a bare `let mut` is a
/// valid statement here, which is what the REPL needs.
///
/// # Errors
/// Returns the first lexer, parser, or nesting-limit diagnostic.
pub fn parse_stmt(src: &str) -> Result<Stmt> {
    on_parse_stack(src, parse_stmt_inner)
}

/// Run a parsing function on the execution substrate, so deep (but bounded)
/// input does not overflow a small caller stack.
///
/// This delegates to [`crate::on_execution_stack`], which uses a dedicated
/// 64 MiB stack on native and runs inline on WebAssembly. The parser's
/// recursion budget ([`parse_recursion_budget`]) is calibrated per substrate
/// so that over-deep grouping is reported as `E1015` rather than trapping.
fn on_parse_stack<T, F>(src: &str, f: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&str) -> Result<T> + Send + 'static,
{
    let src = src.to_string();
    crate::on_execution_stack(move || f(&src))
}

fn parse_inner(src: &str) -> Result<Module> {
    let toks = lex(src)?;
    let mut p = Parser {
        toks,
        pos: 0,
        depth: 0,
        expr_nodes: 0,
    };
    let module = p.module()?;
    // Reject trees deeper than the language limit here, before any later
    // phase clones or walks them on an arbitrary stack. The check is
    // iterative, so it cannot itself overflow.
    enforce_depth(&module)?;
    Ok(module)
}

fn parse_expr_inner(src: &str) -> Result<Expr> {
    let toks = lex(src)?;
    let mut p = Parser {
        toks,
        pos: 0,
        depth: 0,
        expr_nodes: 0,
    };
    p.skip_newlines();
    let e = p.expr()?;
    p.skip_newlines();
    p.expect_eof()?;
    // Enforce the language nesting limit for this entry point too, so a
    // statement/expression parsed for the REPL cannot exceed the same bound
    // as a module.
    check_expr_depth(&e, 1)?;
    Ok(e)
}

fn parse_stmt_inner(src: &str) -> Result<Stmt> {
    let toks = lex(src)?;
    let mut p = Parser {
        toks,
        pos: 0,
        depth: 0,
        expr_nodes: 0,
    };
    p.skip_newlines();
    let s = p.stmt()?;
    p.skip_newlines();
    p.expect_eof()?;
    // Enforce the language nesting limit for this entry point too.
    check_stmt_depth(std::slice::from_ref(&s), 1)?;
    Ok(s)
}

/// Language limit on AST nesting. This is the single bound that keeps every
/// later phase (clone, check, evaluate, drop) safe on a small stack.
pub const MAX_AST_DEPTH: usize = 256;

/// The parser's recursion backstop: an implementation-safety bound on the
/// depth of grouping tokens (parentheses), which add no AST depth and so are
/// not governed by [`MAX_AST_DEPTH`]. It is measured in recursive-descent
/// frames, not AST nodes.
///
/// This is **not** a language limit; it protects the host stack. It is
/// substrate-calibrated:
///
/// * Native runs the parser on a dedicated 64 MiB stack, so the backstop is
///   far above anything the semantic limit can require.
/// * WebAssembly runs the parser inline on the engine stack. A program at the
///   semantic AST limit (256) costs at most ~512 parser frames, so the WASM
///   budget must exceed that to avoid rejecting programs the language permits;
///   it must also stay below the point where the engine stack overflows.
///   1024 satisfies the first: it accepts every AST-valid program plus
///   generous grouping. For the second, the runtime crate reserves a 4 MiB
///   wasm linear stack (`playground/runtime/build.rs`), because the default
///   1 MiB stack is exhausted by the most frame-expensive recursive path
///   (nested call arguments, `f(f(f(…)))`, which recurses through `expr`,
///   `unary`, `postfix`, `atom`, `call_args`, and `cons_arg`) at ~907 frames —
///   below 1024 — which trapped before `E1015` could be reported. With the
///   reserved stack the budget is reached on every path and the backstop
///   reports `E1015` instead of trapping.
///
/// Exceeding it is `E1015` on every substrate; it never redefines the
/// semantic AST limit.
#[must_use]
pub const fn parse_recursion_budget() -> usize {
    #[cfg(target_arch = "wasm32")]
    {
        1024
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        2048
    }
}

/// Iteratively verify that no expression/statement nests beyond
/// [`MAX_AST_DEPTH`]. Uses an explicit heap stack and never recurses.
fn enforce_depth(module: &Module) -> Result<()> {
    // Worklist of (node depth). Items are walked with a small recursive
    // helper bounded by the module structure; expressions and statements use
    // the explicit stack below.
    for item in &module.items {
        match item {
            Item::Fn { body, .. } => check_stmt_depth(body, 1)?,
            Item::Const { value, .. } => check_expr_depth(value, 1)?,
            Item::Expr(e, _) => check_expr_depth(e, 1)?,
            _ => {}
        }
    }
    Ok(())
}

fn check_expr_depth(root: &Expr, start: usize) -> Result<()> {
    let mut stack: Vec<(&Expr, usize)> = vec![(root, start)];
    while let Some((e, depth)) = stack.pop() {
        if depth > MAX_AST_DEPTH {
            return Err(Diag::new(
                codes::NESTING,
                "expression nests too deeply",
                Span::default(),
            ));
        }
        let d = depth + 1;
        match e {
            Expr::Lit(..) | Expr::Name(..) | Expr::FStr(..) => {}
            Expr::Unary(_, o, _) => stack.push((o, d)),
            Expr::Binary(_, l, r, _) => {
                stack.push((l, d));
                stack.push((r, d));
            }
            Expr::Call(f, args, _) => {
                stack.push((f, d));
                for a in args {
                    stack.push((&a.value, d));
                }
            }
            Expr::Method(r, _, args, _) => {
                stack.push((r, d));
                for a in args {
                    stack.push((&a.value, d));
                }
            }
            Expr::Field(r, _, _) => stack.push((r, d)),
            Expr::Index(b, i, _) => {
                stack.push((b, d));
                stack.push((i, d));
            }
            Expr::List(items, _) | Expr::Tuple(items, _) => {
                for i in items {
                    stack.push((i, d));
                }
            }
            Expr::Map(entries, _) => {
                for (k, v) in entries {
                    stack.push((k, d));
                    stack.push((v, d));
                }
            }
            Expr::Construct(_, args, _) => {
                for a in args {
                    stack.push((&a.value, d));
                }
            }
            Expr::Lambda(_, body, _) => stack.push((body, d)),
            Expr::Pipe(l, r, _) => {
                stack.push((l, d));
                stack.push((r, d));
            }
            Expr::Range(l, r, _) => {
                stack.push((l, d));
                stack.push((r, d));
            }
            Expr::If(c, then, els, _) => {
                stack.push((c, d));
                check_stmt_depth(then, d)?;
                if let Some(e) = els {
                    stack.push((e, d));
                }
            }
            Expr::Match(subject, arms, _) => {
                stack.push((subject, d));
                for arm in arms {
                    if let Some(g) = &arm.guard {
                        stack.push((g, d));
                    }
                    check_stmt_depth(&arm.body, d)?;
                }
            }
            Expr::Block(stmts, _) => check_stmt_depth(stmts, d)?,
        }
        // F-string interpolations are expressions too.
        if let Expr::FStr(parts, _) = e {
            for p in parts {
                if let FPart::Expr(inner) = p {
                    stack.push((inner, d));
                }
            }
        }
    }
    Ok(())
}

fn check_stmt_depth(stmts: &[Stmt], start: usize) -> Result<()> {
    let mut stack: Vec<&Stmt> = stmts.iter().collect();
    let mut guard = 0usize;
    while let Some(s) = stack.pop() {
        guard += 1;
        if guard > 10_000_000 {
            return Err(Diag::new(
                codes::NESTING,
                "program nests too deeply",
                Span::default(),
            ));
        }
        match s {
            Stmt::Let { value, .. } => check_expr_depth(value, start)?,
            Stmt::LetPattern { value, .. } => check_expr_depth(value, start)?,
            Stmt::Assign { target, value, .. } => {
                check_expr_depth(target, start)?;
                check_expr_depth(value, start)?;
            }
            Stmt::Expr(e, _) => check_expr_depth(e, start)?,
            Stmt::Return(Some(e), _) | Stmt::Throw(e, _) => check_expr_depth(e, start)?,
            Stmt::Return(None, _) | Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::While(c, body, _) => {
                check_expr_depth(c, start)?;
                stack.extend(body.iter());
            }
            Stmt::Loop(body, _) => stack.extend(body.iter()),
            Stmt::For(_, iter, body, _) => {
                check_expr_depth(iter, start)?;
                stack.extend(body.iter());
            }
            Stmt::Try {
                body,
                catch_body,
                finally,
                ..
            } => {
                stack.extend(body.iter());
                stack.extend(catch_body.iter());
                if let Some(f) = finally {
                    stack.extend(f.iter());
                }
            }
        }
    }
    Ok(())
}

/// Raw recursion budget for the recursive-descent parser.
///
/// This is a **host-safety** backstop, not the language's semantic nesting
/// limit. The semantic limit is [`MAX_AST_DEPTH`] (256), measured on the
/// *AST* and enforced iteratively by [`enforce_depth`]. Grouping tokens such
/// as parentheses recurse in the parser without adding AST depth, so the
/// parser needs its own frame budget; it is set well above what the semantic
/// limit can consume (an AST level costs a small constant number of frames).
/// The budget is substrate-calibrated by [`parse_recursion_budget`], and
/// over-limit input is reported as `E1015`, the same code as the semantic
/// limit, so users see one consistent "nesting" diagnostic.
struct Parser {
    toks: Vec<Token>,
    pos: usize,
    depth: usize,
    /// Number of infix/postfix wraps applied to the expression currently
    /// being built. Bounds the depth of a flat chain without recursing.
    expr_nodes: usize,
}

impl Parser {
    fn at(&self) -> &Tok {
        &self.toks[self.pos.min(self.toks.len() - 1)].tok
    }

    fn span(&self) -> Span {
        self.toks[self.pos.min(self.toks.len() - 1)].span
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos.min(self.toks.len() - 1)].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.at() == t {
            self.bump();
            true
        } else {
            false
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.at(), Tok::Newline) {
            self.bump();
        }
    }

    fn end_stmt(&mut self) {
        if matches!(self.at(), Tok::Semi | Tok::Newline) {
            self.bump();
        }
    }

    fn expect(&mut self, t: &Tok) -> Result<Span> {
        if self.at() == t {
            Ok(self.bump().span)
        } else {
            Err(self.expected(&t.to_string()))
        }
    }

    fn expect_eof(&mut self) -> Result<()> {
        if matches!(self.at(), Tok::Eof) {
            Ok(())
        } else {
            Err(self.expected("end of input"))
        }
    }

    fn expected(&self, what: &str) -> Diag {
        Diag::new(
            codes::EXPECTED,
            format!("expected {what}, found {}", self.at().describe()),
            self.span(),
        )
    }

    fn enter(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > parse_recursion_budget() {
            return Err(Diag::new(
                codes::NESTING,
                "expression nests too deeply",
                self.span(),
            ));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    // ------------------------------------------------------------- module

    fn module(&mut self) -> Result<Module> {
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.at(), Tok::Eof) {
                break;
            }
            items.push(self.item()?);
        }
        Ok(Module { items })
    }

    fn item(&mut self) -> Result<Item> {
        let public = self.eat(&Tok::Pub);
        match self.at().clone() {
            Tok::Fn => self.fn_item(public),
            Tok::Struct => self.struct_item(),
            Tok::Enum => self.enum_item(),
            Tok::Type => self.alias_item(),
            Tok::Use => self.use_item(),
            Tok::Let => self.const_item(),
            Tok::Impl => self.impl_item(),
            _ => {
                let e = self.expr()?;
                let span = span_of(&e);
                self.end_stmt();
                Ok(Item::Expr(e, span))
            }
        }
    }

    fn fn_item(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("function name")?;
        self.expect(&Tok::LParen)?;
        let params = self.params(false)?;
        let ret = if self.eat(&Tok::Arrow) {
            Some(self.ty()?)
        } else {
            None
        };
        let body = self.block()?;
        Ok(Item::Fn {
            name,
            params,
            ret,
            body,
            public,
            span,
        })
    }

    /// `impl Struct { fn method(self, ...) { ... } ... }` — a behavior block
    /// attached to an already-declared nominal struct (`LANGUAGE_SPEC.md`
    /// §17.6). Only `fn` declarations may appear; each must declare the
    /// explicit receiver `self` as its first parameter.
    fn impl_item(&mut self) -> Result<Item> {
        let span = self.span();
        self.bump();
        let target = self.ident("struct name after `impl`")?;
        self.expect(&Tok::LBrace)?;
        let mut methods = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Tok::RBrace) {
                break;
            }
            if matches!(self.at(), Tok::Eof) {
                return Err(self.expected("`}` closing the `impl` block"));
            }
            let public = self.eat(&Tok::Pub);
            if !matches!(self.at(), Tok::Fn) {
                let found = self.at().describe();
                return Err(Diag::new(
                    codes::EXPECTED,
                    format!("expected a `fn` method declaration or `}}` in `impl`, found {found}"),
                    self.span(),
                ));
            }
            methods.push(self.method_item(public)?);
        }
        Ok(Item::Impl {
            target,
            methods,
            span,
        })
    }

    /// `fn name(self, ...) { ... }` inside an `impl` block. Identical to
    /// [`Parser::fn_item`] except that the first parameter MUST be `self`.
    fn method_item(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("method name")?;
        self.expect(&Tok::LParen)?;
        let params = self.params(true)?;
        if params.is_empty() {
            return Err(Diag::new(
                codes::EXPECTED,
                format!("method `{name}` must declare the receiver `self` as its first parameter"),
                span,
            ));
        }
        let ret = if self.eat(&Tok::Arrow) {
            Some(self.ty()?)
        } else {
            None
        };
        let body = self.block()?;
        Ok(Item::Fn {
            name,
            params,
            ret,
            body,
            public,
            span,
        })
    }

    /// Parse a parameter list. When `receiver` is true, the first parameter
    /// must be the keyword `self` (an ordinary binding named `self`); `self`
    /// is a reserved word elsewhere and cannot name any other parameter.
    fn params(&mut self, receiver: bool) -> Result<Vec<Param>> {
        let mut out = Vec::new();
        let mut first = true;
        if self.eat(&Tok::RParen) {
            return Ok(out);
        }
        loop {
            let span = self.span();
            let name = if receiver && first {
                match self.at() {
                    Tok::SelfKw => {
                        self.bump();
                        "self".to_string()
                    }
                    other => {
                        let found = other.describe();
                        return Err(Diag::new(
                            codes::EXPECTED,
                            format!("a method's first parameter must be `self`, found {found}"),
                            span,
                        ));
                    }
                }
            } else {
                self.ident("parameter name")?
            };
            first = false;
            let ty = if self.eat(&Tok::Colon) {
                Some(self.ty()?)
            } else {
                None
            };
            out.push(Param { name, ty, span });
            if !self.eat(&Tok::Comma) {
                self.expect(&Tok::RParen)?;
                return Ok(out);
            }
        }
    }

    fn struct_item(&mut self) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("struct name")?;
        self.expect(&Tok::LBrace)?;
        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Tok::RBrace) {
                break;
            }
            let fname = self.ident("field name")?;
            self.expect(&Tok::Colon)?;
            let fty = self.ty()?;
            fields.push((fname, fty));
            self.skip_newlines();
            if !self.eat(&Tok::Comma) {
                self.skip_newlines();
                self.expect(&Tok::RBrace)?;
                break;
            }
        }
        Ok(Item::Struct { name, fields, span })
    }

    fn enum_item(&mut self) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("enum name")?;
        self.expect(&Tok::LBrace)?;
        let mut variants = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Tok::RBrace) {
                break;
            }
            let vname = self.ident("variant name")?;
            let mut payload = Vec::new();
            if self.eat(&Tok::LParen) {
                if !self.eat(&Tok::RParen) {
                    loop {
                        payload.push(self.ty()?);
                        if !self.eat(&Tok::Comma) {
                            self.expect(&Tok::RParen)?;
                            break;
                        }
                    }
                }
            }
            variants.push((vname, payload));
            self.skip_newlines();
            if !self.eat(&Tok::Comma) {
                self.skip_newlines();
                self.expect(&Tok::RBrace)?;
                break;
            }
        }
        Ok(Item::Enum {
            name,
            variants,
            span,
        })
    }

    fn alias_item(&mut self) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("type alias name")?;
        self.expect(&Tok::Assign)?;
        let target = self.ty()?;
        self.end_stmt();
        Ok(Item::Alias { name, target, span })
    }

    fn use_item(&mut self) -> Result<Item> {
        let span = self.span();
        self.bump();
        let mut path = vec![self.ident("module name")?];
        while self.eat(&Tok::Dot) {
            path.push(self.ident("module path segment")?);
        }
        self.end_stmt();
        Ok(Item::Use { path, span })
    }

    fn const_item(&mut self) -> Result<Item> {
        let span = self.span();
        self.bump();
        if self.eat(&Tok::Mut) {
            return Err(Diag::new(
                codes::EXPECTED,
                "top-level bindings are module constants and are always immutable; `mut` is not allowed here",
                span,
            ));
        }
        let name = self.ident("binding name")?;
        let ann = if self.eat(&Tok::Colon) {
            Some(self.ty()?)
        } else {
            None
        };
        self.expect(&Tok::Assign)?;
        let value = self.expr()?;
        self.end_stmt();
        Ok(Item::Const {
            name,
            ann,
            value,
            span,
        })
    }

    fn ident(&mut self, what: &str) -> Result<String> {
        match self.bump() {
            Token {
                tok: Tok::Ident(n), ..
            } => Ok(n),
            other => {
                let found = other.tok.describe();
                let code = if is_keyword(&other.tok) {
                    codes::RESERVED_NAME
                } else {
                    codes::EXPECTED
                };
                Err(Diag::new(
                    code,
                    format!("expected {what}, found {found}"),
                    other.span,
                ))
            }
        }
    }

    // ---------------------------------------------------------------- types

    fn ty(&mut self) -> Result<TypeExpr> {
        let first = self.ty_member()?;
        if !self.eat(&Tok::Bar) {
            return Ok(first);
        }
        let mut members = vec![first];
        loop {
            members.push(self.ty_member()?);
            if !self.eat(&Tok::Bar) {
                break;
            }
        }
        Ok(TypeExpr::Union(members))
    }

    /// One member of a type expression (a union member).
    fn ty_member(&mut self) -> Result<TypeExpr> {
        Ok(match self.at().clone() {
            Tok::Ident(id) => {
                self.bump();
                match id.as_str() {
                    "int" => TypeExpr::Int,
                    "float" => TypeExpr::Float,
                    "bool" => TypeExpr::Bool,
                    "string" => TypeExpr::String,
                    _ => TypeExpr::Named(id),
                }
            }
            Tok::None => {
                self.bump();
                TypeExpr::None
            }
            Tok::LBracket => {
                self.bump();
                let inner = self.ty()?;
                self.expect(&Tok::RBracket)?;
                TypeExpr::List(Box::new(inner))
            }
            Tok::LBrace => {
                self.bump();
                let k = self.ty()?;
                self.expect(&Tok::Colon)?;
                let v = self.ty()?;
                self.expect(&Tok::RBrace)?;
                TypeExpr::Map(Box::new(k), Box::new(v))
            }
            other => {
                return Err(Diag::new(
                    codes::EXPECTED,
                    format!("expected a type, found {}", other.describe()),
                    self.span(),
                ))
            }
        })
    }

    // --------------------------------------------------------------- blocks

    fn block(&mut self) -> Result<Vec<Stmt>> {
        self.enter()?;
        self.expect(&Tok::LBrace)?;
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.at(), Tok::RBrace | Tok::Eof) {
                break;
            }
            stmts.push(self.stmt()?);
        }
        self.expect(&Tok::RBrace)?;
        self.leave();
        Ok(stmts)
    }

    fn stmt(&mut self) -> Result<Stmt> {
        self.enter()?;
        let r = self.stmt_inner();
        self.leave();
        r
    }

    fn stmt_inner(&mut self) -> Result<Stmt> {
        let span = self.span();
        match self.at().clone() {
            Tok::Let => {
                self.bump();
                let mutable = self.eat(&Tok::Mut);
                // A `let` binds either a single identifier (the ordinary,
                // optionally annotated and optionally mutable form) or a
                // destructuring pattern (§4.7). Parse the pattern first, then
                // route a lone binding name through the ordinary path so
                // `let x`, `let mut x`, and `let x: T` keep their exact
                // behavior.
                let pattern = self.let_pattern()?;
                if let Pattern::Bind(name) = pattern {
                    let ann = if self.eat(&Tok::Colon) {
                        Some(self.ty()?)
                    } else {
                        None
                    };
                    if !self.eat(&Tok::Assign) {
                        return Err(Diag::new(
                            codes::LET_NO_INIT,
                            format!("`let {name}` needs an initializer: `let {name} = ...`"),
                            span,
                        ));
                    }
                    let value = self.expr()?;
                    self.end_stmt();
                    return Ok(Stmt::Let {
                        mutable,
                        name,
                        ann,
                        value,
                        span,
                    });
                }
                // Genuine destructuring: `mut` and annotations are not allowed
                // (§4.7), and the pattern must be followed by `=`.
                if mutable {
                    return Err(Diag::new(
                        codes::EXPECTED,
                        "`mut` is not allowed on a destructuring `let`; every name it binds is immutable",
                        span,
                    ));
                }
                if self.eat(&Tok::Colon) {
                    return Err(Diag::new(
                        codes::EXPECTED,
                        "a destructuring `let` cannot have a type annotation",
                        span,
                    ));
                }
                if !self.eat(&Tok::Assign) {
                    return Err(Diag::new(
                        codes::LET_NO_INIT,
                        "a destructuring `let` needs an initializer: `let <pattern> = ...`",
                        span,
                    ));
                }
                let value = self.expr()?;
                self.end_stmt();
                Ok(Stmt::LetPattern {
                    pattern,
                    value,
                    span,
                })
            }
            Tok::Return => {
                self.bump();
                let value = if starts_expr(self.at()) {
                    Some(self.expr()?)
                } else {
                    None
                };
                self.end_stmt();
                Ok(Stmt::Return(value, span))
            }
            Tok::Throw => {
                self.bump();
                let value = self.expr()?;
                self.end_stmt();
                Ok(Stmt::Throw(value, span))
            }
            Tok::Break => {
                self.bump();
                self.end_stmt();
                Ok(Stmt::Break(span))
            }
            Tok::Continue => {
                self.bump();
                self.end_stmt();
                Ok(Stmt::Continue(span))
            }
            Tok::While => {
                self.bump();
                let cond = self.expr()?;
                let body = self.block()?;
                Ok(Stmt::While(cond, body, span))
            }
            Tok::Loop => {
                self.bump();
                let body = self.block()?;
                Ok(Stmt::Loop(body, span))
            }
            Tok::For => {
                self.bump();
                let pat = self.pattern()?;
                self.expect(&Tok::In)?;
                let iter = self.expr()?;
                let body = self.block()?;
                Ok(Stmt::For(pat, iter, body, span))
            }
            Tok::Try => {
                self.bump();
                let body = self.block()?;
                self.expect(&Tok::Catch)?;
                let catch = self.ident("catch binding")?;
                self.expect(&Tok::Arrow)?;
                let catch_body = self.block()?;
                let finally = if self.eat(&Tok::Finally) {
                    Some(self.block()?)
                } else {
                    None
                };
                self.end_stmt();
                Ok(Stmt::Try {
                    body,
                    catch,
                    catch_body,
                    finally,
                    span,
                })
            }
            _ => {
                let expr = self.expr()?;
                let span = span_of(&expr);
                let op = match self.at() {
                    Tok::Assign => Some(None),
                    Tok::PlusEq => Some(Some(BinOp::Add)),
                    Tok::MinusEq => Some(Some(BinOp::Sub)),
                    Tok::StarEq => Some(Some(BinOp::Mul)),
                    Tok::SlashEq => Some(Some(BinOp::Div)),
                    _ => None,
                };
                if let Some(compound) = op {
                    self.bump();
                    let value = self.expr()?;
                    self.end_stmt();
                    return Ok(Stmt::Assign {
                        target: expr,
                        value,
                        op: compound,
                        span,
                    });
                }
                self.end_stmt();
                Ok(Stmt::Expr(expr, span))
            }
        }
    }

    /// Parse a `let`-position pattern (§4.7).
    ///
    /// This is the restricted subset of [`Self::pattern`] accepted by `let`:
    /// binding names, list patterns, variant patterns, and any nesting of
    /// those. Literal and `none` patterns are rejected with `E1006`, and the
    /// general [`Self::pattern`] used by `for` and `match` is left untouched.
    fn let_pattern(&mut self) -> Result<Pattern> {
        self.enter()?;
        let r = self.let_pattern_inner();
        self.leave();
        r
    }

    fn let_pattern_inner(&mut self) -> Result<Pattern> {
        let span = self.span();
        match self.at().clone() {
            Tok::Ident(n) => {
                self.bump();
                if self.eat(&Tok::LParen) {
                    let mut ps = Vec::new();
                    if !self.eat(&Tok::RParen) {
                        loop {
                            ps.push(self.let_pattern()?);
                            if !self.eat(&Tok::Comma) {
                                self.expect(&Tok::RParen)?;
                                break;
                            }
                        }
                    }
                    Ok(Pattern::Variant(n, ps))
                } else if n.chars().next().is_some_and(char::is_uppercase) {
                    Ok(Pattern::Variant(n, Vec::new()))
                } else {
                    Ok(Pattern::Bind(n))
                }
            }
            Tok::LBracket => {
                self.bump();
                let mut parts = Vec::new();
                if !self.eat(&Tok::RBracket) {
                    loop {
                        self.skip_newlines();
                        parts.push(self.let_pattern()?);
                        if !self.eat(&Tok::Comma) {
                            self.skip_newlines();
                            self.expect(&Tok::RBracket)?;
                            break;
                        }
                        // A trailing comma before `]` is allowed (§4.7).
                        if matches!(self.at(), Tok::RBracket) {
                            self.bump();
                            break;
                        }
                    }
                }
                Ok(Pattern::List(parts))
            }
            other => {
                // Literal and `none` patterns are unsupported in `let` and
                // report `E1006`; a reserved word in binding position keeps
                // its dedicated `E1009` diagnostic (matching the ordinary
                // `let x` path); anything else is an unsupported pattern.
                let code = match other {
                    Tok::Int(_) | Tok::Str(_) | Tok::True | Tok::False | Tok::None => {
                        codes::EXPECTED
                    }
                    ref t if is_keyword(t) => codes::RESERVED_NAME,
                    _ => codes::EXPECTED,
                };
                Err(Diag::new(
                    code,
                    format!(
                        "expected a binding, list, or variant pattern in `let`, found {}",
                        other.describe()
                    ),
                    span,
                ))
            }
        }
    }

    fn pattern(&mut self) -> Result<Pattern> {
        self.enter()?;
        let r = self.pattern_inner();
        self.leave();
        r
    }

    fn pattern_inner(&mut self) -> Result<Pattern> {
        let span = self.span();
        match self.at().clone() {
            Tok::Ident(n) => {
                self.bump();
                if self.eat(&Tok::LParen) {
                    let mut ps = Vec::new();
                    if !self.eat(&Tok::RParen) {
                        loop {
                            ps.push(self.pattern()?);
                            if !self.eat(&Tok::Comma) {
                                self.expect(&Tok::RParen)?;
                                break;
                            }
                        }
                    }
                    Ok(Pattern::Variant(n, ps))
                } else if n.chars().next().is_some_and(char::is_uppercase) {
                    Ok(Pattern::Variant(n, Vec::new()))
                } else {
                    Ok(Pattern::Bind(n))
                }
            }
            Tok::Int(v) => {
                self.bump();
                Ok(Pattern::Int(v))
            }
            Tok::Str(s) => {
                self.bump();
                Ok(Pattern::Str(s))
            }
            Tok::True => {
                self.bump();
                Ok(Pattern::Bool(true))
            }
            Tok::False => {
                self.bump();
                Ok(Pattern::Bool(false))
            }
            Tok::None => {
                self.bump();
                Ok(Pattern::None)
            }
            Tok::LBracket => {
                self.bump();
                let mut parts = Vec::new();
                if !self.eat(&Tok::RBracket) {
                    loop {
                        self.skip_newlines();
                        parts.push(self.pattern()?);
                        if !self.eat(&Tok::Comma) {
                            self.skip_newlines();
                            self.expect(&Tok::RBracket)?;
                            break;
                        }
                        // A trailing comma before `]` is allowed (§4.6).
                        if matches!(self.at(), Tok::RBracket) {
                            self.bump();
                            break;
                        }
                    }
                }
                Ok(Pattern::List(parts))
            }
            other => Err(Diag::new(
                codes::EXPECTED,
                format!("expected a pattern, found {}", other.describe()),
                span,
            )),
        }
    }

    // ---------------------------------------------------------- expressions

    fn expr(&mut self) -> Result<Expr> {
        self.enter()?;
        let saved = self.expr_nodes;
        self.expr_nodes = 0;
        let r = self.expr_bp(0);
        self.expr_nodes = saved;
        self.leave();
        r
    }

    fn expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        let mut lhs = self.unary()?;
        loop {
            let span = self.span();
            // `a..b` — Rust-style range. Its binding power sits between
            // comparison (7/8) and additive (11/12), so both operands are
            // arithmetic expressions: `1 + 2..n - 1` is `(1 + 2)..(n - 1)`.
            // Ranges are right-associative and not chainable with meaning;
            // `a..b..c` parses as `a..(b..c)` and is a runtime type error at
            // evaluation, like any other non-int bound.
            if matches!(self.at(), Tok::DotDot) {
                let (lbp, rbp) = (10, 10);
                if lbp < min_bp {
                    break;
                }
                self.bump();
                if matches!(self.at(), Tok::Eof | Tok::Newline) {
                    return Err(Diag::new(
                        codes::EXPECTED,
                        "expected an expression after `..`",
                        self.span(),
                    ));
                }
                self.count_node(span)?;
                let rhs = self.expr_bp(rbp)?;
                lhs = Expr::Range(Box::new(lhs), Box::new(rhs), span);
                continue;
            }
            let Some((lbp, rbp, op)) = infix(self.at()) else {
                break;
            };
            if lbp < min_bp {
                break;
            }
            self.bump();
            if op.is_none() && matches!(self.at(), Tok::Eof | Tok::Newline) {
                break;
            }
            self.count_node(span)?;
            match op {
                None => {
                    // Pipeline: `x |> f` calls `f(x)`; `x |> f(a)` calls
                    // `f(x, a)`. The left operand becomes the first argument.
                    let rhs = self.expr_bp(rbp)?;
                    let piped = desugar_pipe(lhs, rhs, span);
                    lhs = piped;
                }
                Some(op) => {
                    let rhs = self.expr_bp(rbp)?;
                    lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs), span);
                }
            }
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr> {
        let span = self.span();
        match self.at().clone() {
            Tok::Minus => {
                self.bump();
                // `-9223372036854775808` is `i64::MIN`; the magnitude alone is
                // out of range and is only valid directly under this minus.
                if matches!(self.at(), Tok::IntMinMagnitude) {
                    self.bump();
                    return Ok(Expr::Lit(Lit::Int(i64::MIN), span));
                }
                let e = self.unary()?;
                Ok(Expr::Unary(UnOp::Neg, Box::new(e), span))
            }
            Tok::Not => {
                self.bump();
                let e = self.unary()?;
                Ok(Expr::Unary(UnOp::Not, Box::new(e), span))
            }
            _ => self.postfix(),
        }
    }

    fn postfix(&mut self) -> Result<Expr> {
        let mut e = self.atom()?;
        loop {
            // Only count a node when this iteration actually wraps `e`; a
            // plain operand must not consume the postfix budget. Counting
            // unconditionally made the effective flat-expression limit half
            // the documented one.
            match self.at().clone() {
                Tok::LParen => {
                    let span = self.span();
                    self.count_node(span)?;
                    self.bump();
                    let args = self.call_args()?;
                    e = Expr::Call(Box::new(e), args, span);
                }
                Tok::LBracket => {
                    let span = self.span();
                    self.count_node(span)?;
                    self.bump();
                    let idx = self.expr()?;
                    self.expect(&Tok::RBracket)?;
                    e = Expr::Index(Box::new(e), Box::new(idx), span);
                }
                Tok::Dot => {
                    let span = self.span();
                    self.count_node(span)?;
                    self.bump();
                    let name = self.ident("field or method name")?;
                    if matches!(self.at(), Tok::LParen) {
                        self.bump();
                        let args = self.call_args()?;
                        e = Expr::Method(Box::new(e), name, args, span);
                    } else {
                        e = Expr::Field(Box::new(e), name, span);
                    }
                }
                _ => break,
            }
        }
        Ok(e)
    }

    /// Parse a parenthesized call argument list, up to and including `)`.
    ///
    /// Arguments may be positional (`expr`) or named (`name: expr`). Named
    /// arguments are supported only for directly resolved user functions, but
    /// the parser accepts the form and the checker validates it. Positional
    /// arguments MUST precede named arguments; a positional argument after a
    /// named one is `E1006` (`LANGUAGE_SPEC.md` §4.5).
    fn call_args(&mut self) -> Result<Vec<Arg>> {
        let mut out = Vec::new();
        self.skip_newlines();
        if self.eat(&Tok::RParen) {
            return Ok(out);
        }
        let mut seen_named = false;
        loop {
            self.skip_newlines();
            let arg = self.cons_arg()?;
            if arg.name.is_some() {
                seen_named = true;
            } else if seen_named {
                return Err(Diag::new(
                    codes::EXPECTED,
                    "positional arguments must come before named arguments",
                    span_of(&arg.value),
                ));
            }
            out.push(arg);
            self.skip_newlines();
            if !self.eat(&Tok::Comma) {
                self.expect(&Tok::RParen)?;
                return Ok(out);
            }
        }
    }

    /// Consume one unit of the flat-expression node budget, reporting the
    /// semantic nesting diagnostic if it is exceeded.
    fn count_node(&mut self, span: Span) -> Result<()> {
        self.expr_nodes += 1;
        if self.expr_nodes > MAX_AST_DEPTH {
            return Err(Diag::new(
                codes::NESTING,
                "expression nests too deeply",
                span,
            ));
        }
        Ok(())
    }

    fn atom(&mut self) -> Result<Expr> {
        self.enter()?;
        let span = self.span();
        let e = match self.at().clone() {
            Tok::Int(v) => {
                self.bump();
                Expr::Lit(Lit::Int(v), span)
            }
            Tok::IntMinMagnitude => {
                return Err(Diag::new(
                    codes::INVALID_NUMBER,
                    "integer out of range; `9223372036854775808` is only valid as part of `-9223372036854775808`",
                    span,
                ))
            }
            Tok::Float(v) => {
                self.bump();
                Expr::Lit(Lit::Float(v), span)
            }
            Tok::Str(s) => {
                self.bump();
                Expr::Lit(Lit::Str(s), span)
            }
            Tok::True => {
                self.bump();
                Expr::Lit(Lit::Bool(true), span)
            }
            Tok::False => {
                self.bump();
                Expr::Lit(Lit::Bool(false), span)
            }
            Tok::None => {
                self.bump();
                Expr::Lit(Lit::None, span)
            }
            Tok::FStr(raw) => {
                self.bump();
                let parts = self.fstring(&raw, span)?;
                Expr::FStr(parts, span)
            }
            Tok::SelfKw => {
                // The receiver, as an ordinary binding reference.
                self.bump();
                Expr::Name("self".to_string(), span)
            }
            Tok::Ident(name) => {
                self.bump();
                let is_type_name = name.chars().next().is_some_and(char::is_uppercase);
                if matches!(self.at(), Tok::LBrace) && is_type_name {
                    // struct literal `Point { x: 1 }`
                    self.bump();
                    let mut args = Vec::new();
                    self.skip_newlines();
                    if !self.eat(&Tok::RBrace) {
                        loop {
                            self.skip_newlines();
                            let fname = self.ident("field name")?;
                            self.expect(&Tok::Colon)?;
                            let value = self.expr()?;
                            args.push(Arg {
                                name: Some(fname),
                                value,
                            });
                            self.skip_newlines();
                            if !self.eat(&Tok::Comma) {
                                self.expect(&Tok::RBrace)?;
                                break;
                            }
                        }
                    }
                    return Ok(Expr::Construct(name, args, span));
                }
                if matches!(self.at(), Tok::LParen) && is_type_name {
                    // variant constructor `Ok(x)`
                    self.bump();
                    let mut args = Vec::new();
                    self.skip_newlines();
                    if !self.eat(&Tok::RParen) {
                        loop {
                            self.skip_newlines();
                            args.push(self.cons_arg()?);
                            self.skip_newlines();
                            if !self.eat(&Tok::Comma) {
                                self.expect(&Tok::RParen)?;
                                break;
                            }
                        }
                    }
                    return Ok(Expr::Construct(name, args, span));
                }
                Expr::Name(name, span)
            }
            Tok::LParen => {
                self.bump();
                // lambda?
                if let Some(params) = self.try_lambda_params() {
                    let body = self.expr()?;
                    return Ok(Expr::Lambda(params, Box::new(body), span));
                }
                let first = self.expr()?;
                if self.eat(&Tok::Comma) {
                    let mut items = vec![first];
                    // `(x,)` is a one-element list (§21); the grammar's
                    // optional tail after the comma may be empty.
                    if !matches!(self.at(), Tok::RParen) {
                        loop {
                            self.skip_newlines();
                            items.push(self.expr()?);
                            self.skip_newlines();
                            if !self.eat(&Tok::Comma) {
                                break;
                            }
                            if matches!(self.at(), Tok::RParen) {
                                break;
                            }
                        }
                    }
                    self.expect(&Tok::RParen)?;
                    Expr::Tuple(items, span)
                } else {
                    self.expect(&Tok::RParen)?;
                    first
                }
            }
            Tok::LBracket => {
                self.bump();
                let mut items = Vec::new();
                self.skip_newlines();
                if !self.eat(&Tok::RBracket) {
                    loop {
                        self.skip_newlines();
                        items.push(self.expr()?);
                        self.skip_newlines();
                        if !self.eat(&Tok::Comma) {
                            self.expect(&Tok::RBracket)?;
                            break;
                        }
                        // A trailing comma before `]` is allowed (§4.5).
                        if matches!(self.at(), Tok::RBracket) {
                            self.bump();
                            break;
                        }
                    }
                }
                Expr::List(items, span)
            }
            Tok::LBrace => {
                if self.map_ahead() {
                    self.bump();
                    let mut entries = Vec::new();
                    self.skip_newlines();
                    // `{:}` is the empty-map literal (§20.3): `{` `:` `}`.
                    // Whitespace and newlines around the colon are already
                    // skipped by the lexer and `skip_newlines`, so `{ : }`
                    // and the multi-line form are identical. `{}` never
                    // reaches this branch (`map_ahead` routes it to a block).
                    if self.eat(&Tok::Colon) {
                        self.skip_newlines();
                        self.expect(&Tok::RBrace)?;
                        return Ok(Expr::Map(entries, span));
                    }
                    while !self.eat(&Tok::RBrace) {
                        let k = self.expr()?;
                        self.expect(&Tok::Colon)?;
                        let v = self.expr()?;
                        entries.push((k, v));
                        self.skip_newlines();
                        if !self.eat(&Tok::Comma) {
                            self.skip_newlines();
                            self.expect(&Tok::RBrace)?;
                            break;
                        }
                        self.skip_newlines();
                    }
                    Expr::Map(entries, span)
                } else {
                    let body = self.block()?;
                    Expr::Block(body, span)
                }
            }
            Tok::If => {
                self.bump();
                let cond = self.expr()?;
                let then = self.block()?;
                let els = if self.eat(&Tok::Else) {
                    // `else` accepts an expression, and `if` is an expression,
                    // so `else if B { … }` parses as the nested `Expr::If`
                    // `else { if B { … } }` with no special handling.
                    Some(Box::new(self.expr()?))
                } else {
                    None
                };
                Expr::If(Box::new(cond), then, els, span)
            }
            Tok::Match => {
                self.bump();
                let subject = self.expr()?;
                self.expect(&Tok::LBrace)?;
                let mut arms = Vec::new();
                loop {
                    self.skip_newlines();
                    if self.eat(&Tok::RBrace) {
                        break;
                    }
                    let pat = self.pattern()?;
                    let guard = if self.eat(&Tok::If) {
                        Some(self.expr()?)
                    } else {
                        None
                    };
                    self.expect(&Tok::Arrow)?;
                    let body = if matches!(self.at(), Tok::LBrace) {
                        self.block()?
                    } else {
                        let e = self.expr()?;
                        self.end_stmt();
                        vec![Stmt::Expr(e, span)]
                    };
                    arms.push(Arm {
                        pattern: pat,
                        guard,
                        body,
                    });
                    if matches!(self.at(), Tok::Comma | Tok::Semi) {
                        self.bump();
                    }
                }
                Expr::Match(Box::new(subject), arms, span)
            }
            Tok::Fn => {
                self.bump();
                let params = if self.eat(&Tok::LParen) {
                    let mut ps = Vec::new();
                    if !self.eat(&Tok::RParen) {
                        loop {
                            ps.push(self.ident("lambda parameter")?);
                            if !self.eat(&Tok::Comma) {
                                self.expect(&Tok::RParen)?;
                                break;
                            }
                        }
                    }
                    ps
                } else {
                    vec![self.ident("lambda parameter")?]
                };
                self.expect(&Tok::Arrow)?;
                let body = self.expr()?;
                Expr::Lambda(params, Box::new(body), span)
            }
            other => {
                return Err(Diag::new(
                    codes::EXPECTED,
                    format!("expected an expression, found {}", other.describe()),
                    span,
                ))
            }
        };
        self.leave();
        Ok(e)
    }

    fn cons_arg(&mut self) -> Result<Arg> {
        if let Tok::Ident(name) = self.at().clone() {
            let save = self.pos;
            self.bump();
            if self.eat(&Tok::Colon) {
                let value = self.expr()?;
                return Ok(Arg {
                    name: Some(name),
                    value,
                });
            }
            self.pos = save;
        }
        let value = self.expr()?;
        Ok(Arg { name: None, value })
    }

    /// If the tokens form `(x, y) ->` or `(x) ->`, return the params and
    /// consume through the arrow.
    fn try_lambda_params(&mut self) -> Option<Vec<String>> {
        let save = self.pos;
        let mut names = Vec::new();
        loop {
            match self.at().clone() {
                Tok::Ident(n) => {
                    self.bump();
                    names.push(n);
                }
                Tok::RParen => {
                    self.bump();
                    break;
                }
                _ => {
                    self.pos = save;
                    return None;
                }
            }
            if self.eat(&Tok::Comma) {
                continue;
            }
            if self.eat(&Tok::RParen) {
                break;
            }
            self.pos = save;
            return None;
        }
        if self.eat(&Tok::Arrow) {
            Some(names)
        } else {
            self.pos = save;
            None
        }
    }

    fn map_ahead(&self) -> bool {
        let mut i = self.pos + 1;
        let mut depth = 0i32;
        while i < self.toks.len() {
            match &self.toks[i].tok {
                Tok::Newline => i += 1,
                Tok::RParen | Tok::RBracket | Tok::RBrace if depth == 0 => return false,
                Tok::LParen | Tok::LBracket | Tok::LBrace => {
                    depth += 1;
                    i += 1;
                }
                Tok::RParen | Tok::RBracket | Tok::RBrace => {
                    depth -= 1;
                    i += 1;
                }
                Tok::Colon if depth == 0 => return true,
                _ => i += 1,
            }
        }
        false
    }

    fn fstring(&mut self, raw: &str, span: Span) -> Result<Vec<FPart>> {
        let mut parts = Vec::new();
        let mut lit = String::new();
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' => {
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        lit.push('{');
                        continue;
                    }
                    if !lit.is_empty() {
                        parts.push(FPart::Lit(std::mem::take(&mut lit)));
                    }
                    let mut inner = String::new();
                    let mut depth = 1usize;
                    loop {
                        match chars.next() {
                            Some('}') => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                inner.push('}');
                            }
                            Some('{') => {
                                depth += 1;
                                inner.push('{');
                            }
                            Some(c2) => inner.push(c2),
                            None => {
                                return Err(Diag::new(
                                    codes::UNTERMINATED_STRING,
                                    "unterminated `{` in f-string",
                                    span,
                                ))
                            }
                        }
                    }
                    if inner.trim().is_empty() {
                        return Err(Diag::new(codes::EXPECTED, "empty `{}` in f-string", span));
                    }
                    let e = parse_expr_inner(&inner)?;
                    parts.push(FPart::Expr(e));
                }
                '}' => {
                    if chars.peek() == Some(&'}') {
                        chars.next();
                    }
                    lit.push('}');
                }
                _ => lit.push(c),
            }
        }
        if !lit.is_empty() {
            parts.push(FPart::Lit(lit));
        }
        Ok(parts)
    }
}

/// Desugar `lhs |> rhs` into a call with `lhs` as the first argument:
///
/// * `lhs |> f`        -> `f(lhs)`
/// * `lhs |> f(a, b)`  -> `f(lhs, a, b)`
/// * `lhs |> r.m(a)`   -> `r.m(lhs, a)`
///
/// Any other right-hand side is a callable value, so it becomes `rhs(lhs)`.
fn desugar_pipe(lhs: Expr, rhs: Expr, span: Span) -> Expr {
    match rhs {
        Expr::Call(callee, mut args, cspan) => {
            // The piped value becomes the first *positional* argument; any
            // named arguments follow (§23).
            args.insert(
                0,
                Arg {
                    name: None,
                    value: lhs,
                },
            );
            Expr::Call(callee, args, cspan)
        }
        Expr::Method(recv, name, mut args, mspan) => {
            args.insert(
                0,
                Arg {
                    name: None,
                    value: lhs,
                },
            );
            Expr::Method(recv, name, args, mspan)
        }
        other => Expr::Pipe(Box::new(lhs), Box::new(other), span),
    }
}

/// Infix binding powers: `(left, right, op)`. `None` op means pipeline.
fn infix(t: &Tok) -> Option<(u8, u8, Option<BinOp>)> {
    Some(match t {
        Tok::Pipe => (1, 2, None),
        Tok::Or => (3, 4, Some(BinOp::Or)),
        Tok::And => (5, 6, Some(BinOp::And)),
        Tok::EqEq => (7, 8, Some(BinOp::Eq)),
        Tok::Ne => (7, 8, Some(BinOp::Ne)),
        Tok::Lt => (9, 10, Some(BinOp::Lt)),
        Tok::Le => (9, 10, Some(BinOp::Le)),
        Tok::Gt => (9, 10, Some(BinOp::Gt)),
        Tok::Ge => (9, 10, Some(BinOp::Ge)),
        Tok::Plus => (11, 12, Some(BinOp::Add)),
        Tok::Minus => (11, 12, Some(BinOp::Sub)),
        Tok::Star => (13, 14, Some(BinOp::Mul)),
        Tok::Slash => (13, 14, Some(BinOp::Div)),
        Tok::Percent => (13, 14, Some(BinOp::Rem)),
        Tok::Caret => (16, 15, Some(BinOp::Pow)),
        _ => return None,
    })
}

/// Whether a token is a reserved keyword (cannot be a name).
fn is_keyword(t: &Tok) -> bool {
    matches!(
        t,
        Tok::Let
            | Tok::Mut
            | Tok::Fn
            | Tok::If
            | Tok::Else
            | Tok::Match
            | Tok::While
            | Tok::Loop
            | Tok::For
            | Tok::In
            | Tok::Return
            | Tok::Break
            | Tok::Continue
            | Tok::Use
            | Tok::As
            | Tok::Struct
            | Tok::Enum
            | Tok::Type
            | Tok::And
            | Tok::Or
            | Tok::Not
            | Tok::Try
            | Tok::Catch
            | Tok::Finally
            | Tok::Throw
            | Tok::Pub
            | Tok::Impl
            | Tok::SelfKw
            | Tok::True
            | Tok::False
            | Tok::None
    )
}

fn starts_expr(t: &Tok) -> bool {
    matches!(
        t,
        Tok::Int(_)
            | Tok::Float(_)
            | Tok::Str(_)
            | Tok::FStr(_)
            | Tok::True
            | Tok::False
            | Tok::None
            | Tok::Ident(_)
            | Tok::SelfKw
            | Tok::LParen
            | Tok::LBracket
            | Tok::LBrace
            | Tok::Minus
            | Tok::Not
            | Tok::If
            | Tok::Match
            | Tok::Fn
    )
}

fn span_of(e: &Expr) -> Span {
    match e {
        Expr::Lit(_, s)
        | Expr::Name(_, s)
        | Expr::FStr(_, s)
        | Expr::Unary(_, _, s)
        | Expr::Binary(_, _, _, s)
        | Expr::Call(_, _, s)
        | Expr::Method(_, _, _, s)
        | Expr::Field(_, _, s)
        | Expr::Index(_, _, s)
        | Expr::List(_, s)
        | Expr::Map(_, s)
        | Expr::Construct(_, _, s)
        | Expr::Tuple(_, s)
        | Expr::Lambda(_, _, s)
        | Expr::Pipe(_, _, s)
        | Expr::Range(_, _, s)
        | Expr::If(_, _, _, s)
        | Expr::Match(_, _, s)
        | Expr::Block(_, s) => *s,
    }
}
