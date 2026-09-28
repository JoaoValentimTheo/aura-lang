# Aura Current Status

Authoritative continuation checkpoint for the AURA MASTER CORE COMPLETION
PROGRAM (Zen-to-Win Final Language Core Freeze).

## Result

**AURA CORE PUBLISHED — CI AND DEPLOY GREEN; 0.2.0 RELEASE BLOCKED ON A
VERSIONING CONTRADICTION.**

All Core milestones are complete, all nine Core SPEC GAPs are closed, and
CONF-PLAY-1 is closed. Local validation is green; artifacts are verified and
reproducible; the branch is pushed; GitHub CI is green on the exact pushed SHA;
the website deployment is green.

The requested public `0.2.0` release was NOT created: the repository's
authoritative versioning contradicts it (see "Version target" below). No tag,
release, or version bump was made, and no version-migration policy was
invented.

## Repository

- Branch `rewrite/v3-rust`.
- Program start HEAD: `05c1272`.
- Current HEAD: `7ef7d7a` (`fix(parse): bound container nesting during
  parsing`), equal to `origin/rewrite/v3-rust`.
- Worktree: clean except a pre-existing stray 1-byte `s` (untouched).
- Commits pushed this session: `e567c77`, `296f66f`, `7ef7d7a`.

## CI / deploy (verified)

- CI run `36494488581` on `7ef7d7a7` (7ef7d7a): 14/14 jobs success
  (language contract, miri, pure-Rust-no-CPython, cargo audit, test
  ubuntu/macos/windows, clippy, MSRV 1.83, rustfmt, extended property tests,
  playground wasm runtime, fuzz smoke, website).
- Deploy website run `36494488483` on `7ef7d7a`: build + deploy to GitHub
  Pages success.

## Version target (0.2.0) — CONTRADICTION

The repository's authoritative version sources fix the release line at
`0.0.x`, not `0.2.0`:

- `Cargo.toml` `version = "0.0.2"`.
- `src/lib.rs` `LANGUAGE_VERSION = "0.0.1"`.
- `tests/contract.rs` hard-asserts `aura::VERSION == "0.0.2"` (a released
  identity invariant).
- Tags `v0.0.1`, `v0.0.2` exist; `v0.0.2` is frozen.
- The release workflow requires the tag to equal the `Cargo.toml` version and
  requires `playground/runtimes/<tag-version>/aura_playground_runtime.wasm`.
- `docs/release-notes/v0.0.2.md` frames the line as "After 0.0.2 … a long
  language-maturation cycle".
- The only `0.2.0`-like string in the repository is the historical, unrelated
  Python-transpiler tag `v0.2.0a8` (a different, older product line).

Making `0.2.0` real would require: editing the released-identity test,
deciding a new release/language version model, adding `playground/runtimes/
0.2.0/`, and possibly colliding with the legacy `v0.2.0a8` tag. No
version-migration policy exists, so none was invented. Per the program's
STEP 12 the contradiction is reported rather than resolved.

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
- Playground node (on Node 20, matching CI): manifest 26, completion 7, ABI 67,
  integrity 27, differential 214, syntax 43, browser 59, worker 12, cache 7 —
  0 failed.
- `node playground/build.mjs --check`: manifest matches 3 versions.
- Website: examples 22, links 2089, browser 343, a11y 70 — 0 failed.
- WASM: clean-target build byte-identical to dev.30.
- Python suite 10, property/hardening/corpus/boundaries all pass.

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
Current `dev.30`: 1,654,216 /
`916a8282f7afcf67b89662af89d2f69cf562d9dbe41fe88cab1764a3ef19c578`
(reproduced across clean builds; zero imports). Regenerated this session by the
parser container-nesting fix.

## CI / deploy

VERIFIED: CI `36494488581` on `7ef7d7a` — 14/14 jobs green; Deploy website
`36494488483` on `7ef7d7a` — build + GitHub Pages deploy green. (An earlier CI
run `36490043812` on `e567c77` failed on the pre-existing over-limit grouping
differential case; fixed in `7ef7d7a`.)

## Next phase

AURA FILESYSTEM MODULE SYSTEM. Not started; see `docs/CORE_FREEZE.md`.

## Do not reopen

- The nine Core SPEC GAPs (closed).
- AUDIT-3 without the exact human token.
- Completed conformance/hardening without a reproduced regression.
- Historical runtime artifacts.
