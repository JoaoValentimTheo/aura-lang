# Current Engineering Handoff

Authoritative for **the active task and the exact next action**. Current
repository state lives in `AGENT_STATE.md`; operating rules in `AGENTS.md`.
Keep this file short — it is read at the start of every session.

## Program

B-1 — WebAssembly call-frame implementation nonconformance. The released
runtime traps on the JavaScript engine stack below the normative 512-frame
language limit instead of reporting `E4011`. The chosen remediation is an
explicit-continuation (iterative) evaluator over the existing AST
(`docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`, Option E1).

## Current status

- B-1: **REMOTELY CLOSED** at `bb736fc` (inside pushed history; remote tip
  `089dffe` records the closure). The evaluator migration and production
  cutover are pushed; exact-SHA CI green (all 20 jobs) and Pages deployed.
  Released `0.2.1` runtime still contains the defect; the freshly built
  machine-backed runtime holds the language boundary.
- POST-B1 RUNTIME/WASM EDGE CLOSURE: **REMOTELY CLOSED** at `e576238`
  (pushed 2026-10-05; exact-SHA CI 20/20 green and Pages deployed after a
  GitHub Actions runner incident delayed the first attempts). The stdout
  capture-bound contract is pinned in `playground/runtime/tests/execute.rs`,
  `tests/host.rs`, and `playground/tests/node/b1_boundary.test.mjs`, and
  documented in `docs/playground.md` §2.
- PRE-0.3 FOUNDATION SUPER-TRANSACTION: **IN PROGRESS LOCALLY, NOT PUSHED**
  (push requires a new explicit human authorization). Iteration ledger and
  artifacts: `docs/engineering/RUNTIME_ARCHITECTURE.md`,
  `EXCEPTION_ARCHITECTURE.md`, `CAPABILITY_MODEL.md`,
  `EMBEDDED_PYTHON_ARCHITECTURE.md`, `CRITICAL_SYSTEM_PROFILE.md`,
  `MODULE_ARCHITECTURE.md`, `FUTURE_EXTENSION_BOUNDARIES.md`,
  `DIAGNOSTIC_TAXONOMY.md`, plus `tests/stdlib_coverage.rs` and the extended
  transport-limit, poison-recovery, and benchmark coverage.
- B-1R1 (design): COMPLETE.
- B-1R2 (differential oracle): COMPLETE and mutation-validated.
- B-1R3A-ARCH-1 (Arc AST sharing): RESOLVED.
- B-1R3A (machine skeleton + first executable subset): **COMPLETE AND REMOTELY
  CLOSED** at `c9ade0b` (exact-SHA CI green; website deployed).
- B-1R3B.1 (unary `-`/`not`/`~`): **COMPLETE AND PUSHED** at `1a12b84`.
- B-1R3B.2 (eager binary operators): **COMPLETE AND PUSHED** at `1a12b84`.
- B-1R3B.3 (short-circuit `and`/`or`): **COMPLETE AND PUSHED** at `2f34b9c` via
  the intermediate commits `58a1b16`/`31e313a`/`2f34b9c`.
- B-1R3B.4.1 (list/tuple construction): **COMPLETE AND PUSHED** at `efc66bd`
  (remote-closed).
- B-1R3B.4.2 (map construction): **COMPLETE AND REMOTELY CLOSED** at `5e70677`.
- B-1R3B.5 (range construction): **COMPLETE AND REMOTELY CLOSED** at `cf17689`.
- B-1R3B.6 (index / field reads): **COMPLETE AND REMOTELY CLOSED** at `cf17689`.
- B-1R3B.7 (f-strings): **COMPLETE AND REMOTELY CLOSED** at `52124a0` (pushed
  after a passing adversarial push gate; exact-SHA CI and Pages green; local =
  tracking = server at `52124a0`, ahead/behind 0/0).
- B-1R3B.8 (milestone completion audit + dependency graph): **COMPLETE AND
  REMOTELY CLOSED** at `9cb5e28` — `docs/engineering/B1R3B8_COMPLETION_AUDIT.md`.
- B-1R3C–B-1R3F (Call, Method, Construct, Lambda, Pipe, Assign, LetPattern,
  While, Loop, For, ListComp, MapComp, Match, Try): **COMPLETE AND PUSHED** in
  the B-1 closure range (tip `bb736fc`).
- Production cutover `62dd592`: **COMPLETE AND PUSHED** in the closure range.
  Every production entry point runs the machine; the residual recursive seams
  were closed at `11abad2` (free `aura::execute_with`, REPL `Item::Const`,
  Playground `execute`/`run_module_capture`).
- B-1R4 (full differential): DONE (whole-corpus `engines_agree`; differential
  228/228; syntax conformance 43/43).
- B-1R5 (substrate boundary): PARTIALLY DONE — fresh machine-backed wasm pinned
  by `playground/tests/node/b1_boundary.test.mjs` (63 checks: the frame/pattern
  boundary matrix, the pattern-depth calibration, and the post-B1 stdout
  capture-bound checks) and native CLI/REPL canaries. The Chromium main-thread
  and production-Worker boundary matrix needs the fresh artifact published to
  close (human-gated; frozen `playground/runtimes/**`).
- B-1R6 (red team): DONE — independent read-only review of the full range; all
  fifteen claims confirmed with no falsification; six minor findings, A/B/C/D/F
  fixed in `39caf6f` (field-receiver and tuple order differentials,
  `#[doc(hidden)]` on `Interp::run`, callback-confinement tripwire, wasm
  pattern-depth calibration pin); E did not reproduce.
- B-1R7 (full validation gate): DONE — including the exact-SHA CI run on
  `bb736fc` (all 20 jobs green) after the additive miri fix.
- B-1R8 (recursive-engine removal + oracle switch removal): NOT STARTED —
  human-gated; requires a release decision.

See `AGENT_STATE.md` for the exact SHAs and ahead/behind.

## Production vs experimental engine

- Production/default/CLI/REPL/Playground/library: **explicit-continuation
  machine** since the local cutover `62dd592`. The recursive evaluator is
  retained only as the differential reference (`Compilation::execute_recursive*`)
  and the rollback path.
- The machine handles **every** `Expr` (21) and `Stmt` (12) variant with no
  recursive fallback and no unsupported sentinel: literals, name lookup,
  expression statements, blocks, `let` shadowing and `let` patterns, assignment
  (simple/compound, name/index/field targets), `if`/`while`/`loop`/`for` (lazy
  ranges), `break`/`continue` boundaries, `return`/`throw`, unary/binary/
  short-circuit operators, list/tuple/map/range construction, index/field
  reads, f-strings, user/native/closure/method calls (including the resumable
  `map`/`filter`/`reduce` callback protocol), struct/enum construction,
  lambdas, pipes, list/map comprehensions, `match`, and `try`/`catch`/`finally`.
  The whole corpus is required to agree between engines
  (`tests/evaluator_oracle.rs::engines_agree`).

## Completed in B-1R3B.7 (remote-closed at `52124a0`)

- `src/run/iterative.rs` — added `Cont::FStrNext { parts, index, spec, out, env }`,
  `Machine::start_fstring`/`advance_fstring`, and the `Expr::FStr` arm in
  `start_expr` plus the `Cont::FStrNext` `resume` arm. Parts are consumed left
  to right, exactly once each, mirroring `Interp::eval_inner`'s `Expr::FStr`
  arm: each `FPart::Lit` is appended verbatim (raw text; `{{`/`}}` already
  resolved by the parser), each `FPart::Expr` is evaluated exactly once and
  appended with `v.display()` (no spec) or `Interp::format_value` (with a
  spec). The accumulator `String` lives in the continuation, so nesting does
  not grow the Rust stack. A control signal or diagnostic from an interpolation
  aborts the whole f-string and propagates unchanged, so later parts never run
  and no partial string is observable. No recursive AST evaluation and no
  fallback.
- Oracle: new `tests/oracle/r3b7_golden.tsv` (LF-pinned; 112 supported cases)
  and differential tests `r3b7_fstring_supported_subset_agrees`,
  `iterative_fstring_unsupported_fails_explicitly`, `r3b7_iterative_golden_matches`,
  plus `r3b7_golden_has_lf_pin` and 11 crate-internal f-string unit tests.
  Boundary movements: `fstring` removed from the R3A unsupported set
  (`r3a_golden.tsv` −1 row); `and_skip_fstring`/`or_skip_fstring` moved from the
  R3B.3 supported set into the R3B.7 set (`r3b3_golden.tsv` −2 rows);
  `nested_unsupported_element` (R3B.4.1), `fstring_value` (R3B.4.2), and
  `fstring_index` (R3B.6) moved out of their unsupported sets into supported
  R3B.7 cases. The main `golden.tsv` is byte-unchanged.
- Verified: empty/text-only/escaped-brace shapes; raw (undecoded) literal text;
  the stringification matrix (none/bool/int/float/string/list/tuple-sugar/map/
  range/`<fn>`); the format mini-language (`d b o x X f F e E %`, sign, width,
  fill/align, zero-pad, precision) and its `E3001` type and `E4013`
  precision/width bounds; Unicode; one-level nested f-strings; interpolation
  source order with first-error-wins; control-signal abort; `and`/`or` skipping
  of a whole f-string; composition with List/Map/Range/Index/Field/block/let/
  if; program-mode frame boundary; `E4999` no-fallback unsupported surface;
  host-stack/AST-depth safety and `expr_depth` restoration. A deliberate
  `display`→`debug_repr` stringification mutation was detected by seven R3B.7
  cases and reverted byte-exactly.
- Honest gaps: exactly-once is structural, not differentially falsifiable in the
  current subset (the only side-effecting interpolation constructs — calls,
  assignment, `print` — are still unsupported). Two-level f-string nesting is
  not expressible in the grammar (the innermost level would need the outer
  delimiter, which terminates it). Struct-instance/variant stringification is
  unreachable (`Expr::Construct` unsupported).

## Completed in B-1R3B.6 (remote-closed at `cf17689`)

- `src/run/iterative.rs` — added `Cont::IndexTarget { idx, env, span }`,
  `Cont::IndexApply { base, span }`, and `Cont::FieldReceiver { name, span }`,
  the `Expr::Index` / `Expr::Field` arms in `start_expr`, and the three `resume`
  arms. Index evaluates the target first and exactly once, then the index
  exactly once, then applies the lookup through `Interp::index_get`; a control
  signal from either operand is redelivered and the lookup never runs. Field
  evaluates the receiver once and resolves it exactly like `eval_inner`'s
  `Expr::Field` arm (struct field read with the method-not-a-value `E2003`
  guard, or zero-argument builtin method dispatch). Because struct construction
  is `Expr::Construct` (still unsupported), `Value::Instance` field reads are
  unreachable in this subset; the reachable field surface is the builtin method
  registry and its `E2003` unknown-member diagnostic. No recursive AST
  evaluation and no fallback.
- Oracle: new `tests/oracle/r3b6_golden.tsv` (LF-pinned; 55 index + 23 field =
  78 supported cases) and differential tests
  `r3b6_index_supported_subset_agrees`, `r3b6_field_supported_subset_agrees`,
  `iterative_index_field_unsupported_fails_explicitly`,
  `r3b6_iterative_golden_matches`, plus `r3b6_golden_has_lf_pin`. Boundary
  movement: `index` removed from the R3A unsupported set (now supported;
  `r3a_golden.tsv` −1 row). `field_access` stays in R3A unsupported: it is a
  struct literal via `Expr::Construct`. The main `golden.tsv` is byte-unchanged.
- Verified: target-before-index order and exactly-once; list/tuple/string/map
  lookup; negative-index normalization; `E4019` out-of-range; `E2003` missing
  map key and unknown member; `E3001` non-key-capable map key, unsupported
  base/index combination, and check-time map key-kind mismatch; receiver error
  wins over member resolution; control-signal propagation from target, index,
  and receiver; nested Index/Field and Range composition; program-mode frame
  boundary (Index and Field); unsupported→`E4999` with no fallback; and
  host-stack/AST-depth safety. A deliberate base/index-swap mutation was
  detected by three R3B.6 tests and reverted byte-exactly.
- Historical note (now resolved): struct-instance field reads and
  struct-instance indexing could not be differentially exercised until
  `Expr::Construct` landed in B-1R3C.3, and exactly-once was structural until
  calls/assignment/`print` landed in B-1R3C.1/B-1R3D.1; both gaps are closed by
  the R3C–R3F migration and the `print`-observable order cases in
  `tests/oracle/r3c1.rs`.

## Completed in B-1R3B.5 (remote-closed at `cf17689`)

- `src/run/iterative.rs` — added `Cont::RangeStart { end, env, span }` and
  `Cont::RangeEnd { start, span }`, the `Expr::Range` arm in `start_expr`, and
  the two `resume` arms. The start operand is evaluated first and exactly once;
  its completion retains the value (unvalidated) and schedules the end exactly
  once. **Both** operands complete before either bound is validated, and
  validation is start-first, exactly like `Interp::eval_inner`'s `Expr::Range`
  arm (`src/run/mod.rs:1583`): a non-int bound is `E3001` with the recursive
  message (`range start/end expects an int, found {type}`) at the range
  expression's span, and a start error beats a runtime end error, while an end
  signal/error preempts a runtime-invalid start (the end is evaluated first).
  A valid pair becomes the same `Value::Range(RangeVal { start, end })` that
  `range(a, b)` builds (half-open, end-exclusive). A control signal from either
  operand aborts construction and propagates unchanged; no partial Range
  escapes. No recursive AST evaluation and no fallback.
- Oracle: new `tests/oracle/r3b5_golden.tsv` (LF-pinned; 55 supported cases)
  and differential tests `r3b5_range_supported_subset_agrees`,
  `iterative_range_unsupported_fails_explicitly`, `r3b5_iterative_golden_matches`,
  plus `r3b5_golden_has_lf_pin`. Boundary movements: `range_literal` removed
  from the R3A unsupported set (`r3a_golden.tsv` −1 row); `and_required_range`
  removed from the R3B.3 unsupported set and replaced by six supported range
  short-circuit cases (`r3b3_golden.tsv` +5 rows); `range_element` moved from
  the R3B.4.1 unsupported set into the supported set plus a new program-mode
  `main_list_range` (`r3b4_golden.tsv` +2 rows); `range_value` moved from the
  R3B.4.2 unsupported set into the supported set plus a new program-mode
  `main_map_range` (`r3b42_golden.tsv` +2 rows). The main `golden.tsv` is
  byte-unchanged.
- Verified: start-before-end order and exactly-once scheduling; both-bounds
  evaluated then start-first validation; `E3001` runtime/check-time split;
  `E3001` at the exact range span; control propagation from either bound;
  `i64` extremes (`len()` saturates), descending/equal/empty, negative bounds;
  list/map/if/let/block/program composition; no fallback on unsupported bounds
  (`E4999`); AST-depth `E1015` below/above the limit; deep nesting host-stack
  safe. A deliberate start/end endpoint-swap mutation was detected by the R3B.5
  differential + golden tests and the composite R3B.4/R3B.4.2 tests, then
  reverted byte-exactly.
- Honest gap: exactly-once is structural but not differentially falsifiable in
  the current subset, because the only side-effecting bound constructs
  (calls/assignment/print) are still unsupported (recorded in
  `src/run/iterative.rs`).

## Completed in B-1R3B.4.2 (remote-closed at `5e70677`)

- `src/run/iterative.rs` — added `Cont::MapKeyNext { entries, index, out, env }`
  and `Cont::MapValueNext { key, entries, index, out, env }`, `Machine::start_map`,
  and the `Expr::Map` arm in `start_expr`. Entries run in source order; for each
  entry the key is evaluated and validated with `MapKey::from_value` **before**
  its value is scheduled, exactly like `Interp::eval_inner`'s `Expr::Map` arm
  (`src/run/mod.rs:1464`). An invalid runtime key is `E3001` with the exact
  recursive message at the key expression's span; the value never runs. A
  control signal or diagnostic from a key or value aborts the map and propagates
  unchanged, so later entries never run and no partial map is observable.
  Duplicate keys keep the last value (`BTreeMap::insert`), matching recursion.
  Empty `{:}` completes immediately. The accumulated `BTreeMap` lives in the
  continuations; no `Map`-sized or nested Rust recursion. No recursive AST
  evaluation and no fallback.
- Oracle: new `tests/oracle/r3b42_golden.tsv` (LF-pinned; 57 supported cases:
  49 value-mode and 8 program-mode; 57 golden rows) and differential tests
  `r3b42_map_supported_subset_agrees`,
  `iterative_map_unsupported_fails_explicitly`,
  `r3b42_iterative_golden_matches`. Boundary movements: `map_literal` removed
  from the R3A unsupported set (now supported); `or_required_map`/`and_lhs_map`/
  `or_lhs_map` moved from the R3B.3 unsupported set into the supported set;
  `map_element` moved from the R3B.4.1 unsupported set into the supported set
  (new rows in `r3b3_golden.tsv`/`r3b4_golden.tsv`; `r3a_golden.tsv` −1 row).
  The main `golden.tsv` is byte-unchanged.
- Push-blocker remediation: the R3B.4.1 `main_map_element` program-mode case
  (list containing a Map through a real frame boundary) was dropped without
  replacement when map elements moved to supported coverage; it is restored in
  the R3B.4.2 supported set (`r3b42-program/main_map_element`) with a guard test,
  and `tests/oracle/r3b42_golden.tsv` is now pinned `text eol=lf` in
  `.gitattributes` like every sibling golden.
- Verified: key-before-value and entry-by-entry order; exactly-once key/value;
  runtime invalid-key `E3001` at the key span with value suppression;
  first-error-wins; key/value control-signal abort with later-entry and
  later-unsupported skipping; last-wins duplicate keys; nested maps,
  Map-in-List, List-in-Map; `{:}`; unsupported keys/values → `E4999` with no
  fallback; AST-depth `E1015` below/above limit; deep nesting through a real
  frame boundary and mixed Map/List nesting host-stack safe; and a deliberate
  key/value-order mutation detected by the map oracle tests and reverted.

## Completed in B-1R3B.4.1 (pushed at `efc66bd`)

- `src/run/iterative.rs` — added `Cont::ListNext { items, index, out, env }`,
  `Machine::start_list`, the `Expr::List`/`Expr::Tuple` arm in `start_expr`, and
  the `Cont::ListNext` arm in `resume`. Elements are evaluated left to right,
  exactly once each, with the same environment; a non-`Val` completion from an
  element aborts the list and is redelivered unchanged (later elements are
  never scheduled); completed elements become `Value::list`. `Expr::Tuple`
  shares the list path because the recursive arm is byte-identical and
  `LANGUAGE_SPEC.md` §21 freezes `(a, b)` as list sugar. No recursive AST
  evaluation and no fallback.
- Oracle: new `tests/oracle/r3b4_golden.tsv` (LF-pinned; 37 supported cases)
  and differential tests `r3b4_list_supported_subset_agrees`,
  `iterative_list_unsupported_fails_explicitly`,
  `r3b4_iterative_golden_matches`. `list_literal`/`tuple_literal` moved out of
  the R3A unsupported set (and `r3a_golden.tsv` regenerated accordingly);
  `and`/`or` list-operand cases moved from the R3B.3 unsupported set into the
  supported set (6 new rows in `r3b3_golden.tsv`). The main `golden.tsv` is
  byte-unchanged.
- Verified: order/exactly-once (first error wins; later-error cases), element
  control-signal abort with unsupported-later-element skipping, exact
  diagnostics/spans, nested lists, AST-depth `E1015` below/above the limit,
  deep nesting through a real frame boundary host-stack safe, and a
  deliberate element-order mutation detected by four oracle tests and reverted
  byte-for-byte.

## Completed in B-1R3B.3 (pushed at `2f34b9c`)

- `src/run/iterative.rs` — added `Cont::ShortCircuitLeft { op, rhs, env }` and
  `Cont::ShortCircuitRight`; the machine evaluates the left operand exactly
  once, and only on completion decides from `Value::truthy()` whether the right
  operand is required (`and`: truthy left; `or`: falsy left). A skipped right
  operand is never scheduled (no stdout, mutation, diagnostic, or signal); a
  required right is evaluated exactly once. The result always mirrors the
  recursive arms: `Bool(false)` / `Bool(true)` on the deciding left, otherwise
  `Bool(rhs.truthy())`; control signals propagate via the `val!`-equivalent
  redelivery path. No recursive AST evaluation and no fallback.
- Oracle: new `tests/oracle/r3b3_golden.tsv` (LF-pinned) and differential tests
  `r3b3_short_circuit_supported_subset_agrees`,
  `iterative_short_circuit_unsupported_fails_explicitly`, and
  `r3b3_iterative_golden_matches`. `and_bool`, `or_bool`, and `eager_over_and`
  moved out of the R3B.2 unsupported set (now supported); the call-operand case
  stays. The main `golden.tsv` and all earlier iteratives goldens are unchanged.
- Verified: skipped-RHS error suppression (E4007/E4013/E3001/E4999 must not
  occur), required-RHS exact diagnostics, control-signal propagation, deep
  chains host-stack safe, `and`/`or` decision mutations rejected by the oracle.

## Completed in B-1R3B.2 (pushed at `1a12b84`)

- `src/run/iterative.rs` — added `Cont::BinaryLeft { op, rhs, env, span }` and
  `Cont::BinRight { op, lv, span }`; the machine evaluates the left operand,
  retains it, evaluates the right exactly once, then applies the operator only
  after both complete. Operator semantics are delegated to `Interp::binary`
  (the recursive engine's value-level helper, which never evaluates an AST
  node), so arithmetic, overflow (`E4013`), divide-by-zero (`E4007`), bitwise,
  shift-range, comparison, and equality behavior are identical by construction.
  `and`/`or` (B-1R3B.3) remain unsupported and never fall back.
- Oracle: new `tests/oracle/r3b2_golden.tsv` (LF-pinned) and differential tests
  `r3b2_binary_supported_subset_agrees`, `iterative_binary_unsupported_fails_explicitly`,
  `r3b2_iterative_golden_matches`. The R3A unsupported case `binary_add` was
  removed (now supported) and `r3a_golden.tsv` regenerated accordingly; the main
  `golden.tsv` is unchanged.
- Deliberate operand-swap mutation was detected by both R3B.2 tests and reverted.

## Completed in B-1R3B.1 (local)

- `src/run/iterative.rs` — added `Cont::UnaryApply { op, span }`; the operand is
  evaluated once and the operator is applied at its completion. `apply_unary`
  mirrors `Interp::eval`'s `Expr::Unary` arm exactly (checked `int` negation with
  `E4013`, `not` over `truthy()`, `~` on `int` only, both type errors `E3001`
  with the same messages). Operand control signals (`return`/`throw`/…) propagate
  without applying the operator; a deep unary chain is stack-safe.
- Oracle: `tests/oracle/r3b.rs` + `tests/oracle/r3b_golden.tsv` (LF-pinned) and
  differential tests `r3b_unary_supported_subset_agrees`,
  `iterative_unary_unsupported_fails_explicitly`, `r3b_iterative_golden_matches`.

## Completed in B-1R3A (remotely closed at `c9ade0b`)

- `src/run/iterative.rs` — explicit `Machine`/`Ctrl`/`Cont`/`UserFrame`/
  `FrameBoundary`; one `loop` over `Ctrl` with a `Vec<Cont>` stack; 512/513
  frame accounting and the `E1015` AST-depth guard, unit-tested.
- Oracle extended with an identified R3A subset (`tests/oracle/r3a.rs`),
  strict supported-subset equality, an explicit no-fallback test, and a
  full-field iterative golden (`tests/oracle/r3a_golden.tsv`, LF-pinned).

## Current invariants — do not break

- AST sharing uses `std::sync::Arc` (not `Rc`); runtime `Env`/`Value`/`Closure`
  deliberately stay `Rc`. Do not mechanically convert runtime `Rc` to `Arc`.
- The main oracle golden `tests/oracle/golden.tsv` is byte-exact and must not
  change unless an intended observable semantic change is being made.
- Frozen release runtimes (`0.0.2`, `0.2.0`, `0.2.1`) are immutable, byte for
  byte.
- No `unsafe`; no manual `Send`/`Sync`.
- B-1 is REMOTELY CLOSED: the full migration and the 512-frame boundary hold
  on every substrate (fresh wasm proven; frozen `0.2.1` remains the historical
  artifact). Do not reopen B-1 without a new human gate.

## Do not touch

- `playground/runtimes/**` (frozen artifacts).
- The `v0.2.1` tag or any release.
- Untracked/pre-existing dirty state (see `AGENT_STATE.md` protected local
  state), including `.kilo/**`.
- The website, unless a current public statement is demonstrably false.

## Relevant authority

- Design: `docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`
- Oracle: `docs/engineering/B1R2_DIFFERENTIAL_ORACLE.md`
- Arc decision: `docs/B1R3A_AST_SHARING_DECISION.md`
- B-1 decision package: `docs/WASM_CALL_FRAME_LIMIT_DECISION.md`
- Normative semantics: `docs/LANGUAGE_SPEC.md`

## Required tests for B-1R work

- `cargo test --locked --features evaluator-oracle --test evaluator_oracle`
- `cargo test --locked --features evaluator-oracle --lib iterative`
- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`

At milestone closure, run the full validation floor in `AGENTS.md`.

## Phase ledger — B-1R3B (values and operators)

R3B is **COMPLETE**: every planned microphase (R3B.1—R3B.7) is implemented,
differentially oracled, and remotely closed; R3B.8 audited the milestone. The
B-1R3C–B-1R3F evaluator migration and the production cutover `62dd592` are
implemented, oracle-covered, validated, **pushed, and remote-closed** as part
of the B-1 closure range (tip `bb736fc`).

- R3B.1 unary operators — **COMPLETE AND PUSHED**
- R3B.2 binary operators — **COMPLETE AND PUSHED**
- R3B.3 short-circuit / evaluation order — **COMPLETE AND PUSHED**
- R3B.4.1 list / tuple construction — **COMPLETE AND PUSHED**
- R3B.4.2 map construction — **COMPLETE AND REMOTELY CLOSED**
- R3B.4.3 (reserved: map-key admissibility / nesting if the code shows a
  distinct boundary)
- R3B.5 range — **COMPLETE AND REMOTELY CLOSED**
- R3B.6 index / field reads — **COMPLETE AND REMOTELY CLOSED**
- R3B.7 f-strings — **COMPLETE AND REMOTELY CLOSED**
- R3B.8 milestone completion audit — **COMPLETE (audit only)**; see
  `docs/engineering/B1R3B8_COMPLETION_AUDIT.md`

Use multi-reviewer analysis mainly at milestone closure, not after each
microphase.

## PRE-0.3 iteration ledger (2026-10-05)

Compact record of the Pre-0.3 Foundation super-transaction. Every iteration
was run under the mandatory Jev PRE/POST loop; evidence and dispositions only
(no chain-of-thought).

| # | Workstream | Question / action | Evidence before | Jev PRE | Verification | Jev POST | Commit |
|---|---|---|---|---|---|---|---|
| I1 | A/J/K/H/P/O reconnaissance | Inventory repo order, module identity, stdlib, limits, unsafe/panic, supply chain | two read-only subagent inventories + lead verification (falsified the `[0]`-index concern: test-only) | proceed 0.99 | − (read-only) | accept 0.99 | — |
| I2 | P/Host | Poisoned locks silently drop bytes? | host.rs:568-578 / lib.rs:195-199 confirm silent drop | proceed 0.92 (hidden-corruption p=0.07) | host 17 tests incl. new poison regression; playground execute 19 | commit 0.99 (risk 0.15) | `5edab88` |
| I3 | H | Triplicated `MAX_AST_DEPTH`, magic range/sleep limits | parse:163, run:34, check:152; run:1223 local const; host:370 magic | proceed 0.97 | boundaries 34; oracle 48; fmt/clippy clean | commit 1.0 | `2fd1433` |
| I4 | O | Unused `serde`/`arbitrary` deps | `cargo tree -i serde` = self-only; no derives | proceed 0.92 | all feature configs + 51 suites + fuzz nightly + locks | commit 0.94 | `3186340` |
| I5 | A/25 | Stale "production recursive" claims; divergent errors reference | 7 docs wrong; website duplicate | proceed 0.91 (spec-rewording risk flagged 0.51, mitigated by wording) | syntax_docs 3 (new sync test), grammar 6, website build/tests | commit 0.98 | `c1ffb18` |
| I6 | G | No evaluator performance baseline | bench.rs had shape guards only | proceed 1.0 | 8 stages measured linear; guards 14 pass; fmt/clippy | commit 0.99 | `9151df1` |
| I7 | C | Exception family/custom-exception foundation | throw/catch reality; TryResult snapshots | proceed 0.99 (preemption risk 0.4 → design-only markers strengthened) | doc cross-checked | commit 0.65→ (low, revised wording) | `34c5878` |
| I8 | D/E/F | Capability model, embedded-Python boundary, critical profile | Host trait, pyo3 inventory, resource table | proceed 0.96 | all claims cross-checked to source | accept 0.95 | `8ac9b93` |
| I9 | K/J/L/M/N | Stdlib coverage gaps; module map; AI/Dart/quantum boundaries | inventory found untested surfaces; cycle-coverage suspicion falsified | proceed 0.92 | stdlib_coverage 11 (2 expectation fixes to match documented surface) | commit 0.94 | `a1a5aa4` |
| I10 | WASM | Remaining transport limits untested | args/project/key/name limits had no direct tests | commit scope approved | execute 22, virtual_project 24 (1 expectation corrected to E4020) | commit 0.99 | `eb1e339` |
| I11 | I | Diagnostic taxonomy; E4099/E5003 undocumented | 46 constants / 45 codes; doc gap | proceed 0.99 | sync test green; grammar 6 | (folded) | `d4becc3` |
| I12 | B | Runtime architecture record; state reconciliation | entry points verified with file:line | commit scope approved | production_routing exists; agent-state preflight OK | commit 0.96 | `5b74ebb` |
| I13 | Residuals/§21 | Dispose 5 residuals; deliberate divergence | fresh-wasm pattern + f-string sweeps: no traps | commit scope approved | 3 mutations detected and reverted byte-exact (sha256); wasm 63 checks | commit 0.93 | `0e34fed`, `ef920a8` |
| I14 | Validation/review | Full matrix + independent adversarial review | — | — | 52+52 suites, clippy×2, MSRV, fuzz, wasm 63, node, website, python, miri 117; review: claim not falsified, 7 minor doc corrections + 2 test-precision nits + 1 mirror-drift surface | accept 0.98 (corrections) | `1d38b49` |

Jev totals: PRE consultations: 13; POST classifications: 12; disagreements:
1 (I7: Jev 0.65 commit vs deterministic evidence that the doc was
design-only and non-normative — proceeded, strengthened the non-normative
markers first); Jev-driven extra investigations: 3 (I2 hidden-corruption
question, I5 spec-rewording caution, I7 preemption caution).

## PRE-0.3 adversarial re-audit + human syntax gate (2026-10-06)

Bootstrap (verified, not assumed): branch `rewrite/v3-rust`; local HEAD
`1102b23` (then `150befe` after the repair below); remote
`e576238f7adb52f6ab6d18431602e3fbc2bd3636`; ahead 15 / behind 0; range
`e576238..1102b23` intact; staged none; protected `.kilo/**` churn matches the
documented state; `.codex/**` not tracked; root `s` absent; frozen runtime
hashes byte-identical; tag `v0.2.1` = `3f5f8702`; declared version `0.2.1`.

| # | Workstream | Question / action | Evidence before | Jev PRE | Verification | Jev POST | Commit |
|---|---|---|---|---|---|---|---|
| A1 | Bootstrap | Is the checkpoint true in Git? | local/remote SHAs, reflog linear, frozen hashes, tags | proceed 0.85 (divergence 0.21, defect 0.26) | all 18 audited claims checked; 0 production defects | accept | — |
| A2 | F1 in-range defect | Merged doc-comment delimiter `tests/host.rs:383` | introduced by `2fd1433`; single occurrence | proceed 0.97 | fmt clean; host 19/19 | accept | `150befe` |
| A3 | F2 bare-config hygiene | 13 tests in 8 suites fail bare `--no-default-features` | every failing file unchanged since remote `e576238`; canonical configs 52/52 | future_work 0.99 | reproduced (exit 101); recorded as TD-20 | accept | (doc only, below) |
| A4 | Routing mutation | Does the tripwire catch a rerouted seam? | `src/lib.rs:443` temporarily → `interp.run` | reuse A1 gate | `production_routing` FAILED (2≠3); reverted, sha256 byte-identical | accept | (reverted) |
| A5 | Decision package | Verify candidate claims by execution | 21 probes on the built CLI | A/B `needs_more` 0.60, span 0.65 → revised claims | A and B already constructible; nominal cross-module identity verified; C needs new grammar; catch-selection/raise-site-span are the real gaps | accept (revised) | `docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md` |

Validation executed in this pass: all-features 52/52 suites; canonical
`--no-default-features --features cli,repl,json,regex,time` 52/52; evaluator
oracle 228/228 differential; fresh-WASM boundary 63/63; syntax conformance
43/43 zero imports; browser 66; worker 12; multi-file 42; cache 7; `fmt`
clean; clippy `--all-features` and bare both `-D warnings` clean; MSRV
`+1.83.0 check --all-features` green; nightly fuzz `check --all-targets`
green; Miri 117/117; four fuzz targets ~890k runs clean; website build + tests
green.

**Outcome A stands: no public exception syntax was chosen or implemented;
E1–E6 remain the human gate.** The verified, refined decision package is
`docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md`; it supersedes the
prior session's untracked `plans/EXCEPTION_SYNTAX_DECISION_PACKAGE.md`
(sha256 `647e0e24…`; its content is preserved as a strict subset). TD-20
records the pre-existing bare-config test-hygiene gap (not repaired: outside
the audited range and every affected configuration is unsupported/
documented-against).

Independent adversarial review (fresh read-only session, 2026-10-05):
completion claim **not falsified** across 13 attack areas (frozen artifacts,
protected state, commit coherence, uncommitted state, no-push, doc claims,
resource contracts, poison soundness, semantic safety, test quality, oracle
golden, dual authority, validation honesty). Findings: 7 minor
documentation-accuracy corrections, all applied in this iteration (TryResult
variant names, FrameBoundary location, entry-point citation, taxonomy
sampling wording, PERFORMANCE min-vs-average wording, stale "53" counts,
state-ledger pointer). One pre-existing non-blocking note retained: the
Playground `project.js` mirrors four transport limits without a mechanical
sync test (documented as non-authoritative; runtime refusal is the decision).



| # | Residual | Disposition |
|---|---|---|
| 1 | Real browser/Worker validation of a fresh machine-backed runtime | **Publication-gated** (unchanged). Frozen `playground/runtimes/**` is immutable; the fresh-wasm Node boundary is pinned at 63 checks; the Chromium/Worker matrix closes only when a new runtime is published (human-gated). |
| 2 | `bind_pattern`/`match_pattern` bounded host recursion | **Verified safe and pinned.** Fresh-wasm calibration: deep pattern @700 binds without trap; @766 is structured `E1015`. AST-bounded (`MAX_AST_DEPTH` + substrate parser budget). Recorded in `docs/engineering/RUNTIME_ARCHITECTURE.md` §3. |
| 3 | f-string sub-parser budget composition | **Investigated empirically; no trap found.** Additive frontier sweep (outer/inner grouping, two-level nested f-strings) on fresh wasm is ok-or-`E1015`; grammar bounds nesting depth. Now pinned by 10 fresh-wasm checks in `playground/tests/node/b1_boundary.test.mjs` so a future parser-frame change cannot reintroduce a trap silently. |
| 4 | Poisoned-mutex silent ignore in `BrowserHost` | **FIXED** in `5edab88`: all output/result locks recover via `PoisonError::into_inner`; regression test deliberately poisons the buffer and asserts bytes survive. |
| 5 | Recursive-engine removal | **Human-gated** (B-1R8). The recursive evaluator remains the differential reference (`execution_recursive*`, oracle `engines_agree`); no production entry reaches it (`tests/production_routing.rs`). |

## Exact next action

1. B-1 and the post-B1 runtime/WASM edge closure are **remotely closed**. The
   Pre-0.3 Foundation super-transaction and the 2026-10-06 adversarial re-audit
   are **complete locally and not pushed** (`150befe`). The exception feature is
   **stopped at the human syntax gate**: the human must resolve E1–E6 (an ADR
   and, per `docs/rfcs/README.md`, an accepted RFC for the chosen grammar) in
   `docs/engineering/EXCEPTION_ARCHITECTURE.md` §8 / the decision package
   `docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md` §6. Do not implement
   public exception syntax before that decision.
2. Nothing in this transaction is pushed. Pushing the local range (now
   `e576238..150befe` plus the documentation commits) requires explicit human
   authorization.
3. B-1R8 (remove the recursive engine and the oracle switch) and any runtime
   publication remain separately human-gated.
4. Keep frozen runtimes, `v0.2.1`, and `.kilo/**` untouched.
5. Publication-gated validation debt: the Chromium/Worker boundary against a
   machine-backed runtime awaits a published runtime (release-gated); the
   fresh-wasm boundary is already pinned in Node (63 checks).
6. Pre-0.3 decisions awaiting the human: exception syntax/catching (E1–E6 in
   `docs/engineering/EXCEPTION_ARCHITECTURE.md` §8) and embedded-CPython
   packaging (P1–P5 in `docs/engineering/EMBEDDED_PYTHON_ARCHITECTURE.md` §9).
   Bare-config test-feature gating is tracked as TD-20 (low, v1.1).

## Stop conditions

Stop and report instead of improvising if:

- Git reality contradicts `AGENT_STATE.md`;
- production behavior would change;
- the language contract and current behavior disagree in a way that needs a
  semantic decision (produce a decision package instead);
- a frozen artifact would be touched;
- a genuine oracle blind spot is found that cannot be closed within the slice.
