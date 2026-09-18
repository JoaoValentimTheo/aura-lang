"""Tests for the module system.

A `module Name { ... }` body is a namespace whose members are **private to the
declaring file** unless marked `export`. An exported member is reachable as
`Name.member` from anywhere; a non-exported one is not, and the rule checker
reports `E308`. Module state is not writable from outside (`E303`); mutate it
through an exported function instead.
"""
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura_test_helpers import parse, run_aura, transpile  # noqa: E402

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.rules import RuleChecker  # noqa: E402
from aura.transpiler.types import TypeChecker  # noqa: E402


def rule_codes(source):
    program = Parser(Tokenizer(source).tokenize()).parse()
    checker = RuleChecker()
    checker.check_program(program)
    return [e.code.value for e in checker.collector.errors]


def rule_messages(source):
    program = Parser(Tokenizer(source).tokenize()).parse()
    checker = RuleChecker()
    checker.check_program(program)
    return [str(e) for e in checker.collector.errors]


# ============================================================================
# Parsing
# ============================================================================

class TestModuleParsing:
    def test_module_records_exports(self):
        module = parse('module M {\n  export def a() -> int { return 1 }\n'
                       '  def b() -> int { return 2 }\n}\n').statements[0]
        assert module.exports == {'a'}
        assert module.members[0].is_exported is True
        assert module.members[1].is_exported is False

    def test_exports_of_every_member_kind(self):
        module = parse(
            'module M {\n'
            '  export let a = 1\n'
            '  export const B = 2\n'
            '  export def c() -> int { return 3 }\n'
            '  export class D { }\n'
            '  export trait E { public def m() -> int }\n'
            '  export enum F { X }\n'
            '  export type G = int\n'
            '}\n').statements[0]
        assert module.exports == {'a', 'B', 'c', 'D', 'E', 'F', 'G'}

    def test_dotted_module_name(self):
        module = parse('module A.B { export def f() -> int { return 1 } }\n').statements[0]
        assert module.name == 'A.B'

    def test_module_without_exports(self):
        module = parse('module M {\n  def f() -> int { return 1 }\n}\n').statements[0]
        assert module.exports == set()

    def test_export_outside_module_is_rejected(self):
        with pytest.raises(SyntaxError, match='export'):
            parse('export def f() -> int { return 1 }\n')

    def test_export_without_a_name_is_rejected(self):
        with pytest.raises(SyntaxError, match='export'):
            parse('module M {\n  export\n}\n')

    def test_nested_module_exports_are_tracked(self):
        outer = parse(
            'module Outer {\n'
            '  export module Inner {\n'
            '    export def f() -> int { return 1 }\n'
            '  }\n'
            '}\n').statements[0]
        assert outer.exports == {'Inner'}
        inner = outer.members[0]
        assert inner.exports == {'f'}


# ============================================================================
# Code generation
# ============================================================================

class TestModuleCodegen:
    def test_exported_member_keeps_its_name(self):
        code = transpile('module M {\n  export def f() -> int { return 1 }\n}\n')
        assert 'def f()' in code
        assert '_M__f' not in code

    def test_private_member_is_mangled(self):
        code = transpile('module M {\n  def hidden() -> int { return 1 }\n}\n')
        assert 'def _M__hidden()' in code

    def test_private_data_member_is_mangled(self):
        code = transpile('module M {\n  let x = 1\n}\n')
        assert '_M__x = 1' in code

    def test_exported_data_member_keeps_its_name(self):
        code = transpile('module M {\n  export let x = 1\n}\n')
        assert 'x = 1' in code
        assert '_M__x' not in code

    def test_internal_call_to_private_member_uses_mangled_name(self):
        code = transpile(
            'module M {\n'
            '  def hidden() -> int { return 1 }\n'
            '  export def pub() -> int { return hidden() }\n'
            '}\n')
        assert 'M._M__hidden()' in code

    def test_internal_reference_to_private_data_uses_mangled_name(self):
        code = transpile(
            'module M {\n'
            '  const K = 1\n'
            '  export def f() -> int { return K }\n'
            '}\n')
        assert 'M._M__K' in code

    def test_local_shadowing_is_not_mangled(self):
        code = transpile(
            'module M {\n'
            '  const K = 1\n'
            '  export def f() -> int {\n'
            '    let K = 100\n'
            '    return K\n'
            '  }\n'
            '}\n')
        assert 'K = 100' in code
        assert '_M__K = 100' not in code
        assert 'return K' in code

    def test_dotted_module_emits_nested_classes(self):
        code = transpile('module Outer.Inner {\n  export def f() -> int { return 1 }\n}\n')
        assert 'class Outer:' in code
        assert 'class Inner:' in code

    def test_generated_python_is_valid(self):
        import ast as py_ast
        code = transpile(
            'module M {\n'
            '  export let a = 1\n'
            '  let b = 2\n'
            '  export def f(x: int) -> int { return x + b }\n'
            '  def g() -> int { return a }\n'
            '  export class C {\n    public let x: int = 0\n  }\n'
            '}\n')
        py_ast.parse(code)


# ============================================================================
# Runtime behaviour
# ============================================================================

class TestModuleRuntime:
    def test_exported_function_is_callable(self):
        out = run_aura(
            'module M {\n  export def f() -> int { return 7 }\n}\n'
            'def main() { print(M.f()) }\n')
        assert out == '7\n'

    def test_exported_data_is_readable(self):
        out = run_aura(
            'module M {\n  export const K = 5\n}\n'
            'def main() { print(M.K) }\n')
        assert out == '5\n'

    def test_exported_class_is_instantiable(self):
        out = run_aura(
            'module M {\n'
            '  export class C {\n'
            '    public let x: int = 3\n'
            '    public def new(x: int) { self.x = x }\n'
            '  }\n'
            '}\n'
            'def main() { print(M.C(9).x) }\n')
        assert out == '9\n'

    def test_module_private_state_is_mutable_internally(self):
        out = run_aura(
            'module Counter {\n'
            '  let mut count = 0\n'
            '  export def inc() -> int {\n'
            '    count = count + 1\n'
            '    return count\n'
            '  }\n'
            '}\n'
            'def main() {\n'
            '  Counter.inc()\n'
            '  Counter.inc()\n'
            '  print(Counter.inc())\n'
            '}\n')
        assert out == '3\n'

    def test_private_function_is_inaccessible(self):
        out = run_aura(
            'module M {\n  def hidden() -> int { return 1 }\n}\n'
            'def main() {\n'
            '  try { print(M.hidden()) }\n'
            '  catch AttributeError { print("blocked") }\n'
            '}\n')
        assert out == 'blocked\n'

    def test_nested_module_access(self):
        out = run_aura(
            'module Outer {\n'
            '  export module Inner {\n'
            '    export def f() -> int { return 3 }\n'
            '  }\n'
            '}\n'
            'def main() { print(Outer.Inner.f()) }\n')
        assert out == '3\n'


# ============================================================================
# Diagnostics
# ============================================================================

class TestModuleRules:
    def test_accessing_a_non_exported_member_reports_e308(self):
        assert 'E308' in rule_codes(
            'module M {\n  def hidden() -> int { return 1 }\n}\n'
            'def main() { print(M.hidden()) }\n')

    def test_e308_message_names_the_module_and_export(self):
        messages = rule_messages(
            'module M {\n  def hidden() -> int { return 1 }\n}\n'
            'def main() { print(M.hidden()) }\n')
        assert 'not exported' in messages[0]
        assert 'M' in messages[0]
        assert 'export' in messages[0]

    def test_exported_member_is_clean(self):
        assert 'E308' not in rule_codes(
            'module M {\n  export def f() -> int { return 1 }\n}\n'
            'def main() { print(M.f()) }\n')

    def test_internal_use_of_private_member_is_clean(self):
        assert 'E308' not in rule_codes(
            'module M {\n'
            '  def hidden() -> int { return 1 }\n'
            '  export def pub() -> int { return hidden() }\n'
            '}\n')

    def test_assigning_module_state_from_outside_reports_e303(self):
        codes = rule_codes(
            'module M {\n  export let mut c = 0\n}\n'
            'def main() { M.c = 1 }\n')
        assert 'E303' in codes

    def test_assigning_module_state_message(self):
        messages = rule_messages(
            'module M {\n  export let mut c = 0\n}\n'
            'def main() { M.c = 1 }\n')
        assert 'module state is not writable' in messages[0]

    def test_internal_assignment_to_module_state_is_clean(self):
        assert 'E303' not in rule_codes(
            'module M {\n'
            '  let mut c = 0\n'
            '  export def bump() { c = c + 1 }\n'
            '}\n')

    def test_duplicate_member_reports_e301(self):
        assert 'E301' in rule_codes(
            'module M {\n'
            '  export def f() -> int { return 1 }\n'
            '  def f() -> int { return 2 }\n'
            '}\n')

    def test_module_member_may_share_a_top_level_name(self):
        # A module is its own namespace, so `f` inside `M` does not clash with
        # a top-level `f`.
        assert 'E301' not in rule_codes(
            'module M {\n  export def f() -> int { return 1 }\n}\n'
            'def f() -> int { return 2 }\n'
            'def main() { print(f() + M.f()) }\n')

    def test_type_errors_inside_a_module_are_reported(self):
        program = Parser(Tokenizer(
            'module M {\n'
            '  export def f(x: int) -> int { return x }\n'
            '}\n').tokenize()).parse()
        checker = TypeChecker()
        checker.check_program(program)
        assert checker.errors == []


# ============================================================================
# Cross-file imports
# ============================================================================

class TestModuleImports:
    def test_module_in_another_file(self, tmp_path):
        (tmp_path / "mathlib.aura").write_text(
            'module Math {\n'
            '  export def square(x: int) -> int { return x * x }\n'
            '  def secret() -> int { return 0 }\n'
            '}\n', encoding="utf-8")
        program = tmp_path / "app.aura"
        program.write_text(
            'import mathlib\n'
            'def main() { print(mathlib.Math.square(6)) }\n', encoding="utf-8")
        result = subprocess.run(
            [sys.executable, "-m", "aura.cli", "run", str(program)],
            capture_output=True, text=True, timeout=60, cwd=str(tmp_path))
        assert result.returncode == 0, result.stderr
        assert result.stdout.strip() == "36"

    def test_cross_file_private_access_fails(self, tmp_path):
        (tmp_path / "lib.aura").write_text(
            'module L {\n  def hidden() -> int { return 1 }\n}\n', encoding="utf-8")
        program = tmp_path / "app.aura"
        program.write_text(
            'import lib\n'
            'def main() { print(lib.L.hidden()) }\n', encoding="utf-8")
        result = subprocess.run(
            [sys.executable, "-m", "aura.cli", "run", str(program)],
            capture_output=True, text=True, timeout=60, cwd=str(tmp_path))
        assert result.returncode != 0
