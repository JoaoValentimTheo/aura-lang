"""Extreme tests for data structures and the object system.

Covers the boundaries of collections, generics, enums, pattern matching and
the object model (inheritance depth, MRO, properties, class/static methods,
visibility, abstract contracts, dunder protocols) by executing real programs.
"""
import contextlib
import io
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.transformer import Transformer  # noqa: E402


def run(source):
    program = Parser(Tokenizer(source).tokenize()).parse()
    code = Transformer().transform(program)
    namespace = {"__name__": "__extreme__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, "<extreme>", "exec"), namespace)
        main = namespace.get("main")
        if callable(main):
            main()
    return buffer.getvalue()


def transpile(source):
    program = Parser(Tokenizer(source).tokenize()).parse()
    return Transformer().transform(program)


# ============================================================================
# Structures: collections at the edges
# ============================================================================

def test_empty_list_operations():
    assert run(
        "def main() {\n"
        "  let xs = []\n"
        "  print(len(xs))\n"
        "  print(sum(xs))\n"
        "}"
    ) == "0\n0\n"


def test_nested_data_structure():
    out = run(
        "def main() {\n"
        "  let data = {users: [{name: 'a', tags: [1, 2]}]}\n"
        "  print(data.users[0].name)\n"
        "  print(len(data.users[0].tags))\n"
        "}"
    )
    assert out == "a\n2\n"


def test_list_comprehension_with_filter():
    assert run(
        "def main() {\n"
        "  let xs = [x * x for x in range(20) if x % 3 == 0]\n"
        "  print(xs)\n"
        "}"
    ) == "[0, 9, 36, 81, 144, 225, 324]\n"


def test_dict_comprehension():
    assert run(
        "def main() {\n"
        "  let d = {k: k * 2 for k in range(4)}\n"
        "  print(d[3])\n"
        "}"
    ) == "6\n"


def test_set_deduplication():
    assert run(
        "def main() {\n"
        "  let s = {1, 2, 2, 3, 3, 3}\n"
        "  print(len(s))\n"
        "}"
    ) == "3\n"


def test_tuple_destructuring_swap():
    assert run(
        "def main() {\n"
        "  let mut a = 1\n  let mut b = 2\n"
        "  a, b = b, a\n"
        "  print(a, b)\n"
        "}"
    ) == "2 1\n"


def test_slicing_reverse_and_step():
    assert run(
        "def main() {\n"
        "  let xs = [1, 2, 3, 4, 5]\n"
        "  print(xs[::-1])\n"
        "  print(xs[1:4])\n"
        "  print(xs[::2])\n"
        "}"
    ) == "[5, 4, 3, 2, 1]\n[2, 3, 4]\n[1, 3, 5]\n"


def test_string_unicode_indexing():
    assert run(
        'def main() {\n  print("café"[3])\n}'
    ) == "é\n"


def test_deeply_nested_lists():
    depth = 30
    src = "def main() { let x = " + "[" * depth + "1" + "]" * depth + "\n print(1) }"
    assert run(src) == "1\n"


# ============================================================================
# OOP: inheritance and MRO
# ============================================================================

def test_four_level_inheritance_chain():
    out = run(
        "class A { public def v() -> int { return 1 } }\n"
        "class B(A) { public def v() -> int { return 2 } }\n"
        "class C(B) { public def v() -> int { return 3 } }\n"
        "class D(C) { public def v() -> int { return super.v() + 10 } }\n"
        "def main() { print(D().v()) }"
    )
    assert out == "13\n"


def test_multiple_inheritance_method_resolution():
    out = run(
        "class A { public def a() -> str { return 'a' } }\n"
        "class B { public def b() -> str { return 'b' } }\n"
        "class C(A, B) { }\n"
        "def main() { let c = C()\n print(c.a() + c.b()) }"
    )
    assert out == "ab\n"


def test_super_constructor_chain():
    out = run(
        "class A {\n  public let x: int = 0\n"
        "  public def new(x: int) { self.x = x }\n}\n"
        "class B(A) {\n  public let y: int = 0\n"
        "  public def new(x: int, y: int) { super.new(x)\n self.y = y }\n}\n"
        "def main() { let b = B(1, 2)\n print(b.x, b.y) }"
    )
    assert out == "1 2\n"


def test_inherited_field_default():
    out = run(
        "class Base { public let kind: str = 'base' }\n"
        "class Derived(Base) { public let extra: int = 7 }\n"
        "def main() { let d = Derived()\n print(d.kind, d.extra) }"
    )
    assert out == "base 7\n"


# ============================================================================
# OOP: properties, static and class methods
# ============================================================================

def test_property_computed_value():
    out = run(
        "class Circle {\n  public let r: float = 2.0\n"
        "  public @property\n  def area() -> float { return 3.0 * self.r * self.r }\n}\n"
        "def main() { print(Circle().area) }"
    )
    assert out == "12.0\n"


def test_static_method_and_field():
    out = run(
        "class C {\n  public static let count: int = 0\n"
        "  public static def bump() { C.count = C.count + 1 }\n}\n"
        "def main() { C.bump()\n C.bump()\n print(C.count) }"
    )
    assert out == "2\n"


def test_classmethod_factory():
    out = run(
        "class F {\n  public let v: int = 0\n"
        "  public @classmethod\n  def make() { return cls() }\n}\n"
        "def main() { print(F.make().v) }"
    )
    assert out == "0\n"


# ============================================================================
# OOP: visibility, accessors and abstract contracts
# ============================================================================

def test_private_field_via_accessors():
    out = run(
        "class Account {\n  private let balance: float = 0.0\n}\n"
        "def main() {\n  let a = Account()\n"
        "  a.set_balance(100.0)\n  print(a.get_balance()) }"
    )
    assert out == "100.0\n"


def test_protected_accessible_in_subclass():
    out = run(
        "class A { protected let x: int = 5\n  public def read() -> int { return self.x } }\n"
        "class B(A) { public def double() -> int { return self.x * 2 } }\n"
        "def main() { print(B().double()) }"
    )
    assert out == "10\n"


def test_abstract_method_enforced_then_implemented():
    out = run(
        "trait Shape { public def area() -> float }\n"
        "class Square implements Shape {\n  public let s: float = 3.0\n"
        "  public def area() -> float { return self.s * self.s }\n}\n"
        "def main() { print(Square().area()) }"
    )
    assert out == "9.0\n"


def test_trait_default_method():
    out = run(
        "trait Greeter { public def greet(n: str) -> str { return 'hi ' + n } }\n"
        "class P implements Greeter { }\n"
        "def main() { print(P().greet('bob')) }"
    )
    assert out == "hi bob\n"


def test_trait_inheritance_chain():
    out = run(
        "trait A { public def a() -> str }\n"
        "trait B implements A { public def b() -> str }\n"
        "class C implements B {\n"
        "  public def a() -> str { return 'a' }\n"
        "  public def b() -> str { return 'b' }\n}\n"
        "def main() { let c = C()\n print(c.a() + c.b()) }"
    )
    assert out == "ab\n"


# ============================================================================
# OOP: dunder protocols
# ============================================================================

def test_dunder_str():
    assert run(
        "class P { public let x: int = 5\n"
        "  public def str() -> str { return 'P' } }\n"
        "def main() { print(str(P())) }"
    ) == "P\n"


def test_dunder_len():
    assert run(
        "class Bag { public let n: int = 4\n public def len() -> int { return self.n } }\n"
        "def main() { print(len(Bag())) }"
    ) == "4\n"


def test_dunder_eq_and_lt():
    assert run(
        "class V {\n  public let x: int = 0\n  public def new(x: int) { self.x = x }\n"
        "  public def eq(o) -> bool { return self.x == o.x }\n"
        "  public def lt(o) -> bool { return self.x < o.x }\n}\n"
        "def main() { print(V(1) == V(1))\n print(V(1) < V(2)) }"
    ) == "True\nTrue\n"


def test_dunder_bool():
    assert run(
        "class F { public def bool() -> bool { return false } }\n"
        "def main() { if F() { print('t') } else { print('f') } }"
    ) == "f\n"


def test_dunder_iter():
    assert run(
        "class Count {\n  public let n: int = 3\n"
        "  public def iter() {\n    let mut i = 0\n"
        "    while i < self.n { yield i\n i += 1 }\n  }\n}\n"
        "def main() { for x in Count() { print(x) } }"
    ) == "0\n1\n2\n"


def test_dunder_getitem():
    assert run(
        "class Seq {\n  public let items: list = [10, 20, 30]\n"
        "  public def getitem(i: int) -> int { return self.items[i] }\n}\n"
        "def main() { print(Seq()[1]) }"
    ) == "20\n"


def test_dunder_call():
    assert run(
        "class Adder {\n  public def call(x: int) -> int { return x + 1 }\n}\n"
        "def main() { print(Adder()(41)) }"
    ) == "42\n"


def test_dunder_add():
    assert run(
        "class N {\n  public let v: int = 0\n  public def new(v: int) { self.v = v }\n"
        "  public def add(o) -> int { return self.v + o.v }\n}\n"
        "def main() { print(N(2) + N(3)) }"
    ) == "5\n"


# ============================================================================
# Generics, enums and pattern matching
# ============================================================================

def test_generic_class_roundtrip():
    assert run(
        "class Box[T] {\n  public let item: T = none\n"
        "  public def new(item: T) { self.item = item }\n}\n"
        "def main() { print(Box(42).item) }"
    ) == "42\n"


def test_enum_values():
    assert run(
        "enum Direction { North, South, East, West }\n"
        "def main() { print(Direction.West.value) }"
    ) == "4\n"


def test_match_over_constructor():
    assert run(
        "class Point { public let x: int = 0\n public let y: int = 0 }\n"
        "def main() {\n  let p = Point(1, 2)\n"
        "  match p {\n    case Point(x, y) { print(x + y) }\n    case _ { print('other') }\n  }\n}"
    ) == "3\n"


def test_match_guards():
    assert run(
        "def classify(n) -> str {\n"
        "  match n {\n    case n if n < 0 { return 'neg' }\n"
        "    case 0 { return 'zero' }\n    case _ { return 'pos' }\n  }\n"
        "}\n"
        "def main() { print(classify(-5))\n print(classify(0))\n print(classify(9)) }"
    ) == "neg\nzero\npos\n"


def test_polymorphism_via_base_list():
    out = run(
        "class Shape { public def area() -> float { return 0.0 } }\n"
        "class Sq(Shape) { public let s: float = 2.0\n public def area() -> float { return self.s * self.s } }\n"
        "class Ci(Shape) { public let r: float = 1.0\n public def area() -> float { return 3.0 * self.r * self.r } }\n"
        "def main() {\n  let shapes = [Sq(), Ci()]\n  let mut t = 0.0\n"
        "  for s in shapes { t += s.area() }\n  print(t)\n}"
    )
    assert out == "7.0\n"


def test_nested_class():
    assert run(
        "class Outer { public class Inner { public let v: int = 5 } }\n"
        "def main() { print(Outer.Inner().v) }"
    ) == "5\n"


def test_deeply_nested_closure():
    assert run(
        "def main() {\n"
        "  let make = (n) => (x) => (y) => n + x + y\n"
        "  print(make(1)(2)(3))\n"
        "}"
    ) == "6\n"


def test_method_chaining_returns_self():
    assert run(
        "class Q {\n  public let parts: list = []\n"
        "  public def add(p: str) { self.parts.append(p)\n return self }\n"
        "  public def joined() -> str { return '-'.join(self.parts) }\n}\n"
        "def main() { print(Q().add('a').add('b').add('c').joined()) }"
    ) == "a-b-c\n"
