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
import traceback
from collections import OrderedDict

from aura.parser.to_ast import Parser, Tokenizer
from aura.transpiler.semantics import MutabilityChecker
from aura.transpiler.types import TypeChecker

KEYWORDS = [
    'let', 'mut', 'const', 'def', 'class', 'trait', 'enum', 'module',
    'type', 'import', 'from', 'as', 'return', 'if', 'else', 'unless',
    'guard', 'match', 'case', 'for', 'while', 'until', 'loop', 'break',
    'continue', 'try', 'catch', 'finally', 'throw', 'await', 'async',
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


class AuraLanguageServer:
    def __init__(self, reader=None, writer=None):
        self.reader = reader or sys.stdin.buffer
        self.writer = writer or sys.stdout.buffer
        self.documents = OrderedDict()
        self.shutdown_requested = False
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
        length = int(headers.get(b'content-length', 0))
        if length <= 0:
            return None
        if length > MAX_MESSAGE_BYTES:
            raise ValueError(f"message exceeds {MAX_MESSAGE_BYTES} bytes")
        body = self.reader.read(length)
        if not body:
            return None
        return json.loads(body.decode('utf-8'))

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
            message = self._read_message()
            if message is None:
                break
            try:
                self._handle(message)
            except Exception:  # keep the server alive on internal errors
                if 'id' in message:
                    self._write_message({
                        'jsonrpc': '2.0', 'id': message['id'],
                        'error': {'code': -32603, 'message': traceback.format_exc()},
                    })

    def _handle(self, message):
        method = message.get('method')
        params = message.get('params') or {}
        request_id = message.get('id')

        if method == 'initialize':
            self._respond(request_id, {
                'capabilities': {
                    'textDocumentSync': 1,  # full
                    'hoverProvider': True,
                    'completionProvider': {'triggerCharacters': ['.', ':']},
                    'documentSymbolProvider': True,
                },
                'serverInfo': {'name': 'aura-lsp', 'version': '0.1.0'},
            })
        elif method == 'initialized':
            pass
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
        return {
            'range': {
                'start': {'line': max(0, line - 1), 'character': max(0, column - 1)},
                'end': {'line': max(0, line - 1), 'character': max(0, column)},
            },
            'severity': 1,
            'source': 'aura',
            'message': str(exc),
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
