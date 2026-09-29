# Aura Current Status

Authoritative continuation checkpoint for the AURA MASTER CORE COMPLETION
PROGRAM (Zen-to-Win Final Language Core Freeze).

## Result

**AURA 0.2.0 RELEASE IN PROGRESS — PUBLIC RELEASE `0.2.0`.**

The human resolved the versioning blocker: the next public release is
`0.2.0`. The version authority has been migrated from the `0.0.x` line to
`0.2.0`, the `0.2.0` runtime artifact is built and verified, the Playground
defaults to `0.2.0`, and the website presents `0.2.0` as the current stable
release. Historical `0.0.1`/`0.0.2` releases and the `0.0.2-dev.*` chain are
preserved and frozen.

## Repository

- Branch `rewrite/v3-rust`.
- Current HEAD: the `0.2.0` migration commit(s) (see the git log).
- Worktree: clean except a pre-existing stray 1-byte `s` (untouched).

## Version authority (migrated)

- `Cargo.toml` `version = "0.2.0"`; `aura::VERSION` = `0.2.0`.
- `LANGUAGE_VERSION = "0.2.0"` — this is a language release, so the release
  and language versions coincide; they remain separate constants.
- `tests/contract.rs` asserts the current `0.2.0` identity.
- `playground/runtime/Cargo.toml` `version = "0.2.0"` (the current runtime).
- `playground/build.mjs` derives the channel from the crate version
  (`-dev` suffix ⇒ development, otherwise release) and pins historical
  entries.
- Website `site.config.mjs`: `releaseVersion`/`languageVersion`/
  `runtimeVersion`/`currentRelease` = `0.2.0`, `previousRelease = 0.0.2`.
- Tags `v0.0.1`, `v0.0.2` remain historical; `v0.2.0` is created after CI and
  deploy are green on the exact release SHA.

## Current runtime

- `playground/runtimes/0.2.0/aura_playground_runtime.wasm` — 1,654,161 bytes,
  SHA-256 `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc`,
  reproduced across clean builds, zero imports. Playground API 1, Host ABI 1.

## CI / deploy / release (verified)

- CI run `36499955242` on `668722f`: 14/14 jobs success.
- Deploy website run `36499955213` on `668722f`: build + GitHub Pages deploy
  success. The deployed manifest serves `current: "0.2.0"` and the deployed
  `0.2.0` artifact hash equals the canonical
  `9937fd80…c5bc`.
- Release run `36500653554` on tag `v0.2.0` (commit `668722f`): 5/5 jobs
  success. GitHub release published at
  <https://github.com/JoaoValentimTheo/aura-lang/releases/tag/v0.2.0>; its
  `aura-playground-runtime-0.2.0.wasm` asset matches the canonical artifact
  byte-for-byte (1,654,161 bytes, `9937fd80…c5bc`).

## Commits added by this program (oldest first)

- `c94317f fix(collections): complete collection type coherence`
- `fb0d8a9 feat(collections): add map items()`
- `815001f feat(collections): add list and map comprehensions`
- `dff32d3 fix(syntax): close the Core syntax SPEC GAPs`
- `1a568b3 feat(modules): complete in-source module resolution`
- `954e145 feat(cli): complete the Core CLI and REPL DX`
- `22dab48 feat(playground): complete the Aura editor experience`
- `9dd16f3 docs(core): synchronize the current Aura Core documentation`
- `e413593 chore(ci): close CI-RELIABILITY-1 and seed new Core surfaces`
- `24ea7e1 chore(runtime): record the final Core development runtime`
- `f0ab14b fix(core): resolve three adversarial findings`
- `409e36b fix(site): use WCAG-AA syntax colors shared by both front ends`
- `8bae490 docs(status): final Core completion checkpoint`
- `296f66f test(playground): restore separator in long-stdout scroll fixtures`
- `7ef7d7a fix(parse): bound container nesting during parsing`
- (this session) version migration to `0.2.0` and the `0.2.0` release commit(s)

## Resolved Core SPEC GAPs (all 9)

CONF-PARSE-8, numeric underscores, f-string outer edges, CONF-GRAM-4,
CONF-RESOLVE-6, CONF-RESOLVE-7, CONF-RESOLVE-8, CONF-RESOLVE-9,
CONF-RESOLVE-10. Decision table in `docs/CORE_FREEZE.md`.

CI-RELIABILITY-1 is closed: `slow-unit-*` fuzz artifacts are distinguished from
crash artifacts in both CI workflows.

## AUDIT-3 (human gate)

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

No exact approval token was supplied.

## Validation (final)

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: pass.
- `cargo test --locked --all-targets --all-features`: **745 passed, 0 failed**.
- `cargo test --locked --all-targets --no-default-features --features
  cli,repl,json,regex,time`: **735 passed, 0 failed**.
- `cargo +1.83.0 check --locked --all-features` (MSRV): pass.
- Playground node (on Node 20, matching CI): manifest 34, completion 7, ABI 67,
  integrity 29, differential 214, syntax 43, browser 62, worker 12, cache 7 —
  0 failed.
- `node playground/build.mjs --check`: manifest matches 4 versions.
- Website: examples 22, links 2089, browser 344, a11y 70 — 0 failed.
- WASM: clean-target build byte-identical to `0.2.0`.
- Runtime crate tests: 10 passed. Python suite 10, property/hardening/corpus/
  boundaries all pass.

CONF-PLAY-1 (TEST GAP) is CLOSED: three scroll fixtures still contained the
separator-free `while i < 600 { print(i) i = i + 1 }`, which CONF-PARSE-8 now
correctly rejects, so stdout was empty and the scroll assertions failed. The
fixtures were rewritten with a newline-separated body; the separator rule and
production parser are unchanged. The Playground browser suite is now 59/0 and
the website browser suite 343/0. There are no remaining browser failures.

## Artifacts

Frozen `0.0.2`: 1,366,621 /
`5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`.
Historical `dev.23`: 1,604,958 /
`71072150e67384120c63e22d6176f3683110735b84f74723bea315f79778a528`.
Historical `dev.29`: 1,614,239 /
`aa832ba72578897f6b99650574939011dda25e0d390fdb5efb6fae825816bdd3`.
Historical `dev.30`: 1,654,216 /
`916a8282f7afcf67b89662af89d2f69cf562d9dbe41fe88cab1764a3ef19c578`.
Current `0.2.0`: 1,654,161 /
`9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc`
(reproduced across clean builds; zero imports).

## CI / deploy

See the final `0.2.0` release report for the exact run IDs on the migration
SHA. Earlier verified: CI `36494488581` on `7ef7d7a` — 14/14 jobs green;
Deploy website `36494488483` on `7ef7d7a` — build + GitHub Pages deploy green.

## Next phase

AURA FILESYSTEM MODULE SYSTEM. Not started; see `docs/CORE_FREEZE.md`.

## Do not reopen

- The nine Core SPEC GAPs (closed).
- AUDIT-3 without the exact human token.
- Completed conformance/hardening without a reproduced regression.
- Historical runtime artifacts.
