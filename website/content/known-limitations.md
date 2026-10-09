# Known limitations

Aura is pre-1.0. This page states the current limits honestly; "not claimed" is
not the same as "broken", and nothing here is presented as more ready than it
is.

## Language

- **No inheritance.** The object model is the four pillars without
  class-based inheritance (see the [object model](/docs/guide-oop/)).
- **No user-defined operators or conversion protocols.** Equality and ordering
  are the built-in rules; there is no `__eq__`-style extension point.
- **No exceptions with user-defined catch types** beyond the language's error
  model; see [Errors](/docs/guide-errors/).
- **Static checker is conservative, not a full type system.** It rejects
  definite errors and agrees with the runtime, but does not attempt full
  inference.
- **No general semicolon sequencing.** Since `0.3.1`, `;` no longer separates
  statements or items; it is reserved for the Array type grammar. `let a = 1;
  let b = 2` is `E1006`.
- **Set membership is scalar-only.** A `Set` element must be a key-capable
  scalar (`int`, `bool`, or `string`) — the same requirement a map key has.
  Broader membership is deferred to a future RFC.
- **Array length is a compile-time literal.** An array type `[T; N]` requires
  `N` to be a non-negative integer literal (bounded at `2^24`), not an arbitrary
  const-expression.

## Runtime and limits

- The semantic **AST nesting limit is 256 levels** (expressions, statements,
  types, and modules), reported as `E1015` on every substrate.
- **Nested array literal as a direct call argument.** Contextual Array
  realization (`[a, b]` under an expected `[T; N]`) applies at an annotated
  `let`, a directly resolved function argument, a declared `return`, a struct
  field, and a nested bracket literal under an element type. The one released
  `0.3.1` gap is a **nested** array literal passed **directly** as a call
  argument, which is not realized as an Array; binding it through a `let`
  (or annotating that binding) works. This is a documented limitation inherited
  from the collection campaign, not a silent miscompile.
- **WebAssembly call depth (fixed in `0.3.1`; present in released runtimes up
  to `0.2.1`):** native execution enforces the 512-frame call limit and reports
  `E4011`. In the `0.0.2`–`0.2.1` browser runtimes, execution ran inline on the
  JavaScript engine stack, and a mainstream recursive program could exhaust that
  engine stack below 512 frames, reporting `E4999: Maximum call stack size
  exceeded` instead of `E4011`. The `0.3.1` Keystone runtime runs the
  explicit-continuation evaluator and reports `E4011` at the language boundary;
  the released `0.2.1` runtime stays frozen and keeps the old behavior. See
  [Resource limits](/docs/reference-limits/).
- **Integers are 64-bit.** Arithmetic overflow is a diagnostic (`E4013`), not
  wraparound.
- **Ranges materialize to at most 10,000,000 elements** (`E4013`).
- The **REPL is O(N²) in session size** (TD-13): each submission rebuilds the
  checker from prior declarations. Correctness is unaffected, and a single
  submission is fast; only very long programmatic sessions slow down.

## Python interoperability (`py` feature)

- **Not a Python sandbox.** The `py` feature embeds real CPython with full host
  authority (see [Python interoperability](/docs/python/)).
- Aura does **not** parse or run Python source as Aura, and does not reimplement
  the Python standard library.
- Python → Aura value conversion is by type identity; many objects arrive as
  their `repr` string.

## Platforms

- The CLI ships for Linux, macOS, and Windows (pure Rust, no CPython).
- The **`py` feature is TESTED** on Linux 3.10–3.13, macOS 3.12, and Windows
  3.12. Other CPython versions/platforms are best-effort or unsupported.
- The in-browser **Playground runs WebAssembly** and has **no host filesystem
  access** (`read_file`/`write_file` and the filesystem module provider report
  `E5002`). It can still execute a caller-supplied **virtual multi-file
  project** — the files live in the browser session, not on a disk.

## Pre-1.0 status

- The current **release** is `0.3.1` (codename **Keystone**), which gives
  `List`/`Array`/`Tuple`/`Set`/`Map` distinct identities, adds fixed-length
  contextual Arrays (`[T; N]`), makes `json_decode_as` take a canonical type
  position, removes general semicolon sequencing, and hardens the parser and
  runtime (see the [migration guide](/docs/migration-0-3-1/)).
- The next line, **`0.3.2`**, is **in development as a preview**: it has a
  development runtime artifact (`0.3.2-dev.5`, Host ABI 2) selectable in the
  Playground, but **no release**, version bump, or date. The published `0.3.1`
  runtime is the default and is unchanged. The development runtime adds real
  browser HTTP behind an experimental per-origin permission model and a bounded
  output policy; it is experimental and may change without notice. Candidate
  work beyond it still includes the HTTP standard-library expansion (sessions,
  cookies, streaming, uploads) and local macOS CI consolidation.
- Browser HTTP is **not** available on the frozen `0.3.1` Playground runtime:
  its builtin registry predates the HTTP surface, so a call to
  `http_get`/`http_request` is a compile-time `E2003` (undefined name) and no
  request is possible. Only the development runtime performs browser HTTP, and
  only after per-origin consent.
- Released runtime artifacts are immutable; fixes ship as a new version.
- Semantics may still change before 1.0, always with an ADR and a migration
  note.
