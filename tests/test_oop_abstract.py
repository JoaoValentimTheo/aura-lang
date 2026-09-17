"""Abstract-method enforcement (E309) and trait inheritance tests.

A trait compiles to an abstract base class; every method declared without a
body is a pure signature. The RuleChecker must reject a concrete class that
fails to implement any abstract method it inherits (from a directly
implemented trait or transitively through trait/class inheritance), and traits
may extend other traits.
"""
import contextlib
import io
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
    return [e.split()[1] for e in rule_errors(source)]


def run_aura(source):
    code = Transformer().transform(parse(source))
    namespace = {"__name__": "__abstract_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, "<test>", "exec"), namespace)
    return buffer.getvalue(), code


# ============================================================================
# E309 - unimplemented abstract method
# ============================================================================

def test_unimplemented_trait_method_is_rejected():
    src = (
        "trait Shape { public def area() -> float }\n"
        "class Square implements Shape { public let s: float = 2.0 }"
    )
    assert rule_codes(src) == ["[E309]"]


def test_implemented_trait_method_is_accepted():
    src = (
        "trait Shape { public def area() -> float }\n"
        "class Square implements Shape {\n"
        "  public let s: float = 2.0\n"
        "  public def area() -> float { return self.s * self.s }\n"
        "}"
    )
    assert rule_codes(src) == []


def test_error_names_class_method_and_trait():
    src = (
        "trait Shape { public def area() -> float }\n"
        "class Square implements Shape { public let s: float = 2.0 }"
    )
    message = rule_errors(src)[0]
    assert "'Square'" in message
    assert "'area'" in message
    assert "'Shape'" in message


def test_trait_itself_is_not_required_to_implement():
    src = "trait Shape { public def area() -> float }"
    assert rule_codes(src) == []


def test_trait_with_body_is_not_abstract():
    src = (
        "trait Greeter { public def greet() -> str { return 'hi' } }\n"
        "class Person implements Greeter { }"
    )
    assert rule_codes(src) == []


def test_multiple_traits_all_required():
    src = (
        "trait A { public def a() -> int }\n"
        "trait B { public def b() -> int }\n"
        "class C implements A, B { public def a() -> int { return 1 } }"
    )
    assert rule_codes(src) == ["[E309]"]


def test_multiple_traits_all_implemented():
    src = (
        "trait A { public def a() -> int }\n"
        "trait B { public def b() -> int }\n"
        "class C implements A, B {\n"
        "  public def a() -> int { return 1 }\n"
        "  public def b() -> int { return 2 }\n"
        "}"
    )
    assert rule_codes(src) == []


def test_inherited_implementation_satisfies_trait():
    src = (
        "trait Shape { public def area() -> float }\n"
        "class Base implements Shape {\n"
        "  public def area() -> float { return 0.0 }\n"
        "}\n"
        "class Circle(Base) { }"
    )
    assert rule_codes(src) == []


def test_override_still_satisfies_trait():
    src = (
        "trait Shape { public def area() -> float }\n"
        "class Base implements Shape {\n"
        "  public def area() -> float { return 0.0 }\n"
        "}\n"
        "class Circle(Base) {\n"
        "  public def area() -> float { return 1.0 }\n"
        "}"
    )
    assert rule_codes(src) == []


def test_runtime_abstract_instantiation_fails():
    src = (
        "trait Shape { public def area() -> float }\n"
        "class Square implements Shape { public let s: float = 2.0 }\n"
        "let q = Square()"
    )
    with pytest.raises(TypeError):
        run_aura(src)


# ============================================================================
# Trait inheritance: `trait B implements A` and `trait B(A)`
# ============================================================================

def test_trait_extends_trait_implements_keyword():
    src = (
        "trait Greeter { public def greet() -> str }\n"
        "trait Loud implements Greeter {\n"
        "  public def shout() -> str\n"
        "}\n"
        "class Person implements Loud {\n"
        "  public def greet() -> str { return 'hi' }\n"
        "  public def shout() -> str { return 'HEY' }\n"
        "}"
    )
    assert rule_codes(src) == []


def test_trait_extends_trait_parenthesised():
    src = (
        "trait Greeter { public def greet() -> str }\n"
        "trait Loud(Greeter) { public def shout() -> str }\n"
        "class Person implements Loud {\n"
        "  public def greet() -> str { return 'hi' }\n"
        "  public def shout() -> str { return 'HEY' }\n"
        "}"
    )
    assert rule_codes(src) == []


def test_trait_inheritance_requires_parent_methods():
    src = (
        "trait Greeter { public def greet() -> str }\n"
        "trait Loud implements Greeter { public def shout() -> str }\n"
        "class Person implements Loud {\n"
        "  public def shout() -> str { return 'HEY' }\n"
        "}"
    )
    assert rule_codes(src) == ["[E309]"]


def test_trait_inheritance_runtime_behaviour():
    out, _ = run_aura(
        "trait Greeter { public def greet() -> str }\n"
        "trait Loud(Greeter) { public def shout() -> str }\n"
        "class Person implements Loud {\n"
        "  public def greet() -> str { return 'hi' }\n"
        "  public def shout() -> str { return 'HEY' }\n"
        "}\n"
        "print(Person().greet() + Person().shout())"
    )
    assert out == "hiHEY\n"


def test_trait_inheritance_chained_three_levels():
    src = (
        "trait A { public def a() -> int }\n"
        "trait B implements A { public def b() -> int }\n"
        "trait C implements B { public def c() -> int }\n"
        "class Impl implements C {\n"
        "  public def a() -> int { return 1 }\n"
        "  public def b() -> int { return 2 }\n"
        "  public def c() -> int { return 3 }\n"
        "}"
    )
    assert rule_codes(src) == []


def test_trait_inheritance_chain_missing_leaf_method():
    src = (
        "trait A { public def a() -> int }\n"
        "trait B implements A { public def b() -> int }\n"
        "trait C implements B { public def c() -> int }\n"
        "class Impl implements C {\n"
        "  public def a() -> int { return 1 }\n"
        "  public def b() -> int { return 2 }\n"
        "}"
    )
    assert rule_codes(src) == ["[E309]"]
