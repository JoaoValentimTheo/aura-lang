"""Class member integrity: duplicate members (E301) and visibility (E308).

Aura's generated Python class keeps fields, methods, constants and nested
classes in a single namespace, so two members sharing a name silently
overwrite one another. The language documents "no duplicate declarations in
the same scope", and a class body is such a scope. These tests pin that rule
for methods (which the plain scope check did not cover) and that member access
through a direct instantiation (`C().x`) is subject to the same visibility
enforcement as access through a bound instance (`let c = C(); c.x`).
"""
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Parser, Tokenizer  # noqa: E402
from transpiler.rules import RuleChecker  # noqa: E402


def parse(source):
    return Parser(Tokenizer(source).tokenize()).parse()


def rule_errors(source):
    checker = RuleChecker()
    checker.check_program(parse(source))
    return [str(e) for e in checker.collector.errors]


def rule_codes(source):
    return [m.group(0) for e in rule_errors(source)
            if (m := re.search(r"\[([EW]\d+)\]", e))]


# ============================================================================
# E301 - duplicate members
# ============================================================================

def test_duplicate_method_is_rejected():
    src = (
        "class C {\n"
        "  public def f() -> int { return 1 }\n"
        "  public def f() -> int { return 2 }\n"
        "}"
    )
    assert rule_codes(src) == ["[E301]"]


def test_duplicate_method_reports_both_names():
    src = (
        "class C {\n"
        "  public def f() -> int { return 1 }\n"
        "  public def f() -> int { return 2 }\n"
        "}"
    )
    message = rule_errors(src)[0]
    assert "'f'" in message
    assert "method" in message
    assert "'C'" in message


def test_single_method_is_accepted():
    src = "class C { public def f() -> int { return 1 } }"
    assert rule_codes(src) == []


def test_duplicate_zero_arg_new_is_rejected():
    src = (
        "class C {\n"
        "  public def new() {}\n"
        "  public def new() {}\n"
        "}"
    )
    codes = rule_codes(src)
    assert codes == ["[E301]"]


def test_duplicate_new_uses_aura_spelling():
    src = (
        "class C {\n"
        "  public def new() {}\n"
        "  public def new() {}\n"
        "}"
    )
    # The diagnostic must not leak the Python dunder `__init__`.
    assert "'new'" in rule_errors(src)[0]


def test_overloaded_constructors_are_rejected():
    # Aura has one spelling per construct and no overloading: two `new` methods
    # with different arities are a duplicate definition, not an overload.
    src = (
        "class Point {\n"
        "  public let x: int = 0\n"
        "  public let y: int = 0\n"
        "  public def new(x: int, y: int) { self.x = x\n self.y = y }\n"
        "  public def new(x: int) { self.new(x, x) }\n"
        "}"
    )
    assert rule_codes(src) == ["[E301]"]


def test_method_colliding_with_field_is_rejected():
    src = (
        "class C {\n"
        "  public let x: int = 1\n"
        "  public def x() -> int { return 2 }\n"
        "}"
    )
    assert rule_codes(src) == ["[E301]"]


def test_method_colliding_with_const_is_rejected():
    src = (
        "class C {\n"
        "  public const K = 1\n"
        "  public def K() -> int { return 2 }\n"
        "}"
    )
    assert rule_codes(src) == ["[E301]"]


def test_method_colliding_with_nested_class_is_rejected():
    src = (
        "class Outer {\n"
        "  public def Inner() -> int { return 1 }\n"
        "  public class Inner { }\n"
        "}"
    )
    assert rule_codes(src) == ["[E301]"]


def test_duplicate_method_in_trait_is_rejected():
    src = (
        "trait T {\n"
        "  public def f() -> int\n"
        "  public def f() -> int\n"
        "}"
    )
    assert rule_codes(src) == ["[E301]"]


def test_duplicate_nested_class_is_rejected_once():
    # Two nested classes share a name; the scope check already reports it, and
    # the member check must not add a second diagnostic for the same collision.
    src = (
        "class Outer {\n"
        "  public class Inner { }\n"
        "  public class Inner { }\n"
        "}"
    )
    assert rule_codes(src) == ["[E301]"]


def test_duplicate_field_is_rejected_once():
    src = (
        "class C {\n"
        "  public let x: int = 1\n"
        "  public let x: int = 2\n"
        "}"
    )
    assert rule_codes(src) == ["[E301]"]


def test_distinct_members_are_accepted():
    src = (
        "class C {\n"
        "  public let x: int = 1\n"
        "  public let y: int = 2\n"
        "  public def f() -> int { return 1 }\n"
        "  public def g() -> int { return 2 }\n"
        "  public class Inner { }\n"
        "}"
    )
    assert rule_codes(src) == []


# ============================================================================
# E308 - visibility through a direct instantiation
# ============================================================================

def test_private_field_via_direct_instantiation_is_rejected():
    src = (
        "class C {\n"
        "  private let x: int = 1\n"
        "}\n"
        "def main() { print(C().x) }"
    )
    assert rule_codes(src) == ["[E308]"]


def test_protected_field_via_direct_instantiation_is_rejected():
    src = (
        "class C {\n"
        "  protected let x: int = 1\n"
        "}\n"
        "def main() { print(C().x) }"
    )
    assert rule_codes(src) == ["[E308]"]


def test_private_const_via_direct_instantiation_is_rejected():
    src = (
        "class C {\n"
        "  private const K = 9\n"
        "}\n"
        "def main() { print(C().K) }"
    )
    assert rule_codes(src) == ["[E308]"]


def test_private_method_via_direct_instantiation_is_rejected():
    src = (
        "class C {\n"
        "  private def secret() -> int { return 42 }\n"
        "}\n"
        "def main() { print(C().secret()) }"
    )
    assert rule_codes(src) == ["[E308]"]


def test_public_field_via_direct_instantiation_is_accepted():
    src = (
        "class C {\n"
        "  public let x: int = 1\n"
        "}\n"
        "def main() { print(C().x) }"
    )
    assert rule_codes(src) == []


def test_direct_and_bound_access_agree_on_visibility():
    def cls(vis):
        return "class C {\n  " + vis + " let x: int = 1\n}\n"

    for vis in ("private", "protected"):
        direct = cls(vis) + "def main() { print(C().x) }"
        bound = cls(vis) + "def main() { let c = C()\n print(c.x) }"
        assert rule_codes(direct) == ["[E308]"]
        assert rule_codes(bound) == ["[E308]"]
    assert rule_codes(cls("public") + "def main() { print(C().x) }") == []
    assert rule_codes(
        cls("public") + "def main() { let c = C()\n print(c.x) }") == []


def test_private_access_inside_own_class_is_accepted():
    src = (
        "class C {\n"
        "  private let x: int = 1\n"
        "  public def get() -> int { return self.x }\n"
        "}"
    )
    assert rule_codes(src) == []


def test_private_access_from_subclass_is_rejected():
    src = (
        "class A {\n"
        "  private let x: int = 1\n"
        "}\n"
        "class B extends A {\n"
        "  public def leak() -> int { return self.x }\n"
        "}"
    )
    assert rule_codes(src) == ["[E308]"]
