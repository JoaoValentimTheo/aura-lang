# B-1R3A blocker — AST sharing representation decision required

**Finding ID:** B-1R3A-ARCH-1
**Status:** RESOLVED — OPTION A APPLIED AND VERIFIED (2026-10-03).
**Severity:** HIGH (architectural contradiction inside the approved B-1R1 design)
**Decision:** OPTION A — `Arc`-based immutable AST sharing.
**Owners:** human reviewer / Architecture Decision Council. Human approved
Option A; this file records the resolution, the design amendment, and the
implementation evidence.

> **Resolution outcome.** §16.1 of `docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`
> now specifies `Arc` for AST-sharing types (with the rationale inlined); the
> minimum `Arc` conversion is implemented; `Module`/`Expr`/`Stmt` are proven
> `Send` at compile time; the B-1R2 oracle golden is byte-unchanged; no `unsafe`
> was added. Details: §7 below. The original blocker analysis is preserved in
> §1–§6 for the record.

---

## 1. Summary

B-1R3A's mandate (`docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md` §26) is
"AST sharing + machine skeleton". §16.1 mandates that AST children become
**`Rc`-shared** as the enabling prerequisite for suspension storage.
Implementing the first mechanical step of §16.1 (`Box<Expr>` → `Rc<Expr>`,
`Vec<Stmt>` → `Rc<[Stmt]>`, …) does not compile, because the existing front-end
and execution pipelines require the AST to be **`Send`**, and `Rc` is `!Send`.

The design itself requires `Send` (§22, lines 690–692: "`on_execution_stack`'s
Send boundary must be respected (the machine must stay `Send`-compatible for
`execute_with_host_factory`)"). §16.1 and §22 therefore cannot both be
satisfied by the literal §16.1 representation. This is a genuine
architecture-representation choice, not an implementation defect, so it is
recorded here rather than silently resolved.

## 2. Exact evidence

### 2.1 Reproduced compile failure

Applying only the §16.1 field-type changes to `src/ast/mod.rs` and
`src/run/mod.rs` (`Closure.body`) and running `cargo build --locked` produces
**128** `E0277: cannot be sent between threads safely` errors, for every shared
type:

```
error[E0277]: `Rc<[ast::FPart]>` cannot be sent between threads safely
   --> src/parse/mod.rs:13:5
    = help: within `ast::Module`, the trait `Send` is not implemented for `Rc<[ast::FPart]>`
note: required because it appears within the type `ast::Expr`
note: required because it appears within the type `ast::Item`
note: required because it appears within the type `PhantomData<ast::Item>`
   --> src/parse/mod.rs:55:8

error[E0277]: `Rc<ast::Expr>` cannot be sent between threads safely
   --> src/parse/mod.rs:13:5
...
error[E0277]: `Rc<[ast::Stmt]>` cannot be sent between threads safely
   --> src/lib.rs:336:53
```

(All distinct shared shapes fail: `Rc<Expr>`, `Rc<[Expr]>`, `Rc<[Stmt]>`,
`Rc<[Arg]>`, `Rc<[(Expr, Expr)]>`, `Rc<[FPart]>`, `Rc<[Arm]>`.)

### 2.2 Why `Send` is required (unconditional, all substrates)

* `src/parse/mod.rs:12` `parse(src) -> Result<Module>` calls
  `on_parse_stack(src, parse_inner)`; `on_parse_stack<T: Send + 'static>`
  (`src/parse/mod.rs:55`) delegates to `on_execution_stack`, which on native
  spawns a worker thread and **returns `T` by joining it** (`src/lib.rs:81–112`).
  Therefore `Module: Send` and `Expr: Send` are required.
  `parse_expr`/`parse_stmt` (REPL) have the same requirement.
* `src/lib.rs:405` `compile_owned_source` runs `parse::parse` + `resolve` +
  `check` inside `on_execution_stack(move || …) -> Module`, so
  `Module: Send` is required for the public `compile*`/`Compilation` pipeline.
* `src/lib.rs:336` `Compilation::execute_with_host_factory` moves the
  destructured `module` into `on_source_execution_stack` (also
  `T: Send + 'static`, `src/lib.rs:129–132`) for `run_sourced`; a non-sourced
  path does the same at `src/lib.rs:343`. So `Module: Send` is required for
  **every** public execution entry point on native.
* There is **no** `unsafe impl Send`/`Sync` and no `SendWrapper` anywhere in
  `src/` (verified by grep), so no existing escape hatch applies.

### 2.3 The two design clauses

* §16.1 (line 447–455): "`Box<Expr>` → `Rc<Expr>` … `Vec<Stmt>` →
  `Rc<[Stmt]>` …". Rejected alternatives considered there: an arena with `u32`
  handles (rejected as more invasive) and deep `Clone` in continuations
  (rejected as O(program-size)). **`Arc` was not considered.**
* §22 (line 690–692): "`on_execution_stack`'s Send boundary must be respected
  (the machine must stay `Send`-compatible for `execute_with_host_factory`)."

### 2.4 Reproduction command

```sh
# apply §16.1 field types, then:
cargo build --locked 2>&1 | grep -c "cannot be sent between threads safely"  # 128
```

The experiment was fully reverted; `src/` is byte-identical to the pre-phase
tree (`src/run/mod.rs` SHA-256
`812287ed25d7817f04cb32407293205d4c66287eb162867f4f745abd546caf32`).

## 3. Why this cannot be patched around inside B-1R3A

* The brief forbids `unsafe`; the design forbids `unsafe`.
* The `Send` requirement is not incidental: it is the public embedding contract
  (`execute_with`, `compile*`) on native. Removing it would change the substrate
  architecture (parser relocation off the execution stack) and re-expose the
  parser to host-stack overflow — exactly the class of defect B-1R is fixing.
* Choosing a different shared-ownership representation changes the type that
  §16.2/§16.3 mandate for `Cont`/`Machine`, i.e. it is a design revision, not a
  mechanical detail.

## 4. Options (for human / Council decision)

**Option A — `Arc` for AST-sharing types (recommended).**
Replace `Rc` with `Arc` for the §16.1 AST-sharing types only: `Expr` children,
`Rc<[Expr]>`, `Rc<[Stmt]>`, `Rc<[Arg]>`, `Rc<[(Expr, Expr)]>`, `Rc<[FPart]>`,
`Rc<[Arm]>`, and `Closure.body`. `Cont`/`Machine` use the same `Arc`-shared
nodes. Runtime-only sharing that never crosses the thread boundary
(`Env = Rc<RefCell<EnvData>>`, `Rc<Closure>` created inside the execution
thread) may remain `Rc`, or also move to `Arc` for uniformity. `TypeExpr` and
explicit type-argument collections (`Vec<TypeExpr>`) are **not** converted:
§16.1 does not list them, they never need to be retained in a continuation, and
an owned `Vec<TypeExpr>` is already `Send + Sync`.
* Satisfies §16.1's intent (cheap shared children, no deep clone per
  suspension) and §22's `Send` requirement simultaneously.
* Cost: atomic refcounts on AST clone/drop (cold path; front end, not the
  evaluator hot loop). `PartialEq`/`Clone` semantics unchanged.
* This is a mechanical, semantics-neutral change.

**Option B — Keep `Rc`, move the front end off the execution stack.**
Parse/resolve/check inline or on a `Module` owner thread, transferring only
plain data across boundaries. Rejects §16.1's `Send` implication but reworks the
parser's deep-recursion substrate; high risk to the `E1015` backstop. Not
recommended.

**Option C — Keep `Rc`, keep `Send` via an arena + handle (indices).**
The alternative §16.1 already rejected as "more invasive"; would also require
solving `Send` for the arena. Not recommended.

## 5. Recommendation

Option A (`Arc` for AST-sharing types). It is the smallest change that satisfies
both design clauses, keeps the E1 architecture (explicit continuation machine,
no CPS/IR/bytecode), and is purely a representation choice with no observable
semantic effect. Because it contradicts the literal `Rc` text of §16.1, it
requires explicit human/Council approval before B-1R3A proceeds; §16.1 should
then be amended to say `Arc` (with a one-line rationale referencing this
document).

## 6. Consequence (at time of finding)

**B-1R3A could not proceed until this representation was chosen.** B-1 remains
OPEN. No runtime behavior changed and no evaluator skeleton was committed by the
blocker pass.

## 7. Resolution — Option A applied and verified

**Decision:** `std::sync::Arc` for the AST-sharing types of design §16.1.
`Arc<T>` is `Send + Sync` when `T: Send + Sync`; the AST containment graph is
pure owned data (`String`, `Vec`, `Box`, `Span`, plain enums) and therefore
`Send + Sync`, so every existing boundary is preserved.

**Rejected alternatives:**

| Option | Description | Why rejected |
|---|---|---|
| B | Keep `Rc`; remove/relocate the `Send` thread boundary | Reworks the parser substrate that gives the `E1015` host-stack backstop; re-exposes the parser to stack overflow; larger and riskier than a representation swap. |
| C | `unsafe`/custom `Send` wrappers over `Rc` | Forbidden by the brief and the design ("No `unsafe`"); would assert a false invariant (single-thread `Rc` behind a `Send` facade) and risk real UB. |
| D | Arena / `u32` handles | Larger redesign; §16.1 already rejected it; does not by itself solve `Send`. |
| E | IR / bytecode VM redesign | Explicitly out of scope for E1 (design §15 E3 rejected). |

**Applied conversion (minimum surface):**

| old | new |
|---|---|
| `Box<Expr>` | `Arc<Expr>` |
| `Vec<Expr>` (`List`, `Tuple`) | `Arc<[Expr]>` |
| `Vec<Stmt>` (bodies) | `Arc<[Stmt]>` |
| `Vec<Arg>` (call/method/construct) | `Arc<[Arg]>` |
| `Vec<(Expr, Expr)>` (`Map`) | `Arc<[(Expr, Expr)]>` |
| `Vec<FPart>` (`FStr`) | `Arc<[FPart]>` |
| `Vec<Arm>` (`Match`) | `Arc<[Arm]>` |
| `Closure.body: Vec<Stmt>` | `Arc<[Stmt]>` |

Runtime-only sharing (`Env = Rc<RefCell<EnvData>>`, `Rc<Closure>` created inside
the execution thread) is unchanged — it never crosses a thread boundary and is
not part of §16.1.

**Evidence (see commit + validation for the current run):**
- `parse`, `compile_owned_source`, and `Compilation::execute_with_host_factory`
  all cross `on_execution_stack`/`on_source_execution_stack` (`T: Send + 'static`).
- The literal `Rc` conversion produced ≈128 `E0277` "cannot be sent between
  threads safely" errors (reproduced from current source in the blocker pass).
- With `Arc`, native `cargo check --all-features` and MSRV `+1.83.0 check
  --all-features` are clean; compile-time `assert_send::<Module/Expr/Stmt/Item>()`
  hold.
- `cargo test --test evaluator_oracle --features evaluator-oracle` is green and
  `tests/oracle/golden.tsv` is byte-unchanged.
- `grep -nE '\bunsafe\b|unsafe impl' src` shows no new occurrence.

**B-1R3A may now proceed** to the machine skeleton without a representation
contradiction. B-1 itself remains OPEN until the iterative engine exists and
passes the boundary gates.

### 7.1 Independent review outcome

A fresh read-only reviewer attempted to falsify the representation change
against 20 questions. Result: no `Send`, semantic, provenance, span, ordering,
or `unsafe` regression; oracle golden byte-unchanged. Three findings were
reproduced and addressed:

* **New test violated the fmt/clippy floor** (`ptr_as_ptr`; formatting). Fixed:
  `tests/ast_sharing.rs` now uses `std::ptr::eq(Arc::as_ptr(..), ..)` and is
  `cargo fmt`/`clippy -D warnings` clean.
* **Decision artifact over-stated the applied surface** (listed `TypeExpr` /
  `Vec<TypeExpr>` as converted). Fixed: `TypeExpr` is intentionally not
  converted (not in §16.1; never retained in a continuation; already `Send`),
  and the tables now match the code.
* **Design §19 edit left a duplicated dangling fragment.** Fixed.

One non-blocking cost is recorded: `desugar_pipe` (`src/parse/mod.rs`) now does
`args.to_vec()` before inserting the piped value, because `args` is `Arc<[Arg]>`
rather than an owned `Vec`. This is parse-time only, copies the top-level `Arg`
nodes (their `Expr` children are `Arc`, so subtrees are refcount-bumped, not
copied), and is not on the runtime hot path. It is noted for a possible
follow-up; it is not a correctness or `Send` issue.
