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
use crate::source::SourceId;

/// One resumable execution: the interpreter plus the owned machine driver.
pub struct RunSession {
    interp: Interp,
    driver: Driver,
    /// Top-level initializers not yet run, in source order.
    queue: Vec<GlobalItem>,
    /// The initializer that parked, if any. It is removed from `queue` so a
    /// resume finishes its binding instead of re-driving it (re-driving would
    /// repeat the side effects it already performed).
    parked_item: Option<GlobalItem>,
    /// Whether `main` still has to run.
    main_pending: bool,
    /// A value/error waiting to be fed to the driver on the next step.
    resume: Option<Feed>,
    /// The program finished.
    done: bool,
    /// The source owning `main`, for a provider-backed session. `None` for a
    /// single-source session, where the caller already knows the entry source.
    entry_source: Option<SourceId>,
    /// The source owning the most recent `Step::Diagnostic`. `None` for a
    /// single-source session, where the entry source is the only owner.
    diagnostic_source: Option<SourceId>,
}

/// A top-level initializer the session runs before `main`.
struct GlobalItem {
    /// The source that owns this initializer, for provider-backed sessions.
    source: Option<SourceId>,
    /// The initializer itself.
    kind: GlobalKind,
}

/// The two shapes a top-level initializer takes.
enum GlobalKind {
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
    ///
    /// Queued global initializers are internal bookkeeping, so
    /// [`RunSession::advance`] drains them without returning; a caller that
    /// observes `Completed` can trust the program really finished (and
    /// [`RunSession::is_done`] agrees).
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
    pub fn start(interp: Interp, module: &Module) -> crate::error::Result<RunSession> {
        Ok(Self::start_with_provenance(interp, module, None))
    }

    /// Build and prime a resumable session that retains per-item source
    /// ownership (the multi-source counterpart of [`RunSession::start`]).
    ///
    /// A provider-backed compilation assembles its module from several sources,
    /// so a runtime diagnostic must still name the file that owns the failing
    /// item. `item_sources` carries that one-to-one ownership, exactly as
    /// [`Interp::run_iterative_sourced`] does on the synchronous path.
    ///
    /// # Errors
    /// Returns a diagnostic if declarations fail, or an internal diagnostic if
    /// `item_sources` does not pair with `module.items`.
    pub fn start_sourced(
        interp: Interp,
        module: &Module,
        item_sources: &[SourceId],
        entry_source: SourceId,
    ) -> crate::error::Result<RunSession> {
        if module.items.len() != item_sources.len() {
            return Err(Diag::locationless(
                codes::INTERNAL,
                "runtime source provenance does not match the resolved module",
            ));
        }
        Ok(Self::start_with_provenance(
            interp,
            module,
            Some((item_sources, entry_source)),
        ))
    }

    /// The shared constructor: single-source (`sources == None`) or
    /// provenance-retaining.
    fn start_with_provenance(
        mut interp: Interp,
        module: &Module,
        provenance: Option<(&[SourceId], SourceId)>,
    ) -> RunSession {
        // Declarations first (mirrors `run_iterative` pass 1). Declaration is
        // not expression evaluation and cannot suspend, so it runs eagerly.
        // A sourced session records each closure's owning file so a later call
        // into it attributes its diagnostics correctly.
        match provenance {
            Some((sources, _)) => {
                for (item, source) in module.items.iter().zip(sources.iter().copied()) {
                    match item {
                        Item::Const { .. } | Item::Expr(..) => {}
                        other => interp.declare_item_with_source(other, Some(source)),
                    }
                }
            }
            None => {
                for item in &module.items {
                    match item {
                        Item::Const { .. } | Item::Expr(..) => {}
                        other => interp.declare_item(other),
                    }
                }
            }
        }
        let mut queue = Vec::new();
        for (index, item) in module.items.iter().enumerate() {
            let source = provenance.map(|(sources, _)| sources[index]);
            let kind = match item {
                Item::Const { name, value, .. } => GlobalKind::Const {
                    name: name.clone(),
                    value: value.clone(),
                },
                Item::Expr(e, _) => GlobalKind::Expr(e.clone()),
                _ => continue,
            };
            queue.push(GlobalItem { source, kind });
        }
        let main_pending = interp
            .functions
            .get("main")
            .and_then(|s| s.first())
            .is_some();
        let driver = Driver::new_for(&interp);
        RunSession {
            interp,
            driver,
            queue,
            parked_item: None,
            main_pending,
            resume: None,
            done: false,
            entry_source: provenance.map(|(_, entry)| entry),
            diagnostic_source: None,
        }
    }

    /// The source owning the most recent [`Step::Diagnostic`], for a session
    /// built with [`RunSession::start_sourced`].
    #[must_use]
    pub const fn diagnostic_source(&self) -> Option<SourceId> {
        self.diagnostic_source
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
            if let Some(item) = self.parked_item.take() {
                match outcome {
                    // The initializer finished on resume: complete its binding
                    // and keep going. Falling back into `drain` instead would
                    // re-drive the expression and repeat the work it already
                    // performed before parking.
                    Ok(RunOutcome::Completed(ctl)) => {
                        if let Err(step) = self.finish_item(&item, ctl) {
                            return step;
                        }
                    }
                    Ok(RunOutcome::Parked(effect)) => {
                        self.parked_item = Some(item);
                        return Step::Pending(effect);
                    }
                    Err(d) => {
                        let fallback = item.source;
                        return self.fail_sourced(d, fallback);
                    }
                }
            } else {
                // `main` was parked, so the queue is empty: the outcome is
                // either genuine termination or a further park.
                return self.route(outcome);
            }
        }
        if let Some(effect) = self.driver.pending() {
            return Step::Pending(effect.clone());
        }
        self.drain()
    }

    /// Run the queued initializers, then `main`.
    ///
    /// An initializer is internal bookkeeping, not a stopping point: the loop
    /// continues past one that finishes, so a caller observing `Completed` can
    /// trust the program really finished. A park leaves the in-flight item in
    /// [`RunSession::parked_item`] for [`RunSession::advance`] to complete.
    fn drain(&mut self) -> Step {
        loop {
            if !self.queue.is_empty() {
                let item = self.queue.remove(0);
                // A sourced session attributes the initializer to its own file, so
                // a diagnostic raised while evaluating it names the right one.
                // Cleared first, mirroring `run_iterative_sourced`.
                self.interp.set_current_source(item.source);
                self.interp.clear_error_source();
                let expr = match &item.kind {
                    GlobalKind::Const { value, .. } => value.clone(),
                    GlobalKind::Expr(e) => e.clone(),
                };
                let globals = self.interp.globals.clone();
                let outcome = self.driver.drive_fresh(
                    &mut self.interp,
                    Ctrl::EvalExpr(std::sync::Arc::new(expr), globals),
                );
                match outcome {
                    Ok(RunOutcome::Completed(ctl)) => {
                        if let Err(step) = self.finish_item(&item, ctl) {
                            return step;
                        }
                        continue;
                    }
                    Ok(RunOutcome::Parked(effect)) => {
                        self.parked_item = Some(item);
                        return Step::Pending(effect);
                    }
                    Err(d) => {
                        let fallback = item.source;
                        return self.fail_sourced(d, fallback);
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
                    match self.driver.start_main(&mut self.interp, main) {
                        Ok(RunOutcome::Completed(_)) => continue,
                        Ok(RunOutcome::Parked(effect)) => return Step::Pending(effect),
                        Err(d) => {
                            let uncaught = self.interp.uncaught(d);
                            return self.fail_sourced(uncaught, self.entry_source);
                        }
                    }
                }
            }
            self.done = true;
            return Step::Completed;
        }
    }

    /// Turn a finished initializer's control value into its binding.
    ///
    /// A `const` keeps its value; a bare expression discards it, exactly as
    /// [`Interp::run_iterative_sourced`] does. `finish_global` normalizes an
    /// escaping signal, so a failure here is already user-facing.
    fn finish_item(&mut self, item: &GlobalItem, ctl: Ctl) -> Result<(), Step> {
        let name = match &item.kind {
            GlobalKind::Const { name, .. } => Some(name.as_str()),
            GlobalKind::Expr(_) => None,
        };
        match self.interp.finish_global(ctl) {
            Ok(value) => {
                if let Some(name) = name {
                    self.interp.globals.define(name, value, false);
                }
                Ok(())
            }
            Err(d) => Err(self.fail_sourced(d, item.source)),
        }
    }

    /// Record that `d` came from `fallback` and turn it into a step.
    ///
    /// The raise site wins when it was captured (`uncaught` records it for a
    /// re-raised signal), exactly as `run_iterative_sourced` decides ownership.
    fn fail_sourced(&mut self, d: Diag, fallback: Option<SourceId>) -> Step {
        let owner = self.interp.take_error_source().or(fallback);
        self.fail(d, owner)
    }

    /// Record the owner at the end of the session without consulting it.
    fn fail(&mut self, d: Diag, owner: Option<SourceId>) -> Step {
        self.done = true;
        self.diagnostic_source = owner;
        Step::Diagnostic(d)
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

    /// Translate the outcome of resuming `main`.
    ///
    /// Only `main` can park without leaving an item in
    /// [`RunSession::parked_item`], so a resume that lands here has nothing
    /// queued and cannot resume a second time: `Completed` is genuine
    /// termination.
    fn route(&mut self, outcome: crate::error::Result<RunOutcome>) -> Step {
        match outcome {
            Ok(RunOutcome::Completed(_)) => {
                debug_assert!(self.queue.is_empty() && !self.main_pending);
                self.done = true;
                Step::Completed
            }
            Ok(RunOutcome::Parked(effect)) => Step::Pending(effect),
            Err(d) => {
                let uncaught = self.interp.uncaught(d);
                let entry = self.entry_source;
                self.fail_sourced(uncaught, entry)
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
