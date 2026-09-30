# Aura Technical Debt Register

Known debt, explicitly tracked (not buried in conversation logs). Severity:
HIGH / MEDIUM / LOW.

| ID | Severity | Item | Impact | Deferred reason | Owner | Target |
|---|---|---|---|---|---|---|
| TD-01 | MEDIUM | GitHub Actions use major-version tags, not SHA pins | Supply-chain: a compromised action tag could affect builds/releases | Standard practice; pinning is a hardening step | Supply Chain | v1 |
| TD-02 | MEDIUM | No CHANGELOG until now | Users cannot see changes per release | Not previously maintained | Docs/Release | now (created) |
| TD-03 | MEDIUM | No SBOM / build provenance / artifact signing | Release supply chain below best practice | Needs infra decision | Release Eng | v1 |
| TD-04 | MEDIUM | Reproducible builds not verified | Cannot prove artifact provenance bit-for-bit | Not measured | Release Eng | v1 |
| TD-05 | MEDIUM | No committed performance benchmark suite | Regressions can go unnoticed | Ad-hoc measurement only | Performance | now (T4) |
| TD-06 | MEDIUM | No versioned backward-compatibility fixtures / upgrade tests | Breaking changes could slip silently | Pre-1.0 churn tolerated | Backward Compat | v1 |
| TD-07 | LOW | `E5003` (`FEATURE_UNAVAILABLE`) defined but unused | Dead code constant | Documented in spec | Static Semantics | — |
| TD-08 | LOW | `s` transient file written by a property test at repo root | Untracked artifact noise | Pre-existing test hygiene | QA | — |
| TD-09 | LOW | Boolean lowering uses Python-compatible keywords (`and/or/not`, `True/False/None`) internally | Cosmetic only; not user-visible Aura syntax | Chosen representation | Runtime | — |
| TD-10 | LOW | Pre-1.0 development runtime identities not uniformly documented in one place | Discoverability | Manifest is authoritative | Playground | — |
| TD-11 | MEDIUM | CPython embedded via `auto-initialize`; no explicit init/shutdown policy | Threading/embedding semantics unclear | v1 scope | CPython | v1 |
| TD-12 | LOW | No `docs/engineering` tree until now | Ops docs scattered | Created now | Program | closed |

## Rules

- Debt is added/closed by editing this table with an owner and target.
- A HIGH item blocks the corresponding release if not closed or explicitly
  accepted by a human decision.
