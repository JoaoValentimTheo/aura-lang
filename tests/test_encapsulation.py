"""Encapsulation tests for Aura's class-member visibility model.

Every class/trait member must declare an explicit ``public`` / ``private`` /
``protected`` modifier (E307). Members are enforced both at compile time
(E308, via the RuleChecker) and at runtime (via owner-aware Python name
mangling), and non-public fields get auto-generated ``get_<name>()`` /
``set_<name>(value)`` accessors.
"""
import contextlib
import io
import re
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Parser, Tokenizer  # noqa: E402
from transpiler.rules import RuleChecker  # noqa: E402
from transpiler.transformer import Transformer  # noqa: E402


def parse(source):
    return Parser(Tokenizer(source).tokenize()).parse()


def rule_errors(source):
    checker = RuleChecker()
    checker.check_program(parse(source))
    return [str(e) for e in checker.collector.errors]


def rule_codes(source):
    out = []
    for e in rule_errors(source):
        m = re.search(r"\[([EW]\d+)\]", str(e))
        if m:
            out.append(m.group(0))
    return out




def run_aura(source):
    code = Transformer().transform(parse(source))
    namespace = {"__name__": "__encap_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, "<test>", "exec"), namespace)
    return buffer.getvalue(), code


# ============================================================================
# E307 - missing visibility modifier is a compile error
# ============================================================================

def test_missing_visibility_field_is_rejected():
    assert rule_codes("class C { let x = 1 }") == ["[E307]"]


def test_missing_visibility_method_is_rejected():
    assert rule_codes("class C { def m() {} }") == ["[E307]"]


def test_missing_visibility_nested_class_is_rejected():
    assert rule_codes("class C { class N {} }") == ["[E307]"]


def test_missing_visibility_trait_member_is_rejected():
    assert rule_codes("trait T { let x = 1 }") == ["[E307]"]


def test_all_members_with_modifiers_are_accepted():
    assert rule_errors(
        "class C {\n"
        "  public let x = 1\n"
        "  private let y = 2\n"
        "  protected let z = 3\n"
        "  public def a() {}\n"
        "  private def b() {}\n"
        "  protected def c() {}\n"
        "}"
    ) == []


# ============================================================================
# E308 - inaccessible member access at compile time
# ============================================================================

def test_external_private_access_is_rejected():
    assert rule_codes(
        "class C { private let x = 1 }\nlet c = C()\nlet y = c.x"
    ) == ["[E308]"]


def test_external_protected_access_is_rejected():
    assert rule_codes(
        "class C { protected let x = 1 }\nlet c = C()\nlet y = c.x"
    ) == ["[E308]"]


def test_subclass_private_access_is_rejected():
    assert rule_codes(
        "class B { private let x = 1 }\n"
        "class D extends B { public def m() { return self.x } }"
    ) == ["[E308]"]


def test_internal_private_access_is_accepted():
    assert rule_errors(
        "class C { private let x = 1; public def m() { return self.x } }"
    ) == []


def test_subclass_protected_access_is_accepted():
    assert rule_errors(
        "class B { protected let x = 1 }\n"
        "class D extends B { public def m() { return self.x } }"
    ) == []


def test_external_public_access_is_accepted():
    assert rule_errors(
        "class C { public let x = 1 }\nlet c = C()\nlet y = c.x"
    ) == []


def test_accessor_call_is_accepted():
    assert rule_errors(
        "class C { private let x = 1; public def get_x() { return self.x } }\n"
        "let c = C()\nlet y = c.get_x()"
    ) == []


# ============================================================================
# Auto-generated accessors
# ============================================================================

def test_private_and_protected_get_auto_accessors():
    out, code = run_aura(
        "class C {\n"
        "  private let mut x = 5\n"
        "  protected let mut y = 6\n"
        "  public let z = 7\n"
        "}\n"
        "let c = C()\n"
        "c.set_x(10)\n"
        "c.set_y(20)\n"
        "print(c.get_x(), c.get_y(), c.z)"
    )
    assert out.strip() == "10 20 7"
    assert "def get_x" in code and "def set_x" in code
    assert "def get_y" in code and "def set_y" in code
    # Every field gets a getter. `let` fields are immutable, so no setter.
    assert "def get_z" in code
    assert "def set_z" not in code


def test_manual_accessor_override_wins():
    out, _ = run_aura(
        "class C {\n"
        "  private let x = 5\n"
        "  public def get_x() { return 99 }\n"
        "}\n"
        "print(C().get_x())"
    )
    assert out.strip() == "99"


# ============================================================================
# Owner-aware mangling (the private-in-subclass fix)
# ============================================================================

def test_subclass_reads_inherited_private_via_getter():
    out, _ = run_aura(
        "class Base {\n"
        "  private let secret = 42\n"
        "  public def get_secret() { return self.secret }\n"
        "}\n"
        "class Derived extends Base {\n"
        "  public def reveal() { return self.secret }\n"
        "}\n"
        "let d = Derived()\n"
        "print(d.get_secret(), d.reveal())"
    )
    assert out.strip() == "42 42"


def test_subclass_private_keeps_its_own_owner():
    out, code = run_aura(
        "class Base {\n"
        "  private let b = 1\n"
        "  public def get_b() { return self.b }\n"
        "}\n"
        "class Derived extends Base {\n"
        "  private let d = 2\n"
        "  public def get_d() { return self.d }\n"
        "}\n"
        "let x = Derived()\n"
        "print(x.get_b(), x.get_d())"
    )
    assert out.strip() == "1 2"
    # Base's private field is stored under Base's owner prefix, Derived's under
    # its own - so two private members with the same name never collide.
    assert "self._Base__b" in code
    assert "self._Derived__d" in code


def test_protected_is_shared_with_subclass_at_runtime():
    out, _ = run_aura(
        "class Base {\n"
        "  protected let shared = 10\n"
        "  public def get_shared() { return self.shared }\n"
        "}\n"
        "class Derived extends Base {\n"
        "  public def bump() { self.shared = self.shared + 1 }\n"
        "}\n"
        "let d = Derived()\n"
        "d.bump()\n"
        "print(d.get_shared())"
    )
    assert out.strip() == "11"


# ============================================================================
# Runtime enforcement via name mangling
# ============================================================================

def test_external_private_access_fails_at_runtime():
    with pytest.raises(AttributeError):
        run_aura(
            "class C { private let x = 5 }\nlet c = C()\nprint(c.x)"
        )


def test_external_protected_access_fails_at_runtime():
    with pytest.raises(AttributeError):
        run_aura(
            "class C { protected let x = 5 }\nlet c = C()\nprint(c.x)"
        )


def test_external_private_method_call_fails_at_runtime():
    with pytest.raises(AttributeError):
        run_aura(
            "class C { private def secret() { return 1 } }\n"
            "print(C().secret())"
        )
