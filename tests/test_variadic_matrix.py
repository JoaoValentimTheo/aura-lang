"""Phase 1 — full ``*args`` / ``**kwargs`` context matrix.

The roadmap requires ``*args``/``**kwargs`` to work in *every* context the
grammar permits (GRAMMAR.md 3.2 ``param_list`` and 6.7 ``arg_list``): function
definitions, calls, decorators, lambdas, and collection literals. This module
exercises each context end to end (parse → transpile → execute) and pins the
diagnostics for the forms the grammar rejects.
"""

import contextlib
import io
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Parser, Tokenizer  # noqa: E402
from transpiler.transformer import Transformer  # noqa: E402


def transpile(source: str) -> str:
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    return Transformer().transform(program)


def run_aura(source: str):
    """Transpile, execute, and return (stdout, generated_python)."""
    code = transpile(f"{source}\nmain()")
    namespace = {"__name__": "__aura_variadic__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, namespace)  # noqa: S102 - test harness
    return buffer.getvalue().strip(), code


def parse_error(source: str) -> str:
    with pytest.raises(Exception) as info:
        transpile(source)
    return str(info.value)


# ============================================================================
# Definitions
# ============================================================================

def test_def_star_args():
    out, _ = run_aura("def f(*args) { return len(args) }\ndef main() { print(f(1, 2, 3)) }")
    assert out == "3"


def test_def_star_kwargs():
    out, _ = run_aura("def f(**kw) { return len(kw) }\ndef main() { print(f(a=1, b=2)) }")
    assert out == "2"


def test_def_args_and_kwargs():
    source = (
        "def f(*args, **kw) { return len(args) * 10 + len(kw) }\n"
        "def main() { print(f(1, 2, a=3)) }"
    )
    out, _ = run_aura(source)
    assert out == "21"


def test_def_keyword_only_after_star():
    source = (
        "def f(a, *, mode) { return a + mode }\n"
        "def main() { print(f(1, mode=2)) }"
    )
    out, _ = run_aura(source)
    assert out == "3"


def test_def_keyword_only_default():
    source = (
        "def f(*args, mode='x') { return len(args) }\n"
        "def main() { print(f(1, 2, 3)) }"
    )
    out, _ = run_aura(source)
    assert out == "3"


def test_def_positional_then_variadic():
    source = (
        "def f(first, *rest) { return rest }\n"
        "def main() { print(len(f(1, 2, 3, 4))) }"
    )
    out, _ = run_aura(source)
    assert out == "3"


# ============================================================================
# Calls
# ============================================================================

def test_call_positional_spread_list():
    source = (
        "def add(a, b, c) { return a + b + c }\n"
        "def main() { let xs = [1, 2, 3]\n print(add(*xs)) }"
    )
    out, _ = run_aura(source)
    assert out == "6"


def test_call_keyword_spread_dict():
    source = (
        "def add(a, b) { return a + b }\n"
        "def main() { let kw = {'a': 1, 'b': 2}\n print(add(**kw)) }"
    )
    out, _ = run_aura(source)
    assert out == "3"


def test_call_mixed_positional_and_spread():
    source = (
        "def f(a, b, c) { return a + b * 10 + c * 100 }\n"
        "def main() { let rest = [2, 3]\n print(f(1, *rest)) }"
    )
    out, _ = run_aura(source)
    assert out == "321"


def test_call_adaptive_spread_dict():
    source = (
        "def f(a, b) { return a + b }\n"
        "def main() { let d = {'a': 1, 'b': 2}\n print(f(...d)) }"
    )
    out, _ = run_aura(source)
    assert out == "3"


def test_call_adaptive_spread_iterable():
    source = (
        "def f(a, b) { return a + b }\n"
        "def main() { let xs = [4, 5]\n print(f(...xs)) }"
    )
    out, _ = run_aura(source)
    assert out == "9"


# ============================================================================
# Decorators
# ============================================================================

def test_decorator_spread_arguments():
    """`@deco(*xs, **kw)` keeps the spreads instead of wrapping them."""
    source = "@deco(*[1], **{'x': 2})\ndef target() { return 1 }"
    code = transpile(source)
    assert "@deco(*[1], **" in code
    assert "( * " not in code
    assert "(* [" not in code


def test_decorator_positional_spread():
    source = (
        "def register(*tags) { return (fn) => fn }\n"
        "@register(*['a', 'b'])\n"
        "def target() { return 1 }\n"
        "def main() { print(target()) }"
    )
    out, _ = run_aura(source)
    assert out == "1"


# ============================================================================
# Lambdas (grammar 6.4 shares param_list with def)
# ============================================================================

def test_lambda_star_args():
    source = (
        "def main() { let f = (*a) => len(a)\n print(f(1, 2, 3)) }"
    )
    out, _ = run_aura(source)
    assert out == "3"


def test_lambda_star_kwargs():
    source = (
        "def main() { let f = (**k) => len(k)\n print(f(x=1, y=2)) }"
    )
    out, _ = run_aura(source)
    assert out == "2"


def test_lambda_args_and_kwargs():
    source = (
        "def main() { let f = (*a, **k) => len(a) * 10 + len(k)\n"
        " print(f(1, 2, x=3)) }"
    )
    out, _ = run_aura(source)
    assert out == "21"


def test_lambda_positional_then_variadic():
    source = (
        "def main() { let f = (a, *rest) => len(rest)\n print(f(1, 2, 3)) }"
    )
    out, _ = run_aura(source)
    assert out == "2"


def test_lambda_keyword_only():
    source = (
        "def main() { let f = (a, *, b) => a + b\n print(f(1, b=2)) }"
    )
    out, _ = run_aura(source)
    assert out == "3"


def test_lambda_default_params():
    source = (
        "def main() { let f = (a, b=10) => a + b\n print(f(1), f(1, 2)) }"
    )
    out, _ = run_aura(source)
    assert out == "11 3"


def test_lambda_block_body_with_args():
    source = (
        "def main() { let f = (*a) => { return len(a) }\n print(f(1, 2, 3, 4)) }"
    )
    out, _ = run_aura(source)
    assert out == "4"


def test_lambda_plain_tuple_is_not_a_lambda():
    source = "def main() { let t = (1, 2, 3)\n print(len(t)) }"
    out, _ = run_aura(source)
    assert out == "3"


# ============================================================================
# Collection literals
# ============================================================================

def test_list_spread_literal():
    source = (
        "def main() { let a = [1, 2]\n let b = [*a, 3]\n print(len(b)) }"
    )
    out, _ = run_aura(source)
    assert out == "3"


def test_dict_spread_literal():
    source = (
        "def main() { let a = {'x': 1}\n let b = {**a, 'y': 2}\n"
        " print(len(b)) }"
    )
    out, _ = run_aura(source)
    assert out == "2"


def test_set_spread_literal():
    source = "def main() { let a = {1, 2}\n let b = {*a, 3}\n print(len(b)) }"
    out, _ = run_aura(source)
    assert out == "3"


def test_tuple_spread_literal():
    source = (
        "def main() { let a = [1, 2]\n let t = (*a, 3)\n print(len(t)) }"
    )
    out, _ = run_aura(source)
    assert out == "3"


# ============================================================================
# Rejected forms produce Aura diagnostics, not invalid Python
# ============================================================================

def test_reject_two_star_lambda_params():
    msg = parse_error("def main() { let f = (*a, *b) => a }")
    assert "only one '*'" in msg


def test_reject_duplicate_kwargs_lambda_params():
    msg = parse_error("def main() { let f = (**a, **b) => a }")
    assert "only one '**'" in msg


def test_reject_bare_star_in_tuple():
    msg = parse_error("def main() { let t = (1, *) }")
    assert "bare '*'" in msg


def test_reject_bare_return_spread():
    msg = parse_error("def f(a) { return *a }")
    assert "spread is not allowed" in msg


# ============================================================================
# Matrix completion: methods, composition, and shadowing
# ============================================================================

def test_method_star_args():
    source = (
        "class T { public def m(self, *a) { return len(a) } }\n"
        "def main() { print(T().m(1, 2, 3)) }"
    )
    out, _ = run_aura(source)
    assert out == "3"


def test_method_star_kwargs():
    source = (
        "class T { public def m(self, **k) { return len(k) } }\n"
        "def main() { print(T().m(a=1, b=2)) }"
    )
    out, _ = run_aura(source)
    assert out == "2"


def test_star_args_forwarded_between_functions():
    source = (
        "def g(*a) { return len(a) }\n"
        "def f(*a) { return g(*a) }\n"
        "def main() { print(f(1, 2, 3, 4)) }"
    )
    out, _ = run_aura(source)
    assert out == "4"


def test_spread_of_a_call_result():
    source = (
        "def f() { return [1, 2] }\n"
        "def main() { let x = [*f(), 3]\n print(len(x)) }"
    )
    out, _ = run_aura(source)
    assert out == "3"


def test_kwargs_spread_of_a_call_result():
    source = (
        "def f() { return {'a': 1} }\n"
        "def main() { let d = {**f(), 'b': 2}\n print(len(d)) }"
    )
    out, _ = run_aura(source)
    assert out == "2"


def test_variadic_parameter_shadowing_a_local():
    source = (
        "def f(*args) { return args }\n"
        "def main() { let args = 5\n print(f(1, 2), args) }"
    )
    out, _ = run_aura(source)
    assert out == "(1, 2) 5"


def test_lambda_positional_kwonly_and_variadic_together():
    source = (
        "def main() { let f = (a, *rest, mode='x') => len(rest)\n"
        " print(f(1, 2, 3)) }"
    )
    out, _ = run_aura(source)
    assert out == "2"


def test_decorator_spread_on_member():
    source = (
        "class D { public def reg(self, *a) { return (fn) => fn } }\n"
        "let d = D()\n"
        "@d.reg(*[1, 2])\n"
        "def t() { return 1 }\n"
        "def main() { print(t()) }"
    )
    out, _ = run_aura(source)
    assert out == "1"
