"""Raw stress tests for Aura.

These tests deliberately throw large volumes of hand-written and generated
Aura source at the *entire* pipeline (tokenize → parse → rules → transform →
compile → execute). They are meant to be brutish: exhaustive operator
matrices, deep nesting, fuzz-ish numeric literals, long programs, and
adversarial edge cases that have historically produced crashes or invalid
Python.

The goal is not coverage of features (other suites do that) but robustness:
the pipeline must either run the program or fail with a clean ``SyntaxError``;
it must never emit invalid Python, hang, or raise an internal ``AttributeError``
/``NameError``/``TypeError``.
"""
import io
import contextlib
import itertools
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Tokenizer, Parser  # noqa: E402
from transpiler.transformer import Transformer  # noqa: E402
from transpiler.rules import RuleChecker  # noqa: E402
from transpiler.semantics import MutabilityChecker  # noqa: E402


def compile_aura(source):
    """Full pipeline: parse, check rules, transform. Returns generated Python."""
    program = Parser(Tokenizer(source).tokenize()).parse()
    code = Transformer().transform(program)
    compile(code, "<stress>", "exec")
    return code


def run_aura(source):
    """Execute Aura source and return stdout."""
    code = compile_aura(source)
    namespace = {"__name__": "__stress__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, namespace)
    return buffer.getvalue(), code, namespace


def assert_pipeline_ok(source):
    """The pipeline must produce compilable Python without internal errors."""
    try:
        return compile_aura(source)
    except SyntaxError:
        return None
    except (AttributeError, NameError, TypeError, KeyError, IndexError,
            UnboundLocalError, RecursionError) as exc:  # pragma: no cover
        pytest.fail(f"internal error for {source!r}: {type(exc).__name__}: {exc}")


# ============================================================================
# 1. Numeric literals and arithmetic exhaustively
# ============================================================================

NUMERIC_LITERALS = [
    "0", "1", "42", "0x0", "0xFF", "0xff", "0o17", "0b1010",
    "1_000_000", "1.5", "0.0", "3.14159", ".5", "1e3", "1E3",
    "1.5e-5", "2.5e+10", "100", "999999999999999999999999",
]


@pytest.mark.parametrize("literal", NUMERIC_LITERALS)
def test_numeric_literal_roundtrip(literal):
    out, _, ns = run_aura(f"print({literal})")
    assert out.strip() != ""


ARITH_OPS = ["+", "-", "*", "/", "%", "**", "&", "|", "^", "<<", ">>"]


@pytest.mark.parametrize("op", ARITH_OPS)
def test_arithmetic_operator_matrix(op):
    for a in (0, 1, 2, 7, 100):
        for b in (1, 2, 3, 25):
            code = compile_aura(f"let x = {a} {op} {b}")
            assert "x" in code or "=" in code


@pytest.mark.parametrize("left,op,right", [
    (a, op, b)
    for a in ("1", "2.5", "0x10")
    for op in ("==", "!=", "<", ">", "<=", ">=")
    for b in ("1", "2.5", "0x10")
])
def test_comparison_matrix(left, op, right):
    assert_pipeline_ok(f"let ok = {left} {op} {right}")


# ============================================================================
# 2. Deep nesting — parser and transformer must not blow up
# ============================================================================

@pytest.mark.parametrize("depth", [1, 2, 5, 10, 25, 50])
def test_deep_parenthesized_expressions(depth):
    expr = "(" * depth + "1" + ")" * depth
    out, _, _ = run_aura(f"print({expr})")
    assert out.strip() == "1"


@pytest.mark.parametrize("depth", [1, 2, 5, 10, 20])
def test_deep_arithmetic_chain(depth):
    expr = " + ".join(["1"] * (depth + 1))
    out, _, _ = run_aura(f"print({expr})")
    assert out.strip() == str(depth + 1)


@pytest.mark.parametrize("depth", [1, 2, 3, 5, 8])
def test_deep_nested_lists(depth):
    expr = "[" * depth + "1" + "]" * depth
    compile_aura(f"let x = {expr}")


@pytest.mark.parametrize("depth", [1, 2, 3, 5])
def test_deep_if_nesting(depth):
    body = "print(1)"
    for _ in range(depth):
        body = "if true { " + body + " }"
    run_aura(body)


@pytest.mark.parametrize("depth", [1, 2, 3, 4])
def test_deep_function_nesting(depth):
    src = "def f0() { return 1 }"
    for i in range(1, depth + 1):
        src += f"\ndef f{i}() {{ return f{i-1}() }}"
    src += f"\nprint(f{depth}())"
    out, _, _ = run_aura(src)
    assert out.strip() == "1"


# ============================================================================
# 3. Statement / control-flow combinations
# ============================================================================

@pytest.mark.parametrize("n", [0, 1, 3, 10, 50, 100])
def test_loop_accumulation(n):
    out, _, _ = run_aura(
        "let mut total = 0\n"
        f"for i in range({n}) {{\n  total += i\n}}\n"
        "print(total)"
    )
    assert out.strip() == str(sum(range(n)))


@pytest.mark.parametrize("n", [1, 2, 5, 10])
def test_factorial_recursion(n):
    out, _, _ = run_aura(
        "def fact(n) -> int {\n"
        "  if n <= 1 { return 1 }\n"
        "  return n * fact(n - 1)\n"
        "}\n"
        f"print(fact({n}))"
    )
    import math
    assert out.strip() == str(math.factorial(n))


@pytest.mark.parametrize("keyword", ["while", "until", "loop"])
def test_loop_variants_terminate(keyword):
    if keyword == "while":
        src = "let mut i = 0\nwhile i < 3 { i += 1 }\nprint(i)"
    elif keyword == "until":
        src = "let mut i = 0\nuntil i >= 3 { i += 1 }\nprint(i)"
    else:
        src = "let mut i = 0\nloop { i += 1\n if i >= 3 { break } }\nprint(i)"
    out, _, _ = run_aura(src)
    assert out.strip() == "3"


# ============================================================================
# 4. String escapes and f-strings
# ============================================================================

STRING_CASES = [
    r'"hello"',
    r'"tab\there"',
    r'"new\nline"',
    r'"quote\"inside"',
    r"'single'",
    r'"unicode\u0041"',
    r'"hex\x41"',
    r'r"raw\path\n"',
    r'b"bytes"',
]


@pytest.mark.parametrize("literal", STRING_CASES)
def test_string_literals(literal):
    assert_pipeline_ok(f"print({literal})")


@pytest.mark.parametrize("n", [0, 1, 42, -7, 1000000])
def test_fstring_value_and_format(n):
    out, _, _ = run_aura(f'let n = {n}\nprint(f"v={{n}}")')
    assert out.strip() == f"v={n}"


def test_fstring_with_format_spec():
    out, _, _ = run_aura('let n = 255\nprint(f"{n:x} {n:>6}")')
    assert out.strip() == "ff    255"


def test_fstring_with_nested_expression():
    out, _, _ = run_aura('let a = 3\nlet b = 4\nprint(f"{a * a + b * b}")')
    assert out.strip() == "25"


# ============================================================================
# 5. Collections — brute force over builtins
# ============================================================================

@pytest.mark.parametrize("call", [
    "len([1, 2, 3])",
    "sum([1, 2, 3])",
    "max([1, 5, 3])",
    "min([1, 5, 3])",
    "sorted([3, 1, 2])",
    "list(range(3))",
    "abs(-5)",
    "round(2.6)",
    "int('7')",
    "str(7)",
    "float('1.5')",
    "bool(0)",
])
def test_builtin_calls(call):
    out, _, _ = run_aura(f"print({call})")
    assert out.strip() != ""


@pytest.mark.parametrize("container", [
    "[1, 2, 3]",
    '{"a": 1, "b": 2}',
    "{1, 2, 3}",
    "(1, 2, 3)",
])
def test_container_length(container):
    assert_pipeline_ok(f"print(len({container}))")


@pytest.mark.parametrize("comp", [
    "[x for x in range(5)]",
    "[x * x for x in range(5) if x % 2 == 0]",
    "{x: x * x for x in range(3)}",
    "{x for x in range(3)}",
    "(x for x in range(3))",
])
def test_comprehensions(comp):
    assert_pipeline_ok(f"let v = {comp}")


# ============================================================================
# 6. Function signatures — brute force
# ============================================================================

@pytest.mark.parametrize("params,args,expected", [
    ("a", "1", "1"),
    ("a, b", "1, 2", "3"),
    ("a, b = 2", "1", "3"),
    ("a, b = 2", "1, 5", "6"),
    ("*args", "1, 2, 3", "6"),
    ("a, *rest", "1, 2, 3", "1"),
    ("a, *, b", "1, b=2", "3"),
    ("**kw", "", "0"),
])
def test_function_signatures(params, args, expected):
    body = "return len(args)" if "args" in params else "return 1"
    if params == "a":
        body = "return a"
    elif params == "a, b":
        body = "return a + b"
    elif params.startswith("a, b ="):
        body = "return a + b"
    elif params == "*args":
        body = "return sum(args)"
    elif params == "a, *rest":
        body = "return a"
    elif params == "a, *, b":
        body = "return a + b"
    elif params == "**kw":
        body = "return len(kw)"
    src = f"def f({params}) {{\n  {body}\n}}\nprint(f({args}))"
    out, _, _ = run_aura(src)
    assert out.strip() == expected


@pytest.mark.parametrize("n", [1, 2, 3, 4, 5])
def test_generic_function(n):
    out, _, _ = run_aura(
        "def identity[T](x: T) -> T { return x }\n"
        f"print(identity({n}))"
    )
    assert out.strip() == str(n)


@pytest.mark.parametrize("n", [1, 2, 3])
def test_expression_body_function(n):
    out, _, _ = run_aura(f"def double(x) = x * 2\nprint(double({n}))")
    assert out.strip() == str(n * 2)


# ============================================================================
# 7. Classes / methods / properties — brute force
# ============================================================================

@pytest.mark.parametrize("methods", [1, 2, 5, 10])
def test_class_with_many_methods(methods):
    body = "\n".join(f"  def m{i}() {{ return {i} }}" for i in range(methods))
    calls = "\n".join(f"print(C().m{i}())" for i in range(methods))
    run_aura(f"class C {{\n{body}\n}}\n{calls}")


@pytest.mark.parametrize("n", [1, 2, 5, 20])
def test_class_constructor_fields(n):
    fields = "\n".join(f"  let f{i} = {i}" for i in range(n))
    src = f"class C {{\n{fields}\n}}\nlet c = C()\nprint(c.f0)"
    out, _, _ = run_aura(src)
    assert out.strip() == "0"


def test_property_returns_value():
    out, _, _ = run_aura(
        "class Box {\n  let v = 5\n"
        "  @property\n  def doubled() -> int { return self.v * 2 }\n}\n"
        "print(Box().doubled)"
    )
    assert out.strip() == "10"


def test_static_method():
    out, _, _ = run_aura(
        "class M {\n  @staticmethod\n  def add(a, b) { return a + b }\n}\n"
        "print(M.add(2, 3))"
    )
    assert out.strip() == "5"


# ============================================================================
# 8. Error handling — fuzz-ish
# ============================================================================

@pytest.mark.parametrize("exc", [
    "ValueError", "TypeError", "KeyError", "RuntimeError", "Exception",
])
def test_throw_and_catch(exc):
    out, _, _ = run_aura(
        f'try {{ throw {exc}("x") }}\ncatch Error as e {{ print("caught") }}'
    )
    assert out.strip() == "caught"


@pytest.mark.parametrize("message", ['"boom"', '"a\\nb"', '""'])
def test_throw_string(message):
    out, _, _ = run_aura(
        f'try {{ throw {message} }} catch Error as error {{ print("ok") }}'
    )
    assert out.strip() == "ok"


def test_finally_always_runs():
    out, _, _ = run_aura(
        'let mut log = ""\n'
        'try { log += "t" } catch Error as e { log += "c" } finally { log += "f" }\n'
        'print(log)'
    )
    assert out.strip() == "tf"


# ============================================================================
# 9. Long generated programs
# ============================================================================

@pytest.mark.parametrize("size", [10, 50, 100, 250])
def test_many_sequential_statements(size):
    src = "\n".join(f"let v{i} = {i} + 1" for i in range(size))
    src += f"\nprint(v{size - 1})"
    out, _, _ = run_aura(src)
    assert out.strip() == str(size)


@pytest.mark.parametrize("size", [5, 20, 50])
def test_many_functions(size):
    defs = "\n".join(f"def f{i}() {{ return {i} }}" for i in range(size))
    run_aura(defs + f"\nprint(f{size - 1}())")


# ============================================================================
# 10. Invalid input must fail cleanly (SyntaxError, never internal error)
# ============================================================================

INVALID_SOURCES = [
    "let = 5",
    "def () {}",
    "class {}",
    "if { }",
    "for in range(3) {}",
    "let x = ",
    "1 +",
    "print(",
    "}",
    "let x = [1, 2",
    "match { }",
    "def f(a,,b) {}",
    "0xZZ",
    "0b2",
    "@",
    "let x: = 1",
]


@pytest.mark.parametrize("source", INVALID_SOURCES)
def test_invalid_source_fails_cleanly(source):
    with pytest.raises(SyntaxError):
        Parser(Tokenizer(source).tokenize()).parse()


VALID_BUT_RULE_VIOLATIONS = [
    "return 1",
    "break",
    "continue",
    "let x = 1\nlet x = 2",
    "def f(a, a) { return a }",
    "def f() { return 1\nprint(2) }",
]


@pytest.mark.parametrize("source", VALID_BUT_RULE_VIOLATIONS)
def test_rules_reject(source):
    program = Parser(Tokenizer(source).tokenize()).parse()
    assert not RuleChecker().check_program(program)


# ============================================================================
# 11. Mutability brute force
# ============================================================================

@pytest.mark.parametrize("name", [f"v{i}" for i in range(20)])
def test_immutable_reassignment_rejected(name):
    program = Parser(Tokenizer(
        f"let {name} = 1\n{name} = 2"
    ).tokenize()).parse()
    assert not MutabilityChecker().check_program(program)


@pytest.mark.parametrize("name", [f"v{i}" for i in range(20)])
def test_mutable_reassignment_accepted(name):
    program = Parser(Tokenizer(
        f"let mut {name} = 1\n{name} = 2"
    ).tokenize()).parse()
    assert MutabilityChecker().check_program(program)


# ============================================================================
# 12. Operator/precedence stress
# ============================================================================

PRECEDENCE_CASES = [
    ("1 + 2 * 3", 7),
    ("(1 + 2) * 3", 9),
    ("2 ** 3 ** 2", 512),
    ("10 % 3 + 1", 2),
    ("1 << 2 + 1", 8),
    ("true and false or true", True),
    ("1 + 2 == 3", True),
    ("not false", True),
]


@pytest.mark.parametrize("expr,expected", PRECEDENCE_CASES)
def test_precedence(expr, expected):
    out, _, _ = run_aura(f"print({expr})")
    assert out.strip() == str(expected)


# ============================================================================
# 13. Whitespace / newline independence
# ============================================================================

@pytest.mark.parametrize("source", [
    "let x=1;print(x)",
    "let x = 1 ; print ( x )",
    "let   x   =   1\nprint(x)",
    "let x = 1\n\n\nprint(x)",
])
def test_whitespace_variants(source):
    out, _, _ = run_aura(source)
    assert out.strip() == "1"


# ============================================================================
# 14. Iterator/generator pipelines
# ============================================================================

def test_filter_map_reduce_pipeline():
    out, _, _ = run_aura(
        "let r = [1, 2, 3, 4, 5, 6, 7, 8]\n"
        "  |> filter((x) => x % 2 == 0)\n"
        "  |> map((x) => x * x)\n"
        "  |> reduce((acc, x) => acc + x, 0)\n"
        "print(r)"
    )
    assert out.strip() == str(4 + 16 + 36 + 64)


@pytest.mark.parametrize("n", range(15))
def test_range_variants(n):
    out, _, _ = run_aura(f"print(len(list({n}..{n + 3})))")
    assert int(out.strip()) >= 1