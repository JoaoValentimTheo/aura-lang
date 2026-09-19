"""Transpiler fuzz tests — batch 2.

Verifies that the transpiler produces valid Python for a wide range
of Aura programs without crashing.
"""
from __future__ import annotations

import pytest

from aura.parser.to_ast import Tokenizer, Parser
from aura.transpiler.transformer import Transformer


def _transpile(src):
    tokens = Tokenizer(src).tokenize()
    tree = Parser(tokens).parse()
    t = Transformer()
    return t.transform(tree)


def _compile_check(src):
    code = _transpile(src)
    compile(code, "<test>", "exec")
    return code


# ---------------------------------------------------------------------------
# Variable declarations
# ---------------------------------------------------------------------------

VAR_CASES = [
    ("int", "let x = 42"),
    ("float", "let x = 3.14"),
    ("string", 'let x = "hello"'),
    ("bool_true", "let x = true"),
    ("bool_false", "let x = false"),
    ("none", "let x = none"),
    ("list", "let x = [1, 2, 3]"),
    ("dict", 'let x = {"a": 1}'),
    ("tuple", "let x = (1, 2)"),
    ("set", "let x = {1, 2, 3}"),
    ("mut", "let mut x = 1"),
    ("const", "const X = 42"),
    ("typed", "let x: int = 42"),
    ("typed_string", 'let x: str = "hi"'),
    ("expr_add", "let x = 1 + 2"),
    ("expr_mul", "let x = 2 * 3"),
    ("expr_div", "let x = 6 / 2"),
    ("expr_mod", "let x = 10 % 3"),
    ("expr_power", "let x = 2 ** 10"),
    ("expr_neg", "let x = -5"),
    ("expr_not", "let x = not true"),
    ("expr_and", "let x = true and false"),
    ("expr_or", "let x = true or false"),
    ("expr_eq", "let x = 1 == 1"),
    ("expr_neq", "let x = 1 != 2"),
    ("expr_lt", "let x = 1 < 2"),
    ("expr_gt", "let x = 2 > 1"),
    ("expr_lte", "let x = 1 <= 1"),
    ("expr_gte", "let x = 2 >= 2"),
    ("expr_pipe", "let x = 5 |> (n) => n * 2"),
    ("expr_ternary", "let x = true ? 1 : 2"),
    ("expr_null_coalesce", "let x = none ?? 0"),
    ("expr_range", "let x = 1..10"),
    ("expr_call", "let x = len([1, 2, 3])"),
    ("expr_index", "let x = [1, 2, 3][0]"),
    ("expr_member", 'let x = "hello".length'),
]


@pytest.mark.parametrize("name,src", VAR_CASES, ids=[i[0] for i in VAR_CASES])
def test_var_transpile(name, src):
    _compile_check(src)


# ---------------------------------------------------------------------------
# Functions
# ---------------------------------------------------------------------------

FUNC_CASES = [
    ("simple", "def f() { return 1 }"),
    ("with_params", "def f(a: int, b: int) -> int { return a + b }"),
    ("no_return", "def f() { print(1) }"),
    ("varargs", "def f(*args) { return args }"),
    ("kwargs", "def f(**kwargs) { return kwargs }"),
    ("defaults", 'def f(a: int = 0, b: str = "x") { return a }'),
    ("nested", "def f() { def g() { return 1 } return g() }"),
    ("recursive", "def fib(n: int) -> int { if n <= 1 { return n } return fib(n-1) + fib(n-2) }"),
    ("async_func", "async def f() { return 1 }"),
    ("generator", "def f() { yield 1; yield 2; yield 3 }"),
    ("lambda_simple", "let f = (x) => x + 1"),
    ("lambda_multi", "let f = (a, b) => a + b"),
    ("lambda_block", "let f = (x) => { let y = x * 2; y }"),
    ("lambda_no_params", "let f = () => 42"),
    ("method", "class C { public def m() -> int { return 1 } }"),
    ("static_method", "class C { static def m() -> int { return 1 } }"),
    ("abstract_method", "abstract class C { abstract def m() {} }"),
    ("override_method", "class C extends B { override def m() { } }"),
]


@pytest.mark.parametrize("name,src", FUNC_CASES, ids=[i[0] for i in FUNC_CASES])
def test_func_transpile(name, src):
    _compile_check(src)


# ---------------------------------------------------------------------------
# Control flow
# ---------------------------------------------------------------------------

CONTROL_CASES = [
    ("if_true", "if true { let x = 1 }"),
    ("if_false", "if false { let x = 1 }"),
    ("if_else", "if true { let x = 1 } else { let x = 2 }"),
    ("if_elif", "if false { let x = 1 } else if true { let x = 2 }"),
    ("if_elif_else", "if false { let x = 1 } else if false { let x = 2 } else { let x = 3 }"),
    ("for_list", "for i in [1, 2, 3] { print(i) }"),
    ("for_range", "for i in 0..5 { print(i) }"),
    ("for_with_index", "for (i, v) in [1, 2, 3].enumerate() { print(i) }"),
    ("while_true", "let mut i = 0; while i < 3 { i = i + 1 }"),
    ("loop_break", "loop { break }"),
    ("loop_continue", "loop { continue }"),
    ("loop_with_var", "let mut x = 0; loop { x = x + 1; if x > 10 { break } }"),
    ("match_int", "match 1 {\n  case 1 { \"one\" }\n  case _ { \"other\" }\n}"),
    ("match_string", 'match "x" {\n  case "a" { 1 }\n  case "b" { 2 }\n  case _ { 0 }\n}'),
    ("match_guard", "match 5 {\n  case n if n > 10 { \"big\" }\n  case _ { \"small\" }\n}"),
    ("match_arrow", "match 1 {\n  case 1 -> \"one\"\n  case _ -> \"other\"\n}"),
    ("try_catch", "try { let x = 1 } catch e { print(e) }"),
    ("try_finally", "try { let x = 1 } finally { print(\"done\") }"),
    ("try_catch_finally", "try { let x = 1 } catch e { print(e) } finally { print(\"done\") }"),
    ("guard", 'guard true else { throw Error("fail") }'),
    ("return_simple", "def f() { return 1 }"),
    ("return_none", "def f() { return }"),
    ("break_in_loop", "loop { break }"),
    ("continue_in_loop", "loop { continue }"),
    ("nested_loops", "for i in [1] { for j in [2] { print(i + j) } }"),
    ("nested_if", "if true { if true { let x = 1 } }"),
]


@pytest.mark.parametrize("name,src", CONTROL_CASES, ids=[i[0] for i in CONTROL_CASES])
def test_control_transpile(name, src):
    _compile_check(src)


# ---------------------------------------------------------------------------
# OOP
# ---------------------------------------------------------------------------

OOP_CASES = [
    ("simple_class", "class Point { public x: int; public y: int }"),
    ("class_with_methods", "class C { public def m() -> int { return 1 } }"),
    ("class_inherits", "class Dog extends Animal { public name: str }"),
    ("class_header_fields", "class User(private name: str, mut age: int = 0) { }"),
    ("trait_simple", "trait Drawable { public def draw() }"),
    ("trait_impl", "class Circle extends Drawable { public def draw() { print(\"circle\") } }"),
    ("abstract_class", "abstract class Shape { abstract def area() -> float {} }"),
    ("enum_simple", "enum Color { Red, Green, Blue }"),
    ("enum_with_values", "enum Status { Ok = 0, Error = 1 }"),
    ("type_alias", "type ID = int"),
    ("generic_class", "class Box[T](public value: T) { }"),
    ("trait_extends", "trait Closeable extends Drawable { public def close() }"),
    ("self_access", "class C { public name: str; public def get_name() -> str { return self.name } }"),
    ("constructor", "class C { public x: int }"),
    ("module_simple", "module M { public def helper() { 42 } }"),
]


@pytest.mark.parametrize("name,src", OOP_CASES, ids=[i[0] for i in OOP_CASES])
def test_oop_transpile(name, src):
    _compile_check(src)


# ---------------------------------------------------------------------------
# Imports and modules
# ---------------------------------------------------------------------------

IMPORT_CASES = [
    ("import_stdlib", 'import stdlib.math as math'),
    ("import_python", "import python"),
    ("from_import", "from stdlib.string import to_upper"),
    ("import_alias", 'import stdlib.json as json'),
]


@pytest.mark.parametrize("name,src", IMPORT_CASES, ids=[i[0] for i in IMPORT_CASES])
def test_import_transpile(name, src):
    _compile_check(src)


# ---------------------------------------------------------------------------
# Expressions — complex
# ---------------------------------------------------------------------------

COMPLEX_EXPR_CASES = [
    ("list_comprehension", "let x = [i * 2 for i in [1, 2, 3]]"),
    ("dict_literal", 'let x = {"a": 1, "b": 2}'),
    ("dict_access", 'let x = {"a": 1}["a"]'),
    ("set_literal", "let x = {1, 2, 3}"),
    ("tuple_unpack", "let (a, b) = (1, 2)"),
    ("list_unpack", "let [a, b, c] = [1, 2, 3]"),
    ("spread_in_call", "def f(*args) { return args }; f(*[1, 2, 3])"),
    ("spread_in_dict", 'let x = {**{"a": 1}, "b": 2}'),
    ("fstring_simple", 'let x = f"hello {1 + 2}"'),
    ("fstring_format", 'let x = f"{3.14:.2f}"'),
    ("fstring_escaped_braces", 'let x = f"{{literal}}"'),
    ("member_chain", '"hello".to_upper().length'),
    ("chained_calls", "[1, 2, 3].map((x) => x * 2).filter((x) => x > 2)"),
    ("nested_ternary", "let x = true ? (false ? 1 : 2) : 3"),
    ("complex_bool", "let x = (1 < 2) and (3 > 2) or not false"),
    ("bitwise_ops", "let x = 0xFF & 0x0F | 0xF0 ^ 0xAA ~ 0x00"),
    ("comparison_chain", "let x = 1 < 2 and 2 < 3 and 3 < 4"),
    ("string_ops", 'let x = "hello" + " " + "world"'),
    ("string_multiply", 'let x = "ab" * 3'),
    ("list_ops", "let x = [1, 2] + [3, 4]"),
    ("list_multiply", "let x = [0] * 5"),
    ("is_none", "let x = some_val is none"),
    ("is_not_none", "let x = some_val is not none"),
]


@pytest.mark.parametrize("name,src", COMPLEX_EXPR_CASES, ids=[i[0] for i in COMPLEX_EXPR_CASES])
def test_complex_expr_transpile(name, src):
    _compile_check(src)


# ---------------------------------------------------------------------------
# Full programs (need main)
# ---------------------------------------------------------------------------

FULL_PROGRAM_CASES = [
    ("hello_world", 'def main() { print("Hello, Aura!") }'),
    ("fibonacci", "def fib(n: int) -> int { if n <= 1 { return n } return fib(n-1) + fib(n-2) }\ndef main() { print(fib(10)) }"),
    ("factorial", "def fact(n: int) -> int { if n <= 1 { return 1 } return n * fact(n-1) }\ndef main() { print(fact(10)) }"),
    ("class_usage", "class Point(public x: int, public y: int) { public def dist() -> float { return (self.x ** 2 + self.y ** 2) ** 0.5 } }\ndef main() { let p = Point(3, 4); print(p.dist()) }"),
    ("match_usage", "def describe(x: int) -> str { match x { case 0 { \"zero\" } case n if n > 0 { \"positive\" } case _ { \"negative\" } } }\ndef main() { print(describe(5)) }"),
    ("loop_accumulator", "def main() { let mut sum = 0; for i in 0..10 { sum = sum + i }; print(sum) }"),
    ("dict_usage", 'def main() { let d = {"a": 1, "b": 2}; print(d["a"]) }'),
    ("list_ops_main", "def main() { let x = [1, 2, 3, 4, 5].filter((n) => n > 2); print(x) }"),
    ("string_ops_main", 'def main() { let x = "hello world".to_upper(); print(x) }'),
    ("lambda_main", "def main() { let double = (x) => x * 2; print(double(21)) }"),
    ("async_main", "async def main() { print(42) }"),
    ("try_main", 'def main() { try { print(1) } catch e { print(e) } }'),
    ("nested_func", "def main() { def inner() { return 42 }; print(inner()) }"),
    ("closure", "def main() { let x = 10; let f = () => x; print(f()) }"),
    ("recursion", "def countdown(n: int) { if n > 0 { print(n); countdown(n - 1) } }\ndef main() { countdown(5) }"),
]


@pytest.mark.parametrize("name,src", FULL_PROGRAM_CASES, ids=[i[0] for i in FULL_PROGRAM_CASES])
def test_full_program_transpile(name, src):
    _compile_check(src)
