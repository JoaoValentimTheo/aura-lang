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
and integrated as `73a35d9b` (PR #48, 21/21 checks green), then recorded as
`85de5945` (PR #49, 21/21 checks green), and that record itself recorded as
`fc49c5ee` (PR #50, 21/21 checks green) and `2df1c322` (PR #51, 21/21 checks
green). While
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

G02 has since authored **RFC 0004 — The Diagnostics Contract**
(`docs/rfcs/0004-diagnostics-contract.md`, `NBL-013`), merged as `2df1c322`
(PR #51, 21/21 checks green) together with the new `docs/rfcs/README.md`
index. It documents a third blocking
finding: the shared diagnostic renderer performs **no sanitisation**, so a
perfectly legal source file can forge diagnostic lines (§1.2, risk `R-12`),
and it records a spec-internal contradiction between §30.4 (every code
reachable) and §30.3 (`E4999` must not be reachable) plus a stale citation in
`src/main.rs:26` to a `§47/§48` that the specification does not contain. Its
`D3` (character versus visual columns) and open item 13 (non-ASCII module
directories are skipped silently) are escalated in §10.4.

G02 has also authored **RFC 0005 — Inference and Error-Propagation Rules**
(`docs/rfcs/0005-inference-and-error-propagation.md`, `NBL-012`), now **merged
as PR #53 → `87cea70e`**. Merge records the proposal; §10.5 `D1` remains
unanswered. It documents a fourth blocking finding — generic instantiation
is never propagated to the call site, in two independent parts (§1.7.3, risk
`R-13`) — plus two unrelated, generic-independent holes found while probing it:
indexing is never type-checked (`R-14`) and, more seriously, **equality is never
type-checked and does not fail at run time**, so `1 == "s"` prints `false`
(`R-15`). The validated two-part repair is withheld under gate G02. `D1` is
escalated in §10.5.

G02 has now authored **RFC 0006 — The AIS Agent-Native Capability Set**
(`docs/rfcs/0006-ais-agent-native-capabilities.md`, `NBL-014`), **merged as PR
#55 → `d03d2718`** (21/21 checks green). Merge records the proposal; §10.6
`D1`/`D2`/`D3` remain unanswered and gate G02 still forbids coding the surface.
It is the largest finding set in the campaign: **eighteen
defects (A–R)** in the AIS/0.1 protocol and its MCP projection, every one
reproduced from live probes rather than read off the source. The load-bearing
ones: `Symbol.range` spans only the declaration's leading keyword and
understates true extents by **6.4×** aggregate (`A`, `I`); three of eleven
`SymbolKind` variants can never be constructed while `AIS.md:99-101` documents
them as live (`H`); `Slice.diagnostics` is documented twice and populated
nowhere (`L`); the AIS document path never runs name resolution, so `E2019` is
silently dropped with exit 0 while `E2018`/`E2023` survive carrying the wrong
code `E2003` (`M`, `N`, `O`); the CLI exposes 3 of 7 operations and
`aura://schema` advertises 6 of 7 tools (`B`, `D`); and
`tests/ais_differential.mjs` states in its own docstring that it "runs in CI"
while nothing invokes it (`E`). Three decisions `D1`–`D3` are escalated in
§10.6. No production source was modified; the extent repair was validated by
mutation and reverted under gate G02.

G02 has now authored and merged **RFC 0007 — Project Structure, `aura init`, and
the `os` Capability** (`docs/rfcs/0007-project-structure-init-and-os.md`,
`NBL-015`, merged as PR #57 → `6f5bca98`, with its record and the `G06`
clause-arithmetic errata merged as PR #58 → `8e6107a9`).
Its finding is unusual in this campaign: **`G06` is almost entirely greenfield.**
Three of its five acceptance clauses hold *because the feature is absent* —
there is no environment access to deny, no configuration to make incompatible,
no secret-bearing surface to print. A fourth, deterministic reversible
loading, holds **on evidence** rather than by absence: discovery is
extension- and shape-filtered by `is_aura_module_name`
(`src/module_graph.rs:948-957`), and an `Aura.toml` at the source root probed
`RC=0`. The fifth, cross-OS tests where relevant, is **unevaluable** until
`D3` resolves. `grep -n 'name: "env"' src/stdlib/signatures.rs` returns
nothing; `aura init` exits 2; `grep -rn 'Aura\.toml' --include=*.rs` returns
0 hits. Every design that adds the surface is therefore a net loss of a
property the project currently holds unconditionally.

The backlog's own spelling is unsound: there is no `Std` namespace —
`use Std::Math` and `use std::os` both fail with `E2019`, and
`grep -rn '"Std"' src/` is empty, because the standard library is a flat prelude
of **91** builtin functions rather than a module namespace. `D1` (what
`Aura.toml` may contain), `D2` (whether an `os` surface exists at all), and `D3`
(what `init` writes and how it refuses) are escalated in §10.7. `D1` is the root
decision: `D2` and `D3` are blocked on it.

G02 has also authored and merged **RFC 0008 — Host Database Capability and the
Protected Profile** (`docs/rfcs/0008-host-database-capability.md`, `NBL-016`,
merged as PR #60 → `a70651cc`). Its finding is that `G07` is **structurally
ready and functionally greenfield**: the `Host` boundary that a driver would
plug into already exists as one trait with ten methods (eight required, two
defaulted), and the `http-api`/`http` feature split already demonstrates the
exact shape a database capability needs — a builtin surface that type-checks in
every build and is denied at run time with `E5002` when the host lacks the
capability. Aura's answer to "how do we add a capability without breaking the
sandbox" is therefore not to be invented; it is to be copied. What does not
exist is everything else: no `PendingEffect` variant can carry a connection
handle (`Http` is its only variant), no `Value` or `TypeExpr` variant can
represent an opaque handle, `Returns::Dynamic` resolves to `Ty::Unknown` and is
never consulted by the checker, there is no resource model,
`CRITICAL_SYSTEM_PROFILE.md` §4 has no HTTP row, `CAPABILITY_MODEL.md`'s network
row is stale, and `docs/adr/` has no MSRV or TLS ADR. Three decisions `D1`–`D3`
are escalated in §10.8 and unanswered, so merging the RFC records the proposal
only and codes nothing.

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

### 1.7.3 Generic instantiation is never propagated to the call site

Discovered while opening `NBL-012` and **left unfixed** for the same reason as
§1.7.2.

**Symptom.** `fn id<T>(x: T) -> T { return x }`, called as `id(1)` and bound to
a `string`-annotated name, is accepted by `aura check` with **exit 0**. The
identical non-generic program is correctly rejected with `E3001`, so the gap is
generic-specific rather than a general `let`-inference gap. The accepted program
then fails under `aura run` at **exit 1** with
`error[E2003]: int has no method len` — `check` accepts what `run` rejects.

**Root cause.** `instantiate_return_ty` (`src/check/mod.rs:4497-4534`) discards
the substitution that `infer_substitution` returns, at **`:4530`**
(`let _ = …`), and then calls `ret.erase_params(&sigma)` with an unbound
`sigma`. Every `Ty::Param(n)` therefore falls through to
`unwrap_or(Ty::Unknown)` at `src/types.rs:244`, so every generic call types as
`Unknown`, which `Ty::compatible_with` (`src/types.rs:342-419`) accepts against
any annotation. Unification itself is correct and merely unwired:
`infer_substitution` returns `Option<HashMap<String, Ty>>` and `unify_into`
binds properly; the separate bounds-check path at `:3462-3487` does capture the
substitution but uses it only for `satisfies_bounds`.

**The repair is four lines and was validated, then withheld.** Capturing the
returned substitution and passing it to `erase_params` repairs all five positive
probes and leaves both negative probes at exit 0; two mutations (guard without
capture; capture then ignore) each re-introduce the unsoundness, proving the
capture is load-bearing. The full suite stays `1477 passed / 0 failed`, clippy
exits 0, and all **125** tracked `.aura` files are byte-identical in output and
status (`108` exit 0 / `17` exit 1 before and after). The source was restored to
baseline (`git diff` empty) because gate G02 forbids coding the surface before
the RFC is accepted. Recorded as risk `R-13`; the evidence bundle belongs to
`NBL-012` → RFC 0005.

**A second, independent defect was found underneath this one**, and it is why
the repair is a *two*-part patch rather than a one-liner. `Expr::Method` inference
(`src/check/mod.rs:4101-4115`) calls `instantiate_return_ty` with an `args` slice
that **excludes the receiver**, so an `impl<T>` parameter that occurs only in the
receiver type can never be bound from a call. Repairing D1a alone leaves
`impl<A> Pair<A> { fn get(self) -> A }` returning `Unknown`. The second part
binds impl parameters from the receiver the same way the already-correct
`check_struct_method_args` (`:4882`, binding logic `:4907-4931`) does. Each part
is independently load-bearing: a mutation that removes the receiver binding
leaves `p.get()` accepted.

**Two further holes surfaced while probing this defect and are _not_ part of
it.** Both reproduce with no generic call in the program at all, so neither is a
residual of the repair:

* **Indexing is never type-checked.** `let a = 5; a["k"]` is `check` exit 0 and
  `run` exit 1 (`E3001: cannot index int with string`). Registered as risk
  `R-14`.
* **Equality is never type-checked — and it does not even fail at run time.**
  `print(1 == "s")` is exit 0 under both `check` and `run` and prints `false`;
  `1 != "s"` prints `true`. Ordered comparison *is* checked (`1 < "s"` is
  `E3001`), so the hole is specific to `==`/`!=`. This is a silent-wrong-answer
  defect and is arguably more urgent than either generic defect. Registered as
  risk `R-15`.

Both are recorded in RFC 0005 §7.2 and are deliberately **not** bundled into this
RFC: they live in different code (`Expr::Index` inference; equality checking,
which does not appear to exist) and bundling them would make one authorization
decide three unrelated behaviours.

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
| G02 | `IN_PROGRESS` — RFC 0002 (`Result`) and RFC 0003 (`try` as expression) authored, probe-verified and merged as PR #46 → `6cc96bf5`; RFC 0002 tag-scope premise corrected via PR #47 → `fe0e7312` and PR #48 → `73a35d9b` (both 21/21), and recorded as PR #49 → `85de5945` (21/21); that record itself recorded as PR #50 → `fc49c5ee` (21/21). RFC 0004 (diagnostics contract) plus the RFC index merged as PR #51 → `2df1c322` (21/21). A fourth finding — generic instantiation is never propagated to the call site, so `check` accepts programs `run` rejects — is characterised and carried as evidence for `NBL-012`. Five blocking language findings are escalated: `R-10`/`E2013` vs. `Result` (§10.2), `R-11`/tail-`try` typing plus the grammar question (§10.3), `R-12`/diagnostic format injection (§10.4), `R-13`/generic instantiation plus the two generic-independent holes `R-14`/indexing and `R-15`/equality (§10.5), and `R-16`/the AIS protocol surface (§10.6). RFC 0005 (inference and error propagation) is merged as PR #53 → `87cea70e` and recorded as PR #54 → `422c7a9e`. RFC 0006 (AIS agent-native capabilities, `NBL-014`) is merged as PR #55 → `d03d2718` (21/21) and recorded as PR #56 → `4406c873`; the merge records the proposal only — its three decisions remain escalated in §10.6 and no protocol surface is coded. RFC 0007 (project structure, `aura init`, the `os` capability, `NBL-015`) is authored, probe-verified and merged as PR #57 → `6f5bca98` (21/21); §10.7 `D1`/`D2`/`D3` are escalated and no command or configuration surface is coded. RFC 0007's record and the `G06` clause-arithmetic errata merged as PR #58 → `8e6107a9` (21/21), and that record as PR #59 → `edcce6c7` (21/21). RFC 0008 (Host database capability, `NBL-016`) is merged as PR #60 → `a70651cc` (21/21, authored at `d0e05e46`) with §10.8 `D1`/`D2`/`D3` escalated and no driver, capability, or resource surface coded. Eight of the thirteen G02 P0 RFCs are authored and merged; six remain unauthored (`NBL-017`…`NBL-022`) |
| G03–G19 | `NOT_STARTED` |

A `PASS` requires observed evidence. An environment that was not executed is
reported as not executed, never as `PASS`. `UNVERIFIED` on any mandatory
SQLGuard requirement forces `NO-GO`.

---

## 5. Backlog identifiers

Identifiers are stable for the campaign and must be cited in commits, PR
bodies, and checkpoints. Numbering is **sparse by design**: the third digit
block reserves space inside a wave, so there are 35 identifiers here, not 190.
`NBL-001`…`NBL-190` is the span, not the count.

| ID | Wave | Item | Priority |
|---|---|---|---|
| NBL-001 | G00 | Campaign document, baseline, prerequisite inventory | P0 |
| NBL-002 | G00 | Preflight `agent-state.sh` writer extraction defect | P3 |
| NBL-003 | G01 | Dependency table and ownership map | P0 |
| NBL-004 | G01 | MSRV 1.83 versus driver dependencies — blocker and decision package (§1.5.1, §10.1) | P0 |
| NBL-010 | G02 | RFC: `Result` and recoverable error semantics — **RFC 0002 authored; blocked by §10.2 D1** | P0 |
| NBL-011 | G02 | RFC: `try` as expression without ambiguity — **RFC 0003 authored; blocked by §10.3 D1** | P0 |
| NBL-012 | G02 | RFC: inference and error propagation rules — **RFC 0005 authored; finding: generic instantiation is never propagated to the call site in two independent parts; `check` accepts what `run` rejects. Blocked on §10.5 D1. Probing it also surfaced two generic-independent holes: unchecked indexing (`R-14`) and unchecked equality (`R-15`)** | P0 |
| NBL-013 | G02 | RFC: diagnostics contract and `aura explain` — **RFC 0004 merged (PR #51 → `2df1c322`); blocked by §10.4 D3 and §10.4 item 13** | P0 |
| NBL-014 | G02 | RFC: AIS Agent-Native 0.2 capability set — **authored as RFC 0006, merged via PR #55 → `d03d2718`, still Proposed; blocked by §10.6 D1/D2/D3** | P0 |
| NBL-015 | G02 | RFC: `Aura.toml`, `aura init`, `std::os` — **authored as RFC 0007, merged as PR #57**; `std::os` shown unresolvable, `G06` shown largely greenfield | P0 |
| NBL-016 | G02 | RFC: Host database capability and protected profile — **authored as RFC 0008 and merged as PR #60**; `G07` shown structurally ready and functionally greenfield, the `http-api`/`http` split shown to be the proven template, two stale capability documents found and a third mis-cited | P0 |
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
| R-12 | **The diagnostic renderer performs no sanitisation, so a legal source file can forge diagnostic lines** (§1.2 of RFC 0004). `render_diagnostic` (`src/diagnostic.rs:125-166`) interpolates the unescaped `Value::display` of an uncaught throw and the raw file name into a single-line format. Four vectors are reproduced: a raw ESC byte in source, the ordinary `\n` escape, a newline nested inside a list value, and a file name containing a newline. All survive `--color never` and `NO_COLOR=1` | A user-visible trust failure in every consumer of the shared projection, including AIS and the Playground, which render the same facts by three different code paths. It is not a memory-safety issue and not an execution escape; it is unauthenticated output | Escalated as §10.4. The repair must be a **diagnostic-side** sanitiser: `print` legitimately emits raw newlines (`src/stdlib/mod.rs:101`), so changing `Value::display` globally would break `std::print`. `tests/oracle/mod.rs:440` duplicates the message format independently, so a single-site fix silently drifts |
| R-13 | **Generic instantiation is never propagated to the call site** (§1.7.3). Two parts. (a) `instantiate_return_ty` (`src/check/mod.rs:4497-4534`) discards the substitution returned by `infer_substitution` at **`:4530`** and calls `ret.erase_params` with an unbound `sigma`, so every `Ty::Param` falls through to `Unknown` (`src/types.rs:244`). (b) `Expr::Method` inference (`:4101-4115`) passes an `args` slice that excludes the receiver, so an `impl<T>` parameter occurring only in the receiver can never be bound | `check` accepts what `run` rejects: `fn id<T>(x: T) -> T` called as `id(1)` binds to a `string` annotation at exit 0, then fails at runtime with `E2003`. Every generic free-function *and* method call is affected, and the unsoundness extends to nested returns (`[T]`) | Escalated as RFC 0005 (`NBL-012`). A two-part repair (+53/−2) is validated — 20 probe rows, each mutation independently re-introducing the bug, `1477` tests and clippy green, all **125** tracked `.aura` files byte-identical in output and status — and is **withheld, not committed**; `src/check/mod.rs` is byte-identical to baseline until the RFC is accepted |
| R-14 | **Subscript inference never type-checks the operand** (§1.7.3). `Expr::Index` inference does not consult the operand type; the statement-mode arm at `:6868-6880` does check `contains_none()`, so the hole is inference-specific | `let a = 5` then `a["k"]` is `check` exit 0 / `run` exit 1 (`E3001: cannot index int with string`). Same for `bool` and `float`; reproduces with **no** generic call in the program | Out of scope for RFC 0005 (§7.2). Needs its own finding; deferred, not bundled |
| R-15 | **Equality is never type-checked and does not fail at run time** (§1.7.3). `==`/`!=` accept operands of any two types and evaluate to a boolean. Ordered comparison *is* checked, so the hole is specific to equality | `print(1 == "s")` is exit 0 under `check` **and** `run` and prints `false`; `print(1 != "s")` prints `true`. A **silent wrong answer**, not a loud runtime failure — strictly worse than `R-14` | Out of scope for RFC 0005 (§7.2). Arguably the highest-severity open defect in this ledger; deferred only because it is a different subsystem, and called out as such rather than folded into D1 |
| R-16 | **The AIS/0.1 protocol over-promises on eighteen distinct points** (§1.1–§1.14 of RFC 0006, findings A–R). Load-bearing: (a) `Symbol.range` carries only the leading keyword's span — 6.4× aggregate understatement, with `pub` fields and `mut` params affected too (`A`, `I`); (b) three of eleven `SymbolKind` variants are unconstructible while `AIS.md:99-101` documents all eleven (and `Variant`/`Field`/`TypeParameter` can never appear as a `kind`), which breaks the minor-version promise at `AIS.md:53-55` twice (`H`, `F`); (c) `Slice.diagnostics` is promised at `src/ais.rs:848-853` and `:868` and populated nowhere — all three construction sites pass `Vec::new()` — so a slice over a failing program is indistinguishable from a clean one (`L`); (d) the AIS document path never runs the resolver, so `E2019` is dropped at exit 0 while `E2018` and `E2023` survive carrying the wrong code `E2003` (`M`, `N`, `O`); (e) the CLI exposes 3 of 7 operations and `aura://schema` advertises 6 of 7 tools and 3 of 7 delivery operations, both from one hardcoded literal (`B`, `D`); (f) `tests/ais_differential.mjs` asserts in its own docstring that it "runs in CI" while nothing invokes it (`E`) | Any agent trusting AIS as its semantic authority is misled about location, capability, slice completeness, and program validity — and the wrongness is invisible, because every defect is a passing exit code. `L` and `M` compound: a rejected program yields a clean, diagnostics-free document. `E` means the one harness built to detect this class has never run in CI, which is the rule-7/rule-8 failure this campaign exists to catch | Escalated as RFC 0006 (`NBL-014`, merged via PR #55 → `d03d2718`) with three decisions still open in §10.6. The `range` extent repair is validated by mutation and **withheld, not committed**; `src/ais.rs` and `src/mcp.rs` are byte-identical to baseline |
| R-17 | **An `os`/environment surface would remove a security property the project currently holds unconditionally** (§10.7 `D2`). `G06`'s "Host can deny env" and "no secrets in output" are green **only by absence** today: `grep 'name: "env"' src/stdlib/signatures.rs` returns nothing and `docs/security/TRUST_BOUNDARIES.md:30` confines environment variables to the `py` feature, which `:38` records as verified-absent in CI. Any Aura-visible `env` makes both clauses live obligations across four output surfaces — diagnostics, AIS documents, MCP responses, and the Playground — and RFC 0006 §1.13 proved the AIS path runs a reduced pipeline, so a value that reaches a diagnostic message can reach that pipeline too | 3 of the 5 `G06` clauses are green only because the surface does not exist; a fourth holds on evidence and the fifth is unevaluable, so introducing the surface can only narrow the guarantee | Do not choose `D2` by default; if `D2` is anything but "nothing", treat secret redaction as a new contract with its own RFC, and never let `env` reach a diagnostic message |
| R-18 | **A database capability cannot be represented in any existing IR.** `PendingEffect` has exactly one variant, `Http(HttpRequest)` (`src/host.rs:39-42`), so a resumable connection requires a breaking enum change; `pub enum Value` (`src/run/value.rs:172-206`, 15 variants) and `pub enum TypeExpr` (`src/ast/mod.rs:18-53`, 14 variants) have no opaque or handle form, so a connection cannot cross the stdlib boundary as a value at all; and `Returns::Dynamic` resolves to `Ty::Unknown` (`src/stdlib/signatures.rs:210`) with zero consumers in `src/check/mod.rs`, so a dynamically returning builtin is entirely unchecked | A driver cannot be prototyped behind the current boundary without either a new `Value` variant, a new `TypeExpr` variant, or a handle-free API restricted to opaque integer ids — and the third option silently discards the type safety §8.3 assumes | Do not choose `D1` by default; if a driver is authorized, the `PendingEffect`/`Value`/`TypeExpr` extension is a separate accepted RFC with its own ABI review, and no driver may ship while `E5002` remains a hard fatal rather than a catchable result |

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

**Census — eighteen outstanding decision items across eight decision
packages.** Each package is a `### §10.N` block and a package may carry more
than one question, so package count and item count differ:

| Package | Lettered labels | Unlabelled | Items |
|---|---|---|---|
| §10.1 | — | 1 | 1 |
| §10.2 | `D1a`, `D1b`, `D1c` | — | 3 |
| §10.3 | `D1`, `D2` | — | 2 |
| §10.4 | `D3` | 1 | 2 |
| §10.5 | `D1` | — | 1 |
| §10.6 | `D1`, `D2`, `D3` | — | 3 |
| §10.7 | `D1`, `D2`, `D3` | — | 3 |
| §10.8 | `D1`, `D2`, `D3` | — | 3 |
| **Total** | **16** | **2** | **18** |

The figure "seven human decisions" that appeared in earlier state documents was
incremental drift. At the time it was first written the tree carried ten
lettered labels and two unlabelled items, twelve in all, and it matched neither
figure; with §10.7 added the census is thirteen lettered plus two, fifteen in
all, and with §10.8 it is sixteen lettered plus two, eighteen in all. It is
retired here. Eighteen is not a target; the census is
derived mechanically from the `D`-labels actually present, and it must be
recomputed the same way whenever a `§10.N` block is added.

The counting rule is fixed so that it is reproducible rather than recalled: a
package's items are the **distinct `D<n>` tokens** appearing anywhere in its
block (so `D1a`, `D1b`, `D1c` are three), plus one where the package carries a
decision with no `D`-label at all (`§10.1`), plus one per **`Open item <N>`**
heading (`§10.4`). Applying that rule to the six packages that predate §10.7
reproduces the previously published `1, 3, 2, 2, 1, 3` = 12 exactly, which is
the evidence that the rule is the one the table has always encoded rather than a
new definition fitted to the new section.

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

---

### §10.4 — RFC 0004: the diagnostics contract (`NBL-013`)

RFC 0004 was authored, independently re-verified against the live binary, and
merged as PR #51 → `2df1c322` (21/21 checks green). It carries a five-point
contract (§2), a 23-row evidence table (§6.1), a nine-row mutation table (§6.2)
and twenty required regression tests (§6.4). It changes no grammar. Two items
require a human decision.

**Decision D3 — columns are counts of characters, not visual width.**
`line_col` (`src/error.rs:203-229`) counts Unicode scalar values, which is
exactly what `LANGUAGE_SPEC.md:177` mandates. It is conformant, not defective.
But on a terminal the pointer under a diagnostic can land in the wrong place:
re-measured against a stated display-width model (East-Asian `W`/`F` = 2,
combining = 0, tab stops of 4), the printed column diverges from the visual one
by `0 / −3 / 0 / −1 / −2 / +1` for ascii, tab, CRLF, emoji, CJK and a
decomposed combining sequence. The **delta is stable; the absolute column is
not** — it depends on the source line's shape, so RFC 0004 publishes the delta
and pins the shape it measured.

The cost is real: visual columns would require every consumer to agree on a
width table, and `E1001`-style messages quote the source byte. The three lawful
outcomes are:

| Option | Meaning | Cost | Risk |
|---|---|---|---|
| 1 | Keep character columns (status quo); document the divergence | None now | Terminals stay misaligned for wide characters; the spec already permits it |
| 2 | Add visual columns as a *presentation-only* field, keeping the semantic column authoritative | Medium — one width table shared by four consumers | Two column numbers in one report; AIS and the Playground must be updated together or they drift |
| 3 | Change `line_col` itself to visual width | High | Breaks `LANGUAGE_SPEC.md:177`; needs a spec amendment and a migration note |

**Open item 13 — non-ASCII module directories are silently skipped.** A tree
`©/c.aura` with an owner `©.aura` is not discovered at all: `aura check` exits
`0` with no output, whereas the same tree under `mm/` reports
`mm/c.aura:1:21: error[E1001]`. This is **specified** behaviour —
`docs/FILESYSTEM_MODULES_DESIGN.md:594` requires logical module identifiers to
be byte-exact ASCII — so it is *not* a defect. The gap is that the frozen
outcome matrix at `:774` has no row for it and no user-facing page says it, so a
user sees a silent success. This is a documentation-gap decision, not a code
decision, and it is escalated rather than resolved.

**Blast radius.** Nothing in this RFC blocks G03. It does gate G18's snapshot
and `aura explain` work, because §6.4's twenty tests and §6.2's mutation rows
are the enforcement mechanism, and `aura explain` additionally needs a spec
amendment (§28.3 lists five commands; `src/main.rs:26` cites a `§47/§48` that do
not exist — the spec ends at `## 36`). The format-injection repair (`R-12`) is
withheld because gate G02 forbids changing the affected surface before
acceptance; the concrete fix sites are enumerated in RFC 0004 §6.2.

---

### §10.5 — RFC 0005: inference and error propagation (`NBL-012`)

RFC 0005 was authored from a defect found while opening `NBL-012`. It carries
one decision (**D1**), a 21-row probe matrix re-executed end-to-end against two
saved, sha-pinned binaries, a mutation table, and ten required regression tests.
**Grammar delta: none.** It is **merged as PR #53 → `87cea70e`**; merge records
the proposal only, and gate G02 still forbids coding the surface, so the
validated patch is held as a file, not a commit.

**D1 — authorize the two-part checker repair.** `instantiate_return_ty`
discards the substitution it computes, so every `Ty::Param` in a generic return
erases to `Unknown`, which `Ty::compatible_with` accepts against any annotation.
Repairing that alone is insufficient: `Expr::Method` never binds `impl<T>`
parameters from the receiver either. Both parts are independently load-bearing —
each was mutation-proved by removal. The patch is **+53/−2** in one file,
`src/check/mod.rs`, currently byte-identical to baseline.

**Why D1 is a decision rather than a bug fix.** The spec permits the current
behaviour in exactly one place — §36.3 (`:4048-4059`) says an *unbound* parameter
in a value position is `Unknown` — and the repair only changes programs where a
parameter **is** bound by the call. So the repair is spec-consistent. But it is
strictly more rejecting: any program that relied on `Unknown` satisfying an
annotation starts failing. That is a real compatibility break for downstream
users, and it is the human's call, not an agent's.

**Blast radius.** Blocks `NBL-012` and every later surface whose types assume a
checked generic return. The full corpus is output-neutral (**125** files,
byte-identical, `108`/`17` status split unchanged), and the suite stays
`1477 passed / 0 failed` with clippy at 0, so the risk of the patch *itself* is
low; the open question is policy, not engineering.

**Two unrelated holes were surfaced and deliberately excluded.** §7.2 of the RFC
records them; the ledger carries them as `R-14` (indexing) and `R-15` (equality).
Neither reproduces any differently with generics removed, so neither is a
residual of D1. `R-15` in particular is a **silent wrong answer** — `1 == "s"`
evaluates to `false` with no diagnostic under either `check` or `run` — which
makes it plausibly more urgent than D1, and it is called out here rather than
quietly deferred.

### §10.6 — RFC 0006: the AIS agent-native capability set (`NBL-014`)

Promotes **`R-16`**. Recorded from `docs/rfcs/0006-ais-agent-native-capabilities.md`,
which is **Proposed and not accepted**. The RFC itself is merged as PR #55 →
`d03d2718`; merging a proposal records the evidence and the questions, it does
not answer them. Eighteen defects were reproduced, labelled
`A`–`R`; each was re-verified in ten independent probe sweeps, each sweep in a
fresh temporary directory, and none is read off the source alone. The RFC is
therefore evidence, not a proposal to be trusted on inspection.

**What is actually broken.** Three defects account for most of the harm, and all
three are invisible because each produces a *passing exit code*:

- **`A`/`I` — `Symbol.range` spans only the leading keyword.** A ten-declaration
  fixture reports **39** bytes of extent where the true extents total **249** —
  a **6.4×** understatement — and `fn helper` reports `2..4` against a true
  `141..182`. `pub` struct fields report `pub` and `mut` parameters report `mut`,
  because `src/parse/mod.rs:1365` and `:1426` capture the span **before**
  consuming the modifier. `line`/`column` are correct; only the extent is wrong,
  so nothing downstream notices.
- **`L` — `Slice.diagnostics` is promised twice and populated nowhere.** The type
  doc at `src/ais.rs:848-853` says a slice carries "the diagnostics that mention
  it or its dependencies, bounded by `budget`"; the field doc at `:868` repeats
  it. All three construction sites pass `Vec::new()`. A slice over a program that
  fails `aura check` is byte-identical in shape to a slice over a clean one.
- **`M`/`N`/`O` — the AIS document path never runs the resolver.** `aura check`
  goes `compile_file_with_mode` → provider → `resolve_sourced` → checker;
  `aura ais` goes `parse` → `ais_snapshot_of` → `checked_document`, which calls
  only `Checker::module_narrowings`. The consequence is not merely a gap: `E2019`
  (unknown module) is **dropped entirely at exit 0**, while `E2018` (private
  access) and `E2023` (reserved namespace) survive **carrying the wrong code**,
  `E2003`. Two of the six resolver-only codes are therefore mis-reported, not
  missing.

**The CI finding is the sharpest one.** `tests/ais_differential.mjs` is 329 lines
and its docstring at `:21-22` states "This runs in CI with no model." A repo-wide
grep for any invocation returns **zero** hits — not in `.github/`, not in
`scripts/`, not in any `package.json` (no `package.json` exists). Executed by
hand it passes: `EXIT=0`, four task families, `raw=91 tok` against `ais=2422 tok`,
`Fact carriers: raw=4, ais=12`, and its own final line `EXTERNAL DIFFERENTIAL: NOT
EXECUTED`. This is the exact rule-7/rule-8 failure this campaign exists to catch —
a suite present in the tree that no runner owns.

**Decisions requested — three, none resolvable by an agent.**

**D1 — the `range` contract: widen the extent, or document the narrowing.**
Option A widens `Symbol.range` to the full declaration extent by carrying an end
from the parser (a source change, two files). Option B keeps the emitted shape and
amends `AIS.md:96-101` to say the range covers the leading keyword only (a
documentation change, no code). Option C adds an explicit `keyword_range` and
leaves `range` untouched. A and C are honest; B is cheapest and is the only one
that requires no production change under G02 — but it makes `range` useless for
the navigation an agent actually wants, and it would have to be repeated for
`pub`/`mut` prose. **No option is selected here.**

**D2 — `Slice.diagnostics`: populate it, or delete the promise.** Option A
threads the diagnostics the slice already has in scope into the three
construction sites — `src/mcp.rs:330-345` passes the whole `doc`, so the
information is present and simply discarded. Option B removes the field and
rewrites both doc comments. Option C leaves the field but makes it explicitly
always-empty. A is the only option that matches what the docs say today; B and C
are both honest retractions. Note the migration consequence: dropping the field
violates the "a producer never removes a field within a minor version" promise at
`AIS.md:53-55`, so B requires a version bump, while A does not.

**D3 — the resolver gap and the mis-coded diagnostics.** Option A routes AIS
through `resolve_sourced` so the document sees what `aura check` sees. Option B
keeps the current pipeline and declares in `AIS.md` that AIS reports
checker-stage diagnostics only, with `E2019` and the five other resolver-only
codes explicitly out of scope. Option C fixes only the mis-coding of `E2018` and
`E2023` and leaves `E2019` dropped. A is the only option that makes the exit code
truthful; B is a documented narrowing; C is a partial fix that leaves the most
common agent-facing failure — an unresolvable `use` — silently clean. The RFC
notes that A also changes `aura ais` exit codes for programs that currently pass,
which is a compatibility break that belongs to the human, not to an agent.

**What is withheld.** A validated repair for D1's Option A exists — a parser-side
end-span capture plus the projection at `src/ais.rs:485-511` — and was proven by
mutation in both directions, with the suite at `1477 passed / 0 failed` and
clippy at 0. It is **not committed**: gate G02 forbids coding the affected
surface before the RFC is accepted. `src/ais.rs` and `src/mcp.rs` are byte-identical
to baseline.

**One correction to the record.** The first draft of this section and of the RFC
cited a `RESOLVER-ONLY CODES: 5` figure. The machine census over `src/error.rs`
identifies **six** resolver-only codes — `E2019`, `E2020`, `E2021`, `E2022`,
`E2023`, `E4020` — and the RFC was corrected before commit. A second defect, a
fabricated `E3009`, was removed entirely rather than replaced.

### §10.7 — RFC 0007: project structure, `init`, and the `os` capability (`NBL-015`)

**The gate this package answers to.** `G06` reads: *deterministic reversible
loading, module compatibility, Host can deny env, no secrets in output, cross-OS
tests where relevant*. The RFC treats each clause as a requirement, not as a
description of completed work.

**Why this package is different.** Every other G02 package found defects in an
existing surface. This one finds that **the surface does not exist**, and that
its absence is what satisfies the gate. `G06`'s five clauses, measured:

| Clause | State | Why |
|---|---|---|
| deterministic reversible loading | **green (on evidence)** | discovery is extension- and shape-filtered; `Aura.toml` at the source root is inert (probed `RC=0`) |
| module compatibility | **green (by absence)** | no configuration surface exists, so none can break it |
| Host can deny env | **green (by absence)** | no environment access exists to deny |
| no secrets in output | **green (by absence)** | nothing prints environment or configuration |
| cross-OS tests where relevant | **unevaluable** | blocked on `D3` (open item 5); no project/config surface exists to test |

Three of the five are therefore green *by absence*, a fourth — deterministic
reversible loading — is green **on evidence**, and the fifth is **unevaluable**
until `D3` resolves. That is the finding, and it inverts the usual risk: the
work
is not repair, it is the cautious introduction of a surface that can only
**remove** guarantees.

**The unsound spelling.** The backlog says `std::os`. Probed:

| Program | Verdict |
|---|---|
| `use std::os` | `RC=1`, ``error[E2019]: unknown module `std` in `use` `` |
| `use Std::os` | `RC=1`, ``error[E2019]: unknown module `Std` in `use` `` |
| `use Std::Math` | `RC=1`, ``error[E2019]: unknown module `Std` in `use` `` |

The third row is decisive, because `Std::Math` is the spelling this campaign's
own earlier probes used and it does not resolve either. `grep -rn '"Std"' src/`
is **empty**. `src/stdlib/mod.rs:20` `builtin_names()` derives the entire
standard library from `signatures::builtins()` — a **flat prelude of 91
functions**, not a module namespace. `std::os` is not a module that has yet to
be written; it names an architecture Aura does not have.

**D1 — what `Aura.toml` may contain.** Option A: a marker file, read by
nothing. Option B: project metadata (name, version, authors), read by tooling and
never by the compiler. Option C: compiler-affecting configuration (source root,
features, lint levels). The RFC recommends A or B and holds that C requires its
own RFC, because C is the only option that can falsify the "module compatibility"
clause that is currently green by absence.

**D2 — whether an `os` surface exists, and in what form.** Option A: nothing;
`G06` stays vacuously satisfied. Option B: Host-only — the Host ABI can supply
values, but no Aura program can read the environment. Option C: prelude
functions (`env(name) -> string?`), consistent with the flat prelude. Option D: a
real module namespace — rejected *on evidence*, since the namespace machinery
does not exist. Both C and D **remove** a property the project holds
unconditionally today (`TRUST_BOUNDARIES.md:30` confines environment variables to
the `py` feature). If C is chosen, `G06`'s "no secrets in output" stops being
vacuous and becomes a four-surface obligation: diagnostics, AIS documents, MCP
responses, and the Playground. RFC 0006 §1.13 established that `aura ais` runs a
reduced pipeline; an `env` value that can reach a diagnostic message can reach
that pipeline too.

**D3 — what `aura init` writes, and how it refuses.** `init` is the first
command that would **write to disk**: `grep -rn 'create_dir' src/` returns no
matches. Four sub-questions are bundled — which files (`Aura.toml` alone, or
also a `main.aura` skeleton that must check cleanly under eager discovery,
`:399`), what happens when the target exists, what an unwritable directory
reports (a diagnostic naming the path, never a panic), and whether the contract
test runs on the whole CI matrix. The RFC recommends refuse-if-exists and
`Aura.toml` only, consistent with this repository's standing preference for
preserving existing work over convenience.

**Dependency between the decisions.** `D2` (C) is incoherent without `D1`: a
denial policy needs somewhere to be declared. `D3` is under-specified without
`D1`: `init` cannot write a file whose contents are undecided. **`D1` is the
root decision.** If `D2` resolves to A, `D1` shrinks toward a marker file and
`D3` to writing one file.

**What is withheld.** Nothing was written to production source for this package —
no repair was possible, because there is nothing broken. The single codebase
observation worth carrying forward is that `COMMANDS` is duplicated
(`src/main.rs:28` with 7 entries, `:39` with 5), so `init` must be added to both
or the two builds will disagree about their own help text.

**Corrections to the record.** Two self-caught errors in the RFC's own drafting
are recorded in its §0: a fabricated cross-reference to "RFC 0006 §1.5" (the
`Std::Math` evidence is §1.13), and the observation that `E2019` is not a
literal in `src/error.rs` but `UNKNOWN_MODULE: u16 = 2019` rendered through
`format!("E{:04}: ...")`. Both are cited correctly in the merged text.

### §10.8 — RFC 0008: Host database capability and the protected profile
(`NBL-016`)

`G07` is a five-clause acceptance line and, like `G06`, most of it is satisfied
by absence rather than by construction. The RFC's per-clause reading:

| Clause | Status today | Why |
|---|---|---|
| `ADR per driver/MSRV/TLS` | Absent | `docs/adr/` holds 6 ADRs; no MSRV ADR and no TLS ADR. ADR-0005 is missing from the index table and appears only in a bullet list |
| `Host boundary compiles without drivers` | Greenfield, structurally ready | The `Host` boundary is one trait, `src/host.rs:157-228`, with 10 methods (8 required, 2 defaulted) and 10 implementors |
| `equivalent E5002 capability probes` | Template exists and is proven | The `http-api`/`http` feature split is exactly this shape, proven end-to-end |
| `resource and attack-surface tests` | Absent | No resource model exists; `CRITICAL_SYSTEM_PROFILE.md` §4 has no HTTP row |
| `no real database without authorized test credentials` | Vacuously true, needs an owner | No database code exists — there is nothing to guard yet |

**The template is the finding.** Building with
`--no-default-features --features cli,json,http-api` succeeds, exposes
`http_get`/`http_request` to the checker, and then denies them at run time with
`main.aura:1:19: error[E5002]: this host has no network capability (HTTP is
unavailable)`. `cargo tree --no-default-features --features http-api -e
features | grep -ci ureq` returns `0`, so `http-api` adds a signature surface
without adding a native dependency; `http = ["http-api", "dep:ureq"]` is what
adds the real implementation, and `StdHost::http_request` is itself
`#[cfg(feature = "http")]`, so an `http-api`-only build inherits the trait
default denial even in `StdHost`. A database capability should be built the same
way and not invented from scratch.

**What blocks a driver today.** Four independent representation gaps in the IR,
all verified by probe rather than by reading:

1. `PendingEffect` (`src/host.rs:39`) has exactly one variant,
   `Http(HttpRequest)`, so a resumable database operation requires a breaking
   enum change;
2. `Ty` (`src/types.rs:68`, 17 variants) has no bytes member;
3. `Value` (15 variants, `src/run/value.rs:172-206`) has no `Bytes`, so binary
   crosses as `[int]` (`src/stdlib/ext.rs:651-663`);
4. `TypeExpr` (14 variants, `src/ast/mod.rs:18-53`) has no opaque or handle
   form, so a connection cannot cross the stdlib boundary as a value.

Two further constraints lie outside the IR. `Returns::Dynamic` maps to
`Ty::Unknown` and has zero consumers in `src/check/mod.rs`, so a dynamically
returning builtin is unchecked — and a probe confirmed it:
`let x: int = read_file("a.txt")` passes `check` with `CHK=0` and then fails at
run time with `E3001`. Separately, `E5002` is a **hard fatal** diagnostic, not a
catchable value; `tests/session.rs:393` documents that it behaves like any
other fatal diagnostic — `finally` runs, `catch` does not — so a database API
whose `Result<T, DbError>` (§8.3) is meant to be consumed cannot route a
*capability denial* through that same `Result`. The denial is always fatal.
This is the single most consequential constraint on §8.3 and it is carried as
`R-18`.

**The document drift.** Two capability documents are stale, and a third is
mis-cited rather than stale:

- `CAPABILITY_MODEL.md` §2's 13-row table has no `http_request` row, and its
  `network` row still reads "not exposed"; it also cites `src/host.rs:78` as
  "the `Host` trait" when `:78` is inside `HostError::into_diag` (the trait is
  `:157`).
- `CRITICAL_SYSTEM_PROFILE.md` §4's 15-row resource inventory has no HTTP row.
- `PERFORMANCE.md` is **not** stale — it is a 131-line timing-budget document
  containing **zero** occurrences of `resource` at any casing. The mis-citation
  is in `CRITICAL_SYSTEM_PROFILE.md:19`, whose "Bounded resources" row points at
  "the resource inventory in `docs/engineering/PERFORMANCE.md` and §4 below";
  only the second half of that reference resolves.

**`D1` — what a database capability is allowed to be.** Four options: A, a
feature-gated builtin surface mirroring `http-api`, denied by default at run
time; B, a `Host` trait extension only, with no Aura-visible builtins until a
driver lands; C, a driver-agnostic connection abstraction in `std`; D, defer
entirely. The RFC recommends **A**, because it is the only option whose
authority model has already been proven in this repository and whose blast
radius is a signature table plus a `#[cfg]`, both under the `E5002` template.

**`D2` — the isolation, credential, and resource boundary.** Three readings:
(a) the protected profile means the capability is denied unless a host opts in,
with no Aura-visible configuration; (b) it means an `Aura.toml`-declared
allow-list; (c) it means a separate binary mode. Recommendation is **(a)** —
it needs no configuration parser (`grep -rn 'struct .*Config\|Config::' src/` is
empty) and no file format (there is no `toml` dependency), and it preserves the
property that a default build has no database surface at all.

**`D3` — what the capability tests must prove.** Three readings: (a) `E5002`
equivalence probes only, matching the HTTP precedent; (b) those plus
attack-surface tests for injection and resource exhaustion; (c) those plus a
cross-OS matrix. Recommendation is **(a)** for the same reason `G06`'s cross-OS
clause is unevaluable today: there is no driver to test.

**Dependency between the decisions.** `D2` and `D3` are both downstream of `D1`:
there is no boundary to describe until it is decided what shape the capability
takes. **`D1` is the root decision.**

**What is withheld.** No production source was modified. No driver, no
capability signature, no `PendingEffect` variant, no resource model, and no ADR
was written — every one of those is downstream of `D1` and forbidden by `G02`
until the RFC is accepted.

**Corrections to the record.** Nine drafting errors were caught in the RFC's own
text and are recorded in its §0 rather than silently fixed, including the two
published claims this campaign is most likely to have propagated: that
`PERFORMANCE.md` contains two occurrences of `resource` (it contains zero, at
baseline and in all six commits that ever touched it) and that `http_request` is
the only defaulted `Host` method (`should_cancel` is the other). Neither figure
escaped this RFC.

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
