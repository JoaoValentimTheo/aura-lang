"""Version and release helpers for Aura.

Keeps ``pyproject.toml`` and ``aura/__init__.py`` in lock-step. Used by the
release workflow and available as ``aura version`` / programmatic API.
"""

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PYPROJECT = ROOT / 'pyproject.toml'
INIT = ROOT / 'aura' / '__init__.py'


def get_version():
    """Read the version.

    Prefers ``pyproject.toml`` in a source checkout; falls back to the
    installed package metadata (or ``aura.__version__``) when Aura is installed
    without the repository present.
    """
    if PYPROJECT.is_file():
        text = PYPROJECT.read_text(encoding='utf-8')
        match = re.search(r'^version\s*=\s*"([^"]+)"', text, re.MULTILINE)
        if match:
            return match.group(1)

    try:
        from importlib.metadata import version as _dist_version
        return _dist_version('aura-lang')
    except Exception:
        pass

    try:
        import aura
        return aura.__version__
    except Exception as exc:
        raise RuntimeError('could not determine Aura version') from exc


def set_version(version, pyproject=PYPROJECT, init=INIT):
    """Write ``version`` to both pyproject.toml and aura/__init__.py."""
    # Accept semver and PEP 440 pre-releases (0.1.0a1, 1.0.0b2, 1.0.0-rc.1).
    if not re.fullmatch(r'\d+\.\d+\.\d+(?:[.\-+]?[0-9A-Za-z][0-9A-Za-z.\-]*)?', version):
        raise ValueError(f"invalid semantic version: {version!r}")

    py_text = Path(pyproject).read_text(encoding='utf-8')
    py_text = re.sub(r'^(version\s*=\s*)"[^"]+"', rf'\1"{version}"',
                     py_text, count=1, flags=re.MULTILINE)
    Path(pyproject).write_text(py_text, encoding='utf-8')

    init_path = Path(init)
    if init_path.is_file():
        init_text = init_path.read_text(encoding='utf-8')
        init_text = re.sub(r'^(__version__\s*=\s*)"[^"]+"', rf'\1"{version}"',
                           init_text, count=1, flags=re.MULTILINE)
        init_path.write_text(init_text, encoding='utf-8')
    return version


def bump(part='patch'):
    """Bump major/minor/patch and propagate the new version."""
    major, minor, patch = (int(x) for x in get_version().split('.')[:3])
    if part == 'major':
        major, minor, patch = major + 1, 0, 0
    elif part == 'minor':
        minor, patch = minor + 1, 0
    elif part == 'patch':
        patch += 1
    else:
        raise ValueError("part must be 'major', 'minor' or 'patch'")
    return set_version(f"{major}.{minor}.{patch}")