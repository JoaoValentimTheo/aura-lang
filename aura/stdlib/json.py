"""Aura Standard Library - JSON module."""

import json as _json


def loads(s):
    """Parse JSON string to Python object."""
    return _json.loads(s)


def dumps(obj, indent=None):
    """Serialize object to JSON string (strict: NaN/Infinity rejected)."""
    return _json.dumps(obj, indent=indent, allow_nan=False)


def load(path):
    """Read and parse a JSON file."""
    with open(path, encoding='utf-8') as f:
        return _json.load(f)


def dump(obj, path, indent=None):
    """Write object as JSON to a file (strict: NaN/Infinity rejected)."""
    with open(path, 'w', encoding='utf-8') as f:
        _json.dump(obj, f, indent=indent, allow_nan=False)


def pretty(obj):
    """Serialize to pretty-printed JSON string (indent=2, strict)."""
    return _json.dumps(obj, indent=2, allow_nan=False)


def parse(s):
    """Alias for loads."""
    return _json.loads(s)


def stringify(obj):
    """Alias for dumps (no indent, strict)."""
    return _json.dumps(obj, allow_nan=False)


def is_valid(s):
    """Check if a string is valid, standards-compliant JSON."""
    if not isinstance(s, (str, bytes, bytearray)):
        return False
    try:
        _json.loads(s, parse_constant=_reject_constant)
        return True
    except (ValueError, TypeError):
        return False


def merge(*dicts):
    """Merge multiple dicts (last wins on key conflicts), return JSON string."""
    result = {}
    for d in dicts:
        if not hasattr(d, 'items'):
            raise TypeError("merge() arguments must be mappings")
        result.update(d)
    return _json.dumps(result, allow_nan=False)


def _reject_constant(token):
    raise ValueError(f"invalid JSON constant: {token}")
