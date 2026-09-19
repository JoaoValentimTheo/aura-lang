---
layout: default
title: "Aura Audit Report"
nav_order: 8
---

# Aura Audit Report

> **Historical document.** This report reflects an earlier state of the
> project. It references files that have since been removed (the ANTLR grammar
> `parser/aura.g4` and generated parser) and pre-0.1.0a5 paths. The current
> grammar lives in [language-reference/grammar.md](language-reference/grammar.md) and the code in `aura/`.

Record of the code audit performed against the documentation. Every item
below was verified against the ANTLR grammar (`parser/aura.g4`), the
recursive-descent parser (`parser/to_ast.py`), the transformer
(`transpiler/`), and the standard library (`stdlib/`).

Run the checks with:

```bash
python3 -m pytest tests/ -v
python3 tests/test_runtime.py
python3 tests/test_regressions.py
```

---

## Bugs fixed

### Parser

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 1 | `unless` statement not parsed | Treated as a function call; runtime `NameError` | Added `parse_unless_stmt` |
| 2 | `until` statement not parsed | Invalid Python generated | Added `parse_until_stmt` |
| 3 | `loop` statement not parsed | Invalid Python generated | Added `parse_loop_stmt` |
| 4 | `??=` operator missing | `SyntaxError` on parse | Tokenized `??=`; handled in `transform_ExprStmt` |
| 5 | `**kwargs` parameter broken | `SyntaxError: Expected IDENT but got '**'` | Distinguish `*` from `**`; emit `**name` |
| 6 | Multiple assignment unsupported | `let a, b = 0, 1` failed | Tuple target + `parse_trailing_tuple` |
| 7 | Multiple return unsupported | `return a, b` failed | `parse_trailing_tuple` in `parse_return_stmt` |
| 8 | Bitwise operators missing | `5 & 3` failed | Added `&`, `|`, `^`, `<<`, `>>` precedence and tokenizer entries |
| 9 | `implements` trait syntax unsupported | `Expected '{' but got 'implements'` | Parse `implements` into base classes |
| 10 | Structural type annotations unsupported | `let u: {name: str}` failed | `parse_type` handles `{...}` |
| 11 | `...rest` destructuring generated `... rest` | Invalid Python | Normalize `...`/`*` spreads |
| 12 | Block comments `/* ... */` unsupported | `Unexpected token '/'` | Tokenizer handles block comments |
| 13 | Type alias scan swallowed next statement | `type X = int` then `print(X)` merged | Structural `_scan_type_tokens` |
| 14 | Nested `module` crashed | `NotImplementedError` inside blocks | `transform_Module` on `StatementTransformer` |

### Transformer

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 15 | Global `private`/`protected` mangled only declarations | `UnboundLocalError` on every reference | Mangle only in class scope |
| 16 | Hoisted block lambdas named `__aura_lambda_N` | Python name-mangling inside classes broke calls | Renamed to `_aura_lambda_N` |
| 17 | Hoisted lambdas emitted at module level | `NameError: name 'self' is not defined` | Emit nested defs in the enclosing block |
| 18 | `for ... step` string surgery | Invalid Python for `range(...)`/iterables | `_iterable_with_step` handles range/range-expr/slice |

### Standard library

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 19 | `stdlib.math.abs/min/max/round` used `__builtins__.x` | `AttributeError` on every call | Import `builtins` module |
| 20 | `list_chunk` was O(n²) | Repeated `list(items)` inside the loop | Convert once, then slice |

### Macros

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 21 | `@deprecated("message")` crashed | `'str' object has no attribute '__name__'` | Support both `@deprecated` and `@deprecated("msg")` |

### Consistency pass

| # | Inconsistency | Resolution |
|---|---------------|------------|
| 22 | `//` was both a line comment and a floor-division operator | Removed floor division entirely; `//` is **always** a comment. Integer division is written `int(a / b)` |
| 23 | `await` only worked as a statement | `await` is now a prefix expression (`let x = await f()`), matching the grammar |
| 24 | Grammar allowed `*`/`**`/`...` spread in call arguments | Implemented call spreads (`f(*args)`, `f(**kwargs)`, `f(...args)`) |
| 25 | `stdlib/README.md` used removed `fn(x) { }` syntax and `import collections::*` | Rewritten with lambdas and `from stdlib.... import ...` |
| 26 | `examples/README.md` claimed `unless`/`until`/`loop`/`guard` were deprecated and used `!` | Corrected: these are first-class statements; negation is `not` |
| 27 | Linter warned about the removed `fn` keyword | Now checks `def` |
| 28 | A bare block `{ print(x); let y = 1 }` was misparsed as a dict/set | Disambiguation: a top-level `:`/`,`/`**`/`for` marks a literal; otherwise it is a block |

---

## Performance improvements

1. **Single AST traversal** – `Transformer._transform_program` used to walk the
   entire AST three times (`_collect_decorators`, `_collect_identifiers`,
   `_has_dict_literal`). These are now one pass in `_scan_ast`.
2. **`list_chunk`** – was quadratic; now linear.
3. **Block/for handling** – removed brittle string splitting for `step`.

---

## Documentation and corpus alignment

* **Standardized the test corpus** – generated `.aura` files used removed
  syntax:
  * 100 `class_*.aura` files used `extends` → migrated to `class Sub(Base)`.
  * 286 `oop_tests/*.aura` files used `extends` and explicit `self` params →
    migrated.
  * 71 `collections_tests/*.aura` files used `if a then b else c` → migrated
    to the ternary `a ? b : c`.
  All 6600 `.aura` files in `tests/` now transpile successfully.
* The generator scripts in `tools/` were updated to emit the standardized
  syntax, so regenerated corpora stay consistent.
* **Fuzz suite** – `tests/test_massive_aura_v2.py` parametrized 500,000 seeds
  (≈40 days of runtime). It now defaults to 200 seeds; set
  `AURA_FUZZ_SEEDS=100000` to run a larger campaign.
* **Type aliases** now emit a real runtime name (`type UserId = int` →
  `UserId = int`).
* **Traits** now emit an inheritable base class instead of a comment.
* **Modules** now emit a namespaced class with static members, so
  `MyLib.public_function()` works.
* **`//` is always a comment**; integer division is `int(a / b)`, which also
  exercises the `as int` / `int(...)` casting path.

---

## Verified counts

| Component | Count |
|-----------|-------|
| AST classes | 83 |
| Type classes | 15 |
| Standard-library functions | 138+ |
| Stdlib modules | 7 (math, string, collections, itertools, json, time, io) |
| Runtime prelude macros | 6 (`debug`, `timeit`, `memoize`, `cache`, `must_return`, `deprecated`) |
| Language decorators | 3 (`property`, `staticmethod`, `classmethod`) |
| Test cases | 4798 + 3000 generated + 200 fuzz seeds + 92 regression tests |

---

## New features implemented (Phase 2)

| Feature | Description |
|---------|-------------|
| **Enum declarations** | `enum Status { Active, Disabled }` with values, patterns, and `enum as _aura_enum` prelude |
| **Match as expression** | `let x = match v { case 1 => "a" case _ => "b" }` (arrow syntax returns value) |
| **Try as expression** | `let x = try { risky() } catch e { 0 }` returns value |
| **Generator expressions** | `(x * x for x in range(10))` in parens and call args |
| **Multiple `if` in comprehensions** | `[x for x in r if x > 0 if x < 10]` |
| **Keyword-only `*` marker** | `def f(a, *, b, c)` splits positional/keyword |
| **Labeled loops** | `outer: for ... { break outer }` via exception-based unwinding |
| **F-strings** | `f"Hello {name}"` with nested expressions, format specs, triple quotes, method aliases |
| **Async runtime** | `aura run` detects `async def` and wraps in `asyncio.run()` |
| **Type checker** | Real `TypeInference` + `TypeChecker` with narrowing, arity, return-type checks |
| **CLI `aura test`** | Run `.aura` test files with pass/fail reporting |
| **Stdlib: json** | `loads`, `dumps`, `load`, `dump`, `pretty`, `parse`, `stringify`, `is_valid`, `merge` |
| **Stdlib: time** | `now`, `now_ms`, `sleep`, `clock`, `monotonic`, `perf_counter`, `strftime`, `iso`, `elapsed` |
| **Stdlib: io** | `read`, `write`, `append`, `exists`, `is_file`, `is_dir`, `mkdir`, `ls`, `rm`, `rename`, `basename`, `dirname`, `join`, `read_lines`, `write_lines`, `copy`, `size`, `touch` |
| **Removed `//` floor division** | `//` is always a comment; use `int(a / b)` |
| **Removed `//=` floor-division assign** | From grammar, tokenizer, and transformer |

---

## Phase 3 audit — imports, stdlib, runtime, security, performance

This pass focused on the standard-library import surface, the stdlib module
implementations, f-string escaping, async execution, and documentation drift.
Every item below has a regression test in `tests/test_regressions.py`.

### Import system

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 29 | Brace import compiled by dead code path | `import stdlib.math { sqrt }` worked only because `ImportStmt` fell through to `StatementTransformer`; the `Transformer` duplicated a legacy `_transform_import` that ignored `items` | Routed `Import`, `ImportStmt` and `FromImport` through one implementation; deleted the dead duplicates (`transpiler/transformer.py`) |
| 30 | Legacy `Import` node unhandled | `Program([Import("math")])` raised `NotImplementedError` after dispatch was unified | Added `StatementTransformer.transform_Import` |
| 31 | Grammar did not describe the brace form | `parser/aura.g4` only allowed `import m (as x)?`; the hand-written parser accepted `{a, b}` but the grammar was out of date | Added `ImportNames` rule and `importName` (with `as`) to the grammar |
| 32 | Docs taught non-existent API | `from stdlib.io import read_file, write_file` failed with `ImportError` | Corrected to `read`/`write`; rewrote the Imports section (EN + PT) to document the three real forms |
| 33 | Ambiguous alias+brace combination | `import m as x { a }` parsed the block as a separate statement (alias silently dropped) | Documented the supported forms; the transformer emits both bindings if the AST ever carries both |

### Standard library

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 34 | `math.gcd()` / `math.lcm()` crashed on no arguments | `TypeError: reduce() of empty iterable` | Return identity (`0` / `1`) for empty input |
| 35 | `string.split(s, sep, limit)` truncated parts | `split("a,b,c,d", ",", 2)` returned `["a","b"]` instead of using `maxsplit` | Use Python `maxsplit` semantics |
| 36 | `string.is_numeric` accepted `nan`, `inf`, blanks | `is_numeric("nan") is True` | Require a finite number and non-empty input |
| 37 | `string.pad_*` raised opaque `TypeError` | Multi-char fill produced a low-level Python error | Validate the fill char and raise a clear `ValueError` |
| 38 | `collections.reduce`/`list_reduce` conflated `initial=None` with "no initial" | Explicit `None` seed was ignored | Sentinel-based `_MISSING` default |
| 39 | `collections.take`/`drop` required sequences | `take(iter([...]), n)` raised `TypeError: not subscriptable` | Use `itertools.islice` (works with any iterable) |
| 40 | `AuraDict.__getattr__` intercepted dunder lookups | Broke `copy`, `pickle` and attribute protocol expectations | Dunder names now raise `AttributeError` immediately |
| 41 | JSON serialization emitted invalid JSON | `dumps(float("nan"))` produced `NaN`; `is_valid("NaN")` returned `True` | Strict serialization (`allow_nan=False`) and standards-compliant validation |
| 42 | `json.merge` crashed on non-mappings with a vague error | `object is not iterable` | Validate each argument is a mapping |
| 43 | `io` used locale-dependent encoding | Files written/read differently across platforms | Pin UTF-8 everywhere |
| 44 | `io.read_lines` silently dropped blank lines | Data loss on round-trip | Preserve every line |
| 45 | `io.write_lines` required pre-stringified input | `TypeError` for numeric lists | Stringify each item |
| 46 | `io.rm` swallowed missing-file errors | `missing_ok=True` hid bugs | Raise; callers guard with `exists` |
| 47 | `io.ls` ordering was filesystem-dependent | Non-deterministic output | Sort names |
| 48 | `io.copy` imported `shutil` on every call | Repeated import lookup | Module-level import |

### Runtime, f-strings and async

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 49 | f-string literal text double-escaped | `f'it\'s {1}'` produced invalid Python (`unterminated string literal`) | Quote-aware escaping in `ExpressionTransformer._escape_fstring_text`; only real control chars and the output delimiter are escaped |
| 50 | Top-level `await` generated invalid Python | `'await' outside function` | `cmd_run` wraps the whole program in a coroutine and runs it with `asyncio.run` |
| 51 | Top-level async calls were never awaited | `main()` silently did nothing (`coroutine was never awaited`) | `_await_top_level_async_calls` rewrites bare calls to async functions as `await` |
| 52 | `cmd_run` async branch was dead/duplicated | Executed generated code twice, had an operator-precedence bug in the async function scan, and unused locals | Replaced with a single, deterministic async wrapper |
| 53 | `memoize` crashed on unhashable arguments | `TypeError: unhashable type: 'list'` | Structural, collision-safe key builder with a repr fallback |

### Performance

1. **`io.copy`** – `shutil` is imported once at module load instead of on every call.
2. **`collections`** – `functools.reduce` is bound at module level; the hot
   `reduce`/`list_reduce`/`take`/`drop` helpers no longer perform per-call
   function imports.
3. **`string.split`** – single native call instead of split-then-slice.
4. **`AuraDict.__getattr__`** – short-circuits dunder lookups before touching
   the mapping, avoiding needless `KeyError` handling.

### Security notes

* JSON output is now standards-compliant; `NaN`/`Infinity` can no longer leak
  into documents consumed by strict parsers.
* `is_valid` rejects non-string input and non-standard constants instead of
  catching broad exceptions.
* `io` operations fail loudly (`rm` on a missing path raises) and use a fixed
  UTF-8 encoding, removing locale-dependent behavior.

### Documentation alignment

* `docs/language-reference/`: Imports chapter rewritten with
  the three supported forms; removed `read_file`/`write_file`.
* `stdlib/README.md`: documents `split` `maxsplit`, strict JSON, and the
  corrected `read_lines`/`write_lines` semantics.
* `parser/aura.g4`: grammar now matches the implemented brace-import form.

---

## Phase 4 audit — language rules, mutability, closures, modules

This pass verified the documented language rules against the implementation
and fixed every divergence found. All fixes carry regression tests in
`tests/test_regressions.py`.

### Enforced `let` / `let mut` / `const` semantics

The documentation defines `let` as an immutable binding, `let mut` as mutable,
and `const` as immutable. The `mut` flag was parsed but ignored: any binding
could be reassigned. A new semantic pass (`transpiler/semantics.py`) enforces
the rule at every entry point (`transpile`, `run`, `check`, `test`).

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 54 | `let` reassignment allowed | Immutable bindings could be reassigned silently | `MutabilityChecker` reports a semantic error; `let mut` is required to reassign |
| 55 | `const` reassignment allowed | Constants behaved like variables | `const` is treated as immutable |
| 56 | Member assignment misclassified | `self.x = ...` would look like a rebinding | Member/index targets are exempt (they mutate an object, not a binding) |
| 57 | Loop/catch/with bindings | Loop variables and `catch` names are rebound by the runtime | Treated as mutable |
| 58 | Nested-scope false positives | Shadowing in an inner block must not flag the outer name | Scope-aware lookup; only the closest binding is checked |
| 59 | 215 corpus files violated the rule | Examples and generated corpora reassigned non-`mut` `let`s | Migrated automatically with `tools/migrate_mut.py` (214 files, 428 declarations); the whole 6,609-file corpus now passes |

### Multi-target assignment

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 60 | `a, b = b, a` emitted invalid Python | `(a, (b = b), a)` — a `SyntaxError` | `parse_trailing_tuple` normalizes the tuple to `BinaryOp('=', (a, b), (b, a))`; swap and rotation now work |
| 61 | Multiple assignment targets wrong | `x, y, z = 1, 2, 3` mis-parsed | Same normalization covers N targets |

### Closures

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 62 | Closures could not mutate captured locals | `UnboundLocalError: cannot access local variable 'n'` | Hoisted block lambdas now emit `nonlocal` for names assigned in the closure that live in an enclosing function scope; module scope is never given `nonlocal` |
| 63 | Nested lambdas leaked scope | Inner lambda assignments could wrongly trigger `nonlocal` | Closure analysis stops at nested function/lambda boundaries |

### Enums, `throw` and modules

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 64 | Enum auto-numbering ignored explicit values | `enum E { A, B = 3, C }` gave `C == 3` instead of `4` | Auto values continue from the last explicit integer |
| 65 | `throw "message"` emitted `raise 'message'` | `TypeError: exceptions must derive from BaseException` | String throws are wrapped in `Exception(...)`; `throw ValueError(...)` still works |
| 66 | Local Aura modules could not be imported | `import sibling` failed with `No module named 'sibling'` | New import hook (`transpiler/importer.py`) transpiles `.aura` files next to the script, including dotted packages (`import pkg.util`) and `from pkg.util import x`; Python stdlib/PyPI imports are untouched |

### Verification

* `1845` automated tests pass (core + generated corpus).
* All `6,609` `.aura` files transpile to syntactically valid Python and pass
  the mutability checker.
* All examples run; a realistic multi-package project imports correctly.
* PyPI packages installed in the interpreter running `main.py` are importable
  from Aura (verified with `requests`).

---

## Phase 5 audit — Python interoperability

Verification of how Aura maps onto Python, focused on importing and using
Python code. Three real gaps were found and fixed.

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 67 | Multi-module import unsupported | `import os, sys` failed to parse | `ImportStmt` carries a `modules` list; `import a, b as c` emits the Python form |
| 68 | Raw/byte string prefixes unsupported | `r"\d+"`, `b"bytes"` were lexed as an identifier plus a string, producing `NameError` or wrong escapes | Tokenizer recognizes `r`, `b`, `rb`, `br` prefixes and preserves the literal verbatim (`StrLiteral.raw_literal`) |
| 69 | Python-style slicing unsupported | `items[1:3]`, `items[:2]`, `items[::-1]` raised parse errors | New `SliceExpr` node; `parse_subscript` handles `[start:stop:step]` for lists, strings and safe-index form |

### Confirmed working (no change needed)

| Capability | Evidence |
|------------|----------|
| Python stdlib imports | `os`, `sys`, `json`, `re`, `math`, `datetime`, `collections`, `itertools`, `functools`, `pathlib`, `urllib` |
| PyPI package imports | `requests` (installed in the running interpreter) |
| `from x import a, b as c` | Verified for stdlib modules |
| Python objects in Aura | `OrderedDict`, `defaultdict`, regex match objects, `Decimal`, `Fraction`, `uuid`, `hashlib` |
| Exception introspection | `catch e { e.__class__.__name__ }` reports `JSONDecodeError` etc. |
| Python callbacks | Lambdas passed to `sorted`, `map`, `filter` from Python APIs |

### Known API differences (by design)

* Aura's stdlib `map`/`filter`/`reduce` are **data-first** (`items |> map(fn)`),
  unlike Python's fn-first builtins. Use the pipe form or the stdlib signature.
* Generics are erased; Python values reaching Aura are typed as `Any`.

### Verification

* `6,609` corpus files still transpile to valid Python after the tokenizer change.
* New regression tests cover multi-import, raw/byte strings, slicing, and the
  full stdlib interop surface.
---

## Phase 6 audit — packaging and distribution

Aura is now an installable Python package with a console entry point.

| # | Change | Detail |
|---|--------|--------|
| 70 | Package namespace | Code moved under `aura/` (`aura.parser`, `aura.transpiler`, `aura.stdlib`, `aura.repl`, `aura.tools`) so site-packages is not polluted with generic names like `parser`/`stdlib` |
| 71 | Console script | `pyproject.toml` declares `aura = "aura.cli:main"`; `pip install .` provides the `aura` command |
| 72 | Runtime aliases | `aura/runtime.py` maps `stdlib`/`parser`/`transpiler` into `sys.modules` so existing Aura sources and generated code keep working unchanged |
| 73 | Wheel + sdist | Built with setuptools; only the `aura` package and `main.py` shim are distributed |
| 74 | Source compatibility | Root `parser/`, `transpiler/`, `stdlib/`, `repl/` are re-export shims; `python3 main.py` still works |
| 75 | `aura test` fix | Was shelling out to a non-existent `main.py`; now invokes `python -m aura.cli` and runs in the test file's directory |
| 76 | Formatter safety | `->`/`=>`/compound operators are masked before single-character spacing; string literals and comments are protected; unary `-`/`+` and one-line blocks are preserved; verified across all 6,609 corpus files |

### Verification

* `1845` tests pass; the full 6,609-file corpus transpiles and formats safely.
* Wheel and sdist both install into clean virtualenvs; `aura run`, `aura test`,
  `aura repl`, `aura check`, `aura format` work with no repo on `sys.path`.
* Installed package imports Python stdlib, PyPI packages (`requests`), local
  Aura modules and packages.

---

## Phase 7 audit — standard library, types, tooling, ecosystem

Closing the remaining interoperability and developer-experience gaps.

### Standard library

| # | Change | Detail |
|---|--------|--------|
| 77 | `stdlib.regex` | `match`, `full_match`, `search`, `find_all`, `find_iter` (lazy), `split`, `replace`, `replace_fn`, `subn`, `groups`, `group_dict`, `group`, `escape`, `compile_pattern`, `purge` + flags (`IGNORECASE`/`MULTILINE`/`DOTALL`/`VERBOSE`/`ASCII`/`UNICODE`); parity tested against `re` |
| 78 | `stdlib.os` | Environment, paths, cwd, `listdir`/`walk`, dir ops, process info. Process execution is deliberately excluded |
| 79 | `stdlib.http` | GET/POST/PUT/DELETE + JSON helpers over the standard library, falling back to `requests` when installed; responses are `AuraDict` |

### Type system

| # | Change | Detail |
|---|--------|--------|
| 80 | Function argument type checking | Calls are checked against declared parameter types, not just arity |
| 81 | Method type checking | Methods resolved through the class table are checked the same way |
| 82 | Generic type-parameter checking | `T` resolves in annotations; unused class type parameters are reported |
| 83 | Builtin return inference | `len`, `sum`, `min`, `max`, `sorted`, `str`, `list`, `set`, ... infer real types |
| 84 | Variadic-aware arity | `*args` / `**kwargs` relax the arity check |

### Language fixes

| # | Bug | Symptom | Fix |
|---|-----|---------|-----|
| 85 | String escapes not decoded | `"a\nb"` produced a literal backslash-n | Tokenizer decodes `\n`, `\t`, `\"`, `\\`, `\xNN`, `\uNNNN`, ... for non-raw strings |
| 86 | `UnlessStmt` crashed the type checker | `AttributeError: then_body` | `_check_if` handles both `IfStmt` and `UnlessStmt` |

### Tooling and ecosystem

| # | Change | Detail |
|---|--------|--------|
| 87 | Dependency manager | `aura init/add/install/deps` backed by `aura.toml` |
| 88 | Version tooling | `aura version [major|minor|patch|x.y.z]` keeps `pyproject.toml` and `aura/__init__.py` in sync |
| 89 | Trace debugger | `aura debug [--trace] [--show-code]` with generated→Aura line mapping |
| 90 | Language server | `aura lsp`: diagnostics, hover, completion, document symbols over stdio |
| 91 | CI | GitHub Actions: tests on Python 3.10–3.13, CLI smoke test, distribution build |
| 92 | Release workflow | Tag-driven build, version/tag consistency check, PyPI trusted publishing, GitHub release |
| 93 | `aura version` installed mode | Falls back to package metadata when no `pyproject.toml` is present |

### Verification

* `541` automated tests pass (core + tooling + generated corpus).
* All `6,609` corpus files transpile to valid Python and pass the mutability
  and type checkers with no false positives.
* Installed wheel exposes 15 commands; `run`, `test`, `debug`, `lsp`, `deps`
  and `version` verified from a clean virtualenv.
