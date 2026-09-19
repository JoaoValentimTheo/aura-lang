"""Aura Standard Library - Regular expression module.

Thin, Aura-friendly wrappers over Python's :mod:`re`.
"""

import re as _re

__all__ = [
    "match", "full_match", "search", "find_all", "find_iter",
    "split", "replace", "replace_fn", "subn",
    "groups", "group_dict", "group",
    "escape", "compile_pattern", "purge", "error",
    "IGNORECASE", "MULTILINE", "DOTALL", "VERBOSE", "ASCII", "UNICODE",
]

error = _re.error


def match(pattern, text, flags=0):
    """Match ``pattern`` at the start of ``text``. Returns a match or None."""
    return _re.match(pattern, text, flags)


def full_match(pattern, text, flags=0):
    """Match ``pattern`` against the whole of ``text``."""
    return _re.fullmatch(pattern, text, flags)


def search(pattern, text, flags=0):
    """Search anywhere in ``text``. Returns a match or None."""
    return _re.search(pattern, text, flags)


def find_all(pattern, text, flags=0):
    """Return all non-overlapping matches as strings.

    When the pattern contains groups, a list of tuples is returned instead.
    """
    return _re.findall(pattern, text, flags)


def find_iter(pattern, text, flags=0):
    """Return an iterator over all non-overlapping matches.

    Each item is a match object with ``.group()``, ``.span()``, etc.
    """
    return _re.finditer(pattern, text, flags)


def split(pattern, text, limit=0, flags=0):
    """Split ``text`` by ``pattern``."""
    return _re.split(pattern, text, maxsplit=limit, flags=flags)


def replace(pattern, replacement, text, count=0, flags=0):
    """Replace matches of ``pattern`` with ``replacement`` (string or callable)."""
    return _re.sub(pattern, replacement, text, count=count, flags=flags)


def replace_fn(pattern, func, text, count=0, flags=0):
    """Replace matches using a callable ``func(match) -> str``."""
    return _re.sub(pattern, func, text, count=count, flags=flags)


def subn(pattern, replacement, text, count=0, flags=0):
    """Like ``replace`` but returns ``(new_string, number_of_subs_made)``."""
    return _re.subn(pattern, replacement, text, count=count, flags=flags)


def groups(pattern, text, flags=0):
    """Return captured groups of the first match as a list (or None)."""
    m = _re.search(pattern, text, flags)
    return list(m.groups()) if m else None


def group_dict(pattern, text, flags=0):
    """Return named captured groups of the first match as a dict (or None).

    Example::

        group_dict(r"(?P<year>\\d{4})", "2026-09-18")
        # => {"year": "2026"}
    """
    m = _re.search(pattern, text, flags)
    return m.groupdict() if m else None


def group(match_obj, *indices):
    """Extract group(s) from a match object.

    ``group(m)`` returns the entire match; ``group(m, 0)`` does the same.
    ``group(m, 1)`` returns the first captured group; ``group(m, 1, 2)``
    returns a tuple.

    Returns ``None`` if ``match_obj`` is ``None``.
    """
    if match_obj is None:
        return None
    if not indices:
        return match_obj.group(0)
    if len(indices) == 1:
        return match_obj.group(indices[0])
    return match_obj.group(*indices)


def escape(text):
    """Escape special characters in ``text``."""
    return _re.escape(text)


def compile_pattern(pattern, flags=0):
    """Compile ``pattern`` to a reusable pattern object."""
    return _re.compile(pattern, flags)


def purge():
    """Clear the regex pattern cache."""
    _re.purge()


# Flags
IGNORECASE = _re.IGNORECASE
MULTILINE = _re.MULTILINE
DOTALL = _re.DOTALL
VERBOSE = _re.VERBOSE
ASCII = _re.ASCII
UNICODE = _re.UNICODE
