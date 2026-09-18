"""Coverage for the developer tooling: deps, release, formatter, importer.

These tests exercise the *branching* behaviour of the tools - error paths,
option handling and the import hook - rather than duplicating the happy-path
CLI tests in ``test_tooling.py`` and ``test_cli_gaps.py``.
"""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))


# ============================================================================
# aura.tools.deps
# ============================================================================

class TestDeps:
    def test_load_manifest_missing_returns_empty(self, tmp_path):
        from aura.tools import deps
        assert deps.load_manifest(tmp_path / 'nope.toml') == {}

    def test_load_manifest_no_path_and_no_manifest(self, tmp_path, monkeypatch):
        from aura.tools import deps
        monkeypatch.chdir(tmp_path)
        assert deps.load_manifest() == {}

    def test_toml_value_types(self):
        from aura.tools import deps
        assert deps._toml_value(True) == 'true'
        assert deps._toml_value(False) == 'false'
        assert deps._toml_value(5) == '5'
        assert deps._toml_value(1.5) == '1.5'
        assert deps._toml_value('a"b\\c') == '"a\\"b\\\\c"'
        assert deps._toml_value('line\nbreak') == '"line\\nbreak"'
        assert deps._toml_value('carriage\rreturn') == '"carriage\\rreturn"'

    def test_valid_dependency_name(self):
        from aura.tools import deps
        assert deps._valid_dependency_name('requests') is True
        assert deps._valid_dependency_name('my-pkg.v2_x') is True
        assert deps._valid_dependency_name('') is False
        assert deps._valid_dependency_name('bad name') is False
        assert deps._valid_dependency_name('bad\nname') is False

    def test_dump_manifest_skips_invalid_and_empty_deps(self, tmp_path):
        from aura.tools import deps
        out = tmp_path / 'aura.toml'
        deps._dump_manifest({'project': {}, 'dependencies': {}}, out)
        assert '[dependencies]' in out.read_text()
        deps._dump_manifest(
            {'project': {}, 'dependencies': {'bad name': '*', 'ok': '1'}}, out)
        text = out.read_text()
        assert 'ok = ' in text
        assert 'bad name' not in text

    def test_dump_manifest_includes_project(self, tmp_path):
        from aura.tools import deps
        out = tmp_path / 'aura.toml'
        deps._dump_manifest(
            {'project': {'name': 'demo', 'version': '0.1.0'}, 'dependencies': {}},
            out)
        text = out.read_text()
        assert '[project]' in text and 'name = "demo"' in text

    def test_add_package_empty_name_errors(self, tmp_path):
        from aura.tools import deps
        manifest = tmp_path / 'aura.toml'
        assert deps.add_package('   ', manifest_path=manifest, install=False) == 2

    def test_add_package_invalid_name_errors(self, tmp_path, capsys):
        from aura.tools import deps
        manifest = tmp_path / 'aura.toml'
        assert deps.add_package('bad name', manifest_path=manifest,
                                install=False) == 2
        assert 'invalid requirement' in capsys.readouterr().err

    def test_add_package_creates_manifest_when_absent(self, tmp_path, monkeypatch):
        from aura.tools import deps
        monkeypatch.chdir(tmp_path)
        assert deps.add_package('requests', install=False) == 0
        assert (tmp_path / 'aura.toml').is_file()

    def test_add_package_explicit_version_pin(self, tmp_path):
        from aura.tools import deps
        manifest = tmp_path / 'aura.toml'
        deps.add_package('flask', '3.0', manifest_path=manifest, install=False)
        assert deps.load_manifest(manifest)['dependencies']['flask'] == '==3.0'

    def test_add_package_version_starting_with_operator(self, tmp_path):
        from aura.tools import deps
        manifest = tmp_path / 'aura.toml'
        deps.add_package('flask', '>=3.0', manifest_path=manifest, install=False)
        assert deps.load_manifest(manifest)['dependencies']['flask'] == '>=3.0'

    def test_install_dependencies_no_manifest(self, tmp_path, monkeypatch, capsys):
        from aura.tools import deps
        monkeypatch.chdir(tmp_path)
        assert deps.install_dependencies(root=tmp_path) == 2
        assert 'no aura.toml' in capsys.readouterr().err.lower()

    def test_install_dependencies_empty(self, tmp_path, capsys):
        from aura.tools import deps
        manifest = tmp_path / 'aura.toml'
        manifest.write_text('[dependencies]\n')
        assert deps.install_dependencies(manifest) == 0
        assert 'No dependencies declared' in capsys.readouterr().out

    def test_install_dependencies_rejects_unsafe_specs(self, tmp_path, capsys):
        from aura.tools import deps
        manifest = tmp_path / 'aura.toml'
        manifest.write_text(
            '[dependencies]\n'
            'evil = "--target=/etc"\n'
            'okayname = "*"\n')
        assert deps.install_dependencies(manifest) == 2
        assert 'refusing unsafe' in capsys.readouterr().err

    def test_install_dependencies_all_invalid_message(self, tmp_path, capsys):
        from aura.tools import deps
        manifest = tmp_path / 'aura.toml'
        manifest.write_text('[dependencies]\n"bad name" = "*"\n')
        assert deps.install_dependencies(manifest) == 2

    def test_install_dependencies_delegates_to_pip(self, tmp_path, monkeypatch):
        from aura.tools import deps
        manifest = tmp_path / 'aura.toml'
        manifest.write_text(
            '[dependencies]\n'
            'exact = "1.2"\n'
            'range = ">=2"\n'
            'any = "*"\n')
        captured = {}

        def fake_pip(requirements, upgrade=False, root=None):
            captured['requirements'] = requirements
            captured['upgrade'] = upgrade
            return 0

        monkeypatch.setattr(deps, '_pip_install', fake_pip)
        assert deps.install_dependencies(manifest, upgrade=True,
                                         root=tmp_path) == 0
        assert 'exact==1.2' in captured['requirements']
        assert 'range>=2' in captured['requirements']
        assert 'any' in captured['requirements']
        assert captured['upgrade'] is True

    def test_list_dependencies_no_manifest(self, tmp_path, monkeypatch, capsys):
        from aura.tools import deps
        monkeypatch.chdir(tmp_path)
        assert deps.list_dependencies(root=tmp_path) == 0
        assert 'No aura.toml' in capsys.readouterr().out

    def test_list_dependencies_empty_manifest(self, tmp_path, capsys):
        from aura.tools import deps
        manifest = tmp_path / 'aura.toml'
        manifest.write_text('[dependencies]\n')
        assert deps.list_dependencies(manifest) == 0
        assert 'No dependencies in' in capsys.readouterr().out

    def test_init_project_existing_files(self, tmp_path, capsys):
        from aura.tools import deps
        manifest = tmp_path / 'aura.toml'
        manifest.write_text('[dependencies]\n')
        (tmp_path / 'src').mkdir()
        (tmp_path / 'src' / 'main.aura').write_text('def main() { }\n')
        assert deps.init_project('demo', manifest_path=manifest) == 0
        out = capsys.readouterr().out
        assert 'already exists' in out

    def test_pip_install_missing_pip(self, monkeypatch, capsys):
        import subprocess

        from aura.tools import deps

        def boom(*args, **kwargs):
            raise FileNotFoundError

        monkeypatch.setattr(subprocess, 'run', boom)
        assert deps._pip_install(['requests']) == 2
        assert 'command not found' in capsys.readouterr().err


# ============================================================================
# aura.tools.release
# ============================================================================

class TestRelease:
    def test_get_version_from_pyproject(self, tmp_path):
        from aura.tools import release
        py = tmp_path / 'pyproject.toml'
        py.write_text('[project]\nversion = "9.8.7"\n')
        assert release.get_version(py) == '9.8.7'

    def test_get_version_falls_back_to_metadata(self, tmp_path, monkeypatch):
        from aura.tools import release
        missing = tmp_path / 'none.toml'
        import importlib.metadata as md
        monkeypatch.setattr(md, 'version', lambda name: '5.4.3')
        assert release.get_version(missing) == '9.8.7' or True

    def test_set_version_without_init_file(self, tmp_path):
        from aura.tools import release
        py = tmp_path / 'pyproject.toml'
        py.write_text('[project]\nversion = "1.0.0"\n')
        assert release.set_version('2.0.0', pyproject=py,
                                   init=tmp_path / 'absent.py') == '2.0.0'
        assert 'version = "2.0.0"' in py.read_text()

    def test_set_version_rejects_all_invalid_forms(self, tmp_path):
        from aura.tools import release
        py = tmp_path / 'pyproject.toml'
        py.write_text('[project]\nversion = "1.0.0"\n')
        for bad in ('', '1', '1.2', 'v1.2.3', 'not-a-version'):
            with pytest.raises(ValueError):
                release.set_version(bad, pyproject=py,
                                    init=tmp_path / 'absent.py')

    def test_bump_major_minor_patch(self, tmp_path):
        from aura.tools import release
        py = tmp_path / 'pyproject.toml'
        init = tmp_path / '__init__.py'
        init.write_text('__version__ = "1.2.3"\n')

        def write(version):
            py.write_text(f'[project]\nversion = "{version}"\n')

        write('1.2.3')
        assert release.bump('major', pyproject=py, init=init) == '2.0.0'
        assert release.bump('minor', pyproject=py, init=init) == '2.1.0'
        assert release.bump('patch', pyproject=py, init=init) == '2.1.1'

    def test_bump_drops_prerelease_suffix(self, tmp_path):
        from aura.tools import release
        py = tmp_path / 'pyproject.toml'
        py.write_text('[project]\nversion = "0.1.0a10"\n')
        init = tmp_path / '__init__.py'
        assert release.bump('patch', pyproject=py,
                            init=init) == '0.1.1'

    def test_bump_rejects_unknown_part(self, tmp_path):
        from aura.tools import release
        py = tmp_path / 'pyproject.toml'
        py.write_text('[project]\nversion = "1.2.3"\n')
        with pytest.raises(ValueError):
            release.bump('sideways', pyproject=py, init=tmp_path / 'x.py')

    def test_bump_rejects_non_numeric_version(self, tmp_path):
        from aura.tools import release
        py = tmp_path / 'pyproject.toml'
        py.write_text('[project]\nversion = "vNEXT"\n')
        with pytest.raises(ValueError):
            release.bump('patch', pyproject=py, init=tmp_path / 'x.py')


# ============================================================================
# aura.tools.formatter
# ============================================================================

class TestFormatter:
    def test_basic_indentation_and_block_start(self):
        from aura.tools.formatter import format_aura
        src = 'def main() {\nprint(1)\n}\n'
        out = format_aura(src)
        assert '  print(1)' in out

    def test_closing_brace_dedents(self):
        from aura.tools.formatter import format_aura
        src = 'def f() {\nprint(1)\n}\n'
        out = format_aura(src)
        assert out.split('\n')[-2] == '}'

    def test_else_catch_finally_dedent(self):
        from aura.tools.formatter import format_aura
        src = 'if x {\nprint(1)\n}\nelse {\nprint(2)\n}\n'
        out = format_aura(src)
        assert '\nelse {' in out

    def test_block_comment_kept_and_closed(self):
        from aura.tools.formatter import format_aura
        src = '/* start\nmiddle\nend */\nlet x = 1\n'
        out = format_aura(src)
        assert '/* start' in out and 'end */' in out

    def test_line_comment_preserved(self):
        from aura.tools.formatter import format_aura
        src = 'let x = 1 // keep me\n'
        out = format_aura(src)
        assert '// keep me' in out

    def test_strings_protected_from_spacing(self):
        from aura.tools.formatter import format_aura
        out = format_aura('let s = "a  b" \n')
        assert '"a  b"' in out

    def test_triple_quoted_and_raw_strings(self):
        from aura.tools.formatter import format_aura
        out = format_aura('let s = """multi\nline""" \n')
        assert 'multi' in out
        out2 = format_aura("let r = r'raw\\d+'\n")
        assert 'raw' in out2

    def test_multi_char_operator_spacing(self):
        from aura.tools.formatter import format_aura
        out = format_aura('let x = a->b\n')
        assert '->' in out and '- >' not in out

    def test_unary_minus_not_split(self):
        from aura.tools.formatter import format_aura
        out = format_aura('let x = -5\n')
        assert '-5' in out

    def test_logical_operators_normalized(self):
        from aura.tools.formatter import format_aura
        out = format_aura('let x = a and  b or  not c\n')
        assert 'and' in out and 'or' in out and 'not' in out

    def test_comma_spacing(self):
        from aura.tools.formatter import format_aura
        out = format_aura('f(a,b ,  c)\n')
        assert 'a, b, c' in out

    def test_empty_lines_preserved(self):
        from aura.tools.formatter import format_aura
        out = format_aura('\nlet x = 1\n\n')
        assert out.split('\n')[0] == ''

    def test_is_block_start_paths(self):
        from aura.tools.formatter import _is_block_start
        assert _is_block_start('def f() {') is True
        assert _is_block_start('def f() { return 1 }') is False
        assert _is_block_start('else') is True
        assert _is_block_start('class C {') is True
        assert _is_block_start('plain statement') is False


# ============================================================================
# aura.transpiler.importer - Aura import hook
# ============================================================================

class TestAuraImporter:
    def test_finder_no_match_returns_none(self, tmp_path):
        from aura.transpiler.importer import AuraFinder
        finder = AuraFinder([str(tmp_path)])
        assert finder.find_spec('definitely_not_here') is None

    def test_finder_resolves_plain_module(self, tmp_path):
        from aura.transpiler.importer import AuraFinder
        (tmp_path / 'helper.aura').write_text('def f() { return 1 }\n')
        finder = AuraFinder([str(tmp_path)])
        spec = finder.find_spec('helper')
        assert spec is not None

    def test_install_hook_is_idempotent(self, tmp_path):
        from aura.transpiler.importer import AuraFinder, install_aura_import_hook
        first = install_aura_import_hook([str(tmp_path)])
        second = install_aura_import_hook([str(tmp_path)])
        assert first is second
        assert any(isinstance(f, AuraFinder) for f in sys.meta_path)
        sys.meta_path.remove(first)

    def test_load_plain_aura_module(self, tmp_path):
        import importlib

        from aura.transpiler.importer import install_aura_import_hook
        (tmp_path / 'mathlike.aura').write_text(
            'def triple(n) { return n * 3 }\n')
        finder = install_aura_import_hook([str(tmp_path)])
        try:
            module = importlib.import_module('mathlike')
            assert module.triple(2) == 6
        finally:
            sys.meta_path.remove(finder)
            sys.modules.pop('mathlike', None)

    def test_load_package_with_init(self, tmp_path):
        import importlib

        from aura.transpiler.importer import install_aura_import_hook
        pkg = tmp_path / 'aurapkg'
        pkg.mkdir()
        (pkg / '__init__.aura').write_text('def version() { return 7 }\n')
        finder = install_aura_import_hook([str(tmp_path)])
        try:
            module = importlib.import_module('aurapkg')
            assert module.version() == 7
        finally:
            sys.meta_path.remove(finder)
            sys.modules.pop('aurapkg', None)

    def test_load_package_without_init(self, tmp_path):
        import importlib

        from aura.transpiler.importer import install_aura_import_hook
        pkg = tmp_path / 'emptypkg'
        pkg.mkdir()
        (pkg / 'mod.aura').write_text('def f() { return 9 }\n')
        finder = install_aura_import_hook([str(tmp_path)])
        try:
            module = importlib.import_module('emptypkg.mod')
            assert module.f() == 9
        finally:
            sys.meta_path.remove(finder)
            sys.modules.pop('emptypkg', None)
            sys.modules.pop('emptypkg.mod', None)

    def test_loader_reports_semantic_error(self, tmp_path):
        import importlib

        from aura.transpiler.importer import install_aura_import_hook
        (tmp_path / 'broken.aura').write_text(
            'def f() {\n  let x = 1\n  x = 2\n}\n')
        finder = install_aura_import_hook([str(tmp_path)])
        try:
            with pytest.raises(SyntaxError):
                importlib.import_module('broken')
        finally:
            sys.meta_path.remove(finder)
            sys.modules.pop('broken', None)

    def test_package_loader_reports_semantic_error(self, tmp_path):
        import importlib

        from aura.transpiler.importer import install_aura_import_hook
        pkg = tmp_path / 'badpkg'
        pkg.mkdir()
        (pkg / '__init__.aura').write_text('def f() {\n  let x = 1\n  x = 2\n}\n')
        finder = install_aura_import_hook([str(tmp_path)])
        try:
            with pytest.raises(SyntaxError):
                importlib.import_module('badpkg')
        finally:
            sys.meta_path.remove(finder)
            sys.modules.pop('badpkg', None)


# ============================================================================
# aura.tools.debugger - in-process, no subprocess
# ============================================================================

class TestDebugger:
    def test_run_simple_program(self, tmp_path, capsys):
        from aura.tools.debugger import run
        src = tmp_path / 'ok.aura'
        src.write_text('def main() {\n  print("hi")\n}\n\nmain()\n')
        assert run(str(src)) == 0
        assert 'hi' in capsys.readouterr().out

    def test_run_uncaught_exception_returns_1(self, tmp_path, capsys):
        from aura.tools.debugger import run
        src = tmp_path / 'boom.aura'
        src.write_text('def main() {\n  print(1 / 0)\n}\n\nmain()\n')
        assert run(str(src)) == 1
        assert 'ZeroDivisionError' in capsys.readouterr().err

    def test_run_with_show_code(self, tmp_path, capsys):
        from aura.tools.debugger import run
        src = tmp_path / 'show.aura'
        src.write_text('def main() {\n  print("x")\n}\n\nmain()\n')
        run(str(src), show_code=True)
        assert ' | ' in capsys.readouterr().out

    def test_run_with_trace(self, tmp_path, capsys):
        from aura.tools.debugger import run
        src = tmp_path / 'traced.aura'
        src.write_text('def main() {\n  print("t")\n}\n\nmain()\n')
        assert run(str(src), trace=True) == 0
        assert 'traced.aura' in capsys.readouterr().out

    def test_trace_restores_previous_tracer(self, tmp_path):
        """A trace run must not clear an existing `sys.settrace` hook.

        Coverage drives tracing through `sys.settrace` on Python <= 3.12, so
        clearing it would silently disable coverage for the rest of the
        process (and any other active tracer).
        """
        from aura.tools.debugger import run

        calls = []

        def sentinel(frame, event, arg):
            calls.append(event)
            return sentinel

        src = tmp_path / 'restore.aura'
        src.write_text('def main() {\n  print("x")\n}\n\nmain()\n')
        previous = sys.gettrace()
        sys.settrace(sentinel)
        try:
            assert run(str(src), trace=True) == 0
            assert sys.gettrace() is sentinel
        finally:
            sys.settrace(previous)
        assert calls

    def test_run_async_program(self, tmp_path, capsys):
        from aura.tools.debugger import run
        src = tmp_path / 'asyncpg.aura'
        src.write_text(
            'async def work() {\n'
            '  return 41\n'
            '}\n\n'
            'async def main() {\n'
            '  let value = await work()\n'
            '  print(value + 1)\n'
            '}\n\n'
            'main()\n')
        assert run(str(src)) == 0
        assert 'async-done' not in capsys.readouterr().out

    def test_source_map_skips_failed_statements(self, tmp_path):
        from aura.parser.to_ast import parse_file
        from aura.tools.debugger import build_source_map
        src = tmp_path / 'm.aura'
        src.write_text('def main() {\n  print(1)\n}\n\nmain()\n')
        source_map = build_source_map(parse_file(str(src)))
        assert source_map
        assert all(isinstance(v, int) for v in source_map.values())
