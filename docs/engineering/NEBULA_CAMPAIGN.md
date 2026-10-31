# Directive 008 — NEBULA 0.3.2 Campaign

Status: **G00 PASS** — baseline and prerequisite inventory merged as
`7bd1d6fb` (PR #43, 21/21 checks green). **G01 is the only authorized wave**;
no implementation is authorized before G02.

Baseline: `c4f769077c769483963a0f2b51a789b00472e4cf` on `rewrite/v3-rust`
(local == `origin/rewrite/v3-rust`, tree clean at campaign start). G00 landed
on top of it as `7bd1d6fb`.

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
| G01 | `IN_PROGRESS` — dependency and contract table |
| G02–G19 | `NOT_STARTED` |

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
| NBL-010 | G02 | RFC: `Result` and recoverable error semantics | P0 |
| NBL-011 | G02 | RFC: `try` as expression without ambiguity | P0 |
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
| NBL-030 | G03 | `Result<T,E>` as a library/builtin generic enum | P0 |
| NBL-031 | G03 | `try` expression parsing and checking | P0 |
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
| R-04 | No authorized database credentials are available | G10–G16 cannot claim real integration | Report unexecuted environments honestly; a cell is never `PASS` by compilation alone |
| R-05 | Four backends have genuinely different semantics | False equivalence would be a correctness and security defect | Declare `capability-not-supported` explicitly; never silently diverge |
| R-06 | `py` runs a full CPython process with full authority | In-process CPython cannot be SQLGuard-certified | State the boundary; `production-enforce` must refuse known bypass |
| R-07 | Green CI proves only the wired subset | A passing badge could hide unwired suites | Enumerate invoked test files per job at G18 |
| R-08 | Campaign breadth could encourage splitting into giant PRs | Unreviewable changes | Short-lived branches, one coherent surface per PR, auditable bodies |
| R-09 | Partial P1 delivery could be overstated as stable | Misleading release notes | P1 absent full proof ⇒ `DEFERRED_WITH_APPROVAL`; changelog never marks experimental work stable |

---

## 7. Evidence criteria

A wave may be marked `PASS` only with all of:

1. the focused commands actually executed, quoted with their results;
2. no regression in previously passing suites;
3. normative documentation updated where semantics changed;
4. the diff reviewed by an agent that did not author it;
5. remote CI green for the integrated commit, or an explicit statement that CI
   did not run.

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

The repository's custom Aura agents are used: `Aura Orchestrator`,
`Aura Scout`, `Aura Spec Auditor`, `Aura Deep Auditor`,
`Aura Adversarial Reviewer`, `Aura Artifact Verifier`, `Aura Implementer`.
No agent may approve its own code as sole reviewer.

---

## 10. Blocking policy

A wave is blocked, with a stable tree preserved and an issue or RFC opened with
alternatives, when a requirement is impossible because of MSRV, a cross-database
semantic discrepancy, an isolation risk, OOP or parser breakage, a Native/WASM
regression, or a failing security test. Replacing a problem with a `TODO` and
marking the wave `PASS` is forbidden. A genuine specification choice produces a
decision package rather than a silent resolution.

---

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
