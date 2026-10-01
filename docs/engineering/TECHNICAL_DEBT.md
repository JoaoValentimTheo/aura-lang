# Aura Technical Debt Register

Known debt, explicitly tracked (not buried in conversation logs). Severity:
HIGH / MEDIUM / LOW.

| ID | Severity | Item | Impact | Deferred reason | Owner | Target |
|---|---|---|---|---|---|---|
| TD-01 | MEDIUM | GitHub Actions use major-version tags, not SHA pins | Supply-chain: a compromised action tag could affect builds/releases | Standard practice; pinning is a hardening step | Supply Chain | **closed** — all standard actions SHA-pinned; `ci.yml` least-privilege |
| TD-02 | MEDIUM | No CHANGELOG until now | Users cannot see changes per release | Not previously maintained | Docs/Release | now (created) |
| TD-04 | MEDIUM | Cross-host/cross-toolchain reproducible builds not verified | Cannot prove artifact provenance bit-for-bit across environments | Single-host pure-Rust reproducibility **measured** (bit-identical); cross-host pending | Release Eng | v1 |
| TD-03 | MEDIUM | No SBOM / build provenance / artifact signing | Release supply chain below best practice | **partially closed** — `scripts/sbom.sh` emits a CycloneDX SBOM attached to each release; the release manifest now records a dependency-lock SHA-256 (self-contained provenance). **signed SLSA provenance** now runs at publish (`actions/attest-build-provenance`, SHA-pinned, with `id-token`/`attestations` permissions) attesting every shipped artifact; it is validated at a real tagged release (`gh attestation verify`). Remaining: cryptographic release *signing* (GPG/cosign) of tags/assets | Release Eng | v1 |
| TD-05 | MEDIUM | No committed performance benchmark suite | Regressions can go unnoticed | Ad-hoc measurement only | Performance | now (T4) |
| TD-06 | MEDIUM | No versioned backward-compatibility fixtures / upgrade tests | Breaking changes could slip silently | Pre-1.0 churn tolerated | Backward Compat | **partially closed** — `tests/compat.rs` pins 0.2.0 and the 0.2.1 additions with documented intentional breaks; running released artifacts against the new source is still pending |
| TD-07 | LOW | `E5003` (`FEATURE_UNAVAILABLE`) defined but unused | Dead code constant | Documented in spec | Static Semantics | — |
| TD-08 | LOW | `s` transient file written by a property test at repo root | Untracked artifact noise | Pre-existing test hygiene | QA | — |
| TD-09 | LOW | Boolean lowering uses Python-compatible keywords (`and/or/not`, `True/False/None`) internally | Cosmetic only; not user-visible Aura syntax | Chosen representation | Runtime | — |
| TD-10 | LOW | Pre-1.0 development runtime identities not uniformly documented in one place | Discoverability | Manifest is authoritative | Playground | — |
| TD-11 | MEDIUM | CPython embedded via `auto-initialize`; no explicit init/shutdown policy | Threading/embedding semantics unclear | v1 scope | CPython | v1 |
| TD-12 | LOW | No `docs/engineering` tree until now | Ops docs scattered | Created now | Program | closed |
| TD-14 | MEDIUM | Type-alias expansion amplification (duplicating parameterized aliases) | A param alias that duplicates its argument made a chain expand to 2^n nodes; the syntactic depth limit could not bound it | **closed** — `MAX_TYPE_NODES` flat node budget turns it into `E1015` (`tests/adversarial.rs`) | Static Semantics | closed |
| TD-15 | LOW | Some `E1015` depth diagnostics carry no `file:line:col` | Red-team F5: deep `module`/loop nesting reports `E1015` without a location, unlike most diagnostics | Cosmetic; the code is stable and the message correct | Diagnostics | v1.1 |
| TD-13 | MEDIUM | REPL submissions are O(N²) in session size | Each submission calls `Checker::with_declarations(&decls)` (src/repl.rs:306, 401), which loops over every prior `GlobalDecl`; N submissions cost Σk = O(N²) registration work; measured 16/49/175/661/2576/10162 ms for 100/200/400/800/1600/3200 submissions. **Root cause (2026-09-30):** the full rebuild is also the rollback mechanism (simplest correct state), and its registrations are *synthetic* (`Span::default()`, `source: None`, `order: None`, `public: true`), whereas the live check path registers *real* metadata. A naive "cache the checker and mutate it in place" changes the metadata that cross-submission diagnostics use, so equivalence is not free. | Correctness unaffected; a safe fix needs an exactly-equivalent incremental `absorb_decl` mirroring `with_declarations` per variant, plus a differential harness asserting identical stdout/diagnostics against the always-rebuild oracle over randomized submission sequences. Deferred past v1-core by design. | Static Semantics / DX | v1.1 |

## Rules

- Debt is added/closed by editing this table with an owner and target.
- A HIGH item blocks the corresponding release if not closed or explicitly
  accepted by a human decision.
