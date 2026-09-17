"""Deep coverage for ``StatementTransformer`` in ``aura.transpiler``.

Targets the less-travelled branches: dict destructuring, module nesting,
pattern rendering, global/nonlocal declarations, import forms and the
declaration-name helpers.
"""
import pytest
from aura_test_helpers import run_aura, transpile

from aura.transpiler import ast
from aura.transpiler.transformers.statements import StatementTransformer


def stmt():
    return StatementTransformer()


# ---------------------------------------------------------------------------
# transform dispatch and helpers
# ---------------------------------------------------------------------------

def test_transform_none_returns_empty_string():
    assert stmt().transform(None) == ''


def test_transform_unknown_node_raises():
    class Weird(ast.Stmt):
        pass

    with pytest.raises(NotImplementedError):
        stmt().transform(Weird())


def test_safe_binding_plain_and_keyword():
    sb = StatementTransformer._safe_binding
    assert sb('x') == 'x'
    assert sb('raise') == 'raise_'
    assert sb('(a, raise)') == '(a, raise_)'
    assert sb('[a, *rest]') == '[a, *rest]'
    assert sb(42) == 42


def test_declared_names_forms():
    dn = StatementTransformer._declared_names
    assert dn('') == []
    assert dn('x') == ['x']
    assert dn('(a, b)') == ['a', 'b']
    assert dn('[a, *rest]') == ['a', 'rest']
    assert dn('(1, 2)') == []


def test_indent_levels():
    s = stmt()
    assert s._indent() == ''
    s.indent_level = 2
    assert s._indent() == '        '


# ---------------------------------------------------------------------------
# ConstDecl / VarDecl
# ---------------------------------------------------------------------------

def test_const_decl_emits_comment():
    assert transpile('const K = 1\n').strip() == 'K = 1  # const'


def test_var_decl_without_value_is_none():
    assert transpile('let x\n').strip() == 'x = None'


def test_var_decl_tuple_target_is_safe():
    code = transpile('let (a, b) = (1, 2)\n')
    assert 'a' in code and 'b' in code


# ---------------------------------------------------------------------------
# Dict destructuring
# ---------------------------------------------------------------------------

def test_dict_destructure_basic():
    code = transpile('let {name, age} = {"name": "A", "age": 3}\n')
    assert '_aura_destructure_' in code
    assert "['name']" in code
    assert "['age']" in code


def test_dict_destructure_with_alias():
    code = transpile('let {name: n} = {"name": "A"}\n')
    assert "n = " in code
    assert "['name']" in code


def test_dict_destructure_requires_value():
    s = stmt()
    node = ast.VarDecl('{a}', False, value=None)
    with pytest.raises(SyntaxError):
        s.transform(node)


def test_dict_destructure_rejects_empty_pattern():
    s = stmt()
    node = ast.VarDecl('{}', False, value=ast.Identifier('src'))
    with pytest.raises(SyntaxError):
        s.transform(node)


def test_dict_destructure_rejects_invalid_target():
    s = stmt()
    node = ast.VarDecl('{a: 1}', False, value=ast.Identifier('src'))
    with pytest.raises(SyntaxError):
        s.transform(node)


def test_dict_pattern_fields_parsing():
    fields = StatementTransformer._dict_pattern_fields('{a, "b": c}')
    assert fields == [('a', 'a'), ('b', 'c')]


# ---------------------------------------------------------------------------
# Modules
# ---------------------------------------------------------------------------

def test_nested_module_transpiles_to_nested_classes():
    code = transpile('module Outer.Inner { def f() { return 3 } }\n')
    assert 'class Outer:' in code
    assert 'class Inner:' in code


def test_module_members_become_static():
    code = transpile('module M { def hi() { return 7 } }\n')
    assert '@staticmethod' in code


def test_empty_module_gets_pass():
    code = transpile('module Empty { }\n')
    assert 'class Empty:' in code
    assert 'pass' in code


# ---------------------------------------------------------------------------
# Match statement and patterns
# ---------------------------------------------------------------------------

def test_match_with_guard_and_or_pattern():
    code = transpile(
        'let n = 2\n'
        'match n {\n'
        '  case 1 | 2 if n > 0 { print("yes") }\n'
        '  case _ { print("no") }\n'
        '}\n'
    )
    assert 'case (1 | 2) if (n > 0):' in code


def test_match_list_pattern():
    code = transpile(
        'let pair = [1, 2]\n'
        'match pair {\n'
        '  case [a, b] { print(a + b) }\n'
        '}\n'
    )
    assert 'case [a, b]:' in code


def test_match_dict_pattern():
    code = transpile(
        'let d = {"x": 1}\n'
        'match d {\n'
        '  case {"x": v} { print(v) }\n'
        '}\n'
    )
    assert "case AuraDict({'x': v}):" in code


def test_match_constructor_pattern():
    s = stmt()
    node = ast.MatchStmt(ast.Identifier('p'), [
        ast.MatchCase(ast.ConstructorPattern('Point', [
            ast.IdentifierPattern('x'), ast.IdentifierPattern('y')]), None, []),
    ])
    code = s.transform(node)
    assert 'case Point(x, y):' in code


def test_match_pattern_fallbacks():
    t = stmt()
    assert t._transform_pattern(ast.WildcardPattern()) == '_'
    assert t._transform_pattern(ast.IdentifierPattern('x')) == 'x'
    # Expression nodes are not patterns and fall back to a wildcard.
    assert t._transform_pattern(ast.IntLiteral(3)) == '_'
    assert t._transform_pattern(ast.StrLiteral('s')) == '_'
    assert t._transform_pattern(object()) == '_'


def test_pattern_literal_raw_values():
    t = stmt()
    assert t._transform_pattern(ast.LiteralPattern('raw')) == "'raw'"
    assert t._transform_pattern(ast.LiteralPattern(True)) == 'True'
    assert t._transform_pattern(ast.LiteralPattern(None)) == 'None'
    assert t._transform_pattern(ast.LiteralPattern(5)) == '5'


# ---------------------------------------------------------------------------
# Imports
# ---------------------------------------------------------------------------

def test_import_plain_and_alias():
    assert transpile('import os\n').strip() == 'import os'
    assert transpile('import os as o\n').strip() == 'import os as o'


def test_from_import_variants():
    code = transpile('from stdlib.math import sqrt\n')
    assert code.strip() == 'from stdlib.math import sqrt'
    s = stmt()
    node = ast.FromImport('m', [('*', None)])
    assert s.transform(node) == 'from m import *'
    node = ast.FromImport('m', [('a', 'b'), ('c', None)])
    assert s.transform(node) == 'from m import a as b, c'


def test_import_with_selection_and_alias():
    s = stmt()
    node = ast.ImportStmt('a.b', items=[('x', None)], alias='c')
    code = s.transform(node)
    assert 'import a.b as c' in code
    assert 'from a.b import x' in code


def test_import_with_selection():
    s = stmt()
    node = ast.ImportStmt('a.b', items=[('x', None), ('y', 'z')])
    assert s.transform(node) == 'from a.b import x, y as z'


def test_import_multiple_modules():
    s = stmt()
    node = ast.ImportStmt('', modules=[('a', None), ('b', 'c')])
    assert s.transform(node) == 'import a, b as c'


# ---------------------------------------------------------------------------
# Decorators, static/volatile
# ---------------------------------------------------------------------------

def test_static_function_decorator():
    code = transpile('class C { public static def f() { return 1 } }\n')
    assert '@staticmethod' in code


def test_volatile_function_comment():
    s = stmt()
    func = ast.FunctionDecl('f', [], None, [], is_volatile=True)
    code = s.transform(func)
    assert '# volatile' in code


def test_decorator_with_args_and_kwargs():
    s = stmt()
    dec = ast.Decorator('route', [ast.StrLiteral('/x')], {'method': ast.StrLiteral('get')})
    line = s._decorator_line(dec)
    assert line == "@route('/x', method='get')\n"


def test_decorator_without_args():
    s = stmt()
    dec = ast.Decorator('cached', [], {})
    assert s._decorator_line(dec) == '@cached\n'


# ---------------------------------------------------------------------------
# global / nonlocal declarations
# ---------------------------------------------------------------------------

def test_global_declaration_for_module_binding():
    code = transpile('let mut total = 0\ndef inc() { total += 1 }\n')
    assert 'global total' in code


def test_global_decl_line_empty_when_no_assignments():
    s = stmt()
    assert s._global_decl_line() == ''
    s._global_assignments.append(set())
    assert s._global_decl_line() == ''


def test_record_global_assignment_ignores_non_module_names():
    s = stmt()
    s._global_assignments.append(set())
    s._record_global_assignment('local')
    assert s._global_assignments[-1] == set()


def test_nonlocal_declaration_for_nested_function():
    code = transpile(
        'def outer() {\n'
        '  let mut x = 0\n'
        '  def inner() { x += 1 }\n'
        '  inner()\n'
        '  return x\n'
        '}\n'
    )
    assert 'nonlocal x' in code


def test_nonlocal_decl_line_empty_without_capture():
    s = stmt()
    node = ast.FunctionDecl('inner', [], None, [])
    assert s._nonlocal_decl_line(node, set()) == ''


# ---------------------------------------------------------------------------
# Function params
# ---------------------------------------------------------------------------

def test_function_params_kwonly_and_variadic():
    s = stmt()
    func = ast.FunctionDecl('f', [
        ast.Parameter('a'),
        ast.Parameter('kw', is_kwonly=True),
    ], None, [])
    code = s.transform(func)
    assert '**kw' in code


def test_function_without_body_gets_pass():
    s = stmt()
    code = s.transform(ast.FunctionDecl('f', [], None, None))
    assert code.endswith('): pass')


def test_constructor_is_init():
    code = transpile(
        'class C {\n'
        '  let x: int = 0\n'
        '  public def new(x: int) { self.x = x }\n'
        '}\n'
    )
    assert 'def __init__' in code


# ---------------------------------------------------------------------------
# End-to-end execution
# ---------------------------------------------------------------------------

def test_module_execution_roundtrip():
    out = run_aura('module M { def hi() { return 7 } }\nprint(M.hi())')
    assert out == '7\n'


def test_dict_destructure_execution():
    out = run_aura('let {name, age} = {"name": "A", "age": 3}\nprint(name)\nprint(age)')
    assert out == 'A\n3\n'


def test_match_guard_execution():
    out = run_aura(
        'let n = 5\n'
        'let msg = match n {\n'
        '  case x if x > 3 { "big" }\n'
        '  case _ { "small" }\n'
        '}\n'
        'print(msg)'
    )
    assert out == 'big\n'
