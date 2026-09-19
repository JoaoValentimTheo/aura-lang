"""Statement exhaustive tests - batch 5."""
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
# Variable declarations
# ---------------------------------------------------------------------------
VAR_DECL = [
    "let x = 1",
    "let x = 3.14",
    "let x = true",
    "let x = false",
    "let x = none",
    "let x = \"hello\"",
    "let x = [1, 2, 3]",
    "let x = {\"a\": 1}",
    "let x = (1, 2)",
    "let x = 1 + 2",
    "let x = 2 * 3",
    "let x = 10 / 2",
    "let x = 2 ** 10",
    "let x = 10 % 3",
    "let x = 1 < 2",
    "let x = 1 == 1",
    "let x = true and false",
    "let x = true or false",
    "let x = not true",
    "let x = 1..10",
    "let mut x = 1",
    "const X = 42",
    "let x: int = 42",
    "let x: str = \"hello\"",
    "let x = none ?? 0",
]

@pytest.mark.parametrize("src", VAR_DECL)
def test_var_decl(src):
    _compile(src)


# ---------------------------------------------------------------------------
# Assignment
# ---------------------------------------------------------------------------
ASSIGN = [
    ("let x = 1\nx = 2",),
    ("let mut x = 1\nx = 2",),
    ("let mut x = 1\nx = x + 1",),
    ("let mut x = 1\nx += 1",),
    ("let mut x = 10\nx -= 3",),
    ("let mut x = 2\nx *= 3",),
    ("let mut x = 10\nx /= 2",),
    ("let mut x = 2\nx **= 3",),
    ("let mut x = 10\nx %= 3",),
]

@pytest.mark.parametrize("case", ASSIGN)
def test_assignment(case):
    _compile(case[0])


# ---------------------------------------------------------------------------
# If statements
# ---------------------------------------------------------------------------
IF_CASES = [
    "if true { let x = 1 }",
    "if false { let x = 1 }",
    "if true { let x = 1 } else { let x = 2 }",
    "if false { let x = 1 } else if true { let x = 2 }",
    "if false { let x = 1 } else if false { let x = 2 } else { let x = 3 }",
    "if 1 < 2 { let x = 1 }",
    'if "hello" == "hello" { let x = 1 }',
    "if [1, 2, 3].length > 0 { let x = 1 }",
    "if true and true { let x = 1 }",
    "if false or true { let x = 1 }",
    "if not false { let x = 1 }",
]

@pytest.mark.parametrize("src", IF_CASES)
def test_if(src):
    _compile(src)


# ---------------------------------------------------------------------------
# While loops
# ---------------------------------------------------------------------------
WHILE_CASES = [
    "let mut i = 0\nwhile i < 10 { i += 1 }",
    "let mut i = 10\nwhile i > 0 { i -= 1 }",
    "while false { break }",
    "let mut x = 0\nwhile true { x += 1\nif x > 5 { break } }",
]

@pytest.mark.parametrize("src", WHILE_CASES)
def test_while(src):
    _compile(src)


# ---------------------------------------------------------------------------
# For loops
# ---------------------------------------------------------------------------
FOR_CASES = [
    "for i in [1, 2, 3] { print(i) }",
    "for i in 0..5 { print(i) }",
    "for c in \"hello\" { print(c) }",
    "for i in [10, 20, 30] { let x = i }",
]

@pytest.mark.parametrize("src", FOR_CASES)
def test_for(src):
    _compile(src)


# ---------------------------------------------------------------------------
# Loop
# ---------------------------------------------------------------------------
LOOP_CASES = [
    "loop { break }",
    "loop { continue }",
    "let mut i = 0\nloop { i += 1\nif i >= 5 { break } }",
]

@pytest.mark.parametrize("src", LOOP_CASES)
def test_loop(src):
    _compile(src)


# ---------------------------------------------------------------------------
# Match
# ---------------------------------------------------------------------------
MATCH_CASES = [
    "match 1 {\n  case 1 { \"one\" }\n  case _ { \"other\" }\n}",
    "match 2 {\n  case 1 { \"one\" }\n  case 2 { \"two\" }\n  case _ { \"other\" }\n}",
    'match "hello" {\n  case "hello" { 1 }\n  case _ { 0 }\n}',
    "match 5 {\n  case n if n > 10 { \"big\" }\n  case _ { \"small\" }\n}",
    "match 1 {\n  case 1 -> \"one\"\n  case _ -> \"other\"\n}",
    "match 3 {\n  case 1 -> 1\n  case 2 -> 2\n  case 3 -> 3\n  case _ -> 0\n}",
]

@pytest.mark.parametrize("src", MATCH_CASES)
def test_match(src):
    _compile(src)


# ---------------------------------------------------------------------------
# Functions
# ---------------------------------------------------------------------------
FUNC_CASES = [
    "def f() {}",
    "def f() { return 1 }",
    "def f() { return }",
    "def f(a: int) { print(a) }",
    "def f(a: int, b: int) -> int { return a + b }",
    "def f(a: int = 0) { return a }",
    "def f(a: int = 0, b: str = \"x\") { return a }",
    "def f(*args) { return args }",
    "def f(**kwargs) { return kwargs }",
    "def f(a: int, *args) { return a }",
    "async def f() { return 1 }",
    "def f() { yield 1 }",
    "def f() { yield 1\nyield 2\nyield 3 }",
    "def f() {\n  let x = 1\n  let y = 2\n  return x + y\n}",
    "def f() {\n  if true { return 1 } else { return 2 }\n}",
]

@pytest.mark.parametrize("src", FUNC_CASES)
def test_func(src):
    _compile(src)


# ---------------------------------------------------------------------------
# Lambda
# ---------------------------------------------------------------------------
LAMBDA_CASES = [
    "let f = (x) => x",
    "let f = (x) => x + 1",
    "let f = (x, y) => x + y",
    "let f = () => 42",
    "let f = (x) => { let y = x * 2\ny }",
    "let f = (x) => x > 0",
]

@pytest.mark.parametrize("src", LAMBDA_CASES)
def test_lambda(src):
    _compile(src)


# ---------------------------------------------------------------------------
# Try/Catch
# ---------------------------------------------------------------------------
TRY_CASES = [
    "try { let x = 1 } catch e { print(e) }",
    "try { let x = 1 } finally { print(1) }",
    "try { let x = 1 } catch e { print(e) } finally { print(1) }",
    "try { throw Error(\"msg\") } catch e { print(e) }",
    "try {\n  let x = 1\n  let y = 2\n} catch e {\n  print(e)\n}",
]

@pytest.mark.parametrize("src", TRY_CASES)
def test_try(src):
    _compile(src)


# ---------------------------------------------------------------------------
# Guard
# ---------------------------------------------------------------------------
GUARD_CASES = [
    "guard true else { return }",
    "guard 1 < 2 else { return }",
    "guard \"hello\" != \"\" else { return }",
]

@pytest.mark.parametrize("src", GUARD_CASES)
def test_guard(src):
    _compile(src)


# ---------------------------------------------------------------------------
# Class
# ---------------------------------------------------------------------------
CLASS_CASES = [
    "class C {}",
    "class C { public x: int }",
    "class C { let x: int }",
    "class C { public x: int\npublic y: int }",
    "class C { public def m() { return 1 } }",
    "class C { static def m() { return 1 } }",
    "class C extends B {}",
    "class C extends B { public x: int }",
    "class C { public def m() { return self.x } }",
    "abstract class C { abstract def m() }",
    "class C { private def m() {} }",
    "class C { protected def m() {} }",
]

@pytest.mark.parametrize("src", CLASS_CASES)
def test_class(src):
    _compile(src)


# ---------------------------------------------------------------------------
# Enum
# ---------------------------------------------------------------------------
ENUM_CASES = [
    "enum Color { Red, Green, Blue }",
    "enum Status { Ok = 0, Error = 1 }",
    "enum Direction { North, South, East, West }",
]

@pytest.mark.parametrize("src", ENUM_CASES)
def test_enum(src):
    _compile(src)


# ---------------------------------------------------------------------------
# Type alias
# ---------------------------------------------------------------------------
TYPE_CASES = [
    "type ID = int",
    "type Name = str",
]

@pytest.mark.parametrize("src", TYPE_CASES)
def test_type_alias(src):
    _compile(src)


# ---------------------------------------------------------------------------
# Module
# ---------------------------------------------------------------------------
MODULE_CASES = [
    "module M {}",
    "module M { public def helper() { 42 } }",
    "module M { private def helper() {} }",
]

@pytest.mark.parametrize("src", MODULE_CASES)
def test_module(src):
    _compile(src)


# ---------------------------------------------------------------------------
# Multi-statement programs
# ---------------------------------------------------------------------------
MULTI_CASES = [
    "let x = 1\nlet y = 2\nlet z = x + y",
    "let mut x = 0\nx = x + 1\nx = x + 1",
    "def f() { return 1 }\ndef g() { return 2 }\nlet x = f() + g()",
    "let x = 1\nif x > 0 { let y = x }",
    "for i in 0..3 {\n  let x = i * 2\n}",
    "let x = [1, 2, 3]\nlet y = x.length",
    "let x = {\"a\": 1}\nlet y = x[\"a\"]",
    "let x = (1, 2)\nlet (a, b) = x",
    "const PI = 3.14\ndef area(r: float) -> float { return PI * r * r }",
    "type Score = int\nlet x: Score = 100",
    "def f() {\n  try {\n    return 1\n  } catch e {\n    return 0\n  }\n}",
    "let x = [1, 2, 3]\nfor i in x { print(i) }",
    "let mut count = 0\nfor i in 0..10 {\n  if i % 2 == 0 {\n    count += 1\n  }\n}",
]

@pytest.mark.parametrize("src", MULTI_CASES)
def test_multi_statement(src):
    _compile(src)
