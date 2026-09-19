"""Type exhaustive tests - batch 6a."""
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
# Type annotations in variables
# ---------------------------------------------------------------------------
TYPE_ANNOTATIONS = [
    ("int_literal", "let x: int = 42"),
    ("int_zero", "let x: int = 0"),
    ("int_neg", "let x: int = -1"),
    ("float_literal", "let x: float = 3.14"),
    ("float_zero", "let x: float = 0.0"),
    ("str_literal", "let x: str = \"hello\""),
    ("str_empty", "let x: str = \"\""),
    ("bool_true", "let x: bool = true"),
    ("bool_false", "let x: bool = false"),
    ("list_int", "let x: list = [1, 2, 3]"),
    ("list_empty", "let x: list = []"),
    ("dict_type", "let x: dict = {\"a\": 1}"),
    ("tuple_type", "let x = (1, 2)"),
    ("none_type", "let x = none"),
    ("mut_int", "let mut x: int = 42"),
    ("mut_str", "let mut x: str = \"hello\""),
    ("const_int", "const X: int = 42"),
    ("const_str", "const S: str = \"hello\""),
]

@pytest.mark.parametrize("name,src", TYPE_ANNOTATIONS, ids=[i[0] for i in TYPE_ANNOTATIONS])
def test_type_annotation_var(name, src):
    _compile(src)


# ---------------------------------------------------------------------------
# Type annotations in function params
# ---------------------------------------------------------------------------
PARAM_TYPES = [
    ("int_param", "def f(x: int) { print(x) }"),
    ("float_param", "def f(x: float) { print(x) }"),
    ("str_param", "def f(x: str) { print(x) }"),
    ("bool_param", "def f(x: bool) { print(x) }"),
    ("list_param", "def f(x: list) { print(x) }"),
    ("dict_param", "def f(x: dict) { print(x) }"),
    ("multi_params", "def f(a: int, b: str, c: bool) {}"),
    ("default_int", "def f(x: int = 0) {}"),
    ("default_str", 'def f(x: str = "hi") {}'),
    ("default_bool", "def f(x: bool = true) {}"),
    ("varargs", "def f(*args) {}"),
    ("kwargs", "def f(**kwargs) {}"),
    ("mixed", "def f(a: int, *args, b: str = \"x\", **kwargs) {}"),
]

@pytest.mark.parametrize("name,src", PARAM_TYPES, ids=[i[0] for i in PARAM_TYPES])
def test_param_types(name, src):
    _compile(src)


# ---------------------------------------------------------------------------
# Return types
# ---------------------------------------------------------------------------
RETURN_TYPES = [
    ("return_int", "def f() -> int { return 1 }"),
    ("return_float", "def f() -> float { return 1.0 }"),
    ("return_str", 'def f() -> str { return "hi" }'),
    ("return_bool", "def f() -> bool { return true }"),
    ("return_list", "def f() -> list { return [1] }"),
    ("return_dict", 'def f() -> dict { return {"a": 1} }'),
]

@pytest.mark.parametrize("name,src", RETURN_TYPES, ids=[i[0] for i in RETURN_TYPES])
def test_return_types(name, src):
    _compile(src)


# ---------------------------------------------------------------------------
# Generic type params
# ---------------------------------------------------------------------------
GENERIC_TYPES = [
    ("generic_class", "class Box[T](public value: T) {}"),
    ("generic_func", "def f[T](x: T) -> T { return x }"),
    ("generic_multi", "class Pair[A, B](public first: A, public second: B) {}"),
]

@pytest.mark.parametrize("name,src", GENERIC_TYPES, ids=[i[0] for i in GENERIC_TYPES])
def test_generic_types(name, src):
    _compile(src)
