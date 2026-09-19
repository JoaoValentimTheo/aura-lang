"""Coherence audit: keywords, variables/constants, structures, operators, lists.

Locks in the fixes from the lexical/structure audit:

* Operator precedence matches Python for the shared operators (comparison is
  looser than the bitwise operators and the shifts) and matches `GRAMMAR.md`.
* `x is <literal>` is rejected (identity against a literal is a mistake and
  would leak a Python ``SyntaxWarning``); `x is none` stays valid.
* Collections expose a coherent length/append surface: `.size()`, `.length()`,
  `.len()` mean ``len`` and `.add(x)` appends.
* The documented operator semantics (`/` true division, `%` divisor sign) hold.
"""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura_test_helpers import parse, run_aura, transpile  # noqa: E402

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402


def shape(node):
    """Render a binary/unary expression as a fully-parenthesized string."""
    from aura.transpiler.ast import (
        BinaryOp,
        BoolLiteral,
        Identifier,
        IntLiteral,
        UnaryOp,
    )
    if isinstance(node, BinaryOp):
        return f"({shape(node.left)} {node.op} {shape(node.right)})"
    if isinstance(node, UnaryOp):
        return f"({node.op} {shape(node.operand)})"
    if isinstance(node, IntLiteral):
        return str(node.value)
    if isinstance(node, BoolLiteral):
        return str(node.value)
    if isinstance(node, Identifier):
        return node.name
    return type(node).__name__


def expr_shape(source):
    return shape(Parser(Tokenizer(source).tokenize()).parse_expression())


# ============================================================================
# Operator precedence (matches Python for shared operators)
# ============================================================================

class TestPrecedence:
    @pytest.mark.parametrize("src,expected", [
        # comparison is looser than bitwise: `1 & 2 == 2` is `(1 & 2) == 2`
        ("1 & 2 == 2", "((1 & 2) == 2)"),
        ("1 == 2 & 3", "(1 == (2 & 3))"),
        ("1 | 2 < 3", "((1 | 2) < 3)"),
        ("1 < 2 | 3", "(1 < (2 | 3))"),
        ("1 ^ 2 == 2", "((1 ^ 2) == 2)"),
        # shifts bind tighter than `&`
        ("1 << 2 & 3", "((1 << 2) & 3)"),
        ("1 & 3 << 2", "(1 & (3 << 2))"),
        ("1 << 2 == 4", "((1 << 2) == 4)"),
        # bitwise ordering: `|` < `^` < `&`
        ("1 | 2 & 3", "(1 | (2 & 3))"),
        ("1 & 2 | 3", "((1 & 2) | 3)"),
        # arithmetic
        ("1 + 2 * 3", "(1 + (2 * 3))"),
        ("2 ** 3 ** 2", "(2 ** (3 ** 2))"),
        ("-2 ** 2", "(- (2 ** 2))"),
        # logical vs comparison
        ("1 < 2 and 2 < 3", "((1 < 2) and (2 < 3))"),
        ("not 1 < 2", "(not (1 < 2))"),
        ("not 1 | 2", "(not (1 | 2))"),
    ])
    def test_shape(self, src, expected):
        assert expr_shape(src) == expected

    def test_range_and_coalesce_are_tighter_than_comparison(self):
        # `1 + 2 ?? 3` binds `+` tighter than `??`; the node is a CoalesceExpr.
        from aura.transpiler.ast import CoalesceExpr
        node = Parser(Tokenizer("1 + 2 ?? 3").tokenize()).parse_expression()
        assert isinstance(node, CoalesceExpr)

    def test_comparison_matches_python_values(self):
        assert run_aura("print(1 & 2 == 2)") == "False\n"
        assert run_aura("print(1 == 2 & 3)") == "False\n"
        # (1 | 2) < 3  ==  3 < 3  ==  False
        assert run_aura("print(1 | 2 < 3)") == "False\n"
        assert run_aura("print(1 < 2 | 3)") == "True\n"
        assert run_aura("print(1 << 2 == 4)") == "True\n"

    def test_comparison_does_not_chain(self):
        # Aura is left-associative here, unlike Python's chained comparison.
        assert expr_shape("1 < 2 < 3") == "((1 < 2) < 3)"


# ============================================================================
# Arithmetic semantics (Python, documented)
# ============================================================================

class TestArithmeticSemantics:
    def test_true_division(self):
        assert run_aura("print(7 / 2)") == "3.5\n"

    def test_modulo_follows_divisor_sign(self):
        assert run_aura("print(-7 % 3)") == "2\n"

    def test_power_right_associative(self):
        assert run_aura("print(2 ** 3 ** 2)") == "512\n"

    def test_unary_minus_looser_than_power(self):
        assert run_aura("print(-2 ** 2)") == "-4\n"


# ============================================================================
# Identity vs equality
# ============================================================================

class TestIdentity:
    @pytest.mark.parametrize("src", [
        "let x = 1\nprint(x is 1)",
        "let x = 1\nprint(x is 'a')",
        "let x = 1\nprint(x is true)",
        "let x = 1\nprint(x is [1])",
        "let x = 1\nprint(x is not 1)",
    ])
    def test_identity_against_literal_is_rejected(self, src):
        with pytest.raises(SyntaxError, match="compares identity"):
            parse(src)

    def test_none_identity_is_allowed(self):
        assert run_aura("let x = none\nprint(x is none)") == "True\n"
        assert run_aura("let x = 1\nprint(x is not none)") == "True\n"

    def test_equality_is_unaffected(self):
        assert run_aura("print(1 == 1)") == "True\n"

    def test_no_python_syntax_warning(self):
        # Regenerating the code for `x is none` must not trip Python's own
        # literal-identity warning.
        import warnings
        with warnings.catch_warnings():
            warnings.simplefilter("error")
            code = transpile("let x = none\nprint(x is none)")
            compile(code, "<test>", "exec")


# ============================================================================
# Collection conventions
# ============================================================================

class TestCollectionConveniences:
    @pytest.mark.parametrize("method", ["size", "length", "len"])
    def test_length_spellings_agree(self, method):
        assert run_aura(f"print([1, 2, 3].{method}())") == "3\n"

    def test_size_on_string(self):
        assert run_aura("print('abc'.size())") == "3\n"

    def test_add_appends(self):
        assert run_aura(
            "let mut xs = [1, 2]\nxs.add(3)\nprint(xs)") == "[1, 2, 3]\n"

    def test_contains_and_is_empty(self):
        assert run_aura("print([1, 2].contains(2))") == "True\n"
        assert run_aura("print([].is_empty())") == "True\n"

    def test_user_method_wins_over_convenience(self):
        assert run_aura(
            "class C {\n"
            "  public def add(x: int) -> int { return x + 1 }\n"
            "}\n"
            "print(C().add(1))\n") == "2\n"

    def test_python_list_methods_pass_through(self):
        assert run_aura(
            "let mut xs = [3, 1, 2]\nxs.sort()\nprint(xs)") == "[1, 2, 3]\n"


# ============================================================================
# Open-ended (infinite) ranges
# ============================================================================

class TestOpenRange:
    def test_for_over_open_range(self):
        out = run_aura(
            "for i in 0.. {\n"
            "  if i > 2 { break }\n"
            "  print(i)\n"
            "}\n")
        assert out == "0\n1\n2\n"

    def test_open_range_does_not_swallow_next_statement(self):
        # `let r = 1..` followed by a new statement must not become
        # `range(1, print(1) + 1)`.
        code = transpile("let r = 1..\nprint(1)")
        assert "itertools.count(1)" in code

    def test_infinite_range_imports_itertools(self):
        code = transpile("let r = 0..")
        assert "import itertools" in code

    def test_closed_range_unaffected(self):
        assert run_aura("print(list(0..<3))") == "[0, 1, 2]\n"
        assert run_aura("print(list(1..3))") == "[1, 2, 3]\n"


# ============================================================================
# Constants must be initialised
# ============================================================================

class TestConstInitialisation:
    def test_module_const_without_value_is_rejected(self):
        with pytest.raises(SyntaxError, match="requires a value"):
            parse("const K: int")

    def test_class_const_without_value_is_rejected(self):
        with pytest.raises(SyntaxError, match="requires a value"):
            parse("class C { public const K: int }")

    def test_const_with_value_is_accepted(self):
        assert run_aura("const K = 1\nprint(K)") == "1\n"

    def test_const_reassignment_is_rejected(self):
        # The full pipeline (MutabilityChecker) reports E303.
        from aura.transpiler.semantics import MutabilityChecker
        program = parse("const K = 1\nK = 2")
        checker = MutabilityChecker()
        checker.check_program(program)
        assert any(d.code.value == 'E303' for d in checker.diagnostics)


# ============================================================================
# Modifier order (`let private x` vs `private let x`)
# ============================================================================

class TestModifierOrder:
    @pytest.mark.parametrize("src", [
        "class C { let private x: int = 1 }",
        "class C { public let private x: int = 1 }",
        "class C { let mut private x: int = 1 }",
    ])
    def test_trailing_modifier_is_rejected(self, src):
        with pytest.raises(SyntaxError, match="must come before 'let'"):
            parse(src)

    def test_leading_modifier_is_accepted(self):
        decl = parse("class C { private let x: int = 1 }").statements[0]
        field = decl.body[0]
        assert field.name == 'x'
        assert field.visibility == 'private'

    def test_local_declaration_order_guard(self):
        with pytest.raises(SyntaxError, match="must come before 'let'"):
            parse("let private x = 1")
