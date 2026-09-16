"""Aura Standard Library - Regular expression module.

Thin, Aura-friendly wrappers over Python's :mod:`re`.
"""

import re as _re


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
    """Return all non-overlapping matches as strings."""
    return _re.findall(pattern, text, flags)


def find_iter(pattern, text, flags=0):
    """Return all matches as a list of match objects."""
    return list(_re.finditer(pattern, text, flags))


def split(pattern, text, limit=0, flags=0):
    """Split ``text`` by ``pattern``."""
    return _re.split(pattern, text, maxsplit=limit, flags=flags)


def replace(pattern, replacement, text, count=0, flags=0):
    """Replace matches of ``pattern`` with ``replacement``."""
    return _re.sub(pattern, replacement, text, count=count, flags=flags)


def replace_fn(pattern, replacement, text, count=0, flags=0):
    """Replace matches using a function ``replacement(match)``."""
    return _re.sub(pattern, replacement, text, count=count, flags=flags)


def groups(pattern, text, flags=0):
    """Return captured groups of the first match (or None if no match)."""
    m = _re.search(pattern, text, flags)
    return list(m.groups()) if m else None


def escape(text):
    """Escape special characters in ``text``."""
    return _re.escape(text)


def compile_pattern(pattern, flags=0):
    """Compile ``pattern`` to a reusable pattern object."""
    return _re.compile(pattern, flags)


# Common flags
IGNORECASE = _re.IGNORECASE
MULTILINE = _re.MULTILINE
DOTALL = _re.DOTALL
VERBOSE = _re.VERBOSE
ASCII = _re.ASCII
