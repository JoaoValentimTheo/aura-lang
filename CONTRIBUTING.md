# Contributing to Aura

Thanks for your interest in Aura! This guide covers the development workflow.

## Development setup

```bash
git clone https://github.com/JoaoValentimTheo/aura-lang.git
cd aura-lang
python3 -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"          # pytest, pytest-cov, ruff, mypy
```

Run the toolchain from a checkout with `python3 main.py <command>`, or install
the `aura` console script with `pip install -e .`.

## The pipeline

Aura is a source-to-source transpiler. The pipeline is:

```
.aura source
  └─ aura/parser/to_ast.py        Tokenizer → Parser → AST
       └─ aura/transpiler/ast.py  node definitions + protocol (dunder) map
  └─ aura/transpiler/semantics.py MutabilityChecker
  └─ aura/transpiler/rules.py     RuleChecker
  └─ aura/transpiler/types.py     TypeChecker
  └─ aura/transpiler/transformer.py → transformers/{statements,expressions}.py
  └─ Python source → compile()/exec()
```

`docs/language-reference/grammar.md` is the **single source of truth** for the concrete syntax. If
the parser and the grammar disagree, one of them is a bug.

## Adding a language feature

1. **Grammar first.** Update `docs/language-reference/grammar.md` so the intended syntax is written
   down before you implement it.
2. **AST.** Add the node to `aura/transpiler/ast.py` if needed.
3. **Parser.** Implement the construct in `aura/parser/to_ast.py`. Prefer a
   dedicated node over reconstructing source strings from tokens.
4. **Transformer.** Emit Python in `transformers/statements.py` or
   `transformers/expressions.py`. Record any prelude requirement on the
   transformer (e.g. `expr.has_dict = True`) rather than relying on a separate
   AST scan.
5. **Rules/types.** Enforce structural rules in `rules.py` and, when it matters,
   type behavior in `types.py`.
6. **Tests.** Add focused tests. `tests/test_syntax_complete.py` covers every
   construct end to end; `tests/test_syntax_standard.py` locks canonical vs.
   removed spellings.
7. **Docs.** Update the affected document under `docs/language-reference/` (and its `*.pt_BR.md` sibling).

## Adding a diagnostic

1. **Define the code** in `aura/transpiler/errors.py` following the existing pattern.
2. **Document it** in `docs/ERRORS.md` with the same code, name, and message.
3. **Add a test** that triggers the diagnostic and verifies the message.
4. **Emit it** from the appropriate checker (type, rule, or mutability).

## Adding an AUP pattern

1. **Write the pattern** in `docs/AUP.md` following the existing format.
2. **Add the example** in `examples/aup/<name>.aura` with a `def main()` entry point.
3. **Add a test** in `tests/test_aup.py` that verifies the example compiles and runs.
4. **Update** `examples/README.md` with a row for the new pattern.

## Syntax rules

Aura has exactly one spelling per construct — no synonyms. Do not add aliases.
Removed spellings (`fn`, `init`, `!`, `&&`, `||`, `null`, `<T>`, `volatily`)
must raise a clear `SyntaxError` that names the canonical form. See
`docs/language-reference/grammar.md` Appendix A.

> **Syntax freeze.** As of `0.2.0a1`, Aura's syntax is officially frozen.
> No syntax changes will be made before the stable 1.0 release. If you find
> a syntax bug, register it in the issue tracker but do not change the grammar.

## Tests

```bash
python -m pytest tests/ -q                 # full suite (4,400+ tests)
python -m pytest tests/ -q --cov=aura      # with coverage (floor: 90%)
ruff check aura/                            # lint
mypy aura/                                  # types (advisory)
```

### Test categories

The test suite is organized by category:

- **Static tests**: `test_syntax_complete.py`, `test_syntax_standard.py` — verify syntax constructs
- **Runtime tests**: `test_runtime.py`, `test_regressions.py` — verify transpiled code behavior
- **Type tests**: `test_types_deep.py`, `test_type_matrix.py` — verify type checking
- **OOP tests**: `test_oop_complete.py`, `test_oop_abstract_class.py` — verify class features
- **Property tests**: `test_property_*.py` — Hypothesis-based fuzzing
- **CLI tests**: `test_cli_*.py` — verify CLI commands
- **Security tests**: `test_security.py`, `test_audit_*.py` — verify security hardening

Run a specific category:

```bash
python -m pytest tests/test_syntax_complete.py -q    # syntax tests only
python -m pytest tests/test_runtime.py -q             # runtime tests only
python -m pytest tests/ -k "test_cli" -q              # CLI tests only
```

### Examples

Every example in `examples/` passes `aura check` and runs with `aura run`. The test suite (`tests/test_examples.py`) verifies all examples automatically.

```bash
aura check examples/hello.aura
aura run examples/hello.aura
```

### AUP patterns

The Aura Patterns in `examples/aup/` are verified by `tests/test_aup.py`.

Every change should keep the suite green. New security-relevant behavior belongs
in `tests/test_security.py`.

## Project structure

```
aura-lang/
├── aura/                   # Installable package (pip install .)
│   ├── cli.py              # Console entry point (`aura` command)
│   ├── runtime.py          # stdlib namespace aliases for generated code
│   ├── parser/
│   │   └── to_ast.py       # Tokenizer + recursive-descent parser
│   ├── transpiler/         # Core transpilation logic
│   ├── stdlib/             # Standard library (17 modules)
│   ├── repl/               # Interactive engine
│   ├── lsp/                # Language server (JSON-RPC over stdio)
│   └── tools/              # Formatter, deps, release, debugger, generators
├── examples/               # Working example programs
├── training/               # Structured learning material (for humans and LLMs)
├── tests/                  # Test suite (static, runtime, regression, fuzz)
├── docs/                   # Documentation (Jekyll + Just the Docs)
├── main.py                 # Backward-compatible CLI shim
└── pyproject.toml          # Packaging metadata + console script
```

### Root-level shims

The root-level `parser/`, `transpiler/`, `stdlib/`, `repl/`, and `tools/`
directories are **compatibility shims** that redirect imports to the `aura.*`
equivalents. They exist so that:

1. `from parser.to_ast import parse_file` works from a source checkout
2. Existing tests that import from these shims continue to work
3. The `aura/runtime.py` can set up `sys.modules` aliases for generated code

These shims are **not included in the wheel** (only `main.py` is shipped as a
top-level module). If you are writing new code, always use `aura.*` imports
(e.g., `from aura.parser.to_ast import parse_file`).

## Style

- Python: 4-space indent, 100-column limit, `ruff check` clean.
- No comments unless they explain *why*, not *what*.
- Keep the generated `.aura` corpora out of hand edits; regenerate them with
  `aura/tools/generate_*.py` if needed.

## Commits and releases

- Write focused commits with an imperative subject and a body explaining the
  *why*.
- Changelog entries follow [Keep a Changelog](https://keepachangelog.com/).
  Add notable user-facing changes to the CHANGELOG before a release.
- Versions follow [Semantic Versioning](https://semver.org/). `aura version
  [major|minor|patch]` keeps `pyproject.toml` and `aura/__init__.py` in sync.
- Pushing a `v*` tag runs `.github/workflows/release.yml`: it builds the
  distributions, creates a GitHub release, and (when `PUBLISH_TO_PYPI=true`)
  publishes to PyPI via trusted publishing.

## Reporting issues

Use the GitHub issue tracker. For security problems, follow `SECURITY.md`
instead of opening a public issue.