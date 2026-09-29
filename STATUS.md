# Aura Current Status

**CURRENT OPERATIONAL STATE:** see `AGENT_STATE.md`.

**THIS FILE:** detailed chronological status/history. Historical statements are
preserved as evidence of what was true at the time; current operational
decisions must not require reading this entire ledger.

Filesystem-module phase namespace used for current work:

- FSM-P0 — architecture reconstruction
- FSM-P1 — filesystem module design
- FSM-P2 — multi-source provenance
- FSM-P3 — provider-neutral module graph
- FSM-P4 — native filesystem provider
- FSM-P5 — virtual/WASM VFS foundation

Historical bare `Phase N` headings below predate this namespace and remain only
where changing them would damage chronology.

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

### Phase 2 — multi-source provenance foundation

**COMPLETE. STOP BEFORE FILESYSTEM LOADING.** Phase 2 was implemented and
validated without adding native/VFS module discovery, `mod.aura` loading,
directory traversal, ownership resolution, symlink/case handling, package
management, or a multi-file Playground UI.

Local implementation commit:

- `5fde3bf7e2806f7ad0c284e72ec28d194b7e1c3e` —
  `feat(source): add multi-source provenance foundation`.
- Phase 1 remains the separate parent commit
  `8da50b9bbe304f4814e4b54a5d43ff952a5bdea4` —
  `docs(modules): define filesystem module architecture`.

Source/provenance model now implemented:

- `Span { start, end }` is unchanged and remains source-local.
- `SourceId` is an opaque 8-byte identity. Its packed map scope prevents an id
  from one `SourceMap` from resolving in another map; its local slot follows
  deterministic insertion order. Raw ids are not semantic ordering inputs.
- `Location { source, span }` pairs source identity with the existing byte
  range only at source-aware boundaries.
- `SourceMap` authoritatively owns source display names and source text.
  Text is `Arc<str>` so the native large-stack compiler worker and retained
  diagnostic map share the same source allocation rather than copying it.
- `SourceDiagnostic` wraps the legacy three-field `Diag` at source-aware
  boundaries. `Diag` itself kept its public struct-literal shape, preserving
  downstream source compatibility and keeping provenance out of AST/token
  nodes.
- `DiagnosticReport` owns both the `SourceDiagnostic` and its `SourceMap` and
  renders source name/line/column through that authoritative map. Foreign ids,
  out-of-range spans, reversed spans, and non-UTF-8-boundary spans do not
  silently resolve.

Compilation ownership and compatibility:

- `Compilation` owns the resolved/checked module, `SourceMap`, and entry
  `SourceId`.
- `compile_source_in_map` is the narrow source boundary: parser/resolver/checker
  internals still use source-local `Span`, then the resulting diagnostic is
  lifted into `SourceDiagnostic`.
- `compile_named_with_mode`, `run_named_program_with`, and
  `run_named_toplevel_with` provide source-aware library entry points.
- Existing `compile`, `compile_with_mode`, `run_source`, `run_program_with`, and
  `run_toplevel_with` retain their legacy result shapes and semantics. The
  compatibility execution APIs intentionally unwrap a report back to `Diag`;
  callers that need retained provenance use the named/report APIs.
- Current runtime attribution is correct for today's one-source compilation:
  runtime diagnostics are lifted to that compilation's entry source. Per-item
  cross-source runtime attribution remains a future module-assembly concern;
  no multi-source logical program is assembled in Phase 2.

Surface integration:

- CLI `run`, `check`, and `eval` use source-aware reports. File inputs retain
  their selected display identity; stdin remains `<stdin>` and does not infer a
  filesystem root.
- REPL retains a persistent `SourceMap`; completed submissions receive
  `<repl:1>`, `<repl:2>`, ... identities in stable submission order. The
  existing compact `E####: ...` UI is unchanged.
- Python/Host public contracts are unchanged. Source-aware execution retains
  provenance internally; focused Python coverage remains green.
- Playground runtime compilation uses `<playground>` internally while keeping
  Playground API 1 / Host ABI 1 and the existing source-name-free JSON shape.
- No second resolver was introduced; `src/resolve.rs` remains authoritative.

Permanent provenance coverage (`tests/source_provenance.rs`, 14 tests) proves:

- `Span` remains two `usize` values; `SourceId` and `Option<SourceId>` are
  8 bytes on the current 64-bit target;
- identical byte ranges in distinct sources remain distinct;
- cross-map ids cannot alias;
- source order is stable even when independent maps interleave allocations;
- names/text resolve through the correct map;
- parser/checker offsets match legacy compilation exactly;
- two independent source buffers with identical spans render against their own
  source records;
- current runtime diagnostics are lifted to the correct entry source;
- stdin and non-filesystem display names work without filesystem semantics;
- source-aware rendering is deterministic and rejects malformed source/span
  pairings;
- the legacy `Diag { code, message, span }` struct literal remains valid.

Adversarial review:

- Two independent read-only reviewers inspected provenance identity,
  compilation ownership, diagnostics, CLI, REPL, Python/Host, WASM/Playground,
  public API compatibility, source-copy behavior, AST/token impact, and release
  artifacts.
- Initial review found three concrete defects: adding source state directly to
  public `Diag` broke downstream struct literals; registered source text was
  copied again when moved to the native compiler worker; source-aware rendering
  silently clamped malformed spans. All three were reproduced and fixed.
- The review also challenged raw `SourceId` ordering/stability. Raw ids are now
  explicitly isolation-only and do not implement `Ord`/`Hash`; deterministic
  ordering is `SourceMap::order`, backed by the local insertion slot and tested
  under interleaved maps.
- Final re-review reported no remaining concrete Phase 2 contradiction.

Final Phase 2 validation on the implementation state:

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: pass.
- `cargo test --locked --all-targets --all-features`: **760 passed, 0 failed**.
- `cargo test --locked --all-targets --no-default-features --features
  cli,repl,json,regex,time`: **750 passed, 0 failed**.
- `cargo +1.83.0 check --locked --all-features`: pass.
- Playground runtime crate: **10 passed, 0 failed**.
- `node playground/tests/node/run-all.mjs`: manifest 34, completion 7, ABI 67,
  integrity 29, differential 214, syntax 43, browser 62, worker 12, cache 7;
  **0 failed**.
- `node playground/build.mjs --check`: manifest matches 4 versions.
- Current dirty-code WASM was rebuilt directly and tested rather than relying
  only on the published runtime: ABI **67/0**, differential **214/0**, syntax
  **43/0**, zero imports in syntax conformance. Built artifact: **1,655,697
  bytes**, SHA-256
  `00d5ac851a06f2d909708134e9aa9e4ed4b1b0b19cac53f597e42f20cec691ec`.
- `node website/tests/run-all.mjs`: examples 22, links 2089, base 2088,
  browser 344, a11y 70, reused Playground suite green; **0 failed**.

Released runtime artifacts remain byte-identical and were not staged or
modified:

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

Local closure state before the status-only checkpoint commit: branch
`rewrite/v3-rust`, HEAD
`5fde3bf7e2806f7ad0c284e72ec28d194b7e1c3e`; only this `STATUS.md` update and
the pre-existing one-byte untracked `s` remain outside the implementation
commit. No push, tag, release, version bump, runtime publication, or filesystem
loader action was performed.

The Phase 2 stop gate was later satisfied by an explicit Phase 3 instruction.
The pushed baseline is now `8d9765076867d803658c9a26d544b29f898357ad` on
both local and `origin/rewrite/v3-rust`; GitHub CI run `36516460304` is 14/14
green and Deploy website run `36516460327` succeeded on that exact SHA.

### Phase 3 — provider-neutral module graph foundation

**COMPLETE LOCALLY — FULLY VALIDATED; NOT PUSHED.** The provider-neutral
module graph foundation is closed locally. Native filesystem discovery/loading,
CLI child discovery, package roots/manifests, Playground multi-file UI,
release/tag/version work, and runtime publication have not started.

Remote starting baseline and gate evidence:

- branch: `rewrite/v3-rust`;
- Phase 3 started from exact pushed SHA
  `8d9765076867d803658c9a26d544b29f898357ad`;
- `origin/rewrite/v3-rust` was the same exact SHA before local Phase 3 commits;
- GitHub CI run `36516460304`: **14/14 green** on that exact pushed SHA;
- Deploy website run `36516460327`: **success** on that exact pushed SHA;
- local validation below is local evidence and is not described as GitHub CI.

Architecture and provider boundary:

```text
SourceProvider
  -> opaque SourceKey / SourceDescriptor
  -> deterministic ProviderChild discovery supplied by the provider
  -> ModuleGraphBuilder
  -> ModuleGraph ownership/provenance/topology
  -> recursive ast::Module / Item::Module lowering
  -> existing resolve::resolve semantic authority
  -> existing checker
  -> existing runtime
```

- `src/module_graph.rs` defines opaque `SourceKey`, `SourceDescriptor`,
  `ProviderChild`, `ProviderError`, the `SourceProvider` trait, deterministic
  `InMemorySourceProvider`, `LogicalModulePath`, `ModuleGraphNode`,
  `ModuleGraph`, and `ModuleGraphBuilder`.
- Provider identity (`SourceKey`), compilation identity (`SourceId`), and
  Aura logical module identity (`LogicalModulePath`) are distinct types with
  distinct responsibilities.
- The graph stores source ownership, provenance, topology, deterministic child
  order, and external depth only. It does not implement resolver scopes,
  `use`, visibility, canonical-name rules, checker semantics, or runtime
  semantics.
- Source text remains shared through `Arc<str>`; SourceMap registration follows
  deterministic graph order.
- External wrappers are public logical module containers, appended after
  parent-written items. Provider children are sorted deterministically by
  logical name, display name, and opaque provider key before lowering.

Ownership, cycles, diamonds, and logical lowering:

- duplicate external owners, repeated claims for the same source, one source
  owning conflicting logical paths, and in-source/external ownership
  collisions are rejected deterministically;
- the graph does not merge or augment competing module owners;
- graph construction is iterative; malicious provider ownership cycles
  terminate through duplicate source-identity detection;
- valid Aura semantic reference cycles are not blanket-rejected by the graph;
  they continue into the canonical resolver;
- shared-owner/diamond cases load one physical owner once;
- provider sources lower into recursive `ast::Module` /
  `Item::Module` wrappers and then pass through the existing
  `resolve::resolve` implementation; no second resolver was introduced;
- aliases and `pub use` remain resolver-owned behavior.

Provenance and limits:

- per-item source provenance is carried in sidecar metadata rather than
  changing `Span`, tokens, AST node layouts, or the public legacy `Diag`;
- `resolve_sourced` preserves item/source association while still using the
  existing Resolver and returns the flattened module with item `SourceId`
  attribution;
- checker and runtime sourced paths preserve active source/item attribution,
  including delayed checker diagnostics and runtime failures in child
  functions/lambdas;
- `parse_with_initial_depth` seeds the existing parser recursion accounting
  with external logical module depth so splitting source files cannot bypass
  the current E1015/parser recursion backstop;
- no semantic recursion/depth limit was silently changed.

Compatibility and error surface:

- public `Diag { code, message, span }` remains source-compatible;
- legacy single-source compile/run APIs retain their existing signatures and
  result shapes;
- `compile_provider_with_mode` is additive;
- new production codes are limited to implemented provider-neutral ownership
  classes: `E2020 MODULE_SOURCE_OWNERSHIP`,
  `E2021 DUPLICATE_LOGICAL_SOURCE`, and
  `E2022 MODULE_SOURCE_PATH`;
- provider read failure after ownership resolution uses existing `E4020 IO`;
- no native path/case/symlink policy has been invented in Phase 3.

Permanent focused coverage:

- `cargo test --locked --test module_graph`: **32 passed, 0 failed**.
- Coverage includes one-source/in-source compatibility, one and nested external
  modules, semantic equivalence, deterministic ordering, duplicate ownership,
  in-source/external collisions, equal local spans with distinct SourceIds,
  direct/indirect provider ownership cycles, valid semantic reference cycles,
  shared-owner diamonds, resolver-owned aliases/re-exports, malformed child
  provenance, checker/resolver/runtime child provenance, missing-main entry
  provenance, delayed overload diagnostic ordering, accumulated external
  depth, opaque SourceKeys, missing/read-failing provider sources, and foreign
  SourceId isolation.

Independent adversarial review:

- one independent read-only reviewer inspected `src/module_graph.rs`,
  `src/lib.rs`, `src/parse/mod.rs`, `src/resolve.rs`,
  `src/check/mod.rs`, `src/run/mod.rs`, and
  `tests/module_graph.rs`;
- it specifically challenged deterministic ordering, ownership collisions,
  cycles/diamonds, provenance, depth, resolver authority, public API
  compatibility, AST/token isolation, and opaque provider identity;
- it independently ran the focused module-graph suite: **32 passed, 0 failed**;
- no concrete Phase 3 defect remained after review, so no review-driven code
  change was required.

Full Phase 3 closure validation:

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: pass.
- `cargo test --locked --all-targets --all-features`:
  **792 passed, 0 failed**.
- `cargo test --locked --all-targets --no-default-features --features
  cli,repl,json,regex,time`: **782 passed, 0 failed**.
- `cargo +1.83.0 check --locked --all-features`: pass.
- Playground runtime crate:
  `cargo test --locked --manifest-path playground/runtime/Cargo.toml`:
  **10 passed, 0 failed**.
- `node playground/tests/node/run-all.mjs`: manifest **34/0**,
  completion **7/0**, ABI **67/0**, integrity **29/0**,
  differential **214/0**, syntax conformance **43 explicit native/WASM
  comparisons**, browser **62/0**, worker **12/0**, cache **7/0**.
  Differential detail: **60/60** freshly generated plus **19/19** committed
  programs agree native/WASM; 37-depth TypeExpr sweep reported native ceiling
  2048, wasm ceiling 768, **0 host failures**.
- `node playground/build.mjs --check`: manifest matches **4 versions**.
- `cargo build --locked --manifest-path playground/runtime/Cargo.toml
  --release --target wasm32-unknown-unknown`: pass.
- current local target-only WASM build:
  **1,666,803 bytes**, SHA-256
  `3e56dc2df1185c7859c977c54468788cee2e175b2e3773aab79b5359ee22071a`.
  It was not copied into a published runtime directory.
- `node website/tests/run-all.mjs`: examples **22/0**, links **2089
  checked / 0 errors**, base **2088 refs / 0 violations**, browser **344/0**,
  a11y **70/0**, and reused Playground suite fully green.

Historical runtime immutability re-verified after the Phase 3 build:

- `0.0.2`: **1,366,621 bytes**,
  `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`.
- `0.0.2-dev.23`: **1,604,958 bytes**,
  `71072150e67384120c63e22d6176f3683110735b84f74723bea315f79778a528`.
- `0.0.2-dev.29`: **1,614,239 bytes**,
  `aa832ba72578897f6b99650574939011dda25e0d390fdb5efb6fae825816bdd3`.
- `0.0.2-dev.30`: **1,654,216 bytes**,
  `916a8282f7afcf67b89662af89d2f69cf562d9dbe41fe88cab1764a3ef19c578`.
- `0.2.0`: **1,654,161 bytes**,
  `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc`.

Local Git closure before the status-only checkpoint commit:

- implementation commit:
  `0777d14b3b7f03aba64e3d6e17db4c23d44886e7`
  (`feat(modules): add provider-neutral module graph foundation`);
- remote remains
  `8d9765076867d803658c9a26d544b29f898357ad`;
- the pre-existing unrelated `.codex/config.toml` modification and one-byte
  untracked `s` are preserved outside Phase 3;
- Phase 3 has not been pushed.

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

Next exact action: await explicit human authorization to push the completed
Phase 3 local commits. Do not begin `NativeFilesystemSourceProvider` or any
native filesystem crawler/loader before that authorization and the next
phase instruction.

## Do not reopen

- The nine Core SPEC GAPs (closed).
- AUDIT-3 without the exact human token.
- Completed conformance/hardening without a reproduced regression.
- Historical runtime artifacts.

### Phase 3 remote closure / Phase 4 start — 2026-09-29

- Branch: `rewrite/v3-rust`.
- Phase 3 pushed SHA and `origin/rewrite/v3-rust`:
  `3240b4a737ef3549828150a70b1b174d9ca60880`.
- Exact-SHA GitHub CI run `36577679545`: **success, 14/14 jobs**.
- Exact-SHA Deploy website run `36577679491`: **success, 2/2 jobs**.
- Phase 4 gate is satisfied. Work is now limited to
  `NativeFilesystemSourceProvider`, real filesystem module discovery, and
  wiring `aura run/check <file>` through the existing provider-neutral graph.
- The unrelated local state remains protected:
  `M .codex/config.toml` and `?? s`.
- Phase 4 is local-only: coherent commits may be created, but they must not be
  pushed automatically.
- No tag, release, version bump, or historical runtime publication is
  authorized.

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

Next exact action: implement the native provider against the existing
`SourceProvider` / `ModuleGraphBuilder`, then add focused temp-directory and
CLI integration coverage before broader validation.

### Phase 4 — native filesystem provider local closure — 2026-09-29

**COMPLETE LOCALLY — FULLY VALIDATED; NOT PUSHED.** Phase 4 continues from the
verified Phase 3 remote baseline without resetting the interrupted worktree.
The native filesystem layer is an adapter into the existing provider-neutral
module graph, resolver, checker, and runtime pipeline.

Baseline and protected local state:

- branch: `rewrite/v3-rust`;
- starting/local Phase 3 baseline:
  `3240b4a737ef3549828150a70b1b174d9ca60880`;
- `origin/rewrite/v3-rust` matched the same SHA before Phase 4 commits;
- Phase 3 exact-SHA GitHub CI run `36577679545`: **14/14 success**;
- Phase 3 Deploy website run `36577679491`: **2/2 success**;
- pre-existing unrelated `.codex/config.toml` remains modified and outside
  Phase 4;
- pre-existing untracked `s` remains exactly **1 byte** and outside Phase 4.

Native source architecture and APIs:

- `src/native_source.rs` defines `NativeFilesystemSourceProvider` as a native
  adapter over the existing `SourceProvider` contract;
- `NativeFilesystemSourceProvider::new(path)` selects one entry file, makes
  that file the logical root owner, and makes exactly its parent directory the
  source root;
- `entry_path()` and `source_root()` expose the selected physical entry and
  fixed source root for native callers/tests;
- discovery supports `foo.aura`, `foo/mod.aura`, and nested descendants;
- physical provider identity remains distinct from `SourceId` and logical
  module path identity;
- `compile_file_with_mode(path, mode)` feeds the provider into the existing
  `compile_provider_with_mode` / `ModuleGraphBuilder` / resolver / checker /
  runtime pipeline;
- the native module and compile-file API are gated from `wasm32`; no native
  filesystem semantics were added to the Playground/VFS path;
- public `Diag { code, message, span }`, existing single-source compile APIs,
  Playground API 1, and Host ABI 1 remain unchanged.

Determinism and ownership:

- the original directory-discovery path could report the first raw
  `read_dir` failure before deterministic ordering; discovery now collects,
  classifies, and sorts candidate/error records before selection;
- repeated builds and trees created in different physical creation orders
  produce the same graph order and first diagnostic;
- `foo.aura` and `foo/mod.aura` each own logical child `foo` when unambiguous;
- simultaneous `foo.aura` + `foo/mod.aura` is a deterministic ownership
  collision;
- root-level conflicting `mod.aura`, in-source/external module ownership, and
  duplicate physical/provider ownership are rejected through the existing
  graph ownership rules;
- case-only candidate collisions are deterministic;
- a reproduced macOS case-insensitive-filesystem bug allowed a selected path
  such as `MAIN.aura` to resolve to physical `main.aura` and then rediscover
  the same file as a child. The provider now requires the selected entry
  basename to match an actual parent-directory entry exactly at the `OsStr`
  level. Permanent regression coverage preserves this behavior.

Permission, I/O, and diagnostic paths:

- optional-owner `NotFound` remains absence;
- `PermissionDenied` is no longer treated as absence and maps to the existing
  physical/provider I/O error class (`E4020`) where ownership/load semantics
  require it;
- invalid UTF-8 maps to `E1001` with the source descriptor registered so the
  diagnostic retains correct child provenance;
- provider diagnostics use one root-relative, forward-slash display identity
  instead of leaking host-specific absolute `PathBuf` spellings;
- permanent tests assert temporary-directory prefixes do not appear in
  diagnostics.

Containment and symlink policy:

- absolute `SourceKey`, `..`, and forbidden special-component traversal are
  rejected;
- the selected entry, source root, entry ancestors, child files, module
  directories, and `mod.aura` owners are covered by explicit symlink tests;
- the frozen `NO SYMLINK TRAVERSAL` policy is enforced without making
  canonical physical paths Aura logical identity;
- no upward manifest/root discovery was introduced.

CLI and source provenance:

- `aura check <file>` and `aura run <file>` use the native filesystem provider;
- `aura check -` and `aura run -` remain on the existing single-source path;
- stdin never treats the cwd as an implicit filesystem module root;
- stdin unknown-module failures remain `E2019` and are contextualized at the
  CLI boundary with `filesystem modules are unavailable for <stdin>`;
- real-file fixtures preserve child provenance for parser, resolver, checker,
  delayed checker, runtime, invalid UTF-8, and entry-level missing-main
  diagnostics;
- identical local spans in two physical files remain distinguishable by their
  existing `SourceId` sidecar provenance.

Cycles, diamonds, depth, and provider parity:

- direct and indirect semantic cycles terminate operationally and remain
  resolver/runtime-owned semantics;
- diamond/shared-owner cases load one owner without duplicate ownership;
- a real-file depth fixture proves filesystem wrapper nesting feeds the
  existing parser depth accounting and cannot bypass `E1015` by splitting
  source across files;
- `InMemorySourceProvider` and `NativeFilesystemSourceProvider` fixtures feed
  equivalent logical trees into the same `ModuleGraphBuilder` semantics.

Permanent focused coverage after the final fixes:

- `tests/native_source.rs`: **33 passed, 0 failed**;
- `tests/module_graph.rs`: **32 passed, 0 failed**;
- `tests/cli.rs`: **33 passed, 0 failed**.

Full local Phase 4 validation already completed before closure:

- `cargo fmt --all -- --check`: pass;
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: pass;
- selected no-default clippy with `cli,repl,json,regex,time`: pass;
- `cargo test --locked --all-targets --all-features`:
  **836 passed, 0 failed across 40 test targets**;
- `cargo test --locked --all-targets --no-default-features --features
  cli,repl,json,regex,time`:
  **826 passed, 0 failed across 40 test targets**;
- `cargo +1.83.0 check --locked --all-features`: pass;
- focused Python + REPL: Python **10/0**, REPL **51/0**;
- selected no-default native release build: pass;
- `git diff --check`: pass.

Playground/WASM isolation validation:

- Playground runtime clippy: pass;
- Playground runtime tests: **10 passed, 0 failed**;
- wasm32 release build: pass;
- `node playground/build.mjs --check`: manifest matches **4 versions**;
- Playground Node suite: manifest **34/0**, completion **7/0**, ABI **67/0**,
  integrity **29/0**, differential **214/0**, syntax parity **43 explicit
  native/WASM comparisons**, browser **62/0**, worker **12/0**, cache **7/0**;
- differential detail: **60/60** generated programs plus **19/19** committed
  programs agree native/WASM; TypeExpr sweep preserved native ceiling 2048,
  wasm ceiling 768, and **0 host failures**;
- current target-only local WASM build is **1,666,803 bytes**, SHA-256
  `3e56dc2df1185c7859c977c54468788cee2e175b2e3773aab79b5359ee22071a`;
  it was not copied into any historical or published runtime directory;
- a root-crate `wasm32-unknown-unknown` cross-check that enables native-only
  dependency families still fails in existing dependencies such as `pyo3-ffi`,
  `fd-lock`, and `home`; the actual Playground WASM crate/build and Phase 4
  native-filesystem isolation are green.

Historical runtime immutability re-verified after Phase 4 work:

- `0.0.2`: **1,366,621 bytes**,
  `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`;
- `0.0.2-dev.23`: **1,604,958 bytes**,
  `71072150e67384120c63e22d6176f3683110735b84f74723bea315f79778a528`;
- `0.0.2-dev.29`: **1,614,239 bytes**,
  `aa832ba72578897f6b99650574939011dda25e0d390fdb5efb6fae825816bdd3`;
- `0.0.2-dev.30`: **1,654,216 bytes**,
  `916a8282f7afcf67b89662af89d2f69cf562d9dbe41fe88cab1764a3ef19c578`;
- `0.2.0`: **1,654,161 bytes**,
  `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc`.

Independent adversarial review challenged discovery order, repeated-build
determinism, containment, SourceKey traversal, file/mod and in-source/external
collisions, case behavior, ancestor/module-directory symlinks,
`PermissionDenied`, host-path leakage, stdin isolation, real-file provenance,
E1015, cycles/diamonds, resolver authority, public `Diag` compatibility, and
WASM isolation. **No unresolved concrete Phase 4 defect remained.**

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

Local Git closure after the implementation commit:

- implementation commit:
  `877e8631aff80b97368159024c90c145f6c20583`
  (`feat(modules): complete native filesystem provider`);
- the implementation commit contains only `src/lib.rs`, `src/main.rs`,
  `src/module_graph.rs`, `src/native_source.rs`, `tests/cli.rs`, and
  `tests/native_source.rs`;
- `STATUS.md` remains the only Phase 4 closure change outside that
  implementation commit;
- unrelated `.codex/config.toml` remains modified and unstaged;
- unrelated `s` remains untracked, unstaged, and exactly **1 byte**;
- `origin/rewrite/v3-rust` remains the Phase 3 baseline
  `3240b4a737ef3549828150a70b1b174d9ca60880`;
- Phase 4 has not been pushed.

Next exact action: create the status-only local Phase 4 closure commit, verify
the final worktree/remote split, and stop before push/tag/release/version/runtime
actions.

### Phase 4 — remote closure — 2026-09-29

**REMOTELY CLOSED — EXACT-SHA CI GREEN.** This checkpoint supersedes the
earlier Phase 4 local-closure state for current status while preserving that
historical chronology. The earlier `NOT PUSHED` wording remains correct only
for the local checkpoint at the time it was recorded.

Remote closure state:

- branch: `rewrite/v3-rust`;
- final remotely verified Phase 4 implementation HEAD:
  `ac57d5af2c2d2fce7c6aea20dfa60e6c852f6e99`;
- implementation stack:
  - `877e8631aff80b97368159024c90c145f6c20583` —
    `feat(modules): complete native filesystem provider`;
  - `e2450b6a4e0223a61710bf1c77e0f7ea8f733830` —
    `docs(status): close native filesystem module phase`;
  - `ac57d5af2c2d2fce7c6aea20dfa60e6c852f6e99` —
    `fix(modules): silence non-macos root alias warning`.

Exact-SHA remote verification:

- GitHub CI run `36597793002` on
  `ac57d5af2c2d2fce7c6aea20dfa60e6c852f6e99`: **14/14 jobs success**;
- verified CI surfaces: Ubuntu, macOS, Windows, rustfmt, Clippy, MSRV Rust
  1.83, Miri, cargo audit, extended property tests, fuzz smoke,
  Playground/WASM, language contract, pure-Rust binary / no CPython, and
  website static validation;
- Deploy website run `36597792813` on the same exact SHA: **2/2 jobs
  success**, covering static-site build and GitHub Pages deployment.

Remote-closure CI fix:

- the initial Phase 4 pushed SHA
  `e2450b6a4e0223a61710bf1c77e0f7ea8f733830` exposed one reproduced
  portability defect under `RUSTFLAGS=-D warnings` on non-macOS targets:
  unused variable `path` in `allowed_platform_root_alias(path: &Path)`;
- the minimal correction was:

  ```rust
  #[cfg(not(target_os = "macos"))]
  let _ = path;
  ```

- that correction was committed as
  `ac57d5af2c2d2fce7c6aea20dfa60e6c852f6e99` and changed no filesystem
  semantics.

The remotely verified architecture remains:

```text
NativeFilesystemSourceProvider
    ↓
SourceProvider
    ↓
ModuleGraphBuilder
    ↓
logical ast::Module lowering
    ↓
resolve / resolve_sourced
    ↓
checker
    ↓
runtime
```

There is one graph-building semantic path and one resolver. No filesystem
semantics were added to the checker or runtime. One logical module continues
to have one authoritative owner.

Validated Phase 4 behavior retained by the remote closure includes real
filesystem module loading; `foo.aura`; `foo/mod.aura`; nested filesystem
modules; deterministic discovery and ownership collisions; in-source/external
ownership collision; exact-case behavior and the macOS case-insensitive
regression fix; the no-symlink-traversal policy; root containment;
`PermissionDenied` remaining distinct from absence; portable root-relative
diagnostic paths; invalid UTF-8 provenance; multi-file `run` / `check`; stdin
filesystem isolation and contextual `E2019`; cross-file parser, resolver,
checker, and runtime provenance; cycle/diamond termination; `E1015` across
real filesystem module depth; InMemory/native provider parity; and WASM
isolation.

Previously recorded local Phase 4 validation remains the local evidence for
this implementation:

- focused provider/module/CLI suites: **33 native_source + 32 module_graph +
  33 CLI = 98 passed, 0 failed**;
- all-features Rust: **836 passed, 0 failed**;
- selected no-default Rust: **826 passed, 0 failed**.

Release immutability was preserved through remote closure. No tag, release,
version bump, or runtime publication was created, and the canonical historical
runtime bytes/hashes recorded above remain authoritative and unchanged.

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

Next exact action: **STOP.** FSM-P4 is remotely closed and exact-SHA
CI-green. Do not begin FSM-P5, Browser VFS, multi-file Playground UI, package
management, LSP, async, OOP, macros, release work, or AUDIT-3 work without a
new explicit human instruction.

### FSM-P5 — provider-neutral virtual source / WASM VFS local closure — 2026-09-29

**COMPLETE LOCALLY — FULLY VALIDATED; NOT PUSHED.** FSM-P5 started from
`75c57428ca70c53a3d592fdbb93ec3c5d7cb46f8` on `rewrite/v3-rust`, with the
remote at the same SHA. The pre-existing unrelated `.codex/config.toml` change
and untracked one-byte `s` file remained outside FSM-P5 throughout.

The production virtual-source architecture reuses the existing provider-neutral
pipeline:

```text
caller-supplied virtual source set
    ↓
InMemorySourceProvider
    ↓
SourceProvider
    ↓
ModuleGraphBuilder
    ↓
logical ast::Module lowering
    ↓
resolve / resolve_sourced
    ↓
checker
    ↓
runtime
```

No `VirtualSourceProvider`, browser-specific graph builder, second resolver, or
filesystem semantics were introduced. `SourceKey`, `SourceId`,
`LogicalModulePath`, and user-visible `SourceName` remain distinct; `SourceId`
does not cross the public runtime JSON boundary.

The additive virtual-project request accepted by the current Playground runtime
uses one opaque `entry` key plus a `sources` array. Each source carries `key`,
user-visible `name`, UTF-8 `text`, and optional `{ name, key }` child ownership
links. Arrays are retained at the transport boundary so duplicate keys/fields
can be rejected deterministically. Virtual keys are transport identities, not
OS paths: empty keys, `.`, `..`, slash/backslash forms, drive-like forms, UNC
forms, and other path tricks are rejected before provider construction.

Current transport ceilings are host/runtime policy, separate from Aura language
semantics:

- project request: 2 MiB;
- source count: 4096;
- virtual key: 128 bytes;
- source display name: 1024 bytes;
- existing per-source limit remains 256 KiB.

`SourceName` is required to be unique within a virtual project so two sources
cannot collapse to the same public diagnostic identity after `SourceId` is
intentionally hidden. Sources are sorted by key before provider construction;
children are sorted deterministically before registration; the existing
`ModuleGraphBuilder` remains authoritative for ownership, logical-name validity,
case-only collision policy, graph order, cycles, diamonds, provenance, and
depth accounting.

The WASM/JS boundary remains **Playground API 1 / Host ABI 1**. Existing exports
remain intact. The current runtime adds feature-detectable exports:

- `aura_project_reset()`;
- `aura_project_push(word, nbytes)`;
- `aura_run_project()`.

`playground/web/runtime.mjs` adds `runProject(project, options)` while leaving
`run(source, options)` unchanged. Historical ABI-1 runtimes are not required to
export the project capability, and the loader continues to accept them.
Single-source result JSON retains its existing schema. Virtual-project
diagnostics add only a `source` field containing `SourceName` or `null`; no raw
`SourceId`/`source_id` is serialized.

Single-source compatibility is preserved precisely: a virtual project with one
source and no virtual children uses the same E4027 -> `CompileMode::Module`
evaluation fallback as the existing one-source Playground path. A genuinely
multi-source program without `main` still reports E4027 against the entry
`SourceName`, preserving the program-mode contract and missing-main provenance.

Permanent FSM-P5 coverage now proves virtual source success, nested children,
multiple siblings, insertion-order independence, native/InMemory/virtual
parity, duplicate SourceKey and SourceName rejection, duplicate logical
ownership, reused source ownership, in-source/external collision, case-only
collision, wrong-case and missing semantic references, parser/resolver/checker
and delayed checker provenance, runtime child provenance, equal local spans with
distinct names, semantic cycles, malformed provider cycles, diamonds,
deterministic initialization, E2003 forward-constant behavior, E1015 virtual
depth accounting, malformed/duplicate-field JSON, path tricks, resource limits,
runtime recovery, and legacy single-source ABI compatibility.

The independent adversarial review reproduced two defects during FSM-P5 and
both were fixed with permanent regression coverage:

1. duplicate virtual `SourceName` values could make distinct source diagnostics
   publicly indistinguishable; duplicate names are now rejected deterministically
   as host-policy E4020 before provider construction;
2. a one-source virtual project without `main` initially returned E4027 instead
   of matching the established single-source eval fallback; the fallback is now
   applied only to exactly one virtual source with no virtual children.

The reviewer re-ran both attacks against the real WASM artifact, including
reversed source insertion order, and found no unresolved reproduced defect or
regression from the fixes.

Final FSM-P5 local validation after those fixes:

- `cargo fmt --all -- --check`: success;
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: success;
- all-features Rust: **838 passed, 0 failed**;
- selected no-default Rust: **828 passed, 0 failed**;
- MSRV Rust 1.83 all-features check: success;
- focused root suites: **34 module_graph**, **33 native_source**, **33 CLI**;
- Playground runtime: **10 execute + 2 provider_parity + 19 virtual_project =
  31 passed, 0 failed**;
- current-source WASM ABI: **80 passed, 0 failed**, with zero imports;
- historical `0.2.0` ABI compatibility: **69 passed, 0 failed**;
- current-source native/WASM differential: **214 passed, 0 failed** plus
  **43 explicit syntax comparisons**, with generated parity **60/60 freshly
  generated + 19/19 committed** and the existing 37-depth TypeExpr sweep;
- Playground full suite: manifest **34/0**, completion **7/0**, ABI **69/0** on
  the historical current artifact, integrity **29/0**, differential **214/0**,
  syntax **43/0**, browser **62/0**, worker **12/0**, cache **7/0**;
- `node playground/build.mjs --check`: manifest matches all 4 recorded versions;
- website suite: examples **22/0**, links **2089 checked across 39 pages**,
  base-path **2088 refs across 39 pages**, browser **344/0**, a11y **70/0**,
  plus the reused Playground suite.

The final current-source target-only WASM build is **1,761,796 bytes**, SHA-256
`a0fce70c6e97268f1c7d12a0ddfe27a706452df36e818b9a28d9fd26c8d312fb`.
It remains only in build output and was not copied into a versioned runtime
directory.

Historical runtime immutability was re-verified after final FSM-P5 validation:

- `0.0.2`: 1,366,621 bytes,
  `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`;
- `0.0.2-dev.23`: 1,604,958 bytes,
  `71072150e67384120c63e22d6176f3683110735b84f74723bea315f79778a528`;
- `0.0.2-dev.29`: 1,614,239 bytes,
  `aa832ba72578897f6b99650574939011dda25e0d390fdb5efb6fae825816bdd3`;
- `0.0.2-dev.30`: 1,654,216 bytes,
  `916a8282f7afcf67b89662af89d2f69cf562d9dbe41fe88cab1764a3ef19c578`;
- `0.2.0`: 1,654,161 bytes,
  `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc`.

No tag, release, version bump, runtime publication, multi-file Playground UI,
browser persistence, package management, URL import, LSP, REPL VFS project
loading, async, macro, or AUDIT-3 work was performed.

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

Next exact action: **STOP after local FSM-P5 commits.** FSM-P5 is locally
closed and fully validated. Do not push FSM-P5 or begin multi-file Playground
UI, package management, LSP, async, OOP, macros, release work, or AUDIT-3 work
without a new explicit human instruction.

### Post-FSM-P5 correction pass — authority reconciliation — 2026-09-29

**FSM-P5 LOCALLY VERIFIED; AUTHORITY RECONCILIATION COMPLETE FOR HUMAN
REVIEW.** The correction pass started from local HEAD
`e17f6b58b839d002904ca8afa0a3ad6cd2fe45f9`; the remote tracking ref remained
`75c57428ca70c53a3d592fdbb93ec3c5d7cb46f8` throughout. The pre-existing
`.codex/config.toml` modification and untracked one-byte `s` remained protected
and were not edited, staged, restored, normalized, or deleted.

Independent FSM-P5 review confirmed one provider-neutral semantic pipeline:
virtual project -> `InMemorySourceProvider` / `SourceProvider` ->
`ModuleGraphBuilder` -> logical `ast::Module` -> `resolve_sourced` -> sourced
checker -> sourced runtime -> WASM boundary. No second resolver, second graph
builder, browser-specific Aura semantics, package system, or multi-file
Playground UI was found. `SourceKey`, `SourceId`, `LogicalModulePath`, and
`SourceName` remain distinct identities. Playground API remains `1`; Host ABI
remains `1` with feature-detected additive virtual-project exports.

No new reproducible FSM-P5 production defect was found in this correction
pass. The two defects already found and fixed inside the local FSM-P5 stack
(duplicate `SourceName` ambiguity and one-source virtual eval fallback) were
revalidated by permanent tests. Focused current-pass validation:

- `cargo test --locked --test module_graph`: **34 passed, 0 failed**;
- `cargo test --locked --manifest-path playground/runtime/Cargo.toml`: **31
  passed, 0 failed** across execute/provider-parity/virtual-project tests;
- current-source WASM ABI test: **80 passed, 0 failed**.

Full current-pass validation after authority/documentation correction:

- `cargo fmt --all -- --check`: success;
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: success;
- `cargo test --locked --all-targets --all-features`: **838 passed, 0 failed**;
- selected no-default Rust: **828 passed, 0 failed**;
- `cargo +1.83.0 check --locked --all-features`: success;
- Playground runtime: **31 passed, 0 failed**;
- current-source WASM ABI: **80 passed, 0 failed**;
- Playground full suite: manifest **34/0**, completion **7/0**, stable ABI
  **69/0**, integrity **29/0**, differential **214/0**, syntax **43/0**,
  browser **62/0**, worker **12/0**, cache **7/0**;
- `node playground/build.mjs --check`: manifest matches all 4 versions;
- website: examples **22/0**, links **2089 checked across 39 pages**, base-path
  **2088 refs across 39 pages**, browser **344/0**, a11y **70/0**, plus the
  reused Playground suite.

The rebuilt current-source WASM remains target-only at **1,761,796 bytes**,
SHA-256
`a0fce70c6e97268f1c7d12a0ddfe27a706452df36e818b9a28d9fd26c8d312fb`.
It was not published, copied into a versioned runtime directory, or used for a
version bump. Historical artifact immutability was re-verified:

- `0.0.2`: 1,366,621 bytes,
  `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`;
- `0.2.0`: 1,654,161 bytes,
  `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc`.

Repository authority was reconciled by introducing `AGENT_STATE.md`, reducing
`AGENTS.md` to durable rules plus startup/handoff protocols, mapping current
filesystem work to the `FSM-P<N>` namespace, updating current planning/status,
marking the filesystem design as the frozen FSM-P1 contract, and clarifying in
the spec/contract that logical Aura module identity is independent of native or
virtual source acquisition. Stable-release website statements remain stable
release documentation; one stale OOP statement that incorrectly listed
generics as absent from released Aura was corrected.

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

Next exact action: **HUMAN REVIEW OF FSM-P5 + AUTHORITY RECONCILIATION BEFORE
PUSH.** Do not push, create tags/releases, bump versions, publish runtime bytes,
or begin FSM-P6 without new human authorization.

### Post-FSM-P5 authority/config alignment closure — 2026-09-29

**FSM-P5 LOCALLY VERIFIED + AURA AUTHORITY ALIGNED — READY FOR HUMAN REVIEW
BEFORE PUSH.** This pass started at local `911d6ff004c0cbc2a94fffb1735f12ee1062c121`
with `origin/rewrite/v3-rust` at
`75c57428ca70c53a3d592fdbb93ec3c5d7cb46f8`. The only initial worktree state
outside commits was the now-in-scope project `.codex/config.toml` modification
and the pre-existing untracked one-byte `s`; `s` remained untouched and
unstaged. This entry supersedes only the preceding checkpoint's statement that
`.codex/config.toml` was protected/out of scope; that statement was accurate
for the earlier pass but is no longer current after explicit human scope
expansion.

Independent architecture review reconfirmed the single provider-neutral path:
virtual sources -> `InMemorySourceProvider` / `SourceProvider` ->
`ModuleGraphBuilder` -> logical module tree -> `resolve_sourced` -> checker ->
runtime -> WASM boundary. `SourceKey`, `SourceId`, `LogicalModulePath`, and
user-visible `SourceName` remain distinct. Provider ordering is normalized,
ownership/case collisions are deterministic, provider ownership cycles
terminate, semantic cycles remain resolver/runtime semantics, and malformed
virtual requests remain host-policy diagnostics. Playground API and Host ABI
remain `1`; virtual-project exports are additive and feature-detected.

Findings closed in this alignment pass:

- **PROJECT_CONFIG_STALE:** project-local `[models.new_thread]` and `[agents]`
  routing overrode/conflicted with the global ChatGPT Web route. They were
  removed; `.codex/config.toml` now contains only project-local safety/context,
  output, and history policy. `codex features list` loads without the prior
  project `models is ignored` warning.
- **DOC DRIFT:** `AGENT_STATE.md` still treated project config as unrelated;
  `src/lib.rs`, `src/source.rs`, and `src/resolve.rs` described provider-backed
  assembly as future/in-source-only. These comments/checkpoint statements were
  reconciled without semantic changes.
- **DOC DRIFT:** `LANGUAGE_SPEC.md` §27 now says the *module declaration
  syntax* is in-source, while the existing following normative paragraph keeps
  native filesystem and virtual/in-memory acquisition feeding the same logical
  module tree.
- **NON-ISSUE after adversarial reproduction:** a virtual project exceeding the
  2 MiB host-policy limit returned `E4020` on current WASM, did not trap, and
  the same runtime instance successfully executed a following program.

Global Codex routing remains outside project config and was re-observed as
`chatgpt-web/gpt-5.6-sol`, reasoning `high`, Web bridge
`http://127.0.0.1:17841/v1`, global concurrency `1`, compatibility depth `2`,
`multi_agent = true`, and `multi_agent_v2 = false` (Compatibility V1). No
credential values were inspected or recorded.

Validation rerun for this closure:

- `cargo fmt --all -- --check`: pass;
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: pass;
- `cargo test --locked --all-targets --all-features`: **838 tests, exit 0**;
- `cargo test --locked --all-targets --no-default-features --features
  cli,repl,json,regex,time`: **828 tests, exit 0**;
- `cargo +1.83.0 check --locked --all-features`: pass;
- focused module/VFS parity: `module_graph` **34/0**, `virtual_project`
  **19/0**, `provider_parity` **2/0**;
- `cargo test --locked --test syntax_docs`: **2/0** after the final spec wording
  correction;
- Playground Node: manifest **34/0**, completion **7/0**, ABI **69/0**,
  integrity **29/0**, differential **214/0**, syntax **43 explicit native/WASM
  comparisons**, browser **62/0**, worker **12/0**, cache **7/0**; generated
  parity **60/60 fresh + 19/19 committed**, TypeExpr sweep **37 depths**, zero
  host failures and zero WASM imports;
- `node playground/build.mjs --check`: manifest matches **4** versions;
- website: examples **22/0**, links **2089 checked across 39 pages**, base-path
  **2088 refs across 39 pages**, browser **344/0**, a11y **70/0**, plus the
  reused Playground suite;
- release WASM rebuild: pass, zero imports.

The final current-source target-only WASM is **1,761,796 bytes**, SHA-256
`a0fce70c6e97268f1c7d12a0ddfe27a706452df36e818b9a28d9fd26c8d312fb`.
It remains build output only and was not published or copied into a versioned
runtime directory. Historical artifacts were re-verified unchanged:

- `0.0.2`: **1,366,621 bytes**,
  `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed`;
- `0.2.0`: **1,654,161 bytes**,
  `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc`.

Local commits created by this alignment pass before this status checkpoint:

- `98090d8 chore(codex): align Aura project configuration`;
- `c7bdad3 docs(agent): reconcile post-FSM-P5 authority state`;
- `c2522da docs(spec): clarify module declaration source syntax`.

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

Next exact action: **HUMAN REVIEW BEFORE PUSH.** FSM-P5 remains locally closed
and unpushed. Do not begin FSM-P6, publish runtime bytes, create tags/releases,
or alter AUDIT-3 without new human authorization.
