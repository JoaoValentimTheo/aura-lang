"""Guard that every example in `examples/` stays valid.

An example that no longer parses, type-checks, or runs is documentation that
lies. This module parses, rule-checks, type-checks, transpiles to valid Python,
and executes each `.aura` file under `examples/`, so a stale example fails the
build instead of shipping.

Examples that need the network are run only for parse/check, not execution;
they are listed in `NETWORK_EXAMPLES`.
"""
import ast as py_ast
import re
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import parse_file  # noqa: E402
from aura.transpiler.rules import RuleChecker  # noqa: E402
from aura.transpiler.transformer import Transformer  # noqa: E402
from aura.transpiler.types import TypeChecker  # noqa: E402

EXAMPLES_DIR = ROOT / "examples"
EXAMPLES = sorted(EXAMPLES_DIR.rglob("*.aura"))

# Examples that reach the network or a long computation: checked but not run.
NETWORK_EXAMPLES = {"crypto.aura"}

# Examples whose output is inherently non-deterministic (timeit, threads).
NONDETERMINISTIC = {"macros.aura", "worker_pool.aura"}


def test_examples_directory_is_not_empty():
    assert EXAMPLES, "no examples found"


def test_examples_readme_lists_every_file():
    """Every `.aura` example must be mentioned in examples/README.md."""
    readme = (EXAMPLES_DIR / "README.md").read_text(encoding="utf-8")
    missing = [p.name for p in EXAMPLES if p.name not in readme]
    assert not missing, f"examples/README.md does not mention: {missing}"


@pytest.mark.parametrize("path", EXAMPLES, ids=lambda p: str(p.relative_to(ROOT)))
def test_example_parses_and_type_checks(path):
    program = parse_file(str(path))

    rule_checker = RuleChecker()
    assert rule_checker.check_program(program), (
        f"{path.name} violates Aura rules: "
        + "; ".join(str(e) for e in rule_checker.collector.errors))

    type_checker = TypeChecker()
    assert type_checker.check_program(program), (
        f"{path.name} does not type-check: "
        + "; ".join(str(d) for d in type_checker.diagnostics))


@pytest.mark.parametrize("path", EXAMPLES, ids=lambda p: str(p.relative_to(ROOT)))
def test_example_transpiles_to_valid_python(path):
    program = parse_file(str(path))
    code = Transformer().transform(program)
    py_ast.parse(code)  # raises SyntaxError if the output is not Python


@pytest.mark.parametrize("path", EXAMPLES, ids=lambda p: str(p.relative_to(ROOT)))
def test_example_runs(path):
    if path.name in NETWORK_EXAMPLES:
        pytest.skip("requires network access")
    result = subprocess.run(
        [sys.executable, "-m", "aura.cli", "run", str(path)],
        capture_output=True, text=True, timeout=60, cwd=str(path.parent),
    )
    assert result.returncode == 0, (
        f"{path.name} failed to run:\n{result.stdout}\n{result.stderr}")


@pytest.mark.parametrize("path", EXAMPLES, ids=lambda p: str(p.relative_to(ROOT)))
def test_example_produces_output(path):
    if path.name in NETWORK_EXAMPLES or path.name in NONDETERMINISTIC:
        pytest.skip("output is not deterministic or needs the network")
    result = subprocess.run(
        [sys.executable, "-m", "aura.cli", "run", str(path)],
        capture_output=True, text=True, timeout=60, cwd=str(path.parent),
    )
    assert result.stdout.strip(), f"{path.name} printed nothing"


def test_examples_use_extends_not_parenthesised_bases():
    """No example may use the removed inheritance spellings."""
    offenders = []
    for path in EXAMPLES:
        text = path.read_text(encoding="utf-8")
        if re.search(r"^\s*class\s+\w+\s*\(", text, re.M) or "implements" in text:
            offenders.append(path.name)
    assert not offenders, (
        f"examples use the removed inheritance syntax: {offenders}")


def test_examples_declare_visibility_on_members():
    """Class/trait members in examples must declare their visibility.

    Visibility is required only for members declared directly in a class or
    trait body (E307), so this walks the real AST rather than guessing from
    indentation.
    """
    from aura.transpiler.ast import ClassDecl, ConstDecl, Method, TraitDecl, VarDecl

    offenders = []
    for path in EXAMPLES:
        program = parse_file(str(path))

        def walk(node):
            if node is None:
                return
            if isinstance(node, (list, tuple)):
                for item in node:
                    walk(item)
                return
            if isinstance(node, (ClassDecl, TraitDecl)):
                body = getattr(node, 'body', None) or getattr(node, 'members', None) or []
                for member in body:
                    if isinstance(member, (VarDecl, ConstDecl, Method)):
                        if getattr(member, 'visibility', None) is None:
                            offenders.append(f"{path.name}: {member.name}")
                for member in body:
                    walk(member)
                return
            if hasattr(node, '__dict__'):
                for value in vars(node).values():
                    if isinstance(value, (list, tuple)) or hasattr(value, '__dict__'):
                        walk(value)

        walk(program)

    assert not offenders, f"members missing visibility: {offenders}"