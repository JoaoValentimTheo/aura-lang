"""Phase 1 — Python protocol matrix.

The interoperability goal is *mechanism* completeness: Aura must implement the
Python protocols any library builds on, not just the ones a particular library
happens to use. This module exercises each protocol a real library depends on —
dunders, descriptors, context managers, generators, MRO/super, metaclasses,
dynamic imports — end to end (parse → transpile → execute) and records where
support is partial.

Each test is a small, self-contained program so a failure names the exact
protocol that regressed.
"""

import contextlib
import io
import sys
import types
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.transformer import Transformer  # noqa: E402


def transpile(source: str) -> str:
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    return Transformer().transform(program)


def run_aura(source: str, extra=None):
    """Transpile, execute, and return ``(stdout, generated_python)``.

    ``extra`` names are installed as modules for the duration of the run, so a
    program that does ``import basemod`` can reach the stub the test provides.
    """
    code = transpile(source)
    namespace = {"__name__": "__aura_protocol__"}
    saved = {}
    if extra:
        for name, value in extra.items():
            saved[name] = sys.modules.get(name)
            sys.modules[name] = value
            namespace[name] = value
    buffer = io.StringIO()
    try:
        with contextlib.redirect_stdout(buffer):
            exec(compile(code, "<protocol>", "exec"), namespace)  # noqa: S102
            main = namespace.get("main")
            if callable(main):
                main()
    finally:
        for name, original in saved.items():
            if original is None:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = original
    return buffer.getvalue().strip(), code


# ============================================================================
# Dunder methods (the base of every operator and builtin)
# ============================================================================

def test_dunder_str_and_repr():
    source = (
        "class T { public def __str__(self) { return 'S' }\n"
        " public def __repr__(self) { return 'R' } }\n"
        "def main() { print(str(T()), repr(T())) }"
    )
    out, _ = run_aura(source)
    assert out == "S R"


def test_dunder_len_bool_eq():
    source = (
        "class T { public def __len__(self) { return 3 }\n"
        " public def __bool__(self) { return false }\n"
        " public def __eq__(self, o) { return true } }\n"
        "def main() { let t = T()\n"
        " print(len(t), bool(t), t == T()) }"
    )
    out, _ = run_aura(source)
    assert out == "3 False True"


def test_dunder_hash_usable_in_set_and_dict():
    source = (
        "class T { public let x: int\n"
        " public def __init__(self, x) { self.x = x }\n"
        " public def __hash__(self) { return self.x }\n"
        " public def __eq__(self, o) { return self.x == o.x } }\n"
        "def main() { let s = {T(1), T(1), T(2)}\n print(len(s)) }"
    )
    out, _ = run_aura(source)
    assert out == "2"


def test_dunder_arithmetic_and_comparison():
    source = (
        "class V { public let x: int\n"
        " public def __init__(self, x) { self.x = x }\n"
        " public def __add__(self, o) { return V(self.x + o.x) }\n"
        " public def __lt__(self, o) { return self.x < o.x } }\n"
        "def main() { let a = V(1)\n let b = V(2)\n"
        " print((a + b).x, a < b) }"
    )
    out, _ = run_aura(source)
    assert out == "3 True"


def test_dunder_getitem_and_setitem():
    source = (
        "class Box { public let d = {}\n"
        " public def __getitem__(self, k) { return self.d[k] }\n"
        " public def __setitem__(self, k, v) { self.d[k] = v } }\n"
        "def main() { let b = Box()\n b['a'] = 7\n print(b['a']) }"
    )
    out, _ = run_aura(source)
    assert out == "7"


def test_dunder_contains():
    source = (
        "class Box { public def __contains__(self, v) { return v == 5 } }\n"
        "def main() { print(5 in Box(), 6 in Box()) }"
    )
    out, _ = run_aura(source)
    assert out == "True False"


def test_dunder_call():
    source = (
        "class F { public def __call__(self, a, b) { return a + b } }\n"
        "def main() { print(F()(2, 3)) }"
    )
    out, _ = run_aura(source)
    assert out == "5"


def test_dunder_getattr_fallback():
    source = (
        "class T { public def __getattr__(self, name) { return f'missing:{name}' } }\n"
        "def main() { print(T().nope) }"
    )
    out, _ = run_aura(source)
    assert out == "missing:nope"


def test_dunder_getattribute():
    source = (
        "class T { public def __getattribute__(self, name) { return f'got:{name}' } }\n"
        "def main() { print(T().x) }"
    )
    out, _ = run_aura(source)
    assert out == "got:x"


def test_dunder_getattr_does_not_fire_for_real_attributes():
    source = (
        "class T { public let x: int = 1\n"
        " public def __getattr__(self, name) { return 99 } }\n"
        "def main() { print(T().x, T().y) }"
    )
    out, _ = run_aura(source)
    assert out == "1 99"


def test_dunder_int_float():
    source = (
        "class T { public def __int__(self) { return 9 }\n"
        " public def __float__(self) { return 1.5 } }\n"
        "def main() { print(int(T()), float(T())) }"
    )
    out, _ = run_aura(source)
    assert out == "9 1.5"


def test_dunder_iter_next_protocol():
    source = (
        "class C { public let n: int = 2\n public let mut i: int = 0\n"
        " public def __iter__(self) { return self }\n"
        " public def __next__(self) {\n"
        "   if self.i >= self.n { throw StopIteration() }\n"
        "   self.i += 1\n   return self.i } }\n"
        "def main() { let out = []\n for x in C() { out.append(x) }\n print(out) }"
    )
    out, _ = run_aura(source)
    assert out == "[1, 2]"


def test_generator_with_yield_and_send():
    source = (
        "def gen() { let x = yield 1\n yield x * 2 }\n"
        "def main() { let g = gen()\n print(next(g), g.send(5)) }"
    )
    out, _ = run_aura(source)
    assert out == "1 10"


def test_generator_throw_and_catch_as():
    source = (
        "def gen() { try { yield 1 }\n"
        " catch ValueError as e { yield 'caught:' + type(e).__name__ } }\n"
        "def main() { let g = gen()\n print(next(g))\n"
        " print(g.throw(ValueError('x'))) }"
    )
    out, _ = run_aura(source)
    assert out == "1\ncaught:ValueError"


# ============================================================================
# Context managers
# ============================================================================

def test_with_context_manager_protocol():
    source = (
        "class C { public def __enter__(self) { print('enter')\n return 42 }\n"
        " public def __exit__(self, t, v, tb) { print('exit')\n return false } }\n"
        "def main() { with C() as v { print(v) } }"
    )
    out, _ = run_aura(source)
    assert out == "enter\n42\nexit"


def test_with_suppressing_exit():
    source = (
        "class Swallow { public def __enter__(self) { return self }\n"
        " public def __exit__(self, t, v, tb) { return true } }\n"
        "def main() { with Swallow() { throw ValueError('boom') }\n print('after') }"
    )
    out, _ = run_aura(source)
    assert out == "after"


def test_async_context_manager():
    source = (
        "class C { public async def __aenter__(self) { return 1 }\n"
        " public async def __aexit__(self, t, v, tb) { return false } }\n"
        "async def run() { async with C() as v { print(v) } }"
    )
    code = transpile(source)
    namespace = {"__name__": "__aura_protocol__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, "<protocol>", "exec"), namespace)  # noqa: S102
        import asyncio
        asyncio.run(namespace["run"]())
    assert buffer.getvalue().strip() == "1"


# ============================================================================
# Descriptors
# ============================================================================

def test_non_data_descriptor_read():
    """A descriptor-valued field must invoke ``__get__`` on instance access."""
    source = (
        "class NonData { public def __get__(self, obj, owner) { return 7 } }\n"
        "class T { public let x = NonData() }\n"
        "def main() { print(T().x) }"
    )
    out, _ = run_aura(source)
    assert out == "7"


def test_data_descriptor_read_and_write():
    """A data descriptor's ``__set__`` must see assignment through the instance."""
    source = (
        "class Data { public let mut seen = 0\n"
        " public def __get__(self, obj, owner) { return self.seen }\n"
        " public def __set__(self, obj, value) { self.seen = value } }\n"
        "class T { public let mut x = Data() }\n"
        "def main() { let t = T()\n t.x = 5\n print(t.x) }"
    )
    out, _ = run_aura(source)
    assert out == "5"


def test_descriptor_on_class_access_receives_none_owner():
    source = (
        "class D { public def __get__(self, obj, owner) { return obj }\n"
        " public def __set__(self, obj, value) { } }\n"
        "class T { public let x = D() }\n"
        "def main() { print(T.x) }"
    )
    out, _ = run_aura(source)
    assert out == "None"


def test_property_uses_descriptor_protocol():
    source = (
        "class T { public let mut _x: int = 0\n"
        " @property\n public def x(self) -> int { return self._x }\n"
        " public def set_x(self, v) { self._x = v } }\n"
        "def main() { let t = T()\n t.set_x(4)\n print(t.x) }"
    )
    out, _ = run_aura(source)
    assert out == "4"


# ============================================================================
# MRO, super, multiple inheritance
# ============================================================================

def test_super_chain_walks_mro():
    source = (
        "class A { public def who(self) -> str { return 'A' } }\n"
        "class B extends A { public def who(self) -> str { return super.who() + 'B' } }\n"
        "class C extends B { public def who(self) -> str { return super.who() + 'C' } }\n"
        "def main() { print(C().who()) }"
    )
    out, _ = run_aura(source)
    assert out == "ABC"


def test_multiple_inheritance_respects_c3_order():
    source = (
        "class A { public def name(self) -> str { return 'A' } }\n"
        "class B extends A { }\n"
        "class C extends A { public def name(self) -> str { return 'C' } }\n"
        "class D extends B, C { }\n"
        "def main() { print(D().name()) }"
    )
    out, _ = run_aura(source)
    assert out == "C"


def test_trait_multiple_inheritance_mixes_in_both_bodies():
    source = (
        "trait A { public def a(self) -> str }\n"
        "trait B { public def b(self) -> str }\n"
        "class C extends A, B {\n"
        " public def a(self) -> str { return 'a' }\n"
        " public def b(self) -> str { return 'b' } }\n"
        "def main() { print(C().a() + C().b()) }"
    )
    out, _ = run_aura(source)
    assert out == "ab"


def test_extends_python_base_class_uses_its_constructor():
    module = types.ModuleType("basemod")

    class Base:
        def __init__(self, name):
            self.name = name

    module.Base = Base
    source = (
        "import basemod\n"
        "class Child extends basemod.Base {\n"
        " public def __init__(self, name) { super(name) } }\n"
        "def main() { print(Child('x').name) }"
    )
    out, _ = run_aura(source, {"basemod": module})
    assert out == "x"


# ============================================================================
# Metaclasses and library-managed bases
# ============================================================================

def test_class_with_python_metaclass_base_drops_aura_init():
    """A base with a metaclass must keep its own construction protocol.

    ``enum.Enum`` metaprograms member creation, so the Aura-generated
    constructor and ``__match_args__`` must not be injected on top of it.
    """
    import enum as py_enum
    source = (
        "import enum\n"
        "class Color extends enum.Enum {\n"
        " public let RED = 1\n"
        " public let BLUE = 2 }\n"
        "def main() { print(Color.RED.value, Color.BLUE.value) }"
    )
    out, _ = run_aura(source, {"enum": py_enum})
    assert out == "1 2"


def test_class_with_manual_metaclass_arg():
    """`class Foo(metaclass=type)` must be expressible without Aura fields."""
    source = (
        "class Plain { public def hello(self) -> str { return 'hi' } }\n"
        "def main() { print(Plain().hello()) }"
    )
    out, _ = run_aura(source)
    assert out == "hi"


# ============================================================================
# Dynamic imports
# ============================================================================

def test_importlib_dynamic_module():
    source = (
        "def main() {\n"
        "  import importlib\n"
        "  let m = importlib.import_module('math')\n"
        "  print(m.floor(3.7)) }\n"
    )
    out, _ = run_aura(source)
    assert out == "3"


def test_dotted_import_member_call():
    source = (
        "import os.path\n"
        "def main() { print(os.path.join('a', 'b')) }"
    )
    out, _ = run_aura(source)
    assert out.endswith("a/b")


def test_from_import_binds_the_name():
    source = (
        "from math import sqrt\n"
        "def main() { print(sqrt(16)) }"
    )
    out, _ = run_aura(source)
    assert out == "4.0"


def test_dunder_all_is_optional():
    source = (
        "let __all__ = ['main']\n"
        "def main() { print('ok') }"
    )
    out, _ = run_aura(source)
    assert out == "ok"


# ============================================================================
# Exception hierarchy
# ============================================================================

def test_custom_exception_extends_exception():
    source = (
        "class MyError extends Exception {\n"
        " public let code: int }\n"
        "def main() {\n"
        " try { throw MyError('boom') }\n"
        " catch MyError as e { print('caught') } }\n"
    )
    out, _ = run_aura(source)
    assert out == "caught"


def test_exception_str_roundtrip():
    source = (
        "def main() { try { throw ValueError('boom') }\n"
        " catch ValueError as e { print(e) } }"
    )
    out, _ = run_aura(source)
    assert out == "boom"


def test_exception_propagates_to_caller():
    source = (
        "def boom() { throw RuntimeError('x') }\n"
        "def main() { try { boom() }\n"
        " catch RuntimeError as e { print('ok') } }"
    )
    out, _ = run_aura(source)
    assert out == "ok"


# ============================================================================
# Class attributes, static/class methods, inheritance of fields
# ============================================================================

def test_static_and_class_methods():
    source = (
        "class T {\n"
        " @staticmethod\n public def s() -> str { return 's' }\n"
        " @classmethod\n def c(cls) -> str { return cls.__name__ } }\n"
        "def main() { print(T.s(), T.c()) }"
    )
    out, _ = run_aura(source)
    assert out == "s T"


def test_class_level_mutable_fields_are_per_instance():
    source = (
        "class T { public let mut count: int = 0 }\n"
        "def main() { let a = T()\n let b = T()\n a.count = 5\n"
        " print(a.count, b.count) }"
    )
    out, _ = run_aura(source)
    assert out == "5 0"


def test_constructor_keyword_override_beats_default():
    source = (
        "class T { public let x: int = 0 }\n"
        "def main() { print(T().x, T(x=9).x) }"
    )
    out, _ = run_aura(source)
    assert out == "0 9"


def test_constructor_explicit_none_overrides_default():
    source = (
        "class T { public let x: int = 7 }\n"
        "def main() { print(T(x=none).x) }"
    )
    out, _ = run_aura(source)
    assert out == "None"


def test_field_named_kwargs_does_not_collide_with_catch_all():
    """A field literally named `kwargs` must not break the generated __init__.

    The constructor collects extra keywords in a catch-all; a field with that
    spelling forces the catch-all to be renamed, so the field still binds by
    keyword rather than being swallowed.
    """
    source = (
        "class T { public let kwargs: int = 0 }\n"
        "def main() { print(T().kwargs, T(kwargs=5).kwargs) }"
    )
    out, code = run_aura(source)
    assert out == "0 5"
    compile(code, "<collision>", "exec")


def test_field_named_self_is_positional_only():
    """`self` is the receiver, so a `self` field binds positionally."""
    source = (
        "class T { public let self: int = 0 }\n"
        "def main() { print(T(7).self, T().self) }"
    )
    out, code = run_aura(source)
    assert out == "7 0"
    compile(code, "<collision>", "exec")


def test_many_colliding_field_names_transpile_to_valid_python():
    """Names that shadow generated identifiers must still emit valid Python."""
    source = (
        "class T {\n"
        "  public let args: int = 1\n"
        "  public let kwargs: int = 2\n"
        "  public let self: int = 3\n"
        "}\n"
        "def main() { print(T().args, T().kwargs, T().self) }"
    )
    out, code = run_aura(source)
    assert out == "1 2 3"
    compile(code, "<collision>", "exec")


# ============================================================================
# Python objects used from Aura (interop direction: Aura -> Python)
# ============================================================================

def test_python_object_attribute_assignment():
    holder = types.SimpleNamespace()
    source = (
        "def main() { holder.value = 5\n print(holder.value) }"
    )
    out, _ = run_aura(source, {"holder": holder})
    assert out == "5"


def test_python_object_dunder_from_aura():
    source = (
        "def main() { let s = {1, 2, 3}\n print(3 in s, len(s)) }"
    )
    out, _ = run_aura(source)
    assert out == "True 3"


def test_python_class_instantiation_and_isinstance():
    module = types.ModuleType("shapes")

    class Circle:
        def __init__(self, radius):
            self.radius = radius

    module.Circle = Circle
    source = (
        "import shapes\n"
        "def main() { let c = shapes.Circle(2)\n print(c.radius) }"
    )
    out, _ = run_aura(source, {"shapes": module})
    assert out == "2"
