"""Tests locking in Aura's standardized (canonical) syntax.

Removed spellings must raise a clear ``SyntaxError`` rather than silently
miscompiling, and the canonical forms must keep working. See `docs/language-reference/grammar.md`
and its Appendix A.
"""
import contextlib
import io
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Tokenizer, Parser  # noqa: E402
from transpiler.transformer import Transformer  # noqa: E402


def parse(source):
    return Parser(Tokenizer(source).tokenize()).parse()


def transpile(source):
    return Transformer().transform(parse(source))


def run_aura(source):
    program = parse(source)
    code = Transformer().transform(program)
    namespace = {"__name__": "__aura_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, namespace)
    return buffer.getvalue()


# ============================================================================
# Removed spellings raise clear errors
# ============================================================================

@pytest.mark.parametrize("source,needle", [
    ("fn f() {}", "use 'def'"),
    ("class C { def init() { } }", "use 'new'"),
    ("let x = !true", "use 'not'"),
    ("let x = a && b", "use 'and'"),
    ("let x = a || b", "use 'or'"),
    ("let x = null", "use 'none'"),
    ("def f<T>() {}", "brackets"),
    ("let private x = 1", "before 'let'"),
    ("let x = 1...10", "unexpected '...'"),
    ("volatily x", "volatile"),
])
def test_removed_spelling_errors(source, needle):
    with pytest.raises(SyntaxError) as exc:
        parse(source)
    assert needle in str(exc.value)


# ============================================================================
# Canonical spellings work
# ============================================================================

def test_canonical_def():
    assert "def f" in transpile("def f() { }")


def test_canonical_new():
    out = run_aura(
        "class P { let x: int = 0\n  def new(x: int) { self.x = x } }\nprint(P(3).x)"
    )
    assert out == "3\n"


def test_canonical_not():
    assert run_aura("print(not false)") == "True\n"


def test_canonical_and_or():
    assert run_aura("print(true and false)") == "False\n"
    assert run_aura("print(true or false)") == "True\n"


def test_canonical_none():
    assert run_aura("print(none is none)") == "True\n"


def test_canonical_bracket_generics():
    code = transpile("def f[T]() { }")
    assert "def f" in code
    code = transpile("class Box[T] { let item: T = none }")
    assert "_aura_Generic" in code


def test_canonical_modifier_prefix():
    code = transpile("private let x = 1")
    assert "x = 1" in code


# ============================================================================
# Operator precedence corrections
# ============================================================================

def test_power_binds_tighter_than_unary_minus():
    # -2 ** 2 == -(2 ** 2) == -4, matching Python and the spec.
    assert run_aura("print(-2 ** 2)") == "-4\n"


def test_unary_binds_tighter_than_mul():
    assert run_aura("print(-2 * 3)") == "-6\n"


def test_not_binds_looser_than_in():
    # `not x in y` == `not (x in y)`, matching Python.
    assert run_aura("print(not 3 in [1, 2])") == "True\n"


def test_range_operators_still_work():
    assert run_aura("print(list(1..4))") == "[1, 2, 3, 4]\n"
    assert run_aura("print(list(1..<4))") == "[1, 2, 3]\n"


# ============================================================================
# Canonical destructuring
# ============================================================================

def test_list_destructuring_rest():
    out = run_aura("let [first, ...rest] = [1, 2, 3]\nprint(first)\nprint(rest)")
    assert out == "1\n[2, 3]\n"


def test_tuple_destructuring():
    out = run_aura("let (a, b) = (10, 20)\nprint(a + b)")
    assert out == "30\n"