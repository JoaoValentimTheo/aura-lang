"""Differential testing: Aura vs the equivalent Python program.

For a generated program, we build both the Aura source and the Python it
*should* behave like, run both, and require identical stdout. This catches
miscompiles that a round-trip test would miss (the transpiler and the reference
implementation are independent).

Every program defines ``main`` and prints only deterministic values.
"""
import contextlib
import io
import sys
from pathlib import Path

from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.transformer import Transformer  # noqa: E402

SETTINGS = settings(
    max_examples=150,
    deadline=None,
    suppress_health_check=[HealthCheck.too_slow],
)

# Aura deliberately has no floor-division operator (`//` starts a line
# comment), so the differential suite exercises the operators that do exist.
ASSIGN_OPS = {
    "+": lambda a, b: a + b,
    "-": lambda a, b: a - b,
    "*": lambda a, b: a * b,
    "%": lambda a, b: a % b,
    "**": lambda a, b: a ** b,
}

AUG_OPS = ["+=", "-=", "*=", "%="]


def run_python(source):
    buffer = io.StringIO()
    namespace = {"__name__": "__ref__"}
    with contextlib.redirect_stdout(buffer):
        exec(compile(source, "<ref>", "exec"), namespace)
    return buffer.getvalue()


def run_aura(source):
    program = Parser(Tokenizer(source).tokenize()).parse()
    code = Transformer().transform(program)
    namespace = {"__name__": "__aura_diff__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, "<aura>", "exec"), namespace)
        main = namespace.get("main")
        if callable(main):
            main()
    return buffer.getvalue()


# ---------------------------------------------------------------------------
# Integer arithmetic
# ---------------------------------------------------------------------------

@st.composite
def _int_expr(draw, depth=0):
    """Generate (aura_expr, python_expr) with identical semantics."""
    if depth >= 3:
        v = draw(st.integers(min_value=-50, max_value=50))
        return str(v), str(v)
    op = draw(st.sampled_from(list(ASSIGN_OPS)))
    a_aura, a_py = draw(_int_expr(depth + 1))
    if op == "**":
        b = draw(st.integers(min_value=0, max_value=3))
        b_aura = b_py = str(b)
    elif op == "%":
        b = draw(st.integers(min_value=1, max_value=20))
        b_aura = b_py = str(b)
    else:
        b_aura, b_py = draw(_int_expr(depth + 1))
    return f"({a_aura} {op} {b_aura})", f"({a_py} {op} {b_py})"


@given(_int_expr())
@SETTINGS
def test_integer_arithmetic_matches_python(pair):
    aura_expr, py_expr = pair
    aura_src = f"def main() {{ print({aura_expr}) }}"
    py_src = f"print({py_expr})"
    assert run_aura(aura_src) == run_python(py_src)


# ---------------------------------------------------------------------------
# Boolean / comparison operators
# ---------------------------------------------------------------------------

@st.composite
def _bool_expr(draw, depth=0):
    if depth >= 2:
        v = draw(st.integers(min_value=-20, max_value=20))
        return str(v), str(v)
    op = draw(st.sampled_from(["<", ">", "<=", ">=", "==", "!="]))
    a_aura, a_py = draw(_int_expr())
    b_aura, b_py = draw(_int_expr())
    return f"({a_aura} {op} {b_aura})", f"({a_py} {op} {b_py})"


@given(st.lists(_bool_expr(), min_size=1, max_size=5))
@SETTINGS
def test_boolean_operators_match_python(exprs):
    aura_terms = [f"({' and '.join(a for a, _ in exprs)})"]
    py_terms = [f"({' and '.join(p for _, p in exprs)})"]
    aura_src = "def main() {\n" + "\n".join(
        f"  print({t})" for t in aura_terms) + "\n}"
    py_src = "\n".join(f"print({t})" for t in py_terms)
    assert run_aura(aura_src) == run_python(py_src)


# ---------------------------------------------------------------------------
# String operations
# ---------------------------------------------------------------------------

@st.composite
def _safe_string(draw):
    text = draw(st.text(alphabet=st.characters(
        min_codepoint=97, max_codepoint=122), min_size=0, max_size=10))
    return text


@given(_safe_string(), _safe_string())
@SETTINGS
def test_string_concatenation_matches_python(a, b):
    aura_src = f'def main() {{ print("{a}" + "{b}") }}'
    py_src = f'print("{a}" + "{b}")'
    assert run_aura(aura_src) == run_python(py_src)


@given(_safe_string())
@SETTINGS
def test_string_length_matches_python(text):
    aura_src = f'def main() {{ print(len("{text}")) }}'
    py_src = f'print(len("{text}"))'
    assert run_aura(aura_src) == run_python(py_src)


@given(_safe_string(), st.integers(min_value=0, max_value=5),
       st.integers(min_value=0, max_value=5))
@SETTINGS
def test_string_repetition_matches_python(text, n, _unused):
    aura_src = f'def main() {{ print("{text}" * {n}) }}'
    py_src = f'print("{text}" * {n})'
    assert run_aura(aura_src) == run_python(py_src)


# ---------------------------------------------------------------------------
# List operations
# ---------------------------------------------------------------------------

@given(st.lists(st.integers(min_value=-50, max_value=50), max_size=10))
@SETTINGS
def test_list_sum_and_length_match_python(values):
    literal = "[" + ", ".join(str(v) for v in values) + "]"
    aura_src = (
        f"def main() {{\n"
        f"  let xs = {literal}\n"
        f"  print(len(xs))\n"
        f"  print(sum(xs))\n"
        f"}}"
    )
    py_src = f"xs = {literal}\nprint(len(xs))\nprint(sum(xs))"
    assert run_aura(aura_src) == run_python(py_src)


@given(st.lists(st.integers(min_value=-50, max_value=50), max_size=10))
@SETTINGS
def test_list_indexing_matches_python(values):
    if not values:
        return
    idx = 0
    literal = "[" + ", ".join(str(v) for v in values) + "]"
    aura_src = f"def main() {{ let xs = {literal}\n print(xs[{idx}]) }}"
    py_src = f"xs = {literal}\nprint(xs[{idx}])"
    assert run_aura(aura_src) == run_python(py_src)


# ---------------------------------------------------------------------------
# Dict / set literals
# ---------------------------------------------------------------------------

@given(st.dictionaries(
    keys=st.text(alphabet=st.characters(min_codepoint=97, max_codepoint=122),
                 min_size=1, max_size=5),
    values=st.integers(min_value=-50, max_value=50),
    max_size=8))
@SETTINGS
def test_dict_lookup_matches_python(mapping):
    if not mapping:
        return
    key = sorted(mapping)[0]
    aura_entries = ", ".join(f'"{k}": {v}' for k, v in mapping.items())
    py_entries = ", ".join(f'"{k}": {v}' for k, v in mapping.items())
    aura_src = (
        f'def main() {{\n'
        f'  let d = {{{aura_entries}}}\n'
        f'  print(d["{key}"])\n'
        f'  print(len(d))\n'
        f'}}'
    )
    py_src = (
        f'd = {{{py_entries}}}\n'
        f'print(d["{key}"])\n'
        f'print(len(d))'
    )
    assert run_aura(aura_src) == run_python(py_src)


# ---------------------------------------------------------------------------
# Control flow
# ---------------------------------------------------------------------------

@given(st.lists(st.integers(min_value=-100, max_value=100), max_size=12),
       st.integers(min_value=-100, max_value=100))
@SETTINGS
def test_for_loop_accumulation_matches_python(values, start):
    literal = "[" + ", ".join(str(v) for v in values) + "]"
    aura_src = (
        f"def main() {{\n"
        f"  let mut total = {start}\n"
        f"  for v in {literal} {{ total += v }}\n"
        f"  print(total)\n"
        f"}}"
    )
    py_src = (
        f"total = {start}\n"
        f"for v in {literal}:\n    total += v\n"
        f"print(total)"
    )
    assert run_aura(aura_src) == run_python(py_src)


@given(st.integers(min_value=0, max_value=30),
       st.integers(min_value=0, max_value=30))
@SETTINGS
def test_if_else_matches_python(a, b):
    aura_src = (
        f"def main() {{\n"
        f"  if {a} < {b} {{ print('lt') }} else {{ print('ge') }}\n"
        f"}}"
    )
    py_src = (
        f"if {a} < {b}:\n    print('lt')\nelse:\n    print('ge')"
    )
    assert run_aura(aura_src) == run_python(py_src)


# ---------------------------------------------------------------------------
# Augmented assignment
# ---------------------------------------------------------------------------

@given(st.integers(min_value=-100, max_value=100),
       st.integers(min_value=1, max_value=100),
       st.sampled_from(AUG_OPS))
@SETTINGS
def test_augmented_assignment_matches_python(start, delta, op):
    py_op = op
    aura_src = (
        f"def main() {{\n"
        f"  let mut x = {start}\n"
        f"  x {op} {delta}\n"
        f"  print(x)\n"
        f"}}"
    )
    py_src = f"x = {start}\nx {py_op} {delta}\nprint(x)"
    assert run_aura(aura_src) == run_python(py_src)


# ---------------------------------------------------------------------------
# Truthiness / boolean literals
# ---------------------------------------------------------------------------

@given(st.booleans(), st.booleans())
@SETTINGS
def test_boolean_literals_match_python(a, b):
    aura = "true" if a else "false"
    py = "True" if a else "False"
    aura2 = "true" if b else "false"
    py2 = "True" if b else "False"
    aura_src = f"def main() {{ print({aura} and {aura2})\nprint({aura} or {aura2}) }}"
    py_src = f"print({py} and {py2})\nprint({py} or {py2})"
    assert run_aura(aura_src) == run_python(py_src)
