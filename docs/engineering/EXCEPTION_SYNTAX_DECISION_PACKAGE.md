# Aura 0.3 — Exception System & Custom Exceptions: Decision Package

**Status: DECISION PACKAGE — NO PUBLIC SYNTAX CHOSEN. Outcome A (human gate).**
This package reconstructs current semantics from repository evidence, presents the
authoritative architecture constraints, and asks the human to resolve E1–E6. It
makes **no normative language change** and introduces **no public syntax**.

**Verification pass 2026-10-06 (Code mode).** Every claim below was re-checked
against the repository and by executing the built CLI; the executed-evidence
appendix (§9) records the probes and their exact results. Two claims in the
original draft were refined by that pass: candidates A and B are **already
constructible today with zero new syntax** (their real gap is catch-selection
syntax, not construction), and nominal cross-module variant identity **already
works** (no textual-name collision at runtime).

Authorities consulted (repository evidence, not chat memory):

- [`docs/engineering/EXCEPTION_ARCHITECTURE.md`](./EXCEPTION_ARCHITECTURE.md)
  — DESIGN-ONLY; §2 levels and §7 security are commitments, §3 taxonomy is
  non-normative, §8 lists E1–E6.
- [`docs/LANGUAGE_SPEC.md`](../LANGUAGE_SPEC.md:1497) §14.5–14.6 — normative
  `throw`/`try`/`catch`/`finally` today.
- [`src/run/mod.rs`](../../src/run/mod.rs:189) `enum Ctl` — `Val/Return/Break/Continue/Throw`.
- [`src/run/iterative.rs`](../../src/run/iterative.rs:223) `enum TryResult`
  (`Body/Caught/Fatal`) and `pending_throw` cross-frame recovery.
- [`src/run/value.rs`](../../src/run/value.rs:145) `enum Value` — the runtime value set.
- [`src/error.rs`](../../src/error.rs:256) `mod codes` — 46 constants, 45 distinct codes.
- [`docs/engineering/DIAGNOSTIC_TAXONOMY.md`](./DIAGNOSTIC_TAXONOMY.md)
  §7 — diagnostics vs exceptions are separate mechanisms.
- [`docs/engineering/CAPABILITY_MODEL.md`](./CAPABILITY_MODEL.md)
  §1 — single `Host` authority boundary.
- [`docs/engineering/EMBEDDED_PYTHON_ARCHITECTURE.md`](./EMBEDDED_PYTHON_ARCHITECTURE.md)
  §9 — P1–P5, separately human-gated (not touched here).
- [`docs/rfcs/README.md`](../rfcs/README.md) — RFC index empty; no accepted
  exception RFC.
- [`HUMAN_DECISIONS_QUEUE.md`](../../HUMAN_DECISIONS_QUEUE.md:3) — only HD-1…HD-4
  resolved (ADR-0001…0004); no exception decision.
- [`docs/engineering/CURRENT_HANDOFF.md`](./CURRENT_HANDOFF.md:512)
  — "Pre-0.3 decisions awaiting the human: exception syntax/catching (E1–E6)".

---

## 1. Why this is a hard gate (Outcome A)

Every authority that could authorize exception syntax is in a **not-approved**
state:

| Source | State |
|---|---|
| `EXCEPTION_ARCHITECTURE.md` header | "DESIGN-ONLY. No public syntax is chosen here." |
| `EXCEPTION_ARCHITECTURE.md` §8 | E1–E6 listed as "requiring human authorization" |
| `CURRENT_HANDOFF.md` §Exact next action | E1–E6 named as "awaiting the human" |
| `HUMAN_DECISIONS_QUEUE.md` | exception decision absent (only HD-1…HD-4) |
| `docs/rfcs/` | no accepted RFC |
| `docs/adr/` | ADR-0001…0004 unrelated to exceptions |

Therefore, per the transaction's §8 and §27, **no public-syntax implementation
may proceed.** The correct terminal state is a clean STOP with this decision
package.

---

## 2. Current exception semantics (reconstructed and verified)

**Surface (normative, [`LANGUAGE_SPEC.md`](../LANGUAGE_SPEC.md:1497) §14.5–14.6):**

- `throw e` raises the value of `e` as a *throwable*.
- `try { … } catch x { … }` catches a throwable raised in the body, **including
  through a called function**, and binds `x` to the thrown value.
- The catch binding is followed directly by its block; `catch e -> { … }` is a
  parse error (`E1006`), per [`tests/catch_syntax.rs`](../../tests/catch_syntax.rs:100).
- `catch` is **mandatory**; there is no `try` without `catch` (`E1006`).
- **Only explicit `throw` is catchable.** Runtime diagnostics are not values and
  are not catchable; they propagate and terminate with their diagnostic.
- An uncaught throwable terminates with `E4026` (`THROWN`/`FOREIGN` = `4026`).
- `finally` runs exactly once on every exit path and its control-flow signal
  replaces the pending outcome; a fatal error inside `finally` supersedes.

**Runtime representation (verified):**

- The catchable signal is `Ctl::Throw(Value)` — the payload is **any** `Value`
  ([`src/run/mod.rs:189`](../../src/run/mod.rs:189)).
- Inside `try`, the machine partitions outcomes via
  `TryResult::Body | Caught | Fatal(Diag)`
  ([`src/run/iterative.rs:223`](../../src/run/iterative.rs:223)); only `Body` +
  `Ctl::Throw` is catchable, `Fatal(Diag)` is not.
- Cross-frame throws are recovered through `interp.pending_throw` with `E4099`
  as the internal crossing signal (never user-visible).
- `Cont::TryCatchEnd`/`Cont::TryFinally` snapshot `frames_len`/`depth`/
  `saved_expr_depth` so an error in a catch/finally body cannot leak frames.

**Conclusion:** today's "exception" is an **untyped thrown value**. There is no
family, no nominal identity, no structured message/payload separation, and no
type-based catching. Any 0.3 family model must be **additive** to this.

---

## 3. Failure-level inventory and catchability matrix

Five levels, verified against `EXCEPTION_ARCHITECTURE.md` §2 and `error.rs`:

| Level | Producer | Catchable today | Codes | Proposed 0.3 |
|---|---|---|---|---|
| Compile/check diagnostic | lexer, parser, checker, module graph | No (no program runs) | `E1xxx`–`E3xxx`, `E2020`–`E2022` | **still no** |
| Runtime diagnostic | evaluator value ops | No (fatal) | `E4xxx` (`E4007`,`E4013`,`E4018`,`E4019`,`E4029`,`E4028`) | **still no** |
| Catchable language exception | explicit `throw` | **Yes** | `E4026` only if uncaught | Yes (family-extended) |
| Host/resource failure | host boundary, limits | No (fatal) | `E4011` frames, `E4020` I/O+capture, `E5001`/`E5002` | **still no** |
| Internal interpreter failure | invariant violation | **Never** | `E4999` (+ internal `E4099`) | **never** |

**Non-negotiable (level 2, 4, 5 must not become catchable):** `E4011`, `E4020`,
`E5001`, `E5002`, `E4999`, and all `E1xxx`–`E3xxx`. A catch handler must not be
able to observe or suppress host policy, resource enforcement, capability
denial, or internal corruption.

---

## 4. Proposed architecture constraints (commitments, not syntax)

These restate existing behavior and safety; they do not require a language
decision:

1. **Additive model.** Introduce an internal throwable shape
   `{ family: Tag, message, payload: Value, location }` behind the existing
   `Ctl::Throw(Value)`. A plain thrown value maps to a wildcard family so
   **today's programs are byte-identical**.
2. **One family registry.** Builtin families mirror diagnostic meanings (one
   table; no duplicated semantics). Codes stay `E<n>`; families are **names/tags**,
   never colliding with the numeric code space.
3. **Non-catchability preserved.** Level 2/4/5 remain `TryResult::Fatal(Diag)`.
4. **Iterative-native only.** Family matching, binding, and propagation are
   implemented in the explicit-continuation machine; **no** recursive-evaluator
   fallback and **no** AST-depth-proportional Rust recursion.
5. **Stack safety.** A throwable crosses frames/modules via continuation state,
   not host recursion.
6. **Bounds.** Payloads obey `MAX_VALUE_DEPTH`/`MAX_VALUE_NODES`/render budgets.
7. **WASM parity.** Family matching and propagation are pure evaluator behavior;
   no host dependency; zero-import contract preserved.
8. **Diagnostics are not exceptions.** No `E####` becomes catchable by being
   "converted"; the family mechanism maps onto the explicit-`throw` value path.
9. **Python boundary unchanged for now.** Foreign `py_*` failures stay
   `E5001`/`E5002` unless E5 is approved; P1–P5 untouched.

---

## 5. Syntax candidates (evaluated, none approved)

Reconstructed from `EXCEPTION_ARCHITECTURE.md` §4. All are non-normative.

### Candidate A — tagged value, purely additive to the grammar

```aura
throw ValueError { message: "bad input", payload: 42 }
try { work() } catch e { if e.family == "value" { ... } }
```

- **Grammar ambiguity:** none; `throw <expr>` already accepts any expression.
  **Verified: A is already constructible today with zero new syntax** (a struct
  declaration plus a struct literal). The syntax shown is current Aura, not a
  proposal.
- **Readability:** medium — `family` is resolved dynamically.
- **Checker complexity:** low at parse, medium if family is checked statically.
- **Runtime complexity:** low (tag on the throwable).
- **Custom exceptions:** user defines a struct; no new declaration syntax.
- **Family identity:** **struct equality is nominal** — `AErr {f:"x"} ==
  BErr {f:"x"}` is `false`, same-type equal fields is `true` (executed). But
  structs have **no pattern syntax** (`match` rejects `P { x } -> ...` and
  `P(x) -> ...` with `E1006`/`E3002`), so discrimination is limited to
  field/string comparison (`e.family == "value"`); a wrong-field read is
  `E2003`. Two same-shape structs are distinguishable only if the program
  carries a discriminator field by convention — not enforced.
- **Tooling/WASM:** easy; pure value.
- **Back-compat:** fully additive.

### Candidate B — typed throw with a name (declaration implied)

```aura
throw MyError("bad input", 42)
```

- **Grammar ambiguity:** `Name(args)` already parses as an enum-variant
  construction (verified: `throw MyError("bad", 42)` works today when
  `MyError` is an enum variant). The checker does not need to reinterpret
  call-like syntax; the resolver/checker already distinguish variant
  constructors from functions (`E3002` names the undeclared case).
- **Readability:** high.
- **Checker complexity:** low — the variant machinery already exists.
- **Custom exceptions:** a declared enum variant; zero-payload variants are
  constructed with `A()` (bare `A` is `E2003`, `LANGUAGE_SPEC.md` §18.2).
- **Family identity:** **nominal and robust today.** Variant patterns match by
  canonical tag; the resolver prefixes module paths (`shapes::Color::Red`), and
  duplicate tags *within one scope* are rejected (`E2013`, "variant names must
  be unique across the program"). Two same-named variants in different modules
  are correctly distinguished — executed in both directions, including sibling
  modules each declaring `enum MyError { Bad(string) }` and
  `enum Shared { Dup(int) }`, and a rethrow preserving identity across nested
  handlers.
- **Tooling/WASM:** moderate.
- **Back-compat:** additive if `Name` is a declared error type.

### Candidate C — declaration + construction (nominal)

```aura
struct MyError : Exception { code: int }
throw MyError { code: 7 }
```

- **Grammar ambiguity:** introduces type-inheritance syntax (`: Exception`) and
  interacts with the struct/enum/type system and the checker. **Verified:
  `struct X : Y` is rejected today with `E1006`** (`expected {, found :`), so C
  requires new grammar.
- **Readability:** high.
- **Checker complexity:** **highest** (nominal family rules, exhaustiveness).
- **Custom exceptions:** first-class; extensible families.
- **Family identity:** nominal (structural identity is robust, not string-based).
- **Tooling/WASM:** highest complexity.
- **Back-compat:** additive, but a large surface.

### What is actually missing today (verified, executed)

The gaps are narrower than the original draft implied. Construction and nominal
identity already exist for B; A is expressible but weakly typed. The real gaps:

1. **Catch-selection syntax.** `catch Name as e`, `catch e: Type`, and
   `catch e if ...` are all `E1006`. `catch` binds an untyped value and there is
   no by-family catch clause. Family dispatch is *possible* today only through
   the verbose pattern `try { ... } catch e { match e { Variant(...) -> ...,
   _ -> { throw e } } }` wrapped in an outer `try` — executed and working
   (see §9), but not a direct language feature.
2. **Raise-site span.** `Ctl::Throw(Value)` carries no span. An uncaught throw
   reports `E4026` at the first frame-crossing call site, not at the `throw`
   expression (executed: a 3-frame chain `main -> c -> b -> a` reported the
   `a()` call in `b`, not the throw inside `a`); a caught throw exposes no
   location to the handler at all.
3. **No builtin family registry.** There is no mapping from runtime
   diagnostics (`E4xxx`) to catchable families, by design (level separation).
4. **`throw` is a statement**, not an expression (`throw_stmt = "throw" expr
   statement_end`, `LANGUAGE_SPEC.md` §14.5; a match arm cannot be a bare
   `throw`). Rethrow works only as a statement inside a block arm.

### Decision matrix

| Criterion | A (tagged struct) | B (enum variant) | C (nominal decl) |
|---|---|---|---|
| New syntax needed for construction | none | none | new (`: Base`) |
| New syntax needed for catch selection | none/if-chain | none/match (verbose) | new |
| Grammar ambiguity | none | none (already a construct) | high (inheritance) |
| Readability | medium | high | high |
| Checker complexity | low | low | high |
| Runtime complexity | low | low | low–med |
| Custom exceptions | struct reuse | enum variant | first-class |
| Identity robustness | medium (nominal equality; no patterns) | **high (nominal, module-qualified, verified)** | high (nominal) |
| Payload access | field read (checked) | variant pattern binding | field/pattern |
| Raise-site span | missing | missing | missing |
| WASM parity | easy | easy | moderate |
| LSP/tooling | easy | easy | harder |
| Exhaustiveness | none | possible over one enum | possible |
| Back-compat | additive | additive | additive |

**Architect recommendation (advisory, not binding).** The executed evidence
narrows the choice: a **minimal enum-based family model** (B's construction and
nominal matching, which already work) plus a **new catch-selection clause** is
the smallest coherent step; it needs no inheritance and no new construction
grammar. A single built-in `AuraError` enum (or one enum per family) declared
in the prelude could carry builtin families, with user exceptions as additional
enum variants or user enums distinguished by module. Candidate C's inheritance
adds the most surface for the least additional capability and should be
rejected unless exhaustiveness across an open hierarchy is required.
**This recommendation requires human ratification.**

---

## 6. Decisions requiring human authorization (E1–E6)

| # | Decision | Options | Why gated |
|---|---|---|---|
| E1 | Constructing a structured exception | A / B / C | new public syntax |
| E2 | Catching by family | `catch e if e.family == X` / `catch Name as e` / typed catch / match | new public syntax |
| E3 | Family identity model | nominal types / tags / both | type-system semantics |
| E4 | Any runtime diagnostic family becomes catchable | proposal: **no** | breaking compat / security |
| E5 | `py` exceptions gain an Aura family mirror | yes / no | cross-language guarantee |
| E6 | Reserved-name policy for builtin family names | reserve / plain names / import | ADR-0002 interaction |

Until E1–E6 are answered, the architecture constrains *how* exceptions may be
added, not *whether* or *when*. **No exception grammar, parser, AST, checker, or
runtime family code may be implemented before this gate is resolved.**

---

## 7. What the human is asked to authorize

1. Resolve **E1–E6** (a single written decision is sufficient; an ADR is preferred).
   Because the resolution adds public syntax, `CONTRIBUTING.md` and
   `docs/rfcs/README.md` require an accepted **RFC** (`docs/rfcs/NNNN-*.md`)
   before implementation: motivation, exact grammar delta, migration, and
   enforcement tests. A human decision that selects a syntax should authorize
   that RFC.
2. If syntax is approved, authorize the vertical slice (spec → lexer → parser →
   AST → checker → iterative runtime → host/foreign → CLI/REPL/library → WASM →
   playground → docs → tests), to be implemented in Code mode under the
   transaction's §8 vertical-slice and §23/§24 git rules (local commits only, no push).
3. If syntax is **not** approved, this transaction ends at Outcome A with only
   the reconstruction + architecture + this decision package delivered.

---

## 8. Immediate pre-requisite (Code mode) — SATISFIED 2026-10-06

The mandatory bootstrap stop gate (§1 of the transaction) was executed in Code
mode on 2026-10-06 and passed with no STOP condition:

```text
branch:               rewrite/v3-rust
local HEAD:           1102b23130a6334a81a5331e391f35bd82ba8782
remote (ls-remote):   e576238f7adb52f6ab6d18431602e3fbc2bd3636
ahead/behind:         15 / 0   (range e576238..1102b23 intact)
staged:               none
unstaged/untracked:   documented protected `.kilo/**` churn only; untracked
                      `plans/` (this document, before relocation)
tags:                 v0.0.1, v0.0.2, v0.2.0, v0.2.1 (v0.2.1 = 3f5f8702…)
declared version:     0.2.1 (Cargo.toml and `aura version`)
protected state:      `.codex/**` absent from git; `.kilo/**` unchanged from
                      the documented churn
frozen artifacts:     0.0.2 / 0.2.0 / 0.2.1 SHA-256 byte-identical
```

The Pre-0.3 adversarial audit (`e576238..1102b23`) then ran read-only. One
in-range defect was found and repaired in a separate local commit: a merged
doc-comment delimiter in `tests/host.rs` introduced by `2fd1433` (commit
`150befe`, one character-class change, behavior identical). No production
defect was found; every audited claim was reproduced and corroborated (routing,
non-catchability, single-sourced limits, frozen hashes, native/WASM parity).

---

## 9. Executed-evidence appendix (2026-10-06)

All probes ran against `target/debug/aura` built from the audited range (plus
`150befe`). Selected results, verbatim:

| # | Probe | Result |
|---|---|---|
| 1 | `try { throw "boom" } catch e { print("caught: " + e) }` | `caught: boom` |
| 2 | `try { print(1 / 0) } catch e { ... } finally { ... }` | `E4007` fatal; catch skipped; finally ran |
| 3 | `try { print(x[99]) } catch e { ... }` | `E4019` fatal; finally ran |
| 4 | recursive `f(n)` under `try` | `E4011` fatal; finally ran |
| 5 | uncaught `throw "unhandled"` | `E4026: uncaught value: unhandled`, exit 1 |
| 6 | `py_eval("1/0")` under `try` | `E5001: python: ZeroDivisionError`; fatal |
| 7 | `py_eval` without `py` feature | `E5002` capability diagnostic; fatal |
| 8 | `range(0, 11000001)` materialized in a comprehension | `E4013` fatal; finally ran |
| 9 | throw + rethrow a list payload through nested handlers | identity and payload preserved (`len` 3 → 3) |
| 10 | `throw ValueError { message, payload }` (struct) | caught; fields readable; **no new syntax** |
| 11 | `throw MyError("bad", 42)` (enum variant) | caught; `MyError("bad", 42)` displays |
| 12 | `match e { Errors::ValueError(m) -> ..., Errors::NetError(n) -> ..., _ -> ... }` | nominal dispatch works |
| 13 | two same-named variants in sibling modules, both directions | correctly discriminated (module-qualified) |
| 14 | same-shape structs `AErr`/`BErr` equality | `false` (nominal); same-type `true` |
| 15 | struct pattern `P { x } ->` / `P(x) ->` | `E1006` / `E3002` (no struct patterns) |
| 16 | `struct X : Y` | `E1006` (no inheritance; C needs new grammar) |
| 17 | `catch Name as e`, `catch e: Type`, `catch e if ...`, `catch e ->` | all `E1006` |
| 18 | duplicate variant tag in one scope | `E2013` |
| 19 | 3-frame uncaught throw | `E4026` at frame-crossing call site, not the throw |
| 20 | catch-all + match + rethrow + outer catch (family dispatch today) | works (verbose; requires nested `try`) |
| 21 | variant identity across rethrow | preserved (`E::A(7)` → `n = 7`) |

Supporting validation (canonical configurations): all-features 52/52 suites;
`--no-default-features --features cli,repl,json,regex,time` 52/52; evaluator
oracle 228/228 differential; fresh-WASM boundary 63/63; browser 66; worker 12;
multi-file 42; cache 7; syntax conformance 43/43 zero imports; MSRV
`+1.83.0 check` green; nightly fuzz `check` green; Miri 117/117; four fuzz
targets ~890k runs clean; website build and tests green.

Known pre-existing hygiene gap (out of scope, recorded as TD-20): bare
`cargo test --all-targets --no-default-features` fails 13 tests in 8 suites
because tests call feature-gated builtins (`json`/`regex`/`time`/`py`) without
`#[cfg]` gates. Every failing file is unchanged since remote `e576238`; the
authoritative validation floor and CI always pass explicit feature lists.
