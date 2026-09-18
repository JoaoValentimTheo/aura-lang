"""Resolution of module re-exports against the source folder.

A module can act as a **facade** for a folder: `module App { export Components }`
binds `App.Components` to the `Components` name defined in a sibling source
file. The name is resolved by convention, in this order:

1. a name declared in the same file (the facade re-exports its own member);
2. `components.aura` next to `App.aura` (case-insensitive filename match);
3. `Components/Components.aura` or `Components/__init__.aura` (a subfolder
   named after the symbol, holding it).

An explicit source can be given with `export Components from "components"`,
which resolves `components.aura` or `components/__init__.aura` directly.

Resolution is **static and confined to the source folder**: a resolved path
must stay inside the folder that contains the facade, so a module path can
never read outside the project. Nothing is executed at resolve time — only the
file's text is scanned for the declaration, so a broken sibling cannot make the
resolution fail in a surprising way.
"""

from __future__ import annotations

import re
from pathlib import Path

# Declaration heads that can define a re-exportable name. Matching is textual
# (a light scan, not a full parse) so resolution stays cheap and independent of
# whether the sibling file otherwise type-checks.
_DECL_RE = re.compile(
    r'^\s*(?:export\s+)?(?:public\s+|private\s+|protected\s+)*'
    r'(?:class|trait|enum|type|def|const)\s+([A-Za-z_][A-Za-z0-9_]*)',
    re.MULTILINE,
)


class ReexportError(Exception):
    """Raised when a re-export cannot be resolved to a source file."""


def _safe_child(base: Path, *parts) -> Path | None:
    """Join ``parts`` onto ``base``, returning None if it escapes ``base``."""
    candidate = base.joinpath(*parts)
    try:
        resolved = candidate.resolve()
        base_resolved = base.resolve()
    except OSError:
        return None
    if resolved == base_resolved or base_resolved in resolved.parents:
        return resolved
    return None


def _defines(path: Path, name: str) -> bool:
    """True when ``path`` textually declares ``name`` at any level."""
    try:
        text = path.read_text(encoding='utf-8')
    except (OSError, UnicodeDecodeError):
        return False
    return name in _DECL_RE.findall(text)


def _candidate_files(base: Path, name: str, source: str | None):
    """Yield candidate source files for ``name`` under ``base``."""
    if source:
        # Explicit: `export Name from "components"`. Only a plain dotted path
        # is accepted; anything else was rejected by the parser.
        parts = source.split('.')
        file = _safe_child(base, *parts[:-1], parts[-1] + '.aura')
        if file is not None:
            yield file
        package = _safe_child(base, *parts, '__init__.aura')
        if package is not None:
            yield package
        return

    # Convention 1: a sibling file whose stem matches, case-insensitively.
    try:
        entries = sorted(base.iterdir())
    except OSError:
        entries = []
    for entry in entries:
        if entry.is_file() and entry.suffix == '.aura' and entry.stem.lower() == name.lower():
            yield entry
    # Convention 2: a subfolder named after the symbol.
    package = _safe_child(base, name)
    if package is not None:
        for candidate in (package / (name + '.aura'), package / '__init__.aura'):
            if candidate.is_file():
                yield candidate


def resolve_reexport(name: str, base_dir, source=None):
    """Resolve a re-exported ``name`` to a source file under ``base_dir``.

    Returns ``(path, module_path, declares_name)``:

    * ``path`` — the file that provides the name;
    * ``module_path`` — its dotted import path relative to ``base_dir``
      (e.g. ``"components"`` or ``"Utils.utils"``);
    * ``declares_name`` — True when the file *defines* the name, so the binding
      is ``Mod.Name``; False when the file is a module the name refers to as a
      whole, so the binding is the module itself
      (`export Utils` -> `App.Utils = utils`, a namespace of its exports).

    A sibling is matched by filename first, then by a subfolder. An empty
    result means nothing provides the name, so the caller reports a diagnostic.
    """
    base = Path(base_dir)
    for candidate in _candidate_files(base, name, source):
        if not candidate.is_file():
            continue
        try:
            relative = candidate.relative_to(base)
        except ValueError:
            continue
        parts = list(relative.parts)
        if parts[-1] == '__init__.aura':
            parts = parts[:-1]
        else:
            parts[-1] = Path(parts[-1]).stem
        if not parts:
            continue
        module_path = '.'.join(parts)
        if _defines(candidate, name):
            return candidate, module_path, True
        # The file exists but does not declare the name: expose the module
        # itself as the namespace (`export Utils` -> the `utils` module).
        return candidate, module_path, False
    return None


def find_reexport_conflicts(names, base_dir):
    """Return the names that resolve to more than one *different* file.

    A facade that exports two symbols resolving to the same module path is
    fine; two that resolve to different files but the same requested name
    cannot both be bound. Used to report an ambiguity instead of silently
    picking one.
    """
    resolved = {}
    conflicts = []
    for name in names:
        found = resolve_reexport(name, base_dir)
        if found is None:
            continue
        path, module_path, _declares = found
        if module_path in resolved and resolved[module_path] != path:
            conflicts.append(name)
        resolved[module_path] = path
    return conflicts
