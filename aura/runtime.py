"""Runtime namespace aliases for Aura.

Aura source imports the standard library as ``stdlib.math`` and the toolchain
as ``parser``/``transpiler``. When Aura is installed as the ``aura`` package,
those top-level names do not exist. This module registers ``sys.modules``
aliases so both the documented Aura imports and the internal imports keep
working, without polluting the global namespace with generic package names.
"""

import importlib
import pkgutil
import sys
import threading

_ALIASES = ('stdlib', 'parser', 'transpiler', 'repl', 'tools')
_installed = False
_lock = threading.Lock()


# Aura programs write `import python` to reach the interop bridge. Map that
# friendly name onto the real `aura.stdlib.python` module so the bridge is
# available without a top-level package named `python`.
_SUBMODULE_ALIASES = {
    'python': 'aura.stdlib.python',
}


def _register_submodules(alias, module):
    """Alias ``alias.<sub>`` → the real ``aura.<alias>.<sub>`` module.

    Registering the parent package alone is not enough: once ``transpiler`` is
    aliased to ``aura.transpiler``, a later ``import transpiler.ast`` would let
    the import machinery load a *second*, distinct module object for the same
    file. That breaks ``isinstance`` checks and identity comparisons across the
    toolchain (e.g. a ``Transformer`` not recognising its own ``Program``).
    Pinning each submodule to the already-imported real module keeps a single
    canonical class identity.
    """
    path = getattr(module, '__path__', None)
    if not path:
        return
    for info in pkgutil.walk_packages(path, prefix=f'aura.{alias}.'):
        sub = info.name
        short = sub[len(f'aura.{alias}.'):]
        key = f'{alias}.{short}'
        if key in sys.modules:
            continue
        try:
            sys.modules[key] = importlib.import_module(sub)
        except ImportError:
            continue


def install_runtime_aliases():
    """Register ``stdlib`` → ``aura.stdlib`` (and friends) in ``sys.modules``.

    Idempotent and thread-safe. Existing top-level modules with the same name
    are left alone so running from a source checkout continues to work.
    """
    global _installed
    if _installed:
        return
    with _lock:
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
            _register_submodules(alias, module)
        for alias, target in _SUBMODULE_ALIASES.items():
            if alias in sys.modules:
                continue
            try:
                sys.modules[alias] = importlib.import_module(target)
            except ImportError:
                continue
        _installed = True


def uninstall_runtime_aliases():
    """Remove only the aliases this module installed.

    Used by tests to keep sessions isolated; safe to call when nothing was
    installed.
    """
    global _installed
    with _lock:
        for alias in _ALIASES:
            module = sys.modules.get(alias)
            if module is not None and getattr(module, '__name__', '').startswith('aura.'):
                del sys.modules[alias]
            prefix = f'{alias}.'
            for key in [k for k in sys.modules if k.startswith(prefix)]:
                target = sys.modules.get(key)
                if target is not None and getattr(target, '__name__', '').startswith('aura.'):
                    del sys.modules[key]
        for alias, target in _SUBMODULE_ALIASES.items():
            module = sys.modules.get(alias)
            if module is not None and getattr(module, '__name__', '') == target:
                del sys.modules[alias]
        _installed = False
