"""Runtime namespace aliases for Aura.

Aura source imports the standard library as ``stdlib.math`` and the toolchain
as ``parser``/``transpiler``. When Aura is installed as the ``aura`` package,
those top-level names do not exist. This module registers ``sys.modules``
aliases so both the documented Aura imports and the internal imports keep
working, without polluting the global namespace with generic package names.
"""

import importlib
import sys

_ALIASES = ('stdlib', 'parser', 'transpiler', 'repl', 'tools')
_installed = False


# Aura programs write `import python` to reach the interop bridge. Map that
# friendly name onto the real `aura.stdlib.python` module so the bridge is
# available without a top-level package named `python`.
_SUBMODULE_ALIASES = {
    'python': 'aura.stdlib.python',
}


def install_runtime_aliases():
    """Register ``stdlib`` → ``aura.stdlib`` (and friends) in ``sys.modules``.

    Idempotent. Existing top-level modules with the same name are left alone so
    running from a source checkout continues to work.
    """
    global _installed
    if _installed:
        return
    for alias in _ALIASES:
        if alias in sys.modules:
            continue
        try:
            module = importlib.import_module(f'aura.{alias}')
        except ImportError:
            continue
        sys.modules[alias] = module
    for alias, target in _SUBMODULE_ALIASES.items():
        if alias in sys.modules:
            continue
        try:
            sys.modules[alias] = importlib.import_module(target)
        except ImportError:
            continue
    _installed = True
