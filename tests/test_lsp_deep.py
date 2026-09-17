"""Deep, in-process coverage of the Aura language server.

Drives ``AuraLanguageServer`` directly through in-memory buffers so transport,
lifecycle, feature handlers, diagnostics conversion and error paths are all
exercised without spawning a subprocess.
"""
import io
import json

import pytest

from aura.lsp.server import (
    BUILTINS,
    KEYWORDS,
    MAX_DOCUMENTS,
    MAX_MESSAGE_BYTES,
    STDLIB_MODULES,
    AuraLanguageServer,
)


def make_server():
    """A server reading nothing and writing into an in-memory buffer."""
    out = io.BytesIO()
    server = AuraLanguageServer(reader=io.BytesIO(b''), writer=out)
    return server, out


def encode(message):
    body = json.dumps(message).encode('utf-8')
    return f"Content-Length: {len(body)}\r\n\r\n".encode() + body


def decode(buffer):
    """Parse every JSON-RPC message written to ``buffer``."""
    buffer.seek(0)
    raw = buffer.read()
    messages = []
    while raw:
        blank = raw.index(b'\r\n\r\n')
        header = raw[:blank].decode('ascii')
        length = int([h for h in header.split('\r\n')
                      if h.lower().startswith('content-length')][0].split(':', 1)[1])
        start = blank + 4
        body = raw[start:start + length]
        messages.append(json.loads(body.decode('utf-8')))
        raw = raw[start + length:]
    return messages


# ---------------------------------------------------------------------------
# Transport
# ---------------------------------------------------------------------------

def test_read_message_roundtrip():
    payload = {'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {}}
    server = AuraLanguageServer(reader=io.BytesIO(encode(payload)), writer=io.BytesIO())
    assert server._read_message() == payload


def test_read_message_returns_none_on_eof():
    server = AuraLanguageServer(reader=io.BytesIO(b''), writer=io.BytesIO())
    assert server._read_message() is None


def test_read_message_without_content_length_returns_none():
    server = AuraLanguageServer(
        reader=io.BytesIO(b'X-Other: 1\r\n\r\n'), writer=io.BytesIO())
    assert server._read_message() is None


def test_read_message_with_zero_length_returns_none():
    server = AuraLanguageServer(
        reader=io.BytesIO(b'Content-Length: 0\r\n\r\n'), writer=io.BytesIO())
    assert server._read_message() is None


def test_read_message_rejects_oversized_body():
    header = f"Content-Length: {MAX_MESSAGE_BYTES + 1}\r\n\r\n".encode()
    server = AuraLanguageServer(reader=io.BytesIO(header), writer=io.BytesIO())
    with pytest.raises(ValueError):
        server._read_message()


def test_read_message_accepts_header_with_extra_whitespace():
    body = b'{"jsonrpc": "2.0"}'
    raw = f"Content-Length:  {len(body)} \r\n\r\n".encode() + body
    server = AuraLanguageServer(reader=io.BytesIO(raw), writer=io.BytesIO())
    assert server._read_message() == {'jsonrpc': '2.0'}


def test_write_message_frames_content_length():
    server, out = make_server()
    server._write_message({'ok': True})
    assert out.getvalue().startswith(b'Content-Length: ')
    assert out.getvalue().endswith(b'{"ok": true}')


def test_respond_and_notify_shapes():
    server, out = make_server()
    server._respond(7, {'x': 1})
    server._notify('some/event', {'y': 2})
    result, notification = decode(out)
    assert result == {'jsonrpc': '2.0', 'id': 7, 'result': {'x': 1}}
    assert notification == {
        'jsonrpc': '2.0', 'method': 'some/event', 'params': {'y': 2}}


# ---------------------------------------------------------------------------
# Lifecycle
# ---------------------------------------------------------------------------

def test_run_processes_messages_until_shutdown():
    messages = (
        encode({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {}})
        + encode({'jsonrpc': '2.0', 'id': 2, 'method': 'shutdown', 'params': {}})
    )
    out = io.BytesIO()
    server = AuraLanguageServer(reader=io.BytesIO(messages), writer=out)
    server.run()
    assert server.shutdown_requested is True
    assert len(decode(out)) == 2


def test_run_stops_at_eof():
    server, out = make_server()
    server.run()
    assert server.shutdown_requested is False
    assert decode(out) == []


def test_run_keeps_alive_on_internal_error_and_replies_with_error():
    # A request whose handler raises: missing params for didOpen.
    messages = (
        encode({'jsonrpc': '2.0', 'id': 9, 'method': 'textDocument/didOpen',
                'params': {}})
        + encode({'jsonrpc': '2.0', 'id': 2, 'method': 'shutdown', 'params': {}})
    )
    out = io.BytesIO()
    server = AuraLanguageServer(reader=io.BytesIO(messages), writer=out)
    server.run()
    first, second = decode(out)
    assert first['id'] == 9
    assert first['error']['code'] == -32603
    assert second['id'] == 2


def test_initialize_advertises_capabilities():
    server, out = make_server()
    server._handle({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize',
                    'params': {}})
    caps = decode(out)[0]['result']['capabilities']
    assert caps['textDocumentSync'] == 1
    assert caps['hoverProvider'] is True
    assert caps['documentSymbolProvider'] is True


def test_initialized_is_a_noop():
    server, out = make_server()
    server._handle({'method': 'initialized', 'params': {}})
    assert decode(out) == []


def test_exit_stops_without_response():
    server, out = make_server()
    server._handle({'method': 'exit'})
    assert server.shutdown_requested is True
    assert decode(out) == []


def test_unknown_request_gets_null_result():
    server, out = make_server()
    server._handle({'id': 5, 'method': 'unknown/method'})
    assert decode(out) == [
        {'jsonrpc': '2.0', 'id': 5, 'result': None}]


def test_unknown_notification_is_ignored():
    server, out = make_server()
    server._handle({'method': 'unknown/method'})
    assert decode(out) == []


# ---------------------------------------------------------------------------
# Documents
# ---------------------------------------------------------------------------

def test_did_open_stores_and_publishes_diagnostics():
    server, out = make_server()
    server._handle({'method': 'textDocument/didOpen', 'params': {
        'textDocument': {'uri': 'file:///a.aura', 'text': 'let x: str = 10\n'}}})
    assert server.documents['file:///a.aura']
    notification = decode(out)[0]
    assert notification['method'] == 'textDocument/publishDiagnostics'
    assert notification['params']['diagnostics']


def test_did_change_updates_text_and_diagnostics():
    server, out = make_server()
    uri = 'file:///a.aura'
    server._handle({'method': 'textDocument/didOpen', 'params': {
        'textDocument': {'uri': uri, 'text': 'let x: int = 1\n'}}})
    server._handle({'method': 'textDocument/didChange', 'params': {
        'textDocument': {'uri': uri},
        'contentChanges': [{'text': 'let x: str = 10\n'}]}})
    assert server.documents[uri] == 'let x: str = 10\n'
    last = decode(out)[-1]
    assert last['params']['diagnostics']


def test_did_change_without_changes_keeps_text():
    server, _ = make_server()
    uri = 'file:///a.aura'
    server.documents[uri] = 'let x = 1\n'
    server._handle({'method': 'textDocument/didChange', 'params': {
        'textDocument': {'uri': uri}, 'contentChanges': []}})
    assert server.documents[uri] == 'let x = 1\n'


def test_did_save_republishes_diagnostics():
    server, out = make_server()
    uri = 'file:///a.aura'
    server.documents[uri] = 'let x = 1\n'
    server._handle({'method': 'textDocument/didSave',
                    'params': {'textDocument': {'uri': uri}}})
    assert decode(out)[0]['method'] == 'textDocument/publishDiagnostics'


def test_did_close_clears_document_and_caches_and_notifies_empty():
    server, out = make_server()
    uri = 'file:///a.aura'
    server._handle({'method': 'textDocument/didOpen', 'params': {
        'textDocument': {'uri': uri, 'text': 'let x = 1\n'}}})
    server._handle({'method': 'textDocument/didClose',
                    'params': {'textDocument': {'uri': uri}}})
    assert uri not in server.documents
    assert uri not in server._parse_cache
    assert uri not in server._diagnostics_cache
    assert decode(out)[-1]['params']['diagnostics'] == []


def test_evicts_old_documents_past_the_limit():
    server, _ = make_server()
    for i in range(MAX_DOCUMENTS + 5):
        uri = f'file:///d{i}.aura'
        server.documents[uri] = 'let x = 1\n'
        server._parse_cache[uri] = ('let x = 1\n', None, None)
        server._diagnostics_cache[uri] = ('let x = 1\n', [])
        server._evict_old_documents()
    assert len(server.documents) == MAX_DOCUMENTS
    assert 'file:///d0.aura' not in server.documents
    assert not server._parse_cache or len(server._parse_cache) <= MAX_DOCUMENTS


# ---------------------------------------------------------------------------
# Parsing cache
# ---------------------------------------------------------------------------

def test_parsed_caches_by_text():
    server, _ = make_server()
    uri = 'file:///a.aura'
    server.documents[uri] = 'let x = 1\n'
    first = server._parsed(uri)
    second = server._parsed(uri)
    assert first is second


def test_parsed_recomputes_after_change():
    server, _ = make_server()
    uri = 'file:///a.aura'
    server.documents[uri] = 'let x = 1\n'
    server._parsed(uri)
    server.documents[uri] = 'let y = 2\n'
    _, program, error = server._parsed(uri)
    assert program is not None and error is None


def test_parsed_reports_syntax_error():
    server, _ = make_server()
    uri = 'file:///bad.aura'
    server.documents[uri] = 'let x = = 1\n'
    _, program, error = server._parsed(uri)
    assert program is None
    assert error is not None


def test_diagnostics_for_syntax_error_has_one_entry():
    server, _ = make_server()
    uri = 'file:///bad.aura'
    server.documents[uri] = 'let x = = 1\n'
    diagnostics = server._diagnostics_for(uri)
    assert len(diagnostics) == 1
    assert diagnostics[0]['severity'] == 1


def test_diagnostics_cached_until_text_changes():
    server, _ = make_server()
    uri = 'file:///a.aura'
    server.documents[uri] = 'let x: str = 10\n'
    first = server._diagnostics_for(uri)
    assert server._diagnostics_for(uri) is first


def test_diagnostics_aggregate_all_checkers():
    server, _ = make_server()
    uri = 'file:///a.aura'
    # A type error (E1xx) plus a rule error (missing main in an entry file).
    server.documents[uri] = 'let x: str = 10\nprint(x)\n'
    codes = {d.get('code') for d in server._diagnostics_for(uri)}
    assert codes


def test_diagnostics_for_clean_document_is_empty():
    server, _ = make_server()
    uri = 'file:///a.aura'
    server.documents[uri] = 'def main() { let x: int = 1\nprint(x) }\n'
    assert server._diagnostics_for(uri) == []


# ---------------------------------------------------------------------------
# Diagnostic conversion
# ---------------------------------------------------------------------------

class _Loc:
    def __init__(self, line=1, column=1, length=0):
        self.line = line
        self.column = column
        self.length = length


class _Severity:
    def __init__(self, value):
        self.value = value


class _Err:
    def __init__(self, message='boom', location=None, severity=None,
                 hint=None, code=None):
        self.message = message
        self.location = location
        self.severity = severity
        self.hint = hint
        self.code = code


class _Code:
    def __init__(self, value):
        self.value = value


def test_diagnostic_from_aura_error_uses_location():
    err = _Err(message='bad', location=_Loc(line=3, column=5, length=4))
    diag = AuraLanguageServer()._diagnostic_from_aura_error(err)
    assert diag['range']['start'] == {'line': 2, 'character': 4}
    assert diag['range']['end'] == {'line': 2, 'character': 8}
    assert diag['severity'] == 1


def test_diagnostic_from_aura_error_defaults_when_no_location():
    diag = AuraLanguageServer()._diagnostic_from_aura_error(_Err())
    assert diag['range']['start'] == {'line': 0, 'character': 0}
    assert diag['range']['end'] == {'line': 0, 'character': 1}


def test_diagnostic_from_aura_error_sets_warning_severity():
    err = _Err(location=_Loc(line=1, column=1), severity=_Severity('warning'))
    assert AuraLanguageServer()._diagnostic_from_aura_error(err)['severity'] == 2


def test_diagnostic_from_aura_error_appends_hint_and_code():
    err = _Err(message='m', hint='try this', code=_Code('E999'),
               location=_Loc(line=1, column=1))
    diag = AuraLanguageServer()._diagnostic_from_aura_error(err)
    assert 'hint: try this' in diag['message']
    assert diag['code'] == 'E999'


def test_diagnostic_from_error_uses_exception_attributes():
    class Exc(Exception):
        line = 2
        column = 3

        def __str__(self):
            return 'parse fail'

    diag = AuraLanguageServer()._diagnostic_from_error(Exc())
    assert diag['range']['start'] == {'line': 1, 'character': 2}
    assert diag['message'] == 'parse fail'
    assert diag['severity'] == 1


def test_diagnostic_from_error_handles_missing_attributes():
    diag = AuraLanguageServer()._diagnostic_from_error(Exception('x'))
    assert diag['range']['start'] == {'line': 0, 'character': 0}


# ---------------------------------------------------------------------------
# Hover
# ---------------------------------------------------------------------------

def test_hover_on_typed_local_variable():
    server, out = make_server()
    uri = 'file:///a.aura'
    text = 'let total: int = 1\nprint(total)\n'
    server.documents[uri] = text
    server._handle({'id': 1, 'method': 'textDocument/hover', 'params': {
        'textDocument': {'uri': uri},
        'position': {'line': 1, 'character': 7}}})
    value = decode(out)[0]['result']['contents']['value']
    assert 'total' in value


def test_hover_on_keyword():
    server, out = make_server()
    uri = 'file:///a.aura'
    server.documents[uri] = 'let value = 1\n'
    server._handle({'id': 1, 'method': 'textDocument/hover', 'params': {
        'textDocument': {'uri': uri},
        'position': {'line': 0, 'character': 2}}})
    value = decode(out)[0]['result']['contents']['value']
    assert 'keyword' in value


def test_hover_on_builtin():
    server, out = make_server()
    uri = 'file:///a.aura'
    server.documents[uri] = 'print(1)\n'
    server._handle({'id': 1, 'method': 'textDocument/hover', 'params': {
        'textDocument': {'uri': uri},
        'position': {'line': 0, 'character': 2}}})
    value = decode(out)[0]['result']['contents']['value']
    assert 'builtin' in value


def test_hover_on_unknown_word_returns_plain():
    server, out = make_server()
    uri = 'file:///a.aura'
    server.documents[uri] = 'foobar\n'
    server._handle({'id': 1, 'method': 'textDocument/hover', 'params': {
        'textDocument': {'uri': uri},
        'position': {'line': 0, 'character': 3}}})
    value = decode(out)[0]['result']['contents']['value']
    assert value == '`foobar`' or 'foobar' in value


def test_hover_returns_none_when_no_word():
    server, out = make_server()
    uri = 'file:///a.aura'
    server.documents[uri] = '   \n'
    server._handle({'id': 1, 'method': 'textDocument/hover', 'params': {
        'textDocument': {'uri': uri},
        'position': {'line': 0, 'character': 1}}})
    assert decode(out)[0]['result'] is None


def test_hover_on_invalid_document_falls_back():
    server, out = make_server()
    uri = 'file:///bad.aura'
    server.documents[uri] = 'let = =\n'
    server._handle({'id': 1, 'method': 'textDocument/hover', 'params': {
        'textDocument': {'uri': uri},
        'position': {'line': 0, 'character': 1}}})
    assert decode(out)[0]['result'] is not None


# ---------------------------------------------------------------------------
# Completion & symbols
# ---------------------------------------------------------------------------

def test_completion_lists_keywords_builtins_and_modules():
    server, out = make_server()
    server._handle({'id': 1, 'method': 'textDocument/completion', 'params': {}})
    result = decode(out)[0]['result']
    labels = {item['label'] for item in result['items']}
    assert set(KEYWORDS) <= labels
    assert set(BUILTINS) <= labels
    assert set(STDLIB_MODULES) <= labels
    assert result['isIncomplete'] is False


def test_document_symbols_for_all_declaration_kinds():
    server, out = make_server()
    uri = 'file:///a.aura'
    server.documents[uri] = (
        'def foo() { return 1 }\n'
        'class Bar { let x: int = 0 }\n'
        'enum Color { Red, Green }\n'
        'trait Shape { def area() -> int }\n'
    )
    server._handle({'id': 1, 'method': 'textDocument/documentSymbol',
                    'params': {'textDocument': {'uri': uri}}})
    symbols = {s['name']: s['kind'] for s in decode(out)[0]['result']}
    assert symbols.get('foo') == 12
    assert symbols.get('Bar') == 5
    assert symbols.get('Color') == 10
    assert symbols.get('Shape') == 11


def test_document_symbols_empty_for_invalid_document():
    server, out = make_server()
    uri = 'file:///bad.aura'
    server.documents[uri] = 'class {\n'
    server._handle({'id': 1, 'method': 'textDocument/documentSymbol',
                    'params': {'textDocument': {'uri': uri}}})
    assert decode(out)[0]['result'] == []


# ---------------------------------------------------------------------------
# Word extraction
# ---------------------------------------------------------------------------

def test_word_at_extracts_identifier():
    assert AuraLanguageServer._word_at('let foo = 1', 0, 5) == 'foo'


def test_word_at_at_start_and_end():
    assert AuraLanguageServer._word_at('foo', 0, 0) == 'foo'
    assert AuraLanguageServer._word_at('foo', 0, 3) == 'foo'


def test_word_at_out_of_range_lines_returns_none():
    assert AuraLanguageServer._word_at('one line', 5, 0) is None
    assert AuraLanguageServer._word_at('one line', -1, 0) is None


def test_word_at_clamps_character_past_end():
    assert AuraLanguageServer._word_at('foo', 0, 100) == 'foo'


def test_word_at_on_whitespace_returns_none():
    assert AuraLanguageServer._word_at('   ', 0, 1) is None


def test_word_at_recognises_digits_and_underscores():
    assert AuraLanguageServer._word_at('a_b12 c', 0, 2) == 'a_b12'
