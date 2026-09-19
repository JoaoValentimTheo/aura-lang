"""Expression exhaustive tests - batch 4."""
from __future__ import annotations

import pytest

from aura.parser.to_ast import Tokenizer, Parser
from aura.transpiler.transformer import Transformer


def _compile(src):
    tokens = Tokenizer(src).tokenize()
    tree = Parser(tokens).parse()
    code = Transformer().transform(tree)
    compile(code, "<test>", "exec")
    return code


# ---------------------------------------------------------------------------
# Arithmetic operators
# ---------------------------------------------------------------------------
ARITH_OPS = ["+", "-", "*", "**", "//", "%"]
ARITH_PAIRS = [(a, op, b) for a in ["0", "1", "2", "5", "10", "-3", "100"]
               for op in ARITH_OPS
               for b in ["0", "1", "2", "3", "7", "10", "-2"]]

@pytest.mark.parametrize("a,op,b", ARITH_PAIRS,
                         ids=[f"{a}{op}{b}" for a, op, b in ARITH_PAIRS])
def test_arith_expr(a, op, b):
    _compile(f"let x = {a} {op} {b}")


# ---------------------------------------------------------------------------
# Comparison operators
# ---------------------------------------------------------------------------
CMP_OPS = ["==", "!=", "<", ">", "<=", ">="]
CMP_PAIRS = [(a, op, b) for a in ["0", "1", "5", "10"]
             for op in CMP_OPS
             for b in ["0", "1", "5", "10"]]

@pytest.mark.parametrize("a,op,b", CMP_PAIRS,
                         ids=[f"{a}{op}{b}" for a, op, b in CMP_PAIRS])
def test_comparison_expr(a, op, b):
    _compile(f"let x = {a} {op} {b}")


# ---------------------------------------------------------------------------
# Logical operators
# ---------------------------------------------------------------------------
BOOL_VALS = ["true", "false"]
LOGIC_COMBOS = [(a, op, b) for a in BOOL_VALS
                for op in ["and", "or"]
                for b in BOOL_VALS]

@pytest.mark.parametrize("a,op,b", LOGIC_COMBOS,
                         ids=[f"{a}_{op}_{b}" for a, op, b in LOGIC_COMBOS])
def test_logic_expr(a, op, b):
    _compile(f"let x = {a} {op} {b}")


# ---------------------------------------------------------------------------
# Unary operators
# ---------------------------------------------------------------------------
UNARY_OPS = [
    ("neg", "-"),
    ("not", "not "),
    ("double_neg", "--"),
    ("triple_neg", "---"),
]

@pytest.mark.parametrize("name,op", UNARY_OPS, ids=[i[0] for i in UNARY_OPS])
def test_unary_op(name, op):
    _compile(f"let x = {op}5")


# ---------------------------------------------------------------------------
# Bitwise operators
# ---------------------------------------------------------------------------
BIT_OPS = ["&", "|", "^", "<<", ">>"]
BIT_PAIRS = [(a, op, b) for a in ["0", "1", "3", "7", "15"]
             for op in BIT_OPS
             for b in ["0", "1", "3", "7"]]

@pytest.mark.parametrize("a,op,b", BIT_PAIRS,
                         ids=[f"{a}{op}{b}" for a, op, b in BIT_PAIRS])
def test_bitwise_expr(a, op, b):
    _compile(f"let x = {a} {op} {b}")


# ---------------------------------------------------------------------------
# String literals
# ---------------------------------------------------------------------------
STRING_LITS = [
    '""', '"a"', '"hello"', '"hello world"',
    '"with \\"quotes\\""',
    '"with\\nnewlines"',
    '"with\\ttabs"',
    '"unicode: \\u0041"',
    '"hex: \\x41"',
    '"double \\x41\\x42\\x43"',
    '"edge\\\\case"',
    '"a" * 3',
    '"hello" + " world"',
    '"=" * 10',
]

@pytest.mark.parametrize("src", STRING_LITS)
def test_string_literal(src):
    _compile(f'let x = {src}')


# ---------------------------------------------------------------------------
# List literals
# ---------------------------------------------------------------------------
LIST_LITS = [
    "[]",
    "[1]",
    "[1, 2, 3]",
    '["a", "b", "c"]',
    "[true, false, none]",
    "[[1, 2], [3, 4]]",
    "[1 + 2, 3 * 4, 5 ** 2]",
    "[i for i in [1, 2, 3]]",
    "[x * 2 for x in [1, 2, 3]]",
    "[x for x in [1, 2, 3] if x > 1]",
    "[0] * 5",
    "[None] * 3",
]

@pytest.mark.parametrize("src", LIST_LITS)
def test_list_literal(src):
    _compile(f"let x = {src}")


# ---------------------------------------------------------------------------
# Dict literals
# ---------------------------------------------------------------------------
DICT_LITS = [
    "{}",
    '{"a": 1}',
    '{"a": 1, "b": 2, "c": 3}',
    "{1: \"one\", 2: \"two\"}",
    '{"nested": {"key": "value"}}',
    '{"expr": 1 + 2, "str": "hello"}',
]

@pytest.mark.parametrize("src", DICT_LITS)
def test_dict_literal(src):
    _compile(f"let x = {src}")


# ---------------------------------------------------------------------------
# Tuple literals
# ---------------------------------------------------------------------------
TUPLE_LITS = [
    "(1)",
    "(1, 2)",
    "(1, 2, 3)",
    '("a", "b")',
    "(1, [2, 3])",
    "(true, false)",
]

@pytest.mark.parametrize("src", TUPLE_LITS)
def test_tuple_literal(src):
    _compile(f"let x = {src}")


# ---------------------------------------------------------------------------
# Set literals
# ---------------------------------------------------------------------------
SET_LITS = [
    "{1, 2, 3}",
    '{"a", "b", "c"}',
]

@pytest.mark.parametrize("src", SET_LITS)
def test_set_literal(src):
    _compile(f"let x = {src}")


# ---------------------------------------------------------------------------
# Ternary expressions
# ---------------------------------------------------------------------------
TERNARY = [
    "true ? 1 : 2",
    "false ? 1 : 2",
    "1 < 2 ? 10 : 20",
    'x > 0 ? "pos" : "non-pos"',
    "true ? true ? 1 : 2 : 3",
    "(1 + 2) > 2 ? 1 : 0",
    "true ? (false ? 1 : 2) : 3",
]

@pytest.mark.parametrize("src", TERNARY)
def test_ternary(src):
    _compile(f"let x = {src}")


# ---------------------------------------------------------------------------
# Null coalescing
# ---------------------------------------------------------------------------
NULL_COALESCE = [
    "none ?? 0",
    "none ?? 1",
    "none ?? \"default\"",
    "1 ?? 2",
    '"hello" ?? "world"',
]

@pytest.mark.parametrize("src", NULL_COALESCE)
def test_null_coalesce(src):
    _compile(f"let x = {src}")


# ---------------------------------------------------------------------------
# Pipe operator
# ---------------------------------------------------------------------------
PIPE_CASES = [
    "5 |> (n) => n + 1",
    '"hello" |> (s) => s.length',
    "[1, 2, 3] |> (l) => l.length",
    "10 |> (n) => n * 2 |> (n) => n + 1",
]

@pytest.mark.parametrize("src", PIPE_CASES)
def test_pipe(src):
    _compile(f"let x = {src}")


# ---------------------------------------------------------------------------
# Range expressions
# ---------------------------------------------------------------------------
RANGE_CASES = [
    "0..5",
    "1..10",
    "5..1",
    "0..0",
    "-5..5",
]

@pytest.mark.parametrize("src", RANGE_CASES)
def test_range(src):
    _compile(f"let x = {src}")


# ---------------------------------------------------------------------------
# Function calls
# ---------------------------------------------------------------------------
CALL_CASES = [
    "print(1)",
    "print(1, 2, 3)",
    'print("hello")',
    "print(true)",
    "print(none)",
    "len([1, 2, 3])",
    'len("hello")',
    "str(42)",
    "int(42)",
    "float(3.14)",
    "bool(1)",
    "abs(-5)",
    "max(1, 2)",
    "min(1, 2)",
    "sum([1, 2, 3])",
    "type(42)",
    "isinstance(42, int)",
    "range(5)",
    "range(0, 10)",
    "range(0, 10, 2)",
    "list(range(5))",
    "dict({\"a\": 1})",
    "set([1, 2, 3])",
    "sorted([3, 1, 2])",
    "reversed([1, 2, 3])",
    "enumerate([\"a\", \"b\"])",
    "zip([1, 2], [\"a\", \"b\"])",
    "map((x) => x * 2, [1, 2, 3])",
    "filter((x) => x > 1, [1, 2, 3])",
    "any([true, false, false])",
    "all([true, true, true])",
    "hex(255)",
    "oct(8)",
    "bin(10)",
    "chr(65)",
    "ord(\"A\")",
    "id(42)",
    "hash(\"hello\")",
    "repr(42)",
    "format(3.14, \".1f\")",
]

@pytest.mark.parametrize("src", CALL_CASES)
def test_function_call(src):
    _compile(f"let x = {src}")


# ---------------------------------------------------------------------------
# Method chains
# ---------------------------------------------------------------------------
CHAIN_CASES = [
    '"hello".to_upper()',
    '"HELLO".to_lower()',
    '"  hi  ".trim()',
    "[1, 2, 3].length",
    '"hello world".split(" ")',
    '["a", "b"].join(",")',
    '"hello".contains("ell")',
    '"hello".starts_with("he")',
    '"hello".ends_with("lo")',
    '"hello".replace("l", "x")',
    '"hello".reverse()',
    '"hello".repeat_string(3)',
    '[1, 2, 3].map((x) => x * 2)',
    '[1, 2, 3].filter((x) => x > 1)',
    '[1, 2, 3].reduce((a, b) => a + b)',
    '{"a": 1, "b": 2}.keys()',
    '{"a": 1, "b": 2}.values()',
    '{"a": 1, "b": 2}.items()',
]

@pytest.mark.parametrize("src", CHAIN_CASES)
def test_method_chain(src):
    _compile(f"let x = {src}")


# ---------------------------------------------------------------------------
# Nested expressions
# ---------------------------------------------------------------------------
NESTED_EXPRS = [
    "(1 + 2) * (3 + 4)",
    "((1 + 2) * 3) + 4",
    "(true and false) or (false and true)",
    "not (true and false)",
    "-(-(5))",
    "[[1, 2], [3, [4, 5]]]",
    '{"a": {"b": {"c": 1}}}',
    "(1 + 2) * 3 > (4 + 5) and true",
    "((1 < 2) and (3 < 4)) or ((5 > 6) and (7 > 8))",
    "true ? (1 + 2) : (3 + 4)",
    "none ?? (1 + 2)",
    "5 |> (n) => n + 1 |> (n) => n * 2",
    "let x = 1; let y = 2; let z = x + y; z",
    "[1, 2, 3].map((x) => x ** 2).filter((x) => x > 2)",
]

@pytest.mark.parametrize("src", NESTED_EXPRS)
def test_nested_expr(src):
    _compile(f"let x = {src}")
