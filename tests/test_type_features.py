"""Tests for generic constraints and match exhaustiveness.

Two type-checker features:

* ``E110`` — a generic parameter may declare a constraint (`[T: Comparable]`).
  The constraint must name a builtin type or a class/trait declared in the same
  program, otherwise it is a typo that silently erases.
* ``E109`` — a ``match`` over a known finite domain (``bool``, enum) or any
  scalar must handle every value or provide a catch-all. Reported as a warning
  so it never breaks a build.
"""
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.types import TypeChecker  # noqa: E402


def check(source):
    """Return (passed, [diagnostic codes+messages])."""
    program = Parser(Tokenizer(source).tokenize()).parse()
    checker = TypeChecker()
    passed = checker.check_program(program)
    return passed, [(d.code.value, d.severity.value, d.message)
                    for d in checker.diagnostics]


def codes(source):
    return [code for code, _sev, _msg in check(source)[1]]


def warnings(source):
    return [code for code, sev, _msg in check(source)[1] if sev == 'warning']


# ============================================================================
# Generic constraints (E110)
# ============================================================================

class TestGenericConstraints:
    def test_constraint_on_builtin_is_accepted(self):
        _, diags = check(
            'def first[T: int](items: [T]) -> T { return items[0] }\n')
        assert diags == []

    def test_constraint_on_declared_class_is_accepted(self):
        _, diags = check(
            'class Comparable { }\n'
            'def first[T: Comparable](items: [T]) -> T { return items[0] }\n')
        assert diags == []

    def test_constraint_may_reference_later_class(self):
        _, diags = check(
            'def first[T: Later](items: [T]) -> T { return items[0] }\n'
            'class Later { }\n')
        assert diags == []

    def test_unknown_constraint_is_an_error(self):
        passed, diags = check(
            'def first[T: Nope](items: [T]) -> T { return items[0] }\n')
        assert not passed
        assert diags[0][0] == 'E110'
        assert 'Nope' in diags[0][2]

    def test_class_constraint_unknown(self):
        passed, diags = check('class Box[T: Missing] { let item: T }\n')
        assert not passed
        assert diags[0][0] == 'E110'

    def test_trait_constraint_unknown(self):
        passed, diags = check(
            'trait Mapper[T: Missing] { public def map(x: T) -> T }\n')
        assert not passed
        assert diags[0][0] == 'E110'

    def test_union_constraint_accepts_known_parts(self):
        _, diags = check(
            'class A { }\nclass B { }\n'
            'def f[T: A | B](x: T) -> T { return x }\n')
        assert diags == []

    def test_union_constraint_rejects_unknown_part(self):
        passed, diags = check(
            'class A { }\ndef f[T: A | Nope](x: T) -> T { return x }\n')
        assert not passed
        assert diags[0][0] == 'E110'

    def test_unconstrained_generic_still_works(self):
        _, diags = check('def id[T](x: T) -> T { return x }\n')
        assert diags == []

    def test_constraint_transpiles_to_python(self):
        from aura_test_helpers import transpile  # noqa: E402
        code = transpile('def first[T: Comparable](items: [T]) -> T { return items[0] }\n')
        # The constraint is compile-time only; the emitted Python ignores it.
        assert 'def first' in code
        import ast as py_ast
        py_ast.parse(code)


# ============================================================================
# Match exhaustiveness (E109)
# ============================================================================

class TestMatchExhaustiveness:
    def test_int_match_without_fallback_warns(self):
        assert 'E109' in warnings(
            'def f(x: int) -> str {\n'
            '  match x { case 1 { return "a" } }\n'
            '}\n')

    def test_int_match_with_wildcard_is_clean(self):
        assert 'E109' not in warnings(
            'def f(x: int) -> str {\n'
            '  match x { case 1 { return "a" } case _ { return "b" } }\n'
            '}\n')

    def test_int_match_with_binding_fallback_is_clean(self):
        assert 'E109' not in warnings(
            'def f(x: int) -> str {\n'
            '  match x { case 1 { return "a" } case other { return str(other) } }\n'
            '}\n')

    def test_guarded_wildcard_is_not_a_fallback(self):
        assert 'E109' in warnings(
            'def f(x: int) -> str {\n'
            '  match x { case _ if x > 0 { return "p" } }\n'
            '}\n')

    def test_str_match_without_fallback_warns(self):
        assert 'E109' in warnings(
            'def f(s: str) -> str {\n'
            '  match s { case "a" { return "a" } }\n'
            '}\n')

    def test_bool_match_missing_false_warns(self):
        assert 'E109' in warnings(
            'def f(b: bool) -> str {\n'
            '  match b { case true { return "y" } }\n'
            '}\n')

    def test_bool_match_complete(self):
        assert 'E109' not in warnings(
            'def f(b: bool) -> str {\n'
            '  match b { case true { return "y" } case false { return "n" } }\n'
            '}\n')

    def test_enum_match_missing_member_warns(self):
        assert 'E109' in warnings(
            'enum Color { RED, GREEN, BLUE }\n'
            'def f(c: Color) -> str {\n'
            '  match c { case Color.RED { return "r" } }\n'
            '}\n')

    def test_enum_match_complete(self):
        assert 'E109' not in warnings(
            'enum Color { RED, GREEN }\n'
            'def f(c: Color) -> str {\n'
            '  match c { case Color.RED { return "r" } case Color.GREEN { return "g" } }\n'
            '}\n')

    def test_enum_match_with_fallback(self):
        assert 'E109' not in warnings(
            'enum Color { RED, GREEN, BLUE }\n'
            'def f(c: Color) -> str {\n'
            '  match c { case Color.RED { return "r" } case _ { return "x" } }\n'
            '}\n')

    def test_unknown_subject_is_not_flagged(self):
        # Gradually typed: an unknown subject must never produce a warning.
        assert 'E109' not in warnings(
            'def f(x) -> str {\n'
            '  match x { case 1 { return "a" } }\n'
            '}\n')

    def test_exhaustiveness_is_only_a_warning(self):
        passed, _ = check(
            'def f(x: int) -> str {\n'
            '  match x { case 1 { return "a" } }\n'
            '}\n')
        assert passed


# ============================================================================
# Enum member patterns (parser + runtime)
# ============================================================================

class TestEnumMemberPatterns:
    def test_enum_member_match_runs(self):
        from aura_test_helpers import run_aura  # noqa: E402
        out = run_aura(
            'enum Color { RED, GREEN }\n'
            'def name(c: Color) -> str {\n'
            '  match c { case Color.RED { return "red" } case Color.GREEN { return "green" } }\n'
            '}\n'
            'def main() { print(name(Color.RED)) print(name(Color.GREEN)) }\n')
        assert out == 'red\ngreen\n'

    def test_enum_member_pattern_does_not_bind_variable(self):
        from aura_test_helpers import transpile  # noqa: E402
        code = transpile(
            'enum Color { RED }\n'
            'def f(c: Color) {\n  match c { case Color.RED { print(1) } }\n}\n')
        assert 'case Color.RED:' in code

    def test_bare_identifier_case_is_a_binding(self):
        from aura_test_helpers import run_aura  # noqa: E402
        out = run_aura(
            'def main() {\n'
            '  match 5 { case n if n > 3 { print("big") } }\n'
            '}\n')
        assert out == 'big\n'

    def test_case_body_is_not_swallowed(self):
        # Regression: `case RED {` used to parse RED as a struct literal and
        # consume the body, producing invalid Python.
        from aura_test_helpers import run_aura  # noqa: E402
        out = run_aura(
            'def main() {\n'
            '  let b = true\n'
            '  match b { case true { print("yes") } case false { print("no") } }\n'
            '}\n')
        assert out == 'yes\n'
