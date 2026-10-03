# Differential evaluator oracle (B-1R2)

This directory is the **test-only measuring instrument** for the B-1
engine-stack-independent-evaluator migration. It is *not* an evaluator and it
changes **no** production behavior. The architecture contract is
`docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md` §24–§25.

## What it is

`tests/evaluator_oracle.rs` observes every corpus case through `tests/oracle/mod.rs`
and compares the normalized observable against:

1. the committed golden manifest `tests/oracle/golden.tsv` (pins current
   behavior), and
2. every engine available on this build (baseline self-comparison today;
   recursive vs iterative under the `evaluator-oracle` feature in B-1R3).

## Isolation

Every `observe` call reconstructs the entire run from source: fresh front end,
fresh `Compilation`, fresh execution thread, fresh `Interp`, fresh host, fresh
stdout buffer. No mutable state is shared between observations, so one side can
never contaminate the other. `execution_is_isolated` proves this.

## Normalized observable

Only contract-meaningful fields are compared. Raw `Rc` addresses, `SourceId`
scope bits, and thread ids never enter the comparison. A diagnostic is
normalized to `(code, message, source display name, byte span, line, column)`,
so `E3001` at `a.aura:5:9` is never equal to `E3001` at `b.aura:5:9` even with
identical byte spans. Stdout is compared as **exact bytes**.

The final value is **type-tagged** (`NormValue { ty, repr }`), because
`display()` erases type: `1`, `1.0`, and `"1"` all render as `1` at top level.
Cyclic values cannot hang the comparison because only the bounded `debug_repr`
string is compared, never the `Value` graph.

## Mutation sensitivity

`tests/oracle/mutation_experiments.sh` applies 11 deliberate semantic mutations
to `src/run/mod.rs`/`src/run/value.rs` one at a time, asserts the oracle detects
each, and restores every file (verifying the SHA-256). It exits non-zero if any
mutation escapes or a file is left modified. It is run manually because it edits
a tracked file.

## Engine adapter

`harness::Engine` is the only seam. B-1R3 selects the machine through
`Engine::iterative()` (feature `evaluator-oracle`) and the same
`observe`/`Observable`/golden machinery compares both sides without redesign.

## The `evaluator-oracle` feature

Non-default, not user-facing, no CLI option, no runtime semantic branch in
ordinary builds (see `Cargo.toml`). It is removed in B-1R8 with the recursive
engine. Default builds expose exactly one engine.

## B-1 contract-boundary rule (important)

The old recursive engine is **not** the golden oracle in the region where B-1
occurs. For call-frame depths inside the current cross-substrate-safe region,
`OLD == FUTURE NEW` is the right comparison. For depths where the WASM old
engine host-traps (see `docs/WASM_CALL_FRAME_LIMIT_DECISION.md` §2), the old
WASM trap is **not** expected behavior and must never be encoded as such.

The oracle therefore separates:

* **SEMANTIC DIFFERENTIAL CORPUS** — native-observed normalized behavior, where
  old and new engines must agree (`core`, `call-position`, `callback`,
  `compound`, `finally`, and the native call-frame boundary fixtures).
* **B-1 CONTRACT BOUNDARY CORPUS** — the 512-frame rule, whose expected result
  is the **language spec** plus a native contract-conforming result
  (`b1-boundary`): frame 512 is accepted, frame 513 is `E4011` at the attempted
  call site. B-1R5 must assert this on native, Node cold/warm, Chromium main
  thread, and the production Worker. A host trap is a defect, never an oracle.

## CI strategy

The decision (Step 25) is the minimum mechanism that prevents silent
divergence without adding a redundant job before a second engine exists:

* **Now (B-1R2):** the oracle already runs in normal CI: `ci.yml` runs
  `cargo test --locked --all-targets --all-features`, which covers both the
  default target and the `evaluator-oracle` build. No extra job is added while
  only one real engine exists.
* **From B-1R3:** add an explicit
  `cargo test --locked --test evaluator_oracle --features evaluator-oracle`
  invocation so the second-engine comparison is named and visible. Keep it
  non-default: default builds must contain one engine.

## Memory measurement plan (B-1R3, not now)

The design estimates potentially large continuation populations. B-1R3 must
measure, under `evaluator-oracle`:

* `size_of::<Cont>()`, `size_of::<UserFrame>()`, `size_of::<Ctrl>()`;
* adversarial heap usage near `max legal call depth (512)` ×
  `max legal per-frame expression nesting (256)`.

No resource limit is introduced now.

## Regenerating the golden manifest

```sh
cargo test --locked --test evaluator_oracle regenerate_golden -- --ignored
```

The golden file is a deliberate record of *current* behavior. When a future
evaluator intentionally changes an observable, the change must be reviewed and
the manifest updated in the same commit as the semantic change — never
regenerated casually to make a red test green.
