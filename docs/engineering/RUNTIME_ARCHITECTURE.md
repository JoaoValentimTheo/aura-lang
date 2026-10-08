# Aura Runtime Architecture (Pre-0.3 Foundation)

Status: architecture record of the production runtime as implemented after the
B-1 cutover. It documents the long-term shape so later work does not create a
second accidental authority. No behavior is proposed here.

---

## 1. Production authority

**The explicit-continuation machine (`src/run/iterative.rs`) is the runtime.**
Every production entry point reaches it:

| Entry point | Path | Verified |
|---|---|---|
| `Compilation::execute_with` / `execute_with_host_factory` | `run_iterative` | `src/lib.rs:484` |
| `Compilation::execute_iterative` | `run_iterative` | `src/lib.rs:443` |
| free `aura::execute_with` | `run_iterative` | `src/lib.rs:713` |
| `run_source` / `run_program` | `execute_with*` (call sites) | `src/lib.rs:660`, `:673` |
| CLI (`aura run`, `aura eval`) | `execute_with` | `src/main.rs:215`, `:218` |
| REPL statement/expression/const | machine methods | `src/repl.rs` (B-1R6 review) |
| Playground `execute` / `run_module_capture` | `execute_with_host_factory` / `run_iterative` | `playground/runtime/src/lib.rs:535`, `:647` |

The retained recursive evaluator is reached only through
`execute_recursive*` (`src/lib.rs:352-380`), which exists solely as the
differential reference and rollback path (B-1R8, human-gated).

Mechanical guard: `tests/production_routing.rs` pins each production entry's
machine call spelling; `tests/evaluator_oracle.rs::engines_agree` requires the
engines to agree on the whole corpus.

## 2. Machine structure

| Component | Definition | Role |
|---|---|---|
| `Machine` | `src/run/iterative.rs` | one `loop` over control items; owns the `Vec<Cont>` continuation stack and the user-frame stack |
| `Ctrl` | `:266` | scheduler states (start/resume/deliver/done) |
| `Control` | `:174` | the loop's next action |
| `Cont` | `:285` | continuation frames: one per suspended expression/statement context (list/map/range/call/while/match/try/…), plus accumulator state |
| `UserFrame` | `:620` | one Aura call: closure, env, source, restorable expression-depth budget |
| `Cont::FrameBoundary` | `:612` | call/return crossing bookkeeping (resumed at `:1947`) |

The machine consumes only the continuation stack; it never re-enters the AST
recursively and never grows the host stack per Aura call. `tests/b1_stack_safety.rs`
pins host-stack independence for deep calls, long loops, and deep expressions.

## 3. Shared value-level helpers (deliberate, not dual authority)

The recursive and iterative engines share value-level helpers that **never
evaluate an AST node**: `Interp::binary`, `index_get`, `field_get`, `method`,
`format_value`, `iterate`, `bind_pattern`, `match_pattern`, `error`.

- These are the single semantic authority for their operation: both engines
  call them, so arithmetic, formatting, indexing, matching, and diagnostics
  cannot drift between engines.
- Known residual: `bind_pattern`/`match_pattern` recurse on the **host stack**
  proportional to pattern nesting. Both engines use the identical helper, so
  behavior matches, and pattern nesting is bounded by `MAX_AST_DEPTH`
  (`E1015` above). This is a recorded residual, not a hidden seam; converting
  it to machine work would be a bounded future improvement with no behavior
  change.

## 4. Frames, environments, values

- **Frames**: `MAX_CALL_FRAMES = 512` (`src/run/mod.rs`), enforced on the one
  shared depth counter (`push_frame`, `src/run/iterative.rs:2921`) with `E4011`
  at the call-site span; the machine restores `depth` on every unwind path.
- **Environments**: `Env(Rc<RefCell<EnvData>>)` with `define`/`define_shadowing`/`get`;
  `let` shadows, `let mut`/fields mutate. Deliberately `Rc`, not `Arc` (B-1R3A
  decision; runtime is single-threaded by design).
- **Values**: `Value` in `src/run/value.rs`; `Rc` sharing for list/map; render
  depth/nodes bounded (`MAX_VALUE_DEPTH = 512`, `MAX_VALUE_NODES = 1_000_000`).
- **AST sharing** uses `std::sync::Arc` (B-1R3A-ARCH-1) so a compiled module
  can be executed repeatedly and compared between engines.

## 5. Limits and control signals

- Limits: AST depth 256 (`E1015`), frames 512 (`E4011`), range
  materialization 10M (`E4013`), format width/precision (`E4013`), parser
  recursion budget (2048 native / 768 wasm). Full table in
  `docs/engineering/CRITICAL_SYSTEM_PROFILE.md` §4.
- Control signals: `Ctl::{Val, Return, Break, Continue, Throw}`.
- Fatal vs catchable: only `Ctl::Throw` is catchable; diagnostics are fatal
  (`TryResult::Fatal(Diag)`) but always run `finally`
  (`Cont::TryCatchEnd`/`Cont::TryFinally` snapshot `frames_len`/`depth`/
  `saved_expr_depth` so an error in a catch/finally body cannot leak a frame).
- Cross-frame throws recover the value via the machine's pending-throw path;
  `E4099` is the internal crossing signal, never user-visible.

## 6. Native/WASM differences

| Aspect | Native | WASM |
|---|---|---|
| Entry into execution | `on_execution_stack` gives a dedicated 64 MiB stack | runs inline (no thread, no stack knob) |
| Parser recursion budget | 2048 | 768 (calibrated below the engine ceiling) |
| Parser grouping backstop (pure parentheses) | 2048 | 192 (a grouping level costs several frames; measured ceiling 262) |
| Host | `StdHost`/`LimitedHost`/custom | `LimitedHost`/`BrowserHost` |
| stdout | process stdout (unbounded) | capture buffer (1 MiB policy bound) |
| fs/clock/sleep | available by default | `E5002` |
| CPython | optional (`py`) | absent |
| **Language semantics** | identical | identical (`engines_agree`, fresh-wasm boundary suite) |

WASM restrictions are host policy, never language restrictions.

## 7. Diagnostics and entry errors

All entries return `error::Result<T>` with `Diag` carrying code/message/span or
`SourceDiagnostic` carrying source identity when the diagnostic crosses a
source boundary. The Playground wrapper converts these to the structured JSON
protocol without reinterpretation.

## 8. B-1 scaffolding disposition

- The `evaluator-oracle` feature gates only the differential harness; the
  machine is always compiled. The stale "feature selects the engine" wording
  is corrected in `Cargo.toml` and the design doc.
- The recursive engine and the oracle switch are retained by explicit human
  policy; **B-1R8 removal is human-gated** and is not proposed here.
- No other B-1 scaffolding was found that can be simplified without touching
  the retained reference path.

## 9. Invariants future work must preserve

1. One production engine; no second entry point may bypass the machine.
2. Shared value-level helpers remain the single semantic authority; no helper
   may grow an AST-evaluating path.
3. `depth`/frame accounting has exactly one owner (`push_frame`) with
   restore-on-unwind on every path.
4. `E1015`/`E4011` are fatal and never catchable.
5. The recursive reference stays behaviorally identical while retained
   (`engines_agree`).
