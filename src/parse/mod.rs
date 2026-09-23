//! Recursive-descent + Pratt parser.

use crate::ast::*;
use crate::error::{codes, Diag, Result, Span};
use crate::lex::lex;
use crate::lex::token::{Tok, Token};

/// Parse a full file.
pub fn parse(src: &str) -> Result<Module> {
    let toks = lex(src)?;
    let mut p = Parser {
        toks,
        pos: 0,
        depth: 0,
    };
    p.module()
}

/// Parse one expression (used by the REPL and tests).
pub fn parse_expr(src: &str) -> Result<Expr> {
    let toks = lex(src)?;
    let mut p = Parser {
        toks,
        pos: 0,
        depth: 0,
    };
    p.skip_newlines();
    let e = p.expr()?;
    p.skip_newlines();
    p.expect_eof()?;
    Ok(e)
}

const MAX_DEPTH: usize = 128;

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    depth: usize,
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
        if self.depth > MAX_DEPTH {
            return Err(Diag::new(codes::EXPECTED, "nesting too deep", self.span()));
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
        let params = self.params()?;
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

    fn params(&mut self) -> Result<Vec<Param>> {
        let mut out = Vec::new();
        if self.eat(&Tok::RParen) {
            return Ok(out);
        }
        loop {
            let span = self.span();
            let name = self.ident("parameter name")?;
            if out.iter().any(|p: &Param| p.name == name) {
                return Err(Diag::new(
                    codes::EXPECTED,
                    format!("duplicate parameter `{name}`"),
                    span,
                ));
            }
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
        let _ = self.eat(&Tok::Mut);
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
        let mut base = match self.at().clone() {
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
        };
        if self.eat(&Tok::Bar) {
            self.expect(&Tok::None)?;
            base = TypeExpr::Optional(Box::new(base));
        }
        Ok(base)
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
                let name = self.ident("binding name")?;
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
                Ok(Stmt::Let {
                    mutable,
                    name,
                    ann,
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

    fn pattern(&mut self) -> Result<Pattern> {
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
        let r = self.expr_bp(0);
        self.leave();
        r
    }

    fn expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        let mut lhs = self.unary()?;
        loop {
            let span = self.span();
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
            match op {
                None => {
                    // pipe
                    let rhs = self.expr_bp(rbp)?;
                    lhs = Expr::Pipe(Box::new(lhs), Box::new(rhs), span);
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
            let span = self.span();
            match self.at().clone() {
                Tok::LParen => {
                    self.bump();
                    let mut args = Vec::new();
                    self.skip_newlines();
                    if !self.eat(&Tok::RParen) {
                        loop {
                            self.skip_newlines();
                            args.push(self.arg()?);
                            self.skip_newlines();
                            if !self.eat(&Tok::Comma) {
                                self.expect(&Tok::RParen)?;
                                break;
                            }
                        }
                    }
                    e = Expr::Call(Box::new(e), args, span);
                }
                Tok::LBracket => {
                    self.bump();
                    let idx = self.expr()?;
                    self.expect(&Tok::RBracket)?;
                    e = Expr::Index(Box::new(e), Box::new(idx), span);
                }
                Tok::Dot => {
                    self.bump();
                    let name = self.ident("field or method name")?;
                    if matches!(self.at(), Tok::LParen) {
                        self.bump();
                        let mut args = Vec::new();
                        self.skip_newlines();
                        if !self.eat(&Tok::RParen) {
                            loop {
                                self.skip_newlines();
                                args.push(self.arg_expr()?);
                                self.skip_newlines();
                                if !self.eat(&Tok::Comma) {
                                    self.expect(&Tok::RParen)?;
                                    break;
                                }
                            }
                        }
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

    fn arg(&mut self) -> Result<Expr> {
        // named argument `name: value`
        if let Tok::Ident(name) = self.at().clone() {
            let save = self.pos;
            self.bump();
            if self.eat(&Tok::Colon) {
                let name2 = name.clone();
                self.bump(); // consumed ident
                self.pos = save;
                self.bump();
                self.expect(&Tok::Colon)?;
                let value = self.expr()?;
                return Ok(Expr::Call(
                    Box::new(Expr::Name(format!("__kw_{name2}"), self.span())),
                    vec![value],
                    self.span(),
                ));
            }
            self.pos = save;
        }
        self.arg_expr()
    }

    fn arg_expr(&mut self) -> Result<Expr> {
        self.expr()
    }

    fn atom(&mut self) -> Result<Expr> {
        self.enter()?;
        let span = self.span();
        let e = match self.at().clone() {
            Tok::Int(v) => {
                self.bump();
                Expr::Lit(Lit::Int(v), span)
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
                    }
                }
                Expr::List(items, span)
            }
            Tok::LBrace => {
                if self.map_ahead() {
                    self.bump();
                    let mut entries = Vec::new();
                    self.skip_newlines();
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
                    if matches!(self.at(), Tok::If) {
                        return Err(Diag::new(
                            codes::ELSE_IF,
                            "`else if` is not part of Aura; use `else { if ... }` or `match`",
                            self.span(),
                        ));
                    }
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
                    let e = parse_expr(&inner)?;
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
        | Expr::If(_, _, _, s)
        | Expr::Match(_, _, s)
        | Expr::Block(_, s) => *s,
    }
}
