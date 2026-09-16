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

from aura.parser.to_ast import Tokenizer, Parser
from aura.transpiler.semantics import MutabilityChecker
from aura.transpiler.types import TypeChecker


KEYWORDS = [
    'let', 'mut', 'const', 'def', 'class', 'trait', 'enum', 'module',
    'type', 'import', 'from', 'as', 'return', 'if', 'else', 'unless',
    'guard', 'match', 'case', 'for', 'while', 'until', 'loop', 'break',
    'continue', 'try', 'catch', 'finally', 'throw', 'await', 'async',
    'and', 'or', 'not', 'in', 'is', 'null', 'true', 'false', 'none',
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


class AuraLanguageServer:
    def __init__(self, reader=None, writer=None):
        self.reader = reader or sys.stdin.buffer
        self.writer = writer or sys.stdout.buffer
        self.documents = {}
        self.shutdown_requested = False
        # (uri, text) -> (program, parse_error). Avoids re-parsing the same
        # document for hover, symbols and diagnostics within a request cycle.
        self._parse_cache = {}
        self._parse_cache_uri = None

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

    def _parsed(self, uri):
        """Return ``(program, error)`` for the current text of ``uri``.

        The result is cached per (uri, text), so repeated feature requests on
        the same document version parse only once.
        """
        text = self.documents.get(uri, '')
        cached = self._parse_cache.get(uri)
        if cached is not None and cached[0] == text:
            return cached[1], cached[2]
        try:
            program = Parser(Tokenizer(text).tokenize()).parse()
            error = None
        except Exception as exc:
            program = None
            error = exc
        self._parse_cache[uri] = (text, program, error)
        return program, error

    def _write_message(self, payload):
        body = json.dumps(payload).encode('utf-8')
        header = f"Content-Length: {len(body)}\r\n\r\n".encode('utf-8')
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
            self._publish_diagnostics(doc['uri'])
        elif method == 'textDocument/didChange':
            doc = params['textDocument']
            changes = params.get('contentChanges', [])
            if changes:
                self.documents[doc['uri']] = changes[-1]['text']
            self._publish_diagnostics(doc['uri'])
        elif method == 'textDocument/didSave':
            doc = params['textDocument']
            self._publish_diagnostics(doc['uri'])
        elif method == 'textDocument/didClose':
            doc = params['textDocument']
            self.documents.pop(doc['uri'], None)
            self._parse_cache.pop(doc['uri'], None)
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
        text = self.documents.get(uri, '')
        diagnostics = []
        program, error = self._parsed(uri)
        if program is None:
            diagnostics.append(self._diagnostic_from_error(error))
            self._notify('textDocument/publishDiagnostics',
                         {'uri': uri, 'diagnostics': diagnostics})
            return

        checker = MutabilityChecker()
        if not checker.check_program(program):
            for error in checker.errors:
                diagnostics.append(self._message_diagnostic(error, text))

        type_checker = TypeChecker()
        if not type_checker.check_program(program):
            for error in type_checker.errors:
                diagnostics.append(self._message_diagnostic(error, text))

        self._notify('textDocument/publishDiagnostics',
                     {'uri': uri, 'diagnostics': diagnostics})

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

    def _message_diagnostic(self, message, text):
        line, column = self._locate(message, text)
        return {
            'range': {
                'start': {'line': line, 'character': column},
                'end': {'line': line, 'character': column + 1},
            },
            'severity': 1,
            'source': 'aura',
            'message': message,
        }

    @staticmethod
    def _locate(message, text):
        """Best-effort extraction of (line, column) from an error message."""
        import re
        match = re.search(r'\(line (\d+)\)', message)
        if match:
            return int(match.group(1)) - 1, 0
        match = re.search(r"name '(\w+)'", message)
        if match:
            name = match.group(1)
            for index, line_text in enumerate(text.split('\n')):
                if name in line_text:
                    return index, line_text.index(name)
        return 0, 0

    def _hover(self, params):
        uri = params['textDocument']['uri']
        text = self.documents.get(uri, '')
        position = params['position']
        word = self._word_at(text, position['line'], position['character'])
        if not word:
            return None
        details = None
        try:
            program, error = self._parsed(uri)
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
        program, _ = self._parsed(uri)
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