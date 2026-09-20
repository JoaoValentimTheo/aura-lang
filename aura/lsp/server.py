"""A minimal Language Server for Aura.

Speaks JSON-RPC over stdio using the Language Server Protocol. It provides:

* live **diagnostics** from the parser, mutability checker and type checker;
* **hover** for identifiers (inferred type when available);
* **completion** for keywords, builtins and standard-library modules;
* **document symbols** (functions, classes, enums, traits, modules).

It is intentionally dependency-free so it works with any editor that can
launch ``aura lsp``. Capabilities can grow without changing the transport.
"""

import json
import sys
from collections import OrderedDict

from aura.parser.to_ast import MAX_SOURCE_BYTES, Parser, Tokenizer
from aura.transpiler.semantics import MutabilityChecker
from aura.transpiler.types import TypeChecker

KEYWORDS = [
    'let', 'mut', 'const', 'def', 'class', 'trait', 'enum', 'module',
    'type', 'import', 'from', 'as', 'return', 'if', 'else', 'unless',
    'guard', 'match', 'case', 'for', 'while', 'until', 'loop', 'break',
    'continue', 'try', 'catch', 'finally', 'throw', 'await', 'async',
    'abstract', 'public', 'private', 'protected', 'static', 'volatile',
    'and', 'or', 'not', 'in', 'is', 'true', 'false', 'none',
]

BUILTINS = [
    'print', 'len', 'range', 'int', 'float', 'str', 'bool', 'list', 'dict',
    'set', 'tuple', 'sum', 'min', 'max', 'abs', 'round', 'sorted',
    'enumerate', 'zip', 'map', 'filter', 'reduce',
]

STDLIB_MODULES = [
    'stdlib.math', 'stdlib.string', 'stdlib.collections', 'stdlib.itertools',
    'stdlib.json', 'stdlib.time', 'stdlib.io', 'stdlib.regex', 'stdlib.os',
    'stdlib.http',
]

# Upper bound on a single JSON-RPC message body, guarding against a buggy or
# malicious client forcing an unbounded allocation.
MAX_MESSAGE_BYTES = 16 * 1024 * 1024

# Upper bound on open documents (and their caches), guarding against a client
# that opens unbounded URIs. Oldest documents are evicted first.
MAX_DOCUMENTS = 512


class ProtocolError(ValueError):
    """A malformed JSON-RPC frame (bad header, encoding or JSON)."""


class AuraLanguageServer:
    def __init__(self, reader=None, writer=None):
        self.reader = reader or sys.stdin.buffer
        self.writer = writer or sys.stdout.buffer
        self.documents = OrderedDict()
        self.shutdown_requested = False
        # Spec requires a `-32002` error for requests that arrive before the
        # `initialize` handshake completes.
        self.initialized = False
        # (uri, text) -> (text, program, parse_error). Avoids re-parsing the same
        # document for hover, symbols and diagnostics within a request cycle.
        self._parse_cache = OrderedDict()
        # uri -> (text, diagnostics). Avoids re-running the checkers on an
        # unchanged document.
        self._diagnostics_cache = OrderedDict()

    # -- transport ----------------------------------------------------------

    def _read_message(self):
        headers = {}
        while True:
            line = self.reader.readline()
            if not line:
                return None
            line = line.strip()
            if not line:
                break
            if b':' in line:
                key, value = line.split(b':', 1)
                headers[key.strip().lower()] = value.strip()
        raw_length = headers.get(b'content-length')
        try:
            length = int(raw_length) if raw_length is not None else 0
        except (TypeError, ValueError):
            raise ProtocolError(
                f"invalid Content-Length header: {raw_length!r}") from None
        if length <= 0:
            return None
        if length > MAX_MESSAGE_BYTES:
            raise ProtocolError(f"message exceeds {MAX_MESSAGE_BYTES} bytes")
        body = self.reader.read(length)
        if not body:
            return None
        try:
            text = body.decode('utf-8')
        except UnicodeDecodeError:
            raise ProtocolError("message body is not valid UTF-8") from None
        try:
            return json.loads(text)
        except json.JSONDecodeError as exc:
            raise ProtocolError(f"invalid JSON payload: {exc}") from None

    def _evict_old_documents(self):
        """Keep the open-document (and cache) count bounded."""
        while len(self.documents) > MAX_DOCUMENTS:
            uri, _ = self.documents.popitem(last=False)
            self._parse_cache.pop(uri, None)
            self._diagnostics_cache.pop(uri, None)

    def _parsed(self, uri):
        """Return ``(text, program, error)`` for the current text of ``uri``.

        The result is cached per (uri, text), so repeated feature requests on
        the same document version parse only once.
        """
        text = self.documents.get(uri, '')
        cached = self._parse_cache.get(uri)
        if cached is not None and cached[0] == text:
            return cached
        if len(text.encode('utf-8')) > MAX_SOURCE_BYTES:
            program = None
            error = SyntaxError(
                f"Document too large "
                f"({len(text.encode('utf-8'))} bytes; limit {MAX_SOURCE_BYTES})"
            )
        else:
            try:
                program = Parser(Tokenizer(text).tokenize()).parse()
                error = None
            except Exception as exc:
                program = None
                error = exc
        entry = (text, program, error)
        self._parse_cache[uri] = entry
        return entry

    def _diagnostics_for(self, uri):
        """Return the diagnostics for ``uri``, computing them once per version.

        Mutability and type checks are not re-run for hover/symbols on an
        unchanged document.
        """
        text, program, error = self._parsed(uri)
        cached = self._diagnostics_cache.get(uri)
        if cached is not None and cached[0] == text:
            return cached[1]
        diagnostics = []
        if program is None:
            diagnostics.append(self._diagnostic_from_error(error))
        else:
            from aura.transpiler.rules import RuleChecker

            checker = MutabilityChecker()
            checker.check_program(program)
            type_checker = TypeChecker()
            type_checker.check_program(program)
            rule_checker = RuleChecker()
            rule_checker.check_program(program)
            for err in (list(getattr(checker, 'diagnostics', []))
                        + list(getattr(type_checker, 'diagnostics', []))
                        + list(rule_checker.collector.errors)):
                diagnostics.append(self._diagnostic_from_aura_error(err))
        self._diagnostics_cache[uri] = (text, diagnostics)
        return diagnostics

    def _write_message(self, payload):
        body = json.dumps(payload).encode('utf-8')
        header = f"Content-Length: {len(body)}\r\n\r\n".encode()
        self.writer.write(header + body)
        self.writer.flush()

    def _respond(self, request_id, result):
        self._write_message({'jsonrpc': '2.0', 'id': request_id, 'result': result})

    def _notify(self, method, params):
        self._write_message({'jsonrpc': '2.0', 'method': method, 'params': params})

    # -- lifecycle ----------------------------------------------------------

    def run(self):
        while not self.shutdown_requested:
            try:
                message = self._read_message()
            except ProtocolError:
                # A malformed frame is not recoverable mid-stream: report a
                # JSON-RPC parse error and stop, rather than crashing with a
                # traceback on untrusted input.
                self._write_message({
                    'jsonrpc': '2.0', 'id': None,
                    'error': {'code': -32700, 'message': 'Parse error'},
                })
                break
            if message is None:
                break
            try:
                self._handle(message)
            except Exception:  # keep the server alive on internal errors
                if isinstance(message, dict) and 'id' in message:
                    self._write_message({
                        'jsonrpc': '2.0', 'id': message['id'],
                        'error': {'code': -32603, 'message': 'Internal server error'},
                    })

    def _handle(self, message):
        method = message.get('method')
        params = message.get('params') or {}
        request_id = message.get('id')

        if method == 'initialize':
            self.initialized = True
            self._respond(request_id, {
                'capabilities': {
                    'textDocumentSync': 1,  # full
                    'hoverProvider': True,
                    'completionProvider': {'triggerCharacters': ['.', ':']},
                    'documentSymbolProvider': True,
                    'definitionProvider': True,
                    'referencesProvider': True,
                    'renameProvider': {'prepareProvider': True},
                    'documentFormattingProvider': True,
                },
                'serverInfo': {'name': 'aura-lsp', 'version': '0.1.0'},
            })
        elif method == 'initialized':
            pass
        elif not self.initialized and method not in ('exit', 'shutdown'):
            # Requests before the handshake get the spec-mandated error.
            if request_id is not None:
                self._write_message({
                    'jsonrpc': '2.0', 'id': request_id,
                    'error': {'code': -32002,
                              'message': 'Server not initialized'},
                })
        elif method == 'shutdown':
            self.shutdown_requested = True
            self._respond(request_id, None)
        elif method == 'exit':
            self.shutdown_requested = True
        elif method == 'textDocument/didOpen':
            doc = params['textDocument']
            self.documents[doc['uri']] = doc['text']
            self.documents.move_to_end(doc['uri'])
            self._evict_old_documents()
            self._publish_diagnostics(doc['uri'])
        elif method == 'textDocument/didChange':
            doc = params['textDocument']
            changes = params.get('contentChanges', [])
            if changes:
                self.documents[doc['uri']] = changes[-1]['text']
                self.documents.move_to_end(doc['uri'])
            self._publish_diagnostics(doc['uri'])
        elif method == 'textDocument/didSave':
            doc = params['textDocument']
            self._publish_diagnostics(doc['uri'])
        elif method == 'textDocument/didClose':
            doc = params['textDocument']
            self.documents.pop(doc['uri'], None)
            self._parse_cache.pop(doc['uri'], None)
            self._diagnostics_cache.pop(doc['uri'], None)
            self._notify('textDocument/publishDiagnostics',
                         {'uri': doc['uri'], 'diagnostics': []})
        elif method == 'textDocument/hover':
            self._respond(request_id, self._hover(params))
        elif method == 'textDocument/completion':
            self._respond(request_id, self._completion(params))
        elif method == 'textDocument/documentSymbol':
            self._respond(request_id, self._document_symbols(params))
        elif method == 'textDocument/definition':
            self._respond(request_id, self._definition(params))
        elif method == 'textDocument/references':
            self._respond(request_id, self._references(params))
        elif method == 'textDocument/prepareRename':
            self._respond(request_id, self._prepare_rename(params))
        elif method == 'textDocument/rename':
            self._respond(request_id, self._rename(params))
        elif method == 'textDocument/formatting':
            self._respond(request_id, self._formatting(params))
        elif request_id is not None:
            self._respond(request_id, None)

    # -- features -----------------------------------------------------------

    def _publish_diagnostics(self, uri):
        diagnostics = self._diagnostics_for(uri)
        self._notify('textDocument/publishDiagnostics',
                     {'uri': uri, 'diagnostics': diagnostics})

    def _diagnostic_from_aura_error(self, err):
        """Convert a structured ``AuraError`` into an LSP diagnostic."""
        loc = getattr(err, 'location', None)
        if loc is not None and getattr(loc, 'line', 0):
            line = loc.line - 1
            start_col = max(0, loc.column - 1) if loc.column else 0
            length = getattr(loc, 'length', 0) or 1
        else:
            line = 0
            start_col = 0
            length = 1
        severity = 2 if getattr(err, 'severity', None) is not None \
            and err.severity.value == 'warning' else 1
        message = err.message
        if getattr(err, 'hint', None):
            message = f"{message} (hint: {err.hint})"
        return {
            'range': {
                'start': {'line': line, 'character': start_col},
                'end': {'line': line, 'character': start_col + length},
            },
            'severity': severity,
            'code': getattr(err.code, 'value', None),
            'source': 'aura',
            'message': message,
        }

    def _diagnostic_from_error(self, exc):
        line = getattr(exc, 'line', 1) or 1
        column = getattr(exc, 'column', 1) or 1
        msg = str(exc)
        # Sanitize: strip internal paths and file system details
        import re
        msg = re.sub(r'File ".*?"', 'File "<source>"', msg)
        msg = re.sub(r'/[^\s:]+\.py', '<module>', msg)
        return {
            'range': {
                'start': {'line': max(0, line - 1), 'character': max(0, column - 1)},
                'end': {'line': max(0, line - 1), 'character': max(0, column)},
            },
            'severity': 1,
            'source': 'aura',
            'message': msg,
        }

    def _hover(self, params):
        uri = params['textDocument']['uri']
        text = self.documents.get(uri, '')
        position = params['position']
        word = self._word_at(text, position['line'], position['character'])
        if not word:
            return None
        details = None
        try:
            _, program, error = self._parsed(uri)
            if program is None:
                raise error
            checker = TypeChecker()
            checker.check_program(program)
            if word in checker.context:
                details = f"{word}: {checker.context[word]}"
            elif word in checker.functions:
                details = f"{word}{checker.functions[word]}"
        except Exception:
            details = None
        if details is None:
            if word in KEYWORDS:
                details = f"Aura keyword `{word}`"
            elif word in BUILTINS:
                details = f"Aura builtin `{word}`"
            else:
                details = f"`{word}`"
        return {'contents': {'kind': 'markdown', 'value': details}}

    def _completion(self, params):
        items = [{'label': kw, 'kind': 14} for kw in KEYWORDS]
        items += [{'label': name, 'kind': 3} for name in BUILTINS]
        items += [{'label': mod, 'kind': 9} for mod in STDLIB_MODULES]
        return {'isIncomplete': False, 'items': items}

    def _document_symbols(self, params):
        uri = params['textDocument']['uri']
        symbols = []
        _, program, _ = self._parsed(uri)
        if program is None:
            return symbols
        for stmt in getattr(program, 'statements', []):
            kind = None
            name = getattr(stmt, 'name', None)
            cls = type(stmt).__name__
            if cls == 'FunctionDecl':
                kind = 12
            elif cls == 'ClassDecl':
                kind = 5
            elif cls == 'EnumDecl':
                kind = 10
            elif cls == 'TraitDecl':
                kind = 11
            elif cls == 'Module':
                kind = 2
            if kind and name:
                line = max(0, (getattr(stmt, 'line', 1) or 1) - 1)
                selection = {'start': {'line': line, 'character': 0},
                             'end': {'line': line, 'character': len(name)}}
                symbols.append({
                    'name': name,
                    'kind': kind,
                    'range': selection,
                    'selectionRange': selection,
                })
        return symbols

    # -- definition / references / rename ----------------------------------

    # Token types that name a symbol.
    _NAME_TOKENS = ('IDENT',)

    def _identifier_tokens(self, uri):
        """Return ``[(name, line, column)]`` for every identifier token.

        Positions are 0-indexed (LSP convention). A lexical error yields an
        empty list rather than raising, so navigation features stay usable
        while the document is mid-edit.
        """
        text = self.documents.get(uri, '')
        try:
            tokens = Tokenizer(text).tokenize()
        except Exception:
            return []
        result = []
        for tok in tokens:
            if tok.type in self._NAME_TOKENS:
                result.append((tok.value, (tok.line or 1) - 1,
                               max(0, (tok.column or 1) - 1)))
        return result

    def _declarations(self, uri):
        """Map symbol names to their declaration positions (0-indexed).

        Only the *first* declaration of a name is kept, which is the one an
        editor should jump to. Parameters have no location of their own, so
        they are found by searching the tokens on the declaration line.
        """
        _, program, _ = self._parsed(uri)
        if program is None:
            return {}
        decls = {}
        tokens = self._identifier_tokens(uri)

        def record(name, line, column):
            if name and name not in decls:
                decls[name] = (line, column)

        def first_token_on(name, line0):
            for tname, tline, tcol in tokens:
                if tline == line0 and tname == name:
                    return tcol
            return 0

        def walk(node):
            if node is None:
                return
            if isinstance(node, (list, tuple)):
                for item in node:
                    walk(item)
                return
            loc = getattr(node, 'location', None)
            if loc is not None and getattr(loc, 'line', 0):
                line0 = loc.line - 1
                cls = type(node).__name__
                name = getattr(node, 'name', None)
                if cls in ('FunctionDecl', 'ClassDecl', 'EnumDecl',
                           'TraitDecl', 'Module', 'VarDecl', 'ConstDecl',
                           'TypeDecl') and isinstance(name, str):
                    # Point at the *name token*, not the statement start, so
                    # `def foo` resolves to `foo` (keywords precede the name).
                    record(name, line0, first_token_on(name, line0))
                # Parameters and methods are declared inside the body.
                if cls in ('FunctionDecl', 'Method'):
                    for param in getattr(node, 'params', []) or []:
                        if getattr(param, 'name', None) and param.name != '*':
                            # Params sit on the `def` line; find the token.
                            record(param.name, line0,
                                   first_token_on(param.name, line0))
                body = getattr(node, 'body', None) or getattr(node, 'members', None)
                if isinstance(body, (list, tuple)):
                    for member in body:
                        walk(member)
                return
            if hasattr(node, '__dict__'):
                for value in vars(node).values():
                    if isinstance(value, (list, tuple)) or hasattr(value, '__dict__'):
                        walk(value)

        for stmt in getattr(program, 'statements', []):
            walk(stmt)
        return decls

    def _definition(self, params):
        uri = params['textDocument']['uri']
        text = self.documents.get(uri, '')
        position = params['position']
        word = self._word_at(text, position['line'], position['character'])
        if not word:
            return None
        decls = self._declarations(uri)
        target = decls.get(word)
        if target is None:
            return None
        line, column = target
        pos = {'line': line, 'character': column}
        return {'uri': uri, 'range': {'start': pos, 'end': pos}}

    def _references(self, params):
        uri = params['textDocument']['uri']
        text = self.documents.get(uri, '')
        position = params['position']
        word = self._word_at(text, position['line'], position['character'])
        if not word:
            return []
        include_declaration = (params.get('context') or {}).get(
            'includeDeclaration', True)
        decls = self._declarations(uri)
        locations = []
        for name, line, column in self._identifier_tokens(uri):
            if name != word:
                continue
            if not include_declaration and decls.get(word) == (line, column):
                continue
            locations.append({
                'uri': uri,
                'range': {
                    'start': {'line': line, 'character': column},
                    'end': {'line': line, 'character': column + len(name)},
                },
            })
        return locations

    def _prepare_rename(self, params):
        uri = params['textDocument']['uri']
        text = self.documents.get(uri, '')
        position = params['position']
        word = self._word_at(text, position['line'], position['character'])
        if not word or word in KEYWORDS or word in BUILTINS:
            return None
        # Only offer a rename when the name is an identifier we can place.
        decls = self._declarations(uri)
        if word not in decls:
            return None
        line, column = decls[word]
        return {
            'range': {
                'start': {'line': line, 'character': column},
                'end': {'line': line, 'character': column + len(word)},
            },
            'placeholder': word,
        }

    def _rename(self, params):
        uri = params['textDocument']['uri']
        text = self.documents.get(uri, '')
        position = params['position']
        new_name = params.get('newName', '')
        word = self._word_at(text, position['line'], position['character'])
        if not word or not new_name:
            return None
        if not new_name.isidentifier() or new_name in KEYWORDS:
            return None
        edits = []
        for name, line, column in self._identifier_tokens(uri):
            if name != word:
                continue
            edits.append({
                'range': {
                    'start': {'line': line, 'character': column},
                    'end': {'line': line, 'character': column + len(name)},
                },
                'newText': new_name,
            })
        if not edits:
            return None
        return {'changes': {uri: edits}}

    def _formatting(self, params):
        uri = params['textDocument']['uri']
        text = self.documents.get(uri, '')
        if not text:
            return []
        from aura.tools.formatter import format_aura

        try:
            formatted = format_aura(text)
        except Exception:
            return []
        if formatted == text:
            return []
        lines = text.split('\n')
        # Replace the whole document in one edit; safer than diffing lines.
        return [{
            'range': {
                'start': {'line': 0, 'character': 0},
                'end': {'line': len(lines), 'character': 0},
            },
            'newText': formatted,
        }]

    @staticmethod
    def _word_at(text, line, character):
        lines = text.split('\n')
        if line < 0 or line >= len(lines):
            return None
        text_line = lines[line]
        if character > len(text_line):
            character = len(text_line)
        start = character
        while start > 0 and (text_line[start - 1].isalnum() or text_line[start - 1] == '_'):
            start -= 1
        end = character
        while end < len(text_line) and (text_line[end].isalnum() or text_line[end] == '_'):
            end += 1
        return text_line[start:end] or None


def main():
    server = AuraLanguageServer()
    server.run()
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
