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

## Runtime and limits

- The semantic **AST nesting limit is 256 levels** (expressions, statements,
  types, and modules), reported as `E1015` on every substrate.
- **WebAssembly call depth (open defect, B-1, present in released `0.2.1`):**
  native execution enforces the 512-frame call limit and reports `E4011`. In
  the browser, execution runs inline on the JavaScript engine stack, and a
  mainstream recursive program can exhaust that engine stack below 512 frames;
  the Playground then reports `E4999: Maximum call stack size exceeded` (a
  Worker error) instead of `E4011`. The 512-frame contract itself is unchanged
  and native is unaffected. Fixes ship as a new version; released runtime
  artifacts are immutable (see [Resource limits](/docs/reference-limits/)).
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

- The current **release** is `0.2.1` (published 2026-10-01), which adds
  builtin-name reservation, unified type nesting, filesystem-backed module
  acquisition for the CLI, and the multi-file Playground (see the
  [migration guide](/docs/migration-0-2-1/)).
- There is **no active development line**: the `0.2.1` release train is closed
  and no successor version has been selected.
- Released runtime artifacts are immutable; fixes ship as a new version.
- Semantics may still change before 1.0, always with an ADR and a migration
  note.
