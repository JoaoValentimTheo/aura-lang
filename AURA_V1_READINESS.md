# Aura V1 Readiness

Evidence-based readiness for a justified Aura v1.0. Colour is set by evidence,
never by schedule. Status legend: **GREEN** (ready), **YELLOW** (gaps remain),
**RED** (blocking).

| Category | State | Evidence / gap |
|---|---|---|
| LANGUAGE | GREEN | Core COMPLETE; `AURA_COMPLETENESS_MATRIX.md`; no accidental PARTIAL/MISSING; spec/grammar/token contracts tested |
| TYPE SYSTEM | GREEN | checker↔runtime agreement invariant; typed rejection paths exercised |
| RUNTIME | GREEN | no host panic from input; bounded failure; cycles/bounds tested |
| STDLIB | GREEN | 38 builtins + methods; registry-drift property test; value-semantics tests |
| MODULES | GREEN | providers parity; eager discovery defined; visibility/collision tested |
| CLI | GREEN | `tests/cli.rs`; exit codes; paths; stdin |
| REPL | GREEN | persistence + rollback; `tests/repl.rs` |
| WASM | GREEN (source) / YELLOW (published artifacts) | 0 imports; Host ABI 1; differential 228/228; module/alias depth unified with native. **B-1 is fixed in source and proven on a fresh machine-backed build:** every mainstream recursion shape reaches the 512-frame language limit and reports a structured `E4011` instead of trapping (fresh-wasm suite, 53 checks). The **released** artifacts `0.0.2`/`0.2.0`/`0.2.1` remain affected because frozen artifacts are immutable; they are superseded only by a future human-gated runtime publication. Production (native, CLI, REPL, WASM source build) runs the explicit-continuation machine. `docs/WASM_CALL_FRAME_LIMIT_DECISION.md` |
| PLAYGROUND | GREEN | FSM-P6 suites green; multi-file project state + tabs; capability detection; released `0.2.1` runtime is the selector default |
| CPYTHON | GREEN | normative conversion table; interop matrix + edge cases + a generated conversion matrix (no host failure); call-site source spans tested; TESTED tiers (ADR-0003): Linux 3.10–3.13, macOS 3.12, Windows 3.12 (all CI legs blocking); lifetime/GIL reviewed (dimension M); security boundary explicit (arbitrary CPython authority, no sandbox claimed) |
| SECURITY | GREEN | threat model / trust boundaries / architecture / incident response current; red-team campaign + re-verification recorded, all CRITICAL/HIGH fixed and guarded; deep fuzz campaign clean; no `unsafe` and Miri is a blocking CI gate; cargo audit reviewed; WASM zero-import policy verified; CPython arbitrary-authority boundary documented; release pipeline reviewed; SBOM + signed SLSA provenance (release signing is a supply-chain extra, tracked TD-03) |
| PERFORMANCE | GREEN | sub-quadratic scaling guards for lex/parse/check, module graph, list/map runtime ops, function calls, string iteration, long-string lexing, and alias chains; budgets set (`docs/engineering/PERFORMANCE.md`); no input-driven catastrophic complexity in the deep fuzz campaign; TD-13 (REPL O(N^2)) re-profiled (clean quadratic), root-caused to the per-submission checker rebuild, and explicitly justified with a concrete equivalence-safe fix design recorded |
| RESOURCE LIMITS | GREEN | limits tested limit−1/limit/+1; recovery verified |
| DETERMINISM | GREEN | single-output diagnostics; ordered maps |
| PLATFORM — LINUX | GREEN | CI green |
| PLATFORM — MACOS | GREEN | CI green |
| PLATFORM — WINDOWS | GREEN | CI green |
| DOCS | GREEN | every V1 documentation-gate item is present and current: install, quickstart, language guide, reference, stdlib, modules, REPL, CLI, Python interop, security, Playground, migration, and known limitations; CHANGELOG current; every published example is validated against the runtime and Aura comments are checked; the website presents the released version (`0.2.1`) |
| PACKAGING | GREEN | release workflow builds Linux/macOS/Windows + wasm with per-artifact checksums and a built-binary smoke before release; install guide (`docs/INSTALL.md`, website); end-user installed-binary smoke (`scripts/artifact-smoke.sh`) runs in CI and the release gate; artifact matrix documented; the produced release artifact is exercised post-publish by the release workflow |
| CI | GREEN | comprehensive matrix; all green; now includes a CPython interop version/platform job |
| RELEASE ENGINEERING | GREEN | gated release workflow (validate→build→publish) + policy doc + CHANGELOG; ADR-0001 version scheme; metadata-consistency and release-preflight gates (version identities, runtime artifact, tag immutability); release-manifest + SBOM attached; signed SLSA provenance at publish; documented runbook. Cryptographic tag/asset signing is a supply-chain extra (TD-03) |
| SUPPLY CHAIN | GREEN | every workspace Cargo.lock and the committed npm lockfiles are audited in CI (`cargo audit` x3, `npm audit`); third-party actions SHA-pinned (rust-toolchain tracked, TD-17); CycloneDX SBOM + release manifest with a dependency-lock hash attached to releases; signed SLSA build provenance at publish. Cryptographic tag/asset signing is tracked (TD-03) |
| BACKWARD COMPAT | GREEN | `tests/compat.rs` pins the released 0.2.0 surface *and* the 0.2.1 additions behaviorally with documented intentional breaks; `crossrelease.test.mjs` runs every frozen runtime against its language line's fixtures (released behavior unchanged); compatibility policy in RELEASE_ENGINEERING.md |
| RELEASE-READY (v1 GO) | RED | not yet met — v1 GO requires a *published, post-release-verified* 1.0 (section 110/111). Every readiness capability category is GREEN except WASM (YELLOW; open B-1, item 3), and all semantic/version decisions (HD-1…HD-4) are closed, implemented, pushed, and CI-green. Release `v0.2.1` is published (tag `v0.2.1` = commit `3f5f870`), so the release train itself is proven end to end; the remaining path is the gated prerelease→RC→1.0 train plus closing B-1, not a capability gap in the language or tooling |

## Current blockers toward v1 GO

1. Exactly one readiness capability category is YELLOW: WASM, because B-1
   remains open (item 3 below). It is an implementation nonconformance in the
   released WebAssembly runtime, not a spec conflict and not a `v0.2.1`
   regression. Beyond closing B-1, the remaining path to v1 GO is executing the
   gated prerelease→RC→1.0 train (section 47 onward) with published-artifact
   verification. This does not make the release train itself blocked: the
   freeze/release gate is separately RED only because no post-release-verified
   1.0 exists yet (see the RELEASE-READY row above).
2. All semantic-freeze decisions are resolved (no open HD items); their
   implementations are committed, pushed, released in `v0.2.1`, and CI-green.
   The `v0.2.1` release train is closed (published 2026-10-01; 27/27 check-runs
   green; GitHub Pages deploy green).
3. **WASM (B-1 — fixed in source, proved on a fresh machine-backed build;
   published artifacts superseded only by a future publication):** mainstream
   recursive programs reach the 512-frame language limit and report a
   structured `E4011` on the fresh build; the explicit-continuation machine is
   the production engine on every entry point. The released WASM artifacts
   (`0.0.2`, `0.2.0`, `0.2.1`) still contain the defect because frozen
   artifacts are immutable; publishing a replacement runtime is a separate
   human-gated release decision. Not a `v0.2.1` regression and not a
   release-immutability issue.

## Notes

- The former correctness blocker **B-1** (WASM call-frame implementation
  nonconformance) is **fixed in source and remote-closed**, proved by the
  fresh machine-backed WASM boundary suite; the released frozen artifacts
  remain affected until a future human-gated runtime publication
  (`docs/WASM_CALL_FRAME_LIMIT_DECISION.md`; architecture
  `docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`).
  No unresolved CRITICAL/HIGH *security* blocker is known.
- Frozen `0.0.2`/`0.2.0`/`0.2.1` artifacts verified byte-identical at intake;
  the published `0.2.1` release asset matches the in-repository artifact.
