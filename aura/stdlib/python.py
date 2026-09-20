"""Aura ↔ Python interoperability bridge.

Aura transpiles to Python, so every Aura program can already ``import`` any
Python module. This module makes that bridge *explicit and convenient*: it
gives Aura code a supported surface for dynamic imports, evaluation, attribute
access, type inspection and conversions, instead of reaching for ``__import__``
or ``getattr`` by hand.

Usage from Aura::

    import python

    let requests = python.import_module("requests")
    let text = requests.get("https://example.com").text

    let parsed = python.eval("[x * x for x in range(5)]")
    print(python.type_name(parsed))

    let math = python.load("math")
    print(math.sqrt(2))

Everything here is a thin, total wrapper: failures raise ordinary Python
exceptions with clear messages rather than returning sentinels, so Aura's
``try``/``catch`` works as expected.
"""

from __future__ import annotations

import builtins
import importlib
import sys
import types

__all__ = [
    'import_module',
    'load',
    'reload',
    'is_available',
    'eval',
    'exec_code',
    'compile_source',
    'call',
    'getattr',
    'setattr',
    'hasattr',
    'dir',
    'type_name',
    'is_module',
    'is_callable',
    'is_class',
    'is_instance',
    'to_aura',
    'to_python',
    'builtins',
    'py_builtins',
    'interpreter_version',
    'add_path',
    'site_packages',
    'modules',
    'ModuleProxy',
]


def import_module(name, package=None):
    """Import and return a module by name, wrapped in :class:`ModuleProxy`.

    ``name`` may be a dotted path (``"os.path"``). ``package`` follows
    :func:`importlib.import_module` semantics for relative imports.

    Returns a :class:`ModuleProxy` so attribute access works naturally from
    Aura code (``python.import_module("math").sqrt(2)``).
    """
    if not isinstance(name, str) or not name:
        raise TypeError("import_module() expects a non-empty module name")
    return ModuleProxy(importlib.import_module(name, package))


def load(name, package=None):
    """Alias for :func:`import_module`, named for readability in pipelines."""
    return import_module(name, package)


def reload(module):
    """Reload an already-imported module in place."""
    was_proxy = isinstance(module, ModuleProxy)
    if isinstance(module, ModuleProxy):
        module = module._module
    if not isinstance(module, types.ModuleType):
        raise TypeError("reload() expects a module")
    result = importlib.reload(module)
    return ModuleProxy(result) if was_proxy else result


def is_available(name):
    """Return True when ``name`` can be imported in this interpreter.

    Never raises: a missing or broken module yields ``False``.
    """
    try:
        importlib.import_module(name)
        return True
    except Exception:
        return False


def eval(expression, globals=None, locals=None):
    """Evaluate a Python expression string and return its value.

    .. warning::
        This executes arbitrary Python code. There is no sandbox. Use only
        when you fully trust the expression source. Aura's type safety and
        mutability rules are *not* enforced inside evaluated code.
    """
    if not isinstance(expression, str):
        raise TypeError("eval() expects a string")
    scope = globals if globals is not None else _default_globals()
    return builtins.eval(expression, scope, locals)  # noqa: S307 - explicit bridge


def exec_code(code, globals=None, locals=None):
    """Execute Python statements. Returns the namespace used, so callers can
    inspect newly defined names.

    .. warning::
        This executes arbitrary Python code. There is no sandbox. Use only
        when you fully trust the code source. Aura's type safety and
        mutability rules are *not* enforced inside executed code.
    """
    if not isinstance(code, str):
        raise TypeError("exec() expects a string")
    scope = globals if globals is not None else _default_globals()
    builtins.exec(code, scope, locals)  # noqa: S102 - explicit bridge
    return scope


def compile_source(source, filename='<aura-python>', mode='exec'):
    """Compile Python source and return a code object.

    ``mode`` must be ``'exec'`` (statements), ``'eval'`` (single expression),
    or ``'single'`` (interactive statement). Defaults to ``'exec'``.
    """
    if mode not in ('exec', 'eval', 'single'):
        raise ValueError(f"compile_source() mode must be 'exec', 'eval', or 'single', got {mode!r}")
    return builtins.compile(source, filename, mode)


def call(func, *args, **kwargs):
    """Call ``func`` with the given arguments."""
    if not callable(func):
        raise TypeError(f"{func!r} is not callable")
    return func(*args, **kwargs)


def getattr(obj, name, default=None):
    """Attribute access with an optional default (never raises for missing)."""
    return builtins.getattr(obj, name, default)


def setattr(obj, name, value):
    """Set an attribute on ``obj``."""
    builtins.setattr(obj, name, value)
    return obj


def hasattr(obj, name):
    """Return True when ``obj`` has attribute ``name``."""
    return builtins.hasattr(obj, name)


def dir(obj):
    """Return the attribute names of ``obj``."""
    return builtins.dir(obj)


def type_name(obj):
    """Return the fully-qualified type name of ``obj``."""
    cls = type(obj)
    module = getattr(cls, '__module__', '')
    return f"{module}.{cls.__name__}" if module else cls.__name__


def is_module(obj):
    """Return True when ``obj`` is a Python module."""
    if isinstance(obj, ModuleProxy):
        return True
    return isinstance(obj, types.ModuleType)


def is_callable(obj):
    """Return True when ``obj`` can be called."""
    return callable(obj)


def is_class(obj):
    """Return True when ``obj`` is a class."""
    return isinstance(obj, type)


def is_instance(obj, cls):
    """Return True when ``obj`` is an instance of ``cls``."""
    return isinstance(obj, cls)


def to_aura(obj):
    """Convert a Python value into an Aura-friendly representation.

    Modules become :class:`ModuleProxy`; dicts, lists, tuples, and sets
    are recursively converted; everything else is returned as-is because
    Aura already operates on native Python objects.
    """
    if isinstance(obj, types.ModuleType):
        return ModuleProxy(obj)
    if isinstance(obj, dict):
        return {to_aura(k): to_aura(v) for k, v in obj.items()}
    if isinstance(obj, list):
        return [to_aura(v) for v in obj]
    if isinstance(obj, tuple):
        return tuple(to_aura(v) for v in obj)
    if isinstance(obj, set):
        return {to_aura(v) for v in obj}
    if isinstance(obj, frozenset):
        return frozenset(to_aura(v) for v in obj)
    if isinstance(obj, (int, float, str, bool, type(None))):
        return obj
    return obj


def to_python(obj):
    """Inverse of :func:`to_aura`: unwrap proxies before crossing back."""
    if isinstance(obj, ModuleProxy):
        return obj._module
    return obj


def interpreter_version():
    """Return ``(major, minor, micro, releaselevel, serial)`` of this Python."""
    return sys.version_info


def add_path(path):
    """Prepend ``path`` to ``sys.path`` (idempotent). Returns the path."""
    path = str(path)
    if path not in sys.path:
        sys.path.insert(0, path)
    return path


def site_packages():
    """Return the interpreter's site-packages directories.

    Returns an empty list if the site module is unavailable (e.g., in some
    embedded interpreters).
    """
    import site

    try:
        return list(site.getsitepackages())
    except AttributeError:
        return []


def modules():
    """Return the names of currently-imported modules (sorted)."""
    return sorted(n for n in sys.modules if not n.startswith('_'))


# Backwards-friendly aliases.
py_builtins = builtins


class ModuleProxy:
    """Attribute proxy that yields ``ModuleProxy`` for nested modules.

    This lets Aura write ``let math = python.load("math")`` and then use
    ``math.sqrt(2)`` with normal member access, even though the underlying
    import also returns a module on attribute access.
    """

    __slots__ = ('_module',)

    def __init__(self, module):
        self._module = module

    def __getattr__(self, name):
        value = getattr(self._module, name)
        if isinstance(value, types.ModuleType):
            return ModuleProxy(value)
        return value

    def __setattr__(self, name, value):
        if name == '_module':
            object.__setattr__(self, name, value)
        else:
            setattr(self._module, name, value)

    def __dir__(self):
        return dir(self._module)

    def __repr__(self):
        return f"<python.ModuleProxy {self._module.__name__!r}>"

    def __eq__(self, other):
        if isinstance(other, ModuleProxy):
            return self._module is other._module
        return self._module is other

    def __hash__(self):
        return hash(self._module)

    def __call__(self, *args, **kwargs):
        return self._module(*args, **kwargs)


def _default_globals():
    """A pristine namespace exposing Python builtins for ``eval``/``exec``."""
    import os

    return {'__builtins__': builtins, 'os': os}
