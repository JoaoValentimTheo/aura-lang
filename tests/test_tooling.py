"""Tests for the CLI tooling: deps, version, debugger and LSP."""
import json
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))


# ============================================================================
# Dependency manifest
# ============================================================================

def test_deps_manifest_roundtrip(tmp_path, monkeypatch):
    from aura.tools import deps

    monkeypatch.chdir(tmp_path)
    assert deps.init_project('demo') == 0
    manifest = tmp_path / 'aura.toml'
    assert manifest.is_file()
    assert (tmp_path / 'src' / 'main.aura').is_file()

    deps.add_package('requests', '>=2.28', manifest_path=manifest, install=False)
    data = deps.load_manifest(manifest)
    assert data['dependencies']['requests'] == '>=2.28'

    deps.add_package('pyyaml', manifest_path=manifest, install=False)
    data = deps.load_manifest(manifest)
    assert data['dependencies']['pyyaml'] == '*'

    # A pip-style specifier embedded in the name is recorded verbatim.
    deps.add_package('flask>=3.0', manifest_path=manifest, install=False)
    data = deps.load_manifest(manifest)
    assert data['dependencies']['flask'] == '>=3.0'


def test_deps_find_manifest_walks_up(tmp_path, monkeypatch):
    from aura.tools import deps

    (tmp_path / 'aura.toml').write_text('[dependencies]\nrequests = "*"\n')
    nested = tmp_path / 'a' / 'b'
    nested.mkdir(parents=True)
    monkeypatch.chdir(nested)
    found = deps.find_manifest()
    assert found == tmp_path / 'aura.toml'


def test_deps_list_dependencies(tmp_path, capsys):
    from aura.tools import deps

    manifest = tmp_path / 'aura.toml'
    manifest.write_text('[dependencies]\nrequests = ">=2.0"\n')
    assert deps.list_dependencies(manifest) == 0
    out = capsys.readouterr().out
    assert 'requests' in out


# ============================================================================
# Version tooling
# ============================================================================

def test_version_get_and_set(tmp_path):
    from aura.tools import release

    pyproject = tmp_path / 'pyproject.toml'
    pyproject.write_text('[project]\nname = "x"\nversion = "1.2.3"\n')
    init = tmp_path / '__init__.py'
    init.write_text('__version__ = "1.2.3"\n')

    assert release.get_version.__module__  # sanity
    release.set_version('2.0.0', pyproject=pyproject, init=init)
    assert 'version = "2.0.0"' in pyproject.read_text()
    assert '__version__ = "2.0.0"' in init.read_text()


def test_version_rejects_invalid(tmp_path):
    from aura.tools import release

    pyproject = tmp_path / 'pyproject.toml'
    pyproject.write_text('[project]\nversion = "1.2.3"\n')
    with pytest.raises(ValueError):
        release.set_version('not-a-version', pyproject=pyproject,
                            init=tmp_path / '__init__.py')


# ============================================================================
# Debugger
# ============================================================================

def test_debugger_reports_uncaught_exception(tmp_path):
    source = tmp_path / 'boom.aura'
    source.write_text("let x = 10\nlet y = 0\nprint(x / y)\n")
    result = subprocess.run(
        [sys.executable, str(ROOT / 'main.py'), 'debug', str(source)],
        capture_output=True, text=True,
    )
    assert result.returncode == 1
    assert 'ZeroDivisionError' in result.stderr


def test_debugger_trace_prints_lines(tmp_path):
    source = tmp_path / 'trace.aura'
    source.write_text("let total = 0\nprint(total)\n")
    result = subprocess.run(
        [sys.executable, str(ROOT / 'main.py'), 'debug', str(source), '--trace'],
        capture_output=True, text=True,
    )
    assert result.returncode == 0
    assert 'trace.aura' in result.stdout


def test_source_map_builds_for_program():
    from aura.tools.debugger import build_source_map
    from aura.parser.to_ast import parse_file

    src = Path(ROOT, 'examples', 'hello.aura')
    ast = parse_file(str(src))
    source_map = build_source_map(ast)
    assert source_map
    assert all(isinstance(v, int) for v in source_map.values())


# ============================================================================
# Language server
# ============================================================================

class _LspClient:
    def __init__(self):
        self.proc = subprocess.Popen(
            [sys.executable, '-c', 'from aura.lsp.server import main; main()'],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, cwd=str(ROOT),
        )

    def send(self, message):
        body = json.dumps(message).encode()
        self.proc.stdin.write(
            f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
        self.proc.stdin.flush()

    def read(self):
        headers = {}
        while True:
            line = self.proc.stdout.readline()
            if not line.strip():
                break
            key, value = line.decode().split(':', 1)
            headers[key.strip().lower()] = value.strip()
        length = int(headers['content-length'])
        return json.loads(self.proc.stdout.read(length))

    def close(self):
        self.proc.terminate()


def test_lsp_initialize_and_diagnostics():
    client = _LspClient()
    try:
        client.send({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {}})
        init = client.read()
        assert init['result']['capabilities']['hoverProvider'] is True

        doc = "let x: str = 10\nprint(x)\n"
        client.send({'jsonrpc': '2.0', 'method': 'textDocument/didOpen',
                     'params': {'textDocument': {'uri': 'file:///t.aura', 'text': doc}}})
        diag = client.read()
        assert diag['method'] == 'textDocument/publishDiagnostics'
        assert diag['params']['diagnostics']
    finally:
        client.close()


def test_lsp_completion_offers_keywords():
    client = _LspClient()
    try:
        client.send({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {}})
        client.read()
        client.send({'jsonrpc': '2.0', 'method': 'textDocument/completion',
                     'params': {'textDocument': {'uri': 'file:///t.aura'},
                                'position': {'line': 0, 'character': 0}}})
        completion = client.read()
        labels = {item['label'] for item in completion['result']['items']}
        assert 'let' in labels
        assert 'stdlib.math' in labels
    finally:
        client.close()


def test_lsp_document_symbols():
    client = _LspClient()
    try:
        client.send({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {}})
        client.read()
        doc = "def foo() { return 1 }\nclass Bar { let x: int = 0 }\n"
        client.send({'jsonrpc': '2.0', 'method': 'textDocument/didOpen',
                     'params': {'textDocument': {'uri': 'file:///t.aura', 'text': doc}}})
        client.read()  # diagnostics notification
        client.send({'jsonrpc': '2.0', 'id': 3, 'method': 'textDocument/documentSymbol',
                     'params': {'textDocument': {'uri': 'file:///t.aura'}}})
        symbols = client.read()
        names = {s['name'] for s in symbols['result']}
        assert 'foo' in names and 'Bar' in names
    finally:
        client.close()