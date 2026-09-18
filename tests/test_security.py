"""Security and hardening regression tests.

Each test pins a concrete hardening fix so it cannot silently regress:

  * TOML injection via newlines in `aura add` package names/versions
  * URL scheme allow-listing and SSRF blocking of private/loopback hosts
  * strict JSON serialization (NaN/Infinity rejected)
  * `os.env` allow-list to avoid dumping every secret
  * `os.listdir`-style APIs leaving deleted paths alone
  * the `aura-ide` workspace path-traversal guard (Python side)
"""
import json
import os
import sys
import tempfile
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from aura.tools import deps  # noqa: E402
from aura.stdlib import http as aura_http  # noqa: E402
from aura.stdlib import json as aura_json  # noqa: E402
from aura.stdlib import os as aura_os  # noqa: E402


# ============================================================================
# TOML injection
# ============================================================================

def test_toml_value_escapes_newlines():
    rendered = deps._toml_value('a\n[dependencies]\nevil = "1.0"')
    assert '\n' not in rendered
    assert '\\n' in rendered


def test_toml_value_escapes_quotes_and_backslashes():
    rendered = deps._toml_value('a"b\\c')
    assert rendered == '"a\\"b\\\\c"'


def test_add_package_rejects_newline_in_name(tmp_path, capsys):
    manifest = tmp_path / 'aura.toml'
    rc = deps.add_package('evil\nname', manifest_path=manifest, install=False)
    assert rc == 2
    assert 'invalid requirement' in capsys.readouterr().err.lower()


def test_add_package_rejects_empty_name(tmp_path, capsys):
    manifest = tmp_path / 'aura.toml'
    rc = deps.add_package('   ', manifest_path=manifest, install=False)
    assert rc == 2


def test_add_package_sanitizes_manifest(tmp_path):
    manifest = tmp_path / 'aura.toml'
    assert deps.add_package('requests>=2.0', manifest_path=manifest,
                            install=False) == 0
    data = deps.load_manifest(manifest)
    assert data['dependencies']['requests'] == '>=2.0'


def test_dependency_name_validation():
    assert deps._valid_dependency_name('requests')
    assert deps._valid_dependency_name('my-pkg.v2')
    assert not deps._valid_dependency_name('bad name')
    assert not deps._valid_dependency_name('bad\nname')
    assert not deps._valid_dependency_name('')


# ============================================================================
# HTTP URL / SSRF hardening
# ============================================================================

@pytest.mark.parametrize('url', [
    'file:///etc/passwd',
    'ftp://example.com/x',
    'data:text/plain,hello',
    'javascript:alert(1)',
])
def test_http_rejects_non_http_schemes(url):
    with pytest.raises(ValueError):
        aura_http._validate_url(url)


@pytest.mark.parametrize('url', [
    'http://127.0.0.1/x',
    'http://localhost/x',
    'http://169.254.169.254/latest/meta-data/',
    'http://10.0.0.1/',
    'http://192.168.1.1/',
    'http://172.16.0.1/',
    'http://[::1]/',
    'http://0.0.0.0/',
])
def test_http_blocks_private_and_local_hosts(url):
    with pytest.raises(ValueError):
        aura_http._validate_url(url)


def test_http_allows_public_urls(monkeypatch):
    # Pin resolution so the test is hermetic (no live DNS dependency).
    monkeypatch.setattr(
        aura_http._socket, 'getaddrinfo',
        lambda *a, **k: [(2, 1, 6, '', ('93.184.216.34', 0))])
    assert aura_http._validate_url('http://example.com/x')
    assert aura_http._validate_url('https://example.com')


def test_http_private_opt_out(monkeypatch):
    monkeypatch.setenv('AURA_HTTP_ALLOW_PRIVATE', '1')
    assert aura_http._validate_url('http://127.0.0.1/x')


# ============================================================================
# SSRF guard hardening
# ============================================================================

@pytest.mark.parametrize('host', [
    '::ffff:127.0.0.1',      # IPv4-mapped IPv6 loopback
    '::ffff:169.254.169.254',  # IPv4-mapped link-local (cloud metadata)
])
def test_http_blocks_ipv4_mapped_ipv6(host):
    assert aura_http._is_blocked_host(host)


def test_http_blocks_unresolvable_host(monkeypatch):
    # Fail closed: a name that cannot be resolved must not be allowed, since
    # the client would resolve it again (to anything) at connect time.
    def _fail(*args, **kwargs):
        raise __import__('socket').gaierror('nodename nor servname provided')

    monkeypatch.setattr(aura_http._socket, 'getaddrinfo', _fail)
    assert aura_http._is_blocked_host('this-does-not-resolve.invalid')


def test_http_blocks_empty_hostname():
    assert aura_http._is_blocked_host('')
    assert aura_http._is_blocked_host(None)


def test_http_blocks_host_resolving_to_no_addresses(monkeypatch):
    monkeypatch.setattr(aura_http._socket, 'getaddrinfo', lambda *a, **k: [])
    assert aura_http._is_blocked_host('weird.invalid')


def test_http_blocks_host_with_unparseable_address(monkeypatch):
    monkeypatch.setattr(
        aura_http._socket, 'getaddrinfo',
        lambda *a, **k: [(2, 1, 6, '', ('not-an-ip', 0))])
    assert aura_http._is_blocked_host('weird.invalid')


def test_http_permits_resolved_public_host(monkeypatch):
    monkeypatch.setattr(
        aura_http._socket, 'getaddrinfo',
        lambda *a, **k: [(2, 1, 6, '', ('93.184.216.34', 0))])
    assert not aura_http._is_blocked_host('example.com')


def test_http_validate_url_blocks_unresolvable(monkeypatch):
    def _fail(*args, **kwargs):
        raise __import__('socket').gaierror('nope')

    monkeypatch.setattr(aura_http._socket, 'getaddrinfo', _fail)
    with pytest.raises(ValueError, match='private or local'):
        aura_http._validate_url('http://cannot-resolve.invalid/x')


def test_http_rejects_empty_and_non_string():
    with pytest.raises(ValueError):
        aura_http._validate_url('')
    with pytest.raises(ValueError):
        aura_http._validate_url(None)


# ============================================================================
# JSON strictness
# ============================================================================

@pytest.mark.parametrize('value', [float('nan'), float('inf'), float('-inf')])
def test_json_dumps_rejects_non_finite(value):
    with pytest.raises(ValueError):
        aura_json.dumps({'x': value})


@pytest.mark.parametrize('value', [float('nan'), float('inf')])
def test_json_dump_rejects_non_finite(tmp_path, value):
    target = tmp_path / 'out.json'
    with pytest.raises(ValueError):
        aura_json.dump({'x': value}, str(target))


def test_json_roundtrip_utf8(tmp_path):
    target = tmp_path / 'out.json'
    aura_json.dump({'name': 'café'}, str(target))
    assert aura_json.load(str(target)) == {'name': 'café'}


def test_json_is_valid_rejects_nan():
    assert aura_json.is_valid('{"a": 1}')
    assert not aura_json.is_valid('{"a": NaN}')


# ============================================================================
# os.env allow-list
# ============================================================================

def test_os_env_allowlist(monkeypatch):
    monkeypatch.setenv('AURA_TEST_SECRET', 'hunter2')
    monkeypatch.setenv('AURA_TEST_PUBLIC', 'yes')
    filtered = aura_os.env(['AURA_TEST_PUBLIC'])
    assert filtered == {'AURA_TEST_PUBLIC': 'yes'}
    assert 'AURA_TEST_SECRET' not in filtered


def test_os_env_full_without_allowlist(monkeypatch):
    monkeypatch.setenv('AURA_TEST_ONE', '1')
    assert aura_os.env()['AURA_TEST_ONE'] == '1'


# ============================================================================
# Filesystem safety helpers
# ============================================================================

def test_io_write_lines_empty_is_empty_file(tmp_path):
    from aura.stdlib import io as aura_io
    target = tmp_path / 'empty.txt'
    aura_io.write_lines(str(target), [])
    assert target.read_text() == ''


def test_io_write_lines_roundtrip(tmp_path):
    from aura.stdlib import io as aura_io
    target = tmp_path / 'lines.txt'
    aura_io.write_lines(str(target), ['a', 'b', 'c'])
    assert target.read_text() == 'a\nb\nc\n'
    assert aura_io.read_lines(str(target)) == ['a', 'b', 'c']


# ============================================================================
# aura_ide workspace sandbox (Python implementation)
# ============================================================================

def test_workspace_rejects_escape(tmp_path):
    pytest.importorskip('aura_ide.backend')
    from aura_ide.backend import Workspace
    ws = Workspace(str(tmp_path))
    with pytest.raises(Exception):
        ws.read('../outside.txt')


def test_workspace_allows_inside(tmp_path):
    pytest.importorskip('aura_ide.backend')
    from aura_ide.backend import Workspace
    ws = Workspace(str(tmp_path))
    ws.write('src/main.aura', 'print(1)')
    assert ws.read('src/main.aura') == 'print(1)'

# ============================================================================
# Redirect re-validation and response size cap
# ============================================================================

def test_http_redirect_handler_revalidates():
    from aura.stdlib.http import _SafeRedirectHandler

    handler = _SafeRedirectHandler()
    # A redirect to a private host must be rejected during redirect_request.
    with pytest.raises(ValueError):
        handler.redirect_request(
            None, None, 302, 'Found', {}, 'http://127.0.0.1/secret')


def test_http_response_size_cap(monkeypatch):
    from aura.stdlib import http as aura_http
    import http.server
    import threading

    monkeypatch.setenv('AURA_HTTP_ALLOW_PRIVATE', '1')
    monkeypatch.setenv('AURA_HTTP_MAX_BYTES', '10')

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_GET(self):
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'x' * 1000)

    server = http.server.HTTPServer(('127.0.0.1', 0), Handler)
    port = server.server_address[1]
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        response = aura_http.get(f'http://127.0.0.1:{port}/big')
        assert len(response.body) <= 10
    finally:
        server.shutdown()
        server.server_close()


# ============================================================================
# Dependency manifest validation
# ============================================================================

def test_install_dependencies_rejects_unsafe_name(tmp_path, capsys):
    from aura.tools import deps
    manifest = tmp_path / 'aura.toml'
    manifest.write_text(
        '[dependencies]\n"bad name" = "1.0"\n', encoding='utf-8')
    rc = deps.install_dependencies(str(manifest))
    assert rc == 2
    assert 'unsafe' in capsys.readouterr().err.lower()


def test_install_dependencies_rejects_pip_option(tmp_path, capsys):
    from aura.tools import deps
    manifest = tmp_path / 'aura.toml'
    manifest.write_text(
        '[dependencies]\nrequests = "--target=/etc"\n', encoding='utf-8')
    rc = deps.install_dependencies(str(manifest))
    assert rc == 2


# ============================================================================
# Version bump with a pre-release suffix
# ============================================================================

def test_release_bump_handles_prerelease(tmp_path):
    from aura.tools import release
    pyproject = tmp_path / 'pyproject.toml'
    init = tmp_path / '__init__.py'
    pyproject.write_text('version = "0.1.0a4"\n', encoding='utf-8')
    init.write_text('__version__ = "0.1.0a4"\n', encoding='utf-8')
    assert release.bump('patch', pyproject=pyproject, init=init) == '0.1.1'
    assert '0.1.1' in pyproject.read_text(encoding='utf-8')
    assert '0.1.1' in init.read_text(encoding='utf-8')


# ============================================================================
# LSP document cap
# ============================================================================

def test_lsp_document_cap_evicts_oldest():
    from aura.lsp.server import AuraLanguageServer, MAX_DOCUMENTS

    server = AuraLanguageServer()
    for i in range(MAX_DOCUMENTS + 20):
        server._handle({'method': 'textDocument/didOpen',
                        'params': {'textDocument': {
                            'uri': f'file:///d{i}.aura', 'text': 'let x = 1\n'}}})
    assert len(server.documents) <= MAX_DOCUMENTS
    # The earliest documents were evicted.
    assert 'file:///d0.aura' not in server.documents


# ============================================================================
# Robustness: recursion and size guards
# ============================================================================

def test_deeply_nested_source_is_a_clean_error():
    from aura.parser.to_ast import Tokenizer, Parser

    source = "let x = " + "(" * 5000 + "1" + ")" * 5000
    with pytest.raises(SyntaxError):
        Parser(Tokenizer(source).tokenize()).parse()


def test_parse_file_rejects_oversized_source(tmp_path, monkeypatch):
    from aura.parser import to_ast
    monkeypatch.setattr(to_ast, 'MAX_SOURCE_BYTES', 16)
    path = tmp_path / 'big.aura'
    path.write_text('let x = 1\n' * 10, encoding='utf-8')
    with pytest.raises(SyntaxError):
        to_ast.parse_file(str(path))


def test_runtime_aliases_can_be_removed():
    from aura.runtime import install_runtime_aliases, uninstall_runtime_aliases
    import sys as _sys

    install_runtime_aliases()
    assert 'stdlib' in _sys.modules
    uninstall_runtime_aliases()
    assert 'stdlib' not in _sys.modules or not _sys.modules['stdlib'].__name__.startswith('aura.')


# ============================================================================
# REPL isolation between chunks
# ============================================================================

def test_repl_class_state_does_not_leak_between_chunks():
    """A class defined in one chunk must not affect the next chunk."""
    from aura.repl.engine import AuraREPL
    from aura.parser.to_ast import Tokenizer, Parser

    repl = AuraREPL(output_func=lambda *a: None)
    assert repl.process("class A { public def helper() { return 1 } }").ok
    # A later, unrelated program must be transformed without leaked state.
    program = Parser(Tokenizer("print([1, 2, 3].length())").tokenize()).parse()
    code = repl.transformer.transform(program)
    assert 'len(' in code


def test_repl_recovers_after_deep_nesting_error():
    from aura.repl.engine import AuraREPL

    repl = AuraREPL(output_func=lambda *a: None)
    result = repl.process("let x = " + "(" * 5000 + "1" + ")" * 5000)
    assert result.ok is False
    # The session is still usable.
    assert repl.process("let y = 1").ok
