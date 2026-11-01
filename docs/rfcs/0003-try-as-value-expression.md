# RFC 0003 — `try` as a Value Expression

- **Status:** Proposed — reconstructed for the NEBULA 0.3.2 campaign (Directive
  008, gate G02). **Not accepted.** It records a decision that requires human
  authorization (§2.1) and a checker repair that gate G02 forbids implementing
  before that authorization (§2.2).
- **Scope:** the static type of a `try` statement in tail position; whether
  `try` becomes a syntactic expression. **Grammar delta is proposed as an
  explicit, separate decision (§2.1)** — not silently taken.
- **Authorities:** `docs/LANGUAGE_SPEC.md` §14.5–14.6 (`try`/`catch`/`finally`,
  lines 1720–1762), §15.2 (implicit body value, lines 1808–1831), §15.3
  (`none` and structured expectations, lines 1018–1026), §4.7 (patterns);
  `docs/grammar.md` lines 75–124 (`stmt`, `try_stmt`, `expr`, `atom`);
  `docs/rfcs/0001-exception-catch-selection.md`; **ADR-0002** (builtin-name
  namespace reservation); `src/check/mod.rs` (`block_value_ty` :2357,
  `block_tail_mismatch` :5893); `src/run/mod.rs` (`Stmt::Try` :1264).
- **Human decisions:** two. **D1** — promote `try` to an expression (§2.1).
  **D2** — authorize the three-part checker repair (§2.2). Neither is resolved
  here. Provenance is stated in §0.

## 0. Provenance

The campaign brief (Directive 008 §3.3, `:77`) requires `try` **as an
expression** while also requiring that the existing
`try { … } catch … finally …` statement form must not break, and it directs
that if the two conflict in a way the agent cannot resolve, the agent must
**suspend and request a decision — never a heuristic parser**.

No pre-existing Nebula artifact (RFC draft, inventory, contract) existed in the
tree when G02 began; §0.4 of `docs/engineering/NEBULA_CAMPAIGN.md` records that
absence. This RFC is therefore a **reconstruction from repository evidence**,
not a transcription of an approved design. Every claim below is backed by a
probe recorded in §6; nothing is asserted from the brief alone.

## 1. Motivation

### 1.1 The brief's requirement

Directive 008 §3.3 requires `try` in expression position so that a fallible
computation can be used where a value is expected. §2.4 of the campaign
vocabulary pairs this with `Result<T, E>`. RFC 0002 establishes that `Result`
needs no language change; **`try` as an expression does**.

### 1.2 What already works — and it is more than expected

The investigation found that tail-position `try` **already produces a value at
run time**. This is not documented and not tested, and it is the crux of this
RFC, so it is stated with transcripts.

The evaluator arm (`src/run/mod.rs:1264–1327`) runs the body with
`exec_block(body, env, true)`; on normal completion it hits `Ok(flow) =>
Ok(flow)`, returning the block's last value. So:

```aura
fn f() -> int { try { 41 } catch _ { 99 } }
print(f())
```

prints `41` (check 0, run 0). And on the catch path:

```aura
fn f() -> int { try { throw "x" } catch _ { 99 } }
print(f())
```

prints `99`. A `finally` clause preserves the value:

```aura
fn f() -> int { try { 41 } catch _ { 99 } finally { print(0) } }
print(f())
```

prints `0` then `41`.

**This behaviour is deliberate in the dynamic model** — `finally` replaces a
pending outcome only when it is not `Ctl::Val(_)`, matching
`LANGUAGE_SPEC.md:1760–1762`. **The defect is purely static.**

### 1.3 The static type is wrong — and it is the only such construct

The checker's `block_value_ty` (`src/check/mod.rs:2357–2363`) maps a
`Stmt::Try` tail to `Ty::None`:

```rust
fn block_value_ty(&self, body: &[Stmt]) -> Ty {
    match body.last() {
        Some(Stmt::Expr(e, _)) => self.infer(e),
        Some(Stmt::Return(..) | Stmt::Throw(..)) => Ty::Never,
        _ => Ty::None,          // ← a `try` tail lands here
    }
}
```

`try` is the **only** construct that is statically `none` while dynamically
value-producing. Every sibling construct is typed:

| Construct in tail position | Static | Dynamic | Agreement |
|---|---|---|---|
| `if`/`else` | `int \| string` → `E3005` | — | ✅ typed |
| `match` | `int \| string` → `E3005` | — | ✅ typed |
| `loop { break }` | `none` → accepted | `none` | ✅ agrees |
| `for x in [1] { … }` | `none` → accepted | `none` | ✅ agrees |
| plain block `{ … }` | `T` → `E3005` on mismatch | `T` | ✅ typed |
| **`try`** | **`none` → accepted** | **`T`** | ❌ **diverges** |

The contrast is sharpest inside one program: a plain block tail is checked, a
`try` tail is not.

```aura
fn f() -> int { { "a" } }        // E3005 — checked
fn f() -> int { try { "a" } catch _ { 1 } }   // check 0 — unchecked, runs and yields "a"
```

### 1.4 The consequence: end-to-end unsoundness

Because a `try` tail infers `none`, and `none` is accepted by the tail path, a
function can advertise one type and return another, and the caller's belief is
then enforced in the wrong direction.

```aura
fn f() -> string { try { 5 } catch _ { 6 } }
let s: string = f()
print(s)
```

`check` exits 0 and `run` prints `5`. The caller annotated `s: string` and the
checker accepted it while the callee actually produced `int`. The same shape
with a nominal return type:

```aura
fn f() -> int { try { "hello" } catch _ { "world" } }
let s: string = f()
```

is rejected — but with `error[E3001]: s is annotated as string but its value is
int`, i.e. the checker's **static** belief, not the value the program produces.
The diagnostic is wrong in the same direction as the unsoundness.

### 1.5 Two further defects found while isolating the first

Repairing the `none` inference alone is **not sufficient**, and shipping it
alone would be harmful. Two more defects sit on the same code path; §6 records
the falsification.

**(a) `block_tail_mismatch` never inspects a direct `try` tail.** The function
begins with a `let … else` that only admits `Stmt::Expr`:

```rust
let Some(Stmt::Expr(value, span)) = body.last() else {
    return Ok(None);
};
```

So a *direct* `try` tail returns early and no comparison happens at all. A
*nested* `try` (inside a block, an `if` arm, a `let`) instead reaches the
`actual` computation and receives `Ty::None` from defect §1.3. The two paths
diverge for no reason.

**(b) `none` is accepted where the spec says it must not be.** The tail path
short-circuits before consulting the compatibility relation:

```rust
if matches!(actual, Ty::None | Ty::Never) || return_compatible(expected, &actual) {
    return Ok(None);
}
```

`return_compatible` (→ `return_assignable`, `:40–104`) already encodes the
correct rule, including the deliberate generic carve-out
(`(Ty::Param(_), Ty::None) => true`, §2.3) and a comment stating it
*"preserves the documented generic constructor idiom while rejecting
`-> int { return none }` and structured expectations like `-> [T] { return
none }`"*. **That rejection logic is dead code at tail positions.**

This is not a matter of taste: the spec is explicit. `LANGUAGE_SPEC.md:1018–1026`
says `none` satisfies `-> T` only in a generic body, and that *"a structured
expectation (`-> [T]`, `-> Opt<T>`) is **not** bypassed by `none`"*.
`LANGUAGE_SPEC.md:1819–1826` says the implicit value *"MUST also be compatible
with that type (`E3005` otherwise)"*.

The self-inconsistency is directly observable — the same value and the same
annotation get opposite verdicts depending only on spelling:

| Spelling | `fn f() -> int` | `fn f() -> [int]` | `fn f() -> Opt<int>` |
|---|---|---|---|
| `{ return none }` | `E3005` | `E3005` | `E3005` |
| `{ none }` | **accepted** | **accepted** | **accepted** |

The explicit-`return` column is a ready-made oracle: it is already correct, and
the implicit column disagrees with it.

**One escape hatch is legitimate and must be preserved.** `LANGUAGE_SPEC.md:1819–1826`
also states that *"a body whose last statement is not a value expression — one
ending in a declaration, a loop, or a `return` — has no implicit value and is
not judged by this rule."* So `fn f() -> int { print(1) }` and
`fn f() -> int { }` being accepted is **spec-conformant, not a bug**. Probes J1,
J3, J4, M3, M4 confirm the current behaviour matches that sentence. Any repair
must leave it intact — `let … else` on a non-`Stmt::Expr` tail is correct and
stays.

## 2. Decision

| # | Decision | Implementation |
|---|---|---|
| **D1** | **Escalated — human decision (§2.1).** Whether `try` becomes a syntactic expression. | Not implemented. Grammar untouched by this RFC. |
| **D2** | **Escalated — human decision (§2.2).** Whether to authorise the three-part checker repair. | Not implemented. `src/check/mod.rs` is unchanged at `6f075882`. |
| R1 | Keep the `try` **statement** form exactly as it is. | No change; §14.5–14.6 unmodified. |
| R2 | Tail-position `try` keeps producing a value at run time. | No change; `src/run/mod.rs:1264–1327` unmodified. |
| R3 | Typing a `try` tail must consult **both** arms, not just the body. | Part 2 of the repair, §2.2. |
| R4 | A `try` tail must be judged by the same compatibility relation as `return`. | Part 1 of the repair, §2.2. |
| R5 | A **direct** `try` tail must not escape judgement. | Part 3 of the repair, §2.2. |
| R6 | The spec-sanctioned non-value tail exemption is preserved verbatim. | `let … else` retained for non-`Stmt::Expr`, non-`Stmt::Try` tails. |
| R7 | `try`/`catch`/`finally` scoping, `throw` semantics, and `catch`-mandatory are untouched. | §14.5–14.6 unmodified. |

### 2.1 D1 — the escalated decision (grammar)

**The question.** Today `try` is a *statement* only. `docs/grammar.md:75–77`
puts `try_stmt` in the `stmt` alternation, `:100` defines `expr = pipe`, and the
`atom` production (`:118–124`) admits `block`, `if_expr`, `match_expr` — but
not `try`. So `try` cannot appear where an expression is required.

**Option A — promote `try` to an atom (grammar delta).** Add `try_expr` to the
`atom` production, mirroring the existing `try_stmt`. Minimal delta:

```ebnf
atom = … | try_expr | block | if_expr | match_expr ;
try_expr = "try" block "catch" pattern block [ "finally" block ] ;
```

*Consequences.* `let x = try { 1 } catch _ { 2 }` and
`print(try { 1 } catch _ { 2 })` become legal. `try` in statement position
stays legal and keeps its meaning, so **the two forms overlap** — the statement
becomes a special case of the expression statement. This RFC has **not**
verified that the overlap is unambiguous in every position. It provably is not
in one: `return try { … }` currently reports
`E1006: expected a newline between statements, found 'try'` (§6, probe B5),
which shows the parser treats the operand of `return` as a statement position
and `try` as a statement starter. Resolving that is exactly the
**heuristic-parser** risk the brief forbids the agent from taking unilaterally.

**Option B — do not promote `try`; use a block.** Blocks are already
expressions (`let a = { 1 }` prints `1`), and a `try` statement in tail
position already yields a value. So
`let x = { try { 1 } catch _ { 2 } }` may suffice for the brief's ergonomic
goal with **zero** grammar change. The cost is one extra brace pair.

*Consequences.* No grammar risk. The brief's §3.3 wording ("`try`-expressão")
is then satisfied semantically rather than syntactically. This is the smaller
change and the one this RFC would recommend if forced to choose, but the
choice is not the agent's.

**Option C — suspend `try`-as-expression for 0.3.2** and record it as deferred.
Permitted by the brief's own "if ambiguous, suspend and request a decision".

**Not chosen.** §10.3 of `docs/engineering/NEBULA_CAMPAIGN.md` records all
three. **G03 and every downstream gateway that would consume an expression-form
`try` stay blocked until a human resolves D1.**

### 2.2 D2 — the escalated decision (checker repair)

The repair is a **three-part, coupled** change to `src/check/mod.rs`. All three
parts are required; each was proven necessary by reverse mutation (§6.3).

```rust
// PART 2 — "type a try tail by both arms" (block_value_ty, :2357)
Some(Stmt::Try { body, catch_body, .. }) => {
    let a = self.block_value_ty(body);
    let b = self.block_value_ty(catch_body);
    if a == b { a } else { Ty::Union(vec![a, b]) }
}
```

```rust
// PART 3 — "a direct try tail is judged" (block_tail_mismatch, :5893)
let (value, span) = match body.last() {
    Some(Stmt::Expr(value, span)) => (value, span),
    Some(Stmt::Try { span, .. }) => {
        let actual = self.block_value_ty(body);
        if matches!(actual, Ty::Never) || return_compatible(expected, &actual) {
            return Ok(None);
        }
        return Ok(Some(Diag::new(
            codes::RETURN_MISMATCH,
            format!("function returns `{}` but the value is `{}`",
                    expected.name(), actual.name()),
            *span,
        )));
    }
    _ => return Ok(None),          // R6: the spec-sanctioned exemption
};
```

```rust
// PART 1 — "none is not a free pass" (:5903)
if matches!(actual, Ty::Never) || return_compatible(expected, &actual) {
```

**Why it is escalated rather than applied.** Gate G02
(`NEBULA_CAMPAIGN.md:372`) says: *"every P0 RFC accepted; conflicts decided
explicitly; responsibility/capability map recorded; otherwise do not code the
affected surface."* This RFC is not accepted, so the surface must not be
coded. The change is *validated but withheld* — the patch is preserved as
evidence, not committed. Additionally, PART 3 changes the **observable**
diagnostic on two programs that check cleanly today (§6.2), which is a
behavioural change a human should see before it lands.

## 3. Grammar delta

**None in this RFC.** Options A and B in §2.1 are alternatives, and neither is
chosen. The grammar files are unchanged. If Option A is later authorized, the
delta is exactly the two lines quoted in §2.1 and belongs in a grammar-focused
RFC with its own ambiguity analysis.

## 4. Semantics

### 4.1 What is already correct and must not change

- `catch` remains **mandatory** (`LANGUAGE_SPEC.md:1742`).
- The clause after `catch` remains a full pattern (`:1727–1732`).
- Only explicit `throw` is catchable; runtime diagnostics are not values
  (`:1734–1736`).
- A `finally` that raises a control-flow signal replaces the pending outcome
  (`:1760–1762`).
- A body whose last statement is a declaration, a loop, or a `return` has no
  implicit value and is not judged (`:1819–1826`).
- `none` satisfies a bare generic parameter but not a structured expectation
  (`:1018–1026`).

### 4.2 What the repair changes

Under D2 the static type of a `try` tail becomes the union of its body's value
type and its catch block's value type, and that union is compared against the
declared return type with the same relation `return` already uses. Two
programs change verdict; both are **correct** changes:

```aura
// was accepted (false negative); now E3005 — the body yields `none`, catch yields `int`
fn f() -> int { try { while true { break } } catch _ { 1 } }
// error[E3005]: function returns `int` but the value is `none | int`
```

```aura
// was accepted (false negative); now E3005 — 5 is an `int`, not `none`
fn f() -> none { try { 5 } catch _ { 9 } }
// error[E3005]: function returns `none` but the value is `int`
```

The second is consistent with the pre-existing, unrelated rule that
`fn f() -> none { 5 }` is `E3005`.

### 4.3 Deliberately not changed

- **Exhaustiveness.** There is still **no** exhaustiveness requirement on
  `match`; an unmatched value is `E4029` at run time (`LANGUAGE_SPEC.md:2471–2472`).
  This RFC does not touch it. (RFC 0002 §D2 already records the same.)
- **Discard.** Discarding any expression statement is legal today; this RFC
  does not special-case `Result` or `try`.
- **Generic idiom.** `fn unwrap<T>(o: Opt<T>) -> T { match o { Opt::Some(v) -> v
  Opt::Nothing -> none } }` must keep checking clean. It survives **via the
  `match_expr` path**, not via the tail short-circuit, so removing the
  short-circuit does not regress it (probe r1 = 0 under all variants).

## 5. Migration

**No source migration.** Under D2 no in-tree Aura program changes status.

The three-part repair was swept over **all 125 tracked `.aura` files** (one file
per directory, so the eager sibling-discovery contract cannot contaminate a
result): **17 non-zero before, 17 non-zero after, zero status changes**. The 17
are pre-existing negative fixtures.

Two verdict changes exist only in the probe corpora (§4.2); they are programs
that were silently wrong, not programs that worked.

If D1 Option A is chosen, source compatibility is additive: existing statement
`try` remains legal. If D1 Option B is chosen, nothing changes.

## 6. Enforcement

### 6.1 Evidence — the discriminating probes

`aura` binary built from `6f075882` + the patch described in §2.2. Probes are
one `.aura` file per directory.

| Probe | Source | baseline | D2 |
|---|---|---|---|
| q1 | `fn f() -> int { none }` | 0 | **1** `E3005 … int … none` |
| q2 | `fn f() -> int { return none }` | 1 | 1 (oracle) |
| q3 | `fn f() -> [int] { return none }` | 1 | 1 (oracle) |
| q4 | `fn f() -> [int] { none }` | 0 | **1** `E3005 … [int] … none` |
| q5 | `fn f() -> Opt<int> { return none }` | 1 | 1 (oracle) |
| q6 | `fn f() -> Opt<int> { none }` | 0 | **1** `E3005 … Opt<int> … none` |
| m1 | `fn f() -> string { try { 5 } catch _ { 6 } }` + caller | 0 | **1** `E3005 … string … int` |
| h4 | `fn f() -> int { try { "a" } catch _ { 1 } }` | 0 | **1** `E3005 … int … string \| int` |
| i1 | `fn f() -> int { try { "a" } catch _ { "b" } }` + caller | 1 (`E3001` at caller `5:5`) | 1 (**`E3005` at boundary `2:5`**) |
| p3 | `fn f() -> int { try { "hello" } catch _ { "world" } }` + caller | 1 (same) | 1 (**boundary `2:5`**) |
| j6 | `fn f() -> string { try { 5 } catch _ { 6 } }` | 0 | **1** |
| s1 | `fn f() -> int { { try { 5 } catch _ { 6 } } }` | 0 | 0 |
| s3 | `fn f() -> int { if true { try { 5 } catch _ { 6 } } else { 1 } }` | **1** | **0** |
| s4 | `fn f() -> int { let v = { try { 5 } catch _ { 6 } }  v }` | 0 | 0 |
| s5 | `fn f() -> int { { { try { 5 } catch _ { 6 } } } }` | 0 | 0 |
| s2 | `fn f() -> int { { none } }` | 0 | **1** (correct) |
| p2 | `fn f() -> [int] { { try { 5 } catch _ { 6 } } }` | 0 | **1** `[int] … int` (correct — latent false negative) |
| r1 | generic `unwrap<T>` idiom | 0 | 0 (no regression) |
| r2 | `fn g() -> none { }` + `fn f() -> [int] { g() }` | 0 | **1** (correct) |
| r3 | `fn f() -> none { none }` | 0 | 0 (no regression) |
| r4 | `fn f() -> none { 5 }` | 1 | 1 (unchanged) |
| r5 | `fn f() -> int { "a" }` | 1 | 1 (unchanged) |
| r6 | `fn g() -> none { }` + `fn f() -> Opt<int> { g() }` | 0 | **1** (correct) |
| c4, d2, d3, d5, d6, e1, e3, e4, e5, e6, e8, g5, g6 | valid `try` tails (13 programs) | 0 | **0** (all preserved) |

**One correction to the earlier record.** `s3` is **1 at baseline**, not 0. It
is accepted today only through the `if_expr` path, which does type its arms, so
the `none` from the nested `try` arm produces `int | none` and is rejected. The
repair makes it **0** because the `try` arm now types correctly as `int` — a
genuine repair, not a regression.

### 6.2 Mutation verification in **both** directions

A test that passes on fixed code proves nothing until it fails on broken code.
Each of the three parts was removed from the validated set, rebuilt, and
re-measured.

| Variant | q1/q4/q6 | m1/h4/j6 | s1/s4/s5 | s3 | r2/r6 | verdict |
|---|---|---|---|---|---|---|
| baseline (none) | 0 (miss) | 0 (miss) | 0 | 1 | 0 (miss) | unsound |
| parts 2+3 only | **0** (miss) | 1 | 0 | 0 | **0** (miss) | **PART 1 necessary** |
| parts 1+3 only | 1 | 1 (wrong msg `none`) | **1 false positive** | 1 | 1 | **PART 2 necessary** |
| parts 1+2 only | 1 | **0** (miss) | 0 | 0 | 1 | **PART 3 necessary** |
| **all three** | **1** | **1** | **0** | **0** | **1** | **correct** |

Every part is load-bearing: removing **any** one reintroduces a false negative
or a false positive. Note that parts 1+3 alone produce a *wrong diagnostic*
(`value is none`) rather than a clean pass, because the direct-`try` path
consults `block_value_ty` before it is corrected.

### 6.3 Whole-suite and whole-corpus gates

- `cargo test --locked --all-targets --all-features` → `PASSED=1477 FAILED=0`,
  exit 0, under baseline and under each partial variant and the full repair.
- `cargo clippy --locked --all-targets --all-features -- -D warnings` → exit 0
  under the full repair.
- Corpus sweep, 125 tracked `.aura` files → 17 non-zero before, 17 after,
  `diff` exit 0.

### 6.4 Regression coverage this RFC requires before the repair may land

D2 must not be committed without tests pinning **all** of:

1. `fn f() -> int { none }` → `E3005`, diagnostic **byte-identical** to
   `fn f() -> int { return none }` (q1/q2), and the same for `[int]` and
   `Opt<int>` (q3–q6).
2. `fn f() -> string { try { 5 } catch _ { 6 } }` → `E3005` (m1) — the
   end-to-end unsoundness.
3. `fn f() -> int { try { "a" } catch _ { 1 } }` → `E3005` with the **union**
   `string | int` (h4) — proves both arms are consulted.
4. `fn f() -> int { if true { try { 5 } catch _ { 6 } } else { 1 } }` → clean
   (s3) — proves the false positive does not return.
5. The generic `unwrap<T>` idiom → clean (r1) — proves the §2.3 carve-out
   survives.
6. `fn f() -> int { print(1) }`, `fn f() -> int { }`,
   `fn f() -> int { while false { print(1) } }` → clean (J1/J3/J4) — proves the
   spec-sanctioned non-value exemption survives.
7. `fn f() -> none { none }` → clean (r3) and `fn f() -> none { 5 }` → `E3005`
   (r4) — proves `none` is not simply banned.

Each of these must be **mutation-verified**: revert the corresponding part and
confirm the test fails. A test that cannot fail is worse than no test.

### 6.5 What must not be enforced yet

Gate G03 forbids any downstream API from relying on an expression-form `try`
until D1 is resolved and this RFC is accepted. The checker repair (D2) is
orthogonal — it corrects existing behaviour and adds no capability — but it is
still a **behavioural change gated by G02**, so it stays withheld until a human
authorizes it.

## 7. Open items

| # | Item | Owner |
|---|---|---|
| 1 | **D1** — promote `try` to an atom (Option A), use a block (Option B), or defer (Option C). Requires the `return try` ambiguity (§6, B5) to be addressed. | **Human** |
| 2 | **D2** — authorize the three-part checker repair of §2.2. | **Human** |
| 3 | Add the seven regression tests of §6.4 once D2 is authorized. | Agent, post-authorization |
| 4 | Document tail-position `try` value semantics in `LANGUAGE_SPEC.md` §14.5 — currently **undocumented** and **untested**, despite being deliberate in the evaluator. | Agent, post-D2 |
| 5 | Record the `int \| none` union in a diagnostic golden test, since `Ty::Union` ordering must stay stable. | Agent, post-D2 |
