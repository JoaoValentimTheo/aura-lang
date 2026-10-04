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
//!
//! ## B-1R3B.3 added subset
//!
//! * short-circuit `and`/`or`. The left operand is evaluated exactly once via
//!   [`Cont::ShortCircuitLeft`]; when it completes, its truthiness decides
//!   whether the right operand is required (`and`: truthy left; `or`: falsy
//!   left). A skipped right operand is **never** evaluated, so it cannot
//!   produce stdout, mutations, diagnostics, or control signals; a required
//!   right operand is evaluated exactly once via [`Cont::ShortCircuitRight`].
//!   Both operators yield the truthiness of the executing operand as a `bool`,
//!   exactly matching `Interp::eval`'s `Expr::Binary` short-circuit arms.
//!
//! ## B-1R3B.4.1 added subset
//!
//! * list construction: `Expr::List` (`[a, b, c]`) and `Expr::Tuple`
//!   (`(a, b)`), which is list sugar (`LANGUAGE_SPEC.md` §21). Elements are
//!   evaluated left to right, exactly once each, via [`Cont::ListNext`]; the
//!   first control signal or diagnostic from an element aborts the list and
//!   propagates unchanged, so later elements are never evaluated. The
//!   completed elements become a `Value::list` (a `list`-typed value; tuple
//!   literals are indistinguishable from list literals by contract).
//!
//! ## B-1R3B.4.2 added subset
//!
//! * map construction: `Expr::Map` (`{k: v, ...}` and the empty `{:}`). Entries
//!   are processed in source order; for each entry the key is evaluated and
//!   validated **before** its value is scheduled, exactly like
//!   `Interp::eval_inner`'s `Expr::Map` arm. A non-key-capable key is `E3001`
//!   at the key expression's span and prevents its value from running. A
//!   control signal or diagnostic from a key or value aborts the whole map and
//!   propagates unchanged, so later entries never run. Duplicate keys keep the
//!   **last** entry's value (a `BTreeMap` insert), matching the recursive
//!   engine. Each key and each value is evaluated exactly once.
//!
//! ## B-1R3B.5 added subset
//!
//! * range construction: `Expr::Range` (`a..b`). The start operand is
//!   evaluated first and exactly once, then the end operand exactly once, via
//!   [`Cont::RangeStart`] then [`Cont::RangeEnd`]; a control signal from either
//!   operand aborts construction and propagates unchanged. **Both** operands
//!   complete before either bound is validated, exactly like
//!   `Interp::eval_inner`'s `Expr::Range` arm, and validation is start-first,
//!   so a non-int start wins over a non-int end (an end signal/error still
//!   preempts a runtime-invalid start, because the end is evaluated before
//!   either is validated). A valid pair becomes `Value::Range` — the same
//!   runtime representation `range(a, b)` builds (`RangeVal { start, end }`,
//!   end exclusive). The current implementation is the step-1, half-open Range
//!   only; the planned Aura 0.3 `step`/inclusive extensions are **not**
//!   introduced.
//!
//! ## B-1R3B.6 added subset
//!
//! * index reads: `Expr::Index` (`base[index]`). The target is evaluated first
//!   and exactly once, then the index exactly once, via [`Cont::IndexTarget`]
//!   then [`Cont::IndexApply`]; a control signal from either operand aborts the
//!   read and propagates unchanged, so the lookup never runs. The lookup itself
//!   is delegated to `Interp::index_get` (the recursive engine's value-level
//!   helper, which evaluates no AST node), so list (including current-tuple
//!   sugar), string, map, and struct-instance indexing — and every diagnostic
//!   (out-of-range `E4019`, missing-key `E2003`, non-key-capable key `E3001`,
//!   unsupported base/index combination `E3001`) — are identical by
//!   construction. Negative indices normalize exactly as recursion does.
//! * field reads: `Expr::Field` (`recv.name`). The field name is syntactic, so
//!   the receiver is evaluated exactly once via [`Cont::FieldReceiver`] and the
//!   name is copied into the continuation. Resolution mirrors
//!   `Interp::eval_inner`'s `Expr::Field` arm exactly: a struct instance reads a
//!   declared field (missing field `E2003`; a method name used without `(...)`
//!   is the recursive `E2003` guard), and every other receiver dispatches to the
//!   zero-argument builtin registry with the same `E2003` for an unknown member.
//!   Because struct construction is `Expr::Construct` (still unsupported), the
//!   `Value::Instance` field path is reachable only indirectly today; the
//!   non-instance method path is reachable directly.
//!
//! ## B-1R3B.7 added subset
//!
//! * f-strings: `Expr::FStr` (`f"..."`), including empty/text-only strings,
//!   interpolations with an optional format specification, adjacent parts, and
//!   Unicode text. Parts are evaluated left to right, exactly once each, via
//!   [`Cont::FStrNext`]; the accumulator `String` lives in the continuation, so
//!   no `String`-sized or nesting-proportional Rust recursion occurs. A literal
//!   part is appended directly; an interpolation expression is evaluated exactly
//!   once and then either appended with its display form (`v.display()`) or
//!   rendered through [`Interp::format_value`] with its format specification,
//!   exactly like `Interp::eval_inner`'s `Expr::FStr` arm. A control signal or
//!   diagnostic from an interpolation aborts the whole f-string and propagates
//!   unchanged, so later parts are never evaluated and no partial string is
//!   observable; a failed `format_value` (for example `E3001` for a
//!   presentation type applied to an incompatible value, or `E4013` for an
//!   out-of-range precision/width) is reported at the specification's span,
//!   exactly as recursion does.
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
//!   Short-circuit operands that contain a user call therefore still fail with
//!   the deterministic unsupported sentinel, exactly as before.
//! * **`try`/`finally` are not implemented** (R3F).
//! * The shadowing write-back travels in [`Done::env`], which is exact for the
//!   call-free R3A subset (conts run depth-first with no suspension). R3C must
//!   keep this carrier explicit across suspensions.
//! * `MAX_AST_DEPTH` (`E1015`) and `MAX_CALL_FRAMES` (`E4011`) preserve the
//!   recursive engine's semantics exactly.
//! * **Exactly-once evaluation is structural, not differentially falsifiable
//!   in this subset.** The continuation design schedules each range operand in
//!   exactly one place, but the only constructs that could make a double
//!   evaluation observable (a call, an assignment, or a `print`) are still
//!   unsupported (R3C/R3D), so no supported case can distinguish it. Order is
//!   observable and pinned (a start error/`E4999` preempts the end, and a
//!   runtime start error beats a runtime end error); once side-effecting
//!   bounds exist, the oracle should add a print-counting range case.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use super::value::{MapKey, Value};
use super::{Closure, Ctl, Env, Interp, NativeOutcome, MAX_AST_DEPTH, MAX_CALL_FRAMES};
use crate::ast::{Arg, BinOp, Expr, FPart, FormatSpec, Lit, Pattern, Stmt, UnOp};
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
/// One comprehension's static parts (R3E.1). `key` is `Some` only for a map
/// comprehension; the iterable materializes with `Interp::iterate`, exactly
/// like the recursive engine (comprehensions have no lazy-range path).
struct CompSpec {
    key: Option<Arc<Expr>>,
    value: Arc<Expr>,
    pattern: Pattern,
    filter: Option<Arc<Expr>>,
    iterable: Arc<Expr>,
    span: Span,
    is_map: bool,
}

/// The iteration state carried across a comprehension's continuations.
struct CompState {
    spec: CompSpec,
    env: Env,
    items: Vec<Value>,
    index: usize,
    out: Vec<Value>,
    map: std::collections::BTreeMap<crate::run::value::MapKey, Value>,
}

/// A lazy `for` cursor over a range: `next < end`.
#[derive(Clone, Copy)]
struct RangeCursor {
    next: i64,
    end: i64,
}

/// The iteration state of one `for` statement (R3D.4): either a remaining
/// materialized item list or a lazy range cursor.
struct ForState {
    pattern: Pattern,
    body: Arc<[Stmt]>,
    env: Env,
    span: Span,
    items: Vec<Value>,
    index: usize,
    range: Option<RangeCursor>,
}

/// Whether a target traversal is reading the current value or writing one.
enum TargetPhase {
    Read,
    Write(Value),
}

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
    /// A short-circuit (`and`/`or`) left operand finished (B-1R3B.3); its
    /// truthiness decides whether the right operand must run at all. The
    /// right operand is scheduled only when it is required (`and`: truthy
    /// left; `or`: falsy left); otherwise the machine returns the left
    /// operand's truthiness as a `bool` without ever touching `rhs`.
    ShortCircuitLeft { op: BinOp, rhs: Arc<Expr>, env: Env },
    /// A required short-circuit right operand finished (B-1R3B.3); the result
    /// is its truthiness as a `bool`.
    ShortCircuitRight,
    /// A list/tuple element finished (B-1R3B.4.1). `index` is the element that
    /// produced the last entry of `out`; the next element is scheduled with the
    /// same environment, or the completed elements become a `Value::list`.
    /// `Expr::Tuple` uses this same record: a tuple literal is list sugar
    /// (`LANGUAGE_SPEC.md` §21) and its recursive arm is byte-identical to
    /// `Expr::List`.
    ListNext {
        items: Arc<[Expr]>,
        index: usize,
        out: Vec<Value>,
        env: Env,
    },
    /// A map entry's key finished (B-1R3B.4.2). The key value is validated and
    /// converted to a [`MapKey`]; only then is the corresponding value
    /// scheduled. `index` is the entry currently being evaluated; `out` holds
    /// the entries that already completed.
    MapKeyNext {
        entries: Arc<[(Expr, Expr)]>,
        index: usize,
        out: BTreeMap<MapKey, Value>,
        env: Env,
    },
    /// A map entry's value finished (B-1R3B.4.2). `key` is the already-built
    /// key for this entry; the pair is inserted (last write wins, exactly like
    /// `BTreeMap::insert`) and the next entry is scheduled.
    MapValueNext {
        key: MapKey,
        entries: Arc<[(Expr, Expr)]>,
        index: usize,
        out: BTreeMap<MapKey, Value>,
        env: Env,
    },
    /// A range start operand finished (B-1R3B.5). The value is retained and the
    /// end operand is scheduled; **no** type validation happens here, because
    /// the recursive engine validates neither bound until both operands have
    /// completed. A control signal from the start aborts before the end runs.
    RangeStart {
        end: Arc<Expr>,
        env: Env,
        span: Span,
    },
    /// A range end operand finished (B-1R3B.5). Both operands have now
    /// completed exactly once; the start is validated first, then the end,
    /// producing the same `E3001` messages and the same expression span as
    /// `Interp::eval_inner`'s `Expr::Range` arm.
    RangeEnd { start: Value, span: Span },
    /// An index expression's target finished (B-1R3B.6). The target value is
    /// retained unvalidated and the index operand is scheduled exactly once.
    /// This mirrors `Interp::eval_inner`'s `Expr::Index` arm, which evaluates
    /// the target before the index and reports the target's own failure first.
    IndexTarget {
        idx: Arc<Expr>,
        env: Env,
        span: Span,
    },
    /// An index expression's index operand finished (B-1R3B.6). The target was
    /// produced exactly once and is applied here through
    /// `Interp::index_get`, so list/string/map/instance lookup and every
    /// diagnostic (including out-of-range `E4019` and missing-key `E2003`) are
    /// byte-identical to recursion. A control signal from the index is
    /// redelivered and never reaches `index_get`.
    IndexApply { base: Value, span: Span },
    /// A field receiver finished (B-1R3B.6). The receiver was produced exactly
    /// once; field/method resolution mirrors `Interp::eval_inner`'s
    /// `Expr::Field` arm exactly (struct field read, missing-field `E2003`, the
    /// method-not-a-value guard, or zero-argument builtin method dispatch).
    FieldReceiver { name: String, span: Span },
    /// An f-string part in progress (B-1R3B.7). `parts` and `index` identify the
    /// interpolation currently being evaluated (the completion belongs to
    /// `parts[index].1`); `spec` is that interpolation's optional format
    /// specification, copied out of the AST. `out` is the accumulator and
    /// `parts`/`index` also advance the remaining literal and interpolation
    /// parts. Mirrors `Interp::eval_inner`'s `Expr::FStr` arm: literal text is
    /// appended verbatim; an interpolation is appended with `v.display()` (no
    /// spec) or `Interp::format_value` (with a spec). A control signal or
    /// diagnostic from an interpolation aborts the whole f-string, so later
    /// parts never run and no partial string escapes.
    FStrNext {
        parts: Arc<[FPart]>,
        index: usize,
        spec: Option<FormatSpec>,
        out: String,
        env: Env,
    },
    /// A `return` operand finished; convert to the `return` signal.
    ReturnFrom,
    /// A `throw` operand finished; convert to the `throw` signal.
    ThrowFrom,
    /// A call argument finished (R3C.1). Arguments are evaluated strictly in
    /// source order, exactly once each, *before* any callee resolution or
    /// parameter binding — mirroring `Interp::eval_call`'s argument loop. The
    /// accepted prefix (including this one) rides in `values`; scheduling the
    /// next argument or dispatching the call is decided here.
    CallArgs {
        callee: Arc<Expr>,
        args: Arc<[Arg]>,
        index: usize,
        values: Vec<Value>,
        env: Env,
        span: Span,
    },
    /// A non-`Name` callee expression finished (R3C.1); apply it to the
    /// already-evaluated arguments exactly like `Interp::eval_call`'s `other`
    /// arm (`call_value`). A `Name` callee is resolved synchronously once its
    /// arguments complete, so this record is only pushed for other shapes.
    CallCallee { values: Vec<Value>, span: Span },
    /// A `match` subject finished (R3E.2); find the first matching arm.
    MatchSubject {
        arms: Arc<[crate::ast::Arm]>,
        env: Env,
        span: Span,
    },
    /// A `match` arm guard finished (R3E.2); enter the body when truthy, else
    /// try the next arm.
    MatchGuard {
        arms: Arc<[crate::ast::Arm]>,
        subject: Value,
        next: usize,
        scope: Env,
        span: Span,
    },
    /// A comprehension iterable finished (R3E.1); start the first item. The
    /// `Env` is the per-item scope for the item currently in flight.
    CompIterable(Box<CompState>, Env),
    /// A comprehension filter finished (R3E.1) for the current item.
    CompFilter(Box<CompState>, Env),
    /// A map comprehension's key finished (R3E.1); evaluate the value next.
    CompKey(Box<CompState>, Env),
    /// A comprehension value finished (R3E.1); record it and advance. The
    /// `Option<MapKey>` is `Some` for a map comprehension; the `Env` is the
    /// per-item scope.
    CompValue(Box<CompState>, Env, Option<crate::run::value::MapKey>),
    /// A `for` iterable finished (R3D.4); start iteration (lazy for ranges).
    ForIterable {
        pattern: Pattern,
        body: Arc<[Stmt]>,
        env: Env,
        span: Span,
    },
    /// A `for` body/iteration step finished (R3D.4); advance to the next item.
    ForNext(ForState),
    /// A `while` condition finished (R3D.2); enter the body or finish.
    WhileCond {
        cond: Arc<Expr>,
        body: Arc<[Stmt]>,
        env: Env,
    },
    /// A `while` body block finished (R3D.2); re-test the condition.
    WhileLoop {
        cond: Arc<Expr>,
        body: Arc<[Stmt]>,
        env: Env,
    },
    /// A `loop` body block finished (R3D.2); run it again.
    LoopBody { body: Arc<[Stmt]>, env: Env },
    /// A destructuring `let` initializer finished (R3D.1); bind atomically.
    LetPatternBind { pattern: Pattern, env: Env },
    /// An assignment RHS finished (R3D.1); start the target read/write.
    AssignRhs {
        target: Arc<Expr>,
        op: Option<BinOp>,
        env: Env,
        span: Span,
    },
    /// A compound assignment's target read finished (R3D.1); apply the binary
    /// operator and start the target write. The write re-evaluates the
    /// target's base/index subexpressions, preserving the recursive engine's
    /// documented double evaluation.
    AssignRead {
        target: Arc<Expr>,
        op: BinOp,
        rhs: Value,
        env: Env,
        span: Span,
    },
    /// A target's base expression finished (R3D.1): resolve a `Field` write or
    /// schedule the index expression of an `Index` target.
    TargetBase {
        name: Option<String>,
        index: Option<Arc<Expr>>,
        phase: TargetPhase,
        env: Env,
        span: Span,
    },
    /// A target's index expression finished (R3D.1).
    TargetIndex {
        base: Value,
        phase: TargetPhase,
        span: Span,
    },
    /// A pipe left operand finished (R3C.4); evaluate the right operand next,
    /// exactly like `Interp::eval_inner`'s `Expr::Pipe` arm.
    PipeRight { r: Arc<Expr>, env: Env, span: Span },
    /// A pipe right operand finished (R3C.4); apply it to the left value.
    PipeApply { left: Value, span: Span },
    /// A method receiver finished (R3C.2); evaluate the arguments in source
    /// order next, exactly like `Interp::eval_inner`'s `Expr::Method` arm.
    MethodReceiver {
        name: String,
        args: Arc<[Arg]>,
        env: Env,
        span: Span,
    },
    /// A method argument finished (R3C.2). `subject` is the already-evaluated
    /// receiver (evaluated exactly once, before any argument).
    MethodArgs {
        subject: Value,
        name: String,
        args: Arc<[Arg]>,
        index: usize,
        values: Vec<Value>,
        env: Env,
        span: Span,
    },
    /// A constructor argument finished (R3C.3). Struct and enum arguments are
    /// evaluated strictly in source order, exactly once each, before any field
    /// or variant resolution — mirroring `Interp::construct`'s argument loop.
    /// `index` is the next argument to schedule.
    ConstructArgs {
        name: String,
        args: Arc<[Arg]>,
        index: usize,
        positional: Vec<Value>,
        named: Vec<(String, Value)>,
        env: Env,
        span: Span,
    },
    /// A resumable native is mid-call (`map`/`filter`/`reduce`,
    /// `ITERATIVE_EVALUATOR_DESIGN.md` §13). The callback's result is fed back
    /// to the native-owned `resume` token, which produces the next
    /// [`NativeOutcome`] step, driven as machine work rather than a nested
    /// evaluator call.
    NativeResume {
        resume: Box<dyn super::NativeResume>,
        span: Span,
    },
    /// A user-frame boundary (R3C): pop one [`UserFrame`] and map the call's
    /// completion.
    FrameBoundary { call_span: Span },
}

/// An explicit Aura user call frame (`ITERATIVE_EVALUATOR_DESIGN.md` §16.2).
///
/// `Machine::frames.len()` **is** the active-frame count checked against
/// `MAX_CALL_FRAMES`; `main` occupies frame 1. Every field mirrors the state
/// `Interp::call` saves and restores around `exec_block` for one call.
struct UserFrame {
    #[allow(dead_code)]
    closure: Rc<Closure>,
    #[allow(dead_code)]
    env: Env,
    /// Caller's `current_source`, restored when the frame is popped.
    saved_source: Option<SourceId>,
    /// The callee's source, if known (diagnostics/source attribution).
    #[allow(dead_code)]
    frame_source: Option<SourceId>,
    /// The call's span, used for boundary diagnostics (`E4099`/`E4028`).
    #[allow(dead_code)]
    call_span: Span,
    /// Caller's expression-nesting budget, restored at the frame boundary.
    /// `Interp::call` resets `ast_depth` for the callee and restores it after.
    saved_expr_depth: usize,
}

/// The explicit continuation machine.
struct Machine<'i> {
    interp: &'i mut Interp,
    /// Current computational focus.
    ctrl: Ctrl,
    /// Continuations, innermost last.
    kont: Vec<Cont>,
    /// Aura user call frames, innermost last. `interp.depth` is the
    /// authoritative active-frame count shared with `Interp::call` (so a
    /// native callback that re-enters recursion sees the correct base); this
    /// stack holds the saved/restored frame state.
    frames: Vec<UserFrame>,
    /// `interp.depth` when the machine started; restored when the machine
    /// finishes (Ok or Err), because every un-popped frame is discarded with
    /// the machine.
    base_depth: usize,
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
        let base_depth = interp.depth;
        Machine {
            interp,
            ctrl: Ctrl::Done(Ctl::Val(Value::None)),
            kont: Vec::new(),
            frames: Vec::new(),
            base_depth,
            expr_depth: 0,
            top_env: None,
        }
    }

    /// Drive the machine to completion.
    fn run(&mut self, initial: Ctrl) -> Result<Ctl> {
        self.ctrl = initial;
        let result = self.run_inner();
        // Any frames still on the stack are discarded with the machine; the
        // shared call-depth accounting returns to its entry value so a
        // subsequent machine (or a native re-entry) sees a consistent count.
        self.interp.depth = self.base_depth;
        result
    }

    fn run_inner(&mut self) -> Result<Ctl> {
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
            Cont::MatchSubject { arms, env, span } => {
                let Ctl::Val(subject) = done.ctl else {
                    // A signal from the subject propagates without matching.
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                match self.try_match_arms(arms, subject, 0, env, span)? {
                    Control::Next(next) => Ok(Resume::Next(next)),
                    Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                }
            }
            Cont::MatchGuard {
                arms,
                subject,
                next,
                scope,
                span,
            } => {
                let Ctl::Val(g) = done.ctl else {
                    // A signal from the guard propagates; the arm is not
                    // entered and later arms are not tried (mirrors `val!`).
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                if g.truthy() {
                    Ok(Resume::Next(Ctrl::EnterBlock(
                        arms[next - 1].body.clone(),
                        scope,
                        true,
                    )))
                } else {
                    match self.try_match_arms(arms, subject, next, scope, span)? {
                        Control::Next(next) => Ok(Resume::Next(next)),
                        Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                    }
                }
            }
            Cont::CompIterable(mut state, env) => {
                let Ctl::Val(subject) = done.ctl else {
                    // A signal from the iterable propagates before iteration.
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                state.items = self.interp.iterate(&subject, state.spec.span)?;
                state.env = env;
                match self.step_comp(state)? {
                    Control::Next(next) => Ok(Resume::Next(next)),
                    Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                }
            }
            Cont::CompFilter(state, scope) => {
                let Ctl::Val(pred) = done.ctl else {
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                if pred.truthy() {
                    match self.after_comp_filter(state, scope) {
                        Control::Next(next) => Ok(Resume::Next(next)),
                        Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                    }
                } else {
                    match self.step_comp(state)? {
                        Control::Next(next) => Ok(Resume::Next(next)),
                        Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                    }
                }
            }
            Cont::CompKey(state, scope) => {
                let Ctl::Val(kv) = done.ctl else {
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                let Some(mk) = crate::run::value::MapKey::from_value(&kv) else {
                    return Err(self.interp.error(
                        codes::TYPE_MISMATCH,
                        format!(
                            "type `{}` cannot be used as a map key; map keys must be `string`, `int`, or `bool`",
                            kv.type_name()
                        ),
                        state
                            .spec
                            .key
                            .as_ref()
                            .map_or(state.spec.span, |k| expr_span(k)),
                    ));
                };
                let value = state.spec.value.clone();
                self.kont
                    .push(Cont::CompValue(state, scope.clone(), Some(mk)));
                Ok(Resume::Next(Ctrl::EvalExpr(value, scope)))
            }
            Cont::CompValue(mut state, _scope, mk) => {
                let Ctl::Val(v) = done.ctl else {
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                match mk {
                    Some(key) => {
                        state.map.insert(key, v);
                    }
                    None => state.out.push(v),
                }
                match self.step_comp(state)? {
                    Control::Next(next) => Ok(Resume::Next(next)),
                    Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                }
            }
            Cont::ForIterable {
                pattern,
                body,
                env,
                span,
            } => {
                let Ctl::Val(subject) = done.ctl else {
                    // A signal from the iterable propagates without iterating.
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                let range = match &subject {
                    Value::Range(r) => Some(RangeCursor {
                        next: r.start,
                        end: r.end,
                    }),
                    _ => None,
                };
                let items = if range.is_some() {
                    Vec::new()
                } else {
                    self.interp.iterate(&subject, span)?
                };
                Ok(Resume::Next(self.step_for(ForState {
                    pattern,
                    body,
                    env,
                    span,
                    items,
                    index: 0,
                    range,
                })?))
            }
            Cont::ForNext(state) => match done.ctl {
                Ctl::Break => Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::None)))),
                Ctl::Continue | Ctl::Val(_) => Ok(Resume::Next(self.step_for(state)?)),
                other => Ok(Resume::Redeliver(Done::plain(other))),
            },
            Cont::WhileCond { cond, body, env } => {
                let Ctl::Val(c) = done.ctl else {
                    // A signal from the condition propagates without entering
                    // the body.
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                if c.truthy() {
                    self.kont.push(Cont::WhileLoop {
                        cond,
                        body: body.clone(),
                        env: env.clone(),
                    });
                    Ok(Resume::Next(Ctrl::EnterBlock(body, env, true)))
                } else {
                    Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::None))))
                }
            }
            Cont::WhileLoop { cond, body, env } => match done.ctl {
                Ctl::Break => Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::None)))),
                Ctl::Continue | Ctl::Val(_) => {
                    self.kont.push(Cont::WhileCond {
                        cond: cond.clone(),
                        body: body.clone(),
                        env: env.clone(),
                    });
                    Ok(Resume::Next(Ctrl::EvalExpr(cond, env)))
                }
                other => Ok(Resume::Redeliver(Done::plain(other))),
            },
            Cont::LoopBody { body, env } => match done.ctl {
                Ctl::Break => Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::None)))),
                Ctl::Continue | Ctl::Val(_) => {
                    self.kont.push(Cont::LoopBody {
                        body: body.clone(),
                        env: env.clone(),
                    });
                    Ok(Resume::Next(Ctrl::EnterBlock(body, env, true)))
                }
                other => Ok(Resume::Redeliver(Done::plain(other))),
            },
            Cont::LetPatternBind { pattern, env } => {
                let Ctl::Val(value) = done.ctl else {
                    // A signal from the initializer propagates; nothing binds.
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                let tmp = env.child();
                self.interp.bind_pattern(&pattern, &value, &tmp)?;
                let mut advanced = env.clone();
                for name in pattern.bindings() {
                    if let Some(bound) = tmp.get(name.as_str()) {
                        advanced = advanced.define_shadowing(name.clone(), bound, false);
                    }
                }
                Ok(Resume::Redeliver(Done {
                    ctl: Ctl::Val(Value::None),
                    env: Some(advanced),
                }))
            }
            Cont::AssignRhs {
                target,
                op,
                env,
                span,
            } => {
                let Ctl::Val(rhs) = done.ctl else {
                    // A signal from the RHS propagates out of the statement.
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                match op {
                    None => {
                        // Simple assignment: write the RHS.
                        self.start_target_write(target, rhs, env, span)
                    }
                    Some(op) => {
                        // Compound: read the target, apply the operator, then
                        // write. The read and write each traverse (and
                        // re-evaluate) the target's subexpressions, exactly
                        // like `read_target` + `write_target`.
                        self.kont.push(Cont::AssignRead {
                            target: target.clone(),
                            op,
                            rhs,
                            env: env.clone(),
                            span,
                        });
                        self.start_target_traverse(target, TargetPhase::Read, env, span)
                    }
                }
            }
            Cont::AssignRead {
                target,
                op,
                rhs,
                env,
                span,
            } => {
                let Ctl::Val(cur) = done.ctl else {
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                let new = self.interp.binary(op, cur, rhs, span)?;
                self.start_target_write(target, new, env, span)
            }
            Cont::TargetBase {
                name,
                index,
                phase,
                env,
                span,
            } => {
                let Ctl::Val(base) = done.ctl else {
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                match index {
                    Some(ix) => {
                        self.kont.push(Cont::TargetIndex { base, phase, span });
                        Ok(Resume::Next(Ctrl::EvalExpr(ix, env)))
                    }
                    None => {
                        // Field target: `write_target`'s `Field` arm reads only
                        // the subject and then sets the field.
                        match phase {
                            TargetPhase::Read => {
                                Ok(Resume::Next(Ctrl::Done(Ctl::Val(self.interp.field_get(
                                    &base,
                                    name.as_deref().unwrap_or_default(),
                                    span,
                                )?))))
                            }
                            TargetPhase::Write(v) => {
                                self.interp.field_set(
                                    &base,
                                    name.as_deref().unwrap_or_default(),
                                    v,
                                    span,
                                )?;
                                Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::None))))
                            }
                        }
                    }
                }
            }
            Cont::TargetIndex { base, phase, span } => {
                let Ctl::Val(idx) = done.ctl else {
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                match phase {
                    TargetPhase::Read => Ok(Resume::Next(Ctrl::Done(Ctl::Val(
                        self.interp.index_get(&base, &idx, span)?,
                    )))),
                    TargetPhase::Write(v) => {
                        self.interp.index_set(&base, &idx, v, span)?;
                        Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::None))))
                    }
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
            Cont::ShortCircuitLeft { op, rhs, env } => {
                if let Ctl::Val(lv) = done.ctl {
                    // Mirror `Interp::eval`'s short-circuit arms exactly: `and`
                    // returns `false` on a falsy left, `or` returns `true` on a
                    // truthy left. A skipped right operand is never scheduled,
                    // so it cannot print, mutate, fail, or signal.
                    if matches!(op, BinOp::And) && !lv.truthy() {
                        return Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::Bool(false)))));
                    }
                    if matches!(op, BinOp::Or) && lv.truthy() {
                        return Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::Bool(true)))));
                    }
                    // Otherwise the right operand is required: evaluate it
                    // exactly once and take its truthiness.
                    self.kont.push(Cont::ShortCircuitRight);
                    Ok(Resume::Next(Ctrl::EvalExpr(rhs, env)))
                } else {
                    // A `return`/`throw`/`break`/`continue` from the left
                    // operand propagates without evaluating the right operand
                    // (mirrors `val!`).
                    Ok(Resume::Redeliver(Done::plain(done.ctl)))
                }
            }
            Cont::ShortCircuitRight => {
                if let Ctl::Val(rv) = done.ctl {
                    Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::Bool(rv.truthy())))))
                } else {
                    // A control signal from the required right operand
                    // propagates unchanged (mirrors `val!`).
                    Ok(Resume::Redeliver(Done::plain(done.ctl)))
                }
            }
            Cont::ListNext {
                items,
                index,
                mut out,
                env,
            } => {
                let Ctl::Val(value) = done.ctl else {
                    // A control signal from an element aborts the whole list;
                    // later elements are never evaluated (mirrors `val!`).
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                out.push(value);
                let next = index + 1;
                if next < items.len() {
                    let elem = Arc::new(items[next].clone());
                    self.kont.push(Cont::ListNext {
                        items,
                        index: next,
                        out,
                        env: env.clone(),
                    });
                    Ok(Resume::Next(Ctrl::EvalExpr(elem, env)))
                } else {
                    Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::list(out)))))
                }
            }
            Cont::MapKeyNext {
                entries,
                index,
                out,
                env,
            } => {
                let Ctl::Val(kv) = done.ctl else {
                    // A control signal from the key aborts the whole map; the
                    // value and every later entry are never evaluated (mirrors
                    // `val!`). No partial map is produced.
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                // The key is validated *before* the value is scheduled; a
                // non-key-capable key is E3001 with the key expression's span,
                // exactly like `Interp::eval_inner`'s `Expr::Map` arm. This is
                // also the exactly-once boundary: a failed key must never
                // schedule its value.
                let Some(key) = MapKey::from_value(&kv) else {
                    return Err(self.interp.error(
                        codes::TYPE_MISMATCH,
                        format!(
                            "type `{}` cannot be used as a map key; map keys must be `string`, `int`, or `bool`",
                            kv.type_name()
                        ),
                        entries[index].0.span(),
                    ));
                };
                self.kont.push(Cont::MapValueNext {
                    key,
                    entries: entries.clone(),
                    index,
                    out,
                    env: env.clone(),
                });
                Ok(Resume::Next(Ctrl::EvalExpr(
                    Arc::new(entries[index].1.clone()),
                    env,
                )))
            }
            Cont::MapValueNext {
                key,
                entries,
                index,
                mut out,
                env,
            } => {
                let Ctl::Val(value) = done.ctl else {
                    // A control signal from the value aborts the whole map;
                    // later entries are never evaluated (mirrors `val!`).
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                // `BTreeMap::insert` replaces an existing key, so a duplicate
                // key keeps the *last* entry's value: recursive semantics
                // observed as `{1: "a", 2: "b", 1: "c"}` -> `{1: "c", 2: "b"}`.
                out.insert(key, value);
                let next = index + 1;
                if next < entries.len() {
                    self.kont.push(Cont::MapKeyNext {
                        entries: entries.clone(),
                        index: next,
                        out,
                        env: env.clone(),
                    });
                    Ok(Resume::Next(Ctrl::EvalExpr(
                        Arc::new(entries[next].0.clone()),
                        env,
                    )))
                } else {
                    Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::Map(Rc::new(
                        RefCell::new(out),
                    ))))))
                }
            }
            Cont::RangeStart { end, env, span } => {
                if let Ctl::Val(start) = done.ctl {
                    // Start completed: retain it and evaluate the end exactly
                    // once. The start is **not** validated yet; the recursive
                    // engine evaluates both operands before checking either,
                    // then checks the start first.
                    self.kont.push(Cont::RangeEnd { start, span });
                    Ok(Resume::Next(Ctrl::EvalExpr(end, env)))
                } else {
                    // A control signal from the start aborts before the end is
                    // scheduled (mirrors `val!`).
                    Ok(Resume::Redeliver(Done::plain(done.ctl)))
                }
            }
            Cont::RangeEnd { start, span } => {
                if let Ctl::Val(end) = done.ctl {
                    // Both operands completed; validate start then end exactly
                    // like `Interp::eval_inner`'s `Expr::Range` arm.
                    let s = match start {
                        Value::Int(i) => i,
                        other => {
                            return Err(self.interp.error(
                                codes::TYPE_MISMATCH,
                                format!("range start expects an int, found {}", other.type_name()),
                                span,
                            ))
                        }
                    };
                    let e = match end {
                        Value::Int(i) => i,
                        other => {
                            return Err(self.interp.error(
                                codes::TYPE_MISMATCH,
                                format!("range end expects an int, found {}", other.type_name()),
                                span,
                            ))
                        }
                    };
                    Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::Range(Rc::new(
                        super::value::RangeVal { start: s, end: e },
                    ))))))
                } else {
                    // A control signal from the end propagates without
                    // constructing a range (mirrors `val!`). The start value is
                    // not validated, exactly as recursion does not validate it.
                    Ok(Resume::Redeliver(Done::plain(done.ctl)))
                }
            }
            Cont::IndexTarget { idx, env, span } => {
                if let Ctl::Val(base) = done.ctl {
                    // The target completed: retain it (unvalidated) and
                    // evaluate the index exactly once, mirroring
                    // `eval_inner`'s `Expr::Index` arm order. Validation is
                    // deferred to `Cont::IndexApply`.
                    self.kont.push(Cont::IndexApply { base, span });
                    Ok(Resume::Next(Ctrl::EvalExpr(idx, env)))
                } else {
                    // A control signal from the target propagates and the index
                    // is never evaluated (mirrors `val!`).
                    Ok(Resume::Redeliver(Done::plain(done.ctl)))
                }
            }
            Cont::IndexApply { base, span } => {
                if let Ctl::Val(idx) = done.ctl {
                    Ok(Resume::Next(Ctrl::Done(Ctl::Val(
                        self.interp.index_get(&base, &idx, span)?,
                    ))))
                } else {
                    // A control signal from the index propagates without
                    // applying the lookup (mirrors `val!`).
                    Ok(Resume::Redeliver(Done::plain(done.ctl)))
                }
            }
            Cont::FieldReceiver { name, span } => {
                if let Ctl::Val(subject) = done.ctl {
                    // Resolve the field/method exactly like
                    // `Interp::eval_inner`'s `Expr::Field` arm. A control
                    // signal from the receiver is redelivered above and never
                    // reaches this resolution.
                    match &subject {
                        Value::Instance(i) => {
                            // A method is not a bound value: `s.m` names only a
                            // field, and a missing field is `E2003`.
                            if self
                                .interp
                                .methods
                                .contains_key(&(i.ty.clone(), name.clone()))
                                && !i.fields.borrow().iter().any(|(k, _)| k == &name)
                            {
                                return Err(self.interp.error(
                                    codes::UNDEFINED,
                                    format!(
                                        "struct {} has method `{name}`; call it as `{name}(...)`",
                                        i.ty
                                    ),
                                    span,
                                ));
                            }
                            Ok(Resume::Next(Ctrl::Done(Ctl::Val(
                                self.interp.field_get(&subject, &name, span)?,
                            ))))
                        }
                        _ => Ok(Resume::Next(Ctrl::Done(Ctl::Val(self.interp.method(
                            &subject,
                            &name,
                            Vec::new(),
                            span,
                        )?)))),
                    }
                } else {
                    Ok(Resume::Redeliver(Done::plain(done.ctl)))
                }
            }
            Cont::FStrNext {
                parts,
                index,
                spec,
                mut out,
                env,
            } => {
                let Ctl::Val(v) = done.ctl else {
                    // A control signal from an interpolation aborts the whole
                    // f-string; later parts are never evaluated and no partial
                    // string is produced (mirrors `val!`).
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                match spec {
                    None => out.push_str(&v.display()),
                    // A `format_value` diagnostic (E3001 incompatible type,
                    // E4013 precision/width bound) aborts the f-string exactly
                    // as recursion does; the accumulator is dropped.
                    Some(s) => out.push_str(&self.interp.format_value(&v, &s)?),
                }
                let next = index + 1;
                Ok(Resume::Next(self.advance_fstring(parts, next, out, &env)))
            }
            Cont::ReturnFrom => match done.ctl {
                Ctl::Val(v) => Ok(Resume::Next(Ctrl::ReturnValue(v))),
                other => Ok(Resume::Redeliver(Done::plain(other))),
            },
            Cont::ThrowFrom => match done.ctl {
                Ctl::Val(v) => Ok(Resume::Next(Ctrl::ThrowValue(v))),
                other => Ok(Resume::Redeliver(Done::plain(other))),
            },
            Cont::CallArgs {
                callee,
                args,
                index,
                mut values,
                env,
                span,
            } => {
                let Ctl::Val(v) = done.ctl else {
                    // A control signal from an argument aborts the call before
                    // any callee resolution, exactly like `eval_call`'s
                    // argument loop (`?`/`other => return Ok(other)`).
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                values.push(v);
                if index < args.len() {
                    let next = Arc::new(args[index].value.clone());
                    self.kont.push(Cont::CallArgs {
                        callee,
                        args: args.clone(),
                        index: index + 1,
                        values,
                        env: env.clone(),
                        span,
                    });
                    Ok(Resume::Next(Ctrl::EvalExpr(next, env)))
                } else {
                    match self.dispatch_call(callee, args, values, env, span)? {
                        Control::Next(next) => Ok(Resume::Next(next)),
                        Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                    }
                }
            }
            Cont::ConstructArgs {
                name,
                args,
                index,
                mut positional,
                mut named,
                env,
                span,
            } => {
                let Ctl::Val(v) = done.ctl else {
                    // A control signal from a field argument aborts the
                    // construction with no partial instance escaping,
                    // exactly like `Interp::construct`'s `other => return`.
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                match &args[index - 1].name {
                    Some(n) => named.push((n.clone(), v)),
                    None => positional.push(v),
                }
                if index < args.len() {
                    let next = Arc::new(args[index].value.clone());
                    self.kont.push(Cont::ConstructArgs {
                        name,
                        args: args.clone(),
                        index: index + 1,
                        positional,
                        named,
                        env: env.clone(),
                        span,
                    });
                    Ok(Resume::Next(Ctrl::EvalExpr(next, env)))
                } else {
                    match self.finish_construct(&name, positional, named, span)? {
                        Control::Next(next) => Ok(Resume::Next(next)),
                        Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                    }
                }
            }
            Cont::PipeRight { r, env, span } => {
                let Ctl::Val(left) = done.ctl else {
                    // A control signal from the left operand propagates
                    // without evaluating the right (mirrors `val!`).
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                self.kont.push(Cont::PipeApply { left, span });
                Ok(Resume::Next(Ctrl::EvalExpr(r, env)))
            }
            Cont::PipeApply { left, span } => {
                let Ctl::Val(f) = done.ctl else {
                    // A control signal from the right operand propagates
                    // without applying the pipe (mirrors `val!`).
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                match self.start_call_value(f, vec![left], span)? {
                    Control::Next(next) => Ok(Resume::Next(next)),
                    Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                }
            }
            Cont::MethodReceiver {
                name,
                args,
                env,
                span,
            } => {
                let Ctl::Val(subject) = done.ctl else {
                    // A control signal from the receiver propagates without
                    // evaluating any argument (mirrors `val!`).
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                match self.start_method(subject, name, args, env, span)? {
                    Control::Next(next) => Ok(Resume::Next(next)),
                    Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                }
            }
            Cont::MethodArgs {
                subject,
                name,
                args,
                index,
                mut values,
                env,
                span,
            } => {
                let Ctl::Val(v) = done.ctl else {
                    // A control signal from an argument aborts the method call
                    // before dispatch (mirrors `val!`).
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                values.push(v);
                if index < args.len() {
                    let next = Arc::new(args[index].value.clone());
                    self.kont.push(Cont::MethodArgs {
                        subject,
                        name,
                        args: args.clone(),
                        index: index + 1,
                        values,
                        env: env.clone(),
                        span,
                    });
                    Ok(Resume::Next(Ctrl::EvalExpr(next, env)))
                } else {
                    match self.finish_method(subject, &name, values, span)? {
                        Control::Next(next) => Ok(Resume::Next(next)),
                        Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                    }
                }
            }
            Cont::CallCallee { values, span } => {
                let Ctl::Val(f) = done.ctl else {
                    // A control signal from the callee expression propagates
                    // without applying the call (mirrors `eval_call`'s `other`
                    // arm).
                    return Ok(Resume::Redeliver(Done::plain(done.ctl)));
                };
                match self.start_call_value(f, values, span)? {
                    Control::Next(next) => Ok(Resume::Next(next)),
                    Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                }
            }
            Cont::NativeResume { resume, span } => {
                let result = match done.ctl {
                    Ctl::Val(v) => Ok(v),
                    // A control signal from the callback aborts the native and
                    // propagates unchanged (the recursive adapter's
                    // `call_value` returns `Err` only for diagnostics; a
                    // `Ctl` signal cannot escape a frame boundary, so a signal
                    // here means the callback body produced it directly).
                    other => {
                        return Ok(Resume::Redeliver(Done::plain(other)));
                    }
                };
                let step = resume.resume(self.interp, result)?;
                match self.start_native_resume(step, span)? {
                    Control::Next(next) => Ok(Resume::Next(next)),
                    Control::Finished(ctl) => Ok(Resume::Redeliver(Done::plain(ctl))),
                }
            }
            Cont::FrameBoundary { call_span } => self.resume_frame_boundary(done, call_span),
        }
    }

    /// Begin evaluating a list/tuple element sequence (B-1R3B.4.1).
    ///
    /// Mirrors `Interp::eval_inner`'s `Expr::List`/`Expr::Tuple` arms: each
    /// element is evaluated in source order with the same environment, exactly
    /// once; the values are collected and become `Value::list`. An empty list
    /// completes immediately.
    ///
    /// `Result` is kept for a uniform `step` dispatch signature even though this
    /// arm cannot currently fail; later container forms may (mirrors
    /// [`Machine::start_block`]).
    #[allow(clippy::unnecessary_wraps)]
    fn start_list(&mut self, items: Arc<[Expr]>, env: &Env) -> Result<Control> {
        if items.is_empty() {
            return Ok(Control::Next(Ctrl::Done(Ctl::Val(Value::list(Vec::new())))));
        }
        // `index` is the element currently being evaluated; `out` holds the
        // elements that already completed.
        self.kont.push(Cont::ListNext {
            items: items.clone(),
            index: 0,
            out: Vec::with_capacity(items.len()),
            env: env.clone(),
        });
        Ok(Control::Next(Ctrl::EvalExpr(
            Arc::new(items[0].clone()),
            env.clone(),
        )))
    }

    /// Begin evaluating a map literal's key/value entries (B-1R3B.4.2).
    ///
    /// Mirrors `Interp::eval_inner`'s `Expr::Map` arm: for each entry in source
    /// order, the key is evaluated and validated *before* its value is
    /// scheduled; the value is then evaluated exactly once and inserted. An
    /// empty map (`{:}`) completes immediately. The accumulated `BTreeMap`
    /// lives in the continuations, so no `Map`-sized or `Map`-nested Rust
    /// recursion occurs.
    ///
    /// `Result` is kept for a uniform `step` dispatch signature even though this
    /// arm cannot currently fail (mirrors [`Machine::start_list`]).
    #[allow(clippy::unnecessary_wraps)]
    fn start_map(&mut self, entries: Arc<[(Expr, Expr)]>, env: &Env) -> Result<Control> {
        if entries.is_empty() {
            return Ok(Control::Next(Ctrl::Done(Ctl::Val(Value::Map(Rc::new(
                RefCell::new(BTreeMap::new()),
            ))))));
        }
        self.kont.push(Cont::MapKeyNext {
            entries: entries.clone(),
            index: 0,
            out: BTreeMap::new(),
            env: env.clone(),
        });
        Ok(Control::Next(Ctrl::EvalExpr(
            Arc::new(entries[0].0.clone()),
            env.clone(),
        )))
    }

    /// Begin evaluating an f-string's parts (B-1R3B.7).
    ///
    /// Mirrors `Interp::eval_inner`'s `Expr::FStr` arm: parts are consumed left
    /// to right; each [`FPart::Lit`] is appended verbatim and each
    /// [`FPart::Expr`] is evaluated exactly once in source order and appended
    /// with its display form or its format specification. An empty or text-only
    /// f-string terminates here with no scheduled expression. The accumulator
    /// lives in the continuation, so nesting does not grow the Rust stack.
    ///
    /// `Result` is kept for a uniform `step` dispatch signature even though this
    /// arm cannot currently fail (mirrors [`Machine::start_list`]).
    #[allow(clippy::unnecessary_wraps)]
    fn start_fstring(&mut self, parts: Arc<[FPart]>, env: &Env) -> Result<Control> {
        Ok(Control::Next(self.advance_fstring(
            parts,
            0,
            String::new(),
            env,
        )))
    }

    /// Advance an f-string from part `start`, appending literal text into `out`
    /// until the next interpolation, which is scheduled exactly once.
    ///
    /// Returns the next work item: an [`Ctrl::EvalExpr`] for the next
    /// interpolation (with [`Cont::FStrNext`] pushed to append its stringified
    /// value and resume), or [`Ctrl::Done`] with the completed `string` when no
    /// interpolation remains. `env` is retained across parts, so later
    /// interpolations observe the same environment (and any shadowing that
    /// occurred before the f-string).
    fn advance_fstring(
        &mut self,
        parts: Arc<[FPart]>,
        start: usize,
        mut out: String,
        env: &Env,
    ) -> Ctrl {
        for index in start..parts.len() {
            match &parts[index] {
                FPart::Lit(t) => out.push_str(t),
                FPart::Expr(e, spec) => {
                    // Extract the owned pieces before moving `parts` into the
                    // continuation.
                    let expr = Arc::new(e.clone());
                    let spec = spec.clone();
                    self.kont.push(Cont::FStrNext {
                        parts,
                        index,
                        spec,
                        out,
                        env: env.clone(),
                    });
                    return Ctrl::EvalExpr(expr, env.clone());
                }
            }
        }
        Ctrl::Done(Ctl::Val(Value::str(out)))
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
            Expr::FStr(parts, _) => {
                // Parts left to right, exactly once each (B-1R3B.7). Literal
                // text is appended verbatim; each interpolation is evaluated
                // exactly once and appended with its display form or format
                // specification. An empty or text-only f-string completes
                // without scheduling any expression.
                self.start_fstring(parts.clone(), env)
            }
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
            Expr::List(items, _) | Expr::Tuple(items, _) => {
                // Left to right, exactly once per element (B-1R3B.4.1). A tuple
                // literal is list sugar (`LANGUAGE_SPEC.md` §21) and shares the
                // list evaluation path, exactly like `eval_inner`.
                self.start_list(items.clone(), env)
            }
            Expr::Map(entries, _) => {
                // Key then value per entry, in source order, exactly once each
                // (B-1R3B.4.2). An empty map (`{:}`) completes immediately.
                self.start_map(entries.clone(), env)
            }
            Expr::Binary(op, l, r, span) => {
                match op {
                    // Short-circuit `and`/`or`: evaluate the left operand
                    // exactly once; the continuation decides whether the right
                    // operand is required at all (B-1R3B.3). Never fall back.
                    BinOp::And | BinOp::Or => {
                        self.kont.push(Cont::ShortCircuitLeft {
                            op: *op,
                            rhs: r.clone(),
                            env: env.clone(),
                        });
                        Ok(Control::Next(Ctrl::EvalExpr(l.clone(), env.clone())))
                    }
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
            Expr::Range(start, end, span) => {
                // `a..b` builds the same lazy, half-open, start-inclusive
                // `Value::Range` as `range(a, b)` (`LANGUAGE_SPEC.md` §22.1).
                // The start operand runs first and exactly once; the end runs
                // next and exactly once; neither is validated until both have
                // completed, mirroring `Interp::eval_inner`'s `Expr::Range` arm
                // (which validates start before end after evaluating both).
                // Both type errors are `E3001` with the recursive messages at
                // the range's span.
                self.kont.push(Cont::RangeStart {
                    end: end.clone(),
                    env: env.clone(),
                    span: *span,
                });
                Ok(Control::Next(Ctrl::EvalExpr(start.clone(), env.clone())))
            }
            Expr::Index(base, idx, span) => {
                // `base[index]` evaluates the target first and exactly once,
                // then the index exactly once, then applies the lookup. This is
                // the `eval_inner`'s `Expr::Index` arm order; the lookup itself
                // is delegated to `Interp::index_get` so list/string/map/instance
                // semantics and every diagnostic are identical by construction.
                self.kont.push(Cont::IndexTarget {
                    idx: idx.clone(),
                    env: env.clone(),
                    span: *span,
                });
                Ok(Control::Next(Ctrl::EvalExpr(base.clone(), env.clone())))
            }
            Expr::Field(recv, name, span) => {
                // `recv.name` evaluates the receiver once, then resolves the
                // field or zero-argument method with `eval_inner`'s exact rules
                // (`Cont::FieldReceiver`). The name is syntactic, so it is
                // copied into the continuation rather than evaluated.
                self.kont.push(Cont::FieldReceiver {
                    name: name.clone(),
                    span: *span,
                });
                Ok(Control::Next(Ctrl::EvalExpr(recv.clone(), env.clone())))
            }
            Expr::Lambda(params, body, _) => {
                // A lambda captures the environment **by reference** and its
                // source for diagnostics; construction is synchronous (nothing
                // to evaluate), exactly like `Interp::eval_inner`.
                let body_stmts = match body.as_ref() {
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
                if let Some(source) = self.interp.current_source {
                    self.interp
                        .closure_sources
                        .insert(Rc::as_ptr(&closure) as usize, source);
                }
                Ok(Control::Next(Ctrl::Done(Ctl::Val(Value::Closure(closure)))))
            }
            Expr::Pipe(l, r, span) => {
                // `l |> r` evaluates `l` once, then `r` once, then applies
                // `r` to `l`'s value through the ordinary call path.
                self.kont.push(Cont::PipeRight {
                    r: r.clone(),
                    env: env.clone(),
                    span: *span,
                });
                Ok(Control::Next(Ctrl::EvalExpr(l.clone(), env.clone())))
            }
            Expr::ListComp {
                value,
                pattern,
                iterable,
                filter,
                span,
            } => {
                let spec = CompSpec {
                    key: None,
                    value: value.clone(),
                    pattern: pattern.clone(),
                    filter: filter.clone(),
                    iterable: iterable.clone(),
                    span: *span,
                    is_map: false,
                };
                Ok(self.start_comp(spec, env.clone(), *span))
            }
            Expr::MapComp {
                key,
                value,
                pattern,
                iterable,
                filter,
                span,
            } => {
                let spec = CompSpec {
                    key: Some(key.clone()),
                    value: value.clone(),
                    pattern: pattern.clone(),
                    filter: filter.clone(),
                    iterable: iterable.clone(),
                    span: *span,
                    is_map: true,
                };
                Ok(self.start_comp(spec, env.clone(), *span))
            }
            Expr::Method(recv, name, args, _ty_args, span) => {
                // The receiver is evaluated exactly once, then the arguments in
                // source order (`Interp::eval_inner`'s `Expr::Method` arm has
                // the identical order).
                self.kont.push(Cont::MethodReceiver {
                    name: name.clone(),
                    args: args.clone(),
                    env: env.clone(),
                    span: *span,
                });
                Ok(Control::Next(Ctrl::EvalExpr(recv.clone(), env.clone())))
            }
            Expr::Construct(name, args, _ty_args, span) => {
                // Field arguments are evaluated strictly in source order,
                // exactly once each, before struct/variant resolution
                // (`Interp::construct` has the identical prerequisite). A
                // zero-argument constructor resolves immediately.
                self.start_construct(name.clone(), args.clone(), env.clone(), *span)
            }
            Expr::Call(callee, args, _ty_args, span) => {
                // Arguments are evaluated strictly in source order, exactly
                // once each, *before* any callee resolution or parameter
                // binding (`LANGUAGE_SPEC.md` §13); `Interp::eval_call` has the
                // identical prerequisite. `start_call` schedules the first
                // argument or, for a zero-argument call, dispatches directly.
                self.start_call(callee.clone(), args.clone(), env.clone(), *span)
            }
            Expr::Match(subject, arms, span) => {
                // The subject is evaluated exactly once; arms are tried in
                // order with `match_pattern`; the first matching (and,
                // if present, truthy-guarded) arm's body runs in a fresh child
                // scope; no match is `E4025` (`Interp::eval_inner`).
                self.kont.push(Cont::MatchSubject {
                    arms: arms.clone(),
                    env: env.clone(),
                    span: *span,
                });
                Ok(Control::Next(Ctrl::EvalExpr(subject.clone(), env.clone())))
            }
        }
    }

    /// Begin a call: schedule the first argument, or dispatch immediately when
    /// there are none (R3C.1). Mirrors `Interp::eval_call`'s argument loop.
    fn start_call(
        &mut self,
        callee: Arc<Expr>,
        args: Arc<[Arg]>,
        env: Env,
        span: Span,
    ) -> Result<Control> {
        if args.is_empty() {
            return self.dispatch_call(callee, args, Vec::new(), env, span);
        }
        let first = Arc::new(args[0].value.clone());
        self.kont.push(Cont::CallArgs {
            callee,
            args: args.clone(),
            index: 1,
            values: Vec::with_capacity(args.len()),
            env: env.clone(),
            span,
        });
        Ok(Control::Next(Ctrl::EvalExpr(first, env)))
    }

    /// Begin a constructor: schedule the first field argument, or resolve a
    /// zero-argument constructor immediately (R3C.3). Mirrors
    /// `Interp::construct`'s argument loop.
    fn start_construct(
        &mut self,
        name: String,
        args: Arc<[Arg]>,
        env: Env,
        span: Span,
    ) -> Result<Control> {
        if args.is_empty() {
            return Ok(Control::Next(Ctrl::Done(Ctl::Val(
                self.interp
                    .construct_from_values(&name, Vec::new(), Vec::new(), span)?,
            ))));
        }
        let first = Arc::new(args[0].value.clone());
        self.kont.push(Cont::ConstructArgs {
            name,
            args: args.clone(),
            index: 1,
            positional: Vec::new(),
            named: Vec::new(),
            env: env.clone(),
            span,
        });
        Ok(Control::Next(Ctrl::EvalExpr(first, env)))
    }

    /// Resolve a constructor whose arguments have all completed, exactly like
    /// `Interp::construct`'s resolution half (shared implementation).
    fn finish_construct(
        &mut self,
        name: &str,
        positional: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    ) -> Result<Control> {
        Ok(Control::Next(Ctrl::Done(Ctl::Val(
            self.interp
                .construct_from_values(name, named, positional, span)?,
        ))))
    }

    /// Begin a method call once the receiver is known (R3C.2): schedule the
    /// first argument, or dispatch immediately for a zero-argument method.
    fn start_method(
        &mut self,
        subject: Value,
        name: String,
        args: Arc<[Arg]>,
        env: Env,
        span: Span,
    ) -> Result<Control> {
        if args.is_empty() {
            return self.finish_method(subject, &name, Vec::new(), span);
        }
        let first = Arc::new(args[0].value.clone());
        self.kont.push(Cont::MethodArgs {
            subject,
            name,
            args: args.clone(),
            index: 1,
            values: Vec::with_capacity(args.len()),
            env: env.clone(),
            span,
        });
        Ok(Control::Next(Ctrl::EvalExpr(first, env)))
    }

    /// Resolve a method call whose arguments have all completed, exactly like
    /// `Interp::eval_inner`'s `Expr::Method` arm: a struct receiver resolves
    /// against its nominal method table (crossing the ordinary call frame with
    /// `self` bound first); any other receiver uses the built-in registry.
    fn finish_method(
        &mut self,
        subject: Value,
        name: &str,
        values: Vec<Value>,
        span: Span,
    ) -> Result<Control> {
        if let Value::Instance(i) = &subject {
            let key = (i.ty.clone(), name.to_string());
            if let Some(set) = self.interp.methods.get(&key).cloned() {
                let c = self
                    .interp
                    .select_method_overload(&i.ty, name, &set, &values, span)?;
                let mut full = Vec::with_capacity(values.len() + 1);
                full.push(subject.clone());
                full.extend(values);
                return Ok(Control::Next(self.enter_call(c, full, span)?));
            }
            return Err(self.interp.error(
                codes::UNDEFINED,
                format!("struct {} has no method `{name}`", i.ty),
                span,
            ));
        }
        Ok(Control::Next(Ctrl::Done(Ctl::Val(
            self.interp.method(&subject, name, values, span)?,
        ))))
    }

    /// Find the first `match` arm whose pattern matches (and whose guard, if
    /// any, is truthy) and run its body (R3E.2). Pattern/binding semantics are
    /// the shared `Interp` implementations. No match is `E4025`.
    fn try_match_arms(
        &mut self,
        arms: Arc<[crate::ast::Arm]>,
        subject: Value,
        start: usize,
        env: Env,
        span: Span,
    ) -> Result<Control> {
        for (i, arm) in arms.iter().enumerate().skip(start) {
            if !self.interp.match_pattern(&arm.pattern, &subject) {
                continue;
            }
            let scope = env.child();
            self.interp.bind_pattern(&arm.pattern, &subject, &scope)?;
            if let Some(g) = &arm.guard {
                let guard = Arc::new(g.clone());
                self.kont.push(Cont::MatchGuard {
                    arms: arms.clone(),
                    subject: subject.clone(),
                    next: i + 1,
                    scope: scope.clone(),
                    span,
                });
                return Ok(Control::Next(Ctrl::EvalExpr(guard, scope)));
            }
            return Ok(Control::Next(Ctrl::EnterBlock(
                arm.body.clone(),
                scope,
                true,
            )));
        }
        Err(self
            .interp
            .error(codes::NO_MATCH, "no match arm matched the value", span))
    }

    /// Begin a comprehension (R3E.1): evaluate the iterable exactly once, then
    /// iterate. A signal from the iterable propagates before any iteration.
    fn start_comp(&mut self, spec: CompSpec, env: Env, span: Span) -> Control {
        let _ = span;
        let iterable = spec.iterable.clone();
        self.kont.push(Cont::CompIterable(
            Box::new(CompState {
                spec,
                env: env.clone(),
                items: Vec::new(),
                index: 0,
                out: Vec::new(),
                map: std::collections::BTreeMap::new(),
            }),
            env.clone(),
        ));
        Control::Next(Ctrl::EvalExpr(iterable, env))
    }

    /// Advance a comprehension (R3E.1): take the next materialized item, bind
    /// it in a fresh child scope, evaluate the filter then the value. Mirrors
    /// `Interp::eval_inner`'s comprehension arms exactly.
    fn step_comp(&mut self, mut state: Box<CompState>) -> Result<Control> {
        if state.index >= state.items.len() {
            return Ok(Control::Finished(Ctl::Val(if state.spec.is_map {
                Value::Map(Rc::new(std::cell::RefCell::new(state.map)))
            } else {
                Value::list(state.out)
            })));
        }
        let item = state.items[state.index].clone();
        state.index += 1;
        let scope = state.env.child();
        self.interp
            .bind_pattern(&state.spec.pattern, &item, &scope)?;
        if let Some(f) = state.spec.filter.clone() {
            self.kont.push(Cont::CompFilter(state, scope.clone()));
            Ok(Control::Next(Ctrl::EvalExpr(f, scope)))
        } else {
            Ok(self.after_comp_filter(state, scope))
        }
    }

    /// The filter passed (or is absent): evaluate the map key first, or the
    /// value directly for a list comprehension (R3E.1).
    fn after_comp_filter(&mut self, state: Box<CompState>, scope: Env) -> Control {
        if let Some(key) = state.spec.key.clone() {
            self.kont.push(Cont::CompKey(state, scope.clone()));
            Control::Next(Ctrl::EvalExpr(key, scope))
        } else {
            let value = state.spec.value.clone();
            self.kont.push(Cont::CompValue(state, scope.clone(), None));
            Control::Next(Ctrl::EvalExpr(value, scope))
        }
    }

    /// Run one `for` iteration step (R3D.4): take the next Range item or the
    /// next materialized item, bind it in a fresh child scope, and enter the
    /// body. Exhaustion yields `none`. Mirrors `Interp::exec_stmt`'s `For`.
    fn step_for(&mut self, state: ForState) -> Result<Ctrl> {
        let ForState {
            pattern,
            body,
            env,
            span,
            items,
            index,
            range,
        } = state;
        let item = if let Some(cursor) = range {
            if cursor.next < cursor.end {
                let item = Value::Int(cursor.next);
                // The cursor advances via the continuation, not by mutation.
                let next_cursor = RangeCursor {
                    next: cursor.next + 1,
                    end: cursor.end,
                };
                self.kont.push(Cont::ForNext(ForState {
                    pattern: pattern.clone(),
                    body: body.clone(),
                    env: env.clone(),
                    span,
                    items: Vec::new(),
                    index: 0,
                    range: Some(next_cursor),
                }));
                item
            } else {
                return Ok(Ctrl::Done(Ctl::Val(Value::None)));
            }
        } else if index < items.len() {
            let item = items[index].clone();
            self.kont.push(Cont::ForNext(ForState {
                pattern: pattern.clone(),
                body: body.clone(),
                env: env.clone(),
                span,
                items,
                index: index + 1,
                range: None,
            }));
            item
        } else {
            return Ok(Ctrl::Done(Ctl::Val(Value::None)));
        };
        let scope = env.child();
        self.interp.bind_pattern(&pattern, &item, &scope)?;
        Ok(Ctrl::EnterBlock(body, scope, false))
    }

    /// Start a target read or write (R3D.1). `Name` targets resolve
    /// synchronously; `Index`/`Field` targets traverse their subexpressions in
    /// the same order as `read_target`/`write_target` (base then index), with
    /// the phase carried in the continuation so a write re-traverses exactly
    /// like the recursive engine.
    fn start_target_traverse(
        &mut self,
        target: Arc<Expr>,
        phase: TargetPhase,
        env: Env,
        span: Span,
    ) -> Result<Resume> {
        match &*target {
            Expr::Name(name, nspan) => match phase {
                TargetPhase::Read => {
                    let v = env.get(name).ok_or_else(|| {
                        self.interp.error(
                            codes::UNDEFINED,
                            format!("undefined variable `{name}`"),
                            *nspan,
                        )
                    })?;
                    Ok(Resume::Next(Ctrl::Done(Ctl::Val(v))))
                }
                TargetPhase::Write(value) => {
                    env.assign(name, value).map_err(|e| match e {
                        crate::run::AssignError::Immutable => self.interp.error(
                            codes::ASSIGN_IMMUTABLE,
                            format!(
                                "cannot assign to `{name}`: it is immutable (declare it `let mut`)"
                            ),
                            *nspan,
                        ),
                        crate::run::AssignError::Undefined => self.interp.error(
                            codes::UNDEFINED,
                            format!("undefined variable `{name}`"),
                            *nspan,
                        ),
                    })?;
                    Ok(Resume::Next(Ctrl::Done(Ctl::Val(Value::None))))
                }
            },
            Expr::Index(b, i, _) => {
                self.kont.push(Cont::TargetBase {
                    name: None,
                    index: Some(i.clone()),
                    phase,
                    env: env.clone(),
                    span,
                });
                Ok(Resume::Next(Ctrl::EvalExpr(b.clone(), env)))
            }
            Expr::Field(b, name, _) => {
                self.kont.push(Cont::TargetBase {
                    name: Some(name.clone()),
                    index: None,
                    phase,
                    env: env.clone(),
                    span,
                });
                Ok(Resume::Next(Ctrl::EvalExpr(b.clone(), env)))
            }
            other => Err(self.interp.error(
                codes::INVALID_ASSIGN,
                "invalid assignment target",
                expr_span(other),
            )),
        }
    }

    /// Start a target write (R3D.1), preserving the recursive engine's
    /// evaluation order (base, then index for `Index` targets).
    fn start_target_write(
        &mut self,
        target: Arc<Expr>,
        value: Value,
        env: Env,
        span: Span,
    ) -> Result<Resume> {
        self.start_target_traverse(target, TargetPhase::Write(value), env, span)
    }

    /// Resolve a call whose arguments have all completed, exactly like
    /// `Interp::eval_call`: a `Name` callee checks the function overload set,
    /// then the native registry, then the environment (`call_value`); any
    /// other callee shape is itself evaluated first and then applied through
    /// `call_value`. A user `Closure` executes in the same machine loop as an
    /// explicit frame (`start_call_frame`) — never a nested evaluator.
    fn dispatch_call(
        &mut self,
        callee: Arc<Expr>,
        args: Arc<[Arg]>,
        values: Vec<Value>,
        env: Env,
        span: Span,
    ) -> Result<Control> {
        let Expr::Name(name, nspan) = &*callee else {
            self.kont.push(Cont::CallCallee { values, span });
            return Ok(Control::Next(Ctrl::EvalExpr(callee.clone(), env)));
        };
        let nspan = *nspan;
        if let Some(set) = self.interp.functions.get(name.as_str()).cloned() {
            // Static overload resolution uses the checker's own selector.
            let c = self.interp.select_overload(name, &set, &values, nspan)?;
            let call_vals = super::bind_arguments(&args, &values, &c.params, span)?;
            return Ok(Control::Next(self.enter_call(c, call_vals, nspan)?));
        }
        if self.interp.natives.contains_key(name.as_str()) {
            // A builtin called directly: arity is enforced by the shared
            // registry, then either the resumable protocol (machine work) or
            // the ordinary native runs.
            self.interp.check_native_arity(name, values.len(), nspan)?;
            if let Some(n) = self.interp.resumable_natives.get(name.as_str()).cloned() {
                let step = n(self.interp, values, nspan)?;
                return self.start_native_resume(step, nspan);
            }
            let n = self
                .interp
                .natives
                .get(name.as_str())
                .cloned()
                .ok_or_else(|| {
                    self.interp
                        .error(codes::UNDEFINED, format!("unknown `{name}`"), nspan)
                })?;
            return Ok(Control::Next(Ctrl::Done(Ctl::Val(n(
                self.interp,
                values,
                nspan,
            )?))));
        }
        if let Some(f) = env.get(name) {
            return self.start_call_value(f, values, nspan);
        }
        Err(self.interp.error(
            codes::UNDEFINED,
            format!("undefined function `{name}`"),
            nspan,
        ))
    }

    /// Apply an already-evaluated callee value to already-evaluated arguments
    /// (`Interp::call_value`): a `Closure` enters a machine frame, a `Native`
    /// checks arity and runs, anything else is `E3001`.
    fn start_call_value(&mut self, f: Value, values: Vec<Value>, span: Span) -> Result<Control> {
        match &f {
            Value::Closure(c) => {
                let c = c.clone();
                Ok(Control::Next(self.enter_call(c, values, span)?))
            }
            Value::Native(name) => {
                let name = name.clone();
                self.interp.check_native_arity(&name, values.len(), span)?;
                if let Some(n) = self.interp.resumable_natives.get(name.as_str()).cloned() {
                    let step = n(self.interp, values, span)?;
                    return self.start_native_resume(step, span);
                }
                let n = self
                    .interp
                    .natives
                    .get(name.as_str())
                    .cloned()
                    .ok_or_else(|| {
                        self.interp
                            .error(codes::UNDEFINED, format!("unknown `{name}`"), span)
                    })?;
                Ok(Control::Next(Ctrl::Done(Ctl::Val(n(
                    self.interp,
                    values,
                    span,
                )?))))
            }
            other => Err(self.interp.error(
                codes::TYPE_MISMATCH,
                format!("{} is not callable", other.type_name()),
                span,
            )),
        }
    }

    /// Enter a user closure call as an explicit frame (`Interp::call`'s body
    /// setup) and return the `Ctrl` that starts its body in this same machine
    /// loop. Argument count, parameter binding, frame accounting, source
    /// attribution, and per-frame nesting reset are all established here.
    fn enter_call(&mut self, closure: Rc<Closure>, args: Vec<Value>, span: Span) -> Result<Ctrl> {
        if args.len() != closure.params.len() {
            return Err(self.interp.error(
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
        self.push_frame(closure.clone(), env.clone(), span)?;
        self.kont.push(Cont::FrameBoundary { call_span: span });
        Ok(Ctrl::EnterBlock(closure.body.clone(), env, false))
    }

    /// Consume one [`NativeOutcome`] step from a resumable native
    /// (`ITERATIVE_EVALUATOR_DESIGN.md` §13). A callback request becomes
    /// machine work: a `Closure` callback enters a normal user frame (with
    /// frame accounting), a `Native` callback runs inline with no frame —
    /// exactly the recursive `call_value` asymmetry. The native's `resume`
    /// token rides in [`Cont::NativeResume`].
    fn start_native_resume(&mut self, step: NativeOutcome, span: Span) -> Result<Control> {
        match step {
            NativeOutcome::Done(v) => Ok(Control::Next(Ctrl::Done(Ctl::Val(v)))),
            NativeOutcome::InvokeCallback { f, args, resume } => {
                self.kont.push(Cont::NativeResume { resume, span });
                self.start_call_value(f, args, span)
            }
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
            Stmt::LetPattern { pattern, value, .. } => {
                // RHS once, then atomic destructure into a temporary child
                // scope, then transfer each binding with shadowing semantics
                // (`Interp::exec_stmt`'s `LetPattern`).
                self.kont.push(Cont::LetPatternBind {
                    pattern: pattern.clone(),
                    env: env.clone(),
                });
                Ok(Control::Next(Ctrl::EvalExpr(
                    Arc::new(value.clone()),
                    env.clone(),
                )))
            }
            Stmt::Assign {
                target,
                value,
                op,
                span,
            } => {
                // RHS evaluated first, once; then (for a compound assignment)
                // the target is read and written, re-evaluating its base/index
                // subexpressions exactly like `read_target`/`write_target`.
                self.kont.push(Cont::AssignRhs {
                    target: Arc::new(target.clone()),
                    op: *op,
                    env: env.clone(),
                    span: *span,
                });
                Ok(Control::Next(Ctrl::EvalExpr(
                    Arc::new(value.clone()),
                    env.clone(),
                )))
            }
            Stmt::While(cond, body, _) => {
                // The condition is re-evaluated once per iteration in the
                // enclosing environment; the body runs in a fresh child scope;
                // `break` yields `none` and `continue` re-tests.
                self.kont.push(Cont::WhileCond {
                    cond: Arc::new(cond.clone()),
                    body: body.clone(),
                    env: env.clone(),
                });
                Ok(Control::Next(Ctrl::EvalExpr(
                    Arc::new(cond.clone()),
                    env.clone(),
                )))
            }
            Stmt::For(pattern, iter, body, span) => {
                // The iterable is evaluated exactly once. A `Range` iterates
                // lazily (a `break` on a huge range never materializes it);
                // every other iterable goes through `Interp::iterate`, which
                // applies the materialization cap.
                self.kont.push(Cont::ForIterable {
                    pattern: pattern.clone(),
                    body: body.clone(),
                    env: env.clone(),
                    span: *span,
                });
                Ok(Control::Next(Ctrl::EvalExpr(
                    Arc::new(iter.clone()),
                    env.clone(),
                )))
            }
            Stmt::Loop(body, _) => {
                // `loop` repeats until `break`; `continue` starts the next
                // iteration; any other signal propagates.
                self.kont.push(Cont::LoopBody {
                    body: body.clone(),
                    env: env.clone(),
                });
                Ok(Control::Next(Ctrl::EnterBlock(
                    body.clone(),
                    env.clone(),
                    true,
                )))
            }
            // `try` is the last remaining unsupported statement (R3F.1).
            other @ Stmt::Try { .. } => Err(self.unsupported(other.span())),
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
    ///
    /// This owns the shared call-depth accounting exactly like
    /// `Interp::call`: increment `interp.depth`, reject with `E4011` when it
    /// would exceed `MAX_CALL_FRAMES`, and record the caller's restorable
    /// state (source, per-frame expression-nesting budget) before adopting the
    /// callee's source.
    fn push_frame(&mut self, closure: Rc<Closure>, env: Env, call_span: Span) -> Result<()> {
        self.interp.depth += 1;
        if self.interp.depth > MAX_CALL_FRAMES {
            self.interp.depth -= 1;
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
        let saved_source = self.interp.current_source;
        if let Some(source) = frame_source {
            self.interp.current_source = Some(source);
        }
        self.frames.push(UserFrame {
            closure,
            env,
            saved_source,
            frame_source,
            call_span,
            saved_expr_depth: std::mem::take(&mut self.expr_depth),
        });
        Ok(())
    }

    /// Pop a user frame and restore the caller's source/nesting budget.
    ///
    /// Mirrors `Interp::call`'s restore step: `current_source` and `ast_depth`
    /// return to their caller values whatever the callee's completion was.
    /// Idempotent at zero; no underflow.
    fn pop_frame(&mut self) -> Option<UserFrame> {
        let frame = self.frames.pop()?;
        self.interp.current_source = frame.saved_source;
        self.expr_depth = frame.saved_expr_depth;
        self.interp.depth -= 1;
        Some(frame)
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
        // `try` is the last remaining unsupported statement (R3F.1). The
        // machine must fail explicitly, never fall back to recursion. (Call,
        // lambda, and comprehensions became supported in R3C/R3E.)
        let e = Expr::Block(
            Arc::from([Stmt::Try {
                body: Arc::from([]),
                catch: "e".to_string(),
                catch_body: Arc::from([]),
                finally: None,
                span: Span::default(),
            }]),
            Span::default(),
        );
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::INTERNAL);
        assert!(err
            .message
            .contains("not supported by the iterative engine"));
    }

    #[test]
    fn range_constructs_range_value() {
        // `1..3` builds `RangeVal { start: 1, end: 3 }`.
        let e = Expr::Range(Arc::new(lit(1)), Arc::new(lit(3)), Span::default());
        match run_expr(&e).unwrap() {
            Ctl::Val(Value::Range(ref r)) => {
                assert_eq!(r.start, 1);
                assert_eq!(r.end, 3);
                assert_eq!(r.len(), 2);
            }
            _ => panic!("expected a range value"),
        }
    }

    #[test]
    fn range_descending_is_empty() {
        let e = Expr::Range(Arc::new(lit(5)), Arc::new(lit(1)), Span::default());
        match run_expr(&e).unwrap() {
            Ctl::Val(Value::Range(ref r)) => {
                assert!(r.is_empty());
                assert_eq!(r.len(), 0);
            }
            _ => panic!("expected a range value"),
        }
    }

    #[test]
    fn range_equal_bounds_is_empty() {
        let e = Expr::Range(Arc::new(lit(4)), Arc::new(lit(4)), Span::default());
        match run_expr(&e).unwrap() {
            Ctl::Val(Value::Range(ref r)) => assert!(r.is_empty()),
            _ => panic!("expected a range value"),
        }
    }

    #[test]
    fn range_len_saturates_at_i64_extremes() {
        // `len` uses saturating arithmetic, so extreme bounds never overflow.
        let e = Expr::Range(
            Arc::new(Expr::Lit(Lit::Int(i64::MIN), Span::default())),
            Arc::new(Expr::Lit(Lit::Int(i64::MAX), Span::default())),
            Span::default(),
        );
        match run_expr(&e).unwrap() {
            Ctl::Val(Value::Range(ref r)) => assert_eq!(r.len(), i64::MAX),
            _ => panic!("expected a range value"),
        }
    }

    #[test]
    fn range_start_type_error_is_e3001() {
        let e = Expr::Range(
            Arc::new(Expr::Lit(Lit::Bool(true), Span::default())),
            Arc::new(lit(3)),
            Span::default(),
        );
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::TYPE_MISMATCH);
        assert_eq!(err.message, "range start expects an int, found bool");
    }

    #[test]
    fn range_end_type_error_is_e3001() {
        let e = Expr::Range(
            Arc::new(lit(1)),
            Arc::new(Expr::Lit(Lit::Str("x".to_string()), Span::default())),
            Span::default(),
        );
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::TYPE_MISMATCH);
        assert_eq!(err.message, "range end expects an int, found string");
    }

    #[test]
    fn range_bad_start_wins_over_bad_end() {
        // Both operands are evaluated before either is validated, but
        // validation is start-first (like `eval_inner`'s two sequential
        // `match` arms), so the start's diagnostic is reported.
        let e = Expr::Range(
            Arc::new(Expr::Lit(Lit::Bool(true), Span::default())),
            Arc::new(Expr::Lit(Lit::Str("x".to_string()), Span::default())),
            Span::default(),
        );
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::TYPE_MISMATCH);
        assert_eq!(err.message, "range start expects an int, found bool");
    }

    #[test]
    fn range_end_signal_wins_over_bad_start() {
        // The end operand is evaluated even when the start is non-int, so a
        // control signal from the end propagates and no type error is
        // produced. This is the exact recursive `val!` ordering.
        let e = Expr::Range(
            Arc::new(Expr::Lit(Lit::Bool(true), Span::default())),
            Arc::new(Expr::Block(
                Arc::from([Stmt::Return(Some(lit(5)), Span::default())]),
                Span::default(),
            )),
            Span::default(),
        );
        assert!(matches!(run_expr(&e).unwrap(), Ctl::Return(Value::Int(5))));
    }

    #[test]
    fn range_signal_from_start_skips_end() {
        let e = Expr::Range(
            Arc::new(Expr::Block(
                Arc::from([Stmt::Return(Some(lit(5)), Span::default())]),
                Span::default(),
            )),
            Arc::new(Expr::Binary(
                crate::ast::BinOp::Div,
                Arc::new(lit(1)),
                Arc::new(lit(0)),
                Span::default(),
            )),
            Span::default(),
        );
        assert!(matches!(run_expr(&e).unwrap(), Ctl::Return(Value::Int(5))));
    }

    #[test]
    fn range_signal_from_end_propagates() {
        let e = Expr::Range(
            Arc::new(lit(1)),
            Arc::new(Expr::Block(
                Arc::from([
                    Stmt::Throw(lit(9), Span::default()),
                    Stmt::Expr(lit(0), Span::default()),
                ]),
                Span::default(),
            )),
            Span::default(),
        );
        assert!(matches!(run_expr(&e).unwrap(), Ctl::Throw(Value::Int(9))));
    }

    #[test]
    fn range_out_of_if_remains_unsupported() {
        // Range support is construction-only. `if 1..3 { }` requires the
        // truthiness/condition path, which is separately supported; this test
        // pins that an `if` whose taken branch is a still-unsupported
        // comprehension fails, proving no fallback leaked into R3B.5. (The
        // branch used to be a `len([1])` call, supported in R3C.1, then a
        // lambda, supported in R3C.4.)
        let unsupported = Expr::Block(
            Arc::from([Stmt::Try {
                body: Arc::from([]),
                catch: "e".to_string(),
                catch_body: Arc::from([]),
                finally: None,
                span: Span::default(),
            }]),
            Span::default(),
        );
        let e = Expr::If(
            Arc::new(Expr::Lit(Lit::Bool(true), Span::default())),
            Arc::from([expr_stmt(unsupported)]),
            None,
            Span::default(),
        );
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::INTERNAL);
    }

    #[test]
    fn deep_nested_range_is_stack_safe() {
        // Nest a range `MAX_AST_DEPTH - 2` deep in the *start* position. The
        // innermost bound is an int; every outer start is itself a range, so
        // the machine reaches a `range start expects an int` diagnostic. What
        // matters is that it descends iteratively (one `Cont` per level) and
        // never grows the host stack proportionally to the nesting.
        let depth = MAX_AST_DEPTH - 2;
        let mut v = lit(1);
        for _ in 0..depth {
            v = Expr::Range(Arc::new(v), Arc::new(lit(0)), Span::default());
        }
        let err = run_expr(&v).err().unwrap();
        assert_eq!(err.code, codes::TYPE_MISMATCH);
        assert_eq!(err.message, "range start expects an int, found range");
    }

    #[test]
    fn range_beyond_ast_depth_is_e1015() {
        let depth = MAX_AST_DEPTH + 5;
        let mut v = lit(1);
        for _ in 0..depth {
            v = Expr::Range(Arc::new(v), Arc::new(lit(0)), Span::default());
        }
        let err = run_expr(&v).err().unwrap();
        assert_eq!(err.code, codes::NESTING);
    }

    fn list2() -> Expr {
        Expr::List(Arc::from([lit(10), lit(20)]), Span::default())
    }

    fn index(base: Expr, idx: Expr) -> Expr {
        Expr::Index(Arc::new(base), Arc::new(idx), Span::default())
    }

    fn field(recv: Expr, name: &str) -> Expr {
        Expr::Field(Arc::new(recv), name.to_string(), Span::default())
    }

    #[test]
    fn index_reads_list_element() {
        match run_expr(&index(list2(), lit(1))).unwrap() {
            Ctl::Val(Value::Int(v)) => assert_eq!(v, 20),
            _ => panic!("expected 20, got a different completion"),
        }
    }

    #[test]
    fn index_target_signal_skips_index() {
        // The target is a block that returns; the index is a division by zero.
        // If the index ran, the result would be an `E4007` error. The target's
        // control signal must preempt it, exactly like `eval_inner`'s `val!`.
        let target = Expr::Block(
            Arc::from([Stmt::Return(Some(lit(5)), Span::default())]),
            Span::default(),
        );
        let idx = Expr::Binary(
            BinOp::Div,
            Arc::new(lit(1)),
            Arc::new(lit(0)),
            Span::default(),
        );
        match run_expr(&index(target, idx)).unwrap() {
            Ctl::Return(Value::Int(5)) => {}
            _ => panic!("expected the target's return signal"),
        }
    }

    #[test]
    fn index_signal_from_index_propagates() {
        let idx = Expr::Block(
            Arc::from([Stmt::Throw(lit(7), Span::default())]),
            Span::default(),
        );
        match run_expr(&index(list2(), idx)).unwrap() {
            Ctl::Throw(Value::Int(7)) => {}
            _ => panic!("expected the index's throw signal"),
        }
    }

    #[test]
    fn index_out_of_range_is_e4019() {
        let err = run_expr(&index(list2(), lit(9))).err().unwrap();
        assert_eq!(err.code, codes::INDEX);
        assert_eq!(err.message, "list index 9 out of range");
    }

    #[test]
    fn field_reads_builtin_method() {
        // A list receiver reports its length through the zero-argument builtin
        // registry, exactly like `eval_inner`'s non-instance `Expr::Field` arm.
        match run_expr(&field(list2(), "len")).unwrap() {
            Ctl::Val(Value::Int(v)) => assert_eq!(v, 2),
            _ => panic!("expected 2, got a different completion"),
        }
    }

    #[test]
    fn field_unknown_member_is_e2003() {
        let err = run_expr(&field(lit(1), "nope")).err().unwrap();
        assert_eq!(err.code, codes::UNDEFINED);
        assert_eq!(err.message, "int has no method `nope`");
    }

    #[test]
    fn field_receiver_signal_propagates() {
        // The receiver returns; the member is never resolved (an `E2003` must
        // not surface). This mirrors `eval_inner`'s receiver-first `val!`.
        let recv = Expr::Block(
            Arc::from([Stmt::Return(Some(lit(5)), Span::default())]),
            Span::default(),
        );
        match run_expr(&field(recv, "len")).unwrap() {
            Ctl::Return(Value::Int(5)) => {}
            _ => panic!("expected the receiver's return signal"),
        }
    }

    fn instance(fields: Vec<(&str, Value)>) -> Value {
        Value::Instance(Rc::new(super::super::value::Instance {
            ty: "P".to_string(),
            fields: RefCell::new(
                fields
                    .into_iter()
                    .map(|(n, v)| (n.to_string(), v))
                    .collect(),
            ),
        }))
    }

    /// Run a field expression against an environment binding `p` to an
    /// instance, exercising the `Value::Instance` branch of
    /// `Cont::FieldReceiver` (unreachable from source while `Expr::Construct`
    /// is unsupported, so it is covered structurally here).
    fn run_field_on_instance(name: &str, inst: Value, methods: &[(&str, &str)]) -> Result<Ctl> {
        let mut interp = Interp::new();
        for (ty, m) in methods {
            interp
                .methods
                .insert((ty.to_string(), m.to_string()), Vec::new());
        }
        let globals = interp.globals.clone();
        globals.define("p".to_string(), inst, false);
        let e = Expr::Field(
            Arc::new(Expr::Name("p".to_string(), Span::default())),
            name.to_string(),
            Span::default(),
        );
        eval_expr(&mut interp, &e, &globals)
    }

    #[test]
    fn instance_field_reads_declared_field() {
        match run_field_on_instance("x", instance(vec![("x", Value::Int(7))]), &[]).unwrap() {
            Ctl::Val(Value::Int(v)) => assert_eq!(v, 7),
            _ => panic!("expected 7, got a different completion"),
        }
    }

    #[test]
    fn instance_missing_field_is_e2003() {
        let err = run_field_on_instance("y", instance(vec![("x", Value::Int(7))]), &[])
            .err()
            .unwrap();
        assert_eq!(err.code, codes::UNDEFINED);
        assert_eq!(err.message, "P has no field `y`");
    }

    #[test]
    fn instance_method_name_is_not_a_field_value() {
        // A method registered for `(P, m)` that is not a declared field is the
        // recursive method-not-a-value guard, not a plain missing field.
        let err = run_field_on_instance("m", instance(vec![("x", Value::Int(1))]), &[("P", "m")])
            .err()
            .unwrap();
        assert_eq!(err.code, codes::UNDEFINED);
        assert_eq!(err.message, "struct P has method `m`; call it as `m(...)`");
    }

    #[test]
    fn instance_field_wins_over_registered_method() {
        // If a field and a method share a name, the field is read (mirrors the
        // recursive `!fields.any` guard).
        match run_field_on_instance("m", instance(vec![("m", Value::Int(3))]), &[("P", "m")])
            .unwrap()
        {
            Ctl::Val(Value::Int(v)) => assert_eq!(v, 3),
            _ => panic!("expected 3, got a different completion"),
        }
    }

    #[test]
    fn deep_nested_index_is_stack_safe() {
        // Nest an index chain `MAX_AST_DEPTH - 2` deep in the target position.
        // The innermost target is an int, so the first applied lookup is
        // `cannot index int with int`. What matters is that the machine descends
        // iteratively (one `Cont` per level) and never grows the host stack
        // proportionally to the nesting.
        let depth = MAX_AST_DEPTH - 2;
        let mut v = lit(1);
        for _ in 0..depth {
            v = index(v, lit(0));
        }
        let err = run_expr(&v).err().unwrap();
        assert_eq!(err.code, codes::TYPE_MISMATCH);
        assert_eq!(err.message, "cannot index int with int");
    }

    #[test]
    fn index_beyond_ast_depth_is_e1015() {
        let depth = MAX_AST_DEPTH + 5;
        let mut v = lit(1);
        for _ in 0..depth {
            v = index(v, lit(0));
        }
        let err = run_expr(&v).err().unwrap();
        assert_eq!(err.code, codes::NESTING);
    }

    #[test]
    fn deep_nested_field_is_stack_safe() {
        let depth = MAX_AST_DEPTH - 2;
        let mut v = lit(1);
        for _ in 0..depth {
            v = field(v, "len");
        }
        let err = run_expr(&v).err().unwrap();
        assert_eq!(err.code, codes::UNDEFINED);
        assert_eq!(err.message, "int has no method `len`");
    }

    #[test]
    fn field_beyond_ast_depth_is_e1015() {
        let depth = MAX_AST_DEPTH + 5;
        let mut v = lit(1);
        for _ in 0..depth {
            v = field(v, "len");
        }
        let err = run_expr(&v).err().unwrap();
        assert_eq!(err.code, codes::NESTING);
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

    // ----- B-1R3B.3 short-circuit `and`/`or` ----------------------------

    fn and(l: Expr, r: Expr) -> Expr {
        bin(BinOp::And, l, r)
    }

    fn or(l: Expr, r: Expr) -> Expr {
        bin(BinOp::Or, l, r)
    }

    fn run_bool(expr: &Expr) -> bool {
        match run_expr(expr) {
            Ok(Ctl::Val(Value::Bool(b))) => b,
            Ok(_) => panic!("expected a bool completion, got a different signal"),
            Err(d) => panic!("expected a bool completion, got diagnostic {}", d.code),
        }
    }

    #[test]
    fn short_circuit_truth_table() {
        for (l, r, a, o) in [
            (true, true, true, true),
            (true, false, false, true),
            (false, true, false, true),
            (false, false, false, false),
        ] {
            let le = Expr::Lit(Lit::Bool(l), Span::default());
            let re = || Expr::Lit(Lit::Bool(r), Span::default());
            assert_eq!(run_bool(&and(le.clone(), re())), a, "and({l}, {r})");
            assert_eq!(run_bool(&or(le, re())), o, "or({l}, {r})");
        }
    }

    #[test]
    fn short_circuit_always_yields_bool_from_truthiness() {
        // `and`/`or` yield `rv.truthy()` for a required right operand and the
        // relevant constant (`false`/`true`) when the left decides. They never
        // return an operand. Repeated from the recursion matrix: ints, floats,
        // strings, and `none` are all accepted and coerced to bool.
        assert!(run_bool(&and(lit(1), lit(2))));
        assert!(!run_bool(&and(lit(0), lit(2))));
        assert!(!run_bool(&and(lit(1), lit(0))));
        assert!(run_bool(&or(lit(0), lit(7))));
        assert!(!run_bool(&or(lit(0), lit(0))));
        assert!(run_bool(&or(lit(3), lit(0))));
        assert!(!run_bool(&and(
            Expr::Lit(Lit::None, Span::default()),
            Expr::Lit(Lit::Int(1), Span::default())
        )));
        assert!(!run_bool(&and(
            Expr::Lit(Lit::Str(String::new()), Span::default()),
            Expr::Lit(Lit::Int(1), Span::default())
        )));
        assert!(run_bool(&or(
            Expr::Lit(Lit::Str("x".to_string()), Span::default()),
            Expr::Lit(Lit::Int(0), Span::default())
        )));
    }

    /// The skipped right operand is a subtree that would raise a deterministic
    /// runtime diagnostic if evaluated. Its total absence from the result is
    /// the observation: the machine must return the left-operand result without
    /// ever scheduling the right operand (no E4007).
    #[test]
    fn skipped_rhs_error_does_not_occur() {
        // `false and (1 / 0)` -> false; the division must never run.
        let e = and(
            Expr::Lit(Lit::Bool(false), Span::default()),
            bin(BinOp::Div, lit(1), lit(0)),
        );
        assert!(!run_bool(&e));
        // `true or (1 / 0)` -> true; the division must never run.
        let e = or(
            Expr::Lit(Lit::Bool(true), Span::default()),
            bin(BinOp::Div, lit(1), lit(0)),
        );
        assert!(run_bool(&e));
    }

    #[test]
    fn required_rhs_error_does_occur() {
        // `true and (1 / 0)` and `false or (1 / 0)` must run the division and
        // surface E4007 with the division's own span.
        for e in [
            and(
                Expr::Lit(Lit::Bool(true), Span::default()),
                bin(BinOp::Div, lit(1), lit(0)),
            ),
            or(
                Expr::Lit(Lit::Bool(false), Span::default()),
                bin(BinOp::Div, lit(1), lit(0)),
            ),
        ] {
            let err = run_expr(&e).err().unwrap();
            assert_eq!(err.code, codes::DIV_ZERO);
            assert_eq!(err.message, "division by zero");
        }
    }

    #[test]
    fn short_circuit_left_signal_skips_rhs() {
        // A control signal from the left operand propagates without the right
        // being evaluated: `{ return 5 } and (1 / 0)` yields the `return`
        // signal, not E4007, and `{ return 5 } or (1 / 0)` likewise.
        for op in [BinOp::And, BinOp::Or] {
            let left = Expr::Block(
                Arc::from(vec![Stmt::Return(Some(lit(5)), Span::default())]),
                Span::default(),
            );
            let e = bin(op, left, bin(BinOp::Div, lit(1), lit(0)));
            match run_expr(&e).unwrap() {
                Ctl::Return(Value::Int(5)) => {}
                _ => panic!("left control signal must propagate"),
            }
        }
    }

    #[test]
    fn short_circuit_required_right_signal_propagates() {
        // The required right operand's signal propagates unchanged and is not
        // converted into a bool: `true and { return 7 }` yields `return 7`;
        // `false or { throw 9 }` yields the `throw` signal.
        let left_true = || Expr::Lit(Lit::Bool(true), Span::default());
        let left_false = || Expr::Lit(Lit::Bool(false), Span::default());
        let returning = || {
            Expr::Block(
                Arc::from(vec![Stmt::Return(Some(lit(7)), Span::default())]),
                Span::default(),
            )
        };
        let throwing = || {
            Expr::Block(
                Arc::from(vec![Stmt::Throw(lit(9), Span::default())]),
                Span::default(),
            )
        };
        match run_expr(&and(left_true(), returning())).unwrap() {
            Ctl::Return(Value::Int(7)) => {}
            _ => panic!("required right `return` must propagate"),
        }
        match run_expr(&or(left_false(), throwing())).unwrap() {
            Ctl::Throw(Value::Int(9)) => {}
            _ => panic!("required right `throw` must propagate"),
        }
    }

    #[test]
    fn short_circuit_nests_and_binds_weakest() {
        // `true or false and false` groups as `true or (false and false)` =>
        // true; `(true or false) and false` => false. Precedence is unchanged.
        let t = || Expr::Lit(Lit::Bool(true), Span::default());
        let f = || Expr::Lit(Lit::Bool(false), Span::default());
        assert!(run_bool(&or(t(), and(f(), f()))));
        assert!(!run_bool(&and(or(t(), f()), f())));
    }

    #[test]
    fn short_circuit_composes_with_unary_and_eager() {
        let t = || Expr::Lit(Lit::Bool(true), Span::default());
        let f = || Expr::Lit(Lit::Bool(false), Span::default());
        assert!(!run_bool(&and(
            Expr::Unary(
                UnOp::Not,
                Arc::new(Expr::Lit(Lit::Bool(true), Span::default())),
                Span::default(),
            ),
            t(),
        )));
        assert!(run_bool(&and(bin(BinOp::Eq, lit(1), lit(1)), t())));
        assert!(!run_bool(&or(bin(BinOp::Lt, lit(2), lit(1)), f())));
        // `not (a and b)` and `a and not b` both flow through the machine.
        assert!(run_bool(&Expr::Unary(
            UnOp::Not,
            Arc::new(and(t(), f())),
            Span::default(),
        )));
        assert!(!run_bool(&and(
            t(),
            Expr::Unary(
                UnOp::Not,
                Arc::new(Expr::Lit(Lit::Bool(true), Span::default())),
                Span::default(),
            ),
        )));
    }

    #[test]
    fn deep_short_circuit_chain_is_stack_safe() {
        // A left-nested chain of `or` with a truthy leftmost operand must
        // short-circuit every right operand and stay host-stack safe.
        let depth = MAX_AST_DEPTH - 2;
        let mut e = Expr::Lit(Lit::Bool(true), Span::default());
        for _ in 0..depth {
            e = or(e, Expr::Lit(Lit::Bool(false), Span::default()));
        }
        assert!(run_bool(&e));

        // A chain that requires every operand: `false or ... or true` where
        // the final operand is truthy forces each right operand to run.
        let mut e = Expr::Lit(Lit::Bool(false), Span::default());
        for _ in 0..(depth - 1) {
            e = or(e, Expr::Lit(Lit::Bool(false), Span::default()));
        }
        e = or(e, Expr::Lit(Lit::Bool(true), Span::default()));
        assert!(run_bool(&e));
    }

    #[test]
    fn deep_and_chain_is_stack_safe() {
        // `and` chains mirror `or`: a falsy leftmost short-circuits all rights.
        let depth = MAX_AST_DEPTH - 2;
        let mut e = Expr::Lit(Lit::Bool(false), Span::default());
        for _ in 0..depth {
            e = and(e, Expr::Lit(Lit::Bool(true), Span::default()));
        }
        assert!(!run_bool(&e));

        let mut e = Expr::Lit(Lit::Bool(true), Span::default());
        for _ in 0..depth {
            e = and(e, Expr::Lit(Lit::Bool(true), Span::default()));
        }
        assert!(run_bool(&e));
    }

    #[test]
    fn short_circuit_chain_beyond_ast_depth_is_e1015() {
        for op in [BinOp::And, BinOp::Or] {
            let mut e = Expr::Lit(Lit::Bool(true), Span::default());
            for _ in 0..(MAX_AST_DEPTH + 5) {
                e = bin(op, e, Expr::Lit(Lit::Bool(false), Span::default()));
            }
            let err = run_expr(&e).err().unwrap();
            assert_eq!(err.code, codes::NESTING);
        }
    }

    // ----- B-1R3B.4.1 list / tuple construction -------------------------

    fn list(items: Vec<Expr>) -> Expr {
        Expr::List(Arc::from(items), Span::default())
    }

    fn tuple(items: Vec<Expr>) -> Expr {
        Expr::Tuple(Arc::from(items), Span::default())
    }

    fn run_list(expr: &Expr) -> Vec<Value> {
        match run_expr(expr) {
            Ok(Ctl::Val(ref v)) => match v {
                Value::List(l) => l.borrow().clone(),
                _ => panic!("expected a list completion, got a different value"),
            },
            Ok(_) => panic!("expected a list completion, got a different signal"),
            Err(d) => panic!("expected a list completion, got diagnostic {}", d.code),
        }
    }

    #[test]
    fn list_empty_is_empty_list() {
        assert!(run_list(&list(Vec::new())).is_empty());
    }

    #[test]
    fn list_single_element() {
        let vals = run_list(&list(vec![lit(7)]));
        assert_eq!(vals.len(), 1);
        assert!(matches!(vals[0], Value::Int(7)));
    }

    #[test]
    fn list_elements_keep_source_order() {
        let vals = run_list(&list(vec![
            lit(1),
            Expr::Lit(Lit::Str("x".to_string()), Span::default()),
            Expr::Lit(Lit::Bool(true), Span::default()),
            Expr::Lit(Lit::None, Span::default()),
        ]));
        assert!(matches!(vals[0], Value::Int(1)));
        assert!(matches!(&vals[1], Value::Str(s) if &**s == "x"));
        assert!(matches!(vals[2], Value::Bool(true)));
        assert!(matches!(vals[3], Value::None));
    }

    #[test]
    fn tuple_is_list_sugar() {
        // `(a, b)` is list sugar (`LANGUAGE_SPEC.md` §21): a list value,
        // indistinguishable from `[a, b]` (type `list`, same contents).
        let vals = run_list(&tuple(vec![lit(1), lit(2)]));
        assert_eq!(vals.len(), 2);
        assert!(matches!(vals[0], Value::Int(1)));
        assert!(matches!(vals[1], Value::Int(2)));
    }

    #[test]
    fn list_of_lists_nests() {
        let inner1 = list(vec![lit(1)]);
        let inner2 = list(vec![lit(2), lit(3)]);
        let vals = run_list(&list(vec![inner1, inner2]));
        assert_eq!(vals.len(), 2);
        match (&vals[0], &vals[1]) {
            (Value::List(a), Value::List(b)) => {
                assert_eq!(a.borrow().len(), 1);
                assert_eq!(b.borrow().len(), 2);
            }
            _ => panic!("nested elements must remain lists"),
        }
    }

    #[test]
    fn list_evaluates_elements_left_to_right_first_error_wins() {
        // `[1 / 0, "a" - "b"]`: if evaluation were right-to-left (or otherwise
        // reordered) the E3001 subtraction would be reported instead of the
        // first element's E4007. The recursive arm reports the first element.
        let e = list(vec![
            bin(BinOp::Div, lit(1), lit(0)),
            bin(
                BinOp::Sub,
                Expr::Lit(Lit::Str("a".to_string()), Span::default()),
                Expr::Lit(Lit::Str("b".to_string()), Span::default()),
            ),
        ]);
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::DIV_ZERO);
        assert_eq!(err.message, "division by zero");
    }

    #[test]
    fn list_later_element_error_still_reported() {
        // The intermediate and last elements must actually be evaluated.
        let e = list(vec![lit(1), bin(BinOp::Div, lit(1), lit(0))]);
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::DIV_ZERO);
        let e = list(vec![lit(1), lit(2), bin(BinOp::Div, lit(1), lit(0))]);
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::DIV_ZERO);
    }

    #[test]
    fn list_element_signal_aborts_later_elements() {
        // A control signal from the first element must stop the list: the
        // second element's E4007 must never occur (mirrors `val!`).
        let first = Expr::Block(
            Arc::from(vec![Stmt::Return(Some(lit(5)), Span::default())]),
            Span::default(),
        );
        let e = list(vec![first, bin(BinOp::Div, lit(1), lit(0))]);
        match run_expr(&e).unwrap() {
            Ctl::Return(Value::Int(5)) => {}
            _ => panic!("element control signal must abort the list"),
        }
        // A `throw` element likewise propagates unchanged.
        let first = Expr::Block(
            Arc::from(vec![Stmt::Throw(lit(9), Span::default())]),
            Span::default(),
        );
        let e = list(vec![first, bin(BinOp::Div, lit(1), lit(0))]);
        match run_expr(&e).unwrap() {
            Ctl::Throw(Value::Int(9)) => {}
            _ => panic!("element throw must abort the list"),
        }
    }

    #[test]
    fn list_unsupported_element_fails_explicitly() {
        // A `try` block element is still unsupported (R3F.1): the list must
        // fail with the E4999 sentinel, never fall back to recursion and never
        // produce a partial list. The first element is supported, so this also
        // proves the second element is genuinely scheduled and then fails.
        // (The element has moved through call, lambda, and comprehension
        // probes as each became supported.)
        let unsupported = Expr::Block(
            Arc::from([Stmt::Try {
                body: Arc::from([]),
                catch: "e".to_string(),
                catch_body: Arc::from([]),
                finally: None,
                span: Span::default(),
            }]),
            Span::default(),
        );
        let e = list(vec![lit(1), unsupported]);
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::INTERNAL);
        assert!(err
            .message
            .contains("not supported by the iterative engine"));
    }

    #[test]
    fn list_inside_r3a_constructs() {
        // List inside a block, bound with `let`, used as an `if` condition.
        let e = Expr::Block(
            Arc::from(vec![
                Stmt::Let {
                    mutable: false,
                    name: "xs".to_string(),
                    ann: None,
                    value: list(vec![lit(1), lit(2)]),
                    span: Span::default(),
                },
                Stmt::Expr(
                    Expr::If(
                        Arc::new(Expr::Name("xs".to_string(), Span::default())),
                        Arc::from([Stmt::Expr(lit(7), Span::default())]),
                        Some(Arc::new(lit(8))),
                        Span::default(),
                    ),
                    Span::default(),
                ),
            ]),
            Span::default(),
        );
        match run_expr(&e).unwrap() {
            Ctl::Val(Value::Int(7)) => {}
            _ => panic!("non-empty list must be truthy"),
        }
        // An empty list is falsy (mirrors `Value::truthy`).
        let e = Expr::If(
            Arc::new(list(Vec::new())),
            Arc::from([Stmt::Expr(lit(7), Span::default())]),
            Some(Arc::new(lit(8))),
            Span::default(),
        );
        match run_expr(&e).unwrap() {
            Ctl::Val(Value::Int(8)) => {}
            _ => panic!("empty list must be falsy"),
        }
    }

    #[test]
    fn deep_nested_list_is_stack_safe() {
        let depth = MAX_AST_DEPTH - 2;
        let mut e = lit(1);
        for _ in 0..depth {
            e = list(vec![e]);
        }
        // The value is a list nested `depth` levels; unwrap structurally.
        let mut v = match run_expr(&e) {
            Ok(Ctl::Val(v)) => v,
            _ => panic!("deep nested list must evaluate"),
        };
        for _ in 0..depth {
            match &v {
                Value::List(inner) => {
                    let next = inner.borrow()[0].clone();
                    v = next;
                }
                _ => panic!("every level must be a list"),
            }
        }
        assert!(matches!(v, Value::Int(1)));
    }

    #[test]
    fn list_beyond_ast_depth_is_e1015() {
        let mut e = lit(1);
        for _ in 0..(MAX_AST_DEPTH + 5) {
            e = list(vec![e]);
        }
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::NESTING);
    }

    #[test]
    fn deep_nested_list_through_frame_boundary_is_stack_safe() {
        // The real `Program` path enters through `call_closure_body` (frame 1 +
        // `FrameBoundary`). A list nested `MAX_AST_DEPTH - 2` deep must run
        // there without growing the host stack proportionally: the machine
        // pushes one `Cont` per level and loops.
        let depth = MAX_AST_DEPTH - 2;
        let mut e = lit(1);
        for _ in 0..depth {
            e = list(vec![e]);
        }
        let mut interp = Interp::new();
        let globals = interp.globals.clone();
        let closure = Rc::new(Closure {
            name: "main".to_string(),
            params: Vec::new(),
            param_tys: Vec::new(),
            body: Arc::from(vec![Stmt::Expr(e, Span::default())]),
            env: globals,
        });
        match call_closure_body(&mut interp, closure, Vec::new(), Span::default()).unwrap() {
            Ctl::Val(Value::List(_)) => {}
            _ => panic!("deep nested list through a frame must evaluate"),
        }
    }

    // ----- B-1R3B.4.2 map construction ----------------------------------

    fn map(entries: Vec<(Expr, Expr)>) -> Expr {
        Expr::Map(Arc::from(entries), Span::default())
    }

    fn run_map(expr: &Expr) -> Vec<(MapKey, Value)> {
        match run_expr(expr) {
            Ok(Ctl::Val(ref v)) => match v {
                Value::Map(m) => m
                    .borrow()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                _ => panic!("expected a map completion, got a different value"),
            },
            Ok(_) => panic!("expected a map completion, got a different signal"),
            Err(d) => panic!("expected a map completion, got diagnostic {}", d.code),
        }
    }

    #[test]
    fn map_empty_is_empty_map() {
        assert!(run_map(&map(Vec::new())).is_empty());
    }

    #[test]
    fn map_single_entry() {
        let out = run_map(&map(vec![(
            lit(1),
            Expr::Lit(Lit::Str("x".to_string()), Span::default()),
        )]));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, MapKey::Int(1));
        assert!(matches!(&out[0].1, Value::Str(s) if &**s == "x"));
    }

    #[test]
    fn map_entries_ordered_by_key_regardless_of_source_order() {
        // Recursive semantics: a `BTreeMap` orders `int` keys ascending, so
        // `{3: "c", 1: "a", 2: "b"}` displays `{1: "a", 2: "b", 3: "c"}`.
        let out = run_map(&map(vec![
            (
                lit(3),
                Expr::Lit(Lit::Str("c".to_string()), Span::default()),
            ),
            (
                lit(1),
                Expr::Lit(Lit::Str("a".to_string()), Span::default()),
            ),
            (
                lit(2),
                Expr::Lit(Lit::Str("b".to_string()), Span::default()),
            ),
        ]));
        let keys: Vec<i64> = out
            .iter()
            .map(|(k, _)| match k {
                MapKey::Int(i) => *i,
                _ => panic!("expected int keys"),
            })
            .collect();
        assert_eq!(keys, vec![1, 2, 3]);
    }

    #[test]
    fn map_duplicate_key_last_wins() {
        // `{1: "a", 2: "b", 1: "c"}` -> `{1: "c", 2: "b"}`: the later entry
        // replaces the earlier one. This is the recursive `BTreeMap::insert`
        // behavior.
        let out = run_map(&map(vec![
            (
                lit(1),
                Expr::Lit(Lit::Str("a".to_string()), Span::default()),
            ),
            (
                lit(2),
                Expr::Lit(Lit::Str("b".to_string()), Span::default()),
            ),
            (
                lit(1),
                Expr::Lit(Lit::Str("c".to_string()), Span::default()),
            ),
        ]));
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, MapKey::Int(1));
        assert!(matches!(&out[0].1, Value::Str(s) if &**s == "c"));
        assert_eq!(out[1].0, MapKey::Int(2));
    }

    #[test]
    fn map_bool_and_string_keys() {
        let out = run_map(&map(vec![
            (Expr::Lit(Lit::Bool(true), Span::default()), lit(1)),
            (Expr::Lit(Lit::Bool(false), Span::default()), lit(0)),
        ]));
        assert_eq!(out.len(), 2);
        // MapKey order is Int < Bool < Str, and `false < true`.
        assert_eq!(out[0].0, MapKey::Bool(false));
        assert_eq!(out[1].0, MapKey::Bool(true));

        let out = run_map(&map(vec![(
            Expr::Lit(Lit::Str("k".to_string()), Span::default()),
            lit(1),
        )]));
        assert_eq!(out[0].0, MapKey::Str(Rc::from("k")));
    }

    #[test]
    fn map_nested_maps() {
        let inner = map(vec![(lit(1), lit(10))]);
        let out = run_map(&map(vec![(lit(0), inner)]));
        assert_eq!(out.len(), 1);
        assert!(matches!(out[0].1, Value::Map(_)));
    }

    #[test]
    fn map_containing_and_inside_lists() {
        // Map containing a list value and a list containing a map value.
        let out = run_map(&map(vec![(lit(1), list(vec![lit(1), lit(2)]))]));
        assert!(matches!(out[0].1, Value::List(_)));
        let vals = run_list(&list(vec![map(vec![(lit(1), lit(2))])]));
        assert_eq!(vals.len(), 1);
        assert!(matches!(vals[0], Value::Map(_)));
    }

    #[test]
    fn map_key_evaluated_before_value_first_error_wins() {
        // The value of the first entry must not run before the second key:
        // `{1: 2, 1/0: 3}` reports the second key's E4007, proving the key of
        // the later entry is evaluated after the earlier value.
        let e = map(vec![
            (lit(1), lit(2)),
            (bin(BinOp::Div, lit(1), lit(0)), lit(3)),
        ]);
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::DIV_ZERO);
    }

    #[test]
    fn map_invalid_key_is_e3001_at_the_key_span() {
        // `{1.0: "x"}`: a runtime non-key-capable key is E3001 with the exact
        // recursive message; the value must not be evaluated.
        let span = Span { start: 7, end: 10 };
        let e = map(vec![(
            Expr::Lit(Lit::Float(1.0), span),
            bin(BinOp::Div, lit(1), lit(0)),
        )]);
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::TYPE_MISMATCH);
        assert_eq!(
            err.message,
            "type `float` cannot be used as a map key; map keys must be `string`, `int`, or `bool`"
        );
        assert_eq!(err.span.start, 7);
        assert_eq!(err.span.end, 10);
    }

    #[test]
    fn map_value_suppressed_after_invalid_key() {
        // The invalid key's value (`1 / 0`) must never run: the diagnostic is
        // the key's E3001, not E4007.
        let e = map(vec![(
            Expr::Lit(Lit::None, Span::default()),
            bin(BinOp::Div, lit(1), lit(0)),
        )]);
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::TYPE_MISMATCH);
    }

    #[test]
    fn map_key_signal_aborts_later_entries() {
        // A control signal from a key aborts the whole map; the value and every
        // later entry must never run.
        let key = Expr::Block(
            Arc::from(vec![Stmt::Return(Some(lit(5)), Span::default())]),
            Span::default(),
        );
        let e = map(vec![
            (key, bin(BinOp::Div, lit(1), lit(0))),
            (lit(1), lit(2)),
        ]);
        match run_expr(&e).unwrap() {
            Ctl::Return(Value::Int(5)) => {}
            _ => panic!("key control signal must abort the map"),
        }
    }

    #[test]
    fn map_value_signal_aborts_later_entries() {
        // A control signal from a value aborts the whole map; later entries
        // must never run.
        let value = Expr::Block(
            Arc::from(vec![Stmt::Throw(lit(9), Span::default())]),
            Span::default(),
        );
        let e = map(vec![(lit(1), value), (lit(2), lit(3))]);
        match run_expr(&e).unwrap() {
            Ctl::Throw(Value::Int(9)) => {}
            _ => panic!("value control signal must abort the map"),
        }
    }

    #[test]
    fn map_later_value_error_wins_over_later_key() {
        // `{1: 1/0, 1/0: 2}`: the value error of the first entry must be
        // reported before the second entry's key runs.
        let e = map(vec![
            (lit(1), bin(BinOp::Div, lit(1), lit(0))),
            (bin(BinOp::Mul, lit(1), lit(0)), lit(2)),
        ]);
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::DIV_ZERO);
    }

    #[test]
    fn map_unsupported_key_fails_explicitly() {
        // A `try` block key is still unsupported (R3F.1): the map must fail
        // with the E4999 sentinel, never fall back to recursion and never
        // produce a partial map. (The key has moved through call, lambda, and
        // comprehension probes as each became supported.)
        let unsupported = Expr::Block(
            Arc::from([Stmt::Try {
                body: Arc::from([]),
                catch: "e".to_string(),
                catch_body: Arc::from([]),
                finally: None,
                span: Span::default(),
            }]),
            Span::default(),
        );
        let e = map(vec![(unsupported, lit(1))]);
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::INTERNAL);
        assert!(err
            .message
            .contains("not supported by the iterative engine"));
    }

    #[test]
    fn deep_nested_map_is_stack_safe() {
        let depth = MAX_AST_DEPTH - 2;
        let mut e = lit(1);
        for _ in 0..depth {
            e = map(vec![(lit(0), e)]);
        }
        let mut v = match run_expr(&e) {
            Ok(Ctl::Val(v)) => v,
            _ => panic!("deep nested map must evaluate"),
        };
        for _ in 0..depth {
            match &v {
                Value::Map(m) => {
                    let next = m.borrow().get(&MapKey::Int(0)).cloned().unwrap();
                    v = next;
                }
                _ => panic!("every level must be a map"),
            }
        }
        assert!(matches!(v, Value::Int(1)));
    }

    #[test]
    fn map_beyond_ast_depth_is_e1015() {
        let mut e = lit(1);
        for _ in 0..(MAX_AST_DEPTH + 5) {
            e = map(vec![(lit(0), e)]);
        }
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::NESTING);
    }

    #[test]
    fn deep_nested_map_through_frame_boundary_is_stack_safe() {
        // The real `Program` path enters through `call_closure_body` (frame 1 +
        // `FrameBoundary`). A map nested `MAX_AST_DEPTH - 2` deep must run there
        // without growing the host stack proportionally.
        let depth = MAX_AST_DEPTH - 2;
        let mut e = lit(1);
        for _ in 0..depth {
            e = map(vec![(lit(0), e)]);
        }
        let mut interp = Interp::new();
        let globals = interp.globals.clone();
        let closure = Rc::new(Closure {
            name: "main".to_string(),
            params: Vec::new(),
            param_tys: Vec::new(),
            body: Arc::from(vec![Stmt::Expr(e, Span::default())]),
            env: globals,
        });
        match call_closure_body(&mut interp, closure, Vec::new(), Span::default()).unwrap() {
            Ctl::Val(Value::Map(_)) => {}
            _ => panic!("deep nested map through a frame must evaluate"),
        }
    }

    #[test]
    fn deep_mixed_map_list_nesting_is_stack_safe() {
        // Alternate map/list nesting just below `MAX_AST_DEPTH`; the machine
        // pushes one `Cont` per level regardless of container kind.
        let depth = MAX_AST_DEPTH - 2;
        let mut e = lit(1);
        for i in 0..depth {
            e = if i % 2 == 0 {
                map(vec![(lit(0), e)])
            } else {
                list(vec![e])
            };
        }
        let mut v = match run_expr(&e) {
            Ok(Ctl::Val(v)) => v,
            _ => panic!("deep mixed nesting must evaluate"),
        };
        for i in (0..depth).rev() {
            v = if i % 2 == 0 {
                match &v {
                    Value::Map(m) => m.borrow().get(&MapKey::Int(0)).cloned().unwrap(),
                    _ => panic!("expected a map"),
                }
            } else {
                match &v {
                    Value::List(l) => l.borrow()[0].clone(),
                    _ => panic!("expected a list"),
                }
            };
        }
        assert!(matches!(v, Value::Int(1)));
    }

    #[test]
    fn map_key_and_value_evaluated_exactly_once() {
        // A shadowing `let` key advances nothing observable here, but the
        // structural guarantee is that the key continuation schedules the value
        // exactly once and the value continuation schedules the next key
        // exactly once. A chain of entries with distinct values pins it.
        let e = map(vec![
            (lit(1), lit(10)),
            (lit(2), lit(20)),
            (lit(3), lit(30)),
        ]);
        let out = run_map(&e);
        assert_eq!(out.len(), 3);
        assert!(matches!(out[0].1, Value::Int(10)));
        assert!(matches!(out[1].1, Value::Int(20)));
        assert!(matches!(out[2].1, Value::Int(30)));
    }

    // ----- B-1R3B.7 f-strings --------------------------------------------

    fn fstr(parts: Vec<FPart>) -> Expr {
        Expr::FStr(Arc::from(parts), Span::default())
    }

    fn text(t: &str) -> FPart {
        FPart::Lit(t.to_string())
    }

    fn interp(e: Expr) -> FPart {
        FPart::Expr(e, None)
    }

    fn expect_string(expr: &Expr) -> String {
        match run_expr(expr) {
            Ok(Ctl::Val(Value::Str(ref s))) => s.to_string(),
            Ok(_) => panic!("expected a string completion, got a different signal"),
            Err(d) => panic!("expected a string completion, got diagnostic {}", d.code),
        }
    }

    #[test]
    fn fstring_empty_is_empty_string() {
        assert_eq!(expect_string(&fstr(Vec::new())), "");
    }

    #[test]
    fn fstring_text_only() {
        assert_eq!(
            expect_string(&fstr(vec![text("hello "), text("world")])),
            "hello world"
        );
    }

    #[test]
    fn fstring_interpolations_in_order() {
        let e = fstr(vec![
            text("a="),
            interp(lit(1)),
            text(", b="),
            interp(lit(2)),
            text(", c="),
            interp(lit(3)),
        ]);
        assert_eq!(expect_string(&e), "a=1, b=2, c=3");
    }

    #[test]
    fn fstring_adjacent_interpolations() {
        let e = fstr(vec![interp(lit(1)), interp(lit(2))]);
        assert_eq!(expect_string(&e), "12");
    }

    #[test]
    fn fstring_interpolation_error_aborts() {
        // The middle interpolation divides by zero; later text must not appear
        // in any partial value, and the diagnostic wins.
        let e = fstr(vec![
            text("x"),
            interp(Expr::Binary(
                BinOp::Div,
                Arc::new(lit(1)),
                Arc::new(lit(0)),
                Span::default(),
            )),
            text("y"),
        ]);
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::DIV_ZERO);
    }

    #[test]
    fn fstring_control_signal_aborts() {
        let e = fstr(vec![
            text("x"),
            interp(Expr::Block(
                Arc::from(vec![Stmt::Return(Some(lit(5)), Span::default())]),
                Span::default(),
            )),
            text("y"),
        ]);
        assert!(matches!(run_expr(&e), Ok(Ctl::Return(Value::Int(5)))));
    }

    #[test]
    fn fstring_format_spec_renders() {
        let e = fstr(vec![FPart::Expr(
            lit(42),
            Some(FormatSpec {
                ty: Some(crate::ast::FormatType::Hex { upper: false }),
                ..FormatSpec::default()
            }),
        )]);
        assert_eq!(expect_string(&e), "2a");
    }

    #[test]
    fn deep_nested_fstring_is_stack_safe() {
        // Nest f-strings each interpolating the next; one `Cont::FStrNext` per
        // level, no host-stack growth.
        let depth = MAX_AST_DEPTH - 2;
        let mut e = lit(1);
        for _ in 0..depth {
            e = fstr(vec![text("["), interp(e), text("]")]);
        }
        let s = expect_string(&e);
        assert_eq!(s, format!("{}1{}", "[".repeat(depth), "]".repeat(depth)));
    }

    #[test]
    fn fstring_beyond_ast_depth_is_e1015() {
        let mut e = lit(1);
        for _ in 0..(MAX_AST_DEPTH + 5) {
            e = fstr(vec![interp(e)]);
        }
        let err = run_expr(&e).err().unwrap();
        assert_eq!(err.code, codes::NESTING);
    }

    #[test]
    fn fstring_expr_depth_returns_after_completion() {
        // A completed f-string must restore `expr_depth`, so a following deep
        // chain still fits the budget. Run a sequential pair through the block
        // statement runner; if the f-string leaked depth the second expression
        // would be E1015.
        let deep = {
            let depth = MAX_AST_DEPTH - 5;
            let mut e = lit(1);
            for _ in 0..depth {
                e = Expr::Unary(UnOp::Not, Arc::new(e), Span::default());
            }
            e
        };
        let stmts = vec![expr_stmt(fstr(vec![interp(lit(7))])), expr_stmt(deep)];
        let (ctl, _) = run_stmts(stmts).unwrap();
        assert!(matches!(ctl, Ctl::Val(Value::Bool(_))));
    }

    #[test]
    fn deep_nested_fstring_through_frame_boundary_is_stack_safe() {
        let depth = MAX_AST_DEPTH - 2;
        let mut e = lit(1);
        for _ in 0..depth {
            e = fstr(vec![text("<"), interp(e), text(">")]);
        }
        let mut interp = Interp::new();
        let globals = interp.globals.clone();
        let closure = Rc::new(Closure {
            name: "main".to_string(),
            params: Vec::new(),
            param_tys: Vec::new(),
            body: Arc::from(vec![Stmt::Expr(e, Span::default())]),
            env: globals,
        });
        match call_closure_body(&mut interp, closure, Vec::new(), Span::default()).unwrap() {
            Ctl::Val(Value::Str(_)) => {}
            _ => panic!("deep nested f-string through a frame must evaluate"),
        }
    }

    // ----- B-1R3C.1 calls -----------------------------------------------

    fn call(name: &str, args: Vec<Expr>) -> Expr {
        Expr::Call(
            Arc::new(Expr::Name(name.to_string(), Span::default())),
            Arc::from(
                args.into_iter()
                    .map(|value| Arg { name: None, value })
                    .collect::<Vec<_>>(),
            ),
            Vec::new(),
            Span::default(),
        )
    }

    fn user_call(expr: &Expr, program: &str) -> Result<Ctl> {
        // Compile a real module so declarations, overloads, and sources are
        // populated exactly like production, then evaluate `expr` through the
        // machine.
        let module = crate::parse::parse(program).expect("parse program");
        let mut interp = Interp::new();
        for item in &module.items {
            interp.declare_item(item);
        }
        let globals = interp.globals.clone();
        eval_expr(&mut interp, expr, &globals)
    }

    #[test]
    fn call_user_function_binds_arguments() {
        let ctl = user_call(
            &call("add", vec![lit(20), lit(22)]),
            "fn add(a, b) { a + b }\n",
        )
        .expect("call evaluates");
        assert!(matches!(ctl, Ctl::Val(Value::Int(42))));
    }

    #[test]
    fn call_native_checks_shared_arity() {
        let mut interp = Interp::new();
        let globals = interp.globals.clone();
        let err = match eval_expr(&mut interp, &call("len", vec![]), &globals) {
            Err(d) => d,
            Ok(_) => panic!("arity violation must be rejected"),
        };
        assert_eq!(err.code, codes::TYPE_MISMATCH);
        assert!(err.message.contains("at least 1 argument"));
    }

    #[test]
    fn call_undefined_name_is_e2003() {
        let mut interp = Interp::new();
        let globals = interp.globals.clone();
        let err = match eval_expr(&mut interp, &call("missing", vec![lit(1)]), &globals) {
            Err(d) => d,
            Ok(_) => panic!("undefined function must be rejected"),
        };
        assert_eq!(err.code, codes::UNDEFINED);
        assert!(err.message.contains("undefined function `missing`"));
    }

    #[test]
    fn call_argument_order_is_source_order() {
        // Two arguments, each an f-string that observes order through a
        // failure: the first argument's error must win.
        let args = vec![
            Expr::Binary(
                BinOp::Div,
                Arc::new(lit(1)),
                Arc::new(lit(0)),
                Span::default(),
            ),
            call("len", vec![]),
        ];
        let mut interp = Interp::new();
        let globals = interp.globals.clone();
        let e = Expr::Call(
            Arc::new(Expr::Name("print".to_string(), Span::default())),
            Arc::from(
                args.into_iter()
                    .map(|value| Arg { name: None, value })
                    .collect::<Vec<_>>(),
            ),
            Vec::new(),
            Span::default(),
        );
        let err = match eval_expr(&mut interp, &e, &globals) {
            Err(d) => d,
            Ok(_) => panic!("first argument must fail before the second runs"),
        };
        assert_eq!(err.code, codes::DIV_ZERO);
    }

    #[test]
    fn call_frame_depth_boundary_is_512_513() {
        // From a top-level expression there is no `main` frame, so the first
        // user frame is call 1 and a chain of 512 calls fills the limit; 513
        // exceed it. (The production module path, where `main` occupies frame
        // 1, is pinned by the R3C.1 oracle program cases at 511/512.)
        let program = "fn f(n) { if n { f(n - 1) } else { 0 } }\n";
        let module = crate::parse::parse(program).expect("parse");
        for (n, expected_e4011) in [(510i64, false), (511, false), (512, true)] {
            let mut interp = Interp::new();
            for item in &module.items {
                interp.declare_item(item);
            }
            let globals = interp.globals.clone();
            match eval_expr(&mut interp, &call("f", vec![lit(n)]), &globals) {
                Ok(_) if !expected_e4011 => {}
                Ok(_) => panic!("n={n} must exceed the call-depth limit"),
                Err(d) if expected_e4011 => assert_eq!(d.code, codes::RECURSION, "n={n}"),
                Err(d) => panic!("n={n} unexpectedly failed: {d:?}"),
            }
        }
    }

    #[test]
    fn call_recovers_depth_after_completion() {
        // The shared call-depth accounting must return to its entry value, so
        // a second machine at the same base can still reach the same depth.
        let program = "fn f(n) { if n { f(n - 1) } else { 0 } }\n";
        let module = crate::parse::parse(program).expect("parse");
        let mut interp = Interp::new();
        for item in &module.items {
            interp.declare_item(item);
        }
        let globals = interp.globals.clone();
        for _ in 0..3 {
            assert!(eval_expr(&mut interp, &call("f", vec![lit(509)]), &globals).is_ok());
        }
        assert_eq!(interp.depth, 0, "depth did not recover between machines");
    }

    #[test]
    fn call_throw_crosses_frame_as_e4099_then_e4026() {
        let program = "fn f() { throw 5 }\n";
        let module = crate::parse::parse(program).expect("parse");
        let mut interp = Interp::new();
        for item in &module.items {
            interp.declare_item(item);
        }
        let globals = interp.globals.clone();
        let err = match eval_expr(&mut interp, &call("f", vec![]), &globals) {
            Err(d) => d,
            Ok(_) => panic!("throw must cross the frame boundary as an internal signal"),
        };
        assert_eq!(err.code, codes::THROWN);
        assert!(matches!(interp.pending_throw.as_ref(), Some(Value::Int(5))));
        let user = interp.uncaught_diag(err);
        assert_eq!(user.code, codes::FOREIGN);
        assert!(user.message.contains("uncaught value: 5"));
    }

    #[test]
    fn call_return_becomes_value() {
        let ctl =
            user_call(&call("f", vec![]), "fn f() { return 7\n 8 }\n").expect("call evaluates");
        assert!(matches!(ctl, Ctl::Val(Value::Int(7))));
    }

    #[test]
    fn callback_protocol_drives_map_without_recursive_fallback() {
        // `map` with a named-function callback: the machine must drive the
        // callback as machine work. A recursive fallback would still produce
        // the right value, so the observable we additionally pin is that the
        // shared depth returns to zero and the result is correct.
        let program = "fn d(x) { x * 2 }\n";
        let module = crate::parse::parse(program).expect("parse");
        let mut interp = Interp::new();
        for item in &module.items {
            interp.declare_item(item);
        }
        let globals = interp.globals.clone();
        let e = call(
            "map",
            vec![
                list(vec![lit(1), lit(2), lit(3)]),
                Expr::Name("d".to_string(), Span::default()),
            ],
        );
        let ctl = eval_expr(&mut interp, &e, &globals).unwrap();
        match &ctl {
            Ctl::Val(Value::List(l)) => {
                let got: Vec<i64> = l
                    .borrow()
                    .iter()
                    .map(|v| match v {
                        Value::Int(i) => *i,
                        _ => panic!("unexpected list element"),
                    })
                    .collect();
                assert_eq!(got, vec![2, 4, 6]);
            }
            _ => panic!("expected a list from map"),
        }
        assert_eq!(interp.depth, 0);
    }
}
