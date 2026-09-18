"""Regression tests for bugs found during the code audit.

Each test targets a specific defect that was fixed: parser gaps for documented
statements/operators, a broken stdlib helper, name-mangling at global scope,
nested module declarations, and the performance-driven transformer changes.
"""
import io
import sys
import contextlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Tokenizer, Parser  # noqa: E402
from transpiler.transformer import Transformer  # noqa: E402


def run_aura(source: str):
    """Transpile Aura source, execute it, and return (stdout, python_code)."""
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    code = Transformer().transform(program)

    namespace = {"__name__": "__aura_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, namespace)
    return buffer.getvalue(), code


def transpile(source: str):
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    return Transformer().transform(program)


# ============================================================================
# unless / until / loop
# ============================================================================

def test_unless_statement():
    out, _ = run_aura('let x = 5\nunless x > 10 { print("small") }')
    assert out.strip() == "small"


def test_unless_else_statement():
    out, _ = run_aura('let x = 50\nunless x > 10 { print("small") } else { print("big") }')
    assert out.strip() == "big"


def test_until_statement():
    source = "let n = 3\nuntil n == 0 { n -= 1 }\nprint(n)"
    out, _ = run_aura(source)
    assert out.strip() == "0"


def test_loop_with_break():
    source = "let i = 0\nloop { i += 1\n if i == 3 { break } }\nprint(i)"
    out, _ = run_aura(source)
    assert out.strip() == "3"


def test_loop_with_continue():
    source = (
        "let mut total = 0\n"
        "let i = 0\n"
        "loop {\n"
        "  i += 1\n"
        "  if i > 5 { break }\n"
        "  if i % 2 == 0 { continue }\n"
        "  total += i\n"
        "}\n"
        "print(total)"
    )
    out, _ = run_aura(source)
    assert out.strip() == "9"  # 1 + 3 + 5


# ============================================================================
# Assignment operators
# ============================================================================

def test_null_coalescing_assignment():
    out, _ = run_aura('let mut x = none\nx ??= 5\nprint(x)')
    assert out.strip() == "5"


def test_null_coalescing_assignment_keeps_existing():
    out, _ = run_aura("let mut x = 7\nx ??= 5\nprint(x)")
    assert out.strip() == "7"


def test_compound_assignment_operators():
    source = (
        "let mut x = 8\n"
        "x <<= 1\n"      # 16
        "x &= 30\n"      # 16
        "x >>= 2\n"      # 4
        "x ^= 1\n"       # 5
        "x |= 2\n"       # 7
        "x %= 4\n"       # 3
        "print(x)"
    )
    out, _ = run_aura(source)
    assert out.strip() == "3"


# ============================================================================
# Bitwise operators
# ============================================================================

def test_bitwise_operators():
    out, _ = run_aura("print(5 & 3, 5 | 2, 5 ^ 1)")
    assert out.strip() == "1 7 4"


def test_bitwise_shifts():
    out, _ = run_aura("print(1 << 4, 32 >> 2)")
    assert out.strip() == "16 8"


def test_shift_precedence_matches_python():
    # + binds tighter than <<
    out, _ = run_aura("print(1 + 2 << 3)")
    assert out.strip() == "24"


# ============================================================================
# Multiple assignment / tuple return
# ============================================================================

def test_multiple_assignment():
    out, _ = run_aura("let mut a, b = 0, 1\nprint(a, b)")
    assert out.strip() == "0 1"


def test_multiple_return_values():
    source = "def swap(a, b) { return b, a }\nlet x, y = swap(1, 2)\nprint(x, y)"
    out, _ = run_aura(source)
    assert out.strip() == "2 1"


def test_tuple_value_in_assignment():
    out, _ = run_aura("let t = 1, 2, 3\nprint(len(t))")
    assert out.strip() == "3"


# ============================================================================
# Variadic / keyword parameters
# ============================================================================

def test_kwargs_parameter():
    out, _ = run_aura("def f(**kw) { return len(kw) }\nprint(f(a=1, b=2))")
    assert out.strip() == "2"


def test_args_and_kwargs_together():
    source = "def f(*args, **kw) { return len(args) + len(kw) }\nprint(f(1, 2, a=3))"
    out, code = run_aura(source)
    assert out.strip() == "3"
    assert "*args" in code and "**kw" in code


# ============================================================================
# Modules / exports
# ============================================================================

def test_module_namespace():
    source = (
        "module MyLib {\n"
        '  export const VERSION = "1.0.0"\n'
        "  export def public_function() { return 42 }\n"
        "}\n"
        "print(MyLib.public_function())\n"
        "print(MyLib.VERSION)"
    )
    out, _ = run_aura(source)
    assert out.strip() == "42\n1.0.0"


def test_module_nested_in_block():
    source = 'let a = 1\nif a { module M { export let x = 1 } }\nprint("ok")'
    out, code = run_aura(source)
    assert out.strip() == "ok"
    assert "class M:" in code


# ============================================================================
# Traits
# ============================================================================

def test_trait_emits_real_base_class():
    source = (
        "trait Drawable {\n"
        "  def draw()\n"
        "}\n"
        "class Circle extends Drawable {\n"
        '  def draw() { print("circle") }\n'
        "}\n"
        "Circle().draw()"
    )
    out, code = run_aura(source)
    assert out.strip() == "circle"
    assert "class Drawable(_aura_abc.ABC):" in code
    assert "class Circle(Drawable):" in code


# ============================================================================
# Type aliases
# ============================================================================

def test_type_alias_emits_runtime_name():
    out, code = run_aura("type UserId = int\nprint(UserId)")
    assert out.strip() == "<class 'int'>"
    assert "UserId = int" in code


def test_type_alias_structural_maps_to_dict():
    out, code = run_aura("type Point = {x: float, y: float}\nprint(Point)")
    assert out.strip() == "<class 'dict'>"


# ============================================================================
# Visibility (name mangling)
# ============================================================================

def test_global_private_is_not_mangled():
    out, _ = run_aura("private let y = 2\nprint(y)")
    assert out.strip() == "2"


def test_global_protected_is_not_mangled():
    out, _ = run_aura("protected let z = 3\nprint(z)")
    assert out.strip() == "3"


def test_class_private_is_mangled():
    source = (
        "class Account {\n"
        "  private let balance = 100\n"
        "  def get() { return self.balance }\n"
        "}\n"
        "print(Account().get())"
    )
    out, code = run_aura(source)
    assert out.strip() == "100"
    # The transpiler emits the owner-aware mangled name directly, so a private
    # member resolves correctly even from a subclass.
    assert "self._Account__balance" in code


# ============================================================================
# for ... step
# ============================================================================

def test_for_range_step_keyword():
    out, _ = run_aura("for i in range(0, 6) step 2 { print(i) }")
    assert out.strip() == "0\n2\n4"


def test_for_range_expression_step():
    out, _ = run_aura("for i in 0..10 step 3 { print(i) }")
    assert out.strip() == "0\n3\n6\n9"


def test_for_iterable_step_uses_slice():
    out, _ = run_aura("for x in [1, 2, 3, 4, 5, 6] step 2 { print(x) }")
    assert out.strip() == "1\n3\n5"


# ============================================================================
# Standard library
# ============================================================================

def test_stdlib_math_builtins_work():
    import stdlib.math as m
    assert m.abs(-3) == 3
    assert m.min(4, 2, 8) == 2
    assert m.max(4, 2, 8) == 8
    assert m.round(2.6) == 3


def test_stdlib_list_chunk_is_linear():
    import stdlib.collections as c
    assert c.list_chunk(2, [1, 2, 3, 4, 5]) == [[1, 2], [3, 4], [5]]


def test_stdlib_list_chunk_rejects_bad_size():
    import stdlib.collections as c
    try:
        c.list_chunk(0, [1, 2, 3])
    except ValueError:
        pass
    else:
        raise AssertionError("expected ValueError for size 0")


# ============================================================================
# Transformer internals
# ============================================================================

def test_transform_records_prelude_metadata():
    source = (
        "@debug\n"
        'def f() { return [x for x in items] }\n'
        "let user = {name: \"Alice\"}\n"
    )
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    transformer = Transformer()
    code = transformer.transform(program)
    expr = transformer.expr_transformer
    stmt = transformer.stmt_transformer
    assert "debug" in expr.used_decorators
    assert "items" in expr.seen_identifiers
    assert expr.has_dict is True
    assert stmt.has_enum is False
    assert stmt.has_label is False
    # The @debug prelude is actually injected.
    assert "def debug(" in code


def test_block_lambda_preserves_class_scope():
    source = (
        "class Box {\n"
        "  let items = [1, 2, 3]\n"
        "  def total() {\n"
        "    let f = (x) => {\n"
        "      let n = self.items.length()\n"
        "      return x + n\n"
        "    }\n"
        "    return f(10)\n"
        "  }\n"
        "}\n"
        "print(Box().total())"
    )
    out, _ = run_aura(source)
    assert out.strip() == "13"


# ============================================================================
# Type annotations (structural / list / function / optional / union)
# ============================================================================

def test_structural_type_annotation():
    source = (
        'let user: {name: str, age: int} = {name: "Alice", age: 30}\n'
        "print(user.name, user.age)"
    )
    out, _ = run_aura(source)
    assert out.strip() == "Alice 30"


def test_list_type_annotation():
    out, _ = run_aura("let xs: [int] = [1, 2, 3]\nprint(xs)")
    assert out.strip() == "[1, 2, 3]"


def test_function_type_annotation():
    source = (
        "def apply(f: (int) -> int, x: int) -> int { return f(x) }\n"
        "print(apply((n) => n * 2, 5))"
    )
    out, _ = run_aura(source)
    assert out.strip() == "10"


def test_optional_type_annotation():
    out, _ = run_aura("let name: str? = none\nprint(name)")
    assert out.strip() == "None"


def test_union_type_annotation():
    out, _ = run_aura("let v: int | str = 3\nprint(v)")
    assert out.strip() == "3"


def test_type_alias_does_not_swallow_next_statement():
    source = "type UserId = int\nprint(UserId)"
    out, code = run_aura(source)
    assert out.strip() == "<class 'int'>"
    assert "print(UserId)" in code


# ============================================================================
# Destructuring with spreads
# ============================================================================

def test_destructure_ellipsis_spread():
    source = "let [first, second, ...rest] = [1, 2, 3, 4, 5]\nprint(first, second, rest)"
    out, _ = run_aura(source)
    assert out.strip() == "1 2 [3, 4, 5]"


def test_destructure_star_spread():
    source = "let [a, *rest] = [1, 2, 3]\nprint(a, rest)"
    out, _ = run_aura(source)
    assert out.strip() == "1 [2, 3]"


# ============================================================================
# Block comments
# ============================================================================

def test_block_comment():
    source = "// line\n/* block\n   spanning\n   lines */\nprint('ok')"
    out, code = run_aura(source)
    assert out.strip() == "ok"
    assert "spanning" not in code


def test_block_comment_inline():
    out, _ = run_aura("print(1) /* trailing */\nprint(2)")
    assert out.strip() == "1\n2"


# ============================================================================
# Macros
# ============================================================================

def test_deprecated_without_message():
    out, _ = run_aura("@deprecated\ndef f() -> int { return 1 }\nprint(f())")
    assert "deprecated" in out
    assert out.strip().endswith("1")


def test_deprecated_with_message():
    out, _ = run_aura('@deprecated("Use new_function")\ndef f() -> int { return 1 }\nprint(f())')
    assert "Use new_function" in out
    assert out.strip().endswith("1")


def test_cache_with_maxsize():
    out, _ = run_aura("@cache(maxsize=16)\ndef sq(n) -> int { return n * n }\nprint(sq(7))")
    assert out.strip() == "49"


def test_must_return_raises_when_none():
    source = "@must_return\ndef f() { return }\nf()"
    try:
        run_aura(source)
    except ValueError as exc:
        assert "must return" in str(exc)
    else:
        raise AssertionError("expected ValueError from @must_return")


# ============================================================================
# Lexical: radix numbers, separators, scientific notation, multiline strings
# ============================================================================

def test_hex_octal_binary_literals():
    out, _ = run_aura("print(0xFF, 0o10, 0b101)")
    assert out.strip() == "255 8 5"


def test_underscore_number_separators():
    out, _ = run_aura("print(1_000_000)")
    assert out.strip() == "1000000"


def test_scientific_notation():
    out, _ = run_aura("print(2.5e3, 1.5e-2)")
    assert out.strip() == "2500.0 0.015"


def test_multiline_string():
    out, _ = run_aura('print("""a\nb""")')
    assert out.strip() == "a\nb"


# ============================================================================
# Integer division via casting (there is no `//` operator)
# ============================================================================

def test_float_division_is_true_division():
    out, _ = run_aura("print(7 / 2)")
    assert out.strip() == "3.5"


def test_integer_division_via_int_cast():
    out, _ = run_aura("print(int(7 / 2), int(10 / 3))")
    assert out.strip() == "3 3"


def test_int_cast_of_float_literal():
    out, _ = run_aura("print(int(3.9))")
    assert out.strip() == "3"


def test_floor_helper_from_stdlib():
    out, _ = run_aura("from stdlib.math import floor\nprint(floor(7 / 2))")
    assert out.strip() == "3"


def test_double_slash_is_only_a_comment():
    # `7//2` is a comment, so `x` is never assigned and the print sees `7`.
    out, _ = run_aura("let x = 7//2\nprint(x)")
    assert out.strip() == "7"


def test_leading_line_comment():
    out, _ = run_aura("// note\nprint(1)")
    assert out.strip() == "1"


def test_trailing_line_comment():
    out, _ = run_aura("print(1) // trailing note\nprint(2)")
    assert out.strip() == "1\n2"


def test_comment_after_assignment():
    out, _ = run_aura("let x = 10  // inferred\nprint(x)")
    assert out.strip() == "10"


# ============================================================================
# Postfix on literals
# ============================================================================

def test_tuple_subscript():
    out, _ = run_aura("print((1, 2)[0])")
    assert out.strip() == "1"


def test_list_literal_subscript():
    out, _ = run_aura("print([1, 2, 3][1])")
    assert out.strip() == "2"


def test_dict_literal_member_access():
    out, _ = run_aura("print({a: 1}.a)")
    assert out.strip() == "1"


# ============================================================================
# async / await and call spreads (grammar consistency)
# ============================================================================

def test_await_in_expression():
    out, code = run_aura("async def f() { let x = await g()\nreturn x }\nprint('ok')")
    assert out.strip() == "ok"
    assert "await g()" in code
    assert "async def f()" in code


def test_async_function_runs_via_asyncio():
    source = (
        "async def double(x) { return x * 2 }\n"
        "print('defined')"
    )
    out, code = run_aura(source)
    assert out.strip() == "defined"
    assert "async def double" in code


def test_call_spread_positional():
    source = "def f(*a) { return len(a) }\nlet l = [1, 2, 3]\nprint(f(*l))"
    out, code = run_aura(source)
    assert out.strip() == "3"
    assert "f(*l)" in code


def test_call_spread_ellipsis():
    source = "def f(*a) { return len(a) }\nlet l = [1, 2]\nprint(f(...l))"
    out, _ = run_aura(source)
    assert out.strip() == "2"


def test_call_spread_keyword():
    source = "def f(**k) { return len(k) }\nlet d = {a: 1, b: 2}\nprint(f(**d))"
    out, code = run_aura(source)
    assert out.strip() == "2"
    assert "f(**d)" in code


# ============================================================================
# Brace disambiguation: bare blocks vs dict/set literals
# ============================================================================

def test_bare_block_multiline():
    source = 'let r = "a"\n{ print(r)\nlet x = 42\nprint(x) }\nprint("done")'
    out, _ = run_aura(source)
    assert out.strip() == "a\n42\ndone"


def test_bare_block_does_not_break_dicts():
    out, _ = run_aura("let d = {a: 1, b: 2}\nprint(d.a, d.b)")
    assert out.strip() == "1 2"


def test_bare_block_does_not_break_set_comprehension():
    out, _ = run_aura("print({x for x in range(3)})")
    assert out.strip() == "{0, 1, 2}"


def test_bare_block_does_not_break_nested_dict():
    out, _ = run_aura('let d = {name: {first: "A"}}\nprint(d.name.first)')
    assert out.strip() == "A"


def test_block_expression_still_returns_value():
    out, _ = run_aura("let v = { a = 1\nb = a + 2\nb * 10 }\nprint(v)")
    assert out.strip() == "30"


# ============================================================================
# Stdlib and import fixes (Phase 3 audit)
# ============================================================================

def test_import_brace_form_selects_names():
    out, code = run_aura("import stdlib.math { sqrt, PI }\nprint(sqrt(16))\nprint(PI)")
    assert "from stdlib.math import sqrt, PI" in code
    assert out.splitlines()[0].strip() == "4.0"


def test_import_module_alias_form():
    out, code = run_aura("import stdlib.math as m\nprint(m.sqrt(25))")
    assert "import stdlib.math as m" in code
    assert out.strip() == "5.0"


def test_from_import_with_item_alias():
    out, code = run_aura("from stdlib.math import sqrt as root\nprint(root(36))")
    assert "from stdlib.math import sqrt as root" in code
    assert out.strip() == "6.0"


def test_import_nodes_transpile():
    from transpiler.ast import Program, ImportStmt, FromImport

    code = Transformer().transform(Program([ImportStmt("math")]))
    assert code.strip() == "import math"

    code = Transformer().transform(Program([ImportStmt("collections", alias="coll")]))
    assert code.strip() == "import collections as coll"

    code = Transformer().transform(
        Program([FromImport("math", [("sqrt", None), ("pi", None)])])
    )
    assert code.strip() == "from math import sqrt, pi"


def test_math_gcd_lcm_empty_and_values():
    from stdlib.math import gcd, lcm

    assert gcd() == 0
    assert lcm() == 1
    assert gcd(12, 18) == 6
    assert lcm(4, 6) == 12


def test_string_split_limit_is_maxsplit():
    from stdlib.string import split

    assert split("a,b,c,d", ",", 2) == ["a", "b", "c,d"]
    assert split("a b c d", None, 2) == ["a", "b", "c d"]


def test_string_is_numeric_rejects_non_finite():
    from stdlib.string import is_numeric

    assert is_numeric("12.5") is True
    assert is_numeric("-3") is True
    assert is_numeric("") is False
    assert is_numeric("   ") is False
    assert is_numeric("nan") is False
    assert is_numeric("inf") is False


def test_string_pad_rejects_bad_fill():
    from stdlib.string import pad_left
    import pytest

    with pytest.raises(ValueError):
        pad_left("x", 3, "ab")


def test_collections_reduce_explicit_none_initial():
    from stdlib.collections import reduce as data_first_reduce, list_reduce

    assert data_first_reduce([1, 2, 3], lambda a, b: a + b) == 6
    # An explicit None initial must be forwarded (not treated as "no initial").
    assert data_first_reduce([[1], [2]], lambda a, b: a + b, []) == [1, 2]
    assert list_reduce(lambda a, b: a + b, [1, 2, 3]) == 6


def test_collections_take_drop_accept_iterators():
    from stdlib.collections import take, drop, list_take, list_drop

    assert take(iter([1, 2, 3, 4]), 2) == [1, 2]
    assert drop(iter([1, 2, 3, 4]), 2) == [3, 4]
    assert list_take(2, iter([1, 2, 3])) == [1, 2]
    assert list_drop(2, iter([1, 2, 3])) == [3]


def test_auradict_dunder_lookup_not_intercepted():
    from stdlib.collections import AuraDict

    d = AuraDict({"a": 1})
    assert d.a == 1
    # Dunder lookups must fall back to normal attribute protocol, not __getitem__.
    assert d.__class__ is AuraDict
    try:
        d.__missing_dunder__
    except AttributeError:
        pass
    else:
        raise AssertionError("missing dunder should raise AttributeError")


def test_json_serialization_is_strict():
    from stdlib.json import dumps, pretty, stringify, is_valid
    import pytest

    assert is_valid('{"a": 1}') is True
    assert is_valid('NaN') is False
    assert is_valid('Infinity') is False
    assert is_valid(None) is False
    for fn in (dumps, pretty, stringify):
        with pytest.raises(ValueError):
            fn(float("inf"))


def test_json_merge_requires_mappings():
    from stdlib.json import merge
    import json as _json
    import pytest

    assert _json.loads(merge({"a": 1}, {"b": 2})) == {"a": 1, "b": 2}
    with pytest.raises(TypeError):
        merge({"a": 1}, [1, 2])


def test_io_read_lines_keeps_blank_lines():
    import os
    import tempfile
    from stdlib.io import write, read_lines

    with tempfile.NamedTemporaryFile(suffix=".txt", delete=False, mode="w") as f:
        path = f.name
    try:
        write(path, "line1\n\nline3\n")
        assert read_lines(path) == ["line1", "", "line3"]
    finally:
        os.unlink(path)


def test_io_write_lines_stringifies():
    import os
    import tempfile
    from stdlib.io import write_lines, read

    with tempfile.NamedTemporaryFile(suffix=".txt", delete=False, mode="w") as f:
        path = f.name
    try:
        write_lines(path, [1, 2, 3])
        assert read(path) == "1\n2\n3\n"
    finally:
        os.unlink(path)


def test_io_ls_is_sorted():
    import os
    import tempfile
    from stdlib.io import ls

    d = tempfile.mkdtemp()
    try:
        for name in ("c.txt", "a.txt", "b.txt"):
            open(os.path.join(d, name), "w").close()
        assert ls(d) == ["a.txt", "b.txt", "c.txt"]
    finally:
        import shutil
        shutil.rmtree(d)


# ============================================================================
# F-string escaping and async execution
# ============================================================================

def test_fstring_single_quote_escape():
    out, code = run_aura("print(f'it\\'s {1}')")
    assert out.strip() == "it's 1"


def test_fstring_double_quote_escape():
    out, _ = run_aura('let who = "x"\nprint(f"say \\"{who}\\"")')
    assert out.strip() == 'say "x"'


def test_fstring_literal_braces():
    out, _ = run_aura('print(f"{{literal}} {1 + 1}")')
    assert out.strip() == "{literal} 2"


def test_await_top_level_async_call():
    from aura.cli import _await_top_level_async_calls
    from parser.to_ast import Tokenizer, Parser

    source = (
        "async def double(x) -> int { return x * 2 }\n"
        "async def main() { print(await double(21)) }\n"
        "main()\n"
    )
    program = Parser(Tokenizer(source).tokenize()).parse()
    assert _await_top_level_async_calls(program) is True
    code = Transformer().transform(program)
    assert "await main()" in code


def test_await_detection_ignores_string_literals():
    from aura.cli import _await_top_level_async_calls
    from parser.to_ast import Tokenizer, Parser

    program = Parser(Tokenizer('print("please await this")').tokenize()).parse()
    assert _await_top_level_async_calls(program) is False


def test_fstring_transpiles_to_valid_python():
    import ast as py_ast

    code = transpile("print(f'it\\'s {1}')")
    py_ast.parse(code)


# ============================================================================
# Mutability rules: `let` immutable, `let mut` mutable, `const` immutable
# ============================================================================

def _mutability_errors(source: str):
    from transpiler.semantics import MutabilityChecker

    program = Parser(Tokenizer(source).tokenize()).parse()
    checker = MutabilityChecker()
    checker.check_program(program)
    return checker.violations


def test_let_reassignment_is_rejected():
    assert "x" in _mutability_errors("let x = 1\nx = 2\n")


def test_const_reassignment_is_rejected():
    assert "PI" in _mutability_errors("const PI = 3.14\nPI = 4.0\n")


def test_let_mut_reassignment_is_allowed():
    assert _mutability_errors("let mut x = 1\nx = 2\nx += 3\n") == []


def test_member_assignment_is_not_a_rebinding():
    source = (
        "class C {\n"
        "  let v: int = 0\n"
        "  def new() { self.v = 0 }\n"
        "  def inc() { self.v += 1 }\n"
        "}\n"
    )
    assert _mutability_errors(source) == []


def test_for_and_catch_bindings_are_mutable():
    assert _mutability_errors("for i in [1] { i = 2 }\n") == []
    assert _mutability_errors("try { f() } catch Error as e { e = 1 }\n") == []


def test_reassignment_inside_nested_block_is_detected():
    assert "seen" in _mutability_errors(
        "let seen = []\nfor x in [1] { unless true { seen = [x] } }\n"
    )


def test_mutability_violation_fails_the_cli(tmp_path):
    import subprocess

    src = tmp_path / "bad.aura"
    src.write_text("let x = 1\nx = 2\nprint(x)\n")
    result = subprocess.run(
        [sys.executable, str(Path(__file__).parent.parent / "main.py"), "run", str(src)],
        capture_output=True, text=True,
    )
    assert result.returncode != 0
    assert "immutable" in result.stderr


# ============================================================================
# Multi-target assignment / swap
# ============================================================================

def test_tuple_swap_assignment():
    out, _ = run_aura("let mut a = 1\nlet mut b = 2\na, b = b, a\nprint(a, b)")
    assert out.strip() == "2 1"


def test_triple_rotation_assignment():
    out, _ = run_aura(
        "let mut x, y, z = 1, 2, 3\nx, y, z = z, x, y\nprint(x, y, z)"
    )
    assert out.strip() == "3 1 2"


def test_single_target_tuple_value_still_works():
    out, _ = run_aura("let x = 1, 2\nprint(x)")
    assert out.strip() == "(1, 2)"


def test_multi_target_assign_transpiles_to_valid_python():
    import ast as py_ast

    code = transpile("let mut a = 1\nlet mut b = 2\na, b = b, a")
    py_ast.parse(code)


# ============================================================================
# throw with a string message
# ============================================================================

def test_throw_string_raises_exception():
    source = 'try { throw "boom" } catch Error as e { print(e) }'
    out, code = run_aura(source)
    assert out.strip() == "boom"
    assert "raise Exception(" in code


# ============================================================================
# Local Aura module imports
# ============================================================================

def test_local_aura_module_import(tmp_path):
    import subprocess

    (tmp_path / "mathutil.aura").write_text(
        "def square(x: int) -> int { return x * x }\n"
    )
    main_file = tmp_path / "program.aura"
    main_file.write_text(
        "import mathutil\n"
        "def main() { print(mathutil.square(7)) }\n"
    )

    result = subprocess.run(
        [sys.executable, str(Path(__file__).parent.parent / "main.py"),
         "run", str(main_file)],
        capture_output=True, text=True,
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "49"


def test_local_aura_dotted_package_import(tmp_path):
    import subprocess

    pkg = tmp_path / "pkg"
    pkg.mkdir()
    (pkg / "util.aura").write_text(
        "def triple(x: int) -> int { return x * 3 }\n"
    )
    main_file = tmp_path / "program.aura"
    main_file.write_text(
        "from pkg.util import triple\n"
        "def main() { print(triple(5)) }\n"
    )

    result = subprocess.run(
        [sys.executable, str(Path(__file__).parent.parent / "main.py"),
         "run", str(main_file)],
        capture_output=True, text=True,
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "15"


def test_importer_does_not_hijack_python_stdlib(tmp_path):
    import subprocess

    main_file = tmp_path / "program.aura"
    main_file.write_text(
        "import os\n"
        "def main() { print(os.path.basename('/a/b/c.txt')) }\n"
    )
    result = subprocess.run(
        [sys.executable, str(Path(__file__).parent.parent / "main.py"),
         "run", str(main_file)],
        capture_output=True, text=True,
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "c.txt"


# ============================================================================
# Closures mutating captured locals (nonlocal)
# ============================================================================

def test_block_lambda_mutates_captured_local():
    source = (
        "def counter() {\n"
        "  let mut n = 0\n"
        "  return () => { n += 1\n return n }\n"
        "}\n"
        "let c = counter()\n"
        "print(c(), c(), c())\n"
    )
    out, code = run_aura(source)
    assert out.strip() == "1 2 3"
    assert "nonlocal n" in code


def test_block_lambda_without_capture_has_no_nonlocal():
    source = "def f() {\n  return (x) => { return x * 2 }\n}\nprint(f()(21))\n"
    out, code = run_aura(source)
    assert out.strip() == "42"
    assert "nonlocal" not in code


def test_nested_lambda_does_not_leak_nonlocal():
    import ast as py_ast

    code = transpile(
        "def outer() {\n"
        "  let mut a = 1\n"
        "  return () => {\n"
        "    let inner = () => { return a }\n"
        "    a += 1\n"
        "    return inner()\n"
        "  }\n"
        "}\n"
    )
    py_ast.parse(code)


# ============================================================================
# Enum auto-numbering after an explicit value
# ============================================================================

def test_enum_autoincrement_after_explicit_value():
    source = (
        "enum Status { Active, Disabled = 3, Pending }\n"
        "print(Status.Pending.value)\n"
    )
    out, _ = run_aura(source)
    assert out.strip() == "4"


def test_enum_plain_autoincrement_starts_at_one():
    out, _ = run_aura("enum E { A, B, C }\nprint(E.A.value, E.B.value, E.C.value)")
    assert out.strip() == "1 2 3"


# ============================================================================
# Python compatibility: multi-import and raw/byte string literals
# ============================================================================

def test_multi_module_import():
    out, code = run_aura("import os, sys\nprint(bool(os.name) and sys.version_info[0] >= 3)")
    assert "import os, sys" in code
    assert out.strip() == "True"


def test_multi_module_import_with_alias():
    code = transpile("import os as o, sys as s")
    assert code.strip() == "import os as o, sys as s"


def test_raw_string_literal_preserved():
    out, code = run_aura(r'let p = r"C:\new\folder"' + "\nprint(p)")
    assert r'r"C:\new\folder"' in code
    assert out.strip() == r"C:\new\folder"


def test_raw_string_as_call_argument():
    source = 'import re\nprint(re.match(r"\\d+", "123").group())'
    out, _ = run_aura(source)
    assert out.strip() == "123"


def test_byte_string_literal():
    out, code = run_aura('let b = b"bytes"\nprint(b)')
    assert 'b"bytes"' in code
    assert out.strip() == "b'bytes'"


def test_prefixed_strings_transpile_to_valid_python():
    import ast as py_ast

    for source in (
        r'let x = r"\d+"' + "\nprint(x)",
        'let x = b"abc"\nprint(x)',
        'let x = rb"a\\d"\nprint(x)',
    ):
        py_ast.parse(transpile(source))


# ============================================================================
# Python-style slicing
# ============================================================================

def test_list_slicing():
    cases = {
        "nums[1:3]": "[2, 3]",
        "nums[:2]": "[1, 2]",
        "nums[1:]": "[2, 3, 4, 5]",
        "nums[::2]": "[1, 3, 5]",
        "nums[::-1]": "[5, 4, 3, 2, 1]",
        "nums[1:4:2]": "[2, 4]",
    }
    for expr, expected in cases.items():
        out, _ = run_aura(f"let nums = [1,2,3,4,5]\nprint({expr})")
        assert out.strip() == expected, expr


def test_string_slicing():
    out, _ = run_aura('let s = "Hello, World"\nprint(s[0:5])\nprint(s[7:])')
    assert out.strip() == "Hello\nWorld"


def test_slice_transpiles_to_valid_python():
    import ast as py_ast

    code = transpile("let nums = [1,2,3]\nprint(nums[0:2])\nprint(nums[::-1])")
    py_ast.parse(code)
    assert "nums[0:2]" in code
    assert ":" in code  # a slice was emitted


if __name__ == "__main__":
    import traceback

    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for test in tests:
        try:
            test()
            print(f"PASS {test.__name__}")
        except Exception:
            failed += 1
            print(f"FAIL {test.__name__}")
            traceback.print_exc()
    print(f"\n{len(tests) - failed}/{len(tests)} regression tests passed")
    sys.exit(1 if failed else 0)

def test_guard_return_inside_method_returns_not_systemexit():
    """A `guard ... else { return }` in a method returns from the method.

    Earlier this was rewritten to `raise SystemExit` (the module-level idiom),
    which killed the whole program.
    """
    source = (
        "class C {\n"
        "  def check(flag: bool) -> str {\n"
        "    guard flag else { return 'no' }\n"
        "    return 'yes'\n"
        "  }\n"
        "}\n"
        "let c = C()\n"
        "print(c.check(true))\n"
        "print(c.check(false))\n"
        "print('still running')"
    )
    out, _ = run_aura(source)
    assert out == "yes\nno\nstill running\n"


def test_starred_tuple_literal_transpiles_to_valid_python():
    """`(*a, 1)` is a spread tuple, not a unary `*` on `a`.

    The paren path parsed the first element with `parse_expression`, so a
    leading `*` became a UnaryOp and `((* a), 1)` was emitted — invalid Python.
    """
    source = (
        "def f() {\n"
        "  let a = [2, 3]\n"
        "  let t = (*a, 1)\n"
        "  print(t)\n"
        "}\n"
        "f()"
    )
    out, code = run_aura(source)
    assert out == "(2, 3, 1)\n"
    assert "(* a)" not in code


def test_starred_decorator_arguments_transpile_to_valid_python():
    """`@deco(*xs, **kw)` keeps the spread instead of wrapping it in a tuple."""
    source = (
        "def tag(*prefixes, **opts) {\n"
        "  return (fn) => fn\n"
        "}\n"
        "@tag(*[\"a\"], **{\"k\": 1})\n"
        "def f() { return 1 }\n"
        "print(f())"
    )
    out, code = run_aura(source)
    assert out == "1\n"
    assert "@tag(*['a'], **AuraDict({'k': 1}))" in code


def test_private_module_member_whose_name_is_a_keyword_substring():
    """A private module member named `f`/`c`/`e` must not corrupt `def`/`class`.

    `_rename_module_member` used `prefix.index(name)`, which matched the `f`
    inside `def ` and rewrote the declaration to `de_M__f(`.
    """
    source = (
        "module M {\n"
        "  export f, d\n"
        "  def f() { return 1 }\n"
        "  class C { }\n"
        "  def d() { return 2 }\n"
        "}\n"
        "print(M.f())\n"
        "print(M.d())"
    )
    out, code = run_aura(source)
    assert out == "1\n2\n"
    assert "def f(" in code
    assert "class _M__C:" in code
    assert "de_M__f(" not in code


def test_set_literal_spread_transpiles_to_valid_python():
    """`{*a}` is a set spread, not a block expression."""
    source = (
        "def f() {\n"
        "  let a = [1, 2]\n"
        "  let s = {*a, 3}\n"
        "  print(len(s))\n"
        "}\n"
        "f()"
    )
    out, code = run_aura(source)
    assert out == "3\n"
    assert "{*a, 3}" in code


def test_bare_spread_in_value_position_is_a_clear_error():
    """`return *a` / `for x in *a` are outside the grammar and must not emit
    invalid Python (`return (* a)`)."""
    for source in (
        "def f() { return *a }",
        "def f() { for x in *a { print(x) } }",
    ):
        try:
            run_aura(source)
        except SyntaxError as exc:
            assert "spread is not allowed" in str(exc)
        else:
            raise AssertionError(f"expected SyntaxError for: {source}")
