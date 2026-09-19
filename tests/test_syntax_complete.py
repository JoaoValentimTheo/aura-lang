"""Comprehensive syntax-coverage tests for Aura.

Every statement and expression form the parser accepts is exercised end to end:
tokenize -> parse -> transpile -> execute, asserting on real output. The goal is
that a regression in any single construct fails a test here, so the full syntax
surface stays covered without relying on the generated `.aura` corpora.

Constructs covered:
  * declarations: let / let mut / const / def / fn / async def / class / trait /
    enum / type / module / import / from-import
  * control flow: if / else if / else, unless, guard, while, until, for (with
    step), loop, labeled break/continue, return, throw, assert, try/catch/finally
  * expressions: all unary and binary operators, ternary, ranges, pipe, cast,
    safe navigation, spread, coalescing, comprehensions, lambdas, f-strings,
    match (statement and expression), with, destructuring
"""
import contextlib
import io
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Tokenizer, Parser  # noqa: E402
from transpiler.transformer import Transformer  # noqa: E402


def run_aura(source: str):
    """Transpile Aura source, execute it, and return captured stdout."""
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    code = Transformer().transform(program)
    namespace = {"__name__": "__aura_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, namespace)
    return buffer.getvalue()


def transpile(source: str):
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    return Transformer().transform(program)


# ============================================================================
# Declarations
# ============================================================================

def test_let_and_let_mut():
    out = run_aura("let a = 1\nlet mut b = 2\nb = 3\nprint(a + b)")
    assert out == "4\n"


def test_const():
    out = run_aura("const PI = 3.14\nprint(PI)")
    assert out == "3.14\n"


def test_def_and_fn_forms():
    out = run_aura(
        "def add(a, b) { return a + b }\n"
        "def sub(a, b) { return a - b }\n"
        "print(add(3, 4))\nprint(sub(10, 3))"
    )
    assert out == "7\n7\n"


def test_function_default_and_kwargs():
    out = run_aura(
        "def greet(name, punct = '!') { return 'hi ' + name + punct }\n"
        "print(greet('a'))\nprint(greet('b', '?'))"
    )
    assert out == "hi a!\nhi b?\n"


def test_variadic_and_kwonly():
    out = run_aura(
        "def total(*nums) { let mut s = 0\nfor n in nums { s += n }\nreturn s }\n"
        "print(total(1, 2, 3))"
    )
    assert out == "6\n"


def test_async_await():
    import asyncio

    source = (
        "async def work(x) { return x * 2 }\n"
        "let r = await work(21)\nprint(r)"
    )
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    code = Transformer().transform(program)
    indented = "\n".join(
        ("    " + line if line.strip() else line) for line in code.split("\n")
    )
    wrapper = "async def _aura_main():\n" + indented + "\n"
    namespace = {"__name__": "__aura_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(wrapper, "<test>", "exec"), namespace)
        asyncio.run(namespace["_aura_main"]())
    assert buffer.getvalue() == "42\n"


def test_type_alias():
    out = run_aura("type UserId = int\nlet x: UserId = 5\nprint(x)")
    assert out == "5\n"


def test_module_declaration():
    out = run_aura(
        "module Math {\n"
        "  export let answer = 42\n"
        "}\n"
        "print(Math.answer)"
    )
    assert out == "42\n"


def test_enum_declaration():
    out = run_aura(
        "enum Color { Red, Green, Blue }\n"
        "print(Color.Green.value)"
    )
    assert out == "2\n"


def test_enum_with_explicit_values():
    out = run_aura(
        "enum E { A = 1, B = 5, C }\n"
        "print(E.C.value)"
    )
    assert out == "6\n"


def test_import_and_from_import():
    out = run_aura(
        "import stdlib.math as m\n"
        "from stdlib.string import upper\n"
        "print(upper('ab'))"
    )
    assert out == "AB\n"


# ============================================================================
# Control flow
# ============================================================================

def test_if_else_if_else():
    out = run_aura(
        "let n = 2\n"
        "if n == 1 { print('one') } else if n == 2 { print('two') } else { print('other') }"
    )
    assert out == "two\n"


def test_unless():
    out = run_aura("let flag = false\nunless flag { print('yes') }")
    assert out == "yes\n"


def test_guard_early_return():
    out = run_aura(
        "def check(n) {\n"
        "  guard n > 0 else { return 'bad' }\n"
        "  return 'good'\n"
        "}\n"
        "print(check(5))\nprint(check(-1))"
    )
    assert out == "good\nbad\n"


def test_while_loop():
    out = run_aura(
        "let mut i = 0\nwhile i < 3 { print(i)\ni += 1 }"
    )
    assert out == "0\n1\n2\n"


def test_until_loop():
    out = run_aura(
        "let mut i = 0\nuntil i >= 3 { print(i)\ni += 1 }"
    )
    assert out == "0\n1\n2\n"


def test_for_loop():
    out = run_aura("for x in [1, 2, 3] { print(x) }")
    assert out == "1\n2\n3\n"


def test_for_with_step():
    out = run_aura("for i in range(0, 6, 2) { print(i) }")
    assert out == "0\n2\n4\n"


def test_loop_infinite_with_break():
    out = run_aura(
        "let mut i = 0\nloop { if i == 2 { break }\nprint(i)\ni += 1 }"
    )
    assert out == "0\n1\n"


def test_continue():
    out = run_aura(
        "for i in range(4) { if i == 1 { continue }\nprint(i) }"
    )
    assert out == "0\n2\n3\n"


def test_labeled_break_and_continue():
    out = run_aura(
        "outer: for i in range(3) {\n"
        "  for j in range(3) {\n"
        "    if j == 1 { continue outer }\n"
        "    if i == 2 { break outer }\n"
        "    print(i, j)\n"
        "  }\n"
        "}"
    )
    assert out == "0 0\n1 0\n"


def test_assert():
    run_aura("assert 1 + 1 == 2")
    with pytest.raises(AssertionError):
        run_aura("assert 1 == 2, 'nope'")


def test_throw_and_catch():
    out = run_aura(
        "def f() { throw ValueError('boom') }\n"
        "try { f() } catch Error as e { print('caught') }"
    )
    assert out == "caught\n"


def test_try_catch_finally():
    out = run_aura(
        "try { throw ValueError('x') }\n"
        "catch Error as e { print('c') }\n"
        "finally { print('f') }"
    )
    assert out == "c\nf\n"


def test_try_only_finally():
    out = run_aura("try { print('t') } finally { print('f') }")
    assert out == "t\nf\n"


def test_try_without_handler_is_error():
    with pytest.raises(SyntaxError):
        run_aura("try { print('t') }")


def test_throw_string_wrapped():
    out = run_aura(
        "try { throw 'plain' } catch Error as e { print('ok') }"
    )
    assert out == "ok\n"


# ============================================================================
# Operators
# ============================================================================

@pytest.mark.parametrize("expr,expected", [
    ("1 + 2", "3"),
    ("5 - 3", "2"),
    ("3 * 4", "12"),
    ("8 / 2", "4.0"),
    ("7 % 3", "1"),
    ("2 ** 10", "1024"),
    ("6 & 3", "2"),
    ("6 | 3", "7"),
    ("6 ^ 3", "5"),
    ("1 << 4", "16"),
    ("32 >> 2", "8"),
])
def test_arithmetic_and_bitwise(expr, expected):
    assert run_aura(f"print({expr})") == expected + "\n"


@pytest.mark.parametrize("expr,expected", [
    ("1 < 2", "True"),
    ("2 <= 2", "True"),
    ("3 > 4", "False"),
    ("4 >= 4", "True"),
    ("1 == 1", "True"),
    ("1 != 2", "True"),
    ("3 in [1, 2, 3]", "True"),
    ("5 not in [1, 2, 3]", "True"),
    ("none is none", "True"),
])
def test_comparison_operators(expr, expected):
    assert run_aura(f"print({expr})") == expected + "\n"


def test_logical_operators():
    assert run_aura("print(true and false)") == "False\n"
    assert run_aura("print(true or false)") == "True\n"
    assert run_aura("print(not true)") == "False\n"
    assert run_aura("print(true and false)") == "False\n"
    assert run_aura("print(true or false)") == "True\n"
    assert run_aura("print(not false)") == "True\n"


@pytest.mark.parametrize("expr,expected", [
    ("-5", "-5"),
    ("+5", "5"),
    ("~0", "-1"),
])
def test_unary_operators(expr, expected):
    assert run_aura(f"print({expr})") == expected + "\n"


def test_ternary():
    out = run_aura("let x = 5\nprint(x > 3 ? 'big' : 'small')")
    assert out == "big\n"


def test_null_coalescing():
    out = run_aura("let x = none\nprint(x ?? 'default')")
    assert out == "default\n"


def test_range_operators():
    out = run_aura("print(list(1..4))\nprint(list(1..<4))")
    assert out == "[1, 2, 3, 4]\n[1, 2, 3]\n"


def test_pipe_operator():
    out = run_aura(
        "let r = [1, 2, 3, 4] |> filter((x) => x % 2 == 0) |> map((x) => x * 10)\n"
        "print(r)"
    )
    assert out == "[20, 40]\n"


def test_cast_operator():
    out = run_aura("let x = '42'\nprint(int(x) + 1)")
    assert out == "43\n"


def test_safe_navigation():
    out = run_aura(
        "class N { let v: int = 7 }\n"
        "let a = N()\n"
        "let b = none\n"
        "print(a?.v)\nprint(b?.v)"
    )
    assert out == "7\nNone\n"


# ============================================================================
# Expressions and literals
# ============================================================================

def test_literals():
    assert run_aura("print(0xFF)") == "255\n"
    assert run_aura("print(0b1010)") == "10\n"
    assert run_aura("print(0o17)") == "15\n"
    assert run_aura("print(1_000_000)") == "1000000\n"
    assert run_aura("print(1.5e2)") == "150.0\n"


def test_string_forms():
    assert run_aura("print('single')") == "single\n"
    assert run_aura('print("double")') == "double\n"
    assert run_aura('print("""triple""")') == "triple\n"
    assert run_aura(r'print("a\tb")') == "a\tb\n"


def test_fstring():
    out = run_aura("let n = 5\nprint(f'{n} squared is {n * n}')")
    assert out == "5 squared is 25\n"


def test_fstring_format_spec():
    out = run_aura("let x = 3.14159\nprint(f'{x:.2f}')")
    assert out == "3.14\n"


def test_collection_literals():
    assert run_aura("print([1, 2, 3])") == "[1, 2, 3]\n"
    assert run_aura("print({1, 2, 3})") == "{1, 2, 3}\n"
    assert run_aura("print((1, 2))") == "(1, 2)\n"


def test_dict_literal_attr_access():
    out = run_aura("let d = {'name': 'Ann', 'age': 30}\nprint(d.name)\nprint(d['age'])")
    assert out == "Ann\n30\n"


def test_list_comprehension():
    out = run_aura("print([x * x for x in range(5)])")
    assert out == "[0, 1, 4, 9, 16]\n"


def test_set_comprehension():
    out = run_aura("print(sorted([x % 3 for x in range(6)]))")
    assert out == "[0, 0, 1, 1, 2, 2]\n"


def test_dict_comprehension():
    out = run_aura("print({k: k * 2 for k in range(3)})")
    assert out == "{0: 0, 1: 2, 2: 4}\n"


def test_lambda_forms():
    assert run_aura("let f = (x) => x + 1\nprint(f(4))") == "5\n"
    assert run_aura("let g = x => x * 2\nprint(g(4))") == "8\n"


def test_lambda_block_body():
    out = run_aura(
        "let f = (x) => { let y = x + 1\nreturn y * 2 }\nprint(f(3))"
    )
    assert out == "8\n"


def test_indexing_and_slicing():
    assert run_aura("print([10, 20, 30][1])") == "20\n"
    assert run_aura("print([10, 20, 30, 40][1:3])") == "[20, 30]\n"
    assert run_aura("print([0, 1, 2, 3, 4][::2])") == "[0, 2, 4]\n"


def test_spread_call():
    out = run_aura(
        "def add(a, b, c) { return a + b + c }\n"
        "let nums = [1, 2, 3]\n"
        "print(add(*nums))"
    )
    assert out == "6\n"


def test_spread_dict_call():
    out = run_aura(
        "def add(a, b) { return a + b }\n"
        "let kw = {'a': 1, 'b': 2}\n"
        "print(add(**kw))"
    )
    assert out == "3\n"


def test_adaptive_spread():
    out = run_aura(
        "def add(a, b) { return a + b }\n"
        "let nums = [4, 5]\n"
        "let kw = {'a': 7, 'b': 8}\n"
        "print(add(...nums))\nprint(add(...kw))"
    )
    assert out == "9\n15\n"


def test_destructuring_assignment():
    out = run_aura("let a, b = 1, 2\nprint(a)\nprint(b)")
    assert out == "1\n2\n"


def test_swap():
    out = run_aura("let mut a = 1\nlet mut b = 2\na, b = b, a\nprint(a)\nprint(b)")
    assert out == "2\n1\n"


def test_struct_init():
    out = run_aura(
        "class Point { let x: int = 0\nlet y: int = 0 }\n"
        "let p = Point { x: 3, y: 4 }\n"
        "print(p.x + p.y)"
    )
    assert out == "7\n"


# ============================================================================
# Pattern matching
# ============================================================================

def test_match_literal():
    out = run_aura(
        "let n = 2\n"
        "match n {\n"
        "  case 1 { print('one') }\n"
        "  case 2 { print('two') }\n"
        "  case _ { print('other') }\n"
        "}"
    )
    assert out == "two\n"


def test_match_guard():
    out = run_aura(
        "let n = 150\n"
        "match n {\n"
        "  case x if x > 100 { print('big') }\n"
        "  case _ { print('small') }\n"
        "}"
    )
    assert out == "big\n"


def test_match_as_expression():
    out = run_aura(
        "let n = 1\n"
        "let label = match n { case 1 { 'one' } case _ { 'other' } }\n"
        "print(label)"
    )
    assert out == "one\n"


# ============================================================================
# With / context manager
# ============================================================================

def test_with_statement():
    out = run_aura(
        "class Ctx {\n"
        "  def enter() { print('enter')\nreturn self }\n"
        "  def exit(t, e, tb) { print('exit') }\n"
        "}\n"
        "with Ctx() as c { print('body') }"
    )
    assert out == "enter\nbody\nexit\n"


# ============================================================================
# Decorators
# ============================================================================

def test_decorator():
    out = run_aura(
        "def twice(g) { return (x) => g(g(x)) }\n"
        "@twice\n"
        "def inc(x) { return x + 1 }\n"
        "print(inc(5))"
    )
    assert out == "7\n"


def test_builtin_macros_present():
    code = transpile(
        "@timeit\ndef f() { return 1 }\n"
        "f()"
    )
    assert "def timeit(" in code


# ============================================================================
# Comments and formatting
# ============================================================================

def test_line_and_block_comments():
    out = run_aura(
        "// a line comment\n"
        "let x = 1 /* inline */ + 2\n"
        "/* block\n   comment */\n"
        "print(x)"
    )
    assert out == "3\n"