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

**P2 — variant tags are unique program-wide, unconditionally.**

```console
$ aura check q.aura       # enum Result { Ok(T), Err(E) } plus enum MyThing { Ok(int) }
q.aura:2:16: error[E2013]: variant `Ok` is already declared by enum `Result`;
             variant names must be unique across the program
$ echo $?
1
```

`docs/LANGUAGE_SPEC.md:2407`: "A variant tag MUST be unique **across the whole
program** (`E2013`)". `:3281`: "declaring two variants with the same tag
anywhere is `E2013`".

**P3 — module scope exempts a tag from programme-wide uniqueness.**

```console
$ aura run t.aura         # module M { pub enum Thing { Ok(int) } } + top-level Result{Ok,Err}
1
$ echo $?
0
```

The module-scoped `Ok` and the top-level `Ok` coexist. So E2013 is a rule about
*tag position in the namespace tree*, not about spelling globally.

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
3. **"A local `Result` shadows a prelude one."** It cannot. P2 shows that a
   second declaration of the tag `Ok` is `E2013` regardless of the enum name.
   Shadowing is a *variable* rule — `LANGUAGE_SPEC.md:3282` says a variable
   binding "shadows rather than redeclares" — not a tag rule.

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

Exactly the spelling Directive 008 §2.4 mandates. Cost: because tags are unique
programme-wide (P2), `Ok` and `Err` become **permanently unusable as variant
tags in any user enum in the same compilation unit**. A user writing
`enum Parsed { Ok(int) }` alongside a program that mentions `Result` gets
`E2013`. A user writing it alongside a program that *does not* mention `Result`
does not — so the rule would appear to depend on unrelated code, which is a
notable DX hazard.

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

**Required human input:** choose A or B, or authorise a third path (e.g. an
`@must_use`-style attribute, which is currently impossible because `@` is
unlexable — `E1001`). Until chosen, G03 must not implement `Result`.

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

- **Option A:** programs declaring their own `Result` with `Ok`/`Err` keep
  working only if they are the *only* declaration of those tags in the
  compilation unit — which they already are, since a second one is `E2013`
  today. Programs declaring `Ok`/`Err` on a *different* enum name and also using
  `Result` would newly fail with `E2013`. No in-tree `.aura` file does this.
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
