# Changelog

Notable changes to Aura. Format follows Keep a Changelog; versions follow
`docs/engineering/RELEASE_ENGINEERING.md`. This changelog begins with the
current engineering program; earlier history is in `docs/release-notes/` and
Git.

## [Unreleased]

Unreleased work on the `rewrite/v3-rust` development line. `0.3.1` remains the
current stable release; `0.3.2` has not been versioned, dated, or promoted, and
the Playground's `manifest.current` is still `0.3.1`. The entries below ship
only as numbered `0.3.2-dev.N` preview runtimes, which are selectable in the
Playground but are not releases.

### Added
- **Multi-module programs in hosted sessions (B14).** `aura_session_start_project`
  extends Host ABI 2 — there is no ABI 3 — so a hosted session can run a
  multi-file program through the existing session, consent, and suspension
  mechanism. Diagnostics are provenance-aware, attributing findings to the file
  that produced them. Consent, denial, and suspension paths are reused, not
  widened.
- **Browser hosts document their divergences instead of pretending to parity
  (B6, B7).** A `fetch` with `redirect: "manual"` returns an opaque-redirect
  response whose real status is not readable, so the worker reports `E4020`
  rather than inventing a code; duplicate response headers cannot reach the
  runtime because the platform's `Headers` is a combining collection. Both are
  asserted as contracts.
- **Nightly fuzzing actually runs (TD-23).** `rewrite/v3-rust` is now the
  default branch, which is the only branch whose copy of
  `fuzz-nightly.yml` the `schedule` trigger resolves. Previously the workflow
  existed only on a non-default branch and was never registered at all. Proven
  behaviourally, not structurally: the 03:17 UTC cron fired run `41037980226`
  (created 10:04Z after GitHub's scheduler queue delay), which ran the
  front-end targets for 30 minutes each plus a 5-minute runtime sweep and
  produced no crash artifacts. The workflow file existing was never the
  evidence; an executed scheduled run was.
- **Allocation-based cost guards for the encoder.** One guard previously
  asserted an absolute millisecond bound, which is not portable. It is replaced
  by a byte-per-output-byte measurement, which is deterministic and
  platform-independent.
- **A shared, retrying helper for scaling guards.** The bench suite asserted
  sub-quadratic growth from a single timing sample. All twelve scaling guards
  now retry a limited number of times and take each magnitude's own minimum,
  which distinguishes a transient burst from a real regression without
  loosening any threshold.

### Changed
- **Authoritative test commands and validation evidence.** The pull request
  template now carries the current commands and requires recorded results, so a
  claim without an executed command is visible in review.
- **Documentation was reconciled with the shipped contract.** Version and type
  claims were corrected, a Playground document link that 404'd was fixed, and
  the technical-debt ledger no longer describes defects that have been closed.
  This changelog itself claimed the `0.3.2` line was "planned but not started"
  while the `0.3.2-dev.1`, `-dev.5`, `-dev.7`, and `-dev.8` previews were
  already committed; it now distinguishes "not released" from "nothing exists".
- **The Playground document now matches the runtime it describes.**
  `docs/playground.md` documented `aura_abi_version()` as `1` while the runtime
  declares `ABI_VERSION = 2`, named `0.2.1` as the current published release
  after `0.3.1` had shipped, and counted seven manifest entries when there were
  nine. All three drifted independently and all three are now derived from the
  repository rather than restated: two `tests/contract_sync.rs` guards read
  `ABI_VERSION` out of the runtime source and `current` plus the entry count out
  of `runtimes/manifest.json`, so the prose cannot drift again without a failing
  test. ADR-0001's `Host ABI 1` was deliberately left unchanged — it is a dated
  snapshot of the 2026-09-30 state, not a current claim.

### Fixed
- **Hosted sessions could not resume.** A resumable `aura_session_*` session
  treated a completed step as a completed program, so a `main` that was only
  reached after an earlier step was never executed; two routing defects in the
  same state machine caused the remaining paths to be mis-dispatched. Step
  completion and program completion are now distinct states.
- **Sessions dropped initializer bindings.** Values bound by an initializer
  were not preserved across a park/resume boundary. Pinned by tests covering
  depth, `finally`, and re-entrancy.
- **Value encoding was quadratic in the rendered size.** `Value::display`
  rebuilt each subtree's `String` once per enclosing level, so byte work grew
  with output times depth. The fuzz `runtime` target reported this as a
  libFuzzer *timeout* rather than a crash. The encoder now appends into a single
  buffer: release 0.884s -> 0.024s (37x), debug 3.444s -> 0.268s (12.8x), with
  byte-identical output.
- **Two RustSec advisories in the TLS trust chain** were patched, and the
  unmaintained `rustls-pemfile` dependency was dropped.
- **SemVer precedence was centralized** and comparator bugs corrected, so
  version ordering is decided in one place instead of by ad-hoc string
  comparison.
- **Playground HTTP request caps are measured in UTF-8 bytes**, not characters,
  and the ABI expectations are derived rather than hand-copied.

## [0.3.1] — 2026-10-08

The **Keystone** language release (`release = language = 0.3.1`, codename
`Keystone`). It reopens the collection model and changes observable language
behavior, so it is a language release, not a patch. See
`docs/release-notes/v0.3.1.md` and
`docs/adr/0005-keystone-collection-model.md`.

### Changed
- **Versioning (ADR-0001).** The release and language identities advance to
  `0.3.1` because Keystone changes observable language behavior. The Playground
  runtime is published as the new immutable identity `0.3.1`; `0.0.2`, `0.2.0`,
  and `0.2.1` remain frozen and selectable.
- **Distinct collection identities (ADR-0005).** `List`, `Array`, `Tuple`,
  `Set`, and `Map` are distinct: `List != Array != Tuple != Set != Map`, a
  struct is never equal to a map, and AIS reports distinct `type_name`,
  `families`, and `value_kind`. Tuple is no longer list sugar.
- **Fixed-length contextual Arrays.** Array type syntax is `[T; N]`; a bracket
  literal is a List by default and an Array only under an expected `[T; N]`
  type, through one realization seam; no implicit conversion and no length
  covariance. `[1; 2; 3]` is not an Array literal.
- **Canonical type-position `json_decode_as`.** The second argument is a type
  position parsed by the canonical `ty()` grammar; the string spelling
  normalizes to the same node.
- **No general semicolon sequencing.** `;` in a general position is `E1006`.

### Added
- Set type `{T}` (key-capable scalar members, deterministic order; broader
  membership deferred).

### Fixed
- Bounded grouping/f-string interpolation recursion on WebAssembly, so a deeply
  nested source traps the language boundary instead of the host stack.
- The `0.3.1` WebAssembly runtime runs the explicit-continuation evaluator and
  reports `E4011` at the 512-frame boundary (the B-1 defect in runtimes up to
  `0.2.1`).
- A `json_decode_as` test that was not feature-gated for `json`, so
  `--no-default-features` builds could fail.

## [0.2.1] — 2026-10-01

A language release (`release = language = 0.2.1`), published as tag `v0.2.1`
(commit `3f5f870`). It is a focused, additive step over `0.2.0`: builtin-name
reservation, a unified type-nesting limit, filesystem-backed module
acquisition, a multi-file Playground, and robustness fixes found by independent
adversarial review. See `docs/release-notes/v0.2.1.md` and
`docs/MIGRATION_0_2_1.md`.

### Changed
- **Versioning (ADR-0001).** Release and language versions are distinct
  identities with the invariant `LANGUAGE_VERSION <= RELEASE_VERSION`. The
  source line advances to `0.2.1` (release and language) because the builtin
  reservation changed observable language behavior after tag `v0.2.0`. The
  Playground runtime is promoted from `0.2.1-dev.5` to the clean release
  identity `0.2.1` (same language contract).
- **Builtin-name value-namespace reservation (`E1009`).** A user value binding
  (`let`/`let mut`, parameters, loop/catch/pattern bindings, top-level `let`,
  top-level `fn`, import aliases) may no longer use a registered builtin name.
  Type, module, field, variant, and method namespaces are unaffected
  (ADR-0002).
- **Type-nesting limit (ADR-0004).** Structural `TypeExpr` nesting (`Box<…>`,
  `[T]`, `{K: V}`) now counts toward the semantic `MAX_AST_DEPTH = 256` budget
  on every substrate, removing the former native/WASM acceptance divergence. A
  flat union is unaffected. A resolved-type expansion budget bounds
  exponentially duplicating alias chains as `E1015`.
- **PyO3 boundary conversion is exact-type, not duck-typed.** Only genuine
  `bool`/`int`/`float`/`str` instances convert; a user object implementing
  `__bool__`/`__float__`/`__index__`/`__str__` takes the documented `repr`
  fallback instead of being silently coerced.

### Fixed
- **CPython boundary diagnostics** now point at the Aura call site instead of
  `1:1`. Every `py_eval`/`py_import`/`py_call` failure and every boundary
  conversion error (`E5001`, `E5002`, oversized-int `E4013`) carries the real
  source location.
- **CPython dict keys use exact-type identity.** A Python object implementing
  `__index__` is no longer coerced into an `int` key (rejected `E5002`), and an
  out-of-`i64` integer key is `E4013`, matching the scalar path.
- **CPython depth is bounded symmetrically.** A pathologically deep Python
  container is `E5002` in the Python→Aura direction (matching Aura→Python),
  not a silent `repr` string.
- **Security (red-team train 1):** bounded type-alias expansion depth so a long
  alias chain is `E1015`, never a host stack overflow; bounded *parameterized*
  alias expansion (was a quadratic hang); module nesting (in-source and
  physical) counts toward the 256-level semantic limit on every substrate; and
  string lexing is O(n) instead of O(n²).

### Added
- **Filesystem-backed module acquisition.** `aura run` / `aura check` compile a
  selected file together with its reachable filesystem module tree
  (`NativeFilesystemSourceProvider` over the provider-neutral module graph):
  sibling `<name>.aura` files and `<name>/mod.aura` directories become logical
  child modules consumed by the same canonical resolver. A logical module with
  both a file and a `mod.aura` owner is `E2020`. Native-only; the WebAssembly
  host has no filesystem.
- **Multi-file Playground (FSM-P6).** A project state model (files with opaque
  `SourceKey`s, an active file, an entry file, declared provider child links),
  file tabs with add/rename/set-entry/delete/reset, source-aware diagnostics,
  and an additive Host ABI 1 `aura_project_*` transport (`aura_run_project`)
  that executes a virtual multi-source project through the same resolver,
  checker, and runtime. A one-file project keeps the historical `source` path;
  a runtime without the exports reports a structured capability error.
- `docs/adr/0001`–`0004` recording the Architecture Decision Council's
  resolutions of the queued human decisions HD-1…HD-4.
- A CycloneDX **SBOM** and a machine-readable **release manifest** (with a
  dependency-lock hash), both attached to releases, plus signed SLSA build
  provenance at publish.
- An **end-user installed-binary smoke** (`scripts/artifact-smoke.sh`) and a
  **cross-release** test that runs every frozen runtime against its language
  line's fixtures.
- A website **migration guide** (0.2.1), Python-interop, security, and
  known-limitations pages.
- Engineering organization, V1 readiness, roadmap, human-decision queue,
  security program (threat model, trust boundaries, architecture, incident
  response), CPython compatibility target, release policy, technical-debt and
  risk registers, and this changelog.

## [0.2.0] — 2026-09

- Aura Core: generic string/int/bool-keyed maps, `map.items()`, list/map
  comprehensions, decided Core syntax rules, in-source module semantics,
  completed CLI/REPL, Playground, verified Native/WASM parity.
- Playground `0.2.0-dev.2` development runtime (builtin-name reservation).

## [0.0.2] — 2026-09

- Portable execution: WebAssembly runtime, host boundary, versioned browser
  Playground, website, cross-platform release infrastructure. No language
  change. `0.0.1` language semantics.

## [0.0.1] — 2026-09

- First usable public scripting surface: stdin, text-file I/O, program
  arguments (`read_line`, `read_file`, `write_file`, `args`, `E4020`).
