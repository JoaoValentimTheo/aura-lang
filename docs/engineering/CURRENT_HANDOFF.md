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

- B-1: **OPEN** — full iterative evaluator not implemented; production is
  still recursive.
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
- B-1R3B.8 (milestone completion audit + dependency graph): **COMPLETE (audit
  only)** — `docs/engineering/B1R3B8_COMPLETION_AUDIT.md`.
- R3C…R3G: NOT STARTED (no implementation may begin without explicit human
  authorization).

See `AGENT_STATE.md` for the exact SHAs and ahead/behind.

## Production vs experimental engine

- Production/default/production path: **recursive** engine, unchanged.
- Iterative engine: exists only under the non-default `evaluator-oracle`
  Cargo feature; not reachable from the CLI, REPL, Playground, or library
  production path. It currently supports literals, name lookup, expression
  statements, blocks, `let` shadowing, `if`/`else`, unary `-`/`not`/`~`, the
  eager binary operators (`+ - * / % ^ == != < <= > >= & | << >>`),
  short-circuit `and`/`or` (the skipped operand is never evaluated), list/tuple
  construction (left-to-right, exactly once per element), map construction
  (per entry key then value in source order, exactly once), range
  construction (`a..b`), index / field reads (`base[index]`, `recv.name`), and
  f-strings (`f"..."`, literal/interpolation parts left to right); every other
  construct returns the deterministic `E4999` sentinel and never falls back to
  recursion.

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
- Honest gap: struct-instance field reads and struct-instance indexing cannot be
  differentially exercised yet, because struct construction (`Expr::Construct`)
  remains unsupported (R3C/R3G). Exactly-once is structural, not
  differentially falsifiable, since the only side-effecting operand constructs
  (calls/assignment/print) are still unsupported (recorded in
  `src/run/iterative.rs`).

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
- B-1 must stay OPEN until the full migration and the 512-frame boundary hold
  on every substrate.

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
next implementation phase (R3C) is **NOT STARTED** and requires explicit human
authorization. Each future microphase remains implement → targeted tests →
oracle → checkpoint; do not start one before human review of the previous one.

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

## Exact next action

1. R3B is COMPLETE and remotely closed; R3B.8 (completion audit) is COMPLETE
   (audit only, no implementation) and checkpointed as one documentation-only
   commit on top of `52124a0`.
2. Next authorized action: a fresh push gate over the local
   `52124a0..HEAD` range, then exact remote closure of the B-1R3B.8
   documentation checkpoint (push requires separate human authorization).
3. **Do not start R3C** before that remote closure and its own explicit human
   authorization. The recommended first implementation microphase is R3C.1
   (`Call`) per `docs/engineering/B1R3B8_COMPLETION_AUDIT.md`.
4. Keep `tests/oracle/golden.tsv` byte-unchanged and production on the
   recursive engine.

## Stop conditions

Stop and report instead of improvising if:

- Git reality contradicts `AGENT_STATE.md`;
- production behavior would change;
- the language contract and current behavior disagree in a way that needs a
  semantic decision (produce a decision package instead);
- a frozen artifact would be touched;
- a genuine oracle blind spot is found that cannot be closed within the slice.
