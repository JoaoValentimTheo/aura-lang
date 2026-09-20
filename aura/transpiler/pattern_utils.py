"""Shared utilities for pattern and target name extraction.

Consolidates duplicated logic from expressions.py, rules.py, and semantics.py.
"""
from aura.transpiler.ast import (
    Identifier,
    IdentifierPattern,
    ListLiteral,
    ListPattern,
    SpreadExpr,
    TupleLiteral,
)


def pattern_names(pattern):
    """Extract all binding names from a pattern node.

    Works for IdentifierPattern, Identifier, TupleLiteral, ListLiteral,
    ListPattern, and SpreadExpr. Returns an empty list for unrecognized
    nodes or None.
    """
    if pattern is None:
        return []
    if isinstance(pattern, IdentifierPattern):
        return [pattern.name]
    if isinstance(pattern, Identifier):
        return [pattern.name]
    if isinstance(pattern, (TupleLiteral, ListLiteral)):
        names = []
        for el in pattern.elements:
            names.extend(pattern_names(el))
        return names
    if isinstance(pattern, ListPattern):
        names = []
        for sub in pattern.patterns:
            names.extend(pattern_names(sub))
        if getattr(pattern, 'rest_pattern', None) is not None:
            names.extend(pattern_names(pattern.rest_pattern))
        return names
    if isinstance(pattern, SpreadExpr):
        return pattern_names(pattern.expr)
    return []


def target_names(name):
    """Parse a declaration target string into bound names.

    Plain identifiers return ``[name]``. Tuple targets such as
    ``"(a, b)"`` and list targets such as ``"[a, b]"`` return each
    component name. Parenthesized/bracketed strings with commas are
    split on commas; ``*`` is treated as a separator.
    """
    if not name:
        return []
    if isinstance(name, str) and name[:1] in '([' and name[-1:] in ')]':
        inner = name[1:-1]
        names = []
        for part in inner.replace('*', ' ').split(','):
            token = part.strip().strip('()[]{}')
            if token and token.isidentifier():
                names.append(token)
        return names
    return [name] if isinstance(name, str) else []
