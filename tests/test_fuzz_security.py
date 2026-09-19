"""Fuzzing tests for the Aura parser and transpiler.

These tests use Hypothesis to generate random inputs and verify that
the parser and transpiler never crash with unhandled exceptions.
"""
from __future__ import annotations

import hypothesis
from hypothesis import given, settings, HealthCheck
from hypothesis import strategies as st

from aura.parser.to_ast import Tokenizer, Parser
from aura.transpiler.transformer import Transformer


# Strategy for generating random ASCII text that might resemble Aura code
_random_text = st.text(
    alphabet=st.characters(
        blacklist_categories=('Cs',),
        blacklist_characters='\x00',
    ),
    min_size=0,
    max_size=200,
)

# Strategy for generating simple Aura-like tokens
_identifier = st.from_regex(r'[a-zA-Z_][a-zA-Z0-9_]{0,20}', fullmatch=True)
_integer = st.integers(min_value=-10000, max_value=10000).map(str)
_operator = st.sampled_from(['+', '-', '*', '/', '%', '**', '==', '!=', '<', '>', '<=', '>=', 'and', 'or', 'not'])
_keyword = st.sampled_from(['let', 'if', 'else', 'while', 'for', 'return', 'def', 'class', 'true', 'false', 'none'])
_literal = st.sampled_from(['0', '1', '42', '-1', 'true', 'false', 'none', '""', "''"])


def _safe_parse(source):
    """Parse source, catching all expected errors."""
    try:
        tokens = Tokenizer(source).tokenize()
        tree = Parser(tokens).parse()
        return tree
    except SyntaxError:
        return None
    except RecursionError:
        return None
    except Exception:
        # Any other exception is a bug
        raise


def _safe_transform(tree):
    """Transform AST, catching all expected errors."""
    try:
        return Transformer().transform(tree)
    except SyntaxError:
        return None
    except RecursionError:
        return None
    except Exception:
        raise


@given(source=_random_text)
@settings(max_examples=500, suppress_health_check=[HealthCheck.too_slow])
def test_tokenizer_never_crashes(source):
    """Tokenizer must never raise unhandled exceptions."""
    try:
        Tokenizer(source).tokenize()
    except SyntaxError:
        pass  # Expected for invalid input


@given(source=_random_text)
@settings(max_examples=500, suppress_health_check=[HealthCheck.too_slow])
def test_parser_never_crashes(source):
    """Parser must never raise unhandled exceptions on random input."""
    _safe_parse(source)


@given(source=_random_text)
@settings(max_examples=200, suppress_health_check=[HealthCheck.too_slow])
def test_transpiler_never_crashes(source):
    """Transformer must never raise unhandled exceptions."""
    tree = _safe_parse(source)
    if tree is not None:
        _safe_transform(tree)


# Structured fuzzing: generate small valid-ish Aura fragments
_statement = st.one_of(
    st.just(''),
    st.just('let x = 1'),
    st.just('let y = "hello"'),
    st.just('let z = true'),
    st.just('let w = none'),
    st.just('let a = 1 + 2'),
    st.just('let b = 3 * 4'),
    st.just('if true { }'),
    st.just('if false { } else { }'),
    st.just('while false { }'),
    st.just('for i in 0..5 { }'),
    st.just('return 1'),
    st.just('return "hello"'),
    st.just('return true'),
    st.just('return none'),
    st.just('def f() { return 1 }'),
    st.just('def g(x: int) { return x }'),
    st.just('def h(x: int, y: str) { }'),
    st.just('class C { public x: int }'),
    st.just('class D { public def m() { return 1 } }'),
    st.just('enum E { A, B, C }'),
    st.just('match 1 { case 1 { } case _ { } }'),
    st.just('try { } catch { }'),
    st.just('try { } catch { } finally { }'),
    st.just('guard true else { return 1 }'),
    st.just('1 + 2'),
    st.just('3 * 4'),
    st.just('10 / 2'),
    st.just('10 % 3'),
    st.just('2 ** 8'),
    st.just('1 < 2'),
    st.just('1 > 2'),
    st.just('1 == 2'),
    st.just('1 != 2'),
    st.just('true and false'),
    st.just('true or false'),
    st.just('not true'),
    st.just('[1, 2, 3]'),
    st.just('{"a": 1}'),
    st.just('(1, 2)'),
    st.just('{1, 2, 3}'),
    st.just('1 < 2 ? 3 : 4'),
    st.just('f()'),
    st.just('f(1)'),
    st.just('f(1, 2)'),
    st.just('x.length'),
    st.just('[1, 2].map((x) => x)'),
    st.just('[1, 2].filter((x) => x > 1)'),
)


_program = st.lists(_statement, min_size=0, max_size=10).map(lambda stmts: '\n'.join(stmts))


@given(source=_program)
@settings(max_examples=500, suppress_health_check=[HealthCheck.too_slow])
def test_structured_fuzz_parse(source):
    """Parsing structured Aura fragments must not crash."""
    _safe_parse(source)


@given(source=_program)
@settings(max_examples=300, suppress_health_check=[HealthCheck.too_slow])
def test_structured_fuzz_transpile(source):
    """Transforming structured Aura fragments must not crash."""
    tree = _safe_parse(source)
    if tree is not None:
        _safe_transform(tree)


# Edge cases: deeply nested input
@given(depth=st.integers(min_value=0, max_value=50))
@settings(max_examples=100)
def test_deeply_nested_parens(depth):
    """Deeply nested parentheses must not crash (budget enforced)."""
    source = '(' * depth + '1' + ')' * depth
    _safe_parse(source)


@given(depth=st.integers(min_value=0, max_value=50))
@settings(max_examples=100)
def test_deeply_nested_braces(depth):
    """Deeply nested braces must not crash."""
    source = '{ ' * depth + '1' + ' }' * depth
    _safe_parse(source)


@given(depth=st.integers(min_value=0, max_value=30))
@settings(max_examples=100)
def test_deeply_nested_if(depth):
    """Deeply nested if statements must not crash."""
    source = 'if true { ' * depth + '1' + ' }' * depth
    _safe_parse(source)


# Edge cases: very long identifiers
@given(length=st.integers(min_value=1, max_value=500))
@settings(max_examples=50)
def test_long_identifier(length):
    """Very long identifiers must be handled."""
    source = f"let {'a' * length} = 1"
    _safe_parse(source)


# Edge cases: many newlines
@given(count=st.integers(min_value=0, max_value=200))
@settings(max_examples=50)
def test_many_newlines(count):
    """Many blank lines must not crash."""
    source = '\n' * count + 'let x = 1'
    _safe_parse(source)


# Edge cases: unicode in identifiers
@given(prefix=st.sampled_from(['', '_']))
@settings(max_examples=50)
def test_unicode_identifier(prefix):
    """Unicode identifiers must not crash."""
    source = f"let {prefix}\u00e9 = 1"
    _safe_parse(source)


# Edge cases: empty and near-empty inputs
@given(source=st.sampled_from([
    '',
    ' ',
    '\n',
    '\t',
    '#',
    '//',
    '/*',
    '*/',
    'let',
    'def',
    'class',
    'if',
    'else',
    'while',
    'for',
    'return',
    'true',
    'false',
    'none',
    '()',
    '{}',
    '[]',
    '""',
    "''",
    '0',
    '1',
    '-1',
    '+',
    '-',
    '*',
    '/',
    '%',
    '**',
    '==',
    '!=',
    '<',
    '>',
    '<=',
    '>=',
    'and',
    'or',
    'not',
    'import',
    'module',
    'export',
    'trait',
    'enum',
    'match',
    'case',
    'guard',
    'try',
    'catch',
    'finally',
    'throw',
    'new',
    'self',
    'super',
    'static',
    'abstract',
    'extends',
    'volatile',
    'async',
    'await',
    'yield',
    'lambda',
    'const',
    'mut',
    'public',
    'private',
    'protected',
]))
@settings(max_examples=200)
def test_edge_cases_parse(source):
    """Edge case inputs must not crash the parser."""
    _safe_parse(source)
