"""Tests for `aura venv`, `aura add`, and the dependency manifest.

The dependency tooling records Python requirements in ``aura.toml`` and installs
them into the project's environment: the ``.venv`` when one exists, otherwise
the current interpreter. Tests use ``--no-install`` and a stubbed installer so
no network access or real installation happens.
"""
import os
import shutil
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.tools import deps  # noqa: E402


@pytest.fixture
def project(tmp_path, monkeypatch):
    """A fresh project directory with a manifest, isolated from the repo."""
    monkeypatch.chdir(tmp_path)
    monkeypatch.delenv('AURA_VENV', raising=False)
    monkeypatch.setattr(deps.style, 'enabled', False)
    assert deps.init_project('myapp') == 0
    return tmp_path


def manifest_text(root):
    return (root / 'aura.toml').read_text(encoding='utf-8')


# ============================================================================
# Manifest
# ============================================================================

class TestManifest:
    def test_init_creates_manifest_and_main(self, project):
        assert (project / 'aura.toml').is_file()
        assert (project / 'src' / 'main.aura').is_file()

    def test_init_is_idempotent(self, project):
        assert deps.init_project('myapp') == 0

    def test_manifest_roundtrip(self, project):
        data = deps.load_manifest(project / 'aura.toml')
        assert data['project']['name'] == 'myapp'
        assert data['project']['version'] == '0.1.0'

    def test_find_manifest_walks_upwards(self, project):
        nested = project / 'src' / 'deep'
        nested.mkdir(parents=True)
        assert deps.find_manifest(nested) == project / 'aura.toml'

    def test_project_root_is_manifest_dir(self, project):
        nested = project / 'src'
        assert deps.project_root(nested) == project


# ============================================================================
# aura add
# ============================================================================

class TestAddPackage:
    def test_add_records_a_plain_dependency(self, project):
        assert deps.add_package('requests', install=False) == 0
        data = deps.load_manifest(project / 'aura.toml')
        assert data['dependencies']['requests'] == '*'

    def test_add_records_a_specifier(self, project):
        assert deps.add_package('requests>=2.28', install=False) == 0
        data = deps.load_manifest(project / 'aura.toml')
        assert data['dependencies']['requests'] == '>=2.28'

    def test_add_with_explicit_version_pins(self, project):
        assert deps.add_package('requests', version='2.31.0', install=False) == 0
        data = deps.load_manifest(project / 'aura.toml')
        assert data['dependencies']['requests'] == '==2.31.0'

    def test_add_with_operator_version(self, project):
        assert deps.add_package('requests', version='>=2.0', install=False) == 0
        data = deps.load_manifest(project / 'aura.toml')
        assert data['dependencies']['requests'] == '>=2.0'

    def test_add_dev_dependency(self, project):
        assert deps.add_package('pytest', install=False, dev=True) == 0
        data = deps.load_manifest(project / 'aura.toml')
        assert 'pytest' not in data['dependencies']
        assert data['dependencies']['dev']['pytest'] == '*'

    def test_dev_and_runtime_coexist(self, project):
        deps.add_package('requests', install=False)
        deps.add_package('pytest', install=False, dev=True)
        data = deps.load_manifest(project / 'aura.toml')
        assert data['dependencies']['requests'] == '*'
        assert data['dependencies']['dev']['pytest'] == '*'

    def test_add_extras(self, project):
        assert deps.add_package('requests[security]>=2', install=False) == 0
        text = manifest_text(project)
        assert 'requests' in text
        assert '[security]' in text

    def test_add_updates_an_existing_dependency(self, project):
        deps.add_package('requests', install=False)
        deps.add_package('requests>=3', install=False)
        data = deps.load_manifest(project / 'aura.toml')
        assert data['dependencies']['requests'] == '>=3'

    def test_add_rejects_an_invalid_name(self, project):
        assert deps.add_package('bad name', install=False) == 2

    def test_add_rejects_an_invalid_specifier(self, project):
        assert deps.add_package('requests; rm -rf /', install=False) == 2

    def test_add_rejects_a_pip_option_injection(self, project):
        assert deps.add_package('requests --target=/etc', install=False) == 2

    def test_add_creates_a_manifest_when_missing(self, tmp_path, monkeypatch):
        monkeypatch.chdir(tmp_path)
        monkeypatch.setattr(deps.style, 'enabled', False)
        assert deps.add_package('requests', install=False) == 0
        assert (tmp_path / 'aura.toml').is_file()

    def test_manifest_is_sorted_and_stable(self, project):
        deps.add_package('zope', install=False)
        deps.add_package('attrs', install=False)
        text = manifest_text(project)
        assert text.index('attrs') < text.index('zope')

    def test_dependency_name_normalisation_is_preserved(self, project):
        deps.add_package('Pillow', install=False)
        data = deps.load_manifest(project / 'aura.toml')
        assert 'Pillow' in data['dependencies']


# ============================================================================
# parse_requirement
# ============================================================================

class TestParseRequirement:
    @pytest.mark.parametrize('text,name,spec', [
        ('requests', 'requests', '*'),
        ('requests>=2.28', 'requests', '>=2.28'),
        ('requests==2.31.0', 'requests', '==2.31.0'),
        ('pyyaml>=6,<7', 'pyyaml', '>=6,<7'),
        ('requests[security]', 'requests', '[security]'),
        ('requests[security]>=2', 'requests', '[security]>=2'),
    ])
    def test_valid(self, text, name, spec):
        assert deps.parse_requirement(text) == (name, spec)

    @pytest.mark.parametrize('text', [
        '', '   ', 'bad name', 'req; rm -rf /', 'req --target=/etc', '=1.0',
    ])
    def test_invalid(self, text):
        assert deps.parse_requirement(text) == (None, None)


# ============================================================================
# Specifier safety
# ============================================================================

class TestSpecifierSafety:
    @pytest.mark.parametrize('spec', ['*', '', '>=1', '==1.2.3', '~=1.4',
                                      '>=1,<2', '!=1.0', '<=2.0'])
    def test_valid_specifiers(self, spec):
        assert deps._valid_specifier(spec)

    @pytest.mark.parametrize('spec', [
        '--target=/etc', '-r requirements.txt', '>=1; rm -rf /',
        '>=1 && echo hi', '>=1 | cat', '$(whoami)', '`id`', '>=1\n>=2',
    ])
    def test_rejects_injection(self, spec):
        assert not deps._valid_specifier(spec)


# ============================================================================
# aura remove
# ============================================================================

class TestRemovePackage:
    def test_remove_runtime(self, project):
        deps.add_package('requests', install=False)
        assert deps.remove_package('requests') == 0
        data = deps.load_manifest(project / 'aura.toml')
        assert 'requests' not in data['dependencies']

    def test_remove_dev(self, project):
        deps.add_package('pytest', install=False, dev=True)
        assert deps.remove_package('pytest') == 0
        data = deps.load_manifest(project / 'aura.toml')
        assert 'dev' not in data['dependencies']

    def test_remove_leaves_others(self, project):
        deps.add_package('requests', install=False)
        deps.add_package('attrs', install=False)
        deps.remove_package('requests')
        data = deps.load_manifest(project / 'aura.toml')
        assert list(data['dependencies']) == ['attrs']

    def test_remove_unknown_returns_1(self, project):
        assert deps.remove_package('nope') == 1

    def test_remove_without_manifest(self, tmp_path, monkeypatch):
        monkeypatch.chdir(tmp_path)
        monkeypatch.setattr(deps.style, 'enabled', False)
        assert deps.remove_package('x') == 2


# ============================================================================
# Virtual environment
# ============================================================================

class TestVenv:
    def test_venv_dir_defaults_to_dot_venv(self, project):
        assert deps.venv_dir(project) == project / '.venv'

    def test_venv_dir_env_override(self, project, monkeypatch):
        monkeypatch.setenv('AURA_VENV', '/tmp/custom-venv')
        assert deps.venv_dir(project) == Path('/tmp/custom-venv')

    def test_no_venv_initially(self, project):
        assert not deps.venv_exists(project)
        assert deps.venv_python(project) is None

    def test_environment_python_falls_back_to_current(self, project):
        assert deps.environment_python(project) == sys.executable

    def test_environment_python_prefers_venv(self, project):
        python = deps.venv_python(project)
        if python is None:
            # Simulate a venv without creating a real one.
            binary = project / '.venv' / 'bin'
            binary.mkdir(parents=True)
            fake = binary / 'python'
            fake.write_text('#!/bin/sh\n')
            os.chmod(fake, 0o755)
        assert deps.environment_python(project) == str(deps.venv_python(project))

    def test_create_venv(self, project):
        assert deps.create_venv(root=project, install=False) == 0
        assert deps.venv_exists(project)

    def test_create_venv_is_idempotent(self, project):
        deps.create_venv(root=project, install=False)
        assert deps.create_venv(root=project, install=False) == 0

    def test_create_venv_force_recreates(self, project):
        deps.create_venv(root=project, install=False)
        marker = project / '.venv' / 'marker'
        marker.write_text('x')
        assert deps.create_venv(root=project, install=False, force=True) == 0
        assert not marker.exists()

    def test_venv_info_runs(self, project, capsys):
        assert deps.venv_info(root=project) == 0
        assert 'Environment' in capsys.readouterr().out

    def test_venv_info_with_venv(self, project, capsys):
        deps.create_venv(root=project, install=False)
        assert deps.venv_info(root=project) == 0
        assert '.venv' in capsys.readouterr().out

    def test_shell_into_venv_without_venv_returns_2(self, project):
        assert deps.shell_into_venv(root=project) == 2

    def test_shell_into_venv_prints_activation(self, project, capsys):
        deps.create_venv(root=project, install=False)
        assert deps.shell_into_venv(root=project) == 0
        assert 'activate' in capsys.readouterr().out

    def test_remove_venv(self, project):
        deps.create_venv(root=project, install=False)
        assert deps.remove_venv(root=project) == 0
        assert not deps.venv_exists(project)

    def test_remove_missing_venv_is_ok(self, project):
        assert deps.remove_venv(root=project) == 0

    def test_activate_hint_mentions_activate(self, project):
        assert 'activate' in deps.activate_hint(project / '.venv')


# ============================================================================
# Install (with a stubbed pip)
# ============================================================================

class TestInstall:
    def test_install_runs_pip_with_requirements(self, project, monkeypatch):
        deps.add_package('requests>=2.28', install=False)
        captured = {}

        def fake_run(cmd, **kwargs):
            captured['cmd'] = cmd
            return 0

        monkeypatch.setattr(deps, '_run', fake_run)
        assert deps.install_dependencies(root=project) == 0
        assert 'requests>=2.28' in captured['cmd']
        assert captured['cmd'][0] == sys.executable

    def test_install_includes_dev_by_default(self, project, monkeypatch):
        deps.add_package('pytest', install=False, dev=True)
        captured = {}

        def fake_run(cmd, **kwargs):
            captured['cmd'] = cmd
            return 0

        monkeypatch.setattr(deps, '_run', fake_run)
        deps.install_dependencies(root=project)
        assert 'pytest' in captured['cmd']

    def test_install_can_exclude_dev(self, project, monkeypatch):
        deps.add_package('pytest', install=False, dev=True)
        deps.add_package('requests', install=False)
        captured = {}

        def fake_run(cmd, **kwargs):
            captured['cmd'] = cmd
            return 0

        monkeypatch.setattr(deps, '_run', fake_run)
        deps.install_dependencies(root=project, include_dev=False)
        assert 'pytest' not in captured['cmd']
        assert 'requests' in captured['cmd']

    def test_install_without_manifest_returns_2(self, tmp_path, monkeypatch):
        monkeypatch.chdir(tmp_path)
        monkeypatch.setattr(deps.style, 'enabled', False)
        assert deps.install_dependencies(root=tmp_path) == 2

    def test_install_with_no_dependencies_is_ok(self, project):
        assert deps.install_dependencies(root=project) == 0

    def test_install_refuses_unsafe_entry(self, project, monkeypatch):
        # Write a manifest with an injected specifier directly.
        (project / 'aura.toml').write_text(
            '[dependencies]\nrequests = "--target=/etc"\n', encoding='utf-8')
        monkeypatch.setattr(deps, '_run', lambda *a, **k: 0)
        assert deps.install_dependencies(root=project) == 2

    def test_install_uses_venv_interpreter(self, project, monkeypatch):
        deps.add_package('requests', install=False)
        binary = project / '.venv' / 'bin'
        binary.mkdir(parents=True)
        fake = binary / 'python'
        fake.write_text('#!/bin/sh\n')
        os.chmod(fake, 0o755)
        captured = {}
        monkeypatch.setattr(deps, '_run', lambda cmd, **k: captured.update(cmd=cmd) or 0)
        deps.install_dependencies(root=project)
        assert str(fake) in captured['cmd'][0]


# ============================================================================
# aura deps / lock / doctor
# ============================================================================

class TestDepsListing:
    def test_list_dependencies_without_manifest(self, tmp_path, monkeypatch, capsys):
        monkeypatch.chdir(tmp_path)
        monkeypatch.setattr(deps.style, 'enabled', False)
        assert deps.list_dependencies(root=tmp_path) == 0
        assert 'No aura.toml' in capsys.readouterr().out

    def test_list_empty(self, project, capsys):
        assert deps.list_dependencies(root=project) == 0
        assert 'No dependencies' in capsys.readouterr().out

    def test_list_shows_groups(self, project, capsys, monkeypatch):
        deps.add_package('requests', install=False)
        deps.add_package('pytest', install=False, dev=True)
        monkeypatch.setattr(deps, '_installed_versions', lambda names, r=None: {})
        assert deps.list_dependencies(root=project) == 0
        out = capsys.readouterr().out
        assert 'runtime' in out
        assert 'dev' in out


class TestLock:
    def test_lock_with_nothing_installed(self, project, monkeypatch, capsys):
        deps.add_package('requests', install=False)
        monkeypatch.setattr(deps, '_installed_versions', lambda names, r=None: {})
        assert deps.write_lock(root=project) == 1

    def test_lock_writes_versions(self, project, monkeypatch, capsys):
        deps.add_package('requests', install=False)
        monkeypatch.setattr(deps, '_installed_versions',
                            lambda names, r=None: {str(n).lower(): '2.31.0' for n in names})
        assert deps.write_lock(root=project) == 0
        text = (project / 'aura.lock').read_text(encoding='utf-8')
        assert 'requests = "2.31.0"' in text
        assert '[runtime]' in text

    def test_lock_separates_groups(self, project, monkeypatch):
        deps.add_package('requests', install=False)
        deps.add_package('pytest', install=False, dev=True)
        monkeypatch.setattr(deps, '_installed_versions', lambda names, r=None: {str(n).lower(): '1.0' for n in names})
        deps.write_lock(root=project)
        text = (project / 'aura.lock').read_text(encoding='utf-8')
        assert '[runtime]' in text
        assert '[dev]' in text


class TestDoctor:
    def test_doctor_reports_missing_dependency(self, project, capsys,
                                               monkeypatch):
        deps.add_package('requests', install=False)
        monkeypatch.setattr(deps, '_installed_versions', lambda names, r=None: {})
        assert deps.doctor(root=project) == 1
        assert 'not installed' in capsys.readouterr().out

    def test_doctor_passes_when_installed(self, project, capsys, monkeypatch):
        deps.add_package('requests', install=False)
        monkeypatch.setattr(deps, '_installed_versions',
                            lambda names, r=None: {str(n).lower(): '2.31.0' for n in names})
        assert deps.doctor(root=project) == 0
        assert 'All checks passed' in capsys.readouterr().out

    def test_doctor_notes_missing_venv(self, project, capsys):
        assert deps.doctor(root=project) == 0
        assert 'no virtual environment' in capsys.readouterr().out


# ============================================================================
# CLI wiring
# ============================================================================

class TestCliWiring:
    def test_venv_subcommand_choices(self):
        from aura.cli import build_parser
        parser = build_parser()
        for action in ('init', 'info', 'shell', 'remove'):
            args = parser.parse_args(['venv', action])
            assert args.cmd == 'venv'
            assert args.action == action

    def test_venv_defaults_to_init(self):
        from aura.cli import build_parser
        args = build_parser().parse_args(['venv'])
        assert args.action == 'init'

    def test_add_dev_flag(self):
        from aura.cli import build_parser
        args = build_parser().parse_args(['add', 'pytest', '-D'])
        assert args.dev is True

    def test_remove_command(self):
        from aura.cli import build_parser
        args = build_parser().parse_args(['remove', 'requests', '--uninstall'])
        assert args.cmd == 'remove'
        assert args.uninstall is True

    def test_deps_lock_flag(self):
        from aura.cli import build_parser
        assert build_parser().parse_args(['deps', '--lock']).lock is True

    def test_doctor_command(self):
        from aura.cli import build_parser
        assert build_parser().parse_args(['doctor']).cmd == 'doctor'

    def test_run_command_parses(self):
        from aura.cli import _parse_run_args
        # Recognized `run` options are parsed wherever they appear; the first
        # non-option token is the file, and the rest go to `main(args)`.
        path, verbose, no_main, program_args = _parse_run_args(
            ['run', '-v', 'app.aura', '--', '--x'])
        assert path == 'app.aura'
        assert verbose is True
        assert no_main is False
        assert program_args == ['--x']

    def test_venv_lists_all_commands(self):
        from aura.cli import build_parser
        # Every command dispatched by main() must be parseable.
        parser = build_parser()
        invocations = [
            ['transpile', 'x.aura'], ['check', 'x.aura'], ['format', 'x.aura'],
            ['lint', 'x.aura'], ['repl'], ['test'], ['init'],
            ['add', 'requests'], ['remove', 'requests'], ['install'],
            ['deps'], ['venv'], ['doctor'], ['version'], ['debug', 'x.aura'],
            ['lsp'],
        ]
        for argv in invocations:
            # `run` is excluded: its trailing REMAINDER makes a bare path
            # acceptable, but the path is required, so it is covered elsewhere.
            assert parser.parse_args(argv).cmd == argv[0]