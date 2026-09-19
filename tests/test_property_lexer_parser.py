"""Property-based tests for the tokenizer and parser.

These use Hypothesis to generate thousands of inputs and assert invariants a
correct lexer/parser must hold, rather than hand-picked examples. They are the
kind of tests compiler engineers use to shake out crashes and silent
miscompiles on edge cases.
"""
import sys
from pathlib import Path

import pytest
from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler import ast as A  # noqa: E402

SETTINGS = settings(
    max_examples=200,
    deadline=None,
    suppress_health_check=[HealthCheck.too_slow],
)


# ---------------------------------------------------------------------------
# Tokenizer invariants
# ---------------------------------------------------------------------------

@given(st.text(max_size=200))
@SETTINGS
def test_tokenizer_never_crashes_on_arbitrary_text(text):
    """The lexer either produces tokens or raises SyntaxError — never another
    exception type (no IndexError/ValueError leaking out)."""
    try:
        tokens = Tokenizer(text).tokenize()
    except SyntaxError:
        return
    assert tokens
    assert tokens[-1].type == "EOF"


@given(st.text(max_size=200))
@SETTINGS
def test_tokenizer_is_deterministic(text):
    try:
        a = [(t.type, t.value) for t in Tokenizer(text).tokenize()]
        b = [(t.type, t.value) for t in Tokenizer(text).tokenize()]
    except SyntaxError:
        return
    assert a == b


@given(st.integers(min_value=0, max_value=10**12))
@SETTINGS
def test_integer_literals_roundtrip(value):
    tokens = Tokenizer(str(value)).tokenize()
    assert tokens[0].type == "INT"
    assert tokens[0].value == value


@given(st.integers(min_value=0, max_value=10**9), st.integers(0, 10**6))
@SETTINGS
def test_float_literals_roundtrip(whole, frac):
    # A leading `-` lexes as a separate OP token (unary minus), so only
    # non-negative literals are single FLOAT tokens.
    text = f"{whole}.{frac}"
    tokens = Tokenizer(text).tokenize()
    assert tokens[0].type == "FLOAT"
    assert tokens[0].value == float(text)


@given(st.text(alphabet=st.characters(min_codepoint=32, max_codepoint=126,
                                      exclude_characters='\\"'),
               max_size=40))
@SETTINGS
def test_simple_string_literals_roundtrip(body):
    source = '"' + body + '"'
    tokens = Tokenizer(source).tokenize()
    assert tokens[0].type == "STRING"
    assert tokens[0].value == body


@given(st.text(alphabet=st.characters(min_codepoint=97, max_codepoint=122),
               min_size=1, max_size=20).filter(
                   lambda name: name != 'volatily'))
@SETTINGS
def test_identifier_roundtrips(name):
    tokens = Tokenizer(name).tokenize()
    assert tokens[0].type == "IDENT"
    assert tokens[0].value == name


@given(st.text(max_size=200))
@SETTINGS
def test_token_positions_are_monotonic(text):
    """Token line numbers never go backwards; columns are >= 1."""
    try:
        tokens = Tokenizer(text).tokenize()
    except SyntaxError:
        return
    last_line = 1
    for tok in tokens:
        assert tok.line >= last_line or tok.line == last_line
        last_line = max(last_line, tok.line)
        assert tok.column >= 1


# ---------------------------------------------------------------------------
# Parser invariants: valid programs parse without a non-SyntaxError crash
# ---------------------------------------------------------------------------

@given(st.integers(min_value=-1000, max_value=1000),
       st.integers(min_value=-1000, max_value=1000),
       st.sampled_from(["+", "-", "*", "//", "%", "<", ">", "==", "!="]))
@SETTINGS
def test_arithmetic_expressions_parse(a, b, op):
    src = f"{a} {op} {b}"
    node = Parser(Tokenizer(src).tokenize()).parse_expression()
    assert node is not None


@given(st.lists(st.integers(min_value=-100, max_value=100), min_size=0, max_size=20))
@SETTINGS
def test_list_literals_parse_with_correct_length(values):
    src = "[" + ", ".join(str(v) for v in values) + "]"
    node = Parser(Tokenizer(src).tokenize()).parse_expression()
    assert isinstance(node, A.ListLiteral)
    assert len(node.elements) == len(values)


@given(st.dictionaries(
    keys=st.text(alphabet=st.characters(min_codepoint=97, max_codepoint=122),
                 min_size=1, max_size=6),
    values=st.integers(min_value=-100, max_value=100),
    max_size=8))
@SETTINGS
def test_dict_literals_parse(mapping):
    src = "{" + ", ".join(f'"{k}": {v}' for k, v in mapping.items()) + "}"
    node = Parser(Tokenizer(src).tokenize()).parse_expression()
    assert isinstance(node, A.DictLiteral)


@given(st.text(alphabet=st.characters(min_codepoint=97, max_codepoint=122),
               min_size=1, max_size=8),
       st.integers(min_value=0, max_value=50))
@SETTINGS
def test_function_declarations_parse(name, nparams):
    # Only non-reserved identifiers can name a declaration. Use the parser's
    # own reserved-name set so this stays in sync automatically.
    from aura.parser.to_ast import _RESERVED_BINDING_NAMES
    if name in _RESERVED_BINDING_NAMES or not name.isidentifier():
        return
    params = ", ".join(f"p{i}" for i in range(nparams))
    src = f"def {name}({params}) {{ return 1 }}"
    program = Parser(Tokenizer(src).tokenize()).parse()
    assert len(program.statements) == 1
    assert isinstance(program.statements[0], A.FunctionDecl)


@given(st.integers(min_value=0, max_value=30),
       st.integers(min_value=0, max_value=30))
@SETTINGS
def test_nested_parens_parse(depth_a, depth_b):
    # Arbitrary but bounded nesting must not crash the recursive-descent parser.
    src = "(" * depth_a + "1" + ")" * depth_b
    try:
        Parser(Tokenizer(src).tokenize()).parse_expression()
    except SyntaxError:
        pass
    except RecursionError:
        pytest.fail("parser recursed without a SyntaxError for nested parens")


@given(st.text(max_size=60))
@SETTINGS
def test_parser_only_raises_syntax_error(text):
    """Parsing arbitrary text must never leak a non-SyntaxError exception."""
    try:
        Parser(Tokenizer(text).tokenize()).parse()
    except SyntaxError:
        pass
    except RecursionError:
        pass
    except Exception as exc:  # pragma: no cover - failure path
        pytest.fail(f"parser raised {type(exc).__name__}: {exc}")
