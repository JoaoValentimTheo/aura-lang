"""Aura dependency and virtual-environment tooling.

An Aura project declares its Python dependencies in ``aura.toml``:

```toml
[project]
name = "my-app"
version = "0.1.0"

[dependencies]
requests = ">=2.28"
pyyaml = "*"

[dependencies.dev]
pytest = ">=8"
```

Because Aura transpiles to Python and imports PyPI packages directly, a
dependency is just a Python distribution. ``aura add`` records it in the
manifest, ``aura install`` syncs it into the project's environment, and
``aura venv`` manages that environment (``.venv`` by default).

Every command installs into the *project's* interpreter: the one in ``.venv``
when it exists, otherwise the interpreter running Aura. That keeps a project
self-contained without any extra ceremony.
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

try:  # Python 3.11+
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 fallback
    try:
        import tomli as tomllib  # type: ignore
    except ModuleNotFoundError:
        tomllib = None


MANIFEST_NAME = 'aura.toml'
LOCK_NAME = 'aura.lock'
VENV_DIR = '.venv'
MIN_PYTHON = (3, 10)

# A dependency name is a PEP 508 project name: letters, digits, and the
# separators . - _ (normalised to -).
_NAME_RE = re.compile(r'^[A-Za-z0-9]([A-Za-z0-9._-]*[A-Za-z0-9])?$')
# A version specifier is a comma-separated list of PEP 440 clauses, or a bare
# version (which the installer pins with `==`). A leading `-` is never allowed,
# so a manifest cannot smuggle a pip option.
_SPEC_RE = re.compile(
    r'^\s*(==|!=|<=|>=|<|>|~=|===)\s*[A-Za-z0-9._*+!-]+'
    r'(\s*,\s*(==|!=|<=|>=|<|>|~=|===)\s*[A-Za-z0-9._*+!-]+)*\s*$')
# A bare PEP 440 version, e.g. "1.2.3", "2.0rc1", "1.0.post1".
_VERSION_RE = re.compile(
    r'^\s*[0-9]+(\.[0-9]+)*'
    r'((a|b|rc)[0-9]+)?(\.post[0-9]+)?(\.dev[0-9]+)?\s*$')


# ============================================================================
# Terminal presentation
# ============================================================================

def _supports_color() -> bool:
    """True when ANSI colour should be used on stdout."""
    if os.environ.get('NO_COLOR'):
        return False
    if os.environ.get('AURA_FORCE_COLOR'):
        return True
    return sys.stdout.isatty()


class _Style:
    """Minimal ANSI styling that degrades to plain text."""

    def __init__(self, enabled: bool):
        self.enabled = enabled

    def _wrap(self, code: str, text: str) -> str:
        if not self.enabled:
            return text
        return f"\033[{code}m{text}\033[0m"

    def bold(self, text: str) -> str:
        return self._wrap('1', text)

    def dim(self, text: str) -> str:
        return self._wrap('2', text)

    def green(self, text: str) -> str:
        return self._wrap('32', text)

    def yellow(self, text: str) -> str:
        return self._wrap('33', text)

    def red(self, text: str) -> str:
        return self._wrap('31', text)

    def cyan(self, text: str) -> str:
        return self._wrap('36', text)


style = _Style(_supports_color())


def _check(text: str) -> str:
    return style.green('✓') + ' ' + text


def _bullet(text: str) -> str:
    return style.dim('•') + ' ' + text


# ============================================================================
# Manifest
# ============================================================================

def find_manifest(start=None):
    """Walk upwards from ``start`` looking for ``aura.toml``."""
    current = Path(start or Path.cwd()).resolve()
    for directory in [current, *current.parents]:
        candidate = directory / MANIFEST_NAME
        if candidate.is_file():
            return candidate
    return None


def project_root(start=None) -> Path:
    """Return the project root: the manifest's directory, else the cwd."""
    manifest = find_manifest(start)
    return manifest.parent if manifest else Path(start or Path.cwd()).resolve()


def load_manifest(path=None):
    """Load the manifest, returning ``{}`` when none exists."""
    manifest_path = Path(path) if path else find_manifest()
    if manifest_path is None or not Path(manifest_path).is_file():
        return {}
    if tomllib is None:  # pragma: no cover - only without tomli on 3.10
        raise RuntimeError(
            "no TOML parser available; install 'tomli' to read aura.toml"
        )
    with open(manifest_path, 'rb') as handle:
        return tomllib.load(handle)


def _dump_manifest(data, path):
    """Write the manifest back in a stable, readable TOML form."""
    lines = []
    project = data.get('project', {})
    if project:
        lines.append('[project]')
        for key, value in project.items():
            lines.append(f'{key} = {_toml_value(value)}')
        lines.append('')

    deps = data.get('dependencies', {}) or {}
    runtime = {k: v for k, v in deps.items() if k != 'dev'}
    dev = deps.get('dev', {}) or {}

    lines.append('[dependencies]')
    for name in sorted(runtime):
        if _valid_dependency_name(str(name)):
            lines.append(f'{name} = {_toml_value(runtime[name])}')
    lines.append('')

    if dev:
        lines.append('[dependencies.dev]')
        for name in sorted(dev):
            if _valid_dependency_name(str(name)):
                lines.append(f'{name} = {_toml_value(dev[name])}')
        lines.append('')

    Path(path).write_text('\n'.join(lines).rstrip('\n') + '\n', encoding='utf-8')


def _toml_value(value):
    if isinstance(value, bool):
        return 'true' if value else 'false'
    if isinstance(value, (int, float)):
        return str(value)
    escaped = (str(value)
               .replace('\\', '\\\\')
               .replace('"', '\\"')
               .replace('\n', '\\n')
               .replace('\r', '\\r'))
    return '"' + escaped + '"'


def _valid_dependency_name(name):
    """A dependency key must be a valid PEP 508 project name."""
    return bool(name) and bool(_NAME_RE.match(name))


def _valid_specifier(spec):
    """True when ``spec`` is ``*``, a PEP 440 specifier list, or a bare version.

    Rejects anything that could smuggle a pip option (leading ``-``) or shell
    metacharacters, so a manifest cannot turn into an argument injection.
    """
    if spec in ('', '*', None):
        return True
    text = str(spec)
    return bool(_SPEC_RE.match(text)) or bool(_VERSION_RE.match(text))


def _dependency_groups(data):
    """Yield ``(group, {name: spec})`` for runtime and dev dependencies."""
    deps = data.get('dependencies', {}) or {}
    runtime = {k: v for k, v in deps.items() if k != 'dev'}
    dev = deps.get('dev', {}) or {}
    yield 'runtime', runtime
    yield 'dev', dev


# ============================================================================
# Interpreter / environment
# ============================================================================

def venv_dir(root=None) -> Path:
    """Path to the project's virtual environment."""
    override = os.environ.get('AURA_VENV')
    if override:
        return Path(override)
    return (root or project_root()) / VENV_DIR


def venv_python(root=None) -> Path | None:
    """Return the venv's interpreter, or None when there is no venv."""
    directory = venv_dir(root)
    if os.name == 'nt':  # pragma: no cover - Windows layout
        candidate = directory / 'Scripts' / 'python.exe'
    else:
        candidate = directory / 'bin' / 'python'
    return candidate if candidate.is_file() else None


def venv_exists(root=None) -> bool:
    return venv_python(root) is not None


def environment_python(root=None) -> str:
    """Interpreter that dependencies should be installed into.

    The project's ``.venv`` when it exists, otherwise the interpreter running
    Aura. This keeps a project self-contained when a venv is present and works
    out of the box when it is not.
    """
    python = venv_python(root)
    return str(python) if python else sys.executable


def environment_label(root=None) -> str:
    """Human-readable description of where dependencies will be installed."""
    python = venv_python(root)
    if python:
        return f"{style.cyan(VENV_DIR)} ({python})"
    return f"current interpreter ({sys.executable})"


def _run(cmd, **kwargs):
    """Run a command, echoing it in dim text, returning the exit code."""
    print(style.dim('$ ' + ' '.join(str(part) for part in cmd)))
    try:
        return subprocess.run(cmd, **kwargs).returncode
    except FileNotFoundError:
        print(style.red(f"command not found: {cmd[0]}"), file=sys.stderr)
        return 2


def _pip_install(requirements, upgrade=False, root=None):
    if not requirements:
        return 0
    cmd = [environment_python(root), '-m', 'pip', 'install']
    if upgrade:
        cmd.append('--upgrade')
    cmd += list(requirements)
    return _run(cmd)


# ============================================================================
# aura venv
# ============================================================================

def create_venv(path=None, force=False, python=None, install=True, root=None) -> int:
    """Create the project's virtual environment.

    ``path`` overrides the location; otherwise the project's ``.venv`` is used.
    ``--force`` removes an existing environment first. When ``install`` is set
    and the manifest declares dependencies, they are installed afterwards so a
    fresh clone is one command away from a working environment.
    """
    root = root or project_root()
    directory = Path(path) if path else venv_dir(root)

    if directory.exists() and not force:
        print(_check(f"{style.bold(str(directory))} already exists."))
        print(_bullet("use " + style.cyan("--force") + " to recreate it"))
        if install:
            return install_dependencies(root=root)
        return 0

    if directory.exists() and force:
        shutil.rmtree(directory)
        print(_check(f"removed existing {directory}"))

    interpreter = python or sys.executable
    code = _run([interpreter, '-m', 'venv', str(directory)])
    if code != 0:
        return code

    print(_check(f"created {style.bold(str(directory))}"))
    print(_bullet(f"python: {interpreter}"))

    if install:
        manifest = find_manifest(root)
        if manifest:
            groups = load_manifest(manifest).get('dependencies', {}) or {}
            if groups:
                print()
                return install_dependencies(root=root)

    print()
    print("Activate it with:")
    print("  " + style.cyan(activate_hint(directory)))
    return 0


def activate_hint(directory) -> str:
    """The shell command that activates ``directory`` on this platform."""
    if os.name == 'nt':  # pragma: no cover - Windows layout
        return f"{directory}\\Scripts\\activate"
    return f"source {directory}/bin/activate"


def shell_into_venv(root=None) -> int:
    """Print how to activate the venv, and the python it points at."""
    root = root or project_root()
    directory = venv_dir(root)
    python = venv_python(root)
    if python is None:
        print(style.red(f"No virtual environment at {directory}."),
              file=sys.stderr)
        print(_bullet("create one with " + style.cyan("aura venv init")))
        return 2
    print(_check(f"virtual environment at {style.bold(str(directory))}"))
    print(_bullet(f"python: {python}"))
    print()
    print("Activate it with:")
    print("  " + style.cyan(activate_hint(directory)))
    return 0


def venv_info(root=None) -> int:
    """Print the project, its interpreter, and its dependency status."""
    root = root or project_root()
    manifest = find_manifest(root)
    directory = venv_dir(root)
    python = venv_python(root)

    print(style.bold("Project"))
    if manifest:
        data = load_manifest(manifest)
        project = data.get('project', {}) or {}
        print(_bullet(f"manifest: {manifest}"))
        if project.get('name'):
            print(_bullet(f"name: {project.get('name')}"))
        if project.get('version'):
            print(_bullet(f"version: {project.get('version')}"))
    else:
        print(_bullet(style.yellow("no aura.toml found")))

    print()
    print(style.bold("Environment"))
    if python:
        print(_bullet(f"venv: {directory}"))
        print(_bullet(f"python: {python}"))
    else:
        print(_bullet(f"venv: {style.dim('not created')} ({directory})"))
        print(_bullet(f"python: {sys.executable} {style.dim('(system)')}"))

    print()
    print(style.bold("Dependencies"))
    if not manifest:
        print(_bullet("none declared"))
        return 0
    data = load_manifest(manifest)
    any_declared = False
    for group, deps in _dependency_groups(data):
        if not deps:
            continue
        any_declared = True
        print(_bullet(f"{group} ({len(deps)}):"))
        for name in sorted(deps):
            installed = _installed_version(str(name), root)
            declared = deps[name]
            detail = (style.dim(installed) if installed
                      else style.yellow('not installed'))
            print(f"    {name} {style.dim(str(declared))}  {detail}")
    if not any_declared:
        print(_bullet("none declared"))
    return 0


def remove_venv(root=None, confirm=True) -> int:
    """Delete the project's virtual environment."""
    directory = venv_dir(root or project_root())
    if not directory.exists():
        print(style.yellow(f"No virtual environment at {directory}."))
        return 0
    if confirm and sys.stdin.isatty():
        answer = input(f"Remove {directory}? [y/N] ").strip().lower()
        if answer not in ('y', 'yes'):
            print("Aborted.")
            return 0
    shutil.rmtree(directory)
    print(_check(f"removed {directory}"))
    return 0


# ============================================================================
# aura add / remove / install / deps
# ============================================================================

def parse_requirement(text):
    """Split ``name[extras]`` + optional specifier into ``(name, spec)``.

    Accepts ``requests``, ``requests>=2.28``, ``requests==2.31.0``,
    ``"pyyaml>=6,<7"``. Returns ``(None, None)`` when the text is not a valid
    requirement, so the caller can report a clean error.
    """
    text = text.strip()
    if not text:
        return None, None
    match = re.match(r'^([A-Za-z0-9][A-Za-z0-9._-]*)'
                     r'(\[[A-Za-z0-9,._-]+\])?\s*(.*)$', text)
    if not match:
        return None, None
    name = match.group(1)
    extras = match.group(2) or ''
    spec = match.group(3).strip()
    if not _valid_dependency_name(name):
        return None, None
    if spec and not _valid_specifier(spec):
        return None, None
    return name, (extras + spec if extras or spec else '*')


def add_package(name, version=None, manifest_path=None, install=True,
                dev=False, root=None):
    """Add a dependency to the manifest and install it.

    ``name`` may carry a specifier (``requests>=2.28``); an explicit ``version``
    becomes an exact match unless it already starts with a comparison operator.
    ``dev=True`` records the dependency under ``[dependencies.dev]``.
    """
    root = root or project_root()
    if manifest_path is None:
        manifest_path = find_manifest(root)
    if manifest_path is None:
        manifest_path = root / MANIFEST_NAME
        _dump_manifest({'project': {'name': root.name or 'app',
                                    'version': '0.1.0'},
                        'dependencies': {}}, manifest_path)
        print(_check(f"created {manifest_path}"))

    package_name, spec = parse_requirement(name)
    if package_name is None:
        print(style.red(f"Error: invalid requirement {name!r}."),
              file=sys.stderr)
        print(_bullet("expected a name with an optional specifier, e.g. "
                      + style.cyan("requests") + " or " + style.cyan("requests>=2.28")),
              file=sys.stderr)
        return 2

    if version:
        spec = version if version[0] in '<>=~!,' else f"=={version}"

    requirement = package_name if spec in ('', '*') else f"{package_name}{spec}"

    data = load_manifest(manifest_path)
    deps = data.setdefault('dependencies', {})
    group = deps.setdefault('dev', {}) if dev else deps

    previous = group.get(package_name)
    group[package_name] = spec if spec else '*'
    _dump_manifest(data, manifest_path)

    label = 'dev dependency' if dev else 'dependency'
    verb = 'Updated' if previous is not None else 'Added'
    print(_check(f"{verb} {label} {style.bold(package_name)} "
                 f"{style.dim(str(group[package_name]))} in {manifest_path.name}"))

    if install:
        code = _pip_install([requirement], root=root)
        if code != 0:
            return code
        print(_check(f"installed {package_name} into {environment_label(root)}"))
    return 0


def remove_package(name, manifest_path=None, uninstall=False, root=None):
    """Remove a dependency from the manifest (runtime or dev)."""
    root = root or project_root()
    manifest_path = manifest_path or find_manifest(root)
    if manifest_path is None:
        print(style.red("Error: no aura.toml found."), file=sys.stderr)
        return 2
    data = load_manifest(manifest_path)
    deps = data.get('dependencies', {}) or {}

    removed_from = None
    if name in deps and name != 'dev':
        del deps[name]
        removed_from = 'runtime'
    dev_deps = deps.get('dev') or {}
    if name in dev_deps:
        del dev_deps[name]
        removed_from = 'dev' if removed_from is None else removed_from
        if not dev_deps:
            deps.pop('dev', None)
        else:
            deps['dev'] = dev_deps

    if removed_from is None:
        print(style.yellow(f"{name} is not declared in {manifest_path.name}."))
        return 1

    data['dependencies'] = deps
    _dump_manifest(data, manifest_path)
    print(_check(f"Removed {style.bold(name)} ({removed_from}) from "
                 f"{manifest_path.name}"))

    if uninstall:
        code = _run([environment_python(root), '-m', 'pip', 'uninstall', '-y', name])
        if code != 0:
            return code
    return 0


def install_dependencies(manifest_path=None, upgrade=False, root=None,
                         include_dev=True):
    """Install every dependency declared in the manifest."""
    root = root or project_root()
    manifest_path = manifest_path or find_manifest(root)
    if manifest_path is None:
        print(style.red("Error: no aura.toml found."), file=sys.stderr)
        print(_bullet("run " + style.cyan("aura init") + " to create one"))
        return 2
    data = load_manifest(manifest_path)

    requirements = []
    invalid = []
    count = 0
    for group, deps in _dependency_groups(data):
        if group == 'dev' and not include_dev:
            continue
        for name, spec in deps.items():
            name = str(name)
            if not _valid_dependency_name(name):
                invalid.append(name)
                continue
            spec = '' if spec is None else str(spec)
            if not _valid_specifier(spec):
                invalid.append(name)
                continue
            if spec in ('*', ''):
                requirements.append(name)
            elif spec[0] in '<>=~!':
                requirements.append(f"{name}{spec}")
            else:
                requirements.append(f"{name}=={spec}")
            count += 1

    if invalid:
        for name in invalid:
            print(style.red(f"Error: refusing unsafe dependency entry {name!r}."),
                  file=sys.stderr)
        return 2
    if not requirements:
        print(_bullet("No dependencies declared."))
        return 0

    print(f"Installing {count} dependenc{'y' if count == 1 else 'ies'} into "
          f"{environment_label(root)}")
    code = _pip_install(requirements, upgrade=upgrade, root=root)
    if code == 0:
        print(_check("Dependencies ready."))
    return code


def list_dependencies(manifest_path=None, root=None):
    """Print the declared dependencies, with installed versions."""
    root = root or project_root()
    manifest_path = manifest_path or find_manifest(root)
    if manifest_path is None:
        print(style.yellow("No aura.toml found."))
        return 0
    data = load_manifest(manifest_path)

    printed = False
    for group, deps in _dependency_groups(data):
        if not deps:
            continue
        printed = True
        print(style.bold(f"{group} dependencies") + style.dim(f" ({manifest_path.name})"))
        for name in sorted(deps):
            installed = _installed_version(str(name), root)
            status = style.dim(installed) if installed else style.yellow('not installed')
            print(f"  {name} {style.dim(str(deps[name]))}  {status}")
        print()
    if not printed:
        print(_bullet(f"No dependencies in {manifest_path.name}."))
    return 0


def _installed_version(name, root=None):
    """Return the installed version of ``name``, or None."""
    try:
        result = subprocess.run(
            [environment_python(root), '-m', 'pip', 'show', str(name)],
            capture_output=True, text=True, timeout=30,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode != 0:
        return None
    for line in result.stdout.splitlines():
        if line.startswith('Version:'):
            return line.split(':', 1)[1].strip()
    return None


def init_project(name='app', manifest_path=None, venv=False):
    """Create a starter ``aura.toml`` and ``src/main.aura``."""
    root = Path(manifest_path).parent if manifest_path else Path.cwd()
    manifest = root / MANIFEST_NAME
    created_manifest = False
    if manifest.exists():
        print(_bullet(f"{manifest} already exists."))
    else:
        _dump_manifest({'project': {'name': name, 'version': '0.1.0'},
                        'dependencies': {}}, manifest)
        print(_check(f"Created {style.bold(str(manifest))}"))
        created_manifest = True

    src_dir = root / 'src'
    src_dir.mkdir(exist_ok=True)
    main_file = src_dir / 'main.aura'
    if not main_file.exists():
        main_file.write_text(
            'def main() {\n'
            '  print("Hello from Aura!")\n'
            '}\n',
            encoding='utf-8',
        )
        print(_check(f"Created {style.bold(str(main_file))}"))

    if created_manifest:
        print()
        print("Next steps:")
        print("  " + style.cyan("aura venv init") + "        create .venv and install deps")
        print("  " + style.cyan("aura run src/main.aura") + "  run the program")

    if venv:
        print()
        return create_venv(root=root)
    return 0


# ============================================================================
# Lock file
# ============================================================================

def write_lock(manifest_path=None, root=None) -> int:
    """Record the exact installed versions in ``aura.lock``.

    The lock file is a snapshot for reproducibility: it names each declared
    dependency with the version currently installed in the project environment.
    """
    root = root or project_root()
    manifest_path = manifest_path or find_manifest(root)
    if manifest_path is None:
        print(style.red("Error: no aura.toml found."), file=sys.stderr)
        return 2
    data = load_manifest(manifest_path)
    locked = []
    missing = []
    for group, deps in _dependency_groups(data):
        for name in sorted(deps):
            version = _installed_version(str(name), root)
            if version:
                locked.append((group, str(name), version))
            else:
                missing.append(str(name))
    if not locked:
        print(style.yellow("Nothing installed to lock."))
        return 1
    lines = ["# Generated by aura. Do not edit by hand.",
             "# Exact versions installed in this project's environment.",
             ""]
    for group in ('runtime', 'dev'):
        entries = [e for e in locked if e[0] == group]
        if not entries:
            continue
        lines.append(f'[{group}]')
        for _group, name, version in entries:
            lines.append(f'{name} = {_toml_value(version)}')
        lines.append('')
    (root / LOCK_NAME).write_text('\n'.join(lines), encoding='utf-8')
    print(_check(f"Wrote {style.bold(LOCK_NAME)} with {len(locked)} "
                 f"pinned dependenc{'y' if len(locked) == 1 else 'ies'}"))
    if missing:
        print(_bullet(style.yellow("not installed: " + ", ".join(sorted(missing)))))
    return 0


def doctor(root=None) -> int:
    """Report the health of the project environment."""
    root = root or project_root()
    problems = []
    print(style.bold("Checking the Aura project environment"))
    print()

    if sys.version_info < MIN_PYTHON:
        problems.append(
            f"Python {sys.version_info.major}.{sys.version_info.minor} is older "
            f"than the required {MIN_PYTHON[0]}.{MIN_PYTHON[1]}")
    else:
        print(_check(f"python {sys.version.split()[0]}"))

    manifest = find_manifest(root)
    if manifest is None:
        problems.append("no aura.toml found (run `aura init`)")
    else:
        print(_check(f"manifest {manifest}"))

    if venv_exists(root):
        print(_check(f"virtual environment {venv_dir(root)}"))
    else:
        print(_bullet(style.yellow("no virtual environment (run `aura venv init`)")))

    if manifest:
        data = load_manifest(manifest)
        for _group, deps in _dependency_groups(data):
            for name in sorted(deps):
                if _installed_version(str(name), root):
                    print(_check(f"{name} installed"))
                else:
                    problems.append(f"{name} is declared but not installed")

    print()
    if problems:
        print(style.red(style.bold("Problems")))
        for problem in problems:
            print("  " + style.red("✗") + " " + problem)
        return 1
    print(style.green(style.bold("All checks passed.")))
    return 0
