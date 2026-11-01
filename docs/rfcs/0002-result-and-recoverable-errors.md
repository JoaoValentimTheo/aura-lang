# RFC 0002 — `Result` and Recoverable Error Semantics

- **Status:** Proposed — reconstructed under Directive 008 G02. **Not accepted.
  One decision (D1) is escalated and must not be resolved by the agent.**
- **Scope:** language surface (type identity, construction, tag scope) and
  checker obligation; additive except where D1 says otherwise
- **Backlog:** `NBL-010` (this RFC), `NBL-030` (implementation, G03)
- **Authorities:** `docs/LANGUAGE_SPEC.md` §3.3 (reserved words, builtin
  reservation, `:191`), §18 (enums, tag identity: `:2401`, uniqueness rule
  `:2407`, `:3281`), §19.2 (`match`, no exhaustiveness: `:2471-2472`), §19.3
  (patterns, `:2474`), §14.5–14.6 (`throw`/`try`/`catch`/`finally`, `:1720`,
  `:1754`); RFC 0001; `docs/engineering/NEBULA_CAMPAIGN.md` §8.1
- **Human decisions:** **one — D1 (§2.1).** The tag-scope choice is a
  program-wide, irreversible reservation of two variant names and must be made
  by a human under `NEBULA_CAMPAIGN.md` §2.5.

## 0. Provenance

Directive 008 names `AQL_Nebula_RFC_Draft.md` and
`Aura_0.3.2_Nebula_Inventario_Geral.md` as inputs. **Both are absent from this
repository** (campaign doc §0.4). This RFC is reconstructed from the directive
text plus repository evidence. Its presence implies **no approval**, and no gate
may treat it as already agreed.

§1.1 records four probes that **falsified earlier drafts of this RFC**. The
earlier claims are named in §1.2 and are not repeated as facts anywhere below.

## 1. Motivation

Aura already has a complete exception mechanism: `throw`, a mandatory `catch`
clause, optional `finally`, nominal exception identity, and a rule that only
explicit `throw` is catchable (RFC 0001, E1–E6). That mechanism is the right
tool for defects, invariant violations, and programmer error.

It is the wrong tool for *expected absence and expected refusal* — a lookup
that finds no row, a parse that rejects malformed input, a policy that denies an
operation. Today those cases have only two shapes, both bad:

```aura
// Shape A: sentinel. The type cannot distinguish "no user" from "user id 0".
fn find_user(id: int) -> User {
    if id == 0 { return User { id: 0, name: "" } }
    // ...
}

// Shape B: throw for a non-exceptional outcome. Callers must write try/catch
// for a branch that is part of normal control flow.
fn find_user(id: int) -> User {
    if id == 0 { throw NotFound::Missing(id) }
    // ...
}
```

Shape A makes absence unrepresentable and pushes the mistake to the caller.
Shape B makes ordinary control flow look like failure, which is what §2.3's
"consistent and recoverable error semantics usable by APIs" is meant to
prevent. The campaign requires a `Result` whose branches are **values**, not
throws.

### 1.1 Probes (all run with the built binary, in isolated single-file dirs)

Every probe below lives in its own directory. **This is mandatory**: `aura
check` and `aura run` eagerly compile every eligible sibling `.aura` file, so a
shared directory contaminates results (`docs/FILESYSTEM_MODULES_DESIGN.md:399`,
`:774`; campaign doc §7.2).

**P1 — a user-declared generic `Result` with `Ok`/`Err` already works.**

```console
$ aura run p.aura
7
$ echo $?
0
```

Source: `enum Result<T, E> { Ok(T), Err(E) }`, then
`let r = Ok(7)`, matched with `Ok(v) -> { print(v) }` / `Err(_) -> { print(0) }`.
Bare, unqualified tags work because no module wraps the enum.

**P2 — variant tags are unique *per module*; `E2013` fires within one scope.**

```console
$ aura check q.aura       # enum Result { Ok(T), Err(E) } plus enum MyThing { Ok(int) }
q.aura:2:16: error[E2013]: variant `Ok` is already declared by enum `Result`;
             variant names must be unique across the program
$ echo $?
1
```

**The message text is wrong, and so is the clause it quotes.** The checker keys
`self.variants` by tag within a single resolution scope, so `E2013` fires for
two tags in the *same* module — never across modules. The diagnostic string
"unique across the program" is a stale literal that contradicts both the
implementation and three normative passages of the spec (see §1.1.1).

Same-module duplicate, including inside a nested module (both rejected):

```console
$ aura check dup.aura      # module M { enum A { Ok(int) }  enum B { Ok(string) } }
dup.aura:3:12: error[E2013]: variant `M::Ok` is already declared by enum `M::A`; variant names must be unique across the program
$ aura check dup2.aura     # enum A { Ok(int) }  enum B { Ok(string) }   (root scope)
dup2.aura:2:10: error[E2013]: variant `Ok` is already declared by enum `A`; variant names must be unique across the program
```

Both transcripts are reproduced exactly, including the message tail. Note that
the trailing clause asserts the programme-wide rule **in the very sentence that
reports a module-qualified tag** — the contradiction is visible in a single line
of output.

Cross-module duplicate, in every arrangement probed (all exit `0`):

| Arrangement | Result |
|---|---|
| `enum R { Ok(int) }` at root **+** `module M { Ok(string) }` | `0` |
| `module M { Ok }` declared **before** a root `enum R { Ok }` | `0` |
| sibling `module M { Ok }` and `module N { Ok }` | `0` |
| nested `module M { enum A { Ok } module N { enum B { Ok } } }` | `0` |
| root `Ok(1)` and `M::Ok("s")` both constructed in one program | `0`, prints `Ok(1) M::Ok("s")` |

### 1.1.1 A spec-internal contradiction on tag scope

`docs/LANGUAGE_SPEC.md` states both rules, normatively, in different sections.

**Programme-wide** (the two passages RFC 0002 originally relied on):

- `:2407` §18.1 — "A variant tag MUST be unique **across the whole program**
  (`E2013`); tags are global."
- `:3281` §26 — "declaring two variants with the same tag anywhere is `E2013`."

**Per-module** (three passages, including the frozen-design list):

- `:3399` §27 — "**Normative rule (variant scope, CONF-RESOLVE-6).** Variant
  tags are unique **per module**. Two modules may each declare an `Ok`. Within
  one module, conflicting tags remain invalid. The canonical variant identity is
  the module path plus the tag."
- `:1556` §12 — "Variant tags are unique **per module**, and the canonical tag
  is the module path plus the tag …"
- `:3897` §33.14 — "**Enum variant tags are unique per module**, not globally.
  … so two modules may each declare an `Ok`."

**The implementation agrees with the per-module reading**, and the mechanism is
explicit: `src/resolve.rs:1452-1455` rewrites each variant tag with the module
prefix —

```rust
// Variant tags are canonicalized so two modules may each declare a
// variant with the same local name without collision (`E2013`).
tag: join(prefix, &v.tag),
```

— and the canonical diagnostic proves the prefixing is live: the same-module
case above reports the tag as `M::Ok`, not `Ok`.

So `:2407` and the `:3281` clause are **incorrect**, `E2013`'s message text is
incorrect, and `:3399`/`:1556`/`:3897` describe the shipped behaviour. This is a
specification defect, not a design choice, and **this RFC does not resolve it**;
§2.1 records the consequence for D1.

**This is not a new discovery in this repository, and this RFC does not claim
it as one.** `docs/engineering/EXCEPTION_SYNTAX_DECISION_PACKAGE.md:192`
(verification pass 2026-10-06) already records the same finding with its own
bidirectional execution evidence: "the resolver prefixes module paths
(`shapes::Color::Red`), and duplicate tags *within one scope* are rejected
(`E2013` …). Two same-named variants in different modules are correctly
distinguished — executed in both directions". That package's scope was the
exception system, so the contradiction in `LANGUAGE_SPEC.md` was noted there but
never raised as a specification question.

What is new here is narrower and worth stating precisely: the **two normative
clauses that disagree were never reconciled**, and `E2013`'s message still
repeats the losing side. The per-module *behaviour* is settled and tested; the
*wording* of the specification and of the diagnostic is not.

**P3 — a tag in one module does not collide with the same tag in another.**

```console
$ aura run t.aura         # module M { pub enum Thing { Ok(int) } } + top-level Result{Ok,Err}
1
$ echo $?
0
```

The module-scoped `Ok` and the top-level `Ok` coexist. `E2013` is therefore a
rule about *tag position in the namespace tree*, not about spelling globally —
which is exactly what `:3399`/`:1556`/`:3897` say and `:2407`/`:3281` deny.

**P4 — a module-scoped tag is never reachable unqualified.**

```console
$ aura run u.aura         # module Std { pub enum Result<T,E>{Ok(T),Err(E)} }; use Std::Result; Ok(7)
u.aura:6:34: error[E3002]: `Ok` is not a declared struct or enum variant
$ echo $?
1
```

Qualified forms that **do** work (both verified, printing `7`, exit `0`):

```aura
module Std { pub enum Result<T, E> { Ok(T), Err(E) } }

fn main() {
    let r = Std::Result::Ok(7)        // fully qualified
    // or, after `use Std::Result`:
    // let r = Result::Ok(7)          // enum-qualified
    match r { Std::Result::Ok(v) -> { print(v) }, Std::Result::Err(_) -> { print(0) } }
}
```

A selective variant import is **not** supported:
`use Std::Result::{Ok, Err}` → `E1006: expected module path segment, found '{'`.

### 1.2 Claims this RFC retracts

Three claims in earlier drafts of this document were false. They are recorded
so they are not reintroduced:

1. **"`Result` is declared in the prelude."** There is no Aura prelude.
   `grep -rn prelude src/ docs/LANGUAGE_SPEC.md` returns exactly one hit,
   `pyo3::prelude` at `src/bridge/mod.rs:51`. There is no `std/` directory, no
   `std::` in the spec, and `src/stdlib/mod.rs:20` derives `builtin_names()`
   from `signatures::builtins()` — **functions only**. No mechanism injects a
   builtin *type*: `grep -rn 'builtin_type|BUILTIN_TYPE|prelude_enum|inject_enum' src/`
   returns nothing.
2. **"Exhaustive matching is enforced by the existing checker."** It is not.
   `docs/LANGUAGE_SPEC.md:2471-2472`: "There is **no exhaustiveness
   requirement**. If no arm matches, the result is `E4029` at runtime." A
   non-exhaustive `match` over a `Result` compiles (probe: exit `0`) and fails
   only when it runs (probe: `E4029: no match arm matched the value`, exit `1`).
3. **"A local `Result` shadows a prelude one."** It cannot, but *not* for the
   reason first given here. Shadowing is a *variable* rule —
   `LANGUAGE_SPEC.md:3282` says a variable binding "shadows rather than
   redeclares" — not a tag rule, so "shadowing" never applied to tags at all.
   The real obstacle is narrower than originally stated: an in-scope duplicate
   of the tag `Ok` with the prelude `Result` is `E2013`, but a duplicate in a
   *different module* is not (§1.1.1). The original wording of this item claimed
   programme-wide uniqueness and was wrong.
4. **"Variant tags are unique across the whole program."** False as stated, and
   the error propagated into §2.1's cost analysis. `LANGUAGE_SPEC.md:3399`
   (§27), `:1556` (§12) and `:3897` (§33.14) all specify **per-module**
   uniqueness; the implementation implements per-module uniqueness by prefixing
   tags with the module path (`src/resolve.rs:1452-1455`); probes confirm it
   (§1.1.1). Two clauses — `:2407` (§18.1) and `:3281` (§26) — say the opposite,
   and `E2013`'s message text repeats the wrong one. **The specification
   contradicts itself.** This RFC records the defect and escalates it; it does
   not pick a winner.

## 2. Decision

| # | Decision | Rationale |
|---|---|---|
| R1 | `Result<T, E>` is an ordinary generic enum with variants `Ok(T)` and `Err(E)`. No lexer, parser, or checker change is needed to *declare* it. | P1: already works today. |
| R2 | Variant spellings are `Ok` and `Err`, per Directive 008 §2.4/§3.2. | Matches the campaign's canonical vocabulary; `Err` is verified constructible (P1). |
| R3 | `E` is any type. It is **not** constrained to nominal exception types. | A policy denial or parse failure need not be throwable. |
| R4 | `Result` is matched with the existing `match` and `->` arms. No new destructuring form and **no exhaustiveness requirement is added**. | Reuses §19.4 as it stands; see D2. |
| R5 | Absence is `T \| none`, **not** `Result<T, E>`. | Two distinct concepts must not collapse. |
| R6 | Evaluating a `Result` as a bare statement and discarding it is a checker error. | A silently ignored failure is the defect `Result` exists to prevent; see D3. |
| R7 | `Result` crosses module boundaries as an ordinary value; no special-casing in `pub use` or re-export. | No new resolution rule. |
| **D1** | **Where the type lives — and therefore which tag spelling callers write — is escalated, not chosen.** | P2 vs P3/P4: the two options have materially different costs. |

### 2.1 D1 — the escalated decision (tag scope)

This is the only substantive choice in this RFC, and it is a *language-wide,
effectively permanent* commitment. Both options are fully viable; the agent does
not choose between them.

**Option A — top-level `Result`, bare `Ok(7)` / `Err(e)`.**

Exactly the spelling Directive 008 §2.4 mandates. Cost: `Ok` and `Err` become
**unusable as variant tags in the root scope of any program that mentions
`Result`**. A user writing `enum Parsed { Ok(int) }` at the root gets `E2013` —
but only if their `Ok` is also at the root; a module-scoped `enum Parsed { Ok }`
coexists fine (§1.1.1). So the original framing of this cost — "permanently
unusable in any user enum in the same compilation unit" — **overstated it**.

Two honest residual costs remain, and they are smaller but real:

- the rule is **scope-dependent, not import-dependent**, so a root-level
  `enum Parsed { Ok(int) }` breaks because `Result` merely *exists*, even if
  the program never constructs one — a coherence surprise rather than a
  usability wall;
- `E2013`'s current message says "unique across the program", so the diagnostic
  a user receives will **misdescribe the rule that rejected their code** until
  the message is fixed (§2.1.1).

Verified neutral fact: **no `.aura` file in this repository currently declares
an `Ok` or `Err` variant tag**
(`grep -rn '\bOk\b\|\bErr\b' --include=*.aura .` → no matches). The `Ok`/`Err`
occurrences in `tests/syntax_gaps.rs`, `tests/corpus.rs` and `tests/oracle/mod.rs`
are **Rust** `Result` variants in test code, not Aura tags. So Option A breaks
nothing in-tree today; its cost is prospective.

**Option B — module-scoped `Result`, qualified `Ok`.**

```aura
module Std { pub enum Result<T, E> { Ok(T), Err(E) } }
// callers: Std::Result::Ok(7), or Result::Ok(7) after `use Std::Result`
```

Verified to work (P3/P4). Cost: **`Ok(T)`/`Err(E)` are not reachable
unqualified** — every call site carries a qualifier, deviating from Directive
008 §2.4's stated vocabulary. Selective variant import (`use …::{Ok, Err}`) is
unsupported today, so the qualifier cannot be removed without a second grammar
feature.

Neither option is free. Option A reserves two common words for a side effect of
using a library type; Option B pays a permanent qualifier tax at every call site
and needs a follow-on grammar RFC before it can read like the directive's
example.

### 2.1.1 The decision is now partly a spec question, not only a placement one

The A/B choice above assumes a settled tag-scope rule. It is not settled: §1.1.1
documents that `LANGUAGE_SPEC.md` asserts both programme-wide (`:2407`, `:3281`)
and per-module (`:3399`, `:1556`, `:3897`) uniqueness, that the implementation
is per-module, and that `E2013`'s message text claims the programme-wide rule.

That changes what a human is being asked. Three sub-decisions are entangled, and
**the agent takes none of them**:

| # | Question | Why it is not mechanical |
|---|---|---|
| D1a | Which clause is normative — `:2407`/`:3281` (global) or `:3399`/`:1556`/`:3897` (per-module)? | The implementation already implements per-module, so declaring global normative would make the *spec* right and the *compiler* wrong; declaring per-module right makes two normative passages obsolete. Either way a normative document changes. |
| D1b | Should `E2013`'s message be corrected? | It currently asserts a rule the compiler does not enforce (`src/check/mod.rs:1378`). No test matches the message text — `tests/grammar.rs` matches on the code `codes::DUPLICATE_VARIANT`, and `docs/errors.md:37` gives only the code, a title ("Duplicate enum variant tag") and a trigger example, not the message — so the change is low-risk. It is still **not** decidable by the agent, because the correct wording depends on D1a. |
| D1c | Where does `Result` live — Option A or Option B? | Only decidable **after** D1a. Under per-module uniqueness, Option A's cost is a root-scope reservation; under global uniqueness it is a whole-program reservation. The two answers give materially different DX. |

**Recommended sequencing, not a recommendation of outcome:** answer D1a first. It
is a pure specification question with probe evidence already attached, and it
collapses D1c into a normal cost comparison. Answering D1c first risks choosing a
placement whose stated cost is an artefact of the wrong clause.

**Required human input:** D1a (which clause governs), D1b (message wording), then
A or B for D1c — or authorise a third path (e.g. an `@must_use`-style attribute,
currently impossible because `@` is unlexable — `E1001`). Until D1c is answered,
G03 must not implement `Result`.

### 2.2 What is deliberately absent

No `?` operator, no `unwrap`, no `expect`, no `map`/`and_then` chains, no
monadic `do` notation. Those are library conveniences at best and grammar
changes at worst; none is required to satisfy a P0 item. A future RFC may add
them.

## 3. Grammar delta

**None** for R1–R7 under either D1 option. The declaration is an ordinary
`enum` over existing productions, and construction reuses existing call and
`match` syntax. The `try`-as-expression question is separate (RFC 0003).

## 4. Semantics

- `Ok(v)` / `Err(e)` are ordinary enum values under ordinary nominal identity
  and the ordinary tag rules (§18). Under Option A they are bare; under Option B
  they are qualified (§2.1).
- A zero-payload variant still requires `Tag()` (`E2003` if written bare). Not
  exercised by `Result`, which has payloads in both variants, but it applies to
  any user enum used with `Result`.
- `match` over a `Result` is **not** required to be exhaustive. A missing
  `Err(_)` arm compiles and produces `E4029` at runtime
  (`LANGUAGE_SPEC.md:2471-2472`). See D2 — this RFC does not change that.
- A `Result` may be stored in a field, placed in a collection, returned, and
  passed across the Host boundary as a value.

### D2 — exhaustiveness (explicitly *not* changed)

Making `match` exhaustive over `Result` would be a **breaking change** to every
existing non-exhaustive `match` in the language, and it is not required by any
P0 item. This RFC records the current rule as-is. If the campaign wants
exhaustiveness, it needs its own RFC and a compatibility plan; it must not be
smuggled in as an implementation detail of `Result`.

### D3 — R6 is a special case, stated honestly

Discarding **any** expression as a statement is currently legal — verified:
`1 + 2`, a `5`-returning call, and a string literal as bare statements all
compile and run (exit `0`). R6 therefore cannot be expressed as a general
"values must be used" rule without breaking the language. Because Aura has no
attribute mechanism (`@` → `E1001`), R6 can only be implemented as a checker
rule **hardcoded on the type named `Result`**.

That is a real design wart and the RFC says so rather than hiding it. It is
justified only if D1 Option A is chosen; under Option B the type is
`Std::Result` and the rule would key on that path. A general `must_use`-like
mechanism requires the `@` token class (campaign risk R-02) and is out of scope.

## 5. Migration

- **Option A:** programs declaring their own root-scope `Result` with `Ok`/`Err`
  keep working only if those are the *only* root-scope declarations of those
  tags — which they already are, since a duplicate in one scope is `E2013`
  today. Programs declaring `Ok`/`Err` at root scope on a *different* enum name
  and also declaring `Result` would newly fail with `E2013`, **even if they
  never construct a `Result`**. Programs doing this inside a module are
  unaffected (§1.1.1). No in-tree `.aura` file does either.
- **Option B:** no existing program changes, because no unqualified `Ok`/`Err`
  tag exists in-tree and the module form is opt-in.
- **R6 (both options):** can reject a program that compiles today — one that
  evaluates a `Result` and ignores it. Verified as currently permitted. The
  migration is to `match` it or bind it to `_`. This is the only incompatibility
  in this RFC and must be justified in review, not assumed.

## 6. Enforcement

- `tests/rfc0002_result.rs`: declaration, construction, `match`, nesting in a
  struct field and a list, return position, `Ok` with a generic parameter, `Err`
  carrying a nominal exception type. All verified constructible during
  authoring: a `Result` in a struct field, a `[R<int,string>]` list, and a
  `-> R<int,string>` return position each run and print correctly (exit `0`).
- Negative, available today: duplicate `Ok` tag under Option A is `E2013`;
  `Ok` unqualified under Option B is `E3002`; wrong payload arity is `E3001`
  ("variant `Ok` expects 1 value(s), got 2").
- Negative, **not** available and therefore **not claimed**: "non-exhaustive
  match rejected". It is not a current behaviour (§1.2 item 2, D2). A runtime
  test asserting `E4029` from a `Result` with no matching arm is the honest
  substitute.
- Shadowing: no shadowing test, because shadowing a tag is impossible (§1.2
  item 3). A test asserting `E2013` for a second `Ok` declaration is the honest
  substitute.
- Differential: the iterative engine and the recursive oracle agree on every
  case (`tests/evaluator_oracle.rs::engines_agree`).
- Mutation check, both directions per AGENTS rule 6: removing the R6 check must
  make the discard test fail and restoring it must make it pass; separately,
  changing the D1 option must flip the qualification tests between `E3002` and
  `E2013` expectations — if it does not, the option was not actually implemented.
- Every test file must place each Aura source in its **own directory** (§1.1).

## 7. Open items blocking acceptance

| Item | Blocks | Owner |
|---|---|---|
| **D1** tag scope (A or B, or a third path) | all of G03 `Result` work | human (`§2.5`) |
| **D3** `Result`-hardcoded discard rule vs. a general `must_use` | R6 implementation | RFC review; `@` needs R-02 |
| **D2** exhaustiveness, if ever wanted | a separate RFC | not this one |
