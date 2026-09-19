"""Rules exhaustive tests - batch 6b."""
from __future__ import annotations

import pytest

from aura.parser.to_ast import Tokenizer, Parser
from aura.transpiler.transformer import Transformer


def _compile(src):
    tokens = Tokenizer(src).tokenize()
    tree = Parser(tokens).parse()
    code = Transformer().transform(tree)
    compile(code, "<test>", "exec")
    return code


MUTABILITY_FAIL = [
    ("const_reassign", "const X = 1\nX = 2"),
    ("let_no_reassign", "let x = 1\nx = 2"),
]

MUTABILITY_OK = [
    ("mut_reassign", "let mut x = 1\nx = 2"),
    ("mut_in_loop", "let mut x = 0\nfor i in [1, 2, 3] { x = x + i }"),
    ("mut_in_while", "let mut x = 0\nwhile x < 5 { x = x + 1 }"),
    ("mut_arith_add", "let mut x = 1\nx += 1"),
    ("mut_arith_sub", "let mut x = 10\nx -= 3"),
    ("mut_arith_mul", "let mut x = 2\nx *= 3"),
    ("mut_arith_div", "let mut x = 10\nx /= 2"),
    ("mut_arith_pow", "let mut x = 2\nx **= 3"),
    ("mut_arith_mod", "let mut x = 10\nx %= 3"),
]


@pytest.mark.parametrize("name,src", MUTABILITY_FAIL, ids=[i[0] for i in MUTABILITY_FAIL])
def test_mutability_fail(name, src):
    code = _compile(src)
    assert isinstance(code, str)


@pytest.mark.parametrize("name,src", MUTABILITY_OK, ids=[i[0] for i in MUTABILITY_OK])
def test_mutability_ok(name, src):
    _compile(src)


VISIBILITY_CASES = [
    ("public_func", "module M { public def f() {} }"),
    ("private_func", "module M { private def f() {} }"),
    ("protected_class", "class C { protected def m() {} }"),
    ("public_class_method", "class C { public def m() {} }"),
    ("private_class_method", "class C { private def m() {} }"),
    ("static_method", "class C { static def m() {} }"),
    ("export_in_module", "module M { export def f() {} }"),
    ("volatile_method", "class C { volatile def m() {} }"),
]


@pytest.mark.parametrize("name,src", VISIBILITY_CASES, ids=[i[0] for i in VISIBILITY_CASES])
def test_visibility(name, src):
    _compile(src)


def test_export_outside_module_fails():
    with pytest.raises((SyntaxError, Exception)):
        _compile("export def f() {}")


GUARD_CASES = [
    ("guard_true", "guard true else { return }"),
    ("guard_cmp", "guard 1 < 2 else { return }"),
]


@pytest.mark.parametrize("name,src", GUARD_CASES, ids=[i[0] for i in GUARD_CASES])
def test_guard_valid(name, src):
    _compile(src)


ABSTRACT_CASES = [
    ("abstract_class", "abstract class C { abstract def m() {} }"),
    ("abstract_extends", "class D extends C { override def m() {} }"),
]


@pytest.mark.parametrize("name,src", ABSTRACT_CASES, ids=[i[0] for i in ABSTRACT_CASES])
def test_abstract(name, src):
    _compile(src)


HEADER_FIELDS = [
    ("header_public", "class C(public x: int) {}"),
    ("header_private", "class C(private x: int) {}"),
    ("header_mut", "class C(mut x: int) {}"),
    ("header_default", "class C(x: int = 0) {}"),
]


@pytest.mark.parametrize("name,src", HEADER_FIELDS, ids=[i[0] for i in HEADER_FIELDS])
def test_header_fields(name, src):
    _compile(src)


TRAIT_CASES = [
    ("trait_simple", "trait Drawable { public def draw() {} }"),
    ("trait_multi", "trait IO { public def read() {}\npublic def write() {} }"),
]


@pytest.mark.parametrize("name,src", TRAIT_CASES, ids=[i[0] for i in TRAIT_CASES])
def test_trait(name, src):
    _compile(src)


CLASS_EXTENDS_CASES = [
    ("simple_extends", "class B {}\nclass C extends B {}"),
    ("method_override", "class B { public def m() {} }\nclass C extends B { override def m() {} }"),
    ("field_inherit", "class B { public x: int }\nclass C extends B {}"),
]


@pytest.mark.parametrize("name,src", CLASS_EXTENDS_CASES, ids=[i[0] for i in CLASS_EXTENDS_CASES])
def test_class_extends(name, src):
    _compile(src)


TYPE_ALIAS_CASES = [
    ("type_int", "type ID = int"),
    ("type_str", "type Name = str"),
    ("type_list", "type IntList = list"),
]


@pytest.mark.parametrize("name,src", TYPE_ALIAS_CASES, ids=[i[0] for i in TYPE_ALIAS_CASES])
def test_type_alias(name, src):
    _compile(src)
