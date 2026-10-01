# Aura Technical Debt Register

Known debt, explicitly tracked (not buried in conversation logs). Severity:
HIGH / MEDIUM / LOW.

| ID | Severity | Item | Impact | Deferred reason | Owner | Target |
|---|---|---|---|---|---|---|
| TD-01 | MEDIUM | GitHub Actions use major-version tags, not SHA pins | Supply-chain: a compromised action tag could affect builds/releases | Standard practice; pinning is a hardening step | Supply Chain | **closed for third-party actions** — all standard actions SHA-pinned; `ci.yml` least-privilege. `dtolnay/rust-toolchain` is the tracked exception (TD-17) |
| TD-02 | MEDIUM | No CHANGELOG until now | Users cannot see changes per release | Not previously maintained | Docs/Release | now (created) |
| TD-04 | MEDIUM | Cross-host/cross-toolchain reproducible builds not verified | Cannot prove artifact provenance bit-for-bit across environments | Single-host pure-Rust reproducibility **measured** (bit-identical); cross-host pending | Release Eng | v1 |
| TD-03 | MEDIUM | No SBOM / build provenance / artifact signing | Release supply chain below best practice | **partially closed** — `scripts/sbom.sh` emits a CycloneDX SBOM attached to each release; the release manifest now records a dependency-lock SHA-256 (self-contained provenance). **signed SLSA provenance** now runs at publish (`actions/attest-build-provenance`, SHA-pinned, with `id-token`/`attestations` permissions) attesting every shipped artifact; it is validated at a real tagged release (`gh attestation verify`). Remaining: cryptographic release *signing* (GPG/cosign) of tags/assets | Release Eng | v1 |
| TD-05 | MEDIUM | No committed performance benchmark suite | Regressions can go unnoticed | Ad-hoc measurement only | Performance | now (T4) |
| TD-06 | MEDIUM | No versioned backward-compatibility fixtures / upgrade tests | Breaking changes could slip silently | Pre-1.0 churn tolerated | Backward Compat | **closed** — `tests/compat.rs` pins 0.2.0 and the 0.2.1 additions with documented intentional breaks; `playground/tests/node/crossrelease.test.mjs` runs every *frozen* runtime (0.0.2, 0.0.2-dev.30, 0.2.0, 0.2.0-dev.1/2, 0.2.1-dev.4) against its language line's fixtures, proving released behavior is unchanged |
| TD-07 | LOW | `E5003` (`FEATURE_UNAVAILABLE`) defined but unused | Dead code constant | Documented in spec | Static Semantics | — |
| TD-08 | LOW | `s` transient file written by a property test at repo root | Untracked artifact noise | Pre-existing test hygiene | QA | — |
| TD-09 | LOW | Boolean lowering uses Python-compatible keywords (`and/or/not`, `True/False/None`) internally | Cosmetic only; not user-visible Aura syntax | Chosen representation | Runtime | — |
| TD-10 | LOW | Pre-1.0 development runtime identities not uniformly documented in one place | Discoverability | Manifest is authoritative | Playground | — |
| TD-11 | MEDIUM | CPython embedded via `auto-initialize`; no explicit init/shutdown policy | Threading/embedding semantics unclear | v1 scope | CPython | v1 |
| TD-12 | LOW | No `docs/engineering` tree until now | Ops docs scattered | Created now | Program | closed |
| TD-14 | MEDIUM | Type-alias expansion amplification (duplicating parameterized aliases) | A param alias that duplicates its argument made a chain expand to 2^n nodes; the syntactic depth limit could not bound it | **closed** — `MAX_TYPE_NODES` flat node budget turns it into `E1015` (`tests/adversarial.rs`) | Static Semantics | closed |
| TD-15 | LOW | Some `E1015` depth diagnostics carry no `file:line:col` | Red-team F5: deep `module`/loop nesting reports `E1015` without a location, unlike most diagnostics | Cosmetic; the code is stable and the message correct | Diagnostics | v1.1 |
| TD-13 | MEDIUM | REPL submissions are O(N²) in session size | Each submission calls `Checker::with_declarations(&decls)` (src/repl.rs:306, 401), which loops over every prior `GlobalDecl`; N submissions cost Σk = O(N²) registration work. **Re-profiled 2026-10-01 (release):** 12/19/41/115/388/1454 ms for 100/200/400/800/1600/3200 submissions (~3.5× per doubling — clean quadratic). | **Root cause + equivalence obstacle (2026-10-01):** the full rebuild is also the rollback mechanism, and it is *not* equivalent to "keep the checker and mutate it": `check_stmt` on `let x = 5` records the *inferred* type in `value_types[0]`, whereas `with_declarations` only populates `value_types` for a binding whose persisted `ty` is `Some` (an annotated `let`); an unannotated one is left absent on rebuild. A monotonic cache would therefore type later uses more precisely than the rebuild does — an observable divergence, not just a metadata difference. A safe fix must (a) replicate `with_declarations`' exact per-variant registration including the `value_types` asymmetry, (b) restore the pre-statement state on any failed/rolled-back submission, and (c) be proven equivalent by a differential oracle over randomized submission sequences. Until then the O(N²) is confined to the interactive path (thousands of programmatic submissions) with no correctness impact, which the v1 Performance gate accepts as explicitly justified. | Static Semantics / DX | v1.1 |

## Rules

- Debt is added/closed by editing this table with an owner and target.
- A HIGH item blocks the corresponding release if not closed or explicitly
  accepted by a human decision.
| TD-17 | LOW | `dtolnay/rust-toolchain` referenced by version ref, not SHA | A mutable version ref for one action is a residual supply-chain risk | Version refs are the action's intended interface; SHA-pinning needs an explicit `toolchain:` input at ~19 sites | Supply Chain | v1.1 |
