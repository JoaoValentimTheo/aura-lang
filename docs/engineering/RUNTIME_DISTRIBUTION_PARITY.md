# Runtime Distribution Parity — Directive 007

**Verdict: `PASS`.** The distributed WebAssembly runtime now contains the
corrections that were already in the source tree, the parity claim is measured
against the bytes that actually ship, and a stale distribution can no longer
pass unnoticed.

**Identity:** `0.3.2-dev.9` — a new immutable preview. `0.3.1` is still the
release and `manifest.current` is still `0.3.1`. No stable tag was created, no
stable version was published, and no frozen artifact was overwritten.

| Item | Value |
|---|---|
| Branch | `rewrite/v3-rust` |
| Base commit (before this work) | `85b42fa58e2c8220ddca2d3722d3f9372196c8df` |
| New runtime identity | `0.3.2-dev.9` |
| Artifact | `playground/runtimes/0.3.2-dev.9/aura_playground_runtime.wasm` |
| SHA-256 | `8de654f0acc894595975ac1ec89c9e239a4ef89904ed3a9dd65a8af2589112b5` |
| Bytes | 2 010 781 |
| Host ABI | 2 |
| Language version | `0.3.1` (unchanged — no language-contract change) |
| Channel | `development` (preview; **not** promoted to `current`) |
| Source fingerprint | `0e252f8f40a6c04592864fa9755e4d0c3983ed5ab2a7b4eac4163ee48df5652b` |

## 1. Why a new identity instead of a rebuild

`playground/build.mjs` refuses to overwrite an artifact that already exists, and
that refusal was exercised before anything was changed: a bare
`node playground/build.mjs` on an unmodified tree fails with the immutability
guard. Replacing `0.3.1` or `0.3.2-dev.8` in place would have silently
substituted bytes that users and caches had already seen.

Freeze point: **`31346b53`** is the last commit that touched
`playground/runtime/` and the only commit that created
`playground/runtimes/0.3.2-dev.8/`. **Sixteen `src/` commits** landed after it
without a corresponding artifact:

| Commit | Correction |
|---|---|
| `404980e5` | REPL session state incremental instead of rebuilt per submission |
| `3a42deee` | Host session APIs gated on the `repl` feature |
| `2d7f6356` | Type-parameter frame popped on the error path |
| `dc9d1f39` | `Set` element types enforced on `add`/`has`/`contains`/`remove` |
| `9a31f9b0` | Implicit-return body validated against declared return type |
| `e616765c` | `pub use` re-export chains followed on the import path |
| `67f0c812` | Transitive `pub use` resolved regardless of source order |
| `ce70c908` | Method type parameters validated and scoped |
| `6425f360` | Method type-parameter bounds validated |
| `b5f0527d` | f-string exponent case and non-finite rendering |
| `914e1a5a` | `sort` as a total order; incomparable lists rejected |
| `731218d7` | f-string zero padding placed after the sign |
| `df21f334` | Method call on a type parameter resolved by its bounds |
| `125d5b0d` | JSON decode symmetric with encode; `E4031` for malformed input |
| `21f1b539` | HTTP header values encoded ISO-8859-1 in both directions |
| `77cb34d6` | Every statically empty `range` treated as empty |

Note the manifest convention: `31346b53` itself **replaced** the `0.3.2-dev.7`
entry with `0.3.2-dev.8` while leaving the `dev.7` directory on disk. This work
follows that established convention — the manifest lists the current preview on
a line plus the pinned releases, and superseded preview directories remain as
historical bytes. `0.3.2-dev.8/` is untouched and still on disk and still
loadable by URL; it is simply no longer the line's advertised preview.

## 2. Build reproducibility

Built by the canonical procedure, `node playground/build.mjs`, from pinned
dependencies and the pinned toolchain.

- `rust-toolchain.toml` → `channel = "1.98.1"` (rustfmt, clippy, minimal profile)
- Observed `rustc 1.98.1 (48a229cea 2026-09-01)`, `node v24.22.0`,
  target `wasm32-unknown-unknown`
- No new dependency was added for this work.

**Determinism was verified by a genuine clean rebuild**, not by a partial clean:
`rm -rf playground/runtime/target/wasm32-unknown-unknown` together with
`rm -rf playground/runtimes/0.3.2-dev.9`, then a full dependency recompile. The
result was **byte-identical** (`cmp` clean, same SHA-256).

That claim is **host-scoped**. It shows the build is deterministic on this
machine and this toolchain; it is *not* a cross-host reproducibility proof, and
the report does not claim one. `cargo clean -p aura-playground-runtime` is
**not** a valid clean-rebuild proof here — there is no root `[workspace]`, so it
removes zero files.

## 3. Build-input provenance

New module: `playground/source-fingerprint.mjs`.

It hashes **34 inputs** — the format tag `aura-runtime-build-inputs/v1`, both
`Cargo.toml`/`Cargo.lock` pairs, `rust-toolchain.toml`, and every `.rs` file
under `src/` (27) and `playground/runtime/src/` (2):

```
Cargo.toml                              src/ast/mod.rs
Cargo.lock                              src/bridge/mod.rs
rust-toolchain.toml                     src/check/mod.rs
playground/runtime/Cargo.toml           src/diagnostic.rs
playground/runtime/Cargo.lock           src/error.rs
playground/runtime/src/lib.rs           src/host.rs
playground/runtime/src/bin/            src/lex/mod.rs
  aura-playground-native.rs             src/lex/token.rs
src/ais.rs                              src/lib.rs
src/lex/mod.rs …                        src/main.rs
                                        src/mcp.rs
                                        src/module_graph.rs
                                        src/native_source.rs
                                        src/parse/mod.rs
                                        src/repl.rs
                                        src/resolve.rs
                                        src/run/{iterative,mod,session,value}.rs
                                        src/source.rs
                                        src/stdlib/{ext,mod,signatures}.rs
                                        src/types.rs
                                        src/version.rs
```

The record stream is `${relpath}\0${sha256(file)}\n` per input, repo-relative
POSIX paths, sorted; the digest is `sha256(FORMAT_TAG + "\n" + stream)`. The
format tag is hashed first so that a future change to the input set produces a
different digest instead of a silently ambiguous one.

**Deliberately excluded:** the artifact's own hash, `manifest.json`, and any
commit SHA. Including any of them would create the self-referential and
circular dependency that §5 forbids — the fingerprint describes the *inputs*,
so it can be compared against a tree that has not been committed yet.

## 4. The 18 documented regressions

The mandatory criterion. Cases are taken verbatim from `tests/regressions.rs`
and were run against three substrates: the **committed artifact bytes**, the
**native binary**, and — as baselines — the two previously distributed
artifacts.

| Substrate | PASS | WRONG | TRAP |
|---|---|---|---|
| `0.3.1` (release) | 4/18 | 11 | 3 |
| `0.3.2-dev.8` (previous preview) | 4/18 | 11 | 3 |
| **`0.3.2-dev.9` (new preview)** | **18/18** | **0** | **0** |
| Native `HEAD` | 18/18 | 0 | 0 |

`0.3.2-dev.8` fixed **nothing** relative to `0.3.1` despite carrying ABI 2,
which is the core of the finding: the preview line had been advancing its own
version number while its behaviour stayed frozen sixteen `src/` commits in the
past.

### Divergences corrected in `dev.9`

| # | Behaviour | `0.3.1` / `dev.8` | `dev.9` and native |
|---|---|---|---|
| D1 | `sort` over a list containing incomparable elements | wrong order or trap | rejected, never a self-inconsistent order |
| D2 | `sort` on an empty / single-element list | trap in 3 cases | correct |
| D3 | f-string exponent case (`2.5e10`) | lowercase `e` | `E` as native renders |
| D4 | f-string non-finite (`inf`, `nan`) | trap / wrong text | `inf`, `nan` |
| D5 | f-string zero padding with a sign (`-007`) | padding before sign | sign then padding |
| D6 | f-string explicit precision (`:.2e`) | ignored | honoured → `1.50e10` |
| D7 | `json_decode` on malformed input | wrong diagnostic or silent value | `E4031` |
| D8 | `json_decode` round-trip symmetry | asymmetric with encode | symmetric |

## 5. Supplementary behavioural cases (20/20)

Beyond the eighteen, twenty further cases were proven on the same substrates:

| Substrate | Result |
|---|---|
| `0.3.2-dev.9` | **20/20** |
| Native `HEAD` | **20/20** |
| `dev.9` vs native agreement | **20/20** |
| `0.3.2-dev.8` | 12/20 |

Coverage: `Set.add`/`has`/`contains`/`remove` with an incompatible element type
→ `E3001` (both via an annotated `{int}` set and an inferred one); implicit
return contradicting the declared return type → `E3005`; `json_decode` on
`"{not json"` and `"1 2"` → `E4031`, on a non-string → `E3001`, and a
`json_encode`/`json_decode` round-trip; `range(5,5)`, `5..5`, `range(9,2)`,
`range(-1,-1)` → empty, `range(0,3)` → 3 elements; a method with a bounded type
parameter; f-string exponent precision and non-finite rendering.

## 6. The distributed-bytes suite

`playground/tests/node/distribution-parity.test.mjs` — 39 cases (the 18
corrections plus the 20 supplementary plus an identity assertion).

It exists because every other differential suite compiles or loads a freshly
built `target/` binary. Nothing measured **the file that gets distributed**,
which is precisely how two stale artifacts stayed green through dozens of PRs.
`run-all.mjs` even carried the comment "the fresh wasm is the same source and is
covered by the differential/syntax gates above" — an assumption about the
artifact, not a check of it. That comment now states the assumption honestly and
credits this suite with keeping it true.

The suite loads `playground/runtimes/<current>/aura_playground_runtime.wasm`,
asserts `runtime.runtimeVersion === versionId`, and only then runs the cases, so
a correct artifact named at the wrong version fails instead of passing quietly.

| Target | Result | Exit |
|---|---|---|
| `0.3.2-dev.9` | **39/39** | 0 |
| `0.3.2-dev.8` | 17/39 | 1 |
| `0.3.1` | 17/39 | 1 |
| `dev.9` bytes, wrong version id | — | 1 |

The `dev.8`/`0.3.1` failures are visible and specific, e.g.
`expected E4031, got E3001` on the JSON cases.

## 7. CI protection

Four independent guards, each at the layer where it belongs.

**7.1 Dual-mode `--check` in `playground/build.mjs`.**

- `node playground/build.mjs --check` — integrity of every entry, plus the
  requirement that the **current** identity records a `source_fingerprint`.
  It does **not** compare the fingerprint with the tree.
- `node playground/build.mjs --check --require-source-match` — additionally
  requires the recorded fingerprint to equal the tree's, **and** requires that
  an available manifest entry for the current crate version exists at all.

The split is the whole design. Provenance is a property of a *distribution*, not
of an ordinary edit: a PR that touches `src/` legitimately makes the tree differ
from the last built preview, and §5 forbids failing every such PR because the
next preview cannot exist yet. Presence of the fingerprint is the edit-time
invariant; equality with the tree belongs to the moment a distribution is
approved.

The `sawRuntimeIdentity` guard closes a silent-pass hole found in this work: the
provenance loop only inspects entries that exist, so a manifest that **omitted**
the current identity satisfied `--require-source-match` vacuously. The flag now
fails if no entry for the current runtime version is present.

Mutation-verified in both directions:

| Mutation | `--check` | `--check --require-source-match` |
|---|---|---|
| (none) | exit 0 | exit 0 |
| `src/version.rs` edited | **exit 0** (PR-friendly, as required) | **exit 1**, names the identity and both digests |
| current identity removed from manifest | exit 0 | **exit 1**, "no available manifest entry for the current runtime version" |

**7.2 Tag-build provenance in `release.yml`.** `--require-source-match` runs on
**tag builds only** (`if: startsWith(github.ref, 'refs/tags/v')`), placed before
the version/metadata consistency step. `ci.yml`'s playground job and
`pages.yml` keep plain `--check`; requiring equality there would fail ordinary
PRs for a non-defect. `release.yml` had no Node usage before, so
`actions/setup-node` was added for this one step, using the repository's pinned
action convention.

**7.3 Manifest provenance assertions in `manifest.test.mjs`.** Any
`source_fingerprint` must be a 64-character lowercase hex string; the identity
matching `playground/runtime/Cargo.toml` must be published and must record one;
the frozen `0.3.1` entry must carry **none**; and the fingerprint computation is
asserted idempotent. Directionally verified: stripping the fingerprint from
`dev.9` fails by name, and retro-fitting one onto `0.3.1` fails by name.

**7.4 Identity derivation instead of literal pins.** Five suites
(`session-http`, `project-http`, `browser`, `browser-http`, `http-security`)
hardcoded `0.3.2-dev.8` in artifact paths, labels, and `#version` selections.
They now derive the development identity from
`playground/runtime/Cargo.toml`. Bumping the crate version without building
makes them fail loudly with `ENOENT` on the missing artifact path — which is the
correct behaviour and was mutation-verified by temporarily setting the version
to `0.3.2-dev.99`.

Consequently `distribution-parity` is wired into `run-all.mjs`, which the
`playground` CI job already invokes, so the gate runs wherever the suites run.

**What this does not do.** It does not continuously re-audit *frozen* artifacts
against `HEAD`. A frozen runtime is supposed to preserve the semantics of its
own era, so re-comparing `0.3.1` with today's tree and failing would be wrong.
Equivalence is checked when an identity is built or approved.

## 8. Validation executed

Every command below was actually run. Nothing listed as passing was skipped.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo test --locked --all-targets --all-features` | exit 0 |
| `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` | exit 0 |
| `cargo +1.83.0 check --locked --all-features` (MSRV) | exit 0 |
| `cargo +nightly check --manifest-path fuzz/Cargo.toml --all-targets` | exit 0 |
| `cargo +nightly fuzz run parser -- -max_total_time=45` | 429 338 runs, no crash |
| `cargo +nightly fuzz run checker -- -max_total_time=45` | 256 777 runs, no crash |
| `cargo +nightly fuzz run runtime -- -max_total_time=45` | 729 runs, no crash |
| `node playground/build.mjs --check` | exit 0, 9 versions |
| `node playground/build.mjs --check --require-source-match` | exit 0 |
| `node playground/tests/node/run-all.mjs` | exit 0, all suites green |
| `node playground/tests/node/manifest.test.mjs` | 94 passed, 0 failed |
| `node playground/tests/node/distribution-parity.test.mjs` (dev.9) | 39/39, exit 0 |
| `node website/build.mjs` | 46 pages |
| `node website/tests/run-all.mjs` | exit 0 |

Playground suite detail (all from `run-all.mjs`): ABI 80, integrity 39,
transport-bounds 26, differential 234 (generated parity 60/60 freshly generated
+ 19/19 committed; type-expression sweep 17 depths, unified ceiling 256 wraps,
0 host failures), syntax conformance 77, **distribution parity 39/39**,
B-1 boundary 59, session-http 17, project-http 23, browser 107, worker 12,
multi-file 42, cache 7, browser-http 8, http-security 26.

Artifact integrity re-verified over **all nine** manifest entries after the
build: every available artifact matches its recorded SHA-256 and byte length;
`0.0.1` remains correctly unavailable with a null hash. `0.3.1`
(`96e778ae…`, 1 951 520 B), `0.3.2-dev.8` (`81d07339…`, 1 998 770 B) and every
older artifact are byte-identical to their committed state.

### One flake, classified as pre-existing

`tests/bench.rs::module_graph_scales_subquadratically` failed once with
`doubling N scaled 3.47x, expected sub-quadratic` against a `3.2` ceiling.

It is **not** a regression from this work, which touches no Rust file
(`git diff --stat -- src/ tests/ Cargo.toml Cargo.lock` is empty). Classification:

- passes 3/3 in isolation and 5/5 as a whole file on the working tree;
- passed the full suite on the base commit;
- **the base commit `85b42fa5` itself fails the same guard on repeated
  full-suite runs** — 2 of 3 repetitions failed, with `lex(long_string)` at
  `4.22x` and `3.24x` and no worktree difference at all;
- the guard is a wall-clock ratio on an 8-core machine that was carrying
  substantial background load (`3.34 4.19 3.85`).

So the guard is load-sensitive and pre-existing. The threshold was **not**
loosened: weakening a timing guard to obtain green is explicitly out of bounds.

## 9. Residual risks

| Risk | Severity | Status |
|---|---|---|
| Determinism proven on one host/toolchain only; no cross-host bit-for-bit proof | P3 | Known gap, honestly scoped. A future provenance/SBOM program would address it |
| `tests/bench.rs` ratio guards are load-sensitive and can flake under contention | P3 | Pre-existing, reproduced on the base commit. Not weakened |
| The distributed artifact still executes untrusted code in the Playground | pre-existing | Unchanged by this directive and not claimed otherwise |
| The fingerprint covers Rust sources and manifests, not the Node toolchain or `node_modules` free build | P3 | `wasm32` build is Cargo-only, so this is currently inert; it would matter if the build gained a JS step |
| `dev.8` remains on disk and loadable though no longer the advertised preview | P3 | Historical bytes preserved by design; not a defect |
| CPython is not a sandbox | pre-existing | Reiterated in `RELEASE_CANDIDATE_READINESS.md`; nothing here implies isolation |

No P0 or P1 remains open in the scope of this directive.

## 10. Scope discipline

**Not done, deliberately:** no stable tag, no `0.3.2` publication,
`manifest.current` untouched at `0.3.1`, no frozen artifact overwritten, no
language-semantics change, no LSP/formatter/package-manager work, Campaign 003
not reopened, and no exploratory bug hunt. `PROMOTE_DEV_TO_DEFAULT` remains
`false`, so `dev.9` is reachable through the normal Playground flow without
becoming the default runtime.

The publication checklist is **not executed** and remains a separate human
authorization, as `RELEASE_CANDIDATE_READINESS.md` set out.

## 11. Verdict

`PASS` for Directive 007: the new runtime is versioned and intact, the 18
regressions pass on the distributable bytes with zero wrong results and zero
traps, the known critical divergences are corrected in the new preview, the
protection against a semantically stale distribution is automated, historical
artifacts are intact, and the required CI is green.

This authorizes none of the following, each of which needs its own human
decision: creating a `v0.3.2` tag, publishing a stable release, promoting
`dev.9` to `manifest.current`, or declaring `0.3.2` generally available.
