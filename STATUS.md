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

AURA FILESYSTEM MODULE SYSTEM.

### Phase 0 — architecture reconstruction

Started from `rewrite/v3-rust` at
`a087a1d545a0330c5cbe814258e938c164e2e9bb`. The pre-existing untracked
`s` remains present and untouched. `venv/` appeared as untracked in the
initial status capture but was no longer present at the final status check;
no tracked diff records its removal and the cause is unconfirmed, so it was
not recreated.

Current front-end/runtime pipeline, verified from source:

```text
source text
  -> lexer
  -> parser -> nested ast::Module
  -> resolve::resolve -> flat ast::Module with canonical names
  -> checker
  -> interpreter
```

The authoritative module semantics already live in `src/resolve.rs`.
`Item::Module` and `Item::Use` are parser-level logical structure; the
resolver collects the complete logical tree, owns canonical module paths,
module scopes, `use`, `pub use`, module aliases, visibility, variant
ownership/collisions, and then removes module/import wrappers in `flatten`.
The checker and runtime consume only that resolved flat tree and therefore must
remain filesystem-agnostic.

Filesystem loading must enter before resolver semantics:

```text
SourceProvider
  -> physical discovery / ownership
  -> parse each source with source identity
  -> assemble the same logical Aura module tree used by in-source modules
  -> existing resolve::resolve
  -> existing checker
  -> existing runtime
```

This is the non-negotiable integration boundary: filesystem modules may create
logical module structure, but must not duplicate resolution, visibility,
import/re-export, alias, variant, checker, or runtime rules.

Confirmed current limitations relevant to the design phase:

- `compile_with_mode` accepts one source string and owns parse -> resolve ->
  check.
- CLI `run` / `check` read exactly one source; there is no module graph,
  path normalization, discovery, ownership, or cycle analysis.
- `run_source`, `run_program_with`, and `run_toplevel_with` currently
  discard the supplied file label before compilation.
- `Span` is only a byte range and `Diag` carries no source identifier.
  Multi-file diagnostics therefore require an explicit source/provenance
  design before implementation.
- REPL persistence is rooted in the same resolver `Session`; it has no
  filesystem-relative module context today.
- Browser/WASM uses the same compiler but accepts one source string and has no
  filesystem loader. Runtime `Host::read_file` is execution-time I/O and is
  not a suitable compile-time module-provider boundary.
- Existing initialization semantics register declarations first, evaluate
  top-level constants/expressions in source order, then run `main`. There is
  no existing filesystem-module cycle or cross-file initialization rule to
  reuse; that remains a Phase 1 design decision.

Independent read-only SCOUT and SPEC/TEST subagents were used for this
reconstruction. Mem0 repository memory was queried for prior filesystem-module
decisions and returned no matching memories, so current repository source,
`docs/LANGUAGE_SPEC.md`, `docs/CORE_FREEZE.md`, and tests remain the
authority.

Next exact action: Phase 1 adversarial design. Define the filesystem-module
contract explicitly for `mod.aura`, source ownership, root selection,
file-vs-directory conflicts, logical identity, symlinks/canonicalization,
case collisions, duplicate ownership, missing modules, cycle semantics,
source identity/diagnostic ordering, stdin, and the native/WASM virtual-source
boundary before writing production code.

### Phase 1 — adversarial filesystem-module design

**COMPLETE — DESIGN/EVIDENCE ONLY. No production filesystem-module code was
implemented.** The complete contract is recorded in
`docs/FILESYSTEM_MODULES_DESIGN.md`.

Session state at closure:

- Branch: `rewrite/v3-rust`.
- HEAD: `a087a1d545a0330c5cbe814258e938c164e2e9bb` (unchanged).
- Intended local changes: this `STATUS.md` checkpoint plus the new untracked
  `docs/FILESYSTEM_MODULES_DESIGN.md` design contract.
- Pre-existing untracked `s` remains present and untouched.
- `venv/` was not recreated or removed in this phase.
- No commit, push, tag, release, version bump, runtime bump, or production
  loader implementation was made.

Evidence gathering:

- Mem0 was queried once at the root for earlier filesystem-module decisions;
  it returned no matching memories.
- Two independent read-only agents were used within the project limit: one
  audited source provenance/diagnostics end to end; one adversarially reviewed
  module graph/cycle/ownership behavior. Neither edited files.
- The current source pipeline and exact existing semantics were rechecked in
  `src/error.rs`, lexer/parser/AST, `src/resolve.rs`, checker, runtime, CLI,
  REPL, Python bridge, WASM runtime, Playground tests, and module tests.

Confirmed provenance decision:

- Keep `Span { start, end }` source-local and unchanged in layout.
- Reject putting `SourceId` directly into every `Span`: on 64-bit targets the
  obvious representation would normally grow `Span` from 16 to 24 bytes and
  propagate through tokens and much of the AST.
- Adopt an opaque `SourceId`, first-class `Location { source, span }`, and a
  `SourceMap` owning source display name/provider identity/text.
- Bare spans must not cross source boundaries. Parsed sources and flattened
  emitted items retain source ownership; resolver/checker tables that retain
  cross-source locations use `Location`.
- Multi-source compilation/execution must retain the `SourceMap` for as long
  as diagnostics can reference source text. Raw `SourceId` is internal and is
  not an ABI or language identity.
- `Diag` must become source-aware while retaining a genuinely locationless
  form for diagnostics with no source location. Existing `(src, span)`
  formatting remains only a single-source compatibility boundary.

Confirmed filesystem-module contract:

- The explicitly selected CLI source owns logical root `[]`; its parent is the
  native source root. Aura does not search ancestors for manifests or roots.
- Non-root `dir/mod.aura` owns logical module `dir`.
- `foo.aura` and `foo/mod.aura` are mutually exclusive owners of `foo`; both
  existing is a deterministic source-ownership collision.
- A file-form module can own children through a same-stem directory
  (`foo.aura` + `foo/bar.aura`). If `foo` has no owner, orphan descendants are
  not promoted into the reachable module tree.
- Filesystem-created wrappers are conceptually `pub module`; declarations
  inside each source keep their normal explicit Aura visibility.
- An in-source child module and a filesystem child claiming the same logical
  module collide. There is no merge/reopen/partial-module behavior.
- Logical Aura path is semantic identity. Physical/provider identity is only
  provenance and duplicate-source evidence.
- Discovery is eager over the reachable **owned** physical module tree, not
  reference-driven. This avoids implementing a second resolver and makes an
  unused malformed reachable file behave like an unused malformed in-source
  module.
- Direct children are sorted by bytewise ASCII logical identifier and visited
  depth-first. Directory enumeration/hash/locale order is never semantic.
- Synthetic filesystem-module depth must explicitly consume the existing
  `parse::parse_recursion_budget()` even though discovery itself is iterative;
  exceeding it is `E1015` at the start of the offending child source. This
  preserves the current host-safety calibration and prevents file splitting
  from bypassing the nested-module backstop.
- Physical child wrappers are conceptually appended after all items written in
  the parent source, in sorted child-name order. This gives cross-file
  constants/top-level expressions one deterministic flattened source order.
- Aura source syntax never contains host paths; only `Name(::Name)*` denotes a
  module. Host path syntax is confined to the provider/CLI entry boundary.
- Symlinks are not followed in the first implementation: entry, reachable
  module-file, and reachable module-directory symlinks are rejected. Loops,
  escapes, and broken symlinks therefore fail at the first reachable symlink.
- Hard-link identity is deferred.
- Logical identifiers remain exact ASCII case-sensitive. Providers enumerate
  actual names rather than relying on case-insensitive lookup. A wrong-case
  reference is `E2019` on every host; two reachable physical candidates that
  differ only by ASCII case are rejected as a portability collision.
- No first-file-wins rule exists. Duplicate provider source ownership is an
  explicit error.

Cycle and initialization evidence:

- Reproduced an in-source `A -> B -> A` mutually recursive function cycle:
  success, stdout `0`.
- Reproduced reciprocal module aliases used by mutually recursive functions:
  success, stdout `0`.
- Reproduced a concrete-declaration re-export cycle: success, stdout `2` then
  `1`.
- Independent adversarial review also reproduced `A -> B -> C -> A` and a
  diamond successfully.
- A direct cross-module constant forward read (`a::X = b::Y` while `b::Y` is
  later in flattened order) is `E2003`; reversing source order succeeds.
- Therefore declaration/reference cycles are allowed when existing resolver
  semantics allow them. Filesystem representation does not create a blanket
  cycle error.
- Initialization remains the existing deterministic flattened source order:
  declarations first; constants/top-level expressions in order; then `main`.
  A constant cycle necessarily encounters a forward read and uses existing
  `E2003`. SCC/module-graph initialization is not introduced.

CLI / provider / WASM decisions:

- `aura run <file>` and `aura check <file>` use the selected file as logical
  root regardless of basename.
- `aura run -` / `aura check -` create `<stdin>` as logical root with no native
  filesystem module root; process cwd is never implicit. In-source modules
  remain valid; a reference needing a filesystem child is `E2019` with an
  explicit stdin/root-unavailable explanation.
- Native filesystem and browser virtual sources feed the same graph builder.
  Providers expose opaque source identity, display name, and UTF-8 text; the
  graph builder assigns `SourceId` and owns semantic assembly.
- Current one-source WASM/Playground remains a valid one-entry virtual source
  set. Compiler/runtime VFS support may precede any multi-file editor UI.

Diagnostics contract:

- Reuse `E2019` for unknown/missing logical modules, `E2018` for private
  semantic access, `E4020` for actual source-read I/O failure after ownership
  is known, `E1001` for invalid UTF-8, and existing semantic collision codes.
- New symbolic module-source diagnostics are required for source-ownership,
  case/portability, duplicate-source, and provider/path failures. Exact numeric
  allocation is deliberately deferred to Phase 2 so the error-code contract
  and tests change atomically.
- Diagnostic ordering is fixed: root read/decode/parse; then parent ownership,
  case and in-source collisions; then sorted depth-first children; then the
  existing per-source semantic order.
- Native diagnostic source names are root-relative and use `/` as the portable
  separator; raw OS error text and absolute host paths do not define language
  diagnostics.

Permanent adversarial design coverage is enumerated in
`docs/FILESYSTEM_MODULES_DESIGN.md`: one-file and equivalent in-source/multi-
file programs; nested/deep modules; malformed/missing children; checker/runtime
failure in children; direct/indirect cycles; re-export cycles; diamonds;
duplicate ownership; privacy/public access; aliases/re-exports; case collisions;
symlinks/escape; stdin; native/VFS parity; deterministic diagnostic ordering;
and cross-file constant/top-level ordering.

Validation actually run at Phase 1 closure:

- `cargo fmt --all -- --check`: pass.
- `cargo test --locked --all-targets --all-features`: **745 passed, 0 failed**.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: pass.
- `cargo test --locked --all-targets --no-default-features --features
  cli,repl,json,regex,time`: **735 passed, 0 failed**.
- `node playground/tests/node/run-all.mjs`: pass — manifest 34, completion 7,
  ABI 67, integrity 29, differential 214, syntax 43, browser 62, worker 12,
  cache 7; 0 failed.
- `node playground/build.mjs --check`: pass — manifest matches 4 versions.
- `cargo build --locked --manifest-path playground/runtime/Cargo.toml
  --release --target wasm32-unknown-unknown`: pass.
- `node website/tests/run-all.mjs`: pass — examples 22, links 2089, base 2088,
  browser 344, a11y 70, reused Playground suite green; 0 failed.

Artifact state after validation/build remains byte-identical:

- `0.0.2`: 1,366,621 bytes,
  `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`.
- `0.0.2-dev.23`: 1,604,958 bytes,
  `71072150e67384120c63e22d6176f3683110735b84f74723bea315f79778a528`.
- `0.0.2-dev.29`: 1,614,239 bytes,
  `aa832ba72578897f6b99650574939011dda25e0d390fdb5efb6fae825816bdd3`.
- `0.0.2-dev.30`: 1,654,216 bytes,
  `916a8282f7afcf67b89662af89d2f69cf562d9dbe41fe88cab1764a3ef19c578`.
- `0.2.0`: 1,654,161 bytes,
  `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc`.

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

Phase 1 has no remaining semantic blocker. Implementation-level public API
shape for a source-aware compilation object is deliberately left to Phase 2,
provided it obeys the provenance/lifetime contract. Exact new error-code
numbers, hard-link identity, explicit stdin/eval root flags, REPL filesystem
loading, multi-file Playground UI, package management, dedicated static
initialization-cycle analysis, and symlink support remain deferred.

Next exact action: **STOP after Phase 1.** On a future explicit instruction to
start Phase 2, implement source provenance (`SourceId` / `Location` /
`SourceMap` and source-aware compiled items) before adding native/VFS module
discovery. Do not begin filesystem loading while diagnostics can still
misattribute a local span to an arbitrary source.

## Do not reopen

- The nine Core SPEC GAPs (closed).
- AUDIT-3 without the exact human token.
- Completed conformance/hardening without a reproduced regression.
- Historical runtime artifacts.
