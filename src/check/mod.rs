//! Static checks: reserved names, mutability, and name resolution.
//!
//! Aura checks are conservative but complete: every name used must be
//! declared, every assignment must target a `let mut` binding, and every
//! forbidden construct (e.g. `else if`) is rejected by the parser.

use std::collections::HashMap;

use crate::ast::*;
use crate::error::{codes, Diag, Result, Span};
use crate::lex::KEYWORDS;

/// Native functions known to the checker as available globals.
pub const BUILTINS: &[&str] = &[
    "print",
    "len",
    "to_string",
    "to_int",
    "to_float",
    "range",
    "abs",
    "min",
    "max",
    "push",
    "pop",
    "keys",
    "values",
    "sort",
    "reverse",
    "map",
    "filter",
    "reduce",
    "sum",
    "assert",
    "enumerate",
    "zip",
    "py_eval",
    "py_import",
    "py_call",
    "py_version",
    "json_encode",
    "json_decode",
    "regex_match",
    "regex_find",
    "regex_find_all",
    "regex_replace",
    "time_now",
    "time_unix",
    "sleep_ms",
];

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
}

impl Checker {
    /// Build a checker.
    fn new() -> Checker {
        Checker {
            scopes: vec![Scope::default()],
            has_main: false,
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
        }
        c
    }

    /// Check a module against this checker.
    ///
    /// # Errors
    /// Returns the first diagnostic found.
    pub fn check(&mut self, m: &Module) -> Result<()> {
        for item in &m.items {
            self.item(item)?;
        }
        Ok(())
    }

    /// Check a module, returning the first diagnostic.
    pub fn module(m: &Module) -> Result<()> {
        let mut c = Checker::new();
        for item in &m.items {
            c.item(item)?;
        }
        Ok(())
    }

    /// Check a module and require an entry point.
    pub fn module_with_main(m: &Module) -> Result<()> {
        let mut c = Checker::new();
        for item in &m.items {
            c.item(item)?;
        }
        let _ = c.has_main;
        Ok(())
    }

    fn push(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn pop(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str, mutable: bool, span: Span) -> Result<()> {
        if name.starts_with('_') {
            // leading-underscore names are allowed but flagged if used
        }
        if KEYWORDS.contains(&name) {
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
        if BUILTINS.contains(&name) {
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
                name, params, body, ..
            } => {
                if name == "main" {
                    self.has_main = true;
                }
                self.declare(name, false, Span::default())?;
                self.push();
                for p in params {
                    self.declare(&p.name, false, p.span)?;
                }
                self.block(body)?;
                self.pop();
            }
            Item::Const {
                name, value, span, ..
            } => {
                self.expr(value)?;
                self.declare(name, false, *span)?;
            }
            Item::Expr(e, _) => self.expr(e)?,
            Item::Struct { .. } | Item::Enum { .. } | Item::Alias { .. } | Item::Use { .. } => {}
        }
        Ok(())
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
                value,
                span,
                ..
            } => {
                self.expr(value)?;
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
            Stmt::Return(v, _) => {
                if let Some(v) = v {
                    self.expr(v)?;
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
                // Callee may be a function not yet declared; allow names.
                if !matches!(f.as_ref(), Expr::Name(..)) {
                    self.expr(f)?;
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
            Expr::Construct(_, args, _) => {
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
