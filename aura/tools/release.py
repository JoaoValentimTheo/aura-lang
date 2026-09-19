"""Version and release helpers for Aura.

Keeps ``pyproject.toml`` and ``aura/__init__.py`` in lock-step. Used by the
release workflow and available as ``aura version`` / programmatic API.
"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PYPROJECT = ROOT / 'pyproject.toml'
INIT = ROOT / 'aura' / '__init__.py'


def _project_version_match(text):
    """Match the ``version = "..."`` line inside the ``[project]`` table.

    Searching the whole file would rewrite an unrelated ``version`` key that
    appears in an earlier table (e.g. ``[tool.x]``).
    """
    section = re.search(
        r'^\[project\]\s*$([\s\S]*?)(?=^\[|\Z)', text, re.MULTILINE)
    if not section:
        return None
    return re.search(r'^(version\s*=\s*)"([^"]+)"', section.group(1),
                     re.MULTILINE)


def get_version(pyproject=PYPROJECT):
    """Read the version.

    Prefers ``pyproject.toml`` in a source checkout; falls back to the
    installed package metadata (or ``aura.__version__``) when Aura is installed
    without the repository present.
    """
    pyproject = Path(pyproject)
    if pyproject.is_file():
        text = pyproject.read_text(encoding='utf-8')
        match = _project_version_match(text)
        if match:
            return match.group(2)

    try:
        from importlib.metadata import version as _dist_version
        return _dist_version('aura-language')
    except Exception:
        pass  # PackageNotFoundError expected when not installed

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

    pyproject = Path(pyproject)
    py_text = pyproject.read_text(encoding='utf-8')
    section = re.search(
        r'(^\[project\]\s*$[\s\S]*?)(?=^\[|\Z)', py_text, re.MULTILINE)
    if section is None:
        raise ValueError(f"no [project] table in {pyproject}")
    body = section.group(1)
    if _project_version_match(py_text) is None:
        raise ValueError(f"no version key in [project] of {pyproject}")
    new_body = re.sub(r'^(version\s*=\s*)"[^"]+"', rf'\1"{version}"',
                      body, count=1, flags=re.MULTILINE)
    py_text = py_text[:section.start(1)] + new_body + py_text[section.end(1):]
    pyproject.write_text(py_text, encoding='utf-8')

    init_path = Path(init)
    if init_path.is_file():
        init_text = init_path.read_text(encoding='utf-8')
        init_text = re.sub(r'^(__version__\s*=\s*)"[^"]+"', rf'\1"{version}"',
                           init_text, count=1, flags=re.MULTILINE)
        init_path.write_text(init_text, encoding='utf-8')
    return version


def bump(part='patch', pyproject=PYPROJECT, init=INIT):
    """Bump major/minor/patch and propagate the new version.

    Pre-release suffixes are dropped (``0.1.0a4`` bumped by ``patch`` becomes
    ``0.1.1``), matching how a release graduates from alpha to a normal version.
    """
    release = re.match(r'^(\d+)\.(\d+)\.(\d+)', get_version(pyproject))
    if release is None:
        raise ValueError(f"cannot bump a non-numeric version: {get_version(pyproject)!r}")
    major, minor, patch = (int(x) for x in release.groups())
    if part == 'major':
        major, minor, patch = major + 1, 0, 0
    elif part == 'minor':
        minor, patch = minor + 1, 0
    elif part == 'patch':
        patch += 1
    else:
        raise ValueError("part must be 'major', 'minor' or 'patch'")
    return set_version(f"{major}.{minor}.{patch}", pyproject=pyproject, init=init)
