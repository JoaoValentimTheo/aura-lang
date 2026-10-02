# DECISION REQUIRING HUMAN APPROVAL — WASM call-frame trap below the 512-frame language limit

**Status:** OPEN — do not implement a fix until decided.
**Finding:** BREAK-0.2.1 / B-1 (post-0.2.1 stabilization program, 2026-10-02).
**Nature:** a conflict between two normative rules (`LANGUAGE_SPEC.md` §31.3 and
§31.5) that only a substrate-calibrated-limit decision or an interpreter rework
can resolve. It changes observable acceptance or the call-frame contract, so it
is a specification decision, not a routine fix.
**Affected releases:** `0.0.2`, `0.2.0`, `0.2.1` (all WASM artifacts). The
native CLI is correct on every release. This is not a post-0.2.1 regression:
`src/` and `playground/runtime/src/` are byte-identical to the `v0.2.1` tag.

---

## 1. The problem

`LANGUAGE_SPEC.md` §31.3 states the call-frame limit as a language rule:

> At most **512** simultaneous user call frames MAY be active, including the
> entry call to `main`. Exceeding this is `E4011`. This is a language rule, not
> a host limitation.

`LANGUAGE_SPEC.md` §31.5 states the host-failure rule:

> No syntactically valid, well-formed program may cause a host panic, stack
> overflow, or undefined behavior. Exceeding a limit produces a stable `E####`
> diagnostic.

On WebAssembly, the two rules cannot both hold with the current interpreter.
Each Aura call consumes several WebAssembly call frames, and V8 sizes the
native (engine) stack, not the wasm linear stack, as the binding resource.
Below the 512-frame language limit, a legal recursion exhausts the engine stack
and the guest traps (`RangeError: Maximum call stack size exceeded` in Node; a
Worker error surfaced by the Playground as `E4999`) instead of producing
`E4011`. On native, the dedicated 64 MiB execution thread keeps `E4011`
authoritative and no legal program aborts.

## 2. Reproduction (released `0.2.1` artifact and source at HEAD)

Mainstream shape — a recursive function with an `else` block:

```aura
fn count(n: int) -> int {
  if n <= 0 {
    return 0
  } else {
    return count(n - 1) + 1
  }
}
fn main() { print(count(N)) }
```

| N | native (release CLI) | WASM (Node, committed 0.2.1 artifact) | WASM (Chromium, real Playground Worker) |
|---|---|---|---|
| 195 | `count` prints | prints | prints |
| 196 | prints | prints | **Worker error: `E4999 Maximum call stack size exceeded`** |
| 396 | prints | prints | Worker error |
| 397 | prints | **`RangeError` (host trap, no JSON result)** | Worker error |
| 510 | prints (last accepted; 511 frames incl. `main`) | host trap | Worker error |
| 511 | `E4011` (first rejected) | host trap | Worker error |

The frame count includes the terminal `count(0)` call and the entry `main`, so
`count(N)` consumes N + 2 frames and the first rejected depth is `count(511)`
(native), matching `tests/corpus.rs` (`simple_510` accepted / `simple_511`
`E4011`).

The wasm trap is not caught by `runtime.mjs`; the Worker surfaces it as an
unstructured load error. The instance is not poisoned — a subsequent unrelated
program still runs (verified) — but the language contract is broken and the
native/WASM acceptance boundary diverges by ~115 frames (Node) to ~315 frames
(Chromium).

### Shape dependence (fresh instance per probe, Node)

The trap depth depends on the per-call Rust frame cost of the shape:

| Shape | last structured run | first host trap |
|---|---|---|
| `else`-block recursion | 396 | 397 |
| method recursion (`impl` + `self`) | 396 | 397 |
| `for`-body recursion | 335 | 336 |
| two-level `if`/`else` chain | 273 | 274 |
| `match`-based recursion / thin early return | 511 (`E4011`) | — (reaches the language limit) |

The committed `tests/corpus/call-frames/*` shapes (`simple`, `generic`,
`mutual`) are the thin shape and reach the language limit on WASM; the corpus
therefore passes while mainstream `else`-block recursion does not.

## 3. Root-cause evidence

1. **Not the wasm shadow stack.** Rebuilding the runtime crate with
   `-zstack-size=16777216` (4× the committed 4 MiB) leaves the boundary at
   exactly 397. The committed `build.rs` calibration governs the *parser*
   backstop; it does not size the engine stack the interpreter runs on.
2. **The V8 engine stack is binding.** Running the Node probe with
   `node --stack-size=4000` moves the same program's last structured run from
   396 to 510 — i.e. the interpreter then reaches the full 512-frame language
   limit and `E4011` fires. The engine stack, not the guest linear stack,
   bounds recursion.
3. **The trap is deterministic** for a given shape/instance state, and the
   instance recovers for later runs (a trap does not poison it).
4. Native does not exhibit the defect in the shipped (release) profile: the
   dedicated `INTERP_STACK = 64 MiB` thread makes `E4011` authoritative. (A
   *debug* build inflates Rust frames enough to abort at ~382 on the
   `if`-chain shape; the release profile accepts to the language limit. The CI
   `boundaries` suite runs tests on the production stack configuration and
   passes.)

## 4. Why this is a decision, not a mechanical fix

Any fix changes one of the two normative rules or the interpreter:

- Lowering `MAX_CALL_FRAMES` on WASM only (§31.3 becomes
  substrate-calibrated, like the parser backstop in §31.2) is a **semantic
  change**: a program that recurses 200 deep runs natively and would become
  `E4011` on WASM, reintroducing exactly the native/WASM acceptance divergence
  ADR-0004 removed for type nesting.
- Making the interpreter iterative (explicit frames) removes the engine-stack
  dependency, but that is a runtime rework, not stabilization work, and would
  change the released runtime's behavior beyond a fix.
- Leaving it as-is documents a §31.5 violation, which §31.5 exists to forbid.

## 5. Options

**Option A — substrate-calibrated call-frame limit (documented divergence).**
Cap effective frames on WASM (safely below the measured 273-frame floor, e.g.
192 or 256) and report `E4011`; document the divergence in §31.3 and the known
limitations page; add N-1/N/N+1 parity tests.
*Pro:* small, testable, restores a structured diagnostic on every substrate;
matches the §31.2 precedent for host-safety bounds.
*Con:* knowingly makes acceptance substrate-dependent; a 300-deep recursion is
valid natively and rejected in the browser.

**Option B — iterative interpreter (frame rework).**
Replace the recursive `exec_block`/`expr` evaluation stack with an explicit
heap-managed frame stack so the engine stack no longer bounds Aura recursion.
*Pro:* restores §31.3 and §31.5 on every substrate with identical semantics.
*Con:* a major runtime change needing its own design, differential oracle, and
release; out of stabilization scope.

**Option C — hybrid: capability probe / documented limitation.**
Keep 512 but detect the engine's practical ceiling at startup and lower
`MAX_CALL_FRAMES` to the measured safe value, reporting `E4011`.
*Pro:* no fixed constant to guess; adapts across engines.
*Con:* measurement itself can trap; behavior becomes engine-dependent; still a
documented divergence.

**Recommendation (for the Architecture Decision Council / human owner):**
Option A as the stabilization fix — a conservative WASM cap (with the measured
floor recorded as evidence and N-1/N/N+1 tests) plus an explicit §31.3 note —
with Option B recorded as the v1.1 direction that removes the divergence
properly. The exact cap and the §31.3 wording are the decision.

## 6. Regression coverage proposed with this package

`playground/tests/node/differential.test.mjs` and
`playground/tests/browser.test.mjs` gain a mainstream-shape recursion parity
guard at a depth that is demonstrably safe on every substrate (150), so a
regression that reduces engine-stack headroom further is caught now. The guard
does **not** encode the decision; it pins the working region.

## 7. Evidence commands

```sh
# native (release): accepts count(510); count(511) is E4011
cargo build --locked --release --no-default-features \
  --features cli,repl,json,time | true
./target/release/aura run <count(510)>.aura   # prints
./target/release/aura run <count(511)>.aura   # E4011

# wasm: traps at 397 (Node) / 196 (Chromium) instead of E4011
node playground/tests/node/differential.test.mjs \
  playground/runtimes/0.2.1/aura_playground_runtime.wasm \
  playground/runtime/target/release/aura-playground-native

# engine-stack control: the same probe reaches 510 under a larger V8 stack
node --stack-size=4000 playground/tests/node/<probe>.mjs

# shadow-stack control: 4 MiB vs 16 MiB build.rs stack-size -> identical 397
```

---

**Decision needed:** A, B, or C (or another explicit policy), including the
exact WASM cap if A/C. Until decided, no runtime behavior is changed.
