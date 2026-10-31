# Campaign 003 — Controlled Closure Report

Directive: **AURA — DIRETIVA 005, Operação de Encerramento Controlado
(CRITICAL FIRST)**.

Status: **`COMPLETE`** — the bounded closure criteria of Directive 005 §7 were
met. This is a statement about the *delimitation of the campaign*, not a claim
that Aura is free of every conceivable defect.

Verdict for release-candidate preparation: **`GO`**.

> This report does **not** authorize publishing a stable release. Promotion of a
> release candidate remains a human-gated decision (Directive 005 §8).

---

## 1. Final repository identity

| Item | Value |
|---|---|
| Branch | `rewrite/v3-rust` |
| Final commit | `8cc9ed32caff89ea7265113bffd19c8085a99bd4` |
| Subject | `docs: record PR #38 merge and re-arm the CORE FIRST lead list` |
| Remote | `origin/rewrite/v3-rust` at the same commit (verified with `git fetch origin` + `git rev-parse`) |
| Worktree | clean (`git status --short` empty) |
| Crate version | `0.3.1` ([Cargo.toml](../../Cargo.toml)) |
| Playground current runtime | `0.3.1` "Keystone" ([manifest.json](../../playground/runtimes/manifest.json)) |

Remote and local were reconciled before the audit began and re-verified after
it. `PR #1` does not exist: GitHub returns *"Could not resolve to a PullRequest
with the number 1"*. The campaign's PRs are the contiguous range `#2`–`#38`.

### PR inventory

| Metric | Count |
|---|---|
| Campaign PRs merged in total | **37** (`#2`–`#38`) |
| Open PRs at closure time | 0 |
| Closed-without-merge PRs | 0 |
| Corrective PRs opened **under Directive 005** | **0** (budget 0/6 consumed) |
| Corrective PRs opened for this report | 1 (documentation only) |

No P0 or P1 defect was reproduced, so the Directive 005 §5 correction procedure
was never triggered. The only PR produced by this directive carries this report
plus the state-document reconciliation; no production code was changed.

---

## 2. P0/P1 defect inventory

**Empty.** No P0 and no P1 defect was reproduced in any of the six audited
areas. Consequently:

- there is no `FIXED` entry in the area matrix;
- there is no regression test added by this campaign, because there was no
  reproduced failure to protect against;
- the `≤6` corrective-PR budget is entirely unspent.

### Findings considered and falsified

The audit did surface one candidate that would have qualified as P1 had it been
real. It was reproduced against the governing contract and **falsified**:

| Candidate | Claim | Verdict | Governing contract |
|---|---|---|---|
| Sibling module contamination | Unreferenced `.aura` files in a module directory poison the compilation unit | **FALSIFIED — not a defect** | [FILESYSTEM_MODULES_DESIGN.md](../FILESYSTEM_MODULES_DESIGN.md) §DECIDED: eager structural discovery is the *frozen, intended* contract; every eligible child joins the compilation unit |

Operational consequence only: a probe must run in its own otherwise-empty
directory. That is a property of the intended design, not a defect, and must not
be re-opened.

Other candidates examined and dismissed as out-of-contract or by-design:
struct-shorthand construction `P { x }` (unsupported and unspecified, `E1006`);
first-error-only semantic diagnostics (documented tolerant model, no spec
mandate for aggregation); absence of a `to_set` method (sets use literal
syntax); absence of a single global allocation cap (see §7, R-15).

---

## 3. Six-area final audit matrix

Each area was audited exactly once, using the existing tests, reports and
artifacts first, per Directive 005 §4.

| # | Area | Result | Evidence |
|---|---|---|---|
| 1 | **Lexer / Parser** | `PASS` | Depth bounds `E1015` at 20 000 nested parens/brackets and 50 000 chained unary operators; AST depth bound 256; operator precedence and associativity confirmed; syntax error yields exit 1 with `E1006` and does not poison subsequent compilations; `fuzz lexer` 1 580 266 runs clean; `fuzz parser` 864 543 runs clean |
| 2 | **Checker / Resolver** | `PASS` | Generics, inference, type-parameter frames and bounds, scopes, shadowing, `mut` enforcement (`E2001`), private fields (`E2018`), re-export chains; resolver state rolls back correctly on the error path; only the first semantic diagnostic is reported by design; `fuzz checker` 478 986 runs clean |
| 3 | **Evaluator / Runtime** | `PASS` | Recursion capped at 512 frames (`E4011`); exceptions and `finally` ordering; closures, higher-order calls, `zip` truncation; cyclic values via `push(c, c)` render/compare/hash safely; NaN/infinity ordering conformant; integer overflow and division corner cases return `E4013`; JSON decode depth 512 → `E4031`; `fuzz runtime` 1 937 runs clean with no leaks flagged under the documented `Rc`-retention policy |
| 4 | **Collections / Modules** | `PASS` | Indexing and negative-index wrapping; `sort`/`reverse` are non-destructive; set literal dedup and `add`/`has`/`contains`/`remove` typing; map `get` miss → `none`, key/value type enforcement; tuple/struct field access; cyclic modules detected; eager module discovery matches the frozen design contract |
| 5 | **Host / CPython / WASM** | `PASS` | Capability bounds read from source and exercised: `MAX_HTTP_BODY_BYTES` 8 MiB, `MAX_HTTP_TIMEOUT_MS` 30 000, `MAX_SLEEP_MS` 60 000; restricted hosts return `E5002`; CPython interop suites 12 + 18 + 19 tests green; six remote `cpython interop` CI jobs green (3.10/3.11/3.12/3.13 + Windows + macOS); Playground WASM suites green (browser, worker, multi-file, cache, browser-http, http-security); pure-Rust binary reports `E5002` for `py_*` |
| 6 | **Release / CI / Fuzz** | `PASS` | Playground manifest matches all 9 versions; all 8 available frozen runtimes verified byte-identical to their declared `sha256`/`bytes`; `0.0.1` correctly `available=false`; all five fuzz targets clean with an empty `fuzz/artifacts`; `fmt`, `clippy -D warnings`, MSRV 1.83, Miri, no-default-features matrix, website build and suites all green |

No area is `BLOCKED` and no area is `NOT_VERIFIED`. A `PASS` area must not be
re-audited without new evidence of a defect (Directive 005 §4).

---

## 4. Validation executed

Every command below was actually executed in this campaign. Nothing absent is
recorded as approved.

### 4.1 Language and library

| Command | Result |
|---|---|
| `cargo build --locked --release` | OK; binary reports `0.3.1` |
| `cargo test --locked --all-targets --all-features` | **1477 passed, 0 failed, 14 ignored** across 66 suites |
| `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` | PASS (0 failed) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | PASS (18.70 s) |
| `cargo +1.83.0 check --locked --all-features` | PASS (7.23 s) — MSRV floor holds |
| `cargo +nightly check --manifest-path fuzz/Cargo.toml --all-targets` | PASS |
| `cargo +nightly miri test --locked --lib --no-default-features --features cli` | **132 passed, 0 failed** in 548.89 s |
| `bash scripts/artifact-smoke.sh` | **12 passed, 0 failed** (includes pure-Rust links no CPython) |

### 4.2 Fuzzing

Command form: `cargo +nightly fuzz run <target> -- -max_total_time=90 -rss_limit_mb=6144 -timeout=25`,
with `ASAN_OPTIONS=detect_leaks=0` applied to `runtime` only, per the documented
`Rc`-cycle retention policy ([fuzz/README.md](../../fuzz/README.md),
[LANGUAGE_SPEC.md](../LANGUAGE_SPEC.md) §31.6).

| Target | Runs | Duration | Result |
|---|---|---|---|
| `lexer` | 1 580 266 | 91 s | clean |
| `parser` | 864 543 | 91 s | clean |
| `checker` | 478 986 | 91 s | clean |
| `runtime` | 1 937 | 91 s | clean |
| `ais` | 755 465 | 91 s | clean |

`find fuzz/artifacts -type f` → **empty**. The fuzz corpus is git-ignored
(`fuzz/.gitignore:8`), so seeding it left the worktree clean.

> `ast_gen` is **not** a fuzz target. [fuzz/Cargo.toml](../../fuzz/Cargo.toml)
> declares exactly five `[[bin]]` targets. `ast_gen.rs` is a helper module
> included into `checker.rs` via `#[path]` and is exercised by the `checker`
> runs. This is a documented fact, not a coverage gap.

### 4.3 Playground, website and interoperability

| Command | Result |
|---|---|
| `node playground/build.mjs --check` | manifest matches 9 versions |
| `node playground/tests/node/run-all.mjs` | PASS (exit 0) |
| `node website/build.mjs` | 46 pages built |
| `node website/tests/run-all.mjs` | browser 107, worker 12, multi-file 42, cache 7, browser-http 8, http-security 26 — **0 failed** |
| CPython interop suites | 12 / 18 / 19 tests, 0 failed |
| Frozen runtime manifest verification | 8/8 `sha=OK bytes=OK`, **0 bad**, `0.0.1` correctly unavailable |

### 4.4 Remote CI

| Item | Result |
|---|---|
| CI run `41217451481` (push, `rewrite/v3-rust`, commit `8cc9ed32`) | **success**, 21/21 jobs green |
| Website deploy run `41217451457` | **success** |
| Fuzz nightly — `schedule` event run `41037980226` | success, 14/14 steps |

CI jobs green on the final commit: `cargo audit`, `clippy (all features)`,
`rustfmt`, `minimum supported Rust (1.83)`, `miri (unsafe-free guarantee)`,
`test (macos-latest)`, `test (ubuntu-latest)`, `test (windows-latest)`,
`language contract`, `extended property tests`,
`fuzz smoke (lexer/parser/checker/runtime)`, `playground (wasm runtime)`,
`website (static build + examples + links)`,
`validation orchestrator self-tests`, `pure Rust binary links no CPython`, and
six `cpython interop` jobs (3.10, 3.11, 3.12, 3.13, Windows/py3.12,
macOS/3.12).

No gate was reported green without having been executed. Retired signals (e.g.
`slow-unit-*` / `timeout-*` fuzz artifacts) are excluded by the crash-artifact
gate in both `ci.yml` and `fuzz-nightly.yml`; that exclusion is unchanged and
was not widened.

---

## 5. Deferred defects (P2/P3) — recorded, deliberately not implemented

Per Directive 005 §3, P2/P3 do not authorize a corrective PR in this operation.
Each is already tracked; none is a prerequisite for a P0/P1 correction because
no P0/P1 exists.

| ID | Deferred item | Why deferred |
|---|---|---|
| TD-01 | `dtolnay/rust-toolchain` referenced without a full SHA pin (partially closed) | Supply-chain hygiene; not a runtime defect |
| TD-03 | No SBOM / build provenance attestation | Release-engineering hardening |
| TD-04 | Cross-host build reproducibility unverified for the WASM artifact | Release-engineering hardening |
| TD-07 | `E5003` diagnostic unused | By design; removing or wiring it is cosmetic |
| TD-09 | Internal `bool` lowering — previously suspected | **False positive**; no defect |
| TD-10 | Dev-runtime identity discoverability in the Playground | UX improvement |
| TD-11 | CPython initialization policy, deferred to v1 | Deliberate scope decision |
| TD-13 | REPL O(N²) scanning | Closed by PR #21; residual refinement deferred to v1.1 |
| TD-17 | `dtolnay` SHA pin | Same class as TD-01; v1.1 |
| TD-19 | Recursive engine publication | Closed in source; **publication is human-gated** |
| TD-22 | Historical local-validation gap | Re-run sharded and green; process item |
| TD-24 | Encoder cost | Targeted at v0.3.2 |

None of these represents a demonstrable violation of a security boundary, data
loss, unauthorized execution, a runtime-invalidating failure, a process
abort on valid input, or a type-safety bypass.

---

## 6. Residual risks

| Risk | State | Justification |
|---|---|---|
| **R-15** — unbounded work / allocation growth | **monitored, accepted** | No single global allocation cap exists. Bounded surfaces do exist: range materialization 10 M, value depth 512, value node count 1 M, call frames 512, AST depth 256, JSON decode depth 512, format width/precision. A program can still grow a list or string until the host runs out of memory — that is ordinary host-enforced resource behaviour for a language with dynamic collections, not a P1. Reducing it would be a design change outside this directive |
| **R-10** — frozen released WASM artifacts are stale relative to source | **closed in source, artifacts intentionally frozen** | The recursive-engine fix is in source; published `0.3.1` bytes are immutable by policy. Replacing them is forbidden without human authorization |
| **R-12** — breaking change without a compatibility note | **open** | Compatibility-note obligation; no runtime impact demonstrated in this audit |
| **R-07** — partially mitigated | partially mitigated | Residual surface judged non-critical; no P0/P1 reproduced |
| **R-08**, **R-14** — monitored | monitored | No new evidence of escalation during this campaign |
| 37 orphaned dev-runtime directories under `playground/runtimes/` | **preserved by design** | Immutable historical artifacts. Deleting them would destroy history; they cost only disk |

---

## 7. Native / WASM / CPython compatibility

| Surface | State |
|---|---|
| Native CLI | Verified: build, `run`, `check`, `eval`, stdin, REPL, module import, collections + JSON, non-zero exit paths |
| WASM Playground | Verified: browser, worker, multi-file project, cache, browser-HTTP and HTTP-security suites — 0 failed; runtime ABI 1 for stable `0.3.1`, ABI 2 for `0.3.2-dev.8` |
| CPython host | Verified: interop suites (12 / 18 / 19) plus six remote CI jobs across Python 3.10–3.13 on Linux, macOS and Windows |
| Pure-Rust (no CPython) build | Verified: `py_*` builtins report `E5002`; remote `pure Rust binary links no CPython` job green |

No divergence between native, WASM and normative behaviour was reproduced. The
capability bounds (`MAX_HTTP_BODY_BYTES`, `MAX_HTTP_TIMEOUT_MS`, `MAX_SLEEP_MS`)
are enforced on the native host, and the browser host applies the equivalent
response-size cap.

---

## 8. Stable version state

| Item | State |
|---|---|
| Stable version | **`0.3.1`** ("Keystone") — unchanged |
| `Cargo.toml` version | `0.3.1` |
| `manifest.current` | `0.3.1` |
| Release tags | `v0.0.1`, `v0.0.2`, `v0.2.0`, `v0.2.1`, `v0.3.1` |
| `0.3.2` | Not released, not promoted; `0.3.2-dev.8` is a development preview |
| Frozen `0.3.1` WASM | sha256 `96e778ae50bd261f76d49adf04b8650cc3b60678a14c0e0ac555fb54400256ab`, 1 951 520 bytes — **byte-identical, untouched** |

No version bump, no tag created or moved, no frozen artifact replaced, no
publication performed.

---

## 9. Verdict

**`GO`** — for preparing a release candidate.

Basis:

- all six audit areas returned `PASS`;
- no P0/P1 remains reproduced and uncorrected;
- the full validation floor passed, including Miri, MSRV, no-default-features,
  all five fuzz targets and the remote CI suite on the final commit;
- native, WASM and CPython behaviour agree on every surface exercised;
- the stable `0.3.1` identity and all frozen artifacts are intact.

Conditions attached to this verdict:

1. This `GO` authorizes **release-candidate preparation only**. Promoting a
   stable release requires explicit human authorization (Directive 005 §8).
2. The residual risks in §6 remain open and must travel with the candidate.
3. The deferred P2/P3 items in §5 belong to a future campaign and were not
   silently converted into fixes.

---

## 10. Measured budget consumption

Only measured values are recorded; nothing is estimated or inferred.

| Budget | Limit | Consumed |
|---|---|---|
| Wall clock | 24 h | **≈ 35 min** (session state created `2026-10-31T13:19:39-0300`, closed `2026-10-31T13:55:01-0300`) |
| Corrective PRs | ≤ 6 | **0** |
| Exploration per subsystem | ≤ 90 min | Not exceeded in any of the six areas |

API token consumption, summed over 189 usage records in the session event log:
**11 996 218 input tokens / 61 693 output tokens**. These are provider-reported
counters, not billing figures.

---

## 11. Campaign termination

Directive 005 is closed. This report is the final artifact of Campaign 003.

Out of scope and deliberately not started: new features, general refactors,
website redesign, further bug hunting, and any new campaign. No follow-up work
is authorized by this document.
