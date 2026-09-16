# Changelog

All notable changes to Aura are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and the project uses
[Semantic Versioning](https://semver.org/).

## [0.1.0a1] - 2026-09-16

First alpha release of the rewritten Aura toolchain.

### Added

- Installable package: `pip install .` provides the `aura` console command
  (`aura run/transpile/check/format/lint/test/repl`).
- Standard library modules: `stdlib.regex`, `stdlib.os`, `stdlib.http`.
- `aura init`, `aura add`, `aura install`, `aura deps` and `aura version`
  for project manifests (`aura.toml`) and dependency management.
- Local Aura module and package imports (`import util`, `import pkg.util`,
  `from pkg.util import x`).
- Python-style slicing `x[start:stop:step]`.
- Raw and byte string literals (`r"..."`, `b"..."`, `rb"..."`).
- Multi-module imports (`import os, sys`).
- Generic type-parameter checking and function argument type checks.
- Builtin call type inference (`len`, `sum`, `max`, `sorted`, ...).
- CI (GitHub Actions) and a tag-driven release workflow.

### Changed

- Code reorganized under the `aura` package; root directories are
  compatibility shims.
- `let` bindings are immutable and `const` is immutable; reassignment
  requires `let mut`.
- `throw "message"` raises `Exception("message")`.
- JSON serialization is strict (rejects `NaN`/`Infinity`).
- `io` uses UTF-8 everywhere and preserves blank lines in `read_lines`.

### Fixed

- String escape sequences are now decoded (`"\n"`, `"\t"`, `"\""`, ...).
- Multi-target assignment (`a, b = b, a`) no longer emits invalid Python.
- Closures that mutate captured locals emit `nonlocal`.
- Enum auto-numbering continues after explicit values.
- Raw strings as call arguments parse correctly.
- f-string text escaping no longer double-escapes.
- `aura test` works from an installed package.
- Formatter no longer splits `->` or rewrites string contents.

## [0.1.0] - Initial

First working transpiler: lexer, parser, AST, transformer, type checker,
standard library, examples and test corpus.