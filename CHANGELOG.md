# Changelog

All notable changes to Aura are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and the project uses
[Semantic Versioning](https://semver.org/).

## [0.2.0a6] - 2026-09-20

**Security hardening, macro expansion, interop fixes, CLI/REPL overhaul.**

### Added

- **Compile-time macros `once` and `retry`**: `once(body)` executes the body
  only on the first call (guarded by a hygienic boolean flag); `retry(count,
  body)` wraps the body in a try/except and retries up to `count` times.
- **Shared `pattern_utils.py`**: consolidated duplicated `_pattern_names` and
  `_target_names` logic from expressions, rules, and semantics into a single
  utility module.
- **REPL history persistence**: session history is saved to `~/.aura_history`
  on exit and loaded on startup; tab completion for commands and keywords.
- **Complete project template**: `aura init` now creates `src/`, `tests/`,
  `.gitignore`, `README.md`, and auto-initializes `.venv` with dependencies.

### Changed

- **`aura init` creates full project by default**: venv is created
  automatically (use `--no-venv` to skip). `aura venv` emits a deprecation
  warning.
- **`python.import_module()` returns `ModuleProxy`** for consistency with
  `python.load()`. `reload()` preserves the proxy wrapper.
- **`os.env()` without allowlist** emits a `DeprecationWarning` to prevent
  accidental secret leakage.
- **`compile_source()` validates mode** parameter, rejecting invalid values.

### Fixed

- **Dead code cleanup**: removed unused aliases (`Number`, `String`, `Let`),
  `_DICT_PATTERN_RE`, duplicate `_subject_type` method.
- **Security**: `python.py` eval/exec/compile have prominent docstring
  warnings; LSP sanitizes exception messages to strip internal paths.
- **Python interop**: `to_aura()` handles `frozenset` and primitive types;
  `site_packages()` handles missing `AttributeError` gracefully.
- **CLI**: removed unreachable `run` branch in `main()`.
- **REPL**: Ctrl+C discards buffer (not entire input); Ctrl+D discards buffer
  instead of auto-executing incomplete code.

### Tests

- 4313 tests passing.

## [0.2.0a5] - 2026-09-20

**Critical parser/transpiler fixes, training corpus, docs hardening.**

### Fixed

- **Parser**: `is_kwonly=True` for bare `*` separator in function declarations
  and lambdas; `FloatLiteral` handled in pattern matching; `seen_star` tracking
  prevents multiple bare `*` in function declarations.
- **Transpiler**: `MemberPattern` renders dotted names correctly (`Color.RED`);
  `??=` uses temp variable for non-trivial LHS to avoid triple-evaluation;
  `ForStmt` creates `RangeExpr` copy instead of mutating original AST node.
- **ERRORS.md**: E004 emitter description corrected; E320 documented as
  reserved (dead code, parser uses SyntaxError directly).

### Added

- **Training corpus** (19 files): `training/language/` (10 files),
  `training/reference/` (4 files), `training/anti-patterns/` (5 files).

## [0.2.0a4] - 2026-09-19

**Espelhagem/Kof na sintaxe** — a full syntax pass plus a documentation
restructure that mirrors the Kof language reference.

### Added

- **Documentation site**: GitHub Pages serves `docs/` (just-the-docs), with
  `docs/index.md` as the home and navigation for the tutorial and reference.
  The project README now uses absolute GitHub URLs so every documentation link
  resolves on **both** GitHub and PyPI, and the logo renders on both.
- **Explicit Python interop through `py.`**: `import py.re`, `import py.os.path`
  (binds the last segment), `import py.re as regex`, `import py.re, py.json`,
  and `from py.math import sqrt, pi as PI`. The `py.` prefix separates host
  Python modules from Aura modules; a name that collides with an Aura keyword
  must be aliased (`from py.re import type as re_type`), otherwise it is a
  pointed error. The dynamic `python` bridge remains for run-time access
  (`python.import_module`, `python.getattr`, `python.eval`, ...).
- **Set `{T}` and tuple `(A, B)` type annotations** parse (mapping to
  `Set[T]` / `Tuple[A, B]`), matching the set/tuple literal values.
- `docs/language-reference/` — a complete, evidence-based language reference
  (lexical structure, grammar, syntax, types, type system, expressions,
  statements, functions, closures, classes, modules, semantics, Python
  interop) with `*.pt_BR.md` siblings, replacing the `LANGUAGE.md` /
  `LANGUAGE_PT.md` / `TYPES.md` / `TYPES_PT.md` monoliths.
- `docs/learn/` — a numbered tutorial path.

### Changed

- Foreign spellings from other languages are now rejected with pointed
  messages instead of silently mis-parsing: `elif`/`elsif` (use `else if`),
  `var`, `val`, `fun`/`func`/`function`, `lambda`, `foreach`, `switch`,
  `repeat`, `do`, and `new C()` (use `C()`). `var`, `val`, `fun`, `func`,
  `function`, `foreach`, `switch` and `repeat` are now reserved words.
- Keyword-colliding declaration and parameter names are rejected uniformly
  (`def if() {}`, `let var = 1`, `def f(var) {}`), while `self`/`cls` remain
  valid as explicit receiver parameters and `match`/`case` as method names.

### Fixed

- **A bare `return`/`break`/`continue` no longer swallows the next line's
  identifier** as its value or loop label. A value/label must be on the same
  line as the keyword (`break` newline `print(x)` no longer becomes
  `break print`).
- **Union type aliases**: `type Maybe = int | none` now records the full type
  and emits valid Python (`Maybe = int | None`); previously the union tail after
  `|` was dropped and leaked as a stray statement (invalid `Maybe = int |`), and
  the `none` component was not translated to Python's `None`.
- `enum` members may be separated by `,`, `;`, or a newline (a member per line
  now parses, as documented).
- `match` case bodies now require `->` or `{ ... }`; foreign `case x:` / `case
  x =>` and a missing body give a pointed error instead of silent mis-parse.

### Removed

- The generated test corpora (`tests/*_tests/`, 5550 `.aura` files) and their
  runner, plus the mass generators that produced them. The suite dropped from
  ~11.5k to ~4.4k tests, all behavioural (the removed files were compile-only
  permutations with no assertions).

## [0.2.0a3] - 2026-09-19

**Lexical/structure coherence audit** — keywords, variables, constants,
structures, operators and lists.

### Fixed

- **Operator precedence now matches Python for the shared operators.** The
  bitwise group (`|` < `^` < `&` < shifts) and comparison were previously
  misordered: `1 & 2 == 2` parsed as `1 & (2 == 2)` instead of `(1 & 2) == 2`,
  and `1 == 1 << 2` as `(1 == 1) << 2`. Comparison/equality/membership/identity
  now share one level, looser than bitwise and shifts, exactly as in Python and
  as `docs/GRAMMAR.md` already specified. Values are spaced so left-associative
  nesting is unambiguous.
- The precedence table in `docs/LANGUAGE.md` §13 contradicted the parser and
  `docs/GRAMMAR.md`; it has been corrected.
- `x is <literal>` (e.g. `x is "a"`) is now a pointed parse error instead of
  emitting Python's `SyntaxWarning`; `x is none` / `x is not none` stay valid.
- An open-ended range (`0..`) is now recognised and lowers to `itertools.count`
  (with the import injected). Previously `let r = 1..` silently swallowed the
  next token as the range end (`range(1, print(1) + 1)`); a range now ends at a
  newline, a block `{`, a closer, or `step`.
- `let private x` (trailing modifier) inside a class body now raises a pointed
  error instead of creating a field named `private`.
- `const NAME` without a value now reports "constant 'NAME' requires a value"
  instead of the opaque "Expected '='".

### Changed

- **Collection conveniences unified**: `.size()`, `.length()` and `.len()` all
  mean `len(...)`, and `.add(x)` appends. A user method with the same name still
  wins. Documented the list method surface in `docs/LANGUAGE.md`.
- Documented `/` (true division) and `%` (divisor sign) as Python semantics.

## [0.2.0a2] - 2026-09-19

**OOP syntax freeze** — Alpha 0.2.0 codename: **"Equinox"**, OOP completion.

This release completes the OOP portion of the Equinox syntax freeze: abstract
classes, one constructor style per class, and pointed diagnostics for foreign
modifiers that used to be silently accepted.

### Added

- **Abstract classes** (`abstract class` + `abstract def`): an abstract class
  cannot be instantiated (`E316`) and may declare body-less `abstract def`
  members that a concrete subclass must implement (`E309`). An abstract class
  may defer, charging the obligation transitively to its concrete subclass. It
  compiles to a Python ABC with `@abstractmethod` members, so the same rule is
  enforced at runtime. A trait remains a pure contract; use an abstract class
  when the base needs state or shared concrete behaviour.

### Changed

- **One constructor style per class**: a class may declare header fields *or* a
  manual `def new`, never both. Mixing them is a syntax error, because the
  header already generates a constructor and a second one would leave the
  header fields unassigned.
- `abstract` is now a reserved word.
- `override def` and `abstract def` inside a `trait` are now rejected with a
  pointed message instead of being silently parsed as fields named `override`
  or `abstract`. Foreign member modifiers (`final`, `open`, `sealed`,
  `implements`, ...) are rejected the same way.

### Fixed

- **Transitive abstract obligations**: a concrete class extending a chain of
  abstract classes now correctly satisfies every inherited abstract method once
  it implements them, instead of being flagged because an intermediate abstract
  class left a method open.
- `aura lint` now reports every `def  ` (double-space) occurrence, prints
  diagnostics to `stderr`, and exits non-zero on warnings unless
  `--allow-warnings` is given.
- `aura run` no longer prints a raw `SystemExit` string code; a string exit
  code is written to `stderr` and mapped to exit status 1.
- The generated test-scenario tools only create their output directories when
  run as scripts, so importing them no longer has filesystem side effects.

### Removed

- The unused `ERROR_TEMPLATES` catalogue and `ErrorSeverity.NOTE` (dead code;
  `docs/ERRORS.md` is the reference) and the associated obsolete tests.

## [0.2.0a1] - 2026-09-18

**Syntax freeze** — Alpha 0.2.0 codename: **"Equinox"**.

This release marks the official syntax freeze for Aura. The grammar is now
stable and will not change before the 1.0 release. All subsequent work focuses
on hardening, documentation, and performance.

### Added

- **Venv site-packages auto-discovery**: `aura run` and `aura debug` now
  automatically add the project's `.venv` site-packages to `sys.path`, so
  PyPI packages installed via `aura add` / `aura install` are importable at
  runtime.
- **Security audit document** (`SECURITY_AUDIT.md`): comprehensive threat
  model, findings, and mitigations for the transpiler, CLI, LSP, and stdlib.
- **Fuzzing test suite** (`tests/test_fuzz_security.py`): Hypothesis-based
  fuzzing for the parser and transpiler, covering random input, deeply nested
  constructs, edge cases, and Unicode identifiers.
- **Machine code compilation research** (`docs/PHASE8_MACHINE_CODE_RESEARCH.md`):
  evaluation of Nuitka, Cython, Codon, mypyc, Numba, and PyPy with a
  NO-GO recommendation for alpha 0.2.0.

### Fixed

- **LSP traceback leak**: the LSP server no longer sends full Python
  tracebacks to the editor on internal errors; a sanitized message is
  returned instead.
- **CLI traceback leaks**: `aura transpile` and `aura run` no longer dump
  full Python tracebacks to stderr; only the user-facing error message is
  shown.
- **HTTP response body DoS**: `AURA_HTTP_MAX_BYTES=0` no longer disables the
  32 MiB response body cap; only positive values are accepted, with a 1 MiB
  fallback safety cap.
- **LSP bypasses MAX_SOURCE_BYTES**: the LSP server now enforces the 16 MiB
  source size limit on incoming documents.
- **Structured recursion errors**: `_mutability_diagnostics` and
  `_rule_diagnostics` now return proper `AuraError` objects on
  `RecursionError` instead of raw tuples.

### Changed

- **Error code catalogue** (`docs/ERRORS.md`): E319 documentation corrected
  (emitted by mutability checker, not rule checker); E320 documented as
  parser-level SyntaxError; W101/W102 added to removed codes table.
- **`cli.py` recursion errors**: use `ErrorCode.FATAL` constant instead of
  hard-coded `"[E999]"` strings.
- **Version bumped to `0.2.0a1`**.

### Tests

- 11,348 tests passing (up from 7,196 in the previous session).
- 550 integration tests, 1,000 OOP tests, 100 examples — all clean.
- Zero regressions across all phases.

### Added

- **`stdlib.regex` parity with Python's `re`**: `subn`, `group_dict`, `group`,
  `replace_fn` (callable), a lazy `find_iter`, `purge`, and the `UNICODE`
  flag, plus documented behavior for group tuples and flags. Covered by
  `tests/test_regex_phase0.py` and a Hypothesis parity/fuzz suite.

### Fixed

- **Identifiers now follow Python's own rules** (PEP 3131 XID_Start /
  XID_Continue) instead of `str.isalpha()`/`isalnum()`. The old test accepted
  22 codepoints Python rejects as an identifier start (Arabic ligatures such
  as U+FC5E), which would emit invalid Python, and stopped short of valid
  continuation characters (U+2118, U+212E, ZWJ U+200D). Stress and property
  tests in `tests/test_phase0_fuzz.py` and `tests/test_unicode_phase0.py`.

## [0.1.0a20] - 2026-09-18

Diagnostics and documentation alignment.

### Added

- **A pointed diagnostic for a parenthesised base class**: `class Dog(Animal)`
  now reports that it looks like a base class and suggests `extends`, instead
  of the generic header-field message.

### Changed

- **The precedence table in `docs/LANGUAGE.md`** now matches the parser
  (previously the shift/bitwise order, coalescing vs range, and pipe/assignment
  levels were wrong). `docs/GRAMMAR.md` documents that `yield` is a
  statement-level form whose operand is parsed at the lowest precedence.

## [0.1.0a19] - 2026-09-18

Post-release audit fixes found while reviewing the a18 batch.

### Fixed — standard library security

- **Credentials no longer follow a cross-origin redirect**: `Authorization`,
  `Proxy-Authorization` and `Cookie` headers are dropped when a redirect
  leaves the original scheme/host/port, so an open redirect cannot leak them.
- **`stdlib.asyncio.get_event_loop()`** no longer emits a deprecation warning
  or raises on Python 3.12+: it returns the running loop, a loop already
  installed on the thread, or a fresh installed loop.

### Tests

- Extended `tests/test_audit_round2.py` with the redirect credential rule, the
  same-origin helper and the event-loop accessor.

## [0.1.0a18] - 2026-09-18

Second audit round. Fixed the remaining silent miscompilations in the parser
and transformers, closed two standard-library security gaps, hardened the
post-quantum fallback, and removed redundant AST traversals that dominated
compilation time on large sources.

### Fixed — parser and codegen

- **`yield` followed by a binary expression** (`yield x + 1`) bound the value
  too tightly and emitted `(yield x) + 1`; it now yields the whole expression,
  matching Python. A tuple yield (`yield a, b`) yields `(a, b)`.
- **f-string conversions** were dropped: `f"{v!r}"` and `f"{v!s}"` emitted
  `{v}`. The `!r`/`!s`/`!a` conversion and the `:spec` tail are now parsed and
  preserved.
- **An invalid expression inside an f-string** (`f"{1+}"`) emitted invalid
  Python; it is now a positioned diagnostic.
- **Typed destructuring** (`let (a, b: Int) = ...`) emitted invalid Python
  (`(a, b : Int) = ...`); it is now rejected with a clear error, while dict
  alias patterns (`let {name: n} = ...`) keep working.
- **`?:` and `??`** emitted the left operand twice in the condition, so a
  side-effecting or expensive value was evaluated twice and chained operators
  quadrupled work. Both now go through single-evaluation prelude helpers
  (`_aura_elvis`, `_aura_null_coalesce`).

### Fixed — standard library security

- **SSRF guard bypass**: `http://:50587/` has no hostname, and the blocked-host
  check was skipped when the hostname was missing. It now runs unconditionally
  and fails closed.
- **Carrier-grade NAT** (`100.64.0.0/10`, RFC 6598) is now treated as blocked;
  Python's `ipaddress` does not mark it private.
- **The pure-Python post-quantum reference backend** now emits a one-time
  `RuntimeWarning` when used, so a program cannot silently rely on a
  non-production backend whose signatures are forgeable.
- **`AuraDict`** no longer shadows a data key with an inherited method: a
  mapping with a `keys`/`values`/`items` key returns the value on attribute
  access while the method form still works when no such key exists.

### Changed — performance

- The rule checker's four whole-program traversals were fused into one, and
  the type checker's declaration collection into another, cutting compilation
  time on a 465 KB / 18,400-line file from ~580 ms to ~465 ms (~20%) without
  changing diagnostics.
- The operator-precedence table is a module constant instead of a dict rebuilt
  on every operator token.

### Tests

- Added `tests/test_audit_round2.py` covering `yield` grouping, f-string
  conversions and validation, destructuring, single-evaluation coalescing, the
  SSRF and CGNAT blocks, the reference-backend warning and `AuraDict`
  collisions.

## [0.1.0a17] - 2026-09-18

Internal audit hardening. A full pass over the tokenizer, parser, transformers,
type checker and developer tooling fixed silent miscompilations, crashes on
malformed input and several data-loss bugs, and made compilation of deeply
nested inputs robust instead of crashing with a Python traceback.

### Fixed — parser and tokens

- **Empty control-flow bodies** (`if x { }`, `while c { }`, `for x in xs { }`,
  `loop { }`, `try { } catch { }`) produced invalid Python with no suite after
  the `:`. `_block` now emits `pass`, so an empty Aura body is valid output.
- **`try { } finally { }` dropped the `finally` block**, because an empty
  `finally_body` is falsy; the check is now `is not None`.
- **Uppercase-identifier conditions** (`if Point { ... }`, `for x in Items { }`,
  `guard Ready else { ... }`) were parsed as struct initializations and
  swallowed the block. A new `_no_struct_depth` guard disables the struct
  heuristic while parsing control-flow conditions.
- **Keyword and literal binding names** (`let yield = ...`, `let True = ...`)
  now raise a clear diagnostic naming the keyword, instead of a cryptic
  downstream `SyntaxError`. Python-shaped `True`/`False`/`None` point at Aura's
  `true`/`false`/`none`.
- **Unterminated string and block-comment literals** now raise a positioned
  `SyntaxError` instead of being silently accepted or crashing.
- **Raw strings** with an escaped quote delimiter (`r"a\"b"`) are scanned
  correctly.
- **Perceptual positions** for float and prefixed-string tokens now point at
  the token start rather than the end.

### Fixed — developer tooling

- **`aura add`/`remove` no longer destroy the manifest**: `aura.toml` unknown
  tables (`[tool.*]`, `[scripts]`, ...), inline tables and arrays are preserved
  on rewrite.
- **`aura add -V` validates the specifier**: values such as `--upgrade` or
  `,--no-deps` are rejected instead of being written into the manifest and
  pip.
- **The LSP server survives malformed input**: a bad `Content-Length`, invalid
  UTF-8 or malformed JSON now produces a JSON-RPC parse error (`-32700`)
  instead of crashing the process. Requests before `initialize` receive the
  spec-mandated `-32002`.
- **The formatter preserves multi-line strings** verbatim and no longer
  corrupts spread/varargs or unary expressions (`f(*a, **b)`, `a * -1`).
- **`aura version <x>` and `aura debug <missing>`** report a clean error
  instead of a traceback; `set_version` only rewrites the `version` key inside
  `[project]`.
- **`aura run file.aura -v`** now enables verbose output instead of forwarding
  `-v` to the program; trailing tokens still reach `main(args)`.
- **`aura format -i`** rewrites a file in place, matching the documented
  behaviour.
- **`aura doctor`/`deps`/`lock`** query installed versions in one `pip list`
  call instead of one `pip show` per dependency.
- **Lint naming checks** handle `let mut NAME` and no longer misreport tuple
  destructuring targets.
- **REPL continuation** ignores brackets inside `//` and `/* */` comments.
- **`AURA_VENV`** overrides that point at a filesystem root or the home
  directory are refused before any `rmtree`.
- **CLI startup** no longer imports the compiler for lightweight commands
  (`version`, `doctor`, `deps`, `venv`), cutting their start-up cost.

### Fixed — recursion

- Long operator chains and member chains no longer crash the type checker,
  rule checker, mutability checker or transformer with a raw `RecursionError`.
  A `recursion_budget` context manager scales the interpreter limit to the
  source size and restores it afterwards; `_prepare_entrypoint` walks the AST
  iteratively; residual overflow is reported as a clean `E999` diagnostic.

### Tests

- Added coverage for the empty-body, `try`/`finally`, condition-parsing,
  binding-name, manifest round-trip, specifier-validation, LSP-protocol,
  formatter and recursion fixes.

## [0.1.0a16] - 2026-09-18

Module facades. A module can now be the entry point of a source folder and
re-export the classes and helpers defined in its sibling files, so a folder of
`.aura` files is a single namespaced package from the outside.

### Added

- **Facade re-exports.** A bare `export Name` inside a module (no
  `def`/`class`) re-exports a symbol defined in a sibling source file:

  ```text
  App/
    App.aura          module App { export Components, Utils }
    components.aura   class Components { ... }
    utils.aura        def double(...) / const VERSION
  ```

  ```aura
  import App
  App.Components("header")   // the class from components.aura
  App.Utils.double(21)       // the utils module as a namespace
  ```

  Resolution order: a declaration in the same file, then a sibling file whose
  name matches case-insensitively, then a subfolder named after the symbol
  (`Components/Components.aura` or `Components/__init__.aura`). When the file
  declares the name the binding is that declaration; otherwise the whole
  sibling module becomes the namespace.

- **Explicit re-export source**: `export Widgets from "widgets"` or
  `export X from "pkg.sub"`. The path must be a plain dotted name — separators,
  `..` and absolute paths are rejected, and resolution is confined to the
  facade's folder.

- **`App/App.aura` is a package entry point.** A folder is importable as a
  package when it has `__init__.aura` *or* a file named after the folder. A
  module named `App` in a folder named `App` backs the package directly, so
  `import App` gives `App.Components` without an extra namespace level.

- Multiple names per statement: `export Components, Utils`.

### Changed

- **`E312`: `main` inside a `module` body is an error.** A module is a library
  namespace and the runtime only calls the entry file's top-level `main`, so a
  module's `main` would never run. A top-level `main` in an imported file stays
  allowed (it is simply never called).

- **`E313`: an unresolved re-export is reported by `aura check`**, not only at
  transpile time, with a hint naming the expected sibling file.

- **Imported files are checked with the same rules as the entry file.** The
  import hook previously ran only the mutability checker; it now also runs the
  rule checker, so an invalid import fails loudly at the import site.

- **The import finder confines every resolution to its search roots.** A
  crafted module name (traversal components, empty names, or a symlink pointing
  outside) resolves to nothing instead of building an out-of-tree path, and a
  malformed name no longer raises.

- `docs/LANGUAGE.md`, `docs/GRAMMAR.md`, `docs/ERRORS.md` and
  `docs/COMPLETENESS.md` document the facade syntax; a stale top-level
  `export def` example in the import section was corrected (`export` outside a
  module body is not valid).

### Tests

- `tests/test_module_facade.py` (54 tests): re-export parsing (single, multiple,
  explicit source, mixed with declarations, rejection of traversal paths),
  convention resolution (case-insensitivity, subfolders, `__init__.aura`,
  explicit dotted sources), generated Python (hoisted imports, no nested class
  for a package facade, valid Python), end-to-end programs (class and module
  re-exports, explicit source, local declaration, mixed, facade class used as a
  base, re-import idempotency), diagnostics (`E313`, `E312`, `E308` after a
  re-export), imported-file rule enforcement, and import-hook security
  (traversal, symlinks, stdlib names).

### Added — structural rule enforcement (`E314`–`E321`)

A pass of checks for programs that previously failed only at runtime with a
bare Python error. Each is reported by `aura check`, `aura run` and the LSP,
with a coded message, a source location and a hint.

- **`E314` `UNKNOWN_BASE_CLASS`** — `extends` names a base that is neither a
  declared class/trait nor a builtin exception root. Forward references and
  dotted bases are resolved correctly, so only a genuinely undefined base is
  reported.
- **`E315` `INVALID_INHERITANCE`** — a base listed more than once, a class
  extending itself, or a circular `extends` chain (the message names the
  cycle).
- **`E316` `INSTANTIATE_ABSTRACT`** — instantiating a trait, or a class that
  still has an inherited abstract method without an implementation; the message
  names the missing method(s).
- **`E317` `SELF_IN_STATIC`** — `self`/`cls` used inside a `static` method,
  where neither is bound.
- **`E318` `UNKNOWN_LABEL`** — `break label` / `continue label` where no
  enclosing loop carries that label.
- **`E319` `USED_BEFORE_DECLARED`** — a function-local read before the
  `let`/`const` that declares it later in the same body (the case that would
  otherwise be a Python `UnboundLocalError`). Enclosing-scope, module-level and
  imported names are never reported; a `for` target is scoped to its loop.
- **`E320` `DECORATOR_ON_FIELD`** — a decorator applied to a class/trait field
  (it would silently do nothing); rejected while parsing.
- **`E321` `ABSTRACT_SUPER_CALL`** — `super.m()` where `m` is abstract in the
  parent and has no implementation to call.

`docs/ERRORS.md` lists all eight codes; `docs/LANGUAGE.md` and
`docs/GRAMMAR.md` document the corresponding behaviour at the relevant
sections.

### Fixed — class and trait fields

- **`const` inside a class or trait body is no longer parsed as a field named
  `const`.** `class C { const K = 1 }` previously produced two fields (`const`
  and `K`); it is now a single class-level constant, emitted on the class and
  never as an instance field or constructor parameter. A constant requires a
  value; omitting it is a clear parse error.
- **`const` is immutable through member access.** `C.K = 2` (and `self.K = 2`)
  on a constant is now `E303`; member assignment previously bypassed the
  mutability checker entirely.
- `static const` no longer emits a stray `const = None` class attribute.
- A decorator on a field (class or trait, body or header) is `E320` instead of
  being silently dropped.

- `tests/test_rule_gaps.py` (51 tests): one rejection test and one near-miss
  acceptance test per new code, runtime agreement for abstract instantiation
  and use-before-declaration, plus a no-false-positive sweep over every
  `examples/**/*.aura` file through both the rule and mutability checkers.

## [0.1.0a15] - 2026-09-18

Project-environment tooling. An Aura project can now manage its own virtual
environment and dependencies without leaving the `aura` command.

### Added

- **`aura venv`** manages the project's virtual environment (`.venv` by
  default, or `$AURA_VENV`):
  - `init` creates it and installs the declared dependencies;
  - `info` prints the project, the interpreter, and each dependency's status;
  - `shell` prints the activation command for the current platform;
  - `remove` deletes it (with confirmation);
  - `--force` recreates, `--python <exe>` picks the base interpreter, and
    `--no-install` skips dependency installation.
- **`aura add -D/--dev`** records a dependency under `[dependencies.dev]`
  instead of `[dependencies]`. Dependencies may carry specifiers and extras
  (`aura add "requests[security]>=2"`), and a version defaults to `==` when
  passed with `-V`.
- **`aura remove <pkg>`** deletes a declared dependency (runtime or dev);
  `--uninstall` also removes it from the environment.
- **`aura doctor`** checks the Python version, the manifest, the virtual
  environment, and whether every declared dependency is installed, exiting
  non-zero when something needs attention.
- **`aura deps --lock`** writes `aura.lock` with the exact installed versions,
  separated into `[runtime]` and `[dev]`.
- **`aura init --venv`** creates the environment as part of scaffolding.
- Dependencies install into the project's `.venv` when it exists and into the
  current interpreter otherwise, so a project is self-contained once its venv
  exists and works out of the box before that.
- Colored, consistent output for the project commands (`✓`/`•` markers,
  dimmed commands), disabled by `NO_COLOR` and forced by `AURA_FORCE_COLOR`.

### Changed

- `aura install` installs runtime and dev dependencies and prints a summary;
  it no longer prints `$ python -m pip` without context.
- `aura deps` lists dependencies grouped into runtime and dev, each with the
  installed version when present.
- Dependency specifiers are validated before use: a manifest entry that could
  smuggle a pip option or shell metacharacter is refused, and bare versions
  (`flask = "3.0"`) are pinned with `==` at install time.
- `aura/cli.py` exposes `build_parser()` so the command surface can be
  inspected and tested without executing anything.

### Tests

- `tests/test_deps_venv.py` (91 tests): manifest round-trips, `add`/`remove`
  with runtime and dev groups, specifier and injection rejection, real venv
  creation/inspection/removal, a stubbed installer that asserts the target
  interpreter, lock-file contents, `doctor` outcomes, and CLI wiring for every
  command.

## [0.1.0a14] - 2026-09-18

A module-system release. `module { }` members are now private to their file by
default and become public only through `export`, so a module is a real
encapsulation boundary instead of a namespaced class with an ignored marker.

### Added

- **`export` controls visibility inside `module { }`.** A member declared in a
  module body is private to the declaring file unless prefixed with `export`:

  ```aura
  module MyLib {
    export def public_function() -> int { return 42 }
    export const VERSION = "1.0.0"

    let mut cache = 0                 // private
    def private_helper() -> int { return cache }

    export def refresh() -> int {     // may use private members
      cache = private_helper() + 1
      return cache
    }
  }
  ```

  Every member kind can be exported: `def`, `class`, `trait`, `enum`, `type`,
  `let`, `const` and nested `module`.

- Accessing a non-exported member from outside reports `E308`
  (`'<name>' is not exported from module '<M>'`, with an "add `export`" hint).
  The member is also emitted under a mangled runtime name (`_MyLib__cache`), so
  the boundary holds at runtime as well.

- `Module.exports` in the AST records the exported names, and every module
  member carries `is_exported` (defaulted on `Node`).

### Changed

- **Module state is not writable from outside.** `M.count = 9` reports `E303`;
  mutate module state through an exported function. Assignment inside a module
  function is unchanged.
- A bare `export` (not followed by a declaration) is a syntax error instead of
  being silently ignored, and `export` outside a module body is rejected.
- `docs/LANGUAGE.md`, `docs/GRAMMAR.md`, `docs/ERRORS.md` and
  `docs/COMPLETENESS.md` document the module rules; the grammar now models
  `export` as module-specific rather than a general modifier.

### Fixed

- **An internal call to a private module member emitted an undefined name.** A
  non-exported member is now renamed at its declaration and at every internal
  reference (`M._MyLib__helper()`), instead of only at the declaration.
- **A local that shadowed a module member was renamed.** The declaration rename
  is scoped to the module body's own indent level, so `let K = 100` inside a
  module function keeps its binding while `K` still resolves the module member.

### Tests

- `tests/test_modules.py` (34 tests): export parsing for every member kind,
  mangling and internal-reference codegen, runtime accessibility, `E308`
  (external access) and `E303` (external assignment), duplicates, shadowing,
  nested and dotted modules, and cross-file imports.
- The generated `integration_tests` corpus (1000 files) and its generator were
  updated to export the class they use from outside the module.

## [0.1.0a13] - 2026-09-17

An OOP release with a full-codebase clean-up. Inheritance is now spelled with
`extends` only, and a class can declare its fields in its header, which drives
the constructor and generates accessors automatically. The audit also removed
dead code, tightened syntax, made the diagnostics catalogue honest, brought the
REPL to parity with `aura check`, and organised the examples and documentation.

### Added

- **Class header fields.** A class may declare its fields in the header:

  ```aura
  class User(private name: str, mut age: int = 0, public id: int = 0) { }
  ```

  Each field becomes an instance field, a constructor parameter, and a
  getter. A `mut` field also gets a setter; an immutable field gets only a
  getter. Visibility defaults to `private` and may be set per field
  (`public`/`protected`/`private`). Fields may declare defaults, and a field
  without a default may not follow one with a default. Every field needs a
  type annotation or a default.

- With a base class, the subclass header declares only its **own** fields and
  inherited fields are passed by name; the generated constructor forwards the
  rest to `super().__init__(**kwargs)`:

  ```aura
  class Admin extends User(email: str) { }

  let a = Admin(email: "a@x.com", name: "bob")
  ```

- A manual `def new(...)` wins over the generated constructor, while the
  accessors are still generated. A method you declare with the same name as an
  accessor wins over the generated one.

- `ClassDecl.header_fields` in the AST carries `(Parameter, visibility, mutable)`
  per header field.

- `aura test` runs each file with `--no-main`, so a `.aura` test file may drive
  itself (`stdlib.testing` + `t.run_all()`) without declaring `main`.

- `aura lint --allow-warnings` (documented, previously missing): reports style
  issues but exits 0.

- `tests/test_examples.py` guards the `examples/` tree: every file is parsed,
  rule-checked, type-checked, transpiled to valid Python, and executed, and the
  examples README must list every file.

### Changed

- **Inheritance is `extends` only.** The parenthesised base form
  (`class Dog(Animal)`) and `implements` are no longer Aura. Both now produce a
  pointed syntax error suggesting `extends`:
  - `class Dog(Animal)` — `(` after a class name introduces header fields, so a
    bare name there is an error;
  - `class Dog implements Trait` — reported as "not Aura; extend with
    `extends`".
  Traits extend traits with `extends` too.
- **Class and trait `let` fields are immutable by default**, matching `let` at
  module and local scope. Use `let mut` (or `mut`) for a field that changes.
  Consequently an immutable field no longer receives a setter.
- **Accessors are generated for every instance field**, not only for
  `private`/`protected` ones: a getter always, and a setter only for a mutable
  field. This gives public fields a stable API without exposing storage.
- **Type arguments use brackets only.** `List<T>` in a type annotation is now a
  pointed syntax error (write `List[T]`), matching the rule for type
  parameters. Previously the two spellings were both accepted.
- **The REPL runs the full checker set.** `aura repl` now runs the
  `TypeChecker` in addition to the structural-rule and mutability checkers, so a
  type error (`E101`), wrong-arity call (`E105`), or incompatible operand
  (`E108`) is reported instead of executed. The entry-point rule (`main`) stays
  off, because a REPL chunk is a fragment. The module docstring no longer claims
  output identical to `aura run`.
- `docs/README.md` and `README.md` were reorganised with badges and a complete,
  link-checked documentation index.
- `examples/README.md` was rewritten: a numbered learning path, the full AUP
  table, corrected style notes, and no removed syntax.
- The exception hierarchy (`extends Error`), custom errors, and typed `catch`
  are unchanged; only the parenthesised spelling was removed.
- `docs/LANGUAGE.md`, `docs/LANGUAGE_PT.md`, `docs/GRAMMAR.md`, `docs/TYPES.md`,
  `docs/TYPES_PT.md`, `docs/AUP.md` and `docs/COMPLETENESS.md` were updated for
  the new syntax, accessor rules, and the corrected diagnostics catalogue.

### Fixed

- **A block lambda inside a labeled loop produced a `NameError`.** The
  `_labeled_loop` transformer collected hoisted helper functions and then
  deleted them without emitting the `def`, so any block lambda or block
  expression inside `label: for/while/until/loop` referenced an undefined
  `_aura_lambda_N`. It now emits the helpers in place, exactly as a plain block
  does.
- **`_prepare_entrypoint` could double-wrap `await`.** Rewriting top-level calls
  to async functions was not idempotent; on a reused `Program` (the REPL) an
  already-awaited call could be wrapped again. The rewrite now skips a call that
  is already awaited.
- **Inherited visibility was dropped for multiple inheritance.** The
  transformer's visibility map was keyed on the whole comma-joined base string
  (`"A, B"`), so only a single bare base ever matched and inherited `protected`
  members were emitted unmangled (a runtime `AttributeError`). The base string
  is now split, so every base's members are inherited.
- **`__match_args__` used the raw Aura field name.** For a `private`/`protected`
  field the emitted tuple named an attribute that does not exist; it now uses
  the mangled runtime name, so positional pattern matching works.
- **A subclass could bypass visibility with `super`.** `super.member` did not
  resolve to a class, so a `private` parent member read through `super` was not
  reported. `super` now resolves to the first base and is treated as internal
  access: `protected` is allowed, `private` is rejected with `E308`.
- **`const` members were exempt from the missing-visibility rule.** A class or
  trait `const` without a modifier now reports `E307`, as documented.
- **`ClassType` inheritance was dead code.** `parent` was never assigned and
  `infer` never produced a `ClassType`, so `get_field_type`/`get_method_type`
  never ran. `ClassType` now holds a `bases` list (multiple inheritance),
  bases are linked in a second pass (so a later-declared base still resolves),
  and constructor/method calls are checked through the checker's context.
  Inherited method arity is now enforced (`E105`).
- The type checker now resets `classes`, `functions`, `context` and `_enum_typed`
  per program, so a reused checker (the REPL) no longer carries stale class
  types across chunks.
- A header field and a body field with the same name now report `E301` instead
  of silently generating two assignments.
- `aura version <invalid>` now reports a clean error and exits 2 instead of
  raising an uncaught `ValueError`.

### Removed

- **Dead AST nodes and code paths**: `ElvisExpr` (the `?:` operator is handled
  as a coalescing `BinaryOp`, so the node was never constructed),
  `Import` (superseded by `ImportStmt`), `Transformer._transform_legacy`, and
  `ClassDecl.is_static`/`is_volatile` (parsed but never emitted or checked).
- **Diagnostic codes that were documented but never emitted**, now listed in
  `docs/ERRORS.md` under "Removed codes" so the catalogue matches the code:
  `E001`–`E003` (the parser raises a structured `SyntaxError` with
  line/column instead), `E102`–`E104`/`E107` (gradual typing defers unknown
  names to runtime), `E401`/`E402` (the CLI reports plain messages with a
  non-zero exit), and `W101`/`W102` (unused variable/import — removed rather
  than shipped as a half-working analysis).
- `TypeInference`'s single-`parent` inheritance model (`ClassType.parent`),
  replaced by `ClassType.bases`.
- The `implements` and parenthesised-base grammar productions for classes and
  traits, and the `<T>` type-argument spelling.
- The `examples/tests/` and `examples/aup/tests/` empty directory scaffolding
  (32 empty directories).
- `session_backup.txt` is gitignored; it is a working transcript, not project
  documentation.

### Tests

- `tests/test_oop_header.py` (59 tests): header grammar, defaults, visibility,
  mutability, accessors, inheritance, diagnostics, and regressions for the
  bugs fixed here.
- `tests/test_examples.py` (88 cases): parses, rule-checks, type-checks,
  transpiles, and runs every example, and asserts the examples README lists
  every file.
- The OOP and language corpora (`tests/oop_tests`, `tests/success_tests_aura`,
  `examples/`) and the embedded Aura sources in the test suite were migrated
  from the parenthesised/inherits forms to `extends`.

## [0.1.0a12] - 2026-09-17

A feature release across the language, type checker, LSP, standard library and
tooling. It adds generic constraints, custom exception hierarchies, enum-member
pattern matching, match-exhaustiveness analysis, an assertions/matchers module,
native async file and HTTP helpers, and a full set of editor navigation
features. It also hardens the HTTP SSRF guard and fixes several declaration and
matching bugs.

### Added

#### Language

- **Custom exception hierarchies.** The exception root `Error` needs no import
  and is aliased to Python's `Exception`. A class may extend it with either
  spelling — `class MyError extends Error { ... }` or
  `class MyError(Error) { ... }` — and `super(message)` now maps to
  `super().__init__(message)` (previously `super(msg)` was emitted verbatim and
  failed at runtime with "super() argument 1 must be a type"). Multi-level
  hierarchies work: `catch NotFoundError`, then `catch AppError`, then
  `catch Error`, matched in order.

- **Generic constraints.** A type parameter may declare a constraint after a
  colon: `def smallest[T: Comparable](items: [T]) -> T`, on functions, classes
  and traits. The constraint must be a builtin type or a class/trait declared
  anywhere in the same program; a union (`[T: int | str]`) is accepted. An
  unknown constraint is reported as `E110` with a hint. Constraints are
  compile-time only and do not change the emitted Python.

- **Enum-member pattern matching.** A `case` may name an enum member with a
  dotted pattern (`case Color.RED`) so the case compares against the constant
  instead of binding a new variable. A bare identifier (`case RED`) remains a
  binding pattern, matching the documented `case n if ...` semantics.

- **Match-exhaustiveness analysis (`E109`).** A `match` over `bool` (both
  literals), an enum value (all members), or a scalar (`int`/`str`) must handle
  every case or provide a catch-all. A missing fallback is reported as `E109`,
  a *warning*, so it never fails a build. A guarded wildcard (`case _ if cond`)
  is correctly not treated as a catch-all, while `case name` is.

#### Standard library

- **`stdlib.testing`**: assertions and matchers for Aura test files, including
  `test`, `equal`, `not_equal`, `is_true`/`is_false`, `is_none`/`is_not_none`,
  `contains`/`not_contains`, `starts_with`/`ends_with`, `greater`/`less` and
  the `_or_equal` variants, `has_length`, `is_empty`, `in_range`,
  `approx_equal`, `raises`, `fail`, `skip`, `case`, `run_all`, and the
  `TestFailure` type. `aura test` now surfaces the `stdlib.testing` failure
  summary (`N/M passed, K failed` plus `FAIL <name>` lines) instead of the
  generic last traceback line.

- **Native async I/O**: `stdlib.io` gains `read_async`, `write_async`,
  `append_async`, `exists_async`, `is_file_async`, `is_dir_async`,
  `mkdir_async`, `ls_async`, `rm_async`, `rename_async`, `read_lines_async`,
  `write_lines_async`, `copy_async`, `size_async` and `touch_async`;
  `stdlib.http` gains `arequest`, `aget`, `apost`, `aput`, `adelete`,
  `aget_json` and `apost_json`. All await the vetted synchronous
  implementation on a worker thread, so errors and (for HTTP) security checks
  are identical. `read_async`/`write_async` are re-exported from `aura.stdlib`.

#### Tooling

- **LSP navigation and formatting.** `aura lsp` now advertises and implements
  `textDocument/definition`, `textDocument/references`,
  `textDocument/prepareRename`, `textDocument/rename` and
  `textDocument/formatting`, in addition to diagnostics, hover, completion and
  document symbols. Definition resolves functions, classes, enums, traits,
  modules, variables, constants and parameters; references and rename operate
  on every identifier token; formatting replaces the document with
  `format_aura` output. All features degrade cleanly on unparseable documents.

#### Tests

- `tests/test_custom_errors.py` (15 tests): both inheritance spellings,
  `super(message)`, multi-level catches, root `catch Error`, `finally`, and an
  uncaught-error diagnostic.
- `tests/test_type_features.py` (26 tests): `E110` constraints (function,
  class, trait, unions, forward-referenced classes) and `E109` exhaustiveness
  (bool, enum, scalars, guards, fallbacks, gradual-typing opt-out), plus
  enum-member pattern runtime behaviour.
- `tests/test_lsp_navigation.py` (24 tests): capabilities, definition,
  references, prepare/rename, formatting, and robustness on broken documents.
- `tests/test_async_io.py` (18 tests): every `*_async` IO helper, the async
  HTTP security guards, and end-to-end Aura `async def` programs.
- `tests/test_testing_module.py` (35 tests): the public surface and failure
  paths of `stdlib.testing`.
- `tests/test_declaration_fixes.py` (35 tests): class/trait `const` members,
  `E303` on constant reassignment, and module-scope data access.
- `tests/aura_test_helpers.py` now awaits `async def main`, mirroring
  `aura run`, so async programs are testable through the shared helpers.

### Fixed

- **`case RED { ... }` swallowed the case body.** A capitalized identifier in a
  case pattern was parsed as a struct literal (`RED(**{...})`), producing
  invalid Python. Struct-init is now suppressed while parsing a pattern, so the
  `{` always starts the case body.
- **`const` as a class or trait member.** `class C { const K = 1 }` created a
  stray field literally named `const`; it is now a class-level constant that
  lives on the class, never on an instance, never becomes a constructor
  parameter, and respects `public`/`private`/`protected`/`static` mangling.
- **Assigning to a class-level constant.** `C.K = 2` and `self.K = 2` now
  report `E303` with a source location and a `let mut` hint instead of silently
  succeeding at runtime.
- **Bare module data members inside module functions.** A reference to a
  module-level `const`/`let`/`static` from a function in the same module
  resolved to `NameError`; it now resolves through the module class, while
  locals and parameters correctly shadow it.
- **Formatter operator corruption.** Restoring multi-character operators after
  spacing normalization could re-split `..<` into `.. <`, producing invalid
  code. Spacing is now normalized while the operators are still masked, and
  inline block comments (`/* ... */`) are protected like strings.
- **HTTP SSRF guard failed open on unresolvable hosts.** `_is_blocked_host`
  returned `False` when DNS resolution raised `gaierror`, allowing a hostname
  that could resolve to anything at connect time. It now fails closed (an
  unresolvable name is blocked), classifies IPv4-mapped IPv6 addresses
  (`::ffff:127.0.0.1`) by their embedded IPv4 address, and treats an empty
  address list or unparseable address as blocked. `AURA_HTTP_ALLOW_PRIVATE=1`
  still opts out.

### Changed

- `aura check` surfaces `E109`/`E110` through the normal diagnostics channel;
  `E109` is a warning and does not change the exit code.
- `docs/ERRORS.md` documents `E109` (`NON_EXHAUSTIVE_MATCH`) and `E110`
  (`UNKNOWN_TYPE_CONSTRAINT`); the catalogue-sync test keeps the enum and the
  document aligned.
- `docs/LANGUAGE.md` documents generic constraints, custom exception types,
  enum-member matching with exhaustiveness, and the async standard library.
- `docs/COMPLETENESS.md` raises generics, pattern matching, error handling,
  `test`, `format`, `lsp` and the stdlib dimension to reflect the release.

### Removed

- Working session transcripts (`session_backup.txt`) are ignored rather than
  tracked; they are not project documentation.

## [0.1.0a11] - 2026-09-17

### Fixed

- `aura debug --trace` restored a cleared `sys.settrace` hook instead of the
  previous one, silently disabling coverage (and any other active tracer) for
  the rest of the process on Python ≤ 3.12. It now saves and restores the
  existing trace/profile hooks.

- Identifiers that are Python reserved words (`raise`, `class`, `lambda`,
  `import`, ...) are now emitted with a safe spelling (`raise_`) everywhere
  they appear: local bindings, function/method/constructor parameters,
  keyword-argument calls, member and attribute names, destructuring targets,
  and match patterns. Previously such names produced `SyntaxError` in the
  generated Python (e.g. an auto-generated `__init__` for a field named
  `raise`). `True`, `False` and `None` are left untouched.

- Runtime import aliases now pin each *submodule* (`transpiler.ast`, ...) to
  the already-imported `aura.*` module. Importing an aliased submodule could
  otherwise load a second, distinct module object for the same file, breaking
  `isinstance`/identity checks across the toolchain.
- The debugger now invokes `main` like `aura run` (including `async main` and
  `args`), instead of only defining it.
- REPL `:py <statement>` runs statements instead of failing on the `eval`
  compile outside the fallback path.
- The property tests for generated class fields now assert the *emitted* name
  (`py_safe_name`), so a field named after a Python keyword (e.g. `raise` →
  `raise_`) is checked against the real attribute instead of the raw Aura
  spelling. This closes the regression the reserved-word change introduced in
  `test_property_transpiler.py`.

### Added

- `tests/test_repl_deep.py` (46 tests): deep REPL coverage.
- `tests/test_stdlib_deep_collections_string.py` (71 tests): `collections`
  and `string` standard-library coverage.
- `tests/test_cli_deep.py` (33 tests): in-process CLI coverage.
- `tests/test_stdlib_wrappers.py` (66 tests): public surface of `math`, `os`,
  `io`, `time`, `regex`, `threading`, `asyncio`, `python` and the offline
  helpers of `http`, including env overrides, error paths and async primitives.
- `tests/test_tools_coverage.py` (55 tests): branching behaviour of `deps`,
  `release`, `formatter`, the Aura import hook (`importer`) and the debugger
  run loop, exercised in-process.
- `tests/test_runtime_backend_repl.py` (27 tests): runtime alias
  install/uninstall, crypto backend selection and reference round-trips, and
  REPL edge branches (`:load`/`:run`/`:ast` failures, `_assigned_names`,
  `_first_error`).
- `tests/test_rules_traversal.py` (31 tests): rule-checker traversal of
  modules, match/try/with, nested functions, guard-else, pattern helpers and
  transitive abstract-method resolution.
- `tests/test_statements_deep.py`: deep `StatementTransformer` coverage for
  dict destructuring, modules, patterns and import forms.

### Changed

- The test suite's coverage floor is raised from 65 % to **90 %**; the suite
  currently measures **91.3 %** branch-aware coverage over `aura/`.

### Notes

- `stdlib.asyncio.as_completed` keeps its pass-through contract (a
  synchronously iterable of awaitables, matching `asyncio.as_completed` on
  every supported interpreter); the docstring now spells this out, since the
  async-iterator form only exists on Python 3.13+.
- `tests/test_statements_deep.py` was corrected to the language's actual
  behaviour: pattern fallbacks return a wildcard for non-pattern expression
  nodes, dict patterns emit `AuraDict(...)`, select imports use
  `ImportStmt(items=..., alias=...)`, constructors are declared
  `public def new(...)`.
- `tests/test_property_lexer_parser.py` and `tests/test_property_transpiler.py`
  exclude `volatily` from their identifier strategies: the tokenizer
  deliberately rejects that one spelling as a typo for `volatile`, so it is not
  a valid identifier. This removes a latent Hypothesis flake.

## [0.1.0a10] - 2026-09-17

### Fixed

- `itertools.repeat(value)` without `times` yields an infinite iterator
  instead of raising `TypeError`.
- `asyncio.wait` / `asyncio.as_completed` accept raw coroutines (they wrap them
  in tasks) instead of raising `TypeError`.
- `crypto.hkdf_sha256` rejects output above the RFC 5869 limit (8160 bytes for
  SHA-256) with a clear error instead of an obscure `ValueError`.
- `http`: the `requests` backend now follows redirects while re-validating
  every hop (SSRF guard) and streams the body under the configured size cap,
  matching the urllib backend and the documentation.
- Resolved the top-level `join` shadow: `stdlib.join` is the string join, and
  `stdlib.io.join` is exported as `stdlib.path_join`.

### Changed

- The type checker reports too-few-argument calls (`E105`); default, variadic
  and keyword arguments are respected.
- `stdlib.INF` and `stdlib.NAN` are exported; `crypto_backend` is in
  `stdlib.__all__`.
- Documented the `math` extras and the `python` and `crypto_backend` modules in
  the standard-library reference.

### Removed

- Dead code: unused CLI compatibility wrappers, `parse_value`, `list_macros`,
  `_offset_for_prelude`, `CompilationException`, `format_error_message`,
  `AuraTypeError`, `NeverType`, the `ANY/NONE/INT/FLOAT/STR/BOOL` aliases, the
  REPL `BANNER`, two formatter constants, and the `collections` convenience
  aliases (`aura_dict`, `list_from`, `dict_from`, `set_from`,
  `map_list`/`filter_list`/`reduce_list`/`find_in_list`).

### Added

- `tests/test_extreme_rules.py` (65 tests): every enforced `E##`/`W##` code with
  a boundary near-miss, CLI extremes and catalogue completeness.
- `tests/test_extreme_oop.py` (37 tests): deep inheritance, MRO, properties,
  class/static methods, visibility, abstract contracts, dunder protocols,
  generics, enums, pattern matching and collection edges.
- `tests/test_stdlib_regressions.py` (14 tests): the fixes above.

## [0.1.0a9] - 2026-09-17

### Added

- **Diagnostics standard.** Every diagnostic is now a coded `E##`/`W##` with a
  real `file:line:column`; `docs/ERRORS.md` is the single source of truth and a
  test keeps it in sync with `errors.py`.
  - New `W` namespace: `W001` line-too-long, `W002` trailing-whitespace,
    `W003` naming, `W004` spacing, `W101` unused-variable, `W102`
    unused-import, `W103` unused-type-parameter.
  - `E303 REASSIGN_IMMUTABLE` replaces the uncoded mutability error.
  - Removed 25 codes that were defined but never emitted (`E201`–`E204`,
    `E303`(old), `E304`–`E306`, plus dead syntax/type stubs).
- Property-based tests (Hypothesis) for the lexer, parser and transpiler
  (`tests/test_property_lexer_parser.py`, `tests/test_property_transpiler.py`).
- Differential suite comparing Aura against equivalent Python
  (`tests/test_differential.py`).
- `tests/test_diagnostics.py`, `tests/test_type_diagnostics.py`,
  `tests/test_cli_diagnostics.py`; the Aura Pattern standard is enforced by
  `tests/test_aup.py`.
- `hypothesis` added to the `dev` extra.

### Changed

- Parser errors carry structured `line`/`column`/`filename`; the LSP and CLI
  report real positions instead of defaulting to `1:1`.
- The CLI prints diagnostics uniformly as `file:line:column: SEVERITY [code]`.
- The LSP runs the rule checker too, reports `W` codes with warning severity,
  and attaches the code and range.
- The REPL keeps the diagnostic code instead of dropping it.
- The linter reports style issues under `W00x` codes (it previously misused
  `E004`); coverage floor raised from 60% to 65% (~71% now).

### Fixed

- The lexer accepted Unicode digits (`¹`, `٣`) and then crashed in `int()`;
  number scanning is now ASCII-only.
- A string literal whose content is a keyword (`"yield"`, `"not"`, `"await"`)
  was misparsed as that keyword and produced invalid Python.
- A dict/set literal whose first element was a keyword-like string
  (`{"if": 1}`) was misclassified as a block.

## [0.1.0a8] - 2026-09-17

### Fixed

- **PyPI README links.** All relative links in `README.md` (documentation,
  examples, `LICENSE`, `CONTRIBUTING`, `SECURITY`, `CHANGELOG`) are now
  absolute GitHub URLs, so they resolve on the PyPI project page instead of
  producing `404`s. The stale `0.1.0a5` status line is updated.

## [0.1.0a7] - 2026-09-17

### Added

- **Program entry point.** An entry file run with `aura run` must declare a
  top-level `main`; the runtime invokes it (no trailing `main()` call). `main`
  may take no parameters or a single `args`, may return an `int` exit code, and
  may be `async` (awaited automatically). Missing `main` is `E310`; a wrong
  signature is `E311`. Imported modules need no `main`.
- `aura run` forwards trailing arguments to the program's `main(args)`.
- `tests/test_main_entrypoint.py`: 17 tests for `E310`/`E311`, auto-invocation,
  argument forwarding, async `main`, exit codes and nested-function `nonlocal`.

### Changed

- `examples/` migrated to the `main` entry point.

### Fixed

- A nested `def` that assigns to a captured local of an enclosing function now
  emits `nonlocal`, instead of raising `UnboundLocalError` (which could
  deadlock a lock-guarded thread example).

## [0.1.0a6] - 2026-09-17

### Added

- **Real encapsulation.** Every class/trait member must declare an explicit
  `public`, `private` or `protected` visibility; omitting it is a compile error
  (`E307`).
- `E308`: the rule checker rejects access to a non-public member from outside
  the class, resolving `self`/`cls` and simple `let x = Class(...)` instances.
- Auto-generated `get_<name>()` / `set_<name>(value)` accessors for every
  non-public field (overridable by an explicit method).
- Owner-aware name mangling: `private` → `_<DefiningClass>__name`,
  `protected` → `_name`. A private member keeps its defining class's prefix even
  when referenced from a subclass, fixing the inherited-private lookup.
- `tests/test_encapsulation.py`: 20 tests covering `E307`, `E308`, accessors,
  owner-aware mangling and runtime enforcement.
- **Abstract-method enforcement.** A concrete class that fails to implement an
  abstract method inherited from a trait is rejected at compile time (`E309`),
  instead of only failing when instantiated.
- **Trait inheritance.** Traits may extend other traits via `trait B(A)` or
  `trait B implements A`; abstract methods are inherited transitively.
- `tests/test_oop_abstract.py`: 16 tests covering `E309`, transitive abstract
  inheritance and trait-to-trait inheritance.

### Changed

- `protected` members are accessible in the declaring class **and its
  subclasses**; `private` members only in the declaring class.
- Parser attaches source locations to class/trait members.

### Fixed

- Tokenizer reported every token's column as the position **after** the token
  (e.g. `class` at column 6 instead of 1); now reports the token's start.
- The `tests/*_tests/` corpora and `examples/` were migrated to explicit
  visibility modifiers (816 files).

## [0.1.0a5] - 2026-09-16

### Added

- `docs/GRAMMAR.md`: the canonical EBNF grammar, now the single source of truth
  for Aura's concrete syntax (precedence table, removed spellings, formatting).
- **Concurrency.** `stdlib.threading` (spawn/join, locks, events, semaphores,
  pools) and `stdlib.asyncio` (tasks, gather, queues) with a `global`-scope
  codegen fix so functions can mutate module-level state safely.
- **Post-quantum cryptography.** `stdlib.crypto` with real SHA-3/SHAKE/HMAC/HKDF
  primitives and a pluggable ML-KEM/ML-DSA backend (`pqc` extra), falling back
  to a clearly-labelled non-production reference implementation.
- **Aura Patterns (AUP).** `docs/AUP.md` and ten runnable `examples/aup/*.aura`
  programs covering option, builder, strategy, pipeline, errors, memoization,
  observer, resources, worker pools and hybrid crypto.
- `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `LICENSE`, and
  `.editorconfig`.
- CI lint/type/coverage jobs; `ruff`, `mypy`, `pytest-cov` and `pytest-timeout`
  in the dev extra.

### Changed

- **Standardized syntax.** One spelling per construct; redundant aliases now
  raise a clear `SyntaxError`:
  - `fn` → `def`, `init` → `new`
  - `!` → `not`, `&&` → `and`, `||` → `or`
  - `null` → `none`, `volatily` → `volatile`
  - `<T>` generics → `[T]`
  - `let private x` → `private let x`
- Fixed operator precedence: unary `-` now binds looser than `**`
  (`-2 ** 2 == -4`) and `not` binds looser than comparisons
  (`not a in b == not (a in b)`), matching Python.
- `aura lint` exits non-zero when warnings are reported.

### Fixed

- Functions that assign to a module-level binding now emit `global`, fixing
  `UnboundLocalError` (and making shared-state threading possible).
- `guard ... else { return }` inside a method returns from the method instead of
  raising `SystemExit` and killing the program.
- `1...10` no longer silently truncates to `1`; it is a clear error.
- `try` without `catch`/`finally` is a clear error.
- `release.bump` no longer crashes on pre-release versions.
- Latent bug where the AST `UnionType` annotation type shadowed the type
  system's `UnionType`.
- Deeply nested or oversized source raises a clean `SyntaxError` instead of a
  `RecursionError` traceback.

### Security

- HTTP: redirects are re-validated at every hop (SSRF), and response bodies are
  size-capped (`AURA_HTTP_MAX_BYTES`).
- `aura install` validates dependency names and rejects pip-option specifiers.
- `aura_ide` resolves language-feature paths through the workspace sandbox and
  escapes the project name against TOML injection.
- LSP bounds open documents and caches.
- `errors.ErrorCollector` now bounds its collection after `max_errors`.

### Performance

- Removed a redundant full AST walk (`Transformer._scan_ast`); prelude needs
  are recorded during transformation.
- Type checker imports AST nodes once at module scope instead of per node.
- LSP caches diagnostics per document version.
- Constant class-body assembly (list + join) instead of repeated concatenation.

### Removed

- `tests/collections_tests/` (generated against an obsolete stdlib API and not
  a reliable oracle).

## [0.1.0a4] - 2026-09-16

A cleanup, hardening and completeness pass: dead weight removed, security bugs
fixed, performance hot spots eliminated, the object system completed, and a
full syntax + OOP test suite added.

### Added

- **Complete object system.** `SPECIAL_METHOD_NAMES` in
  `aura/transpiler/ast.py` maps readable Aura method names to Python protocols
  (`str`→`__str__`, `len`→`__len__`, `eq`/`lt`/…, `iter`, `getitem`,
  `contains`, `call`, arithmetic operators, context managers, …), applied at
  both definition and call sites. `init` is now an alias for `new`.
- **Traits compile to real ABCs** (`abc.ABC` + `@abstractmethod`), so missing
  implementations fail at instantiation time instead of call time.
- **Generic classes and traits** (`class Box[T]`, `trait Container[T]`)
  transpile to `Generic[...]`.
- `yield` (with and without a value) is now supported.
- `fn` is accepted as an alias for `def` at statement level.
- `&&`/`||` and `is not` are accepted as operator spellings.
- New test suites: `tests/test_syntax_complete.py`, `tests/test_oop_complete.py`,
  `tests/test_security.py`, `tests/test_stdlib_coverage.py`, and
  `tests/test_aura_corpora.py` (runs the 5000-file `.aura` corpora).

### Changed

- Auto-generated constructors use a module-level sentinel so an explicit
  `None` can override a field default.
- `aura lint` now exits non-zero when warnings are reported.
- `aura-ide` workspace resolution compares against a path separator boundary
  (path-traversal fix).

### Fixed

- `!x` transpiles to `not x` instead of invalid `(! x)`.
- `try { … }` with no `catch`/`finally` is a clear syntax error.
- Empty function, method and trait bodies emit `pass`.
- Static fields are always emitted on the class; trait fields support
  `let`/`mut`, type annotations and defaults.
- Private protocol methods are no longer double-mangled (`__str__` not
  `____str__`).

### Security

- `aura add` rejects package names with newlines/quotes (TOML injection) and
  escapes control characters in written values.
- HTTP rejects loopback/link-local/private hosts (SSRF) unless
  `AURA_HTTP_ALLOW_PRIVATE=1`; non-HTTP(S) schemes were already blocked.
- `json.dump` rejects NaN/Infinity, matching `dumps`/`pretty`.
- `os.env(allowlist)` can return only selected variables.
- The LSP bounds incoming message size.

### Removed

- The unused ANTLR-generated parser, `.antlr` Java artifacts and grammar files
  (~1.3 MB), the `transpiler/types.py.bak` snapshot, the empty `src/` package,
  byte-identical duplicate `stdlib/` and `tools/` modules, and assorted dead
  code and unused imports.

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