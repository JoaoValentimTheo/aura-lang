"""Aura dependency manifest and installer.

Aura projects declare Python dependencies in ``aura.toml``:

```toml
[project]
name = "my-app"
version = "0.1.0"

[dependencies]
requests = ">=2.28"
pyyaml = "*"
```

Because Aura transpiles to Python and imports PyPI packages directly, a
dependency is just a Python distribution. ``aura add`` records it and pipes it
to the active interpreter's package installer.
"""

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


def find_manifest(start=None):
    """Walk upwards from ``start`` looking for ``aura.toml``."""
    current = Path(start or Path.cwd()).resolve()
    for directory in [current, *current.parents]:
        candidate = directory / MANIFEST_NAME
        if candidate.is_file():
            return candidate
    return None


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

    deps = data.get('dependencies', {})
    lines.append('[dependencies]')
    for name in sorted(deps):
        if not _valid_dependency_name(str(name)):
            continue
        lines.append(f'{name} = {_toml_value(deps[name])}')
    if not deps:
        lines[-1] = '[dependencies]'
    lines.append('')

    Path(path).write_text('\n'.join(lines), encoding='utf-8')


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
    """Dependency keys must be bare TOML keys (no newlines or quotes)."""
    return bool(name) and all(ch.isalnum() or ch in '._-' for ch in name)


def _pip():
    """Return the package-installer command, preferring ``python -m pip``."""
    return [sys.executable, '-m', 'pip']


def _require_manifest(path):
    if path is None:
        print("No aura.toml found. Run 'aura init' first.", file=sys.stderr)
        return None
    return path


def add_package(name, version=None, manifest_path=None, install=True):
    """Add a dependency to ``aura.toml`` and install it.

    ``name`` may include a specifier, e.g. ``requests>=2.28``; an explicit
    ``version`` becomes a ``==`` pin unless it already starts with an operator.
    """
    if manifest_path is None:
        manifest_path = find_manifest()
    if manifest_path is None:
        # Create a minimal manifest in the current directory.
        manifest_path = Path.cwd() / MANIFEST_NAME
        _dump_manifest({'project': {}, 'dependencies': {}}, manifest_path)

    name = name.strip()
    if not name:
        print("Error: package name must not be empty.", file=sys.stderr)
        return 2
    spec = version
    if any(op in name for op in ('==', '>=', '<=', '~=', '!=', '>', '<')):
        package_name = name.split('==')[0].split('>=')[0].split('<=')[0] \
            .split('~=')[0].split('!=')[0].split('>')[0].split('<')[0].strip()
        requirement = name
        spec = None
    else:
        package_name = name
        if version and version[0] in '<>=~!':
            requirement = f"{name}{version}"
        elif version:
            requirement = f"{name}=={version}"
        else:
            requirement = name

    if not _valid_dependency_name(package_name):
        print(f"Error: invalid package name {package_name!r}.", file=sys.stderr)
        return 2

    data = load_manifest(manifest_path)
    data.setdefault('dependencies', {})
    if spec is None:
        # Preserve the raw requirement text after the package name.
        raw = name[len(package_name):].strip()
        data['dependencies'][package_name] = raw if raw else '*'
    else:
        data['dependencies'][package_name] = version
    _dump_manifest(data, manifest_path)
    print(f"Added {package_name} to {manifest_path}")

    if install:
        return _pip_install([requirement])
    return 0


def install_dependencies(manifest_path=None, upgrade=False):
    """Install every dependency declared in the manifest."""
    manifest_path = _require_manifest(manifest_path or find_manifest())
    if manifest_path is None:
        return 2
    data = load_manifest(manifest_path)
    deps = data.get('dependencies', {})
    if not deps:
        print("No dependencies declared.")
        return 0
    requirements = []
    for name, spec in deps.items():
        if spec in ('*', '', None):
            requirements.append(name)
        elif str(spec)[0] in '<>=~!':
            requirements.append(f"{name}{spec}")
        else:
            requirements.append(f"{name}=={spec}")
    return _pip_install(requirements, upgrade=upgrade)


def list_dependencies(manifest_path=None):
    """Print the declared dependencies."""
    manifest_path = manifest_path or find_manifest()
    if manifest_path is None:
        print("No aura.toml found.")
        return 0
    data = load_manifest(manifest_path)
    deps = data.get('dependencies', {})
    if not deps:
        print(f"No dependencies in {manifest_path}.")
        return 0
    print(f"Dependencies ({manifest_path}):")
    for name in sorted(deps):
        print(f"  {name} {deps[name]}")
    return 0


def init_project(name='app', manifest_path=None):
    """Create a starter ``aura.toml`` and ``src/main.aura``."""
    root = Path(manifest_path).parent if manifest_path else Path.cwd()
    manifest = root / MANIFEST_NAME
    if manifest.exists():
        print(f"{manifest} already exists.")
    else:
        _dump_manifest({'project': {'name': name, 'version': '0.1.0'},
                        'dependencies': {}}, manifest)
        print(f"Created {manifest}")

    src_dir = root / 'src'
    src_dir.mkdir(exist_ok=True)
    main_file = src_dir / 'main.aura'
    if not main_file.exists():
        main_file.write_text(
            'def main() {\n'
            '  print("Hello from Aura!")\n'
            '}\n\n'
            'main()\n',
            encoding='utf-8',
        )
        print(f"Created {main_file}")
    return 0


def _pip_install(requirements, upgrade=False):
    cmd = _pip() + ['install']
    if upgrade:
        cmd.append('--upgrade')
    cmd += requirements
    print("$ " + " ".join(cmd))
    try:
        result = subprocess.run(cmd)
    except FileNotFoundError:
        print("pip is not available for this interpreter.", file=sys.stderr)
        return 2
    return result.returncode