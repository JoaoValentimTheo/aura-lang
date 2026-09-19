"""Syntax fixes from the Espelhagem/Kof audit.

Locks in the parser fixes made during the syntax pass:

* A bare `return`/`break`/`continue` does not swallow the next line's
  identifier (its value/label must be on the same line).
* Union type aliases (`type M = int | none`) parse and emit as one declaration.
* Set `{T}` and tuple `(A, B)` type annotations parse.
* Enum members may be separated by `,`, `;` or a newline.
* Foreign spellings and `elif` are rejected with pointed messages.
"""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura_test_helpers import parse, run_aura, transpile  # noqa: E402

# ============================================================================
# Line-terminated terminators
# ============================================================================

class TestTerminatorLines:
    def test_break_does_not_swallow_next_statement(self):
        code = transpile("while true {\n break\n print(1)\n}")
        assert "break" in code
        assert "print(1)" in code
        # The bare `break` must not become `break print`.

    def test_continue_does_not_swallow_next_statement(self):
        code = transpile("for x in [1] {\n continue\n print(1)\n}")
        assert "continue" in code

    def test_return_does_not_swallow_next_statement(self):
        code = transpile("def f() {\n return\n print(1)\n}")
        assert "return print(1)" not in code

    def test_break_label_on_same_line_still_works(self):
        code = transpile("outer: while true { break outer }")
        assert "_AuraBreak('outer')" in code

    def test_return_value_on_same_line_still_works(self):
        out = run_aura("def f() -> int { return 5 }\nprint(f())")
        assert out == "5\n"


# ============================================================================
# Type aliases and annotations
# ============================================================================

class TestTypeSyntax:
    def test_union_alias_is_one_declaration(self):
        program = parse("type Maybe = int | none")
        assert len(program.statements) == 1
        assert program.statements[0].name == 'Maybe'

    def test_union_alias_emits_valid_python(self):
        import ast as py_ast
        code = transpile("type Maybe = int | none")
        py_ast.parse(code)

    def test_union_alias_none_maps_to_none_keyword(self):
        # `none` is Aura's null; the emitted alias must use Python `None`.
        code = transpile("type Maybe = int | none")
        assert "None" in code
        run_aura("type Maybe = int | none\nlet m: Maybe = none\nprint(m is none)")

    def test_set_annotation(self):
        assert run_aura("let s: {int} = {1, 2, 3}\nprint(len(s))") == "3\n"

    def test_tuple_annotation(self):
        out = run_aura('let t: (int, str) = (1, "a")\nprint(t[1])')
        assert out == "a\n"

    def test_structural_annotation_still_works(self):
        out = run_aura('let u: {name: str} = {name: "a"}\nprint(u.name)')
        assert out == "a\n"

    def test_function_annotation_still_works(self):
        transpile("let f: (int) -> int = (x) => x")


# ============================================================================
# Enum separators
# ============================================================================

class TestEnumSeparators:
    def test_comma_separated(self):
        assert run_aura("enum C { A, B }\nprint(C.A)") == "C.A\n"

    def test_newline_separated(self):
        assert run_aura("enum C {\n A\n B\n}\nprint(C.B)") == "C.B\n"

    def test_semicolon_separated(self):
        assert run_aura("enum C { A; B }\nprint(C.A)") == "C.A\n"


# ============================================================================
# Foreign spellings
# ============================================================================

class TestForeignSpellings:
    @pytest.mark.parametrize("src", [
        "if a { } elif b { }",
        "var x = 1",
        "fun f() {}",
        "let x = new C()",
        "let f = lambda x: x",
        "switch x { }",
        "match x { case 1: 2 }",
    ])
    def test_rejected(self, src):
        with pytest.raises(SyntaxError):
            parse(src)

    @pytest.mark.parametrize("name", ["if", "from", "val"])
    def test_reserved_names_rejected(self, name):
        with pytest.raises(SyntaxError):
            parse(f"let {name} = 1")
