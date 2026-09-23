# POST-FEATURE_002 — Checker Stack Hardening Audit

**Scope:** the single implementation detail introduced by Feature 002:
the checker now runs on the large interpreter stack because the enlarged AST
reduced recursion headroom.
**Baseline:** `9015b08` (`feat: implement named function arguments`)
**This is an audit, not Feature 003.**

---

## Executive Summary

The audit verified that moving the checker onto the large interpreter stack is
a **pure execution-environment hardening change**: it does not alter any
language-level resource limit or diagnostic. All semantic limits
(`MAX_AST_DEPTH = 256`, `MAX_CALL_FRAMES = 512`, the parser recursion backstop,
the 10,000,000-element range cap) are unchanged and are still enforced
independently of native stack size.

The audit also found and fixed **one real gap** exposed by the same line of
investigation, and **one recurrence of the original latent hazard**:

1. **AST-depth enforcement gap (corrected).** `parse_stmt` and `parse_expr` did
   **not** apply the iterative `enforce_depth` check that `parse` (module) does.
   The REPL's statement-first path therefore accepted an over-deep statement
   and then walked it in the checker on the small caller thread, producing a
   native stack overflow instead of `E1015`. Fixed by enforcing the AST limit
   on every parse entry point.
2. **REPL entry-point hardening (completed).** The shipped `aura repl` now runs
   the whole session on the large stack, so its behavior no longer depends on
   the platform's default main-thread stack size. The library's `run_with`
   testable core remains caller-thread-based and is documented as such.

After the fixes, CLI and REPL agree exactly at the nesting boundary (253
accepted, 254 rejected with `E1015`), no supported entry point overflows, and
the full suite passes (194 all-features / 188 no-default-features).

Outcome: **PASS WITH TEST HARDENING** (minimal, targeted source fix + tests +
this report).

---

## Stack Change

**Where.** `src/lib.rs`.

* `on_interp_thread` spawns a thread with `INTERP_STACK = 64 MiB`.
* `compile_with_mode` runs the checker via `check_on_big_stack`, which uses
  `on_interp_thread`, returning the module after checking.
* A new `pub fn on_large_stack` exposes the same large-stack runner so the REPL
  can use it.
* `repl::run` (the shipped `aura repl`) now runs the whole session inside
  `on_large_stack`.

**Why.** The checker recurses over the AST (`Checker::expr`/`stmt`). A program
at the semantic nesting limit needs a few MiB of native stack to check; the
caller's thread may have less (notably the Windows main thread). Running the
checker on the same bounded large stack used for parsing and execution makes
the language limit (`E1015`/`E4011`) the only bound a user can hit, per §31.5.

**Stack owner.** `lib::on_interp_thread` / `on_large_stack` (64 MiB,
`.name("aura-interp")`).

**Affected entry points.** CLI `run`/`check`/`eval` (via `compile_with_mode`),
REPL (via `repl::run`). The Python bridge does not invoke the checker and is
unaffected.

**Previously.** Parsing already ran on a 64 MiB stack (`PARSE_STACK`); the
checker ran on the caller thread. Execution already ran on the interpreter
thread. Only the checker's context changed.

---

## Resource Limits

All limits are unchanged constants; the native stack does not affect them.

| Limit | Value | Location | Enforced |
|---|---|---|---|
| AST nesting (`E1015`) | 256 | `parse::MAX_AST_DEPTH`, `check::MAX_AST_DEPTH`, `run::MAX_AST_DEPTH` | iteratively, after parse, on every entry point |
| Parser recursion backstop (`E1015`) | 2048 frames | `parse::MAX_PARSE_DEPTH` | parser |
| Flat-chain node budget (`E1015`) | 256 | `parse` `count_node` | parser |
| Call frames (`E4011`) | 512 | `run::MAX_CALL_FRAMES` | runtime |
| Range materialization (`E4013`) | 10,000,000 | `run` `MAX_RANGE_MATERIALIZE` | runtime |

**No `N+1` acceptance.** A larger native stack does not make an over-limit
program valid: `254` nested lists is rejected by `E1015` on every entry point,
exactly as before.

---

## Semantic Stability

* **Language limits preserved.** Boundary probing shows CLI and REPL both
  accept 253 and reject 254 (with the `print(...)` wrapper) with `E1015`. The
  same constants appear in parse, check, and run.
* **Diagnostics preserved.** Over-limit input yields `E1015`; runaway recursion
  yields `E4011`; the range cap yields `E4013`. No `E4999`, panic, or abort.
* **No native overflow at supported limits.** The shipped CLI and REPL handle a
  valid near-limit program without crashing.

---

## Entry Points

| Path | Checker context | Result |
|---|---|---|
| CLI `run` / `check` / `eval` | large stack (`compile_with_mode`) | safe |
| REPL (`aura repl`) | large stack (`repl::run` → `on_large_stack`) | safe |
| REPL testable core (`run_with`) | caller thread (by design) | documented; enforces `E1015` at parse |
| Python bridge | does not run the checker | unaffected |

The REPL and CLI agree at the nesting boundary. The testable core remains a
library helper that runs on the caller's thread; it now enforces the semantic
limit during parsing, so over-limit input is rejected rather than overflowed.

---

## Feature 002 Interaction

* **Named calls.** Deeply nested named/mixed/reordered calls are bounded by the
  same `MAX_AST_DEPTH`; `check_user_call` and `bind_arguments` do not introduce
  an unbounded recursion path (they are iterative over the argument list).
* **Deep nesting.** Verified: nested call chains (`id(id(...))`) at 300 deep
  return `E1015`.
* **Pipeline.** `x |> f(y: 1)` desugars to an ordinary call and is covered by
  the same limits.
* **No double evaluation.** Verified by side-effecting probes: each argument
  evaluates once, in source order.

---

## Platform Considerations

* The large stack is a fixed 64 MiB thread, independent of the platform's
  default main-thread stack (Linux/macOS ~8 MiB; Windows PE default 1 MiB).
* Running the shipped REPL and all CLI paths on the large stack makes their
  behavior platform-independent for programs up to the semantic limit.
* The `run_with` testable core runs on the caller's thread; its safety near the
  limit depends on the caller's stack. This is documented, not hidden.
* No unsafe code, no thread-local or global mutable state, no lifetime beyond
  the owned/moved module. `on_large_stack` propagates a `Result` and never
  panics; a thread-start failure becomes a `FOREIGN` diagnostic and a panicked
  worker becomes an `INTERNAL` diagnostic, both non-fatal to the host beyond a
  clean error.

---

## Performance Sanity

Measured in release on this machine:

* `compile` (parse thread + check thread) for a trivial program: ~72 µs.
* `run_source` (compile + execute threads): ~122 µs.
* A bare 64 MiB thread spawn: ~32 µs.

The overhead is dominated by parse/check work, not thread creation; the CLI is
a batch tool, so the cost is negligible. No benchmark framework was added.

---

## Corrections Applied

1. `src/parse/mod.rs`: `parse_expr_inner` calls `check_expr_depth(&e, 1)`;
   `parse_stmt_inner` calls `check_stmt_depth(std::slice::from_ref(&s), 1)`.
   This enforces the same `MAX_AST_DEPTH`/`E1015` bound as `parse`, closing the
   over-limit path through the REPL.
2. `src/repl.rs`: `run()` runs the session on the large stack (owned buffered
   handles); the per-submission check calls were reverted to the simpler direct
   form (the parser now enforces the limit, and the shipped session is already
   on the large stack).
3. Regression tests:
   * `tests/boundaries.rs::checker_survives_the_semantic_nesting_limit`
     (near-limit accepted; over-limit `E1015`; deep call chain `E1015`).
   * `tests/repl.rs::repl_nesting_boundary_is_a_diagnostic_not_a_crash`
     (over-limit `E1015`; modest nested submission evaluates).

---

## Final Conclusion

**STACK HARDENING VERIFIED.** The large-stack change is an
execution-environment hardening; no language limit, diagnostic, or semantics
changed. The audit additionally closed an AST-depth enforcement gap in the
`parse_stmt`/`parse_expr` entry points and completed REPL entry-point
hardening. No correction beyond this scope is required before Feature 003.
