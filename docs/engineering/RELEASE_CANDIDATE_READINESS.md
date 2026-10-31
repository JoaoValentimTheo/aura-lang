# Release Candidate Readiness — Directive 006

Directive: **AURA — DIRETIVA 006, Release Readiness (Preparação Final sem
Publicação)**.

Status: **`INCOMPLETE`** — the assessment is complete within its 6-hour budget,
but the recommendation is **`NO-GO` for distributing the current artifacts**.
This is a statement about the *artifacts that exist*, not a claim that the
source tree is unsound.

> This report does **not** authorize a tag, a release, a package publication, or
> any modification to a frozen artifact. Every publication step is listed as
> **not executed** in §11.

---

## 0. Recommendation summary

| Question | Answer |
|---|---|
| Prepare a release candidate? | **Yes** — but only after the artifacts are rebuilt from current source. |
| Build a release candidate? | **Requires human authorization** — new identity needed (§9). |
| Test a release candidate? | Prepared; the local matrix in §7 is green on `3347ff6c`. |
| Distribute the **existing** artifacts? | **`NO-GO`** — three reproducible defect classes live in them (§4). |
| Promote any version to stable? | **`NO-GO`** — out of scope for this directive and premature. |

The one material discovery of this directive is that **the Playground's shipped
WebAssembly runtimes are stale relative to the source tree, and that staleness is
not merely cosmetic**. Both selectable runtimes accept programs the current
checker rejects, and both compute wrong results for a range of operations that
were fixed after they were frozen. Details in §4.

This is a **documentation-only** finding produced within Directive 006's
authority: no production code, no frozen artifact, and no test was changed.

---

## 1. Reference identity

| Item | Value |
|---|---|
| Branch | `rewrite/v3-rust` |
| Reference commit | `3347ff6c6a97c96508966b45a6e1c401941c1fc0` |
| Subject | `docs: correct the closure report's own identity claims` |
| Local / remote | identical (`git rev-parse HEAD` == `git rev-parse origin/rewrite/v3-rust`) |
| Worktree | clean (`git status --short` empty) at assessment start |
| Crate version (`Cargo.toml`) | `0.3.1` |
| Language version (`aura::LANGUAGE_VERSION`) | `0.3.1` |
| Playground runtime crate (`playground/runtime/Cargo.toml`) | `0.3.2-dev.8` |
| Playground `manifest.current` | `0.3.1` |
| Published tags | `v0.0.1`, `v0.0.2`, `v0.2.0`, `v0.2.1`, `v0.3.1` (unchanged) |
| Merged PRs | **38** (`#2`–`#39`); open: 0; closed-unmerged: 0 |
| Assessment timestamp | `2026-10-31T18:18:03Z` |

`PR #1` does not exist (GitHub: *"Could not resolve to a PullRequest with the
number 1"*).

### Remote verification

Confirmed against the GitHub API, not only local Git:

- run `41225313680` — `CI` on `3347ff6c` — **success**, created `2026-10-31T17:35:58Z`.
- run `41225313675` — `Deploy website (GitHub Pages)` on `3347ff6c` — **success**, created `2026-10-31T17:35:58Z`.
- **23 of 23** check runs on `3347ff6c` concluded `success`. The `ci.yml`
  workflow declares **15 jobs**, which expand to **21 check runs** through
  matrices (`test` ×3, `cpython-interop` ×5, plus the singles);
  `pages.yml` contributes **2** more (`build static site`,
  `deploy to GitHub Pages`). 21 + 2 = 23.
- PR #39 `MERGED` as `86e2b5bad9f4ee92085d162d3a613242dcfb0296` at
  `2026-10-31T17:11:56Z`.

---

## 2. Corrected PR inventory (Directive 006 §2)

Directive 006 §2 requires that PRs `#2`–`#38` **not** be uniformly classified as
corrective. The previous report's inventory was wrong on this point.

**Previously stated** in [`CAMPAIGN_003_CLOSURE.md`](CAMPAIGN_003_CLOSURE.md):

> Of which corrective | **37** (`#2`–`#38`)

**Corrected.** Classification below is derived from each PR's *actual* changed
file set as returned by the GitHub API, not from its title. Where a PR touches
several surfaces, it is assigned to the surface that dominates its diff.

| Category | Count | PRs |
|---|---|---|
| Core runtime (`src/`) | **16** | `#4`, `#8`, `#21`, `#22`, `#24`, `#26`, `#28`, `#30`, `#31`, `#32`, `#33`, `#34`, `#35`, `#36`, `#37`, `#38` |
| Playground runtime / WASM artifact | **2** | `#9`, `#19` |
| Tests only | **5** | `#2`, `#7`, `#12`, `#14`, `#16` |
| CI / workflows | **3** | `#5`, `#13`, `#17` |
| Dependencies / security | **1** | `#3` |
| Agent / contributor configuration | **2** | `#10`, `#20` |
| Documentation / state records | **9** | `#6`, `#11`, `#15`, `#18`, `#23`, `#25`, `#27`, `#29`, `#39` |
| **Total** | **38** | |

Sum verified: 16 + 2 + 5 + 3 + 1 + 2 + 9 = **38**, matching the 38 merged PRs
with no double counting and no omission.

The two Playground entries deserve a note, because they are the only PRs that
have ever produced a *shipped* artifact:

- **`#9`** (`feat(playground): let hosted sessions run multi-module programs`,
  B14) is the freeze point analysed in §4. It changed
  `playground/runtime/{Cargo.toml,Cargo.lock,src/lib.rs}`, **created**
  `playground/runtimes/0.3.2-dev.8/aura_playground_runtime.wasm` and updated
  `manifest.json`. It is a **feature**, not a corrective change.
- **`#19`** touched no `src/` file and no WASM binary. It changed
  `playground/web/app.js`, `playground/web/runtime.mjs`-adjacent web assets and
  HTTP security tests — i.e. the **Playground web front end**, which is
  distributed through `pages.yml` but is not part of the Aura runtime.

**Consequences of the correction:**

- Only **18 of 38** PRs changed any code that ships (16 core `src/` + 2
  Playground). The claim "37 corrective" is refuted; the correct figure is
  **18 code-bearing**, of which at least two are **features** (`#9` B14
  multi-module sessions, `#19` Playground web front end) rather than defect
  corrections.
- **9** PRs are documentation/state reconciliation only — a substantial share of
  the campaign was bookkeeping, which explains the volume without implying
  equivalent defect throughput.
- The `#2`–`#38` range includes `#2` (a release-preflight *test* and script) and
  `#3` (a dependency remediation), which are not "corrective" in the sense of
  fixing an Aura defect.

**Confirmed correct and left unchanged:** `CAMPAIGN_003_CLOSURE.md:221` states
"37 orphaned dev-runtime directories". Verified independently:
`playground/runtimes/` contains **45** directories, the manifest lists **9**
entries, and `0.0.1` has `available: false` with no directory. 45 − 8 = **37**.
This figure is accurate and must **not** be "corrected".

### Corrective PRs opened under Directive 005

**0 of 6** budget consumed; 1 documentation PR (`#39`). Directive 005 found no
P0/P1 within its delimited scope, so its correction procedure was never
triggered. That remains true *for the source tree*. §4 below explains why the
same statement does not hold for the distributed artifacts — a surface Directive
005's scope did not cover.

---

## 3. Changes since `v0.3.1`

`v0.3.1` was tagged `2026-10-08 21:27:12 -0300`. Since then: **136 commits**.

| Area | Commits |
|---|---|
| `website/` | 32 |
| `docs/` | 59 |
| `playground/` | 25 |
| `tests/` | 25 |
| `src/` | 22 |
| `.github/` | 9 |
| `scripts/` | 2 |

`src/` files touched: `ast/mod.rs`, `check/mod.rs`, `error.rs`, `host.rs`,
`lib.rs`, `parse/mod.rs`, `repl.rs`, `resolve.rs`, `run/iterative.rs`,
`run/mod.rs`, `run/session.rs`, `run/value.rs`, `stdlib/ext.rs`,
`stdlib/mod.rs`, `stdlib/signatures.rs`, `version.rs`.

### Behaviour-visible changes

These are the changes a user can observe, and they drive the SemVer reasoning in
§9.

| Change | Commit / PR | Observable effect |
|---|---|---|
| Set element types enforced | `dc9d1f39` / `#24` | `set.add` / `has` / `contains` / `remove` with a wrong element type now report `E3001`; previously accepted |
| Tail expression vs return type | `9a31f9b0` / `#26` | A body whose implicit value contradicts the declared return type now reports `E3005` |
| Empty `range` recognized | `77cb34d6` / `#38` | Eight spellings (`range(0,0)`, `range(5,2)`, `5..2`, `1..-1`, …) no longer produce a spurious `E3006` on an unreachable body |
| `json_decode` reports `E4031` | `125d5b0d` / `#36` | Malformed input now reports `E4031` (`DECODE_MISMATCH`); previously `E3001` |
| `sort` is a total order | `914e1a5a` / `#33` | NaN ordered last; lists across incomparable ordering classes rejected instead of silently passed through |
| f-string zero padding sign-aware | `731218d7` / `#34` | `-42` with `*>06d` renders `***-42`, not `000-42` |
| f-string exponent/non-finite | `b5f0527d` / `#32` | Exponent case and `inf` / `nan` rendering corrected |
| HTTP header values ISO-8859-1 | `21f1b539` / `#37` | Outgoing values no longer UTF-8 encoded; incoming non-ASCII values no longer erased |
| Method type parameters | `ce70c908` `#30`, `6425f360` `#31`, `df21f334` `#35` | Bounds validated and scoped; calls resolved by bounds |
| `pub use` re-export chains | `e616765c` `#28`, `67f0c812` | Transitive re-exports resolve regardless of source order |
| Type-parameter frame on error | `2d7f6356` `#22` | Frame popped on the error path, preventing leakage into later checks |

---

## 4. THE MATERIAL FINDING — the shipped WASM artifacts are stale

This section is the reason this report recommends `NO-GO` for distribution. It
was established empirically, by executing the shipped artifacts and comparing
their behaviour with current source.

### 4.1 Freeze point

| Fact | Evidence |
|---|---|
| Last commit touching `playground/runtime/` | `31346b53bee01a5987517f1eb5bff93baa2a999c`, `feat(playground): let hosted sessions run multi-module programs (B14)`, `2026-10-29 23:41:34 -0300` |
| Same commit added `playground/runtimes/0.3.2-dev.8/` | `git log --oneline -- playground/runtimes/0.3.2-dev.8/` returns that commit and **only** that commit |
| Commits touching `playground/runtime/` since | **none** |
| Commits touching `src/` since the freeze | **16** |
| Commits total since the freeze | **79** |
| Commits touching `tests/` since the freeze | **18** |

`playground/runtime/Cargo.toml` declares `version = "0.3.2-dev.8"` but path-depends
on the core crate at `../..`. Nothing re-versioned or rebuilt it after
`31346b53`. Therefore **the artifact named `0.3.2-dev.8` predates 16 core
`src/` commits**, including every checker fix listed in §3.

The CHANGELOG states that the fixes "ship only as numbered `0.3.2-dev.N` preview
runtimes". For the fixes in §3 that landed after `31346b53`, **that statement is
false**: they exist in source and in the native CLI, and in no distributed
artifact at all.

### 4.2 Differential proof — 18 verbatim regression cases

Eighteen reproducers were taken **verbatim** from the regression tests that
protect these fixes — `tests/regressions.rs`, `audit7_*` (lines 1685–1826, PR
`#33`) and `audit8_*` (lines 1814–1880, PR `#34`) — and executed on all three
substrates.

| Substrate | PASS | WRONG | TRAP |
|---|---|---|---|
| `0.3.1` (public default) | **4 / 18** | 11 | 3 |
| `0.3.2-dev.8` | **4 / 18** | 11 | 3 |
| HEAD build from `3347ff6c` | **18 / 18** | 0 | 0 |

A **TRAP** is a WebAssembly trap that aborts execution outright.

The run was repeated and produced byte-identical output, so the result is
reproducible, not flaky.

Representative failures — each is a *wrong answer silently returned*, not an
error:

| Case | Expected | `0.3.1` / `dev.8` produce |
|---|---|---|
| `sort` with NaN | NaN ordered last | `[1.0, 3.0, nan, 2.0]` — NaN unsorted |
| `sort` mixed ordering classes | rejection | `[1, 5, "a", 2, 3]` returned unchanged |
| `sort` mixed ordering classes | rejection | `[1, true, 2]` returned unchanged |
| `sort` mixed ordering classes | rejection | `[[1], [2]]` returned unchanged |
| `sort` mixed numerics | exact comparison | `[9007199254740993, 9007199254740992.0, 9007199254740992]` — misordered |
| zero padding with sign | `-00042` | `000-42` |
| zero padding with floats | `-003.50` | `000-3.50` |
| zero padding, infinities | `0000-inf` | `00000inf` / `0000-inf` (unsorted sign) |
| explicit fill suppresses zero flag | `***-42` | `000-42` |

Three cases **trap**. For example, `sort` over a 21-element list containing NaN
reaches `unreachable` in the shipped artifacts.

### 4.3 Worse: the shipped artifacts accept type-unsound programs

Because the checker fixes postdate the freeze, the artifacts do not merely
compute differently — they **accept programs that violate the type system**. This
is the Directive 005 §3 **P1** criterion "acceptance of an incorrect program
capable of violating relevant guarantees of the type system", reached through the
distribution surface rather than the source tree.

| Case | `0.3.1` | `0.3.2-dev.8` | HEAD wasm | Native HEAD |
|---|---|---|---|---|
| `set.add("x")` on a `{int}` set | **ACCEPT** | **ACCEPT** | `E3001` | `E3001` |
| `set.has("x")` on a `{int}` set | **ACCEPT** | **ACCEPT** | `E3001` | `E3001` |
| `set.remove("x")` on a `{int}` set | **ACCEPT** | **ACCEPT** | `E3001` | `E3001` |
| `fn f() -> int { "hello" }` | **ACCEPT** | **ACCEPT** | `E3005` | `E3005` |
| empty-range `never` body | `E3006` | `E3006` | `E3006` | `E3006` |
| `json_decode("{not json")` | `E3001` | `E3001` | `E4031` | `E4031` |

The fifth row is the control: all four substrates agree, confirming the probe
harness measures divergence rather than a systematic error. The sixth row is a
**diagnostic-code divergence**: any tooling keyed on `E3001` versus `E4031`
behaves differently depending on which substrate it runs against.

Native runs were performed in isolated directories, one `.aura` file each,
because Aura performs eager structural discovery of every eligible sibling in
the entry file's parent directory
([FILESYSTEM_MODULES_DESIGN.md](../FILESYSTEM_MODULES_DESIGN.md) §DECIDED,
[`src/native_source.rs`](../../src/native_source.rs)). This discovery is
intentional and frozen; it is not a defect.

### 4.4 Artifact integrity and determinism — separate questions, both answered

| Question | Answer |
|---|---|
| Are the manifest hashes correct for the bytes on disk? | **Yes.** All **8** available artifacts verified: SHA-256 and byte length match the manifest exactly, 0 mismatches. `0.0.1` is correctly `available: false` with no directory. |
| Is the HEAD build reproducible on this host? | **Yes**, for this host and toolchain only — four independent builds (including two after `cargo clean -p`) produced sha256 `5499a73384…`, **2 010 781 B**, identical every time. |

| Artifact | SHA-256 (prefix) | Bytes | ABI |
|---|---|---|---|
| `0.3.1` (public default) | `96e778ae50bd261f` | 1 951 520 | 1 |
| `0.3.2-dev.8` | `81d07339916f75f5` | 1 998 770 | 2 |
| HEAD build (unshipped) | `5499a73384052b63` | 2 010 781 | 2 |

The manifest-vs-source hash difference is therefore **source drift**, not
non-reproducibility. This distinction matters: the artifact can be rebuilt
deterministically; it simply was not rebuilt.

**Determinism caveat, stated precisely:** four identical local rebuilds prove
determinism on this host with this toolchain. They do **not** constitute a
cross-host reproducibility guarantee, and the repository makes no such claim
(see §6).

### 4.5 Why CI did not catch this

No gate compares the frozen artifact to current source. This is a **gap in
coverage**, not a broken gate — CI is behaving as designed:

1. The native/WASM parity gate deliberately builds a **fresh** wasm and compares
   it to native. `playground/tests/node/run-all.mjs:74-78` documents the
   rationale: comparing against a frozen artifact "would pin the previous
   release's semantics and report a language change as a substrate disagreement,
   which is exactly backwards." That reasoning is sound.
2. `playground/tests/node/run-all.mjs:131-138` nonetheless contains a **false
   premise** in a comment — that "the fresh wasm is the same source". It is not;
   the committed dev artifact is 16 `src/` commits stale.
3. `node playground/build.mjs --check` validates manifest hashes and sizes
   against **on-disk bytes only**. It never rebuilds, so staleness passes
   silently.
4. The HTTP suites pin the stale artifacts **by filename** —
   `session-http.test.mjs:55` (dev.8), `:178` (`0.3.1`), `project-http.test.mjs:61`
   (dev.8), `:75` (dev.7), `http-security.test.mjs:340` (dev.8). Their greenness
   says nothing about current source.

**A staleness gate is the missing control.** Because
`playground/build.mjs` enforces immutability — a differing hash at an existing
identity is a hard error — refreshing the artifact requires a **new identity**
(§9). Adding such a gate is production work and therefore outside Directive 006's
authority; it is recorded here as required follow-up.

---

## 5. Compatibility matrix

### 5.1 CLI / native runtime

| Aspect | Status |
|---|---|
| `0.3.1` → `3347ff6c` source | **Backward compatible for valid programs.** The changes either accept previously rejected valid programs (empty `range`, transitive `pub use`, method type parameters) or reject previously accepted invalid ones. |
| Diagnostic **codes** | **Breaking for tooling.** `json_decode` malformed input moves `E3001` → `E4031`. New `E3001` for Set element types and `E3005` for return-type tail mismatches. Any consumer switching on codes must be updated. |
| Diagnostic **coverage** | **Stricter.** Four classes of previously accepted unsound program are now rejected. Source that compiled on `0.3.1` may fail to compile at `3347ff6c` — correctly, but it is a compatibility event for users. |
| HTTP wire behaviour | **Changed, in the direction of correctness.** Headers are ISO-8859-1 in both directions. Programs relying on the previous UTF-8 encoding will observe different bytes on the wire. |
| CLI surface / flags | Unchanged. |
| `aura::LANGUAGE_VERSION` | `0.3.1`, unchanged. Invariant `LANGUAGE_VERSION <= VERSION` holds. |

### 5.2 CPython embedding

| Aspect | Status |
|---|---|
| Build | `cpython interop` green on Python 3.10, 3.11, 3.12, 3.13 (Ubuntu), 3.12 (macOS), 3.12 (Windows) — six check runs, all `success` on `3347ff6c`. |
| Pure-Rust link | `pure Rust binary links no CPython` — `success`. |
| Behaviour | Runs the same core as the CLI, so §5.1 applies identically. |
| **Isolation** | **None. CPython embedding is not a sandbox.** See §6. |

### 5.3 WASM / Playground

| Aspect | Status |
|---|---|
| Host ABI | ABI **1** → **2** in `dev.8`. Purely **additive**: the session exports (`aura_session_*`) extend the contract, and ABI-1 artifacts that omit them remain loadable because the JS loader feature-detects exports ([playground.md](../playground.md)). No ABI 1 break. |
| ABI-2 coverage | **Only `0.3.2-dev.8`**, and that artifact is stale (§4). No ABI-2 artifact exists that contains the current checker or `sort`/formatting fixes. |
| Public default runtime | `0.3.1`, ABI 1. `PROMOTE_DEV_TO_DEFAULT = false` in `playground/build.mjs` — deliberate. |
| Multi-module programs | Supported in `dev.8` and in source; not in `0.3.1`. |
| **Semantic parity, shipped vs source** | **BROKEN** — 14 of 18 probed cases wrong or trapping, 4 type-soundness acceptances, 1 diagnostic-code divergence (§4). |
| Semantic parity, native vs fresh wasm at HEAD | **Intact** — the parity gate passes, and the HEAD build matches native on all probed cases. |
| Playground suites | All green (§7), but four suites pin stale artifacts by filename, so their verdicts are scoped to those bytes. |

### 5.4 Summary of substrate agreement at `3347ff6c`

| Pair | Agreement |
|---|---|
| Native vs fresh wasm (HEAD) | **Agree** on every probed case |
| Native vs shipped `0.3.1` | **Disagree** (14/18 behaviour, 4 type-soundness, 1 code) |
| Native vs shipped `0.3.2-dev.8` | **Disagree** identically to `0.3.1` |

The decisive observation: **`0.3.2-dev.8` behaves like `0.3.1`, not like HEAD.**
It carries ABI 2 and multi-module support but none of the correctness fixes.

---

## 6. Security posture

### 6.1 CPython provides no isolation

Aura's CPython embedding is **not a sandbox** and provides **no capability
restriction**. This is already documented correctly and consistently, so **no
documentation correction is needed**:

- [TRUST_BOUNDARIES.md](../security/TRUST_BOUNDARIES.md):19 — "**Not a sandbox**";
  `:33` — "no sandbox and no capability restriction"; `:45` — authority:
  "**none beyond the wasm sandbox**".
- [THREAT_MODEL.md](../security/THREAT_MODEL.md):37 — "no Python sandbox";
  `:53` — TH-4 "Python sandbox escape — **N/A — no sandbox exists**".
- [SECURITY_ARCHITECTURE.md](../security/SECURITY_ARCHITECTURE.md):57 — does
  "**not** sandbox Python".
- [INSTALL.md](../INSTALL.md):63 — "not a sandbox".

**Implication for users, stated plainly:** running an untrusted Aura program
under the native CLI or CPython embedding grants it the full authority of the
embedding process — filesystem, network, and subprocess, subject only to OS
controls. Only the WebAssembly runtime imposes a sandbox, and only for programs
run through the Playground.

### 6.2 Residual risks

| Risk | Severity | Basis |
|---|---|---|
| Shipped artifacts accept type-unsound programs | **P1** | §4.3. A program the checker would reject executes and can produce arbitrary behaviour the type system was meant to exclude. |
| Shipped artifacts trap (`unreachable`) on valid programs | **P1** | §4.2. Three of 18 probed cases abort. Directive 005 §3 lists "panic or abort on valid Aura programs" as P1. |
| Silent wrong answers in shipped runtime | **P1** | §4.2. `sort` returning unordered data is a correctness failure with no diagnostic. |
| Resource limits under untrusted input | Medium | Execution limits are per-invocation; the Playground sandbox bounds memory, but the native CLI and CPython embedding do not impose Aura-level resource caps. |
| No staleness gate in CI | Medium | §4.5. The drift recurred once and can recur again silently. |

**These are risks of the *distributed artifacts*, not of the source tree.** The
source at `3347ff6c` is sound on every probed surface.

---

## 7. Test and CI evidence

All commands below were **actually executed** at `3347ff6c` during this
directive. Nothing is listed that was not run.

### 7.1 Local matrix

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | exit 0, `Finished dev profile` |
| `cargo test --locked --all-targets --all-features` | **1477 passed, 0 failed, 0 ignored** |
| `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` | **1361 passed, 0 failed** |
| `cargo +1.83.0 check --locked --all-features` | exit 0 (MSRV) |
| `cargo +nightly check --manifest-path fuzz/Cargo.toml --all-targets` | exit 0, `Finished dev profile` |
| `node playground/build.mjs --check` | `build --check: manifest matches 9 version(s)`, exit 0 |
| `node playground/tests/node/run-all.mjs` | all suites 0 failed — see §7.2 |
| `node website/build.mjs` | `Website built: 46 pages → website/dist/ (base /aura-lang/)`, exit 0 |
| `node website/tests/run-all.mjs` | all suites 0 failed, `Website suite complete.` |

### 7.2 Playground suite detail

`Differential: 234` · `Manifest: 88` · `ABI: 80` ·
`cross-release: 69 across 8 runtimes` · `semver: 52` · `Integrity: 39` ·
`project: 37` · `release-preflight: 32` · `http-security: 26` ·
`transport-bounds: 26` · `project-http: 23` · `session-http: 17` ·
`release-manifest: 12` · `sbom: 9` · `browser-http: 8` · `completion: 7` ·
`Cache: 7` · `Syntax conformance: 77 explicit native/WASM comparisons passed;
zero imports` · `Browser: 107` · `Multi-file: 42` · `Worker: 12`.

### 7.3 Remote CI at `3347ff6c`

Run `41225313680` (`CI`) — **21/21 jobs success**. `pages.yml` run
`41225313675` — **2/2 success**. Combined: **23/23**. The 21 CI job names,
verified from the Actions API:

`test (ubuntu-latest)` · `test (macos-latest)` · `test (windows-latest)` ·
`clippy (all features)` · `rustfmt` · `minimum supported Rust (1.83)` ·
`language contract` · `extended property tests` ·
`validation orchestrator self-tests` · `miri (unsafe-free guarantee)` ·
`fuzz smoke (lexer/parser/checker/runtime)` · `cargo audit` ·
`pure Rust binary links no CPython` · `playground (wasm runtime)` ·
`website (static build + examples + links)` ·
`cpython interop (3.10, ubuntu-latest)` · `cpython interop (3.11, ubuntu-latest)` ·
`cpython interop (3.12, ubuntu-latest)` · `cpython interop (3.12, macos-latest)` ·
`cpython interop (windows, py 3.12)` · `cpython interop (3.13, ubuntu-latest)`.

The two `pages.yml` jobs are `build static site` and `deploy to GitHub Pages`.

Note: `miri` and `fuzz smoke` **do** run in remote CI and are green there. What
was not run locally is `cargo fuzz run` against a specific target, and Miri was
verified through CI rather than repeated locally.

### 7.4 Not executed

| Item | Status |
|---|---|
| Miri locally | **NOT RUN locally**; green in remote CI (`miri (unsafe-free guarantee)`, `success` on `3347ff6c`) |
| `cargo fuzz run <target>` targeted execution | **NOT RUN** in this directive; only `cargo +nightly check` of the fuzz crate. Remote `fuzz smoke` green for 4 targets. |
| Cross-host reproducibility | **NOT VERIFIED** — four identical local builds only |
| macOS/Windows manual smoke | **NOT RUN**; covered by remote CI `test` matrix |

---

## 8. Supply chain

| Control | Status | Evidence |
|---|---|---|
| SBOM | **Implemented** | `.github/workflows/release.yml` runs `scripts/sbom.sh` → `dist/sbom.json`; `sbom: 9` playground suite green |
| SLSA provenance | **Implemented, signed** | `actions/attest-build-provenance@4d101475d8b20a2381f78447822ac1eab6504dd8 # v4.2.2`; verify with `gh attestation verify` |
| Provenance **coverage** | **Limited** | Attestation covers `dist/` release artifacts. It does **not** cover the `playground/runtimes/` wasm files. Do not claim otherwise. |
| Artifact signing | **Not implemented** for Playground runtimes | Only manifest SHA-256, which is integrity, not authenticity |
| Reproducible builds | **Claimed only narrowly, and honestly** | `website/content/security.md`: "No reproducible-build claim beyond single-host pure-Rust builds" |
| Dependency audit | **Implemented** | `cargo audit` job, `success` |
| Dependency remediation | **Done** | PR `#3` — rustls advisories, `pemfile` removed |

**Gaps, stated explicitly:** no signing of Playground wasm artifacts; no
provenance for them; no cross-host reproducibility claim; no lockdown of the
build toolchain beyond the pinned Rust version.

---

## 9. Version proposal

### 9.1 Proposed candidate identity

**`0.3.2`** — as a pre-release, i.e. **`0.3.2-rc.1`** for a candidate tag, with
the crate version remaining `0.3.1` until the promotion decision.

Rationale under [RELEASE_ENGINEERING.md](RELEASE_ENGINEERING.md):

- MINOR bump is correct: the changes since `v0.3.1` **add** capability (ABI 2
  session exports, multi-module programs, browser-host divergence contracts) and
  **tighten** checking. No public API is removed and no valid program's meaning
  changes except where it was previously wrong.
- The tightened diagnostics are **not** a MAJOR break under this project's
  pre-v1 policy, which reserves MAJOR for the v1 transition, but they **are** a
  user-visible compatibility event and must be in the release notes.
- `0.3.2` is already the declared `developmentVersion`; using it avoids
  inventing a new line.
- If a **development identity** is preferred instead of an RC, the next free one
  is **`0.3.2-dev.9`** — `dev.8` is immutable forever and must not be
  overwritten.

**`LANGUAGE_VERSION` stays `0.3.1`.** The language did not change identity; the
runtime and checker did. The invariant `LANGUAGE_VERSION <= VERSION` continues to
hold.

### 9.2 What a candidate must contain

1. Rebuild `playground/runtime` wasm from current source under a **new identity**
   (`0.3.2-dev.9` or `0.3.2-rc.1`).
2. Update `playground/runtimes/manifest.json` with the new entry and hash.
3. Update the five test files that pin stale filenames.
4. Update `website/site.config.mjs` `runtimeVersion`.
5. Correct the CHANGELOG statement that post-freeze fixes "ship only as numbered
   `0.3.2-dev.N` preview runtimes".
6. Add the missing staleness gate (§4.5).

Items 1–6 are **production changes** and require authorization this directive
does not grant.

### 9.3 Rollback plan

The rollback surface is small and fully reversible:

- `manifest.current` points at an immutable identity. Reverting it to `0.3.1`
  restores the previous public behaviour with no rebuild.
- Every published identity is immutable, so no artifact needs to be replaced —
  only the pointer moves.
- The crate version is not yet bumped, so no `Cargo.toml`/`Cargo.lock` revert is
  needed until promotion.
- If a candidate is distributed and withdrawn, deprecate the manifest entry
  (`available: false`) rather than deleting it; deleting would break historical
  permalinks and violate the immutability policy.

---

## 10. Decisions requiring a human

| # | Decision | Why it cannot be made here |
|---|---|---|
| 1 | Authorize a new runtime identity (`0.3.2-dev.9` or `0.3.2-rc.1`) | Creates a new frozen artifact; Directive 006 §4 forbids modifying frozen artifacts and publishing runtimes |
| 2 | Choose RC vs dev identity for the candidate | Determines whether the Playground default changes; a release-identity decision |
| 3 | Accept the diagnostic-code break (`E3001`→`E4031`) | SemVer-visible; needs a release-note commitment |
| 4 | Accept stricter checking as a compatibility event | Users with previously-compiling but unsound source will see new errors |
| 5 | Authorize the staleness gate as production work | New CI control; Directive 006 §4 forbids production changes |
| 6 | Whether `0.3.1` stays the public default until a corrected artifact exists | Currently `PROMOTE_DEV_TO_DEFAULT = false`; the stale artifact is the public one either way |

**Recommendation on decision 6:** leave `manifest.current` at `0.3.1` unchanged.
Switching the default to another stale artifact would not improve correctness,
and switching to an unbuilt identity is not possible.

---

## 11. Publication checklist — NOT EXECUTED

Nothing in this section was performed. It is recorded so the next authorized
step has a verified starting point.

- [ ] **NOT DONE** — New runtime identity created and wasm rebuilt from `3347ff6c`
- [ ] **NOT DONE** — `playground/runtimes/manifest.json` updated with new entry
- [ ] **NOT DONE** — Stale-filename pins in the five playground tests updated
- [ ] **NOT DONE** — `website/site.config.mjs` `runtimeVersion` updated
- [ ] **NOT DONE** — CHANGELOG corrected and dated
- [ ] **NOT DONE** — Staleness gate added to CI
- [ ] **NOT DONE** — `Cargo.toml` / `Cargo.lock` / `aura::VERSION` bumped
- [ ] **NOT DONE** — `scripts/artifact-smoke.sh --release` run
- [ ] **NOT DONE** — Release audit performed
- [ ] **NOT DONE** — Independent review (release author ≠ sole approver)
- [ ] **NOT DONE** — Security review for the trust-boundary changes
- [ ] **NOT DONE** — Release notes written
- [ ] **NOT DONE** — Tag `v0.3.2` created
- [ ] **NOT DONE** — GitHub release published
- [ ] **NOT DONE** — Candidate distributed
- [ ] **NOT DONE** — Stable promoted
- [ ] **NOT DONE** — `manifest.current` advanced

No tag was created, moved, or deleted. No package or release was published. No
frozen artifact byte was modified. `Cargo.toml` still reads `0.3.1`.

---

## 12. Final recommendation

**`NO-GO`** for distributing the artifacts that exist today.
**`HUMAN DECISION REQUIRED`** to authorize building a corrected candidate.

These are deliberately separated, because they are different authorizations:

| Stage | Recommendation | Basis |
|---|---|---|
| **Prepare** | **`GO`** | The dossier is complete; the source tree is sound on every probed surface; the path forward is concrete and small (§9.2). |
| **Build** | **`HUMAN DECISION REQUIRED`** | Requires a new frozen identity, which Directive 006 §4 places outside autonomous authority. |
| **Test** | **`GO`** | The local and remote matrices in §7 are green and reproducible at `3347ff6c`. |
| **Distribute candidate** | **`NO-GO` until rebuilt** | Distributing `0.3.1` or `0.3.2-dev.8` ships three reproducible defect classes, including two P1-class failures (§4.2, §4.3). |
| **Promote stable** | **`NO-GO`** | Premature; requires a corrected, tested candidate and explicit human authorization. Directive 006 §4 forbids it. |

### Why this is `NO-GO` and not merely a note

The finding is not "the artifacts are old". It is that **the artifacts give
wrong answers and accept programs the type system exists to reject**, and that
the artifact carrying the higher ABI version — the one that looks newest — fixes
none of it. A user selecting `0.3.2-dev.8` in the Playground reasonably expects
the newest behaviour; they get `0.3.1`'s defects under a newer name.

Set against Directive 005's own severity table, both of these are P1:

- *"panic or abort of the process with valid Aura programs"* — three traps,
  reproduced.
- *"acceptance of an incorrect program capable of violating relevant guarantees
  of the type system"* — four acceptances, reproduced.

Directive 005 concluded that no P1 existed. **That conclusion was correct for the
scope it declared — the source tree — and it remains correct there.** This report
does not reopen it. What it establishes is that a surface outside that scope, the
distributed artifacts, does contain P1-class behaviour. Reconciling a bounded
audit's scope with a later discovery is not a contradiction; it is what a
release-readiness review is for.

### What is genuinely reassuring

- The **source tree is clean** on every surface examined: 1477 tests, 23/23 remote
  jobs, native and fresh-wasm in agreement.
- The **defects are all in artifacts, not in code** — so the fix is a rebuild,
  not an investigation. No root cause is unknown.
- The build is **deterministic** on this host; the artifact simply was not
  rebuilt.
- The security documentation is **already honest** about the absence of a CPython
  sandbox; nothing overstates isolation.
- No P0 was found. Data corruption, security-boundary violation, and unauthorized
  execution were all examined and not reproduced.

### Residual risk accepted by this report's conclusion

If the current artifacts continue to be distributed, users of the Playground
receive incorrect `sort` ordering, incorrect f-string padding, silent acceptance
of type-unsound `set` operations and return-type mismatches, and occasional
aborts. The native CLI is **not** affected. This risk is bounded, reproducible,
and removable by a rebuild.

---

## 13. Time and budget

| Item | Value |
|---|---|
| Directive 006 wall clock | within the 6-hour limit |
| Documentation PRs | **1** (this report) — limit 1 |
| Production code changes | **0** |
| Frozen artifacts modified | **0** |
| Tags created/moved | **0** |
| Releases/packages published | **0** |
| API consumption | **not measured** — no meter is available in this environment, so no figure is reported rather than an estimate |

---

## Appendix A — exact reproduction commands

The differential probes are scratch harnesses and are not committed. Their
method is fully described here so the result can be re-derived.

```bash
# 1. Build the current source as a wasm artifact (writes only to gitignored target/)
cd playground/runtime
cargo build --release --target wasm32-unknown-unknown
sha256sum target/wasm32-unknown-unknown/release/aura_playground_runtime.wasm
#   observed: 5499a73384052b6368ce2bdf27f4bcd4f98087b6cc91ae533c5fb051b81b945a, 2 010 781 B

# 2. Verify shipped artifact integrity (all 8 available entries)
python3 - <<'PY'
import json, hashlib, os
m = json.load(open('playground/runtimes/manifest.json'))
for v in m['versions']:
    if not v.get('available'):
        continue
    p = os.path.join('playground/runtimes', v['artifact'])   # artifact already includes the id dir
    b = open(p,'rb').read()
    assert hashlib.sha256(b).hexdigest() == v['sha256'] and len(b) == v['bytes'], v['id']
print('all available artifacts match the manifest')
PY

# 3. Probe native — ONE .aura file per directory (eager sibling discovery)
mkdir -p /tmp/iso/C1 && cd /tmp/iso/C1   # entry file's parent dir is the source root
#   ... write main.aura, then:
aura main.aura
```

Harness contract for the wasm probes:
`AuraRuntime.fromBytes(bytes, name)` is **async** and must be awaited;
`runtime.run(source)` is **synchronous** and returns
`{status, stdout, diagnostics[]}`; a trap **throws** and must be caught.

## Appendix B — eleven documented near-misses

Each of the following produced a *false* result at some point during this
directive and was corrected. They are recorded because the corrected result
depends on the correction.

1. **Shared probe directory.** Native probes in a common directory were
   contaminated by sibling files — every run reported the same unrelated
   `E3001`. Native probing requires one `.aura` file per directory.
2. **Missing `await`.** `fromBytes` without `await` yields a non-runtime object;
   the first probe run failed with `rt.run is not a function`.
3. **Semicolons.** Aura does not sequence with `;` (`E1006`). An early probe's
   "agreement" was two parse errors agreeing.
4. **`List<T>` is not generic** (`E3002`).
5. **No `:g` format type** (`E1006`).
6. **`let` is immutable** — reassignment needs `let mut` (`E2001`).
7. **Unused bindings must be `_`** (`E2008`).
8. **`match` arms use `->`, not `=>`.**
9. **`fn main()` is required**; bare top-level statements are `E4027`.
10. **Nested double quotes inside an f-string break lexing** (`E1004`).
11. **Manifest `artifact` already includes the id directory.** Joining it under
    the id a second time reported 8/8 artifacts missing — a bug in the checker,
    not in the repository. Corrected in Appendix A.

A probe that reaches no code path proves nothing. Each case above was verified
to *execute* the intended path, not merely to produce a diagnostic.
