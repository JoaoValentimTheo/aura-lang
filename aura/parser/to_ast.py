"""
Complete recursive descent parser for Aura.
Handles expressions, control flow, functions, classes, and more.
"""
import contextlib
import re
import unicodedata

from aura.transpiler.ast import *

# A module path in `export Name from "..."` must be a plain dotted name. This
# rejects separators, traversal and NUL so the value can never escape the
# source folder when the transformer resolves it.
_INVALID_MODULE_PATH = re.compile(r'[\\/\x00]|\.\.')

# Identifier characters follow Python's own rules (PEP 3131: XID_Start /
# XID_Continue) rather than `str.isalpha()`/`isalnum()`. `isalpha()` accepts
# 22 codepoints Python rejects as an identifier start (e.g. Arabic ligatures
# U+FC5E..U+FDFB) and `isalnum()` misses two valid continuation characters
# (U+2118, U+212E), so guessing here can emit invalid Python or split a name.
_IDENT_ASCII_START = frozenset(
    'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_'
)
_IDENT_ASCII_CONT = frozenset(
    'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789'
)


def _ident_start(char):
    """True when ``char`` may begin an identifier."""
    if char in _IDENT_ASCII_START:
        return True
    return char.isidentifier()


def _ident_continue(char):
    """True when ``char`` may appear after the first identifier character."""
    if char in _IDENT_ASCII_CONT:
        return True
    # A continuation character is any standalone identifier codepoint, or an
    # XID_Continue-only one (combining marks, ZWJ-like format characters),
    # which Python recognizes as part of an identifier when not leading.
    return char.isidentifier() or ('a' + char).isidentifier()

# ==============================================================================
# Tokenizer
# ==============================================================================

class Token:
    def __init__(self, type, value, line, column):
        self.type = type
        self.value = value
        self.line = line
        self.column = column

    def __repr__(self):
        return f"Token({self.type}, {repr(self.value)})"


def _is_ascii_digit(ch: str) -> bool:
    """True only for ``0``-``9``.

    ``str.isdigit()`` accepts Unicode digits like ``¹`` or ``٣`` which
    ``int()`` then rejects. Number scanning must be ASCII-only so the lexer
    never produces a token it cannot convert.
    """
    return '0' <= ch <= '9'


def annotate_syntax_error(exc, line=None, column=None, filename=None):
    """Attach structured location attributes to a ``SyntaxError``.

    Python's ``SyntaxError`` has no public ``line``/``column``/``filename``
    contract for us to set directly, so ``setattr`` is used; mypy-safe and the
    CLI/LSP read these back via ``getattr``.
    """
    if line is not None:
        exc.line = line  # type: ignore[attr-defined]
    if column is not None:
        exc.column = column  # type: ignore[attr-defined]
    if filename is not None:
        exc.filename = filename  # type: ignore[attr-defined]
    return exc

class Tokenizer:
    def __init__(self, source, filename: str = "<aura>"):
        self.source = source
        self.pos = 0
        self.line = 1
        self.column = 1
        self.tokens: list = []
        self.filename = filename

    def error(self, message, token=None):
        """Build a ``SyntaxError`` carrying the tokenizer's position.

        When ``token`` is given, its line/column are used instead of the
        tokenizer's current cursor, so an error raised after scanning past a
        construct still points at where the construct began.
        """
        line = getattr(token, 'line', None) or self.line
        column = getattr(token, 'column', None) or self.column
        exc = SyntaxError(f"{message} (line {line})")
        return annotate_syntax_error(exc, line, column, self.filename)

    _ESCAPES = {
        'n': '\n', 't': '\t', 'r': '\r', '0': '\0',
        '\\': '\\', '"': '"', "'": "'", 'b': '\b',
        'f': '\f', 'v': '\v', 'a': '\a',
    }

    @classmethod
    def _decode_escapes(cls, raw):
        """Decode backslash escapes in a non-raw string body.

        Unknown escapes keep the escaped character (e.g. ``\\d`` becomes ``d``,
        matching Aura's pragmatic string semantics rather than raising).
        """
        if '\\' not in raw:
            return raw
        out = []
        i = 0
        n = len(raw)
        while i < n:
            ch = raw[i]
            if ch == '\\' and i + 1 < n:
                nxt = raw[i + 1]
                if nxt in cls._ESCAPES:
                    out.append(cls._ESCAPES[nxt])
                elif nxt == 'x' and i + 3 < n:
                    try:
                        out.append(chr(int(raw[i + 2:i + 4], 16)))
                        i += 4
                        continue
                    except ValueError:
                        out.append(nxt)
                elif nxt == 'u' and i + 5 < n:
                    try:
                        cp = int(raw[i + 2:i + 6], 16)
                        if 0xD800 <= cp <= 0xDFFF:
                            raise ValueError("lone surrogate")
                        out.append(chr(cp))
                        i += 6
                        continue
                    except ValueError:
                        out.append(nxt)
                elif nxt == 'U' and i + 9 < n:
                    try:
                        out.append(chr(int(raw[i + 2:i + 10], 16)))
                        i += 10
                        continue
                    except ValueError:
                        out.append(nxt)
                else:
                    out.append(nxt)
                i += 2
                continue
            out.append(ch)
            i += 1
        return ''.join(out)

    def tokenize(self):
        length = len(self.source)
        while self.pos < length:
            char = self.source[self.pos]

            # Whitespace
            if char.isspace():
                if char == '\n':
                    self.line += 1
                    self.column = 1
                else:
                    self.column += 1
                self.pos += 1
                continue

            # Comments
            # `//` always starts a single-line comment. Aura has no floor
            # division operator; use integer casting on a float division
            # (e.g. `int(a / b)`).
            if char == '/' and self.pos + 1 < length and self.source[self.pos + 1] == '/':
                while self.pos < length and self.source[self.pos] != '\n':
                    self.pos += 1
                continue

            # Block comments: /* ... */ (may span multiple lines).
            if char == '/' and self.pos + 1 < length and self.source[self.pos + 1] == '*':
                comment_line, comment_col = self.line, self.column
                self.pos += 2
                while self.pos < length and not (
                    self.source[self.pos] == '*' and self.pos + 1 < length
                    and self.source[self.pos + 1] == '/'
                ):
                    if self.source[self.pos] == '\n':
                        self.line += 1
                        self.column = 1
                    self.pos += 1
                if self.pos >= length:
                    raise self.error(
                        "unterminated block comment",
                        Token('OP', '/*', comment_line, comment_col),
                    )
                self.pos += 2  # skip closing */
                continue

            # Identifiers and Keywords
            if _ident_start(char):
                start = self.pos
                start_line, start_col = self.line, self.column
                while self.pos < length and _ident_continue(self.source[self.pos]):
                    self.pos += 1
                value = unicodedata.normalize('NFC', self.source[start:self.pos])

                # F-string support (special case: identifier 'f' followed by quote).
                # Supports escapes, nested braces and triple quotes. The raw
                # inner text is stored; parsing into parts happens later so
                # interpolated Aura code can be transformed.
                if value.lower() == 'f' and self.pos < length and self.source[self.pos] in ('"', "'"):
                    quote = self.source[self.pos]
                    is_triple = self.source[self.pos:self.pos+3] == quote * 3
                    closing = quote * 3 if is_triple else quote
                    self.pos += len(closing)
                    start_str = self.pos
                    depth = 0
                    while self.pos < length:
                        if self.source[self.pos] == '\\':
                            self.pos += 2
                            continue
                        if self.source[self.pos] == '{':
                            depth += 1
                        elif self.source[self.pos] == '}':
                            if depth > 0:
                                depth -= 1
                        elif depth == 0 and self.source[self.pos:self.pos+len(closing)] == closing:
                            break
                        if self.source[self.pos] == '\n':
                            self.line += 1
                        self.pos += 1
                    str_value = self.source[start_str:self.pos]
                    self.pos += len(closing)  # skip closing delimiter
                    self.column += (self.pos - start)
                    self.tokens.append(Token('FSTRING', (quote, str_value), start_line, start_col))
                    continue

                # Raw / byte string prefixes: r"", b"", rb"", br"" (any case).
                # The literal is kept verbatim so backslashes are not
                # re-escaped and bytes semantics are preserved.
                if (value.lower() in ('r', 'b', 'rb', 'br')
                        and self.pos < length
                        and self.source[self.pos] in ('"', "'")):
                    quote = self.source[self.pos]
                    is_triple = self.source[self.pos:self.pos + 3] == quote * 3
                    closing = quote * 3 if is_triple else quote
                    body_start = self.pos + len(closing)
                    j = body_start
                    while j < length:
                        # In a raw string a backslash still escapes the quote
                        # delimiter (`r"a\"b"` is one string), so skip a
                        # backslash and the character after it even for `r`.
                        if self.source[j] == '\\':
                            j += 2
                            continue
                        if self.source[j] == '\n':
                            self.line += 1
                        if self.source[j:j + len(closing)] == closing:
                            break
                        j += 1
                    if j >= length:
                        raise self.error(
                            "unterminated string literal",
                            Token('RAWSTRING', self.source[start:body_start], start_line, start_col),
                        )
                    end = j + len(closing)
                    raw_literal = self.source[start:end]
                    self.column += (end - start)
                    self.pos = end
                    self.tokens.append(Token('RAWSTRING', raw_literal, start_line, start_col))
                    continue

                self.column += (self.pos - start)
                if value == 'volatily':
                    raise self.error(
                        "'volatily' is not a keyword; did you mean 'volatile'?"
                    )
                self.tokens.append(Token('IDENT', value, start_line, start_col))
                continue

            # Numbers
            if _is_ascii_digit(char):
                start = self.pos
                start_line, start_col = self.line, self.column

                # Radix prefixes: 0x, 0o, 0b (with optional _ separators).
                if char == '0' and self.pos + 1 < length and self.source[self.pos + 1] in 'xXoObB':
                    prefix = self.source[self.pos + 1].lower()
                    self.pos += 2
                    digits = ''
                    while self.pos < length and (
                        '0' <= self.source[self.pos] <= '9'
                        or 'a' <= self.source[self.pos].lower() <= 'f'
                        or self.source[self.pos] == '_'
                    ):
                        if self.source[self.pos] != '_':
                            digits += self.source[self.pos]
                        self.pos += 1
                    base = {'x': 16, 'o': 8, 'b': 2}[prefix]
                    try:
                        value = int(digits, base)
                    except ValueError:
                        raise self.error(
                            f"Invalid base-{base} integer literal "
                            f"'0{prefix}{digits}'"
                        ) from None
                    self.column += (self.pos - start)
                    self.tokens.append(Token('INT', value, start_line, start_col))
                    continue

                # Decimal integer part (underscores allowed as separators).
                while self.pos < length and (_is_ascii_digit(self.source[self.pos]) or self.source[self.pos] == '_'):
                    self.pos += 1

                # Check for float dot vs range (..)
                is_float = False
                if (self.pos < length and self.source[self.pos] == '.'
                        and self.pos + 1 < length and _is_ascii_digit(self.source[self.pos + 1])):
                    is_float = True
                    self.pos += 1  # consume dot
                    while self.pos < length and (_is_ascii_digit(self.source[self.pos]) or self.source[self.pos] == '_'):
                        self.pos += 1

                # Scientific notation: 1e10, 1.5e-5
                if self.pos < length and self.source[self.pos] in 'eE':
                    nxt = self.source[self.pos + 1] if self.pos + 1 < length else ''
                    after = self.source[self.pos + 2] if self.pos + 2 < length else ''
                    if _is_ascii_digit(nxt) or (nxt in '+-' and _is_ascii_digit(after)):
                        is_float = True
                        self.pos += 1
                        if self.source[self.pos] in '+-':
                            self.pos += 1
                        while self.pos < length and _is_ascii_digit(self.source[self.pos]):
                            self.pos += 1

                raw = self.source[start:self.pos].replace('_', '')
                self.column += (self.pos - start)

                if is_float:
                    self.tokens.append(Token('FLOAT', float(raw), start_line, start_col))
                else:
                    self.tokens.append(Token('INT', int(raw), start_line, start_col))
                continue

            # Leading-dot floats: `.5` (but not `..` ranges or member access).
            if (char == '.' and self.pos + 1 < length
                    and _is_ascii_digit(self.source[self.pos + 1])):
                start = self.pos
                self.pos += 1
                while self.pos < length and (_is_ascii_digit(self.source[self.pos]) or self.source[self.pos] == '_'):
                    self.pos += 1
                if self.pos < length and self.source[self.pos] in 'eE':
                    nxt = self.source[self.pos + 1] if self.pos + 1 < length else ''
                    after = self.source[self.pos + 2] if self.pos + 2 < length else ''
                    if _is_ascii_digit(nxt) or (nxt in '+-' and _is_ascii_digit(after)):
                        self.pos += 1
                        if self.source[self.pos] in '+-':
                            self.pos += 1
                        while self.pos < length and _is_ascii_digit(self.source[self.pos]):
                            self.pos += 1
                raw = self.source[start:self.pos].replace('_', '')
                start_line, start_col = self.line, self.column
                self.column += (self.pos - start)
                self.tokens.append(Token('FLOAT', float(raw), start_line, start_col))
                continue

            # Strings
            if char in ('"', "'"):
                quote = char
                start_line, start_col = self.line, self.column
                # Triple-quoted multi-line string.
                if self.source[self.pos:self.pos+3] == quote * 3:
                    self.pos += 3
                    start = self.pos
                    while self.pos < length and self.source[self.pos:self.pos+3] != quote * 3:
                        if self.source[self.pos] == '\\':
                            self.pos += 2
                            continue
                        if self.source[self.pos] == '\n':
                            self.line += 1
                        self.pos += 1
                    if self.pos >= length:
                        raise self.error(
                            "unterminated string literal",
                            Token('STRING', quote, start_line, start_col),
                        )
                    value = self.source[start:self.pos]
                    self.pos += 3  # skip closing quotes
                    self.column += (self.pos - start)
                    self.tokens.append(Token('STRING', self._decode_escapes(value), start_line, start_col))
                    continue

                self.pos += 1
                start = self.pos
                while self.pos < length and self.source[self.pos] != quote:
                    if self.source[self.pos] == '\\':
                        self.pos += 2 # Skip escaped char
                        continue
                    if self.source[self.pos] == '\n':
                        self.line += 1
                    self.pos += 1
                if self.pos >= length:
                    raise self.error(
                        "unterminated string literal",
                        Token('STRING', quote, start_line, start_col),
                    )
                value = self.source[start:self.pos]
                self.pos += 1 # Skip closing quote
                self.column += (self.pos - start + 2)
                self.tokens.append(Token('STRING', self._decode_escapes(value), start_line, start_col))
                continue

            # Operators involving multiple chars
            # Check 3 chars first
            if self.pos + 2 < length:
                three_chars = self.source[self.pos:self.pos+3]
                if three_chars in ('..<', '...', '??=', '**=', '<<=', '>>='):
                    self.tokens.append(Token('OP', three_chars, self.line, self.column))
                    self.pos += 3
                    self.column += 3
                    continue

            if self.pos + 1 < length:
                two_chars = self.source[self.pos:self.pos+2]
                if two_chars in ('==', '!=', '<=', '>=', '->', '=>', '&&', '||', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<', '>>', '..', '??', '?:', '|>', '**', '?.', '?['):
                    self.tokens.append(Token('OP', two_chars, self.line, self.column))
                    self.pos += 2
                    self.column += 2
                    continue

            # Single char operators
            self.tokens.append(Token('OP', char, self.line, self.column))
            self.pos += 1
            self.column += 1

        self.tokens.append(Token('EOF', '', self.line, self.column))
        return self.tokens

# Keywords that can begin a statement. Used to decide whether a bare `yield`
# has a value or stands alone.
_STATEMENT_KEYWORDS = frozenset({
    'let', 'const', 'def', 'async', 'class', 'trait', 'enum', 'type',
    'module', 'import', 'from', 'if', 'unless', 'until', 'while', 'for',
    'loop', 'guard', 'throw', 'return', 'break', 'continue', 'assert', 'try',
    'with', 'match', 'case', 'else', 'catch', 'finally', 'public', 'private',
    'protected', 'static', 'abstract', 'export',
})

# Operator precedence, hoisted out of `Parser.get_precedence` so the parser
# does not rebuild a ~40-entry dict on every operator token.
_PRECEDENCE = {
    '=': 1, '+=': 1, '-=': 1, '*=': 1, '/=': 1, '%=': 1,
    '**=': 1, '&=': 1, '|=': 1, '^=': 1, '<<=': 1, '>>=': 1, '??=': 1,
    '?': 2,  # Ternary
    'or': 3,
    'and': 4,
    # Bitwise operators bind looser than equality but tighter than
    # `and`/`or`, mirroring the grammar's bitwiseOr/Xor/And chain.
    '|': 4.2,
    '^': 4.4,
    '&': 4.6,
    '==': 5, '!=': 5, '<': 6, '>': 6, '<=': 6, '>=': 6,
    'in': 6, 'not in': 6, 'is': 6, 'is not': 6,
    '..': 7, '..<': 7,  # Range
    '??': 8, '?:': 8,  # Null coalescing/Elvis
    # Shifts bind looser than additive but tighter than comparison,
    # matching Python (`1 << 2 + 1` == `1 << 3`).
    '<<': 5.5, '>>': 5.5,
    '+': 9, '-': 9,
    '*': 10, '/': 10, '%': 10, 'as': 10,
    '**': 11,
    '|>': 1,  # Pipe has the lowest precedence, handled separately
    '.': 12, '[': 12, '(': 12, '?.': 12,
}

# Upper bound on source size, guarding against accidental multi-gigabyte input.
MAX_SOURCE_BYTES = 16 * 1024 * 1024

# Aura keywords that cannot be used as a binding name. Declaring one is a
# parse-time error rather than a confusing downstream failure.
_RESERVED_BINDING_NAMES = frozenset({
    'if', 'else', 'unless', 'until', 'while', 'for', 'in', 'loop', 'guard',
    'match', 'case', 'try', 'catch', 'finally', 'throw', 'return', 'break',
    'continue', 'yield', 'def', 'class', 'trait', 'enum', 'module', 'import',
    'from', 'as', 'let', 'const', 'type', 'async', 'await', 'spawn', 'and',
    'or', 'not', 'is', 'true', 'false', 'none', 'self', 'super', 'new',
    'public', 'private', 'protected', 'static', 'volatile', 'abstract',
    'export', 'with', 'assert', 'fn',
})

# Python boolean/None spellings that Aura deliberately spells differently.
_PYTHON_LITERAL_ALIASES = {
    'True': 'true',
    'False': 'false',
    'None': 'none',
}

# ==============================================================================
# Parser
# ==============================================================================

class Parser:
    # Aura method names that map to Python dunder names. The canonical table
    # lives in ``aura.transpiler.ast`` so the parser and the transformers agree.
    _SPECIAL_METHOD_NAMES = SPECIAL_METHOD_NAMES

    def __init__(self, tokens, filename: str = "<aura>"):
        self.tokens = tokens
        self._ntokens = len(tokens)
        self.pos = 0
        self.filename = filename
        # Depth of `case`-pattern parsing. Inside a pattern, `{` starts the
        # case body, never a struct literal, so struct-init must be suppressed.
        self._pattern_depth = 0
        # Depth of control-flow condition parsing. A condition like
        # `if Point { ... }` must treat `{` as the block opener, never a
        # struct literal, even when the condition is an uppercase identifier.
        self._no_struct_depth = 0

    # --- Diagnostics ---
    def _check_binding_name(self, token):
        """Reject keywords and Python-shaped literal spellings as bindings.

        Diagnosing these at the declaration site gives a clear message instead
        of a confusing error later in the pipeline.
        """
        if token.value in _RESERVED_BINDING_NAMES:
            hint = ""
            if token.value in ('true', 'false', 'none'):
                hint = (
                    f" ('{token.value}' is a literal; choose a different name)")
            raise self.error(
                f"'{token.value}' is a reserved keyword and cannot be used as "
                f"a variable name{hint}", token)
        alias = _PYTHON_LITERAL_ALIASES.get(token.value)
        if alias is not None:
            raise self.error(
                f"'{token.value}' is not an Aura literal; write '{alias}' "
                "instead (and choose a different binding name)", token)
        return token.value

    def error(self, message, token=None):
        """Build a ``SyntaxError`` carrying structured line/column info.

        The CLI and the LSP read ``.line``/``.column``/``.filename`` from the
        exception, so diagnostics point at the real source position instead of
        defaulting to line 1. The message text keeps the ``... (line N)``
        suffix for tools that only show the string.
        """
        token = token if token is not None else self.peek()
        exc = SyntaxError(f"{message} (line {token.line})")
        return annotate_syntax_error(exc, token.line, token.column, self.filename)

    # --- Token Management ---
    def peek(self, offset=0):
        # `self.tokens` always ends with an EOF token, so an index within range
        # is the common case; the cached length avoids a `len()` subcall on the
        # hot path (peek is called hundreds of thousands of times).
        index = self.pos + offset
        if index < self._ntokens:
            return self.tokens[index]
        return self.tokens[-1]

    def consume(self, expected_type=None, expected_value=None):
        if self.pos >= self._ntokens:
            raise self.error("Unexpected end of file",
                             self.tokens[-1] if self.tokens else Token('EOF', '', 1, 1))
        token = self.tokens[self.pos]
        self.pos += 1
        if expected_type and token.type != expected_type:
            raise self.error(
                f"Expected {expected_type} but got {token.type} '{token.value}'",
                token)
        if expected_value and token.value != expected_value:
            raise self.error(
                f"Expected '{expected_value}' but got '{token.value}'", token)
        return token

    def match(self, value):
        if self.tokens[self.pos].value == value:
            self.pos += 1
            return True
        return False

    def check(self, value):
        return self.tokens[self.pos].value == value

    def check_type(self, type_name):
        return self.tokens[self.pos].type == type_name

    # --- Main Entry Point ---
    def parse(self):
        statements = []
        try:
            while self.peek().type != 'EOF':
                stmt = self.parse_statement()
                if stmt:
                    statements.append(stmt)
        except RecursionError:
            raise self.error("source is nested too deeply to parse") from None
        # A real file path (not the `<aura>` placeholder) is kept so the
        # transformer can resolve sibling re-exports relative to it.
        source_path = None if self.filename in (None, '<aura>') else self.filename
        return Program(statements, source_path=source_path)

    # --- Statements ---
    def parse_modifiers(self):
        """Parse visibility and other modifiers.

        Returns ``visibility=None`` when no visibility keyword was written, so
        the rule checker can require an explicit modifier on class members.
        """
        visibility = None
        is_static = False
        is_volatile = False
        is_abstract = False

        while self.peek().value in ['public', 'private', 'protected', 'static', 'volatile', 'abstract']:
            val = self.consume().value
            if val in ['public', 'private', 'protected']:
                visibility = val
            elif val == 'static':
                is_static = True
            elif val == 'volatile':
                is_volatile = True
            elif val == 'abstract':
                is_abstract = True

        return visibility, is_static, is_volatile, is_abstract

    def _parse_decorator_name(self):
        """Parse a decorator's callee, which may be a dotted member path.

        A decorator is usually a bare name (``@memoize``), but framework-style
        routing attaches one to an attribute of an object
        (``@app.route("/")``). Member paths are stored as a dotted string so
        the transformer can render ``@{name}`` verbatim.
        """
        parts = [self.consume(expected_type='IDENT').value]
        while self.check('.'):
            self.consume()
            parts.append(self.consume(expected_type='IDENT').value)
        return ".".join(parts)

    def _parse_decorator_arguments(self):
        """Parse ``(args..., **kw, key=value)`` for a decorator.

        Starred arguments become :class:`SpreadExpr` (``*args`` and
        ``**kwargs`` in a decorator is valid Python), unlike a plain
        expression where a leading ``*`` would be read as multiplication.
        """
        args = []
        kwargs = {}
        if not self.match('('):
            return args, kwargs
        if not self.check(')'):
            while True:
                is_named = (self.check_type('IDENT')
                            and self.peek(1).value in ('=', ':'))
                if is_named:
                    key = self.consume().value
                    self.consume()
                    kwargs[key] = self.parse_expression()
                elif self.match('**'):
                    args.append(SpreadExpr(self.parse_expression(), is_dict=True))
                elif self.match('*') or self.match('...'):
                    args.append(SpreadExpr(self.parse_expression(), is_dict=False))
                else:
                    args.append(self.parse_expression())
                if not self.match(','): break
        self.consume(expected_value=')')
        return args, kwargs

    def parse_statement(self):
        """Parse one statement and record its source location for diagnostics."""
        start = self.peek()
        if start.value == '...':
            raise self.error(
                "unexpected '...' at start of a statement; use '..'/'..<' "
                "for ranges", start)
        stmt = self._parse_statement()
        if stmt is not None:
            with contextlib.suppress(AttributeError, TypeError):
                stmt.line = start.line
                stmt.location = SourceLocation(
                    self.filename, start.line, start.column, 0)
        return stmt

    def _parse_statement(self):
        # 1. Handle Decorators and Modifiers (Prefixes)
        decorators = []
        visibility = 'public'
        is_static = False
        is_volatile = False
        is_abstract = False


        while True:
            token = self.peek()
            if token.value == 'export':
                # `export` is consumed before the declaration (see
                # `parse_module_decl`); reaching it here means an export outside
                # a module, which is meaningless.
                raise self.error(
                    "'export' is only meaningful inside a 'module' body")
            elif token.value == '@':
                # Parse decorators
                while self.match('@'):
                    dec_name = self._parse_decorator_name()
                    dec_args, dec_kwargs = self._parse_decorator_arguments()
                    decorators.append(Decorator(dec_name, dec_args, dec_kwargs))
            elif token.value in ['public', 'private', 'protected', 'static', 'volatile', 'abstract']:
                v, s, vol, abst = self.parse_modifiers()
                # Last visibility wins, flags accumulate. At module scope
                # visibility is metadata only, so an omitted modifier means
                # public (class members are handled separately).
                if v is not None: visibility = v
                if s: is_static = True
                if vol: is_volatile = True
                if abst: is_abstract = True
            else:
                break

        if is_abstract and not (self.check('class') or
                                self.check('def') or
                                (self.check('async')
                                 and self.peek(1).value == 'def')):
            raise self.error(
                "'abstract' applies to a class or a method "
                "(write 'abstract class C' or 'abstract def f()')",
                self.peek())

        # Labeled loop: `outer: for x in ... { ... }`.
        if (self.check_type('IDENT') and self.peek(1).value == ':'
                and self.peek(2).value in ('for', 'while', 'until', 'loop')):
            label = self.consume().value
            self.consume(expected_value=':')
            loop = self.parse_statement()
            if isinstance(loop, (ForStmt, WhileStmt, UntilStmt, LoopStmt)):
                loop.label = label
            return loop

        token = self.peek()

        if token.value == 'let':
            return self.parse_var_decl(visibility, is_static, is_volatile)
        elif token.value == 'mut':
            # `mut x = 1` would otherwise parse as a bare `mut` reference
            # followed by an assignment, silently declaring an immutable `x`.
            # Only `let mut x` declares a mutable binding outside a class header.
            raise self.error(
                "'mut' cannot start a statement; write 'let mut x = ...'", token)
        elif token.value == 'const':
            return self.parse_const_decl()
        elif token.value == 'fn':
            raise self.error(
                "'fn' is not part of Aura; use 'def' instead", token)
        elif token.value in ('def', 'async'):
            # `async def` declares an async function; `async with` opens an
            # async context manager. Anything else after `async` is an error.
            if token.value == 'async' and self.peek(1).value == 'with':
                self.consume()
                return self.parse_with_stmt(is_async=True)
            return self.parse_function_decl(decorators, visibility, is_static, is_volatile,
                                             is_abstract=is_abstract)
        elif token.value == 'class':
            return self.parse_class_decl(decorators, visibility, is_abstract=is_abstract)
        elif token.value == 'module':
            return self.parse_module_decl()
        elif token.value == 'type':
             return self.parse_type_decl()
        elif token.value == 'trait':
             return self.parse_trait_decl()
        elif token.value == 'enum':
             return self.parse_enum_decl(visibility)
        elif token.value == 'import':
             return self.parse_import_stmt()
        elif token.value == 'from':
             return self.parse_from_import_stmt()
        elif token.value == 'if':
            return self.parse_if_stmt()
        elif token.value == 'unless':
            return self.parse_unless_stmt()
        elif token.value == 'until':
            return self.parse_until_stmt()
        elif token.value == 'loop':
            return self.parse_loop_stmt()
        elif token.value == 'while':
            return self.parse_while_stmt()
        elif token.value == 'for':
            return self.parse_for_stmt()
        elif token.value == 'guard':
            return self.parse_guard_stmt()
        elif token.value == 'throw':
            return self.parse_throw_stmt()
        elif token.value == 'return':
            return self.parse_return_stmt()
        elif token.value == 'break':
            self.consume()
            label = None
            if self.check_type('IDENT'):
                label = self.consume().value
            if self.check(';'): self.consume()
            return BreakStmt(label)
        elif token.value == 'continue':
            self.consume()
            label = None
            if self.check_type('IDENT'):
                label = self.consume().value
            if self.check(';'): self.consume()
            return ContinueStmt(label)

        elif token.value == 'assert':
            return self.parse_assert_stmt()

        elif token.value == 'try':
            return self.parse_try_stmt()
        elif token.value == 'with':
            return self.parse_with_stmt()
        elif token.value == 'match':
            return self.parse_match_stmt()
        elif token.value == 'await':
             # await as statement (expression ignored)
             expr = self.parse_expression()
             if self.check(';'): self.consume()
             return ExprStmt(expr)

        elif token.value == 'case':
            return self.parse_case_stmt()

        elif token.value == ';':
            self.consume()
            return None
        # elif token.value == '}' - Let parse_expression failure handle it

        # Expression statement
        expr = self.parse_expression()
        expr = self.parse_trailing_tuple(expr)
        if self.check(';'):
            self.consume()
        return ExprStmt(expr)

    def parse_trailing_tuple(self, expr):
        """Collect comma-separated expressions into a TupleLiteral.

        Used at statement level for forms like ``return a, b`` and
        ``x = 1, 2``. Comma is intentionally *not* part of ``parse_expression``
        so that argument lists keep working.
        """
        if not self.check(','):
            return expr
        elements = [expr]
        while self.match(','):
            if self.check(';') or self.check('}') or self.check(')') or self.check(']'):
                break
            elements.append(self.parse_expression())

        # Multi-target assignment: `a, b = b, a` parses as a tuple whose middle
        # element is a BinaryOp('='), because comma binds looser than `=`.
        # Normalize it to `BinaryOp('=', (a, b), (b, a))` so the transformer
        # emits a single Python assignment instead of an invalid tuple.
        assign_index = next(
            (i for i, el in enumerate(elements)
             if isinstance(el, BinaryOp) and el.op == '='),
            None,
        )
        if assign_index is not None:
            assign = elements[assign_index]
            targets = list(elements[:assign_index]) + [assign.left]
            values = [assign.right] + list(elements[assign_index + 1:])
            return BinaryOp(
                TupleLiteral(targets),
                '=',
                TupleLiteral(values),
            )

        # For assignments, the tuple belongs to the right-hand side only:
        # `x = 1, 2` must become `x = (1, 2)`, not `(x = 1), 2`.
        if isinstance(expr, BinaryOp) and expr.op == '=' and len(elements) > 1:
            expr.right = TupleLiteral(elements[1:])
            return expr

        # `yield a, b` yields the tuple `(a, b)`, so the comma-separated
        # elements become the yield's operand rather than forming a tuple
        # around the yield itself.
        if isinstance(expr, UnaryOp) and expr.op == 'yield':
            tail = [expr.operand] if expr.operand is not None else []
            expr.operand = TupleLiteral(tail + elements[1:])
            return expr
        return TupleLiteral(elements)

    def parse_block(self):
        self.consume(expected_value='{')
        statements = []
        while not self.check('}') and not self.check('EOF'):
            stmt = self.parse_statement()
            if stmt:
                statements.append(stmt)
        self.consume(expected_value='}')
        return statements

    # --- Declarations ---
    def parse_var_decl(self, visibility='public', is_static=False, is_volatile=False):
        self.consume(expected_value='let')
        mutable = False
        if self.match('mut'):
            mutable = True

        # Reject the documented-but-wrong `let private x` form: visibility
        # modifiers belong *before* `let` (e.g. `private let x`).
        if self.peek().value in ('public', 'private', 'protected', 'static', 'volatile'):
            tok = self.peek()
            raise self.error(
                "visibility/modifier must come before 'let', not after it "
                f"(write '{tok.value} let ...')", tok)

        # Destructuring check (if starts with { or [ or ()
        name = ""
        if self.check('{') or self.check('[') or self.check('('):
            sequence_pattern = not self.check('{')
            # Capture tokens until we hit ':' or '='
            # Basic balanced consumption
            # This is a heuristic to get the pattern string for Python
            # A full implementation would parse a Pattern node.

            stack = []
            start_pos = self.pos
            while True:
                tok = self.peek()
                if tok.value in '([{':
                    stack.append(tok.value)
                elif tok.value in ')]}' and stack: stack.pop()

                if (tok.value == ':' or tok.value == '=') and not stack:
                    break

                # Check EOF
                if tok.type == 'EOF': break

                # Consume
                self.consume()

            # Reconstruct string from tokens range
            # This is tricky because we don't have source slice easily from tokens logic above
            # But we can reconstruct from tokens values
            pat_tokens = self.tokens[start_pos:self.pos]

            # A type annotation inside a bracketed destructuring target
            # (`let (a, b: Int)`) has no Python equivalent and would emit
            # invalid syntax. Reject it with a diagnostic instead of
            # miscompiling. Dict patterns use `:` as the alias separator, so
            # only sequence patterns are checked.
            if sequence_pattern and any(t.value == ':' for t in pat_tokens):
                bad = next(t for t in pat_tokens if t.value == ':')
                raise self.error(
                    "type annotations are not allowed inside a destructuring "
                    "pattern; declare the type after the binding "
                    "(e.g. 'let (a, b): (int, int) = ...')", bad)

            # Simple spacing reconstruction
            parts = []
            for _i, t in enumerate(pat_tokens):
                parts.append(str(t.value))
                # Add heuristic grouping?
                # Python is picky about spaces? No.
                # (x,y) is fine.

            # If we just join everything, we might get (x,y).
            # But tokens are: '(', 'x', ',', 'y', ')'
            # join -> "(x, y)"
            # Let's try to just join with spaces, assuming Python handles spaces.
             # Actually Tokenizer stripped spaces.
             # But ( x , y ) is valid python.
            name = " ".join(t.value for t in pat_tokens)

            # Specific replacement for spread operator in list destructuring
            # Aura: *rest or ...rest. Python: *rest.
            name = name.replace("... ", "*")
            name = name.replace("* ", "*")

        else:
             name = self._check_binding_name(self.consume(expected_type='IDENT'))
             # Multiple assignment: `let a, b = 1, 2` becomes a tuple target.
             # Collect the remaining names and store them as a tuple pattern so
             # the transformer can emit Python tuple unpacking.
             extra_names = []
             while self.match(','):
                 extra_names.append(
                     self._check_binding_name(self.consume(expected_type='IDENT')))
             if extra_names:
                 name = "(" + ", ".join([name] + extra_names) + ")"
        type_annotation = None

        if self.match(':'):
            type_annotation = self.parse_type()

        value = None
        if self.match('='):
            value = self.parse_expression()
            # Tuple assignment: `let a, b = 1, 2`.
            value = self.parse_trailing_tuple(value)

        if self.check(';'):
            self.consume()

        return VarDecl(name, mutable, type_annotation, value, visibility, is_static, is_volatile)

    def parse_const_decl(self):
        self.consume(expected_value='const')
        name = self._check_binding_name(self.consume(expected_type='IDENT'))
        type_annotation = None
        if self.match(':'):
            type_annotation = self.parse_type()

        self.consume(expected_value='=')
        value = self.parse_expression()
        if self.check(';'): self.consume()
        return ConstDecl(name, type_annotation, value)

    def _parse_type_params(self, owner_name):
        """Parse an optional ``[T, U: Constraint]`` type-parameter list.

        Returns ``(names, constraints)`` where ``constraints`` maps a parameter
        name to the constraint text (a type name such as ``Comparable``). An
        empty list yields ``([], {})``. Type parameters use brackets only:
        ``def foo[T](...)``; ``<T>`` is rejected with a pointed message.
        """
        names = []
        constraints = {}
        if self.match('['):
            while True:
                pname = self.consume(expected_type='IDENT').value
                names.append(pname)
                if self.match(':'):
                    constraints[pname] = self.parse_type()
                if not self.match(','):
                    break
            self.consume(expected_value=']')
        elif self.check('<'):
            tok = self.peek()
            raise self.error(
                "type parameters use brackets, not '<...>' "
                f"(write '[T]' on {owner_name!r})", tok)
        return names, constraints

    def parse_function_decl(self, decorators=None, visibility='public', is_static=False, is_volatile=False, name_override=None, is_abstract=False):
        # Support 'def' (async def also supported). 'fn' is not part of Aura.
        is_async = False
        if self.check('async'):
            self.consume()
            is_async = True

        if self.check('fn'):
            tok = self.peek()
            raise self.error(
                "'fn' is not part of Aura; use 'def' instead", tok)
        self.consume(expected_value='def')

        name = self.consume(expected_type='IDENT').value
        if name_override is not None:
            name = name_override

        # Type parameters use brackets only: `def foo[T](...)`.
        type_params, type_constraints = self._parse_type_params(name)

        self.consume(expected_value='(')
        params = []
        if not self.check(')'):
            while True:
                # Handle *, *args and **kwargs
                is_variadic = False
                is_kwonly = False
                if self.check('**'):
                    self.consume()
                    is_variadic = True
                    is_kwonly = True
                elif self.check('*'):
                    self.consume()
                    # Bare `*` marks the following parameters keyword-only.
                    if self.check(',') or self.check(')'):
                        params.append(Parameter('*', None, None, is_variadic=False, is_kwonly=False))
                        if not self.match(','):
                            break
                        continue
                    is_variadic = True

                p_name = self.consume(expected_type='IDENT').value
                p_type = None
                if self.match(':'):
                    p_type = self.parse_type()

                default = None
                if self.match('='):
                    default = self.parse_expression()

                params.append(Parameter(p_name, p_type, default, is_variadic=is_variadic, is_kwonly=is_kwonly))
                if not self.match(','):
                    break
        self.consume(expected_value=')')

        return_type = None
        if self.match('->'):
            return_type = self.parse_type()

        if is_abstract:
            # An `abstract def` is a pure signature: it has no body. A `{`
            # or `=` here would give it an implementation, which contradicts
            # the declaration, so reject it with a pointed message.
            if self.check('{'):
                raise self.error(
                    f"abstract method '{name}' cannot have a body; "
                    f"remove 'abstract' or provide the implementation in a "
                    f"subclass",
                    self.peek())
            if self.check('='):
                raise self.error(
                    f"abstract method '{name}' cannot have an expression body; "
                    f"remove 'abstract' or provide the implementation in a "
                    f"subclass",
                    self.peek())
            if self.check(';'):
                self.consume()
            return FunctionDecl(name, params, return_type, None,
                                is_async=is_async, type_params=type_params,
                                type_constraints=type_constraints,
                                decorators=decorators, visibility=visibility,
                                is_static=is_static, is_volatile=is_volatile,
                                is_abstract=True)

        # Handle expression body: fn foo() = expr
        if self.match('='):
             expr = self.parse_expression()
             # Wrap in return stmt
             body = [ReturnStmt(expr)]
        else:
            body = self.parse_block()

        return FunctionDecl(name, params, return_type, body, is_async=is_async, type_params=type_params, type_constraints=type_constraints, decorators=decorators, visibility=visibility, is_static=is_static, is_volatile=is_volatile, is_abstract=is_abstract)

    def _parse_class_header_fields(self, class_name):
        """Parse `(private name: str, age: int = 0)` after a class name.

        Each entry declares an instance field. A leading visibility modifier
        (`private`/`protected`/`public`) or `mut` is optional; the default is
        ``private`` with no modifier, matching Aura's encapsulation-first
        design. `mut` (or `let mut`) declares a mutable field and thus yields a
        setter. Fields default to immutable, so they yield only a getter.

        Returns a list of ``(Parameter, visibility, mutable)`` triples; the
        parameter carries the name, type and optional default so downstream
        code reuses the normal parameter machinery.
        """
        self.consume(expected_value='(')
        fields = []
        if self.check(')'):
            self.consume()
            return fields
        seen = set()
        while True:
            visibility = 'private'
            mutable = False
            # Visibility / mutability modifiers, in any order before the name.
            while True:
                if self.match('public'):
                    visibility = 'public'
                elif self.match('private'):
                    visibility = 'private'
                elif self.match('protected'):
                    visibility = 'protected'
                elif self.match('mut'):
                    mutable = True
                elif self.match('let'):
                    if self.match('mut'):
                        mutable = True
                else:
                    break
            if self.match('const'):
                tok = self.peek()
                raise self.error(
                    f"constant fields are declared in the class body, not the "
                    f"header of '{class_name}'", tok)
            field_name = self.consume(expected_type='IDENT').value
            if field_name in seen:
                raise self.error(
                    f"duplicate header field '{field_name}' in '{class_name}'")
            seen.add(field_name)
            type_annotation = None
            if self.match(':'):
                type_annotation = self.parse_type()
            default = None
            if self.match('='):
                default = self.parse_expression()
            if type_annotation is None and default is None:
                # A bare name in the header would be indistinguishable from the
                # removed parenthesised base form; require a type or default.
                # If it is a lone uppercase name, the writer almost certainly
                # meant a base class, so point them at `extends`.
                if field_name[:1].isupper() and self.check(')'):
                    raise self.error(
                        f"'{class_name}({field_name})' looks like a base class; "
                        f"Aura spells inheritance 'extends' "
                        f"(write 'class {class_name} extends {field_name}')")
                raise self.error(
                    f"header field '{field_name}' needs a type annotation or a "
                    f"default (write '{field_name}: Type')")
            # A required field may not follow an optional one, mirroring the
            # rule for function parameters (otherwise the ctor is ambiguous).
            if default is None and any(f[0].default is not None for f in fields):
                raise self.error(
                    f"header field '{field_name}' without a default cannot "
                    f"follow a field with a default")
            fields.append((Parameter(field_name, type_annotation, default),
                           visibility, mutable))
            if not self.match(','):
                break
        self.consume(expected_value=')')
        return fields

    def parse_class_decl(self, decorators=None, visibility='public', is_abstract=False):
        self.consume(expected_value='class')
        name = self.consume(expected_type='IDENT').value

        # Generics: Strict [T] only (Zen of Aura)
        type_params, type_constraints = self._parse_type_params(name)

        base_class = None
        # Inheritance uses `extends` only: `class A extends B`,
        # `class A extends B, C` (multiple) or a dotted base `pkg.Base`.
        # Traits are extended the same way. There is no parenthesised base
        # form; `(` after the name or bases always introduces header fields.
        if self.check('implements'):
            tok = self.peek()
            raise self.error(
                "'implements' is not Aura; extend with 'extends' "
                f"(write 'class {name} extends TraitName')", tok)
        if self.match('extends'):
            bases = []
            while True:
                base = self.consume(expected_type='IDENT').value
                while self.check('.') and self.peek(1).type == 'IDENT':
                    self.consume()
                    base += "." + self.consume(expected_type='IDENT').value
                bases.append(base)
                if not self.match(','):
                    break
            base_class = ", ".join(bases)

        # Class header fields: `class User(private name: str, age: int = 0)`
        # or `class Admin extends User(email: str)`. Each entry becomes an
        # instance field (private by default) and drives the generated
        # constructor and accessors. Only the class's own fields are declared
        # here; inherited fields arrive through `extends`.
        header_fields = []
        if self.check('('):
            header_fields = self._parse_class_header_fields(name)

        self.consume(expected_value='{')
        members = []
        while not self.check('}') and not self.check('EOF'):
            member_start = self.peek()
            visibility = None
            is_static = False
            is_volatile = False

            # Parse modifiers and decorators in any order. `public`,
            # `private`, `protected`, `static`, `volatile` and `@decorator`
            # all describe the member that follows, so both
            # `@staticmethod public def f` and `public @staticmethod def f`
            # (and the same on one line or across lines) are accepted.
            member_decorators = []
            is_classmethod = False
            is_property = False
            member_abstract = False
            while True:
                if self.match('public'): visibility = 'public'
                elif self.match('private'): visibility = 'private'
                elif self.match('protected'): visibility = 'protected'
                elif self.match('static'): is_static = True
                elif self.match('volatile'): is_volatile = True
                elif self.match('abstract'): member_abstract = True
                elif self.check('override'):
                    # Overriding is implicit: declaring a method with the same
                    # name in a subclass replaces the inherited one. Accepting
                    # `override` silently would turn it into a field named
                    # `override`, so point the writer at the Aura rule.
                    raise self.error(
                        "'override' is not Aura: overriding is implicit "
                        "(declare 'public def name(...)' with the same name)",
                        self.peek())
                elif self.match('@'):
                    dec_name = self._parse_decorator_name()
                    dec_args, dec_kwargs = self._parse_decorator_arguments()
                    member_decorators.append(Decorator(dec_name, dec_args, dec_kwargs))
                    if dec_name == 'staticmethod': is_static = True
                    elif dec_name == 'classmethod': is_classmethod = True
                    elif dec_name == 'property': is_property = True
                else:
                    break

            if member_abstract and not (
                    self.check('def')
                    or self.check('class')
                    or (self.check('async') and self.peek(1).value == 'def')):
                raise self.error(
                    "'abstract' applies to a method or a nested class; write "
                    "'abstract def name(...)' or 'abstract class Name'",
                    self.peek())

            # A bare identifier followed directly by a member keyword is a
            # modifier Aura does not have (`final def`, `open class`,
            # `implements def`, ...). Without this guard it would parse as a
            # field with that name and the real member would follow, silently
            # accepting the foreign spelling. `async def` is the one real
            # spelling in this shape.
            if (self.peek().type == 'IDENT'
                    and self.peek().value != 'async'
                    and self.peek(1).value in ('def', 'class', 'fn', 'async')):
                foreign = self.peek().value
                raise self.error(
                    f"'{foreign}' is not an Aura modifier; remove it and "
                    f"declare the member directly",
                    self.peek())

            if self.check('def') or self.check('fn') or (
                    self.check('async') and self.peek(1).value == 'def'):
                # Aura method names map to Python dunder names (new -> __init__,
                # str -> __str__, len -> __len__, ...). Names already written as
                # dunders are preserved verbatim. The table is shared with the
                # transformers via ``SPECIAL_METHOD_NAMES``.
                if self.check('fn'):
                    tok = self.peek()
                    raise self.error(
                        "'fn' is not part of Aura; use 'def' instead", tok)
                # `async def` is two tokens, so the method name sits one token
                # further on (`async` at +0, `def` at +1, name at +2).
                name_offset = 2 if self.check('async') else 1
                method_name = self.peek(name_offset).value
                if method_name == 'init':
                    tok = self.peek(name_offset)
                    raise self.error(
                        "'init' is not the Aura constructor; use 'new' "
                        "instead", tok)
                override = self._SPECIAL_METHOD_NAMES.get(method_name)
                func = self.parse_function_decl(
                    decorators=member_decorators,
                    name_override=override,
                    is_abstract=member_abstract,
                )
                method = Method(func.name, func.params, func.return_type, func.body,
                                is_static=is_static, is_classmethod=is_classmethod,
                                is_property=is_property, visibility=visibility,
                                is_volatile=is_volatile, decorators=member_decorators,
                                owner=name, is_async=func.is_async,
                                is_abstract=member_abstract)
                method.with_location(self._member_location(member_start))
                members.append(method)
            elif self.check('class'):
                # Nested class: `class Inner { ... }`.
                inner = self.parse_class_decl(
                    decorators=member_decorators,
                    visibility=visibility,
                    is_abstract=member_abstract,
                )
                inner.with_location(self._member_location(member_start))
                members.append(inner)
            else:
                 # Fields: x: Int = 1
                if self.check('}') or self.check('EOF'): break
                if member_decorators:
                    # A decorator was parsed but the member is a field; a
                    # decorator would silently do nothing, so reject it.
                    names = ', '.join('@' + d.name for d in member_decorators)
                    raise self.error(
                        f"{names} cannot be applied to a field; decorators "
                        f"belong on a 'def'", self.peek())
                # `let`, `let mut`, `const` and bare `mut` all declare a member.
                # `const` is a class-level constant: it must be initialised,
                # lives on the class and is never an instance field. A plain
                # `let` field is immutable, exactly like `let` at module and
                # local scope; `let mut` / `mut` opts into a setter and runtime
                # reassignment.
                is_const = False
                field_mutable = False
                if self.peek().value == 'const':
                    self.consume()
                    is_const = True
                    field_mutable = False
                elif self.peek().value == 'let':
                    self.consume()
                    if self.match('mut'):
                        field_mutable = True
                elif self.match('mut'):
                    field_mutable = True
                if self.peek().type == 'IDENT':
                    field_name = self.consume().value
                    t = None
                    if self.match(':'):
                        t = self.parse_type()
                    v = None
                    if self.match('='):
                        v = self.parse_expression()

                    if self.check(';'): self.consume()
                    if is_const:
                        if v is None:
                            raise self.error(
                                f"constant '{field_name}' requires a value")
                        member = ConstDecl(
                            field_name, t, v, visibility=visibility,
                            is_static=is_static, is_volatile=is_volatile,
                            owner=name)
                    else:
                        member = VarDecl(field_name, field_mutable, t, v,
                                         visibility=visibility, is_static=is_static,
                                         is_volatile=is_volatile, owner=name)
                    member.with_location(self._member_location(member_start))
                    members.append(member)
                else:
                    raise self.error(f"Unexpected token in class: {self.peek().value}")

        self.consume(expected_value='}')
        if header_fields:
            # A class that declares its fields in the header already gets a
            # generated constructor from those fields. A manual `new` would
            # silently shadow it, leaving the header fields declared but never
            # assigned: `class C(x: int) { def new(...) {} }` produces a
            # `get_x` accessor that reads a value nobody set. Reject the mix so
            # each class has exactly one constructor style.
            manual_new = next(
                (m for m in members
                 if isinstance(m, Method) and m.name in ('__init__', 'new')),
                None)
            if manual_new is not None:
                raise self.error(
                    f"class '{name}' declares header fields and a manual "
                    f"'new'; the header already generates a constructor, so "
                    f"declare the fields in the body (or drop 'new')",
                    getattr(manual_new, 'location', None) or self.peek())
        return ClassDecl(name, members, base_class, type_params, decorators, visibility, type_constraints=type_constraints, header_fields=header_fields, is_abstract=is_abstract)

    def _member_location(self, tok):
        """Build a SourceLocation for a class member's first token."""
        return SourceLocation(
            filename=self.filename,
            line=getattr(tok, 'line', 0),
            column=getattr(tok, 'column', 0),
        )

    def parse_module_decl(self):
        self.consume(expected_value='module')
        name = self.consume(expected_type='IDENT').value
        # Dotted module names (`module stdlib.collections`) nest the emitted
        # namespaces as `stdlib.collections`.
        while self.match('.'):
            name += "." + self.consume(expected_type='IDENT').value

        # Parse the module body as a list of member declarations. Members are
        # private to the file unless prefixed with `export`.
        self.consume(expected_value='{')
        members = []
        exports = set()
        reexports = []
        while not self.check('}') and not self.check('EOF'):
            is_exported = self.match('export')
            if not is_exported:
                member = self.parse_statement()
                if member is None:
                    continue
                member.is_exported = False
                members.append(member)
                continue

            # `export` is either a modifier on a declaration, or a bare
            # re-export of a name declared elsewhere.
            if self.check('}') or self.check('EOF') or self.check(';'):
                raise self.error(
                    "`export` must be followed by a declaration or a name "
                    "(def, class, trait, enum, type, let, const, module, or "
                    "`export Name`)")
            if self._starts_exported_declaration():
                member = self.parse_statement()
                if member is None:
                    continue
                member_name = getattr(member, 'name', None)
                if member_name is None:
                    raise self.error(
                        "`export` must precede a named declaration "
                        "(def, class, trait, enum, type, let, const or module)")
                exports.add(member_name)
                member.is_exported = True
                members.append(member)
                continue

            # Otherwise: re-export one or more names.
            item = self._parse_item_export()
            reexports.append(item)
            for requested in item.names:
                exports.add(requested)

        self.consume(expected_value='}')
        return Module(name, members, exports, reexports)

    # Keywords that begin a declaration and so prove `export` is a modifier
    # rather than the start of a bare re-export.
    _DECLARATION_KEYWORDS = (
        'def', 'class', 'trait', 'enum', 'type', 'let', 'const', 'module',
        'async', 'public', 'private', 'protected', 'static', 'volatile', '@',
    )

    def _starts_exported_declaration(self):
        """True when the next tokens begin a declaration (so `export` is a
        modifier). False for a bare name, which is a re-export."""
        return self.peek().value in self._DECLARATION_KEYWORDS

    def _parse_item_export(self):
        """Parse the body of a bare `export Name[, Name2] [from "module"]`."""
        names = [self.consume(expected_type='IDENT').value]
        while self.match(','):
            names.append(self.consume(expected_type='IDENT').value)
        source = None
        if self.match('from'):
            token = self.peek()
            if token.type != 'STRING':
                raise self.error(
                    "`export ... from` needs a quoted module path "
                    '(e.g. export Name from "components")', token)
            source = self.consume().value
            if not source or _INVALID_MODULE_PATH.search(source):
                raise self.error(
                    f"invalid module path {source!r} in `export ... from`")
        if self.check(';'):
            self.consume()
        return ItemExport(names, source)

    def parse_type_decl(self):
        self.consume(expected_value='type')
        name = self.consume(expected_type='IDENT').value

        # Generics: type Result[T, E] only
        type_params, _type_constraints = self._parse_type_params(name)

        self.consume(expected_value='=')

        # Capture the type expression from the token stream. Scanning is
        # structural: at depth 0 we stop as soon as the next token cannot
        # continue a type (so `type X = int\nprint(X)` does not swallow the
        # following statement), while nested brackets/braces are tracked.
        type_tokens = self._scan_type_tokens()
        type_text = self._tokens_to_type_text(type_tokens)

        if self.check(';'):
            self.consume()
        return TypeDecl(name, type_text, type_params)

    # Operators that can continue a type expression after a complete type.
    _TYPE_CONTINUATIONS = ('[', '<', '|', '?', '.', '->', ',')

    def _scan_type_tokens(self):
        """Consume tokens that form a single type expression and return them."""
        start = self.pos
        depth = 0
        consumed_any = False
        while True:
            tok = self.peek()
            if tok.type == 'EOF':
                break
            if tok.value in ('(', '[', '{'):
                depth += 1
                self.consume()
                consumed_any = True
                continue
            if tok.value in (')', ']', '}'):
                if depth == 0:
                    break
                depth -= 1
                self.consume()
                continue
            if depth > 0:
                # Inside brackets everything continues the type.
                self.consume()
                consumed_any = True
                continue
            # At depth 0: literals and identifiers start/continue a type.
            if tok.type in ('IDENT', 'INT', 'FLOAT', 'STRING') or tok.value in ('true', 'false', 'none', '.'):
                if not consumed_any:
                    self.consume()
                    consumed_any = True
                    continue
                # A bare identifier immediately after a complete type begins a
                # new statement, unless it is a continuation operator.
                break
            if tok.value in self._TYPE_CONTINUATIONS:
                self.consume()
                consumed_any = True
                continue
            break
        return self.tokens[start:self.pos]

    def _tokens_to_type_text(self, tokens):
        """Reconstruct a readable type expression from tokens."""
        parts = []
        prev = None
        for tok in tokens:
            value = str(tok.value)
            # `{` starts a structural type whose keys are bare identifiers.
            if value in ('(', '[', '{'):
                if value == '{':
                    parts.append('{')
                else:
                    parts.append(value)
            elif value in (')', ']', '}'):
                parts.append(value)
            elif value == ',':
                parts.append(', ')
            elif value == ':':
                parts.append(': ')
            elif value == '|':
                parts.append(' | ')
            elif value == '.':
                parts.append('.')
            elif value == '?':
                parts.append('?')
            else:
                if prev is not None and prev not in ('(', '[', '{', '<', '.', ' ', ','):
                    parts.append(' ')
                parts.append(value)
            prev = value
        return "".join(parts).strip()

    # --- Type Parsing ---
    _TYPE_START_VALUES = {'(', '[', '{'}
    _TYPE_START_TYPES = {'IDENT'}

    def parse_type(self):
        token = self.peek()
        if token.type not in self._TYPE_START_TYPES and token.value not in self._TYPE_START_VALUES:
            raise self.error(
                f"Expected a type but got '{token.value}'", token)
        token = self.consume()
        t_name = str(token.value)

        # Function type: (T) -> R
        if t_name == '(':
             # Parse arg types
             args = []
             if not self.check(')'):
                 while True:
                     args.append(self.parse_type())
                     if not self.match(','): break
             self.consume(expected_value=')')
             self.consume(expected_value='->')
             ret = self.parse_type()
             return f"({', '.join(str(a) for a in args)}) -> {ret}"

        if t_name == '[':
             # Array/List type [T] or [T, U]
             arg = self.parse_type()
             while self.match(','):
                 arg += ", " + self.parse_type()
             self.consume(expected_value=']')
             return f"List[{arg}]" # Map to List type for Python

        if t_name == '{':
             # Structural type: {name: str, age: int}
             fields = []
             while not self.check('}'):
                 # Optional visibility/modifier keywords inside a field list.
                 if self.peek().value in ('public', 'private', 'protected'):
                     self.consume()
                 field_name = self.consume(expected_type='IDENT').value
                 if self.match('?'):
                     field_name += "?"
                 self.consume(expected_value=':')
                 field_type = self.parse_type()
                 fields.append(f"{field_name}: {field_type}")
                 if not self.match(','):
                     break
             self.consume(expected_value='}')
             return "{" + ", ".join(fields) + "}"

        while True:
            # Generics use brackets only: `List[Int]`, `Result[T, E]`. The
            # angle form `<T>` is not Aura and is rejected with a pointed error,
            # matching `_parse_type_params`.
            if self.match('['):
                arg = self.parse_type()
                while self.match(','):
                     arg += ", " + self.parse_type()
                self.consume(expected_value=']')
                t_name += f"[{arg}]"
            elif self.check('<'):
                tok = self.peek()
                raise self.error(
                    "type arguments use brackets, not '<...>' "
                    f"(write '{t_name}[T]')", tok)
            # Optional: String?
            elif self.match('?'):
                t_name += "?"
            # Union: Int | Str
            elif self.match('|'):
                rhs = self.parse_type()
                t_name += f" | {rhs}"
            else:
                break
        return t_name

    def parse_import_stmt(self):
        self.consume(expected_value='import')
        module = self.consume(expected_type='IDENT').value
        while self.check('.') and self.peek(1).type == 'IDENT':
            self.consume()  # consume '.'
            module += "." + self.consume(expected_type='IDENT').value

        items = []
        if self.match('{'):
            while True:
                name = self.consume(expected_type='IDENT').value
                item_alias = None
                if self.match('as'):
                    item_alias = self.consume(expected_type='IDENT').value
                items.append((name, item_alias))
                if not self.match(','): break
            self.consume(expected_value='}')

        alias = None
        if self.match('as'):
            alias = self.consume(expected_type='IDENT').value

        # Multiple imports: `import a, b as c, d.e`.
        if not items and self.check(','):
            modules = [(module, alias)]
            while self.match(','):
                next_module = self.consume(expected_type='IDENT').value
                while self.check('.') and self.peek(1).type == 'IDENT':
                    self.consume()
                    next_module += "." + self.consume(expected_type='IDENT').value
                next_alias = None
                if self.match('as'):
                    next_alias = self.consume(expected_type='IDENT').value
                modules.append((next_module, next_alias))
            if self.check(';'): self.consume()
            return ImportStmt(module, items, alias, modules)

        if self.check(';'): self.consume()
        return ImportStmt(module, items, alias)

    def parse_from_import_stmt(self):
        self.consume(expected_value='from')
        module = self.consume(expected_type='IDENT').value
        while self.check('.') and self.peek(1).type == 'IDENT':
            self.consume()
            module += "." + self.consume(expected_type='IDENT').value
        self.consume(expected_value='import')

        items = []
        if self.match('*'):
            items.append(('*', None))
        else:
            while True:
                name = self.consume(expected_type='IDENT').value
                alias = None
                if self.match('as'):
                    alias = self.consume(expected_type='IDENT').value
                items.append((name, alias))
                if not self.match(','):
                    break
        if self.check(';'): self.consume()
        return FromImport(module, items)

    def parse_trait_decl(self, visibility='public'):
        self.consume(expected_value='trait')
        name = self.consume(expected_type='IDENT').value

        # Traits may declare generic parameters: `trait Mapper[T] { ... }`.
        type_params, type_constraints = self._parse_type_params(name)

        # Traits extend other traits with `extends` only, mirroring classes:
        # `trait Loud extends Greeter` or `trait Loud extends A, B`.
        bases = []
        if self.check('('):
            tok = self.peek()
            raise self.error(
                "parenthesised bases are not Aura; write "
                f"'trait {name} extends Base'", tok)
        if self.check('implements'):
            tok = self.peek()
            raise self.error(
                "'implements' is not Aura; extend the trait with 'extends' "
                f"(write 'trait {name} extends TraitName')", tok)
        if self.match('extends'):
            while True:
                base = self.consume(expected_type='IDENT').value
                while self.check('.') and self.peek(1).type == 'IDENT':
                    self.consume()
                    base += "." + self.consume(expected_type='IDENT').value
                bases.append(base)
                if not self.match(','):
                    break
        base_class = ", ".join(bases) if bases else None

        self.consume(expected_value='{')
        members = []
        while not self.check('}') and not self.check('EOF'):
            member_start = self.peek()
            # Member modifiers and decorators, in any order (see the class
            # member loop): `@staticmethod public def f` and
            # `public @staticmethod def f` are equivalent.
            member_visibility = None
            is_static = False
            is_volatile = False
            member_decorators = []
            is_classmethod = False
            is_property = False
            while True:
                if self.match('public'): member_visibility = 'public'
                elif self.match('private'): member_visibility = 'private'
                elif self.match('protected'): member_visibility = 'protected'
                elif self.match('static'): is_static = True
                elif self.match('volatile'): is_volatile = True
                elif self.check('abstract'):
                    # Every trait method without a body is already abstract, so
                    # `abstract` is redundant here. Rejecting it stops the
                    # keyword from being silently consumed as a field name.
                    raise self.error(
                        "'abstract' is not used in a trait: a method with no "
                        "body is already abstract (write 'def f() -> T')",
                        self.peek())
                elif self.check('override'):
                    raise self.error(
                        "'override' is not Aura: overriding is implicit "
                        "(declare 'public def name(...)' with the same name)",
                        self.peek())
                elif self.match('@'):
                    dec_name = self._parse_decorator_name()
                    dec_args, dec_kwargs = self._parse_decorator_arguments()
                    member_decorators.append(Decorator(dec_name, dec_args, dec_kwargs))
                    if dec_name == 'staticmethod': is_static = True
                    elif dec_name == 'classmethod': is_classmethod = True
                    elif dec_name == 'property': is_property = True
                else:
                    break

            # A bare identifier followed directly by a member keyword is a
            # modifier Aura does not have (see the class member loop).
            if (self.peek().type == 'IDENT'
                    and self.peek().value != 'async'
                    and self.peek(1).value in ('def', 'class', 'fn', 'async')):
                foreign = self.peek().value
                raise self.error(
                    f"'{foreign}' is not an Aura modifier; remove it and "
                    f"declare the member directly",
                    self.peek())

            if self.check('def') or self.check('fn') or (
                    self.check('async') and self.peek(1).value == 'def'):
                # Reuse the full function parser so parameters, defaults,
                # types, generics and default bodies are all preserved.
                is_async = False
                if self.check('async'):
                    self.consume()
                    is_async = True
                self.consume()  # eat def/fn
                func = self.parse_function_decl_after_keyword()
                func.is_async = is_async
                method = Method(
                    func.name, func.params, func.return_type, func.body,
                    is_static=is_static, is_classmethod=is_classmethod,
                    is_property=is_property, visibility=member_visibility,
                    is_volatile=is_volatile,
                    decorators=member_decorators, owner=name,
                    is_async=func.is_async,
                )
                method.with_location(self._member_location(member_start))
                members.append(method)
            elif self.peek().type == 'IDENT':
                # Field declaration inside a trait: `name: Type`,
                # `let name: Type`, `let name: Type = default`, `mut name`,
                # or `const NAME = value`. A plain `let` is immutable; `mut`
                # opts in, matching class fields and ordinary bindings.
                if member_decorators:
                    names = ', '.join('@' + d.name for d in member_decorators)
                    raise self.error(
                        f"{names} cannot be applied to a field; decorators "
                        f"belong on a 'def'", self.peek())
                is_const = False
                field_mutable = False
                if self.peek().value == 'const':
                    self.consume()
                    is_const = True
                elif self.peek().value == 'let':
                    self.consume()
                    if self.check('mut'):
                        self.consume()
                        field_mutable = True
                elif self.peek().value == 'mut':
                    self.consume()
                    field_mutable = True
                member_name = self.consume().value
                field_type = None
                if self.match(':'):
                    field_type = self.parse_type()
                default = None
                if self.match('='):
                    default = self.parse_expression()
                if is_const:
                    if default is None:
                        raise self.error(
                            f"constant '{member_name}' requires a value")
                    field = ConstDecl(
                        member_name, field_type, default,
                        visibility=member_visibility, is_static=is_static,
                        is_volatile=is_volatile, owner=name)
                else:
                    field = VarDecl(member_name, field_mutable, field_type, default,
                                    visibility=member_visibility, is_static=is_static,
                                    is_volatile=is_volatile, owner=name)
                field.with_location(self._member_location(member_start))
                members.append(field)
            else:
                break

        self.consume(expected_value='}')
        return TraitDecl(name, members, type_params, visibility, base_class,
                         type_constraints=type_constraints)

    def parse_function_decl_after_keyword(self):
        """Parse a function signature/body when `def`/`fn` was already consumed.

        Used by trait declarations so they share the exact function grammar.
        """
        name = self.consume(expected_type='IDENT').value

        type_params, type_constraints = self._parse_type_params(name)

        self.consume(expected_value='(')
        params = []
        if not self.check(')'):
            while True:
                is_variadic = False
                is_kwonly = False
                if self.check('**'):
                    self.consume()
                    is_variadic = True
                    is_kwonly = True
                elif self.check('*'):
                    self.consume()
                    if self.check(',') or self.check(')'):
                        params.append(Parameter('*', None, None))
                        if not self.match(','):
                            break
                        continue
                    is_variadic = True

                p_name = self.consume(expected_type='IDENT').value
                p_type = None
                if self.match(':'):
                    p_type = self.parse_type()
                default = None
                if self.match('='):
                    default = self.parse_expression()
                params.append(Parameter(p_name, p_type, default,
                                        is_variadic=is_variadic, is_kwonly=is_kwonly))
                if not self.match(','):
                    break
        self.consume(expected_value=')')

        return_type = None
        if self.match('->'):
            return_type = self.parse_type()

        body = None
        if self.match('='):
            body = [ReturnStmt(self.parse_expression())]
        elif self.check('{'):
            body = self.parse_block()

        return FunctionDecl(name, params, return_type, body,
                            type_params=type_params,
                            type_constraints=type_constraints)

    def parse_enum_decl(self, visibility='public'):
        """Parse `enum Name { A, B = 3, C }` into an EnumDecl."""
        self.consume(expected_value='enum')
        name = self.consume(expected_type='IDENT').value
        self.consume(expected_value='{')
        members = []
        while not self.check('}') and not self.check('EOF'):
            member_name = self.consume(expected_type='IDENT').value
            value = None
            if self.match('='):
                value = self.parse_expression()
            members.append((member_name, value))
            if not self.match(','):
                break
        self.consume(expected_value='}')
        if self.check(';'):
            self.consume()
        return EnumDecl(name, members, visibility)

    # --- Control Flow ---
    def parse_condition(self):
        """Parse a control-flow condition.

        Inside a condition, a ``{`` always opens the statement body, so the
        struct-literal heuristic must be suppressed. Without this,
        ``if Point { ... }`` (or any uppercase identifier condition) would be
        parsed as a struct initialization and swallow the block.
        """
        self._no_struct_depth += 1
        try:
            return self.parse_expression()
        finally:
            self._no_struct_depth -= 1

    def parse_if_stmt(self):
        self.consume(expected_value='if')
        cond = self.parse_condition()
        then_body = self.parse_block()
        else_body = None
        if self.match('else'):
            else_body = [self.parse_if_stmt()] if self.check('if') else self.parse_block()
        return IfStmt(cond, then_body, else_body)

    def parse_guard_stmt(self):
        self.consume(expected_value='guard')
        cond = self.parse_condition()
        self.consume(expected_value='else')
        else_body = self.parse_block()
        return GuardStmt(cond, else_body)

    def parse_throw_stmt(self):
        self.consume(expected_value='throw')
        val = self.parse_expression()
        if self.check(';'): self.consume()
        return ThrowStmt(val)


    def parse_while_stmt(self):
        self.consume(expected_value='while')
        cond = self.parse_condition()
        body = self.parse_block()
        return WhileStmt(cond, body)

    def parse_unless_stmt(self):
        self.consume(expected_value='unless')
        cond = self.parse_condition()
        body = self.parse_block()
        else_body = None
        if self.match('else'):
            else_body = [self.parse_if_stmt()] if self.check('if') else self.parse_block()
        return UnlessStmt(cond, body, else_body)

    def parse_until_stmt(self):
        self.consume(expected_value='until')
        cond = self.parse_condition()
        body = self.parse_block()
        return UntilStmt(cond, body)

    def parse_loop_stmt(self):
        self.consume(expected_value='loop')
        body = self.parse_block()
        return LoopStmt(body)



    def parse_for_stmt(self):
        self.consume(expected_value='for')
        # Check for parenthesis (optional in Aura but good to handle)
        has_paren = self.match('(')

        # Supports: for x in ... OR for (i, x) in ...
        targets = []
        if self.check_type('IDENT'):
             targets.append(self.consume().value)

        while self.match(','):
             targets.append(self.consume(expected_type='IDENT').value)

        if len(targets) == 1:
             pattern = IdentifierPattern(targets[0])
        else:
             pattern = ListPattern([IdentifierPattern(t) for t in targets])

        if has_paren:
             self.consume(expected_value=')')

        self.consume(expected_value='in')
        self._reject_bare_spread("a 'for' iterable")
        iterable = self.parse_condition()

        step = None
        if self.match('step'):
            step = self.parse_condition()

        body = self.parse_block()
        return ForStmt(pattern, iterable, body, step)

    def _reject_bare_spread(self, what):
        """Raise a clear error when a value position starts with `*`/`**`.

        `*args`/`**kwargs` are only meaningful in call arguments (and a `*`
        element inside a list/set/tuple/dict literal). Anywhere else a leading
        `*` would otherwise parse as a unary multiply and emit invalid Python
        such as `return (* a)`.
        """
        tok = self.peek()
        if tok.type == 'OP' and tok.value in ('*', '**'):
            raise self.error(
                f"'{tok.value}' spread is not allowed in {what}; "
                f"it is only valid in call arguments and list/set/tuple/dict "
                f"literals", tok)

    def parse_return_stmt(self):
        self.consume(expected_value='return')
        val = None
        if not self.check(';') and not self.check('}'):
            self._reject_bare_spread("a return value")
            val = self.parse_expression()
            val = self.parse_trailing_tuple(val)
        if self.check(';'): self.consume()
        return ReturnStmt(val)


    def parse_try_stmt(self):
        self.consume(expected_value='try')
        try_body = self.parse_block()
        catch_clauses = []
        while self.match('catch'):
            exc_type = None
            var_name = None
            # One spelling per concept:
            #   `catch { }`            -> catch everything
            #   `catch Type { }`       -> catch a type
            #   `catch Type as e { }`  -> catch a type and bind it
            #   `catch as e { }`       -> bind everything (bare `catch e` is
            #                             rejected: a lone identifier is a type)
            if self.match('as'):
                var_name = self.consume(expected_type='IDENT').value
            elif self.check_type('IDENT') and self.peek().value not in ('{', 'as'):
                exc_type = self.consume().value
                if self.check_type('IDENT') and self.peek().value not in ('{', 'as'):
                    # `catch Value Error` is not a valid form.
                    raise self.error(
                        "invalid catch clause; write 'catch Type as name'",
                        self.peek())
                if self.match('as'):
                    var_name = self.consume(expected_type='IDENT').value

            body = self.parse_block()
            catch_clauses.append(CatchClause(exc_type, var_name, body))

        finally_body = None
        if self.match('finally'):
            finally_body = self.parse_block()

        if not catch_clauses and finally_body is None:
            raise self.error(
                "try requires at least one catch clause or a finally block")

        return TryStmt(try_body, catch_clauses, finally_body)

    def parse_with_stmt(self, is_async=False):
        self.consume(expected_value='with')
        items = []
        while True:
            expr = self.parse_expression(11) # Precedence > 10 to avoid consuming 'as'
            var_name = None
            if self.match('as'):
                var_name = self.consume(expected_type='IDENT').value
            items.append((expr, var_name))

            if not self.match(','):
                break

        body = self.parse_block()
        return WithStmt(items, body, is_async=is_async)

    def parse_match_stmt(self):
        self.consume(expected_value='match')
        expr = self.parse_expression()
        self.consume(expected_value='{')
        cases = []
        while not self.check('}') and not self.check('EOF'):
             # case pattern { ... } OR pattern -> stmt
             self.match('case')
             pat_node = None

             # Parse the pattern. A bare identifier is a binding pattern or an
             # enum member, never a call: the `{` that follows starts the case
             # body. Parsing it as a full expression would swallow the body as
             # a block argument, so identifiers and member paths are handled
             # explicitly before falling back to expression parsing.
             pat_node = self._parse_match_pattern()

             guard = None
             if self.match('if'):
                 guard = self.parse_expression()

             body = []
             if self.match('{'):
                 while not self.check('}'):
                     stmt = self.parse_statement()
                     if stmt: body.append(stmt)
                 self.consume(expected_value='}')
             elif self.match('->'):
                 stmt = self.parse_statement()
                 if stmt: body = [stmt]
             else:
                 # Maybe implicit block or just expr?
                 pass

             cases.append(MatchCase(pat_node, guard, body))

        self.consume(expected_value='}')
        return MatchStmt(expr, cases)

    def _parse_match_pattern(self):
        """Parse a single `case` pattern into a pattern node.

        Handled forms: wildcard, literal, bare identifier / enum member,
        dotted member path (`Color.RED`), tuple/list destructuring with an
        optional `*rest`, constructor patterns (`Some(x)`), or-patterns
        (`1 | 2`), and `_ as name` bindings.
        """
        self._pattern_depth += 1
        try:
            pattern_expr = self.parse_expression()
        finally:
            self._pattern_depth -= 1
        return self._pattern_from_expr(pattern_expr)

    def _pattern_from_expr(self, pattern_expr):
        if isinstance(pattern_expr, Identifier) and pattern_expr.name == '_':
            return WildcardPattern()
        if isinstance(pattern_expr, (IntLiteral, StrLiteral, BoolLiteral, NoneLiteral)):
            return LiteralPattern(pattern_expr)
        if isinstance(pattern_expr, Identifier):
            return IdentifierPattern(pattern_expr.name)
        if isinstance(pattern_expr, MemberExpr):
            # `Color.RED` is an enum-member pattern; keep the member path so
            # the match can refer to the real constant.
            return MemberPattern(pattern_expr)
        if isinstance(pattern_expr, (TupleLiteral, ListLiteral)):
            patterns = []
            for elem in (pattern_expr.elements if isinstance(pattern_expr.elements, list) else []):
                if isinstance(elem, Identifier):
                    patterns.append(IdentifierPattern(elem.name))
                elif isinstance(elem, (IntLiteral, StrLiteral, BoolLiteral, NoneLiteral)):
                    patterns.append(LiteralPattern(elem))
                elif isinstance(elem, UnaryOp) and elem.op == '*':
                    if isinstance(elem.operand, Identifier):
                        rest_pattern = IdentifierPattern(elem.operand.name)
                        return ListPattern(patterns, rest_pattern=rest_pattern)
                    patterns.append(LiteralPattern(elem))
                else:
                    patterns.append(self._pattern_from_expr(elem))
            return ListPattern(patterns)
        if isinstance(pattern_expr, CallExpr):
            pat_name = pattern_expr.func.name if isinstance(pattern_expr.func, Identifier) else "unknown"
            subpatterns = []
            for arg in pattern_expr.args:
                if isinstance(arg, Identifier):
                    subpatterns.append(IdentifierPattern(arg.name))
                elif isinstance(arg, (IntLiteral, StrLiteral, BoolLiteral)):
                    subpatterns.append(LiteralPattern(arg))
                else:
                    subpatterns.append(self._pattern_from_expr(arg))
            return ConstructorPattern(pat_name, subpatterns)
        if isinstance(pattern_expr, BinaryOp) and pattern_expr.op == '|':
            alternatives = [self._pattern_from_expr(pattern_expr.left),
                            self._pattern_from_expr(pattern_expr.right)]
            parts = []
            for alt in alternatives:
                if isinstance(alt, OrPattern):
                    parts.extend(alt.patterns)
                else:
                    parts.append(alt)
            return OrPattern(parts)
        return LiteralPattern(pattern_expr)

    def parse_assert_stmt(self):
        self.consume(expected_value='assert')
        condition = self.parse_expression()
        message = None
        if self.match(','):
            message = self.parse_expression()

        # Check for optional semicolon
        if self.check(';'):
            self.consume()

        return AssertStmt(condition, message)

    def parse_case_stmt(self):
        self.consume(expected_value='case')
        pattern = self.parse_expression()

        guard = None
        if self.match('if'):
            guard = self.parse_expression()

        body = self.parse_block()
        return MatchCase(pattern, guard, body)

    # --- Expressions (Pratt Parser) ---
    def parse_expression(self, min_prec=0):
        # Prefix operators
        token = self.peek()
        if token.type == 'IDENT' and token.value == 'yield':
            self.consume()
            # `yield` may carry a value or stand alone. Do not let it swallow
            # the following statement, so stop at a block/statement boundary.
            if (self.check('}') or self.check(';') or self.check_type('EOF')
                    or (self.peek().type == 'IDENT'
                        and self.peek().value in _STATEMENT_KEYWORDS)):
                lhs = UnaryOp('yield', operand=None)
            else:
                # Python's `yield` is the loosest operator: `yield x + 1`
                # means `yield (x + 1)`, not `(yield x) + 1`. Parse the
                # operand just above assignment/pipe so the whole expression
                # is captured.
                lhs = UnaryOp('yield', operand=self.parse_expression(2))
        elif token.type == 'OP' and token.value == '!':
            raise self.error(
                "'!' is not part of Aura; use 'not' instead", token)
        elif token.type == 'IDENT' and token.value == 'not':
            self.consume()
            # `not` binds looser than comparisons (so `not a in b` is
            # `not (a in b)`) but tighter than `and`/`or`, matching Python.
            rhs = self.parse_expression(6)
            lhs = UnaryOp('not', operand=rhs)
        elif ((token.type == 'OP' and token.value in ('-', '+', '~'))
                or (token.type == 'IDENT' and token.value == 'await')):
            op = token.value
            self.consume()
            # Arithmetic unary binds looser than `**` (so `-2 ** 2` is
            # `-(2 ** 2)`) but tighter than `*`/`/` (so `-2 * 3` is `(-2) * 3`).
            rhs = self.parse_expression(11)
            lhs = UnaryOp(op, operand=rhs)
        elif token.type == 'OP' and token.value in ('*', '**', '...'):
            # `*`/`**`/`...` are spread markers, not prefix operators. Every
            # context that permits them consumes the marker before calling
            # `parse_expression` (call arguments, list/set/tuple/dict literals,
            # lambda parameter lists). Reaching here means the spread sits in a
            # plain value position, where it would emit invalid Python such as
            # `(* a)`; reject it with a positioned diagnostic instead.
            raise self.error(
                f"'{token.value}' spread is not allowed in an expression; "
                f"it is only valid in call arguments and list/set/tuple/dict "
                f"literals", token)
        else:
            lhs = self.parse_primary()

        while True:
            pk = self.peek()
            op = None
            if pk.type == 'OP':
                op = pk.value
            elif pk.type == 'IDENT':
                if pk.value in ('in', 'is', 'and', 'or', 'as'):
                    op = pk.value
                elif pk.value == 'not':
                    # `not in`: only when directly followed by `in`.
                    if self.pos + 1 < len(self.tokens) and self.tokens[self.pos + 1].value == 'in':
                        op = 'not in'

                # Check for 'is not'
                if op == 'is':
                    if self.pos + 1 < len(self.tokens) and self.tokens[self.pos + 1].value == 'not':
                        op = 'is not'

            if not op:
                # `...` is a prefix spread only; seeing it between operands
                # (e.g. `1...10`) is always a mistake, not a valid operator.
                if pk.type == 'OP' and pk.value == '...':
                    raise self.error(
                        "unexpected '...' in expression; use '..' or '..<' "
                        "for ranges", pk)
                break

            if op in ('&&', '||'):
                canonical = 'and' if op == '&&' else 'or'
                tok = self.peek()
                raise self.error(
                    f"'{op}' is not part of Aura; use '{canonical}' instead",
                    tok)

            prec = self.get_precedence(op)

            if prec < min_prec or prec == 0:
                break

            # Special case for pipe |>
            if op == '|>':
                self.consume()
                rhs = self.parse_expression(prec + 1)
                lhs = PipeExpr(lhs, rhs)
                continue

            # Special case for range ..
            if op == '..' or op == '..<':
                self.consume()
                rhs = self.parse_expression(prec + 1)

                step = None
                if self.match('step'):
                    step = self.parse_expression(prec + 1)

                exclusive = (op == '..<')
                lhs = RangeExpr(lhs, rhs, exclusive=exclusive, step=step)
                continue

            # Special case for conditional ternary ? :
            if op == '?':
                self.consume() # eat ?
                true_expr = self.parse_expression(0)
                self.consume(expected_value=':')
                false_expr = self.parse_expression(min_prec)
                lhs = CondExpr(lhs, true_expr, false_expr) # lhs is the condition
                continue

            # Special case for null coalescing ??
            if op == '??':
                self.consume()
                rhs = self.parse_expression(prec) # right associative? usually left
                lhs = CoalesceExpr(lhs, rhs)
                continue

            # Special case for 'as' cast
            if op == 'as':
                self.consume()
                t_name = self.parse_type()
                rhs = Identifier(t_name)
                lhs = BinaryOp(lhs, 'as', rhs)
                continue

            self.consume() # consume first part of op (or whole op)
            if op == 'not in' or op == 'is not':
                self.consume() # consume 2nd part ('in' or 'not')

            rhs = self.parse_expression(prec + 1 if self.is_left_assoc(op) else prec)
            lhs = BinaryOp(lhs, op, rhs)

        return lhs

    def get_precedence(self, op):
        # Higher number = higher precedence
        return _PRECEDENCE.get(op, 0)

    def is_left_assoc(self, op):
        return op != '**' and op != '=' and op != '??'

    def parse_primary(self):
        token = self.peek()
        # print(f"DEBUG: parse_primary peek={token}")

        if token.type == 'INT':
            self.consume()
            return self.parse_postfix(IntLiteral(token.value))
        elif token.type == 'FLOAT':
            self.consume()
            return self.parse_postfix(FloatLiteral(token.value))
        elif token.type == 'STRING':
            self.consume()
            return self.parse_postfix(StrLiteral(token.value))
        elif token.type == 'RAWSTRING':
            self.consume()
            return self.parse_postfix(StrLiteral(token.value, raw_literal=token.value))
        elif token.type == 'FSTRING':
            self.consume()
            quote, raw = token.value
            node = FStringLiteral(self._parse_fstring_parts(raw), quote=quote)
            return self.parse_postfix(node)

        elif token.type == 'IDENT':
            if token.value == 'true':
                self.consume()
                return BoolLiteral(True)
            elif token.value == 'false':
                self.consume()
                return BoolLiteral(False)
            elif token.value == 'null':
                raise self.error(
                    "'null' is not part of Aura; use 'none' instead", token)
            elif token.value == 'none':
                self.consume()
                return NoneLiteral()
            # Lambda is handled via '(' ... '=>' or check special syntax if needed

            elif token.value == 'if':
                return self.parse_if_stmt()
            elif token.value == 'match':
                 stmt = self.parse_match_stmt()
                 return MatchExpr(stmt.expr, stmt.cases)
            elif token.value == 'try':
                 stmt = self.parse_try_stmt()
                 return TryExpr(stmt.try_body, stmt.catch_clauses, stmt.finally_body)

            self.consume()
            node = Identifier(token.value)
            # Single-parameter lambda without parens: `x => expr`
            if self.match('=>'):
                body = self.parse_lambda_body()
                return LambdaExpr([Parameter(token.value)], body)
            return self.parse_postfix(node)

        elif token.value == '[':
            return self.parse_postfix(self.parse_list_literal())
        elif token.value == '{':
            return self.parse_postfix(self.parse_dict_or_block())
        elif token.value == '(':
            self.consume() # match (
            # Empty tuple () or () =>
            if self.check(')'):
                self.consume() # Consume the closing ')'
                if self.match('=>'):
                    # 0-arg lambda
                    body = self.parse_lambda_body()
                    return LambdaExpr([], body)
                return self.parse_postfix(TupleLiteral([]))

            # Lambda parameter list: `(...) => ...`. Detected by lookahead so
            # the parameter grammar (defaults, `*args`, `**kwargs`, bare `*`)
            # is parsed directly instead of going through tuple-expression
            # parsing, which cannot represent those forms.
            if self._is_lambda_params_ahead():
                params = self._parse_lambda_params()
                self.consume(expected_value=')')
                self.consume(expected_value='=>')
                body = self.parse_lambda_body()
                return LambdaExpr(params, body)

            if self.match('**'):
                expr = SpreadExpr(self.parse_expression(), is_dict=True)
            elif self.match('*'):
                if self.check(',') or self.check(')'):
                    raise self.error(
                        "a bare '*' is only allowed in a lambda parameter list")
                expr = SpreadExpr(self.parse_expression(), is_dict=False)
            elif self.match('...'):
                expr = SpreadExpr(self.parse_expression(), is_dict=False)
            else:
                expr = self.parse_expression(0)

            # Generator expression: `(expr for pattern in iterable ...)`.
            if self.check('for'):
                self.consume(expected_value='for')
                comprehensions = []
                while True:
                    pattern = self.parse_expression(7)
                    if self.match(','):
                        pats = [pattern]
                        while True:
                            pats.append(self.parse_expression(7))
                            if not self.match(','): break
                        pattern = TupleLiteral(pats)
                    self.consume(expected_value='in')
                    iterable = self.parse_expression()
                    filters = []
                    while self.match('if'):
                        filters.append(self.parse_expression())
                    comprehensions.append((pattern, iterable, filters))
                    if not self.match('for'):
                        break
                self.consume(expected_value=')')
                return ComprehensionExpr(expr, comprehensions, expr_type='generator')

            # Check for Tuple: (expr, expr)
            if self.match(','):
                elements = [expr]
                while True:
                    if self.check(')'): break
                    if self.match('**'):
                        elements.append(SpreadExpr(self.parse_expression(), is_dict=True))
                    elif self.match('*'):
                        if self.check(',') or self.check(')'):
                            raise self.error(
                                "a bare '*' is only allowed in a lambda "
                                "parameter list")
                        elements.append(SpreadExpr(self.parse_expression(), is_dict=False))
                    elif self.match('...'):
                        elements.append(SpreadExpr(self.parse_expression(), is_dict=False))
                    else:
                        elements.append(self.parse_expression())
                    if not self.match(','): break
                self.consume(expected_value=')')

                if self.check('=>'):
                    raise self.error("Invalid parameter in lambda")
                return self.parse_postfix(TupleLiteral(elements))

            self.consume(expected_value=')')
            if isinstance(expr, SpreadExpr):
                # A parenthesised spread without a trailing comma is not a
                # tuple (`(*a,)` is); a bare `(*a)` has no valid Python form.
                raise self.error(
                    "'*'/'**' spread inside parentheses needs a trailing "
                    "comma to form a tuple, or belongs in a call argument list",
                    token)
            return self.parse_postfix(expr)

        raise self.error(f"Unexpected token {token}", token)

    def _parse_fstring_parts(self, raw):
        """Split an f-string's raw body into text and interpolated expressions.

        ``{{`` and ``}}`` are literal braces; ``{ ... }`` interpolates an Aura
        expression (parsed recursively so method aliases are transformed later).
        Format specifiers after ``:`` are preserved verbatim.
        """
        parts = []
        text = []
        i = 0
        n = len(raw)
        while i < n:
            ch = raw[i]
            if ch == '{':
                if i + 1 < n and raw[i + 1] == '{':
                    text.append('{')
                    i += 2
                    continue
                # Collect the interpolation, respecting nested braces.
                if text:
                    parts.append(''.join(text))
                    text = []
                depth = 1
                j = i + 1
                while j < n and depth > 0:
                    c = raw[j]
                    if c == '{':
                        depth += 1
                    elif c == '}':
                        depth -= 1
                        if depth == 0:
                            break
                    j += 1
                inner = raw[i + 1:j].strip()
                expr_src, conversion, fmt_spec = self._split_fstring_field(inner)
                expr_src = expr_src.strip()
                if expr_src:
                    try:
                        sub_tokens = Tokenizer(expr_src).tokenize()
                        expr = Parser(sub_tokens).parse_expression()
                    except Exception as exc:
                        raise self.error(
                            f"invalid expression in f-string: {expr_src!r} "
                            f"({exc})") from None
                    spec = conversion + (":" + fmt_spec if fmt_spec is not None else "")
                    parts.append((expr, spec))
                i = j + 1
                continue
            if ch == '}':
                if i + 1 < n and raw[i + 1] == '}':
                    text.append('}')
                    i += 2
                    continue
                text.append('}')
                i += 1
                continue
            text.append(ch)
            i += 1
        if text:
            parts.append(''.join(text))
        return parts

    def _split_fstring_field(self, inner):
        """Split an f-string field into (expression, conversion, format_spec).

        The conversion is the optional ``!r``/``!s``/``!a`` suffix and the
        format spec the optional ``:...`` tail. Both are located at bracket
        depth zero so a `:` inside a slice or dict is not mistaken for the
        format separator.
        """
        depth = 0
        i = 0
        n = len(inner)
        expr_end = n
        conversion = ''
        fmt_spec = None
        while i < n:
            ch = inner[i]
            if ch in '([{':
                depth += 1
            elif ch in ')]}':
                depth -= 1
            elif depth == 0 and ch == ':' and fmt_spec is None:
                expr_end = i
                fmt_spec = inner[i + 1:]
                break
            elif depth == 0 and ch == '!':
                expr_end = i
                i += 1
                if i < n and inner[i] in 'rsa':
                    conversion = '!' + inner[i]
                    i += 1
                if i < n and inner[i] == ':':
                    fmt_spec = inner[i + 1:]
                break
            i += 1
        return inner[:expr_end], conversion, fmt_spec

    def _is_lambda_params_ahead(self):
        """Return True when the current `(` ... `)` is a lambda parameter list.

        Scans past balanced brackets/parens without consuming tokens; the
        parameter list is a lambda's when the matching `)` is followed by `=>`.
        A `=>` nested deeper (e.g. a default that is itself a lambda) is not
        the current lambda's marker.
        """
        depth = 1
        i = self.pos
        n = self._ntokens
        while i < n:
            tok = self.tokens[i]
            if tok.value in ('(', '[', '{'):
                depth += 1
            elif tok.value in (')', ']', '}'):
                depth -= 1
                if depth == 0:
                    return (i + 1 < n and self.tokens[i + 1].value == '=>')
            i += 1
        return False

    def _parse_lambda_params(self):
        """Parse a lambda parameter list using the `def` param grammar.

        The opening `(` has already been consumed; the caller consumes the
        closing `)`. Mirrors `_parse_function`'s parameter loop so lambdas and
        functions accept the same forms (see GRAMMAR.md 6.4).
        """
        params = []
        seen_star = False
        seen_kw = False
        while True:
            is_variadic = False
            is_kwonly = False
            if self.check('**'):
                if seen_kw:
                    raise self.error(
                        "a lambda may declare only one '**' parameter")
                self.consume()
                is_variadic = True
                is_kwonly = True
                seen_kw = True
            elif self.check('*'):
                self.consume()
                if self.check(',') or self.check(')'):
                    if seen_star:
                        raise self.error(
                            "a lambda may declare only one '*' parameter")
                    seen_star = True
                    params.append(Parameter('*'))
                    if not self.match(','):
                        break
                    continue
                if seen_star:
                    raise self.error(
                        "a lambda may declare only one '*' parameter")
                seen_star = True
                is_variadic = True

            name_tok = self.consume(expected_type='IDENT')
            p_name = name_tok.value
            p_type = None
            if self.match(':'):
                p_type = self.parse_type()
            default = None
            if self.match('='):
                default = self.parse_expression()
            params.append(Parameter(
                p_name, p_type, default,
                is_variadic=is_variadic, is_kwonly=is_kwonly))
            if not self.match(','):
                break
        return params

    def parse_lambda_body(self):
        """Parse the body of a lambda: either `{ stmts }` block or an expression."""
        if self.check('{'):
            # A block lambda always returns its last expression.
            body = self.parse_expr_block()
            return body
        return self.parse_expression()

    def parse_expr_block(self):
        """Parse a `{ ... }` block in expression position, returning a BlockExpr."""
        self.consume(expected_value='{')
        statements = []
        while not self.check('}') and not self.check('EOF'):
            stmt = self.parse_statement()
            if stmt:
                statements.append(stmt)
        self.consume(expected_value='}')
        return BlockExpr(statements)

    def parse_subscript(self):
        """Parse the contents of `[...]`: an index or a Python-style slice.

        `a[start:stop:step]` becomes a ``SliceExpr``; any component may be
        omitted. Without a top-level `:` it is a plain index expression.
        """
        start = None
        if not self.check(':'):
            start = self.parse_expression()
        if not self.check(':'):
            return start

        self.consume(expected_value=':')
        stop = None
        if not self.check(']') and not self.check(':'):
            stop = self.parse_expression()
        step = None
        if self.match(':') and not self.check(']'):
            step = self.parse_expression()
        return SliceExpr(None, start, stop, step)

    def parse_postfix(self, node):
        while True:
            # print(f"DEBUG: parse_postfix peek={self.peek()}")
            if self.match('('):
                args = []
                kwargs = {}
                if not self.check(')'):
                    while True:
                        # A trailing comma before `)` is allowed, so each
                        # branch below stops when the next token closes the
                        # argument list.
                        if self.check(')'):
                            break
                        # Spread arguments: `f(*items)`, `f(**mapping)`, `f(...value)`.
                        # `...value` is *adaptive*: a dict spreads as keyword
                        # arguments, anything else as positional arguments.
                        if self.check('**') or self.check('...'):
                            marker = self.consume().value
                            expr = self.parse_expression()
                            if marker == '**' or isinstance(expr, DictLiteral):
                                is_dict = True
                            elif isinstance(expr, (ListLiteral, TupleLiteral)):
                                is_dict = False
                            else:
                                is_dict = None  # decide at runtime
                            args.append(SpreadExpr(expr, is_dict=is_dict))
                            if not self.match(','):
                                break
                            continue
                        if self.check('*'):
                            self.consume()
                            args.append(SpreadExpr(self.parse_expression(), is_dict=False))
                            if not self.match(','):
                                break
                            continue
                        # Check for named arg: IDENT :|= ...
                        is_named = False
                        if self.check_type('IDENT'):
                            # Need lookahead. pos is IDENT. next?
                            next_token_idx = self.pos + 1
                            if next_token_idx < len(self.tokens):
                                next_val = self.tokens[next_token_idx].value
                                if next_val in (':', '='):
                                    is_named = True

                        if is_named:
                            key = self.consume().value # Consume IDENT
                            self.consume() # Consume : or =
                            val = self.parse_expression()
                            kwargs[key] = val
                        else:
                            if kwargs:
                                raise self.error(
                                    "Positional argument follows keyword argument")
                            first_arg = self.parse_expression()
                            # Generator expression as a sole call argument:
                            # `sum(x for x in items)`.
                            if self.check('for') and not args:
                                self.consume(expected_value='for')
                                comprehensions = []
                                while True:
                                    pattern = self.parse_expression(7)
                                    if self.match(','):
                                        pats = [pattern]
                                        while True:
                                            pats.append(self.parse_expression(7))
                                            if not self.match(','): break
                                        pattern = TupleLiteral(pats)
                                    self.consume(expected_value='in')
                                    iterable = self.parse_expression()
                                    filters = []
                                    while self.match('if'):
                                        filters.append(self.parse_expression())
                                    comprehensions.append((pattern, iterable, filters))
                                    if not self.match('for'):
                                        break
                                args.append(ComprehensionExpr(first_arg, comprehensions, expr_type='generator'))
                                break
                            args.append(first_arg)

                        if not self.match(','):
                            break
                self.consume(expected_value=')')
                node = CallExpr(node, args, kwargs)
            elif self.peek().value == '{':
                # Struct initialization: User { x: 1 }
                # Ambiguity with control flow: if x { ... } vs if x {} ...
                # Heuristic: Only allow if node is Capitalized Identifier (or member access)
                is_struct = False
                if not self._pattern_depth and not self._no_struct_depth:
                    if isinstance(node, Identifier) and node.name[0].isupper():
                        is_struct = True
                    elif isinstance(node, BinaryOp) and node.op == '.':
                         # Check rhs
                         if isinstance(node.rhs, Identifier) and node.rhs.name[0].isupper():
                             is_struct = True

                if is_struct:
                    self.consume()
                    d = self.parse_dict_body() # parse content and closing '}'
                    # Create CallExpr with SpreadExpr(d, is_dict=True)
                    node = CallExpr(node, [SpreadExpr(d, is_dict=True)])
                else:
                    break # Not struct init, treat as end of expression (block starts)
            elif self.match('['):
                index = self.parse_subscript()
                self.consume(expected_value=']')
                if isinstance(index, SliceExpr):
                    index.obj = node
                    node = index
                else:
                    node = IndexExpr(node, index)
            elif self.match('.'):
                member = self.consume(expected_type='IDENT').value
                node = MemberExpr(node, member)
            elif self.match('?.'): # Safe nav
                member = self.consume(expected_type='IDENT').value
                node = SafeNavExpr(node, member, is_index=False)
            elif self.match('?['): # Safe index
                index = self.parse_subscript()
                self.consume(expected_value=']')
                if isinstance(index, SliceExpr):
                    index.obj = node
                    node = index
                else:
                    node = SafeNavExpr(node, index, is_index=True)
            else:
                break
        return node

    def parse_list_literal(self):
        self.consume(expected_value='[')
        if self.match(']'):
            return ListLiteral([])

        if self.match('*') or self.match('...'):
            first = SpreadExpr(self.parse_expression(), is_dict=False)
        else:
            first = self.parse_expression()

        # Check for comprehension: [x for x in list]
        if self.match('for'):
             comprehensions = []
             # First for was already matched
             while True:
                 # Support full pattern (e.g. k, v or (k, v))
                 pattern = self.parse_expression(7)
                 if self.match(','):
                     pats = [pattern]
                     while True:
                         pats.append(self.parse_expression(7))
                         if not self.match(','): break
                     pattern = TupleLiteral(pats)

                 self.consume(expected_value='in')
                 iterable = self.parse_expression()
                 filters = []
                 while self.match('if'):
                     filters.append(self.parse_expression())

                 comprehensions.append((pattern, iterable, filters))

                 if not self.match('for'):
                     break

             self.consume(expected_value=']')
             return ComprehensionExpr(first, comprehensions, expr_type='list')

        elements = [first]
        while self.match(','):
            if self.check(']'): break
            if self.match('*') or self.match('...'):
                elements.append(SpreadExpr(self.parse_expression(), is_dict=False))
            else:
                elements.append(self.parse_expression())

        self.consume(expected_value=']')
        return ListLiteral(elements)

    def parse_dict_or_block(self):
         self.consume(expected_value='{')
         return self.parse_dict_body()

    def parse_dict_body(self):
        if self.match('}'):
            return DictLiteral([])

        # Heuristic: Check for statement keywords -> Block. Only a bare keyword
        # token counts: a string literal such as `"if"` is a dict key/value,
        # not a statement.
        tok = self.peek()
        stmt_keywords = ('let', 'const', 'return', 'while', 'for', 'if', 'try', 'match', 'guard', 'unless')
        if tok.type == 'IDENT' and tok.value in stmt_keywords:
            return self.parse_block_expr_internal(first_stmt=None)

        # Heuristic: `{ ident = ... }` where `=` is assignment (not a dict
        # key `:` or a comparison) is a block, not a dict/set literal.
        if tok.type == 'IDENT' and self.peek(1).value == '=' and self.peek(1).type != '==':
            return self.parse_block_expr_internal(first_stmt=None)

        # Heuristic: a literal that *starts* with a spread (`{*a}`, `{**a}`,
# `{...a}`) is unambiguous — a block never begins with `*`. This must be
# checked before the generic scan below, which only looks for `:`/`,`.
        if tok.type == 'OP' and tok.value in ('*', '**', '...'):
            return self.parse_set_or_dict_body()

        # A top-level `:` or `,` (or a `**` spread) uniquely identifies a
        # dict/set literal. Without one, the braces contain statements, so a
        # bare block such as `{ print(x); let y = 1 }` is handled correctly.
        depth = 1
        i = self.pos
        is_literal = False
        while i < len(self.tokens):
            t = self.tokens[i]
            v = t.value
            op = t.type == 'OP'
            if op and v in ('{', '(', '['):
                depth += 1
            elif op and v in ('}', ')', ']'):
                depth -= 1
                if depth == 0:
                    break
            elif depth == 1 and (
                    (op and v in (':', ',', '**'))
                    or (t.type == 'IDENT' and v == 'for')
            ):
                is_literal = True
                break
            i += 1
        if not is_literal:
            return self.parse_block_expr_internal(first_stmt=None)

        return self.parse_set_or_dict_body()

    def parse_set_or_dict_body(self):
        """Parse the elements of a set/dict literal, up to and including `}`.

        The opening `{` has already been consumed. Handles both spreads and
        comprehensions.
        """
        elements = []
        is_dict = False

        while not self.check('}'):
            if self.match('**') or self.match('...'):
                expr = self.parse_expression()
                elements.append(SpreadExpr(expr, is_dict=True))
                is_dict = True
            elif self.match('*'):
                expr = self.parse_expression()
                elements.append(SpreadExpr(expr, is_dict=False))
            else:
                expr = self.parse_expression()
                if self.match(':'):
                    val = self.parse_expression()
                    if self.match('for'):
                        return self.parse_dict_comp_body(expr, val)
                    # Bare identifiers used as dict keys become string keys:
                    # `{name: "Alice"}` -> `{"name": "Alice"}`.
                    if isinstance(expr, Identifier):
                        expr = StrLiteral(expr.name)
                    elements.append((expr, val))
                    is_dict = True
                else:
                    if self.match('for'):
                        return self.parse_set_comp_body(expr)
                    elements.append(expr)
            if not self.match(','): break

        self.consume(expected_value='}')
        if is_dict: return DictLiteral(elements)
        else: return SetLiteral(elements)

    def parse_dict_comp_body(self, key, val):
        comprehensions = []
        while True:
            pattern = self.parse_expression(7)
            if self.match(','):
                pats = [pattern]
                while True:
                    pats.append(self.parse_expression(7))
                    if not self.match(','): break
                pattern = TupleLiteral(pats)
            self.consume(expected_value='in')
            iterable = self.parse_expression()
            filters = []
            while self.match('if'):
                filters.append(self.parse_expression())
            comprehensions.append((pattern, iterable, filters))
            if not self.match('for'): break
        self.consume(expected_value='}')
        return ComprehensionExpr((key, val), comprehensions, expr_type='dict')

    def parse_set_comp_body(self, first):
        comprehensions = []
        while True:
            pattern = self.parse_expression(7)
            if self.match(','):
                pats = [pattern]
                while True:
                    pats.append(self.parse_expression(7))
                    if not self.match(','): break
                pattern = TupleLiteral(pats)
            self.consume(expected_value='in')
            iterable = self.parse_expression()
            filters = []
            while self.match('if'):
                filters.append(self.parse_expression())
            comprehensions.append((pattern, iterable, filters))
            if not self.match('for'): break
        self.consume(expected_value='}')
        return ComprehensionExpr(first, comprehensions, expr_type='set')

    def parse_block_expr_internal(self, first_stmt=None):
        statements = []
        if first_stmt:
            statements.append(first_stmt)

        while not self.check('}') and not self.check('EOF'):
            stmt = self.parse_statement()
            if stmt: statements.append(stmt)

        self.consume(expected_value='}')
        return BlockExpr(statements)

def parse_file(path: str) -> Program:
    """Parse Aura file using recursive descent parser."""
    with open(path, encoding='utf-8') as f:
        source = f.read()
    if len(source) > MAX_SOURCE_BYTES:
        exc = SyntaxError(
            f"{path}: source is too large "
            f"({len(source)} bytes; limit {MAX_SOURCE_BYTES})"
        )
        raise annotate_syntax_error(exc, 1, 1, path)
    tokenizer = Tokenizer(source, filename=path)
    tokens = tokenizer.tokenize()
    parser = Parser(tokens, filename=path)
    return parser.parse()
