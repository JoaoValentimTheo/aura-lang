# Aura v3 — Mandatory Corrections Report

**Scope:** the mandatory architectural corrections demanded by
[`ARCHITECTURE_REVIEW.md`](ARCHITECTURE_REVIEW.md) §G.1 (F-01, F-02/F-03,
F-14) plus the "soon" semantic-completion items §G.2 (F-04, F-05, F-06, F-10,
F-15) and the documentation items F-07/F-08/F-13/F-17.

This report records only what changed, what deliberately did not, and how the
result was verified. The review itself is a point-in-time assessment and is
left unmodified.

No new language feature, module, generic, trait, async construct, or VM was
added. The 256-level nesting limit and 512-frame call limit were not touched.
No diagnostic code was renumbered.

---

## Fixed

### F-01 (P1) — float addition returned `E4999`
`Interp::numeric` (`src/run/mod.rs`) matched `Sub|Mul|Div|Rem|Pow` but omitted
`Add`; `binary()` routed `Add` to `numeric()` only for non-`Int`/`Str`/`List`
pairs, so `1.5 + 2.5`, `1 + 2.5`, and `2.5 + 1` hit the internal-error arm.
`Add` is now handled in both the integer and float paths of `numeric()`, and
the match is total over `BinOp`.

### F-02 / F-03 (P2) — methods and builtins are now validated statically
Introduced `src/stdlib/signatures.rs`, one registry consumed by **both** the
checker and the runtime:

* `builtins()` / `methods()` are the single source of truth for name, arity
  (`min_args`/`max_args`), parameter type classes, and return type.
* `stdlib::builtin_names()` and `stdlib::arity()` are now derived from the
  registry, so the runtime and the checker cannot disagree.
* The checker validates every direct call against `builtin(name)` and every
  method call against `method(receiver_class, name)`: unknown method is
  `E2003`; wrong arity or argument type is `E3001`.

### F-04 (P2) — function return types are inferred
`Checker::functions` now records each function's declared return type (parsed
leniently during hoisting). `infer(Expr::Call)` returns it, and
`infer(Expr::Method)`/`infer(Expr::Call)` consult the signature registry for
builtins. Annotated top-level bindings (`Item::Const` with an annotation) are
now checked too — previously the annotation on a top-level `let` was ignored
entirely. `let x: int = f()` where `f() -> string` is now `E3001` at check
time.

### F-05 (P2) — one orderability predicate
Added `Ty::orderable_with`, mirroring runtime `Value::comparable_with`. The
checker rejects provably incomparable orderings (`[1] < [2]` is `E3001`)
before execution; when a side is `Unknown`, it stays permissive so there are no
false positives.

### F-06 (P2) — function equality by identity
`Value::equals` now compares closures by `Rc::ptr_eq` and natives by name.
`g == g` is `true`; distinct functions are not equal. Documented in
`docs/contract.md` §6/§7.

### F-10 (P2) — the Python boundary no longer loses data
In `src/bridge/mod.rs`:

* A Python `int` outside `i64` is rejected with `E4013` instead of being
  silently demoted to a lossy `f64`.
* A dict key that is not a string is rejected with `E5002` instead of being
  stringified, which previously collapsed distinct keys such as `1` and
  `"1"`.

### F-07 / F-08 / F-13 / F-17 — documentation
* `docs/contract.md`: documented `finally` precedence (a control-flow signal
  raised in `finally` replaces the pending outcome); documented that `pub` is
  inert uniformly on `fn`/`struct`/`enum`/`type` and `use` is inert anywhere;
  expanded §6 with the newly-enforced static rules.
* `docs/grammar.md`: corrected the stale `pipe` production — `x |> f(a)`
  desugars at parse time to `f(x, a)`.
* `docs/errors.md`: broadened `E5002` to cover a value that cannot cross the
  boundary, and noted `E4013` for an out-of-range Python integer.
* `README.md`: added an "Architecture" note on the single pipeline and the
  shared signature registry.

---

## Not fixed (deliberately)

These are explicit non-goals of this phase; each is a language-design or
future-work decision, not a defect that blocks the current architecture.

| Finding | Reason |
|---|---|
| F-09 — `Expr::Tuple` is a list | Intentional surface; lowering at parse time is a cleanup, not a correctness fix. |
| F-11 — no `try` without `catch` | Freeze or extend deliberately (grammar decision). |
| F-12 — `match` arms cannot be bare control flow | Language-design decision. |
| F-16 — `Range` ordering unspecified | Equality/display are defined and tested; ordering falls through to "incomparable", which is now consistently a static error. |

Nothing in the NO-GO list (generics, traits, async, VM, module language) was
started.

---

## Semantic changes

Changes a user could observe:

1. `1.5 + 2.5` and friends now evaluate instead of erroring (`E4999` gone).
2. Method typos and wrong method arity are caught by `aura check`
   (`E2003` / `E3001`).
3. Builtin arity/type errors (`len([1], [2])`) are caught by `aura check`.
4. `let x: int = f()` with `f() -> string` is a static `E3001`, including at
   top level (previously the top-level annotation was ignored).
5. `[1] < [2]` is a static `E3001`.
6. `g == g` is `true` (was `false`).
7. A Python integer beyond `i64` is `E4013`; a non-string Python dict key is
   `E5002`.
8. Chained comparison such as `1 < 2 <= 3` is a static `E3001` (it always was
   a runtime `E3001`; it is now caught earlier). The grammar sample that
   relied on the permissive checker was corrected to a type-correct form.

No existing valid program changed meaning.

---

## Architecture changes

* **One signature registry** (`src/stdlib/signatures.rs`): `Signature`,
  `MethodSig`, `Param`, `Accepts`, `TypeClass`, `Returns`, plus
  `builtin()`/`method()`/`method_exists_anywhere()`. Runtime and checker are
  two consumers of one table.
* **`Ty` hoisted** from `src/check/types.rs` to `src/types.rs`; `check`
  re-exports it (`pub use crate::types;`) so existing paths keep working.
* **One execution pipeline** (`src/lib.rs`): `CompileMode`,
  `compile(src, mode)`, `compile_with_mode`, `compile_module`, and
  `execute(module, stdout)`. `run_source`, `run_toplevel_stdout`, and
  `run_program` are thin wrappers; `aura check` now goes through `compile`;
  the REPL checks through `Checker::check_mode`. The "does a module need a
  `main`?" decision lives in exactly one method (`Checker::check_mode`).

---

## Tests added

| Test | Location | Locks down |
|---|---|---|
| `f01_float_addition_is_not_an_internal_error` | `tests/regressions.rs` | F-01 |
| `f01_arithmetic_matrix` | `tests/regressions.rs` | F-01 (operator × type-pair table) |
| `f02_unknown_method_is_rejected_statically` | `tests/regressions.rs` | F-02 |
| `f02_builtin_arity_and_types_are_static` | `tests/regressions.rs` | F-03 |
| `f04_return_type_propagates_to_annotated_binding` | `tests/regressions.rs` | F-04 |
| `f05_orderability_is_checked_once` | `tests/regressions.rs` | F-05 |
| `f06_function_equality_is_identity` | `tests/regressions.rs` | F-06 |
| `f10_python_boundary_rejects_lossy_values` | `tests/regressions.rs` (cfg `py`) | F-10 |
| `checker_and_evaluator_agree` | `tests/property.rs` | F-15 (grammar-directed differential, 2000 cases) |
| `pipeline_passes_the_left_operand_as_the_first_argument` | `tests/grammar.rs` | F-17 |
| `finally_control_flow_overrides_the_pending_outcome` | `tests/run.rs` | F-07 |
| `pub_and_use_are_inert_but_parse` | `tests/run.rs` | F-08 / F-13 |
| `oversized_python_int_is_rejected_not_truncated`, `non_string_python_dict_keys_are_rejected` | `tests/python.rs` | F-10 |

The differential property test asserts the cross-layer invariant directly:
if the checker accepts a well-formed expression, the runtime must never fail
with a front-end code (`E1xxx`) or `E4999`; if the checker rejects, the code
must be a type mismatch. This is the bug class F-01 belonged to.

---

## Verification

Run from a clean tree on `rewrite/v3-rust`:

```
cargo fmt --all -- --check                                             # clean
cargo clippy --all-targets --all-features -- -D warnings               # clean
cargo clippy --all-targets --no-default-features --features cli,repl,json,regex,time -- -D warnings   # clean
cargo clippy --all-targets --no-default-features --features cli -- -D warnings                        # clean
cargo test --all-features                                              # all pass
cargo test --no-default-features --features cli,repl,json,regex,time   # all pass
PROPTEST_CASES=2048 cargo test --test property --all-features          # all pass
cargo build --release --no-default-features --features cli,repl,json,regex,time
cargo tree --no-default-features --features cli,repl,json,regex,time   # no pyo3
cargo +nightly miri test --lib --no-default-features --features cli    # clean
```

The four reproduction commands from the review's appendix now behave as
intended:

```
1.5 + 2.5                  -> 4.0
[1].nope()  (check)        -> E2003
len([1], [2])  (check)     -> E3001
fn f() -> string / let x: int = f()  (check)  -> E3001
```

---

## Remaining risks

* **Conservative inference.** The checker never reports a mismatch it cannot
  prove, so a receiver or operand typed `Unknown` (e.g. an unannotated
  parameter) is not validated. This is intentional (no false positives) but
  means method/orderability checks are incomplete for dynamically-typed code.
* **`pub` remains inert.** F-08's inconsistency is documented, not unified in
  the AST; `Item::Fn` still carries `public` while other items drop it.
* **Generate-time gap.** The differential generator covers arithmetic,
  comparison, equality, and `if`, not calls/methods/closures; the signature
  registry is exercised by the regression tests instead.
* **Miri is `continue-on-error` in CI** and was run locally on the lib target
  only.
* **Python boundary** changes are only observable with `--features py`; the
  no-`py` build is unaffected.

---

## Final invariant

> For every program, `parse → check → execute` is the only path; the checker
> and the runtime share one signature registry and one orderability predicate;
> and whenever the checker accepts a program it never fails with a front-end
> diagnostic or an internal error. `aura check` and `aura run` differ only in
> whether `fn main` is required.
