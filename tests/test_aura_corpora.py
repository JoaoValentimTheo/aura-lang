"""Run the generated `.aura` corpora as real end-to-end programs.

The corpora under ``tests/*_tests/`` were produced by the generator tools and
are excluded from ordinary pytest collection to keep the default suite fast.
This module exercises the ones that are valid, executable programs, so the
syntax/scope/OOP/security surface they cover stays honest.

``collections_tests`` is intentionally skipped: it was generated against a
different stdlib API contract (e.g. ``reduce(initial, fn)`` and bare
``sort``/``contains`` helpers) and is not a reliable oracle.
"""
import contextlib
import io
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from aura.parser.to_ast import parse_file  # noqa: E402
from aura.runtime import install_runtime_aliases  # noqa: E402
from aura.transpiler.transformer import Transformer  # noqa: E402

CORPORA = [
    'oop_tests',
    'syntax_scope_tests',
    'secure_aura_tests',
    'speed_aura_tests',
    'integration_tests',
]

BASE = Path(__file__).parent


def _corpus_files(name):
    return sorted((BASE / name).glob('*.aura'))


def _run_program(path):
    """Transpile and execute one Aura file, returning captured stdout."""
    install_runtime_aliases()
    program = parse_file(str(path))
    code = Transformer().transform(program)
    namespace = {'__name__': '__aura_corpus__'}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, str(path), 'exec'), namespace)
    return buffer.getvalue()


@pytest.mark.parametrize('corpus', CORPORA)
def test_corpus_executes(corpus):
    files = _corpus_files(corpus)
    if not files:
        pytest.skip(f"no files in {corpus}")
    failures = []
    for path in files:
        try:
            _run_program(path)
        except Exception as exc:  # noqa: BLE001 - report every failure
            failures.append(f"{path.name}: {type(exc).__name__}: {exc}")
    assert not failures, (
        f"{len(failures)}/{len(files)} files in {corpus} failed:\n"
        + "\n".join(failures[:20])
    )


@pytest.mark.parametrize('corpus', CORPORA)
def test_corpus_transpiles_to_valid_python(corpus):
    """Every corpus file must at least compile to valid Python."""
    files = _corpus_files(corpus)
    if not files:
        pytest.skip(f"no files in {corpus}")
    failures = []
    for path in files:
        try:
            program = parse_file(str(path))
            code = Transformer().transform(program)
            compile(code, str(path), 'exec')
        except Exception as exc:  # noqa: BLE001
            failures.append(f"{path.name}: {type(exc).__name__}: {exc}")
    assert not failures, (
        f"{len(failures)}/{len(files)} files in {corpus} failed to transpile:\n"
        + "\n".join(failures[:20])
    )