# B-1R3B.8 — Completion Audit and Remaining B-1 Dependency Graph

Status: **COMPLETE (AUDIT)**. This document is planning/architecture authority for
the remaining B-1 iterative-evaluator surface. It contains **no implementation
authorization**: R3C and every later phase remain NOT STARTED, and production
remains on the recursive evaluator.

Authority basis: repository source at `52124a0` (R3B.7 remotely closed),
`docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`, `docs/LANGUAGE_SPEC.md`, and an
independent read-only verification pass (see §Independent review).

## 1. Mechanically reconstructed coverage matrix

`Expr` has **21** variants and `Stmt` has **12** (src/ast/mod.rs).

### Expr coverage

| Variant | Iterative status | Evidence |
|---|---|---|
| `Lit` | supported | `start_expr` |
| `Name` | supported | `start_expr` |
| `FStr` | supported (R3B.7) | `start_expr` |
| `Block` | supported | `start_expr` |
| `If` | supported | `start_expr` |
| `Unary` | supported (R3B.1) | `start_expr` |
| `List` | supported (R3B.4.1) | `start_expr` |
| `Tuple` | supported (R3B.4.1, list sugar §21) | `start_expr` |
| `Map` | supported (R3B.4.2) | `start_expr` |
| `Binary` | supported (R3B.2) | `start_expr` |
| `Range` | supported (R3B.5) | `start_expr` |
| `Index` | supported (R3B.6) | `start_expr` |
| `Field` | supported (R3B.6; struct-instance arm unreachable until Construct) | `start_expr` |
| `Call` | **unsupported** | wildcard `E4999` |
| `Method` | **unsupported** | wildcard `E4999` |
| `ListComp` | **unsupported** | wildcard `E4999` |
| `MapComp` | **unsupported** | wildcard `E4999` |
| `Construct` | **unsupported** | wildcard `E4999` |
| `Lambda` | **unsupported** | wildcard `E4999` |
| `Pipe` | **unsupported** | wildcard `E4999` |
| `Match` | **unsupported** | wildcard `E4999` |

### Stmt coverage

| Variant | Iterative status | Evidence |
|---|---|---|
| `Expr` | supported | `start_stmt` |
| `Let` | supported | `start_stmt` |
| `Return` | supported | `start_stmt` + `Cont::ReturnFrom` |
| `Throw` | supported | `start_stmt` + `Cont::ThrowFrom` |
| `Break` | supported (delivered; loop boundary pending) | `start_stmt` |
| `Continue` | supported (delivered; loop boundary pending) | `start_stmt` |
| `LetPattern` | **unsupported** | wildcard `E4999` |
| `Assign` | **unsupported** | wildcard `E4999` |
| `While` | **unsupported** | wildcard `E4999` |
| `Loop` | **unsupported** | wildcard `E4999` |
| `For` | **unsupported** | wildcard `E4999` |
| `Try` | **unsupported** | wildcard `E4999` |

There are exactly **two** unsupported wildcard arms (one for expressions, one
for statements). Both return the deterministic `E4999` sentinel
("construct is not supported by the iterative engine"). There is **no recursive
fallback**: no AST-evaluating `Interp` helper (`eval`, `eval_inner`,
`exec_block`, `exec_stmt`, `call`, `call_value`, `read_target`, `write_target`,
`construct`, `iterate`, `bind_pattern`, `match_pattern`) is called from the
iterative machine. Only value-level helpers are shared (`binary`, `index_get`,
`field_get`, `method`, `format_value`, `error`), which never evaluate AST nodes.

`Item` variants are declaration-only at runtime and are handled identically by
both engines (`fn`/`struct`/`enum`/`impl` register tables; `const`/top-level
expression execute; `trait`/`alias`/`use`/`module` are front-end/resolver
concerns with no runtime action). Multi-source compilation is explicitly
rejected by the iterative entry point rather than silently recursing.

**Confirmed inventory (no discrepancies):** exactly 8 unsupported `Expr`
(`Call`, `Method`, `ListComp`, `MapComp`, `Construct`, `Lambda`, `Pipe`,
`Match`) and exactly 6 unsupported `Stmt` (`LetPattern`, `Assign`, `While`,
`Loop`, `For`, `Try`). The prior R3B.7 list is complete and no stale claim was
found.

**Sentinel-coverage nuance (audit finding, informational):** the dedicated
`r3a::unsupported_cases` set (11 R3A golden `E4999` rows) covers `call`,
`method_call`, `field_access`, `lambda`, `pipe`, `match`, `list_comp`,
`while_stmt`, `loop_stmt`, `let_pattern`, `assign`. There is **no** dedicated
single-construct `E4999` row for `map_comp`, `for`, or `try` in any committed
phase oracle; those variants are nevertheless confirmed unsupported
mechanically (the shared statement wildcard) and dynamically (an independent
probe run for this audit found each returns E4999 with no fallback). When their
phases land, adding one explicit sentinel case for each closed gap is
recommended so the boundary is pinned by a committed golden rather than by the
wildcard arm alone.

## 2. Recursive semantics of every remaining construct

Line references are to src/run/mod.rs unless noted.

### Expr::Call — `eval_call`, ~L1642
- **AST:** `Call(callee, args: Arc<[Arg]>, ty_args, span)`.
- **Order:** every argument expression is evaluated in **source order, once,
  before binding**; then callee resolution: `Expr::Name` → overload set →
  `select_overload` → `bind_arguments` → `Interp::call`; else native registry
  (`check_native_arity`); else an environment value via `call_value`; a
  non-`Name` callee is evaluated then `call_value`.
- **Frames:** every user-closure call enters `Interp::call` (depth +1, ≤512;
  expression-nesting budget reset to 0 for the callee body; `current_source`
  swap; `return`→value; `throw`→`E4099`/`THROWN` with `pending_throw`;
  `break`/`continue` crossing a call → `E4028` LOOP_CONTROL).
- **Exactly-once:** all args exactly once; callee once; `bind_arguments` only
  maps; no re-evaluation.
- **Diagnostics:** `E3001` arity/overload/name-mismatch at the callee or call
  span; `E4011` at the call span.
- **Dependencies:** `select_overload`, `bind_arguments`, `call`, `call_value`,
  native registry, `pending_throw`/`uncaught` normalization.

### Expr::Method — ~L1402
- **AST:** `Method(recv, name, args, ty_args, span)`.
- **Order:** receiver evaluated once, then arguments in source order, then:
  struct instance → `select_method_overload` + `self.call(&c, full, span)` (the
  ordinary call-frame path, with `self` bound as the first parameter); every
  other receiver → the builtin method registry (`Interp::method` →
  `stdlib::method`) with the evaluated arguments, including higher-order
  `map`/`filter`/`reduce`, whose callbacks re-enter Aura via `call_value_pub`
  (see §Reentrancy).
- **Frames:** instance methods cross a real user frame; builtin methods do not
  (except the reentrant callbacks, which do).
- **Dependencies:** everything `Call` needs, plus the method-overload selector,
  the method table, and `Value::Instance` (⇒ `Construct`).

### Expr::Construct — `construct`, ~L1890
- **AST:** `Construct(name, args, ty_args, span)`.
- **Order:** all field argument expressions evaluated in source order first;
  then struct form (all-positional or all-named; unknown/duplicate/missing
  fields → `E3002`/`E3001`) or enum-variant form; produces
  `Value::Instance`/`Value::Variant`.
- **Diagnostics:** `E3001` mixed/duplicate/missing fields; `E3002` unknown
  field/struct.
- **Dependencies:** struct/enum declaration tables (`declare_item`); enables
  `Expr::Field`'s currently-unreachable instance arm and `Match`'s variant
  values.

### Expr::Lambda — ~L1555
- **AST:** `Lambda(params, body, span)`.
- **Semantics:** captures the **current environment by reference** (`Rc<Env>`);
  a block body becomes the function body, otherwise the expression becomes an
  implicit `return`. Registers the closure's source for diagnostics.
- **Dependencies:** `Closure`, closure-source registration; its *use* depends
  entirely on `Call`/`Method`/`Pipe`.

### Expr::Pipe — ~L1575
- **AST:** `Pipe(lhs, rhs, span)`.
- **Semantics:** evaluate `lhs` once, then `rhs` once, then
  `call_value(rhs_value, [lhs_value], span)`. Exactly one argument; a
  non-callable RHS is `E3001`.
- **Dependencies:** `call_value` ⇒ `Call`/`call`; lambda/closure values.

### Expr::ListComp / Expr::MapComp — ~L1483 / ~L1505
- **AST:** `{ key, value, pattern, iterable, filter, span }` / list analog.
- **Order:** iterable evaluated once; per item: fresh child scope, bind the
  pattern, evaluate the optional filter in that scope (`truthy` decides), then
  evaluate the element (list) or **key before value** (map) in the same scope.
  Map keys go through `MapKey::from_value` with `E3001` at the key span;
  duplicate keys last-wins.
- **Iteration:** `self.iterate(&subject, span)` materializes non-Range
  iterables with a 10,000,000-element cap (`E4013` beyond); a `Value::Range`
  subject iterates lazily in `For` only (comprehensions materialize).
- **Diagnostics:** pattern mismatch `E3001`; filter/element/key errors propagate
  first-error-wins.
- **Dependencies:** `iterate`, `bind_pattern` (recursive), `MapKey`, `Call` for
  no part of the shape itself (but element/filter expressions may contain any
  supported expression, including calls).

### Expr::Match — ~L1617
- **AST:** `Match(subject, arms, span)`.
- **Order:** subject once; first arm whose **pattern matches** (and whose guard
  is `truthy` when present) wins; each arm gets a fresh child scope with its
  pattern bindings; no match → `E4025` (NO_MATCH).
- **Patterns:** `_`/bind always match; literal patterns must match; list/variant
  patterns require matching arity and recurse (`match_pattern`, `bind_pattern`);
  a variant pattern names a declared tag.
- **Dependencies:** pattern matching/binding (`match_pattern`, `bind_pattern`);
  `Construct` for enum-variant values (variant patterns cannot match anything
  else); guard evaluation (any expression).

### Stmt::LetPattern — ~L762
- **AST:** `LetPattern { pattern, value, span }`.
- **Semantics:** evaluate value once; bind into a **temporary child scope**
  (atomic: a mismatch never leaves a partial binding); on success transfer each
  bound name into the enclosing scope with `define_shadowing` (new binding
  identity, §16.3). Immutable bindings.
- **Dependencies:** `bind_pattern`; shadowing semantics identical to `Let`.

### Stmt::Assign — ~L785
- **AST:** `Assign { target, value, op, span }`.
- **Order:** RHS evaluated **first**, once; simple assign writes; compound
  (`op: Some`) reads the target (`read_target`) then applies `binary`, then
  `write_target`. Targets: `Name` (immutability → `E4027` ASSIGN_IMMUTABLE),
  `Index` (base then index evaluated; `index_set`), `Field` (`field_set`);
  any other target → `E4004` INVALID_ASSIGN.
- **⚠ Double-evaluation:** for a compound assignment the target's
  **subexpressions are evaluated twice** — once by `read_target` and once by
  `write_target` (`a[idx()] += 10` calls `idx()` twice). This is current
  recursive behavior; the iterative engine must reproduce it exactly or the
  spec must be changed by an explicit decision. This is a **semantic trap** to
  pin in R3D.
- **Dependencies:** `read_target`, `write_target`, `index_set`, `field_set`,
  `binary`, `Env::assign`.

### Stmt::While / Stmt::Loop — ~L820 / ~L836
- **While:** condition evaluated once per iteration in the enclosing env; body
  in a fresh child scope; `Break`→unit, `Continue`→next iteration, any other
  signal propagates; falsy condition → unit.
- **Loop:** body forever; `Break`→unit; `Continue`→next iteration; other
  signals propagate. No iteration cap (a user `loop` can run forever by
  design).
- **Dependencies:** none beyond block execution; control-flow boundaries for
  `Break`/`Continue`/`Return`/`Throw`.

### Stmt::For — ~L848
- **AST:** `For(pattern, iterable, body, span)`.
- **Order:** iterable once; Range subjects iterate lazily (`start..end`,
  half-open) so `break` on a huge range never materializes; all other subjects
  materialize via `iterate` (10,000,000 cap). Each item: fresh child scope,
  `bind_pattern`, body; `Break`→unit immediately, `Continue`→next, other
  signals propagate.
- **Dependencies:** `iterate`, `bind_pattern`, optional `Construct`/`Match` for
  richer patterns; the loop-boundary control model.

### Stmt::Try — ~L884
- **AST:** `Try { body, catch, catch_body, finally, span }`.
- **Semantics:** run `body` in a new scope; only an explicit `throw` is
  catchable — both a direct `Ctl::Throw` and the cross-frame `THROWN`
  diagnostic (using `pending_throw` to recover the original thrown value).
  Runtime diagnostics (E4007/E4013/E3001/…) are **fatal and propagate**. Catch
  scope binds the caught value. `finally` always runs in the enclosing scope;
  a non-`Val` completion in `finally` overrides the pending result; otherwise
  the original result survives.
- **Dependencies:** `pending_throw` (set at `Interp::call`'s frame boundary and
  by the iterative `FrameBoundary`), throw propagation, finally override
  semantics (15 combinations already pinned by the corpus "finally" matrix).

### Reentrancy (native → Aura callbacks)

Six stdlib sites re-enter the evaluator with a user function value:
`map`/`filter`/`reduce` as free functions (src/stdlib/mod.rs:311/322/336) and as
list methods (src/stdlib/mod.rs:669/678/690), all through
`Interp::call_value_pub` → `Interp::call`. In the recursive engine this nests
engine frames naturally. In the iterative machine a native invoked
synchronously cannot suspend the machine mid-native, so resuming a callback
requires either (a) running a **nested** machine for the callback call
(bounded by the callback's own frame accounting, acceptable if tracked), or
(b) a resumable native protocol where the native returns a request-to-call
that the machine schedules as a continuation. Whichever is chosen must be
decided in **R3G.1**; it is the principal reason R3G exists as its own phase
rather than folding into R3C.1. `sort`/`sort_by` do **not** re-enter (they use
value comparators), and no other callback-taking builtin exists today.

## 3. Dependency DAG

Nodes are the 14 remaining constructs plus observability/utility nodes:
`SIDE` = side-effecting expressions available for exactly-once differential
testing (`print`/assignment effects through a call), `ITER` = an explicit
iteration/loop boundary carrying `Break`/`Continue`, `PAT` = pattern binding,
`FRAME` = cross-frame throw conversion (`THROWN`).

| From → To | Kind | Reason |
|---|---|---|
| `Lambda` → `Call` | implementation | a closure is only usable by being called; `call_value`, frames, argument binding |
| `Pipe` → `Call` | implementation | `Pipe` is literally `call_value(rhs, [lhs])` |
| `Method` → `Call` | implementation | instance-method dispatch reuses the ordinary call-frame machinery (`self.call(&c, …)`) |
| `Method` → `Construct` | semantic | the instance-method path is unreachable without `Value::Instance`; without `Construct`, `Method` only exercises the builtin registry (already reachable via `Field`) |
| `Construct` → `Match` (variant arms) | semantic | enum-variant values require `Construct`; variant patterns cannot match otherwise |
| `Construct` → `Field` (instance reads) | semantic | unlocks the currently-unreachable instance arm |
| `Method` → `Field` | testability | instance field/method reads become differentially observable |
| `Call` → `SIDE` | testability | calls make `print`-based side effects observable inside expressions, closing the exactly-once gaps from R3B.4–R3B.7 |
| `Call` → `Construct` | none (independent) | both evaluate argument expressions; neither needs the other |
| `Call` → `ListComp/MapComp` | implementation | an element/key/filter expression may contain a call; the comprehension machinery itself does not |
| `Assign` → `SIDE` | implementation | assignment is the second side-effect channel (mutation) |
| `Assign` → `While`/`Loop` | semantic | loops without mutation can terminate but cannot accumulate; `break`/`continue` already exist as statements |
| `Assign` → `For` | semantic | accumulator patterns need mutation |
| `LetPattern` → `For` | implementation | `For`'s per-item binding is `bind_pattern` (iteration must also work with simple bind patterns) |
| `Match` → `LetPattern` | shared primitive | both use `bind_pattern`/`match_pattern`; `Match` has guards and multiple arms, `LetPattern` is a single atomic destructure |
| `For` → `ListComp`/`MapComp` | semantic | comprehensions are the expression form of the `For` iteration protocol (filter + value) |
| `While`/`Loop` → `SIDE` | testability | loops with mutation/calls make non-termination and boundary effects observable |
| `Try` → `Call` | control-flow | the cross-frame `THROWN` → `pending_throw` catch path is only reachable through a call boundary |
| `Try` → `FRAME` | implementation | `finally` override and pending-value semantics interact with frame unwinding |
| `Construct` → `Lambda` | none | independent |
| `Call` → `Try` | testability (not implementation) | try/catch of runtime errors is fatal-only; `throw` from a callee is the interesting path |

No edge was invented for convenience; each is justified by a code path above.

## 4. Strongly connected components

There are **no genuine cycles**. The apparent `Call ↔ Lambda` mutual relation
is one-directional in practice: `Lambda` *creates* a closure value and needs no
call machinery; only *using* a lambda needs `Call`. The `Call ↔ Try` relation
likewise resolves one way: `Try`'s frame-crossing path needs `Call`, not the
reverse. Therefore the whole remaining surface is a DAG whose root is `Call`.

The only shared primitive that could create a cycle —
`bind_pattern`/`match_pattern` — is **already implemented recursively in the
recursive engine**; porting it as an explicit continuation serves `For`,
`LetPattern`, `Match`, and comprehensions without any of them depending on
another. It is a **leaf utility**, not a cycle.

## 5. Recommended implementation order (R3C and beyond)

Ordered by dependency depth and falsifiability, not wall-clock time. Each row
is a proposed microphase; every microphase is implement → targeted differential
oracle → golden → checkpoint.

| Phase | Constructs | Prerequisites | Semantics frozen | New oracle capability | Principal risk |
|---|---|---|---|---|---|
| **R3C.1** | `Call` (args, overloads, `bind_arguments`, native calls, `call_value`, child frames, `E4011`) | R3B machine, `FrameBoundary` | argument order/exactly-once, frame accounting 512/513, return/throw conversion | side-effect-observable expressions (`print` inside args), first differential exactly-once proofs | frame/continuation nesting; mutual recursion depth |
| **R3C.2** | `Method` (instance methods + builtin registry + receiver order) | R3C.1 | receiver-before-args, overload selection, method-not-a-value `E2003` | instance-method dispatch once `Construct` exists (R3C.3) | couples to `Construct` for full coverage |
| **R3C.3** | `Construct` (struct + enum variant), unlocks `Field` instance reads | R3C.1 | field-argument order, named/positional rules, duplicate/missing `E3001`, `E3002` | `Value::Instance`/`Variant` become reachable; enables variant `Match` later | partial construction must never escape |
| **R3C.4** | `Lambda` + `Pipe` | R3C.1 | capture-by-reference, implicit return, single-arg pipe | closures as call arguments; pipe chains | closure env identity vs. shadowing |
| **R3D.1** | `Assign` (simple + compound, index/field targets) + explicit target-continuation model | R3C.1 | RHS-first, compound double-evaluation (see §2), immutability `E4027`, `E4004` | mutation as an observable channel; exactly-once closure of all earlier phases | the compound double-evaluation trap |
| **R3D.2** | `While` + `Loop` + loop-boundary `Break`/`Continue` continuations | R3D.1 (recommended) | per-iteration scope, condition-once, signal routing | observable accumulating loops; non-termination safety | continuation growth on long loops (must be constant) |
| **R3D.3** | `LetPattern` (`bind_pattern` as explicit continuation) | none beyond R3B | atomic destructure + shadowing transfer | destructuring through frames | pattern recursion depth vs. `MAX_AST_DEPTH` |
| **R3D.4** | `For` (lazy Range + materialized others) + iteration helper | R3D.3, R3D.1 | per-item scope, lazy Range/break, 10,000,000 cap | loops over collections; lazy huge-range break | lazy-range continuation must not materialize |
| **R3E.1** | `ListComp` / `MapComp` | R3D.4, R3D.3 | per-item scope, filter-then-element, key-before-value, last-wins | comprehension differential coverage | nested comprehension state |
| **R3E.2** | `Match` (patterns, guards, arms) | R3C.3 (variants), R3D.3 (binding) | first-match-wins, guard in arm scope, `E4025` | enum/variant dispatch | guard signal propagation |
| **R3F.1** | `Try` (catch `Throw` + cross-frame `THROWN`, finally override) | R3C.1, R3D.* | 15 finally combinations, fatal-error non-catchability | throw/catch across frames; finally ordering | pending-throw recovery, double-run of finally paths |
| **R3G.1** | callback protocol (`map`/`filter`/`reduce` reentry) | R3C.1, R3C.4 | resumable native→machine callbacks | higher-order native differential coverage | nested machine vs. continuation protocol |
| **R3G.2** | optional call-free fast path | R3C.* | performance only, no semantics | — | must be differential-identical |

The **critical path** is `R3C.1 → R3D.3 → R3D.4 → R3E.1` for
comprehensions, and `R3C.1 → R3D.1 → R3D.2` for loops; in parallel within the
same dependency layer, `R3C.2`/`R3C.3`/`R3C.4` after R3C.1, and `R3D.3` can
start immediately after R3B (it needs no call).

## 6. Exact R3C boundary

**R3C should be the Call family**, decomposed as: R3C.1 `Call`, R3C.2
`Method`, R3C.3 `Construct`, R3C.4 `Lambda` + `Pipe`. Rationale:

- `Call` is the highest-leverage prerequisite: it unlocks side-effect
  observability, frames-in-the-machine, and is required by `Method`, `Lambda`,
  `Pipe`, cross-frame `Try`, and any callable callback protocol.
- `Method` is *not* a separate R3 phase from `Call`: its instance path is the
  same frame machinery with an extra selector, so keeping it in R3C avoids a
  duplicated call implementation. Splitting it out would only create a second
  phase that copies R3C.1.
- `Construct` belongs in R3C because it is the only way to make `Method`'s and
  `Field`'s instance paths testable, and it is independent of `Call`.
- `Lambda` + `Pipe` belong in R3C because a lambda is only useful when called,
  and `Pipe` is a one-argument call; both are thin layers over R3C.1.
- Statements and control flow (`Assign`, `While`, `Loop`, `LetPattern`, `For`)
  belong to **R3D**; comprehensions (`ListComp`/`MapComp`) to **R3E** with
  `Match`; `Try` to **R3F**; the callback protocol and fast path to **R3G**.
  This matches the design document's phase plan, with the refinement that
  comprehensions move from "R3E with lambdas" to "R3E after For", because they
  are the expression form of the loop protocol.

The phase boundary follows dependencies, not naming: `Call` first because
everything else either needs it or is strengthened by it.

## 7. Call leverage and exactly-once observability

**Conclusion: yes.** `Call` is the first construct that can make an
interpolation/bound/element evaluated *observably more than once*. Today every
R3B.4–R3B.7 golden has **zero** nonempty-stdout rows; exactly-once claims are
pinned structurally. With `Call` + `print`, a differential case such as
`f"{side_effecting()}{other()}"` or `a[side_effecting()]` order can be asserted
as an exact stdout sequence, converting the structural claims below into
differential ones.

**Deferred-strengthening ledger** (do not forget these; add the named test in
the phase that unlocks it):

| Phase | Claim now | Current evidence | Missing mechanism | Unlocked by | Test to add later |
|---|---|---|---|---|---|
| R3B.2 | eager binary evaluates left then right, once | order visible only via diagnostics | side-effecting operands | R3C.1 | `print`-count differential for `a() + b()` (corpus case exists; add to supported set) |
| R3B.3 | skipped short-circuit operand never runs | skipped-error suppression | observable skip | R3C.1 | `false and print_side()` stdout-empty differential |
| R3B.4.1 | list elements once, left-to-right | first-error-wins only | side-effecting elements | R3C.1 | element `print`-sequence differential |
| R3B.4.2 | map key before value, once each | first-error-wins only | side-effecting entries | R3C.1 | key/value `print`-sequence differential |
| R3B.5 | range start then end, once each | first-error-wins only | side-effecting bounds | R3C.1 | bound `print`-sequence differential |
| R3B.6 | index target then index, receiver once | first-error-wins only | side-effecting operand | R3C.1 | target/index `print`-sequence differential |
| R3B.7 | f-string parts once, left-to-right | first-error-wins and abort | side-effecting interpolation | R3C.1 | interpolation `print`-sequence differential |
| R3B.6/7 | struct-instance `Field`/stringification | unreachable | `Construct` | R3C.3 | instance field read + display differential |
| R3B.7 | variant stringification | unreachable | `Construct` (enum) | R3C.3 | enum variant display differential |
| R3C.1 | frames/`E4011` iterative | synthetic boundary tests | real deep recursion | R3C.1 itself | 510/511/512/513 recursion differential through the machine |
| R3D.1 | assignment exactly-once (compound is twice by design) | recursive-only tests | iterative path | R3D.1 | compound-target double-evaluation differential |
| R3D.2 | loops constant continuation growth | — | loops | R3D.2 | long-loop memory/stack probe |

## 8. Stack-safety analysis for the remaining surface

| Construct family | AST recursion | Call-frame recursion | Continuation growth | Host-stack dependence | Notes |
|---|---|---|---|---|---|
| `Call`/`Method`/`Lambda`/`Pipe` | argument nesting only (already bounded) | **yes — the decisive one** | one `FrameBoundary` per frame; must be reclaimed | must remain zero: call bodies run on the same machine loop | the whole B-1 point |
| `Construct` | field-argument nesting (bounded) | no | one continuation per field | none beyond machine | partial instance must not escape on error |
| `Assign` (index/field targets) | target subexpression nesting (bounded) | no (unless targets call) | fixed small chain | none | compound duplicate evaluation is semantic, not stack |
| `While`/`Loop`/`For` | body/cond nesting (bounded) | no | **must be constant per iteration**, not accumulating | none | the second decisive stack-safety property |
| `LetPattern`/`Match` | pattern nesting — currently parser-bounded only | no | one continuation per pattern level | `bind_pattern` recursion proportional to pattern depth | see below |
| `ListComp`/`MapComp` | expression nesting (bounded) | no (unless elements call) | per-item bounded | none | `iterate` materialization is heap |
| `Try` | body nesting (bounded) | cross-frame via `THROWN` | one handler continuation | none | `finally` must not double-run |

**Pattern-depth caveat (pre-existing, must be handled in R3D.3/R3E.2):**
`Pattern` nesting is *not* counted by `check_expr_depth` (it walks expressions
and statements only) nor by the parser's semantic depth pass; pattern source
nesting is bounded only by the parse-time recursion budget (2048 native, 768
wasm). Measured: a fully-matching nested list pattern (value and pattern nested
equally) is accepted at depth 254 and rejected at 255 with `E1015` — but that
cutoff is enforced by the **value expression's** depth, not the pattern's.
A pattern-only deep binding (`let [[[…a…]]] = xs`) compiles to depth 2 000
(within the parser budget) and fails fast at runtime with `E3001` because the
shallow value mismatches, so no deep `bind_pattern` recursion occurs there.
Consequently, a *successful* deep match currently cannot exceed ~255 because
the value itself must be a literal expression bounded by `MAX_AST_DEPTH`.

The latent risk appears once calls/loops exist (R3C.1+): a value can then be
built deeper than 256 dynamically (e.g. by recursion), while the pattern source
can still be up to the 2048/768 parser budget; `bind_pattern`/`match_pattern`
recurse on the **host stack** proportional to pattern/value depth with no
`MAX_AST_DEPTH` accounting. The iterative port must therefore count pattern
nesting toward `MAX_AST_DEPTH` in its binding continuation, and the recursive
engine's equivalent behavior should be re-verified against a dynamically deep
value when `Call` lands. This is a **deferred correctness obligation**, not a
present defect (patterns are unsupported in the machine today, and no current
program can build a value deeper than the literal limit).

**Decisive cutover step:** `R3C.1` (`Call`) is where the machine gains real
user frames; **`R3D.2`/`R3D.4`** (loops) are where frame/continuation growth
must be proven constant. B-1 is not "fixed" by any single phase; the production
cutover prerequisites are listed in §Production cutover.

## 9. Security / correctness boundary risks

- **Malformed continuation state:** a call must resume exactly one
  `Cont::FrameBoundary`; `Break`/`Continue` from a callee must become
  `E4028` (never escape into an outer loop), `Return` must become a value.
- **Stale environment capture:** `Lambda` captures by reference; `Assign`
  mutates in place. The iterative machine must not clone environments in a way
  that breaks closure identity (the recursive engine uses `Rc<RefCell<…>>` and
  `define_shadowing` identity).
- **Duplicate/skipped evaluation:** compound `Assign` deliberately evaluates
  target subexpressions twice (recursive behavior) — pin it, do not "fix" it
  silently. Comprehension filters must short-circuit per item.
- **Signal swallowing/duplication:** `Try` must catch only `Throw`/`THROWN`,
  must not swallow runtime diagnostics, and `finally` must not run twice nor
  lose the original result; cross-frame `THROWN` must consume `pending_throw`
  exactly once.
- **Frame unwinding:** `E4011` must leave the frame stack intact; a throw
  crossing a frame must pop exactly that frame.
- **Runaway loops:** `While`/`Loop` may run forever by language design; `For`
  over a materialized iterable is capped (10,000,000); `For` over `Range` is
  lazy and may also run forever — no artificial cap should be added that
  changes semantics.
- **Partial construction:** `Construct`/comprehensions/maps must not expose a
  partially built value on error or signal.
- **Diagnostic/span drift:** overload errors at the callee span or call span;
  method errors follow receiver/argument spans; pattern errors at the pattern
  span; comprehension key errors at the key expression span.

## 10. Test architecture per proposed phase

For every microphase, the evidence classes are: focused unit tests in
`src/run/iterative.rs`; recursive-vs-iterative differential cases; a dedicated
LF-pinned golden (`tests/oracle/<phase>_golden.tsv`); program/frame-boundary
cases; first-error-wins; exactly-once via `print` side effects **once R3C.1
exists**; `Return`/`Throw` and, where applicable, `Break`/`Continue`;
stack-safety and AST-depth boundaries; diagnostic/span parity; a
deliberate-divergence mutation detected and reverted byte-exactly;
unsupported/no-fallback E4999 cases; and historical oracle conservation (every
case moved from an older unsupported set must keep its value/program mode and
intent). Do not weaken golden normalization.

## 11. Quantified remaining work

- **Remaining construct families:** 14 constructs (8 Expr + 6 Stmt).
- **Proposed microphases:** 12 (R3C.1–R3C.4, R3D.1–R3D.4, R3E.1–R3E.2,
  R3F.1, R3G.1), with R3G.2 optional/performance.
- **Critical path (implementation depth):** `Call → LetPattern → For →
  ListComp/MapComp` (4 phases) for collections; `Call → Assign → While/Loop`
  (3 phases) for loops; `Call → Construct → Match` (3) for dispatch; `Call →
  Try` (2).
- **Parallelizable after R3C.1:** `Method`+`Construct`+`Lambda`/`Pipe`
  (R3C.2–R3C.4 are independent of each other), and `LetPattern` (R3D.3) is
  independent entirely. `While`/`Loop` (R3D.2) can proceed beside `For`
  (R3D.4).
- **Sequential locks:** R3E.1 requires R3D.4+R3D.3; R3E.2 requires R3C.3 and
  R3D.3; R3F.1 requires R3C.1 and the R3D control layer; R3G.1 requires R3C.1
  and R3C.4.
- **Highest-risk phase:** **R3D.2/R3D.4** (loops) — constant continuation
  growth, `Break`/`Continue` routing, and lazy-range semantics.
- **Highest-leverage phase:** **R3C.1** (`Call`) — unlocks frames, side-effect
  observability, and the majority of remaining constructs.
- **Production-cutover prerequisites:** every microphase complete; call-free
  and call-heavy recursion safe at 512/513 on native, Node, Chromium, and
  Worker; callbacks resumable (R3G.1); loop continuation growth constant;
  oracle switch/recursive engine removal (design B-1R8), which is **not**
  authorized here.
- **Final B-1 closure gate:** the design document's B-1R5/R3G gates — the
  512-frame boundary on every substrate, full differential corpus over both
  engines, and human release decision. R3B completion does not close B-1.

## 12. Deferred-strengthening ledger

Recorded in §7 and in the phase table of §5. Keys: R3B.2–R3B.7 exactly-once
obligations unlock at R3C.1; instance/variant observability unlocks at R3C.3;
compound-assignment double-evaluation at R3D.1; loop stack-safety at R3D.2/4;
pattern-depth bound at R3D.3; callback reentrancy at R3G.1. Each row names the
concrete test to add; none may be dropped when its phase is implemented.

## 13. Independent review

A separate read-only reviewer (no file modifications) independently verified:
exact unsupported inventory via both wildcard arms; no AST-evaluating helper
calls; every dependency claim above with file:line evidence; absence of any
other unsupported current-language feature; and the absence of future-language
variants. All PASS. The reviewer also independently reproduced the stale
documentation contradiction that this audit reconciles (see §14). The
reviewer's only counterexample-class finding was the pre-existing pattern-depth
bound (§8), recorded as a deferred obligation.

## 14. Repository mutation status

This audit is **read-only for implementation**: no production source, tests,
goldens, frozen runtimes, or `.kilo/**` were modified. The only repository
changes are the state/handoff reconciliation this mission explicitly requires
(Git wins; a stale `AGENT_STATE.md`/`CURRENT_HANDOFF.md` had to be corrected so
future sessions do not attempt a second push of an already-closed phase) and
this planning document. No push, no tag, no release, no version change.

**Uncommitted-revision caveat (reviewer finding):** the state-document
reconciliation and this document were initially produced as **uncommitted
worktree changes**; the previously committed revision at `52124a0` still
described B-1R3B.7 as local/unpushed, so a fresh clone reading only committed
history would see the stale claim until this checkpoint commit was created. Per
the mission's read-only/no-push policy no commit was created during the audit
itself; this checkpoint persists the reconciliation. This checkpoint is
**local only** until a separately authorized push closes it remotely.

## 15. Status after this audit

- **B-1:** OPEN (production recursive; WASM defect remains in `v0.2.1`).
- **B-1R3B:** COMPLETE (R3A through R3B.7 remotely closed).
- **B-1R3B.8:** COMPLETE (this audit), checkpointed as one documentation-only
  commit; **not yet remotely closed** (push requires separate authorization).
- **R3C:** NOT STARTED, and **R3C.1 (`Call`) NOT STARTED**. Next mission after
  the checkpoint's remote closure is R3C.1 (`Call`) only, following the design
  document's microphase discipline and requiring explicit human authorization.
