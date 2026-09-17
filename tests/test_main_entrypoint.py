"""Program entry-point tests: the ``main`` convention.

An entry file executed with ``aura run`` must declare a top-level ``main``;
the runtime invokes it (with command-line arguments when it asks for them), so
the programmer never writes a trailing ``main()`` call. Imported modules are
left alone and need no ``main``.
"""
import contextlib
import io
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.rules import RuleChecker  # noqa: E402


def parse(source):
    return Parser(Tokenizer(source).tokenize()).parse()


def rule_errors(source, require_main=True):
    checker = RuleChecker()
    checker.check_program(parse(source), require_main=require_main)
    return [str(e) for e in checker.collector.errors]


def rule_codes(source, require_main=True):
    return [e.split()[1] for e in rule_errors(source, require_main)]


def run_cli(source, *args, tmp_path=None):
    path = tmp_path / "program.aura"
    path.write_text(source)
    return subprocess.run(
        [sys.executable, str(ROOT / "main.py"), "run", str(path), *args],
        capture_output=True, text=True, timeout=30,
    )


# ============================================================================
# E310 - missing main
# ============================================================================

def test_missing_main_is_rejected():
    assert rule_codes("print('hi')") == ["[E310]"]


def test_missing_main_is_accepted_for_modules():
    assert rule_codes("print('hi')", require_main=False) == []


def test_main_present_is_accepted():
    assert rule_codes("def main() { print('hi') }") == []


def test_async_main_is_accepted():
    assert rule_codes("async def main() { print('hi') }") == []


def test_main_with_args_is_accepted():
    assert rule_codes("def main(args: [string]) { print(args) }") == []


# ============================================================================
# E311 - invalid main signature
# ============================================================================

def test_main_with_two_params_is_rejected():
    assert rule_codes("def main(a: int, b: int) {}") == ["[E311]"]


def test_main_with_non_args_param_is_rejected():
    assert rule_codes("def main(data: [string]) {}") == ["[E311]"]


def test_main_with_variadic_is_rejected():
    assert rule_codes("def main(*args) {}") == ["[E311]"]


# ============================================================================
# Runtime: the runtime invokes main, the programmer does not
# ============================================================================

def test_runtime_invokes_main(tmp_path):
    result = run_cli("def main() { print('hello from main') }\n", tmp_path=tmp_path)
    assert result.returncode == 0, result.stderr
    assert result.stdout == "hello from main\n"


def test_no_trailing_call_needed(tmp_path):
    # A trailing `main()` is not required and must not be written.
    result = run_cli(
        "def main() { print('once') }\n", tmp_path=tmp_path)
    assert result.stdout == "once\n"


def test_explicit_call_is_not_run_twice(tmp_path):
    result = run_cli(
        "def main() { print('once') }\nmain()\n", tmp_path=tmp_path)
    assert result.stdout == "once\n"


def test_runtime_passes_arguments(tmp_path):
    result = run_cli(
        "def main(args: [string]) { print(args) }\n",
        "alpha", "beta", tmp_path=tmp_path,
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "['alpha', 'beta']"


def test_async_main_is_awaited(tmp_path):
    result = run_cli(
        "async def value() -> int { return 7 }\n"
        "async def main() { print(await value()) }\n",
        tmp_path=tmp_path,
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout == "7\n"


def test_main_return_value_is_exit_code(tmp_path):
    result = run_cli("def main() -> int { return 3 }\n", tmp_path=tmp_path)
    assert result.returncode == 3


def test_missing_main_fails_the_cli(tmp_path):
    result = run_cli("print('hi')\n", tmp_path=tmp_path)
    assert result.returncode != 0
    assert "E310" in result.stderr


# ============================================================================
# Nested functions capture and mutate enclosing locals (nonlocal)
# ============================================================================

def test_nested_function_mutates_captured_local():
    from aura.transpiler.transformer import Transformer
    code = Transformer().transform(parse(
        "def main() {\n"
        "  let mut count = 0\n"
        "  def bump() { count += 1 }\n"
        "  bump()\n"
        "  bump()\n"
        "  print(count)\n"
        "}"
    ))
    assert "nonlocal count" in code
    namespace = {"__name__": "__test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, "<test>", "exec"), namespace)
        namespace["main"]()
    assert buffer.getvalue() == "2\n"


def test_nested_function_does_not_declare_nonlocal_for_own_locals():
    from aura.transpiler.transformer import Transformer
    code = Transformer().transform(parse(
        "def main() {\n"
        "  def inner() { let x = 1\n return x }\n"
        "  print(inner())\n"
        "}"
    ))
    assert "nonlocal" not in code
