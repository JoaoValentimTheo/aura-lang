"""Deep coverage of the expression transformer internals.

Complements ``test_transpiler_transformers.py`` by exercising the special-case
branches of ``ExpressionTransformer``: literal escaping, member conveniences,
hoisted lambdas/match/try blocks, patterns and closure collection.
"""
import pytest

from aura.transpiler import ast
from aura.transpiler.transformers.expressions import (
    ExpressionTransformer,
    _collect_closure_names,
    _pattern_names,
    mangle_member,
    py_safe_name,
)


def xf():
    return ExpressionTransformer()


# ---------------------------------------------------------------------------
# py_safe_name / mangle_member
# ---------------------------------------------------------------------------

def test_py_safe_name_variants():
    assert py_safe_name('raise') == 'raise_'
    assert py_safe_name('True') == 'True'
    assert py_safe_name('False') == 'False'
    assert py_safe_name('None') == 'None'
    assert py_safe_name('plain') == 'plain'
    assert py_safe_name('not valid') == 'not valid'


def test_mangle_member_visibility():
    assert mangle_member('Point', 'x', 'public') == 'x'
    assert mangle_member('Point', 'x', 'protected') == '_x'
    assert mangle_member('Point', 'x', 'private') == '_Point__x'
    assert mangle_member('Point', 'raise', 'public') == 'raise_'


# ---------------------------------------------------------------------------
# Literals
# ---------------------------------------------------------------------------

def test_transform_none_returns_literal():
    assert xf().transform(None) == 'None'


def test_transform_unknown_node_raises():
    class Weird(ast.Expr):
        pass

    with pytest.raises(NotImplementedError):
        xf().transform(Weird())


def test_transform_float_bool_none_literals():
    t = xf()
    assert t.transform(ast.FloatLiteral(1.5)) == '1.5'
    assert t.transform(ast.BoolLiteral(True)) == 'True'
    assert t.transform(ast.BoolLiteral(False)) == 'False'
    assert t.transform(ast.NoneLiteral()) == 'None'


def test_transform_str_literal_uses_raw_when_present():
    t = xf()
    assert t.transform(ast.StrLiteral('hi')) == "'hi'"
    node = ast.StrLiteral('hi')
    node.raw_literal = '"hi"'
    assert t.transform(node) == '"hi"'


def test_transform_fstring_escapes_text_and_interpolates():
    t = xf()
    node = ast.FStringLiteral(['a{b}', (ast.Identifier('x'), '')], quote='"')
    out = t.transform(node)
    assert out == 'f"a{{b}}{x}"'


def test_escape_fstring_text_handles_control_and_quote():
    esc = ExpressionTransformer._escape_fstring_text
    assert esc('a\nb', '"') == 'a\\nb'
    assert esc('a\tb', '"') == 'a\\tb'
    assert esc('a"b', '"') == 'a\\"b'
    # A backslash-escaped quote is left intact.
    assert esc('a\\"b', '"') == 'a\\"b'


def test_transform_dict_literal_with_spread_and_fallback():
    t = xf()
    node = ast.DictLiteral([
        (ast.StrLiteral('k'), ast.IntLiteral(1)),
        ast.SpreadExpr(ast.Identifier('extra'), is_dict=True),
        ast.IntLiteral(99),
    ])
    out = t.transform(node)
    assert t.has_dict is True
    assert "'k': 1" in out
    assert '**extra' in out
    assert '99' not in out


def test_transform_tuple_single_and_multi():
    t = xf()
    assert t.transform(ast.TupleLiteral([ast.IntLiteral(1)])) == '(1,)'
    assert t.transform(ast.TupleLiteral([ast.IntLiteral(1),
                                         ast.IntLiteral(2)])) == '(1, 2)'


def test_transform_set_literal():
    assert xf().transform(ast.SetLiteral([ast.IntLiteral(1),
                                          ast.IntLiteral(2)])) == '{1, 2}'


# ---------------------------------------------------------------------------
# Binary / unary operators
# ---------------------------------------------------------------------------

def test_transform_binary_coalesce_and_elvis():
    t = xf()
    node = ast.BinaryOp(ast.Identifier('a'), '??', ast.Identifier('b'))
    assert t.transform(node) == '(a if a is not None else b)'
    node = ast.BinaryOp(ast.Identifier('a'), '?:', ast.Identifier('b'))
    assert t.transform(node) == '(a if a else b)'


def test_transform_binary_safe_nav_and_cast():
    t = xf()
    node = ast.BinaryOp(ast.Identifier('user'), '?.', ast.Identifier('name'))
    assert t.transform(node) == '(user.name if user is not None else None)'
    node = ast.BinaryOp(ast.Identifier('v'), 'as', ast.Identifier('int'))
    assert t.transform(node) == 'int(v)'


def test_transform_binary_default_operator_map():
    t = xf()
    node = ast.BinaryOp(ast.Identifier('a'), 'and', ast.Identifier('b'))
    assert t.transform(node) == '(a and b)'


def test_transform_unary_yield_and_ops():
    t = xf()
    assert t.transform(ast.UnaryOp('yield', None)) == 'yield'
    assert t.transform(ast.UnaryOp('yield', ast.IntLiteral(1))) == 'yield 1'
    assert t.transform(ast.UnaryOp('~', ast.IntLiteral(1))) == '(~ 1)'


# ---------------------------------------------------------------------------
# Member conveniences on calls
# ---------------------------------------------------------------------------

def test_call_member_length_is_empty_contains():
    t = xf()
    obj = ast.Identifier('s')
    assert t.transform(ast.CallExpr(ast.MemberExpr(obj, 'length'), [])) == 'len(s)'
    assert t.transform(ast.CallExpr(ast.MemberExpr(obj, 'is_empty'), [])) == '(not s)'
    assert t.transform(ast.CallExpr(ast.MemberExpr(obj, 'contains'),
                                    [ast.Identifier('x')])) == '(x in s)'


def test_call_member_slice_and_char_at():
    t = xf()
    obj = ast.Identifier('s')
    assert t.transform(ast.CallExpr(ast.MemberExpr(obj, 'slice'),
                                    [ast.IntLiteral(1)])) == 's[1:]'
    assert t.transform(ast.CallExpr(ast.MemberExpr(obj, 'slice'),
                                    [ast.IntLiteral(1),
                                     ast.IntLiteral(2)])) == 's[1:2]'
    assert t.transform(ast.CallExpr(ast.MemberExpr(obj, 'char_at'),
                                    [ast.IntLiteral(0)])) == 's[0]'


def test_call_member_alias_and_unknown():
    t = xf()
    obj = ast.Identifier('s')
    assert t.transform(ast.CallExpr(ast.MemberExpr(obj, 'to_upper'), [])) == 's.upper()'
    assert t.transform(ast.CallExpr(ast.MemberExpr(obj, 'custom'), [])) == 's.custom()'


def test_call_member_known_dunder_is_mangled():
    t = xf()
    t.member_visibilities['__eq__'] = ('private', 'Point')
    node = ast.CallExpr(ast.MemberExpr(ast.Identifier('p'), 'eq'),
                        [ast.Identifier('q')])
    assert t.transform(node) == 'p._Point____eq__(q)'


def test_call_adaptive_spread_uses_aura_call():
    t = xf()
    node = ast.CallExpr(ast.Identifier('f'),
                        [ast.SpreadExpr(ast.Identifier('args'), is_dict=None)])
    assert t.transform(node) == '_aura_call(f, args)'
    assert t._needs_aura_call is True


def test_render_call_args_with_dict_and_positional_spread():
    t = xf()
    node = ast.CallExpr(ast.Identifier('f'), [
        ast.IntLiteral(1),
        ast.SpreadExpr(ast.Identifier('a'), is_dict=False),
        ast.SpreadExpr(ast.Identifier('b'), is_dict=True),
        ast.SpreadExpr(ast.Identifier('c')),
    ], {'k': ast.IntLiteral(2)})
    out = t.transform(node)
    assert out == 'f(1, *a, **b, *c, k=2)'


# ---------------------------------------------------------------------------
# Index, slice, member, safe-nav
# ---------------------------------------------------------------------------

def test_transform_index_and_slice():
    t = xf()
    assert t.transform(ast.IndexExpr(ast.Identifier('a'),
                                     ast.IntLiteral(0))) == 'a[0]'
    assert t.transform(ast.SliceExpr(ast.Identifier('a'),
                                     start=ast.IntLiteral(1))) == 'a[1:]'
    assert t.transform(ast.SliceExpr(ast.Identifier('a'), step=ast.IntLiteral(2))) == 'a[::2]'


def test_transform_member_super_is_called():
    t = xf()
    assert t.transform(ast.MemberExpr(ast.Identifier('super'),
                                     'new')) == 'super().new'


def test_transform_member_known_protocol_method():
    t = xf()
    t.known_method_names.add('__str__')
    assert t.transform(ast.MemberExpr(ast.Identifier('p'), 'str')) == 'p.__str__'


def test_transform_safe_nav_member_and_index():
    t = xf()
    node = ast.SafeNavExpr(ast.Identifier('u'), 'name')
    assert t.transform(node) == '(u.name if u is not None else None)'
    node = ast.SafeNavExpr(ast.Identifier('u'), ast.IntLiteral(0), is_index=True)
    assert t.transform(node) == '(u[0] if u is not None else None)'


def test_transform_safe_nav_known_protocol_method():
    t = xf()
    t.known_method_names.add('__len__')
    node = ast.SafeNavExpr(ast.Identifier('u'), 'len')
    assert t.transform(node) == '(u.__len__ if u is not None else None)'


# ---------------------------------------------------------------------------
# Pipe, ternary, coalescing
# ---------------------------------------------------------------------------

def test_transform_pipe_into_call_inserts_argument():
    t = xf()
    node = ast.PipeExpr(ast.Identifier('v'),
                        ast.CallExpr(ast.Identifier('f'), [ast.IntLiteral(1)]))
    assert t.transform(node) == 'f(v, 1)'


def test_transform_pipe_into_identifier():
    t = xf()
    node = ast.PipeExpr(ast.Identifier('v'), ast.Identifier('f'))
    assert t.transform(node) == 'f(v)'


def test_transform_cond_elvis_coalesce_nodes():
    t = xf()
    assert t.transform(ast.CondExpr(ast.BoolLiteral(True), ast.IntLiteral(1),
                                    ast.IntLiteral(0))) == '(1 if True else 0)'
    assert t.transform(ast.ElvisExpr(ast.Identifier('a'),
                                     ast.IntLiteral(0))) == '(a if a else 0)'
    assert t.transform(ast.CoalesceExpr(ast.Identifier('a'),
                                        ast.IntLiteral(0))) == '(a if a is not None else 0)'


# ---------------------------------------------------------------------------
# If statement in expression position
# ---------------------------------------------------------------------------

def test_if_stmt_extracts_branch_values():
    t = xf()
    node = ast.IfStmt(ast.Identifier('c'),
                      [ast.ExprStmt(ast.IntLiteral(1))],
                      [ast.ExprStmt(ast.IntLiteral(2))])
    assert t.transform(node) == '(1 if c else 2)'


def test_if_stmt_missing_else_and_complex_last_stmt():
    t = xf()
    node = ast.IfStmt(ast.Identifier('c'), [], None)
    assert t.transform(node) == '(None if c else None)'
    # A trailing non-expression statement falls back to None.
    node = ast.IfStmt(ast.Identifier('c'),
                      [ast.ExprStmt(ast.IntLiteral(1))],
                      [ast.BreakStmt()])
    assert t.transform(node) == '(1 if c else None)'


# ---------------------------------------------------------------------------
# Ranges
# ---------------------------------------------------------------------------

def test_transform_range_infinite_and_step():
    t = xf()
    node = ast.RangeExpr(ast.IntLiteral(0), None)
    assert t.transform(node) == 'itertools.count(0)'
    node = ast.RangeExpr(ast.IntLiteral(0), ast.IntLiteral(10),
                         exclusive=True, step=ast.IntLiteral(2))
    assert t.transform(node) == 'range(0, 10, 2)'


# ---------------------------------------------------------------------------
# Lambda and block hoisting
# ---------------------------------------------------------------------------

def test_transform_simple_lambda():
    t = xf()
    node = ast.LambdaExpr([ast.Parameter('a')], ast.BinaryOp(
        ast.Identifier('a'), '+', ast.IntLiteral(1)))
    assert t.transform(node) == '(lambda a: (a + 1))'


def test_hoist_block_lambda_includes_nonlocal():
    t = xf()
    owner = t._make_stmt_transformer()
    owner.function_scopes = [{'count'}]
    t.stmt_transformer = owner
    body = ast.BlockExpr([ast.ExprStmt(
        ast.BinaryOp(ast.Identifier('count'), '=', ast.IntLiteral(1)))])
    node = ast.LambdaExpr([ast.Parameter('a')], body)
    name = t.transform(node)
    assert name.startswith('_aura_lambda_')
    joined = '\n'.join(t.hoisted_functions)
    assert 'nonlocal count' in joined


def test_nonlocal_declaration_requires_scope():
    t = xf()
    assert t._nonlocal_declaration([], set()) == ''


def test_transform_empty_block_returns_none():
    assert xf().transform(ast.BlockExpr([])) == 'None'


def test_transform_block_returns_last_expr():
    t = xf()
    node = ast.BlockExpr([ast.ExprStmt(ast.IntLiteral(1)),
                          ast.ExprStmt(ast.IntLiteral(2))])
    assert t.transform(node).startswith('_aura_block_')


# ---------------------------------------------------------------------------
# Comprehensions
# ---------------------------------------------------------------------------

def test_comprehension_dict_term():
    t = xf()
    node = ast.ComprehensionExpr(
        (ast.Identifier('k'), ast.Identifier('v')),
        [(ast.TupleLiteral([ast.Identifier('k'), ast.Identifier('v')]),
          ast.DictLiteral([]), [])],
        'dict',
    )
    out = t.transform(node)
    assert out.startswith('{k: v for')
    assert '.items()' in out


def test_comprehension_generator_and_filters():
    t = xf()
    node = ast.ComprehensionExpr(
        ast.Identifier('x'),
        [(ast.IdentifierPattern('x'), ast.Identifier('xs'),
          [ast.BinaryOp(ast.Identifier('x'), '>', ast.IntLiteral(0))])],
        'generator',
    )
    assert t.transform(node) == '(x for x in xs if (x > 0))'


def test_comprehension_unknown_type_raises():
    t = xf()
    node = ast.ComprehensionExpr(ast.Identifier('x'), [], 'weird')
    with pytest.raises(NotImplementedError):
        t.transform(node)


def test_needs_items_variants():
    needs = ExpressionTransformer._needs_items
    assert needs(ast.DictLiteral([]), 'AuraDict({})') is True
    assert needs(ast.Identifier('d'), 'd.items()') is False
    assert needs(ast.Identifier('d'), 'd.keys()') is False
    assert needs(ast.Identifier('d'), 'dict()') is True
    assert needs(ast.Identifier('d'), 'AuraDict(x)') is True
    assert needs(ast.Identifier('d'), 'd') is False


# ---------------------------------------------------------------------------
# Spread
# ---------------------------------------------------------------------------

def test_transform_spread_dict_and_positional():
    t = xf()
    assert t.transform(ast.SpreadExpr(ast.Identifier('a'),
                                      is_dict=True)) == '**a'
    assert t.transform(ast.SpreadExpr(ast.Identifier('a'),
                                      is_dict=False)) == '*a'


# ---------------------------------------------------------------------------
# Match / Try expressions
# ---------------------------------------------------------------------------

def test_transform_match_expr_with_guard_and_catch_all():
    t = xf()
    node = ast.MatchExpr(ast.Identifier('n'), [
        ast.MatchCase(ast.LiteralPattern(ast.IntLiteral(1)), None,
                      [ast.ExprStmt(ast.StrLiteral('one'))]),
        ast.MatchCase(ast.WildcardPattern(), None,
                      [ast.ExprStmt(ast.StrLiteral('other'))]),
    ])
    out = t.transform(node)
    assert out.startswith('_aura_match_')
    joined = '\n'.join(t.hoisted_functions)
    assert 'match ' in joined
    # The explicit wildcard is the only catch-all; no synthetic default added.
    assert joined.count('case _:') == 1
    assert "return 'other'" in joined


def test_transform_match_expr_without_catch_all_adds_default():
    t = xf()
    node = ast.MatchExpr(ast.Identifier('n'), [
        ast.MatchCase(ast.LiteralPattern(ast.IntLiteral(1)), None,
                      [ast.ExprStmt(ast.StrLiteral('one'))]),
    ])
    t.transform(node)
    joined = '\n'.join(t.hoisted_functions)
    assert 'case _:' in joined


def test_transform_try_expr_with_finally():
    t = xf()
    node = ast.TryExpr(
        [ast.ExprStmt(ast.IntLiteral(1))],
        [ast.CatchClause('ValueError', 'e', [ast.ExprStmt(ast.IntLiteral(2))])],
        [ast.ExprStmt(ast.IntLiteral(3))],
    )
    out = t.transform(node)
    assert out.startswith('_aura_try_')
    joined = '\n'.join(t.hoisted_functions)
    assert 'except ValueError as e:' in joined
    assert 'finally:' in joined


def test_case_value_variants():
    t = xf()
    assert t._case_value([]) == ('None', [])
    tail, rest = t._case_value([ast.ExprStmt(ast.IntLiteral(1))])
    assert tail == '1' and rest == []
    tail, rest = t._case_value([ast.ReturnStmt(ast.IntLiteral(2))])
    assert tail == '2' and rest == []
    tail, rest = t._case_value([ast.BreakStmt()])
    assert tail == 'None' and rest


# ---------------------------------------------------------------------------
# Patterns
# ---------------------------------------------------------------------------

def test_transform_pattern_variants():
    t = xf()
    assert t._transform_pattern(ast.IdentifierPattern('x')) == 'x'
    assert t._transform_pattern(ast.WildcardPattern()) == '_'
    assert t._transform_pattern(ast.ListPattern(
        [ast.IdentifierPattern('a')], rest_pattern=ast.IdentifierPattern('r'))) == '[a, *r]'
    assert t._transform_pattern(ast.DictPattern(
        {'k': ast.IdentifierPattern('v')},
        rest_pattern=ast.IdentifierPattern('r'))) == '{k=v, **r}'


def test_transform_list_pattern_node():
    t = xf()
    assert t.transform(ast.ListPattern(
        [ast.IdentifierPattern('a')],
        rest_pattern=ast.IdentifierPattern('r'))) == '[a, *r]'


def test_transform_dict_pattern_node():
    t = xf()
    assert t.transform(ast.DictPattern(
        {'k': ast.IdentifierPattern('v')})) == '{"k": v}'


def test_pattern_names_variants():
    assert _pattern_names(None) == []
    assert _pattern_names(ast.IdentifierPattern('x')) == ['x']
    assert _pattern_names(ast.Identifier('y')) == ['y']
    assert _pattern_names(ast.TupleLiteral([ast.Identifier('a'),
                                            ast.Identifier('b')])) == ['a', 'b']
    assert _pattern_names(ast.ListPattern(
        [ast.IdentifierPattern('a')],
        rest_pattern=ast.IdentifierPattern('r'))) == ['a', 'r']
    assert _pattern_names(ast.SpreadExpr(ast.Identifier('z'))) == ['z']
    assert _pattern_names(ast.IntLiteral(1)) == []


# ---------------------------------------------------------------------------
# Closure name collection
# ---------------------------------------------------------------------------

def test_collect_closure_names_assignment_and_declarations():
    assigned = set()
    declared = set()
    _collect_closure_names([
        ast.ExprStmt(ast.BinaryOp(ast.Identifier('count'), '=', ast.IntLiteral(1))),
        ast.VarDecl('local', False, value=ast.IntLiteral(0)),
        ast.FunctionDecl('helper', [], None, []),
        ast.ForStmt(ast.IdentifierPattern('i'), ast.Identifier('xs'), []),
    ], assigned, declared)
    assert 'count' in assigned
    assert {'local', 'helper', 'i'} <= declared


def test_collect_closure_names_tuple_assignment():
    assigned = set()
    declared = set()
    _collect_closure_names([
        ast.ExprStmt(ast.TupleLiteral([
            ast.BinaryOp(ast.Identifier('a'), '=', ast.IntLiteral(1)),
            ast.Identifier('b'),
        ])),
    ], assigned, declared)
    assert {'a', 'b'} <= assigned


def test_collect_closure_names_control_flow():
    assigned = set()
    declared = set()
    _collect_closure_names([
        ast.WithStmt([(ast.Identifier('f'), 'fh')], []),
        ast.TryStmt([], [ast.CatchClause('E', 'err', [])], []),
        ast.MatchStmt(ast.Identifier('x'), [ast.MatchCase(ast.WildcardPattern(), None, [])]),
        ast.IfStmt(ast.Identifier('c'), [], []),
        ast.WhileStmt(ast.Identifier('c'), []),
        ast.ReturnStmt(ast.Identifier('r')),
        ast.BlockExpr([]),
    ], assigned, declared)
    assert {'fh', 'err'} <= declared


def test_collect_closure_names_nested_lambda_scope():
    assigned = set()
    declared = set()
    _collect_closure_names([
        ast.LambdaExpr([ast.Parameter('p')],
                       ast.ExprStmt(ast.BinaryOp(ast.Identifier('p'), '=',
                                                 ast.IntLiteral(1)))),
    ], assigned, declared)
    assert 'p' in declared


# ---------------------------------------------------------------------------
# _make_stmt_transformer
# ---------------------------------------------------------------------------

def test_make_stmt_transformer_shares_class_context():
    t = xf()
    owner = t._make_stmt_transformer()
    owner.class_members = {'m'}
    owner.in_class_scope = True
    owner.function_scopes = [{'x'}]
    t.stmt_transformer = owner
    child = t._make_stmt_transformer()
    assert child.class_members == {'m'}
    assert child.in_class_scope is True
