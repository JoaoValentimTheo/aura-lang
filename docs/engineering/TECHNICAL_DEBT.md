# Aura Technical Debt Register

Known debt, explicitly tracked (not buried in conversation logs). Severity:
HIGH / MEDIUM / LOW.

| ID | Severity | Item | Impact | Deferred reason | Owner | Target |
|---|---|---|---|---|---|---|
| TD-01 | MEDIUM | GitHub Actions use major-version tags, not SHA pins | Supply-chain: a compromised action tag could affect builds/releases | Standard practice; pinning is a hardening step | Supply Chain | **closed for third-party actions** — all standard actions SHA-pinned; `ci.yml` least-privilege. `dtolnay/rust-toolchain` is the tracked exception (TD-17) |
| TD-02 | MEDIUM | No CHANGELOG until now | Users cannot see changes per release | Not previously maintained | Docs/Release | now (created) |
| TD-04 | MEDIUM | Cross-host/cross-toolchain reproducible builds not verified | Cannot prove artifact provenance bit-for-bit across environments | Single-host pure-Rust reproducibility **measured** (bit-identical); cross-host pending | Release Eng | v1 |
| TD-03 | MEDIUM | No SBOM / build provenance / artifact signing | Release supply chain below best practice | **partially closed** — `scripts/sbom.sh` emits a CycloneDX SBOM attached to each release; the release manifest now records a dependency-lock SHA-256 (self-contained provenance). **signed SLSA provenance** now runs at publish (`actions/attest-build-provenance`, SHA-pinned, with `id-token`/`attestations` permissions) attesting every shipped artifact; it is validated at a real tagged release (`gh attestation verify`). Remaining: cryptographic release *signing* (GPG/cosign) of tags/assets | Release Eng | v1 |
| TD-05 | MEDIUM | No committed performance benchmark suite | Regressions can go unnoticed | **stale — already fixed**: `tests/bench.rs` holds shape guards (always run) plus an `#[ignore]` timing report, with baselines and budgets in `docs/engineering/PERFORMANCE.md` | Performance | closed (stale) |
| TD-06 | MEDIUM | No versioned backward-compatibility fixtures / upgrade tests | Breaking changes could slip silently | Pre-1.0 churn tolerated | Backward Compat | **closed** — `tests/compat.rs` pins 0.2.0 and the 0.2.1 additions with documented intentional breaks; `playground/tests/node/crossrelease.test.mjs` runs every *frozen* runtime (0.0.2, 0.0.2-dev.30, 0.2.0, 0.2.0-dev.1/2, 0.2.1-dev.4) against its language line's fixtures, proving released behavior is unchanged |
| TD-07 | LOW | `E5003` (`FEATURE_UNAVAILABLE`) defined but unused | Dead code constant | Documented in spec | Static Semantics | — |
| TD-08 | LOW | `s` transient file written by a property test at repo root | Untracked artifact noise | Pre-existing test hygiene | QA | **closed** — `run_registry_hermetic` runs the generated `write_file("s", "s")` against `LimitedHost::silent()`, so the capability boundary reports `E5002` and the real filesystem is never touched; root `s` is absent. |
| TD-09 | LOW | Boolean lowering uses Python-compatible keywords (`and/or/not`, `True/False/None`) internally | Cosmetic only; not user-visible Aura syntax | Chosen representation | Runtime | — |
| TD-10 | LOW | Pre-1.0 development runtime identities not uniformly documented in one place | Discoverability | Manifest is authoritative | Playground | — |
| TD-11 | MEDIUM | CPython embedded via `auto-initialize`; no explicit init/shutdown policy | Threading/embedding semantics unclear | v1 scope | CPython | v1 |
| TD-12 | LOW | No `docs/engineering` tree until now | Ops docs scattered | Created now | Program | closed |
| TD-14 | MEDIUM | Type-alias expansion amplification (duplicating parameterized aliases) | A param alias that duplicates its argument made a chain expand to 2^n nodes; the syntactic depth limit could not bound it | **closed** — `MAX_TYPE_NODES` flat node budget turns it into `E1015` (`tests/adversarial.rs`) | Static Semantics | closed |
| TD-15 | LOW | Some `E1015` depth diagnostics carry no `file:line:col` | Red-team F5: deep `module`/loop nesting reported `E1015` without a location | **closed** — added `Expr::span()`/`Stmt::span()` and attributed the expression, statement, and module depth diagnostics to the offending node (`tests/boundaries.rs::depth_diagnostics_carry_a_source_location`) | Diagnostics | closed |
| TD-13 | MEDIUM | REPL submissions are O(N²) in session size | Each submission calls `Checker::with_declarations(&decls)` (src/repl.rs:306, 401), which loops over every prior `GlobalDecl`; N submissions cost Σk = O(N²) registration work. **Re-profiled 2026-10-01 (release):** 12/19/41/115/388/1454 ms for 100/200/400/800/1600/3200 submissions (~3.5× per doubling — clean quadratic). | **Root cause + equivalence obstacle (2026-10-01):** the full rebuild is also the rollback mechanism, and it is *not* equivalent to "keep the checker and mutate it": `check_stmt` on `let x = 5` records the *inferred* type in `value_types[0]`, whereas `with_declarations` only populates `value_types` for a binding whose persisted `ty` is `Some` (an annotated `let`); an unannotated one is left absent on rebuild. A monotonic cache would therefore type later uses more precisely than the rebuild does — an observable divergence, not just a metadata difference. A safe fix must (a) replicate `with_declarations`' exact per-variant registration including the `value_types` asymmetry, (b) restore the pre-statement state on any failed/rolled-back submission, and (c) be proven equivalent by a differential oracle over randomized submission sequences. Until then the O(N²) is confined to the interactive path (thousands of programmatic submissions) with no correctness impact, which the v1 Performance gate accepts as explicitly justified. | Static Semantics / DX | v1.1 |


## Keystone release-candidate debt classification (2026-10-06)

Every open item was classified for the Aura 0.3 Keystone closure:

| Item | Classification |
|---|---|
| TD-22 (interrupted monolithic validation) | **LOCAL VALIDATION GAP, NOT A DEFECT** — the 2026-10-08 canonicalization campaign's single `cargo test --locked --all-features` run was user-stopped after ~50 min (repeated macOS first-exec cost across relinked binaries). All semicolon- and range-affected suites were re-run sharded and are green (syntax_conformance 21/0, syntax_gaps 6/0, corpus 2/0, grammar 6/0, parser 48/0, boundaries 35/0, run 63/0, checker 55/0, contract 11/0, regressions 72/0, compat 17/0, examples 1/0, syntax_docs 3/0, language_metadata 6/0, evaluator_oracle 48/0, keystone_value_algebra 18/0, ais_properties 10/0, modules 20/0, io 20/0; fmt + clippy `-D warnings` clean). Re-run the full matrix sharded via the watchdog before any push. | 
| TD-21 (optionality permissiveness) | **REAL DEFECT IN THE KEYSTONE CONTRACT — FIXED** (`7959487`); deleted from this register |
| TD-20 (ungated feature tests) | **REAL DEFECT IN THE KEYSTONE VALIDATION SURFACE — FIXED** (`6bec6e3`); closed |
| TD-08 (`s` root artifact) | **STALE — ALREADY FIXED** by the hermetic host; closed |
| TD-05 (no benchmark suite) | **STALE — ALREADY FIXED** (`tests/bench.rs`, `PERFORMANCE.md`); closed |
| TD-15 (depth diagnostics without location) | closed previously (evidence in the row) |
| TD-07 (`E5003` unused) | **ARCHITECTURAL LIMITATION, BY DESIGN** — the spec states `E5003` "is not part of the normative surface"; a feature-gated builtin absent from a reduced build is `E2003`, which is the same code an unknown builtin gets, so the diagnostic is not less precise in any way a program can observe |
| TD-09 (internal boolean lowering names) | **FALSE POSITIVE** — internal representation only; no user-visible surface |
| TD-10 (dev runtime identities) | **ARCHITECTURAL LIMITATION** — the manifest is authoritative; discoverability only |
| TD-11 (CPython init policy) | **LEGITIMATE FUTURE FEATURE** (v1 scope); unchanged by Keystone, which explicitly kept Python behind the provider boundary |
| TD-13 (REPL O(N²)) | **LEGITIMATE FUTURE FEATURE** — no correctness impact; the row records the equivalence obstacle and the v1.1 plan |
| TD-14, TD-06, TD-12, TD-18 | closed previously (evidence in the rows) |
| TD-19 (recursive engine) | closed in source; frozen-artifact and B-1R8 residuals are **HUMAN-GATED**, not defects in current Keystone behavior |
| TD-01/TD-02/TD-03/TD-04/TD-17 (release/supply-chain) | **LEGITIMATE FUTURE RELEASE ENGINEERING** — outside the implemented language/runtime scope; TD-02 is fixed (CHANGELOG exists) |

No item classified "REAL DEFECT IN THE KEYSTONE CONTRACT" remains open.

## Rules

- Debt is added/closed by editing this table with an owner and target.
- A HIGH item blocks the corresponding release if not closed or explicitly
  accepted by a human decision.
| TD-17 | LOW | `dtolnay/rust-toolchain` referenced by version ref, not SHA | A mutable version ref for one action is a residual supply-chain risk | Version refs are the action's intended interface; SHA-pinning needs an explicit `toolchain:` input at ~19 sites | Supply Chain | v1.1 |
| TD-18 | LOW | npm dev/test dependencies had no committed lockfile | Non-reproducible local browser-test installs; no audit trail | Now committed (`website/package-lock.json`, `playground/package-lock.json`) and audited in CI | Supply Chain | closed |
| TD-19 | HIGH | Recursive tree-walking interpreter consumes unbounded host engine-stack frames per Aura call (B-1) | `CLOSED IN SOURCE` — the explicit-continuation machine is the production engine on every entry point since `62dd592` (remote-closed at `bb736fc`); the fresh machine-backed WASM build reaches the 512-frame limit and reports structured `E4011` (63-check boundary suite). Residual: the **frozen released artifacts** `0.0.2`/`0.2.0`/`0.2.1` still contain the defect because historical artifacts are immutable; they are superseded only when a new runtime is published (human-gated). B-1R8 (removal of the retained recursive differential reference) remains human-gated. | Runtime / Resource Safety | closed in source; publication human-gated |
| TD-20 | LOW | 13 tests across 8 suites call feature-gated builtins (`json_*`/`regex_*`/`time_*`/`py_*`) without `#[cfg]` gates | **closed** — every feature-dependent test now carries its `#[cfg(feature = …)]` gate (and two Unknown-boundary tests were rewritten to a feature-independent unannotated call, preserving what they actually pin). All three matrices are green: bare `--no-default-features` 1014/0, canonical `cli,repl,json,regex,time` 1148/0, `--all-features` 1223/0; both library builds are warning-free. | QA / Static Semantics | closed |
| TD-22 | MEDIUM | `json_encode` silently approximated value kinds with no exact JSON form (range/function/non-finite float → `null`, variant → payload/string/array) | **REAL DEFECT IN THE KEYSTONE VALUE ALGEBRA — FIXED** — `null` is the encoding of `none` (and of the §31.6 truncation), so `json_decode(json_encode(x))` collapsed `0..3`/closure/`nan`/`none` into one value, and `A(1)`/`B(1)` encoded identically. Now a deterministic `E3001` naming the kind, uniform with the existing non-string-map-key rule. See `docs/engineering/JSON_VALUE_ALGEBRA_DECISION.md`; regression evidence in `tests/keystone_json.rs`, matrix in `tests/keystone_value_algebra.rs`. | Value Algebra / JSON | closed |
