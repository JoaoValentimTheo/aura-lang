//! Static checks: reserved names, mutability, and name resolution.
//!
//! Aura checks are conservative but complete: every name used must be
//! declared, every assignment must target a `let mut` binding, and every
//! forbidden construct (e.g. `else if`) is rejected by the parser.

use std::collections::HashMap;

use crate::ast::*;
use crate::check::types::Ty;
use crate::error::{codes, Diag, Result, Span};

pub mod types;

/// A lexical scope of bindings.
#[derive(Debug, Default)]
struct Scope {
    /// name -> mutable
    vars: HashMap<String, bool>,
    /// names declared in this exact scope, for redeclaration checks
    declares: HashMap<String, Span>,
}

/// The checker. Reports the first error, matching the CLI contract.
pub struct Checker {
    scopes: Vec<Scope>,
    /// Whether a `main` function was seen.
    pub has_main: bool,
    /// Names of every top-level function (hoisted), for call resolution.
    functions: HashMap<String, Span>,
    /// Names of every top-level type (struct, enum, alias).
    types: HashMap<String, Span>,
    /// The kind of each user type: `struct`, `enum`, or `alias`.
    type_kinds: HashMap<String, String>,
    /// Enum variant tags declared anywhere, with defining enum name.
    variants: HashMap<String, String>,
    /// The declared return type of the function currently being checked.
    return_type: Option<Ty>,
    /// The declared type of annotated bindings in scope, innermost last.
    value_types: Vec<HashMap<String, Ty>>,
    /// `_`-prefixed parameters of the function currently being checked, with
    /// their spans. By contract, each must remain unused.
    underscore_params: Vec<(String, Span)>,
    /// Names referenced while checking the current function body.
    used_names: HashMap<String, Span>,
}

impl Checker {
    /// Build a checker.
    fn new() -> Checker {
        Checker {
            scopes: vec![Scope::default()],
            has_main: false,
            functions: HashMap::new(),
            types: HashMap::new(),
            type_kinds: HashMap::new(),
            variants: HashMap::new(),
            return_type: None,
            value_types: vec![HashMap::new()],
            underscore_params: Vec::new(),
            used_names: HashMap::new(),
        }
    }

    /// Build a checker that already knows a set of global names (used by the
    /// REPL to carry declarations across submissions).
    #[must_use]
    pub fn with_globals(names: &[String]) -> Checker {
        let mut c = Checker::new();
        for name in names {
            c.scopes[0].declares.insert(name.clone(), Span::default());
            c.scopes[0].vars.insert(name.clone(), false);
            c.functions.insert(name.clone(), Span::default());
        }
        c
    }

    /// Check a module against this checker.
    ///
    /// # Errors
    /// Returns the first diagnostic found.
    pub fn check(&mut self, m: &Module) -> Result<()> {
        self.hoist(m)?;
        for item in &m.items {
            self.item(item)?;
        }
        Ok(())
    }

    /// Pre-pass: collect every top-level name before checking bodies, so that
    /// forward references between functions and constants resolve. This makes
    /// the checker agree with the runtime's declaration-then-initialize order.
    fn hoist(&mut self, m: &Module) -> Result<()> {
        let mut declared: HashMap<String, Span> = HashMap::new();
        for item in &m.items {
            match item {
                Item::Fn { name, span, .. } => {
                    if declared.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::REDECLARED,
                            format!("`{name}` is already declared in this scope"),
                            *span,
                        ));
                    }
                    self.functions.entry(name.clone()).or_insert(*span);
                    self.scopes[0].declares.entry(name.clone()).or_insert(*span);
                    self.scopes[0].vars.insert(name.clone(), false);
                }
                Item::Const { name, span, .. } => {
                    if declared.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::REDECLARED,
                            format!("`{name}` is already declared in this scope"),
                            *span,
                        ));
                    }
                    self.scopes[0].declares.entry(name.clone()).or_insert(*span);
                    self.scopes[0].vars.insert(name.clone(), false);
                }
                Item::Struct { name, span, .. } => {
                    if self.types.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::DUPLICATE_TYPE,
                            format!("type `{name}` is already declared"),
                            *span,
                        ));
                    }
                    self.type_kinds.insert(name.clone(), "struct".to_string());
                }
                Item::Enum {
                    name,
                    variants,
                    span,
                } => {
                    if self.types.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::DUPLICATE_TYPE,
                            format!("type `{name}` is already declared"),
                            *span,
                        ));
                    }
                    self.type_kinds.insert(name.clone(), "enum".to_string());
                    for (tag, _) in variants {
                        if let Some(prev) = self.variants.get(tag) {
                            if prev != name {
                                return Err(Diag::new(
                                    codes::DUPLICATE_VARIANT,
                                    format!(
                                        "variant `{tag}` is already declared by enum `{prev}`; variant names must be unique across the program"
                                    ),
                                    *span,
                                ));
                            }
                        } else {
                            self.variants.insert(tag.clone(), name.clone());
                        }
                    }
                }
                Item::Alias { name, span, .. } => {
                    if self.types.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::DUPLICATE_TYPE,
                            format!("type `{name}` is already declared"),
                            *span,
                        ));
                    }
                    self.type_kinds.insert(name.clone(), "alias".to_string());
                }
                Item::Use { .. } | Item::Expr(..) => {}
            }
        }
        // Validate every written type annotation now that all names are known.
        for item in &m.items {
            match item {
                Item::Struct { fields, .. } => {
                    for (_, fty) in fields {
                        Ty::from_expr(fty, &self.type_kinds, Span::default())?;
                    }
                }
                Item::Enum { variants, .. } => {
                    for (_, payload) in variants {
                        for pty in payload {
                            Ty::from_expr(pty, &self.type_kinds, Span::default())?;
                        }
                    }
                }
                Item::Alias { target, .. } => {
                    Ty::from_expr(target, &self.type_kinds, Span::default())?;
                }
                Item::Fn { params, ret, .. } => {
                    for p in params {
                        if let Some(pty) = &p.ty {
                            Ty::from_expr(pty, &self.type_kinds, p.span)?;
                        }
                    }
                    if let Some(rt) = ret {
                        Ty::from_expr(rt, &self.type_kinds, Span::default())?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Check a module, returning the first diagnostic.
    ///
    /// # Errors
    /// Returns the first diagnostic found.
    pub fn module(m: &Module) -> Result<()> {
        let mut c = Checker::new();
        c.check(m)
    }

    /// Check a module and require an entry point.
    ///
    /// # Errors
    /// Returns `E4027` when the module declares no `fn main`.
    pub fn module_with_main(m: &Module) -> Result<()> {
        let mut c = Checker::new();
        c.check(m)?;
        c.require_main()
    }

    /// Require that an entry point was declared.
    ///
    /// # Errors
    /// Returns `E4027` when no `fn main` was seen.
    pub fn require_main(&self) -> Result<()> {
        if self.has_main {
            Ok(())
        } else {
            Err(Diag::new(
                codes::NO_MAIN,
                "this program has no `fn main()`, so there is nothing to run",
                Span::default(),
            ))
        }
    }

    fn push(&mut self) {
        self.scopes.push(Scope::default());
        self.value_types.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.scopes.pop();
        self.value_types.pop();
    }

    fn declare(&mut self, name: &str, mutable: bool, span: Span) -> Result<()> {
        if name.starts_with('_') {
            // leading-underscore names are allowed but flagged if used
        }
        if crate::lex::KEYWORDS.contains(&name) {
            return Err(Diag::new(
                codes::RESERVED_NAME,
                format!("`{name}` is a reserved word and cannot be used as a name"),
                span,
            ));
        }
        let Some(scope) = self.scopes.last_mut() else {
            return Ok(());
        };
        if scope.declares.contains_key(name) {
            return Err(Diag::new(
                codes::REDECLARED,
                format!("`{name}` is already declared in this scope"),
                span,
            ));
        }
        scope.declares.insert(name.to_string(), span);
        scope.vars.insert(name.to_string(), mutable);
        Ok(())
    }

    fn lookup(&self, name: &str) -> Option<bool> {
        if crate::stdlib::builtin_names().contains(&name) {
            return Some(false);
        }
        for scope in self.scopes.iter().rev() {
            if let Some(m) = scope.vars.get(name) {
                return Some(*m);
            }
        }
        None
    }

    fn item(&mut self, item: &Item) -> Result<()> {
        match item {
            Item::Fn {
                name,
                params,
                ret,
                body,
                ..
            } => {
                if name == "main" {
                    if !params.is_empty() {
                        return Err(Diag::new(
                            codes::INVALID_MAIN,
                            format!(
                                "`main` is the program entry point and takes no arguments, but {} were declared",
                                params.len()
                            ),
                            params[0].span,
                        ));
                    }
                    self.has_main = true;
                }
                // The name is already declared by `hoist`; only the body scope
                // and parameter bindings are introduced here.
                self.push();
                for p in params {
                    self.declare(&p.name, false, p.span)?;
                    if let Some(pty) = &p.ty {
                        let t = self.annotation(pty, p.span)?;
                        self.value_types
                            .last_mut()
                            .map(|m| m.insert(p.name.clone(), t));
                    }
                }
                let saved_return = self.return_type.clone();
                let saved_underscore = std::mem::take(&mut self.underscore_params);
                let saved_used = std::mem::take(&mut self.used_names);
                self.return_type = match ret {
                    Some(rt) => Some(self.annotation(rt, Span::default())?),
                    None => None,
                };
                for p in params {
                    if p.name.starts_with('_') {
                        self.underscore_params.push((p.name.clone(), p.span));
                    }
                }
                self.block(body)?;
                // Contract: a parameter whose name starts with `_` must be
                // unused. Using it is E2009.
                for (pname, _pspan) in std::mem::take(&mut self.underscore_params) {
                    let use_span = self.used_names.get(&pname).copied();
                    if let Some(use_span) = use_span {
                        self.return_type = saved_return;
                        self.underscore_params = saved_underscore;
                        self.used_names = saved_used;
                        self.pop();
                        return Err(Diag::new(
                            codes::UNUSED_PARAM,
                            format!(
                                "parameter `{pname}` starts with `_`, which means it must stay unused; remove the `_` to use it"
                            ),
                            use_span,
                        ));
                    }
                }
                self.return_type = saved_return;
                self.underscore_params = saved_underscore;
                self.used_names = saved_used;
                self.pop();
            }
            Item::Const { value, .. } => {
                self.expr(value)?;
            }
            Item::Expr(e, _) => self.expr(e)?,
            Item::Struct { .. } | Item::Enum { .. } | Item::Alias { .. } | Item::Use { .. } => {}
        }
        Ok(())
    }

    /// Validate a pattern: every variant pattern names a declared variant,
    /// and a pattern never binds the same name twice.
    fn check_pattern(&self, pat: &Pattern) -> Result<()> {
        match pat {
            Pattern::Variant(tag, ps) => {
                if !self.variants.contains_key(tag) && !self.types.contains_key(tag) {
                    return Err(Diag::new(
                        codes::UNKNOWN_TYPE,
                        format!("`{tag}` is not a declared enum variant"),
                        Span::default(),
                    ));
                }
                for p in ps {
                    self.check_pattern(p)?;
                }
            }
            Pattern::List(ps) => {
                for p in ps {
                    self.check_pattern(p)?;
                }
            }
            _ => {}
        }
        let mut seen: Vec<String> = Vec::new();
        for b in pat.bindings() {
            if seen.contains(&b) {
                return Err(Diag::new(
                    codes::DUPLICATE_BINDING,
                    format!("`{b}` is bound more than once in the same pattern"),
                    Span::default(),
                ));
            }
            seen.push(b);
        }
        Ok(())
    }

    /// Infer the type of an expression, conservatively. Returns
    /// [`Ty::Unknown`] whenever the checker cannot be certain.
    fn infer(&self, e: &Expr) -> Ty {
        match e {
            Expr::Lit(l, _) => match l {
                Lit::Int(_) => Ty::Int,
                Lit::Float(_) => Ty::Float,
                Lit::Str(_) => Ty::String,
                Lit::Bool(_) => Ty::Bool,
                Lit::None => Ty::Unknown,
            },
            Expr::FStr(..) => Ty::String,
            Expr::List(items, _) => {
                let mut elem = Ty::Unknown;
                for item in items {
                    let t = self.infer(item);
                    if !matches!(t, Ty::Unknown) {
                        elem = t;
                        break;
                    }
                }
                Ty::List(Box::new(elem))
            }
            Expr::Map(entries, _) => {
                let mut val = Ty::Unknown;
                for (_, v) in entries {
                    let t = self.infer(v);
                    if !matches!(t, Ty::Unknown) {
                        val = t;
                        break;
                    }
                }
                Ty::Map(Box::new(val))
            }
            Expr::Construct(name, _, _) => {
                if self.variants.contains_key(name) {
                    Ty::Unknown
                } else {
                    Ty::Named(name.clone())
                }
            }
            Expr::Unary(UnOp::Not, _, _) => Ty::Bool,
            Expr::Unary(UnOp::Neg, inner, _) => self.infer(inner),
            Expr::Binary(op, l, _, _) => match op {
                BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::And
                | BinOp::Or => Ty::Bool,
                _ => self.infer(l),
            },
            Expr::If(_, _, _, _) | Expr::Match(_, _, _) | Expr::Block(_, _) => Ty::Unknown,
            Expr::Name(name, _) => {
                for scope in self.value_types.iter().rev() {
                    if let Some(t) = scope.get(name) {
                        return t.clone();
                    }
                }
                Ty::Unknown
            }
            Expr::Pipe(_, _, _)
            | Expr::Call(_, _, _)
            | Expr::Method(_, _, _, _)
            | Expr::Field(_, _, _)
            | Expr::Index(_, _, _)
            | Expr::Tuple(_, _)
            | Expr::Lambda(_, _, _) => Ty::Unknown,
        }
    }

    fn annotation(&self, t: &TypeExpr, span: Span) -> Result<Ty> {
        Ty::from_expr(t, &self.type_kinds, span)
    }

    fn block(&mut self, body: &[Stmt]) -> Result<()> {
        self.push();
        for s in body {
            self.stmt(s)?;
        }
        self.pop();
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> Result<()> {
        match s {
            Stmt::Let {
                mutable,
                name,
                ann,
                value,
                span,
            } => {
                self.expr(value)?;
                if let Some(ann) = ann {
                    let expected = self.annotation(ann, *span)?;
                    let actual = self.infer(value);
                    if !expected.compatible_with(&actual) {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!(
                                "`{name}` is annotated as `{}` but its value is `{}`",
                                expected.name(),
                                actual.name()
                            ),
                            *span,
                        ));
                    }
                    self.value_types
                        .last_mut()
                        .map(|m| m.insert(name.clone(), expected));
                }
                self.declare(name, *mutable, *span)?;
            }
            Stmt::Assign {
                target,
                value,
                op: _,
                span,
            } => {
                self.expr(value)?;
                match target {
                    Expr::Name(name, nspan) => match self.lookup(name) {
                        Some(true) => {}
                        Some(false) => {
                            return Err(Diag::new(
                                codes::ASSIGN_IMMUTABLE,
                                format!(
                                "cannot assign to `{name}`: it is immutable (declare it `let mut`)"
                            ),
                                *nspan,
                            ))
                        }
                        None => {
                            return Err(Diag::new(
                                codes::UNDEFINED,
                                format!("undefined variable `{name}`"),
                                *nspan,
                            ))
                        }
                    },
                    Expr::Index(base, idx, _) => {
                        self.expr(base)?;
                        self.expr(idx)?;
                    }
                    Expr::Field(base, _, _) => self.expr(base)?,
                    _ => {
                        return Err(Diag::new(
                            codes::INVALID_ASSIGN,
                            "invalid assignment target",
                            *span,
                        ))
                    }
                }
            }
            Stmt::Expr(e, _) => self.expr(e)?,
            Stmt::Return(v, span) => {
                if let Some(v) = v {
                    self.expr(v)?;
                    if let Some(expected) = self.return_type.clone() {
                        let actual = self.infer(v);
                        if !expected.compatible_with(&actual) {
                            return Err(Diag::new(
                                codes::RETURN_MISMATCH,
                                format!(
                                    "function returns `{}` but the value is `{}`",
                                    expected.name(),
                                    actual.name()
                                ),
                                *span,
                            ));
                        }
                    }
                }
            }
            Stmt::Throw(v, _) => self.expr(v)?,
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::While(c, body, _) => {
                self.expr(c)?;
                self.block(body)?;
            }
            Stmt::Loop(body, _) => self.block(body)?,
            Stmt::For(pat, iter, body, _) => {
                self.expr(iter)?;
                self.push();
                for b in pat.bindings() {
                    self.declare(&b, false, Span::default())?;
                }
                for s in body {
                    self.stmt(s)?;
                }
                self.pop();
            }
            Stmt::Try {
                body,
                catch,
                catch_body,
                finally,
                ..
            } => {
                self.block(body)?;
                self.push();
                self.declare(catch, false, Span::default())?;
                for s in catch_body {
                    self.stmt(s)?;
                }
                self.pop();
                if let Some(f) = finally {
                    self.block(f)?;
                }
            }
        }
        Ok(())
    }

    fn expr(&mut self, e: &Expr) -> Result<()> {
        match e {
            Expr::Lit(_, _) => {}
            Expr::Name(name, span) => {
                if self.lookup(name).is_none() {
                    return Err(Diag::new(
                        codes::UNDEFINED,
                        format!("undefined variable `{name}`"),
                        *span,
                    ));
                }
                self.used_names.entry(name.clone()).or_insert(*span);
            }
            Expr::FStr(parts, _) => {
                for p in parts {
                    if let FPart::Expr(inner) = p {
                        self.expr(inner)?;
                    }
                }
            }
            Expr::Unary(_, o, _) => self.expr(o)?,
            Expr::Binary(_, l, r, _) => {
                self.expr(l)?;
                self.expr(r)?;
            }
            Expr::Call(f, args, _) => {
                match f.as_ref() {
                    Expr::Name(name, span) => {
                        // A direct call must resolve to a function, a builtin,
                        // or a callable local. Functions are hoisted, so
                        // forward references resolve.
                        let known = self.functions.contains_key(name)
                            || crate::stdlib::builtin_names().contains(&name.as_str())
                            || self.lookup(name).is_some();
                        if !known {
                            return Err(Diag::new(
                                codes::UNDEFINED,
                                format!("undefined function `{name}`"),
                                *span,
                            ));
                        }
                    }
                    other => self.expr(other)?,
                }
                for a in args {
                    self.expr(a)?;
                }
            }
            Expr::Method(r, _, args, _) => {
                self.expr(r)?;
                for a in args {
                    self.expr(a)?;
                }
            }
            Expr::Field(r, _, _) => self.expr(r)?,
            Expr::Index(b, i, _) => {
                self.expr(b)?;
                self.expr(i)?;
            }
            Expr::List(vs, _) | Expr::Tuple(vs, _) => {
                for v in vs {
                    self.expr(v)?;
                }
            }
            Expr::Map(kvs, _) => {
                for (k, v) in kvs {
                    self.expr(k)?;
                    self.expr(v)?;
                }
            }
            Expr::Construct(name, args, span) => {
                // The name must be a declared struct or an enum variant.
                if !self.types.contains_key(name) && !self.variants.contains_key(name) {
                    return Err(Diag::new(
                        codes::UNKNOWN_TYPE,
                        format!("`{name}` is not a declared struct or enum variant"),
                        *span,
                    ));
                }
                for a in args {
                    self.expr(&a.value)?;
                }
            }
            Expr::Lambda(ps, body, _) => {
                self.push();
                for p in ps {
                    self.declare(p, false, Span::default())?;
                }
                self.expr(body)?;
                self.pop();
            }
            Expr::Pipe(l, r, _) => {
                self.expr(l)?;
                self.expr(r)?;
            }
            Expr::If(c, then, els, _) => {
                self.expr(c)?;
                self.block(then)?;
                if let Some(e) = els {
                    self.expr(e)?;
                }
            }
            Expr::Match(subject, arms, _) => {
                self.expr(subject)?;
                for arm in arms {
                    self.check_pattern(&arm.pattern)?;
                    self.push();
                    for b in arm.pattern.bindings() {
                        self.declare(&b, false, Span::default())?;
                    }
                    if let Some(g) = &arm.guard {
                        self.expr(g)?;
                    }
                    for s in &arm.body {
                        self.stmt(s)?;
                    }
                    self.pop();
                }
            }
            Expr::Block(body, _) => self.block(body)?,
        }
        Ok(())
    }
}
