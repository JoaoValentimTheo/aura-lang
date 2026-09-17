"""Property-based tests for the transpiler and the object system.

The transpiler's generated Python must always be syntactically valid and, for
programs that use a generated class, must survive a Python ``compile`` step.
Visibility and mangling properties are checked by executing generated programs
and asserting the observable behaviour.
"""
import ast as py_ast
import sys
from pathlib import Path

from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.transformer import Transformer  # noqa: E402
from aura.transpiler.transformers.expressions import py_safe_name  # noqa: E402

SETTINGS = settings(
    max_examples=150,
    deadline=None,
    suppress_health_check=[HealthCheck.too_slow],
)

_IDENT = st.text(alphabet=st.characters(min_codepoint=97, max_codepoint=122),
                 min_size=1, max_size=8)

_KEYWORDS = {
    "def", "let", "if", "else", "for", "while", "return", "class", "trait",
    "match", "case", "import", "from", "with", "try", "catch", "finally",
    "throw", "self", "true", "false", "none", "and", "or", "not", "in", "is",
    "async", "await", "yield", "break", "continue", "pass", "mut", "const",
    "public", "private", "protected", "static", "volatile", "super", "spawn",
    "guard", "loop", "until", "unless", "enum", "type", "module", "export",
    "implements", "assert", "new", "fn", "init", "null", "as", "is", "in",
    "else", "elif", "pass",
}


def _safe_identifier():
    """A lowercase identifier that is not an Aura keyword or keyword typo.

    The tokenizer deliberately rejects `volatily` as a typo for `volatile`, so
    it can never be a valid identifier and must be excluded here too.
    """
    return st.text(alphabet=st.characters(min_codepoint=97, max_codepoint=122),
                   min_size=1, max_size=8).filter(
                       lambda s: s not in _KEYWORDS and s != 'volatily')


@given(st.integers(min_value=-1000, max_value=1000))
@SETTINGS
def test_transpiled_literals_are_valid_python(value):
    program = Parser(Tokenizer(f"def main() {{ print({value}) }}").tokenize()).parse()
    code = Transformer().transform(program)
    py_ast.parse(code)  # raises SyntaxError if invalid


@given(st.lists(st.integers(min_value=-100, max_value=100), max_size=20))
@SETTINGS
def test_transpiled_list_is_valid_python(values):
    literal = "[" + ", ".join(str(v) for v in values) + "]"
    program = Parser(Tokenizer(
        f"def main() {{ let xs = {literal}\n print(xs) }}").tokenize()).parse()
    py_ast.parse(Transformer().transform(program))


@given(st.text(alphabet=st.characters(min_codepoint=97, max_codepoint=122),
               max_size=20).filter(lambda s: '"' not in s and "\\" not in s))
@SETTINGS
def test_transpiled_strings_are_valid_python(body):
    program = Parser(Tokenizer(
        f'def main() {{ print("{body}") }}').tokenize()).parse()
    py_ast.parse(Transformer().transform(program))


@given(st.dictionaries(
    keys=st.text(alphabet=st.characters(min_codepoint=97, max_codepoint=122),
                 min_size=1, max_size=5),
    values=st.integers(min_value=-100, max_value=100),
    max_size=8))
@SETTINGS
def test_transpiled_dict_is_valid_python(mapping):
    entries = ", ".join(f'"{k}": {v}' for k, v in mapping.items())
    program = Parser(Tokenizer(
        f"def main() {{ let d = {{{entries}}}\n print(d) }}").tokenize()).parse()
    py_ast.parse(Transformer().transform(program))


@given(st.lists(_safe_identifier(), min_size=1, max_size=6, unique=True))
@SETTINGS
def test_generated_auto_init_is_valid_python(field_names):
    fields = "\n".join(
        f"  public let {name}: int = 0" for name in field_names)
    src = f"class C {{\n{fields}\n}}"
    program = Parser(Tokenizer(src).tokenize()).parse()
    py_ast.parse(Transformer().transform(program))


@given(st.lists(_safe_identifier(), min_size=1, max_size=6, unique=True))
@SETTINGS
def test_auto_init_assigns_all_fields(field_names):
    fields = "\n".join(
        f"  public let {name}: int = 0" for name in field_names)
    src = f"class C {{\n{fields}\n}}"
    program = Parser(Tokenizer(src).tokenize()).parse()
    namespace: dict = {"__name__": "__prop__"}
    exec(compile(Transformer().transform(program), "<prop>", "exec"), namespace)
    instance = namespace["C"]()
    for name in field_names:
        assert getattr(instance, py_safe_name(name)) == 0


# ---------------------------------------------------------------------------
# Visibility and mangling properties
# ---------------------------------------------------------------------------

@given(_safe_identifier())
@SETTINGS
def test_private_field_is_mangled_with_owner(field):
    src = (
        f"class Box {{\n"
        f"  private let {field}: int = 42\n"
        f"  public def read() -> int {{ return self.{field} }}\n"
        f"}}"
    )
    program = Parser(Tokenizer(src).tokenize()).parse()
    code = Transformer().transform(program)
    assert f"_Box__{field}" in code


@given(_safe_identifier())
@SETTINGS
def test_protected_field_uses_single_underscore(field):
    src = f"class Box {{\n  protected let {field}: int = 1\n}}"
    program = Parser(Tokenizer(src).tokenize()).parse()
    code = Transformer().transform(program)
    safe = py_safe_name(field)
    assert f"self._{safe} =" in code or f"_{safe} =" in code
    assert f"__{safe}" not in code.replace(f"_{safe}", "")


@given(_safe_identifier(), st.integers(min_value=-1000, max_value=1000))
@SETTINGS
def test_private_field_roundtrips_through_accessor(field, value):
    src = (
        f"class Box {{\n"
        f"  private let {field}: int = 0\n"
        f"}}\n"
        f"def main() {{\n"
        f"  let b = Box()\n"
        f"  b.set_{field}({value})\n"
        f"  print(b.get_{field}())\n"
        f"}}"
    )
    program = Parser(Tokenizer(src).tokenize()).parse()
    namespace: dict = {"__name__": "__prop__"}
    import contextlib
    import io
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(Transformer().transform(program), "<prop>", "exec"), namespace)
        namespace["main"]()
    assert buffer.getvalue() == f"{value}\n"


@given(st.integers(min_value=0, max_value=5))
@SETTINGS
def test_inheritance_chain_depth_is_valid_python(depth):
    parts = ["class C0 { public def v() -> int { return 0 } }"]
    for i in range(1, depth + 1):
        parts.append(
            f"class C{i}(C{i - 1}) {{ public def v() -> int {{ return {i} }} }}")
    src = "\n".join(parts)
    program = Parser(Tokenizer(src).tokenize()).parse()
    py_ast.parse(Transformer().transform(program))


@given(st.integers(min_value=1, max_value=6))
@SETTINGS
def test_multiple_trait_implementation_is_valid_python(n):
    traits = [f"trait T{i} {{ public def m{i}() -> int }}" for i in range(n)]
    impls = "\n".join(
        f"  public def m{i}() -> int {{ return {i} }}" for i in range(n))
    names = ", ".join(f"T{i}" for i in range(n))
    src = "\n".join(traits) + f"\nclass C implements {names} {{\n{impls}\n}}"
    program = Parser(Tokenizer(src).tokenize()).parse()
    py_ast.parse(Transformer().transform(program))
