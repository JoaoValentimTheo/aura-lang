"""Extra combinatorial tests - batch 7f."""
from __future__ import annotations
import pytest
from aura.parser.to_ast import Tokenizer, Parser
from aura.transpiler.transformer import Transformer


def _compile(src):
    tokens = Tokenizer(src).tokenize()
    tree = Parser(tokens).parse()
    code = Transformer().transform(tree)
    compile(code, '<test>', 'exec')
    return code


# Power 8x8
BASES = ['0', '1', '2', '3', '5', '10', '15', '20']
EXPS = ['0', '1', '2', '3', '4', '5', '6', '7']
POWER_BIG = [(f'{b}**{e}', f'def main() {{ let z = {b} ** {e} }}')
             for b in BASES for e in EXPS]


@pytest.mark.parametrize('name,src', POWER_BIG, ids=[i[0] for i in POWER_BIG])
def test_power_big(name, src):
    _compile(src)


# CMP 8x8x6
INTS = ['0', '1', '2', '3', '5', '10', '42', '100']
CMPS = ['==', '!=', '<', '>', '<=', '>=']
CMP_BIG = [(f'{x}{op}{y}', f'def main() {{ let z = {x} {op} {y} }}')
           for x in INTS for op in CMPS for y in INTS]


@pytest.mark.parametrize('name,src', CMP_BIG, ids=[i[0] for i in CMP_BIG])
def test_cmp_big(name, src):
    _compile(src)


# Bool chains
BOOL_CHAINS = [
    ('and_chain', 'true and true and true'),
    ('or_chain', 'false or false or true'),
    ('mixed', 'true and false or true and true'),
    ('not_chain', 'not not not not true'),
    ('complex', '(true or false) and (not false)'),
    ('parens', '(true and false) or (true and true)'),
    ('and_or', 'true and true or false and true'),
    ('or_and', 'true or false and false or true'),
    ('not_and', 'not false and true'),
    ('not_or', 'not true or true'),
]


@pytest.mark.parametrize('name,src', BOOL_CHAINS, ids=[i[0] for i in BOOL_CHAINS])
def test_bool_chains(name, src):
    _compile(f'def main() {{ let x = {src} }}')


# Nested ternary
TERN_NESTED = [
    ('basic', 'true ? (true ? 1 : 2) : 3'),
    ('deep', 'true ? (true ? (true ? 1 : 2) : 3) : 4'),
    ('cond', '1 < 2 ? (3 > 2 ? 10 : 20) : 30'),
    ('mixed', 'true ? (false ? 1 : 2) : (true ? 3 : 4)'),
    ('triple', '(1 < 2) ? ((2 < 3) ? 1 : 2) : 3'),
]


@pytest.mark.parametrize('name,src', TERN_NESTED, ids=[i[0] for i in TERN_NESTED])
def test_ternary_nested(name, src):
    _compile(f'def main() {{ let x = {src} }}')


# Expr arith
EXPR_ARITH = [
    ('double_parens', '(1 + 2) * (3 + 4)'),
    ('mixed_ops', '1 + 2 * 3 - 4'),
    ('power_neg', '(-2) ** 3'),
    ('modulo_chain', '100 % 7 % 3'),
    ('nested_mul', '(2 * (3 + 4)) * 5'),
    ('sub_parens', '(10 - (3 + 2)) * 2'),
    ('deep_parens', '((1 + 2) * (3 + 4)) - ((5 + 6) * (7 + 8))'),
    ('power_mul', '2 ** 3 * 3 ** 2'),
    ('div_parens', '(10 + 5) * (20 - 10)'),
    ('neg_mul', '(-3) * (-5) + (-7) * 2'),
]


@pytest.mark.parametrize('name,src', EXPR_ARITH, ids=[i[0] for i in EXPR_ARITH])
def test_expr_arith(name, src):
    _compile(f'def main() {{ let x = {src} }}')


# Guard complex
GUARD_COMPLEX = [
    ('and', 'def f(x: int) -> int { guard x > 0 and x < 100 else { return -1 } return x }'),
    ('or', 'def f(x: int) -> int { guard x == 0 or x == 1 else { return -1 } return x }'),
    ('not', 'def f(x: int) -> int { guard not (x < 0) else { return -1 } return x }'),
]


@pytest.mark.parametrize('name,src', GUARD_COMPLEX, ids=[i[0] for i in GUARD_COMPLEX])
def test_guard_complex(name, src):
    _compile(src)


# Enum programs
ENUM_PROGS = [
    ('traffic', 'enum Traffic { Red, Yellow, Green }'),
    ('cardinal', 'enum Cardinal { North, South, East, West }'),
    ('planet', 'enum Planet { Mercury, Venus, Earth, Mars, Jupiter }'),
    ('code', 'enum Code { Ok = 200, NotFound = 404, Error = 500 }'),
    ('season', 'enum Season { Spring, Summer, Autumn, Winter }'),
]


@pytest.mark.parametrize('name,src', ENUM_PROGS, ids=[i[0] for i in ENUM_PROGS])
def test_enum_progs(name, src):
    _compile(src)


# Nested functions
NESTED_PROGS = [
    ('simple', 'def f() { def g() { return 1 } return g() }'),
    ('deep', 'def f() { def g() { def h() { return 1 } return h() } return g() }'),
]


@pytest.mark.parametrize('name,src', NESTED_PROGS, ids=[i[0] for i in NESTED_PROGS])
def test_nested_progs(name, src):
    _compile(src)


# Complex programs
COMPLEX_PROGS = [
    ('fib_class', 'class Fib { static def compute(n: int) -> int { if n <= 1 { return n } return Fib.compute(n - 1) + Fib.compute(n - 2) } }'),
    ('safe_div', 'def safe_div(a: int, b: int) -> int { guard b != 0 else { return 0 } return a / b }'),
    ('list_ops', 'def main() { let x = [1, 2, 3, 4, 5]\nlet y = x.filter((n) => n > 2) }'),
    ('class_counter', 'class Counter { public let count: int }'),
    ('match_classify', 'def classify(x: int) -> str { match x {\n  case 0 { "zero" }\n  case 1 { "one" }\n  case _ { "many" }\n} }'),
]


@pytest.mark.parametrize('name,src', COMPLEX_PROGS, ids=[i[0] for i in COMPLEX_PROGS])
def test_complex_progs(name, src):
    _compile(src)


# Default params
DEFP_PROGS = [
    ('int_def', 'def f(x: int = 0) -> int { return x }'),
    ('multi', 'def f(a: int = 0, b: str = "x", c: bool = false) {}'),
    ('varargs', 'def f(*args) {}'),
    ('kwargs', 'def f(**kwargs) {}'),
    ('mixed', 'def f(x: int, *args) {}'),
    ('mixed2', 'def f(x: int, **kwargs) {}'),
    ('all', 'def f(x: int = 0, *args, y: str = "hi", **kwargs) {}'),
]


@pytest.mark.parametrize('name,src', DEFP_PROGS, ids=[i[0] for i in DEFP_PROGS])
def test_default_params(name, src):
    _compile(src)


# LC advanced
LC_ADVANCED = [
    ('cond', '[i * i for i in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] if i % 2 == 0]'),
    ('identity', '[i for i in [1, 2, 3]]'),
    ('transform', '[i * 10 for i in [1, 2, 3]]'),
    ('filter', '[i for i in [1, 2, 3, 4, 5] if i > 2]'),
    ('transform_filter', '[i * 2 for i in [1, 2, 3, 4, 5] if i > 2]'),
]


@pytest.mark.parametrize('name,src', LC_ADVANCED, ids=[i[0] for i in LC_ADVANCED])
def test_lc_advanced(name, src):
    _compile(f'def main() {{ let x = {src} }}')


# For patterns
FOR_PROGS = [
    ('if_inside', 'def main() { let mut c = 0\nfor i in 0..20 { if i % 2 == 0 { c = c + 1 } } }'),
    ('nested', 'def main() { let mut t = 0\nfor i in 0..5 { for j in 0..5 { t = t + 1 } } }'),
]


@pytest.mark.parametrize('name,src', FOR_PROGS, ids=[i[0] for i in FOR_PROGS])
def test_for_progs(name, src):
    _compile(src)


# While patterns
WHILE_PROGS = [
    ('countdown', 'def main() { let mut i = 10\nwhile i > 0 { i = i - 1 } }'),
    ('nested', 'def main() { let mut i = 0\nwhile i < 3 { let mut j = 0\nwhile j < 3 { j = j + 1 }\ni = i + 1 } }'),
]


@pytest.mark.parametrize('name,src', WHILE_PROGS, ids=[i[0] for i in WHILE_PROGS])
def test_while_progs(name, src):
    _compile(src)


# Method chain tests (no quotes in method names, safe)
CHAIN_PROGS = [
    ('list_len', '[1, 2, 3].length'),
    ('list_filter_map', '[1, 2, 3, 4, 5].filter((x) => x > 2).map((x) => x * 10)'),
    ('list_map_filter', '[1, 2, 3, 4, 5].map((x) => x * 2).filter((x) => x > 4)'),
]


@pytest.mark.parametrize('name,src', CHAIN_PROGS, ids=[i[0] for i in CHAIN_PROGS])
def test_chain_progs(name, src):
    _compile(f'def main() {{ let x = {src} }}')


# More negation depth
for d in range(1, 21):
    exec(f'def test_neg_depth_{d}():\n    _compile("def main() {{ let x = {"-" * d}5 }}")\n')


# List multiplication sizes
for sz in range(1, 21):
    exec(f'def test_list_mul_{sz}():\n    _compile("def main() {{ let x = [1, 2] * {sz} }}")\n')


# String concatenation lengths (using int concat for simplicity)
for i in range(2, 21):
    expr = ' + '.join(['1'] * i)
    exec(f'def test_int_concat_{i}():\n    _compile("def main() {{ let x = {expr} }}")\n')
