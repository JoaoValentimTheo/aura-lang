"""Deep coverage of ``MutabilityChecker`` in ``aura.transpiler.semantics``.

Exercises scope handling, every statement form, pattern binding, multi
assignment, ``collect_bindings`` and the diagnostic location fallbacks.
"""

from aura.transpiler import ast
from aura.transpiler.ast import SourceLocation
from aura.transpiler.errors import ErrorCode
from aura.transpiler.semantics import MutabilityChecker, _Scope


def run(*statements):
    checker = MutabilityChecker()
    program = ast.Program(list(statements))
    ok = checker.check_program(program)
    return ok, checker


def codes(checker):
    return [d.code for d in checker.diagnostics]


def assign(name, value, op='='):
    return ast.ExprStmt(ast.BinaryOp(ast.Identifier(name), op, value))


# ---------------------------------------------------------------------------
# _Scope
# ---------------------------------------------------------------------------

def test_scope_declare_ignores_invalid_names():
    scope = _Scope()
    scope.declare(None, True)
    scope.declare('', True)
    assert scope.bindings == {}


def test_scope_lookup_walks_parents():
    parent = _Scope()
    parent.declare('x', True)
    child = _Scope(parent)
    assert child.lookup('x') is True
    assert child.lookup('missing') is None


# ---------------------------------------------------------------------------
# Basic mutability rules
# ---------------------------------------------------------------------------

def test_let_is_immutable_but_let_mut_allows_reassignment():
    ok, checker = run(ast.VarDecl('x', False, value=ast.IntLiteral(1)),
                      assign('x', ast.IntLiteral(2)))
    assert ok is False
    assert codes(checker) == [ErrorCode.REASSIGN_IMMUTABLE]

    ok, checker = run(ast.VarDecl('x', True, value=ast.IntLiteral(1)),
                      assign('x', ast.IntLiteral(2)))
    assert ok is True
    assert checker.diagnostics == []


def test_const_reassignment_reports_violation():
    ok, checker = run(ast.ConstDecl('K', value=ast.IntLiteral(1)),
                      assign('K', ast.IntLiteral(2)))
    assert ok is False
    assert checker.violations == ['K']


# ---------------------------------------------------------------------------
# check_program / collect_bindings
# ---------------------------------------------------------------------------

def test_check_program_with_initial_bindings():
    checker = MutabilityChecker()
    program = ast.Program([assign('x', ast.IntLiteral(1))])
    assert checker.check_program(program, initial_bindings={'x': False}) is False
    assert checker.check_program(program, initial_bindings={'x': True}) is True


def test_collect_bindings_merges_seed_and_redeclaration():
    checker = MutabilityChecker()
    program = ast.Program([
        ast.VarDecl('a', True, value=ast.IntLiteral(1)),
        ast.ConstDecl('B', value=ast.IntLiteral(2)),
        ast.VarDecl('(c, d)', True, value=ast.IntLiteral(3)),
    ])
    bindings = checker.collect_bindings(program, seed={'a': False, 'z': True})
    assert bindings['a'] is True
    assert bindings['B'] is False
    assert bindings['c'] is True
    assert bindings['d'] is True
    assert bindings['z'] is True


# ---------------------------------------------------------------------------
# visit(): statement forms
# ---------------------------------------------------------------------------

def test_visit_none_and_sequence():
    checker = MutabilityChecker()
    checker.visit(None)
    checker.visit([None, ast.VarDecl('x', True)])
    assert checker._scope.lookup('x') is True


def test_if_else_condition_reassignment():
    ok, checker = run(
        ast.VarDecl('c', False, value=ast.BoolLiteral(True)),
        ast.IfStmt(assign('c', ast.BoolLiteral(False)), [], []),
    )
    assert ok is False


def test_unless_stmt_and_guard_stmt():
    ok, checker = run(
        ast.VarDecl('c', False, value=ast.BoolLiteral(True)),
        ast.UnlessStmt(assign('c', ast.BoolLiteral(False)), [], []),
        ast.GuardStmt(assign('c', ast.BoolLiteral(True)), []),
    )
    # Two reassignments of the immutable binding.
    assert len(checker.diagnostics) == 2


def test_while_and_until_stmt_bodies():
    ok, checker = run(
        ast.VarDecl('n', False, value=ast.IntLiteral(0)),
        ast.WhileStmt(ast.BoolLiteral(True), [assign('n', ast.IntLiteral(1))]),
        ast.UntilStmt(ast.BoolLiteral(False), [assign('n', ast.IntLiteral(2))]),
    )
    assert len(checker.diagnostics) == 2


def test_for_stmt_binds_loop_variable_mutable():
    ok, checker = run(
        ast.ForStmt(ast.IdentifierPattern('i'), ast.Identifier('xs'),
                    [assign('i', ast.IntLiteral(0))]),
    )
    assert ok is True


def test_loop_stmt_visits_body():
    ok, checker = run(
        ast.VarDecl('n', False, value=ast.IntLiteral(0)),
        ast.LoopStmt([assign('n', ast.IntLiteral(1))]),
    )
    assert ok is False


def test_match_stmt_binds_pattern_and_guard():
    case = ast.MatchCase(
        ast.IdentifierPattern('v'), ast.BoolLiteral(True),
        [assign('v', ast.IntLiteral(1))],
    )
    ok, checker = run(ast.MatchStmt(ast.Identifier('x'), [case]))
    assert ok is True


def test_try_stmt_binds_catch_variable():
    clause = ast.CatchClause('ValueError', 'err', [assign('err', ast.IntLiteral(1))])
    ok, checker = run(ast.TryStmt([], [clause], []))
    assert ok is True


def test_with_stmt_binds_targets():
    ok, checker = run(ast.WithStmt(
        [(ast.Identifier('open'), 'fh')],
        [assign('fh', ast.IntLiteral(1))],
    ))
    assert ok is True


def test_lambda_params_are_local():
    lam = ast.LambdaExpr([ast.Parameter('p')],
                         ast.ExprStmt(ast.BinaryOp(ast.Identifier('p'), '=',
                                                   ast.IntLiteral(1))))
    ok, checker = run(ast.ExprStmt(lam))
    assert ok is True


def test_block_expr_uses_nested_scope():
    block = ast.BlockExpr([
        ast.VarDecl('inner', False, value=ast.IntLiteral(1)),
        assign('inner', ast.IntLiteral(2)),
    ])
    ok, checker = run(ast.ExprStmt(block))
    assert ok is False


def test_module_visits_members():
    ok, checker = run(
        ast.VarDecl('m', False, value=ast.IntLiteral(1)),
        ast.Module('M', [ast.VarDecl('x', True)]),
    )
    assert ok is True


def test_return_stmt_visits_value():
    ok, checker = run(
        ast.VarDecl('r', False, value=ast.IntLiteral(1)),
        ast.ReturnStmt(assign('r', ast.IntLiteral(2))),
    )
    assert ok is False


def test_enum_type_and_trait_decls_are_ignored():
    ok, checker = run(
        ast.EnumDecl('E', []),
        ast.TypeDecl('T', ast.SimpleType('int')),
        ast.TraitDecl('Tr', []),
    )
    assert ok is True


# ---------------------------------------------------------------------------
# Functions and classes
# ---------------------------------------------------------------------------

def test_function_defaults_evaluated_in_enclosing_scope():
    ok, checker = run(
        ast.VarDecl('d', False, value=ast.IntLiteral(1)),
        ast.FunctionDecl('f', [
            ast.Parameter('a', default=assign('d', ast.IntLiteral(2))),
        ], None, []),
    )
    assert ok is False


def test_function_params_are_mutable_locally():
    func = ast.FunctionDecl('f', [ast.Parameter('a')], None,
                            [assign('a', ast.IntLiteral(1))])
    ok, checker = run(func)
    assert ok is True


def test_function_expression_body_visited():
    func = ast.FunctionDecl('f', [], None, ast.Identifier('g'))
    ok, checker = run(func)
    assert ok is True


def test_class_body_uses_nested_scope():
    klass = ast.ClassDecl('C', [ast.VarDecl('x', True)])
    ok, checker = run(klass)
    assert ok is True


# ---------------------------------------------------------------------------
# Assignments
# ---------------------------------------------------------------------------

def test_member_and_index_assignments_are_object_mutations():
    ok, checker = run(
        ast.VarDecl('o', False, value=ast.Identifier('obj')),
        ast.ExprStmt(ast.BinaryOp(ast.MemberExpr(ast.Identifier('o'), 'x'),
                                  '=', ast.IntLiteral(1))),
        ast.ExprStmt(ast.BinaryOp(ast.IndexExpr(ast.Identifier('o'),
                                                ast.IntLiteral(0)),
                                  '=', ast.IntLiteral(1))),
    )
    assert ok is True


def test_tuple_target_assignment_collects_names():
    ok, checker = run(
        ast.VarDecl('a', False, value=ast.IntLiteral(1)),
        ast.ExprStmt(ast.BinaryOp(
            ast.TupleLiteral([ast.Identifier('a'), ast.Identifier('b')]),
            '=', ast.TupleLiteral([ast.IntLiteral(1), ast.IntLiteral(2)]))),
    )
    assert ok is False
    assert checker.violations == ['a']


def test_tuple_target_with_spread_and_member():
    ok, checker = run(
        ast.VarDecl('a', False, value=ast.IntLiteral(1)),
        ast.ExprStmt(ast.BinaryOp(
            ast.ListLiteral([ast.SpreadExpr(ast.Identifier('a')),
                             ast.MemberExpr(ast.Identifier('o'), 'x')]),
            '=', ast.Identifier('src'))),
    )
    assert ok is False


def test_multi_assignment_tuple_literal():
    ok, checker = run(
        ast.VarDecl('a', False, value=ast.IntLiteral(1)),
        ast.ExprStmt(ast.TupleLiteral([
            ast.Identifier('a'),
            ast.BinaryOp(ast.Identifier('b'), '=', ast.IntLiteral(2)),
        ])),
    )
    assert ok is False
    assert checker.violations == ['a']


def test_plain_tuple_literal_expression_is_visited():
    ok, checker = run(ast.ExprStmt(ast.TupleLiteral([ast.IntLiteral(1),
                                                     ast.IntLiteral(2)])))
    assert ok is True


# ---------------------------------------------------------------------------
# Patterns and target names
# ---------------------------------------------------------------------------

def test_bind_pattern_variants():
    checker = MutabilityChecker()
    checker._bind_pattern(None, True)
    checker._bind_pattern(ast.Identifier('i'), True)
    checker._bind_pattern(ast.TupleLiteral([ast.Identifier('a'),
                                            ast.Identifier('b')]), True)
    checker._bind_pattern(ast.ListPattern(
        [ast.IdentifierPattern('x')],
        rest_pattern=ast.IdentifierPattern('r')), True)
    checker._bind_pattern(ast.SpreadExpr(ast.Identifier('s')), True)
    checker._bind_pattern(ast.IntLiteral(1), True)
    for name in ('i', 'a', 'b', 'x', 'r', 's'):
        assert checker._scope.lookup(name) is True


def test_target_names_forms():
    checker = MutabilityChecker()
    assert checker._target_names(None) == []
    assert checker._target_names('x') == ['x']
    assert checker._target_names('(a, b)') == ['a', 'b']
    assert checker._target_names('[c, d]') == ['c', 'd']


def test_split_names_skips_non_identifiers():
    assert MutabilityChecker._split_names('a, 1, b') == ['a', 'b']


# ---------------------------------------------------------------------------
# Locations
# ---------------------------------------------------------------------------

def test_reassignment_uses_node_location():
    node = ast.Identifier('x')
    node.location = SourceLocation(line=3, column=1)
    ok, checker = run(
        ast.VarDecl('x', False, value=ast.IntLiteral(1)),
        ast.ExprStmt(ast.BinaryOp(node, '=', ast.IntLiteral(2))),
    )
    assert checker.diagnostics[0].location.line == 3


def test_reassignment_falls_back_to_statement_location():
    stmt = assign('x', ast.IntLiteral(2))
    stmt.location = SourceLocation(line=7, column=1)
    ok, checker = run(
        ast.VarDecl('x', False, value=ast.IntLiteral(1)),
        stmt,
    )
    assert checker.diagnostics[0].location.line == 7


def test_reassignment_falls_back_to_line_attribute():
    target = ast.Identifier('x')
    target.line = 9
    ok, checker = run(
        ast.VarDecl('x', False, value=ast.IntLiteral(1)),
        ast.ExprStmt(ast.BinaryOp(target, '=', ast.IntLiteral(2))),
    )
    assert checker.diagnostics[0].location.line == 9


def test_reassignment_without_any_location_is_allowed():
    # A bare node without location is still reported (loc stays None).
    checker = MutabilityChecker()
    checker._scope.declare('x', False)
    checker._record_reassignment('x', ast.Identifier('x'))
    assert checker.diagnostics[0].location is None


def test_pop_scope_at_root_is_noop():
    checker = MutabilityChecker()
    root = checker._scope
    checker._pop_scope()
    assert checker._scope is root


def test_error_message_contains_hint():
    ok, checker = run(
        ast.VarDecl('x', False, value=ast.IntLiteral(1)),
        assign('x', ast.IntLiteral(2)),
    )
    err = checker.diagnostics[0]
    assert err.hint is not None
    assert 'let mut x' in err.hint
