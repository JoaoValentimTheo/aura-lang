"""
Complete recursive descent parser for Aura.
Handles expressions, control flow, functions, classes, and more.
"""
import contextlib

from aura.transpiler.ast import *

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

class Tokenizer:
    def __init__(self, source, filename: str = "<aura>"):
        self.source = source
        self.pos = 0
        self.line = 1
        self.column = 1
        self.tokens = []
        self.filename = filename

    def error(self, message):
        """Build a ``SyntaxError`` carrying the tokenizer's position."""
        exc = SyntaxError(f"{message} (line {self.line})")
        exc.line = self.line
        exc.column = self.column
        exc.filename = self.filename
        return exc

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
                        out.append(chr(int(raw[i + 2:i + 6], 16)))
                        i += 6
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
                self.pos += 2
                while self.pos < length and not (
                    self.source[self.pos] == '*' and self.pos + 1 < length
                    and self.source[self.pos + 1] == '/'
                ):
                    if self.source[self.pos] == '\n':
                        self.line += 1
                        self.column = 1
                    self.pos += 1
                if self.pos < length:
                    self.pos += 2  # skip closing */
                continue

            # Identifiers and Keywords
            if char.isalpha() or char == '_':
                start = self.pos
                start_line, start_col = self.line, self.column
                while self.pos < length and (self.source[self.pos].isalnum() or self.source[self.pos] == '_'):
                    self.pos += 1
                value = self.source[start:self.pos]

                # F-string support (special case: identifier 'f' followed by quote).
                # Supports escapes, nested braces and triple quotes. The raw
                # inner text is stored; parsing into parts happens later so
                # interpolated Aura code can be transformed.
                if value == 'f' and self.pos < length and self.source[self.pos] in ('"', "'"):
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
                        if not value.lower().startswith('r') and self.source[j] == '\\':
                            j += 2
                            continue
                        if self.source[j] == '\n':
                            self.line += 1
                        if self.source[j:j + len(closing)] == closing:
                            break
                        j += 1
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
            if char.isdigit():
                start = self.pos
                start_line, start_col = self.line, self.column

                # Radix prefixes: 0x, 0o, 0b (with optional _ separators).
                if char == '0' and self.pos + 1 < length and self.source[self.pos + 1] in 'xXoObB':
                    prefix = self.source[self.pos + 1].lower()
                    self.pos += 2
                    digits = ''
                    while self.pos < length and (self.source[self.pos].isalnum() or self.source[self.pos] == '_'):
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
                while self.pos < length and (self.source[self.pos].isdigit() or self.source[self.pos] == '_'):
                    self.pos += 1

                # Check for float dot vs range (..)
                is_float = False
                if (self.pos < length and self.source[self.pos] == '.'
                        and self.pos + 1 < length and self.source[self.pos + 1].isdigit()):
                    is_float = True
                    self.pos += 1  # consume dot
                    while self.pos < length and (self.source[self.pos].isdigit() or self.source[self.pos] == '_'):
                        self.pos += 1

                # Scientific notation: 1e10, 1.5e-5
                if self.pos < length and self.source[self.pos] in 'eE':
                    nxt = self.source[self.pos + 1] if self.pos + 1 < length else ''
                    after = self.source[self.pos + 2] if self.pos + 2 < length else ''
                    if nxt.isdigit() or (nxt in '+-' and after.isdigit()):
                        is_float = True
                        self.pos += 1
                        if self.source[self.pos] in '+-':
                            self.pos += 1
                        while self.pos < length and self.source[self.pos].isdigit():
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
                    and self.source[self.pos + 1].isdigit()):
                start = self.pos
                self.pos += 1
                while self.pos < length and (self.source[self.pos].isdigit() or self.source[self.pos] == '_'):
                    self.pos += 1
                if self.pos < length and self.source[self.pos] in 'eE':
                    nxt = self.source[self.pos + 1] if self.pos + 1 < length else ''
                    after = self.source[self.pos + 2] if self.pos + 2 < length else ''
                    if nxt.isdigit() or (nxt in '+-' and after.isdigit()):
                        self.pos += 1
                        if self.source[self.pos] in '+-':
                            self.pos += 1
                        while self.pos < length and self.source[self.pos].isdigit():
                            self.pos += 1
                raw = self.source[start:self.pos].replace('_', '')
                self.column += (self.pos - start)
                self.tokens.append(Token('FLOAT', float(raw), self.line, self.column))
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
    'let', 'const', 'def', 'fn', 'async', 'class', 'trait', 'enum', 'type',
    'module', 'import', 'from', 'if', 'unless', 'until', 'while', 'for',
    'loop', 'guard', 'throw', 'return', 'break', 'continue', 'assert', 'try',
    'with', 'match', 'case', 'else', 'catch', 'finally', 'public', 'private',
    'protected', 'static', 'export',
})

# Upper bound on source size, guarding against accidental multi-gigabyte input.
MAX_SOURCE_BYTES = 16 * 1024 * 1024

# ==============================================================================
# Parser
# ==============================================================================

class Parser:
    # Aura method names that map to Python dunder names. The canonical table
    # lives in ``aura.transpiler.ast`` so the parser and the transformers agree.
    _SPECIAL_METHOD_NAMES = SPECIAL_METHOD_NAMES

    def __init__(self, tokens, filename: str = "<aura>"):
        self.tokens = tokens
        self.pos = 0
        self.filename = filename

    # --- Diagnostics ---
    def error(self, message, token=None):
        """Build a ``SyntaxError`` carrying structured line/column info.

        The CLI and the LSP read ``.line``/``.column``/``.filename`` from the
        exception, so diagnostics point at the real source position instead of
        defaulting to line 1. The message text keeps the ``... (line N)``
        suffix for tools that only show the string.
        """
        token = token if token is not None else self.peek()
        exc = SyntaxError(f"{message} (line {token.line})")
        exc.line = token.line
        exc.column = token.column
        exc.filename = self.filename
        # Keep the message suffix consistent for the no-location callers.
        return exc

    # --- Token Management ---
    def peek(self, offset=0):
        if self.pos + offset < len(self.tokens):
            return self.tokens[self.pos + offset]
        return self.tokens[-1]

    def consume(self, expected_type=None, expected_value=None):
        if self.pos >= len(self.tokens):
            raise self.error("Unexpected end of file",
                             self.tokens[-1] if self.tokens else Token('EOF', '', 1, 1))
        token = self.tokens[self.pos]
        # print(f"DEBUG: consume {token} pos={self.pos}")
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
        token = self.peek()
        if token.value == value:
            self.pos += 1
            return True
        return False

    def check(self, value):
        return self.peek().value == value

    def check_type(self, type_name):
        return self.peek().type == type_name

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
        return Program(statements)

    # --- Statements ---
    def parse_modifiers(self):
        """Parse visibility and other modifiers.

        Returns ``visibility=None`` when no visibility keyword was written, so
        the rule checker can require an explicit modifier on class members.
        """
        visibility = None
        is_static = False
        is_volatile = False

        while self.peek().value in ['public', 'private', 'protected', 'static', 'volatile']:
            val = self.consume().value
            if val in ['public', 'private', 'protected']:
                visibility = val
            elif val == 'static':
                is_static = True
            elif val == 'volatile':
                is_volatile = True

        return visibility, is_static, is_volatile

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


        while True:
            token = self.peek()
            if token.value == 'export':
                # `export` marks public members inside a module; Python has no
                # equivalent, so it is simply skipped.
                self.consume()
            elif token.value == '@':
                # Parse decorators
                while self.match('@'):
                    dec_name = self.consume(expected_type='IDENT').value
                    dec_args = []
                    dec_kwargs = {}
                    if self.match('('):
                        if not self.check(')'):
                            while True:
                                is_named = (self.check_type('IDENT')
                                            and self.peek(1).value in ('=', ':'))
                                if is_named:
                                    key = self.consume().value
                                    self.consume()
                                    dec_kwargs[key] = self.parse_expression()
                                else:
                                    dec_args.append(self.parse_expression())
                                if not self.match(','): break
                        self.consume(expected_value=')')
                    decorators.append(Decorator(dec_name, dec_args, dec_kwargs))
            elif token.value in ['public', 'private', 'protected', 'static', 'volatile']:
                v, s, vol = self.parse_modifiers()
                # Last visibility wins, flags accumulate. At module scope
                # visibility is metadata only, so an omitted modifier means
                # public (class members are handled separately).
                if v is not None: visibility = v
                if s: is_static = True
                if vol: is_volatile = True
            else:
                break

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
        elif token.value == 'const':
            return self.parse_const_decl()
        elif token.value == 'fn':
            raise self.error(
                "'fn' is not part of Aura; use 'def' instead", token)
        elif token.value in ('def', 'async'):
            return self.parse_function_decl(decorators, visibility, is_static, is_volatile)
        elif token.value == 'class':
            return self.parse_class_decl(decorators, visibility, is_static, is_volatile)
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
             name = self.consume(expected_type='IDENT').value
             # Multiple assignment: `let a, b = 1, 2` becomes a tuple target.
             # Collect the remaining names and store them as a tuple pattern so
             # the transformer can emit Python tuple unpacking.
             extra_names = []
             while self.match(','):
                 extra_names.append(self.consume(expected_type='IDENT').value)
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
        name = self.consume(expected_type='IDENT').value
        type_annotation = None
        if self.match(':'):
            type_annotation = self.parse_type()

        self.consume(expected_value='=')
        value = self.parse_expression()
        if self.check(';'): self.consume()
        return ConstDecl(name, type_annotation, value)

    def parse_function_decl(self, decorators=None, visibility='public', is_static=False, is_volatile=False, name_override=None):
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
        type_params = []
        if self.match('['):
            while True:
                type_params.append(self.consume(expected_type='IDENT').value)
                if not self.match(','):
                    break
            self.consume(expected_value=']')
        elif self.check('<'):
            tok = self.peek()
            raise self.error(
                "type parameters use brackets, not '<...>' "
                f"(write 'def {name}[T]')", tok)

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

        # Handle expression body: fn foo() = expr
        if self.match('='):
             expr = self.parse_expression()
             # Wrap in return stmt
             body = [ReturnStmt(expr)]
        else:
            body = self.parse_block()

        return FunctionDecl(name, params, return_type, body, is_async=is_async, type_params=type_params, decorators=decorators, visibility=visibility, is_static=is_static, is_volatile=is_volatile)

    def parse_class_decl(self, decorators=None, visibility='public', is_static=False, is_volatile=False):
        self.consume(expected_value='class')
        name = self.consume(expected_type='IDENT').value

        # Generics: Strict [T] only (Zen of Aura)
        type_params = []
        if self.match('['):
            while True:
                type_params.append(self.consume(expected_type='IDENT').value)
                if not self.match(','): break
            self.consume(expected_value=']')

        base_class = None
        # Inheritance: class Foo(Base) or multiple: class Foo(A, B).
        # Base names may be dotted (module.Class).
        if self.match('('):
            bases = []
            while True:
                base = self.consume(expected_type='IDENT').value
                while self.check('.') and self.peek(1).type == 'IDENT':
                    self.consume()
                    base += "." + self.consume(expected_type='IDENT').value
                bases.append(base)
                if not self.match(','):
                    break
            self.consume(expected_value=')')
            base_class = ", ".join(bases)

        # Trait implementation: class Foo implements TraitA, TraitB
        # Python has no traits, so implemented traits become mixins/base classes.
        if self.match('implements'):
            traits = [self.consume(expected_type='IDENT').value]
            while self.match(','):
                traits.append(self.consume(expected_type='IDENT').value)
            base_class = ", ".join([base_class] + traits) if base_class else ", ".join(traits)

        self.consume(expected_value='{')
        members = []
        while not self.check('}') and not self.check('EOF'):
            member_start = self.peek()
            visibility = None
            is_static = False
            is_volatile = False

            # Parse modifiers. A class member must declare its visibility
            # explicitly; the rule checker reports a member with none.
            while True:
                if self.match('public'): visibility = 'public'
                elif self.match('private'): visibility = 'private'
                elif self.match('protected'): visibility = 'protected'
                elif self.match('static'): is_static = True
                elif self.match('volatile'): is_volatile = True
                else: break

            # Check for methods
            member_decorators = []
            is_classmethod = False
            is_property = False
            while self.match('@'):
                dec_name = self.consume(expected_type='IDENT').value
                dec_args = []
                dec_kwargs = {}
                if self.match('('):
                    if not self.check(')'):
                        while True:
                            is_named = (self.check_type('IDENT')
                                        and self.peek(1).value in ('=', ':'))
                            if is_named:
                                key = self.consume().value
                                self.consume()
                                dec_kwargs[key] = self.parse_expression()
                            else:
                                dec_args.append(self.parse_expression())
                            if not self.match(','): break
                    self.consume(expected_value=')')
                member_decorators.append(Decorator(dec_name, dec_args, dec_kwargs))
                if dec_name == 'staticmethod': is_static = True
                elif dec_name == 'classmethod': is_classmethod = True
                elif dec_name == 'property': is_property = True

            if self.check('def') or self.check('fn'):
                # Aura method names map to Python dunder names (new -> __init__,
                # str -> __str__, len -> __len__, ...). Names already written as
                # dunders are preserved verbatim. The table is shared with the
                # transformers via ``SPECIAL_METHOD_NAMES``.
                if self.check('fn'):
                    tok = self.peek()
                    raise self.error(
                        "'fn' is not part of Aura; use 'def' instead", tok)
                method_name = self.peek(1).value
                if method_name == 'init':
                    tok = self.peek(1)
                    raise self.error(
                        "'init' is not the Aura constructor; use 'new' "
                        "instead", tok)
                override = self._SPECIAL_METHOD_NAMES.get(method_name)
                func = self.parse_function_decl(
                    decorators=member_decorators,
                    name_override=override,
                )
                method = Method(func.name, func.params, func.return_type, func.body,
                                is_static=is_static, is_classmethod=is_classmethod,
                                is_property=is_property, visibility=visibility,
                                is_volatile=is_volatile, decorators=member_decorators,
                                owner=name)
                method.with_location(self._member_location(member_start))
                members.append(method)
            elif self.check('class'):
                # Nested class: `class Inner { ... }`.
                inner = self.parse_class_decl(
                    decorators=member_decorators,
                    visibility=visibility,
                )
                inner.with_location(self._member_location(member_start))
                members.append(inner)
            else:
                 # Fields: x: Int = 1
                if self.check('}') or self.check('EOF'): break
                # `let`, `let mut` and bare `mut` all declare a field.
                field_mutable = True
                if self.peek().value == 'let':
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
                    field = VarDecl(field_name, field_mutable, t, v,
                                    visibility=visibility, is_static=is_static,
                                    is_volatile=is_volatile, owner=name)
                    field.with_location(self._member_location(member_start))
                    members.append(field)
                else:
                    raise self.error(f"Unexpected token in class: {self.peek().value}")

        self.consume(expected_value='}')
        return ClassDecl(name, members, base_class, type_params, decorators, visibility, is_static, is_volatile)

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
        # Handle nested module names if needed (e.g. stdlib.collections)
        while self.match('.'):
            name += "." + self.consume(expected_type='IDENT').value

        # Parse the module body as a list of member declarations.
        self.consume(expected_value='{')
        members = []
        while not self.check('}') and not self.check('EOF'):
            if self.match('export'):
                pass  # visibility marker; Python has no equivalent
            member = self.parse_statement()
            if member is not None:
                members.append(member)
        self.consume(expected_value='}')
        return Module(name, members)

    def parse_type_decl(self):
        self.consume(expected_value='type')
        name = self.consume(expected_type='IDENT').value

        # Generics: type Result[T, E] only
        type_params = []
        if self.match('['):
            while True:
                type_params.append(self.consume(expected_type='IDENT').value)
                if not self.match(','): break
            self.consume(expected_value=']')

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
            if tok.type in ('IDENT', 'INT', 'FLOAT', 'STRING') or tok.value in ('true', 'false', 'none', 'null', '.'):
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
            # Generics: List[Int] (Python/Aura legacy) OR Result<T> (Aura new?)
            if self.match('['):
                arg = self.parse_type()
                while self.match(','):
                     arg += ", " + self.parse_type()
                self.consume(expected_value=']')
                t_name += f"[{arg}]"
            elif self.match('<'):
                arg = self.parse_type()
                while self.match(','):
                     arg += ", " + self.parse_type()
                self.consume(expected_value='>')
                t_name += f"[{arg}]" # Map <T> to [T] for Python typing compatibility
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
        type_params = []
        if self.match('['):
            while True:
                type_params.append(self.consume(expected_type='IDENT').value)
                if not self.match(','):
                    break
            self.consume(expected_value=']')

        # Traits may extend other traits: `trait Loud implements Greeter(...)`
        # or `trait Loud(Greeter)`, mirroring class inheritance.
        bases = []
        if self.match('('):
            if not self.check(')'):
                while True:
                    bases.append(self.consume(expected_type='IDENT').value)
                    if not self.match(','):
                        break
            self.consume(expected_value=')')
        if self.match('implements'):
            while True:
                bases.append(self.consume(expected_type='IDENT').value)
                if not self.match(','):
                    break
        base_class = ", ".join(bases) if bases else None

        self.consume(expected_value='{')
        members = []
        while not self.check('}') and not self.check('EOF'):
            member_start = self.peek()
            # Member modifiers. Visibility must be explicit; the rule checker
            # reports a member with none.
            member_visibility = None
            is_static = False
            while True:
                if self.match('public'): member_visibility = 'public'
                elif self.match('private'): member_visibility = 'private'
                elif self.match('protected'): member_visibility = 'protected'
                elif self.match('static'): is_static = True
                else: break

            # Member decorators: `@property`, `@staticmethod`, `@classmethod`.
            member_decorators = []
            is_classmethod = False
            is_property = False
            while self.match('@'):
                dec_name = self.consume(expected_type='IDENT').value
                dec_args = []
                dec_kwargs = {}
                if self.match('('):
                    if not self.check(')'):
                        while True:
                            is_named = (self.check_type('IDENT')
                                        and self.peek(1).value in ('=', ':'))
                            if is_named:
                                key = self.consume().value
                                self.consume()
                                dec_kwargs[key] = self.parse_expression()
                            else:
                                dec_args.append(self.parse_expression())
                            if not self.match(','): break
                    self.consume(expected_value=')')
                member_decorators.append(Decorator(dec_name, dec_args, dec_kwargs))
                if dec_name == 'staticmethod': is_static = True
                elif dec_name == 'classmethod': is_classmethod = True
                elif dec_name == 'property': is_property = True

            if self.check('def') or self.check('fn'):
                # Reuse the full function parser so parameters, defaults,
                # types, generics and default bodies are all preserved.
                self.consume()  # eat def/fn
                func = self.parse_function_decl_after_keyword()
                method = Method(
                    func.name, func.params, func.return_type, func.body,
                    is_static=is_static, is_classmethod=is_classmethod,
                    is_property=is_property, visibility=member_visibility,
                    decorators=member_decorators, owner=name,
                )
                method.with_location(self._member_location(member_start))
                members.append(method)
            elif self.peek().type == 'IDENT':
                # Field declaration inside a trait: `name: Type`,
                # `let name: Type`, `let name: Type = default`, `mut name`.
                if self.peek().value in ('let', 'mut'):
                    self.consume()
                    if self.check('mut'):
                        self.consume()
                member_name = self.consume().value
                field_type = None
                if self.match(':'):
                    field_type = self.parse_type()
                default = None
                if self.match('='):
                    default = self.parse_expression()
                field = VarDecl(member_name, True, field_type, default,
                                visibility=member_visibility, is_static=is_static,
                                owner=name)
                field.with_location(self._member_location(member_start))
                members.append(field)
            else:
                break

        self.consume(expected_value='}')
        return TraitDecl(name, members, type_params, visibility, base_class)

    def parse_function_decl_after_keyword(self):
        """Parse a function signature/body when `def`/`fn` was already consumed.

        Used by trait declarations so they share the exact function grammar.
        """
        name = self.consume(expected_type='IDENT').value

        type_params = []
        if self.match('['):
            while True:
                type_params.append(self.consume(expected_type='IDENT').value)
                if not self.match(','):
                    break
            self.consume(expected_value=']')

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
                            type_params=type_params)

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
    def parse_if_stmt(self):
        self.consume(expected_value='if')
        cond = self.parse_expression()
        then_body = self.parse_block()
        else_body = None
        if self.match('else'):
            else_body = [self.parse_if_stmt()] if self.check('if') else self.parse_block()
        return IfStmt(cond, then_body, else_body)

    def parse_guard_stmt(self):
        self.consume(expected_value='guard')
        cond = self.parse_expression()
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
        cond = self.parse_expression()
        body = self.parse_block()
        return WhileStmt(cond, body)

    def parse_unless_stmt(self):
        self.consume(expected_value='unless')
        cond = self.parse_expression()
        body = self.parse_block()
        else_body = None
        if self.match('else'):
            else_body = [self.parse_if_stmt()] if self.check('if') else self.parse_block()
        return UnlessStmt(cond, body, else_body)

    def parse_until_stmt(self):
        self.consume(expected_value='until')
        cond = self.parse_expression()
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
        iterable = self.parse_expression()

        step = None
        if self.match('step'):
            step = self.parse_expression()

        body = self.parse_block()
        return ForStmt(pattern, iterable, body, step)

    def parse_return_stmt(self):
        self.consume(expected_value='return')
        val = None
        if not self.check(';') and not self.check('}'):
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
            # `catch { }`            -> bare catch
            # `catch e { }`          -> exception binding, no type
            # `catch Type { }`       -> type only
            # `catch Type as e { }`  -> type and binding
            if self.check_type('IDENT') and self.peek().value != '{':
                first = self.consume().value
                if self.match('as'):
                    exc_type = first
                    var_name = self.consume(expected_type='IDENT').value
                elif self.peek().value == '{':
                    var_name = first
                else:
                    exc_type = first
                    if self.check_type('IDENT') and self.peek().value != '{':
                        var_name = self.consume().value

            body = self.parse_block()
            catch_clauses.append(CatchClause(exc_type, var_name, body))

        finally_body = None
        if self.match('finally'):
            finally_body = self.parse_block()

        if not catch_clauses and finally_body is None:
            raise self.error(
                "try requires at least one catch clause or a finally block")

        return TryStmt(try_body, catch_clauses, finally_body)

    def parse_with_stmt(self):
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
        return WithStmt(items, body)

    def parse_match_stmt(self):
        self.consume(expected_value='match')
        expr = self.parse_expression()
        self.consume(expected_value='{')
        cases = []
        while not self.check('}') and not self.check('EOF'):
             # case pattern { ... } OR pattern -> stmt
             self.match('case')
             pat_node = None

             # Parse pattern (simplified as expr for now)
             pattern_expr = self.parse_expression()

             if isinstance(pattern_expr, Identifier) and pattern_expr.name == '_':
                pat_node = WildcardPattern()
             elif isinstance(pattern_expr, (IntLiteral, StrLiteral, BoolLiteral)):
                pat_node = LiteralPattern(pattern_expr)
             elif isinstance(pattern_expr, Identifier):
                 pat_node = IdentifierPattern(pattern_expr.name)
             elif isinstance(pattern_expr, (TupleLiteral, ListLiteral)):
                 # Convert tuple/list literal to ListPattern for destructuring

                 patterns = []
                 for elem in (pattern_expr.elements if isinstance(pattern_expr.elements, list) else []):
                     if isinstance(elem, Identifier):
                         patterns.append(IdentifierPattern(elem.name))
                     elif isinstance(elem, (IntLiteral, StrLiteral, BoolLiteral)):
                         patterns.append(LiteralPattern(elem))
                     elif isinstance(elem, UnaryOp) and elem.op == '*':
                         # This is the *rest part
                         if isinstance(elem.operand, Identifier):
                             rest_pattern = IdentifierPattern(elem.operand.name)
                             # Return early or handle as rest
                             pat_node = ListPattern(patterns, rest_pattern=rest_pattern)
                             break
                     else:
                         # Fallback/Recurse needed for nested? For now literal fallback
                         patterns.append(LiteralPattern(elem))

                 pat_node = ListPattern(patterns)
             elif isinstance(pattern_expr, CallExpr):
                 # Convert CallExpr to ConstructorPattern (e.g. Some(x), Err(msg))

                 pat_name = pattern_expr.func.name if isinstance(pattern_expr.func, Identifier) else "unknown"
                 subpatterns = []
                 for arg in pattern_expr.args:
                     if isinstance(arg, Identifier):
                         subpatterns.append(IdentifierPattern(arg.name))
                     elif isinstance(arg, (IntLiteral, StrLiteral, BoolLiteral)):
                         subpatterns.append(LiteralPattern(arg.value))
                     else:
                         subpatterns.append(LiteralPattern(arg))

                 pat_node = ConstructorPattern(pat_name, subpatterns)
             else:
                pat_node = LiteralPattern(pattern_expr)

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
        if token.value == 'yield':
            self.consume()
            # `yield` may carry a value or stand alone. Do not let it swallow
            # the following statement, so stop at a block/statement boundary.
            if (self.check('}') or self.check(';') or self.check_type('EOF')
                    or (self.peek().type == 'IDENT'
                        and self.peek().value in _STATEMENT_KEYWORDS)):
                lhs = UnaryOp('yield', operand=None)
            else:
                lhs = UnaryOp('yield', operand=self.parse_expression(13))
        elif token.type == 'OP' and token.value == '!':
            raise self.error(
                "'!' is not part of Aura; use 'not' instead", token)
        elif token.value == 'not':
            self.consume()
            # `not` binds looser than comparisons (so `not a in b` is
            # `not (a in b)`) but tighter than `and`/`or`, matching Python.
            rhs = self.parse_expression(6)
            lhs = UnaryOp('not', operand=rhs)
        elif (token.type == 'OP' and token.value in ('-', '+', '~', '*', '**', '...')
                or token.value == 'await'):
            op = token.value
            self.consume()
            # Arithmetic unary binds looser than `**` (so `-2 ** 2` is
            # `-(2 ** 2)`) but tighter than `*`/`/` (so `-2 * 3` is `(-2) * 3`).
            rhs = self.parse_expression(11)
            lhs = UnaryOp(op, operand=rhs)
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
        precedences = {
            '=': 1, '+=': 1, '-=': 1, '*=': 1, '/=': 1, '%=': 1,
            '**=': 1, '&=': 1, '|=': 1, '^=': 1, '<<=': 1, '>>=': 1, '??=': 1,
            '?': 2, # Ternary
            'or': 3,
            'and': 4,
            # Bitwise operators bind looser than equality but tighter than
            # `and`/`or`, mirroring the grammar's bitwiseOr/Xor/And chain.
            '|': 4.2,
            '^': 4.4,
            '&': 4.6,
            '==': 5, '!=': 5, '<': 6, '>': 6, '<=': 6, '>=': 6, 'in': 6, 'not in': 6, 'is': 6, 'is not': 6,
            '..': 7, '..<': 7, # Range
            '??': 8, '?:': 8, # Null coalescing/Elvis
            # Shifts bind looser than additive but tighter than comparison,
            # matching Python (`1 << 2 + 1` == `1 << 3`).
            '<<': 5.5, '>>': 5.5,
            '+': 9, '-': 9,
            '*': 10, '/': 10, '%': 10, 'as': 10,
            '**': 11,
            '|>': 1, # Pipe usually low prec, handled separately?
            '.': 12, '[': 12, '(': 12, '?.': 12
        }
        return precedences.get(op, 0)

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
                    elements.append(self.parse_expression())
                    if not self.match(','): break
                self.consume(expected_value=')')

                # Check for arrow function: (x, y) => ...
                if self.match('=>'):
                    params = []
                    for el in elements:
                        if isinstance(el, Identifier):
                            params.append(Parameter(el.name))
                        else:
                            raise self.error("Invalid parameter in lambda")

                    body = self.parse_lambda_body()
                    return LambdaExpr(params, body)

                return self.parse_postfix(TupleLiteral(elements))

            self.consume(expected_value=')')
            # Check for arrow function: (Params) => Expr
            if self.match('=>'):
                # expr is the params part. It could be Identifier or TupleLiteral
                params = []
                if isinstance(expr, Identifier):
                    params.append(Parameter(expr.name))
                elif isinstance(expr, TupleLiteral):
                    for el in expr.elements:
                        if isinstance(el, Identifier):
                            params.append(Parameter(el.name))

                body = self.parse_lambda_body()
                return LambdaExpr(params, body)

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
                expr_src, sep, fmt_spec = inner.partition(':')
                expr_src = expr_src.strip()
                if expr_src:
                    try:
                        sub_tokens = Tokenizer(expr_src).tokenize()
                        expr = Parser(sub_tokens).parse_expression()
                    except Exception:
                        expr = Identifier(expr_src)
                    parts.append((expr, sep + fmt_spec if sep else ''))
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

        # Heuristic: Check for statement keywords -> Block
        tok = self.peek()
        stmt_keywords = ('let', 'const', 'return', 'while', 'for', 'if', 'try', 'match', 'guard', 'unless')
        if tok.value in stmt_keywords:
            return self.parse_block_expr_internal(first_stmt=None)

        # Heuristic: `{ ident = ... }` where `=` is assignment (not a dict
        # key `:` or a comparison) is a block, not a dict/set literal.
        if tok.type == 'IDENT' and self.peek(1).value == '=' and self.peek(1).type != '==':
            return self.parse_block_expr_internal(first_stmt=None)

        # A top-level `:` or `,` (or a `**` spread) uniquely identifies a
        # dict/set literal. Without one, the braces contain statements, so a
        # bare block such as `{ print(x); let y = 1 }` is handled correctly.
        depth = 1
        i = self.pos
        is_literal = False
        while i < len(self.tokens):
            v = self.tokens[i].value
            if v in ('{', '(', '['):
                depth += 1
            elif v in ('}', ')', ']'):
                depth -= 1
                if depth == 0:
                    break
            elif depth == 1 and v in (':', ',', '**', 'for'):
                is_literal = True
                break
            i += 1
        if not is_literal:
            return self.parse_block_expr_internal(first_stmt=None)

        # Parsing logic handles Dict (with spread) vs Set vs Block
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
        exc.filename = path
        exc.line = 1
        exc.column = 1
        raise exc
    tokenizer = Tokenizer(source, filename=path)
    tokens = tokenizer.tokenize()
    parser = Parser(tokens, filename=path)
    return parser.parse()

def parse_value(value_str: str) -> Node:
    """Parse a single expression/value string."""
    tokenizer = Tokenizer(value_str)
    tokens = tokenizer.tokenize()
    parser = Parser(tokens)
    return parser.parse_expression()
