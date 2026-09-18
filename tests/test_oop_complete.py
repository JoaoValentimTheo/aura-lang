"""Comprehensive OOP-coverage tests for Aura.

Exercises the full object-oriented surface end to end, asserting on real
runtime behaviour:

  * classes with and without an explicit constructor (`new` / `init`)
  * field declarations: `let`, `let mut`, typed, defaulted, static, private,
    protected, public
  * inheritance (single, multiple, dotted base names) and `super`
  * traits as abstract base classes, with default and abstract methods
  * methods: instance, static, class, property, dunder protocols
  * visibility: private/protected name mangling for fields and methods
  * nested classes, generics, enums, pattern matching over constructors
  * auto-generated constructors, `__match_args__`, and default override
"""
import contextlib
import io
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Tokenizer, Parser  # noqa: E402
from transpiler.transformer import Transformer  # noqa: E402


def run_aura(source: str):
    """Transpile Aura source, execute it, and return captured stdout."""
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    code = Transformer().transform(program)
    namespace = {"__name__": "__aura_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, namespace)
    return buffer.getvalue()


def run_aura_ns(source: str):
    """Like run_aura but also return the namespace for introspection."""
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    code = Transformer().transform(program)
    namespace = {"__name__": "__aura_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, namespace)
    return buffer.getvalue(), namespace


def transpile(source: str):
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    return Transformer().transform(program)


# ============================================================================
# Construction
# ============================================================================

def test_class_with_new_constructor():
    out = run_aura(
        "class Point {\n"
        "  let x: int = 0\n"
        "  let y: int = 0\n"
        "  def new(x: int, y: int) { self.x = x\nself.y = y }\n"
        "}\n"
        "let p = Point(3, 4)\n"
        "print(p.x)\nprint(p.y)"
    )
    assert out == "3\n4\n"


def test_init_is_rejected_as_constructor():
    """The canonical constructor is `new`; `init` is a clear error."""
    with pytest.raises(SyntaxError):
        run_aura(
            "class Point {\n"
            "  let x: int = 0\n"
            "  def init(x: int) { self.x = x }\n"
            "}\n"
        )


def test_class_with_new_constructor_two():
    out = run_aura(
        "class Point {\n"
        "  let x: int = 0\n"
        "  def new(x: int) { self.x = x }\n"
        "}\n"
        "print(Point(9).x)"
    )
    assert out == "9\n"


def test_auto_generated_constructor():
    out = run_aura(
        "class User {\n"
        "  let name: str = 'anon'\n"
        "  let age: int = 0\n"
        "}\n"
        "let u = User('Ann', 30)\n"
        "print(u.name)\nprint(u.age)"
    )
    assert out == "Ann\n30\n"


def test_auto_constructor_positional_and_keyword():
    out = run_aura(
        "class P { let a: int = 1\nlet b: int = 2 }\n"
        "let p = P(b=20)\n"
        "print(p.a)\nprint(p.b)"
    )
    assert out == "1\n20\n"


def test_field_default_override_with_none():
    """An explicit None must override a default, not fall back to it."""
    out = run_aura(
        "class P { let x: int = 5 }\n"
        "let p = P(None)\n"
        "print(p.x)"
    )
    assert out == "None\n"


def test_field_default_used_when_omitted():
    out = run_aura(
        "class P { let x: int = 7 }\n"
        "print(P().x)"
    )
    assert out == "7\n"


def test_match_args_generated():
    _, ns = run_aura_ns(
        "class P { let x: int = 0\nlet y: int = 0 }"
    )
    assert ns["P"].__match_args__ == ("x", "y")


# ============================================================================
# Inheritance
# ============================================================================

def test_single_inheritance_override():
    out = run_aura(
        "class Animal {\n"
        "  def speak() -> str { return '...' }\n"
        "}\n"
        "class Dog extends Animal {\n"
        "  def speak() -> str { return 'woof' }\n"
        "}\n"
        "print(Dog().speak())"
    )
    assert out == "woof\n"


def test_super_method_call():
    out = run_aura(
        "class A {\n"
        "  def greet() -> str { return 'A' }\n"
        "}\n"
        "class B extends A {\n"
        "  def greet() -> str { return super.greet() + 'B' }\n"
        "}\n"
        "print(B().greet())"
    )
    assert out == "AB\n"


def test_super_constructor_call():
    out = run_aura(
        "class A {\n"
        "  let name: str = ''\n"
        "  def new(name: str) { self.name = name }\n"
        "}\n"
        "class B extends A {\n"
        "  def new(name: str) { super.new(name) }\n"
        "}\n"
        "print(B('Zed').name)"
    )
    assert out == "Zed\n"


def test_multiple_inheritance():
    out = run_aura(
        "class A { def a() -> str { return 'a' } }\n"
        "class B { def b() -> str { return 'b' } }\n"
        "class C extends A, B { }\n"
        "let c = C()\n"
        "print(c.a() + c.b())"
    )
    assert out == "ab\n"


def test_inherited_fields_available():
    out = run_aura(
        "class Base { let kind: str = 'base' }\n"
        "class Derived extends Base { let extra: int = 1 }\n"
        "let d = Derived()\n"
        "print(d.kind)\nprint(d.extra)"
    )
    assert out == "base\n1\n"


# ============================================================================
# Methods: static / class / property
# ============================================================================

def test_static_method():
    out = run_aura(
        "class Math {\n"
        "  @staticmethod\n"
        "  def double(x: int) -> int { return x * 2 }\n"
        "}\n"
        "print(Math.double(5))"
    )
    assert out == "10\n"


def test_class_method():
    out = run_aura(
        "class Factory {\n"
        "  let kind: str = 'x'\n"
        "  @classmethod\n"
        "  def create(cls) { return cls() }\n"
        "}\n"
        "print(Factory.create().kind)"
    )
    assert out == "x\n"


def test_property():
    out = run_aura(
        "class Circle {\n"
        "  let r: float = 2.0\n"
        "  @property\n"
        "  def area() -> float { return 3.0 * self.r * self.r }\n"
        "}\n"
        "print(Circle().area)"
    )
    assert out == "12.0\n"


def test_static_field_shared_and_mutable():
    out = run_aura(
        "class Counter {\n"
        "  public static let count: int = 0\n"
        "  public static def inc() { Counter.count = Counter.count + 1 }\n"
        "}\n"
        "Counter.inc()\nCounter.inc()\n"
        "print(Counter.count)"
    )
    assert out == "2\n"


# ============================================================================
# Visibility and name mangling
# ============================================================================

def test_private_field_mangled():
    code = transpile(
        "class S {\n"
        "  private let secret: int = 1\n"
        "  def get() { return self.secret }\n"
        "}"
    )
    assert "__secret" in code

    out = run_aura(
        "class S {\n"
        "  private let secret: int = 42\n"
        "  def get() { return self.secret }\n"
        "}\n"
        "print(S().get())"
    )
    assert out == "42\n"


def test_private_method_mangled():
    code = transpile(
        "class S {\n"
        "  private def hidden() { return 1 }\n"
        "  def visible() { return self.hidden() }\n"
        "}"
    )
    assert "__hidden" in code


def test_protected_member_prefix():
    code = transpile(
        "class S {\n"
        "  protected let value: int = 1\n"
        "}"
    )
    assert "_value" in code


def test_private_protocol_method_not_double_mangled():
    code = transpile(
        "class S {\n"
        "  private def str() -> str { return 's' }\n"
        "}"
    )
    # A private protocol method must stay a valid dunder, not `____str__`.
    assert "def __str__(self)" in code
    assert "____str__" not in code


# ============================================================================
# Dunder / protocol methods
# ============================================================================

def test_str_protocol():
    out = run_aura(
        "class P {\n"
        "  let x: int = 0\n"
        "  def new(x: int) { self.x = x }\n"
        "  def str() -> str { return f'P({self.x})' }\n"
        "}\n"
        "print(str(P(5)))"
    )
    assert out == "P(5)\n"


def test_len_protocol():
    out = run_aura(
        "class Bag {\n"
        "  let n: int = 3\n"
        "  def len() -> int { return self.n }\n"
        "}\n"
        "print(len(Bag()))"
    )
    assert out == "3\n"


def test_eq_and_lt_protocols():
    out = run_aura(
        "class V {\n"
        "  let x: int = 0\n"
        "  def new(x: int) { self.x = x }\n"
        "  def eq(other) -> bool { return self.x == other.x }\n"
        "  def lt(other) -> bool { return self.x < other.x }\n"
        "}\n"
        "print(V(1) == V(1))\nprint(V(1) < V(2))"
    )
    assert out == "True\nTrue\n"


def test_bool_protocol():
    out = run_aura(
        "class F {\n"
        "  def bool() -> bool { return false }\n"
        "}\n"
        "if F() { print('truthy') } else { print('falsy') }"
    )
    assert out == "falsy\n"


def test_iter_protocol():
    out = run_aura(
        "class Count {\n"
        "  let n: int = 3\n"
        "  def iter() {\n"
        "    let i = 0\n"
        "    while i < self.n {\n"
        "      yield i\n"
        "      i += 1\n"
        "    }\n"
        "  }\n"
        "}\n"
        "for x in Count() { print(x) }"
    )
    assert out == "0\n1\n2\n"


def test_getitem_protocol():
    out = run_aura(
        "class Seq {\n"
        "  let items: list = [10, 20, 30]\n"
        "  def getitem(i: int) -> int { return self.items[i] }\n"
        "}\n"
        "print(Seq()[1])"
    )
    assert out == "20\n"


def test_contains_protocol():
    out = run_aura(
        "class Bag {\n"
        "  def new() { }\n"
        "  def contains(x: int) -> bool { return x == 5 }\n"
        "}\n"
        "print(5 in Bag())"
    )
    assert out == "True\n"


def test_call_protocol():
    out = run_aura(
        "class Adder {\n"
        "  def new() { }\n"
        "  def call(x: int) -> int { return x + 1 }\n"
        "}\n"
        "print(Adder()(41))"
    )
    assert out == "42\n"


def test_add_protocol():
    out = run_aura(
        "class N {\n"
        "  let v: int = 0\n"
        "  def new(v: int) { self.v = v }\n"
        "  def add(other) -> int { return self.v + other.v }\n"
        "}\n"
        "print(N(2) + N(3))"
    )
    assert out == "5\n"


def test_hash_protocol():
    out = run_aura(
        "class K {\n"
        "  let v: int = 0\n"
        "  def new(v: int) { self.v = v }\n"
        "  def hash() -> int { return self.v }\n"
        "}\n"
        "let s = {K(1), K(2)}\n"
        "print(len(s))"
    )
    assert out == "2\n"


# ============================================================================
# Traits
# ============================================================================

def test_trait_compiles_to_abc():
    code = transpile(
        "trait Shape {\n"
        "  def area() -> float\n"
        "}"
    )
    assert "class Shape(_aura_abc.ABC):" in code
    assert "@_aura_abc.abstractmethod" in code


def test_trait_abstract_blocks_instantiation():
    tokens = Tokenizer(
        "trait Shape {\n"
        "  def area() -> float\n"
        "}\n"
        "let s = Shape()"
    ).tokenize()
    program = Parser(tokens).parse()
    code = Transformer().transform(program)
    with pytest.raises(TypeError):
        exec(code, {"__name__": "__aura_test__"})


def test_trait_default_method():
    out = run_aura(
        "trait Greeter {\n"
        "  def greet(name: str) -> str { return 'hi ' + name }\n"
        "}\n"
        "class Person extends Greeter { }\n"
        "print(Person().greet('bob'))"
    )
    assert out == "hi bob\n"


def test_trait_with_implementation():
    out = run_aura(
        "trait Shape {\n"
        "  def area() -> float { return 0.0 }\n"
        "}\n"
        "class Square extends Shape {\n"
        "  let side: float = 3.0\n"
        "  def area() -> float { return self.side * self.side }\n"
        "}\n"
        "print(Square().area())"
    )
    assert out == "9.0\n"


def test_trait_field_default():
    out = run_aura(
        "trait Named {\n"
        "  let name: str = 'anon'\n"
        "}\n"
        "class Thing extends Named { }\n"
        "print(Thing().name)"
    )
    assert out == "anon\n"


def test_class_extends_multiple_traits():
    out = run_aura(
        "trait A { def a() -> str { return 'a' } }\n"
        "trait B { def b() -> str { return 'b' } }\n"
        "class C extends A, B { }\n"
        "print(C().a() + C().b())"
    )
    assert out == "ab\n"


# ============================================================================
# Generics
# ============================================================================

def test_class_generics():
    code = transpile(
        "class Box[T] {\n"
        "  let item: T = none\n"
        "}"
    )
    assert "_aura_Generic" in code


def test_generic_class_usable():
    out = run_aura(
        "class Box[T] {\n"
        "  let item: T = none\n"
        "  def new(item: T) { self.item = item }\n"
        "}\n"
        "print(Box(42).item)"
    )
    assert out == "42\n"


def test_generic_trait():
    code = transpile(
        "trait Container[T] {\n"
        "  def get() -> T\n"
        "}"
    )
    assert "_aura_Generic" in code


# ============================================================================
# Nested classes and enums
# ============================================================================

def test_nested_class():
    out = run_aura(
        "class Outer {\n"
        "  class Inner {\n"
        "    let v: int = 5\n"
        "  }\n"
        "}\n"
        "print(Outer.Inner().v)"
    )
    assert out == "5\n"


def test_enum_attributes():
    out = run_aura(
        "enum Direction { North, South, East, West }\n"
        "print(Direction.West.value)"
    )
    assert out == "4\n"


def test_match_over_constructor():
    out = run_aura(
        "class Point { let x: int = 0\nlet y: int = 0 }\n"
        "let p = Point(1, 2)\n"
        "match p {\n"
        "  case Point(x, y) { print(x + y) }\n"
        "  case _ { print('other') }\n"
        "}"
    )
    assert out == "3\n"


# ============================================================================
# Combined / integration
# ============================================================================

def test_full_bank_account_example():
    out = run_aura(
        "class BankAccount {\n"
        "  let owner: str = ''\n"
        "  let balance: float = 0.0\n"
        "  def new(owner: str, balance: float = 0.0) {\n"
        "    self.owner = owner\n"
        "    self.balance = balance\n"
        "  }\n"
        "  def deposit(amount: float) {\n"
        "    self.balance += amount\n"
        "  }\n"
        "  def withdraw(amount: float) -> bool {\n"
        "    if amount <= self.balance {\n"
        "      self.balance -= amount\n"
        "      return true\n"
        "    }\n"
        "    return false\n"
        "  }\n"
        "}\n"
        "let a = BankAccount('Alice', 100.0)\n"
        "a.deposit(50.0)\n"
        "print(a.balance)\n"
        "print(a.withdraw(500.0))\n"
        "print(a.balance)"
    )
    assert out == "150.0\nFalse\n150.0\n"


def test_polymorphism_via_base_list():
    out = run_aura(
        "class Shape {\n"
        "  def area() -> float { return 0.0 }\n"
        "}\n"
        "class Square extends Shape {\n"
        "  let s: float = 2.0\n"
        "  def area() -> float { return self.s * self.s }\n"
        "}\n"
        "class Circle extends Shape {\n"
        "  let r: float = 1.0\n"
        "  def area() -> float { return 3.0 * self.r * self.r }\n"
        "}\n"
        "let shapes = [Square(), Circle()]\n"
        "let mut total = 0.0\n"
        "for s in shapes { total += s.area() }\n"
        "print(total)"
    )
    assert out == "7.0\n"