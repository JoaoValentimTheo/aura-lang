"""Regression coverage for the second audit round (0.1.0a18).

Targets the correctness, security and performance findings left over after the
first hardening pass: `yield` precedence, f-string conversions and validation,
typed destructuring rejection, single-evaluation coalescing, the SSRF
empty-host bypass, the CGNAT gap, the forgeable reference PQC backend warning,
`AuraDict` key/method collisions, and the fused multi-walk traversals.
"""
import contextlib
import copy
import io
import pickle
import textwrap
import warnings

import pytest

from aura.parser.to_ast import Parser, Tokenizer
from aura.stdlib.collections import AuraDict
from aura.transpiler.transformer import Transformer


def gen(source):
    ast = Parser(Tokenizer(textwrap.dedent(source).lstrip()).tokenize()).parse()
    return Transformer().transform(ast)


def compiles(source):
    code = gen(source)
    compile(code, '<test>', 'exec')
    return code


# ---------------------------------------------------------------------------
# `yield` precedence and grouping
# ---------------------------------------------------------------------------

def test_yield_binds_looser_than_addition():
    # `yield 1 + 1` must mean `yield (1 + 1)`, not `(yield 1) + 1`.
    code = compiles('def gen() {\n  yield 1 + 1\n}\n')
    assert 'yield (1 + 1)' in code
    assert '(yield 1) + 1' not in code


def test_yield_tuple_operand_is_parenthesized():
    code = compiles('def gen() {\n  yield 1, 2\n}\n')
    assert 'yield (1, 2)' in code


def test_bare_yield_is_unchanged():
    code = compiles('def gen() {\n  yield\n}\n')
    assert 'yield' in code
    compiled = compile(code, '<test>', 'exec')
    ns = {}
    exec(compiled, ns)
    assert list(ns['gen']()) == [None]


def test_yield_runtime_value():
    code = gen('def gen() {\n  yield 1 + 1\n}\n')
    ns = {}
    exec(compile(code, '<test>', 'exec'), ns)
    assert list(ns['gen']()) == [2]


# ---------------------------------------------------------------------------
# f-string conversions and validation
# ---------------------------------------------------------------------------

def test_fstring_repr_conversion_is_kept():
    code = compiles('def main() {\n  let v = "x"\n  print(f"{v!r}")\n}\n')
    assert '!r' in code


def test_fstring_conversion_runtime():
    code = gen('def main() {\n  let v = "x"\n  print(f"{v!r}")\n}\n')
    ns = {}
    exec(compile(code, '<test>', 'exec'), ns)
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        ns['main']()
    assert buf.getvalue().strip() == "'x'"


def test_fstring_format_spec_still_works():
    code = gen('def main() {\n  let n = 42\n  print(f"{n:>5}")\n}\n')
    assert '{n:>5}' in code


def test_fstring_invalid_expression_is_a_diagnostic():
    with pytest.raises(SyntaxError) as info:
        gen('def main() {\n  print(f"{1+}")\n}\n')
    assert 'f-string' in str(info.value)


# ---------------------------------------------------------------------------
# Destructuring
# ---------------------------------------------------------------------------

def test_typed_sequence_destructuring_is_rejected():
    with pytest.raises(SyntaxError) as info:
        gen('let (a, b: Int) = (1, 2)\n')
    assert 'destructuring' in str(info.value)


def test_plain_sequence_destructuring_still_works():
    code = compiles('let (a, b) = (1, 2)\n')
    assert 'a' in code and 'b' in code


def test_dict_destructuring_alias_still_works():
    code = compiles('let {name: n} = {"name": "A"}\n')
    assert "['name']" in code


# ---------------------------------------------------------------------------
# Coalescing evaluates the left side once
# ---------------------------------------------------------------------------

def test_null_coalesce_uses_helper():
    code = compiles('let x = a ?? b\n')
    assert '_aura_null_coalesce(a, b)' in code


def test_elvis_uses_helper():
    code = compiles('let x = a ?: b\n')
    assert '_aura_elvis(a, b)' in code


def test_chained_elvis_evaluates_once():
    code = gen('let c = a ?: b ?: c\n')
    # `a` appears exactly once as the argument, not duplicated in a condition.
    assert '_aura_elvis(_aura_elvis(' in code
    assert 'a if a' not in code


def test_coalesce_side_effect_evaluated_once():
    code = gen(
        'def ping() {\n  print("ping")\n  return none\n}\n'
        'def main() {\n  let v = ping() ?? 1\n}\n'
    )
    assert 'ping()' in code
    assert code.count('ping()') == 2  # def + call, condition does not repeat it


# ---------------------------------------------------------------------------
# SSRF hardening
# ---------------------------------------------------------------------------

def test_empty_host_url_is_blocked():
    from aura.stdlib import http
    with pytest.raises(ValueError):
        http._validate_url('http://:50587/')


def test_cgnat_address_is_blocked():
    from aura.stdlib import http
    with pytest.raises(ValueError):
        http._validate_url('http://100.64.0.1/')


def test_loopback_still_blocked():
    from aura.stdlib import http
    with pytest.raises(ValueError):
        http._validate_url('http://127.0.0.1/')


# ---------------------------------------------------------------------------
# Reference PQC backend warns
# ---------------------------------------------------------------------------

def test_reference_backend_warns():
    from aura.stdlib import crypto_backend
    if crypto_backend._select().production:
        pytest.skip('a production PQC backend is installed')
    crypto_backend._WARNED_REFERENCE = False
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter('always')
        kp = crypto_backend.kem_keypair('ML-KEM-768')
    assert kp
    assert any(issubclass(w.category, RuntimeWarning) for w in caught)


# ---------------------------------------------------------------------------
# AuraDict key/method collision
# ---------------------------------------------------------------------------

def test_auradict_key_shadows_method():
    d = AuraDict({'keys': 1, 'values': 2})
    assert d.keys == 1
    assert d.values == 2


def test_auradict_methods_still_work_without_collision():
    d = AuraDict({'a': 1})
    assert list(d.keys()) == ['a']
    assert list(d.items()) == [('a', 1)]


def test_auradict_dunder_and_pickle_survive():
    d = AuraDict({'keys': 1})
    assert type(d).__name__ == 'AuraDict'
    assert copy.copy(d) == d
    assert pickle.loads(pickle.dumps(d)) == d
