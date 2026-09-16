"""Complete AST node definitions for Aura language based on docs."""
from dataclasses import dataclass


@dataclass
class SourceLocation:
    """Track source code location for error reporting."""
    filename: str = ""
    line: int = 0
    column: int = 0
    length: int = 0

    def __str__(self):
        return f"{self.filename}:{self.line}:{self.column}"

class Node:
    """Base AST node with optional source location."""
    def __init__(self):
        self.location: SourceLocation | None = None

    def with_location(self, location: SourceLocation) -> 'Node':
        """Attach source location to node."""
        self.location = location
        return self


class Stmt(Node):
    pass

# ============================================================================
# Top-level structures
# ============================================================================

class Program(Node):
    def __init__(self, statements):
        self.statements = statements

class Module(Node):
    def __init__(self, name, members):
        self.name = name
        self.members = members

# ============================================================================
# Declarations
# ============================================================================

class VarDecl(Stmt):
    def __init__(self, name, mutable, type_annotation=None, value=None, visibility='public', is_static=False, is_volatile=False):
        self.name = name
        self.mutable = mutable
        self.type_annotation = type_annotation
        self.value = value
        self.visibility = visibility
        self.is_static = is_static
        self.is_volatile = is_volatile

class ConstDecl(Stmt):
    def __init__(self, name, type_annotation=None, value=None):
        self.name = name
        self.type_annotation = type_annotation
        self.value = value

class FunctionDecl(Stmt):
    def __init__(self, name, params, return_type=None, body=None,
                 is_async=False, type_params=None, decorators=None, visibility='public', is_static=False, is_volatile=False):
        self.name = name
        self.params = params or []
        self.return_type = return_type
        self.body = body
        self.is_async = is_async
        self.type_params = type_params or []
        self.decorators = decorators or []
        self.visibility = visibility
        self.is_static = is_static
        self.is_volatile = is_volatile

class ClassDecl(Stmt):
    def __init__(self, name, body, base_class=None, type_params=None, decorators=None, visibility='public', is_static=False, is_volatile=False):
        self.name = name
        self.body = body
        self.base_class = base_class
        self.type_params = type_params or []
        self.decorators = decorators or []
        self.visibility = visibility
        self.is_static = is_static
        self.is_volatile = is_volatile

class TraitDecl(Stmt):
    def __init__(self, name, members, type_params=None, visibility='public'):
        self.name = name
        self.members = members
        self.type_params = type_params or []
        self.visibility = visibility

class TypeDecl(Stmt):
    def __init__(self, name, type_expr, type_params=None):
        self.name = name
        self.type_expr = type_expr
        self.type_params = type_params or []

class EnumDecl(Stmt):
    def __init__(self, name, members, visibility='public'):
        self.name = name
        self.members = members  # list of (name, optional_value_expr)
        self.visibility = visibility

# ============================================================================
# Parameters and method definitions
# ============================================================================

class Parameter(Node):
    def __init__(self, name, type_annotation=None, default=None, is_variadic=False, is_kwonly=False):
        self.name = name
        self.type_annotation = type_annotation
        self.default = default
        self.is_variadic = is_variadic  # *args
        self.is_kwonly = is_kwonly

class Method(Node):
    def __init__(self, name, params, return_type=None, body=None,
                 is_static=False, is_classmethod=False, is_property=False,
                 visibility='public', is_volatile=False, decorators=None):
        self.name = name
        self.params = params
        self.return_type = return_type
        self.body = body
        self.is_static = is_static
        self.is_classmethod = is_classmethod
        self.is_property = is_property
        self.visibility = visibility
        self.is_volatile = is_volatile
        self.decorators = decorators or []

# ============================================================================
# Statements
# ============================================================================

class ExprStmt(Stmt):
    def __init__(self, expr):
        self.expr = expr

class IfStmt(Stmt):
    def __init__(self, condition, then_body, else_body=None):
        self.condition = condition
        self.then_body = then_body
        self.else_body = else_body

class UnlessStmt(Stmt):
    def __init__(self, condition, body, else_body=None):
        self.condition = condition
        self.body = body
        self.else_body = else_body

class GuardStmt(Stmt):
    def __init__(self, condition, else_body):
        self.condition = condition
        self.else_body = else_body

class WhileStmt(Stmt):
    def __init__(self, condition, body, label=None):
        self.condition = condition
        self.body = body
        self.label = label

class UntilStmt(Stmt):
    def __init__(self, condition, body, label=None):
        self.condition = condition
        self.body = body
        self.label = label

class ForStmt(Stmt):
    def __init__(self, pattern, iterable, body, step=None, label=None):
        self.pattern = pattern
        self.iterable = iterable
        self.body = body
        self.step = step
        self.label = label

class LoopStmt(Stmt):
    def __init__(self, body, label=None):
        self.body = body
        self.label = label

class BreakStmt(Stmt):
    def __init__(self, label=None):
        self.label = label

class ContinueStmt(Stmt):
    def __init__(self, label=None):
        self.label = label

class AssertStmt(Stmt):
    def __init__(self, condition, message=None):
        self.condition = condition
        self.message = message

class ReturnStmt(Stmt):
    def __init__(self, value=None):
        self.value = value

class ThrowStmt(Stmt):
    def __init__(self, value=None):
        self.value = value

class TryStmt(Stmt):
    def __init__(self, try_body, catch_clauses=None, finally_body=None):
        self.try_body = try_body
        self.catch_clauses = catch_clauses or []
        self.finally_body = finally_body

class CatchClause(Node):
    def __init__(self, exception_type, var_name, body):
        self.exception_type = exception_type
        self.var_name = var_name
        self.body = body

class WithStmt(Stmt):
    def __init__(self, items, body):
        self.items = items  # list of (expr, optional_var_name) tuples
        self.body = body

class MatchStmt(Stmt):
    def __init__(self, expr, cases):
        self.expr = expr
        self.cases = cases

class MatchCase(Node):
    def __init__(self, pattern, guard=None, body=None):
        self.pattern = pattern
        self.guard = guard
        self.body = body

# ============================================================================
# Expressions
# ============================================================================

class Expr(Node):
    pass

class Identifier(Expr):
    def __init__(self, name):
        self.name = name

class IntLiteral(Expr):
    def __init__(self, value):
        self.value = int(value)

class FloatLiteral(Expr):
    def __init__(self, value):
        self.value = float(value)

class StrLiteral(Expr):
    def __init__(self, value, raw_literal=None):
        self.value = value
        # When set, this is the exact Python literal text (including prefixes
        # such as r""/b""), emitted verbatim so raw/byte semantics survive.
        self.raw_literal = raw_literal

class FStringLiteral(Expr):
    """An f-string split into literal text and interpolated expressions.

    ``parts`` is a list of either ``str`` (literal text, already escaped) or
    Expression nodes (the contents of a ``{ ... }`` interpolation).
    """
    def __init__(self, parts, quote='"'):
        self.parts = parts
        self.quote = quote

class BoolLiteral(Expr):
    def __init__(self, value):
        self.value = value

class NoneLiteral(Expr):
    pass

class ListLiteral(Expr):
    def __init__(self, elements):
        self.elements = elements

class DictLiteral(Expr):
    def __init__(self, pairs):
        self.pairs = pairs  # list of (key, value) tuples

class SetLiteral(Expr):
    def __init__(self, elements):
        self.elements = elements

class TupleLiteral(Expr):
    def __init__(self, elements):
        self.elements = elements

class BinaryOp(Expr):
    def __init__(self, left, op, right):
        self.left = left
        self.op = op
        self.right = right

class UnaryOp(Expr):
    def __init__(self, op, operand):
        self.op = op
        self.operand = operand

class CallExpr(Expr):
    def __init__(self, func, args, kwargs=None):
        self.func = func
        self.args = args
        self.kwargs = kwargs or {}

class IndexExpr(Expr):
    def __init__(self, obj, index):
        self.obj = obj
        self.index = index

class SliceExpr(Expr):
    """Python-style slice: ``obj[start:stop:step]`` (any part optional)."""
    def __init__(self, obj, start=None, stop=None, step=None):
        self.obj = obj
        self.start = start
        self.stop = stop
        self.step = step

class MemberExpr(Expr):
    def __init__(self, obj, member):
        self.obj = obj
        self.member = member

class PipeExpr(Expr):
    def __init__(self, left, right):
        self.left = left
        self.right = right

class CondExpr(Expr):
    """Ternary: condition ? true_expr : false_expr"""
    def __init__(self, condition, true_expr, false_expr):
        self.condition = condition
        self.true_expr = true_expr
        self.false_expr = false_expr

class ElvisExpr(Expr):
    """Elvis operator: value ?: default"""
    def __init__(self, value, default):
        self.value = value
        self.default = default

class CoalesceExpr(Expr):
    """Null coalescing: value ?? default"""
    def __init__(self, value, default):
        self.value = value
        self.default = default

class SafeNavExpr(Expr):
    """Safe navigation: obj?.member or obj?[index]"""
    def __init__(self, obj, member_or_index=None, is_index=False):
        self.obj = obj
        self.member_or_index = member_or_index
        self.is_index = is_index

class RangeExpr(Expr):
    def __init__(self, start, end, exclusive=False, step=None):
        self.start = start
        self.end = end
        self.exclusive = exclusive
        self.step = step

class LambdaExpr(Expr):
    def __init__(self, params, body):
        self.params = params
        self.body = body

class ComprehensionExpr(Expr):
    def __init__(self, expr, comprehensions, expr_type='list'):
        self.expr = expr  # what to generate
        self.comprehensions = comprehensions  # list of (pattern, iterable, filters)
        self.expr_type = expr_type  # 'list', 'dict', 'set'

class SpreadExpr(Expr):
    """Spread operator: *items or **dict"""
    def __init__(self, expr, is_dict=False):
        self.expr = expr
        self.is_dict = is_dict

class MatchExpr(Expr):
    """Match as expression (returns value from case)"""
    def __init__(self, expr, cases):
        self.expr = expr
        self.cases = cases

class TryExpr(Expr):
    """try/catch/finally in expression position (returns a value)."""
    def __init__(self, try_body, catch_clauses=None, finally_body=None):
        self.try_body = try_body
        self.catch_clauses = catch_clauses or []
        self.finally_body = finally_body

class BlockExpr(Expr):
    """Block that evaluates to last statement"""
    def __init__(self, statements):
        self.statements = statements

# ============================================================================
# Patterns (for pattern matching and destructuring)
# ============================================================================

class Pattern(Node):
    pass

class LiteralPattern(Pattern):
    def __init__(self, value):
        self.value = value

class IdentifierPattern(Pattern):
    def __init__(self, name):
        self.name = name

class WildcardPattern(Pattern):
    pass

class ConstructorPattern(Pattern):
    def __init__(self, name, subpatterns):
        self.name = name
        self.subpatterns = subpatterns

class ListPattern(Pattern):
    def __init__(self, patterns, rest_pattern=None):
        self.patterns = patterns
        self.rest_pattern = rest_pattern  # *items pattern

class DictPattern(Pattern):
    def __init__(self, field_patterns, rest_pattern=None):
        self.field_patterns = field_patterns  # {field: pattern}
        self.rest_pattern = rest_pattern  # **rest pattern

class OrPattern(Pattern):
    def __init__(self, patterns):
        self.patterns = patterns

class AsPattern(Pattern):
    def __init__(self, pattern, binding_name):
        self.pattern = pattern
        self.binding_name = binding_name

# ============================================================================
# Type annotations
# ============================================================================

class TypeAnnotation(Node):
    pass

class SimpleType(TypeAnnotation):
    def __init__(self, name):
        self.name = name

class GenericType(TypeAnnotation):
    def __init__(self, name, type_args):
        self.name = name
        self.type_args = type_args

class FunctionType(TypeAnnotation):
    def __init__(self, param_types, return_type):
        self.param_types = param_types
        self.return_type = return_type

class UnionType(TypeAnnotation):
    def __init__(self, types):
        self.types = types

class StructuralType(TypeAnnotation):
    def __init__(self, fields):
        self.fields = fields  # {name: type}

class OptionalType(TypeAnnotation):
    def __init__(self, base_type):
        self.base_type = base_type

# ============================================================================
# Other nodes
# ============================================================================

class Decorator(Node):
    def __init__(self, name, args=None, kwargs=None):
        self.name = name
        self.args = args or []
        self.kwargs = kwargs or {}

class Import(Node):
    def __init__(self, module, alias=None):
        self.module = module
        self.alias = alias

class ImportStmt(Stmt):
    def __init__(self, module, items=None, alias=None, modules=None):
        self.module = module
        self.items = items or []
        self.alias = alias
        # For `import a, b as c`: list of (module_path, optional_alias).
        self.modules = modules or []

class FromImport(Stmt):
    def __init__(self, module, items):
        self.module = module
        self.items = items  # list of (name, optional_alias)

# Backward compatibility (old names)
Number = IntLiteral
String = StrLiteral

# ============================================================================
# Python protocol (dunder) mapping
# ============================================================================
# Aura spells Python protocols with readable method names. Defining a method
# whose name appears here implements the corresponding Python protocol, both
# at the definition site and at every call/member-access site. Names already
# written as dunders are preserved verbatim.
SPECIAL_METHOD_NAMES = {
    # Construction and lifecycle
    'new': '__init__',
    'destroy': '__del__',
    'enter': '__enter__',
    'exit': '__exit__',
    # Representation and conversion
    'str': '__str__',
    'repr': '__repr__',
    'format': '__format__',
    'bytes': '__bytes__',
    'bool': '__bool__',
    'int': '__int__',
    'float': '__float__',
    'hash': '__hash__',
    'len': '__len__',
    'index': '__index__',
    'round': '__round__',
    # Comparison
    'eq': '__eq__',
    'ne': '__ne__',
    'lt': '__lt__',
    'le': '__le__',
    'gt': '__gt__',
    'ge': '__ge__',
    # Container protocols
    'getitem': '__getitem__',
    'setitem': '__setitem__',
    'delitem': '__delitem__',
    'contains': '__contains__',
    'iter': '__iter__',
    'reversed': '__reversed__',
    # Callable
    'call': '__call__',
    # Arithmetic operators
    'add': '__add__',
    'sub': '__sub__',
    'mul': '__mul__',
    'truediv': '__truediv__',
    'floordiv': '__floordiv__',
    'mod': '__mod__',
    'pow': '__pow__',
    'matmul': '__matmul__',
    'neg': '__neg__',
    'pos': '__pos__',
    'abs': '__abs__',
    'invert': '__invert__',
    # Reflected arithmetic operators
    'radd': '__radd__',
    'rsub': '__rsub__',
    'rmul': '__rmul__',
    'rtruediv': '__rtruediv__',
    'rfloordiv': '__rfloordiv__',
    'rmod': '__rmod__',
    'rpow': '__rpow__',
    # In-place arithmetic operators
    'iadd': '__iadd__',
    'isub': '__isub__',
    'imul': '__imul__',
    'itruediv': '__itruediv__',
    'ifloordiv': '__ifloordiv__',
    'imod': '__imod__',
    'ipow': '__ipow__',
    # Bitwise operators
    'and': '__and__',
    'or': '__or__',
    'xor': '__xor__',
    'lshift': '__lshift__',
    'rshift': '__rshift__',
    # Copy, subclass hooks
    'copy': '__copy__',
    'deepcopy': '__deepcopy__',
    'init_subclass': '__init_subclass__',
}


def python_method_name(name):
    """Return the Python method name for an Aura method name."""
    return SPECIAL_METHOD_NAMES.get(name, name)
Let = VarDecl
