"""Tests for Aura's structural rule enforcement (E314-E321).

These rules catch invalid programs at check time that would otherwise crash at
runtime with a confusing Python error (a bare `NameError`, `TypeError` or
`AttributeError`). Each test asserts both that the bad program is rejected with
the right code and that the correct program is accepted.
"""
import subprocess
import sys
import textwrap
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.rules import RuleChecker  # noqa: E402
from aura.transpiler.semantics import MutabilityChecker  # noqa: E402
from aura.transpiler.types import TypeChecker  # noqa: E402


def codes(source):
    """All diagnostic codes a program produces (rules, semantics, types)."""
    program = Parser(Tokenizer(textwrap.dedent(source).lstrip()).tokenize()).parse()
    found = []

    rules = RuleChecker()
    rules.check_program(program)
    found += [e.code.value for e in rules.collector.errors]

    semantics = MutabilityChecker()
    semantics.check_program(program)
    found += [d.code.value for d in semantics.diagnostics]

    types = TypeChecker()
    types.check_program(program)
    found += [d.code.value for d in types.diagnostics]
    return found


def run_aura_src(source, tmp_path):
    path = tmp_path / 'main.aura'
    path.write_text(textwrap.dedent(source).lstrip(), encoding='utf-8')
    return subprocess.run(
        [sys.executable, '-m', 'aura.cli', 'run', str(path)],
        capture_output=True, text=True, timeout=60, cwd=str(tmp_path))


# ============================================================================
# E314 — unknown base class
# ============================================================================

class TestUnknownBase:
    def test_unknown_base_is_rejected(self):
        assert 'E314' in codes('''
            class C extends Nope { }
            def main() { C() }
        ''')

    def test_known_base_is_accepted(self):
        assert 'E314' not in codes('''
            class A { }
            class C extends A { }
            def main() { C() }
        ''')

    def test_forward_reference_base_is_accepted(self):
        # The base is declared later in the file.
        assert 'E314' not in codes('''
            class C extends A { }
            class A { }
            def main() { C() }
        ''')

    def test_builtin_error_base_is_accepted(self):
        assert 'E314' not in codes('''
            class MyError extends Error { }
            def main() { MyError() }
        ''')

    def test_multiple_builtin_exception_bases(self):
        for base in ('ValueError', 'TypeError', 'RuntimeError', 'Exception'):
            assert 'E314' not in codes(
                f'class E extends {base} {{ }}\ndef main() {{ E() }}\n'), base

    def test_dotted_base_is_accepted(self):
        # A dotted base is resolved through its namespace at runtime.
        assert 'E314' not in codes('''
            class C extends App.Base { }
            def main() { C() }
        ''')

    def test_trait_base_is_accepted(self):
        assert 'E314' not in codes('''
            trait T { public def m() -> int }
            class C extends T { public def m() -> int { return 1 } }
            def main() { C() }
        ''')


# ============================================================================
# E315 — invalid inheritance
# ============================================================================

class TestInvalidInheritance:
    def test_duplicate_base_is_rejected(self):
        assert 'E315' in codes('''
            class A { }
            class C extends A, A { }
            def main() { C() }
        ''')

    def test_circular_inheritance_is_rejected(self):
        assert 'E315' in codes('''
            class A extends B { }
            class B extends A { }
            def main() { A() }
        ''')

    def test_three_class_cycle_is_rejected(self):
        assert 'E315' in codes('''
            class A extends B { }
            class B extends C { }
            class C extends A { }
            def main() { A() }
        ''')

    def test_self_inheritance_is_rejected(self):
        assert 'E315' in codes('''
            class A extends A { }
            def main() { A() }
        ''')

    def test_distinct_bases_are_accepted(self):
        assert 'E315' not in codes('''
            class A { }
            class B { }
            class C extends A, B { }
            def main() { C() }
        ''')

    def test_diamond_is_accepted(self):
        # A diamond is valid; only a *cycle* is rejected.
        assert 'E315' not in codes('''
            class Base { }
            class A extends Base { }
            class B extends Base { }
            class D extends A, B { }
            def main() { D() }
        ''')

    def test_cycle_message_names_the_chain(self):
        program = Parser(Tokenizer(
            'class A extends B { }\nclass B extends A { }\n').tokenize()).parse()
        checker = RuleChecker()
        checker.check_program(program)
        messages = [str(e) for e in checker.collector.errors]
        assert any('A' in m and 'B' in m for m in messages)


# ============================================================================
# E316 — instantiating an abstract type
# ============================================================================

class TestInstantiateAbstract:
    def test_trait_instantiation_is_rejected(self):
        assert 'E316' in codes('''
            trait T { public def m() -> int }
            def main() { T() }
        ''')

    def test_abstract_class_instantiation_is_rejected(self):
        assert 'E316' in codes('''
            trait T { public def m() -> int }
            class C extends T { }
            def main() { C() }
        ''')

    def test_abstract_message_names_the_method(self):
        program = Parser(Tokenizer(
            'trait T { public def missing_method() -> int }\n'
            'class C extends T { }\n'
            'def main() { C() }\n').tokenize()).parse()
        checker = RuleChecker()
        checker.check_program(program)
        assert any('missing_method' in str(e) for e in checker.collector.errors)

    def test_implemented_class_is_accepted(self):
        assert 'E316' not in codes('''
            trait T { public def m() -> int }
            class C extends T { public def m() -> int { return 1 } }
            def main() { print(C().m()) }
        ''')

    def test_abstract_inherited_through_two_levels(self):
        assert 'E316' in codes('''
            trait T { public def m() -> int }
            class A extends T { }
            class B extends A { }
            def main() { B() }
        ''')

    def test_implementation_at_the_middle_level_is_enough(self):
        assert 'E316' not in codes('''
            trait T { public def m() -> int }
            class A extends T { public def m() -> int { return 1 } }
            class B extends A { }
            def main() { print(B().m()) }
        ''')

    def test_runtime_agrees(self, tmp_path):
        result = run_aura_src('''
            trait T { public def m() -> int }
            def main() { T() }
        ''', tmp_path)
        assert result.returncode != 0


# ============================================================================
# E317 — self/cls in a static method
# ============================================================================

class TestSelfInStatic:
    def test_self_in_static_method_is_rejected(self):
        assert 'E317' in codes('''
            class C {
              public static def f() -> int { return self.x }
            }
            def main() { C.f() }
        ''')

    def test_cls_in_static_method_is_rejected(self):
        assert 'E317' in codes('''
            class C {
              public static def f() -> int { return cls.x }
            }
            def main() { C.f() }
        ''')

    def test_self_in_instance_method_is_accepted(self):
        assert 'E317' not in codes('''
            class C {
              public let x: int = 1
              public def f() -> int { return self.x }
            }
            def main() { print(C().f()) }
        ''')

    def test_cls_in_classmethod_is_accepted(self):
        assert 'E317' not in codes('''
            class C {
              public let x: int = 1
              public @classmethod
              def make() -> int { return cls.x }
            }
            def main() { print(C.make()) }
        ''')

    def test_static_method_without_self_is_accepted(self):
        assert 'E317' not in codes('''
            class C {
              public static def f(x: int) -> int { return x + 1 }
            }
            def main() { print(C.f(1)) }
        ''')


# ============================================================================
# E318 — unknown label
# ============================================================================

class TestUnknownLabel:
    def test_unknown_break_label_is_rejected(self):
        assert 'E318' in codes('''
            def main() {
              for i in range(3) { break nope }
            }
        ''')

    def test_unknown_continue_label_is_rejected(self):
        assert 'E318' in codes('''
            def main() {
              for i in range(3) { continue nope }
            }
        ''')

    def test_valid_break_label_is_accepted(self):
        assert 'E318' not in codes('''
            def main() {
              outer: for i in range(3) {
                for j in range(3) { break outer }
              }
            }
        ''')

    def test_valid_continue_label_is_accepted(self):
        assert 'E318' not in codes('''
            def main() {
              outer: for i in range(3) {
                for j in range(3) { continue outer }
              }
            }
        ''')

    def test_inner_label_not_visible_from_outer_loop(self):
        assert 'E318' in codes('''
            def main() {
              for i in range(3) {
                inner: for j in range(3) { }
                break inner
              }
            }
        ''')

    def test_unlabeled_break_is_accepted(self):
        assert 'E318' not in codes('''
            def main() {
              for i in range(3) { break }
            }
        ''')


# ============================================================================
# E319 — use before declaration
# ============================================================================

class TestUseBeforeDeclaration:
    def test_local_used_before_let_is_rejected(self):
        assert 'E319' in codes('''
            def main() {
              print(x)
              let x = 1
            }
        ''')

    def test_const_used_before_declaration_is_rejected(self):
        assert 'E319' in codes('''
            def main() {
              print(K)
              const K = 1
            }
        ''')

    def test_declared_before_use_is_accepted(self):
        assert 'E319' not in codes('''
            def main() {
              let x = 1
              print(x)
            }
        ''')

    def test_module_level_name_is_accepted(self):
        assert 'E319' not in codes('''
            let total = 5
            def main() {
              print(total)
              let local = 1
              print(local)
            }
        ''')

    def test_parameter_is_accepted(self):
        assert 'E319' not in codes('''
            def f(x: int) -> int {
              print(x)
              let y = 1
              return x + y
            }
            def main() { print(f(1)) }
        ''')

    def test_for_target_read_elsewhere_is_accepted(self):
        # A `for` target is scoped to its loop (and a comprehension to the
        # comprehension), so a read elsewhere is not a use-before-declaration.
        assert 'E319' not in codes('''
            def main() {
              let primes = [n for n in range(2, 10)]
              for n in primes { print(n) }
            }
        ''')

    def test_declaration_inside_a_block_is_not_leaked(self):
        assert 'E319' not in codes('''
            def main() {
              if true {
                let inner = 1
                print(inner)
              }
            }
        ''')

    def test_runtime_error_is_now_a_diagnostic(self, tmp_path):
        result = run_aura_src('''
            def main() { print(x)
              let x = 1
            }
        ''', tmp_path)
        assert result.returncode != 0
        assert 'E319' in result.stdout + result.stderr


# ============================================================================
# E320 — decorator on a field
# ============================================================================

class TestDecoratorOnField:
    @pytest.mark.parametrize('decorator', ['property', 'staticmethod', 'classmethod'])
    def test_decorator_on_class_field_is_rejected(self, decorator):
        with pytest.raises(SyntaxError):
            Parser(Tokenizer(
                f'class C {{\n  public @{decorator}\n  let x: int = 0\n}}\n'
            ).tokenize()).parse()

    def test_decorator_on_trait_field_is_rejected(self):
        with pytest.raises(SyntaxError):
            Parser(Tokenizer(
                'trait T {\n  public @property\n  let x: int = 0\n}\n'
            ).tokenize()).parse()

    def test_decorator_on_method_is_accepted(self):
        assert 'E320' not in codes('''
            class C {
              private let w: int = 4
              public @property
              def area() -> int { return self.w * 2 }
            }
            def main() { print(C().area) }
        ''')

    def test_decorator_on_header_field_is_rejected(self):
        with pytest.raises(SyntaxError):
            Parser(Tokenizer(
                'class C(@property x: int) { }\n').tokenize()).parse()


# ============================================================================
# E321 — super call to an abstract method
# ============================================================================

class TestAbstractSuperCall:
    def test_super_to_abstract_is_rejected(self):
        assert 'E321' in codes('''
            trait T { public def m() -> int }
            class C extends T {
              public def m() -> int { return super.m() }
            }
            def main() { print(C().m()) }
        ''')

    def test_super_to_concrete_is_accepted(self):
        assert 'E321' not in codes('''
            class A { public def m() -> int { return 5 } }
            class C extends A {
              public def m() -> int { return super.m() + 1 }
            }
            def main() { print(C().m()) }
        ''')

    def test_super_init_on_a_concrete_base_is_accepted(self):
        assert 'E321' not in codes('''
            class A {
              public let x: int = 0
              public def new(x: int) { self.x = x }
            }
            class C extends A {
              public def new() { super(1) }
            }
            def main() { print(C().get_x()) }
        ''')


# ============================================================================
# No false positives on the existing corpora
# ============================================================================

class TestNoFalsePositives:
    def test_all_examples_pass_the_new_rules(self):
        from aura.parser.to_ast import parse_file
        examples = sorted((ROOT / 'examples').rglob('*.aura'))
        assert examples, 'no examples found'
        for path in examples:
            program = parse_file(str(path))
            checker = RuleChecker()
            assert checker.check_program(program), (
                f"{path.name}: "
                + "; ".join(str(e) for e in checker.collector.errors))

    def test_all_examples_pass_semantics(self):
        from aura.parser.to_ast import parse_file
        for path in sorted((ROOT / 'examples').rglob('*.aura')):
            program = parse_file(str(path))
            checker = MutabilityChecker()
            assert checker.check_program(program), (
                f"{path.name}: " + "; ".join(str(e) for e in checker.errors))
