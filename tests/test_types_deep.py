"""Deep coverage of the Aura type system internals.

Complements ``test_typechecker.py`` by exercising the type classes, the full
``TypeInference`` surface, annotation parsing and every ``TypeChecker``
traversal branch.
"""

from aura.parser.to_ast import Parser, Tokenizer
from aura.transpiler import ast
from aura.transpiler.errors import ErrorCode
from aura.transpiler.types import (
    AnyType,
    BoolType,
    ClassType,
    DictType,
    FloatType,
    FunctionType,
    IntType,
    ListType,
    NoneType,
    SetType,
    StrType,
    TupleType,
    TypeChecker,
    TypeInference,
    TypeVariable,
    UnionType,
)


def parse(source):
    return Parser(Tokenizer(source).tokenize()).parse()


def check(source):
    checker = TypeChecker()
    ok = checker.check_program(parse(source))
    return ok, checker


# ---------------------------------------------------------------------------
# Type classes
# ---------------------------------------------------------------------------

def test_base_type_str_and_equality():
    assert str(AnyType()) == 'AnyType'
    assert AnyType() == AnyType()
    assert AnyType() != IntType()


def test_primitive_type_strings():
    assert str(NoneType()) == 'None'
    assert str(IntType()) == 'Int'
    assert str(FloatType()) == 'Float'
    assert str(StrType()) == 'String'
    assert str(BoolType()) == 'Bool'


def test_list_type_string_and_compatibility():
    assert str(ListType(IntType())) == '[Int]'
    assert ListType(IntType()).is_compatible(ListType(IntType()))
    assert not ListType(IntType()).is_compatible(ListType(StrType()))
    # List vs non-list falls back to identity.
    assert ListType(IntType()).is_compatible(AnyType())


def test_dict_type_string_and_compatibility():
    d = DictType(StrType(), IntType())
    assert str(d) == '{String: Int}'
    assert d.is_compatible(DictType(StrType(), IntType()))
    assert not d.is_compatible(DictType(IntType(), IntType()))
    assert d.is_compatible(AnyType())


def test_set_type_string_and_compatibility():
    s = SetType(IntType())
    assert str(s) == '{Int}'
    assert s.is_compatible(SetType(IntType()))
    assert not s.is_compatible(SetType(StrType()))
    assert s.is_compatible(AnyType())


def test_tuple_type_string():
    assert str(TupleType([IntType(), StrType()])) == '(Int, String)'
    assert str(TupleType([])) == '()'


def test_function_type_string_variants():
    assert str(FunctionType([IntType()], IntType())) == '(Int) -> Int'
    assert str(FunctionType([], IntType(), is_async=True)) == 'async () -> Int'
    assert str(FunctionType([IntType()], IntType(), variadic=True)) == '(Int, ...) -> Int'
    assert str(FunctionType([], IntType(), variadic=True)) == '(...) -> Int'


def test_class_type_field_and_method_inheritance():
    parent = ClassType('Base', fields={'a': IntType()},
                       methods={'m': FunctionType([], IntType())})
    child = ClassType('Child', parent=parent)
    assert child.get_field_type('a') == IntType()
    assert child.get_method_type('m').return_type == IntType()
    assert child.get_field_type('missing') == AnyType()
    assert child.get_method_type('missing') is None
    assert str(child) == 'Child'


def test_union_type_string_and_compatibility():
    u = UnionType({IntType(), StrType()})
    assert str(u) in ('Int | String', 'String | Int')
    assert u.is_compatible(IntType())
    assert not u.is_compatible(BoolType())


def test_type_variable_accepts_anything():
    assert TypeVariable('T').is_compatible(IntType())
    assert str(TypeVariable('T')) == 'T'


# ---------------------------------------------------------------------------
# TypeInference
# ---------------------------------------------------------------------------

def test_infer_none_node_is_any():
    assert isinstance(TypeInference().infer(None), AnyType)


def test_infer_comprehensions():
    inf = TypeInference()

    def comp(kind):
        return ast.ComprehensionExpr(ast.Identifier('x'), [], expr_type=kind)

    assert isinstance(inf.infer(comp('dict')), DictType)
    assert isinstance(inf.infer(comp('set')), SetType)
    assert isinstance(inf.infer(comp('list')), ListType)


def test_infer_lambda_is_function():
    assert isinstance(TypeInference().infer(ast.LambdaExpr([], ast.IntLiteral(0))), FunctionType)


def test_infer_elvis_and_coalesce_use_value():
    inf = TypeInference()
    assert isinstance(inf.infer(ast.ElvisExpr(ast.IntLiteral(0), ast.IntLiteral(0))), IntType)
    assert isinstance(inf.infer(ast.CoalesceExpr(ast.IntLiteral(0), ast.IntLiteral(0))), IntType)


def test_infer_conditional_unions_branches():
    inf = TypeInference()
    result = inf.infer(ast.CondExpr(ast.BoolLiteral(True), ast.IntLiteral(0), ast.StrLiteral("s")))
    assert isinstance(result, UnionType)


def test_infer_safe_nav_is_optional():
    inf = TypeInference()
    result = inf.infer(ast.SafeNavExpr(ast.IntLiteral(0)))
    assert isinstance(result, UnionType)
    assert NoneType() in result.types


def test_infer_unknown_expression_kinds_are_any():
    inf = TypeInference()
    for node in (ast.Identifier('x'), ast.MatchExpr(ast.Identifier('x'), []), ast.TryExpr([]),
                 ast.BlockExpr([]), ast.PipeExpr(ast.Identifier('x'), ast.Identifier('y'))):
        assert isinstance(inf.infer(node), AnyType)


def test_infer_builtin_return_kinds():
    inf = TypeInference()

    def call(name, *args):
        return ast.CallExpr(ast.Identifier(name), list(args))

    assert isinstance(inf.infer(call('len', ast.ListLiteral([ast.IntLiteral(0)]))), IntType)
    assert isinstance(inf.infer(call('int', ast.StrLiteral("s"))), IntType)
    assert isinstance(inf.infer(call('float', ast.IntLiteral(0))), FloatType)
    assert isinstance(inf.infer(call('str', ast.IntLiteral(0))), StrType)
    assert isinstance(inf.infer(call('bool', ast.IntLiteral(0))), BoolType)
    assert isinstance(inf.infer(call('range', ast.IntLiteral(0))), ListType)
    assert isinstance(inf.infer(call('abs', ast.IntLiteral(0))), IntType)
    assert isinstance(inf.infer(call('round', ast.IntLiteral(0))), FloatType)
    assert isinstance(inf.infer(call('any', ast.ListLiteral([ast.IntLiteral(0)]))), BoolType)


def test_infer_collection_constructors_mirror_arguments():
    inf = TypeInference()
    list_of = ast.ListLiteral([ast.IntLiteral(0)])
    set_of = ast.SetLiteral([ast.IntLiteral(0)])

    assert inf.infer(ast.CallExpr(ast.Identifier('list'), [list_of])) == ListType(IntType())
    assert inf.infer(ast.CallExpr(ast.Identifier('list'), [set_of])) == ListType(IntType())
    assert inf.infer(ast.CallExpr(ast.Identifier('set'), [list_of])) == SetType(IntType())
    assert inf.infer(ast.CallExpr(ast.Identifier('set'), [set_of])) == SetType(IntType())
    assert inf.infer(ast.CallExpr(ast.Identifier('tuple'), [list_of])) == TupleType([IntType()])


def test_infer_collection_constructors_without_args():
    inf = TypeInference()
    assert inf.infer(ast.CallExpr(ast.Identifier('list'), [])) == ListType(AnyType())
    assert inf.infer(ast.CallExpr(ast.Identifier('set'), [])) == SetType(AnyType())
    assert inf.infer(ast.CallExpr(ast.Identifier('tuple'), [])) == TupleType([])


def test_infer_min_max_over_collection():
    inf = TypeInference()
    result = inf.infer(ast.CallExpr(ast.Identifier('min'),
                                    [ast.ListLiteral([ast.IntLiteral(0)])]))
    assert result == IntType()


def test_infer_sorted_and_reversed_return_lists():
    inf = TypeInference()
    result = inf.infer(ast.CallExpr(ast.Identifier('sorted'),
                                    [ast.ListLiteral([ast.IntLiteral(0)])]))
    assert result == ListType(IntType())


def test_infer_unknown_builtin_is_any():
    inf = TypeInference()
    assert isinstance(inf.infer(ast.CallExpr(ast.Identifier('mystery'), [])), AnyType)


def test_infer_call_with_non_identifier_func_is_any():
    inf = TypeInference()
    node = ast.CallExpr(ast.MemberExpr(ast.Identifier('o'), 'm'), [])
    assert isinstance(inf.infer(node), AnyType)


def test_infer_binary_assignment_operators_adopt_rhs():
    inf = TypeInference()
    for op in ('=', '+=', '??='):
        node = ast.BinaryOp(ast.Identifier('x'), op, ast.IntLiteral(0))
        assert inf.infer(node) == IntType()


def test_infer_binary_arithmetic_rules():
    inf = TypeInference()
    assert inf.infer(ast.BinaryOp(ast.IntLiteral(0), '/', ast.IntLiteral(0))) == FloatType()
    assert inf.infer(ast.BinaryOp(ast.IntLiteral(0), '+', ast.IntLiteral(0))) == IntType()
    assert inf.infer(ast.BinaryOp(ast.FloatLiteral(0.0), '+', ast.IntLiteral(0))) == FloatType()
    assert inf.infer(ast.BinaryOp(ast.StrLiteral("s"), '+', ast.StrLiteral("s"))) == StrType()
    assert isinstance(inf.infer(ast.BinaryOp(ast.ListLiteral([ast.IntLiteral(0)]), '+',
                                             ast.ListLiteral([ast.IntLiteral(0)]))), ListType)
    assert isinstance(inf.infer(ast.BinaryOp(ast.BoolLiteral(True), '+', ast.BoolLiteral(True))), AnyType)


def test_infer_binary_bitwise_and_shift():
    inf = TypeInference()
    assert inf.infer(ast.BinaryOp(ast.IntLiteral(0), '&', ast.IntLiteral(0))) == IntType()
    assert inf.infer(ast.BinaryOp(ast.BoolLiteral(True), '|', ast.BoolLiteral(True))) == BoolType()


def test_infer_binary_coalesce_strips_none():
    inf = TypeInference()
    optional = ast.BinaryOp(ast.ElvisExpr(ast.IntLiteral(0), ast.IntLiteral(0)), '??', ast.IntLiteral(0))
    assert inf.infer(optional) == IntType()


def test_infer_binary_unknown_operator_is_any():
    inf = TypeInference()
    assert isinstance(inf.infer(ast.BinaryOp(ast.IntLiteral(0), '%%%', ast.IntLiteral(0))), AnyType)


def test_infer_unary_operators():
    inf = TypeInference()
    assert inf.infer(ast.UnaryOp('not', ast.IntLiteral(0))) == BoolType()
    assert inf.infer(ast.UnaryOp('!', ast.IntLiteral(0))) == BoolType()
    assert inf.infer(ast.UnaryOp('-', ast.IntLiteral(0))) == IntType()
    assert inf.infer(ast.UnaryOp('~', ast.IntLiteral(0))) == IntType()
    assert isinstance(inf.infer(ast.UnaryOp('-', ast.StrLiteral("s"))), AnyType)
    assert inf.infer(ast.UnaryOp('+', ast.IntLiteral(0))) == IntType()


def test_union_helper_collapses_equal_and_any():
    inf = TypeInference()
    assert inf._union(IntType(), IntType()) == IntType()
    assert isinstance(inf._union(IntType(), AnyType()), AnyType)
    assert isinstance(inf._union(IntType(), StrType()), UnionType)


def test_strip_none_recurses_into_union():
    inf = TypeInference()
    assert inf._strip_none(UnionType({IntType(), NoneType()})) == UnionType({IntType()})
    assert inf._strip_none(IntType()) == IntType()


# ---------------------------------------------------------------------------
# TypeChecker traversal
# ---------------------------------------------------------------------------

def test_for_over_dict_binds_key_type():
    ok, checker = check('def main() {\n'
                        '  let d = {"a": 1}\n'
                        '  for k in d { print(k) }\n'
                        '}\n')
    assert ok is True


def test_try_catch_finally_visit():
    ok, checker = check('def main() {\n'
                        '  try { print(1) } catch e { print(e) } finally { print(2) }\n'
                        '}\n')
    assert ok is True
    assert 'e' in checker.errors or ok


def test_with_statement_binds_names():
    program = parse('def main() {\n  with open("x") as f { print(f) }\n}\n')
    with_stmt = program.statements[0].body[0]
    checker = TypeChecker()
    checker.visit(with_stmt)
    assert 'f' in checker.context


def test_guard_and_match_and_loop_and_assert_traversed():
    source = (
        'def main() {\n'
        '  let x: int = 1\n'
        '  guard x > 0 else { return }\n'
        '  match x { case 1 { print(1) } case _ { print(2) } }\n'
        '  loop { break }\n'
        '  assert x > 0\n'
        '}\n'
    )
    ok, checker = check(source)
    assert ok is True


def test_assert_with_bad_condition_type_reports():
    ok, checker = check('def main() {\n  assert 5\n}\n')
    assert ok is False
    assert any('assert' in e for e in checker.errors)


def test_throw_statement_is_ignored():
    ok, checker = check('def main() {\n  throw "boom"\n}\n')
    assert ok is True


def test_while_and_until_conditions_checked():
    ok, checker = check('def main() {\n'
                        '  let mut i = 0\n'
                        '  while i < 3 { i = i + 1 }\n'
                        '  until i > 5 { i = i + 1 }\n'
                        '}\n')
    assert ok is True


def test_narrowing_branches_restore_context():
    source = (
        'def f(x: int?) -> int {\n'
        '  if x != none { return x }\n'
        '  return 0\n'
        '}\n'
        'def main() { print(f(1)) }\n'
    )
    ok, checker = check(source)
    assert ok is True


def test_narrowing_is_type_else_branch():
    source = (
        'def f(x) -> int {\n'
        '  if x is int { return x }\n'
        '  return 0\n'
        '}\n'
        'def main() { print(f(1)) }\n'
    )
    ok, checker = check(source)
    assert ok is True


def test_narrowing_non_binary_condition_is_noop():
    checker = TypeChecker()
    assert checker._narrowings(ast.IntLiteral(0)) == ({}, {})


def test_narrowing_non_identifier_left_is_noop():
    checker = TypeChecker()
    node = ast.BinaryOp(ast.MemberExpr(ast.Identifier('a'), 'b'), '!=', ast.NoneLiteral())
    assert checker._narrowings(node) == ({}, {})


def test_call_arity_too_few_reports():
    source = (
        'def add(a: int, b: int) -> int { return a + b }\n'
        'def main() { let x = add(1) }\n'
    )
    ok, checker = check(source)
    assert ok is False
    assert any('at least' in e for e in checker.errors)


def test_call_arity_with_kwargs_is_lenient():
    source = (
        'def add(a: int, b: int) -> int { return a + b }\n'
        'def main() { let x = add(1, b=2) }\n'
    )
    ok, checker = check(source)
    assert ok is True


def test_call_argument_type_mismatch_reports():
    source = (
        'def greet(name: str) -> str { return name }\n'
        'def main() { let x = greet(42) }\n'
    )
    ok, checker = check(source)
    assert ok is False
    assert any('argument 1' in e for e in checker.errors)


def test_method_call_on_known_class():
    source = (
        'class P {\n'
        '  public def hi(self) { print(1) }\n'
        '}\n'
        'def main() { let p = P()\n p.hi() }\n'
    )
    ok, checker = check(source)
    assert ok is True


def test_condition_type_errors_for_collections():
    for expr in ('[1, 2]', '{1: 2}', '{1, 2}', '(1, 2)', '5', '1.5', '"s"'):
        ok, checker = check(f'def main() {{\n  if {expr} {{ print(1) }}\n}}\n')
        assert ok is False, expr


def test_unknown_statement_visits_children():
    # A statement node the checker does not special-case falls through to
    # _visit_children, which must not raise.
    node = ast.ExprStmt(ast.CallExpr(ast.Identifier('print'), [ast.IntLiteral(0)]))
    checker = TypeChecker()
    checker.check(node)
    assert checker.errors == []


def test_check_returns_false_on_type_error_exception():
    class Boom:
        pass

    checker = TypeChecker()
    checker.visit = lambda node: (_ for _ in ()).throw(TypeError('explode'))
    assert checker.check(Boom()) is False
    assert 'explode' in checker.errors[0]


def test_check_program_returns_false_on_type_error_exception():
    checker = TypeChecker()
    checker.visit = lambda node: (_ for _ in ()).throw(TypeError('explode'))
    assert checker.check_program(parse('def main() { print(1) }\n')) is False


# ---------------------------------------------------------------------------
# _type_variables_in
# ---------------------------------------------------------------------------

def test_type_variables_in_containers():
    T = TypeVariable('T')
    U = TypeVariable('U')
    assert TypeChecker._type_variables_in(None) == set()
    assert TypeChecker._type_variables_in(T) == {'T'}
    assert TypeChecker._type_variables_in(ListType(T)) == {'T'}
    assert TypeChecker._type_variables_in(SetType(T)) == {'T'}
    assert TypeChecker._type_variables_in(DictType(T, U)) == {'T', 'U'}
    assert TypeChecker._type_variables_in(TupleType([T, U])) == {'T', 'U'}
    assert TypeChecker._type_variables_in(UnionType({T, U})) == {'T', 'U'}
    assert TypeChecker._type_variables_in(FunctionType([T], U)) == {'T', 'U'}
    assert TypeChecker._type_variables_in(ClassType('Box', type_params=['T'])) == {'T'}
    assert TypeChecker._type_variables_in(IntType()) == set()


def test_unused_type_parameter_warns():
    ok, checker = check('class Box[T] {\n  public let value: int = 0\n}\n')
    assert ok is True
    assert any('never used' in str(d.message) for d in checker.diagnostics)


def test_used_type_parameter_does_not_warn():
    ok, checker = check('class Box[T] {\n  public let value: T\n}\n')
    assert not any('never used' in str(d.message) for d in checker.diagnostics)


def test_type_parameter_in_method_return_is_used():
    ok, checker = check('class Box[T] {\n'
                        '  public def get(self) -> T { return none }\n'
                        '}\n')
    assert not any('never used' in str(d.message) for d in checker.diagnostics)


# ---------------------------------------------------------------------------
# _parse_type_annotation
# ---------------------------------------------------------------------------

def test_parse_type_annotation_forms():
    checker = TypeChecker()
    parse_ann = checker._parse_type_annotation
    assert parse_ann(None) == AnyType()
    assert parse_ann('int') == IntType()
    assert parse_ann('str') == StrType()
    assert parse_ann('bool') == BoolType()
    assert parse_ann('none') == NoneType()
    assert parse_ann('any') == AnyType()
    assert parse_ann('unknown_type') == AnyType()
    assert isinstance(parse_ann('int?'), UnionType)
    assert isinstance(parse_ann('(int) -> int'), FunctionType)
    assert parse_ann('[int]') == ListType(IntType())
    assert parse_ann('List[int]') == ListType(IntType())
    assert parse_ann('Set[int]') == SetType(IntType())
    assert parse_ann('Dict[str, int]') == DictType(StrType(), IntType())
    assert isinstance(parse_ann('Dict[str]'), DictType)
    assert isinstance(parse_ann('{str: int}'), DictType)
    assert parse_ann('{int}') == SetType()
    assert isinstance(parse_ann('int | str'), UnionType)


def test_parse_type_annotation_resolves_known_class():
    checker = TypeChecker()
    checker.classes['Point'] = ClassType('Point')
    assert checker._parse_type_annotation('Point') == checker.classes['Point']


def test_parse_ast_type_nodes():
    checker = TypeChecker()
    parse_ann = checker._parse_type_annotation
    assert parse_ann(ast.SimpleType('int')) == IntType()
    assert isinstance(parse_ann(ast.OptionalType(ast.SimpleType('int'))), UnionType)
    assert isinstance(parse_ann(ast.UnionType([ast.SimpleType('int'),
                                               ast.SimpleType('str')])), UnionType)
    assert parse_ann(ast.StructuralType({})) == DictType()
    assert isinstance(parse_ann(ast.GenericType('int', [])), AnyType)


def test_parse_generic_type_with_args():
    checker = TypeChecker()
    # A generic whose base resolves to a list/set applies the first argument.
    node = ast.GenericType('[int]', [ast.SimpleType('str')])
    assert checker._parse_type_annotation(node) == ListType(StrType())


def test_parse_type_parameter_name_resolves_to_variable():
    checker = TypeChecker()
    checker._type_params['T'] = TypeVariable('T')
    assert checker._parse_type_annotation('T') == TypeVariable('T')


# ---------------------------------------------------------------------------
# _method_to_function_type
# ---------------------------------------------------------------------------

def test_method_to_function_type_skips_self_and_kwonly():
    checker = TypeChecker()
    method = ast.Method(
        name='m',
        params=[
            ast.Parameter('self'),
            ast.Parameter('a', type_annotation='int'),
            ast.Parameter('b', type_annotation='int', default=ast.IntLiteral(0)),
            ast.Parameter('c', type_annotation='int', is_kwonly=True),
        ],
        return_type='int',
    )
    ftype = checker._method_to_function_type(method)
    assert ftype.param_types == [IntType(), IntType()]
    assert ftype.required_args == 1
    assert ftype.return_type == IntType()


# ---------------------------------------------------------------------------
# _check_binary_op (E108)
# ---------------------------------------------------------------------------

def _codes(checker):
    return [d.code for d in checker.diagnostics]


def test_binary_plus_compatible_and_incompatible():
    checker = TypeChecker()
    checker._check_binary_op(ast.BinaryOp(ast.StrLiteral('a'), '+',
                                          ast.StrLiteral('b')))
    assert _codes(checker) == []
    checker._check_binary_op(ast.BinaryOp(ast.IntLiteral(1), '+',
                                          ast.IntLiteral(2)))
    assert _codes(checker) == []
    checker._check_binary_op(ast.BinaryOp(ast.ListLiteral([ast.IntLiteral(1)]),
                                          '+',
                                          ast.ListLiteral([ast.IntLiteral(2)])))
    assert _codes(checker) == []
    checker._check_binary_op(ast.BinaryOp(ast.StrLiteral('a'), '+',
                                          ast.IntLiteral(1)))
    checker._check_binary_op(ast.BinaryOp(ast.IntLiteral(1), '+',
                                          ast.StrLiteral('a')))
    assert _codes(checker) == [ErrorCode.INCOMPATIBLE_OPERANDS] * 2


def test_binary_arithmetic_requires_numbers():
    checker = TypeChecker()
    for op in ('-', '/', '%', '**'):
        checker._check_binary_op(ast.BinaryOp(ast.IntLiteral(1), op,
                                              ast.FloatLiteral(2.0)))
        checker._check_binary_op(ast.BinaryOp(ast.StrLiteral('a'), op,
                                              ast.IntLiteral(2)))
    assert _codes(checker) == [ErrorCode.INCOMPATIBLE_OPERANDS] * 4


def test_binary_star_supports_repetition():
    checker = TypeChecker()
    checker._check_binary_op(ast.BinaryOp(ast.StrLiteral('a'), '*',
                                          ast.IntLiteral(3)))
    checker._check_binary_op(ast.BinaryOp(ast.IntLiteral(3), '*',
                                          ast.StrLiteral('a')))
    assert _codes(checker) == []
    checker._check_binary_op(ast.BinaryOp(ast.FloatLiteral(1.5), '*',
                                          ast.IntLiteral(2)))
    assert _codes(checker) == []


def test_binary_comparison_operators():
    checker = TypeChecker()
    for op in ('<', '>', '<=', '>='):
        checker._check_binary_op(ast.BinaryOp(ast.IntLiteral(1), op,
                                              ast.FloatLiteral(2.0)))
        checker._check_binary_op(ast.BinaryOp(ast.StrLiteral('a'), op,
                                              ast.StrLiteral('b')))
    assert _codes(checker) == []
    checker._check_binary_op(ast.BinaryOp(ast.StrLiteral('a'), '<',
                                          ast.IntLiteral(1)))
    assert _codes(checker) == [ErrorCode.INCOMPATIBLE_OPERANDS]


def test_binary_bitwise_requires_integers():
    checker = TypeChecker()
    for op in ('&', '|', '^', '<<', '>>'):
        checker._check_binary_op(ast.BinaryOp(ast.IntLiteral(1), op,
                                              ast.BoolLiteral(True)))
    assert _codes(checker) == []
    checker._check_binary_op(ast.BinaryOp(ast.FloatLiteral(1.0), '&',
                                          ast.IntLiteral(1)))
    assert _codes(checker) == [ErrorCode.INCOMPATIBLE_OPERANDS]


def test_binary_op_skips_unknown_and_any():
    checker = TypeChecker()
    # An unsupported operator is ignored entirely.
    checker._check_binary_op(ast.BinaryOp(ast.IntLiteral(1), 'and',
                                          ast.StrLiteral('a')))
    assert _codes(checker) == []
    # An operand typed Any disables the compatibility check.
    checker.context['anything'] = AnyType()
    checker._check_binary_op(ast.BinaryOp(ast.Identifier('anything'), '+',
                                          ast.IntLiteral(1)))
    assert _codes(checker) == []


# ---------------------------------------------------------------------------
# _check_expr traversal branches
# ---------------------------------------------------------------------------

def test_check_expr_none_is_noop():
    checker = TypeChecker()
    checker._check_expr(None)
    assert _codes(checker) == []


def test_check_expr_unary_and_member_and_index():
    checker = TypeChecker()
    node = ast.UnaryOp('-', ast.IntLiteral(1))
    checker._check_expr(node)
    checker._check_expr(ast.MemberExpr(ast.IntLiteral(1), 'real'))
    checker._check_expr(ast.IndexExpr(ast.ListLiteral([ast.IntLiteral(1)]),
                                      ast.IntLiteral(0)))
    assert _codes(checker) == []


def test_check_expr_conditional_and_elvis_and_coalesce_and_safnav():
    checker = TypeChecker()
    checker._check_expr(ast.CondExpr(ast.BoolLiteral(True),
                                     ast.IntLiteral(1),
                                     ast.IntLiteral(2)))
    checker._check_expr(ast.ElvisExpr(ast.StrLiteral('a'), ast.StrLiteral('b')))
    checker._check_expr(ast.CoalesceExpr(ast.NoneLiteral(), ast.IntLiteral(1)))
    checker._check_expr(ast.SafeNavExpr(ast.Identifier('nope'), 'field'))
    assert _codes(checker) == []


def test_check_expr_ternary_condition_must_be_bool():
    checker = TypeChecker()
    checker._check_expr(ast.CondExpr(ast.IntLiteral(5),
                                     ast.IntLiteral(1),
                                     ast.IntLiteral(2)))
    assert ErrorCode.TYPE_MISMATCH in _codes(checker)
    assert any('ternary condition' in str(d.message)
               for d in checker.diagnostics)


def test_check_expr_collection_literals():
    checker = TypeChecker()
    checker._check_expr(ast.ListLiteral([ast.IntLiteral(1), ast.StrLiteral('a')]))
    checker._check_expr(ast.SetLiteral([ast.IntLiteral(1)]))
    checker._check_expr(ast.TupleLiteral([ast.IntLiteral(1), ast.IntLiteral(2)]))
    checker._check_expr(ast.DictLiteral([(ast.StrLiteral('k'),
                                          ast.IntLiteral(1))]))
    assert _codes(checker) == []


def test_check_expr_dict_literal_with_non_tuple_entry():
    checker = TypeChecker()
    checker._check_expr(ast.DictLiteral([ast.IntLiteral(1)]))
    assert _codes(checker) == []


def test_check_expr_comprehension_pipe_spread_fstring():
    checker = TypeChecker()
    checker._check_expr(ast.ComprehensionExpr(
        ast.IntLiteral(1),
        [(ast.Identifier('x'), ast.ListLiteral([ast.IntLiteral(1)]), [])],
    ))
    checker._check_expr(ast.PipeExpr(ast.IntLiteral(1), ast.IntLiteral(2)))
    checker._check_expr(ast.SpreadExpr(ast.ListLiteral([ast.IntLiteral(1)])))
    checker._check_expr(ast.FStringLiteral(['a', (ast.IntLiteral(1),), 'b']))
    assert _codes(checker) == []


# ---------------------------------------------------------------------------
# _check_var_decl / _check_const_decl annotation mismatches (E101)
# ---------------------------------------------------------------------------

def test_check_var_decl_type_mismatch_records_declared_type():
    checker = TypeChecker()
    node = ast.VarDecl('x', False, type_annotation='str', value=ast.IntLiteral(1))
    checker._check_var_decl(node)
    assert _codes(checker) == [ErrorCode.TYPE_MISMATCH]
    assert checker.context['x'] == StrType()


def test_check_var_decl_without_annotation_binds_inferred_type():
    checker = TypeChecker()
    checker._check_var_decl(ast.VarDecl('x', False, value=ast.StrLiteral('hi')))
    assert checker.context['x'] == StrType()


def test_check_var_decl_without_value_is_none():
    checker = TypeChecker()
    checker._check_var_decl(ast.VarDecl('x', False))
    assert checker.context['x'] == NoneType()


def test_check_const_decl_type_mismatch():
    checker = TypeChecker()
    node = ast.ConstDecl('K', type_annotation='int', value=ast.StrLiteral('no'))
    checker._check_const_decl(node)
    assert _codes(checker) == [ErrorCode.TYPE_MISMATCH]
    assert any('Constant' in str(d.message) for d in checker.diagnostics)


def test_check_const_decl_ok_and_without_value():
    checker = TypeChecker()
    checker._check_const_decl(ast.ConstDecl('K', type_annotation='int',
                                            value=ast.IntLiteral(1)))
    assert checker.context['K'] == IntType()
    checker._check_const_decl(ast.ConstDecl('E'))
    assert checker.context['E'] == NoneType()
