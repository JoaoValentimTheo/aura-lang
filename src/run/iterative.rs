//! Explicit-continuation (iterative) evaluator — B-1R3A.
//!
//! This is the **real** explicit continuation machine from
//! `docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md` §16. It is compiled only
//! with the non-default `evaluator-oracle` feature, so default and production
//! builds contain no path to it (`§24`, `§5.6`). Production execution stays on
//! the recursive interpreter in `super`.
//!
//! ## Why this is not recursive
//!
//! Pending execution lives in [`Machine::kont`] (a `Vec<Cont>` continuation
//! stack) and [`Machine::ctrl`] (the current [`Ctrl`] work item). Evaluation is
//! one `loop { match ctrl { ... } }`; the Rust call stack does **not** encode
//! nesting. `deliver`/`resume` are iterative: no helper recurses per suspended
//! node.
//!
//! ## B-1R3A supported subset
//!
//! * expression literals (`int`/`float`/`string`/`bool`/`none`);
//! * name lookup with the exact recursive resolution order
//!   (environment → single-overload function → native);
//! * expression statements and sequential statement blocks (scoped and
//!   unscoped), including `let` shadowing write-back;
//! * empty and multi-statement blocks;
//! * block expressions and `if`/`else` expressions.
//!
//! ## B-1R3B.1 added subset
//!
//! * unary operators (`-`, `not`, `~`), with the operand evaluated exactly
//!   once, before the operator, via [`Cont::UnaryApply`]. Semantics, spans,
//!   and diagnostics mirror `Interp::eval`'s `Expr::Unary` arm exactly.
//!
//! ## B-1R3B.2 added subset
//!
//! * eager (non-short-circuit) binary operators: `+`, `-`, `*`, `/`, `%`, `^`,
//!   `==`, `!=`, `<`, `<=`, `>`, `>=`, `&`, `|`, `<<`, `>>`. Evaluation is
//!   left-to-right and exactly once per operand, via [`Cont::BinaryLeft`] then
//!   [`Cont::BinRight`]; the operator is applied only after both operands
//!   complete. Application reuses `Interp::binary` (the recursive engine's
//!   value-level operator helper, which never evaluates an AST node), so
//!   arithmetic, overflow, comparison, equality, and diagnostic behavior are
//!   identical by construction.
//! * `and`/`or` are **short-circuit** and belong to B-1R3B.3; they remain
//!   unsupported here and fail with the deterministic `E4999` sentinel.
//!
//! ## Explicitly unsupported (R3B–R3F)
//!
//! Every other `Expr`/`Stmt` form fails with a deterministic
//! `E4999` (`codes::INTERNAL`) "not supported by the iterative engine". It
//! **never** falls back to the recursive evaluator, so the differential oracle
//! cannot mistake recursive execution for iterative progress.
//!
//! ## Honest gaps
//!
//! * **Calls are not implemented** (R3C). `UserFrame`/`push_frame`/`pop_frame`
//!   and `Cont::FrameBoundary` establish the 512/513 accounting model and are
//!   exercised synthetically by unit tests; no `Ctrl` produces a frame yet.
//! * **`try`/`finally` are not implemented** (R3F).
//! * The shadowing write-back travels in [`Done::env`], which is exact for the
//!   call-free R3A subset (conts run depth-first with no suspension). R3C must
//!   keep this carrier explicit across suspensions.
//! * `MAX_AST_DEPTH` (`E1015`) and `MAX_CALL_FRAMES` (`E4011`) preserve the
//!   recursive engine's semantics exactly.

use std::rc::Rc;
use std::sync::Arc;

use super::value::Value;
use super::{Closure, Ctl, Env, Interp, MAX_AST_DEPTH, MAX_CALL_FRAMES};
use crate::ast::{BinOp, Expr, Lit, Stmt, UnOp};
use crate::error::{codes, Result, Span};
use crate::source::SourceId;

/// What the machine does after processing one control item.
enum Control {
    /// Continue with this next work item.
    Next(Ctrl),
    /// No continuations remain; the machine finished with this completion.
    Finished(Ctl),
}

/// A completion travelling to the innermost continuation.
///
/// `env` carries the environment write-back produced by a `let` shadowing,
/// mirroring `exec_stmt`'s `&mut Env` (`super::Interp::exec_block`). It is
/// consumed by the immediately enclosing [`Cont::Block`].
struct Done {
    ctl: Ctl,
    env: Option<Env>,
}

impl Done {
    /// A completion with no environment change.
    fn plain(ctl: Ctl) -> Done {
        Done { ctl, env: None }
    }
}

/// The outcome of resuming one continuation.
enum Resume {
    /// Hand this completion to the next outer continuation.
    Redeliver(Done),
    /// Continue the machine loop with this work item.
    Next(Ctrl),
}

/// The current computational focus (`ITERATIVE_EVALUATOR_DESIGN.md` §16.2).
enum Ctrl {
    /// Evaluate an expression in an environment.
    EvalExpr(Arc<Expr>, Env),
    /// Execute a statement in its resolution environment.
    EvalStmt(Arc<Stmt>, Env),
    /// Enter a statement block (`body`, lexical parent, scoped).
    EnterBlock(Arc<[Stmt]>, Env, bool),
    /// A value/signal is ready; deliver it to the innermost continuation.
    Done(Ctl),
    /// A `return` operand finished evaluating; produce the `return` signal.
    ReturnValue(Value),
    /// A `throw` operand finished evaluating; produce the `throw` signal.
    ThrowValue(Value),
}

/// One explicit continuation record (`ITERATIVE_EVALUATOR_DESIGN.md` §16.3).
///
/// Only the R3A subset is populated; R3B–R3F add the remaining variants
/// mechanically from the suspension matrices (§7/§8).
enum Cont {
    /// Decrement the per-frame AST-depth counter when the wrapped expression
    /// completes (mirrors `Interp::eval`'s increment/decrement).
    ExprDepth,
    /// A statement block in progress. The value of each completed statement
    /// travels in [`Done`], so no separate `last` slot is needed.
    Block {
        body: Arc<[Stmt]>,
        /// Index of the next statement to run.
        index: usize,
        /// Current environment of the block; advances on `let` shadowing.
        local: Env,
        /// Whether the block created a child scope.
        scoped: bool,
    },
    /// A `let` initializer finished; install the binding with shadowing and
    /// hand the advanced environment to the enclosing block.
    LetBind {
        name: String,
        mutable: bool,
        env: Env,
    },
    /// An `if` condition finished; select the branch.
    IfBranch {
        then: Arc<[Stmt]>,
        els: Option<Arc<Expr>>,
        env: Env,
    },
    /// A unary operand finished; apply the operator (B-1R3B.1).
    UnaryApply { op: UnOp, span: Span },
    /// A binary left operand finished; remember it and evaluate the right
    /// operand (B-1R3B.2). Only eager operators reach here.
    BinaryLeft {
        op: BinOp,
        rhs: Arc<Expr>,
        env: Env,
        span: Span,
    },
    /// A binary right operand finished; apply the operator (B-1R3B.2).
    BinRight { op: BinOp, lv: Value, span: Span },
    /// A `return` operand finished; convert to the `return` signal.
    ReturnFrom,
    /// A `throw` operand finished; convert to the `throw` signal.
    ThrowFrom,
    /// A user-frame boundary (R3C): pop one [`UserFrame`] and map the call's
    /// completion. Not produced by the R3A subset.
    #[allow(dead_code)]
    FrameBoundary { call_span: Span },
}

/// An explicit Aura user call frame (`ITERATIVE_EVALUATOR_DESIGN.md` §16.2).
///
/// `Machine::frames.len()` **is** the active-frame count checked against
/// `MAX_CALL_FRAMES`; `main` occupies frame 1. The fields beyond the accounting
/// model are established for R3C (calls).
#[allow(dead_code)]
struct UserFrame {
    closure: Rc<Closure>,
    env: Env,
    saved_source: Option<SourceId>,
    frame_source: Option<SourceId>,
    call_span: Span,
}

/// The explicit continuation machine.
struct Machine<'i> {
    interp: &'i mut Interp,
    /// Current computational focus.
    ctrl: Ctrl,
    /// Continuations, innermost last.
    kont: Vec<Cont>,
    /// Aura user call frames; `len()` **is** the active-frame count.
    frames: Vec<UserFrame>,
    /// Expression nesting depth of the expression currently being evaluated
    /// (mirrors `Interp::ast_depth`, including its per-frame reset in R3C).
    expr_depth: usize,
    /// Environment produced by the last top-level statement, if any (used by a
    /// REPL-style global statement entry).
    #[allow(dead_code)]
    top_env: Option<Env>,
}

impl<'i> Machine<'i> {
    /// Create a machine over `interp`.
    fn new(interp: &'i mut Interp) -> Machine<'i> {
        Machine {
            interp,
            ctrl: Ctrl::Done(Ctl::Val(Value::None)),
            kont: Vec::new(),
            frames: Vec::new(),
            expr_depth: 0,
            top_env: None,
        }
    }

    /// Drive the machine to completion.
    fn run(&mut self, initial: Ctrl) -> Result<Ctl> {
        self.ctrl = initial;
        loop {
            // The current focus is machine state, not a Rust call frame.
            let ctrl = std::mem::replace(&mut self.ctrl, Ctrl::Done(Ctl::Val(Value::None)));
            match self.step(ctrl)? {
                Control::Next(next) => self.ctrl = next,
                Control::Finished(ctl) => return Ok(ctl),
            }
        }
    }

    /// Process one control item.
    fn step(&mut self, ctrl: Ctrl) -> Result<Control> {
        match ctrl {
            Ctrl::Done(ctl) => self.deliver(Done::plain(ctl)),
            Ctrl::EvalExpr(e, env) => self.start_expr(&e, &env),
            Ctrl::EvalStmt(s, env) => self.start_stmt(&s, &env),
            Ctrl::EnterBlock(body, parent, scoped) => self.start_block(body, parent, scoped),
            Ctrl::ReturnValue(v) => self.deliver(Done::plain(Ctl::Return(v))),
            Ctrl::ThrowValue(v) => self.deliver(Done::plain(Ctl::Throw(v))),
        }
    }

    /// Deliver a completion to the innermost continuation, or finish.
    ///
    /// Iterative: continuation records that only transform or forward a
    /// completion (for example [`Cont::ExprDepth`]) loop here rather than
    /// recursing.
    fn deliver(&mut self, mut done: Done) -> Result<Control> {
        loop {
            let Some(cont) = self.kont.pop() else {
                if done.env.is_some() {
                    self.top_env = done.env.take();
                }
                return Ok(Control::Finished(done.ctl));
            };
            match self.resume(cont, done)? {
                Resume::Redeliver(next) => done = next,
                Resume::Next(next) => return Ok(Control::Next(next)),
            }
        }
    }

    /// Resume one continuation with a completion.
    fn resume(&mut self, cont: Cont, done: Done) -> Result<Resume> {
        match cont {
            Cont::ExprDepth => {
                self.expr_depth -= 1;
                Ok(Resume::Redeliver(done))
            }
            Cont::Block {
                body,
                index,
                local,
                scoped,
            } => {
                let Ctl::Val(value) = done.ctl else {
                    // `return`/`throw`/`break`/`continue` propagate outward
                    // through the block unchanged (§9).
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                // A `let` may have advanced the block's environment; adopt it
                // exactly where `exec_block` advances `local`.
                let local = done.env.unwrap_or(local);
                if index < body.len() {
                    let stmt = Arc::new(body[index].clone());
                    self.kont.push(Cont::Block {
                        body,
                        index: index + 1,
                        local: local.clone(),
                        scoped,
                    });
                    Ok(Resume::Next(Ctrl::EvalStmt(stmt, local)))
                } else {
                    Ok(Resume::Redeliver(Done::plain(Ctl::Val(value))))
                }
            }
            Cont::LetBind { name, mutable, env } => {
                if let Ctl::Val(value) = done.ctl {
                    let advanced = env.define_shadowing(name, value, mutable);
                    // The statement's own value is `none`; the advanced
                    // environment rides to the enclosing block.
                    Ok(Resume::Redeliver(Done {
                        ctl: Ctl::Val(Value::None),
                        env: Some(advanced),
                    }))
                } else {
                    Ok(Resume::Redeliver(Done::plain(done.ctl)))
                }
            }
            Cont::IfBranch { then, els, env } => {
                if let Ctl::Val(value) = done.ctl {
                    if value.truthy() {
                        Ok(Resume::Next(Ctrl::EnterBlock(then, env, true)))
                    } else if let Some(e) = els {
                        Ok(Resume::Next(Ctrl::EvalExpr(e, env)))
                    } else {
                        Ok(Resume::Redeliver(Done::plain(Ctl::Val(Value::None))))
                    }
                } else {
                    Ok(Resume::Redeliver(Done::plain(done.ctl)))
                }
            }
            Cont::UnaryApply { op, span } => {
                if let Ctl::Val(v) = done.ctl {
                    Ok(Resume::Next(Ctrl::Done(self.apply_unary(op, v, span)?)))
                } else {
                    // A `return`/`throw`/`break`/`continue` from the operand
                    // propagates unchanged (mirrors `val!` in `Interp::eval`).
                    Ok(Resume::Redeliver(Done::plain(done.ctl)))
                }
            }
            Cont::BinaryLeft { op, rhs, env, span } => {
                if let Ctl::Val(lv) = done.ctl {
                    // Left completed: retain it and evaluate right exactly once.
                    self.kont.push(Cont::BinRight { op, lv, span });
                    Ok(Resume::Next(Ctrl::EvalExpr(rhs, env)))
                } else {
                    // A control signal from the left operand propagates without
                    // evaluating the right operand (mirrors `val!`).
                    Ok(Resume::Redeliver(Done::plain(done.ctl)))
                }
            }
            Cont::BinRight { op, lv, span } => {
                if let Ctl::Val(rv) = done.ctl {
                    Ok(Resume::Next(Ctrl::Done(Ctl::Val(
                        self.interp.binary(op, lv, rv, span)?,
                    ))))
                } else {
                    // A control signal from the right operand propagates without
                    // applying the operator (mirrors `val!`).
                    Ok(Resume::Redeliver(Done::plain(done.ctl)))
                }
            }
            Cont::ReturnFrom => match done.ctl {
                Ctl::Val(v) => Ok(Resume::Next(Ctrl::ReturnValue(v))),
                other => Ok(Resume::Redeliver(Done::plain(other))),
            },
            Cont::ThrowFrom => match done.ctl {
                Ctl::Val(v) => Ok(Resume::Next(Ctrl::ThrowValue(v))),
                other => Ok(Resume::Redeliver(Done::plain(other))),
            },
            Cont::FrameBoundary { call_span } => self.resume_frame_boundary(done, call_span),
        }
    }

    /// Apply a unary operator to an already-evaluated operand (B-1R3B.1).
    ///
    /// This mirrors `Interp::eval`'s `Expr::Unary` arm exactly: same operators,
    /// same type rules, same diagnostic codes/messages, same `E4013` overflow
    /// handling, and the same expression span. No fallback to the recursive
    /// evaluator.
    fn apply_unary(&self, op: UnOp, v: Value, span: Span) -> Result<Ctl> {
        match op {
            UnOp::Neg => match v {
                Value::Int(i) => Ok(Ctl::Val(Value::Int(i.checked_neg().ok_or_else(|| {
                    self.interp.error(codes::OVERFLOW, "integer overflow", span)
                })?))),
                Value::Float(f) => Ok(Ctl::Val(Value::Float(-f))),
                other => Err(self.interp.error(
                    codes::TYPE_MISMATCH,
                    format!("cannot negate {}", other.type_name()),
                    span,
                )),
            },
            UnOp::Not => Ok(Ctl::Val(Value::Bool(!v.truthy()))),
            UnOp::BitNot => match v {
                Value::Int(i) => Ok(Ctl::Val(Value::Int(!i))),
                other => Err(self.interp.error(
                    codes::TYPE_MISMATCH,
                    format!(
                        "operator `~` requires an integer, found {}",
                        other.type_name()
                    ),
                    span,
                )),
            },
        }
    }

    /// Map a completed call's outcome at its user-frame boundary
    /// (`ITERATIVE_EVALUATOR_DESIGN.md` §9 rule 3). Not reached by the R3A
    /// subset; unit-tested directly.
    fn resume_frame_boundary(&mut self, done: Done, call_span: Span) -> Result<Resume> {
        let _ = self.pop_frame();
        match done.ctl {
            Ctl::Val(v) | Ctl::Return(v) => Ok(Resume::Redeliver(Done::plain(Ctl::Val(v)))),
            Ctl::Throw(v) => {
                self.interp.pending_throw = Some(v.clone());
                Err(self.interp.error(
                    codes::THROWN,
                    format!("uncaught value: {}", v.display()),
                    call_span,
                ))
            }
            Ctl::Break => {
                Err(self
                    .interp
                    .error(codes::LOOP_CONTROL, "`break` outside a loop", call_span))
            }
            Ctl::Continue => {
                Err(self
                    .interp
                    .error(codes::LOOP_CONTROL, "`continue` outside a loop", call_span))
            }
        }
    }

    /// Begin evaluating an expression, preserving the `E1015` AST-depth guard.
    fn start_expr(&mut self, e: &Arc<Expr>, env: &Env) -> Result<Control> {
        self.expr_depth += 1;
        if self.expr_depth > MAX_AST_DEPTH {
            self.expr_depth -= 1;
            return Err(self.interp.error(
                codes::NESTING,
                "expression nests too deeply to evaluate",
                e.span(),
            ));
        }
        // The guard is popped (and the depth decremented) when this expression
        // completes, exactly like `eval`'s trailing `ast_depth -= 1`.
        self.kont.push(Cont::ExprDepth);
        match &**e {
            Expr::Lit(l, _) => Ok(Control::Next(Ctrl::Done(Ctl::Val(literal(l))))),
            Expr::Name(name, span) => self.eval_name(name, *span, env),
            Expr::Block(body, _) => Ok(Control::Next(Ctrl::EnterBlock(
                body.clone(),
                env.clone(),
                true,
            ))),
            Expr::If(cond, then, els, _) => {
                self.kont.push(Cont::IfBranch {
                    then: then.clone(),
                    els: els.clone(),
                    env: env.clone(),
                });
                Ok(Control::Next(Ctrl::EvalExpr(cond.clone(), env.clone())))
            }
            Expr::Unary(op, operand, span) => {
                // Operand first, exactly once; the operator is applied when the
                // operand completes (`Cont::UnaryApply`).
                self.kont.push(Cont::UnaryApply {
                    op: *op,
                    span: *span,
                });
                Ok(Control::Next(Ctrl::EvalExpr(operand.clone(), env.clone())))
            }
            Expr::Binary(op, l, r, span) => {
                match op {
                    // `and`/`or` short-circuit: B-1R3B.3. Never fall back.
                    BinOp::And | BinOp::Or => Err(self.unsupported(*span)),
                    // Eager binary: left first, exactly once.
                    _ => {
                        self.kont.push(Cont::BinaryLeft {
                            op: *op,
                            rhs: r.clone(),
                            env: env.clone(),
                            span: *span,
                        });
                        Ok(Control::Next(Ctrl::EvalExpr(l.clone(), env.clone())))
                    }
                }
            }
            other => Err(self.unsupported(expr_span(other))),
        }
    }

    /// Resolve a name with the recursive engine's exact order: environment,
    /// then a single-overload function value, then a native reference.
    fn eval_name(&self, name: &str, span: Span, env: &Env) -> Result<Control> {
        if let Some(v) = env.get(name) {
            return Ok(Control::Next(Ctrl::Done(Ctl::Val(v))));
        }
        if let Some(closure) = self
            .interp
            .functions
            .get(name)
            .and_then(|s| s.first())
            .cloned()
        {
            return Ok(Control::Next(Ctrl::Done(Ctl::Val(Value::Closure(closure)))));
        }
        if self.interp.natives.contains_key(name) {
            return Ok(Control::Next(Ctrl::Done(Ctl::Val(Value::Native(
                name.to_string(),
            )))));
        }
        Err(self.interp.error(
            codes::UNDEFINED,
            format!("undefined variable `{name}`"),
            span,
        ))
    }

    /// Begin executing a statement.
    fn start_stmt(&mut self, s: &Arc<Stmt>, env: &Env) -> Result<Control> {
        match &**s {
            Stmt::Expr(e, _) => Ok(Control::Next(Ctrl::EvalExpr(
                Arc::new(e.clone()),
                env.clone(),
            ))),
            Stmt::Let {
                name,
                value,
                mutable,
                ..
            } => {
                self.kont.push(Cont::LetBind {
                    name: name.clone(),
                    mutable: *mutable,
                    env: env.clone(),
                });
                Ok(Control::Next(Ctrl::EvalExpr(
                    Arc::new(value.clone()),
                    env.clone(),
                )))
            }
            Stmt::Return(value, _) => match value {
                Some(e) => {
                    self.kont.push(Cont::ReturnFrom);
                    Ok(Control::Next(Ctrl::EvalExpr(
                        Arc::new(e.clone()),
                        env.clone(),
                    )))
                }
                None => Ok(Control::Next(Ctrl::Done(Ctl::Return(Value::None)))),
            },
            Stmt::Throw(value, _) => {
                self.kont.push(Cont::ThrowFrom);
                Ok(Control::Next(Ctrl::EvalExpr(
                    Arc::new(value.clone()),
                    env.clone(),
                )))
            }
            Stmt::Break(_) => Ok(Control::Next(Ctrl::Done(Ctl::Break))),
            Stmt::Continue(_) => Ok(Control::Next(Ctrl::Done(Ctl::Continue))),
            other => Err(self.unsupported(other.span())),
        }
    }

    /// Begin executing a statement block.
    ///
    /// `Result` is kept for a uniform `step` dispatch signature even though this
    /// arm cannot currently fail; later block forms may.
    #[allow(clippy::unnecessary_wraps)]
    fn start_block(&mut self, body: Arc<[Stmt]>, parent: Env, scoped: bool) -> Result<Control> {
        // `exec_block`: a scoped block gets a fresh child; an unscoped block
        // (a call body, a `for` body) aliases the parent.
        let local = if scoped { parent.child() } else { parent };
        if body.is_empty() {
            return Ok(Control::Next(Ctrl::Done(Ctl::Val(Value::None))));
        }
        let stmt = Arc::new(body[0].clone());
        self.kont.push(Cont::Block {
            body,
            index: 1,
            local: local.clone(),
            scoped,
        });
        Ok(Control::Next(Ctrl::EvalStmt(stmt, local)))
    }

    /// The deterministic "unsupported in the iterative engine" diagnostic.
    fn unsupported(&self, span: Span) -> crate::error::Diag {
        self.interp.error(
            codes::INTERNAL,
            "construct is not supported by the iterative engine (B-1R3A)",
            span,
        )
    }

    /// Push a user frame, enforcing the 512/513 contract (`§17`).
    #[allow(dead_code)]
    fn push_frame(&mut self, closure: Rc<Closure>, env: Env, call_span: Span) -> Result<()> {
        if self.frames.len() >= MAX_CALL_FRAMES {
            return Err(self.interp.error(
                codes::RECURSION,
                "call depth limit exceeded",
                call_span,
            ));
        }
        let frame_source = self
            .interp
            .closure_sources
            .get(&(Rc::as_ptr(&closure) as usize))
            .copied();
        self.frames.push(UserFrame {
            closure,
            env,
            saved_source: self.interp.current_source,
            frame_source,
            call_span,
        });
        Ok(())
    }

    /// Pop a user frame (idempotent at zero; no underflow).
    #[allow(dead_code)]
    fn pop_frame(&mut self) -> Option<UserFrame> {
        self.frames.pop()
    }
}

/// Map a literal AST node to its runtime value.
fn literal(l: &Lit) -> Value {
    match l {
        Lit::Int(i) => Value::Int(*i),
        Lit::Float(f) => Value::Float(*f),
        Lit::Str(s) => Value::str(s.clone()),
        Lit::Bool(b) => Value::Bool(*b),
        Lit::None => Value::None,
    }
}

/// Evaluate `e` in `env` with the explicit continuation machine.
pub(crate) fn eval_expr(interp: &mut Interp, e: &Expr, env: &Env) -> Result<Ctl> {
    let mut machine = Machine::new(interp);
    machine.run(Ctrl::EvalExpr(Arc::new(e.clone()), env.clone()))
}

/// Enter a user closure's body as Aura user frame 1 and run it with the
/// machine. This is the R3A entry for a module's `main`; general first-class
/// calls (arguments, overloads) are R3C work, so this only supports the
/// parameterless entry call, which is exactly `main`'s contract.
///
/// The body runs as an unscoped block, exactly like `Interp::call`'s
/// `exec_block(&closure.body, &env, false)`.
pub(crate) fn call_closure_body(
    interp: &mut Interp,
    closure: Rc<Closure>,
    args: Vec<Value>,
    span: Span,
) -> Result<Ctl> {
    if args.len() != closure.params.len() {
        return Err(interp.error(
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
    let mut machine = Machine::new(interp);
    machine.push_frame(closure.clone(), env.clone(), span)?;
    machine.kont.push(Cont::FrameBoundary { call_span: span });
    machine.run(Ctrl::EnterBlock(closure.body.clone(), env, false))
}

/// Execute `s` in `env` with the machine. Returns the completion and the
/// possibly-advanced environment (a top-level `let` shadowing).
#[allow(dead_code)]
pub(crate) fn exec_stmt(interp: &mut Interp, s: &Stmt, env: &Env) -> Result<(Ctl, Option<Env>)> {
    let mut machine = Machine::new(interp);
    let ctl = machine.run(Ctrl::EvalStmt(Arc::new(s.clone()), env.clone()))?;
    Ok((ctl, machine.top_env.take()))
}

/// Span accessor used only for the unsupported diagnostic.
fn expr_span(e: &Expr) -> Span {
    e.span()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::ast::{BinOp, Stmt};
    use crate::error::codes;

    fn lit(i: i64) -> Expr {
        Expr::Lit(Lit::Int(i), Span::default())
    }

    fn expr_stmt(e: Expr) -> Stmt {
        Stmt::Expr(e, Span::default())
    }

    fn run_expr(expr: &Expr) -> Result<Ctl> {
        let mut interp = Interp::new();
        let globals = interp.globals.clone();
        eval_expr(&mut interp, expr, &globals)
    }

    /// Run statements as a scoped block expression, returning the completion.
    fn run_stmts(stmts: Vec<Stmt>) -> Result<(Ctl, Option<Env>)> {
        let mut interp = Interp::new();
        let globals = interp.globals.clone();
        let mut machine = Machine::new(&mut interp);
        let ctl = machine.run(Ctrl::EnterBlock(Arc::from(stmts), globals, true))?;
        Ok((ctl, machine.top_env.take()))
    }

    #[test]
    fn literal_int_evaluates() {
        assert!(matches!(
            run_expr(&lit(42)).unwrap(),
            Ctl::Val(Value::Int(42))
        ));
    }

    #[test]
    fn sequential_block_returns_last_value() {
        let (ctl, _) = run_stmts(vec![
            expr_stmt(lit(1)),
            expr_stmt(lit(2)),
            expr_stmt(lit(3)),
        ])
        .unwrap();
        match ctl {
            Ctl::Val(Value::Int(v)) => assert_eq!(v, 3),
            _ => panic!("expected 3, got a different completion"),
        }
    }

    #[test]
    fn empty_block_is_none() {
        let (ctl, _) = run_stmts(Vec::new()).unwrap();
        assert!(matches!(ctl, Ctl::Val(Value::None)));
    }

    #[test]
    fn undefined_name_is_e2003() {
        let e = Expr::Name("nope".to_string(), Span::default());
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::UNDEFINED);
        assert_eq!(err.message, "undefined variable `nope`");
    }

    #[test]
    fn let_shadowing_advances_block_env() {
        // let x = 1; let x = 2; x  ->  2
        let stmts = vec![
            Stmt::Let {
                mutable: false,
                name: "x".to_string(),
                ann: None,
                value: lit(1),
                span: Span::default(),
            },
            Stmt::Let {
                mutable: false,
                name: "x".to_string(),
                ann: None,
                value: lit(2),
                span: Span::default(),
            },
            expr_stmt(Expr::Name("x".to_string(), Span::default())),
        ];
        let (ctl, _) = run_stmts(stmts).unwrap();
        match ctl {
            Ctl::Val(Value::Int(v)) => assert_eq!(v, 2),
            _ => panic!("expected 2, got a different completion"),
        }
    }

    #[test]
    fn scoped_block_does_not_leak_binding_to_parent() {
        // A scoped block (`exec_block(..., true)`) must define into a child
        // environment, so a binding created inside is invisible to the
        // enclosing scope. This is the environment-aliasing property the design
        // requires; it is not observable through ordinary programs (the checker
        // rejects out-of-scope references), so it is pinned structurally here.
        let mut interp = Interp::new();
        let parent = interp.globals.clone();
        let inner = vec![Stmt::Let {
            mutable: false,
            name: "only_inside".to_string(),
            ann: None,
            value: lit(1),
            span: Span::default(),
        }];
        let mut machine = Machine::new(&mut interp);
        machine
            .run(Ctrl::EnterBlock(
                Arc::from(inner.clone()),
                parent.clone(),
                true,
            ))
            .unwrap();
        assert!(
            parent.get("only_inside").is_none(),
            "scoped block leaked a binding into its parent"
        );

        // An unscoped block aliases the parent, so the same binding does leak —
        // exactly like `exec_block(..., false)`.
        let mut interp = Interp::new();
        let parent = interp.globals.clone();
        let mut machine = Machine::new(&mut interp);
        machine
            .run(Ctrl::EnterBlock(Arc::from(inner), parent.clone(), false))
            .unwrap();
        assert!(
            parent.get("only_inside").is_some(),
            "unscoped block did not alias its parent"
        );
    }

    #[test]
    fn unsupported_construct_fails_explicitly() {
        // A list literal is R3B.4 territory and remains unsupported. The
        // machine must fail explicitly, never fall back to recursion.
        let e = Expr::List(Arc::from(vec![lit(1), lit(2)]), Span::default());
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::INTERNAL);
        assert!(err
            .message
            .contains("not supported by the iterative engine"));
    }

    #[test]
    fn return_signal_propagates_out_of_block() {
        let stmts = vec![
            expr_stmt(lit(1)),
            Stmt::Return(Some(lit(7)), Span::default()),
            expr_stmt(lit(9)),
        ];
        let (ctl, _) = run_stmts(stmts).unwrap();
        assert!(matches!(ctl, Ctl::Return(Value::Int(7))));
    }

    #[test]
    fn throw_signal_propagates_out_of_block() {
        let stmts = vec![Stmt::Throw(lit(3), Span::default()), expr_stmt(lit(9))];
        let (ctl, _) = run_stmts(stmts).unwrap();
        assert!(matches!(ctl, Ctl::Throw(Value::Int(3))));
    }

    #[test]
    fn if_expression_selects_branch() {
        let e = Expr::If(
            Arc::new(Expr::Lit(Lit::Bool(true), Span::default())),
            Arc::from([expr_stmt(lit(10))]),
            Some(Arc::new(lit(20))),
            Span::default(),
        );
        match run_expr(&e).unwrap() {
            Ctl::Val(Value::Int(v)) => assert_eq!(v, 10),
            _ => panic!("expected 10, got a different completion"),
        }
        let e = Expr::If(
            Arc::new(Expr::Lit(Lit::Bool(false), Span::default())),
            Arc::from([expr_stmt(lit(10))]),
            Some(Arc::new(lit(20))),
            Span::default(),
        );
        match run_expr(&e).unwrap() {
            Ctl::Val(Value::Int(v)) => assert_eq!(v, 20),
            _ => panic!("expected 20, got a different completion"),
        }
    }

    #[test]
    fn ast_depth_limit_is_e1015() {
        // Nest `if` expressions past MAX_AST_DEPTH along the *taken* path
        // (`else`, with a false condition); the machine must produce E1015
        // exactly as the recursive evaluator does.
        let mut e = lit(0);
        for _ in 0..(MAX_AST_DEPTH + 10) {
            e = Expr::If(
                Arc::new(Expr::Lit(Lit::Bool(false), Span::default())),
                Arc::from(Vec::<Stmt>::new()),
                Some(Arc::new(e)),
                Span::default(),
            );
        }
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::NESTING);
    }

    #[test]
    fn frame_accounting_is_exact_512_513() {
        let mut interp = Interp::new();
        let globals = interp.globals.clone();
        let closure = Rc::new(Closure {
            name: "f".to_string(),
            params: Vec::new(),
            param_tys: Vec::new(),
            body: Arc::from(Vec::<Stmt>::new()),
            env: globals.clone(),
        });
        let mut machine = Machine::new(&mut interp);
        for i in 0..MAX_CALL_FRAMES {
            machine
                .push_frame(closure.clone(), globals.clone(), Span::default())
                .unwrap_or_else(|e| panic!("frame {} rejected: {e:?}", i + 1));
        }
        assert_eq!(machine.frames.len(), MAX_CALL_FRAMES);
        let err = machine
            .push_frame(closure.clone(), globals.clone(), Span::default())
            .err()
            .unwrap();
        assert_eq!(err.code, codes::RECURSION);
        assert_eq!(err.message, "call depth limit exceeded");
        assert_eq!(
            machine.frames.len(),
            MAX_CALL_FRAMES,
            "rejected frame was pushed"
        );
        for _ in 0..MAX_CALL_FRAMES {
            assert!(machine.pop_frame().is_some());
        }
        assert!(machine.pop_frame().is_none(), "frame underflow");
    }

    #[test]
    fn frame_boundary_converts_return_to_value() {
        let mut interp = Interp::new();
        let globals = interp.globals.clone();
        let closure = Rc::new(Closure {
            name: "f".to_string(),
            params: Vec::new(),
            param_tys: Vec::new(),
            body: Arc::from(Vec::<Stmt>::new()),
            env: globals.clone(),
        });
        let mut machine = Machine::new(&mut interp);
        machine
            .push_frame(closure, globals, Span::default())
            .unwrap();
        machine.kont.push(Cont::FrameBoundary {
            call_span: Span::default(),
        });
        let ctl = machine
            .deliver(Done::plain(Ctl::Return(Value::Int(5))))
            .unwrap();
        match ctl {
            Control::Finished(Ctl::Val(Value::Int(5))) => {}
            _ => panic!("expected Finished(Val(5)), got a different control"),
        }
        assert!(machine.frames.is_empty());
    }

    #[test]
    fn frame_boundary_converts_throw_to_e4099() {
        let mut interp = Interp::new();
        let globals = interp.globals.clone();
        let closure = Rc::new(Closure {
            name: "f".to_string(),
            params: Vec::new(),
            param_tys: Vec::new(),
            body: Arc::from(Vec::<Stmt>::new()),
            env: globals.clone(),
        });
        let mut machine = Machine::new(&mut interp);
        machine
            .push_frame(closure, globals, Span::default())
            .unwrap();
        machine.kont.push(Cont::FrameBoundary {
            call_span: Span::default(),
        });
        let err = machine
            .deliver(Done::plain(Ctl::Throw(Value::Int(1))))
            .err()
            .unwrap();
        assert_eq!(err.code, codes::THROWN);
        assert!(machine.frames.is_empty());
    }

    #[test]
    fn unary_neg_i64_min_overflows() {
        // `-(-9223372036854775808)` negates i64::MIN -> E4013, same as the
        // recursive `checked_neg`.
        let operand = Expr::Unary(
            UnOp::Neg,
            Arc::new(Expr::Lit(Lit::Int(i64::MIN), Span::default())),
            Span::default(),
        );
        let e = Expr::Unary(UnOp::Neg, Arc::new(operand), Span::default());
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::OVERFLOW);
        assert_eq!(err.message, "integer overflow");
    }

    #[test]
    fn unary_not_uses_truthiness() {
        for (v, expected) in [
            (Lit::Int(0), true),
            (Lit::Int(3), false),
            (Lit::Bool(true), false),
            (Lit::None, true),
            (Lit::Str(String::new()), true),
            (Lit::Str("x".to_string()), false),
        ] {
            let e = Expr::Unary(
                UnOp::Not,
                Arc::new(Expr::Lit(v, Span::default())),
                Span::default(),
            );
            match run_expr(&e).unwrap() {
                Ctl::Val(Value::Bool(b)) => assert_eq!(b, expected),
                _ => panic!("`not` must yield bool"),
            }
        }
    }

    #[test]
    fn unary_bitnot_requires_int() {
        let e = Expr::Unary(
            UnOp::BitNot,
            Arc::new(Expr::Lit(Lit::Float(1.5), Span::default())),
            Span::default(),
        );
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::TYPE_MISMATCH);
        assert_eq!(err.message, "operator `~` requires an integer, found float");
    }

    #[test]
    fn unary_operand_evaluated_once_and_first() {
        // `- (side-effect)` style is not expressible here, but the operand's
        // own control signal must propagate without the operator being applied:
        // `-(return 5)` yields the `return` signal, never `Val(-5)`.
        let operand = Expr::Unary(
            UnOp::Not, // would produce bool if applied
            Arc::new(Expr::Lit(Lit::Int(0), Span::default())),
            Span::default(),
        );
        let e = Expr::Unary(UnOp::Neg, Arc::new(operand), Span::default());
        // `-not 0` is `-true` -> E3001 (cannot negate bool); this proves the
        // operator sees the operand's *value* (`bool`), not its source.
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::TYPE_MISMATCH);
        assert_eq!(err.message, "cannot negate bool");
    }

    #[test]
    fn deep_unary_chain_is_stack_safe() {
        // A chain of `not` operators just below `MAX_AST_DEPTH` must evaluate
        // without consuming the Rust call stack proportionally: the machine
        // pushes one `Cont` per operator and loops. `not` on a final `true`
        // flips parsimoniously, so the expected result depends only on parity.
        let depth = MAX_AST_DEPTH - 2;
        let mut e = Expr::Lit(Lit::Bool(true), Span::default());
        for _ in 0..depth {
            e = Expr::Unary(UnOp::Not, Arc::new(e), Span::default());
        }
        let expected = depth % 2 == 0; // even number of `not` -> true
        match run_expr(&e).unwrap() {
            Ctl::Val(Value::Bool(b)) => assert_eq!(b, expected),
            _ => panic!("deep unary chain must yield bool"),
        }
    }

    #[test]
    fn unary_chain_beyond_ast_depth_is_e1015() {
        // One past the guard must produce E1015, exactly like the recursive
        // evaluator's AST-depth guard.
        let mut e = Expr::Lit(Lit::Bool(true), Span::default());
        for _ in 0..(MAX_AST_DEPTH + 5) {
            e = Expr::Unary(UnOp::Not, Arc::new(e), Span::default());
        }
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::NESTING);
    }

    #[test]
    fn machine_type_sizes_are_bounded() {
        eprintln!(
            "size_of::<Machine>()    = {}",
            std::mem::size_of::<Machine>()
        );
        eprintln!("size_of::<Ctrl>()       = {}", std::mem::size_of::<Ctrl>());
        eprintln!("size_of::<Cont>()       = {}", std::mem::size_of::<Cont>());
        eprintln!(
            "size_of::<UserFrame>()  = {}",
            std::mem::size_of::<UserFrame>()
        );
        eprintln!("size_of::<Done>()       = {}", std::mem::size_of::<Done>());
        assert!(std::mem::size_of::<Cont>() < 1024, "Cont variant inflated");
        assert!(std::mem::size_of::<Ctrl>() < 1024, "Ctrl variant inflated");
    }

    // ----- B-1R3B.2 binary operators ------------------------------------

    fn bin(op: BinOp, l: Expr, r: Expr) -> Expr {
        Expr::Binary(op, Arc::new(l), Arc::new(r), Span::default())
    }

    #[test]
    fn binary_arithmetic_and_overflow() {
        // `2 + 3 * 4` = 14, left-to-right over the parsed tree.
        let e = bin(BinOp::Add, lit(2), bin(BinOp::Mul, lit(3), lit(4)));
        assert!(matches!(run_expr(&e).unwrap(), Ctl::Val(Value::Int(14))));

        // `i64::MAX + 1` -> E4013 integer overflow.
        let e = bin(
            BinOp::Add,
            Expr::Lit(Lit::Int(i64::MAX), Span::default()),
            lit(1),
        );
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::OVERFLOW);
        assert_eq!(err.message, "integer overflow");

        // `1 / 0` -> E4007 division by zero.
        let e = bin(BinOp::Div, lit(1), lit(0));
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::DIV_ZERO);
        assert_eq!(err.message, "division by zero");
    }

    #[test]
    fn binary_comparison_and_equality() {
        assert!(matches!(
            run_expr(&bin(BinOp::Eq, lit(1), lit(1))).unwrap(),
            Ctl::Val(Value::Bool(true))
        ));
        assert!(matches!(
            run_expr(&bin(BinOp::Lt, lit(1), lit(2))).unwrap(),
            Ctl::Val(Value::Bool(true))
        ));
        // A genuinely incomparable pair is a type error (E3001).
        let e = bin(
            BinOp::Lt,
            Expr::Lit(Lit::Bool(true), Span::default()),
            Expr::Lit(Lit::Bool(false), Span::default()),
        );
        // bool < bool is *comparable* in Aura (comparable_with), so this is a
        // bool result, not an error. The error case is a mixed incomparable
        // pair such as none < none.
        assert!(matches!(run_expr(&e).unwrap(), Ctl::Val(Value::Bool(_))));
        let e = bin(
            BinOp::Lt,
            Expr::Lit(Lit::None, Span::default()),
            Expr::Lit(Lit::None, Span::default()),
        );
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::TYPE_MISMATCH);
        assert_eq!(err.message, "cannot compare none with none");
    }

    #[test]
    fn binary_left_signal_skips_right() {
        // `(return 5) + (1 / 0)`: the left `return` signal must propagate
        // without evaluating the right operand, so no E4007 is produced and no
        // operator is applied. A binary expression cannot contain `return`
        // syntactically here, so model it with a nested block that returns.
        let left = Expr::Block(
            Arc::from(vec![Stmt::Return(Some(lit(5)), Span::default())]),
            Span::default(),
        );
        let e = bin(BinOp::Add, left, bin(BinOp::Div, lit(1), lit(0)));
        match run_expr(&e).unwrap() {
            Ctl::Return(Value::Int(5)) => {}
            _ => panic!("left control signal must propagate"),
        }
    }

    #[test]
    fn deep_binary_chain_is_stack_safe() {
        // A left-nested `+` chain just below `MAX_AST_DEPTH` must evaluate
        // without consuming the Rust call stack per node.
        let depth = MAX_AST_DEPTH - 2;
        let mut e = lit(1);
        for _ in 0..depth {
            e = bin(BinOp::Add, e, lit(1));
        }
        match run_expr(&e).unwrap() {
            Ctl::Val(Value::Int(v)) => assert_eq!(v, depth as i64 + 1),
            _ => panic!("deep binary chain must yield int"),
        }
    }

    #[test]
    fn binary_chain_beyond_ast_depth_is_e1015() {
        let mut e = lit(1);
        for _ in 0..(MAX_AST_DEPTH + 5) {
            e = bin(BinOp::Add, e, lit(1));
        }
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::NESTING);
    }

    #[test]
    fn short_circuit_operators_remain_unsupported() {
        for op in [BinOp::And, BinOp::Or] {
            let e = bin(
                op,
                Expr::Lit(Lit::Bool(true), Span::default()),
                Expr::Lit(Lit::Bool(false), Span::default()),
            );
            let err = run_expr(&e).err().unwrap();
            assert_eq!(err.code, codes::INTERNAL);
            assert!(err
                .message
                .contains("not supported by the iterative engine"));
        }
    }
}
