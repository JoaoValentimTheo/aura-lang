"""Run every Aura Pattern (AUP) example as a real program.

Each file under ``examples/aup/`` must transpile and execute. This keeps the
documented patterns honest: if the language changes in a way that breaks an
idiom, a test fails.
"""
import contextlib
import io
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import parse_file  # noqa: E402
from aura.transpiler.transformer import Transformer  # noqa: E402

AUP_DIR = ROOT / "examples" / "aup"
AUP_EXAMPLES = sorted(AUP_DIR.glob("*.aura"))


def _run(path):
    program = parse_file(str(path))
    code = Transformer().transform(program)
    namespace = {"__name__": "__aup_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, str(path), "exec"), namespace)
    return buffer.getvalue()


def test_aup_examples_exist():
    assert AUP_EXAMPLES, "no AUP examples found under examples/aup/"


@pytest.mark.parametrize("path", AUP_EXAMPLES, ids=lambda p: p.stem)
def test_aup_example_runs(path):
    output = _run(path)
    assert output.strip(), f"{path.name} produced no output"


def test_aup_examples_are_documented():
    """Every example is listed in docs/AUP.md."""
    doc = (ROOT / "docs" / "AUP.md").read_text(encoding="utf-8")
    missing = [p.name for p in AUP_EXAMPLES if p.name not in doc]
    assert not missing, f"examples not documented in docs/AUP.md: {missing}"