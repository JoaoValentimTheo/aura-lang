# Changelog

Notable changes to Aura. Format follows Keep a Changelog; versions follow
`docs/engineering/RELEASE_ENGINEERING.md`. This changelog begins with the
current engineering program; earlier history is in `docs/release-notes/` and
Git.

## [Unreleased]

Nothing yet. The `0.2.1` release line is closed; no successor version has been
selected.

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
