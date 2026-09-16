"""Built-in macros for Aura.

Aura macros are ordinary decorators backed by a small runtime prelude. The
transpiler injects the prelude at the top of the generated Python file only
when one of the built-in decorators is actually used, so plain programs stay
clean.
"""
from typing import List

# Names of the decorators provided by the runtime prelude.
BUILTIN_MACROS = (
    "debug",
    "timeit",
    "memoize",
    "cache",
    "must_return",
    "deprecated",
)


PRELUDE = '''# --- Aura runtime prelude (auto-injected for @debug/@timeit/@memoize/...) ---
import functools as _aura_functools
import time as _aura_time


def debug(fn):
    @_aura_functools.wraps(fn)
    def _wrapper(*args, **kwargs):
        print(f"DEBUG: enter {fn.__name__} args={args} kwargs={kwargs}")
        try:
            result = fn(*args, **kwargs)
        except Exception as exc:
            print(f"DEBUG: {fn.__name__} raised {exc!r}")
            raise
        print(f"DEBUG: exit {fn.__name__} -> {result!r}")
        return result
    return _wrapper


def timeit(fn):
    @_aura_functools.wraps(fn)
    def _wrapper(*args, **kwargs):
        _start = _aura_time.perf_counter()
        try:
            return fn(*args, **kwargs)
        finally:
            print(f"{fn.__name__} took {_aura_time.perf_counter() - _start:.4f}s")
    return _wrapper


def memoize(fn):
    _cache = {}

    def _make_key(value):
        # Try to hash directly; fall back to a repr-based key for unhashable
        # arguments (lists, dicts, sets) so memoization never crashes.
        try:
            hash(value)
            return value
        except TypeError:
            if isinstance(value, dict):
                return ("__dict__", tuple(sorted(
                    (_make_key(k), _make_key(v)) for k, v in value.items()
                )))
            if isinstance(value, (list, tuple)):
                return ("__seq__", tuple(_make_key(v) for v in value))
            if isinstance(value, (set, frozenset)):
                return ("__set__", frozenset(_make_key(v) for v in value))
            return ("__repr__", repr(value))

    @_aura_functools.wraps(fn)
    def _wrapper(*args, **kwargs):
        key = (_make_key(args),
               tuple(sorted((k, _make_key(v)) for k, v in kwargs.items())))
        if key not in _cache:
            _cache[key] = fn(*args, **kwargs)
        return _cache[key]
    _wrapper.cache_clear = _cache.clear
    return _wrapper


def cache(maxsize=128):
    return _aura_functools.lru_cache(maxsize=maxsize)


def must_return(fn):
    @_aura_functools.wraps(fn)
    def _wrapper(*args, **kwargs):
        result = fn(*args, **kwargs)
        if result is None:
            raise ValueError(f"{fn.__name__} must return a value")
        return result
    return _wrapper


def deprecated(fn=None, *, message=None):
    # Supports both ``@deprecated`` and ``@deprecated("message")``.
    def decorate(target):
        @_aura_functools.wraps(target)
        def _wrapper(*args, **kwargs):
            note = message or f"{target.__name__} is deprecated"
            print(f"WARNING: {note}")
            return target(*args, **kwargs)
        return _wrapper

    if callable(fn):
        return decorate(fn)
    if isinstance(fn, str) and message is None:
        message = fn
    return decorate
# --- end Aura runtime prelude ---
'''


def prelude_needed(decorator_names) -> bool:
    """Return True if any decorator is provided by the runtime prelude."""
    return any(name in BUILTIN_MACROS for name in decorator_names)


def list_macros() -> List[str]:
    return list(BUILTIN_MACROS)


# Names from the Aura standard library that are data-first and therefore
# useful with the pipe operator (e.g. `items |> map(fn)`).
STDLIB_PRELUDE_NAMES = ("map", "filter", "reduce", "take", "drop")

STDLIB_PRELUDE = (
    "from stdlib.collections import map, filter, reduce, take, drop\n"
)

# Dict literals transpile to AuraDict so `user.name` works alongside `user["name"]`.
DICT_PRELUDE = "from stdlib.collections import AuraDict\n"

# Enums transpile to Python enums.
ENUM_PRELUDE = "import enum as _aura_enum\n"

# Labeled break/continue need exception-based unwinding in Python.
LABEL_PRELUDE = '''# --- Aura labeled loop support ---
class _AuraBreak(Exception):
    def __init__(self, label):
        self.label = label


class _AuraContinue(Exception):
    def __init__(self, label):
        self.label = label
# --- end Aura labeled loop support ---
'''


def stdlib_prelude_needed(free_names) -> bool:
    """Return True if any data-first stdlib helper is referenced."""
    return any(name in STDLIB_PRELUDE_NAMES for name in free_names)