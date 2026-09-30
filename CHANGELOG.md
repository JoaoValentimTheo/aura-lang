# Changelog

Notable changes to Aura. Format follows Keep a Changelog; versions follow
`docs/engineering/RELEASE_ENGINEERING.md`. This changelog begins with the
current engineering program; earlier history is in `docs/release-notes/` and
Git.

## [Unreleased]

### Fixed
- **CPython boundary diagnostics** now point at the Aura call site instead of
  `1:1`. Every `py_eval`/`py_import`/`py_call` failure and every boundary
  conversion error (`E5001`, `E5002`, oversized-int `E4013`) carries the real
  source location.

### Added
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
