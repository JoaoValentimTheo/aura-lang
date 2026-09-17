"""Coverage for CLI branches not exercised by the happy-path CLI tests.

Focuses on error paths, verbose output and lint/version/add/install/deps
dispatch, which are easy to leave uncovered.
"""
import io
import sys
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura import cli  # noqa: E402


def write(tmp_path, name, source):
    path = tmp_path / name
    path.write_text(source)
    return str(path)


def capture(func, *args, **kwargs):
    out, err = io.StringIO(), io.StringIO()
    with redirect_stdout(out), redirect_stderr(err):
        code = func(*args, **kwargs)
    return code, out.getvalue(), err.getvalue()


# ---------------------------------------------------------------------------
# Diagnostic helpers
# ---------------------------------------------------------------------------

def test_mutability_diagnostics_survives_recursion_error(monkeypatch):
    class Boom:
        def check_program(self, _ast):
            raise RecursionError()

    monkeypatch.setattr(cli, 'MutabilityChecker', Boom)
    assert cli._mutability_diagnostics(object()) == []


def test_rule_diagnostics_survives_recursion_error(monkeypatch):
    import aura.transpiler.rules as rules

    class Boom:
        def check_program(self, _ast, require_main=False):
            raise RecursionError()
        collector = None

    monkeypatch.setattr(rules, 'RuleChecker', Boom)
    assert cli._rule_diagnostics(object()) == []


def test_report_diagnostic_uses_fallback_path_when_no_location(capsys):
    class Diag:
        location = None

        def __str__(self):
            return 'thing failed'

    cli._report_diagnostic(Diag(), fallback_path='prog.aura')
    assert 'prog.aura: thing failed' in capsys.readouterr().err


def test_report_diagnostic_prints_plain_without_fallback(capsys):
    class Diag:
        location = object()

        def __str__(self):
            return 'located'

    cli._report_diagnostic(Diag())
    assert 'located' in capsys.readouterr().err


# ---------------------------------------------------------------------------
# cmd_transpile / cmd_check / cmd_format
# ---------------------------------------------------------------------------

def test_transpile_transform_failure(monkeypatch, tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'def main() { }\n')
    monkeypatch.setattr(cli.Transformer, 'transform',
                        lambda self, ast: (_ for _ in ()).throw(RuntimeError('boom')))
    code = cli.cmd_transpile(path)
    err = capsys.readouterr().err
    assert code == 2
    assert 'Error transpiling' in err


def test_transpile_verbose_writes_ast_to_stderr(tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'def main() { print(1) }\n')
    code, out, err = capture(cli.cmd_transpile, path, None, True)
    assert code == 0
    assert '# AST:' in err


def test_check_parse_error_returns_two(tmp_path, capsys):
    path = write(tmp_path, 'bad.aura', 'let = =\n')
    code, out, err = capture(cli.cmd_check, path)
    assert code == 2
    assert 'Error parsing' in err


def test_check_verbose_lists_bindings(tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'let total: int = 1\n')
    code, out, err = capture(cli.cmd_check, path, True)
    assert code in (0, 1)
    assert 'Inferred' in err


def test_check_reports_mutability_and_rules(tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'def main() {\n  let x = 1\n  x = 2\n}\n')
    code, out, err = capture(cli.cmd_check, path)
    assert code == 1
    assert 'FAIL' in err


def test_format_error_path(monkeypatch, tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'def main() { }\n')
    import aura.tools.formatter as formatter
    monkeypatch.setattr(formatter, 'format_aura',
                        lambda *a, **k: (_ for _ in ()).throw(RuntimeError('bad')))
    code, out, err = capture(cli.cmd_format, path)
    assert code == 2
    assert 'Error formatting' in err


def test_format_to_file(tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'def main() { print(1) }\n')
    out_path = tmp_path / 'out.aura'
    code, out, err = capture(cli.cmd_format, path, str(out_path))
    assert code == 0
    assert out_path.exists()


# ---------------------------------------------------------------------------
# cmd_lint
# ---------------------------------------------------------------------------

def test_lint_missing_file(capsys):
    code, out, err = capture(cli.cmd_lint, '/nope/missing.aura')
    assert code == 2
    assert 'File not found' in err


def test_lint_long_line(tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'let x = ' + '1' * 120 + '\n')
    code, out, err = capture(cli.cmd_lint, path)
    assert code == 1
    assert 'W001' in out


def test_lint_trailing_whitespace(tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'let x = 1   \n')
    code, out, err = capture(cli.cmd_lint, path)
    assert code == 1
    assert 'W002' in out


def test_lint_upper_case_variable(tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'let TOTAL = 1\n')
    code, out, err = capture(cli.cmd_lint, path)
    assert code == 1
    assert 'W003' in out


def test_lint_multiple_spaces_after_def(tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'def  main() { }\n')
    code, out, err = capture(cli.cmd_lint, path)
    assert code == 1
    assert 'W004' in out


def test_lint_clean_file(tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'def main() {\n  let x = 1\n}\n')
    code, out, err = capture(cli.cmd_lint, path)
    assert code == 0
    assert 'no style issues' in out


# ---------------------------------------------------------------------------
# cmd_run
# ---------------------------------------------------------------------------

def test_run_verbose_prints_generated_code(tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'def main() { print(7) }\n')
    code, out, err = capture(cli.cmd_run, path, True)
    assert code == 0
    assert '# Generated Python code:' in err
    assert '7' in out


def test_run_async_program(tmp_path, capsys):
    path = write(tmp_path, 'p.aura',
                 'from stdlib.asyncio import sleep\n'
                 'async def work() -> int { await sleep(0)\nreturn 5 }\n'
                 'async def main() { let x = await work()\nprint(x) }\n')
    code, out, err = capture(cli.cmd_run, path)
    assert code == 0
    assert '5' in out


def test_run_system_exit_none_is_zero(tmp_path):
    path = write(tmp_path, 'p.aura',
                 'def main() { if true { return } }\n')
    code, out, err = capture(cli.cmd_run, path)
    assert code == 0


# ---------------------------------------------------------------------------
# cmd_test
# ---------------------------------------------------------------------------

def test_test_verbose_marks_pass_and_fail(tmp_path, capsys):
    (tmp_path / 'good.aura').write_text('def main() { print(1) }\n')
    (tmp_path / 'bad.aura').write_text('def main() { print(oops_undefined) }\n')
    code, out, err = capture(cli.cmd_test, str(tmp_path), True)
    assert code == 1
    assert 'RUN' in err
    assert '1' in out  # at least one pass reported


def test_test_no_files_found(tmp_path, capsys):
    code, out, err = capture(cli.cmd_test, str(tmp_path), False, '*.nomatch')
    assert code == 2
    assert 'No *.nomatch files found' in err


# ---------------------------------------------------------------------------
# Dependency commands
# ---------------------------------------------------------------------------

def test_cmd_add_delegates(monkeypatch):
    import aura.tools.deps as deps
    calls = {}

    def fake_add(package, version, install=True):
        calls.update(package=package, version=version, install=install)
        return 0

    monkeypatch.setattr(deps, 'add_package', fake_add)
    assert cli.cmd_add('requests', '2.0', no_install=True) == 0
    assert calls == {'package': 'requests', 'version': '2.0', 'install': False}


def test_cmd_install_delegates(monkeypatch):
    import aura.tools.deps as deps
    captured = {}
    monkeypatch.setattr(deps, 'install_dependencies',
                        lambda upgrade=False: captured.setdefault('upgrade', upgrade) or 0)
    cli.cmd_install(upgrade=True)
    assert captured['upgrade'] is True


def test_cmd_deps_delegates(monkeypatch):
    import aura.tools.deps as deps
    monkeypatch.setattr(deps, 'list_dependencies', lambda: 0)
    assert cli.cmd_deps() == 0


def test_cmd_init_delegates(monkeypatch):
    import aura.tools.deps as deps
    captured = {}
    monkeypatch.setattr(deps, 'init_project',
                        lambda name='app': captured.setdefault('name', name) or 0)
    cli.cmd_init('myapp')
    assert captured['name'] == 'myapp'


# ---------------------------------------------------------------------------
# Version
# ---------------------------------------------------------------------------

def test_cmd_version_prints_current(capsys):
    code, out, err = capture(cli.cmd_version)
    assert code == 0
    assert out.strip()


def test_cmd_version_bump_patch(monkeypatch, capsys):
    import aura.tools.release as release
    monkeypatch.setattr(release, 'bump', lambda kind: '0.1.0a11')
    monkeypatch.setattr(release, 'set_version', lambda v: v)
    code, out, err = capture(cli.cmd_version, 'patch')
    assert code == 0
    assert '0.1.0a11' in out


def test_cmd_version_set_explicit(monkeypatch, capsys):
    import aura.tools.release as release
    monkeypatch.setattr(release, 'bump', lambda kind: 'unused')
    monkeypatch.setattr(release, 'set_version', lambda v: f'set {v}')
    code, out, err = capture(cli.cmd_version, '9.9.9')
    assert code == 0
    assert 'set 9.9.9' in out


# ---------------------------------------------------------------------------
# main dispatch / misc commands
# ---------------------------------------------------------------------------

def test_main_dispatches_lsp(monkeypatch):
    import aura.lsp.server as lsp
    monkeypatch.setattr(lsp, 'main', lambda: 0)
    assert cli.main(['lsp']) == 0


def test_main_dispatches_repl(monkeypatch):
    import aura.repl.engine as engine

    class FakeRepl:
        def run(self):
            return 0

    monkeypatch.setattr(engine, 'AuraREPL', lambda: FakeRepl())
    assert cli.main(['repl']) == 0


def test_main_lint_dispatch(tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'let TOTAL = 1\n')
    code, out, err = capture(cli.main, ['lint', path])
    assert code == 1


def test_cmd_repl_handles_eof(monkeypatch):
    import aura.repl.engine as engine

    class EofRepl:
        def run(self):
            raise EOFError()

    monkeypatch.setattr(engine, 'AuraREPL', lambda: EofRepl())
    assert cli.cmd_repl() == 0


def test_install_aura_imports_is_best_effort(monkeypatch, tmp_path):
    import aura.transpiler.importer as importer
    monkeypatch.setattr(importer, 'install_aura_import_hook',
                        lambda paths: (_ for _ in ()).throw(RuntimeError('x')))
    # Must not raise.
    cli._install_aura_imports(str(tmp_path / 'x.aura'))


def test_lint_internal_error(monkeypatch, tmp_path, capsys):
    path = write(tmp_path, 'p.aura', 'let TOTAL = 1\n')

    class BoomCollector:
        def __init__(self, _path):
            self.errors = []

        def add_warning(self, *a, **k):
            raise RuntimeError('nope')

        def format(self):
            return ''

    monkeypatch.setattr(cli, 'ErrorCollector', BoomCollector)
    code, out, err = capture(cli.cmd_lint, path)
    assert code == 2
    assert 'Error linting' in err


def test_test_internal_exception_is_reported(monkeypatch, tmp_path, capsys):
    (tmp_path / 'a.aura').write_text('def main() { }\n')
    import subprocess
    monkeypatch.setattr(subprocess, 'run',
                        lambda *a, **k: (_ for _ in ()).throw(RuntimeError('spawn fail')))
    code, out, err = capture(cli.cmd_test, str(tmp_path), True)
    assert code == 1
    assert 'spawn fail' in out


def test_test_timeout_is_reported(monkeypatch, tmp_path, capsys):
    (tmp_path / 'a.aura').write_text('def main() { }\n')
    import subprocess
    monkeypatch.setattr(subprocess, 'run',
                        lambda *a, **k: (_ for _ in ()).throw(subprocess.TimeoutExpired('x', 30)))
    code, out, err = capture(cli.cmd_test, str(tmp_path), True)
    assert code == 1
    assert 'timeout' in out


@pytest.mark.parametrize('subcommand', [
    ['transpile', '--help'],
    ['check', '--help'],
    ['format', '--help'],
    ['run', '--help'],
    ['test', '--help'],
    ['init', '--help'],
    ['add', '--help'],
    ['install', '--help'],
    ['deps', '--help'],
    ['version', '--help'],
    ['debug', '--help'],
])
def test_main_subcommand_help(subcommand, capsys):
    with pytest.raises(SystemExit) as excinfo:
        cli.main(subcommand)
    assert excinfo.value.code == 0


def test_main_debug_dispatch(tmp_path, monkeypatch):
    import aura.tools.debugger as debugger
    monkeypatch.setattr(debugger, 'run', lambda path, trace=False, show_code=False: 0)
    assert cli.main(['debug', str(tmp_path / 'x.aura')]) == 0


def test_main_deps_dispatch(monkeypatch):
    import aura.tools.deps as deps
    monkeypatch.setattr(deps, 'list_dependencies', lambda: 0)
    assert cli.main(['deps']) == 0


def test_main_install_dispatch(monkeypatch):
    import aura.tools.deps as deps
    monkeypatch.setattr(deps, 'install_dependencies', lambda upgrade=False: 0)
    assert cli.main(['install']) == 0


def test_main_init_dispatch(monkeypatch):
    import aura.tools.deps as deps
    monkeypatch.setattr(deps, 'init_project', lambda name='app': 0)
    assert cli.main(['init', 'demo']) == 0


def test_main_add_dispatch(monkeypatch):
    import aura.tools.deps as deps
    monkeypatch.setattr(deps, 'add_package',
                        lambda package, version, install=True: 0)
    assert cli.main(['add', 'requests', '--no-install']) == 0


def test_main_version_dispatch(monkeypatch, capsys):
    import aura.tools.release as release
    monkeypatch.setattr(release, 'get_version', lambda: '0.1.0a10')
    assert cli.main(['version']) == 0
    assert '0.1.0a10' in capsys.readouterr().out


@pytest.mark.parametrize('argv', [
    ['transpile', 'x.aura'],
    ['check', 'x.aura'],
    ['format', 'x.aura'],
    ['run', 'x.aura'],
    ['test', '.'],
])
def test_main_command_dispatch(monkeypatch, argv):
    name = argv[0]
    dispatched = {}

    def fake(*a, **k):
        dispatched['called'] = True
        return 0

    monkeypatch.setattr(cli, f'cmd_{name}', fake)
    assert cli.main(argv) == 0
    assert dispatched['called'] is True
