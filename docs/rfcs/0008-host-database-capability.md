# RFC 0008 — Host database capability and the protected profile

- **Backlog item:** `NBL-016` (`| NBL-016 | G02 | RFC: Host database capability and protected profile | P0 |`)
- **Gate:** `G02` — every P0 RFC is authored and escalated before any production
  surface changes.
- **Status:** Proposed. **Not authorized for implementation.**
- **Baseline:** `rewrite/v3-rust` @ `edcce6c7a587f8f47cdcf2057c9df400c9b93420`.
- **Grammar delta:** none. §3 records why a database capability needs no new
  grammar, and shows the two places where existing grammar is the actual
  constraint.
- **Decisions escalated:** `D1`, `D2`, `D3` (§2).

This RFC is bound by the `G07` acceptance line, quoted verbatim from
`docs/engineering/NEBULA_CAMPAIGN.md`:

> **G07** | Host Database, driver matrix, resource model | ADR per driver/MSRV/TLS;
> Host boundary compiles without drivers; equivalent `E5002` capability probes;
> resource and attack-surface tests; **no real database without authorized test
> credentials**

Every clause of that line is treated below as a *requirement to be satisfied or
explicitly refused*, never as a description of work already done.

## 0. Provenance

This RFC was written from repository evidence only. The campaign document named
three source artifacts in its §0.4 trigger check —
`AQL_Nebula_RFC_Draft.md`, `NEBULA_SQLGUARD_CONTRACT.md`, and
`Aura_0.3.2_Nebula_Inventario_Geral.md` — and **none of them exists in the
repository**. Everything below is reconstructed from `src/`, `docs/`, `tests/`,
and live probes against a locally built binary. Corrections made while writing
are listed here rather than silently applied.

1. **A first `Host` implementor census returned 9.** The pattern
   `grep -rn 'impl Host for'` misses one implementor that is spelled with a
   fully-qualified path, and misses a nested one. The corrected census is **10**
   (§1.3). The wrong figure never left this RFC.
2. **A first `http_request` override census returned "8 overrides / 2 inherit".**
   That double-counted the trait's own default method and missed two
   implementors entirely. The correct reading is **6 textual occurrences of
   `fn http_request`**, of which **one is the trait default**, leaving **5 real
   overrides**, and **5 implementors inherit** the default (§1.4).
3. **A capability probe initially reported `read_file` as "available"** because
   a missing file returns `none` rather than an error. That is a real property
   of the filesystem surface, not a measurement artefact, but it means
   `read_file` cannot be used as an availability probe. The corrected probes use
   `http_get`, which is genuinely absent from a default build (§1.5).
4. **The claim that `args()` redacts credentials was falsified.** A probe printed
   `["--******"]`; writing the same value to a file and dumping it with `od -c`
   showed `--api-key=PROBE-PLACEHOLDER` verbatim. The masking was performed by the tool
   harness, not by Aura. Aura has **no** credential masking (§1.8).
5. **A first credential probe polluted three `.aura` files into one directory**
   and produced `E2003: undefined function http_get` for an unrelated reason.
   Every probe in this RFC was re-run with exactly one `.aura` file per
   directory, per the campaign's established rule.
6. **§1.2 first claimed `PERFORMANCE.md` "is cited by
   `CRITICAL_SYSTEM_PROFILE.md` §4" and "contains **two** occurrences of
   `resource`/`Resource`".** Both halves were wrong. The citations are at `:19`
   (a row of §1) and `:53` (a constraint in §3); neither is inside §4, which
   spans `:60`–`:82`. And `resource` occurs **zero** times at any casing —
   verified by `grep -ci`, by `grep -oi | wc -l`, by a Python case-insensitive
   scan, and by running the same count against all six commits that have ever
   touched the file. `:19` is itself a *dangling half-reference*: it names "the
   resource inventory in `docs/engineering/PERFORMANCE.md` **and §4 below**",
   and only the second half resolves. §1.1, §1.2, §1.6, §6.1 row 34, and open
   item 7 were rewritten. The wrong figure never left this RFC.
7. **§1.3 first said `http_request` is "the only one" of the ten `Host` methods
   with a default body.** Two do: `should_cancel` (`src/host.rs:207`, returning
   `false`) and `http_request` (`:223`). Only the latter is a denial. Eight of
   the ten methods are required declarations. §1.3 was rewritten.
8. **The §1.1 gate table first said two of the six ADRs are unindexed.** Exactly
   one is: `docs/adr/README.md`'s table has five rows (0001, 0002, 0003, 0004,
   0006), and `0005-keystone-collection-model.md` appears only in the "Accepted
   ADRs" bullet list at `:36`. §1.2 had the figure right; §1.1 disagreed with
   it. §1.1 was corrected to match.
9. **The credential examples used a password-shaped literal.** It was a
   placeholder, never a real secret, but it trips secret scanners and it made
   the terminal tool-harness mask its own display, which corrupted one of my
   measurements. Every example now uses `--api-key=PROBE-PLACEHOLDER`.
10. **§2.3 first reported the `protected profile` census as "five hits" with no
    frame.** Five is the *baseline* figure and it is correct, but it was stated
    as a bare total, so the statement was already false where it was written:
    eleven occurrences sit in this RFC and three more arrive with the batch's
    other edits, taking the working-tree figure to nineteen. The sentence now
    names the commit (`edcce6c7`), names the two files the five live in, and
    says why the count is pinned rather than live.

## 1. Motivation

### 1.1 What `G07` asks for, and which parts already hold

`G07` is a five-clause acceptance line. Its clauses are not equally satisfiable
today, and the difference is the whole subject of this RFC.

| Clause | Status today | Evidence |
|---|---|---|
| `ADR per driver/MSRV/TLS` | **Absent** | `docs/adr/` holds 6 ADRs; no MSRV ADR, no TLS ADR. ADR-0005 is missing from the index table (§1.2) |
| `Host boundary compiles without drivers` | **Greenfield, but structurally ready** | No driver exists, so the clause is true by absence; the `Host` boundary is a single trait (§1.3) |
| `equivalent E5002 capability probes` | **Template exists and is proven** | The `http-api`/`http` split is exactly this shape, proven end-to-end (§1.5) |
| `resource and attack-surface tests` | **Absent** | `CRITICAL_SYSTEM_PROFILE.md` §4 has no HTTP row; its §1 "Bounded resources" row names a `PERFORMANCE.md` inventory that does not exist (§1.6) |
| `no real database without authorized test credentials` | **Vacuously true, needs an owner** | No database code exists; there is nothing to guard *yet* |

So `G07` is not a description of a partially built database layer. It is a
requirement to build one **in a shape that the current architecture supports**
and to leave the parts that the architecture does not support explicitly
refused.

### 1.2 Finding — two stale capability documents, a mis-cited third, and a missing ADR index row

The capability documents disagree with the code in ways that matter to a
database driver contract, because a driver is where resource limits, TLS, and
credential handling all converge.

**`docs/engineering/CAPABILITY_MODEL.md` (5 201 bytes)** presents a 13-row
capability inventory. Its `network` row reads **"not exposed / not exposed"** in
both default columns. That was true before the HTTP capability landed and is
false now: `http_request` and `http_get` exist as builtins under the `http-api`
feature, and `StdHost` provides a real implementation under `http`. The table
has **no `http_request` row at all**, and does not mention `HTTP_METHODS`,
`MAX_HTTP_BODY_BYTES`, `MAX_HTTP_TIMEOUT_MS`, or `PendingEffect`.

The same file cites "the `Host` trait (`src/host.rs:78`)". Line 78 is inside a
`HostError` construction, not the trait; the trait is at `src/host.rs:157`.

**`docs/engineering/CRITICAL_SYSTEM_PROFILE.md` §4** is a 15-row resource-limit
inventory with **no HTTP row** — no body cap, no timeout cap, no method
allow-list. A database capability introduces strictly more resource dimensions
(pool size, statement timeout, row cap, byte cap, parameter count) and would
inherit the same gap.

**`docs/engineering/PERFORMANCE.md`** contains no resource inventory.
`CRITICAL_SYSTEM_PROFILE.md` does not claim it does in §4; the two references
are at `:19` (the "Bounded resources" row of §1, which names "the resource
inventory in `docs/engineering/PERFORMANCE.md` **and §4 below**" — only the
second half resolves) and at `:53` (§3, constraint 2). `resource` does not
occur in `PERFORMANCE.md` at any casing: `grep -ci` returns **0**, both at this
baseline and in all six commits that have touched the file. It is a 131-line
timing-budget document, so the inventory `:19` names does not exist, and the
document it names is the wrong kind of document.

**`docs/adr/`** holds six ADRs. `docs/adr/README.md` indexes **five** — ADR-0005
(`0005-keystone-collection-model.md`) appears in the "Accepted ADRs" bullet list
at the bottom but **not** in the index table. There is **no MSRV ADR and no TLS
ADR**. `G07` requires "ADR per driver/MSRV/TLS"; none of the three prerequisites
has an existing foundation.

### 1.3 Finding — the `Host` boundary is one trait, and it has ten implementors

`Host` is declared at `src/host.rs:157` and has **ten** methods:

`write_stdout`, `read_line`, `args`, `read_file`, `write_file`, `now_unix`,
`now_local`, `sleep_ms`, `should_cancel`, `http_request`.

Eight are required methods. **Two have default bodies** — `should_cancel`
(`:207`, returning `false`) and `http_request` (`:223`). Only the latter is a
denial:

```rust
fn http_request(&self, _request: &HttpRequest) -> HostResult<HttpResponse> {
    Err(HostError::unavailable(
        "this host has no network capability (HTTP is unavailable)",
    ))
}
```

A corrected census (`grep -rn 'impl \(aura::host::\)\?Host for'`) finds **10**
implementors:

| # | Implementor | File:line | Kind |
|---|---|---|---|
| 1 | `StdHost` | `src/host.rs:399` | production, native |
| 2 | `LimitedHost` | `src/host.rs:698` | production, wrapping |
| 3 | `BrowserHost` | `src/host.rs:1089` | production, WASM-facing |
| 4 | `SuspendHost` | `tests/session.rs:42` | test |
| 5 | `NoNet` | `tests/session.rs:474` | test, nested in `mod` |
| 6 | `SuspendHost` | `tests/session_depth.rs:18` | test |
| 7 | `FixedHost` | `tests/host.rs:190` | test |
| 8 | `NoNetworkHost` | `tests/http.rs:174` | test |
| 9 | `SuspendHost` | `tests/session_finally.rs:15` | test |
| 10 | `SuspendHost` | `playground/runtime/src/lib.rs:1098` | WASM |

The structural point that matters for this RFC: **adding an eleventh method
with a denying default body changes no implementor's source.** That is how the
HTTP capability landed, and it is the only mechanism available for a database
capability that must not be mandatory.

### 1.4 Finding — five implementors override `http_request`, five inherit the default

`grep -rn 'fn http_request' src/ tests/ playground/runtime/src/` returns exactly
**6** sites. One is the trait's own default (`src/host.rs:223`); the other five
are overrides:

- `src/host.rs:487` — `StdHost`, gated `#[cfg(feature = "http")]`
- `tests/session.rs:70`
- `tests/session_depth.rs:43`
- `tests/session_finally.rs:43`
- `playground/runtime/src/lib.rs:1148`

Per-implementor, brace-matched:

| Implementor | Overrides | Inherits | `http_request` |
|---|---|---|---|
| `StdHost` | 9 | 1 | override, `#[cfg(feature = "http")]` only |
| `LimitedHost` | 8 | 2 | **default** |
| `BrowserHost` | 8 | 2 | **default** |
| `tests/session.rs SuspendHost` | 9 | 1 | override (suspends) |
| `NoNet` | 8 | 2 | **default** |
| `tests/session_depth.rs SuspendHost` | 9 | 1 | override |
| `FixedHost` | 8 | 2 | **default** |
| `NoNetworkHost` | 8 | 2 | **default** |
| `tests/session_finally.rs SuspendHost` | 9 | 1 | override |
| WASM `SuspendHost` | 9 | 1 | override (suspends) |

Two asymmetries are worth naming because a database capability would inherit
both:

1. **`StdHost`'s override is conditional on the feature, not on the host.** With
   `http` compiled out, `StdHost` does not merely deny — it *has no override*,
   and the trait default denies. This is why the denial message is identical in
   both builds.
2. **The WASM `SuspendHost` does not deny; it suspends.** It returns
   `HostError::pending(PendingEffect::Http(request.clone()))`, not
   `unavailable`. `BrowserHost`, which is the native-side browser projection,
   denies. So "browser denies HTTP" is true only of `BrowserHost`; the WASM
   runtime parks the effect and lets the embedder answer it.

### 1.5 Finding — the `http-api`/`http` split is the template, and it is proven

`Cargo.toml` declares two features:

- `http-api = []` (`:47`) — **no dependencies**
- `http = ["http-api", "dep:ureq"]` (`:51`)

`http-api` gates the *language surface*: the `http_request` and `http_get`
`Signature` entries in `src/stdlib/signatures.rs` (`:602`, `:615`) and
`ext::http::install(it)` in `src/stdlib/mod.rs` (`:89`). `http` additionally
pulls the native provider.

Three measurements pin the split:

- **91** builtin signatures total; exactly **2** are `http-api`-gated.
- `cargo tree --no-default-features --features http-api -e features | grep -ci ureq`
  → **0**. The API half adds no native dependency, which is exactly what the
  `Cargo.toml` comment at `:44` claims ("a build that executes HTTP through a
  *host*").
- Building `cargo build --locked --no-default-features --features cli,json,http-api`
  succeeds (`BUILD_RC=0`), and in that build:

| Probe | Result |
|---|---|
| `aura check` on a program calling `http_get` | `CHK_RC=0` |
| `aura run` on the same program | `E5002: this host has no network capability (HTTP is unavailable)`, `RC=1` |
| the same `http_get` call inside `try`/`catch` | `E5002`, `RC=1` — **not caught** |
| default build, same program | `E2003: undefined function http_get`, `RC=1` |

This is the shape a database capability must copy: **the builtin surface is
registered and type-checks under a dependency-free `*-api` feature, and
execution reaches a denying default method.**

There is a companion fact that constrains the design: **`E5002` is fatal and
uncatchable.** `tests/session.rs:393` (`an_unavailable_capability_inside_try_runs_finally_and_propagates`)
pins the behaviour — `finally` runs, `catch` does not intercept, and nothing
after the statement runs. A database capability therefore has two very different
denial shapes available:

- `E5002` (fatal), matching HTTP and the browser filesystem; or
- a value-level failure the program can inspect, which §2.2 `D2` must choose
  because `G09`/`SG-05` require timeout and row-cap failures to be *reportable*,
  not fatal.

### 1.6 Finding — there is no resource model for a driver to plug into

`CRITICAL_SYSTEM_PROFILE.md` §4 has 15 rows and no HTTP row. Its §1 "Bounded
resources" row names a `PERFORMANCE.md` resource inventory that does not exist —
`PERFORMANCE.md` is a timing-budget document with zero `resource` occurrences.
`src/host.rs` declares exactly three limits, all HTTP-local:

- `MAX_HTTP_BODY_BYTES: usize = 8 * 1024 * 1024` (`:234`)
- `MAX_HTTP_TIMEOUT_MS: u64 = 30_000` (`:237`)
- `HTTP_METHODS: [&str; 6] = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"]` (`:240`)

`SG-05` requires *"timeout, max rows/bytes, connections, memory, parameters,
cancellation, and N+1 budget with explicit errors. No silent truncation; no full
in-memory filtering."* **Seven** dimensions, of which the current architecture
supplies **one** reusable mechanism (a timeout constant with a clamp) and **one**
applicable trait method (`should_cancel`). Pool size, row cap, byte cap,
parameter count, and N+1 budget have no home.

### 1.7 Finding — `PendingEffect` has exactly one variant

`src/host.rs:39`:

```rust
pub enum PendingEffect {
    /// An HTTP request the embedder must perform and feed back.
    Http(HttpRequest),
}
```

A resumable database capability — the only shape that works under WASM, where
there is no socket — requires a **second variant**. That is a **breaking enum
change** for every crate that matches on `PendingEffect` exhaustively. There is
precedent for the cost: `tests/session.rs:471`
(`a_denied_capability_never_becomes_a_pending_effect`) asserts that a denied
capability produces `Unavailable` and never `Pending`, precisely so the effect
accounting stays exact.

### 1.8 Finding — there is no credential surface, and `args()` is a leak channel

Aura has no credential masking. `grep -rn '\*\*\*\*\*\*\|redact\|REDACT' src/ --include=*.rs`
returns nothing; `grep -rn 'password\|passwd\|secret' src/ -i` returns three
doc-comment mentions and no code. `args()` is a plain pass-through to
`std::env::args`, so a program invoked as `aura run app.aura -- --api-key=PROBE-PLACEHOLDER`
can read and print the value.

`TRUST_BOUNDARIES.md` lists "environment variables" as available to `py`, which
is consistent: `py` is documented as **not a sandbox** (`SG-08` says so
explicitly). The consequence for `SG-04` ("credentials never printed") is that
**there is no trusted place to put a credential today.** Not an environment
variable, not a CLI argument, not a project file (there is no project file
parser — §1.9).

### 1.9 Finding — there is no configuration surface at all

`grep -rn 'Aura\.toml\|aura\.toml' src/ --include=*.rs` → empty.
`grep -rn 'struct .*Config\|Config::' src/` → empty.
`grep -n 'toml' Cargo.toml` → empty.

So a database capability cannot be configured from a project file, because there
is no project file. It cannot be configured from the environment, because there
is no environment access. It can only be configured **in-process, by the
embedder, through the `Host` implementation** — which is precisely the boundary
`G07` names.

### 1.10 Finding — the result shapes `§8.3` and `§8.5` specify are expressible, with three hard constraints

The campaign's `§8.3` specifies each operation as returning `Result<T, DbError>`
with absence as `T | none`; `§8.5` specifies `query_many<T> → [T]` and
`query_one<T> → T | none`. Probed against the current compiler:

**Works.** A user-declared `enum DbResult<T> { Ok(T), Bad(DbError) }` compiles
and runs; `match` over it works; `let Bad(e) = find(2)` destructuring works;
returning `[User]` and indexing `rs[0].name` works; returning `User | none` and
narrowing with `if o == none` works. `§8.3`'s shape is expressible **today**.

**Constraint 1 — no struct/record patterns.** `match u { User { name } -> ... }`
is `E1006: expected ->, found {`. `pub enum Pattern` (`src/ast/mod.rs:259`)
has exactly 8 variants and none is record-like. `docs/grammar.md:218` states
verbatim that no struct pattern exists.
Consumption must go through a wildcard/`Bind` arm and then field access, or
through an equality test and then field access. Both were probed and work.

**Constraint 2 — `let` destructuring accepts only unqualified variant names.**
`let Good(v) = f()` works. `let DbResult::Good(v) = f()` is
`E2005: a destructuring let needs an initializer`, even when an initializer is
present. `match` accepts qualified patterns; `let` does not. This asymmetry is
`RFC 0002`'s subject, not this one, but a database result type inherits it.

**Constraint 3 — a DB result enum must not use the tags `Ok`/`Err`.**
`enum Other { Ok(int) }` beside `enum DbResult<T> { Ok(T), ... }` is
`E2013: variant names must be unique across the program` (`src/check/mod.rs:1378`).
`§8.3`'s `Result<T, DbError>` cannot be spelled with standard tags until
`RFC 0002`'s `D1` resolves. A database capability must therefore use
**campaign-owned tag names** (`Done`/`Failed` as placeholders, not a
recommendation).

**One shape fact the campaign does not state.** `read_file`'s signature is
`Returns::Dynamic`, and `Returns::Dynamic` resolves to `Ty::Unknown`
(`src/stdlib/signatures.rs:210`). `let x: int = read_file("a.txt")` therefore
**passes `check` and fails at `run`** with `E3001: expected a number, found none`.
**32 of 91** builtin signatures are `Returns::Dynamic`. If a database capability
is added the same way, every query result is untyped at check time, and every
mistake becomes a runtime error. That is the single largest design cost in this
RFC, and it is why §2.1 `D1` exists.

### 1.11 Finding — a new soundness defect, found while probing this RFC

`let <Variant>(x) = expr` is **unchecked for variant mismatch**:

```
aura check → CHK_RC=0
aura run   → main.aura:4:7: error[E3001]: variant pattern mismatch, RUN_RC=1
```

Root cause: `Stmt::LetPattern` (`src/check/mod.rs:6007`) calls `check_pattern`,
and `check_pattern` (`:3868`) for `Pattern::Variant(tag, ps, span)` only asks
whether `tag` is a *declared* variant — it never compares it against the
scrutinee's type. The checker's own comment at `:6000` states it performs no
inference for destructuring. The runtime site, `src/run/mod.rs:1651` inside
`:1635-1665`, is the **only** occurrence of `variant pattern mismatch` in the
source.

This is consistent with the specification, which says there is **no
exhaustiveness requirement** (`docs/LANGUAGE_SPEC.md:2471-2472`; failure is
`E4029` at runtime). A non-exhaustive `match` likewise passes `check` and fails
at `run`. So this is documented behaviour for `match` and an **undocumented
consequence** for `let`. It is recorded here because a database capability whose
result type is an enum depends on destructuring being trustworthy, and today it
is not. It is **out of scope** for this RFC — it belongs to `RFC 0005`'s family —
but it must be named, not discovered later.

### 1.12 What is working and must not regress

- `Host` stays a single trait with denying defaults; no implementor is forced to
  change (§1.3).
- `E5002` stays fatal and uncatchable, or `§2.2 D2` explicitly changes it
  campaign-wide — not for databases alone (§1.5).
- The default build keeps **no** database surface, and a database-*api* build
  adds **no** native dependency (§1.5).
- `PendingEffect` accounting stays exact: a denied capability never becomes a
  pending effect (`tests/session.rs:471`).
- No real database is contacted without authorized credentials (§6.3).

## 2. Decision

Three decisions are escalated. They are ordered by dependency: `D1` blocks `D2`
and `D3`.

### 2.1 D1 — How a database capability reaches the language surface

The problem in one line: **HTTP got away with `Returns::Dynamic` because an HTTP
response is a map with a `status` key and nothing type-checks it. A database row
is a typed record, and typing it does not fit the current generic machinery.**

Four options:

**Option A — builtin signatures, `Returns::Dynamic`.** Add `db_open`, `db_query`,
… as ordinary builtins with dynamic returns, mirroring HTTP exactly.

- *For:* zero architectural change; the proven `http-api` template applies
  unchanged; smallest possible first step; a SQLite vertical slice (`G10`)
  becomes reachable.
- *Against:* **every result is untyped.** §1.10's probe shows
  `let x: int = read_file(...)` compiles and fails at runtime. For a database
  this is not a corner case, it is the normal path. It also collides with
  `RFC 0005`'s finding that generic instantiation is never propagated to the
  call site — a `query_one<T>` built on the current checker would silently accept
  `let u: Wrong = query_one("...")`.

**Option B — trait objects in the `Host`, projected as opaque handles.** Add
`Host::db_connect`/`db_execute` returning an opaque handle value, with the
language seeing a nominal type supplied by the host.

- *For:* keeps authority in the `Host`; matches `G07`'s boundary clause exactly.
- *Against:* **`Value` has no handle variant.** `src/run/value.rs:172` has
  `Int`, `Float`, `Str`, `Bool`, `None`, `List`, `Array`, `Tuple`, `Set`, `Map`,
  `Instance`, `Variant`, `Closure`, `Native`, `Range` — fifteen variants and no
  opaque type. `TypeExpr` (`src/ast/mod.rs:18`) likewise has no nominal-handle
  form beyond `Named`/`App`. This option requires a runtime and type-system
  addition first.

**Option C — declare drivers as Aura `trait` implementations.** Use the
existing nominal `struct`/`impl`/`trait` machinery (`§8.1`), with a generated
Repository as a static `impl` (`NBL-020`).

- *For:* no runtime change at all; uses only what exists; `§8.1` already forbids
  dynamic dispatch, so a static `impl` is the sanctioned shape.
- *Against:* nothing in the compiler can *call* a driver — a `trait` in Aura is
  a static contract with no host binding. It gives the user a shape but no
  execution. This option is a **future composition layer**, not a capability.

**Option D — do nothing until `RFC 0005` (`D1`) and `RFC 0002` (`D1a/D1b/D1c`)
resolve.**

- *For:* honest. Both open decisions are prerequisites for a typed result
  surface, and `G07` cannot be satisfied before them without encoding the same
  defect twice.
- *Against:* blocks `G10`–`G15` entirely, and the campaign has no completion path
  if the human decisions stall.

**Recommendation: A now, with a written commitment to revisit on `RFC 0005`.**
`G10` ("first vertical slice: SQLite") is the earliest gate that needs a driver,
and an untyped dynamic surface is sufficient to prove the vertical slice, the
SQLGuard negative tests (`G09`), and the `E5002` capability probes. Typing can
be retrofitted additively when `RFC 0005` lands. Option D is the safer answer and
the honest one; A is the one that produces evidence. **This is a human decision
because it trades a known soundness gap for schedule, and the campaign's own
rules forbid resolving that silently.**

### 2.2 D2 — How a denied or failing database operation is reported

Two shapes are available and they are not interchangeable (§1.5):

**(a) `E5002`, fatal and uncatchable** — identical to HTTP and the browser
filesystem. Matches "equivalent `E5002` capability probes" literally.

- *For:* one mechanism, one test, one message shape; the campaign's `G07` text
  names `E5002` explicitly; `tests/session.rs:393` already pins the semantics.
- *Against:* `SG-05` requires timeout, row cap, and byte cap failures to be
  *explicit errors the program can act on*. A fatal diagnostic is not
  actionable, and `SG-05` also forbids silent truncation — so a row cap cannot
  quietly return fewer rows and cannot kill the program. **(a) is
  incompatible with `SG-05` for resource failures.**

**(b) A value-level failure the program inspects** — `DbResult<T, DbError>`,
written in Aura, with `E5002` reserved for *capability absence* only.

- *For:* satisfies `SG-05`; separates "this host has no database" (a boundary
  fact) from "this query exceeded its budget" (an application fact).
- *Against:* requires the result enum of §1.10 Constraint 3, and inherits the
  unchecked-destructuring defect of §1.11.

**(c) Both, split by cause** — `E5002` for absence of the capability, a
value-level `DbError` for everything that happens after a connection exists.

- *For:* each mechanism is used where it is correct; `G07`'s `E5002` clause is
  satisfied for the capability boundary, and `SG-05` is satisfied for resources.
- *Against:* two error paths to test, and the split must be stated normatively
  or implementers will pick differently.

**Recommendation: (c).** It is the only option that satisfies both `G07` and
`SG-05` without redefining either. It also preserves the existing `E5002`
contract untouched, which `G07`'s "equivalent probes" clause is really asking
for.

### 2.3 D3 — What "protected profile" is, and where it lives

`NBL-016`'s title names a **protected profile**, and `SG-01` uses the phrase:
*"No raw SQL/BSON in the protected profile."* Nothing in the repository defines
it. At the baseline commit, `git grep -n -i -e 'protected profile' -e
'ProtectedProfile' edcce6c7` returns exactly **five** hits — two in
`CURRENT_HANDOFF.md` and three in `NEBULA_CAMPAIGN.md`. All five are campaign
prose. None is in `docs/LANGUAGE_SPEC.md`, in `docs/adr/`, or in any `.rs`
file, so the phrase has a plan and no definition. The count is pinned to the
baseline on purpose: this RFC's own existence raises the working-tree figure,
and a bare count would falsify itself on merge.

Three readings:

**(a) A build configuration.** "Protected profile" = a feature combination
(`database-api` without `py`, without `evaluator-oracle`, with SQLGuard
enforced). Cheap, testable, and consistent with `SG-08`'s statement that `py`
cannot be SQLGuard-certified — a profile that excludes `py` follows directly.

**(b) A runtime mode.** A flag or `Host` property that switches enforcement at
run time. More flexible, but it means the *same binary* can be non-protected,
which weakens the word "profile" to "setting".

**(c) A declaration in a project file.** Impossible today: there is no project
file (§1.9), and `RFC 0007`'s `D1` has not resolved whether one will exist.

**Recommendation: (a), and only after the `database-api` feature exists.** (b)
makes a security property switchable at run time, which is the failure mode
`SG-08` is written to prevent. (c) is blocked on `RFC 0007` `D1`. If (a) is
chosen, `G07`'s "ADR per driver/MSRV/TLS" clause should be read as requiring an
ADR that fixes the profile's membership, and this RFC records that no such ADR
exists (§1.2).

## 3. Grammar delta

**None.**

A database capability adds *builtin functions*, not syntax. `RFC 0007` used the
same "None" answer and recorded why; the reasoning holds here and there is an
additional constraint worth naming.

The reason no grammar is needed: `db_open("...")` is an ordinary call
expression, already covered by the existing grammar. The result shapes are
ordinary `struct`, `enum`, `[T]`, and `T | none` — all of which parse today
(§1.10).

The constraint that *looks* like a grammar question and is not: `entity struct`
and the metadata/relation syntax of `NBL-017` **will** need grammar. `entity` is
not a keyword, and `@tag` is unlexable (`E1001`). That work is `NBL-017`'s
grammar delta, not this RFC's. Recording the boundary here prevents a later
reader from assuming this RFC authorized it.

## 4. Semantics

### 4.1 The four contracts at risk

1. **The `Host` boundary** — one trait, two defaulted methods, ten implementors
   (§1.3). At risk because a database method with a non-trivial default would be
   the first *denying* one to carry state, and a required method would break ten
   implementors.
2. **`E5002` fatality** — pinned by `tests/session.rs:393`, uncatchable (§1.5).
   At risk because resource failures want to be catchable.
3. **`PendingEffect` exactness** — a denied capability never becomes a pending
   effect (`tests/session.rs:471`). At risk because a second variant changes the
   enum (§1.7).
4. **Credential inertness** — no masking exists, so nothing may assume a value
   reaching output is safe (§1.8). At risk because a DSN is a credential and a
   DSN in a diagnostic is a leak.

### 4.2 The clauses that would hold by absence rather than by construction

Stated plainly, because `RFC 0007`'s errata established that a clause satisfied
because the feature does not exist must be labelled that way:

| Clause | Why it holds today | What would make it hold by construction |
|---|---|---|
| `Host boundary compiles without drivers` | no driver exists | a CI job that builds with `database-api` and no driver feature |
| `no real database without authorized test credentials` | no database code exists | a test that fails if a driver test runs without credentials present |
| `ADR per driver/MSRV/TLS` | the ADRs would be trivially true | three ADRs, one per subject, accepted before any driver merges |

Only the third is a hard `G07` prerequisite and it is **unsatisfiable today**
(§1.2): the MSRV prerequisite is independently blocked by `NBL-004`'s proven
finding that every current driver exceeds MSRV 1.83, with `rusqlite 0.37.0` the
only survivor.

### 4.3 What is working and must not regress

Listed in §1.12. The one addition: the dependency table must not grow a driver
crate before §2.1 `D1` resolves, because the MSRV consequence is already known
and would be a second, silent `NBL-004`.

### 4.4 How the three decisions interact

- `D1` blocks `D2`: the error type is part of the result type, so choosing `D2`
  (c) presupposes a result surface to put it in.
- `D1` blocks `D3`: a profile that selects a feature set presupposes features
  that `D1` names.
- `D3` does not feed back into `D1`/`D2`, which is why it is last.

### 4.5 One observation that is not a finding

The `Host` trait's asymmetry — eight required methods, two defaulted — is not a
defect. `http_request` is defaulted precisely because "no network" is a
legitimate host, whereas "no stdout" is not; `should_cancel` is defaulted
because "never cancel" is the ordinary case. A database method belongs on the
defaulted side for the same reason: "no database" is a legitimate host. This is
recorded because a reader could otherwise conclude the asymmetry is accidental.

## 5. Migration

Nothing to migrate. No database surface exists, no configuration file exists, and
no program can currently reference a database builtin. The migration work is
`NBL-070`'s (`G07`), not this RFC's.

The one migration-adjacent risk: if §2.1 chooses Option A, then when `RFC 0005`
lands and types become checkable, programs written against the dynamic surface
may need annotation. That cost should be accepted explicitly at `D1` time, not
discovered at `G07` time.

## 6. Enforcement

### 6.1 The evidence table

| # | Claim | Probe | Result |
|---|---|---|---|
| 1 | `Host` has ten methods | `grep` over `src/host.rs:157-…` | 10 |
| 2 | Exactly two have a default body | `:207` (`should_cancel`), `:223` (`http_request`) | 2 |
| 3 | Ten implementors | `grep -rn 'impl \(aura::host::\)\?Host for'` | 10 |
| 4 | Five override `http_request` | `grep -rn 'fn http_request'` → 6, minus the default | 5 |
| 5 | Five inherit | per-implementor brace match | 5 |
| 6 | `http-api` adds no dependency | `cargo tree … \| grep -ci ureq` | 0 |
| 7 | `http-api` registers two builtins | `grep -c '#\[cfg(feature = "http-api")\]' signatures.rs` | 2 |
| 8 | 91 builtins total | `grep -c 'name: "' signatures.rs` | 91 |
| 9 | Default build: absent → `E2003` | `aura run` with `http_get` | `RC=1`, `E2003` |
| 10 | `http-api` build: present, denied → `E5002` | `aura run` with `http_get` | `RC=1`, `E5002` |
| 11 | Same program type-checks | `aura check` | `RC=0` |
| 12 | `E5002` is uncatchable | `try`/`catch` around `http_get` | `RC=1`, not caught |
| 13 | No database crate | `Cargo.toml` / `Cargo.lock` | none |
| 14 | No database builtin | five name guesses | all `E2003` |
| 15 | No config parser | `grep -rn 'struct .*Config' src/` | empty |
| 16 | No `toml` dependency | `grep -n 'toml' Cargo.toml` | empty |
| 17 | No environment access | `grep 'name: "env"'` | NONE |
| 18 | `args()` is unmasked | write to file, `od -c` | `--api-key=PROBE-PLACEHOLDER` verbatim |
| 19 | No credential masking | `grep -rn 'redact\|REDACT' src/` | empty |
| 20 | `PendingEffect` has one variant | `src/host.rs:39-42` | `Http` only |
| 21 | `Value` has no handle variant | `src/run/value.rs:172-206` | 15 variants |
| 22 | `Returns::Dynamic` → `Ty::Unknown` | `signatures.rs:210`; `let x: int = read_file(...)` | `check`=0, `run`=`E3001` |
| 23 | 32 signatures are dynamic | `grep -c 'returns: Returns::Dynamic'` | 32 |
| 24 | No struct pattern | `match u { User { name } -> … }` | `E1006` |
| 25 | `[User]` indexing works | `rs[0].name` | `ada` |
| 26 | `User \| none` narrowing works | `if o == none` | `absent` |
| 27 | Qualified `let` pattern rejected | `let DbResult::Good(v) = …` | `E2005` |
| 28 | `Ok`/`Err` tags collide | two enums with `Ok` | `E2013` |
| 29 | `let` destructure unchecked | wrong variant, `check` then `run` | `CHK=0`, `RUN=1` |
| 30 | Runtime site is unique | `grep -rn 'variant pattern mismatch'` | `src/run/mod.rs:1651` |
| 31 | Spec has no exhaustiveness rule | `LANGUAGE_SPEC.md:2471-2472` | "no exhaustiveness requirement" |
| 32 | Three limits exist, all HTTP | `src/host.rs:234/237/240` | 3 |
| 33 | `CRITICAL_SYSTEM_PROFILE` §4 rows | its `\|`-row census, minus header | 15, no HTTP row |
| 34 | `PERFORMANCE.md` has no resource inventory | `grep -ci 'resource'` | 0 (timing-budget doc) |
| 35 | Six ADRs, five indexed | `ls docs/adr/0*.md` vs index table | 6 vs 5 |
| 36 | No MSRV ADR, no TLS ADR | `ls docs/adr/` | absent |
| 37 | No "protected profile" in code | `grep -i '\bprofile\b' --include=*.rs src/` | empty |
| 38 | No DB/SQL in the spec | `grep -i 'database\|sql\|mongo\|transaction' LANGUAGE_SPEC.md` | empty |

### 6.2 Tests that would have caught each finding

1. **Implementor census** — assert the count of `impl … Host for` is exactly the
   registered list, so a new implementor fails a test rather than silently
   changing the census. (Both earlier wrong censuses would have failed this.)
2. **Override/default split** — assert that every implementor either overrides
   `http_request` or is on an explicit "inherits the denial" allow-list.
3. **Default build has no database surface** — `E2003` for `db_open` in the
   default build.
4. **`database-api` build registers and denies** — `check`=0, `run`=`E5002`,
   mirroring the HTTP test pair.
5. **`database-api` adds no dependency** — `cargo tree --no-default-features
   --features database-api | grep -ci <driver>` = 0, mirroring the `ureq` probe.
6. **`E5002` stays uncatchable** — a `try`/`catch` around a denied capability
   exits non-zero, extending the existing `tests/session.rs:393` test to the new
   capability.
7. **No credential in any output surface** — a four-surface test (diagnostic,
   AIS document, MCP response, rendered HTML) asserting a DSN never appears.
   **This test does not exist today and `G06` already needs it.**
8. **`Returns::Dynamic` is not silently accepted where a concrete type is
   required** — the §1.10 probe as a regression test. It currently passes
   `check` and fails `run`, which is the defect, so the test would have to
   assert the *desired* behaviour after `RFC 0005` `D1` resolves.
9. **Resource limits have a home** — a test that enumerates every declared limit
   and asserts each appears in `CRITICAL_SYSTEM_PROFILE.md` §4. This would have
   caught the missing HTTP rows.
10. **ADR index completeness** — assert every `NNNN-*.md` in `docs/adr/` has a
    row in the index table. This would have caught the missing ADR-0005 row.

### 6.3 Fixtures used

One `.aura` file per directory, always, per the campaign's established rule.

- `p1`–`p4` — `match` over `DbResult`, unqualified `let`, wrong-variant `let`,
  qualified `let`.
- `s1`–`s4` — struct pattern, field access on a returned struct, `[User]`
  indexing plus `User | none` narrowing, `E2013` tag collision.
- `dyn` — `let x: int = read_file(...)` to prove `Returns::Dynamic` leniency.
- `e5002` — `http_get` under the default build and under `http-api`.
- `catch3`, `catch4` — `try`/`catch` around a filesystem read and around
  `http_get`.

**No fixture contacts a real database.** `G07`'s final clause forbids it without
authorized credentials, and none exist.

## 7. Open items

1. **`D1` — how the capability reaches the language surface** (§2.1). Root
   decision; blocks `D2` and `D3`.
2. **`D2` — how a denial or resource failure is reported** (§2.2). The only
   decision that can change an existing, tested contract (`E5002` fatality).
3. **`D3` — what "protected profile" means** (§2.3). Blocked on `D1`, and
   reading (c) is additionally blocked on `RFC 0007` `D1`.
4. **`G07`'s "ADR per driver/MSRV/TLS" is unsatisfiable today.** The MSRV
   prerequisite is independently blocked: `NBL-004` proved every current driver
   exceeds MSRV 1.83, with `rusqlite 0.37.0` the only survivor. The TLS
   prerequisite has no ADR and no crate.
5. **`§1.11`'s unchecked destructuring is out of scope but load-bearing.** A
   database result enum depends on `let <Variant>(x) = …` being trustworthy. It
   belongs to `RFC 0005`'s family and should be tracked there.
6. **`§1.10`'s `Returns::Dynamic` leniency affects 32 of 91 builtins** and is
   not confined to databases. If `D1` chooses Option A, this RFC adds a 33rd
   instance of a known defect.
7. **Two stale capability documents and two stale references should be
   corrected in one batch** (§1.2): `CAPABILITY_MODEL.md` (missing HTTP row,
   wrong trait line reference) and `CRITICAL_SYSTEM_PROFILE.md` §4 (no HTTP
   rows) are stale; `PERFORMANCE.md` is mis-*cited* — the §1 row of
   `CRITICAL_SYSTEM_PROFILE.md` names it as a resource inventory it does not
   contain; and `docs/adr/README.md` is missing the ADR-0005 index row. None of
   this requires a decision; all of it requires authorization to edit.
8. **`PendingEffect` is a breaking enum (§1.7).** If a resumable database
   capability is required for WASM, the enum change needs its own compatibility
   note, because `tests/session.rs:471` pins the current exactness.
