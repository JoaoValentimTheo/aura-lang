# B-1 — WASM call-frame implementation nonconformance

**Status:** OPEN — **REMEDIATION BLOCKED**, human decision required. No runtime
behavior was changed by this pass; the released language contract is preserved.
The WebAssembly implementation does not currently satisfy it, and the
contract-preserving remedy is a runtime rework outside a stabilization pass.
**Finding:** BREAK-0.2.1 / B-1; correction pass 2026-10-02.
**Classification (corrected):** **IMPLEMENTATION NONCONFORMANCE**, WebAssembly
substrate. `LANGUAGE_SPEC.md` §31.3 and §31.5 reinforce each other; they are not
in conflict. §31.3 states the 512-frame limit as a language rule, "not a host
limitation"; §31.5 forbids host stack overflow for any syntactically valid,
well-formed program. The current tree-walking interpreter violates both on
WebAssembly.
The first stabilization pass misclassified this as "a conflict between §31.3
and §31.5 that only a substrate-calibrated-limit decision or an interpreter
rework can resolve." There is no conflict to resolve between the two rules:
the required behavior is one contract — 512 user call frames, `E4011` beyond,
structured diagnostics, no host failure — identical on every substrate.
**Affected releases:** every published WebAssembly artifact (`0.0.2`, `0.2.0`,
`0.2.1`). The native CLI is conformant on every release. `src/` and
`playground/runtime/src/` are byte-identical to the `v0.2.1` tag, so this is
not a post-0.2.1 regression. `v0.2.1` remains immutable and contains the
defect.

---

## 1. The released contract (as written and as tested)

`LANGUAGE_SPEC.md` §31.3:

> At most **512** simultaneous user call frames MAY be active, including the
> entry call to `main`. Exceeding this is `E4011`. This is a language rule, not
> a host limitation.

`LANGUAGE_SPEC.md` §31.5:

> No syntactically valid, well-formed program may cause a host panic, stack
> overflow, or undefined behavior. Exceeding a limit produces a stable `E####`
> diagnostic.

Answers established from spec, source (`src/run/mod.rs`), and the committed
boundary corpus (`tests/corpus/call-frames/`, `tests/corpus.rs`):

1. A user call frame is one invocation of a **user-defined function, method,
   or closure**. Builtins do not consume a language frame.
2. `main` counts (the limit includes the entry call to `main`).
3. Exactly 512 simultaneous frames are permitted; frame 513 is the first
   over-limit frame and raises `E4011` at the recursive call site.
4. `count(N)` consumes **N + 2** frames (the base-case call and `main`), so
   `count(510)` is accepted (512 frames) and `count(511)` is `E4011` (513).
5. `E4011` is raised when the call that would create frame 513 is entered;
   it is a normal diagnostic at the call expression's span.
6. Builtins are not counted. A closure invoked **from a builtin** (for
   example a `map` callback) is counted — verified: a recursion through a
   callback fails roughly twice as fast as direct recursion.
7. Method calls are counted (they route through the same `Interp::call`).
8. Closure calls are counted.
9. Module boundaries are irrelevant; the same frame accounting applies.
10. The contract is substrate-independent by its own wording. §31.2's explicit
    carve-out for the *parser recursion backstop* ("implementation-safety
    bound ... may differ between execution substrates") does **not** apply to
    §31.3, which says the opposite: "a language rule, not a host limitation."

## 2. Reproduction (released `0.2.1` artifact unless noted)

All WASM probes use a **fresh instance per case** (see §3 on poisoning) with
the production loader (`playground/web/runtime.mjs`) and the real Playground
UI for the Worker column. Native is the shipped release CLI.

| Shape (Aura) | native | Node 24 default (fresh, cold) | Chromium 153 main thread | Chromium 153 **production Worker** |
|---|---|---|---|---|
| thin early-return (`if n <= 0 { return 0 }\n return 1 + f(n-1)`) | 510 ok / 511 `E4011` | 510 ok / 511 `E4011` | cold: traps ~360–400; warmed: 510 ok / 511 `E4011` | traps at **360** |
| `else`-block recursion | 510 ok / 511 `E4011` | traps at **387** (cold) | cold: traps ~400; warmed: reaches 510/511 | traps at **196** |
| `match` recursion | 510 ok / 511 `E4011` | traps at **459** (cold) | — | traps at **233** |
| closure (self-passing) recursion | 510 ok / 511 `E4011` | traps at **356** | — | — |
| in-source module recursion | 510 ok / 511 `E4011` | traps at **387** | — | — |
| method (`impl` + `self`) recursion | 510 ok / 511 `E4011` | 510 ok / 511 `E4011` | — | — |
| mutual recursion | 510 ok / 511 `E4011` | 510 ok / 511 `E4011` | — | — |
| `for`-body recursion | 510 ok / 511 `E4011` | 510 ok / 511 `E4011` | — | — |
| `try`/`catch` recursion | 510 ok / 511 `E4011` | 510 ok / 511 `E4011` | — | — |
| two-branch `if`/`else` chain | 510 ok / 511 `E4011` | traps at **419** | — | — |
| no-arg infinite (`fn f() { f() }`) | `E4011` | `E4011` | — | `E4011` (structured) |

Notes on the WASM columns:

* A trap is a JavaScript `RangeError: Maximum call stack size exceeded` in
  Node, or `E4999 … Maximum call stack size exceeded` (worker error) in the
  Playground. It is **not** `E4011`, and no structured Aura result is
  produced.
* Chromium's **main thread** does not uniformly reach the boundary either: a
  cold first execution of the `else` shape traps around 400, and only after
  warm-up (repeated shallower runs) does the main thread accept `count(510)`
  and report `E4011` at 511. The Playground, however, always executes in a
  fresh **Web Worker** (the hard-cancellation design), and the Worker's
  engine stack is much smaller. That is the production path and the narrowest
  surface.
* Even the *thinnest finite* shape (early return) traps at 360 in the
  production Worker; only the maximally thin `fn f() { f() }` body reaches
  the 512-frame limit and reports `E4011` there.

### Node engine-stack control (causality)

| Node invocation | `else`-shape first trap |
|---|---|
| default (`--stack-size≈984`) | 387 |
| `--stack-size=500` | 193 |
| `--stack-size=1200` | 473 |
| `--stack-size=4000` | none before the limit: 510 ok / 511 `E4011` |

The trap depth is linear in the engine stack size (≈2.5 KB of V8 stack per
Aura call for this shape), which is the direct causality evidence.

### Shadow-stack control (not the binding resource)

Rebuilding the runtime crate with `-zstack-size=16777216` (4× the committed
4 MiB) produces a byte-different artifact with the **identical** trap depth
(387 for the `else` shape). The committed `build.rs` calibration sizes the
parser backstop's stack, not the resource that binds recursion.

### Build-flag / binaryen controls (no lever reaches the contract)

Every alternative build of the same source was measured cold, fresh instance,
`else` shape:

| Build | first trap |
|---|---|
| committed release artifact | 387 |
| `-zstack-size=16777216` (shadow stack ×4) | 387 (identical) |
| `wasm-opt -O1` (binaryen 132) | 402 |
| `wasm-opt -O2` / `-O3` / `-Oz` | 356 |
| `opt-level=3` rebuild | 366 |
| `opt-level=2, codegen-units=16` rebuild | 382 |
| Node `--stack-size=4000` (host stack) | none: full 510/511 |

No artifact-side lever approaches 512; the per-call cost is the number of
WebAssembly engine frames the interpreter structure demands, which
optimization does not remove. The only reachable lever is the host engine
stack size, which a browser does not expose to the artifact.

### Thresholds are state-dependent, not constants

The trap depth depends on the shape, the engine, and the current JIT/execution
state: the same `else` shape traps at 387 when scanned cold from depth 0, at
435–436 when scanned from 300 in the same process, and at different depths on
other engines. Only the *violation* is stable; the depth is not a contract
constant.

### Cross-release

The same probe on the frozen artifacts (Node, fresh instance, cold scan from
depth 0) first fails at 317 (`0.0.2`), 389 (`0.2.0`), and 387 (`0.2.1`) for
the `else` shape; all three tags declare `MAX_CALL_FRAMES = 512`. The failure
kind differs by release: `0.0.2` — which predates the committed
`-zstack-size` linker calibration — fails with `memory access out of bounds`
(a guest shadow-stack failure), while `0.2.0`/`0.2.1` fail with the JavaScript
`Maximum call stack size exceeded` (an engine-stack failure). Either way the
contract is violated and the depth is far below 512. The defect predates
`0.2.1` and was never tracked.

### Post-trap instance state (secondary host-failure effect)

* One trap does not poison the instance: an unrelated trivial program still
  runs `ok` afterwards (5/5 repetitions), and a shallow recursion still runs.
* Repeated traps **degrade the instance cumulatively**. After two consecutive
  deep-recursion traps (`else` at 510) with no intervening success, shallow
  programs still run (measured `count(30)`…`count(150)` → `ok`), but deeper
  execution — measured `count(200)` and up — fails with `memory access out of
  bounds` instead of running or trapping cleanly. A third trap without an
  intervening success fails directly with `memory access out of bounds`.
* A successful shallow run between traps does **not** restore the instance:
  `trap → count(100) ok → trap → count(200)` reproduces `memory access out of
  bounds` (3/3; also 6/6 in the reviewer's independent probe).
* Two traps followed by a shallow success (`count(50) ok`) and then a trivial
  program still behave normally in the measured depth range, so the corrupted
  state is not a permanent global failure — it is a degraded engine/shadow
  stack whose usable depth shrinks.
* The Playground creates a **fresh Worker per run**, so the product contains
  the degradation per run; any host that reuses an instance across runs (the
  Node harness, embedders) is exposed. A JS `catch` of the `RangeError`
  cannot restore the instance or yield a structured `E4011`, so "catch and
  convert" is not a remedy.

## 3. Root cause

One Aura call is executed as a chain of recursive Rust→WebAssembly calls
(`eval → eval_inner → eval_call → call → exec_block → exec_stmt → eval …`).
Each such chain consumes V8 (SpiderMonkey/JSC) **engine-stack** frames per
Aura call. The interpreter's own counter (`depth`, `MAX_CALL_FRAMES = 512`)
correctly bounds the logical contract, but the engine stack can be exhausted
first, and the artifact has no control over engine-stack sizing:

* the wasm linear ("shadow") stack is not the binding resource (16 MiB control
  build: identical behavior);
* there are zero wasm imports, so there are no JS→WASM→JS transitions in the
  execution path — the frames are wasm-to-wasm engine frames;
* browsers expose no stack-size knob to the artifact; `node --stack-size` is a
  host flag and not shippable in the Playground;
* per-call host cost varies by control-flow shape and by JIT state, so a fixed
  host-safe watermark cannot be proven safe across engines and browser
  classes.

## 4. Why this is blocked rather than fixed in this pass

The contract-preserving fixes were evaluated and each fails a requirement of
the task:

1. **Increase the shadow/linear stack** — proven ineffective (§2).
2. **Catch the trap in JS and convert it to `E4011`** — repeated traps degrade
   the instance (a later, otherwise-legal run fails `memory access out of
   bounds`) and no structured language result can be produced; the acceptance
   boundary would still diverge.
3. **Lower `MAX_CALL_FRAMES` on WASM (substrate-calibrated cap)** — a semantic
   change: a program recursing 300 deep runs natively and would be rejected
   in the browser; this contradicts §31.3's own "not a host limitation"
   wording and reintroduces exactly the divergence ADR-0004 removed for type
   nesting. It cannot be proven safe across unmeasured engines either.
4. **Probe the engine ceiling at startup and cap dynamically** — the probe
   itself can trap; behavior becomes engine-dependent; still a contract
   change.
5. **Narrow transform (for example trampolining direct self-recursion)** —
   cannot prove the general 512-frame contract across expression shapes,
   `try`/`catch`, callbacks through builtins, methods, and closures; leaves
   engine-stack dependence in general recursion.
6. **Artifact-side build/codegen levers** (linker stack size, `wasm-opt`
   at any level, Rust `opt-level`/`codegen-units`) — all measured; none
   approaches the contract (§2, build-flag controls).

The only contract-preserving remedy is an **engine-stack-independent
evaluator**: replacing implicit Rust recursion in `src/run/mod.rs` (2,303
lines; 51 recursive `eval` call sites; `exec_block`/`exec_stmt`; `Ctl`
propagation; `try`/`finally` via `pending_throw`; closures; methods; overload
selection; source provenance; and the builtin higher-order callbacks that
re-enter Aura through `call_value_pub`) with an explicit managed frame or
continuation stack. That is a runtime-architecture change with its own design,
differential oracle, review (AURA_ENGINEERING_ORG.md lists "Resource limit
change" and runtime architecture under Runtime review + Red Team + human
decision), performance program, and release. It is disproportionate for a
stabilization pass, and the mandate explicitly forbids starting unrelated
runtime redesigns or changing the released semantics automatically.

## 5. Options for the human decision

* **Option B — engine-independent evaluator (contract-preserving,
  recommended).** Replace the recursive evaluation core with an explicit
  frame/continuation machine so the 512-frame contract and `E4011` hold on
  every substrate. Cost: a dedicated runtime program; differential oracle
  against the current interpreter; performance validation. Risk: the largest
  change to the runtime since the rewrite.
* **Option A — substrate-calibrated cap (contract change).** Cap effective
  frames on WASM (safely below every measured floor, e.g. 150–192) and report
  `E4011`; amend §31.3 and the public limitations. Cost: small, testable.
  Risk: knowingly makes acceptance substrate-dependent; still cannot be
  proven safe for every browser class.
* **Option C — probe/dynamic cap (contract change).** As A, but measured at
  runtime. Cost: medium. Risk: the measurement itself can trap; behavior is
  engine-dependent.
* **Do nothing (status quo).** The released defect stays; §31.5 remains
  violated in the Playground. Not acceptable as a durable state.

The Council/human must choose. Nothing in this pass changes runtime behavior.

## 6. Coverage added by this pass (and what is intentionally absent)

Added, and green:

* Native boundary corpus for six recursion shapes at the exact boundary:
  `else`, `match`, `method`, `closure`, `module`, and `try`/`catch`
  (`tests/corpus/call-frames/*_510.aura` = accepted, `*_511.aura` = `E4011`).
  These are the durable native oracles for the eventual evaluator work.
* A safe-depth (150) cross-substrate **shape matrix** in
  `playground/tests/node/differential.test.mjs` covering the mainstream
  `else`, `match`, closure, module, method, mutual, `for`-body, and
  two-branch shapes (native and WASM must agree).
* A Worker-path assertion that the thinnest boundary shape reaches a
  structured `E4011` in the real Playground (`browser.test.mjs`), alongside
  the existing mainstream-shape safe-depth guard.

Intentionally absent until remediation lands: WASM limit−1/limit/limit+1
assertions. They cannot pass while the defect exists, and encoding the defect
as an expected failure is not acceptable coverage. They must be added by the
remediation program.

## 7. Evidence commands

```sh
# native boundary (release CLI): count(510) prints, count(511) is E4011
./target/release/aura run tests/corpus/call-frames/else_510.aura
./target/release/aura run tests/corpus/call-frames/else_511.aura

# wasm: mainstream shape traps below the limit (fresh instance per case)
node --input-type=module -e '...'   # see the correction-pass probe harness

# engine-stack control: same artifact, larger engine stack reaches 510
node --stack-size=4000 <probe> playground/runtimes/0.2.1/aura_playground_runtime.wasm

# shadow-stack control: 4 MiB vs 16 MiB linker stack -> identical behavior
```

## 8. Decision required

Choose Option A, B, or C (or another explicit policy). Until then:

* `v0.2.1` remains immutable and contains the defect;
* native behavior and the released contract are unchanged;
* B-1 stays open and blocks the post-0.2.1 freeze.
