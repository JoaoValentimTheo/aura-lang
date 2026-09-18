"""Deep in-process tests for the Aura CLI.

These call the ``cmd_*`` functions directly to exercise the real code paths
(argument handling, readers, reporters, exit codes) rather than shelling out.
"""
import io
import sys
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.cli import (  # noqa: E402
    _prepare_entrypoint,
    cmd_check,
    cmd_debug,
    cmd_format,
    cmd_run,
    cmd_test,
    cmd_transpile,
    main,
)


def write(tmp_path, name, source):
    path = tmp_path / name
    path.write_text(source)
    return str(path)


def capture(func, *args, **kwargs):
    """Run ``func`` capturing stdout/stderr; return (code, out, err)."""
    out, err = io.StringIO(), io.StringIO()
    with redirect_stdout(out), redirect_stderr(err):
        code = func(*args, **kwargs)
    return code, out.getvalue(), err.getvalue()


# ============================================================================
# cmd_transpile
# ============================================================================

def test_transpile_to_stdout(tmp_path, capsys):
    path = write(tmp_path, "t.aura", "def f() -> int { return 1 }")
    assert cmd_transpile(path) == 0
    assert "def f" in capsys.readouterr().out


def test_transpile_to_file(tmp_path):
    path = write(tmp_path, "t.aura", "def f() -> int { return 1 }")
    out = tmp_path / "t.py"
    code, text, _ = capture(cmd_transpile, path, str(out))
    assert code == 0
    assert out.is_file()
    assert "def f" in out.read_text()


def test_transpile_verbose(tmp_path):
    path = write(tmp_path, "t.aura", "def f() -> int { return 1 }")
    code, _, err = capture(cmd_transpile, path, None, True)
    assert code == 0
    assert "AST" in err


def test_transpile_missing_file():
    code, _, err = capture(cmd_transpile, "/nonexistent/x.aura")
    assert code == 2
    assert "not found" in err.lower()


def test_transpile_syntax_error(tmp_path):
    path = write(tmp_path, "bad.aura", "def f( { }")
    code, _, err = capture(cmd_transpile, path)
    assert code == 2


# ============================================================================
# cmd_check
# ============================================================================

def test_check_ok(tmp_path, capsys):
    path = write(tmp_path, "t.aura", "def main() { let x = 1\n print(x) }")
    assert cmd_check(path) == 0
    assert "passed" in capsys.readouterr().out


def test_check_verbose(tmp_path, capsys):
    path = write(tmp_path, "t.aura", "def main() { let x = 1 }")
    assert cmd_check(path, verbose=True) == 0
    err = capsys.readouterr().err
    assert "Inferred" in err


def test_check_missing_file():
    code, _, err = capture(cmd_check, "/nonexistent/x.aura")
    assert code == 2


# ============================================================================
# cmd_format
# ============================================================================

def test_format_to_stdout(tmp_path, capsys):
    path = write(tmp_path, "t.aura", "def f() {\n    return 1\n}\n")
    assert cmd_format(path) == 0
    assert "def f" in capsys.readouterr().out


def test_format_to_file(tmp_path):
    path = write(tmp_path, "t.aura", "def f() {\n    return 1\n}\n")
    out = tmp_path / "formatted.aura"
    code, text, _ = capture(cmd_format, path, str(out))
    assert code == 0
    assert out.is_file()


def test_format_missing_file():
    code, _, err = capture(cmd_format, "/nonexistent/x.aura")
    assert code == 2


# ============================================================================
# cmd_run
# ============================================================================

def test_run_basic(tmp_path, capsys):
    path = write(tmp_path, "t.aura", 'def main() { print("ok") }')
    assert cmd_run(path) == 0
    assert capsys.readouterr().out == "ok\n"


def test_run_with_program_args(tmp_path, capsys):
    path = write(tmp_path, "t.aura",
                 "def main(args: [string]) { print(args) }")
    assert cmd_run(path, program_args=["a", "b"]) == 0
    assert capsys.readouterr().out.strip() == "['a', 'b']"


def test_run_verbose(tmp_path, capsys):
    path = write(tmp_path, "t.aura", 'def main() { print("x") }')
    assert cmd_run(path, verbose=True) == 0
    assert "Generated Python code" in capsys.readouterr().err


def test_run_return_code(tmp_path):
    path = write(tmp_path, "t.aura", "def main() -> int { return 4 }")
    assert cmd_run(path) == 4


def test_run_missing_file():
    code, _, err = capture(cmd_run, "/nonexistent/x.aura")
    assert code == 2


def test_run_runtime_error(tmp_path, capsys):
    path = write(tmp_path, "t.aura", "def main() { let x = 1 / 0 }")
    assert cmd_run(path) == 1
    assert "Runtime error" in capsys.readouterr().err


def test_run_no_main(tmp_path, capsys):
    path = write(tmp_path, "t.aura", "print(1)")
    assert cmd_run(path) == 2
    assert "[E310]" in capsys.readouterr().err


# ============================================================================
# cmd_debug
# ============================================================================

def test_debug_runs_program(tmp_path, capsys):
    path = write(tmp_path, "t.aura", 'def main() { print("dbg") }')
    assert cmd_debug(path) == 0
    assert "dbg" in capsys.readouterr().out


def test_debug_trace(tmp_path, capsys):
    path = write(tmp_path, "t.aura", "def main() { let x = 1\n print(x) }")
    assert cmd_debug(path, trace=True) == 0


def test_debug_show_code(tmp_path, capsys):
    path = write(tmp_path, "t.aura", "def main() { print(1) }")
    assert cmd_debug(path, show_code=True) == 0


def test_debug_uncaught_exception(tmp_path, capsys):
    path = write(tmp_path, "t.aura", "def main() { let x = 1 / 0 }")
    code = cmd_debug(path)
    assert code == 1
    assert "ZeroDivisionError" in capsys.readouterr().err


# ============================================================================
# cmd_test
# ============================================================================

def test_test_directory_all_pass(tmp_path, capsys):
    d = tmp_path / "suite"
    d.mkdir()
    (d / "a.aura").write_text('def main() { print(1) }')
    (d / "b.aura").write_text('def main() { print(2) }')
    assert cmd_test(str(d)) == 0
    assert "2/2 passed" in capsys.readouterr().out


def test_test_reports_failure(tmp_path, capsys):
    d = tmp_path / "suite"
    d.mkdir()
    (d / "ok.aura").write_text('def main() { print(1) }')
    # A runtime error is a failure; a missing `main` is not, because a test
    # file may drive itself (e.g. `stdlib.testing` + `t.run_all()`).
    (d / "bad.aura").write_text('def main() { throw Error("boom") }')
    assert cmd_test(str(d)) == 1
    assert "failed" in capsys.readouterr().out


def test_test_does_not_require_main(tmp_path, capsys):
    d = tmp_path / "suite"
    d.mkdir()
    (d / "selfdriven.aura").write_text('import stdlib.testing as t\n'
                                       't.test("x", () => { t.equal(1, 1) })\n'
                                       't.run_all()\n')
    assert cmd_test(str(d)) == 0
    assert "passed" in capsys.readouterr().out


def test_test_no_files(tmp_path, capsys):
    empty = tmp_path / "empty"
    empty.mkdir()
    code, out, err = capture(cmd_test, str(empty))
    assert code == 2


def test_test_single_file(tmp_path):
    path = write(tmp_path, "one.aura", 'def main() { print(1) }')
    assert cmd_test(path) == 0


# ============================================================================
# _prepare_entrypoint
# ============================================================================

def test_prepare_entrypoint_sync():
    from aura.parser.to_ast import Parser, Tokenizer
    program = Parser(Tokenizer("def main() { print(1) }").tokenize()).parse()
    has_async, snippet = _prepare_entrypoint(program)
    assert has_async is False
    assert "main()" in snippet


def test_prepare_entrypoint_async():
    from aura.parser.to_ast import Parser, Tokenizer
    program = Parser(Tokenizer("async def main() { print(1) }").tokenize()).parse()
    has_async, snippet = _prepare_entrypoint(program)
    assert has_async is True
    assert "await main()" in snippet


def test_prepare_entrypoint_with_args():
    from aura.parser.to_ast import Parser, Tokenizer
    program = Parser(Tokenizer("def main(args: [string]) {}").tokenize()).parse()
    _, snippet = _prepare_entrypoint(program)
    assert "main(_aura_argv)" in snippet


def test_prepare_entrypoint_explicit_call_skips_invocation():
    from aura.parser.to_ast import Parser, Tokenizer
    program = Parser(Tokenizer("def main() {}\nmain()").tokenize()).parse()
    _, snippet = _prepare_entrypoint(program)
    assert snippet == ""


# ============================================================================
# argparse entry point
# ============================================================================

def test_main_version_command(capsys):
    assert main(["version"]) == 0
    assert capsys.readouterr().out.strip()


def test_main_help_exits(capsys):
    with pytest.raises(SystemExit):
        main(["--help"])


def test_main_no_command_exits_nonzero(capsys):
    # No subcommand: argparse reports usage and exits non-zero.
    with pytest.raises(SystemExit) as info:
        main([])
    assert info.value.code != 0
