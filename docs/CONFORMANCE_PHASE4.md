# AURA LANGUAGE CONFORMANCE PASS — PHASE 4

## 1. Scope

Function execution, stack frames, recursion, closures and capture, blocks,
loops, `return`, `break`/`continue`, `try`/`catch`/`finally`, `throw`, runtime
errors, and host cancellation — verifying that checker-approved programs
execute per `docs/LANGUAGE_SPEC.md` §§14–15, 23, 31.

Starting HEAD: `73c7b63`.

## 2. Function Execution

| Property | Observed | Spec | Status |
|---|---|---|---|
| Parameter binding | by position/named; `mut` grants body capability | §15 | correct |
| `mut` param does not mutate caller's value | `f(mut x)` reassigns a copy; caller unaffected | §16.6 | correct |
| Implicit `none` return | a function without `return` yields `none` | §15 | correct |
| Recursion / mutual recursion | hoisted, works | §15.6 | correct |
| Generic functions, methods, callable values | resolve and dispatch | §§15.7, 17 | correct |
| Call-frame limit | 510 recursive calls accepted, 511 rejected with `E4011` — i.e. 512 frames including the entry `main` | §31.3 | correct |
| Deep recursion is an Aura diagnostic, never a host overflow | `E4011` | §31.5 | correct |

## 3. Closures

* Lexical capture by reference semantics: a closure observes later mutations of
  a captured `mut` variable (`x = 5` then calling prints `5`).
* Two closures over one variable share state; the object is not snapshotted.
* Each loop iteration binds a **fresh** variable, so closures created in a loop
  capture distinct values (`[() -> i]` in `for i in 0..3` prints `0 1 2`).
* Closures returned from functions retain their captures.
* Recursive local closures are not supported (the binding is not in scope
  inside its own initializer) — a documented consequence of lexical binding, a
  DESIGN LIMITATION, not a bug.

## 4. Control Flow

`while`, `loop`, `for`, nested `break`/`continue`, `return` inside loops,
`return` inside `try`, and `try` inside loops all behave per spec. `break` and
`continue` outside a loop are rejected (`E2015`). No hidden incorrect state
was observed across nesting.

## 5. Exceptions

| Case | Observed | Spec | Status |
|---|---|---|---|
| `throw e` / `catch x` binding | binds the thrown value | §14.5 | correct |
| Only explicit `throw` is catchable | runtime `E4xxx` are not catchable and propagate | §14.5 | correct |
| Uncaught throw | `E4026` with the rendered value | §14.5 | correct |
| `finally` runs on every exit path | normal, `return`, `break`, `continue`, `throw` | §14.6 | correct |
| `finally` control-flow signal replaces pending outcome | `throw` in `finally` overrides a `return` | §14.6 | correct |
| `catch` mandatory | `try` without `catch` is `E1006` | §14.5 | correct |
| Nested catch / rethrow | innermost catch wins; rethrow propagates | §14.5 | correct |

`catch`/`finally` must be written on the same line as the preceding `}` (§3.7);
the parser enforces this.

## 6. Entry Point

`main` is required for `run`; a missing/renamed/nested `main` is `E4027`, and a
`main` declared with parameters is `E2011`. Multiple `main` declarations are
caught. These match the `E4027`/`E2011` contract.

## 7. Host Cancellation

`Host::should_cancel` is documented as **advisory**; the hard-cancel mechanism
is external termination of the execution instance (`src/host.rs`). A bare CLI
runaway `while true` loop therefore does not self-terminate — by design, not a
defect. The browser/worker host bounds execution externally and by node/depth
budgets. No `E4999` is reachable from a user mistake (§31.5). No regression to
prior `E4999` work was found.

## 8. Value Structural Depth

Rendering and JSON encoding are bounded at `MAX_VALUE_DEPTH = 512`; teardown is
iterative. Verified with a 600-deep nested list: rendering truncates with `…`
rather than overflowing.

## 9. Findings

**No new implementation defects.** All probed execution semantics match the
normative contract. The recursive-local-closure restriction is a documented
DESIGN LIMITATION.

## 10. Tests

Existing suites `tests/run.rs`, `tests/io.rs`, `tests/adversarial.rs`,
`tests/boundaries.rs` cover this layer; all pass. No new permanent test was
required because no defect was found.

## 11. Phase-5 Handoff

Execution semantics are verified and match the checker's static model. The only
runtime-vs-static split is the documented §9.1 arithmetic case and the
`Unknown` boundary; both were confirmed in Phase 3.
