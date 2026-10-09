//! Owned, resumable execution sessions (D1, `PLAYGROUND_032_CAMPAIGN.md`).
//!
//! A [`RunSession`] drives a module through the explicit-continuation machine
//! and *parks* whenever a Host reports a pending effect (today: an HTTP
//! request). The caller performs the effect out of band — in the Playground, a
//! Web Worker calls `fetch` — and feeds the result back with
//! [`RunSession::resume_effect`]. The machine continues from exactly the
//! continuation it suspended on: no program restart, no re-evaluation of the
//! request's arguments, no duplicated side effects.
//!
//! ## Ownership (why there is no self-reference)
//!
//! The machine driver ([`session_impl::Session`]) owns only the machine state
//! (focus, continuations, frames). The interpreter is passed to each drive call
//! and borrowed only for that call. A `RunSession` therefore holds `&mut
//! Interp` and the driver as *siblings*, never one inside the other, so nothing
//! ever refers to itself and a suspension survives across calls.
//!
//! ## Suspension contract
//!
//! A pending effect:
//!
//! * is *parked*, never turned into a diagnostic — the program has not failed;
//! * does not trigger `catch` and does not run `finally` early (the try
//!   machinery lives in the continuation stack, untouched by parking);
//! * preserves every frame, environment, span, recursion counter, and pending
//!   continuation;
//! * resumes at the exact native continuation that requested it, so HTTP
//!   arguments are not re-evaluated and no preceding `print` repeats.
//!
//! A resumed effect that *failed* is delivered to the same continuation as an
//! error, so `try`/`catch`/`finally` see it exactly as a synchronous
//! `Host::http_request` failure. Native and browser semantics stay identical
//! for the *language*; only the host transport differs.

use super::iterative::{Ctrl, RunOutcome, Session as Driver};
use super::value::Value;
use super::{Ctl, Diag, Interp, Span};
use crate::ast::{Expr, Item, Module};
use crate::error::codes;

/// One resumable execution: the interpreter plus the owned machine driver.
pub struct RunSession {
    interp: Interp,
    driver: Driver,
    /// Top-level initializers not yet run, in source order.
    queue: Vec<GlobalItem>,
    /// Whether `main` still has to run.
    main_pending: bool,
    /// A value/error waiting to be fed to the driver on the next step.
    resume: Option<Feed>,
    /// The program finished.
    done: bool,
}

/// A top-level initializer the session runs before `main`.
enum GlobalItem {
    /// `const NAME = expr`.
    Const { name: String, value: Expr },
    /// A bare top-level expression.
    Expr(Expr),
}

/// A completed effect fed back into the driver.
enum Feed {
    /// The effect produced a value.
    Value(Value),
    /// The effect failed with a diagnostic.
    Error(Diag),
}

/// The observable outcome of one `advance`.
#[derive(Debug)]
pub enum Step {
    /// The program completed.
    Completed,
    /// The machine parked on a Host effect the caller must perform.
    Pending(crate::host::PendingEffect),
    /// The machine failed with a diagnostic.
    Diagnostic(Diag),
}

impl RunSession {
    /// Build and prime a resumable session over `module`, taking ownership of
    /// `interp`.
    ///
    /// The session owns the interpreter for its whole life, so a caller (a
    /// WASM host, a test) can park it in a slot and resume it across calls
    /// without a self-reference. It runs the same module semantics as
    /// [`Interp::run_iterative`] but returns control at each pending Host
    /// effect.
    ///
    /// # Errors
    /// Returns a diagnostic if declarations fail.
    pub fn start(mut interp: Interp, module: &Module) -> crate::error::Result<RunSession> {
        // Declarations first (mirrors `run_iterative` pass 1). Declaration is
        // not expression evaluation and cannot suspend, so it runs eagerly.
        for item in &module.items {
            match item {
                Item::Const { .. } | Item::Expr(..) => {}
                other => interp.declare_item(other),
            }
        }
        let mut queue = Vec::new();
        for item in &module.items {
            match item {
                Item::Const { name, value, .. } => queue.push(GlobalItem::Const {
                    name: name.clone(),
                    value: value.clone(),
                }),
                Item::Expr(e, _) => queue.push(GlobalItem::Expr(e.clone())),
                _ => {}
            }
        }
        let main_pending = interp
            .functions
            .get("main")
            .and_then(|s| s.first())
            .is_some();
        let driver = Driver::new_for(&interp);
        Ok(RunSession {
            interp,
            driver,
            queue,
            main_pending,
            resume: None,
            done: false,
        })
    }

    /// The pending effect, if the session is parked.
    #[must_use]
    pub fn pending(&self) -> Option<&crate::host::PendingEffect> {
        self.driver.pending()
    }

    /// The number of effects this session has performed.
    #[must_use]
    pub fn effect_count(&self) -> u64 {
        self.driver.effect_count()
    }

    /// Whether the session has completed.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Run until the program completes, fails, or parks on an effect.
    pub fn advance(&mut self) -> Step {
        if self.done {
            return Step::Completed;
        }
        // A parked session only advances through an explicit resume.
        if let Some(feed) = self.resume.take() {
            let outcome = match feed {
                Feed::Value(v) => self.driver.resume_value(&mut self.interp, v),
                Feed::Error(d) => self.driver.resume_error(&mut self.interp, d),
            };
            return self.route(outcome);
        }
        if let Some(effect) = self.driver.pending() {
            return Step::Pending(effect.clone());
        }
        // Next queued initializer, else `main`.
        if !self.queue.is_empty() {
            let item = self.queue.remove(0);
            match item {
                GlobalItem::Const { name, value } => {
                    let globals = self.interp.globals.clone();
                    let outcome = self.driver.drive_fresh(
                        &mut self.interp,
                        Ctrl::EvalExpr(std::sync::Arc::new(value), globals),
                    );
                    if let Ok(RunOutcome::Completed(ctl)) = outcome {
                        match self.interp.finish_global(ctl) {
                            Ok(v) => self.interp.globals.define(name, v, false),
                            Err(d) => return Step::Diagnostic(d),
                        }
                        return Step::Completed;
                    }
                    return self.route_parked_or_err(outcome);
                }
                GlobalItem::Expr(e) => {
                    let globals = self.interp.globals.clone();
                    let outcome = self.driver.drive_fresh(
                        &mut self.interp,
                        Ctrl::EvalExpr(std::sync::Arc::new(e), globals),
                    );
                    if let Ok(RunOutcome::Completed(ctl)) = outcome {
                        if let Err(d) = self.interp.finish_global(ctl) {
                            return Step::Diagnostic(d);
                        }
                        return Step::Completed;
                    }
                    return self.route_parked_or_err(outcome);
                }
            }
        }
        if self.main_pending {
            self.main_pending = false;
            let main = self
                .interp
                .functions
                .get("main")
                .and_then(|s| s.first())
                .cloned();
            if let Some(main) = main {
                return match self.driver.start_main(&mut self.interp, main) {
                    Ok(outcome) => self.route(Ok(outcome)),
                    Err(d) => Step::Diagnostic(self.interp.uncaught(d)),
                };
            }
        }
        self.done = true;
        Step::Completed
    }

    /// Queue a completed effect's value for the next `advance` (does not run).
    ///
    /// A caller that drives the loop itself (the WASM ABI) uses this plus
    /// [`RunSession::advance`], so a terminal diagnostic from the feed is
    /// observed exactly once and never overwritten by a second advance.
    ///
    /// # Errors
    /// Returns an internal diagnostic if no effect is pending (a stale or
    /// duplicate resume), leaving the session unchanged.
    pub fn enqueue_effect(&mut self, value: Value) -> Result<(), Diag> {
        if self.done || self.driver.pending().is_none() {
            return Err(Diag::new(
                codes::INTERNAL,
                "resumed a completed or non-parked execution session",
                Span::default(),
            ));
        }
        self.resume = Some(Feed::Value(value));
        Ok(())
    }

    /// Queue a *failed* effect for the next `advance` (does not run).
    ///
    /// # Errors
    /// Returns an internal diagnostic if no effect is pending.
    pub fn enqueue_failure(&mut self, diag: Diag) -> Result<(), Diag> {
        if self.done || self.driver.pending().is_none() {
            return Err(Diag::new(
                codes::INTERNAL,
                "failed an effect on a completed or non-parked execution session",
                Span::default(),
            ));
        }
        self.resume = Some(Feed::Error(diag));
        Ok(())
    }

    /// Feed a completed effect's value and continue (queue + one `advance`).
    ///
    /// A resume with no pending effect (a stale completion, or a duplicate
    /// resume after the run finished) is rejected with an internal diagnostic
    /// rather than silently ignored or replayed.
    pub fn resume_effect(&mut self, value: Value) -> Step {
        if let Err(d) = self.enqueue_effect(value) {
            return Step::Diagnostic(d);
        }
        self.advance()
    }

    /// Feed a *failed* effect and continue (queue + one `advance`). The
    /// diagnostic reaches the parked continuation, so `try`/`catch`/`finally`
    /// see it.
    pub fn fail_effect(&mut self, diag: Diag) -> Step {
        if let Err(d) = self.enqueue_failure(diag) {
            return Step::Diagnostic(d);
        }
        self.advance()
    }

    /// Route an outcome that is not `Completed` (the caller consumed the
    /// `Completed` case to finish a global). `Err` and `Parked` are the only
    /// remaining shapes.
    fn route_parked_or_err(&mut self, outcome: crate::error::Result<RunOutcome>) -> Step {
        match outcome {
            Ok(RunOutcome::Completed(_)) => Step::Completed,
            Ok(RunOutcome::Parked(effect)) => Step::Pending(effect),
            Err(d) => {
                self.done = true;
                Step::Diagnostic(d)
            }
        }
    }

    /// Translate a driver outcome into a step.
    fn route(&mut self, outcome: crate::error::Result<RunOutcome>) -> Step {
        match outcome {
            Ok(RunOutcome::Completed(_)) => {
                self.done = self.queue.is_empty() && !self.main_pending;
                Step::Completed
            }
            Ok(RunOutcome::Parked(effect)) => Step::Pending(effect),
            Err(d) => {
                self.done = true;
                Step::Diagnostic(d)
            }
        }
    }
}

/// Marker so an impossible internal state is never silently ignored.
#[allow(dead_code)]
fn internal(msg: &'static str) -> Diag {
    Diag::new(codes::INTERNAL, msg, Span::default())
}

/// A no-op reference to `Ctl` kept for the driver's public surface stability.
#[allow(dead_code)]
type CompletedCtl = Ctl;
