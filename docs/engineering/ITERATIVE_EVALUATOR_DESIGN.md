# Engine-stack-independent Aura evaluator — design

**Status:** DESIGN ONLY. No runtime behavior changed. This document is the
contract for B-1R2/R3. B-1 stays OPEN until an implementation passes the
differential oracle and the 512-frame boundary on native, Node, and Chromium.

**Finding:** B-1 — HIGH implementation nonconformance, WebAssembly call-frame
behavior. See `docs/WASM_CALL_FRAME_LIMIT_DECISION.md` for the reproduction,
causality, and the option analysis behind the choice made here.

---

## 1. Problem

Aura's released contract (`LANGUAGE_SPEC.md` §31.3, §31.5) allows at most 512
simultaneous user call frames, `main` included; exceeding the limit is
`E4011`; the rule is substrate-independent; no syntactically valid, well-formed
program may cause host stack overflow. The current tree-walking interpreter
implements each Aura user call as a chain of recursive Rust→WebAssembly calls.
On WebAssembly the JavaScript engine stack can be exhausted before the
interpreter's `depth` counter reaches 512, so a legal program traps (Node
`RangeError`, Playground `E4999`) instead of receiving `E4011`. Measured first
traps on the released `0.2.1` artifact: `else`-recursion 387 in Node (cold) and
196 in the production Playground Worker; `match` 459/233; closure 356; module
387; if-chain 419; the Chromium main thread reaches the boundary only after
warm-up. Every released WASM artifact is affected; native conforms on a
dedicated 64 MiB thread stack. Increasing the guest/shadow stack, `wasm-opt`
levels, and Rust codegen levels do not help (measured in the decision package).

The only contract-preserving direction is an evaluator whose *host* stack does
not grow with the number of active Aura calls. This document specifies that
evaluator precisely enough to implement, review, and differentially test.

## 2. Released contract (must be preserved exactly)

* A user call frame is one invocation of a user function, method, or closure.
* `main` counts as one frame (the entry call).
* Exactly 512 simultaneous frames are permitted; frame 513 is the first
  over-limit frame and produces `E4011` at the attempted call site.
* Builtins do not consume a language frame. A closure/method invoked *through a
  builtin callback* (e.g. `map`) does consume one.
* The accounting is independent of module boundaries and substrate.
* Per-frame expression nesting is capped at 256 AST levels by the runtime's
  own counter (`MAX_AST_DEPTH`, `src/run/mod.rs:26`); a callee starts with a
  fresh budget (reset in `call` via `std::mem::take(&mut self.ast_depth)`,
  `src/run/mod.rs:608`). Statement nesting is capped at the same 256 by the
  checker for every program that goes through the normal compile pipeline
  (`src/check/mod.rs`); both produce `E1015`.

## 3. Current evaluator architecture (measured, real names)

Entry points (all in `src/run/mod.rs` unless noted):

```
Interp::run (349)                     top-level module: declare, init consts/exprs, call main
Interp::run_sourced (379)             same, with per-item SourceId provenance; used by providers
Interp::call (580)                    ONE user call: arity → env child → depth++ → exec_block → depth--
Interp::exec_block (641)              statement sequence; scoped child env; shadowing-aware
Interp::exec_stmt (656)               statements (let/let-pattern/assign/expr/return/throw/
                                      break/continue/while/loop/for/try)
Interp::eval (1192)                   AST-depth guard (MAX_AST_DEPTH) → eval_inner
Interp::eval_inner (1207)             all 21 Expr variants
Interp::eval_call (1555)              args in source order → resolve Name: functions → natives →
                                      env value → regular call; else eval callee → call_value
Interp::call_value (1700)             Closure → call; Native → arity + invoke; else E3001
Interp::call_value_pub (1723)         stdlib higher-order callback entry
Helpers: read_target (846), write_target (881), iterate (926), index_get/set (959/1010),
         field_get/set (1049/1072), bind_pattern (1096), match_pattern (1157),
         select_overload (1615), select_method_overload (1654),
         check_native_arity (1690), construct (1772), binary (1874), numeric (2098),
         method (2180) → crate::stdlib::method (src/stdlib/mod.rs)
```

Top-level drivers call `Interp::call(&main, vec![], Span::default())` at
`src/run/mod.rs:372` and `:430`. The REPL drives `exec_stmt_globals` (1746),
`eval_globals` (1728), and `run_item` (1756) on the same interpreter.

Recursive edges, classified:

| Edge | Kind | Bound |
|---|---|---|
| `call → exec_block → exec_stmt → eval → eval_call → call` | **USER-CALL RECURSION** | 512 frames, but host stack unbounded per frame → **B-1** |
| `eval → eval_inner → eval` (subexpressions, blocks, branches, match bodies, comprehensions) | AST RECURSION | `MAX_AST_DEPTH = 256` per frame |
| `exec_stmt → exec_block → ...` (branches, loops, try) | CONTROL-FLOW RECURSION | AST depth 256 per frame |
| `bind_pattern`/`match_pattern` self-recursion | AST RECURSION (pattern depth) | **parser recursion backstop** (2048 native / 768 WASM), *not* the 256 AST budget — see §19 note and the adjacent finding |
| `call_value_pub → call → exec_block → ... → map native → call_value_pub` | **REENTRANT CALLBACK RECURSION** | user frames 512, host stack unbounded per callback level |
| `execute_with → on_execution_stack` (native: 64 MiB thread; wasm: inline) | SUBSTRATE WRAPPER | — |

The host-recursive edges that must be removed are the two marked **USER-CALL**
and **REENTRANT CALLBACK**: both grow the Rust/wasm call stack once per active
Aura user frame.

## 4. Why B-1 occurs

Each active Aura call holds, on the host stack: the Rust frames of
`call → exec_block → exec_stmt → eval → eval_inner → eval_call` and any
control-flow nesting down to the current call site. Measured V8 cost is about
2.5 KB per Aura call for the `else` shape, and it varies by control-flow shape
(else 387, match 459, closure 356, module 387, if-chain 419 cold first traps)
and by JIT state. The wasm *linear* stack is not the binding resource (a 16 MiB
linker-stack build behaves identically); the engine stack is. A browser does
not expose engine-stack sizing to the artifact, so the only fix is to stop
consuming engine frames per Aura call.

## 5. Design requirements

1. Active Aura user frames consume **no** host stack proportional to their
   count. Host stack use must be O(1) in active frames; the existing 256-level
   AST cap may remain as a per-step bound only if the machine never nests Rust
   calls per active frame.
2. Preserve §2 exactly: 512-frame accounting, `main` counting, `E4011` at the
   attempted call site, fresh per-call AST budget.
3. Preserve all semantic observables in §14 (evaluation order, short-circuit,
   shadowing, closure capture/mutation, methods/`self`, control flow,
   `try/catch/finally`, diagnostics with code/message/span/source, module
   init order, determinism, recovery).
4. Preserve the public API: `Interp::new/silent/with_host`, `run`,
   `run_sourced`, `call`, `call_value`, `call_value_pub`, `eval_globals`,
   `exec_stmt_globals`, `run_item`, `finish_global`, `host`, `native`, and
   the `Compilation` pipeline in `src/lib.rs` are unchanged for embedders.
   The AST is public; the `Rc` sharing in §16.1 changes a few public field
   types mechanically (pre-1.0, no compatibility promise).
5. No `unsafe`; no new public API surface.
6. Native and WebAssembly share the same engine code path; native keeps its
   64 MiB wrapper only as a safety margin for the remaining bounded recursion
   (none required) and for the parser backstop.
7. Keep every existing diagnostic reachable with identical text and spans.

## 6. Interpreter state inventory (what the Rust stack stores today)

For a suspendable user call, everything below must become explicit state:

| State | Today | Where it lives after the redesign |
|---|---|---|
| callee identity/body | `Rc<Closure>` parameter | `UserFrame.closure: Rc<Closure>` |
| closure environment | `Closure.env` (`Env` = `Rc<RefCell<EnvData>>`) | captured in `UserFrame.env` (child scope) |
| caller continuation | Rust stack (implicit) | `Vec<Cont>` records (§16) |
| local environment | `exec_block`'s `local: Env` | `Cont::Block.local: Env` (advances on shadowing) |
| parameter bindings | `env.define` before body | `UserFrame.env` at frame setup |
| `self`/receiver | ordinary first parameter | same (parameter 0) |
| module/source identity | `self.current_source` + `closure_sources` | `UserFrame.saved_source`, `frame_source` |
| current statement/expr | Rust program counter | `Ctrl::EvalStmt{stmt}` / `Ctrl::EvalExpr{expr}` with `Rc` sharing |
| pending operands | Rust locals (`lv`, `out`, …) | `Cont` payload fields |
| return destination | Rust call frame | `Cont::CallReturn` record |
| source span/diagnostic | `Span` locals and `span_of(expr)` | recomputed from the current node (`Rc`/`&Expr`) |
| call-depth counter | `self.depth` | `Machine.frames.len()` |
| control-flow state | `Ctl` in `Result` | `Ctl` delivered to the machine loop |
| try/catch/finally | Rust `Result` + `pending_throw` | `Cont::TryHandler` / `Cont::Finally` markers |
| loop state | Rust `loop` | `Cont::WhileCond` / `Cont::LoopBody` / `Cont::ForNext` |
| match state | Rust `for arm` | `Cont::MatchArm` with arm index |
| temporaries | Rust locals | `Value`s held in `Cont` records |
| mutation visibility | shared `Env`/`Rc<RefCell<…>>` | unchanged |
| host reference | `Interp.host` | unchanged (machine is a field/child of `Interp`) |
| stdout/input/args | host | unchanged |

## 7. Expression suspension matrix

For each `Expr` variant (`src/ast/mod.rs:284`): whether evaluation can
suspend (a nested user call or a builtin that re-enters Aura exists in its
subexpressions), and the state a continuation record must retain.

| Variant | May suspend? | Continuation record | Order constraint |
|---|---|---|---|
| `Lit`, `Name` | no | — | — |
| `FStr(parts)` | yes (each `FPart::Expr`) | `FStrNext { parts, index, out }` | parts left→right; format applied per part |
| `Unary(op, x)` | yes | `UnaryApply { op, span }` | operand first |
| `Binary(op, l, r)` (arith/cmp) | yes | `BinRight { op, lv, span }` then apply | left→right |
| `Binary(And, l, r)` | yes | `BinAndRight { span }` pushed only when `lv` truthy; result `Bool(rv.truthy())` | short-circuit: RHS must not run when `lv` falsy |
| `Binary(Or, l, r)` | yes | `BinOrRight { span }` pushed only when `lv` falsy | short-circuit |
| `Call(callee, args, _, span)` | yes | `CallArgs { callee: Rc<Expr> or resolved, args, index, vals, env }` then `ApplyCall` | **args in source order first**, then resolve; `Name` resolution order: functions → natives → env value → `E2003`; non-`Name` callee evaluated after args |
| `Method(recv, name, args, _, span)` | yes | `MethodRecv { name, args, env }`, `MethodArgs { recv, name, index, vals }`, then dispatch | **receiver first, then args left→right** (`eval_inner` 1316–1320); after args, struct table (`select_method_overload`) or stdlib `method` |
| `Field(recv, name)` | yes | `FieldRecv { name, span }` | receiver first; a non-instance receiver dispatches a zero-arg builtin method (`eval_inner` 1362) |
| `Index(base, idx)` | yes | `IndexBase { idx, env }`, `IndexKey { base, span }` | **base first, then index** (`eval_inner` 1366–1367) |
| `List(items)` | yes | `ListNext { items, idx, out }` | left→right |
| `Map(entries)` | yes | `MapKey { entries, idx, out }`, `MapValue { key, entries, idx, out }` | per entry: key then value; `MapKey::from_value` check between |
| `ListComp` | yes | `CompIterStart`, `CompNext`, `CompFilter`, `CompValue` | iterable once **via `iterate` (materializes the iterable; range-1e7 cap)**; per item fresh scope; bind → filter → value |
| `MapComp` | yes | `CompKey`, `CompValue` variants | same iteration model; key before value |
| `Construct(name, args, _, span)` | yes | `ConstructArgs`, then field/variant assembly | args in source order; named/positional rules as `construct` (1772) |
| `Tuple(items)` | yes | same as `List` | left→right |
| `Lambda(params, body)` | no (body not executed at creation) | none; building the closure is atomic | closure captures the current `env`; body calls execute only when the closure is invoked |
| `Pipe(l, r)` | yes | `PipeLeft`, `PipeRight { arg }` then `call_value` | `l` then `r` (callee value) then call |
| `Range(a, b)` | yes | `RangeStart`, `RangeEnd { s }` | left→right; type checks per `eval_inner` 1496 |
| `If(cond, then, else)` | yes | `IfBranch` (branch selection after cond) | cond first; only one branch runs |
| `Match(subject, arms)` | yes | `MatchArm { arm, scope }` / guard | subject; arms in order; first match; guard in arm scope; no arm → `E4029` (`NO_MATCH`) |
| `Block(stmts)` | yes | `Cont::Block { body, index, local, scoped }` | statements left→right; last value is block value |

## 8. Statement suspension matrix

| Variant | Continuation | Notes |
|---|---|---|
| `Let` | `LetBind { name, mutable }` | initializer may suspend; binding installed with `define_shadowing` (shadow identity) |
| `LetPattern` | `LetPatternTransfer { pattern }` | initializer first; bind into temp child; transfer each name via `define_shadowing` |
| `Assign` | `AssignRhs`, `AssignRead { op }`, `AssignWrite { target }` | RHS first; for compound, read current target, `binary`, then write. **Compound targets are evaluated twice**, exactly as today: `read_target` evaluates the base/index (or field receiver), then `write_target` evaluates them again (`read_target` 846–879, `write_target` 881–920). Verified observable: `a[idx()] += 10` prints `idx` twice. A machine that caches the read location would diverge; the design requires re-evaluation. |
| `Expr` | none (result delivered) | expression value is the block value |
| `Return` | `ReturnValue` | value may suspend; then `Ctl::Return` |
| `Throw` | `ThrowValue` | value may suspend; then `Ctl::Throw` |
| `Break`/`Continue` | none | immediate signals |
| `While` | `Cont::WhileCond { cond, body, env }` | cond re-evaluated each iteration; body scoped (`exec_block(..., true)`); `Break`→`None`; `Continue`/val→next |
| `Loop` | `Cont::LoopBody { body, env }` | same signal mapping; body scoped |
| `For` | `Cont::ForNext { pat, body, env, state }` | `Range` iterates lazily (no materialization); other iterables via `iterate` (bounded 10M); per item **non-scoped** child scope (`exec_block(..., false)`); `Break`→`None` |
| `Try` | `Cont::TryHandler { catch, catch_body, env }` + `Cont::Finally { finally_body, env, pending }` | §12 |

## 9. Control-flow signal model

`Ctl` (`src/run/mod.rs:166`) is preserved unchanged: `Val`, `Return`, `Break`,
`Continue`, `Throw`. The machine delivers each step's outcome to the next
continuation, which may absorb it or forward it. Propagation rules, exactly as
the current code:

1. In expression position, `Return`/`Break`/`Continue`/`Throw` propagate
   outward through every enclosing expression (e.g. `if c { break }` as an
   expression); they are never converted by arithmetic/collection steps. The
   `val!` macro does not convert them: it writes `match e? { Ctl::Val(v) => v,
   other => return Ok(other) }`, so any non-`Val` outcome immediately leaves
   the enclosing expression. `Ctl::value` is called in exactly one place today
   (`finish_global`, 565–566) and converts a residual signal into
   `E4026`/`E4030`/`E2015`; the machine reproduces that by delivering a
   non-`Val` outcome to the frame boundary / loop boundary / top-level driver
   and letting those convert.
2. At a **loop boundary marker**, `Break` → loop result `None`; `Continue` →
   next iteration; everything else forwards.
3. At a **user-frame boundary marker**, `Return(v)` → the call's value;
   `Throw(v)` → `Err(E4099 THROWN)` (`codes::THROWN`; `E4026` is the
   user-facing conversion applied only at the top level) with
   `pending_throw = Some(v)` recorded — matching `Interp::call` (624–633);
   `Break`/`Continue` escaping the body → `E2015` (`LOOP_CONTROL`) with the
   call span.
4. At the **top-level driver**, residual `Return`/`Break`/`Continue` →
   `E4030`/`E2015`, and `Ctl::Throw` → `E4026`, all via `Ctl::value`
   (565–566); matching `finish_global` (565) and `eval_toplevel` (1738).
5. `Err(diag)` is not catchable except `E4099 THROWN`; it aborts the machine
   run outward (matching `?` propagation) after frame cleanup.

## 10. Environment / closure / method model

Unchanged types: `Env(Rc<RefCell<EnvData>>)` with `get`, `define`,
`define_shadowing`, `assign` (140–157). The machine stores `Env` clones in
frames and continuations; cloning is a refcount bump, so aliasing semantics are
identical. Closure capture is unchanged: `Closure.env` (64) is the definition
environment; `Lambda` captures the current `env` (1480). Shadowing keeps
binding identity because `define_shadowing` still creates a child frame; the
machine advances `Cont::Block.local` exactly where `exec_block` advances
`local`.

Methods: struct receivers resolve in the machine through
`select_method_overload` (1654) and are ordinary closures with the receiver as
parameter 0 (`declare_item_with_source`, 499–520). Builtin methods still route
to `crate::stdlib::method` and are atomic except `map`/`filter`/`reduce`
(§13). `self` is just parameter 0.

## 11. (reserved) Multi-file / module identity

Module flattening is done before execution (`src/module_graph.rs`,
`into_parts` at 451); the runtime sees a flat `Module` plus per-item
`SourceId`s. Module initialization is an ordinary sequence of top-level
`Const`/`Expr` items followed by the `main` call (`run_sourced`, 379–436).
Module bodies can call user functions, so their expressions are machine work
like any other. The machine stores per-frame `saved_source` and applies
`closure_sources` exactly as `call` does (608–621); `last_error_source` is set
once, at the innermost frame that first observes an error, preserving
`run_sourced`'s owner attribution (417, 431).

## 12. Try / catch / finally model (exact current semantics)

Current behavior (`exec_stmt` `Stmt::Try`, 801–842), to be reproduced step for
step:

1. Run the try body. Outcome `o`:
   * `Ok(Ctl::Throw(v))` → run catch with `v` bound;
   * `Err(diag)` with `diag.code == THROWN` → take `pending_throw` (fallback:
     the diagnostic message as a string value) and run catch;
   * `Ok(other)` → result = `other`; catch not run;
   * `Err(other)` → result = `Err`; catch not run.
2. If a `finally` exists: run it. A **diagnostic error** from `finally`
   propagates instead of the pending result (`?` at 835) — including over a
   pending fatal `Err`. A non-`Val` signal (`Return`/`Break`/`Continue`/`Throw`)
   from `finally` **replaces** the pending result. A `Val` leaves the pending
   result untouched. Precedence: finally's own fatal error beats finally's
   signal (the `?` runs first); finally's signal beats the pending result.
3. Scopes are exact: the try body is `exec_block(body, env, true)` (scoped),
   the catch body is `exec_block(catch_body, &scope, false)` where
   `scope = env.child()` with the caught value defined once in it (the `false`
   is safe because `scope` itself is fresh and discarded; a `let` in the catch
   body still cannot leak to `env`), and the finally body is
   `exec_block(f, env, true)` (scoped).
4. The `return`-inside-`finally` case is the subtle one and follows directly
   from (2): `finally { return 7 }` produces `Ok(Ctl::Return(7))`, which
   replaces the pending result, so the surrounding function returns `7` even
   if the try body had already returned another value. The machine must not
   special-case `Return` away from the replacement rule.

Machine representation: `Cont::TryHandler` is pushed before the body; when the
body (or a callee's converted `E4099`) delivers `Throw`/`THROWN`, the handler
runs the catch and then pushes `Cont::Finally` carrying the *pending result*
(which may be an `Err`, a `Ctl` signal, or a `Val`). `Cont::Finally` runs the
finally body and, on `Val`/success, re-delivers the pending result; on any
other outcome or error, delivers the finally's outcome instead. This preserves
all cases, including `return` inside `finally`, error inside `finally`, and
nested `try` inside `finally`.

Interaction with frame boundaries: a `throw` inside a called function reaches
the caller's `try` only after the frame boundary converts it to `E4099` +
`pending_throw` (rule 3 in §9), so a `try` in the *callee* catches its own
frame's throws, and a `try` in the *caller* catches the converted signal of a
callee throw exactly as today.

## 13. Reentrancy and the Python boundary

* Stdlib higher-order builtins re-enter Aura from Rust today:
  `map`/`filter`/`reduce` as free functions (`src/stdlib/mod.rs:300–340`) and
  as list methods (`:660–695`) call `it.call_value_pub(...)` in a loop. Left
  as-is, each callback would be a user frame executed by a nested machine whose
  host stack adds up across 512 frames. **Design:** these six call sites are
  converted to a machine-driven callback protocol (below), so callback
  iterations run as machine work with no host-stack growth.
* `Value::Native` callbacks consume **no** user frame; `Value::Closure`
  callbacks consume one. This asymmetry is contractual and must be preserved
  exactly (`call_value` 1700–1712: `Closure` → `call` → depth++, `Native` →
  direct invoke, no depth change). Consequence, verified: `[1].map(to_string)`
  adds no frame, while `[1].map((x) -> x)` adds one. Today's
  `rec(510)`-with-native-callback program prints `1` (native spends no frame);
  the same program with a closure callback is `E4011`. The callback protocol
  below must therefore apply frame accounting **only when the callback value
  is a `Closure`**.
* **Callback protocol (selected):** an internal native may produce
  `NativeOutcome::Done(Value)` (today's behavior) or
  `NativeOutcome::InvokeCallback { f, args, resume }` where `resume` is a
  small native-owned state token. For a `Closure` callback the machine pushes
  a normal user frame (depth++, `E4011` if that would be frame 513); for a
  `Value::Native` callback the machine invokes it inline, as today, with no
  frame. When the callback returns, the machine calls the native's `resume`
  continuation (another `NativeOutcome` or `Done`). `map`/`filter`/`reduce`
  implement `resume` as "continue iterating with the accumulated result".
  Suspension of a closure callback therefore no longer costs host stack.
  **Snapshot semantics preserved:** today `map`/`filter`/`reduce` snapshot the
  receiver list (`as_list` clones, `src/stdlib/mod.rs:481–488`; list methods
  `l.borrow().clone()`, 668/676/687), so callback mutation of that list does
  not change the iterated elements. The protocol must iterate the same
  snapshot taken at entry. **Span preserved:** each callback call keeps the
  outer `map`/`filter`/`reduce` call-site span passed to `call_value_pub`
  today (311/322/336, 669/678/690).
  **Public API constraint:** `Interp::native` is a public embedder API whose
  callback type is `Fn(&mut Interp, Vec<Value>, Span) -> Result<Value>`. It
  stays exactly as-is, wrapping its result in `Done`. Only the six stdlib
  registrations move to a new private registration path (`native_resumable`,
  or direct insertion into `natives` with the internal outcome enum); no
  public signature changes.
* **Python bridge:** Aura closures do not cross into Python (`src/bridge` has
  no `Value::Closure`/callable conversion), and Python values are not callable
  from Aura; there is no Python→Aura reentrancy path. Python calls are atomic
  natives, do not consume user frames, and cannot suspend. No change.
* **Host-registered natives:** `Interp::native` is public; an embedder could
  register a native that itself calls `Interp::call`/`call_value`. Today that
  nests Rust calls inside the interpreter frame that invoked the native. Such
  a native cannot receive the `&mut Interp` machine loop, so the design cannot
  make it resumable; it starts a nested machine run exactly as it nests a
  nested Rust call today. This is unchanged behavior for an unsupported
  embedding pattern, and it is out of scope for B-1.
* **REPL and embedders** calling `eval_globals`/`exec_stmt_globals`/
  `call_value`/`call_value_pub` drive one machine per public call. The
  top-level *statement/expression* entry itself consumes no Aura user frame
  (the REPL has no implicit `main`), but a user `call_value` of a `Closure`
  does consume frame 1 and can reach `E4011` at the 513th frame exactly as
  today (`call_value` → `call`, depth++). `run`/`run_sourced` additionally
  consume frame 1 for the entry `main` call (`Span::default()`).

## 14. Semantic observables to preserve (migration oracle)

For every program, both engines must agree on:

1. accept/reject status;
2. normalized stdout bytes;
3. final value where observable (REPL `eval_globals` `Ctl`);
4. diagnostic `code`;
5. diagnostic message text (contract-sensitive messages are exact);
6. diagnostic source `SourceId`/file and `(line, column)` span;
7. side effects on shared values (list/map/instance mutation through `self`,
   captured variables, globals);
8. short-circuit evaluation counts (observable via prints in skipped branches);
9. evaluation order of arguments/receivers/operands (observable via prints);
10. module initialization order (top-level `Const`/`Expr` items);
11. closure capture and captured-mutation visibility;
12. `try`/`catch`/`finally` outcomes for all 15 signal/pending combinations;
13. `E4011` boundary (frame 512 allowed, 513 rejected) at the attempted call
    site, with the same span;
14. resource-limit codes (`E1015`, `E4011`, `E4013` for range materialization,
    `E4019` for index, per existing code) and recovery afterward (a fresh run
    on the same interpreter behaves as today; REPL: an error followed by a
    valid submission);
15. deterministic ordering everywhere (maps are `BTreeMap`; diagnostics
    ordered by construction).

## 15. Architecture options

### E1 — Explicit continuation machine over the AST (considerable, selected)

A single `Machine` loop consumes `Ctrl` work items and produces values through
a `Vec<Cont>` continuation stack. All evaluation is iterative: host stack use
is O(1) in active Aura frames. User frames are explicit records; loops, frames,
and `try` are boundary markers in the same continuation stack. The AST is
interpreted directly; no lowering.

*Semantic fidelity:* highest — the step set is derived mechanically from the
21 `Expr` and 12 `Stmt` arms, and helpers (`binary`, `index_get`, …) are reused
unchanged.
*Blast radius:* `src/run/mod.rs` control structure (the same public API, the
same helpers); `Closure.body` shape and AST child sharing (§16.1).
*Control flow:* explicit markers; specified in §9/§12.
*`try`/`finally`:* explicit handler records, all cases enumerated.
*Performance:* dispatch overhead per step; bounded-recursion fast path for
call-free subtrees keeps most arithmetic near today's cost (§21).
*Risk of becoming a bytecode VM:* low — no IR, no lowering, no new compile
stage.

### E2 — Boxed-closure CPS (rejected)

Transform `eval`/`exec_stmt` into continuation-passing style with
`Rc<dyn FnOnce(Result<Ctl>) -> Result<()>>` continuations. O(1) host stack,
mechanical derivation from today's code, but allocates a closure per
subexpression step, complicates the 512 accounting (frames become implicit in
closure chains), makes diagnostics/`?` propagation less uniform, and would be
rewritten again for performance. Rejected: higher runtime cost and lower
reviewability than E1 for the same semantics.

### E3 — AST → internal instruction IR + iterative VM (rejected)

Lower the AST once per `Compilation` into a small stack-machine IR and
interpret that. Removes AST recursion entirely and is fast, but adds a new
compiler stage, a new debug surface, span mapping through lowering, and a
second place where semantics can drift; it materially risks becoming the
"bytecode VM project" this program must not become. Rejected: not the minimum
architecture; E1 already achieves O(1) host stack.

**No AST lowering is necessary** (explicit question): E1 executes the existing
AST directly with an explicit continuation stack; the AST is small and stable,
and every variant maps to at most a few step kinds.

## 16. Selected design

### 16.1 Prerequisite: shareable AST nodes

Continuations must retain pending subexpressions when a call suspends. Holding
`&Expr` across machine steps would require self-referential borrows of
`Rc<Closure>` bodies. The design therefore makes AST children cheap to share.
The field-type changes are mechanical and complete:

* `Box<Expr>` → `Rc<Expr>` (`Expr::Unary/Binary/Call/Method/Field/Index/Pipe/
  Range/If` operands, `ListComp/MapComp` sub-expressions, `Lambda` body);
* `Vec<Expr>` → `Rc<[Expr]>` (`List`, `Tuple`);
* `Vec<Stmt>` → `Rc<[Stmt]>` (`Expr::If` then-branch, `Expr::Block`,
  `Stmt::While/Loop/Try` bodies, `Closure.body`);
* `Vec<Arg>` → `Rc<[Arg]>` (`Call`, `Method`, `Construct` args);
* `Vec<(Expr, Expr)>` → `Rc<[(Expr, Expr)]>` (`Map` entries);
* `Vec<FPart>` → `Rc<[FPart]>` (`FStr`);
* `Vec<Arm>` → `Rc<[Arm]>` (`Match`).

`Rc` derefs like `Box`, so matcher code is unchanged; construction sites
change mechanically. `PartialEq`/`Clone` semantics are unchanged (`Rc`
compares contents). This is a pure ownership-layout change with no semantic
effect, and it is the enabling prerequisite for suspension storage. The
alternative (an arena with `u32` handles) is rejected as more invasive; the
alternative of owning deep `Clone`s in continuations is rejected as
O(program-size) per suspension.

### 16.2 Machine state

```text
struct Machine<'i> {
    interp: &'i mut Interp,        // host, tables, source tracking
    ctrl:  Ctrl,                   // current computational focus
    kont:  Vec<Cont>,              // continuations, innermost last
    frames: Vec<UserFrame>,        // Aura user call frames; len() == depth
    pending_throw: Option<Value>,  // mirrors Interp::pending_throw at boundaries
}

enum Ctrl {                        // what to do next
    EvalExpr(Rc<Expr>, Env),
    EvalStmt(Rc<Stmt>, Env),           // statement + its resolution environment
    EnterBlock(Rc<[Stmt]>, Env, bool), // body, lexical parent, scoped
    ApplyCall { callee, args },        // after argument evaluation
    ReturnValue(Value),
    ThrowValue(Value),
    Done(Ctl),                         // value/signal ready
}

struct UserFrame {
    closure: Rc<Closure>,
    env: Env,                      // call scope (params bound)
    saved_source: Option<SourceId>,
    frame_source: Option<SourceId>,
    call_span: Span,               // for boundary diagnostics (E4011/E2015)
}
```

**Environment write-back (shadowing).** Today `exec_stmt` takes `env: &mut Env`
and a shadowing `let` does `*env = env.define_shadowing(...)` (`mod.rs:675`),
which changes what *later statements in that block* resolve. The machine
reproduces this by keeping the **current environment of the statement list in
the enclosing `Cont::Block` record**: `EnterBlock` creates `local` (a child
when `scoped`, else a clone of the parent) and pushes
`Cont::Block { body, index, local, scoped }`. Before each statement the
statement's `Env` is cloned out of `Cont::Block.local`; when the statement
completes with a value, the value is stored and `local` is upgraded to the
environment the statement produced. The upgrade travels with the completion:
`Ctrl::Done` for a statement carries `(Ctl, Option<Env>)` (or the block
continuation reads a `produced_env: Option<Env>` slot set by the statement
machinery). `While`/`Loop`/`For`/`Try` bodies push their own `Block`, so a
shadowing `let` inside a loop body is confined to that iteration, exactly as
`exec_block(body, env, true)` is today. This mirrors `exec_stmt_globals`
(`mod.rs:1747–1751`) at the top level, where the REPL adopts the advanced
environment.

### 16.3 Continuation records

Derived from §7/§8; representative set (not exhaustive):

```text
enum Cont {
    // expressions
    UnaryApply { op: UnOp, span: Span },
    BinRight { op: BinOp, lv: Value, span: Span },
    BinAndRight { span: Span }, BinOrRight { span: Span },
    FStrNext { parts: Rc<[FPart]>, index: usize, out: String },
    CallArgs { callee: Rc<Expr>, args: Rc<[Arg]>, index: usize, vals: Vec<Value>, env: Env, span: Span },
    ApplyCalleeValue { vals: Vec<Value>, span: Span },
    MethodRecv { name: String, args: Rc<[Arg]>, index: usize, vals: Vec<Value>, env: Env, span: Span },
    FieldRecv { name: String, span: Span },
    IndexBase { idx: Rc<Expr>, env: Env, span: Span }, IndexKey { base: Value, span: Span },
    ListNext { items: Rc<[Expr]>, index: usize, out: Vec<Value>, env: Env },
    MapKey { ... }, MapValue { key: MapKey, ... },
    CompNext { kind, ... }, CompFilter { ... }, CompValue { ... }, CompKey { ... },
    ConstructArgs { name: String, args: ..., index: usize, positional: Vec<Value>, named: Vec<(String, Value)>, env: Env, span: Span },
    PipeRight { arg: Value, env: Env, span: Span },
    RangeEnd { start: Value, span: Span },
    IfBranch { then: Rc<[Stmt]>, els: Option<Rc<Expr>>, env: Env, span: Span },
    MatchArm { subject: Value, arms: Rc<[Arm]>, index: usize, env: Env, span: Span },
    // statements
    Block { body: Rc<[Stmt]>, index: usize, local: Env, scoped: bool, last: Value },
    LetBind { name: String, mutable: bool },
    LetPatternTransfer { pattern: Rc<Pattern> },
    AssignRhs { target: Rc<Expr>, op: Option<BinOp>, env: Env, span: Span },
    AssignRead { target: Rc<Expr>, rhs: Value, op: BinOp, env: Env, span: Span },
    AssignWrite { target: Rc<Expr>, value: Value, env: Env, span: Span },
    ReturnValue, ThrowValue,
    WhileCond { cond: Rc<Expr>, body: Rc<[Stmt]>, env: Env },
    LoopBody { body: Rc<[Stmt]>, env: Env },
    ForNext { pat: Rc<Pattern>, body: Rc<[Stmt]>, env: Env, state: ForState },
    TryHandler { catch: String, catch_body: Rc<[Stmt]>, env: Env },
    Finally { finally_body: Rc<[Stmt]>, env: Env, pending: Pending },
    // boundaries
    FrameBoundary { call_span: Span },   // pop UserFrame here
    LoopBoundary,
}
```

The distinction required by the contract is explicit: `UserFrame` records are
**Aura user call frames** (counted against 512); `Cont::FrameBoundary` is the
internal evaluator marker that pops one. Continuation records other than frame
boundaries do **not** consume the 512 budget.

### 16.4 Value/temporary model

No separate value stack: values travel as `Ctrl::Done(Ctl)` results into the
innermost continuation, and continuations hold the operands they still need
(`lv`, `out`, `vals`, …). This halves the state a reviewer must track and
makes every temporary's owner obvious.

## 17. Exact `E4011` accounting

* `Machine.frames.len()` is the authoritative active-frame count.
* `main` enters through the same call path, so it occupies frame 1.
* Frame setup order, preserving current observables: evaluate arguments →
  resolve overload/`bind_arguments` → arity check (`call`, 581) → **frame
  check** → push `UserFrame` → push `FrameBoundary` → run body.
* Frame check: if `frames.len() == MAX_CALL_FRAMES` (512) the attempted frame
  would be 513 → return `Diag::new(codes::RECURSION, "call depth limit
  exceeded", span)` where `span` is exactly the span the current code passes:
  callee-name span for `Expr::Name` calls, the call-expression span for
  value callees/`Pipe`/`Construct`-free paths, and the method expression span
  for methods; `Span::default()` for the entry `main` call.
* A rejected frame is not pushed; no `FrameBoundary` is pushed.
* Frame pop happens when a `FrameBoundary` is crossed for **any** outcome —
  `Val`/`Return` (call value), converted `Throw` (`E4099`), escaping
  `Break`/`Continue` (`E2015`), or `Err` (abort). No RAII; cleanup is data.
* On `Err` aborting a top-level entry, the machine clears `frames`/`kont`
  before returning, so a reused interpreter (REPL) starts clean, matching
  today's `depth == 0` after an error.
* Per-frame AST budget: entering a frame saves `interp.ast_depth` and resets it
  to 0; the `E1015` guard on expression descent uses the same counter and the
  same diagnostic (span of the offending expression); leaving the frame
  restores the saved value (current `call`, 608/622).

## 18. Diagnostic / source provenance design

* `interp.current_source` is saved into `UserFrame.saved_source` and set to the
  frame's closure source (from `closure_sources`) at frame entry; restored at
  frame pop. Unchanged from `call` (609–621).
* `interp.last_error_source` is set exactly once, when an `Err` first crosses a
  frame boundary and `last_error_source.is_none()`; never overwritten after
  that, matching `call` (618–620). `run_sourced` reads it at item/main
  boundaries (417, 431). Note: a rejected `E4011` frame never enters a frame
  boundary (the check at 600–604 returns before setup), so it does **not**
  set `last_error_source`; `run_sourced`'s `unwrap_or(source)` fallback
  attributes it to the item being evaluated. The machine must reproduce this
  (no frame → no attribution write).
* `Interp::run` (unsourced, 349–376) registers functions without
  `closure_sources` entries and calls `main` with `Span::default()`;
  `run_sourced` (379–436) sets `entry_source` and clears/sets
  `last_error_source`. The machine preserves both behaviors; it must not
  assume per-item sources are always present.
* `E4011` uses the call-site span from §17, never `Span::default()` except for
  the `main` entry as today.
* All other diagnostics are produced by the same helper functions
  (`binary`, `index_get`, `format_value`, `select_overload`, …) with the same
  spans, because the machine calls those helpers with the same arguments.

## 19. Memory / lifetime model

* No `unsafe`. Frames and continuations own `Rc` handles (`Closure`, AST
  nodes) and `Env` clones; dropping the machine releases them like today's
  stack unwind.
* Bounds: `frames ≤ 512`. Per-frame continuation growth is bounded by the
  AST descent along the current path. The continuation stack is global, so
  when frame *k* is suspended its pending continuations sit below frame
  *k+1*'s: `kont ≈ Σ over active frames of that frame's own nesting path`.
  Along one path, expression descent ≤ 256 (`E1015`) and statement/block
  nesting ≤ 256 (checker-enforced for every source-compiled program), so a
  frame holds on the order of ≤ 512 continuation records; the pathological
  worst case is ≈ 512 × 512 ≈ 2.6×10⁵ records ≈ tens of MB at ~100–200
  bytes each. Practical programs are orders of magnitude smaller.
* **Statement nesting is not separately capped by the runtime.** The
  evaluator recurses `exec_block → exec_stmt → exec_block`, and only the
  checker caps nested statements at 256 for programs compiled through
  `Compilation`/`compile*` (CLI, WASM runtime, REPL). The parser backstop
  (2048 native / 768 WASM) is the only bound above that, and a hand-built
  `Module` passed to `Interp::run` bypasses both. The machine must therefore
  make block/statement descent iterative (it does: `Cont::Block`) and keep a
  block-entry guard so host stack stays O(1) in statement nesting as well as
  in user calls; no new *language* limit is introduced.
* **Adjacent finding (recorded, not designed here):** deeply nested
  *patterns* are bounded today only by the substrate-calibrated parser
  backstop, not by any 256 semantic limit: measured native accepts depth 2000
  (rejects 2100) and the released `0.2.1` WASM accepts ~700 but rejects 766
  and above; both report `E1015`. `bind_pattern`/`match_pattern` recurse per
  pattern level, so this is a second host-stack-recursion surface and a
  native/WASM acceptance divergence. It is independent of B-1; the machine
  design must at minimum preserve today's behavior, and the evaluator work
  should either include patterns in the explicit stack or carry a documented
  bound. Logged as a coverage gap for the remediation program, not fixed in
  this design-only pass.
* Temporaries held in continuations keep `Value`s alive exactly while the
  evaluator would hold Rust locals today; no observable lifetime change.
* Deep value structures (lists, maps, instances, strings) keep their current
  `Rc`-shared identity; the machine never deep-clones values for control.

## 20. (reserved) Multi-source/provider interaction

Provider-backed compilations (`src/module_graph.rs`) produce the same flat
`Module` + `item_sources` consumed by `run_sourced`; the machine is agnostic to
whether items came from disk, memory, or a virtual project. `SourceId`s are
carried per item and per frame as today. No provider change.

## 21. Performance model

Expected costs and mitigations:

* One `Ctrl`/`Cont` push+pop per AST step replaces a Rust call. Likely 1.5–3×
  slower in the worst case for arithmetic-heavy loops.
* Mitigation (not required for correctness, planned for R3G): a
  **call-free fast path** — a subtree containing no call-suspension point
  (`Call`, `Method`, `Pipe`, callbacks) executes with the existing recursive
  helper bounded by the 256 AST cap. That keeps common arithmetic near today's
  cost while suspending only where suspension is possible. This path is
  semantics-preserving because a call-free subtree cannot suspend.
* Benchmarks (before/after, per milestone): arithmetic loop, non-recursive
  calls, nested calls, closures, methods, `map`/`reduce` callbacks, moderate
  recursion (depth 100/400/510), large lists/maps, deep expression trees,
  REPL session scaling (TD-13).
* Methodology: `cargo bench`-style timed runs on a quiesced machine, median of
  N, recorded in `docs/engineering/PERFORMANCE.md`; no assertion on absolute
  numbers, only on observable equivalence and no catastrophic regression.

## 22. Native behavior risk

The machine replaces the recursive engine on **both** substrates (one code
path; no substrate divergence). Native relies on the machine for semantics
exactly as WASM does; the native 64 MiB wrapper stays for the parser backstop
and any remaining bounded recursion, not for user calls. Risks: native stack
traces in panics change shape (internal only); `Mem::take` of `ast_depth`
behavior must be replicated per frame; `on_execution_stack`'s Send boundary
must be respected (the machine must stay `Send`-compatible for
`execute_with_host_factory`); `Host` access stays on the execution thread.
Native is the semantic oracle for every migration phase, and the differential
oracle runs natively (both engines in one binary) before WASM is trusted.

## 23. WASM behavior risk

After the machine, the engine stack per Aura frame drops to O(1); the 512
limit becomes reachable in the production Worker because no engine-stack
growth occurs per frame. Residual risks: the machine loop itself must avoid
deep recursion in *helpers* called per step (`Equals`/display on deep values
are already bounded by `MAX_VALUE_DEPTH`); `bind_pattern`/`match_pattern`
remain recursive and are bounded only by the substrate-calibrated parser
backstop (768 on WASM) — the same bound that admitted the program at parse
time, so the machine adds no new exposure, but the evaluator program should
make them iterative or add a semantic pattern-depth limit (see §19 adjacent
finding); the `map`/`filter`/`reduce` callback protocol must not re-introduce
nested machines; the Worker's smaller engine stack no longer matters but
remains a boundary test.

## 24. Differential oracle design

* During migration only, the interpreter supports two internal engines:
  `Recursive` (today's code) and `Iterative` (the machine). The switch is a
  private field; for the differential harness only it is reachable through a
  `#[doc(hidden)]` constructor gated by a non-default `evaluator-oracle`
  feature, deleted in R8. No existing public signature changes, and default
  builds contain only the recursive engine until R5.
* The oracle compiles each source **once** into an immutable `Module`, then
  runs that same module in **two fresh `Interp` instances**, each with its own
  `Host`, so no mutable state is shared and neither run can observe the other.
  Comparison normalizes: status, stdout bytes, diagnostic code/message,
  `SourceDiagnostic` source identity and span (`line:column`), and for the REPL
  the returned `Ctl` value display.
* Where a `Host` records effects (silent host captures stdout, args, input),
  the two runs use separate host instances and compare captured bytes.
* Integration point: a new `tests/evaluator_oracle.rs` integration test (built
  only with the `evaluator-oracle` feature) that loads the existing corpus
  fixtures and asserts equality of the normalized observable tuple. A
  `#[cfg(test)]` crate-internal unit harness additionally covers paths that need
  private state (REPL engine reuse, resume-after-error). The feature and hook
  are removed in R8, so no dual semantics survive the program.

## 25. Oracle corpus

Reuse first: every fixture under `tests/corpus/**` (including the six-shape
`call-frames` boundary set), the playground differential fixtures, and
`tests/contract.rs`/`compat.rs` sources. Add only what the suspension matrix
requires and current fixtures lack:

* each `Expr` variant with a call in each operand position (`a() + b()`,
  `f(a())`, `[a(), b()]`, `{k(): v()}`, `x[i()]`, `r.field()`, `a()..b()`,
  `cond() ? ...`, `match s() { ... }`, f-string with call, pipe with call,
  comprehension with call in iterable/filter/value/key, construct args);
* `finally` matrix: all 15 combinations of pending `{Val, Return, Break,
  Continue, Throw}` × finally outcome `{Val, signal, error}`, plus nested
  `finally` and call-in-finally;
* closure mutation through a deep call chain; method recursion; mutual
  recursion; cross-module calls; callbacks (`map`/`filter`/`reduce`) nested
  and recursive; the callback frame asymmetry (`map(to_string)` adds no frame;
  `map((x) -> x)` adds one; both at the 512 boundary); callback list-snapshot
  observability (callback mutates the receiver list, iteration unchanged);
  compound-assignment double evaluation (`a[idx()] += 1` prints `idx` twice);
* diagnostics: undefined name (`E2003`), type mismatch (`E3001`), index
  (`E4019`), division (`E4007`), overflow (`E4013`), E1015, E4011, E2015
  (break outside loop), no-match (`E4029`), uncaught throw (`E4026`), with
  span checks;
* runtime errors inside callbacks and after recovery (REPL: error then valid
  submission).
Randomized generation is deferred to R4; the deterministic corpus is the
contract.

## 26. Implementation phase plan

Derived from the architecture (each phase independently reviewable; no single
giant commit):

* **B-1R2 — Oracle scaffolding.** Internal engine switch + normalized
  comparator + deterministic corpus; recursive engine remains the default and
  the only production path. No behavior change.
* **B-1R3A — AST sharing + machine skeleton.** `Rc`-share AST children/bodies
  (mechanical, no semantic change); `Machine`, `Ctrl`, `UserFrame`,
  `FrameBoundary`, `E4011` accounting; literals/names/expression statements,
  sequential blocks. Oracle green.
* **B-1R3B — Values and operators.** Unary/binary (incl. short-circuit),
  lists/maps/tuples, index/field reads, ranges, f-strings, formatting. Oracle
  green.
* **B-1R3C — Calls.** Args/source order, overloads, `bind_arguments`, named
  args, closures, first-class natives, methods/`self`, pipes, constructors;
  frames for all call variants. Oracle green.
* **B-1R3D — Statements and loops.** `let`/shadowing, `let` patterns,
  assignment (simple/compound/index/field targets), `if`/`while`/`loop`/`for`
  (lazy ranges), `break`/`continue` boundaries, `return`/`throw`. Oracle green.
* **B-1R3E — Match, comprehensions, lambdas.** Oracle green.
* **B-1R3F — Try/catch/finally.** All 15 pending combinations; boundary
  conversion `Throw`→`E4099`. Oracle green.
* **B-1R3G — Callbacks and fast path.** `map`/`filter`/`reduce` callback
  protocol; optional call-free recursive fast path; performance measurements.
  Oracle green.
* **B-1R4 — Full differential.** Whole corpus + compat/contract/cross-subsystem
  suites over both engines; property/fuzz differential where available.
* **B-1R5 — Substrate boundary.** 512-frame limit−1/limit/limit+1 across
  native release, Node cold/warm, Chromium main thread, production Worker;
  safe-depth shape matrix stays green; recovery after `E4011`.
* **B-1R6 — Red team.** Adversarial review against the design contract
  (independent read-only reviewer).
* **B-1R7 — Full validation gate.** `cargo fmt/clippy/test` matrix, all
  features and no-default features, MSRV, playground and website suites;
  frozen artifacts unchanged.
* **B-1R8 — Freeze review.** Remove the recursive engine and the oracle switch;
  the iterative engine becomes the only engine; re-run the full gate; record
  results; human release decision (no implicit release).

## 27. Rollback strategy

* The recursive engine stays in the tree, selectable, until R4 proves parity;
  every phase commits with the iterative engine **off** by default until R5.
* If a divergence appears, the phase is reverted (small commits, bisectable);
  no released artifact, tag, or frozen runtime is ever touched.
* The engine switch is internal and documented as temporary; R8 deletes it
  together with the recursive code, so no long-lived dual semantics.

## 28. Risk register additions (proposed, to be recorded in R2)

| Risk | Severity | Mitigation |
|---|---|---|
| AST `Rc` conversion changes equality/clone behavior subtly | MEDIUM | Mechanical change + full gate; `PartialEq` compares contents; existing tests are extensive |
| Suspension state misses a semantic nuance (evaluation order, guards) | HIGH | Per-variant continuation matrix (§7/§8) + differential oracle + adversarial review |
| `finally` pending-result handling diverges | HIGH | 15-combination matrix fixture set; dedicated oracle test group |
| Callback protocol changes `map`/`filter`/`reduce` observable behavior | MEDIUM | Oracle + existing stdlib tests; protocol pins snapshot-at-entry iteration and call-site span; native callbacks stay frame-free, closure callbacks frame-counted |
| `closure_sources` keyed by raw `Rc::as_ptr` address, never pruned | MEDIUM | Pre-existing risk (address reuse after a closure drop can misattribute a source in long sessions); the machine reuses the map unchanged. R2 must add a regression probe (allocate/drop many closures, assert source attribution) and, if reachable, take the key off a monotonic id instead of the address |
| Compound-assignment target double-evaluation mistaken for a cacheable read | MEDIUM | Matrix §8 pins re-evaluation; corpus adds `a[idx()] += 1` observability case |
| Native callback mistaken for a counted user frame (or vice versa) | HIGH | §13 pins the `Closure`-only frame rule; corpus adds the `map(to_string)`-at-the-boundary case and the closure-callback counterpart |
| Performance regression | MEDIUM | Fast path (R3G), benchmarks, no correctness trade |
| Machine state keeps growing on long runs | LOW | Bounds: frames ≤ 512, kont ≤ 512×256 (formally ≤ 512 frames × 512 continuation records per frame path); measured in R4 memory checks |
| Native-only behavior changes | MEDIUM | Native is the oracle; differential runs natively first; full native suite in every phase |

## 29. Definition of done (B-1 remediation)

B-1 may be closed only when: the machine is the only engine; the full
differential and native suites are green; the 512-frame boundary holds on
native, Node cold/warm, Chromium main thread, and the production Worker; the
shape matrix traps nowhere below the limit; recovery after `E4011` works; the
frozen artifacts are untouched; an independent reviewer cannot falsify the
implementation; and the full validation gate is green. Until then B-1 stays
OPEN and no runtime release is authorized by this document.
