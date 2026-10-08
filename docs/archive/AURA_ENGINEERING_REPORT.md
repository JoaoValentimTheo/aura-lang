# Aura Engineering Report — Road to 1.0 (Train T0–T5)

> **Historical record.** This report describes the T0–T5 engineering trains at
> HEAD `584af85`. The HD-4/version-identity questions it records as open were
> later resolved by ADR-0001 and released as `0.2.1`. For current state see
> `AGENT_STATE.md`.

Program: **Aura Autonomous Engineering Organization — Road to Aura 1.0**.
Base `6d25985` → HEAD `584af85` (pushed; CI green). Nothing was released or
tagged; frozen artifacts unchanged.

## 1. Executive summary

The organization established its durable operating scaffolding (roles,
readiness matrix, roadmap, decision queue, security program, CPython target,
release policy, benchmarks, registers, changelog), then executed five
engineering trains: a real bug fix, a full CPython interoperability definition
and test matrix with multi-version CI, a security program with supply-chain
hardening, a performance baseline with scaling guards, and backward-
compatibility fixtures.

Before v1.0, the remaining blockers are **human semantic/version decisions**,
not undiscovered engineering. Every category that can be advanced without a
human decision has been advanced and independently verified where required.

## 2. Starting / final state

- Start: base `6d25985` = remote; CI green; frozen `0.0.2`/`0.2.0` intact.
- Final: `584af85` = remote; CI green (run `36777716800`); frozen intact.

## 3. What was delivered

### Real defect fixed
- **CPython boundary diagnostic provenance.** `py_eval`/`py_import`/`py_call`
  failures and all boundary conversion errors were emitted at `Span::default()`
  (`1:1`). Threaded the call-site span through both conversion directions.
  Regression-tested; **independently reviewed and not falsified**.

### Security program
- `SECURITY.md`, `docs/security/{THREAT_MODEL,TRUST_BOUNDARIES,SECURITY_ARCHITECTURE,WASM_IMPORT_POLICY,INCIDENT_RESPONSE}.md`.
- Explicitly documents that the `py` feature is **arbitrary CPython authority —
  not a sandbox**.
- **WASM zero-import policy**, independently verified via
  `WebAssembly.Module.imports` (0 imports for 0.0.2, 0.2.0, 0.2.0-dev.2).
- **Supply chain:** all standard GitHub Actions pinned to commit SHAs; CI runs
  least-privilege (`contents: read`). TD-01 closed.

### CPython interoperability
- `docs/CPYTHON_COMPATIBILITY_TARGET.md`: 12 compatibility dimensions, each with
  explicit v1 scope, plus the normative value-conversion table.
- `tests/interop_matrix.rs` (14 tests) validating the table in both directions.
- CI job `cpython-interop` green on CPython **3.10/3.11/3.12/3.13** (Linux) and
  **3.12** (macOS).

### Performance
- `tests/bench.rs`: noise-robust scaling shape guards for lex/parse/check/
  module-graph/runtime/REPL; `#[ignore]` timing report.
- `docs/engineering/PERFORMANCE.md`: baseline (lex/parse/check linear to
  N=1600) and budgets.
- Found a genuine **O(N²) REPL characteristic** (documented as TD-13, guard
  fails only on cubic) — a performance fact, not a correctness bug.

### Compatibility & release
- `tests/compat.rs` pins the released language surface behaviorally.
- `docs/engineering/RELEASE_ENGINEERING.md`, `CHANGELOG.md`, `docs/INSTALL.md`,
  `docs/engineering/{TECHNICAL_DEBT,RISK_REGISTER}.md`.
- **Measured** single-host pure-Rust build reproducibility (bit-identical).

## 4. Independent review outcomes

- Bridge fix: **not falsified** (zero remaining `Span::default()` under `py`;
  provenance reaches every diagnostic).
- Frozen-artifact verification: **all PASS** (hashes/sizes/manifest/imports).
- Filesystem module boundary red team: symlinks rejected; children
  qualified-only; no defect.
- **One MEDIUM finding, self-corrected:** `tests/compat.rs` mislabeled a
  post-`v0.2.0` rule as released 0.2.0 (the reservation commit `c8ded06` is not
  in tag `v0.2.0`). Fixed; exposed a real version-identity divergence recorded
  as HD-4 evidence.
- **One worktree incident:** 13 tracked `.kilo/` operator-config files were
  found deleted and restored via `git checkout -- .kilo/`; no history impact.
- Full record: `docs/engineering/REVIEW_RECORD_T0_T5.md`.

## 5. Human decisions required (the actual v1 blockers)

| ID | Decision | Blocks |
|---|---|---|
| **HD-4** | Pre-v1 version scheme; may release > language pre-1.0? | The first patch release and every pre-1.0 tag |
| **HD-3** | Supported CPython versions (proposed 3.10–3.13, enforced) | Formal CPython claim |
| **HD-2** | AUDIT-3 TypeExpr nesting policy | Semantic freeze |
| **HD-1** | Module-member identifiers vs builtin reservation | Semantic freeze (low impact) |

Evidence for each is in `HUMAN_DECISIONS_QUEUE.md`. The concrete new evidence:
HEAD declares `VERSION == LANGUAGE_VERSION == 0.2.0` but differs semantically
from tag `v0.2.0`, and `tests/contract.rs` forbids release ≠ language — so no
honest patch release is possible until HD-4 is decided.

## 6. Validation

- Local gate: fmt, all-features (905), no-default (879), clippy `-D warnings`,
  MSRV, Miri, runtime crate, playground, website, build `--check` — all green.
- Remote CI: green across ubuntu/macos/windows, CPython 3.10–3.13, MSRV, Miri,
  clippy, cargo audit, fuzz smoke, playground, website.
- Frozen: `0.0.2` = 1,366,621 / `5a4ad3f7…`; `0.2.0` = 1,654,161 / `9937fd80…`
  (byte-identical, re-verified).
- Git audit: `git diff --check 6d25985..HEAD` clean; only untracked `s`.

## 7. Remaining risks

No known CRITICAL/HIGH. Open: TD-03 (SBOM/provenance), TD-04 (cross-host
reproducibility), TD-11 (CPython threading policy), TD-13 (REPL O(N²)),
R-08/R-12/R-13 (perf/compat/versioning). None blocks non-release engineering.

## 8. Recommended next human action

1. Decide **HD-4** — this unblocks the first honest gated release
   (patch `0.2.1`, or a prerelease scheme). Recommendation: allow release >
   language pre-1.0.
2. Confirm **HD-3** supported CPython versions.
3. Schedule the semantic-freeze decisions (**HD-2**, **HD-1**) before any
   `1.0.0-rc` train.

---

FINAL: AURA COMPLETION PROGRAM — HUMAN DECISIONS REQUIRED

All independently executable engineering for this program stage is complete,
verified, and pushed; a justified Aura v1.0 requires the four queued human
decisions (chiefly HD-4) before release tagging and the RC train can proceed.
