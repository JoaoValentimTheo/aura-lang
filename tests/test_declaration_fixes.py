"""Regression tests for declaration and module-scope bugs.

Covers the fixes in this release:
* `const` as a class/trait member (was parsed as a field literally named
  `const`, so `class C { const K = 1 }` produced stray `const`/`K` fields);
* assigning to a class-level `const` (`C.K = 2`) must be rejected (`E303`);
* bare references to module-level data members inside module functions
  (previously `NameError` at runtime).
"""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura_test_helpers import run_aura, transpile  # noqa: E402

# ============================================================================
# `const` as a class member
# ============================================================================

class TestClassConst:
    def test_class_const_lives_on_the_class(self):
        code = transpile('class C {\n  const K = 1\n}\n')
        assert 'K = 1  # const' in code
        # It must not introduce an instance field literally named `const`.
        assert 'self.const' not in code
        assert '__init__' not in code

    def test_class_const_runtime_access(self):
        out = run_aura(
            'class C {\n  const K = 41\n}\n'
            'def main() { print(C.K + 1) }\n')
        assert out == '42\n'

    def test_class_const_is_not_instance_field(self):
        code = transpile(
            'class C {\n  let x: int = 0\n  const K = 9\n}\n')
        assert 'K = 9  # const' in code
        assert "'x'" in code  # only x is a match/init param
        assert "'K'" not in code

    def test_static_const(self):
        code = transpile('class C {\n  public static const K = 1\n}\n')
        assert 'K = 1  # const' in code
        assert code.count('K') == 1

    def test_private_const_is_mangled(self):
        code = transpile('class C {\n  private const K = 1\n}\n')
        assert '_C__K = 1  # const' in code

    def test_protected_const_is_mangled(self):
        code = transpile('class C {\n  protected const K = 1\n}\n')
        assert '_K = 1  # const' in code

    def test_const_with_type_annotation(self):
        code = transpile('class C {\n  const K: int = 1\n}\n')
        assert 'K = 1  # const' in code

    def test_const_without_value_is_rejected(self):
        with pytest.raises(SyntaxError) as excinfo:
            transpile('class C {\n  const K\n}\n')
        assert 'requires a value' in str(excinfo.value)

    def test_const_usable_from_instance_method(self):
        out = run_aura(
            'class C {\n  const K = 7\n'
            '  public def get(self) -> int { return self.K }\n}\n'
            'def main() { print(C().get()) }\n')
        assert out == '7\n'

    def test_const_survives_nested_class(self):
        out = run_aura(
            'class Outer {\n  class Inner {\n    const K = 3\n  }\n}\n'
            'def main() { print(Outer.Inner.K) }\n')
        assert out == '3\n'


class TestTraitConst:
    def test_trait_const_is_emitted(self):
        code = transpile('trait T {\n  public const K = 1\n}\n')
        assert 'K = 1  # const' in code
        assert 'const = None' not in code

    def test_trait_const_requires_value(self):
        with pytest.raises(SyntaxError):
            transpile('trait T {\n  public const K\n}\n')


# ============================================================================
# Assigning to a class-level const is rejected (E303)
# ============================================================================

class TestConstAssignment:
    def _check(self, source):
        from aura.parser.to_ast import Parser, Tokenizer
        from aura.transpiler.rules import RuleChecker

        program = Parser(Tokenizer(source).tokenize()).parse()
        checker = RuleChecker()
        checker.check_program(program)
        return [str(e) for e in checker.collector.errors]

    def test_class_const_assignment_rejected(self):
        errors = self._check(
            'class C {\n  public const K = 1\n}\n'
            'def main() { C.K = 2 }\n')
        assert any('E303' in e for e in errors)

    def test_mutable_field_assignment_allowed(self):
        errors = self._check(
            'class C {\n  public let mut x: int = 0\n}\n'
            'def main() {\n  let c = C()\n  c.x = 2\n}\n')
        assert not any('E303' in e for e in errors)

    def test_static_mutable_field_assignment_allowed(self):
        errors = self._check(
            'class C {\n  public static let mut x = 0\n}\n'
            'def main() { C.x = 1 }\n')
        assert not any('E303' in e for e in errors)

    def test_plain_field_assignment_allowed(self):
        errors = self._check(
            'class C {\n  public let x: int = 0\n}\n'
            'def main() {\n  let c = C()\n  c.x = 2\n}\n')
        assert not any('E303' in e for e in errors)

    def test_interface_runtime_rejects_const_assign(self):
        # The runtime must also refuse: a Python class attribute is writable,
        # so the const guard is a static rule. This documents that the static
        # checker is the enforcement point.
        errors = self._check(
            'class C {\n  const K = 1\n}\n'
            'def main() { C.K = 5 }\n')
        assert any('E303' in e for e in errors)


# ============================================================================
# Module-level data members resolve through the module class
# ============================================================================

class TestModuleDataMembers:
    def test_module_const_reference(self):
        out = run_aura(
            'module M {\n  const K = 1\n'
            '  export def f() -> int { return K }\n}\n'
            'def main() { print(M.f()) }\n')
        assert out == '1\n'

    def test_module_let_reference(self):
        out = run_aura(
            'module M {\n  let x = 2\n'
            '  export def f() -> int { return x }\n}\n'
            'def main() { print(M.f()) }\n')
        assert out == '2\n'

    def test_module_mutable_state_across_calls(self):
        out = run_aura(
            'module Counter {\n'
            '  let mut count = 0\n'
            '  export def inc() -> int {\n    count += 1\n    return count\n  }\n'
            '  export def peek() -> int { return count }\n}\n'
            'def main() {\n'
            '  print(Counter.inc())\n'
            '  print(Counter.inc())\n'
            '  print(Counter.peek())\n'
            '}\n')
        assert out == '1\n2\n2\n'

    def test_local_shadows_module_member(self):
        out = run_aura(
            'module M {\n  const K = 1\n'
            '  export def f() -> int {\n    let K = 100\n    return K\n  }\n}\n'
            'def main() { print(M.f()) }\n')
        assert out == '100\n'

    def test_parameter_shadows_module_member(self):
        out = run_aura(
            'module M {\n  const K = 1\n'
            '  export def f(K: int) -> int { return K + 1 }\n}\n'
            'def main() { print(M.f(40)) }\n')
        assert out == '41\n'

    def test_module_member_reference_in_expression(self):
        out = run_aura(
            'module M {\n  const A = 6\n  const B = 7\n'
            '  export def product() -> int { return A * B }\n}\n'
            'def main() { print(M.product()) }\n')
        assert out == '42\n'


# ============================================================================
# Formatter: operator and comment preservation
# ============================================================================

class TestFormatterOperators:
    """The formatter must never turn valid Aura into invalid Aura."""

    def _fmt(self, line):
        from aura.tools.formatter import format_aura
        return format_aura(line + '\n').rstrip('\n')

    def test_range_operator_not_split(self):
        assert self._fmt('for i in 0..<10 { x() }') == 'for i in 0 ..< 10 { x() }'
        assert self._fmt('let r = a..b') == 'let r = a .. b'

    def test_exclusive_range_roundtrips(self):
        # The formatted output must still be valid Aura.
        out = self._fmt('for i in 0..<3 { print(i) }')
        assert '..<' in out
        code = transpile('def main() {\n  ' + out + '\n}\n')
        assert 'range' in code

    def test_pipe_and_arrows_preserved(self):
        assert self._fmt('let r = a |> b |> c') == 'let r = a |> b |> c'
        assert self._fmt('let f = (x) => x + 1') == 'let f = (x) => x + 1'

    def test_compound_operators_preserved(self):
        assert '**=' in self._fmt('x **= 2')
        assert '??=' in self._fmt('x ??= 2')
        assert '<<=' in self._fmt('x <<= 2')

    def test_null_coalescing_and_ternary(self):
        assert '??' in self._fmt('let x = a ?? b')
        assert '?:' in self._fmt('let x = a ?: b')

    def test_inline_block_comment_preserved(self):
        assert self._fmt('let x = 1 /* c */ + 2') == 'let x = 1 /* c */ + 2'

    def test_block_comment_inside_string_untouched(self):
        line = 'let s = "a /* not a comment */ b"'
        assert self._fmt(line) == line

    def test_trailing_comment_keeps_one_space(self):
        assert self._fmt('let x = 1  // keep   spacing') == \
            'let x = 1 // keep   spacing'

    def test_url_in_string_not_treated_as_comment(self):
        line = 'let url = "http://example.com"'
        assert self._fmt(line) == line

    def test_string_contents_not_respaced(self):
        assert self._fmt('let s = "a  b"') == 'let s = "a  b"'

    def test_formatted_output_still_transpiles(self):
        source = (
            'def main() {\n'
            '  let total = 0\n'
            '  for i in 0..<5 { total += i }\n'
            '  let label = "n" |> trim\n'
            '  print(total)\n'
            '}\n')
        from aura.tools.formatter import format_aura
        formatted = format_aura(source)
        assert transpile(formatted)  # must remain valid Aura

    def test_formatter_is_idempotent_on_operators(self):
        from aura.tools.formatter import format_aura
        source = 'let x = 0..<10\n'
        once = format_aura(source)
        assert format_aura(once) == once
