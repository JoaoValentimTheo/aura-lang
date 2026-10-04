//! The tree-walking interpreter.
//!
//! Control flow (`return`, `break`, `continue`, `throw`) is represented as a
//! first-class value of [`Ctl`], so it propagates through expression position
//! (for example `if cond { break }`) without special cases.

pub mod value;

/// The explicit-continuation (iterative) evaluator — B-1R3A.
///
/// Compiled only with the non-default `evaluator-oracle` feature so default and
/// production builds contain no path to it (`ITERATIVE_EVALUATOR_DESIGN.md`
/// §24). Production execution stays on the recursive engine below.
#[cfg(feature = "evaluator-oracle")]
pub(crate) mod iterative;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use crate::ast::*;
use crate::error::{codes, Diag, Result, SourceDiagnostic, Span};
use crate::source::SourceId;
use value::{Instance, MapKey, Value, Variant};

/// Maximum number of simultaneously active user call frames, including the
/// entry call to `main`. Exceeding it is `E4011`. This is the single
/// authoritative recursion policy for the language; the host stack is sized
/// large enough that a host overflow is unreachable before this limit.
pub const MAX_CALL_FRAMES: usize = 512;

/// Maximum expression nesting depth the evaluator will descend. Bounds host
/// stack usage for deeply nested expressions.
pub const MAX_AST_DEPTH: usize = 256;

/// Maximum f-string format *precision* (digits after the decimal point).
///
/// Rust's `format!` rejects a width/precision above `u16::MAX` with a host
/// panic ("Formatting argument out of range"); Aura MUST NOT let a program
/// reach that panic (`LANGUAGE_SPEC.md` §31.5). This is the largest value the
/// formatter accepts, kept as the language's own bound so the limit produces a
/// stable `E4013` on every substrate rather than a panic (native) or a trap
/// (WebAssembly).
pub const MAX_FORMAT_PRECISION: usize = u16::MAX as usize;

/// Maximum f-string format *width* (minimum field width, in characters).
///
/// A field width pads the output with `fill`; an enormous width would allocate
/// an enormous string. Native would attempt the allocation (and eventually
/// abort) while WebAssembly traps on memory exhaustion, so the two substrates
/// would diverge. Bounding the width keeps the behavior identical on every
/// substrate and a stable `E4013` beyond the bound (`LANGUAGE_SPEC.md` §31.5).
/// The value matches the range-materialization cap (§31.4): a generous but
/// bounded amount of materialized output.
pub const MAX_FORMAT_WIDTH: usize = 10_000_000;

/// A user-defined function.
#[derive(Debug)]
pub struct Closure {
    /// Name (for diagnostics).
    pub name: String,
    /// Parameter names and whether each is declared `mut` (which grants
    /// mutable capability over the argument inside the body).
    pub params: Vec<(String, bool)>,
    /// Parameter type annotations, resolved at declaration time, aligned with
    /// `params`. `None` is an unannotated parameter. Used by overload
    /// resolution (`LANGUAGE_SPEC.md` §15.7).
    pub param_tys: Vec<Option<crate::types::Ty>>,
    /// Body.
    ///
    /// `Arc`-shared so the iterative evaluator can retain a pending body in a
    /// continuation without deep-cloning (`ITERATIVE_EVALUATOR_DESIGN.md`
    /// §16.1). Cloning a `Closure` body is a refcount bump, not a copy.
    pub body: Arc<[Stmt]>,
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

    /// Define `name` as an ordinary `let`/`let mut` binding, shadowing any
    /// binding of the same name already present in this frame.
    ///
    /// Returns the environment that subsequent statements must resolve names
    /// against: `self` for a fresh name, or a new child frame when an existing
    /// binding was shadowed. Creating a child frame, rather than overwriting
    /// the slot in place, is what preserves binding identity: a closure that
    /// already captured `self` keeps seeing the previous binding, while later
    /// code resolves the shadowing one (`LANGUAGE_SPEC.md` §16.3).
    #[must_use]
    pub fn define_shadowing(&self, name: impl Into<String>, value: Value, mutable: bool) -> Env {
        let name = name.into();
        if self.0.borrow().vars.contains_key(&name) {
            let child = self.child();
            child.define(name, value, mutable);
            child
        } else {
            self.define(name, value, mutable);
            self.clone()
        }
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
    /// Resumable natives (`map`/`filter`/`reduce`), keyed by name
    /// (`ITERATIVE_EVALUATOR_DESIGN.md` §13). The ordinary `natives` entry for
    /// each of these names is an adapter that drives the same implementation
    /// through [`Interp::call_value`], so the recursive engine is unchanged;
    /// the iterative machine drives the resumable form directly so a closure
    /// callback runs as machine work with no nested host stack.
    resumable_natives: HashMap<String, ResumableNative>,

    /// Top-level functions, by name, as an ordered overload set
    /// (`LANGUAGE_SPEC.md` §15.7).
    functions: HashMap<String, Vec<Rc<Closure>>>,
    /// Methods, keyed by `(struct name, method name)`, each an ordered
    /// overload set. Keyed by the nominal struct so method names never collide
    /// across types (`LANGUAGE_SPEC.md` §17.6).
    methods: HashMap<(String, String), Vec<Rc<Closure>>>,
    structs: HashMap<String, Vec<String>>,
    variants: HashMap<String, (String, usize)>,
    depth: usize,
    /// Current expression nesting depth, guarding against host stack
    /// exhaustion from deeply nested (but syntactically flat) programs.
    ast_depth: usize,
    /// A thrown value in flight across a call boundary; consumed by `try`.
    pending_throw: Option<Value>,
    closure_sources: HashMap<usize, SourceId>,
    current_source: Option<SourceId>,
    last_error_source: Option<SourceId>,
    /// The host capability boundary: standard output, standard input, program
    /// arguments, and optional filesystem/clock/sleep capabilities. Every
    /// language-visible interaction with the outside world goes through this;
    /// the standard library never touches the operating system directly.
    host: Box<dyn crate::host::Host>,
}

type Native = Rc<dyn Fn(&mut Interp, Vec<Value>, Span) -> Result<Value>>;

/// One step of a *resumable* native (`ITERATIVE_EVALUATOR_DESIGN.md` §13).
///
/// A native that needs to call back into Aura (today only `map`, `filter`, and
/// `reduce`) returns [`NativeOutcome::InvokeCallback`] instead of blocking on
/// a nested evaluator call. The iterative machine turns that request into
/// machine work (a user frame for a `Closure` callback, an inline invocation
/// for a `Native` one) and feeds the callback's result back into the same
/// `resume` token. The recursive engine drives the identical protocol through
/// [`Interp::drive_resumable`], so both engines share one implementation and
/// one callback order, and a machine-driven native never re-enters the
/// recursive AST evaluator.
pub(crate) enum NativeOutcome {
    /// The native finished with this value.
    Done(Value),
    /// A callback call is required before the native can continue.
    InvokeCallback {
        /// The callable value (`Closure` or `Native`).
        f: Value,
        /// The callback's arguments, already evaluated.
        args: Vec<Value>,
        /// Native-owned continuation state; consumed by `resume`.
        resume: Box<dyn NativeResume>,
    },
}

/// Native-owned resumable state (`ITERATIVE_EVALUATOR_DESIGN.md` §13).
pub(crate) trait NativeResume {
    /// Consume the callback's result and produce the native's next step.
    ///
    /// # Errors
    /// Propagates the callback's diagnostic or a native-owned diagnostic.
    fn resume(self: Box<Self>, it: &mut Interp, result: Result<Value>) -> Result<NativeOutcome>;
}

/// A native that can suspend on a callback (private registration path).
type ResumableNative = Rc<dyn Fn(&mut Interp, Vec<Value>, Span) -> Result<NativeOutcome>>;

impl Interp {
    /// Drive a resumable native to completion with the recursive call path.
    ///
    /// This is the recursive engine's adapter for the callback protocol
    /// (`ITERATIVE_EVALUATOR_DESIGN.md` §13): each `InvokeCallback` is executed
    /// through [`Interp::call_value`], preserving the recursive frame
    /// accounting (`Closure` spends a frame, `Native` does not). The iterative
    /// machine drives the same `NativeOutcome` states directly as machine
    /// work instead of calling this helper.
    pub(crate) fn drive_resumable(
        &mut self,
        mut step: Result<NativeOutcome>,
        span: Span,
    ) -> Result<Value> {
        loop {
            match step? {
                NativeOutcome::Done(v) => return Ok(v),
                NativeOutcome::InvokeCallback { f, args, resume } => {
                    let result = self.call_value(f, args, span);
                    step = resume.resume(self, result);
                }
            }
        }
    }
}

impl Interp {
    /// Create an interpreter with the standard library installed.
    #[must_use]
    pub fn new() -> Interp {
        let mut it = Interp {
            globals: Env::root(),
            natives: HashMap::new(),
            resumable_natives: HashMap::new(),
            functions: HashMap::new(),
            methods: HashMap::new(),
            structs: HashMap::new(),
            variants: HashMap::new(),
            depth: 0,
            ast_depth: 0,
            pending_throw: None,
            closure_sources: HashMap::new(),
            current_source: None,
            last_error_source: None,
            host: crate::host::default_host(),
        };
        crate::stdlib::install(&mut it);
        it
    }

    /// Create an interpreter that discards standard output.
    ///
    /// On native this keeps the real filesystem, clock, and sleep
    /// capabilities and only silences `print`; on WebAssembly the host is
    /// capability-limited.
    #[must_use]
    pub fn silent() -> Interp {
        let mut it = Interp::new();
        it.host = crate::host::silent_host();
        it
    }

    /// Create an interpreter with an explicit host.
    #[must_use]
    pub fn with_host(host: Box<dyn crate::host::Host>) -> Interp {
        let mut it = Interp::new();
        it.host = host;
        it
    }

    /// Replace the host capability boundary.
    pub fn set_host(&mut self, host: Box<dyn crate::host::Host>) {
        self.host = host;
    }

    /// The host capability boundary.
    #[must_use]
    pub fn host(&self) -> &dyn crate::host::Host {
        self.host.as_ref()
    }

    /// The host capability boundary, mutably.
    pub fn host_mut(&mut self) -> &mut dyn crate::host::Host {
        self.host.as_mut()
    }

    /// Register a native function.
    pub fn native<F>(&mut self, name: &str, f: F)
    where
        F: Fn(&mut Interp, Vec<Value>, Span) -> Result<Value> + 'static,
    {
        self.natives.insert(name.to_string(), Rc::new(f));
    }

    /// Register a resumable native and its recursive-engine adapter.
    ///
    /// The adapter drives the same implementation with
    /// [`Interp::call_value`], so the recursive engine's observable behavior
    /// (including the callback frame accounting of a `Closure` versus a
    /// `Native` callback) is exactly the pre-protocol behavior. The iterative
    /// machine looks the name up in `resumable_natives` instead and schedules
    /// each [`NativeOutcome::InvokeCallback`] as machine work.
    pub(crate) fn native_resumable(
        &mut self,
        name: &str,
        f: impl Fn(&mut Interp, Vec<Value>, Span) -> Result<NativeOutcome> + 'static,
    ) {
        let f: ResumableNative = Rc::new(f);
        let adapter = f.clone();
        self.natives.insert(
            name.to_string(),
            Rc::new(move |it: &mut Interp, args: Vec<Value>, span: Span| {
                let step = adapter(it, args, span);
                it.drive_resumable(step, span)
            }),
        );
        self.resumable_natives.insert(name.to_string(), f);
    }

    /// Register a global value.
    pub fn global(&mut self, name: &str, v: Value) {
        self.globals.define(name.to_string(), v, false);
    }

    /// Configure the execution context: program arguments and an optional
    /// standard-input source.
    ///
    /// The output sink is the substrate default (process stdout on native),
    /// matching the library entry points that use this. Embedders that need a
    /// custom sink should build a host and call [`Interp::set_host`].
    pub fn set_context(
        &mut self,
        args: Vec<String>,
        input: Option<Box<dyn std::io::BufRead + Send>>,
    ) {
        self.host = crate::host::default_host_from_parts(args, input);
    }

    /// The program arguments exposed to `args()`.
    ///
    /// The former `program_args(&self) -> &[String]` is gone: arguments are
    /// now owned by the host, which reports them as owned data. Use
    /// [`Interp::host`]`.args()` for direct access; the `args()` builtin is the
    /// language-visible path.
    #[must_use]
    pub fn program_args(&self) -> Vec<String> {
        self.host.args()
    }

    /// Read one line from the host, returning `None` at end of input.
    ///
    /// The host is responsible for terminator normalization (a trailing `\n`
    /// and a preceding `\r` are removed). A read failure is `E4020`.
    ///
    /// # Errors
    /// Returns the host failure as a diagnostic.
    pub fn read_input_line(&mut self) -> Result<Option<String>> {
        self.host
            .read_line()
            .map_err(|e| e.into_diag(Span::default()))
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
                    let ctl = self.eval_toplevel(value, &globals)?;
                    let v = self.finish_global(ctl)?;
                    self.globals.define(name.clone(), v, false);
                }
                Item::Expr(e, _) => {
                    let globals = self.globals.clone();
                    let ctl = self.eval_toplevel(e, &globals)?;
                    self.finish_global(ctl)?;
                }
                _ => {}
            }
        }
        if let Some(main) = self.functions.get("main").and_then(|s| s.first()).cloned() {
            if let Err(d) = self.call(&main, Vec::new(), Span::default()) {
                return Err(self.uncaught(d));
            }
        }
        Ok(())
    }

    /// Execute a module with the experimental explicit-continuation evaluator
    /// (B-1R3A).
    ///
    /// Behavior-neutral for production: this method exists only with the
    /// non-default `evaluator-oracle` feature and is never called by the
    /// recursive engine or any public entry point. It mirrors [`Interp::run`]'s
    /// declaration and initialization order, and routes the entry `main` call
    /// through the machine. It is deliberately **not** an embedder API.
    ///
    /// Supported subset and explicit unsupported surface: see
    /// `src/run/iterative.rs`. Unsupported constructs return `E4999`; they never
    /// fall back to recursion.
    ///
    /// # Errors
    /// Returns the first diagnostic the iterative machine produces.
    #[cfg(feature = "evaluator-oracle")]
    pub fn run_iterative(&mut self, module: &Module) -> Result<()> {
        // Pass 1: declarations (recursive helper; declaration is not
        // expression evaluation and carries no pending execution state).
        for item in &module.items {
            self.declare_item(item);
        }
        // Pass 2: top-level constants and expressions in source order, run
        // through the machine so an unsupported construct cannot silently use
        // the recursive path.
        for item in &module.items {
            match item {
                Item::Const { name, value, .. } => {
                    let globals = self.globals.clone();
                    let ctl = self.iterative_eval(value, &globals)?;
                    let v = self.finish_global(ctl)?;
                    self.globals.define(name.clone(), v, false);
                }
                Item::Expr(e, _) => {
                    let globals = self.globals.clone();
                    let ctl = self.iterative_eval(e, &globals)?;
                    self.finish_global(ctl)?;
                }
                _ => {}
            }
        }
        if let Some(main) = self.functions.get("main").and_then(|s| s.first()).cloned() {
            if let Err(d) = iterative::call_closure_body(self, main, Vec::new(), Span::default()) {
                return Err(self.uncaught(d));
            }
        }
        Ok(())
    }

    /// Execute a resolved module with the explicit-continuation machine,
    /// retaining per-item source provenance (the sourced counterpart of
    /// [`Interp::run_iterative`]). The cutover path uses this so multi-source
    /// provider compilations are attribute-compatible with the recursive
    /// `run_sourced`.
    pub(crate) fn run_iterative_sourced(
        &mut self,
        module: &Module,
        item_sources: &[SourceId],
        entry_source: SourceId,
    ) -> std::result::Result<(), SourceDiagnostic> {
        if module.items.len() != item_sources.len() {
            return Err(SourceDiagnostic::locationless(Diag::locationless(
                codes::INTERNAL,
                "runtime source provenance does not match the resolved module",
            )));
        }

        self.last_error_source = None;
        for (item, source) in module.items.iter().zip(item_sources.iter().copied()) {
            self.declare_item_with_source(item, Some(source));
        }

        for (item, source) in module.items.iter().zip(item_sources.iter().copied()) {
            self.current_source = Some(source);
            self.last_error_source = None;
            let result = match item {
                Item::Const { name, value, .. } => {
                    let globals = self.globals.clone();
                    self.iterative_eval(value, &globals).and_then(|ctl| {
                        let value = self.finish_global(ctl)?;
                        self.globals.define(name.clone(), value, false);
                        Ok(())
                    })
                }
                Item::Expr(expr, _) => {
                    let globals = self.globals.clone();
                    self.iterative_eval(expr, &globals)
                        .and_then(|ctl| self.finish_global(ctl).map(|_| ()))
                }
                _ => Ok(()),
            };
            if let Err(diagnostic) = result {
                let owner = self.last_error_source.take().unwrap_or(source);
                return Err(SourceDiagnostic::new(diagnostic, owner));
            }
        }

        if let Some(main) = self.functions.get("main").and_then(|s| s.first()).cloned() {
            self.current_source = Some(entry_source);
            self.last_error_source = None;
            if let Err(diagnostic) =
                iterative::call_closure_body(self, main, Vec::new(), Span::default())
            {
                let owner = self.last_error_source.take().unwrap_or(entry_source);
                return Err(SourceDiagnostic::new(self.uncaught(diagnostic), owner));
            }
        }
        Ok(())
    }

    /// Evaluate one expression with the iterative machine (feature-gated),
    /// normalizing an internal throw exactly like `eval_toplevel`.
    #[cfg(feature = "evaluator-oracle")]
    pub(crate) fn iterative_eval(&mut self, e: &Expr, env: &Env) -> Result<Ctl> {
        match iterative::eval_expr(self, e, env) {
            Ok(c) => Ok(c),
            Err(d) => Err(self.uncaught(d)),
        }
    }

    /// Evaluate an expression in the global scope with the iterative machine.
    ///
    /// Hidden and feature-gated: used only by the differential oracle's value
    /// path (the REPL-equivalent final-value observable). It mirrors
    /// [`Interp::eval_globals`] including uncaught-throw normalization.
    ///
    /// # Errors
    /// Returns the iterative machine's first diagnostic.
    #[cfg(feature = "evaluator-oracle")]
    #[doc(hidden)]
    pub fn eval_globals_iterative(&mut self, e: &Expr) -> Result<Ctl> {
        let globals = self.globals.clone();
        self.iterative_eval(e, &globals)
    }

    pub(crate) fn run_sourced(
        &mut self,
        module: &Module,
        item_sources: &[SourceId],
        entry_source: SourceId,
    ) -> std::result::Result<(), SourceDiagnostic> {
        if module.items.len() != item_sources.len() {
            return Err(SourceDiagnostic::locationless(Diag::locationless(
                codes::INTERNAL,
                "runtime source provenance does not match the resolved module",
            )));
        }

        self.last_error_source = None;
        for (item, source) in module.items.iter().zip(item_sources.iter().copied()) {
            self.declare_item_with_source(item, Some(source));
        }

        for (item, source) in module.items.iter().zip(item_sources.iter().copied()) {
            self.current_source = Some(source);
            self.last_error_source = None;
            let result = match item {
                Item::Const { name, value, .. } => {
                    let globals = self.globals.clone();
                    self.eval_toplevel(value, &globals).and_then(|ctl| {
                        let value = self.finish_global(ctl)?;
                        self.globals.define(name.clone(), value, false);
                        Ok(())
                    })
                }
                Item::Expr(expr, _) => {
                    let globals = self.globals.clone();
                    self.eval_toplevel(expr, &globals)
                        .and_then(|ctl| self.finish_global(ctl).map(|_| ()))
                }
                _ => Ok(()),
            };
            if let Err(diagnostic) = result {
                let owner = self.last_error_source.take().unwrap_or(source);
                return Err(SourceDiagnostic::new(diagnostic, owner));
            }
        }

        if let Some(main) = self
            .functions
            .get("main")
            .and_then(|set| set.first())
            .cloned()
        {
            self.current_source = Some(entry_source);
            self.last_error_source = None;
            if let Err(diagnostic) = self.call(&main, Vec::new(), Span::default()) {
                let owner = self.last_error_source.take().unwrap_or(entry_source);
                return Err(SourceDiagnostic::new(self.uncaught(diagnostic), owner));
            }
        }
        Ok(())
    }

    /// Register one declaration into the interpreter's symbol tables.
    fn declare_item(&mut self, item: &Item) {
        self.declare_item_with_source(item, None);
    }

    fn declare_item_with_source(&mut self, item: &Item, source: Option<SourceId>) {
        match item {
            Item::Fn {
                name,
                type_params,
                params,
                body,
                ..
            } => {
                let tparams: Vec<String> = type_params.iter().map(|p| p.name.clone()).collect();
                let closure = Rc::new(Closure {
                    name: name.clone(),
                    params: params.iter().map(|p| (p.name.clone(), p.mutable)).collect(),
                    param_tys: params
                        .iter()
                        .map(|p| {
                            p.ty.as_ref()
                                .map(|t| crate::types::Ty::from_expr_erased(t, &tparams))
                        })
                        .collect(),
                    body: body.clone(),
                    env: self.globals.clone(),
                });
                if let Some(source) = source {
                    self.closure_sources
                        .insert(Rc::as_ptr(&closure) as usize, source);
                }
                self.functions
                    .entry(name.clone())
                    .or_default()
                    .push(closure);
            }
            Item::Impl {
                target,
                type_params,
                methods,
                ..
            } => {
                // Methods are stored under `Struct.method`, keyed by the
                // nominal type, so two structs may share a method name. Each
                // method is an ordinary closure whose first parameter is the
                // receiver (`LANGUAGE_SPEC.md` §17.6). Generic parameters are
                // erased for overload selection, exactly as for a function.
                let impl_tparams: Vec<String> =
                    type_params.iter().map(|p| p.name.clone()).collect();
                for m in methods {
                    if let Item::Fn {
                        name,
                        type_params: mtype_params,
                        params,
                        body,
                        ..
                    } = m
                    {
                        let mut tparams = impl_tparams.clone();
                        tparams.extend(mtype_params.iter().map(|p| p.name.clone()));
                        let closure = Rc::new(Closure {
                            name: format!("{target}.{name}"),
                            params: params.iter().map(|p| (p.name.clone(), p.mutable)).collect(),
                            param_tys: params
                                .iter()
                                .map(|p| {
                                    p.ty.as_ref()
                                        .map(|t| crate::types::Ty::from_expr_erased(t, &tparams))
                                })
                                .collect(),
                            body: body.clone(),
                            env: self.globals.clone(),
                        });
                        if let Some(source) = source {
                            self.closure_sources
                                .insert(Rc::as_ptr(&closure) as usize, source);
                        }
                        self.methods
                            .entry((target.clone(), name.clone()))
                            .or_default()
                            .push(closure);
                    }
                }
            }
            Item::Struct { name, fields, .. } => {
                self.structs.insert(
                    name.clone(),
                    fields.iter().map(|f| f.name.clone()).collect(),
                );
            }
            Item::Enum { name, variants, .. } => {
                for v in variants {
                    self.variants
                        .insert(v.tag.clone(), (name.clone(), v.payload.len()));
                }
            }
            _ => {}
        }
    }

    fn error(&self, code: u16, msg: impl Into<String>, span: Span) -> Diag {
        Diag::new(code, msg, span)
    }

    /// Convert an uncaught internal `THROWN` signal into the user-facing
    /// uncaught-throw diagnostic (`E4026`), preserving the thrown value's
    /// display. `E4099` is an internal call-boundary signal and MUST NOT reach
    /// the user (`LANGUAGE_SPEC.md` §34.3); every non-`try` exit path funnels
    /// through here. Any other diagnostic is returned unchanged.
    fn uncaught(&mut self, d: Diag) -> Diag {
        if d.code == codes::THROWN {
            let shown = self
                .pending_throw
                .take()
                .map_or_else(|| "uncaught value".to_string(), |v| v.display());
            self.error(codes::FOREIGN, format!("uncaught value: {shown}"), d.span)
        } else {
            d
        }
    }

    /// Finish a top-level statement or expression: map a residual control-flow
    /// signal (`return`/`break`/`continue`) or an explicit `throw` to its
    /// user-visible diagnostic, and normalize an internal throw signal. Used
    /// by the top-level module pass and the REPL, so those surfaces agree with
    /// `aura run` (`LANGUAGE_SPEC.md` §28.5).
    pub fn finish_global(&mut self, c: Ctl) -> Result<Value> {
        match c.value(self, Span::default()) {
            Ok(v) => Ok(v),
            Err(d) => Err(self.uncaught(d)),
        }
    }

    /// Normalize an internal throw signal into the user-facing `E4026`
    /// diagnostic. Used by the REPL so a runtime error reports the same code
    /// as `aura run` (`LANGUAGE_SPEC.md` §28.5).
    pub fn uncaught_diag(&mut self, d: Diag) -> Diag {
        self.uncaught(d)
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
        for ((p, mutable), v) in closure.params.iter().zip(args) {
            env.define(p.clone(), v, *mutable);
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
        let saved_source = self.current_source;
        if let Some(source) = self
            .closure_sources
            .get(&(Rc::as_ptr(closure) as usize))
            .copied()
        {
            self.current_source = Some(source);
        }
        let r = self.exec_block(&closure.body, &env, false);
        if r.is_err() && self.last_error_source.is_none() {
            self.last_error_source = self.current_source;
        }
        self.current_source = saved_source;
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
        let mut local = if scoped { env.child() } else { env.clone() };
        let mut last = Value::None;
        for s in body {
            // A shadowing `let` advances `local` to a new frame so later
            // statements resolve the new binding while existing closures keep
            // the old one (`LANGUAGE_SPEC.md` §16.3).
            match self.exec_stmt(s, &mut local)? {
                Ctl::Val(v) => last = v,
                other => return Ok(other),
            }
        }
        Ok(Ctl::Val(last))
    }

    fn exec_stmt(&mut self, s: &Stmt, env: &mut Env) -> Result<Ctl> {
        match s {
            Stmt::Let {
                name,
                value,
                mutable,
                ..
            } => {
                // A `return`/`throw`/`break`/`continue` raised while computing
                // the initializer propagates out of the statement unchanged
                // (§14.4); it is not converted to a value-position error here.
                let v = match self.eval(value, env)? {
                    Ctl::Val(v) => v,
                    other => return Ok(other),
                };
                // An ordinary `let`/`let mut` binding shadows any existing
                // binding of the same name in this scope by installing a new
                // binding identity; later statements in the block resolve the
                // new one (`LANGUAGE_SPEC.md` §16.3).
                *env = env.define_shadowing(name.clone(), v, *mutable);
                Ok(Ctl::Val(Value::None))
            }
            Stmt::LetPattern { pattern, value, .. } => {
                // Destructuring `let` is atomic (§4.7): match into a temporary
                // child scope so a mismatch never leaves a partial binding in
                // the real environment, then transfer every bound name on
                // success.
                let v = match self.eval(value, env)? {
                    Ctl::Val(v) => v,
                    other => return Ok(other),
                };
                let tmp = env.child();
                self.bind_pattern(pattern, &v, &tmp)?;
                for name in pattern.bindings() {
                    if let Some(bound) = tmp.get(&name) {
                        // A destructuring `let` shadows like a simple one: a
                        // new binding identity for each name.
                        *env = env.define_shadowing(name, bound, false);
                    }
                }
                Ok(Ctl::Val(Value::None))
            }
            Stmt::Assign {
                target,
                value,
                op,
                span,
            } => {
                // A control-flow signal raised while computing the RHS or the
                // target's subexpressions propagates out of the statement
                // unchanged (§13, §14.4); it is never reinterpreted as a
                // value-position error or an uncaught throw here.
                let rhs = match self.eval(value, env)? {
                    Ctl::Val(v) => v,
                    other => return Ok(other),
                };
                let new = match op {
                    None => rhs,
                    Some(op) => {
                        let cur = match self.read_target(target, env)? {
                            Ctl::Val(v) => v,
                            other => return Ok(other),
                        };
                        self.binary(*op, cur, rhs, *span)?
                    }
                };
                match self.write_target(target, new, env, *span)? {
                    Ctl::Val(_) => Ok(Ctl::Val(Value::None)),
                    other => Ok(other),
                }
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
                let c = match self.eval(cond, env)? {
                    Ctl::Val(v) => v,
                    other => return Ok(other),
                };
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
                let subject = match self.eval(iter, env)? {
                    Ctl::Val(v) => v,
                    other => return Ok(other),
                };
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

    fn read_target(&mut self, target: &Expr, env: &Env) -> Result<Ctl> {
        match target {
            Expr::Name(name, span) => env.get(name).map(Ctl::Val).ok_or_else(|| {
                self.error(
                    codes::UNDEFINED,
                    format!("undefined variable `{name}`"),
                    *span,
                )
            }),
            Expr::Index(b, i, span) => {
                let base = match self.eval(b, env)? {
                    Ctl::Val(v) => v,
                    other => return Ok(other),
                };
                let idx = match self.eval(i, env)? {
                    Ctl::Val(v) => v,
                    other => return Ok(other),
                };
                Ok(Ctl::Val(self.index_get(&base, &idx, *span)?))
            }
            Expr::Field(base, name, span) => {
                let subject = match self.eval(base, env)? {
                    Ctl::Val(v) => v,
                    other => return Ok(other),
                };
                Ok(Ctl::Val(self.field_get(&subject, name, *span)?))
            }
            other => Err(self.error(
                codes::INVALID_ASSIGN,
                "invalid assignment target",
                span_of(other),
            )),
        }
    }

    fn write_target(&mut self, target: &Expr, value: Value, env: &Env, span: Span) -> Result<Ctl> {
        match target {
            Expr::Name(name, nspan) => env
                .assign(name, value)
                .map(|()| Ctl::Val(Value::None))
                .map_err(|e| match e {
                    AssignError::Immutable => self.error(
                        codes::ASSIGN_IMMUTABLE,
                        format!(
                            "cannot assign to `{name}`: it is immutable (declare it `let mut`)"
                        ),
                        *nspan,
                    ),
                    AssignError::Undefined => self.error(
                        codes::UNDEFINED,
                        format!("undefined variable `{name}`"),
                        *nspan,
                    ),
                }),
            Expr::Index(b, i, ispan) => {
                let base = match self.eval(b, env)? {
                    Ctl::Val(v) => v,
                    other => return Ok(other),
                };
                let idx = match self.eval(i, env)? {
                    Ctl::Val(v) => v,
                    other => return Ok(other),
                };
                self.index_set(&base, &idx, value, *ispan)?;
                Ok(Ctl::Val(Value::None))
            }
            Expr::Field(base, name, fspan) => {
                let subject = match self.eval(base, env)? {
                    Ctl::Val(v) => v,
                    other => return Ok(other),
                };
                self.field_set(&subject, name, value, *fspan)?;
                Ok(Ctl::Val(Value::None))
            }
            _ => Err(self.error(codes::INVALID_ASSIGN, "invalid assignment target", span)),
        }
    }

    // ------------------------------------------------------------ values

    fn iterate(&mut self, v: &Value, span: Span) -> Result<Vec<Value>> {
        match v {
            Value::List(l) => Ok(l.borrow().clone()),
            Value::Str(s) => Ok(s.chars().map(|c| Value::str(c.to_string())).collect()),
            Value::Map(m) => Ok(m.borrow().keys().map(MapKey::to_value).collect()),
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
            (Value::Map(m), key) => match MapKey::from_value(key) {
                Some(k) => m.borrow().get(&k).cloned().ok_or_else(|| {
                    self.error(
                        codes::UNDEFINED,
                        format!("map has no key {}", k.repr()),
                        span,
                    )
                }),
                None => Err(self.error(
                    codes::TYPE_MISMATCH,
                    format!("type `{}` cannot be used as a map key", key.type_name()),
                    span,
                )),
            },
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
            (Value::Map(m), key) => match MapKey::from_value(key) {
                Some(k) => {
                    m.borrow_mut().insert(k, value);
                    Ok(())
                }
                None => Err(self.error(
                    codes::TYPE_MISMATCH,
                    format!("type `{}` cannot be used as a map key", key.type_name()),
                    span,
                )),
            },
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
            Pattern::Bind(n, _) => {
                if n != "_" {
                    env.define(n.clone(), value.clone(), false);
                }
                Ok(())
            }
            Pattern::List(ps, span) => match value {
                // `span` is the list pattern's span, used below.
                Value::List(l) => {
                    let l = l.borrow();
                    if l.len() != ps.len() {
                        return Err(self.error(
                            codes::TYPE_MISMATCH,
                            "list pattern arity mismatch",
                            *span,
                        ));
                    }
                    for (p, v) in ps.iter().zip(l.iter()) {
                        self.bind_pattern(p, v, env)?;
                    }
                    Ok(())
                }
                _ => Err(self.error(codes::TYPE_MISMATCH, "pattern expects a list", *span)),
            },
            Pattern::Variant(tag, ps, span) => match value {
                // `span` is the variant pattern's span, used below.
                Value::Variant(v) => {
                    if &v.tag != tag || v.payload.len() != ps.len() {
                        return Err(self.error(
                            codes::TYPE_MISMATCH,
                            "variant pattern mismatch",
                            *span,
                        ));
                    }
                    for (p, val) in ps.iter().zip(v.payload.iter()) {
                        self.bind_pattern(p, val, env)?;
                    }
                    Ok(())
                }
                _ => Err(self.error(codes::TYPE_MISMATCH, "pattern expects a variant", *span)),
            },
            // A literal pattern binds nothing but MUST match (§19.3). `let`
            // rejects literal patterns at parse time, so this arm is reached
            // only through `for`/`match`; validating here keeps a literal
            // pattern in `for` assertive rather than a silent no-op.
            Pattern::Int(..) | Pattern::Str(..) | Pattern::Bool(..) | Pattern::None(_) => {
                if self.match_pattern(pat, value) {
                    Ok(())
                } else {
                    Err(self.error(
                        codes::TYPE_MISMATCH,
                        "literal pattern does not match the value",
                        pat.span(),
                    ))
                }
            }
        }
    }

    fn match_pattern(&self, pat: &Pattern, value: &Value) -> bool {
        match pat {
            Pattern::Bind(..) => true,
            Pattern::Int(i, _) => matches!(value, Value::Int(n) if n == i),
            Pattern::Str(s, _) => matches!(value, Value::Str(n) if &**n == s),
            Pattern::Bool(b, _) => matches!(value, Value::Bool(n) if n == b),
            Pattern::None(_) => matches!(value, Value::None),
            Pattern::List(ps, _) => match value {
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
            Pattern::Variant(tag, ps, _) => match value {
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
                } else if let Some(closure) =
                    self.functions.get(name).and_then(|s| s.first()).cloned()
                {
                    // A named function used as a value. A bare function
                    // reference does not carry argument types, so only a
                    // single-overload function can be used this way; a
                    // first-class reference to an overloaded name is deferred
                    // (`LANGUAGE_SPEC.md` §15.7).
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
                for p in parts.iter() {
                    match p {
                        FPart::Lit(t) => out.push_str(t),
                        FPart::Expr(e, spec) => {
                            let v = val!(self.eval(e, env));
                            match spec {
                                None => out.push_str(&v.display()),
                                Some(s) => out.push_str(&self.format_value(&v, s)?),
                            }
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
                    UnOp::BitNot => match v {
                        Value::Int(i) => Ok(Ctl::Val(Value::Int(!i))),
                        other => Err(self.error(
                            codes::TYPE_MISMATCH,
                            format!(
                                "operator `~` requires an integer, found {}",
                                other.type_name()
                            ),
                            *span,
                        )),
                    },
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
            Expr::Call(callee, args, _ty_args, span) => self.eval_call(callee, args, env, *span),
            Expr::Method(recv, name, args, _ty_args, span) => {
                let subject = val!(self.eval(recv, env));
                let mut vals = Vec::with_capacity(args.len());
                for a in args.iter() {
                    vals.push(val!(self.eval(&a.value, env)));
                }
                // A struct receiver resolves against its nominal method table
                // only (§17.6); any other receiver uses the built-in registry.
                if let Value::Instance(i) = &subject {
                    let key = (i.ty.clone(), name.clone());
                    if let Some(set) = self.methods.get(&key).cloned() {
                        // Resolve the method overload by the non-receiver
                        // argument values (`LANGUAGE_SPEC.md` §17.7).
                        let c = self.select_method_overload(&i.ty, name, &set, &vals, *span)?;
                        let mut full = Vec::with_capacity(vals.len() + 1);
                        full.push(subject.clone());
                        full.extend(vals);
                        return Ok(Ctl::Val(self.call(&c, full, *span)?));
                    }
                    return Err(self.error(
                        codes::UNDEFINED,
                        format!("struct {} has no method `{name}`", i.ty),
                        *span,
                    ));
                }
                Ok(Ctl::Val(self.method(&subject, name, vals, *span)?))
            }
            Expr::Field(recv, name, span) => {
                let subject = val!(self.eval(recv, env));
                match &subject {
                    Value::Instance(i) => {
                        // A method is not a bound value: `s.m` names only a
                        // field, and a missing field is `E2003` (§17.6).
                        if self.methods.contains_key(&(i.ty.clone(), name.clone()))
                            && !i.fields.borrow().iter().any(|(k, _)| k == name)
                        {
                            return Err(self.error(
                                codes::UNDEFINED,
                                format!(
                                    "struct {} has method `{name}`; call it as `{name}(...)`",
                                    i.ty
                                ),
                                *span,
                            ));
                        }
                        Ok(Ctl::Val(self.field_get(&subject, name, *span)?))
                    }
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
                for i in items.iter() {
                    out.push(val!(self.eval(i, env)));
                }
                Ok(Ctl::Val(Value::list(out)))
            }
            Expr::Map(entries, _) => {
                let mut map = std::collections::BTreeMap::new();
                for (k, v) in entries.iter() {
                    let kv = val!(self.eval(k, env));
                    let Some(key) = MapKey::from_value(&kv) else {
                        return Err(self.error(
                            codes::TYPE_MISMATCH,
                            format!(
                                "type `{}` cannot be used as a map key; map keys must be `string`, `int`, or `bool`",
                                kv.type_name()
                            ),
                            span_of(k),
                        ));
                    };
                    let vv = val!(self.eval(v, env));
                    map.insert(key, vv);
                }
                Ok(Ctl::Val(Value::Map(Rc::new(RefCell::new(map)))))
            }
            Expr::ListComp {
                value,
                pattern,
                iterable,
                filter,
                span,
            } => {
                // The iterable is evaluated exactly once (§24); each iteration
                // gets a fresh scope, and the filter and value see the pattern
                // bindings. Errors and `throw`s propagate normally.
                let subject = val!(self.eval(iterable, env));
                let mut out = Vec::new();
                for item in self.iterate(&subject, *span)? {
                    let scope = env.child();
                    self.bind_pattern(pattern, &item, &scope)?;
                    if let Some(f) = filter {
                        if !val!(self.eval(f, &scope)).truthy() {
                            continue;
                        }
                    }
                    out.push(val!(self.eval(value, &scope)));
                }
                Ok(Ctl::Val(Value::list(out)))
            }
            Expr::MapComp {
                key,
                value,
                pattern,
                iterable,
                filter,
                span,
            } => {
                let subject = val!(self.eval(iterable, env));
                let mut map = std::collections::BTreeMap::new();
                for item in self.iterate(&subject, *span)? {
                    let scope = env.child();
                    self.bind_pattern(pattern, &item, &scope)?;
                    if let Some(f) = filter {
                        if !val!(self.eval(f, &scope)).truthy() {
                            continue;
                        }
                    }
                    // Key before value, ordinary map-key admissibility, and
                    // ordinary duplicate-key behavior (§25).
                    let kv = val!(self.eval(key, &scope));
                    let Some(mk) = MapKey::from_value(&kv) else {
                        return Err(self.error(
                            codes::TYPE_MISMATCH,
                            format!(
                                "type `{}` cannot be used as a map key; map keys must be `string`, `int`, or `bool`",
                                kv.type_name()
                            ),
                            span_of(key),
                        ));
                    };
                    let vv = val!(self.eval(value, &scope));
                    map.insert(mk, vv);
                }
                Ok(Ctl::Val(Value::Map(Rc::new(RefCell::new(map)))))
            }
            Expr::Construct(name, args, _ty_args, span) => self.construct(name, args, env, *span),
            Expr::Tuple(items, _) => {
                let mut out = Vec::with_capacity(items.len());
                for i in items.iter() {
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
                    expr => Arc::from([Stmt::Return(Some(expr.clone()), Span::default())]),
                };
                let closure = Rc::new(Closure {
                    name: "<lambda>".to_string(),
                    params: params.iter().map(|p| (p.name.clone(), p.mutable)).collect(),
                    param_tys: params
                        .iter()
                        .map(|p| p.ty.as_ref().map(crate::types::Ty::from_expr_lenient))
                        .collect(),
                    body: body_stmts,
                    env: env.clone(),
                });
                if let Some(source) = self.current_source {
                    self.closure_sources
                        .insert(Rc::as_ptr(&closure) as usize, source);
                }
                Ok(Ctl::Val(Value::Closure(closure)))
            }
            Expr::Pipe(l, r, span) => {
                let arg = val!(self.eval(l, env));
                let f = val!(self.eval(r, env));
                Ok(Ctl::Val(self.call_value(f, vec![arg], *span)?))
            }
            // `a..b` is the same Range value `range(a, b)` builds: a lazy,
            // half-open, start-inclusive range. Bounds must be ints; there is
            // no implicit coercion (`LANGUAGE_SPEC.md` §22).
            Expr::Range(start, end, span) => {
                let s = val!(self.eval(start, env));
                let e = val!(self.eval(end, env));
                let s = match s {
                    Value::Int(i) => i,
                    other => {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!("range start expects an int, found {}", other.type_name()),
                            *span,
                        ))
                    }
                };
                let e = match e {
                    Value::Int(i) => i,
                    other => {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!("range end expects an int, found {}", other.type_name()),
                            *span,
                        ))
                    }
                };
                Ok(Ctl::Val(Value::Range(Rc::new(value::RangeVal {
                    start: s,
                    end: e,
                }))))
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
                for arm in arms.iter() {
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

    fn eval_call(&mut self, callee: &Expr, args: &[Arg], env: &Env, span: Span) -> Result<Ctl> {
        // Evaluate every argument expression in **source order**, once, before
        // any parameter binding. Parameter binding (below) must never reorder
        // evaluation (`LANGUAGE_SPEC.md` §13).
        let mut vals = Vec::with_capacity(args.len());
        for a in args {
            match self.eval(&a.value, env)? {
                Ctl::Val(v) => vals.push(v),
                other => return Ok(other),
            }
        }
        match callee {
            Expr::Name(name, nspan) => {
                if let Some(set) = self.functions.get(name).cloned() {
                    // Resolve the overload by argument value types, using the
                    // same selector the checker uses (`crate::types::resolve_overload`)
                    // so the two never disagree. A single candidate is used
                    // directly; the checker has already rejected ambiguous and
                    // unmatched direct calls, so a runtime miss here is a
                    // dynamically-typed path (an `Unknown` receiver/argument).
                    let c = self.select_overload(name, &set, &vals, *nspan)?;
                    // Bind by parameter name: positional arguments fill the
                    // next unfilled parameter; named arguments fill their
                    // parameter. `vals` stays in source order; only the
                    // association changes (§15.7).
                    let call_vals = bind_arguments(args, &vals, &c.params, span)?;
                    return Ok(Ctl::Val(self.call(&c, call_vals, *nspan)?));
                }
                if let Some(n) = self.natives.get(name).cloned() {
                    // Builtins are positional; named arguments were rejected
                    // by the checker. Arity is enforced against the shared
                    // registry here too, so a directly named builtin cannot
                    // bypass it.
                    self.check_native_arity(name, vals.len(), *nspan)?;
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

    /// Select the overload of `name` whose parameter types accept the runtime
    /// argument values, using the same selector the checker uses
    /// (`crate::types::resolve_overload`). A single-overload function (or one
    /// with no typed parameters) is returned directly, so ordinary calls and
    /// dynamically-typed arguments keep their existing permissive behavior.
    fn select_overload(
        &self,
        name: &str,
        set: &[Rc<Closure>],
        args: &[Value],
        span: Span,
    ) -> Result<Rc<Closure>> {
        if set.len() == 1 {
            return Ok(set[0].clone());
        }
        let candidates: Vec<crate::types::OverloadParams> =
            set.iter().map(|c| c.param_tys.clone()).collect();
        let actual: Vec<crate::types::Ty> = args.iter().map(Value::ty).collect();
        match crate::types::resolve_overload(&candidates, &actual) {
            crate::types::OverloadResolution::Selected(i) => Ok(set[i].clone()),
            crate::types::OverloadResolution::Ambiguous(_) => Err(self.error(
                codes::TYPE_MISMATCH,
                format!("call to `{name}` is ambiguous"),
                span,
            )),
            crate::types::OverloadResolution::NoMatch => {
                // Fall back to a single unannotated candidate if one exists;
                // otherwise report no match. The checker rejects statically
                // provable mismatches, so this path is for dynamic values.
                if let Some(c) = set.iter().find(|c| c.param_tys.iter().all(Option::is_none)) {
                    Ok(c.clone())
                } else {
                    Err(self.error(
                        codes::TYPE_MISMATCH,
                        format!("no overload of `{name}` accepts these arguments"),
                        span,
                    ))
                }
            }
        }
    }

    /// The method analogue of [`Interp::select_overload`]. The receiver is
    /// `params[0]`, so candidates are compared on the remaining parameters.
    fn select_method_overload(
        &self,
        struct_name: &str,
        name: &str,
        set: &[Rc<Closure>],
        args: &[Value],
        span: Span,
    ) -> Result<Rc<Closure>> {
        if set.len() == 1 {
            return Ok(set[0].clone());
        }
        let candidates: Vec<crate::types::OverloadParams> = set
            .iter()
            .map(|c| c.param_tys.iter().skip(1).cloned().collect())
            .collect();
        let actual: Vec<crate::types::Ty> = args.iter().map(Value::ty).collect();
        match crate::types::resolve_overload(&candidates, &actual) {
            crate::types::OverloadResolution::Selected(i) => Ok(set[i].clone()),
            crate::types::OverloadResolution::Ambiguous(_) => Err(self.error(
                codes::TYPE_MISMATCH,
                format!("call to `{struct_name}.{name}` is ambiguous"),
                span,
            )),
            crate::types::OverloadResolution::NoMatch => Err(self.error(
                codes::TYPE_MISMATCH,
                format!("no overload of `{struct_name}.{name}` accepts these arguments"),
                span,
            )),
        }
    }

    /// Enforce a builtin's registry arity at runtime. Used on every native
    /// invocation path so that a builtin reached dynamically (a first-class
    /// value or a pipeline) is validated exactly as a direct call is
    /// (`LANGUAGE_SPEC.md` §25). Natives absent from the registry (internal
    /// helpers) impose no constraint.
    fn check_native_arity(&self, name: &str, count: usize, span: Span) -> Result<()> {
        if let Some(sig) = crate::stdlib::signatures::builtin(name) {
            if let Some(message) = sig.check_arity(count) {
                return Err(self.error(codes::TYPE_MISMATCH, message, span));
            }
        }
        Ok(())
    }

    /// Call any callable value.
    pub fn call_value(&mut self, f: Value, args: Vec<Value>, span: Span) -> Result<Value> {
        match &f {
            Value::Closure(c) => self.call(c, args, span),
            Value::Native(name) => {
                // A builtin referenced as a value (for example `let f = len`)
                // bypasses the checker's static builtin-call validation; the
                // registry arity is therefore enforced here, so the runtime is
                // authoritative for every builtin invocation (§25).
                self.check_native_arity(name, args.len(), span)?;
                let n = self.natives.get(name).cloned().ok_or_else(|| {
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
        match self.eval(e, &globals) {
            Ok(c) => Ok(c),
            Err(d) => Err(self.uncaught(d)),
        }
    }

    /// Evaluate a top-level item's expression, normalizing an internal throw
    /// signal into the user-facing diagnostic.
    fn eval_toplevel(&mut self, e: &Expr, env: &Env) -> Result<Ctl> {
        match self.eval(e, env) {
            Ok(c) => Ok(c),
            Err(d) => Err(self.uncaught(d)),
        }
    }

    /// Execute a single statement in the global scope (used by the REPL).
    pub fn exec_stmt_globals(&mut self, s: &Stmt) -> Result<Ctl> {
        let mut globals = self.globals.clone();
        let r = self.exec_stmt(s, &mut globals);
        // A `let` at the REPL top level may have advanced to a shadowing
        // frame; adopt it so the new binding persists for later submissions.
        self.globals = globals;
        r
    }

    /// Register or execute a single top-level item (used by the REPL).
    pub fn run_item(&mut self, item: &Item) -> Result<()> {
        match item {
            Item::Const { name, value, .. } => {
                let globals = self.globals.clone();
                let ctl = self.eval_toplevel(value, &globals)?;
                let v = self.finish_global(ctl)?;
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
        Ok(Ctl::Val(
            self.construct_from_values(name, named, positional, span)?,
        ))
    }

    /// Build a struct instance or enum variant from already-evaluated field
    /// values (`Expr::Construct`'s resolution half).
    ///
    /// Field-argument order, named/positional rules, duplicate/missing field
    /// diagnostics, and variant arity are the single implementation shared by
    /// `Interp::construct` (recursive engine) and the explicit-continuation
    /// machine, which evaluates the arguments as machine work and calls this.
    pub(crate) fn construct_from_values(
        &mut self,
        name: &str,
        named: Vec<(String, Value)>,
        positional: Vec<Value>,
        span: Span,
    ) -> Result<Value> {
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
            return Ok(Value::Instance(Rc::new(Instance {
                ty: name.to_string(),
                fields: RefCell::new(values),
            })));
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
            return Ok(Value::Variant(Rc::new(Variant {
                ty,
                tag: name.to_string(),
                payload: positional,
            })));
        }
        Err(self.error(
            codes::UNKNOWN_TYPE,
            format!("unknown constructor `{name}`"),
            span,
        ))
    }

    fn binary(&mut self, op: BinOp, l: Value, r: Value, span: Span) -> Result<Value> {
        use BinOp::{
            Add, BitAnd, BitOr, Div, Eq, Ge, Gt, Le, Lt, Mul, Ne, Pow, Rem, Shl, Shr, Sub,
        };
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
            BitAnd => self.bitwise(l, r, span, |a, b| a & b, "&"),
            BitOr => self.bitwise(l, r, span, |a, b| a | b, "|"),
            Shl => self.shift(l, r, span, false),
            Shr => self.shift(l, r, span, true),
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

    /// Bitwise AND/OR on two integers.
    fn bitwise(
        &mut self,
        l: Value,
        r: Value,
        span: Span,
        f: fn(i64, i64) -> i64,
        op: &str,
    ) -> Result<Value> {
        match (&l, &r) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(f(*a, *b))),
            _ => Err(self.error(
                codes::TYPE_MISMATCH,
                format!(
                    "operator `{op}` requires two integers, found {} and {}",
                    l.type_name(),
                    r.type_name()
                ),
                span,
            )),
        }
    }

    /// `<<` / `>>` on two integers. A negative or oversized shift count is a
    /// runtime error, not a host panic.
    fn shift(&mut self, l: Value, r: Value, span: Span, right: bool) -> Result<Value> {
        let (Value::Int(a), Value::Int(b)) = (&l, &r) else {
            return Err(self.error(
                codes::TYPE_MISMATCH,
                format!(
                    "operator `{}` requires two integers, found {} and {}",
                    if right { ">>" } else { "<<" },
                    l.type_name(),
                    r.type_name()
                ),
                span,
            ));
        };
        let n = u32::try_from(*b).ok().filter(|n| *n < 64).ok_or_else(|| {
            self.error(
                codes::OVERFLOW,
                format!("shift amount `{b}` is out of range"),
                span,
            )
        })?;
        // Shifts never overflow: bits shifted out are discarded.
        Ok(Value::Int(if right {
            ((*a as u64) >> n) as i64
        } else {
            a.wrapping_shl(n)
        }))
    }

    /// Render a value under an f-string format specification
    /// (`LANGUAGE_SPEC.md` §3.6.4). The mini-language is small: an optional
    /// sign and type coerce or present the value, then width, fill, and
    /// alignment pad it.
    fn format_value(&mut self, v: &Value, spec: &FormatSpec) -> Result<String> {
        use crate::ast::{Align, FormatType, Sign};
        // Bound the precision before it reaches Rust's formatter: a precision
        // above `u16::MAX` is a host panic, and a precision above the language
        // bound is a stable `E4013` on every substrate (`LANGUAGE_SPEC.md`
        // §31.5). The bound also covers a float with no explicit format type.
        if let Some(p) = spec.precision {
            if p > MAX_FORMAT_PRECISION {
                return Err(self.error(
                    codes::OVERFLOW,
                    format!("format precision {p} exceeds the {MAX_FORMAT_PRECISION} limit"),
                    spec.span,
                ));
            }
        }
        // Bound the width before any padding allocation so native and
        // WebAssembly behave identically (no native abort, no wasm trap).
        if let Some(w) = spec.width {
            if w > MAX_FORMAT_WIDTH {
                return Err(self.error(
                    codes::OVERFLOW,
                    format!("format width {w} exceeds the {MAX_FORMAT_WIDTH} limit"),
                    spec.span,
                ));
            }
        }
        // 1. Produce the core text under the presentation type.
        let mut text = match (spec.ty, v) {
            (None, _) => v.display(),
            (Some(FormatType::Dec), Value::Int(i)) => i.to_string(),
            (Some(FormatType::Binary), Value::Int(i)) => format!("{:b}", *i as u64),
            (Some(FormatType::Octal), Value::Int(i)) => format!("{:o}", *i as u64),
            (Some(FormatType::Hex { upper }), Value::Int(i)) => {
                if upper {
                    format!("{:X}", *i as u64)
                } else {
                    format!("{:x}", *i as u64)
                }
            }
            (Some(FormatType::Fixed), Value::Float(f)) => {
                format!("{:.*}", spec.precision.unwrap_or(6), f)
            }
            (Some(FormatType::Fixed), Value::Int(i)) => {
                format!("{:.*}", spec.precision.unwrap_or(6), *i as f64)
            }
            (Some(FormatType::Exp), Value::Float(f)) => {
                format!("{:.*e}", spec.precision.unwrap_or(6), f)
            }
            (Some(FormatType::Exp), Value::Int(i)) => {
                format!("{:.*e}", spec.precision.unwrap_or(6), *i as f64)
            }
            (Some(FormatType::Percent), Value::Float(f)) => {
                format!("{:.*}%", spec.precision.unwrap_or(6), f * 100.0)
            }
            (Some(FormatType::Percent), Value::Int(i)) => {
                format!("{:.*}%", spec.precision.unwrap_or(6), *i as f64 * 100.0)
            }
            (Some(ty), other) => {
                return Err(self.error(
                    codes::TYPE_MISMATCH,
                    format!(
                        "format type `{}` does not apply to {}",
                        format_type_name(ty),
                        other.type_name()
                    ),
                    spec.span,
                ))
            }
        };
        // 2. Precision for a plain float without an explicit float type.
        if spec.ty.is_none() {
            if let (Some(p), Value::Float(f)) = (spec.precision, v) {
                text = format!("{f:.p$}");
            }
        }
        // 3. Sign.
        match spec.sign {
            Some(Sign::Plus) if !text.starts_with('-') => text.insert(0, '+'),
            Some(Sign::Space) if !text.starts_with('-') => text.insert(0, ' '),
            _ => {}
        }
        // 4. Width, fill, and alignment. Numeric values default to
        // right-alignment; everything else to left.
        if let Some(width) = spec.width {
            let len = text.chars().count();
            if len < width {
                let pad = width - len;
                let numeric = matches!(v, Value::Int(_) | Value::Float(_));
                let align = spec
                    .align
                    .unwrap_or(if numeric { Align::Right } else { Align::Left });
                let fill = spec.fill.unwrap_or(' ');
                let (left, right) = match align {
                    Align::Left => (0, pad),
                    Align::Right => (pad, 0),
                    Align::Center => (pad / 2, pad - pad / 2),
                };
                let mut padded = String::with_capacity(width);
                padded.extend(std::iter::repeat_n(fill, left));
                padded.push_str(&text);
                padded.extend(std::iter::repeat_n(fill, right));
                text = padded;
            }
        }
        Ok(text)
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

/// Bind evaluated argument values to a user function's parameters.
///
/// `args` and `vals` are both in **source order**; `params` is the callee's
/// parameter names in declaration order. Positional arguments fill the next
/// unfilled parameter; named arguments fill the parameter with that exact
/// name. The result is the values in parameter order, ready for the existing
/// positional call machinery.
///
/// The checker already validated the call for directly resolved functions
/// (§15.7), so the mapping is total here; any residual problem (for example a
/// named argument reaching a call the checker could not resolve) is reported
/// as a runtime `E3001`, preserving runtime validation as the final layer.
/// The source spelling of a format type, for diagnostics.
fn format_type_name(t: crate::ast::FormatType) -> &'static str {
    use crate::ast::FormatType;
    match t {
        FormatType::Dec => "d",
        FormatType::Binary => "b",
        FormatType::Octal => "o",
        FormatType::Hex { upper: false } => "x",
        FormatType::Hex { upper: true } => "X",
        FormatType::Fixed => "f",
        FormatType::Exp => "e",
        FormatType::Percent => "%",
    }
}

fn bind_arguments(
    args: &[Arg],
    vals: &[Value],
    params: &[(String, bool)],
    span: Span,
) -> Result<Vec<Value>> {
    let mut bound: Vec<Option<Value>> = vec![None; params.len()];
    let mut next_positional = 0usize;
    for (i, arg) in args.iter().enumerate() {
        let value = vals.get(i).cloned().unwrap_or(Value::None);
        match &arg.name {
            None => {
                if next_positional >= params.len() {
                    return Err(Diag::new(
                        codes::TYPE_MISMATCH,
                        format!("expected {} argument(s), got {}", params.len(), args.len()),
                        span,
                    ));
                }
                bound[next_positional] = Some(value);
                next_positional += 1;
            }
            Some(param_name) => {
                let Some(index) = params.iter().position(|(p, _)| p == param_name) else {
                    return Err(Diag::new(
                        codes::TYPE_MISMATCH,
                        format!("no parameter named `{param_name}`"),
                        span,
                    ));
                };
                if bound[index].is_some() {
                    return Err(Diag::new(
                        codes::TYPE_MISMATCH,
                        format!("parameter `{param_name}` is given more than once"),
                        span,
                    ));
                }
                bound[index] = Some(value);
            }
        }
    }
    bound
        .into_iter()
        .map(|v| v.ok_or_else(|| Diag::new(codes::TYPE_MISMATCH, "missing argument", span)))
        .collect()
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
        | Expr::Call(_, _, _, s)
        | Expr::Method(_, _, _, _, s)
        | Expr::Field(_, _, s)
        | Expr::Index(_, _, s)
        | Expr::List(_, s)
        | Expr::Map(_, s)
        | Expr::Construct(_, _, _, s)
        | Expr::Tuple(_, s)
        | Expr::Lambda(_, _, s)
        | Expr::Pipe(_, _, s)
        | Expr::Range(_, _, s)
        | Expr::If(_, _, _, s)
        | Expr::Match(_, _, s)
        | Expr::Block(_, s) => *s,
        Expr::ListComp { span, .. } | Expr::MapComp { span, .. } => *span,
    }
}

impl Default for Interp {
    fn default() -> Interp {
        Interp::new()
    }
}
