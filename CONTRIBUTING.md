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

`docs/GRAMMAR.md` is the **single source of truth** for the concrete syntax. If
the parser and the grammar disagree, one of them is a bug.

## Adding a language feature

1. **Grammar first.** Update `docs/GRAMMAR.md` so the intended syntax is written
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
7. **Docs.** Update `docs/LANGUAGE.md` (and `LANGUAGE_PT.md`).

## Syntax rules

Aura has exactly one spelling per construct — no synonyms. Do not add aliases.
Removed spellings (`fn`, `init`, `!`, `&&`, `||`, `null`, `<T>`, `volatily`)
must raise a clear `SyntaxError` that names the canonical form. See
`docs/GRAMMAR.md` Appendix A.

## Tests

```bash
python -m pytest tests/ -q                 # full suite
python -m pytest tests/ -q --cov=aura      # with coverage (floor: 60%)
ruff check aura/                            # lint
mypy aura/                                  # types (advisory)
```

Every change should keep the suite green. New security-relevant behavior belongs
in `tests/test_security.py`.

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