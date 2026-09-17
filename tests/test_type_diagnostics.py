"""Tests for structured type diagnostics (E1xx) and their codes/locations.

The type checker emits ``AuraError`` diagnostics with a code, a real location
and a hint, exactly like the rule and mutability checkers.
"""
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.types import TypeChecker  # noqa: E402


def check(source):
    program = Parser(Tokenizer(source).tokenize()).parse()
    checker = TypeChecker()
    checker.check_program(program)
    return checker


def codes(source):
    return [d.code.value for d in check(source).diagnostics]


def diag(source):
    return check(source).diagnostics[0]


# ---------------------------------------------------------------------------
# Codes
# ---------------------------------------------------------------------------

def test_variable_type_mismatch_is_e101():
    assert codes('def main() { let x: int = "s" }') == ["E101"]


def test_return_type_mismatch_is_e101():
    assert codes('def f() -> int { return "s" }\ndef main() { print(f()) }') \
        == ["E101"]


def test_condition_type_mismatch_is_e101():
    assert codes("def main() { if 1 { print(2) } }") == ["E101"]


def test_incompatible_operands_is_e108():
    assert codes('def main() { let x: int = 1 + "s" }') == ["E108"]


def test_wrong_argument_type_is_e106():
    assert codes('def f(a: int) -> int { return a }\ndef main() { f("s") }') \
        == ["E106"]


def test_wrong_argument_count_is_e105():
    assert codes("def f(a: int) -> int { return a }\ndef main() { f(1, 2, 3) }") \
        == ["E105"]


def test_unused_type_parameter_is_w103():
    checker = check("class Box[T] { public let x: int = 1 }")
    assert [d.code.value for d in checker.diagnostics] == ["W103"]
    # A warning must not fail the check.
    assert checker.errors == []


# ---------------------------------------------------------------------------
# Locations and hints
# ---------------------------------------------------------------------------

def test_type_diagnostic_has_location():
    d = diag('def main() { let x: int = "s" }')
    assert d.location is not None
    assert d.location.line == 1


def test_type_diagnostic_location_points_at_declaration_line():
    d = diag('def main() {\n  let x: int = "s"\n}')
    assert d.location is not None
    assert d.location.line == 2


def test_type_diagnostic_formats_with_code_and_location():
    text = str(diag('def main() { let x: int = "s" }'))
    assert "[E101]" in text
    assert ":1:" in text


# ---------------------------------------------------------------------------
# Well-typed programs produce no diagnostics
# ---------------------------------------------------------------------------

def test_valid_program_has_no_type_diagnostics():
    src = (
        "def add(a: int, b: int) -> int { return a + b }\n"
        "def main() {\n"
        "  let x: int = add(1, 2)\n"
        "  print(x)\n"
        "}"
    )
    assert codes(src) == []


def test_any_typed_code_is_not_flagged():
    assert codes("def main() { let x = 1\n print(x) }") == []


def test_generic_class_with_used_param_is_clean():
    src = (
        "class Box[T] {\n"
        "  public let item: T = none\n"
        "  public def get() -> T { return self.item }\n"
        "}"
    )
    assert codes(src) == []
