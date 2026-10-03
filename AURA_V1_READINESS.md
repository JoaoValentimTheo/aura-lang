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
| WASM | YELLOW | 0 imports; Host ABI 1; differential 228/228; module/alias depth unified with native. **Open B-1 (implementation nonconformance):** mainstream recursive shapes exhaust the JavaScript engine stack below the 512-frame language limit and trap instead of `E4011` — `LANGUAGE_SPEC` §31.3 is a language rule and §31.5 forbids host failures; the two reinforce each other (corrected classification). Fresh-instance first-trap depths: else 387 (Node cold) / 196 (production Worker), match 459/233, closure 356, module 387, if-chain 419; thin shapes reach the boundary except in the Worker (360); the Chromium main thread reaches it only after warm-up. Binding resource is the engine stack, not the 4 MiB guest stack (16 MiB control identical; `--stack-size=4000` restores the limit). Repeated traps degrade a reused instance (deeper execution later fails `memory access out of bounds`). Every released WASM artifact is affected (`0.0.2` 317, `0.2.0` 389, `0.2.1` 387); native conforms. Remediation blocked: the contract-preserving fix is an engine-independent evaluator (runtime-architecture program), and a substrate-calibrated cap would be a semantics change. `docs/WASM_CALL_FRAME_LIMIT_DECISION.md` (rewritten, OPEN, Options A/B/C) |
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
3. **WASM (B-1, YELLOW — implementation nonconformance; remediation
   architecture designed, implementation not started):** mainstream recursive
   programs exhaust the JavaScript engine stack below the 512-frame language
   limit instead of reporting `E4011`
   (§31.3 is a language rule; §31.5 forbids host failures — they reinforce
   each other, so this is an implementation defect, not a spec conflict).
   Present in every released WASM artifact and in HEAD; native conforms. The
   contract-preserving remedy is an engine-stack-independent evaluator (an
   explicit frame/continuation stack) — a runtime-architecture program; any
   substrate-calibrated cap would change released semantics and is not
   authorized automatically. `docs/WASM_CALL_FRAME_LIMIT_DECISION.md`
   (OPEN — Options A/B/C, recommendation B); the implementation contract for
   Option B is `docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md` (design-only
   pass 2026-10-02; no behavior change). Not a
   `v0.2.1` regression and not a release-immutability issue.

## Notes

- One unresolved correctness blocker is known: **B-1** (WASM call-frame
  implementation nonconformance: the engine stack traps below the language
  limit; `docs/WASM_CALL_FRAME_LIMIT_DECISION.md`, OPEN; architecture
  designed in `docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`, implementation
  not started).
  No unresolved CRITICAL/HIGH *security* blocker is known.
- Frozen `0.0.2`/`0.2.0`/`0.2.1` artifacts verified byte-identical at intake;
  the published `0.2.1` release asset matches the in-repository artifact.
