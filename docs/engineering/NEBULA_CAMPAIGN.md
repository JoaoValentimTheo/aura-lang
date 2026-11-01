# Directive 008 — NEBULA 0.3.2 Campaign

Status: **G00 PASS**, **G01 PASS** — baseline and prerequisite inventory merged
as `7bd1d6fb` (PR #43, 21/21 checks green); closure and G01 authorisation
merged as `390f763c` (PR #44, 21/21 checks green); the G01 dependency record
merged as `6f075882` (PR #45, 21/21 checks green). **G02 is the only
authorized wave**; no implementation is authorized before G02.

G01 is complete and carries one unresolved item: the §1.5.1 MSRV-versus-drivers
conflict, escalated as a human decision in §10.1. G01 is marked `PASS` because
the gate asks for an approved dependency table, roles, backlog, risks, and
evidence criteria — all delivered — and because a disclosed blocking finding
with a decision package is the required G01 output, not a failure of it. The
decision in §10.1 still gates G07 onward.

G02 (`IN_PROGRESS`) has authored RFC 0002 and RFC 0003 and merged both as
`6cc96bf5` (PR #46, 21/21 checks green). RFC 0002 was then corrected — its
original tag-scope premise was falsified — recorded as `fe0e7312` (PR #47)
and integrated as `73a35d9b` (PR #48, 21/21 checks green). While
validating RFC 0002 it produced a second blocking language finding: the tag
scope that governs `E2013` **makes a prelude or builtin `Result<T,E>` with
unqualified `Ok`/`Err` unviable as things stand** — and, on re-verification,
`LANGUAGE_SPEC.md` turned out to **contradict itself** about that scope
(`:2407`/`:3281` say programme-wide, `:3399`/`:1556`/`:3897` say per module;
the compiler implements per module). See §1.7.1a and risk `R-10`. It is
escalated as a human decision in §10.2 and blocks `NBL-030` plus every
downstream `Result` API.
Like §10.1, it is a decision package, not a gate failure — but it also means the
P0 RFC set cannot be frozen until it is answered.

While validating RFC 0002's evidence, G02 also uncovered a **pre-existing
checker soundness defect** independent of any new syntax: a `try` statement in
tail position already produces a value at run time, but the checker types it as
`none`, and `none` bypasses the implicit-return check that `return none` fails
(§1.7.2, risk `R-11`). A three-part repair was validated end to end — 1477/1477
tests, clippy clean, zero status changes across all 125 tracked `.aura` files,
and every part proven necessary by reverse mutation — and then **withheld**,
because gate G02 forbids coding the affected surface before RFC acceptance.
`src/check/mod.rs` is byte-identical to baseline. The finding and the two
grammar alternatives are escalated in §10.3 and recorded in
`docs/rfcs/0003-try-as-value-expression.md`.

Baseline: `c4f769077c769483963a0f2b51a789b00472e4cf` on `rewrite/v3-rust`
(local == `origin/rewrite/v3-rust`, tree clean at campaign start). G00 landed
on top of it as `7bd1d6fb`; G00 closure landed as `390f763c`.

Published `0.3.1` Keystone and every frozen runtime under
`playground/runtimes/` are immutable. `0.3.2` is **not** released, not
versioned in the changelog, and not promoted. No tag, release, deployment,
or `manifest.current` change is authorized for this campaign.

---

## 0. Authority and provenance

This document is the campaign plan required by Directive 008 §0.5. It records
what was *verified* in the repository, never what was assumed.

Three documents the directive names as inputs do not exist in this workspace:

| Directive reference | Status in this repository |
|---|---|
| `AQL_Nebula_RFC_Draft.md` | **ABSENT** |
| `NEBULA_SQLGUARD_CONTRACT.md` | **ABSENT** |
| `Aura_0.3.2_Nebula_Inventario_Geral.md` | **ABSENT** |

At the baseline commit `c4f76907`, `docs/engineering/` contained no `*NEBULA*`
file and no `*AQL*` file anywhere in the tree, and `docs/rfcs/` contained only
`README.md` and `0001-exception-catch-selection.md`. This document is the first
`*NEBULA*` file to enter `docs/engineering/`, and it is the plan, not one of the
three absent inputs.

Consequence, per AGENTS hard-won rule 16 ("never state a command that you did
not actually run") and the §0.4 provenance requirement: the G02 RFC set must be
**reconstructed from the directive text plus repository evidence**, each RFC
carrying an explicit provenance note stating that its nominal source document
was absent and that no approval is implied by its presence. No gate may treat
an RFC as "already agreed" on the basis of a document that was never here.

Normative authorities remain:

| Question | Authority |
|---|---|
| Language and runtime semantics | `docs/LANGUAGE_SPEC.md`, accepted `docs/adr/*` |
| Grammar | `docs/grammar.md` |
| RFC process | `docs/rfcs/README.md` |
| Filesystem module loading | `docs/FILESYSTEM_MODULES_DESIGN.md` |
| Current repository state | `AGENT_STATE.md` |
| Active task | `docs/engineering/CURRENT_HANDOFF.md` |
| Campaign plan | this document |

There is no `AQL_VERSION` or `SQLGUARD_VERSION` in the tree today. Both are
introduced by this campaign as **own** namespaces, distinct from `AIS_VERSION`,
the Host ABI version, the workspace crate version, and `language_version`.

---

## 1. Verified baseline

Everything in this section was executed or measured, not recalled.

### 1.1 Git and CI state

| Fact | Value |
|---|---|
| Branch | `rewrite/v3-rust` |
| `HEAD` | `c4f769077c769483963a0f2b51a789b00472e4cf` |
| `origin/rewrite/v3-rust` | `c4f769077c769483963a0f2b51a789b00472e4cf` |
| Working tree at start | clean |
| Open pull requests | 0 |

`scripts/agent-state.sh` reported `BRANCH=rewrite/v3-rust`,
`LOCAL_HEAD=c4f76907…`, `REMOTE_HEAD=c4f76907…`, `STAGED=NO`,
`DIRTY=YES` (the single P3 preflight fix described in §1.6),
`WRITER=NONE`, `S_STATE=ABSENT`, `AUTHORITY=OK`,
`PROJECT_CODEX_CONFIG=CLEAN`.

### 1.2 Directive 007 evidence intact

`AGENT_STATE.md` still records the dev.9 parity closure: PR #41 merged as
`ba8204ac`, post-merge CI run `41240576369` green 21/21, Pages deploy
`41240576317` succeeded, `0.3.2-dev.9` identity
2 010 781 B / ABI 2 / sha256 `8de654f0…112b5` / fingerprint `0e252f8f…652b`.
PR #42 landed as `c4f76907` and corrected a flaky-claim in the parity report.

### 1.3 Frozen artifacts unaltered

`playground/runtimes/` holds **47 tracked files**: `manifest.json` plus **46**
`.wasm` artifacts. Only **8** of those 46 are registered in
`manifest.json`; the other **38** are retained historical development builds
(e.g. `0.0.2-dev`, `0.0.2-dev.2` … `0.0.2-dev.29`, `0.2.1-dev.1` … `0.2.1-dev.5`,
`0.3.2-dev.1`, `0.3.2-dev.5`, `0.3.2-dev.7`, `0.3.2-dev.8`) that predate or sit
outside the current registry. `0.0.1` has `"artifact": null` — it predates the
WebAssembly substrate and executes nothing.

All 8 registered artifacts were re-hashed and byte-counted against
`manifest.json`: **8 verified, 0 mismatched, 0 missing**. `git status --short
playground/runtimes` is empty, so no frozen artifact is modified, added, or
deleted. The 38 unregistered historical artifacts were left untouched.

| Identity | Channel | Release | Language | ABI | Bytes | Available |
|---|---|---|---|---|---|---|
| `0.0.1` | release | 0.0.1 | 0.0.1 | — (null) | — (null) | false |
| `0.0.2` | release | 0.0.2 | 0.0.1 | 1 | 1 366 621 | true |
| `0.0.2-dev.30` | development | 0.0.2 | 0.0.1 | 1 | 1 654 216 | true |
| `0.2.0-dev.1` | development | 0.2.0 | 0.2.0 | 1 | 1 767 068 | true |
| `0.2.0-dev.2` | development | 0.2.0 | 0.2.0 | 1 | 1 767 723 | true |
| `0.2.0` | release | 0.2.0 | 0.2.0 | 1 | 1 654 161 | true |
| `0.2.1` | release | 0.2.1 | 0.2.1 | 1 | 1 768 322 | true |
| `0.3.1` | release | 0.3.1 | 0.3.1 | 1 | 1 951 520 | true |
| `0.3.2-dev.9` | development | 0.3.2 | 0.3.1 | 2 | 2 010 781 | true |

`manifest.current` is `"0.3.1"`; `playground_api_version` is `1`; the registry
holds 9 entries. The top-level keys are `playground_api_version`, `current`,
and `versions` — there is no `runtimes` key.

### 1.4 Version namespaces today

| Namespace | Current value | Source |
|---|---|---|
| Crate version | `0.3.1` | `Cargo.toml` |
| Edition | 2021 | `Cargo.toml` |
| MSRV | `1.83` | `Cargo.toml` (`rust-version`) |
| AIS protocol | `0.1` | `src/ais.rs:40` (`AIS_VERSION`) |
| Host ABI | `2` | `playground/runtime/src/lib.rs:66` (`ABI_VERSION`) |
| Playground API | `1` | `playground/runtimes/manifest.json` |
| AQL | **does not exist** | — |
| SQLGuard | **does not exist** | — |

### 1.5 Dependency reality

`Cargo.toml` / `Cargo.lock` contain no `sqlx`, `sqlite`, `rusqlite`,
`postgres`, `mysql`, `mongodb`, or `tokio`. Current optional dependencies are
`serde`, `serde_json`, `regex`, `chrono`, `rustyline`, `pyo3`, `ureq`.
Default features are `cli`, `repl`, `json`, `regex`, `time`, `py`.

Every database, async, and pooling dependency required by G07–G16 is **net-new
supply-chain surface** and must pass `cargo audit` review as part of its wave.

### 1.5.1 G01 dependency study — every candidate driver conflicts with MSRV 1.83

This subsection is the empirical core of G01. Each row is a reproduced command,
not a documentation lookup. Probe crates declared `rust-version = "1.83"`,
resolved a lockfile with Cargo 1.98.1, then compiled with `cargo +1.83.0
check --locked` in a fresh directory.

| Driver | Version probed | Declared MSRV | `cargo +1.83.0` result | Blocking package |
|---|---|---|---|---|
| rusqlite | 0.37.0 (`bundled`) | not declared | **PASS** (exit 0) | none |
| rusqlite | 0.40.2 | 1.85 | FAIL (exit 101) | `hashlink 0.12.2` |
| rusqlite | 0.41.1 (latest) | 1.89.0, edition 2024 | FAIL | `hashlink 0.12.2` |
| tokio-postgres | 0.7.13 | 1.81 (declared) | FAIL (exit 101) | `block-buffer 0.12.1` |
| tokio-postgres | 0.7.18 (latest) | 1.85, edition 2024 | FAIL | `block-buffer 0.12.1` |
| sqlx | 0.7.4 | not declared | FAIL (exit 101) | `icu_provider 2.4.0` |
| sqlx | 0.9.0 (latest) | 1.94.0 | FAIL | `zeroize 1.9.1` |
| mysql_async | 0.31.3 | not declared | FAIL (exit 101) | `getrandom 0.4.3` |
| mysql_async | 0.37.1 (latest) | not declared | FAIL | undeclared transitive |
| mongodb | 3.5.2 | **1.83 (declared and correct)** | FAIL (exit 101) | `getrandom 0.4.3` |
| mongodb | 3.9.1 (latest) | 1.88 | FAIL | `getrandom 0.4.3` |
| libsql | 0.9.24 | not declared | FAIL (exit 101) | undeclared transitive |
| libsql | 0.10.0 (latest) | not declared | FAIL | undeclared transitive |

**The failure is not a version-selection mistake.** It is caused by hard
version requirements inside the dependency graph that cannot be satisfied by
any combination compatible with Rust 1.83:

- `digest` requires `block-buffer = "^0.12"`. Every release in the
  `0.12` line is edition 2024 / MSRV 1.85. The last usable release,
  `block-buffer 0.10.4`, is outside the allowed range, so
  `cargo update -p block-buffer --precise 0.10.4` fails with
  `failed to select a version for the requirement block-buffer = "^0.12"`.
- `icu_normalizer` and `icu_properties` require `icu_provider = "^2.4.0"`.
  `icu_provider 2.4.0` and `2.3.0` are edition 2024 / MSRV 1.88; `2.2.0` needs
  1.86. Only `2.1.0` (1.83) and `2.0.0` (1.82) would work, and both are below
  the required range.
- `uuid` requires `getrandom 0.4`. Every `getrandom 0.4.x` release is edition
  2024 / MSRV 1.85. The workable `0.3.x` line is below the required range.

**Ordinary resume measures do not converge.** Pinning each blocker in turn
reveals another. On a minimal `sqlx 0.7.4` graph (188 packages, 5 of them
edition 2024), pinning `idna_adapter 1.2.1`, `zeroize 1.8.2`, and
`tempfile 3.20.0` successively exposed `icu_provider`, which is unsatisfiable.
Attempting the same rescue for `mongodb 3.5.2` produced a graph that resolved
but compiled with **1638 errors**, because the pinned transitive versions are
mutually inconsistent with the macro crate's expectations.

**The final and most important observation.** `mongodb 3.5.2` declares
`rust-version = "1.83"` and is edition 2021 — the declaration is *correct* and
still the build fails, because MSRV declarations are not enforced across the
transitive graph. This is precisely why `AGENTS.md` rule 12 ("an untested
environment assumption is where a newly wired gate fails first") and the rule
that `rust_version: null` means *undeclared, not compatible* both matter here.

**Consequence for the campaign.** Adding any of these drivers either
regresses the mandated §18.2 floor (`cargo +1.83.0 check --locked
--all-features`, which is green today — reproduced after `touch src/lib.rs`,
exit 0) or requires the driver to be excluded from `--all-features` by
feature-gating, which formally amends the published validation floor. Per
§20.3 this is an MSRV-class impediment and must be recorded and escalated, not
worked around. The available outcomes are enumerated in §10.1 below.

### 1.6 Preflight defect corrected (P3)

`scripts/agent-state.sh` extracted the `## Writer` value with an exact-line
`sed` match. Because `AGENT_STATE.md` records
`NONE (single-writer discipline; …)`, the script reported `WRITER=UNKNOWN` on a
clean tree. This is a trivial, non-semantic P3 tooling defect within the §0.5
allowance. The extraction now matches the leading token only:

```sh
writer=$(sed -n '/^## Writer$/,/^## /{/^NONE/p;/^WEB-GPT/p;}' AGENT_STATE.md \
    | head -n 1 | sed 's/[[:space:]].*//')
```

Observed before: `WRITER=UNKNOWN`. Observed after: `WRITER=NONE`. Writer
*policy* is unchanged; only the parser is. `CURRENT_TRACK`, `CURRENT_PHASE`,
and `AUDIT3` remain exact-line and brittle and were deliberately left alone as
out of scope.

### 1.7 Language-surface probes

Each probe ran in a fresh empty directory and its output was confirmed
byte-for-byte with `od -c`. Probes never reused a prior working directory.

| Surface | Probe | Result |
|---|---|---|
| `@` | source containing a bare `@` | `error[E1001]: unexpected character '@'` — no `At` token exists in `src/lex` |
| `entity`, `relation` | `let entity = 1; let relation = 2; print(entity + relation)` | exact `3\n` — not reserved words |
| `try` as expression | `let x = try { 1 } catch e { 2 }` | `error[E1006]: expected an expression, found \`try\`` |
| generic enum | `enum R<T,E> { Ok(T), Err(E) }` + `R::<int,string>::Ok(7)` + `match` | exact `7\n` — generic enums already work |
| match arms | `pattern -> …` | valid; `=>` is invalid |

`Tok::Try` is reachable only through `stmt_inner` (`src/parse/mod.rs:1979`),
with further uses at `:3249`, `:3285`, `:3733`. `docs/grammar.md:97` defines
`try_stmt = "try" block "catch" pattern block [ "finally" block ]` and
`docs/LANGUAGE_SPEC.md:1720-1781` makes `catch` mandatory, stating that `try`
without `catch` is deliberately absent.

These results define the G03 starting point: `Result` can very likely be an
ordinary generic enum rather than new machinery, while `try`-as-expression must
widen an existing statement-only form without creating ambiguity.

#### 1.7.1 Enum tag scope — the decisive G02 finding

> **CORRECTION (2026-11-01).** The first version of this section asserted that
> tags are unique *programme-wide* and treated that as settled. **It is not
> settled: `LANGUAGE_SPEC.md` contradicts itself.** See §1.7.1a. The probe rows
> below are unchanged and remain accurate; only the *interpretation* was wrong.

`docs/LANGUAGE_SPEC.md:2407` states that a variant tag "MUST be unique **across
the whole program**" (`E2013`), and `:3281` repeats it — but `:3399` (§27),
`:1556` (§12) and `:3897` (§33.14) state the opposite, and the implementation
follows those three. No ADR governs the question.

#### 1.7.1a The specification contradicts itself on tag scope

Both rules are written normatively, in different sections:

| Reading | Clauses | Verbatim |
|---|---|---|
| Programme-wide | `:2407` §18.1 | "A variant tag MUST be unique **across the whole program** (`E2013`); tags are global." |
| Programme-wide | `:3281` §26 | "declaring two variants with the same tag anywhere is `E2013`" |
| **Per-module** | `:3399` §27 | "**Normative rule (variant scope, CONF-RESOLVE-6).** Variant tags are unique **per module**. Two modules may each declare an `Ok`. Within one module, conflicting tags remain invalid. The canonical variant identity is the module path plus the tag." |
| **Per-module** | `:1556` §12 | "Variant tags are unique **per module**, and the canonical tag is the module path plus the tag" |
| **Per-module** | `:3897` §33.14 | "**Enum variant tags are unique per module**, not globally. … so two modules may each declare an `Ok`." |

**The implementation is per-module, and the prefixing is observable.**
`src/resolve.rs:1452-1455` rewrites each tag with the module path:

```rust
// Variant tags are canonicalized so two modules may each declare a
// variant with the same local name without collision (`E2013`).
tag: join(prefix, &v.tag),
```

The diagnostic proves it: a duplicate inside `module M` reports the tag as
`M::Ok`, not `Ok`.

| Probe (one `.aura` file per directory) | Exit | Result |
|---|---|---|
| two enums with `Ok` at root scope | 1 | `E2013` |
| two enums with `Ok` inside the same `module M` | 1 | `E2013`, tag reported as `M::Ok` |
| root `enum R { Ok(int) }` + `module M { Ok(string) }` | 0 | coexist |
| module declared *before* the root enum, same tags | 0 | coexist |
| sibling modules `M { Ok }` and `N { Ok }` | 0 | coexist |
| nested `module M { enum A { Ok } module N { enum B { Ok } } }` | 0 | coexist |
| root `Ok(1)` and `M::Ok("s")` both constructed and printed | 0 | prints `Ok(1) M::Ok("s")` |
| root `enum Result { Ok, Err }` + root `enum Parsed { Ok }` | 1 | `E2013` **even though `Result` is never constructed** |

**`E2013`'s message is also wrong.** `src/check/mod.rs:1378` emits "variant
names must be unique across the program" for a rule the compiler enforces
per-scope. No test matches that string (`tests/grammar.rs` matches the *code*),
and `docs/errors.md:37` records only the code, a title, and a trigger — so
correcting the wording is low-risk, but the correct wording depends on which
clause wins.

**Prior art — this is not a new discovery.** `docs/engineering/
EXCEPTION_SYNTAX_DECISION_PACKAGE.md:192` (verification pass 2026-10-06) already
recorded the per-module behaviour with its own bidirectional execution evidence,
including sibling modules each declaring an `Ok`. That package's scope was the
exception system, so it noted the behaviour without raising the spec
contradiction. What is recorded here as new is only that **the two normative
clauses were never reconciled** and the diagnostic still repeats the losing one.

**Nothing here is decided by an agent.** §10.2 escalates it.

Probes, each in a directory containing exactly one `.aura` file:

| Probe | Source | Result |
|---|---|---|
| Duplicate tag | `enum A { Ok(int) }` + `enum B { Ok(string) }` | `E2013: variant 'Ok' is already declared by enum 'A'`; exit 1 |
| Qualified does not escape | same, with `A::Ok(1)` / `B::Bad(2)` bodies | still `E2013`; uniqueness is on the tag name, not the path |
| Module escapes | `module M { pub enum Thing { Ok(int) } }` + top-level `Result{Ok,Err}` | exit 0 — a module-scoped tag coexists |
| Imported tag is not unqualified | `use Std::Result` then `Ok(7)` | `E3002: 'Ok' is not a declared struct or enum variant`; exit 1 |
| Braced import unsupported | `use Std::Result::{Ok, Err}` | `E1006: expected module path segment, found '{'`; exit 1 |
| Qualified construction works | `Std::Ok(7)`, `Std::Result::Ok(7)`, `Result::Ok(7)` after `use` | all exit 0, print `7` |
| `Ok`/`Err` not reserved | `LANGUAGE_SPEC.md:191` reserved list | absent from it; usable as tags |
| `Err` is constructible | `enum Result<T,E> { Ok(T), Err(E) }` + match | exit 0, prints `7` |
| In-tree corpus unaffected | `grep '\bOk\b\|\bErr\b' --include=*.aura` over 125 files | 0 matches — the `Ok`/`Err` hits in `tests/` are Rust, not Aura |

**Consequence.** A prelude-level or builtin `Result<T,E>` with `Ok`/`Err`
cannot be introduced without one of three choices, none of which the agent may
make. This is recorded as risk `R-10` and escalated in §10.2. Directive 008
§3.2's phrase "apoiado no modelo de `enum`/generics existente **quando viável**"
supplies the lawful exit, but the exit is a decision, not an implementation
detail.

Two further corrections came from the same probe series, and both falsify
plausible assumptions that must not be rebuilt:

- **Discarding any expression is legal today.** `1 + 2`, a call returning
  `int`, and a string literal used as bare statements all compile and run
  (exit 0). A rule that rejects discarding `Result` specifically is therefore a
  targeted special case, not an extension of a general rule.
- **There is no Aura prelude and no builtin type registry.** `grep -rn prelude`
  over `src/` and `docs/LANGUAGE_SPEC.md` returns exactly one hit,
  `pyo3::prelude` (`src/bridge/mod.rs:51`). `src/stdlib/mod.rs:20` derives
  `builtin_names()` from `signatures::builtins()`, and
  `src/stdlib/signatures.rs` models **callables only**
  (`Accepts`/`TypeClass`/`Returns`). `grep -rn 'builtin_type\|prelude_enum\|inject_enum' src/`
  returns nothing, and no `std/` directory exists.

### 1.7.2 `try` in tail position — a three-part checker defect

`attempt`/`try`-as-expression is a G02 RFC topic (RFC 0003). Investigating it
exposed a **soundness defect that exists independently of any new syntax**: a
`try` statement in tail position already produces a value at run time, but the
checker types it as `none`.

The evaluator is deliberate: `src/run/mod.rs:1264` runs the body with
`exec_block(body, env, true)` and returns the block's last value on normal
completion, while `finally` only replaces a pending outcome that is not
`Ctl::Val(_)` (`LANGUAGE_SPEC.md:1760–1762`). So `fn f() -> int { try { 41 }
catch _ { 99 } }` prints `41` and `fn f() -> int { try { throw "x" } catch _ {
99 } }` prints `99` — both check clean.

The checker disagrees. `block_value_ty` (`src/check/mod.rs:2357`) maps a
`Stmt::Try` tail to `Ty::None`, making `try` the **only** construct that is
statically `none` while dynamically value-producing. Probes are one `.aura`
file per directory; `check`/`run` exit codes shown.

| Probe | Program | `check` | `run` |
|---|---|---|---|
| control | `fn f() -> int { if true { "a" } else { 1 } }` | 1 (`E3005 int \| string`) | — |
| control | `fn f() -> int { match true { true -> "a"  _ -> 1 } }` | 1 (`E3005 int \| string`) | — |
| control | `fn f() -> int { { "a" } }` | 1 (`E3005`) | — |
| token | `fn f() -> int { try { "a" } catch _ { 1 } }` | **0** | 0 (`a`) |
| token | `fn f() -> [int] { try { 5 } catch _ { 6 } }` | **0** | 0 (`5`) |
| token | `fn f() -> Opt<int> { try { 5 } catch _ { 6 } }` | **0** | 0 (`5`) |
| unsound | `fn f() -> string { try { 5 } catch _ { 6 } }` then `let s: string = f()` | **0** | 0 (`5`) |
| oracle | `fn f() -> int { return none }` | 1 (`E3005`) | — |
| hole | `fn f() -> int { none }` | **0** | 0 (`none`) |
| hole | `fn f() -> [int] { none }` | **0** | 0 (`none`) |
| hole | `fn f() -> Opt<int> { none }` | **0** | 0 (`none`) |

That last block is a separate, deeper defect: the same value under the same
annotation is rejected when written `return none` and accepted when written
`none`, because `block_tail_mismatch` (`src/check/mod.rs:5893`) short-circuits
on `Ty::None` **before** consulting `return_compatible` — the very relation
that already implements the spec's rule. `LANGUAGE_SPEC.md:1018–1026` states
that `none` does **not** bypass a structured expectation (`-> [T]`,
`-> Opt<T>`), and `:1819–1826` states the implicit body value "MUST also be
compatible with that type (`E3005` otherwise)".

A third gap sits between them: `block_tail_mismatch` opens with a
`let Some(Stmt::Expr(..)) = body.last() else { return Ok(None) }`, so a
**direct** `try` tail never reaches the comparison at all, even after the other
two are repaired.

**The repair is three coupled parts and was validated, then withheld.** It was
applied, rebuilt, and measured in every partial configuration; each part is
independently necessary (removing any one reintroduces a false negative or a
false positive), `cargo test --locked --all-targets --all-features` stays
`1477 passed / 0 failed`, `cargo clippy … -D warnings` exits 0, and a sweep of
all **125** tracked `.aura` files shows **17 non-zero before and 17 after —
zero status changes**. The source was then restored byte-for-byte to baseline
(sha256 `1232b6874dd2e9c4f556085b4f3b240c120e6dedf77baace9a8e5d6aa738f532`,
`git diff` empty) because gate G02 forbids coding the affected surface before
the RFC is accepted. Details, including the reverse-mutation matrix, are in
`docs/rfcs/0003-try-as-value-expression.md` §6.

**One spec-sanctioned exemption must survive any repair.** `LANGUAGE_SPEC.md:1819–1826`
says a body ending in a declaration, a loop, or a `return` has no implicit
value and is not judged. `fn f() -> int { print(1) }` and `fn f() -> int { }`
being accepted is therefore **correct**, not part of the defect.

### 1.8 Test and CI topology

`.github/workflows/ci.yml` jobs: `fmt`, `clippy`, `test`, `cpython-interop`,
`cpython-interop-windows`, `no-python`, `msrv`, `proptest`, `validator-tests`,
`contract`, `miri`, `audit`, `fuzz`, `playground`, `website`. Additional
workflows: `fuzz-nightly.yml`, `pages.yml`, `release.yml`.

Per AGENTS hard-won rule 7, a green badge proves only that the *wired subset*
ran. G18 must enumerate the test files each job actually invokes before any
matrix cell is marked PASS.

---

## 2. Scope

### 2.1 In scope (P0 — release-gating)

Consistent and recoverable error semantics usable by APIs; safe inference;
minimal DX and AIS; project configuration and Host capability boundaries;
typed entities and relations; SQLGuard enforcement; four honest database
drivers; explicit CRUD and repository surface; AQL/0.1; verifiable migrations;
integrated tests; correct artifacts.

### 2.2 In scope only as separate, individually-gated expansions (P1)

Repository generation from annotated traits, `aura test`, `aura fmt`,
`aura init`, broader DX/AIS, Playground improvements, Python Interop 2.0,
cooperative async/await, LSP. A P1 item that cannot reach full proof and
compatibility is recorded as `DEFERRED_WITH_APPROVAL`; it never blocks P0.

### 2.3 Out of scope for 0.3.2 without a new human decision (P2)

Universal PyPI transpilation, macros, class inheritance, Spring-like
proxies/reflection, threads/locks/channels, generic parallel execution,
free-form SQL/BSON in the protected profile, automatic destructive migrations,
a universal remote package manager, a new VM/bytecode, and advanced AQL
(subqueries, aggregations) without an RFC.

### 2.4 Absolute prohibitions

`--force` on the canonical branch; deleting tests to go green; raising limits
without evidence; weakening lints or security; suppressing errors with
`Unknown`/`Any`; building APIs that bypass the Host; overwriting frozen WASM;
rewriting the iterative engine into a different VM; giant unsplit pull
requests; auto-publishing versions.

### 2.5 Human approval required before proceeding

Incompatible public syntax or semantic change; Host defaults or permission
changes; changes to `py` default or sandbox policy; MSRV bump; a driver whose
dependency chain fails audit; a new WASM distribution identity; disabling an
implemented feature; release, tag, deploy, or stable promotion; real secrets or
external database access; destructive migrations.

---

## 3. Waves

Waves execute strictly in order. A wave starts only after the previous gate is
`PASS` on the integrated branch. Read-only research of later waves may proceed
in parallel; implementation may not.

| Wave | Scope | Gate |
|---|---|---|
| **G00** | Reconcile state; verify 007 evidence; confirm no frozen artifact changed; publish prerequisite inventory and baseline in a documentation PR | clean, reconciled, published; failure ⇒ stop and report |
| **G01** | Dependency table, roles and ownership, ID'd backlog, risks, evidence criteria | approved table; no new code before critical contracts |
| **G02** | Freeze all P0 RFCs and interfaces | every P0 RFC accepted; conflicts decided explicitly; responsibility/capability map recorded; otherwise do not code the affected surface |
| **G03** | Language foundations: `Result`, recoverable errors, `try` as expression | verifiable semantics, no legacy divergence, correct diagnostics, tests and fuzz green, normative docs synced |
| **G04** | DX 2.0: diagnostics as an engineering contract | renderer regression comparison, stable snapshots, no code/span/CI regression, no format injection |
| **G05** | AIS Agent-Native: minimal context, maximal proof | AIS/0.1 clients still valid; v0.2 has schemas and roundtrip; semantic and DX data agree; quantitative tests reproducible |
| **G06** | Project structure, `init`, configuration, `std::os` | deterministic reversible loading, module compatibility, Host can deny env, no secrets in output, cross-OS tests where relevant |
| **G07** | Host Database, driver matrix, resource model | ADR per driver/MSRV/TLS; Host boundary compiles without drivers; equivalent `E5002` capability probes; resource and attack-surface tests; **no real database without authorized test credentials** |
| **G08** | `entity struct`, metadata, relations, cardinality | positive and negative lexer/parser/checker/AST tests; nominal legacy intact; cardinality and metadata tests; module compatibility; typed schema IR with versioned serialization |
| **G09** | SQLGuard/0.1 enforcement before execution | instrumented policy engine and verifiers; negative proofs for SQL injection, NoSQL operator injection, tenant bypass, secret leakage, undue privilege, timeout, and missing policy; **no driver receives a production query before this gate is PASS** |
| **G10** | First vertical slice: SQLite | end-to-end app compiled and run on SQLite; CRUD + relations + SQLGuard negatives green; no unverified query reaches the driver |
| **G11** | PostgreSQL with contract suite and RLS | documented semantic parity with SQLite for portable operations; RLS proven on the real role; tenant and injection negatives green |
| **G12** | MySQL with contract suite and honest limits | MySQL covers every portable P0 operation or reports precisely `capability-not-supported` where an RFC permits; no silent divergence |
| **G13** | MongoDB: documental backend without false equivalence | honest CRUD/portable-cardinality coverage; differences documented; Mongo security proven in real integration |
| **G14** | AQL/0.1: own parser, verified plans, translation | adversarial AQL parser/checker; typing by projection; four backends with golden/contract suites and injection/isolation negatives; differences explicitly rejected |
| **G15** | Repository by traits: contract first, generation later | manual Repository functional and equivalent to the DB API; generation released only with complete checker, origin, override/collision, and backend tests; if P1 does not fit, record `DEFERRED_WITH_APPROVAL` without blocking P0 |
| **G16** | Migrations, SQLGuard preflight, real application | `SQLGuard PASS` in a configured integration environment with auditable evidence; any non-verifiable operation documented as NO-GO, never certified safe |
| **G17** | Separated P1 expansions: Python, async, tooling | P1 complete only with full proof and compatibility; otherwise RFC + `DEFERRED_WITH_APPROVAL`; no experimental feature appears as stable in the changelog |
| **G18** | Quality matrix, CI, and proof before each merge | every P0 feature row green or explicitly justified NO-GO; no simulated tests; no gap masked by Rust compiling without real integration |
| **G19** | Release candidate and promotion conditions | canonical branch clean, no unreviewed PRs, final artifact parity, SQLGuard demonstrated, real integrated tests, consistent docs, evidence-signed report; **end the campaign with no tag and no publication** |

---

## 4. Status vocabulary

Every wave uses exactly one of:

`NOT_STARTED` · `IN_PROGRESS` · `BLOCKED` · `PASS` · `NO_GO` ·
`DEFERRED_WITH_APPROVAL`

Current wave status:

| Wave | Status |
|---|---|
| G00 | `PASS` — PR #43, merge `7bd1d6fb`, 21/21 checks |
| G01 | `PASS` — PR #45, merge `6f075882`, 21/21 checks; dependency table and MSRV blocker disclosed (§1.5.1, §10.1) |
| G02 | `IN_PROGRESS` — RFC 0002 (`Result`) and RFC 0003 (`try` as expression) authored, probe-verified and merged as PR #46 → `6cc96bf5`; RFC 0002 tag-scope premise corrected via PR #47 → `fe0e7312` and PR #48 → `73a35d9b` (both 21/21). Two blocking language findings are escalated: `R-10`/`E2013` vs. `Result` (§10.2) and `R-11`/tail-`try` typing plus the grammar question (§10.3). The remaining ten P0 RFCs are under authoring |
| G03–G19 | `NOT_STARTED` |

A `PASS` requires observed evidence. An environment that was not executed is
reported as not executed, never as `PASS`. `UNVERIFIED` on any mandatory
SQLGuard requirement forces `NO-GO`.

---

## 5. Backlog identifiers

Identifiers are stable for the campaign and must be cited in commits, PR
bodies, and checkpoints. Numbering is **sparse by design**: the third digit
block reserves space inside a wave, so there are 34 identifiers here, not 190.
`NBL-001`…`NBL-190` is the span, not the count.

| ID | Wave | Item | Priority |
|---|---|---|---|
| NBL-001 | G00 | Campaign document, baseline, prerequisite inventory | P0 |
| NBL-002 | G00 | Preflight `agent-state.sh` writer extraction defect | P3 |
| NBL-003 | G01 | Dependency table and ownership map | P0 |
| NBL-004 | G01 | MSRV 1.83 versus driver dependencies — blocker and decision package (§1.5.1, §10.1) | P0 |
| NBL-010 | G02 | RFC: `Result` and recoverable error semantics — **RFC 0002 authored; blocked by §10.2 D1** | P0 |
| NBL-011 | G02 | RFC: `try` as expression without ambiguity — **RFC 0003 authored; blocked by §10.3 D1** | P0 |
| NBL-012 | G02 | RFC: inference and error propagation rules | P0 |
| NBL-013 | G02 | RFC: diagnostics contract and `aura explain` | P0 |
| NBL-014 | G02 | RFC: AIS Agent-Native 0.2 capability set | P0 |
| NBL-015 | G02 | RFC: `Aura.toml`, `aura init`, `std::os` | P0 |
| NBL-016 | G02 | RFC: Host database capability and protected profile | P0 |
| NBL-017 | G02 | RFC: `entity struct`, metadata, relations, cardinality | P0 |
| NBL-018 | G02 | RFC: CRUD surface and transactions | P0 |
| NBL-019 | G02 | RFC: AQL/0.1 grammar, plans, result shapes | P0 |
| NBL-020 | G02 | RFC: Repository in traits | P0 |
| NBL-021 | G02 | RFC: SQLGuard/0.1 control set SG-01…SG-08 | P0 |
| NBL-022 | G02 | RFC: SQL and Mongo driver contract | P0 |
| NBL-030 | G03 | `Result<T,E>` as a library/builtin generic enum — **blocked by §10.2** | P0 |
| NBL-031 | G03 | `try` expression parsing and checking — **blocked by §10.3 D1 and D2** | P0 |
| NBL-040 | G04 | Rich diagnostic projection and renderer | P0 |
| NBL-050 | G05 | AIS/0.2 additive capabilities | P0 |
| NBL-060 | G06 | Project/config loading and Host env denial | P0 |
| NBL-070 | G07 | Database capability and resource model | P0 |
| NBL-080 | G08 | Entity/metadata/relation syntax and schema IR | P0 |
| NBL-090 | G09 | SQLGuard policy engine and verifiers | P0 |
| NBL-100 | G10 | SQLite driver and first vertical slice | P0 |
| NBL-110 | G11 | PostgreSQL driver and RLS | P0 |
| NBL-120 | G12 | MySQL driver and declared limits | P0 |
| NBL-130 | G13 | MongoDB driver and declared differences | P0 |
| NBL-140 | G14 | AQL parser, planner, translators | P0 |
| NBL-150 | G15 | Repository contract and generation | P1 |
| NBL-160 | G16 | Migrations and SQLGuard preflight | P0 |
| NBL-170 | G17 | Python Interop 2.0, async/await, tooling | P1 |
| NBL-180 | G18 | Quality matrix and wired-test enumeration | P0 |
| NBL-190 | G19 | Release candidate readiness report | P0 |

---

## 6. Risks

| ID | Risk | Impact | Mitigation |
|---|---|---|---|
| R-01 | `try`-as-expression cannot coexist unambiguously with the catch-mandatory `try_stmt` | Blocks G03 and therefore every `Result`-returning API | Suspend that specific form and request a decision; never insert a heuristic parser |
| R-02 | `@` metadata requires a token class that does not exist | G08 syntax may need a lexer change with compatibility cost | Decide in the G02 RFC with positive and negative grammar tests before implementing |
| R-03 | No database, async, or pooling dependency exists | G07–G16 are all net-new supply-chain surface | Per-driver ADR; `cargo audit`; human approval when a chain fails |
| R-03a | **Every candidate driver breaks the mandated Rust 1.83 floor** (§1.5.1). `block-buffer ^0.12` and `icu_provider ^2.4.0` are hard requirements whose entire usable ranges are edition 2024 / MSRV ≥ 1.85 | G07 onward is unreachable without a human decision; the §18.2 `--all-features` floor cannot stay intact for any driver except `rusqlite 0.37.0` | Escalated as §10.1; no driver is adopted and no floor is amended until a human chooses |
| R-04 | No authorized database credentials are available | G10–G16 cannot claim real integration | Report unexecuted environments honestly; a cell is never `PASS` by compilation alone |
| R-05 | Four backends have genuinely different semantics | False equivalence would be a correctness and security defect | Declare `capability-not-supported` explicitly; never silently diverge |
| R-06 | `py` runs a full CPython process with full authority | In-process CPython cannot be SQLGuard-certified | State the boundary; `production-enforce` must refuse known bypass |
| R-07 | Green CI proves only the wired subset | A passing badge could hide unwired suites | Enumerate invoked test files per job at G18 |
| R-08 | Campaign breadth could encourage splitting into giant PRs | Unreviewable changes | Short-lived branches, one coherent surface per PR, auditable bodies |
| R-09 | Partial P1 delivery could be overstated as stable | Misleading release notes | P1 absent full proof ⇒ `DEFERRED_WITH_APPROVAL`; changelog never marks experimental work stable |
| R-10 | **Contradictory tag-scope rules (`E2013`) make a prelude or builtin `Result<T,E>` with `Ok`/`Err` unviable** (§1.7.1a). `LANGUAGE_SPEC.md:2407`/`:3281` say tags are unique programme-wide; `:3399`/`:1556`/`:3897` say per module; the compiler implements per module by prefixing tags with the module path. A root-scope `Result` claims both spellings for the root scope only — a module-scoped `Ok` still coexists (exit 0), a root-scope one fails (exit 1) even if `Result` is never constructed. A module-scoped `Result` avoids that but its tags are unreachable unqualified (`E3002`), and braced variant import is a parse error (`E1006`). `E2013`'s message text asserts the losing (global) rule | Blocks `NBL-030` and therefore G03, plus every downstream `Result`-returning API in G07–G17. The directive's §2.4 vocabulary (`Ok(T)`/`Err(E)` unqualified) cannot be satisfied without a choice | Escalated as §10.2 (D1a scope, D1b message, D1c placement) with three lawful outcomes; no option is selected by an agent. §3.2's "quando viável" is the lawful exit, but exercising it is a human decision |
| R-11 | **`try` in tail position is statically `none` while dynamically value-producing** (§1.7.2), and `none` bypasses the implicit-return check that `return none` fails | End-to-end unsoundness: a function annotated `-> string` can return `5` and the caller's `let s: string` is accepted. Blocks any G03+ surface whose types assume a checked return | Escalated as §10.3; the validated three-part repair is withheld, not committed, and `src/check/mod.rs` is byte-identical to baseline |

---

## 7. Evidence criteria

A wave may be marked `PASS` only with all of:

1. the focused commands actually executed, quoted with their results;
2. no regression in previously passing suites;
3. normative documentation updated where semantics changed;
4. the diff reviewed by an agent that did not author it;
5. remote CI green for the integrated commit, or an explicit statement that CI
   did not run.

**Probe isolation is mandatory.** `aura check` and `aura run` eagerly discover
and compile every eligible `.aura` file in the entry file's directory
(`docs/FILESYSTEM_MODULES_DESIGN.md:399`, matrix row `:774`; source root is the
entry file's parent, `:211`). A malformed sibling therefore produces
diagnostics attributed to a file the probe did not intend to test. Every probe
runs in a directory freshly created for it and containing **exactly one**
`.aura` file, and a probe directory is never reused. An earlier apparent
"multi-file diagnostics defect" was investigated and falsified this way: the
behaviour is a frozen, intended contract, not a defect.

Validation floor for milestone closure, as listed in `AGENTS.md`:

```
cargo fmt --all -- --check
cargo test --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time
cargo +1.83.0 check --locked --all-features
cargo +nightly check --manifest-path fuzz/Cargo.toml --all-targets
node playground/tests/node/run-all.mjs
node playground/build.mjs --check
node website/build.mjs
node website/tests/run-all.mjs
```

`--check --require-source-match` is reserved for tag and release candidates.
`cargo clean -p` is not valid clean-rebuild proof in this repository because
there is no root `[workspace]`.

### 7.1 Required pull request body

Problem and reproduction; scope; contract; tests before and after;
compatibility; risks; authority source; CI URL; Native and WASM behavior;
feature-gate effects; summarized diff. Security review has veto power. A failed
gate means no merge and no progress to the next wave.

---

## 8. Architecture constraints

### 8.1 OOP philosophy (immutable)

`struct` is nominal and typed; `impl` is behavior; `trait` is a static
contract; generic bounds and composition exist. There are **no classes,
inheritance, RTTI, Java-style proxies, or dynamic dispatch**. A generated
Repository must appear as a traceable static `impl` and must not redefine a
common `trait`.

### 8.2 SQLGuard controls

| ID | Requirement |
|---|---|
| SG-01 | Bind-only. No raw SQL/BSON in the protected profile; no unvalidated AST may become a plan. |
| SG-02 | Pre-driver checks on type, field, projection, relation, operator, and limit. Adversarial BSON such as `{"$ne": null}` must not become an operator. |
| SG-03 | Tenant and auth derived from trusted Host context; isolation across every operation including joins, relations, link/unlink, transactions, and caches. Deny when tenant is absent. RLS is defense-in-depth on PostgreSQL only. |
| SG-04 | Credentials never printed; TLS validated; least privilege; separate application and migration accounts. |
| SG-05 | Timeout, max rows/bytes, connections, memory, parameters, cancellation, and N+1 budget with explicit errors. No silent truncation; no full in-memory filtering. |
| SG-06 | No automatic DDL at startup; destructive migrations blocked by default; drift and concurrent-migration detection; audit trail. |
| SG-07 | Audit records plan ID, policy hash, entity, backend, operation, trace ID, and decision — with no secrets or PII. Caches keyed by schema, policy, and driver, and tenant-safe. |
| SG-08 | `production-enforce` refuses known bypass. `py` is **not** a sandbox: it has full CPython process authority, so untrusted Aura code with in-process CPython cannot be SQLGuard-certified. |

### 8.3 Database API surface

`insert`, `insert_many`, `find`, `find_one`, `find_many`, `update`, `delete`,
`count`, `related`, `link`, `unlink` — each returning `Result<T, DbError>`.
Absence is `T | none`. `query_one` must error on duplicates, detected in the
AQL stage.

### 8.4 Cardinality

1:1 required, 0:1, 1:N, N:N. `[T]` means **0..N**, never 1..N. SQL uses
foreign keys, `UNIQUE`, `NOT NULL`, and junction tables. MongoDB uses a
declared reference or embedded policy and must not claim foreign-key or
atomicity parity.

### 8.5 AQL result shapes

`query_many<T>` yields `[T]`; `query_one<T>` yields `T | none` and fails if more
than one row matches; `query_scalar<T>` requires exactly one row and value;
`COUNT` yields `int` with checked overflow; `SELECT DISTINCT entity` compares
persistent identity; 1:N and N:N joins may multiply rows; `LEFT JOIN` preserves
absence.

---

## 9. Delegation

The lead agent is the sole writer of the worktree. Every reviewer is read-only:
no edits, staging, commits, pushes, resets, restores, cleans, or deletions. At
most two subagents run concurrently. The lead independently reproduces every
finding before acting on it, because reviewer consensus is not evidence.

| Role | Responsibility |
|---|---|
| Lead | Tasks, interfaces, integration, sole writes, gate adjudication |
| Language | RFCs, parser, checker, AST, grammar |
| Runtime / Host | Capabilities, ABI compatibility, resource model |
| SQL | SQLite, PostgreSQL, MySQL drivers, isolated |
| Mongo | MongoDB driver, isolated |
| Security | Independent red-team tests, veto power |
| DX / AIS | Provenance, diagnostics, Agent-Native surface |
| Release | Hashes, CI, frozen-artifact verification |

The repository's custom Aura agents are the preferred reviewers:
`Aura Orchestrator`, `Aura Scout`, `Aura Spec Auditor`, `Aura Deep Auditor`,
`Aura Adversarial Reviewer`, `Aura Artifact Verifier`, `Aura Implementer`.
No agent may approve its own code as sole reviewer.

### 9.1 Environment constraint — custom Aura agents unavailable

As of G00 and reconfirmed during G01, every `Aura *` custom agent fails to
launch with:

```
Model 'customendpoint/Robin/deepseek-ai/deepseek-v4.1-flash-1017' is not available.
```

Each custom agent is pinned to that endpoint, and the model is not present in
the available set. Passing an explicit `model` override is also rejected for
these agent types in practice.

This is recorded rather than worked around. Per the degradation rule in
`AGENTS.md` ("if parallel subagents are unavailable, run fresh sequential
review sessions instead — the protocol degrades cleanly and must not be
faked"), the lead performs the reviewer duties directly: reproducing every
claim, re-running each command, and adversarially re-reading its own published
text. Every checkpoint states plainly which reviews were executed by the lead
because the agents could not run. A review is never reported as delegated when
it was not.

---

## 10. Blocking policy

A wave is blocked, with a stable tree preserved and an issue or RFC opened with
alternatives, when a requirement is impossible because of MSRV, a cross-database
semantic discrepancy, an isolation risk, OOP or parser breakage, a Native/WASM
regression, or a failing security test. Replacing a problem with a `TODO` and
marking the wave `PASS` is forbidden. A genuine specification choice produces a
decision package rather than a silent resolution.

### 10.1 Open decision — MSRV 1.83 versus the required database drivers

Recorded by G01 and **not resolved by the agent**, because raising the MSRV is
on the human-approval list in §2.5. The evidence is in §1.5.1. Only one
candidate driver compiles on Rust 1.83 at all, and only when pinned to an older
release.

The three legitimate outcomes:

1. **SQLite-only with documented NO-GO for the rest.** Accept `rusqlite 0.37.0`
   (proven green on 1.83) as the sole P0 driver. Postgres, MySQL, and MongoDB
   become documented `NO_GO` outcomes in the G18 matrix rather than silent
   omissions. This preserves the MSRV contract and the §18.2 floor exactly as
   published. Cost: P0 "four drivers with an honest matrix" becomes "one driver
   plus three grounded NO-GOs", which requires an explicit scope change since
   the directive lists the four drivers as P0.
2. **Authorised MSRV bump.** Raise `rust-version` to at least 1.94 (the floor
   imposed by `sqlx 0.9.0`, the highest of the candidates) and update the
   `msrv` CI job, `Cargo.toml`, and every document that states 1.83. This
   unblocks the entire driver set at the cost of dropping Rust 1.83 users.
   This is explicitly a human decision and cannot be taken by the agent.
3. **Feature-gated drivers and an amended floor.** Keep `rust-version = "1.83"`
   for the default build, declare each driver under a non-default feature, and
   amend the §18.2 floor so `--all-features` is replaced by a named feature
   set. This was proven to work mechanically: with the optional dependency
   disabled the same project compiles on 1.83 with exit 0, and with the feature
   enabled it fails with exit 101. Cost: `--all-features` can no longer mean
   "everything builds", which weakens a validation command that is a published
   contract, so it needs the same authorisation as option 2.

`libsql` is excluded from all three options: it fails on 1.83 in every version
probed and cannot serve as the SQLite driver.

No option is selected here. G01 records the finding; the choice belongs to a
human, per §2.5 and §20.3.

### 10.2 Open decision — `Result` versus enum-tag scope

Recorded by G02 and **not resolved by the agent**. This decision was first
recorded on a premise that later evidence falsified, and the correction matters
(§1.7.1a): **`LANGUAGE_SPEC.md` contradicts itself on tag scope.** `:2407`
(§18.1) and `:3281` (§26) say tags are unique programme-wide; `:3399` (§27,
`CONF-RESOLVE-6`), `:1556` (§12) and `:3897` (§33.14, a *Frozen Design
Decision*) say per module. The implementation is **per module** and prefixes
tags with the module path (`src/resolve.rs:1452-1455`), so sibling modules may
each declare `Ok` — verified by execution in both declaration orders.

The campaign's mandated vocabulary — `Result<T,E>` with unqualified
`Ok(T)`/`Err(E)` (§2.4) — therefore cannot be delivered as a prelude or
builtin type until the scope question is answered. Evidence is in §1.7.1a; the
RFC is `docs/rfcs/0002-result-and-recoverable-errors.md`, whose decisions
**D1a (which clause is normative), D1b (`E2013` message wording)** and
**D1c (placement)** record this escalation.

**D1a must be answered first.** It is a pure specification question with the
probe evidence already attached. Answering D1c before D1a risks choosing a
placement whose stated cost is an artefact of the losing clause.

After D1a, the three legitimate outcomes:

1. **Top-level `Result`, reserving `Ok` and `Err` in the root scope.** Callers
   write `Ok(7)` / `Err(e)` exactly as §2.4 requires. Cost, restated precisely
   after the correction: `Ok` and `Err` become unusable as variant tags **in
   the root scope**, not "in the same compilation unit" as first recorded —
   a module-scoped `enum Parsed { Ok(string) }` still coexists (exit 0), but a
   root-level one fails (exit 1) **even when `Result` is never constructed**, so
   the breakage depends on whether unrelated code merely *declares* `Result` —
   a DX hazard and an effective reserved-word change. It is *prospectively*
   breaking: no `.aura` file declares either tag today (0 of 125 files), so
   nothing in-tree breaks. It also requires reconciling `:2407`/`:3281` with
   `:3399`/`:1556`/`:3897`, and needs §2.5 approval plus an ADR.
2. **Module-scoped `Result`, qualified construction.** `module Std { pub enum
   Result<T,E> { Ok(T), Err(E) } }` — proven to coexist with a user `Ok`
   (exit 0). Callers write `Std::Result::Ok(7)`, or `Result::Ok(7)` after
   `use Std::Result`. Cost: unqualified `Ok(7)` is `E3002`, directly
   contradicting §2.4's notation, and selective variant import
   (`use Std::Result::{Ok, Err}`) is a parse error today (`E1006`), so the
   qualifier cannot be removed without a second grammar feature.
3. **Declare per-module uniqueness normative and retire the global clauses.**
   This makes the specification match the shipped compiler instead of
   overriding it: `:2407` and `:3281` are rewritten, `:3399`/`:1556`/`:3897`
   stand, and `E2013`'s message ("unique across the program") is corrected to
   describe the scope it actually enforces. Cost: it is still a normative spec
   change requiring an ADR and a compatibility review, and it retroactively
   legitimises something the compiler has been doing without authority — but
   unlike the original framing of this option, it **removes** no shipped
   behaviour and breaks no existing program. It also makes Option 2 legally
   supported by §33.14 rather than merely tolerated.

Directive 008 §3.2 says `Result` must rest on "o modelo de `enum`/generics
existente **quando viável**". All three options are viable today; the phrase
supplies a lawful exit from any of them, but choosing between them is a language
decision, not an implementation detail. `NBL-030` and every downstream `Result`
API stay blocked until a human answers D1a and then selects one.

**Separate, smaller defect — `E2013`'s message text.** `src/check/mod.rs:1378`
emits "variant names must be unique across the program" for a check that is
scope-local. This is worth reporting independently of D1a because it is a
diagnostic that **misdescribes the rule that rejected the user's code**. It is
low-risk to change: no test matches the string (`tests/grammar.rs` matches
`codes::DUPLICATE_VARIANT`), and `docs/errors.md:37` records only the code, a
title, and a trigger example. It is filed as D1b rather than fixed because the
correct wording depends on which clause wins.

---

### 10.3 Open decision — `try` as a value expression, and the tail-`try` defect

Directive 008 §3.3 requires `try` as an expression while also requiring that
the existing `try { … } catch … finally …` statement form keep working, and it
forbids resolving the conflict with a heuristic parser. Two decisions are
therefore open; `docs/rfcs/0003-try-as-value-expression.md` is the full record.

**Decision D1 — grammar.** Three lawful outcomes, none chosen:

| # | Option | Cost | Risk |
|---|---|---|---|
| 1 | Add `try_expr` to the `atom` production (`docs/grammar.md:118–124`) | Grammar delta of two lines; `try` becomes legal in expression position | The statement and expression forms overlap. `return try { … }` already reports `E1006: expected a newline between statements, found 'try'`, so at least one position is provably ambiguous today. Resolving it is exactly the heuristic the brief forbids |
| 2 | Do not touch the grammar; use a block (`let x = { try { 1 } catch _ { 2 } }`) | Zero grammar risk — blocks are already expressions and a tail `try` already yields a value | Brief §3.3 is satisfied semantically, not syntactically; one extra brace pair |
| 3 | Defer expression-form `try` out of 0.3.2 | None now | Brief §3.3 not delivered |

**Decision D2 — the checker repair.** The three-part repair of §1.7.2 is
validated but not committed. It is a **behavioural change**: two probe programs
that check cleanly today would report `E3005`, both correctly. Gate G02
(`:372`) forbids coding the affected surface before the RFCs are accepted, so
the change waits. `src/check/mod.rs` is at baseline.

**Blast radius of the deferral.** `NBL-011`, `NBL-031` and D1 stay blocked;
RFC 0002's `Result` work is unaffected because it needs no grammar change. G03
and any downstream gateway that consumes an expression-form `try` stay blocked
until D1 is resolved. Nothing in G04–G19 may assume a checked implicit return
until D2 is authorized.

## 11. Wave checkpoint format

Every checkpoint records, concisely in chat and fully in the repository:

```
WAVE:
STATUS:
HEAD:
PRs:
CHECKS:
TESTS_REALMENTE_EXECUTADOS:
PROVAS:
RISCOS:
DECISÕES:
PRÓXIMA ONDA AUTORIZADA:
```

`AGENT_STATE.md` must allow resumption without recreating a pull request or
repeating a completed audit.

---

## 12. Success condition and mandatory stop

A person can develop an Aura application with an explicit project and
configuration, typed entities and relations, safe verifiable operations on four
databases, AQL and Repository access without raw SQL, useful diagnostics,
traceable AIS, and reproducible documentation — while Native and WASM runtimes
show no regression and security limits are declared. No system is claimed to be
100% invulnerable.

**MANDATORY STOP.** After the G19 readiness report, only these actions remain,
and each requires a separate, explicit, specific human instruction: creating tag
`v0.3.2`; modifying `manifest.current`; promoting a channel; publishing a
release; changing default feature policy; performing a real migration. On
`NO-GO`, present a minimal correction plan without circumventing the blockers.
