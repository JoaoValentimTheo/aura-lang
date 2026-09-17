"""Run every Aura Pattern (AUP) example as a real program.

Each file under ``examples/aup/`` must transpile and execute. This keeps the
documented patterns honest: if the language changes in a way that breaks an
idiom, a test fails. The suite also enforces the AUP standard (header format,
sequential numbering, and matching table and section entries in docs/AUP.md).
"""
import contextlib
import io
import re
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import parse_file  # noqa: E402
from aura.transpiler.transformer import Transformer  # noqa: E402

AUP_DIR = ROOT / "examples" / "aup"
AUP_EXAMPLES = sorted(AUP_DIR.glob("*.aura"))
AUP_DOC = ROOT / "docs" / "AUP.md"

# `// AUP-NN: Title.`
HEADER_RE = re.compile(r"^// AUP-(\d{2}): (.+)$")


def _run(path):
    program = parse_file(str(path))
    code = Transformer().transform(program)
    namespace = {"__name__": "__aup_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, str(path), "exec"), namespace)
        # Entry files declare `main`; the runtime invokes it (the programmer
        # never calls it). Mirror that here.
        main = namespace.get("main")
        if callable(main):
            main()
    return buffer.getvalue()


def _header(path):
    return path.read_text(encoding="utf-8").splitlines()[0]


def test_aup_examples_exist():
    assert AUP_EXAMPLES, "no AUP examples found under examples/aup/"


@pytest.mark.parametrize("path", AUP_EXAMPLES, ids=lambda p: p.stem)
def test_aup_example_runs(path):
    output = _run(path)
    assert output.strip(), f"{path.name} produced no output"


def test_aup_examples_are_documented():
    """Every example is listed in docs/AUP.md."""
    doc = AUP_DOC.read_text(encoding="utf-8")
    missing = [p.name for p in AUP_EXAMPLES if p.name not in doc]
    assert not missing, f"examples not documented in docs/AUP.md: {missing}"


# ---------------------------------------------------------------------------
# AUP standard: header, numbering and documentation must agree
# ---------------------------------------------------------------------------

@pytest.mark.parametrize("path", AUP_EXAMPLES, ids=lambda p: p.stem)
def test_aup_header_format(path):
    """Every example starts with `// AUP-NN: Title.`."""
    assert HEADER_RE.match(_header(path)), (
        f"{path.name}: first line must match '// AUP-NN: Title.'")


def test_aup_numbers_are_unique_and_sequential():
    numbers = []
    for path in AUP_EXAMPLES:
        match = HEADER_RE.match(_header(path))
        assert match, f"{path.name} has no AUP number"
        numbers.append(int(match.group(1)))
    assert len(set(numbers)) == len(numbers), f"duplicate AUP numbers: {numbers}"
    assert sorted(numbers) == list(range(1, len(numbers) + 1)), (
        f"AUP numbers must be 01..{len(numbers):02d} with no gaps: {sorted(numbers)}")


@pytest.mark.parametrize("path", AUP_EXAMPLES, ids=lambda p: p.stem)
def test_aup_example_has_table_row(path):
    """docs/AUP.md's index table lists the pattern with the matching number."""
    match = HEADER_RE.match(_header(path))
    number = match.group(1)
    doc = AUP_DOC.read_text(encoding="utf-8")
    assert re.search(rf"^\|\s*{number}\s*\|", doc, re.MULTILINE), (
        f"docs/AUP.md table has no row for AUP-{number} ({path.name})")


@pytest.mark.parametrize("path", AUP_EXAMPLES, ids=lambda p: p.stem)
def test_aup_example_has_section(path):
    """docs/AUP.md has a `## NN — Title` section for the pattern."""
    number = HEADER_RE.match(_header(path)).group(1)
    doc = AUP_DOC.read_text(encoding="utf-8")
    assert re.search(rf"^## {number} — ", doc, re.MULTILINE), (
        f"docs/AUP.md has no '## {number} — ...' section for {path.name}")


def test_aup_examples_all_define_main():
    """Each pattern is an entry file, so it must declare `main`."""
    for path in AUP_EXAMPLES:
        text = path.read_text(encoding="utf-8")
        assert re.search(r"\bdef main\s*\(", text), (
            f"{path.name}: AUP examples must define 'main'")
