"""Branch coverage for ``aura.transpiler.rules`` traversal.

The extreme-rules suite pins one program per diagnostic code. These tests
instead exercise the *traversal* paths: statements and patterns that must be
walked (module nesting, match/try/with, nested functions, guard-else) plus the
helper predicates used during access control and pattern declaration.
"""
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler import ast  # noqa: E402
from aura.transpiler.rules import RuleChecker  # noqa: E402


def parse(source):
    return Parser(Tokenizer(source).tokenize()).parse()


def rule_errors(source, require_main=False):
    program = parse(source)
    checker = RuleChecker()
    checker.check_program(program, require_main=require_main)
    return checker.errors


# ============================================================================
# Traversal: statements
# ============================================================================

class TestRuleTraversal:
    def test_module_scope_and_members(self):
        # A module pushes/pops its own scope and checks member visibility.
        errors = rule_errors(
            'module M {\n'
            '  public def f() -> int { return 1 }\n'
            '}\n')
        assert errors == []

    def test_module_member_without_visibility_is_allowed(self):
        # Module members are implicitly public; no visibility diagnostic.
        errors = rule_errors(
            'module M {\n'
            '  def f() -> int { return 1 }\n'
            '}\n')
        assert not any('E307' in e for e in errors)

    def test_enum_and_type_declarations_are_declared(self):
        errors = rule_errors(
            'enum Color { RED, GREEN }\n'
            'type Alias = int\n'
            'def f() -> int { return 1 }\n')
        assert errors == []

    def test_trait_method_signature_is_abstract(self):
        errors = rule_errors(
            'trait Shape {\n'
            '  public def area() -> int\n'
            '}\n'
            'class Square extends Shape {\n'
            '  public def area() -> int { return 4 }\n'
            '}\n')
        assert errors == []

    def test_for_and_loop_bodies(self):
        errors = rule_errors(
            'def f() -> int {\n'
            '  let total = 0\n'
            '  for item in [1, 2] {\n'
            '    total += item\n'
            '  }\n'
            '  loop {\n'
            '    break\n'
            '  }\n'
            '  return total\n'
            '}\n')
        assert errors == []

    def test_match_scopes_and_guard(self):
        errors = rule_errors(
            'def f(n: int) -> int {\n'
            '  match n {\n'
            '    case x if x > 0 { return x }\n'
            '    case _ { return 0 }\n'
            '  }\n'
            '}\n')
        assert errors == []

    def test_try_catch_finally_with_binding(self):
        errors = rule_errors(
            'def f() -> int {\n'
            '  try {\n'
            '    return 1\n'
            '  } catch Error as err {\n'
            '    return 2\n'
            '  } finally {\n'
            '    print("done")\n'
            '  }\n'
            '}\n')
        assert errors == []

    def test_with_statement_binding(self):
        errors = rule_errors(
            'def f() -> int {\n'
            '  with open("x") as handle {\n'
            '    return 1\n'
            '  }\n'
            '}\n')
        assert errors == []

    def test_throw_and_assert_are_walked(self):
        errors = rule_errors(
            'def f(x: int) -> int {\n'
            '  assert x > 0, "must be positive"\n'
            '  if x < 0 { throw "negative" }\n'
            '  return x\n'
            '}\n')
        assert errors == []

    def test_lambda_body_is_walked(self):
        errors = rule_errors(
            'def f() -> int {\n'
            '  let add = (x) => x + 1\n'
            '  return add(1)\n'
            '}\n')
        assert errors == []

    def test_await_in_async_function_ok(self):
        errors = rule_errors(
            'async def work() -> int { return 1 }\n'
            'async def f() -> int {\n'
            '  return await work()\n'
            '}\n')
        assert errors == []

    def test_member_and_call_traversal(self):
        errors = rule_errors(
            'class Box {\n'
            '  public let value: int = 1\n'
            '  public def get(self) -> int { return self.value }\n'
            '}\n'
            'def f() -> int {\n'
            '  let b = Box()\n'
            '  return b.get()\n'
            '}\n')
        assert errors == []

    def test_safe_navigation_traversal(self):
        errors = rule_errors(
            'class Box {\n'
            '  public let value: int = 1\n'
            '}\n'
            'def f(b: Box) -> int {\n'
            '  return b?.value\n'
            '}\n')
        assert errors == []


# ============================================================================
# Pattern helpers
# ============================================================================

class TestPatternHelpers:
    def _checker(self):
        return RuleChecker()

    def test_pattern_names_variants(self):
        checker = self._checker()
        assert checker._pattern_names(None) == []
        assert checker._pattern_names(ast.IdentifierPattern('a')) == ['a']
        assert checker._pattern_names(ast.Identifier('b')) == ['b']
        assert checker._pattern_names(ast.WildcardPattern()) == []
        assert checker._pattern_names(
            ast.SpreadExpr(ast.IdentifierPattern('rest'))) == ['rest']

    def test_pattern_names_nested_lists(self):
        checker = self._checker()
        pattern = ast.ListPattern([
            ast.IdentifierPattern('head'),
            ast.ListPattern([ast.IdentifierPattern('inner')],
                            rest_pattern=ast.IdentifierPattern('tail')),
        ])
        assert checker._pattern_names(pattern) == ['head', 'inner', 'tail']

    def test_target_names_variants(self):
        checker = self._checker()
        assert checker._target_names('') == []
        assert checker._target_names('x') == ['x']
        assert checker._target_names('(a, b)') == ['a', 'b']
        assert checker._target_names('[a, *rest]') == ['a', 'rest']
        assert checker._target_names(42) == []

    def test_is_assignable_variants(self):
        checker = self._checker()
        assert checker._is_assignable(ast.Identifier('x')) is True
        assert checker._is_assignable(
            ast.MemberExpr(ast.Identifier('self'), 'x')) is True
        assert checker._is_assignable(
            ast.IndexExpr(ast.Identifier('a'), ast.IntLiteral(0))) is True
        assert checker._is_assignable(
            ast.TupleLiteral([ast.Identifier('a'), ast.Identifier('b')])) is True
        assert checker._is_assignable(
            ast.SpreadExpr(ast.Identifier('rest'))) is True
        assert checker._is_assignable(ast.IntLiteral(1)) is False

    def test_declare_pattern_registers_names(self):
        checker = self._checker()
        checker._push_scope()
        checker._declare_pattern(ast.IdentifierPattern('item'))
        assert 'item' in checker._scope_stack[-1]
        checker._pop_scope()


# ============================================================================
# Abstract-method resolution
# ============================================================================

class TestAbstractResolution:
    def test_transitive_abstract_from_grandbase(self):
        errors = rule_errors(
            'trait Root {\n'
            '  public def run() -> int\n'
            '}\n'
            'class Mid extends Root {\n'
            '  public def run() -> int { return 1 }\n'
            '}\n'
            'class Leaf extends Mid {\n'
            '}\n'
            'def f() -> int { return 1 }\n')
        assert errors == []

    def test_missing_transitive_abstract_reported(self):
        errors = rule_errors(
            'trait Root {\n'
            '  public def run() -> int\n'
            '}\n'
            'class Mid extends Root {\n'
            '  public def run() -> int { return 1 }\n'
            '}\n'
            'class Broken extends Root {\n'
            '}\n'
            'def f() -> int { return 1 }\n')
        assert any('E309' in e for e in errors)

    def test_abstract_check_ignores_non_class(self):
        checker = RuleChecker()
        # A non-ClassDecl node must be ignored by _check_abstract_implemented.
        checker._check_abstract_implemented(ast.Identifier('x'))
        assert checker.errors == []

    def test_lookup_member_unknown_class_and_cycle(self):
        checker = RuleChecker()
        assert checker._lookup_member('Nope', 'x', set()) == (None, None)
        checker._classes['A'] = {'members': {}, 'bases': ['B'],
                                 'is_trait': False, 'abstracts': {}}
        checker._classes['B'] = {'members': {}, 'bases': ['A'],
                                 'is_trait': False, 'abstracts': {}}
        assert checker._lookup_member('A', 'missing', set()) == (None, None)

    def test_object_class_resolution(self):
        checker = RuleChecker()
        assert checker._object_class(ast.Identifier('plain')) is None
        checker._class_stack.append('Box')
        assert checker._object_class(ast.Identifier('self')) == 'Box'
        checker._class_stack.pop()
        checker._instance_scopes.append({'b': 'Box'})
        assert checker._object_class(ast.Identifier('b')) == 'Box'

    def test_loc_without_location_uses_line(self):
        checker = RuleChecker()
        node = ast.Identifier('x')
        node.line = 12
        loc = checker._loc(node)
        assert loc is not None and loc.line == 12

    def test_loc_none_node(self):
        checker = RuleChecker()
        assert checker._loc(None) is None

    def test_declare_ignores_non_string_and_empty(self):
        checker = RuleChecker()
        checker._push_scope()
        checker._declare(None)
        checker._declare('')
        checker._declare(42)
        assert checker._scope_stack[-1] == set()

    def test_visit_handles_lists_and_none(self):
        checker = RuleChecker()
        checker.visit(None)
        checker.visit([None, ast.Identifier('x')])

    def test_duplicate_parameter_and_lambda_params(self):
        errors = rule_errors(
            'def f(a: int, a: int) -> int { return a }\n')
        assert any('E301' in e for e in errors)

    def test_guard_else_return_is_allowed(self):
        errors = rule_errors(
            'def main() {\n'
            '  let x = 1\n'
            '  guard x > 0 else { return }\n'
            '  print(x)\n'
            '}\n',
            require_main=True)
        assert errors == []

    def test_private_access_from_subclass_reported(self):
        errors = rule_errors(
            'class Base {\n'
            '  private let secret: int = 1\n'
            '}\n'
            'class Child extends Base {\n'
            '  public def peek(self) -> int { return self.secret }\n'
            '}\n'
            'def f() -> int { return 1 }\n')
        assert any('E308' in e for e in errors)

    def test_public_member_access_from_outside_ok(self):
        errors = rule_errors(
            'class Box {\n'
            '  public let value: int = 1\n'
            '}\n'
            'def f() -> int {\n'
            '  let b = Box()\n'
            '  return b.value\n'
            '}\n')
        assert errors == []
