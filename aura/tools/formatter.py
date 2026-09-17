"""AST-aware Aura formatter - normalizes style, indentation, and spacing."""

import re

# Multi-character operators, longest first so masking is unambiguous.
_MULTI_OPS = ['**=', '??=', '<<=', '>>=', '->', '=>', '>=', '<=', '==', '!=',
              '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<', '>>',
              '??', '?:', '|>', '**', '..<', '..']

_STRING_RE = re.compile(
    r'(?:[rRbBfF]{0,2})("""(?:.|\n)*?"""|\'\'\'(?:.|\n)*?\'\'\'|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\')'
)
_UNARY_RE = re.compile(r'(?:(?<=[(,=\[\s])|^)\s*([-+])\s*(?=[\w\d(])')
_SINGLE_OP_RE = {
    op: re.compile(r'\s*' + re.escape(op) + r'\s*')
    for op in ('=', '+', '-', '*', '/', '%', '<', '>')
}
_MULTI_OP_RE = {
    op: re.compile(r'\s*' + re.escape(op) + r'\s*')
    for op in _MULTI_OPS
}
_AND_RE = re.compile(r'\band\b')
_OR_RE = re.compile(r'\bor\b')
_NOT_RE = re.compile(r'\bnot\b')
_REPEAT_SPACE_RE = re.compile(r'  +')
_COMMA_RE = re.compile(r',(\S)')
_SPACE_COMMA_RE = re.compile(r'\s+,')


def format_aura(source: str, width: int = 100, indent: int = 2) -> str:
    """Format Aura source code with proper indentation and style."""
    lines = source.split('\n')
    result = []
    indent_level = 0
    in_block_comment = False

    for line in lines:
        stripped = line.strip()

        # Handle block comments
        if in_block_comment:
            result.append(' ' * (indent_level * indent) + stripped)
            if '*/' in stripped:
                in_block_comment = False
            continue

        if stripped.startswith('/*'):
            if '*/' not in stripped:
                in_block_comment = True
            result.append(' ' * (indent_level * indent) + stripped)
            continue

        # Empty lines
        if not stripped:
            result.append('')
            continue

        # Comments
        if stripped.startswith('//'):
            result.append(' ' * (indent_level * indent) + stripped)
            continue

        # Decrease indent for closing braces
        if stripped in ('}', ):
            indent_level = max(0, indent_level - 1)

        # Decrease indent for else/catch/finally
        if stripped.startswith(('else', 'catch', 'finally')):
            indent_level = max(0, indent_level - 1)

        # Format the line
        formatted = _format_line(stripped)

        # Apply indentation
        indented = ' ' * (indent_level * indent) + formatted
        result.append(indented)

        # Increase indent for block starters
        if _is_block_start(stripped):
            indent_level += 1

    return '\n'.join(result)


def _format_line(line: str) -> str:
    """Format a single line of Aura code.

    Multi-character operators are masked before single-character spacing so
    that ``->`` is never split into ``- >``. String literals are protected so
    their contents are never rewritten.
    """
    # Protect string literals (single, double, triple, f/r/b prefixed).
    strings = []

    def _mask(match):
        strings.append(match.group(0))
        return f"\x00{len(strings) - 1}\x00"

    line = _STRING_RE.sub(_mask, line)

    # Comments: format only the code before `//`.
    comment = ''
    if '//' in line:
        idx = line.index('//')
        comment = line[idx:]
        line = line[:idx]

    # Mask multi-character operators so they survive single-char rules.
    placeholders = {}
    for i, op in enumerate(_MULTI_OPS):
        token = f"\x01{i}\x01"
        if op in line:
            placeholders[token] = op
            line = line.replace(op, token)

    # Normalize spaces around single-character operators.
    # Protect unary +/- first so `-5` is not turned into `- 5`.
    unary = []

    def _mask_unary(match):
        unary.append(match.group(0).lstrip())
        return f"\x02{len(unary) - 1}\x02"

    line = _UNARY_RE.sub(lambda m: _mask_unary(m), line)

    for op, pattern in _SINGLE_OP_RE.items():
        line = pattern.sub(f' {op} ', line)

    for i, original in enumerate(unary):
        line = line.replace(f"\x02{i}\x02", original)

    # Restore multi-char operators with normalized spacing.
    for token, op in placeholders.items():
        line = line.replace(token, op)
    # Collapse any spaces that restoration may have left around them.
    for op, pattern in _MULTI_OP_RE.items():
        line = pattern.sub(f' {op} ', line)

    # Normalize logical operators.
    line = _AND_RE.sub('and', line)
    line = _OR_RE.sub('or', line)
    line = _NOT_RE.sub('not', line)

    # Remove trailing whitespace and collapse repeated spaces.
    line = line.rstrip()
    line = _REPEAT_SPACE_RE.sub(' ', line)

    # Ensure space after comma and none before.
    line = _COMMA_RE.sub(r', \1', line)
    line = _SPACE_COMMA_RE.sub(',', line)

    # Restore strings and the comment, preserving original comment text.
    for i, original in enumerate(strings):
        line = line.replace(f"\x00{i}\x00", original)

    return line + comment


def _is_block_start(line: str) -> bool:
    """Check if a line opens a block that needs indentation.

    A one-line block such as ``def f() { return 1 }`` ends with ``}`` and must
    not indent the following lines, so only an *unclosed* ``{`` counts.
    """
    stripped = line.strip()

    opens_brace = ('{' in stripped
                   and stripped.count('{') > stripped.count('}'))

    # A fully closed one-liner never opens a block.
    if '{' in stripped and not opens_brace:
        return False

    if opens_brace:
        return True

    # Block keywords whose body is on following lines (no `{` on this line).
    if stripped in ('else', 'catch', 'finally'):
        return True
    for kw in ('def ', 'class ', 'if ', 'for ', 'while ', 'loop',
               'until ', 'unless ', 'match ', 'try', 'trait ',
               'module ', 'enum ', 'async def '):
        if stripped.startswith(kw):
            return True

    return False
