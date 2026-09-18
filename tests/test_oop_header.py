"""Tests for the class header and `extends`-only inheritance.

Aura's class header declares fields inline:

    class User(private name: str, mut age: int = 0, public id: int)

Each field becomes an instance field. A getter is always generated; a setter
only when the field is mutable (`mut`/`let mut`). Fields are `private` by
default. Inheritance uses `extends` exclusively — the parenthesised `class
Foo(Bar)` form and `implements` are not Aura.
"""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.rules import RuleChecker  # noqa: E402
from aura.transpiler.types import TypeChecker  # noqa: E402
from aura_test_helpers import parse, run_aura, transpile  # noqa: E402


def rule_codes(source):
    program = Parser(Tokenizer(source).tokenize()).parse()
    checker = RuleChecker()
    checker.check_program(program)
    return [e.code.value for e in checker.collector.errors]


def type_codes(source):
    program = Parser(Tokenizer(source).tokenize()).parse()
    checker = TypeChecker()
    checker.check_program(program)
    return [d.code.value for d in checker.diagnostics]


# ============================================================================
# Grammar: header fields
# ============================================================================

class TestHeaderGrammar:
    def test_basic_header_parses(self):
        program = parse('class User(name: str, password: str) {\n}\n')
        decl = program.statements[0]
        assert [f[0].name for f in decl.header_fields] == ['name', 'password']

    def test_default_visibility_is_private(self):
        decl = parse('class User(name: str) {\n}\n').statements[0]
        assert decl.header_fields[0][1] == 'private'

    def test_explicit_visibility_per_field(self):
        decl = parse(
            'class U(private a: int, protected b: int, public c: int) {\n}\n'
        ).statements[0]
        assert [f[1] for f in decl.header_fields] == ['private', 'protected', 'public']

    def test_mut_is_recorded(self):
        decl = parse('class U(a: int, mut b: int) {\n}\n').statements[0]
        assert [f[2] for f in decl.header_fields] == [False, True]

    def test_let_mut_is_accepted(self):
        decl = parse('class U(let mut a: int) {\n}\n').statements[0]
        assert decl.header_fields[0][2] is True

    def test_default_value_is_stored(self):
        decl = parse('class U(a: int = 7) {\n}\n').statements[0]
        assert decl.header_fields[0][0].default is not None

    def test_empty_header_is_allowed(self):
        decl = parse('class U() {\n}\n').statements[0]
        assert decl.header_fields == []

    def test_type_annotation_is_stored(self):
        decl = parse('class U(a: str) {\n}\n').statements[0]
        assert decl.header_fields[0][0].type_annotation == 'str'

    def test_duplicate_header_field_is_rejected(self):
        with pytest.raises(SyntaxError, match='duplicate header field'):
            parse('class U(a: int, a: int) {\n}\n')

    def test_required_after_optional_is_rejected(self):
        with pytest.raises(SyntaxError, match='cannot follow a field with a default'):
            parse('class U(a: int = 0, b: int) {\n}\n')

    def test_bare_name_without_type_is_rejected(self):
        # This keeps `class C(A)` from looking like a parenthesised base list.
        with pytest.raises(SyntaxError, match='needs a type annotation or a default'):
            parse('class U(a) {\n}\n')

    def test_const_in_header_is_rejected(self):
        with pytest.raises(SyntaxError, match='constant fields are declared'):
            parse('class U(const K = 1) {\n}\n')

    def test_header_precedes_extends_error(self):
        # Header comes after `extends`; a header before it is a syntax error.
        with pytest.raises(SyntaxError):
            parse('class A(name: str) extends B {\n}\n')


# ============================================================================
# Grammar: extends-only inheritance
# ============================================================================

class TestExtendsOnly:
    def test_extends_single(self):
        decl = parse('class B extends A {\n}\n').statements[0]
        assert decl.base_class == 'A'

    def test_extends_multiple(self):
        decl = parse('class C extends A, B {\n}\n').statements[0]
        assert decl.base_class == 'A, B'

    def test_extends_dotted(self):
        decl = parse('class C extends pkg.Base {\n}\n').statements[0]
        assert decl.base_class == 'pkg.Base'

    def test_parenthesised_base_is_a_syntax_error(self):
        with pytest.raises(SyntaxError):
            parse('class B(A) {\n}\n')

    def test_implements_is_a_syntax_error(self):
        with pytest.raises(SyntaxError, match='implements'):
            parse('class B implements A {\n}\n')

    def test_implements_error_suggests_extends(self):
        with pytest.raises(SyntaxError, match='extends'):
            parse('class B implements A {\n}\n')

    def test_trait_extends_trait(self):
        decl = parse('trait Loud extends Greeter {\n}\n').statements[0]
        assert decl.base_class == 'Greeter'

    def test_trait_parenthesised_base_is_rejected(self):
        with pytest.raises(SyntaxError):
            parse('trait Loud(Greeter) {\n}\n')

    def test_trait_implements_is_rejected(self):
        with pytest.raises(SyntaxError, match='implements'):
            parse('trait Loud implements Greeter {\n}\n')

    def test_extends_plus_header(self):
        decl = parse('class Admin extends User(email: str) {\n}\n').statements[0]
        assert decl.base_class == 'User'
        assert [f[0].name for f in decl.header_fields] == ['email']


# ============================================================================
# Code generation
# ============================================================================

class TestHeaderCodegen:
    def test_header_becomes_constructor_params(self):
        code = transpile('class U(name: str, age: int = 0) {\n}\n')
        assert 'def __init__(self, name=_aura_unset, age=_aura_unset, **kwargs)' in code

    def test_default_is_applied_when_omitted(self):
        code = transpile('class U(age: int = 0) {\n}\n')
        assert '0 if age is _aura_unset else age' in code

    def test_immutable_field_has_getter_only(self):
        code = transpile('class U(name: str) {\n}\n')
        assert 'def get_name' in code
        assert 'def set_name' not in code

    def test_mut_field_has_getter_and_setter(self):
        code = transpile('class U(mut age: int = 0) {\n}\n')
        assert 'def get_age' in code
        assert 'def set_age' in code

    def test_private_field_storage_is_mangled(self):
        code = transpile('class U(name: str) {\n}\n')
        assert 'self._U__name' in code

    def test_public_field_storage_is_not_mangled(self):
        code = transpile('class U(public id: int) {\n}\n')
        assert 'self.id' in code
        assert 'def get_id' in code

    def test_protected_field_is_mangled(self):
        code = transpile('class U(protected age: int = 0) {\n}\n')
        assert 'self._age' in code

    def test_match_args_use_runtime_names(self):
        code = transpile('class U(private a: int, public b: int) {\n}\n')
        assert "__match_args__ = ('_U__a', 'b')" in code

    def test_manual_new_wins(self):
        code = transpile(
            'class U(name: str) {\n'
            '  public def new(name: str) { self.name = name }\n'
            '}\n')
        # Exactly one __init__, the manual one.
        assert code.count('def __init__') == 1
        assert 'def __init__(self, name):' in code

    def test_manual_new_still_gets_accessors(self):
        code = transpile(
            'class U(name: str) {\n'
            '  public def new(name: str) { self.name = name }\n'
            '}\n')
        assert 'def get_name' in code

    def test_manual_new_emits_field_defaults(self):
        # With a manual ctor, header fields also exist as class-level defaults
        # so the generated accessors always resolve.
        code = transpile(
            'class U(name: str) {\n'
            '  public def new(name: str) { self.name = name }\n'
            '}\n')
        assert '_U__name = None' in code

    def test_custom_accessor_wins(self):
        code = transpile(
            'class U(mut name: str) {\n'
            '  public def set_name(value: str) { }\n'
            '}\n')
        assert code.count('def set_name') == 1

    def test_extends_forwards_kwargs_to_super(self):
        code = transpile('class A extends B(email: str) {\n}\n')
        assert 'super().__init__(**kwargs)' in code

    def test_generated_python_is_valid(self):
        import ast as py_ast
        code = transpile(
            'class U(private name: str, mut age: int = 0, public id: int = 1) {\n}\n')
        py_ast.parse(code)


# ============================================================================
# Runtime behaviour
# ============================================================================

class TestHeaderRuntime:
    def test_constructor_and_getters(self):
        out = run_aura(
            'class User(name: str, mut age: int = 0) {\n}\n'
            'def main() {\n'
            '  let u = User("ana")\n'
            '  print(u.get_name())\n'
            '  print(u.get_age())\n'
            '}\n')
        assert out == 'ana\n0\n'

    def test_setter_updates_value(self):
        out = run_aura(
            'class User(mut age: int = 0) {\n}\n'
            'def main() {\n'
            '  let u = User()\n'
            '  u.set_age(30)\n'
            '  print(u.get_age())\n'
            '}\n')
        assert out == '30\n'

    def test_public_field_is_directly_accessible(self):
        out = run_aura(
            'class U(public id: int = 5) {\n}\n'
            'def main() { print(U().id) }\n')
        assert out == '5\n'

    def test_explicit_none_overrides_default(self):
        out = run_aura(
            'class U(name: str? = "x") {\n}\n'
            'def main() { print(U(none).get_name()) }\n')
        assert out == 'None\n'

    def test_extends_inherits_accessors(self):
        out = run_aura(
            'class User(name: str) {\n}\n'
            'class Admin extends User(email: str) {\n}\n'
            'def main() {\n'
            '  let a = Admin(email: "x", name: "bob")\n'
            '  print(a.get_name())\n'
            '  print(a.get_email())\n'
            '}\n')
        assert out == 'bob\nx\n'

    def test_inherited_setter_works_from_child(self):
        out = run_aura(
            'class User(mut age: int = 0) {\n}\n'
            'class Admin extends User(email: str) {\n}\n'
            'def main() {\n'
            '  let a = Admin(email: "x")\n'
            '  a.set_age(42)\n'
            '  print(a.get_age())\n'
            '}\n')
        assert out == '42\n'

    def test_method_uses_header_field(self):
        out = run_aura(
            'class User(name: str) {\n'
            '  public def greet() -> str { return "hi " + self.get_name() }\n'
            '}\n'
            'def main() { print(User("ana").greet()) }\n')
        assert out == 'hi ana\n'

    def test_setter_runs_through_custom_logic(self):
        out = run_aura(
            'class Counter(mut n: int = 0) {\n'
            '  public def set_n(value: int) { self.n = value * 2 }\n'
            '}\n'
            'def main() {\n'
            '  let c = Counter()\n'
            '  c.set_n(5)\n'
            '  print(c.get_n())\n'
            '}\n')
        assert out == '10\n'


# ============================================================================
# Diagnostics
# ============================================================================

class TestHeaderDiagnostics:
    def test_header_and_body_field_collide(self):
        assert 'E301' in rule_codes(
            'class C(name: str) {\n  public let name: str = ""\n}\n')

    def test_const_member_requires_visibility(self):
        assert 'E307' in rule_codes('class C {\n  const K = 1\n}\n')

    def test_const_with_visibility_is_clean(self):
        assert 'E307' not in rule_codes('class C {\n  public const K = 1\n}\n')

    def test_super_private_access_is_rejected(self):
        assert 'E308' in rule_codes(
            'class Base {\n  private let s: int = 1\n}\n'
            'class Child extends Base {\n'
            '  public def peek() -> int { return super.s }\n}\n')

    def test_super_protected_access_is_allowed(self):
        assert 'E308' not in rule_codes(
            'class Base {\n  protected let s: int = 1\n}\n'
            'class Child extends Base {\n'
            '  public def peek() -> int { return super.s }\n}\n')

    def test_inherited_method_arity_is_checked(self):
        assert 'E105' in type_codes(
            'class A {\n  public def f(x: int) -> int { return x }\n}\n'
            'class B extends A { }\n'
            'def g(b: B) { return b.f(1, 2, 3) }\n')

    def test_inherited_method_arity_ok(self):
        assert 'E105' not in type_codes(
            'class A {\n  public def f(x: int) -> int { return x }\n}\n'
            'class B extends A { }\n'
            'def g(b: B) { return b.f(1) }\n')

    def test_missing_visibility_on_body_field(self):
        assert 'E307' in rule_codes('class C {\n  let x: int = 0\n}\n')


# ============================================================================
# Regression: bugs found during the OOP audit
# ============================================================================

class TestOopAuditRegressions:
    def test_multiple_bases_inherit_visibility(self):
        # The visibility map used to be keyed on the whole comma-joined base
        # string, so multiple inheritance silently dropped inherited members.
        code = transpile(
            'class A { protected let x: int = 1 }\n'
            'class B { public def m() -> int { return 0 } }\n'
            'class C extends A, B { public def read() -> int { return self.x } }\n')
        assert 'self._x' in code

    def test_body_let_field_is_immutable(self):
        code = transpile('class C {\n  private let x: int = 0\n}\n')
        assert 'def get_x' in code
        assert 'def set_x' not in code

    def test_body_let_mut_field_has_setter(self):
        code = transpile('class C {\n  private let mut x: int = 0\n}\n')
        assert 'def set_x' in code

    def test_trait_field_let_is_immutable(self):
        decl = parse('trait T {\n  public let x: int = 0\n}\n').statements[0]
        assert decl.members[0].mutable is False

    def test_trait_field_mut_is_mutable(self):
        decl = parse('trait T {\n  public let mut x: int = 0\n}\n').statements[0]
        assert decl.members[0].mutable is True

    def test_class_decl_has_no_dead_static_flags(self):
        decl = parse('class C {\n}\n').statements[0]
        assert not hasattr(decl, 'is_static')
        assert not hasattr(decl, 'is_volatile')