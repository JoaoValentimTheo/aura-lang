"""AST-aware Aura formatter - normalizes style, indentation, and spacing."""

import re

# Multi-character operators, longest first so masking is unambiguous.
_MULTI_OPS = ['**=', '??=', '<<=', '>>=', '->', '=>', '>=', '<=', '==', '!=',
              '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<', '>>',
              '??', '?:', '|>', '**', '..<', '..']

_STRING_RE = re.compile(
    r'(?:[rRbBfF]{0,2})("""(?:.|\n)*?"""|\'\'\'(?:.|\n)*?\'\'\'|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\')'
)
# A block comment that opens and closes on one line: `/* ... */`.
_BLOCK_COMMENT_RE = re.compile(r'/\*.*?\*/')
_UNARY_RE = re.compile(r'(?:(?<=[(,=\[\s])|^)\s*([-+])\s*(?=[\w\d(])')
# A prefix `*`/`**` (spread, varargs) is attached to the following name and
# must not be spaced like the multiplication/power operators. Matched after
# `(`, `,`, `[` or `{`, or at the start of a parameter list.
_SPREAD_RE = re.compile(r'(?:(?<=[(,\[{])|^)\s*(\*\*|\*)\s*(?=[A-Za-z_])')
_SINGLE_OP_RE = {
    op: re.compile(r'\s*' + re.escape(op) + r'\s*')
    for op in ('=', '+', '-', '*', '/', '%', '<', '>')
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
    # The triple-quote delimiter currently open, or None. Lines inside a
    # multi-line string are passed through verbatim: reformatting them would
    # change the string's value.
    in_triple = None

    for line in lines:
        stripped = line.strip()

        # Multi-line string literals are opaque: emit the raw line untouched
        # until the closing delimiter is seen.
        if in_triple is not None:
            result.append(line)
            if in_triple in line:
                in_triple = None
            continue

        opener = _open_triple_quote(stripped)
        if opener is not None:
            result.append(' ' * (indent_level * indent) + stripped)
            if not _triple_closes(stripped, opener):
                in_triple = opener
            continue

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


def _open_triple_quote(line: str):
    """Return the triple-quote delimiter that opens an unfinished string.

    ``None`` when the line has no triple-quoted literal, or when it opens and
    closes on the same line. Triple quotes inside single-line strings are
    ignored by scanning quotes left to right.
    """
    i = 0
    length = len(line)
    while i < length:
        ch = line[i]
        if ch in ('"', "'"):
            triple = line[i:i + 3]
            if triple == ch * 3:
                end = line.find(triple, i + 3)
                if end == -1:
                    return triple
                i = end + 3
                continue
            # Single-line string: skip to its closing quote.
            i += 1
            while i < length and line[i] != ch:
                if line[i] == '\\':
                    i += 1
                i += 1
        i += 1
    return None


def _triple_closes(line: str, triple: str) -> bool:
    """True when ``line`` contains a closing ``triple`` delimiter.

    The opener itself is never counted: the first occurrence after the opener
    position is treated as the close.
    """
    start = line.find(triple)
    if start == -1:
        return False
    return line.find(triple, start + 3) != -1


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

    # Protect inline block comments (after strings, so a `/*` inside a string
    # literal is not mistaken for a comment).
    block_comments = []

    def _mask_block(match):
        block_comments.append(match.group(0))
        return f"\x04{len(block_comments) - 1}\x04"

    line = _BLOCK_COMMENT_RE.sub(_mask_block, line)

    # Comments: format only the code before `//`.
    comment = ''
    if '//' in line:
        idx = line.index('//')
        comment = line[idx:]
        line = line[:idx]

    # Protect prefix `*`/`**` (spread, varargs) before the multi-char operator
    # pass, so `**` in `f(**b)` is not masked as the power operator. The sigil
    # and the name it prefixes are kept together.
    spreads = []

    def _mask_spread(match):
        spreads.append(match.group(1) + match.group(0).strip().lstrip('*'))
        return f"\x03{len(spreads) - 1}\x03"

    line = _SPREAD_RE.sub(_mask_spread, line)

    # Mask multi-character operators so they survive single-char rules.
    for i, op in enumerate(_MULTI_OPS):
        token = f"\x01{i}\x01"
        if op in line:
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

    # Normalize spacing around multi-char operators while they are still
    # masked. Doing this after restoring them is unsafe: `..` would re-split a
    # restored `..<` into `.. <`, producing invalid code.
    for i, op in enumerate(_MULTI_OPS):
        token = f"\x01{i}\x01"
        if token in line:
            line = re.sub(r'\s*' + re.escape(token) + r'\s*', f' {token} ', line)
            line = line.replace(token, op)

    # Restore prefix `*`/`**` only now, so they are never treated as operators.
    for i, original in enumerate(spreads):
        line = line.replace(f"\x03{i}\x03", original)

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
    for i, original in enumerate(block_comments):
        line = line.replace(f"\x04{i}\x04", original)

    line = line.rstrip()
    if comment and line:
        # Keep a single space between code and an attached trailing comment.
        return line + ' ' + comment
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
