# Aura Engineering Checkpoint

Resumable operational state for the Road-to-1.0 program.

## State

```
PROGRAM HEAD:             3f5f8702 (pushed) + local post-release reconciliation (unpushed)
REMOTE HEAD:              3f5f8702
CURRENT RELEASE:         v0.2.1 (tagged; published 2026-10-01; immutable)
CURRENT TRAIN:           TRAIN 6 — COMPLETE (Aura 0.2.1 published); next program not selected
LATEST DEV RUNTIME:      0.2.1 (promoted release runtime); 0.2.1-dev.5 preserved
TEAM STATUS:             language/runtime/security GREEN; CPython, performance,
                         release engineering, docs advancing
DECISIONS CLOSED:        HD-1, HD-2 (AUDIT-3), HD-3, HD-4 — all four, via ADR-0001…0004
ACTIVE FINDINGS:         train-1 F1–F4 (FIXED); re-verification N1(HIGH),
                         N2/N3(MEDIUM) (FIXED); F5(LOW, tracked TD-15),
                         F6(INFO, by design)
BUGS FIXED:              F1–F4; re-verification N1–N3 (parameterized-alias
                         expansion hang + depth bypass, physical module-depth
                         bypass); CPython dict-key identity + depth symmetry;
                         alias expansion amplification (TD-14)
PERFORMANCE RESULTS:     parser fuzz 200k + checker 150k + runtime 80k clean;
                         13 sub-quadratic scaling guards incl. map/call/string
SECURITY RESULTS:        campaign recorded (docs/security/CAMPAIGN_LOG.md);
                         no known CRITICAL/HIGH open
COMMITS:                 ADR-0001…0004; security F1–F4; CPython fixes; runtime
                         0.2.1-dev.4; SBOM + release manifest; perf breadth;
                         compat fixtures; decision-doc resolutions
PUSHES:                  …→3f5f8702 (all CI-green; tag v0.2.1)
CI:                      GREEN on 3f5f8702 (27/27 check-runs, incl. release publish)
RELEASES:                v0.2.1 published 2026-10-01 (WASM 1,768,322 / 48c456fc…;
                         Linux/macOS/Windows tarballs; manifest + SBOM attached)
V1 GREEN:                every readiness capability category — LANGUAGE, TYPE
                         SYSTEM, RUNTIME, STDLIB, MODULES, REPL, CLI, RESOURCE
                         LIMITS, DETERMINISM, WASM, PLAYGROUND, LINUX/MACOS/
                         WINDOWS, CI, CPYTHON, SECURITY, PERFORMANCE, DOCS,
                         PACKAGING, RELEASE ENGINEERING, SUPPLY CHAIN,
                         BACKWARD COMPAT
V1 YELLOW:               none
V1 RED:                  v1 GO — requires a published, post-release-verified 1.0;
                         the prerelease→RC→1.0 train remains
FROZEN HASHES:           0.0.2 = 1,366,621 / 5a4ad3f7…; 0.2.0 = 1,654,161 / 9937fd80…;
                         0.2.1 = 1,768,322 / 48c456fc…
NEXT PHASE:              NOT SELECTED — human review of the post-v0.2.1
                         reconciliation first; do not start a new program
```

## Train 1 — completed work

- **HD-4 → ADR-0001**: release distinct from language version;
  `LANGUAGE_VERSION <= RELEASE_VERSION`; line advances to 0.2.1.
- **HD-1 → ADR-0002**: module members may reuse builtin spellings; reservation
  scoped to the user-visible value namespace.
- **HD-3 → ADR-0003**: CPython support tiers (TESTED Linux 3.10–3.13, macOS
  3.12; best-effort other 3.10–3.13; Windows 3.12 later promoted to TESTED
  2026-10-01 when its CI leg proved reliable).
- **HD-2 / AUDIT-3 → ADR-0004**: structural type nesting counts toward the
  semantic AST limit (256) on every substrate; flat unions exempt.
- **TD-14 (new)**: checker type-alias expansion budget bounds exponentially
  duplicating alias chains (`E1015`).
- **CPython bridge**: exact-type identity for dict keys (was duck-typed
  `__index__`); depth bounded symmetrically (Python→Aura rejects, no silent
  `repr`); doc wording reconciled to the implementation.
- Runtime `0.2.1-dev.4`; playbook/website/docs synced to the new version line.

## Train 2 — completed work

- **SBOM** (`scripts/sbom.sh`, tested): CycloneDX 1.5 over both workspaces,
  attached to releases; the SBOM half of TD-03 is closed.
- **Release manifest** (`scripts/release-manifest.sh`, tested): machine-readable
  identities (ADR-0001 invariant, runtimes, ADR-0003 matrix) attached to
  releases.
- **End-user artifact smoke** (`scripts/artifact-smoke.sh`): 12 checks against
  the installed binary, run in CI and the release gate.
- **Performance breadth**: sub-quadratic guards for map ops, function calls,
  string iteration, long-string lexing, and alias-chain resolution.
- **Compatibility**: `tests/compat.rs` now pins the `0.2.1` additions with
  documented intentional breaks; stale AUDIT-3/HD-1 "pending" markers removed
  across `AGENTS.md`, decision packages, corpus, and property tests.
- **WASM runtime** `0.2.1-dev.4` carries the train-1 + re-verification hardening.

## Root-caused, deferred (not defects)

- **TD-13** (REPL O(N²)): `Checker::with_declarations` rebuilds the environment
  each submission and is *also* the rollback mechanism; its registrations are
  synthetic (`Span::default()`, `source: None`) unlike the live path. A safe
  fix needs an exactly-equivalent incremental `absorb_decl` + a differential
  oracle. Design recorded in `docs/engineering/TECHNICAL_DEBT.md` (v1.1).

## Train 3 — completed work

- **Security re-verification closure**: N1 (HIGH parameterized-alias hang),
  N2 (depth-limit bypass), N3 (physical module-depth bypass) fixed; deep fuzz
  campaign clean (parser 2.0M+, checker 1.18M, runtime 1,087 runs).
- **CPython formalization**: the Windows 3.12 `py` probe ran reliably green, so
  ADR-0003 is amended and Windows is promoted to TESTED (blocking CI leg); the
  lifetime/GIL review (dimension M) is documented; exact-type dict-key rule.
- **Supply chain**: CycloneDX SBOM + release manifest with a dependency-lock
  SHA-256 (self-contained provenance). Signed SLSA attestation remains tracked.
- **Performance**: sub-quadratic guards across lex/parse/check, module graph,
  map/call/string runtime ops, long-string lexing, alias chains; TD-13
  (REPL O(N²)) root-caused and explicitly justified; PERFORMANCE → GREEN.
- **Packaging**: end-user installed-binary smoke in CI and the release gate;
  PACKAGING → GREEN.
- **Docs**: 0.2.1 migration guide (repo + website); stale AUDIT-3/HD-1 "pending"
  markers removed across `AGENTS.md`, decision packages, corpus, property tests.
- **Runtimes**: `0.2.1-dev.3` (train-1 hardening) and `0.2.1-dev.4`
  (re-verification checker fixes), both zero-import.

## Train 4 — completed work

- **Signed provenance**: `actions/attest-build-provenance` (SHA-pinned) attests
  every shipped artifact at publish (TD-03; release signing remains).
- **Website version awareness**: stable vs development version labeled
  site-wide (ADR-0001 identity model).
- **Cross-release compatibility** (TD-06, closed): every frozen runtime runs its
  language line's fixtures with unchanged output; BACKWARD COMPAT → GREEN.
- **CPython formalization**: conversion edge cases + a generated conversion
  matrix (no host failure); CPYTHON → GREEN.

## Train 5 — completed work

- **TD-15 closed**: depth-limit `E1015` now points at the offending node
  (added `Expr::span()`/`Stmt::span()`); new dev runtime `0.2.1-dev.5`.
- **Release path prepared**: `docs/release-notes/v0.2.1.md` written; the
  release preflight passes everything except the runtime promotion.

## Train 6 — COMPLETE: Aura 0.2.1 published

- **Runtime promoted**: `playground/runtime` advanced from `0.2.1-dev.5` to the
  clean release identity `0.2.1`; built and pinned the `0.2.1` WASM artifact
  (`1768322` bytes, sha256 `48c456fc…`), zero host imports, Host ABI 1. It is
  contract-identical to `0.2.1-dev.5` (no semantic change intervened).
- **Immutability**: `FROZEN_0_2_1` pinned in `playground/build.mjs` with a
  dedupe guard so the release entry survives a later crate advance; `--check`
  and the frozen-artifact sweep both cover it.
- **QA**: `tests/cross_subsystem.rs` added (8 cross-subsystem interaction
  cases); Playground browser label test made channel-aware (no longer assumes
  the current runtime is always a development build).
- **Released**: `v0.2.1` tagged at `3f5f870` (annotated tag object
  `df755340…`) and published 2026-10-01T20:29:36Z, non-draft, non-prerelease.
  Release workflow `36920912666` succeeded (validate + three platform builds +
  publish); 27/27 check-runs green on the release SHA; GitHub Pages deploy
  green. A tag-triggered preflight defect found while publishing was fixed in
  `3f5f870` and guarded by `release_preflight.test.mjs`.

## Next tasks

1. **HUMAN REVIEW OF POST-v0.2.1 STATE + WEBSITE ALIGNMENT.** The `0.2.1`
   release train is closed; a local reconciliation pass (authority docs +
   website `0.2.1` alignment + release-version drift guard) awaits human review
   before any push decision.
2. Remaining non-blocking extras: cryptographic tag/asset signing (TD-03),
   `rust-toolchain` SHA-pinning (TD-17), TD-13 incremental REPL (v1.1).
3. The next major engineering program is **not selected**; do not start one
   without explicit human authorization.
