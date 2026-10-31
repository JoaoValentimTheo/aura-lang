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
- **A statically empty `range` was not recognized, so sound `never` functions
  were rejected.** Reachability analysis treated a `for` body as unreachable
  for `[]`, `{:}`, `set{}`, and a `..` range with equal literal bounds, but not
  for the builtin spelling `range(a, b)`, and the range check compared bounds
  for *equality* rather than for `end <= start`. Eight shapes therefore
  reported a spurious `E3006` on a body that can never run: `range(0, 0)`,
  `range(3, 3)`, `range(n, n)`, `range(5, 2)`, `range(0)`, `range(-5)`, `5..2`,
  and `1..-1`. The spec already stated the correct contract in three places —
  §22.1 requires `a..b` and `range(a, b)` to be "statically equivalent as well
  as dynamically equal", defines "an empty or descending range (`b <= a`) is
  empty", and defines `range(n)` as `range(0, n)`; §5.2 names `range(n, n)`
  explicitly as statically empty. So this was a conformance defect, not an
  unspecified choice, and no spec change accompanies the fix. `range_bound`
  now folds a unary minus (the parser only pre-folds `-9223372036854775808`),
  so `1..-1` compares by value; two occurrences of the *same name* on both
  sides are recognized as equal, because assignments are statements and a
  binding cannot change value between two reads inside one argument list.
  Everything else stays conservative: a call, a field, an index, or two
  *different* names may ascend and remains non-empty, so `range(a, b)` over two
  parameters and a range returned by a function still report `E3006`.
  `a_statically_empty_range_builtin_has_no_reachable_return` fails on the old
  code and passes on this one.
- **HTTP header values were destroyed in one direction and mis-encoded in the
  other.** Response header values outside ASCII were erased: `to_str()` rejects
  bytes ≥ 0x80 and `unwrap_or("")` turned the rejection into an empty string, so
  a server sending `x-latin: caf\xE9` reached Aura as `""`. The same
  `to_str()` also drove `header_lines`, so re-emitting the response into a new
  request produced `x-latin: ` on the wire — the value was gone end to end.
  Outgoing values had the opposite defect: they were encoded as UTF-8, so
  `café` went out as `0xC3 0xA9` where the browser host sends the ISO-8859-1
  byte `0xE9`. Both directions now follow the browser, which is the reference
  host: response values decode as ISO-8859-1 (lossless, so no `header_bytes`
  key is needed, unlike `body`), and request values encode as ISO-8859-1 with a
  value above `U+00FF` reported as `E4020` instead of being silently sent as
  UTF-8. That rejection is parity rather than a new restriction — the browser
  already threw `TypeError` for it, which `worker.js` has always surfaced as
  `E4020`. This is a **wire-octet compatibility change**: a native program that
  sends non-ASCII request header values will now emit different bytes than
  before, and one that sends values above `U+00FF` will now fail where it
  previously succeeded by accident. Both are recorded in
  `docs/adr/0006-http-header-value-encoding.md`, which is the governing
  contract. `non_ascii_response_header_value_is_preserved`,
  `outgoing_non_ascii_header_value_uses_latin1_octets`, and
  `header_value_above_latin1_is_reported_not_sent` fail on the old code and
  pass on this one.
- **JSON decode could not read back what JSON encode produced.** The encoder was
  deliberately bounded at 512 levels of value nesting and wrote `null` past it,
  but the decoder went through `serde_json::from_str`, whose deserializer caps
  nesting at 128. A 200-deep recursive `struct Node { child: Node | none }`
  round-tripped to a 2014-byte document that `json_decode` then rejected with
  `E3001: recursion limit exceeded`. The `from_json_depth` guard was dead code
  for deep documents because serde rejected them first. Decoding now runs one
  shared `parse_json` gate for all three entry points: a single-pass,
  allocation-free `max_nesting` pre-scan rejects documents deeper than
  `MAX_VALUE_DEPTH` before the parser runs, then `Deserializer` with
  `disable_recursion_limit` parses and `de.end()` preserves the trailing-content
  rejection that `from_str` provided. Documents up to 512 levels now round-trip
  byte-identically, and a 100 000-deep hostile document is a clean diagnostic
  rather than a stack overflow. A second, coupled defect went with it:
  `json_decode` reported malformed or over-deep input as `E3001`
  (`TYPE_MISMATCH`, a checker code) while `json_decode_as` and typed decode used
  `E4031` (`DECODE_MISMATCH`, runtime) for the same document. All three now
  report `E4031`; a genuinely ill-typed argument such as `json_decode(42)` still
  reports `E3001`. `E4031` was missing from `LANGUAGE_SPEC.md` §30.2 and from
  `error_samples()`, which `§30.4`'s reachability claim depends on; both are now
  present.
- **Method calls on a type parameter were resolved by name existence, not by
  bounds.** A generic body with `T: Show` could not call `x.show()` unless some
  struct somewhere in the program happened to declare a method named `show`:
  the checker had no method table for a bare `T` and fell back to asking
  whether the name existed on *any* type. Both directions were wrong. Legal
  bound-granted calls were rejected with `E2003`, and an unconstrained
  `fn render<T>(x: T) { x.show() }` was *accepted* whenever an unrelated struct
  had an inherent `show`, deferring the failure to a runtime `no method`
  error. Resolution now consults the parameter's bounds first
  (`LANGUAGE_SPEC.md` §36.4, `docs/GENERICS.md` §5: *"a bound is a static
  contract: it grants the body the right to call the trait's methods on a
  value of type `T`"*), runs the same overload and visibility resolution the
  nominal arm uses, enforces `mut self`, and reports a parameter-specific
  `E2003` when no bound grants the name. Built-in methods (`len`, `map`,
  `trim`, …) stay callable on a parameter, because they resolve against the
  runtime value and a type parameter has no static method class.
- **Zero-padded numbers put the zeros outside the sign.** `f"{-42:06d}"`
  rendered `000-42` instead of `-00042`, and the same sign-ordering error
  affected every float presentation (`f"{-0.5:010.3f}"` → `0000-0.500`). The
  parser recorded the `0` flag only as a fill character, so the formatter
  applied generic fill padding after inserting the sign. `FormatSpec` now
  carries an explicit `zero_pad` bit and the formatter pads between the sign
  and the digits. Two coupled bugs went with it: an explicit fill was silently
  overwritten by the flag (`f"{-42:*>06d}"` → `000-42` instead of `***-42`),
  and the spec sentence describing the flag was wrong. Bare alignments and
  non-numeric values keep their previous output.
- **`sort` aborted the interpreter on lists it could not order.** The comparator
  folded every incomparable pair to "equal", which is intransitive: a list
  containing `NaN`, a union-typed list, and a list mixing `int` and `float`
  above `2^53` each either misordered or, past the small-sort threshold, killed
  the run with `E4999`. `sort` now uses a total order — `NaN` last, mixed
  numerics compared exactly rather than through §10.4's rounding promotion —
  and rejects lists it genuinely cannot order with `E3001`, which is what §12
  already required. The normative text was part of the defect and is corrected
  in the same change.
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
