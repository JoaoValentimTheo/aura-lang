"""Regression coverage for the internal audit hardening pass.

Each test targets a specific bug fixed in 0.1.0a17: empty control-flow bodies,
`try`/`finally`, uppercase-identifier conditions, reserved binding names,
unterminated literals, manifest preservation, specifier validation, LSP
protocol robustness, formatter fidelity, recursion budgeting and the tooling
error paths.
"""
import io
import json
import os
import tempfile
import textwrap
from pathlib import Path

import pytest

from aura.parser.to_ast import Parser, Tokenizer, parse_file
from aura.transpiler.transformer import Transformer


def gen(source):
    ast = Parser(Tokenizer(textwrap.dedent(source).lstrip()).tokenize()).parse()
    return Transformer().transform(ast)


def compiles(source):
    code = gen(source)
    compile(code, '<test>', 'exec')
    return code


# ---------------------------------------------------------------------------
# Empty control-flow bodies
# ---------------------------------------------------------------------------

@pytest.mark.parametrize('source', [
    'if True { }',
    'while Flag { }',
    'for x in Items { }',
    'unless Done { }',
    'until Done { }',
    'loop { }',
    'def f() { }',
    'try { } catch { }',
    'try { } finally { }',
    'try { } catch { } finally { }',
    'if a { } else { }',
    'match x { case 1 { } }',
    'guard Ready else { }',
])
def test_empty_body_compiles(source):
    assert 'pass' in compiles(source)


def test_try_finally_keeps_finally_block():
    code = compiles('try { print(1) } finally { print(2) }')
    assert 'finally:' in code
    assert 'print(2)' in code


# ---------------------------------------------------------------------------
# Uppercase-identifier conditions
# ---------------------------------------------------------------------------

@pytest.mark.parametrize('source,expected', [
    ('if Point { print(1) }', 'if Point:'),
    ('while Flag { print(1) }', 'while Flag:'),
    ('for x in Items { print(x) }', 'for x in Items:'),
    ('guard Ready else { return }', 'if not (Ready):'),
])
def test_uppercase_condition_is_not_struct_init(source, expected):
    code = compiles(source)
    assert expected in code


def test_struct_init_still_works():
    code = compiles('let p = Point{x: 1, y: 2}')
    assert 'Point(' in code


# ---------------------------------------------------------------------------
# Binding-name diagnostics
# ---------------------------------------------------------------------------

@pytest.mark.parametrize('name', ['yield', 'match', 'if', 'try', 'not', 'while'])
def test_reserved_keyword_binding_is_rejected(name):
    with pytest.raises(SyntaxError) as excinfo:
        Parser(Tokenizer(f"let {name} = 1\n").tokenize()).parse()
    assert 'reserved keyword' in str(excinfo.value)


@pytest.mark.parametrize('name,alias', [
    ('True', 'true'), ('False', 'false'), ('None', 'none'),
])
def test_python_literal_binding_is_rejected(name, alias):
    with pytest.raises(SyntaxError) as excinfo:
        Parser(Tokenizer(f"let {name} = 1\n").tokenize()).parse()
    assert alias in str(excinfo.value)


def test_const_reserved_name_is_rejected():
    with pytest.raises(SyntaxError):
        Parser(Tokenizer('const yield = 1\n').tokenize()).parse()


# ---------------------------------------------------------------------------
# Unterminated literals
# ---------------------------------------------------------------------------

@pytest.mark.parametrize('source,needle', [
    ('let x = "abc', 'unterminated string'),
    ("let x = 'abc", 'unterminated string'),
    ('let x = r"abc', 'unterminated string'),
    ('let x = """abc', 'unterminated string'),
    ('/* unterminated\nlet x = 1', 'unterminated block comment'),
])
def test_unterminated_literals_raise(source, needle):
    with pytest.raises(SyntaxError) as excinfo:
        Parser(Tokenizer(source).tokenize()).parse()
    assert needle in str(excinfo.value)


def test_raw_string_with_escaped_quote_scans():
    code = gen(r'let x = r"a\"b"')
    compile(code, '<test>', 'exec')


# ---------------------------------------------------------------------------
# Recursion budget
# ---------------------------------------------------------------------------

def test_recursion_budget_restores_limit():
    import sys

    from aura.transpiler.errors import recursion_budget

    before = sys.getrecursionlimit()
    with recursion_budget(10000):
        assert sys.getrecursionlimit() >= before
    assert sys.getrecursionlimit() == before


def test_long_operator_chain_compiles():
    source = "let x = " + " + ".join(["1"] * 1200) + "\n"
    with tempfile.NamedTemporaryFile('w', suffix='.aura', delete=False) as handle:
        handle.write(source)
        path = handle.name
    try:
        ast = parse_file(path)
        from aura.transpiler.errors import recursion_budget
        with recursion_budget(os.path.getsize(path)):
            Transformer().transform(ast)
    finally:
        os.unlink(path)


# ---------------------------------------------------------------------------
# Manifest preservation and specifier validation
# ---------------------------------------------------------------------------

def test_manifest_preserves_unknown_tables_and_values():
    import tomllib

    from aura.tools import deps

    source = (
        '[project]\nname = "app"\nversion = "0.1.0"\n\n'
        '[tool.pytest]\naddopts = "-q"\nmarkers = ["slow"]\n\n'
        '[scripts]\nrun = "python main.py"\n\n'
        '[dependencies]\nrequests = { version = ">=2", optional = true }\n'
        'flask = ["a", "b"]\n'
    )
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, 'aura.toml')
        with open(path, 'w', encoding='utf-8') as handle:
            handle.write(source)
        data = deps.load_manifest(path)
        data['dependencies']['pyyaml'] = '>=6'
        deps._dump_manifest(data, path)
        with open(path, 'rb') as handle:
            parsed = tomllib.load(handle)
        assert parsed['tool']['pytest']['addopts'] == '-q'
        assert parsed['tool']['pytest']['markers'] == ['slow']
        assert parsed['scripts']['run'] == 'python main.py'
        assert parsed['dependencies']['requests'] == {
            'version': '>=2', 'optional': True}
        assert parsed['dependencies']['flask'] == ['a', 'b']
        assert parsed['dependencies']['pyyaml'] == '>=6'


@pytest.mark.parametrize('spec', ['==--upgrade', '==--no-deps', '==-1',
                                  '>=-1,--target=/tmp/evil', '1.0;rm -rf /'])
def test_invalid_specifier_rejected(spec):
    from aura.tools.deps import _valid_specifier

    assert not _valid_specifier(spec)


@pytest.mark.parametrize('spec', ['2.28', '>=2', '~=1.2', '!=1.0', '1.2.3',
                                  '2.0rc1', '1.0.post1', '*'])
def test_valid_specifier_accepted(spec):
    from aura.tools.deps import _valid_specifier

    assert _valid_specifier(spec)


def test_add_package_rejects_bad_version(monkeypatch, capsys):
    from aura.tools import deps

    with tempfile.TemporaryDirectory() as tmp:
        code = deps.add_package('requests', version='--upgrade',
                                install=False, root=Path(tmp))
        assert code == 2
        assert 'invalid version' in capsys.readouterr().err


# ---------------------------------------------------------------------------
# LSP protocol robustness
# ---------------------------------------------------------------------------

def test_lsp_survives_bad_content_length():
    from aura.lsp.server import AuraLanguageServer

    out = io.BytesIO()
    server = AuraLanguageServer(
        reader=io.BytesIO(b'Content-Length: abc\r\n\r\n{}'), writer=out)
    server.run()
    assert b'-32700' in out.getvalue()


def test_lsp_survives_bad_json():
    from aura.lsp.server import AuraLanguageServer

    body = b'not json'
    raw = b'Content-Length: %d\r\n\r\n' % len(body) + body
    out = io.BytesIO()
    server = AuraLanguageServer(reader=io.BytesIO(raw), writer=out)
    server.run()
    assert b'-32700' in out.getvalue()


def test_lsp_requires_initialize():
    from aura.lsp.server import AuraLanguageServer

    payload = json.dumps({'jsonrpc': '2.0', 'id': 1,
                          'method': 'textDocument/hover'}).encode()
    raw = b'Content-Length: %d\r\n\r\n' % len(payload) + payload
    out = io.BytesIO()
    server = AuraLanguageServer(reader=io.BytesIO(raw), writer=out)
    server.run()
    assert b'-32002' in out.getvalue()


# ---------------------------------------------------------------------------
# Formatter fidelity
# ---------------------------------------------------------------------------

@pytest.mark.parametrize('source,expected', [
    ('f(*a, **b)', 'f(*a, **b)'),
    ('let xs = [*a, *b]', 'let xs = [*a, *b]'),
    ('def f(*args, **kw) { }', 'def f(*args, **kw) { }'),
    ('let d = {**base, "k": 1}', 'let d = {**base, "k": 1}'),
    ('let p = a ** b', 'let p = a ** b'),
    ('let z = a * b', 'let z = a * b'),
])
def test_formatter_preserves_spread_and_power(source, expected):
    from aura.tools.formatter import format_aura

    assert format_aura(source) == expected


def test_formatter_preserves_multiline_string():
    from aura.tools.formatter import format_aura

    source = 'let doc = """\nsome = text\na  b   c\n"""\nlet x=1\n'
    out = format_aura(source)
    assert 'a  b   c' in out


# ---------------------------------------------------------------------------
# Tooling error paths
# ---------------------------------------------------------------------------

def test_debug_missing_file_reports_cleanly(tmp_path, capsys):
    from aura.cli import cmd_debug

    code = cmd_debug(str(tmp_path / 'missing.aura'))
    assert code == 2
    assert 'File not found' in capsys.readouterr().err


def test_version_scope_only_project_table(tmp_path):
    from aura.tools import release

    path = tmp_path / 'pyproject.toml'
    path.write_text(
        '[tool.x]\nversion = "9.9.9"\n\n[project]\nname = "a"\n'
        'version = "0.1.0"\n', encoding='utf-8')
    assert release.get_version(path) == '0.1.0'
    release.set_version('0.2.0', pyproject=path, init=tmp_path / 'none.py')
    text = path.read_text(encoding='utf-8')
    assert 'version = "9.9.9"' in text
    assert 'version = "0.2.0"' in text


def test_unsafe_venv_dir_refused(monkeypatch, capsys):
    from aura.tools import deps

    monkeypatch.setenv('AURA_VENV', '/')
    assert deps.remove_venv(confirm=False) == 2
    assert 'Refusing' in capsys.readouterr().err


# ---------------------------------------------------------------------------
# CLI run argument routing
# ---------------------------------------------------------------------------

def test_run_args_route_options_and_program_args():
    from aura.cli import _parse_run_args

    assert _parse_run_args(['run', 'f.aura', '-v']) == ('f.aura', True, False, [])
    assert _parse_run_args(['run', 'f.aura', 'a', 'b']) == (
        'f.aura', False, False, ['a', 'b'])
    assert _parse_run_args(['run', 'f.aura', '--', '-x']) == (
        'f.aura', False, False, ['-x'])
    assert _parse_run_args(['run', '--no-main', 'f.aura', 'x']) == (
        'f.aura', False, True, ['x'])
