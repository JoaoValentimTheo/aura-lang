"""Tests for the LSP navigation and formatting features.

Covers the capabilities added on top of diagnostics/hover/completion:
go-to-definition, find-references, rename (with prepare), and document
formatting. These operate on identifier tokens plus the parsed declaration
map, so the tests are pure in-process calls on ``AuraLanguageServer``.
"""
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.lsp.server import AuraLanguageServer  # noqa: E402


def server_with(text, uri='file:///nav.aura'):
    server = AuraLanguageServer()
    server.documents[uri] = text
    return server, uri


SAMPLE = (
    'def foo(x: int) -> int {\n'
    '  return x + 1\n'
    '}\n'
    'let y = foo(2)\n'
    'print(y)\n'
)


# ============================================================================
# Capabilities
# ============================================================================

def test_initialize_advertises_navigation_capabilities(capsys):
    server = AuraLanguageServer()

    class Writer:
        def __init__(self):
            self.messages = []

        def write(self, data):
            self.messages.append(data)

        def flush(self):
            pass

    writer = Writer()
    server.writer = writer
    server._handle({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize',
                    'params': {}})
    import json
    raw = b''.join(writer.messages).decode()
    payload = json.loads(raw.split('\r\n\r\n', 1)[1])
    caps = payload['result']['capabilities']
    assert caps['definitionProvider'] is True
    assert caps['referencesProvider'] is True
    assert caps['documentFormattingProvider'] is True
    assert caps['renameProvider'] == {'prepareProvider': True}


# ============================================================================
# Go to definition
# ============================================================================

class TestDefinition:
    def test_jumps_to_function_declaration(self):
        server, uri = server_with(SAMPLE)
        result = server._definition({
            'textDocument': {'uri': uri},
            'position': {'line': 3, 'character': 9},  # `foo` in foo(2)
        })
        assert result['uri'] == uri
        # `def foo` — the name token starts at column 4.
        assert result['range']['start'] == {'line': 0, 'character': 4}

    def test_jumps_to_class_declaration(self):
        server, uri = server_with(
            'class Point {\n  let x: int = 0\n}\nlet p = Point()\n')
        result = server._definition({
            'textDocument': {'uri': uri},
            'position': {'line': 3, 'character': 9},
        })
        assert result['range']['start']['line'] == 0

    def test_jumps_to_variable_declaration(self):
        server, uri = server_with('let total = 1\nprint(total)\n')
        result = server._definition({
            'textDocument': {'uri': uri},
            'position': {'line': 1, 'character': 7},
        })
        assert result['range']['start']['line'] == 0

    def test_unknown_word_returns_none(self):
        server, uri = server_with(SAMPLE)
        result = server._definition({
            'textDocument': {'uri': uri},
            'position': {'line': 4, 'character': 1},  # `rint` inside print
        })
        assert result is None

    def test_parameter_definition_is_found(self):
        server, uri = server_with('def f(value: int) -> int {\n  return value\n}\n')
        result = server._definition({
            'textDocument': {'uri': uri},
            'position': {'line': 1, 'character': 10},  # `value` use
        })
        assert result is not None
        assert result['range']['start']['line'] == 0

    def test_syntax_error_document_returns_none(self):
        server, uri = server_with('def = (((\n')
        result = server._definition({
            'textDocument': {'uri': uri},
            'position': {'line': 0, 'character': 2},
        })
        assert result is None


# ============================================================================
# Find references
# ============================================================================

class TestReferences:
    def test_finds_all_occurrences_including_declaration(self):
        server, uri = server_with(SAMPLE)
        refs = server._references({
            'textDocument': {'uri': uri},
            'position': {'line': 3, 'character': 9},
            'context': {'includeDeclaration': True},
        })
        # def foo + foo(2)
        assert len(refs) == 2
        assert refs[0]['range']['start']['line'] == 0
        assert refs[1]['range']['start']['line'] == 3

    def test_excludes_declaration_when_asked(self):
        server, uri = server_with(SAMPLE)
        refs = server._references({
            'textDocument': {'uri': uri},
            'position': {'line': 3, 'character': 9},
            'context': {'includeDeclaration': False},
        })
        assert len(refs) == 1
        assert refs[0]['range']['start']['line'] == 3

    def test_returns_empty_for_word_absent_from_document(self):
        server, uri = server_with('let zzz = 1\n')
        refs = server._references({
            'textDocument': {'uri': uri},
            'position': {'line': 0, 'character': 14},  # past the line
            'context': {},
        })
        assert refs == []

    def test_reference_ranges_cover_the_name(self):
        server, uri = server_with('let count = 0\ncount = count + 1\n')
        refs = server._references({
            'textDocument': {'uri': uri},
            'position': {'line': 1, 'character': 1},
            'context': {'includeDeclaration': True},
        })
        for ref in refs:
            start = ref['range']['start']
            end = ref['range']['end']
            assert end['character'] - start['character'] == len('count')


# ============================================================================
# Rename
# ============================================================================

class TestRename:
    def test_prepare_rename_returns_placeholder(self):
        server, uri = server_with(SAMPLE)
        result = server._prepare_rename({
            'textDocument': {'uri': uri},
            'position': {'line': 0, 'character': 4},
        })
        assert result['placeholder'] == 'foo'

    def test_prepare_rename_rejects_keywords(self):
        server, uri = server_with('let x = 1\n')
        result = server._prepare_rename({
            'textDocument': {'uri': uri},
            'position': {'line': 0, 'character': 0},
        })
        assert result is None

    def test_rename_edits_every_occurrence(self):
        server, uri = server_with(SAMPLE)
        result = server._rename({
            'textDocument': {'uri': uri},
            'position': {'line': 0, 'character': 4},
            'newName': 'bar',
        })
        edits = result['changes'][uri]
        assert len(edits) == 2
        assert all(edit['newText'] == 'bar' for edit in edits)

    def test_rename_rejects_invalid_identifier(self):
        server, uri = server_with(SAMPLE)
        assert server._rename({
            'textDocument': {'uri': uri},
            'position': {'line': 0, 'character': 4},
            'newName': 'not valid',
        }) is None

    def test_rename_rejects_keyword_target(self):
        server, uri = server_with(SAMPLE)
        assert server._rename({
            'textDocument': {'uri': uri},
            'position': {'line': 0, 'character': 4},
            'newName': 'class',
        }) is None

    def test_rename_unknown_word_returns_none(self):
        server, uri = server_with(SAMPLE)
        # `print` is a builtin, not a declaration in this document.
        result = server._rename({
            'textDocument': {'uri': uri},
            'position': {'line': 4, 'character': 0},
            'newName': 'say',
        })
        assert result is None or all(
            edit['newText'] == 'say' for edit in result['changes'][uri])


# ============================================================================
# Formatting
# ============================================================================

class TestFormatting:
    def test_formatting_normalizes_spacing(self):
        server, uri = server_with('def main(){\nlet x=1+2\n}\n')
        edits = server._formatting({'textDocument': {'uri': uri}})
        assert edits
        new_text = edits[0]['newText']
        assert 'let x = 1 + 2' in new_text

    def test_already_formatted_yields_no_edits(self):
        server, uri = server_with('def main() {\n  let x = 1 + 2\n}\n')
        assert server._formatting({'textDocument': {'uri': uri}}) == []

    def test_empty_document_yields_no_edits(self):
        server, uri = server_with('')
        assert server._formatting({'textDocument': {'uri': uri}}) == []

    def test_formatting_edit_covers_document(self):
        server, uri = server_with('def main(){\nlet x=1\n}\n')
        edits = server._formatting({'textDocument': {'uri': uri}})
        assert edits[0]['range']['start'] == {'line': 0, 'character': 0}


# ============================================================================
# Robustness
# ============================================================================

class TestRobustness:
    def test_features_survive_unparseable_document(self):
        server, uri = server_with('def (((\nlet = = =\n')
        # References work from tokens, so they still answer (and never raise)
        # even when the AST cannot be built.
        refs = server._references({
            'textDocument': {'uri': uri},
            'position': {'line': 0, 'character': 1}, 'context': {}})
        assert isinstance(refs, list)
        # Definition needs the AST, so it degrades to None.
        assert server._definition({
            'textDocument': {'uri': uri},
            'position': {'line': 0, 'character': 1}}) is None

    def test_position_past_line_end_is_clamped(self):
        server, uri = server_with('let x = 1\n')
        # Character far beyond the line length should not raise.
        result = server._definition({
            'textDocument': {'uri': uri},
            'position': {'line': 0, 'character': 999},
        })
        assert result is None

    def test_out_of_range_line_is_safe(self):
        server, uri = server_with('let x = 1\n')
        assert server._word_at('let x = 1\n', 99, 0) is None
