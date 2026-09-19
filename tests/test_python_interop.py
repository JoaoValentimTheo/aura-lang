"""Python interop (`py.`) and foreign-keyword rejection.

Aura's host-Python interop is explicit: a module imported through the `py.`
prefix is a host Python module, and the name bound is the last path segment.
Names that collide with Aura keywords must be aliased. Foreign spellings from
other languages are rejected with a pointed message instead of being silently
parsed as identifiers.
"""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura_test_helpers import parse, transpile  # noqa: E402


def first(source):
    return parse(source).statements[0]


# ============================================================================
# `py.` prefix: parsing
# ============================================================================

class TestPythonImportParsing:
    def test_import_py_module_is_marked_and_stripped(self):
        node = first("import py.re")
        assert node.is_python is True
        assert node.module == 're'

    def test_import_py_module_with_alias(self):
        node = first("import py.re as regex")
        assert node.is_python is True
        assert node.module == 're'
        assert node.alias == 'regex'

    def test_import_py_dotted_module(self):
        node = first("import py.os.path")
        assert node.is_python is True
        assert node.module == 'os.path'

    def test_from_py_import_is_marked(self):
        node = first("from py.math import sqrt, floor")
        assert node.is_python is True
        assert node.module == 'math'
        assert node.items == [('sqrt', None), ('floor', None)]

    def test_plain_import_is_not_python(self):
        node = first("import util")
        assert node.is_python is False
        assert node.module == 'util'

    def test_stdlib_import_is_not_python(self):
        node = first("from stdlib.math import sqrt")
        assert node.is_python is False
        assert node.module == 'stdlib.math'


# ============================================================================
# `py.` prefix: code generation
# ============================================================================

class TestPythonImportCodegen:
    def test_binds_last_segment(self):
        # `import py.os.path` binds `path`, not `os`.
        code = transpile("import py.os.path")
        assert "import os.path as path" in code

    def test_simple_module_binds_itself(self):
        assert "import re as re" in transpile("import py.re")

    def test_alias_wins(self):
        assert "import re as regex" in transpile("import py.re as regex")

    def test_multiple_python_modules(self):
        code = transpile("import py.re, py.json")
        assert "re as re" in code
        assert "json as json" in code

    def test_from_python_import(self):
        code = transpile("from py.math import sqrt, pi as PI")
        assert "from math import sqrt, pi as PI" in code


# ============================================================================
# Keyword collisions require an alias
# ============================================================================

class TestPythonKeywordCollision:
    @pytest.mark.parametrize("name", ['type', 'from', 'in', 'is', 'class', 'match'])
    def test_reserved_name_requires_alias(self, name):
        with pytest.raises(SyntaxError, match="reserved Aura keyword"):
            parse(f"from py.re import {name}")

    def test_reserved_name_with_alias_is_accepted(self):
        node = first("from py.re import type as re_type")
        assert node.items == [('type', 're_type')]

    def test_non_reserved_name_needs_no_alias(self):
        node = first("from py.re import sub")
        assert node.items == [('sub', None)]


# ============================================================================
# Foreign keywords
# ============================================================================

class TestForeignKeywords:
    @pytest.mark.parametrize("src,canonical", [
        ("var x = 1", "'let mut'"),
        ("fun f() {}", "'def'"),
        ("function f() {}", "'def'"),
        ("foreach x in [1] { }", "'for x in xs'"),
        ("switch x { }", "'match"),
        ("repeat { } until true", "'loop"),
        ("let x = new C()", r"construct with 'Type\(args\)'"),
        ("let f = lambda x: x", "arrow lambda"),
    ])
    def test_foreign_spelling_is_rejected(self, src, canonical):
        with pytest.raises(SyntaxError, match=canonical):
            parse(src)

    def test_elif_is_rejected(self):
        with pytest.raises(SyntaxError, match="'else if'"):
            parse("if a { } elif b { }")

    def test_else_if_is_accepted(self):
        stmt = first("if a { } else if b { } else { }")
        assert stmt.else_body is not None

    def test_case_colon_is_rejected(self):
        with pytest.raises(SyntaxError, match="not Aura case syntax"):
            parse("match x { case 1: 2 }")

    def test_case_arrow_is_accepted(self):
        parse("match x { case 1 -> 2 }")
