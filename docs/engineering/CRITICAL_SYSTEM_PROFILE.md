# Critical-System Profile Requirements (Pre-0.3 Foundation)

Status: requirements inventory. **Aura is not certified for voting systems,
avionics, medical devices, nuclear systems, automotive safety, or any other
safety-critical environment, and this document does not claim otherwise.**
It records what a future restricted/deterministic profile would require, so
current architecture does not block that work. Certification itself is outside
this transaction.

---

## 1. What already exists

| Requirement | Status today | Evidence |
|---|---|---|
| Memory safety | `unsafe_code = "deny"`; zero `unsafe` in `src/**`; Miri is a blocking CI gate | `Cargo.toml` lints; CI `miri` job |
| Panic-free on user input | `unwrap_used`/`expect_used`/`panic` denied; fuzz + property suites; no panic from crafted input found | `Cargo.toml`; `fuzz/**`; `tests/adversarial.rs` |
| Deterministic execution | no randomness, threads, network, or wall-clock language surface except the host clock/sleep; host is injectable (`silent_host`, `LimitedHost`) | `src/host.rs`; `docs/engineering/CAPABILITY_MODEL.md` |
| Bounded resources | AST depth (256), call frames (512), range materialization (10M), format width/precision, node/type budgets, value render depth/nodes, stdin/stdout/args/source bounds at the WASM boundary | `src/parse`, `src/run`, `playground/runtime/src/lib.rs`; see the resource inventory in `docs/engineering/PERFORMANCE.md` and §4 below |
| Integer overflow policy | checked arithmetic; `E4013`; no wrapping | `src/run/mod.rs` arithmetic helpers |
| Reproducible builds | same-host bit-identical measured; cross-host pending (TD-04) | `docs/engineering/RELEASE_ENGINEERING.md` |
| Dependency pinning | committed lockfiles, `--locked`, SHA-pinned Actions, `cargo audit` CI | `.github/workflows/ci.yml`; TD-01 |
| Supply-chain provenance | SBOM (`scripts/sbom.sh`), lockfile hash in the release manifest, signed SLSA provenance at publish | TD-03 |
| Fuzzing | lexer/parser/checker/runtime targets, nightly smoke, deep campaign recorded | `fuzz/**`; `docs/security/CAMPAIGN_LOG.md` |
| Property testing | proptest suites + registry-drift properties | `tests/property*.rs` |
| Structured diagnostics | stable codes; source attribution | `src/error.rs`; `docs/errors.md` |
| Capability restriction | `Host` is the only authority; a build can supply a minimal host | `docs/engineering/CAPABILITY_MODEL.md` |

## 2. What a future restricted/deterministic profile would add

- **Deterministic JSON/rendering mode** (already deterministic; would need a
  frozen contract statement per release line).
- **Resource accounting**: a profile-level budget for total steps, allocations,
  and output bytes with a hard stop (today's bounds are per-operation).
- **Audit log**: a structured record of capability uses (which host calls, how
  many bytes) suitable for post-incident review.
- **Static analysis**: an accepted-subset checker or a documented coding
  standard for regulated use; not present today.
- **Formal-methods candidates**: the value model, the exception model, and the
  resource limits are the natural first targets for machine-checked proofs.
- **Cross-toolchain reproducible builds** (TD-04) with attested provenance.
- **Panic-free runtime guarantee**: today "no panic from user input" is
  evidence-based; a profile would need an enforced no-panic build (e.g.
  `panic = "abort"` plus a documented abort policy) and proof coverage.
- **Versioned profile contract**: a profile identifier that pins language,
  runtime, resource policy, and capability set together.

## 3. Architectural constraints that must not break

1. No hidden global authority (see `CAPABILITY_MODEL.md`) — a profile must be
   able to enumerate every capability a program can reach.
2. Resource limits must remain named, documented, diagnostically reported, and
   substrate-consistent (see `docs/engineering/PERFORMANCE.md`).
3. No catchable resource/host enforcement (see
   `docs/engineering/EXCEPTION_ARCHITECTURE.md` §2, §7).
4. WASM zero-import contract stays: the module cannot request host authority.
5. Native and WASM must not diverge in accept/reject behavior without an
   explicit, documented policy difference.

## 4. Resource-limit inventory (as of this checkpoint)

| Limit | Value | Diagnostic | Substrate |
|---|---|---|---|
| AST/type/module nesting (`MAX_AST_DEPTH`) | 256 | `E1015` | all |
| Parser recursion budget | 2048 native / 768 wasm | `E1015` | calibrated |
| Parser grouping backstop (pure parentheses) | 2048 native / 192 wasm | `E1015` | calibrated below the measured engine ceiling |
| Call frames (`MAX_CALL_FRAMES`) | 512 | `E4011` | all |
| Range materialization | 10,000,000 | `E4013` | all |
| Format precision | u16::MAX | `E4013` | all |
| Format width | 10,000,000 | `E4013` | all |
| Value render depth (`MAX_VALUE_DEPTH`) | 512 | elision / `E5002` (bridge) | all |
| Value render nodes (`MAX_VALUE_NODES`) | 1,000,000 | elision | all |
| Type nodes (`MAX_TYPE_NODES`) | 100,000 | `E1015` | all |
| Python conversion nodes | 1,000,000 each way | `E5002` | py only |
| Sleep clamp (`MAX_SLEEP_MS`) | 60,000 ms | clamp (no error) | native host only |
| WASM source / stdin / stdout | 256 KiB / 1 MiB / 1 MiB | `E4020` | wasm boundary |
| WASM args / arg bytes | 256 / 16 KiB | `E4020` | wasm boundary |
| WASM project bytes / sources | 2 MiB / 4096 | `E4020` | wasm boundary |

Each limit has a rationale in its declaration doc-comment; the wasm-boundary
limits are host application policy (documented in `docs/playground.md` §2).

## 5. Explicit non-claims

- Not certified, not certifiable today, and not claiming any safety level.
- No claim that WASM restrictions apply to native.
- No claim that reverse-engineering resistance or tamper resistance exists.
