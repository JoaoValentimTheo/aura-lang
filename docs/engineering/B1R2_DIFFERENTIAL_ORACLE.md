# B-1R2 — differential evaluator oracle

**Status:** COMPLETE — differential semantic oracle built and mutation-validated.
No runtime behavior changed; no evaluator implemented. B-1 stays **OPEN**.

**Authority:** `docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md` §24–§26.
**Instrument:** `tests/evaluator_oracle.rs`, `tests/oracle/mod.rs`,
`tests/oracle/cases.rs`, golden manifest `tests/oracle/golden.tsv`,
`tests/oracle/README.md`.

---

## 1. Purpose

B-1R2 is the **measuring instrument**, not the treatment. It proves that the
oracle can *detect semantic divergence* before B-1R3A begins implementing the
explicit continuation machine. A passing self-comparison is necessary but not
sufficient; §5 records the deliberate mutation experiments that prove the
oracle can fail.

## 2. Architecture

* **Engine adapter.** `harness::Engine` is the single seam. `Engine::recursive`
  is the current tree-walker; `Engine::iterative` selects the future machine
  under the non-default `evaluator-oracle` feature. On a default build both
  sides run the recursive evaluator; that only proves the harness.
* **Isolation.** Each `observe` call reconstructs the run from source: fresh
  front end, fresh `Compilation`, fresh execution thread, fresh `Interp`, fresh
  host, fresh stdout buffer. No `Interp`/`Env`/`Host`/runtime state is shared.
  `execution_is_isolated` proves it.
* **Normalization.** Observable =
  `(stdout exact bytes, optional typed final value, completion)`, where
  completion is `Ok | Compile(NormDiag) | Runtime(NormDiag)` and `NormDiag` is
  `(code, message, source display name, byte span, line, column)`. Raw `Rc`
  addresses and `SourceId` scopes never enter comparison.
* **Value comparison model (Step 6).** The final value is `NormValue { ty,
  repr }` — a **type-tagged** debug representation. This closes the strongest
  normalization blind spot: `1` (int), `1.0` (float), and `"1"` (string) all
  render as `1` at top level under `display()`, so an untagged observable would
  miss a type change. `none`, `bool`, `int`, `float`, `string`, `list`, `map`,
  `struct`, `enum`, `fn`, and `range` are all distinguishable, and container
  element types are visible through the recursive debug representation. Cyclic
  values cannot hang comparison because only the bounded `debug_repr` string is
  compared, never the `Value` graph.
* **Golden manifest.** `tests/oracle/golden.tsv` pins current observed behavior
  for every case, including the B-1 boundary corpus and the deliberately
  preserved compound-assignment double evaluation.

## 3. Corpus

98 deterministic cases. Groups: `core`, `diag`, `provenance`, `call-position`,
`callback`, `compound`, `finally`, `b1-boundary`, `module`, `value`.

* **call-position matrix (16):** binary left/right, function argument,
  short-circuit (and/or), index base+key, method receiver+arg, list items, map
  key+value, f-string interpolation, comprehension value/filter, match guard,
  return expression, assignment RHS, range bounds, construct args.
* **callback frame coverage:** `[1].map(to_string)` (native, no frame) vs
  `[1].map((x) -> x)` (closure, one frame); closure recursion at the 512
  boundary is `E4011` (`closure_callback_counts_frame_at_boundary`); the native
  callback at the same depth is accepted
  (`native_callback_adds_no_frame_at_boundary`); list-snapshot behavior pinned.
* **compound assignment:** `a[idx()] += 10` prints `idx` twice
  (`compound/index_target_double_eval`). Labeled
  **CURRENT OBSERVABLE BEHAVIOR PRESERVED FOR EVALUATOR MIGRATION**; not
  classified as desirable and not fixed here.
* **try/catch/finally matrix:** pending `{Val, Return, Throw, Break, Continue}`
  × finally `{Val, Return, Throw}` plus fatal-error-in-finally, nested try in
  finally, and call-in-finally. (The Aura grammar requires `catch` before
  `finally`; the structurally-legal combinations are captured.)
* **module attribution:** runtime error in an external child, three-source
  chain error in the leaf, return-to-caller, and module init ordering.
* **provenance:** byte-identical sources `a.aura` and `b.aura` produce the same
  `E4007` with distinct source names — the oracle keeps them separate.
* **B-1 boundary:** the existing `tests/corpus/call-frames/*_510/_511` fixtures
  are reused (not duplicated).

## 4. Baseline self-comparison (Step 17)

`oracle_corpus_matches_golden` and `engines_agree`: 100% agreement over all 98
cases on the current engine, and recursive==recursive. `golden_manifest_is_reproducible`
proves the committed manifest regenerates byte-for-byte.

## 5. Mutation sensitivity matrix (Step 18/19)

Each mutation was a temporary edit to `src/run/mod.rs`, applied alone, run
against `oracle_corpus_matches_golden`, and fully reverted. `src/run/mod.rs` was
restored to SHA-256
`812287ed25d7817f04cb32407293205d4c66287eb162867f4f745abd546caf32` after every
experiment (verified).

| # | Mutation | Expected to fail | Did fail | Observable that detected it | Reverted |
|---|---|---|---|---|---|
| M1 | Reverse function-argument evaluation order | YES | **YES** | stdout order (`b\na\n3` vs `a\nb\n3`) | YES |
| M2 | Evaluate RHS of `false and rhs()` | YES | **YES** | stdout side effect (`t\n` appears) | YES |
| M3 | Add `+1` to integer addition | YES | **YES** | stdout value (`8` vs `7`, `4` vs `3`, …) | YES |
| M4 | `E4011` code changed to `E4013` | YES | **YES** | diagnostic code (4013 vs 4011) | YES |
| M5 | Drop `current_source` frame attribution | YES | **YES** | diagnostic source name (`main.aura` vs `child.aura`/`leaf.aura`) | YES |
| M6 | Division-by-zero span set to `Span::default()` | YES | **YES** | span/line/column (0:1 vs 20:21) | YES |
| M7 | Captured mutable assignment writes `i+1` | YES | **YES** | stdout value (`5` vs `3`, `15` vs `7`) | YES |
| M8 | `finally` result no longer overrides pending outcome | YES | **YES** | stdout value and completion (`1` vs `7`; `ok` vs `E4026`) | YES |
| M9 | Method receiver replaced with `none` | YES | **YES** | runtime diagnostic (`none has no method`, `E2003`) | YES |
| M10 | Module initialization order reversed | YES | **YES** | stdout order (`child-init` before `entry-init`) | YES |
| M11 | `Value::Str` reported as type `int` (type erasure) | YES | **YES** | type-tagged value observable (`string` vs `int`) | YES |

No meaningful mutation escaped. M11 is the regression guard for the type-tag
blind spot found in review; it mutates `src/run/value.rs` instead of
`src/run/mod.rs`. The one attempted mutation that did *not* diverge — changing a
lambda's captured env from `env` to `env.child()` — was behavior-neutral because
`child()` still resolves through the parent chain; it is recorded as *not a
semantic mutation*, not as an oracle blind spot, and a genuine captured-mutation
mutation (M7) was used instead.

**Reproducible.** The matrix is no longer prose only: `tests/oracle/mutation_experiments.sh`
applies each mutation, asserts the oracle fails, restores the file, and verifies
the SHA-256 is unchanged. It exits non-zero if any mutation escapes or any file
is left modified. Run it manually (`sh tests/oracle/mutation_experiments.sh`);
it is deliberately not a `cargo test` because it edits a tracked file.

## 6. Reentrancy classification (Step 15)

**Classification: UNSPECIFIED (out-of-scope for B-1).**

Evidence: `LanguageSpec` does not define a host/native re-entry contract.
`Interp::native` is a public embedder API whose callback type is
`Fn(&mut Interp, Vec<Value>, Span) -> Result<Value>`; an embedder can call
`Interp::call`/`call_value` from inside it, which nests a Rust call inside the
interpreter frame that invoked the native. `reentrancy_classification`
demonstrates this synthetically (a registered native calls `call_value` on a
closure it is handed and returns `41`). The design (`ITERATIVE_EVALUATOR_DESIGN.md`
§13) already records this pattern as unsupported and out of scope: such a native
cannot receive the machine loop, so it starts a nested machine run exactly as it
nests a nested Rust call today. **Architecture risk recorded:** the future
machine must not assume it is the only execution in progress if it ever wants to
support this pattern; B-1R2 makes no language decision.

## 7. Closure-source lifetime probe (Step 14)

`closure_source_lifetime_probe` creates 2000 closures in a loop, then forces a
runtime diagnostic from a lambda declared in a *different* source
(`child.aura`). Result: repeated creation/destruction (a) did not make
observation nondeterministic, and (b) did not misattribute the lambda's source
— the diagnostic still resolves to `child.aura`.

The pre-existing risk is real but not reproduced as an independent defect here:
`closure_sources` is keyed by raw `Rc` address and never pruned, so address
reuse *could* misattribute in a sufficiently long session with interleaved
allocations. That is a **separate, unassigned finding**, not folded into B-1 and
not fixed in B-1R2 (the mission forbids fixing it here). The oracle probe
preserves current behavior.

## 8. B-1 contract-boundary rule (Step 20)

The old recursive engine is **not** the golden oracle where B-1 occurs.

* **SEMANTIC DIFFERENTIAL CORPUS** — native-observed normalized behavior, where
  old and future engines must agree (`core`, `call-position`, `callback`,
  `compound`, `finally`, and the native call-frame fixtures).
* **B-1 CONTRACT BOUNDARY CORPUS** — the 512-frame rule, whose expected result is
  the **language spec plus a native contract-conforming result**
  (`b1-boundary`): frame 512 accepted, frame 513 `E4011` at the attempted call
  site. It is asserted through native behavior today; B-1R5 must assert it on
  Node cold/warm, Chromium main thread, and the production Worker.

A host trap (`E4999`/`RangeError`) is a defect and is **never** encoded as
expected oracle behavior.

## 9. Performance (Step 24)

98 cases. Suite wall time after build: ~0.18 s of test execution, < 0.35 s
including cargo overhead (median of 3). The dual-engine run (feature build) is
expected to be ~2× the corpus observation cost; still sub-second. Correctness
first; no optimization attempted.

## 10. CI strategy (Step 25)

* **Now (B-1R2):** the oracle runs automatically in the normal CI test job.
  `ci.yml` line 70 runs `cargo test --locked --all-targets --all-features`, so
  `tests/evaluator_oracle.rs` runs both as a default target and again with
  `evaluator-oracle` enabled (exercising the two-engine `engines_agree` path and
  `oracle_feature_gate_is_documented`). No new job is needed while there is only
  one real engine; adding a redundant one now would be the expensive,
  no-second-engine job the design says not to add.
* **From B-1R3:** add an explicit dedicated invocation
  (`cargo test --locked --test evaluator_oracle --features evaluator-oracle`) in
  the CI job so the second-engine comparison is named, visible, and cannot be
  dropped by a later `--all-features` regression. It must stay non-default:
  default builds contain exactly one engine (`feature_is_not_default`,
  `oracle_feature_gate_is_documented`).

## 11. Memory measurement plan (Step 16)

B-1R3 (not now) must measure, under `evaluator-oracle`:
`size_of::<Cont>()`, `size_of::<UserFrame>()`, `size_of::<Ctrl>()`, and
adversarial heap usage near `max legal call depth (512)` × `max legal per-frame
expression nesting (256)`. No resource limit is introduced in B-1R2.

## 11a. Independent review findings and corrections (Step 26)

A fresh read-only reviewer attempted to falsify the oracle against 20 questions.
Findings and responses (each reproduced before changing code):

| # | Finding | Response |
|---|---|---|
| 1 | Isolation real, but the proof test was weak | Strengthened `execution_is_isolated`: interleaves two different programs, asserts no stdout leakage, and detects aliased observable bytes. |
| 2 | No stderr / host-failure modeling | **Accepted limitation.** `Host` exposes only `write_stdout` (`src/host.rs:83`); Aura has no stderr channel. A future engine that writes diagnostics outside `Host` would be outside the language surface. Recorded, not modeled. |
| 12 | Value normalization hid type distinctions (`1`/`1.0`/`"1"`) | **Fixed.** `NormValue { ty, repr }` is type-tagged; added `value/float_expression`, `value/string_number_expression`, etc.; M11 mutation proves the tag catches type erasure. |
| 15 | NaN/±inf/signed-zero untested | **Fixed.** Added `float/nan_inf_signed_zero` and `float/integral_float_display`. |
| 17 | `b1-boundary` group name misleading (510/511 vs 512/513) | Clarified in §3/§8: `count(510)` *is* frame 512 accepted and `count(511)` *is* frame 513 rejected per the decision package. Node/Chromium/Worker assertion is explicitly deferred to B-1R5. |
| 18 | Isolation proof weak | Same as #1. |
| 20 | Engine seam inert; `observe` ignored its engine arg; no explicit feature CI | **Fixed.** `observe` now threads `engine` to the single `run_compiled` seam; the `evaluator-oracle` branch is the named B-1R3 wiring point. No fake machine was added. CI note corrected (the feature already runs under `--all-features`). |
| — | Mutation matrix was prose, not reproducible | **Fixed.** `tests/oracle/mutation_experiments.sh` reproduces all mutations and verifies restoration. |
| — | Golden manifest is tautological by construction | **Mitigated.** Added `hand_verified_anchors`, a small hand-checked set (values, evaluation order, short circuit, captured mutation, diagnostic code/source/span) independent of the generated file. The README records that regeneration must accompany a reviewed semantic change, never to hide one. |

The reviewer's falsified claims were #12 and #20; both are fixed. The remaining
partial items (#2 stderr, #17 native-only) are honestly documented limitations,
not silent gaps.

## 12. Non-goals (Step 23)

No `ExplicitEvaluator`, `UserFrame`, `Cont`, or `Ctrl` placeholder was added.
The only production file touched is `Cargo.toml` (a non-default feature flag,
no code). `src/run/mod.rs` is byte-identical to the pre-phase tree.
