# Changelog

All notable changes to Aura are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and the project uses
[Semantic Versioning](https://semver.org/).

## [0.1.0a3] - 2026-09-16

This alpha focuses on making Aura's *rules* real and enforceable, hardening the
parser and transpiler against malformed and adversarial input, consolidating
the Python interoperability bridge so community PyPI libraries are first-class,
and rebuilding the REPL on top of the same pipeline the CLI uses. It is a large
correctness release: existing programs keep working, but many latent bugs that
produced invalid Python or silent misbehaviour are now fixed.

### Added

#### Language rules (new `aura.transpiler.rules`)

- New `RuleChecker` implements Aura's structural language rules independently of
  type inference, with stable error codes from `aura.transpiler.errors`:
  - **E301** duplicate declaration of the same name in one lexical scope
    (`let x = 1; let x = 2`), and duplicate parameter names in a function or
    method signature.
  - **E004** `return` outside a function.
  - **E004** `break` / `continue` outside a loop.
  - **E004** `await` outside an `async` function. Top-level `await` remains
    legal because the CLI runs async programs inside a coroutine.
  - **E004** `self` used outside a class method.
  - **E004** assignment to an invalid target (`1 = 2`, calls, literals).
  - **E002** / **E302** unreachable statement after `return` / `throw` /
    `break` / `continue`.
- The rules run on `aura check` and `aura run`, and inside the REPL, so
  "strict by default" is enforced by every entry point instead of being
  documentation-only.
- Enum, `type` and `trait` declarations are now included in scope tracking.

#### Top-level guard exit

- `guard cond else { return }` at the top level now compiles to
  `raise SystemExit(...)` and cleanly terminates the program with exit code 0.
  Previously it emitted a bare `return` at module level, which is invalid
  Python. A bare `guard ... else { return }` is the idiomatic early exit and is
  explicitly permitted by the rules checker.

#### Python interoperability bridge (new `aura.stdlib.python`)

- New `python` module (importable as `import python` or
  `import stdlib.python`) provides a supported surface over Python:
  - `import_module(name, package=None)` / `load(name)` for dynamic imports;
    `load` returns a `ModuleProxy` so nested modules resolve through normal
    member access.
  - `eval(expr)`, `exec_code(code)`, `compile_source(source)`.
  - `call`, `getattr`, `setattr`, `hasattr`, `dir`, `type_name`.
  - `is_module`, `is_callable`, `is_class`, `is_instance`, `is_available`.
  - `to_aura` / `to_python` conversions (modules are wrapped in `ModuleProxy`).
  - `reload`, `add_path`, `site_packages`, `modules`, `interpreter_version`,
    `builtins` / `py_builtins`.
- `aura.runtime.install_runtime_aliases()` registers the friendly `python`
  alias next to `stdlib`, so `import python` works from source checkouts and
  from an installed package.
- `examples/python_interop.aura` demonstrates importing the standard library,
  `from ... import`, dynamic `import_module`, regex use and `python.eval`.

#### REPL overhaul (`aura.repl.engine`)

- Rewritten REPL built on the real parser, rule checker and transformer, so
  typed code behaves exactly like `aura run`.
- **Value echoing**: a bare expression prints its value (`repr`) and stores the
  result in `_`.
- **Persistent sessions**: variables, functions, classes, enums, traits and
  imports survive across lines.
- **Smart multi-line input**: continues while brackets/strings are open or the
  buffer does not parse yet and ends with a continuation token (`=`, `and`,
  `->`, ...). A complete chunk executes immediately, even with a trailing token.
- **Robust error recovery**: syntax, rule and runtime errors are reported and
  the session keeps the bindings defined so far.
- **Commands**: `:help`, `:vars`, `:type <expr>`, `:ast <code>`,
  `:py <python>`, `:load <file.aura>`, `:run <file.aura>`, `:history`,
  `:reset`, `:quit`/`:q`/`:exit`.
- Injectable `input_func`/`output_func` for testing, and a `main()` entry point
  for `python -m aura.repl.engine`.
- Backwards-compatible `locals` property and `process_buffer()` retained.

#### Tests

- `tests/test_stress_raw.py`: 290 raw stress tests that throw hand-written and
  generated Aura at the whole pipeline. They cover exhaustive operator and
  comparison matrices, deep nesting, long programs, radix/float literals,
  string escapes, f-strings, collection builtins, comprehensions, function
  signature matrices, class shapes, error handling, invalid-input cleanliness
  and precedence. The pipeline must run the program or raise a clean
  `SyntaxError` — never an internal error.
- `tests/test_language_rules.py`: 81 focused contract tests for the rules,
  mutability, tokenizer fixes, modules, functions, traits, empty classes,
  dict comprehensions, the Python bridge and the REPL.

### Fixed

#### Tokenizer / parser

- Invalid radix literals (`0xZZ`, `0b12`, `0o8`, `0x`) now raise a clear
  `SyntaxError` instead of an internal `ValueError` from `int()`.
- Leading-dot floats (`.5`, `.5e2`) are tokenized as floats; they previously
  produced a stray `.` operator followed by an integer.
- Removed dead, crash-prone `not in` handling that referenced a non-existent
  `self.source` on the parser; `not in` / `is not` dispatch is now explicit.
- `none` is accepted as a null literal everywhere `null` is (previously `none`
  leaked into generated Python as an undefined name).
- Missing type after `:` (`let x: = 1`, `def f(x: = 1)`) now raises
  `SyntaxError` instead of silently consuming `=` as a type name.
- Bitwise shift precedence now matches Python: `1 << 2 + 1` is `1 << 3` (= 8),
  not `(1 << 2) + 1` (= 5).

#### Traits

- Trait methods now preserve their full signature (parameters, defaults,
  keyword-only and variadic markers, return type, generics). Previously all
  parameters were discarded and replaced by `(self, *args, **kwargs)`.
- Trait methods with a default body keep that implementation; signature-only
  methods emit `raise NotImplementedError`.
- Trait field declarations are emitted as class attributes, and trait members
  can include visibility modifiers.
- Trait generic parameters (`trait Mapper[T]`) are parsed and retained.
- Trait method bodies are now checked as function bodies, so `return` and
  `self` inside them are valid.

#### Classes and modules

- Empty classes (`class Empty {}`) now emit a correctly indented `pass`; they
  previously produced a class header with no body and broke compilation.
- Dotted module names (`module Outer.Inner { ... }`) now compile to nested
  classes so `Outer.Inner.member` resolves; they previously emitted the invalid
  `class Outer.Inner:`.

#### Comprehensions

- `for (k, v) in iterable` only adds `.items()` when the iterable is provably
  dict-shaped (a dict literal, `dict(...)`/`AuraDict(...)`, or an existing
  `.items()`/`.keys()` call). Dict comprehensions over a list of pairs no longer
  crash with `'list' object has no attribute 'items'`.

#### CLI

- `aura check` now reports accurate issue counts across type, mutability and
  structural rules instead of only counting type errors.
- `aura run` treats a bare `SystemExit` (from a top-level guard exit) as a
  successful exit (code 0) instead of failing with code 1.
- Mutability and rule checks are shared helpers that return error lists, so all
  entry points (`transpile`, `check`, `run`) print consistent diagnostics.
- `aura repl` delegates to the rebuilt REPL engine.

#### Versioning / packaging

- Package version bumped to `0.1.0a3` (matches `aura.__version__`).

### Changed

- `aura check` / `aura run` now perform structural rule checking in addition to
  type and mutability checks. Programs that previously passed with dead code,
  misplaced `break`/`return`, duplicate declarations or invalid assignment
  targets will now be reported at check/run time.
- Top-level `await` is explicitly supported and no longer flagged.
- The REPL no longer executes only the last expression with an empty namespace;
  it runs full statements against a persistent namespace.

### Removed

- Dead parser branch that referenced a non-existent `self.source` attribute.
- Dead unreachable `return TraitDecl(...)` statement left over in the trait
  parser.

## [0.1.0a2] - 2026-09-16

Documentation and package-metadata fix.

### Fixed

- README documentation links now use absolute GitHub URLs, so they work on
  the PyPI project page (relative links resolved against pypi.org and 404'd).
- Added `[project.urls]` metadata (Homepage, Repository, Documentation,
  Changelog, Issues) to the published package.

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