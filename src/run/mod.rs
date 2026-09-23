//! The tree-walking interpreter.
//!
//! Control flow (`return`, `break`, `continue`, `throw`) is represented as a
//! first-class value of [`Ctl`], so it propagates through expression position
//! (for example `if cond { break }`) without special cases.

pub mod value;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::error::{codes, Diag, Result, Span};
use value::{Instance, Value, Variant};

/// Maximum number of simultaneously active user call frames, including the
/// entry call to `main`. Exceeding it is `E4011`. This is the single
/// authoritative recursion policy for the language; the host stack is sized
/// large enough that a host overflow is unreachable before this limit.
pub const MAX_CALL_FRAMES: usize = 512;

/// Maximum expression nesting depth the evaluator will descend. Bounds host
/// stack usage for deeply nested expressions.
pub const MAX_AST_DEPTH: usize = 256;

/// A user-defined function.
#[derive(Debug)]
pub struct Closure {
    /// Name (for diagnostics).
    pub name: String,
    /// Parameter names.
    pub params: Vec<String>,
    /// Body.
    pub body: Vec<Stmt>,
    /// Captured environment.
    pub env: Env,
}

/// An environment: a scope chain of bindings.
#[derive(Debug, Clone)]
pub struct Env(Rc<RefCell<EnvData>>);

#[derive(Debug)]
struct EnvData {
    vars: HashMap<String, (Value, bool)>,
    parent: Option<Env>,
}

impl Env {
    /// A fresh root environment.
    #[must_use]
    pub fn root() -> Env {
        Env(Rc::new(RefCell::new(EnvData {
            vars: HashMap::new(),
            parent: None,
        })))
    }

    /// A child scope.
    #[must_use]
    pub fn child(&self) -> Env {
        Env(Rc::new(RefCell::new(EnvData {
            vars: HashMap::new(),
            parent: Some(self.clone()),
        })))
    }

    /// Define a binding.
    pub fn define(&self, name: impl Into<String>, value: Value, mutable: bool) {
        self.0
            .borrow_mut()
            .vars
            .insert(name.into(), (value, mutable));
    }

    /// Read a binding.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Value> {
        let mut cur = Some(self.0.clone());
        while let Some(data) = cur {
            if let Some((v, _)) = data.borrow().vars.get(name) {
                return Some(v.clone());
            }
            cur = data.borrow().parent.as_ref().map(|e| e.0.clone());
        }
        None
    }

    /// Assign to an existing mutable binding.
    fn assign(&self, name: &str, value: Value) -> std::result::Result<(), AssignError> {
        let mut cur = Some(self.0.clone());
        while let Some(data) = cur {
            let parent = {
                let mut b = data.borrow_mut();
                if let Some(slot) = b.vars.get_mut(name) {
                    if slot.1 {
                        slot.0 = value;
                        return Ok(());
                    }
                    return Err(AssignError::Immutable);
                }
                b.parent.as_ref().map(|e| e.0.clone())
            };
            cur = parent;
        }
        Err(AssignError::Undefined)
    }
}

enum AssignError {
    Immutable,
    Undefined,
}

/// The result of evaluating an expression or statement.
pub enum Ctl {
    /// A normal value.
    Val(Value),
    /// `return v`
    Return(Value),
    /// `break`
    Break,
    /// `continue`
    Continue,
    /// `throw v`
    Throw(Value),
}

impl Ctl {
    /// Extract a value, or propagate the control flow as a diagnostic when
    /// encountered in a position where it is not allowed.
    fn value(self, it: &Interp, span: Span) -> Result<Value> {
        match self {
            Ctl::Val(v) => Ok(v),
            Ctl::Throw(v) => Err(it.error(
                codes::FOREIGN,
                format!("uncaught value: {}", v.display()),
                span,
            )),
            Ctl::Return(_) => Err(it.error(
                codes::RETURN_POSITION,
                "`return` cannot be used as a value here",
                span,
            )),
            Ctl::Break => Err(it.error(codes::LOOP_CONTROL, "`break` outside a loop", span)),
            Ctl::Continue => Err(it.error(codes::LOOP_CONTROL, "`continue` outside a loop", span)),
        }
    }
}

/// The interpreter.
pub struct Interp {
    globals: Env,
    natives: HashMap<String, Native>,

    functions: HashMap<String, Rc<Closure>>,
    structs: HashMap<String, Vec<String>>,
    variants: HashMap<String, (String, usize)>,
    depth: usize,
    /// Current expression nesting depth, guarding against host stack
    /// exhaustion from deeply nested (but syntactically flat) programs.
    ast_depth: usize,
    /// A thrown value in flight across a call boundary; consumed by `try`.
    pending_throw: Option<Value>,
    /// Output sink for `print`.
    pub stdout: Box<dyn std::io::Write>,
}

type Native = Rc<dyn Fn(&mut Interp, Vec<Value>, Span) -> Result<Value>>;

impl Interp {
    /// Create an interpreter with the standard library installed.
    #[must_use]
    pub fn new() -> Interp {
        let mut it = Interp {
            globals: Env::root(),
            natives: HashMap::new(),
            functions: HashMap::new(),
            structs: HashMap::new(),
            variants: HashMap::new(),
            depth: 0,
            ast_depth: 0,
            pending_throw: None,
            stdout: Box::new(std::io::stdout()),
        };
        crate::stdlib::install(&mut it);
        it
    }

    /// Create an interpreter that discards output.
    #[must_use]
    pub fn silent() -> Interp {
        let mut it = Interp::new();
        it.stdout = Box::new(std::io::sink());
        it
    }

    /// Register a native function.
    pub fn native<F>(&mut self, name: &str, f: F)
    where
        F: Fn(&mut Interp, Vec<Value>, Span) -> Result<Value> + 'static,
    {
        self.natives.insert(name.to_string(), Rc::new(f));
    }

    /// Register a global value.
    pub fn global(&mut self, name: &str, v: Value) {
        self.globals.define(name.to_string(), v, false);
    }

    /// Execute a module and call `main` if present.
    ///
    /// Initialization order is deliberate and matches the checker:
    /// declarations (functions, structs, enums) are registered first, then
    /// top-level constants and expressions are evaluated in source order.
    /// This makes forward references between functions valid.
    pub fn run(&mut self, module: &Module) -> Result<()> {
        // Pass 1: declarations.
        for item in &module.items {
            self.declare_item(item);
        }
        // Pass 2: initialization in source order.
        for item in &module.items {
            match item {
                Item::Const { name, value, .. } => {
                    let globals = self.globals.clone();
                    let v = self.eval(value, &globals)?.value(self, Span::default())?;
                    self.globals.define(name.clone(), v, false);
                }
                Item::Expr(e, _) => {
                    let globals = self.globals.clone();
                    self.eval(e, &globals)?.value(self, Span::default())?;
                }
                _ => {}
            }
        }
        if let Some(main) = self.functions.get("main").cloned() {
            if let Err(d) = self.call(&main, Vec::new(), Span::default()) {
                if d.code == codes::THROWN {
                    let shown = self
                        .pending_throw
                        .take()
                        .map_or_else(|| "uncaught value".to_string(), |v| v.display());
                    return Err(self.error(
                        codes::FOREIGN,
                        format!("uncaught value: {shown}"),
                        Span::default(),
                    ));
                }
                return Err(d);
            }
        }
        Ok(())
    }

    /// Register one declaration into the interpreter's symbol tables.
    fn declare_item(&mut self, item: &Item) {
        match item {
            Item::Fn {
                name, params, body, ..
            } => {
                let closure = Rc::new(Closure {
                    name: name.clone(),
                    params: params.iter().map(|p| p.name.clone()).collect(),
                    body: body.clone(),
                    env: self.globals.clone(),
                });
                self.functions.insert(name.clone(), closure);
            }
            Item::Struct { name, fields, .. } => {
                self.structs.insert(
                    name.clone(),
                    fields.iter().map(|(f, _)| f.clone()).collect(),
                );
            }
            Item::Enum { name, variants, .. } => {
                for (tag, payload) in variants {
                    self.variants
                        .insert(tag.clone(), (name.clone(), payload.len()));
                }
            }
            _ => {}
        }
    }

    fn error(&self, code: u16, msg: impl Into<String>, span: Span) -> Diag {
        Diag::new(code, msg, span)
    }

    /// Call a user closure, returning its value.
    pub fn call(&mut self, closure: &Rc<Closure>, args: Vec<Value>, span: Span) -> Result<Value> {
        if args.len() != closure.params.len() {
            return Err(self.error(
                codes::TYPE_MISMATCH,
                format!(
                    "`{}` expects {} argument(s), got {}",
                    closure.name,
                    closure.params.len(),
                    args.len()
                ),
                span,
            ));
        }
        let env = closure.env.child();
        for (p, v) in closure.params.iter().zip(args) {
            env.define(p.clone(), v, false);
        }
        // The limit counts every active call frame, including the entry call
        // to `main`. Exceeding it is a language-level diagnostic (E4011), not
        // a host stack overflow.
        self.depth += 1;
        if self.depth > MAX_CALL_FRAMES {
            self.depth -= 1;
            return Err(self.error(codes::RECURSION, "call depth limit exceeded", span));
        }
        // Expression nesting is scoped to a single call body: a callee starts
        // with a fresh nesting budget, so recursion is bounded by the call
        // limit (E4011) rather than by the expression-nesting limit.
        let saved_ast_depth = std::mem::take(&mut self.ast_depth);
        let r = self.exec_block(&closure.body, &env, false);
        self.ast_depth = saved_ast_depth;
        self.depth -= 1;
        match r? {
            Ctl::Return(v) | Ctl::Val(v) => Ok(v),
            Ctl::Throw(v) => {
                self.pending_throw = Some(v.clone());
                Err(self.error(
                    codes::THROWN,
                    format!("uncaught value: {}", v.display()),
                    span,
                ))
            }
            Ctl::Break => Err(self.error(codes::LOOP_CONTROL, "`break` outside a loop", span)),
            Ctl::Continue => {
                Err(self.error(codes::LOOP_CONTROL, "`continue` outside a loop", span))
            }
        }
    }

    fn exec_block(&mut self, body: &[Stmt], env: &Env, scoped: bool) -> Result<Ctl> {
        let local = if scoped { env.child() } else { env.clone() };
        let mut last = Value::None;
        for s in body {
            match self.exec_stmt(s, &local)? {
                Ctl::Val(v) => last = v,
                other => return Ok(other),
            }
        }
        Ok(Ctl::Val(last))
    }

    fn exec_stmt(&mut self, s: &Stmt, env: &Env) -> Result<Ctl> {
        match s {
            Stmt::Let {
                name,
                value,
                mutable,
                ..
            } => {
                let v = self.eval(value, env)?.value(self, Span::default())?;
                env.define(name.clone(), v, *mutable);
                Ok(Ctl::Val(Value::None))
            }
            Stmt::Assign {
                target,
                value,
                op,
                span,
            } => {
                let rhs = self.eval(value, env)?.value(self, *span)?;
                let new = match op {
                    None => rhs,
                    Some(op) => {
                        let cur = self.read_target(target, env)?;
                        self.binary(*op, cur, rhs, *span)?
                    }
                };
                self.write_target(target, new, env, *span)?;
                Ok(Ctl::Val(Value::None))
            }
            Stmt::Expr(e, _) => self.eval(e, env),
            Stmt::Return(v, _span) => {
                match v {
                    Some(v) => match self.eval(v, env)? {
                        Ctl::Val(value) => Ok(Ctl::Return(value)),
                        // A `return`/`break`/`throw` produced while computing
                        // the returned expression propagates directly; for
                        // example `return match x { 0 -> { return 5 } ... }`.
                        other => Ok(other),
                    },
                    None => Ok(Ctl::Return(Value::None)),
                }
            }
            Stmt::Throw(v, _span) => match self.eval(v, env)? {
                Ctl::Val(value) => Ok(Ctl::Throw(value)),
                other => Ok(other),
            },
            Stmt::Break(_) => Ok(Ctl::Break),
            Stmt::Continue(_) => Ok(Ctl::Continue),
            Stmt::While(cond, body, _) => loop {
                let c = self.eval(cond, env)?.value(self, Span::default())?;
                if !c.truthy() {
                    return Ok(Ctl::Val(Value::None));
                }
                match self.exec_block(body, env, true)? {
                    Ctl::Break => return Ok(Ctl::Val(Value::None)),
                    Ctl::Continue | Ctl::Val(_) => {}
                    other => return Ok(other),
                }
            },
            Stmt::Loop(body, _) => loop {
                match self.exec_block(body, env, true)? {
                    Ctl::Break => return Ok(Ctl::Val(Value::None)),
                    Ctl::Continue | Ctl::Val(_) => {}
                    other => return Ok(other),
                }
            },
            Stmt::For(pat, iter, body, span) => {
                let subject = self.eval(iter, env)?.value(self, *span)?;
                // Ranges iterate lazily, so a `break` on the first element of a
                // huge range never materializes the whole range.
                let run = |this: &mut Self, item: Value| -> Result<Option<Ctl>> {
                    let scope = env.child();
                    this.bind_pattern(pat, &item, &scope)?;
                    match this.exec_block(body, &scope, false)? {
                        Ctl::Break => Ok(Some(Ctl::Val(Value::None))),
                        Ctl::Continue | Ctl::Val(_) => Ok(None),
                        other => Ok(Some(other)),
                    }
                };
                if let Value::Range(r) = &subject {
                    let mut i = r.start;
                    while i < r.end {
                        if let Some(sig) = run(self, Value::Int(i))? {
                            return Ok(sig);
                        }
                        i += 1;
                    }
                    return Ok(Ctl::Val(Value::None));
                }
                let items = self.iterate(&subject, *span)?;
                for item in items {
                    if let Some(sig) = run(self, item)? {
                        return Ok(sig);
                    }
                }
                Ok(Ctl::Val(Value::None))
            }
            Stmt::Try {
                body,
                catch,
                catch_body,
                finally,
                span,
            } => {
                // Only an explicit `throw` is catchable. Runtime diagnostics
                // (division by zero, overflow, out-of-range access, ...) are
                // fatal and propagate, so internal error codes never leak into
                // program values as strings.
                let outcome = self.exec_block(body, env, true);
                let mut result = match outcome {
                    Ok(Ctl::Throw(v)) => {
                        let scope = env.child();
                        scope.define(catch.clone(), v, false);
                        self.exec_block(catch_body, &scope, false)
                    }
                    // A `throw` inside a called function crosses the call
                    // boundary as the internal THROWN signal; the pending
                    // value is the original thrown value.
                    Err(diag) if diag.code == codes::THROWN => {
                        let thrown = self
                            .pending_throw
                            .take()
                            .unwrap_or_else(|| Value::str(diag.message.clone()));
                        let scope = env.child();
                        scope.define(catch.clone(), thrown, false);
                        self.exec_block(catch_body, &scope, false)
                    }
                    Ok(flow) => Ok(flow),
                    Err(diag) => Err(diag),
                };
                if let Some(f) = finally {
                    let fin = self.exec_block(f, env, true)?;
                    if !matches!(fin, Ctl::Val(_)) {
                        result = Ok(fin);
                    }
                }
                let _ = span;
                result
            }
        }
    }

    fn read_target(&mut self, target: &Expr, env: &Env) -> Result<Value> {
        match target {
            Expr::Name(name, span) => env.get(name).ok_or_else(|| {
                self.error(
                    codes::UNDEFINED,
                    format!("undefined variable `{name}`"),
                    *span,
                )
            }),
            Expr::Index(b, i, span) => {
                let base = self.eval(b, env)?.value(self, *span)?;
                let idx = self.eval(i, env)?.value(self, *span)?;
                self.index_get(&base, &idx, *span)
            }
            Expr::Field(base, name, span) => {
                let subject = self.eval(base, env)?.value(self, *span)?;
                self.field_get(&subject, name, *span)
            }
            other => Err(self.error(
                codes::INVALID_ASSIGN,
                "invalid assignment target",
                span_of(other),
            )),
        }
    }

    fn write_target(&mut self, target: &Expr, value: Value, env: &Env, span: Span) -> Result<()> {
        match target {
            Expr::Name(name, nspan) => env.assign(name, value).map_err(|e| match e {
                AssignError::Immutable => self.error(
                    codes::ASSIGN_IMMUTABLE,
                    format!("cannot assign to `{name}`: it is immutable (declare it `let mut`)"),
                    *nspan,
                ),
                AssignError::Undefined => self.error(
                    codes::UNDEFINED,
                    format!("undefined variable `{name}`"),
                    *nspan,
                ),
            }),
            Expr::Index(b, i, ispan) => {
                let base = self.eval(b, env)?.value(self, *ispan)?;
                let idx = self.eval(i, env)?.value(self, *ispan)?;
                self.index_set(&base, &idx, value, *ispan)
            }
            Expr::Field(base, name, fspan) => {
                let subject = self.eval(base, env)?.value(self, *fspan)?;
                self.field_set(&subject, name, value, *fspan)
            }
            _ => Err(self.error(codes::INVALID_ASSIGN, "invalid assignment target", span)),
        }
    }

    // ------------------------------------------------------------ values

    fn iterate(&mut self, v: &Value, span: Span) -> Result<Vec<Value>> {
        match v {
            Value::List(l) => Ok(l.borrow().clone()),
            Value::Str(s) => Ok(s.chars().map(|c| Value::str(c.to_string())).collect()),
            Value::Map(m) => Ok(m.borrow().keys().map(Value::str).collect()),
            Value::Range(r) => {
                // Materializing a range is bounded so a pathological range
                // cannot exhaust memory; the cap is part of the runtime model.
                const MAX_RANGE_MATERIALIZE: i64 = 10_000_000;
                let n = r.len();
                if n > MAX_RANGE_MATERIALIZE {
                    return Err(self.error(
                        codes::OVERFLOW,
                        format!("range of {n} elements exceeds the {MAX_RANGE_MATERIALIZE} element limit"),
                        span,
                    ));
                }
                let mut out = Vec::with_capacity(usize::try_from(n).unwrap_or(0));
                let mut i = r.start;
                while i < r.end {
                    out.push(Value::Int(i));
                    i += 1;
                }
                Ok(out)
            }
            other => Err(self.error(
                codes::NOT_ITERABLE,
                format!("{} is not iterable", other.type_name()),
                span,
            )),
        }
    }

    fn index_get(&mut self, base: &Value, idx: &Value, span: Span) -> Result<Value> {
        match (base, idx) {
            (Value::List(l), Value::Int(i)) => {
                let l = l.borrow();
                let n = normalize(*i, l.len()).ok_or_else(|| {
                    self.error(codes::INDEX, format!("list index {i} out of range"), span)
                })?;
                Ok(l[n].clone())
            }
            (Value::Str(s), Value::Int(i)) => {
                let chars: Vec<char> = s.chars().collect();
                let n = normalize(*i, chars.len()).ok_or_else(|| {
                    self.error(codes::INDEX, format!("string index {i} out of range"), span)
                })?;
                Ok(Value::str(chars[n].to_string()))
            }
            (Value::Map(m), Value::Str(k)) => m.borrow().get(&**k).cloned().ok_or_else(|| {
                self.error(codes::UNDEFINED, format!("map has no key \"{k}\""), span)
            }),
            (Value::Instance(i), Value::Str(k)) => i
                .fields
                .borrow()
                .iter()
                .find(|(name, _)| name == &**k)
                .map(|(_, v)| v.clone())
                .ok_or_else(|| {
                    self.error(
                        codes::UNDEFINED,
                        format!("{} has no field `{k}`", i.ty),
                        span,
                    )
                }),
            _ => Err(self.error(
                codes::TYPE_MISMATCH,
                format!("cannot index {} with {}", base.type_name(), idx.type_name()),
                span,
            )),
        }
    }

    fn index_set(&mut self, base: &Value, idx: &Value, value: Value, span: Span) -> Result<()> {
        match (base, idx) {
            (Value::List(l), Value::Int(i)) => {
                let mut l = l.borrow_mut();
                let n = normalize(*i, l.len()).ok_or_else(|| {
                    self.error(codes::INDEX, format!("list index {i} out of range"), span)
                })?;
                l[n] = value;
                Ok(())
            }
            (Value::Map(m), Value::Str(k)) => {
                m.borrow_mut().insert((*k).to_string(), value);
                Ok(())
            }
            (Value::Instance(i), Value::Str(k)) => {
                let mut fields = i.fields.borrow_mut();
                match fields.iter_mut().find(|(name, _)| name == &**k) {
                    Some(slot) => {
                        slot.1 = value;
                        Ok(())
                    }
                    None => Err(self.error(
                        codes::UNDEFINED,
                        format!("{} has no field `{k}`", i.ty),
                        span,
                    )),
                }
            }
            _ => Err(self.error(codes::INVALID_ASSIGN, "invalid assignment target", span)),
        }
    }

    fn field_get(&mut self, base: &Value, name: &str, span: Span) -> Result<Value> {
        match base {
            Value::Instance(i) => i
                .fields
                .borrow()
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
                .ok_or_else(|| {
                    self.error(
                        codes::UNDEFINED,
                        format!("{} has no field `{name}`", i.ty),
                        span,
                    )
                }),
            other => Err(self.error(
                codes::TYPE_MISMATCH,
                format!("{} has no fields or methods", other.type_name()),
                span,
            )),
        }
    }

    fn field_set(&mut self, base: &Value, name: &str, value: Value, span: Span) -> Result<()> {
        match base {
            Value::Instance(i) => {
                let mut fields = i.fields.borrow_mut();
                match fields.iter_mut().find(|(k, _)| k == name) {
                    Some(slot) => {
                        slot.1 = value;
                        Ok(())
                    }
                    None => Err(self.error(
                        codes::UNDEFINED,
                        format!("{} has no field `{name}`", i.ty),
                        span,
                    )),
                }
            }
            other => Err(self.error(
                codes::TYPE_MISMATCH,
                format!("{} has no fields", other.type_name()),
                span,
            )),
        }
    }

    fn bind_pattern(&mut self, pat: &Pattern, value: &Value, env: &Env) -> Result<()> {
        match pat {
            Pattern::Bind(n) => {
                if n != "_" {
                    env.define(n.clone(), value.clone(), false);
                }
                Ok(())
            }
            Pattern::List(ps) => match value {
                Value::List(l) => {
                    let l = l.borrow();
                    if l.len() != ps.len() {
                        return Err(self.error(
                            codes::TYPE_MISMATCH,
                            "list pattern arity mismatch",
                            Span::default(),
                        ));
                    }
                    for (p, v) in ps.iter().zip(l.iter()) {
                        self.bind_pattern(p, v, env)?;
                    }
                    Ok(())
                }
                _ => Err(self.error(
                    codes::TYPE_MISMATCH,
                    "pattern expects a list",
                    Span::default(),
                )),
            },
            Pattern::Variant(tag, ps) => match value {
                Value::Variant(v) => {
                    if &v.tag != tag || v.payload.len() != ps.len() {
                        return Err(self.error(
                            codes::TYPE_MISMATCH,
                            "variant pattern mismatch",
                            Span::default(),
                        ));
                    }
                    for (p, val) in ps.iter().zip(v.payload.iter()) {
                        self.bind_pattern(p, val, env)?;
                    }
                    Ok(())
                }
                _ => Err(self.error(
                    codes::TYPE_MISMATCH,
                    "pattern expects a variant",
                    Span::default(),
                )),
            },
            _ => Ok(()),
        }
    }

    fn match_pattern(&self, pat: &Pattern, value: &Value) -> bool {
        match pat {
            Pattern::Bind(_) => true,
            Pattern::Int(i) => matches!(value, Value::Int(n) if n == i),
            Pattern::Str(s) => matches!(value, Value::Str(n) if &**n == s),
            Pattern::Bool(b) => matches!(value, Value::Bool(n) if n == b),
            Pattern::None => matches!(value, Value::None),
            Pattern::List(ps) => match value {
                Value::List(l) => {
                    let l = l.borrow();
                    l.len() == ps.len()
                        && ps
                            .iter()
                            .zip(l.iter())
                            .all(|(p, v)| self.match_pattern(p, v))
                }
                _ => false,
            },
            Pattern::Variant(tag, ps) => match value {
                Value::Variant(v) => {
                    &v.tag == tag
                        && v.payload.len() == ps.len()
                        && ps
                            .iter()
                            .zip(v.payload.iter())
                            .all(|(p, val)| self.match_pattern(p, val))
                }
                _ => false,
            },
        }
    }

    // ------------------------------------------------------- expressions

    /// Evaluate an expression, propagating control flow.
    pub(crate) fn eval(&mut self, e: &Expr, env: &Env) -> Result<Ctl> {
        self.ast_depth += 1;
        if self.ast_depth > MAX_AST_DEPTH {
            self.ast_depth -= 1;
            return Err(self.error(
                codes::NESTING,
                "expression nests too deeply to evaluate",
                span_of(e),
            ));
        }
        let result = self.eval_inner(e, env);
        self.ast_depth -= 1;
        result
    }

    fn eval_inner(&mut self, e: &Expr, env: &Env) -> Result<Ctl> {
        macro_rules! val {
            ($e:expr) => {
                match $e? {
                    Ctl::Val(v) => v,
                    other => return Ok(other),
                }
            };
        }
        match e {
            Expr::Lit(l, _) => Ok(Ctl::Val(match l {
                Lit::Int(i) => Value::Int(*i),
                Lit::Float(f) => Value::Float(*f),
                Lit::Str(s) => Value::str(s.clone()),
                Lit::Bool(b) => Value::Bool(*b),
                Lit::None => Value::None,
            })),
            Expr::Name(name, span) => {
                if let Some(v) = env.get(name) {
                    Ok(Ctl::Val(v))
                } else if let Some(closure) = self.functions.get(name).cloned() {
                    // A named function used as a value.
                    Ok(Ctl::Val(Value::Closure(closure)))
                } else if self.natives.contains_key(name) {
                    // A native used as a value: wrap it in a callable value.
                    Ok(Ctl::Val(Value::Native(name.clone())))
                } else {
                    Err(self.error(
                        codes::UNDEFINED,
                        format!("undefined variable `{name}`"),
                        *span,
                    ))
                }
            }
            Expr::FStr(parts, _) => {
                let mut out = String::new();
                for p in parts {
                    match p {
                        FPart::Lit(t) => out.push_str(t),
                        FPart::Expr(e) => {
                            let v = val!(self.eval(e, env));
                            out.push_str(&v.display());
                        }
                    }
                }
                Ok(Ctl::Val(Value::str(out)))
            }
            Expr::Unary(op, operand, span) => {
                let v = val!(self.eval(operand, env));
                match op {
                    UnOp::Neg => match v {
                        Value::Int(i) => {
                            Ok(Ctl::Val(Value::Int(i.checked_neg().ok_or_else(|| {
                                self.error(codes::OVERFLOW, "integer overflow", *span)
                            })?)))
                        }
                        Value::Float(f) => Ok(Ctl::Val(Value::Float(-f))),
                        other => Err(self.error(
                            codes::TYPE_MISMATCH,
                            format!("cannot negate {}", other.type_name()),
                            *span,
                        )),
                    },
                    UnOp::Not => Ok(Ctl::Val(Value::Bool(!v.truthy()))),
                }
            }
            Expr::Binary(op, l, r, span) => {
                if matches!(op, BinOp::And) {
                    let lv = val!(self.eval(l, env));
                    if !lv.truthy() {
                        return Ok(Ctl::Val(Value::Bool(false)));
                    }
                    let rv = val!(self.eval(r, env));
                    return Ok(Ctl::Val(Value::Bool(rv.truthy())));
                }
                if matches!(op, BinOp::Or) {
                    let lv = val!(self.eval(l, env));
                    if lv.truthy() {
                        return Ok(Ctl::Val(Value::Bool(true)));
                    }
                    let rv = val!(self.eval(r, env));
                    return Ok(Ctl::Val(Value::Bool(rv.truthy())));
                }
                let lv = val!(self.eval(l, env));
                let rv = val!(self.eval(r, env));
                Ok(Ctl::Val(self.binary(*op, lv, rv, *span)?))
            }
            Expr::Call(callee, args, span) => self.eval_call(callee, args, env, *span),
            Expr::Method(recv, name, args, span) => {
                let subject = val!(self.eval(recv, env));
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(val!(self.eval(a, env)));
                }
                Ok(Ctl::Val(self.method(&subject, name, vals, *span)?))
            }
            Expr::Field(recv, name, span) => {
                let subject = val!(self.eval(recv, env));
                match &subject {
                    Value::Instance(_) => Ok(Ctl::Val(self.field_get(&subject, name, *span)?)),
                    _ => Ok(Ctl::Val(self.method(&subject, name, Vec::new(), *span)?)),
                }
            }
            Expr::Index(base, idx, span) => {
                let b = val!(self.eval(base, env));
                let i = val!(self.eval(idx, env));
                Ok(Ctl::Val(self.index_get(&b, &i, *span)?))
            }
            Expr::List(items, _) => {
                let mut out = Vec::with_capacity(items.len());
                for i in items {
                    out.push(val!(self.eval(i, env)));
                }
                Ok(Ctl::Val(Value::list(out)))
            }
            Expr::Map(entries, _) => {
                let mut map = std::collections::BTreeMap::new();
                for (k, v) in entries {
                    let kv = val!(self.eval(k, env));
                    let key = match kv {
                        Value::Str(s) => s.to_string(),
                        other => {
                            return Err(self.error(
                                codes::TYPE_MISMATCH,
                                format!("map keys must be strings, found {}", other.type_name()),
                                span_of(k),
                            ))
                        }
                    };
                    let vv = val!(self.eval(v, env));
                    map.insert(key, vv);
                }
                Ok(Ctl::Val(Value::Map(Rc::new(RefCell::new(map)))))
            }
            Expr::Construct(name, args, span) => self.construct(name, args, env, *span),
            Expr::Tuple(items, _) => {
                let mut out = Vec::with_capacity(items.len());
                for i in items {
                    out.push(val!(self.eval(i, env)));
                }
                Ok(Ctl::Val(Value::list(out)))
            }
            Expr::Lambda(params, body, _) => {
                let body_stmts = match body.as_ref() {
                    // A block-bodied lambda uses its block as the function
                    // body, so an explicit `return` inside it works and the
                    // last expression is the implicit return value.
                    Expr::Block(stmts, _) => stmts.clone(),
                    expr => vec![Stmt::Return(Some(expr.clone()), Span::default())],
                };
                Ok(Ctl::Val(Value::Closure(Rc::new(Closure {
                    name: "<lambda>".to_string(),
                    params: params.clone(),
                    body: body_stmts,
                    env: env.clone(),
                }))))
            }
            Expr::Pipe(l, r, span) => {
                let arg = val!(self.eval(l, env));
                let f = val!(self.eval(r, env));
                Ok(Ctl::Val(self.call_value(f, vec![arg], *span)?))
            }
            Expr::If(cond, then, els, _) => {
                let c = val!(self.eval(cond, env));
                if c.truthy() {
                    return self.exec_block(then, env, true);
                }
                if let Some(e) = els {
                    return self.eval(e, env);
                }
                Ok(Ctl::Val(Value::None))
            }
            Expr::Match(subject, arms, span) => {
                let s = val!(self.eval(subject, env));
                for arm in arms {
                    if self.match_pattern(&arm.pattern, &s) {
                        let scope = env.child();
                        self.bind_pattern(&arm.pattern, &s, &scope)?;
                        if let Some(g) = &arm.guard {
                            let gv = val!(self.eval(g, &scope));
                            if !gv.truthy() {
                                continue;
                            }
                        }
                        return self.exec_block(&arm.body, &scope, true);
                    }
                }
                Err(self.error(codes::NO_MATCH, "no match arm matched the value", *span))
            }
            Expr::Block(body, _) => self.exec_block(body, env, true),
        }
    }

    fn eval_call(&mut self, callee: &Expr, args: &[Expr], env: &Env, span: Span) -> Result<Ctl> {
        let mut vals = Vec::with_capacity(args.len());
        for a in args {
            match self.eval(a, env)? {
                Ctl::Val(v) => vals.push(v),
                other => return Ok(other),
            }
        }
        match callee {
            Expr::Name(name, nspan) => {
                if let Some(c) = self.functions.get(name).cloned() {
                    return Ok(Ctl::Val(self.call(&c, vals, *nspan)?));
                }
                if let Some(n) = self.natives.get(name).cloned() {
                    return Ok(Ctl::Val(n(self, vals, *nspan)?));
                }
                if let Some(f) = env.get(name) {
                    return Ok(Ctl::Val(self.call_value(f, vals, *nspan)?));
                }
                Err(self.error(
                    codes::UNDEFINED,
                    format!("undefined function `{name}`"),
                    *nspan,
                ))
            }
            other => {
                let f = self.eval(other, env)?;
                match f {
                    Ctl::Val(f) => Ok(Ctl::Val(self.call_value(f, vals, span)?)),
                    other => Ok(other),
                }
            }
        }
    }

    /// Call any callable value.
    pub fn call_value(&mut self, f: Value, args: Vec<Value>, span: Span) -> Result<Value> {
        match f {
            Value::Closure(c) => self.call(&c, args, span),
            Value::Native(name) => {
                let n = self.natives.get(&name).cloned().ok_or_else(|| {
                    self.error(codes::UNDEFINED, format!("unknown `{name}`"), span)
                })?;
                n(self, args, span)
            }
            other => Err(self.error(
                codes::TYPE_MISMATCH,
                format!("{} is not callable", other.type_name()),
                span,
            )),
        }
    }

    /// Public wrapper for stdlib higher-order functions.
    pub fn call_value_pub(&mut self, f: Value, args: Vec<Value>, span: Span) -> Result<Value> {
        self.call_value(f, args, span)
    }

    /// Evaluate an expression in the global scope (used by the REPL).
    pub fn eval_globals(&mut self, e: &Expr) -> Result<Ctl> {
        let globals = self.globals.clone();
        self.eval(e, &globals)
    }

    /// Execute a single statement in the global scope (used by the REPL).
    pub fn exec_stmt_globals(&mut self, s: &Stmt) -> Result<Ctl> {
        let globals = self.globals.clone();
        self.exec_stmt(s, &globals)
    }

    /// Register or execute a single top-level item (used by the REPL).
    pub fn run_item(&mut self, item: &Item) -> Result<()> {
        match item {
            Item::Const { name, value, .. } => {
                let globals = self.globals.clone();
                let v = self.eval(value, &globals)?.value(self, Span::default())?;
                self.globals.define(name.clone(), v, false);
                Ok(())
            }
            other => {
                self.declare_item(other);
                Ok(())
            }
        }
    }

    fn construct(&mut self, name: &str, args: &[Arg], env: &Env, span: Span) -> Result<Ctl> {
        let mut positional = Vec::new();
        let mut named = Vec::new();
        for a in args {
            match self.eval(&a.value, env)? {
                Ctl::Val(v) => match &a.name {
                    Some(n) => named.push((n.clone(), v)),
                    None => positional.push(v),
                },
                other => return Ok(other),
            }
        }
        if let Some(fields) = self.structs.get(name).cloned() {
            let mut values = Vec::new();
            if !named.is_empty() && !positional.is_empty() {
                return Err(self.error(
                    codes::TYPE_MISMATCH,
                    format!("`{name}` mixes named and positional fields; use one form"),
                    span,
                ));
            }
            if !named.is_empty() {
                // Reject names that are not declared fields and duplicates,
                // rather than silently ignoring them.
                for (n, _) in &named {
                    if !fields.iter().any(|f| f == n) {
                        return Err(self.error(
                            codes::UNDEFINED,
                            format!("`{name}` has no field `{n}`"),
                            span,
                        ));
                    }
                    if named.iter().filter(|(m, _)| m == n).count() > 1 {
                        return Err(self.error(
                            codes::TYPE_MISMATCH,
                            format!("field `{n}` is given more than once for `{name}`"),
                            span,
                        ));
                    }
                }
                for f in &fields {
                    let found = named.iter().find(|(n, _)| n == f).map(|(_, v)| v.clone());
                    match found {
                        Some(v) => values.push((f.clone(), v)),
                        None => {
                            return Err(self.error(
                                codes::TYPE_MISMATCH,
                                format!("missing field `{f}` for `{name}`"),
                                span,
                            ))
                        }
                    }
                }
            } else {
                if positional.len() != fields.len() {
                    return Err(self.error(
                        codes::TYPE_MISMATCH,
                        format!(
                            "`{name}` expects {} field(s), got {}",
                            fields.len(),
                            positional.len()
                        ),
                        span,
                    ));
                }
                for (f, v) in fields.iter().zip(positional) {
                    values.push((f.clone(), v));
                }
            }
            return Ok(Ctl::Val(Value::Instance(Rc::new(Instance {
                ty: name.to_string(),
                fields: RefCell::new(values),
            }))));
        }
        if let Some((ty, arity)) = self.variants.get(name).cloned() {
            if let Some((n, _)) = named.first() {
                return Err(self.error(
                    codes::TYPE_MISMATCH,
                    format!("variant `{name}` is positional; `{n}: ...` is not allowed here"),
                    span,
                ));
            }
            if positional.len() != arity {
                return Err(self.error(
                    codes::TYPE_MISMATCH,
                    format!("variant `{name}` expects {arity} value(s)"),
                    span,
                ));
            }
            return Ok(Ctl::Val(Value::Variant(Rc::new(Variant {
                ty,
                tag: name.to_string(),
                payload: positional,
            }))));
        }
        Err(self.error(
            codes::UNKNOWN_TYPE,
            format!("unknown constructor `{name}`"),
            span,
        ))
    }

    fn binary(&mut self, op: BinOp, l: Value, r: Value, span: Span) -> Result<Value> {
        use BinOp::{Add, Div, Eq, Ge, Gt, Le, Lt, Mul, Ne, Pow, Rem, Sub};
        match op {
            Eq => Ok(Value::Bool(l.equals(&r))),
            Ne => Ok(Value::Bool(!l.equals(&r))),
            Lt | Le | Gt | Ge => {
                // An unordered result (e.g. a NaN operand) yields `false`,
                // matching IEEE semantics; only genuinely incomparable *types*
                // are an error.
                match l.cmp_val(&r) {
                    Some(ord) => Ok(Value::Bool(match op {
                        Lt => ord.is_lt(),
                        Le => ord.is_le(),
                        Gt => ord.is_gt(),
                        _ => ord.is_ge(),
                    })),
                    None if l.comparable_with(&r) => Ok(Value::Bool(false)),
                    None => Err(self.error(
                        codes::TYPE_MISMATCH,
                        format!("cannot compare {} with {}", l.type_name(), r.type_name()),
                        span,
                    )),
                }
            }
            Add => match (&l, &r) {
                (Value::Int(a), Value::Int(b)) => {
                    Ok(Value::Int(a.checked_add(*b).ok_or_else(|| {
                        self.error(codes::OVERFLOW, "integer overflow", span)
                    })?))
                }
                (Value::Str(a), Value::Str(b)) => {
                    let mut s = a.to_string();
                    s.push_str(b);
                    Ok(Value::str(s))
                }
                (Value::List(a), Value::List(b)) => {
                    let mut out = a.borrow().clone();
                    out.extend(b.borrow().iter().cloned());
                    Ok(Value::list(out))
                }
                _ => self.numeric(op, l, r, span),
            },
            Sub | Mul | Div | Rem | Pow => self.numeric(op, l, r, span),
            // `and`/`or` are handled by short-circuit in `eval` and never
            // reach here; returning an internal error rather than panicking
            // keeps the "no panics" invariant even if that ever changes.
            BinOp::And | BinOp::Or => Err(self.error(
                codes::INTERNAL,
                "internal error: logical operator reached the arithmetic path",
                span,
            )),
        }
    }

    fn numeric(&mut self, op: BinOp, l: Value, r: Value, span: Span) -> Result<Value> {
        use BinOp::{Add, Div, Mul, Pow, Rem, Sub};
        match (&l, &r) {
            (Value::Int(a), Value::Int(b)) => {
                // `i64::MIN / -1` and `i64::MIN % -1` overflow; Rust's `/`
                // and `%` panic on those, so the checked forms are mandatory.
                let result = match op {
                    Add => a.checked_add(*b),
                    Sub => a.checked_sub(*b),
                    Mul => a.checked_mul(*b),
                    Div => {
                        if *b == 0 {
                            return Err(self.error(codes::DIV_ZERO, "division by zero", span));
                        }
                        a.checked_div(*b)
                    }
                    Rem => {
                        if *b == 0 {
                            return Err(self.error(codes::DIV_ZERO, "division by zero", span));
                        }
                        a.checked_rem(*b)
                    }
                    Pow => {
                        if *b < 0 {
                            None
                        } else {
                            u32::try_from(*b).ok().and_then(|e| a.checked_pow(e))
                        }
                    }
                    _ => None,
                };
                result
                    .map(Value::Int)
                    .ok_or_else(|| self.error(codes::OVERFLOW, "integer overflow", span))
            }
            _ => {
                let a = self.as_f64(&l, span)?;
                let b = self.as_f64(&r, span)?;
                Ok(Value::Float(match op {
                    Add => a + b,
                    Sub => a - b,
                    Mul => a * b,
                    Div => {
                        if b == 0.0 {
                            return Err(self.error(codes::DIV_ZERO, "division by zero", span));
                        }
                        a / b
                    }
                    Rem => {
                        // Contract §7: division by zero is E4007 for `%` as
                        // well, on both int and float. `b == 0.0` is true for
                        // `-0.0` too, so all zero spellings behave alike.
                        if b == 0.0 {
                            return Err(self.error(codes::DIV_ZERO, "division by zero", span));
                        }
                        a % b
                    }
                    Pow => a.powf(b),
                    _ => {
                        return Err(self.error(
                            codes::INTERNAL,
                            "internal error: non-arithmetic operator reached the numeric path",
                            span,
                        ))
                    }
                }))
            }
        }
    }

    fn as_f64(&self, v: &Value, span: Span) -> Result<f64> {
        match v {
            Value::Int(i) => Ok(*i as f64),
            Value::Float(f) => Ok(*f),
            other => Err(self.error(
                codes::TYPE_MISMATCH,
                format!("expected a number, found {}", other.type_name()),
                span,
            )),
        }
    }

    fn method(&mut self, recv: &Value, name: &str, args: Vec<Value>, span: Span) -> Result<Value> {
        crate::stdlib::method(self, recv, name, args, span)
    }
}

fn normalize(i: i64, len: usize) -> Option<usize> {
    // `len + i` can overflow when `i` is near `i64::MIN`; compute with i128.
    let len = i128::try_from(len).ok()?;
    let idx = if i < 0 {
        len.checked_add(i128::from(i))?
    } else {
        i128::from(i)
    };
    if idx < 0 || idx >= len {
        None
    } else {
        usize::try_from(idx).ok()
    }
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

impl Default for Interp {
    fn default() -> Interp {
        Interp::new()
    }
}
