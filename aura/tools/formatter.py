"""AST-aware Aura formatter - normalizes style, indentation, and spacing."""

import re
import sys
from pathlib import Path

# Keywords that start a block
BLOCK_STARTERS = {
    'def', 'class', 'if', 'else', 'elif', 'for', 'while', 'loop',
    'until', 'unless', 'match', 'try', 'catch', 'finally', 'trait',
    'module', 'enum', 'guard', 'async',
}

# Keywords that should have a space after them
KEYWORD_SPACE_AFTER = {
    'let', 'const', 'mut', 'def', 'class', 'if', 'else', 'elif',
    'for', 'while', 'loop', 'until', 'unless', 'return', 'throw',
    'import', 'from', 'as', 'match', 'case', 'try', 'catch',
    'finally', 'trait', 'module', 'enum', 'async', 'await',
    'guard', 'else', 'break', 'continue',
}


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

    string_re = re.compile(
        r'(?:[rRbBfF]{0,2})("""(?:.|\n)*?"""|\'\'\'(?:.|\n)*?\'\'\'|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\')'
    )
    line = string_re.sub(_mask, line)

    # Comments: format only the code before `//`.
    comment = ''
    if '//' in line:
        idx = line.index('//')
        comment = line[idx:]
        line = line[:idx]

    # Mask multi-character operators so they survive single-char rules.
    multi = ['**=', '??=', '<<=', '>>=', '->', '=>', '>=', '<=', '==', '!=',
             '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<', '>>',
             '??', '?:', '|>', '**', '..<', '..']
    placeholders = {}
    for i, op in enumerate(multi):
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

    line = re.sub(r'(?:(?<=[(,=\[\s])|^)\s*([-+])\s*(?=[\w\d(])',
                  lambda m: _mask_unary(m), line)

    for op in ('=', '+', '-', '*', '/', '%', '<', '>'):
        line = re.sub(r'\s*' + re.escape(op) + r'\s*', f' {op} ', line)

    for i, original in enumerate(unary):
        line = line.replace(f"\x02{i}\x02", original)

    # Restore multi-char operators with normalized spacing.
    for token, op in placeholders.items():
        line = line.replace(token, op)
    # Collapse any spaces that restoration may have left around them.
    for op in multi:
        line = re.sub(r'\s*' + re.escape(op) + r'\s*', f' {op} ', line)

    # Normalize logical operators.
    line = re.sub(r'\band\b', 'and', line)
    line = re.sub(r'\bor\b', 'or', line)
    line = re.sub(r'\bnot\b', 'not', line)

    # Remove trailing whitespace and collapse repeated spaces.
    line = line.rstrip()
    line = re.sub(r'  +', ' ', line)

    # Ensure space after comma and none before.
    line = re.sub(r',(\S)', r', \1', line)
    line = re.sub(r'\s+,', ',', line)

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
