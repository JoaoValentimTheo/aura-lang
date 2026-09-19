"""Complete type system with inference, checking, and narrowing for Aura."""
from dataclasses import dataclass, field
from typing import Any

# AST nodes are imported once at module scope (not per visited node) to keep the
# checker's hot paths fast.
from aura.transpiler.ast import (
    AsPattern,
    AssertStmt,
    BinaryOp,
    BlockExpr,
    BoolLiteral,
    CallExpr,
    ClassDecl,
    CoalesceExpr,
    ComprehensionExpr,
    CondExpr,
    ConstDecl,
    ConstructorPattern,
    DictLiteral,
    EnumDecl,
    ExprStmt,
    FloatLiteral,
    ForStmt,
    FStringLiteral,
    FunctionDecl,
    GenericType,
    GuardStmt,
    Identifier,
    IdentifierPattern,
    IfStmt,
    IndexExpr,
    IntLiteral,
    LambdaExpr,
    ListLiteral,
    LiteralPattern,
    LoopStmt,
    MatchExpr,
    MatchStmt,
MemberExpr,
    MemberPattern,
    Method,
    Module,
    Node,
    NoneLiteral,
    OptionalType,
    OrPattern,
    PipeExpr,
    Program,
    RangeExpr,
    ReturnStmt,
    SafeNavExpr,
    SetLiteral,
    SimpleType,
    SpreadExpr,
    Stmt,
    StrLiteral,
    StructuralType,
    ThrowStmt,
    TraitDecl,
    TryExpr,
    TryStmt,
    TupleLiteral,
    TypeDecl,
    UnaryOp,
    UnlessStmt,
    UntilStmt,
    VarDecl,
    WhileStmt,
    WildcardPattern,
    WithStmt,
)
from aura.transpiler.ast import (
    UnionType as AstUnionType,
)
from aura.transpiler.errors import AuraError, ErrorCode, ErrorSeverity

# Sentinel for "no binding present".
_MISSING = object()

# ============================================================================
# Type Classes
# ============================================================================

@dataclass(eq=False)
class Type:
    """Base type class."""
    def __str__(self):
        return self.__class__.__name__

    def __eq__(self, other):
        return isinstance(other, self.__class__)

    def __hash__(self):
        return hash(self.__class__.__name__)

    def is_compatible(self, other: 'Type') -> bool:
        """Check if this type is compatible with another type."""
        return self == other or isinstance(other, AnyType) or isinstance(self, AnyType)

@dataclass(eq=False)
class AnyType(Type):
    """Unknown/dynamic type."""
    pass

@dataclass(eq=False)
class NoneType(Type):
    """Null/None type."""
    def __str__(self):
        return "None"

@dataclass(eq=False)
class IntType(Type):
    """Integer type."""
    def __str__(self):
        return "Int"

@dataclass(eq=False)
class FloatType(Type):
    """Float type."""
    def __str__(self):
        return "Float"

@dataclass(eq=False)
class StrType(Type):
    """String type."""
    def __str__(self):
        return "String"

@dataclass(eq=False)
class BoolType(Type):
    """Boolean type."""
    def __str__(self):
        return "Bool"

@dataclass(eq=False)
class ListType(Type):
    """List type with element type."""
    element_type: Type = field(default_factory=lambda: AnyType())

    def __str__(self):
        return f"[{self.element_type}]"

    def is_compatible(self, other: Type) -> bool:
        if isinstance(other, ListType):
            return self.element_type.is_compatible(other.element_type)
        return super().is_compatible(other)

@dataclass(eq=False)
class DictType(Type):
    """Dict type with key and value types."""
    key_type: Type = field(default_factory=lambda: AnyType())
    value_type: Type = field(default_factory=lambda: AnyType())

    def __str__(self):
        return f"{{{self.key_type}: {self.value_type}}}"

    def is_compatible(self, other: Type) -> bool:
        if isinstance(other, DictType):
            return (self.key_type.is_compatible(other.key_type) and
                    self.value_type.is_compatible(other.value_type))
        return super().is_compatible(other)

@dataclass(eq=False)
class SetType(Type):
    """Set type with element type."""
    element_type: Type = field(default_factory=lambda: AnyType())

    def __str__(self):
        return f"{{{self.element_type}}}"

    def is_compatible(self, other: Type) -> bool:
        if isinstance(other, SetType):
            return self.element_type.is_compatible(other.element_type)
        return super().is_compatible(other)

@dataclass(eq=False)
class TupleType(Type):
    """Tuple type with element types."""
    element_types: list[Type] = field(default_factory=list)

    def __str__(self):
        types_str = ", ".join(str(t) for t in self.element_types)
        return f"({types_str})"

@dataclass(eq=False)
class FunctionType(Type):
    """Function type with parameter and return types."""
    param_types: list[Type] = field(default_factory=list)
    return_type: Type = field(default_factory=lambda: AnyType())
    is_async: bool = False
    variadic: bool = False
    type_params: list[str] = field(default_factory=list)
    # Number of parameters that must be supplied positionally (no default,
    # not variadic/keyword-only). Used to catch too-few-argument calls.
    required_args: int = 0

    def __str__(self):
        params = ", ".join(str(t) for t in self.param_types)
        if self.variadic:
            params = (params + ", ...") if params else "..."
        prefix = "async " if self.is_async else ""
        return f"{prefix}({params}) -> {self.return_type}"

@dataclass(eq=False)
class ClassType(Type):
    """Class type with fields and methods."""
    name: str
    fields: dict[str, Type] = field(default_factory=dict)
    methods: dict[str, FunctionType] = field(default_factory=dict)
    # Direct base classes. Aura allows multiple inheritance (`extends A, B`),
    # so this is a list, not a single parent. Populated during registration.
    bases: list['ClassType'] = field(default_factory=list)
    type_params: list[str] = field(default_factory=list)

    def __str__(self):
        return self.name

    def __eq__(self, other):
        # Identity is the class name, not just the runtime class: two distinct
        # classes must not compare equal (they would otherwise collapse in a
        # union or a set of types).
        return isinstance(other, ClassType) and other.name == self.name

    def __hash__(self):
        return hash(('ClassType', self.name))

    def get_field_type(self, field_name: str) -> Type:
        """Get field type, searching the base classes' fields too."""
        if field_name in self.fields:
            return self.fields[field_name]
        for base in self.bases:
            found = base.get_field_type(field_name)
            if not isinstance(found, AnyType):
                return found
        return AnyType()

    def get_method_type(self, method_name: str) -> FunctionType | None:
        """Get method type, searching the base classes' methods too."""
        if method_name in self.methods:
            return self.methods[method_name]
        for base in self.bases:
            found = base.get_method_type(method_name)
            if found is not None:
                return found
        return None

@dataclass(eq=False)
class UnionType(Type):
    """Union of multiple types."""
    types: set[Type] = field(default_factory=set)

    def __str__(self):
        types_str = " | ".join(sorted(str(t) for t in self.types))
        return types_str

    def is_compatible(self, other: Type) -> bool:
        return any(t.is_compatible(other) for t in self.types)

@dataclass(eq=False)
class TypeVariable(Type):
    """Generic type variable (T, U, etc.)."""
    name: str
    constraints: list[Type] = field(default_factory=list)

    def __str__(self):
        return self.name

    def __eq__(self, other):
        # Distinct type variables (`T`, `U`) must not compare equal; the base
        # class equality is type-only and would collapse them in a set/union.
        return isinstance(other, TypeVariable) and other.name == self.name

    def __hash__(self):
        return hash(('TypeVariable', self.name))

    def is_compatible(self, other: Type) -> bool:
        # Generic parameters are erased at runtime, so they accept any value.
        return True

# ============================================================================
# Type Inference
# ============================================================================

class TypeInference:
    """Infer types from AST nodes.

    Aura is gradually typed: unknown values become ``AnyType`` and never
    produce false positives. The inference is deliberately conservative.
    """

    # Aura builtin type names -> type instances.
    BUILTIN_NAMES = {
        'int': IntType(),
        'float': FloatType(),
        'str': StrType(),
        'string': StrType(),
        'bool': BoolType(),
        'none': NoneType(),
        'null': NoneType(),
        'any': AnyType(),
    }

    def __init__(self):
        # Backwards-compatible PascalCase map.
        self.builtin_types = {
            'Int': IntType(), 'Float': FloatType(), 'String': StrType(),
            'Bool': BoolType(), 'None': NoneType(), 'Any': AnyType(),
            'int': IntType(), 'float': FloatType(), 'str': StrType(),
            'bool': BoolType(), 'none': NoneType(),
        }

    def infer(self, node) -> Type:
        """Infer the type of an AST node."""

        if node is None:
            return AnyType()
        if isinstance(node, IntLiteral):
            return IntType()
        if isinstance(node, FloatLiteral):
            return FloatType()
        if isinstance(node, StrLiteral):
            return StrType()
        if isinstance(node, FStringLiteral):
            return StrType()
        if isinstance(node, BoolLiteral):
            return BoolType()
        if isinstance(node, NoneLiteral):
            return NoneType()
        if isinstance(node, ListLiteral):
            if node.elements:
                return ListType(self.infer(node.elements[0]))
            return ListType(AnyType())
        if isinstance(node, SetLiteral):
            if node.elements:
                return SetType(self.infer(node.elements[0]))
            return SetType(AnyType())
        if isinstance(node, TupleLiteral):
            return TupleType([self.infer(e) for e in node.elements])
        if isinstance(node, DictLiteral):
            if node.pairs:
                first = node.pairs[0]
                if isinstance(first, tuple):
                    return DictType(self.infer(first[0]), self.infer(first[1]))
            return DictType(AnyType(), AnyType())
        if isinstance(node, RangeExpr):
            return ListType(IntType())
        if isinstance(node, ComprehensionExpr):
            if node.expr_type == 'dict':
                return DictType(AnyType(), AnyType())
            if node.expr_type == 'set':
                return SetType(AnyType())
            return ListType(AnyType())
        if isinstance(node, LambdaExpr):
            return FunctionType()
        if isinstance(node, BinaryOp):
            return self._infer_binary_op(node)
        if isinstance(node, UnaryOp):
            return self._infer_unary_op(node)
        if isinstance(node, CoalesceExpr):
            return self.infer(node.value)
        if isinstance(node, CondExpr):
            return self._union(self.infer(node.true_expr), self.infer(node.false_expr))
        if isinstance(node, SafeNavExpr):
            return self._optional(self.infer(node.obj))
        if isinstance(node, (Identifier, MemberExpr, IndexExpr, SpreadExpr,
                             MatchExpr, TryExpr, BlockExpr, PipeExpr)):
            return AnyType()
        if isinstance(node, CallExpr):
            return self._infer_call(node)
        return AnyType()

    # Builtin return types: name -> (return kind, which-arg-to-mirror).
    # `element` means list-of-element_type-of-arg0, `arg0` mirrors arg0, etc.
    _BUILTIN_RETURNS = {
        'len': 'int', 'abs': 'arg0', 'sum': 'arg0_num', 'round': 'float',
        'min': 'arg0_iter', 'max': 'arg0_iter', 'sorted': 'arg0_list',
        'list': 'list', 'set': 'set', 'tuple': 'tuple', 'dict': 'dict',
        'int': 'int', 'float': 'float', 'str': 'str', 'bool': 'bool',
        'range': 'range', 'enumerate': 'list', 'zip': 'list',
        'reversed': 'arg0_list', 'any': 'bool', 'all': 'bool',
        'input': 'str', 'chr': 'str', 'ord': 'int', 'hex': 'str',
        'oct': 'str', 'bin': 'str', 'repr': 'str', 'format': 'str',
    }

    def _infer_call(self, node) -> Type:
        if not isinstance(node.func, Identifier):
            return AnyType()
        name = node.func.name
        kind = self._BUILTIN_RETURNS.get(name)
        if kind is None:
            return AnyType()
        args = list(node.args)
        if kind == 'int':
            return IntType()
        if kind == 'float':
            return FloatType()
        if kind == 'str':
            return StrType()
        if kind == 'bool':
            return BoolType()
        if kind == 'range':
            return ListType(IntType())
        if kind == 'list':
            if args:
                inner = self.infer(args[0])
                if isinstance(inner, ListType):
                    return inner
                if isinstance(inner, SetType):
                    return ListType(inner.element_type)
            return ListType(AnyType())
        if kind == 'set':
            if args:
                inner = self.infer(args[0])
                if isinstance(inner, (ListType, SetType)):
                    return SetType(inner.element_type)
            return SetType(AnyType())
        if kind == 'tuple':
            if args:
                inner = self.infer(args[0])
                if isinstance(inner, ListType):
                    return TupleType([inner.element_type])
            return TupleType([])
        if kind == 'dict':
            return DictType(AnyType(), AnyType())
        if kind == 'arg0':
            return self.infer(args[0]) if args else AnyType()
        if kind == 'arg0_num':
            return self.infer(args[0]) if args else IntType()
        if kind == 'arg0_iter':
            if args:
                inner = self.infer(args[0])
                if isinstance(inner, (ListType, SetType)):
                    return inner.element_type
            return AnyType()
        if kind == 'arg0_list':
            if args:
                inner = self.infer(args[0])
                if isinstance(inner, (ListType, SetType)):
                    return ListType(inner.element_type)
            return ListType(AnyType())
        return AnyType()

    def _infer_binary_op(self, node) -> Type:
        op = getattr(node, 'op', getattr(node, 'operator', None))

        # Assignment-like operators adopt the RHS type.
        if op in ('=', '+=', '-=', '*=', '/=', '%=', '**=', '&=', '|=', '^=',
                  '<<=', '>>=', '??='):
            return self.infer(node.right)

        left = self.infer(node.left)
        right = self.infer(node.right)

        if op in ('+', '-', '*', '/', '%', '**'):
            if op == '/' and isinstance(left, (IntType, FloatType)) and isinstance(right, (IntType, FloatType)):
                return FloatType()
            if op == '+' and isinstance(left, StrType) and isinstance(right, StrType):
                return StrType()
            if isinstance(left, (IntType, FloatType)) and isinstance(right, (IntType, FloatType)):
                if isinstance(left, FloatType) or isinstance(right, FloatType):
                    return FloatType()
                return IntType()
            if isinstance(left, ListType) and isinstance(right, ListType) and op == '+':
                return ListType(self._union(left.element_type, right.element_type))
            return AnyType()
        if op in ('<', '>', '<=', '>=', '==', '!=', 'is', 'is not', 'in', 'not in',
                  'and', 'or', '&&', '||'):
            return BoolType()
        if op in ('&', '|', '^', '<<', '>>'):
            if isinstance(left, BoolType):
                return BoolType()
            return IntType()
        if op in ('??', '?:'):
            return self._strip_none(left)
        if op == '?.':
            return self._optional(right)
        if op == 'as':
            return AnyType()
        return AnyType()

    def _infer_unary_op(self, node) -> Type:
        op = getattr(node, 'op', getattr(node, 'operator', None))
        operand = self.infer(node.operand)
        if op in ('not', '!'):
            return BoolType()
        if op == '-':
            return operand if isinstance(operand, (IntType, FloatType)) else AnyType()
        if op == '~':
            return IntType()
        return operand

    # -- helpers ------------------------------------------------------------

    @staticmethod
    def _optional(base: Type) -> Type:
        return UnionType({base, NoneType()})

    @staticmethod
    def _strip_none(base: Type) -> Type:
        if isinstance(base, UnionType):
            return UnionType(base.types - {NoneType()})
        return base

    @staticmethod
    def _union(a: Type, b: Type) -> Type:
        if a == b:
            return a
        if isinstance(a, AnyType) or isinstance(b, AnyType):
            return AnyType()
        return UnionType({a, b})


# ============================================================================
# Type Checker
# ============================================================================

class TypeChecker:
    """Check type compatibility and report real errors.

    The checker walks the real Aura AST. It reports:
      * declared-vs-actual mismatches on variables/constants,
      * obviously invalid binary operands (e.g. ``int + str``),
      * wrong argument counts for known functions,
      * return-type mismatches.
    Anything it cannot infer is treated as ``AnyType`` and accepted.
    """

    _COMPAT_OPS = {
        '+', '-', '*', '/', '%', '**',
        '==', '!=', '<', '>', '<=', '>=',
        'and', 'or', 'is', 'is not', 'in', 'not in',
        '&', '|', '^', '<<', '>>',
    }

    def __init__(self):
        self.inference = TypeInference()
        self.context: dict[str, Type] = {}
        self.classes: dict[str, ClassType] = {}
        self.functions: dict[str, FunctionType] = {}
        self.errors: list[str] = []
        # Structured diagnostics (code + location); ``errors`` mirrors these as
        # formatted strings for backward compatibility.
        self.diagnostics: list[AuraError] = []
        self._current_loc = None
        self._return_stack: list[Any] = []
        # Active generic type parameters (name -> TypeVariable).
        self._type_params: dict[str, TypeVariable] = {}
        # Class/trait names declared in the current program (constraint scope).
        self._declared_types: set = set()
        # Enum declarations in the current program (for match exhaustiveness).
        self._enum_decls: dict = {}
        # Names whose declared annotation is an enum, even before the enum's
        # own class type is resolvable (`Color` is not a registered class).
        self._enum_typed: dict = {}

    @staticmethod
    def _loc(node):
        if node is None:
            return None
        return getattr(node, 'location', None)

    def _here(self, node=None):
        return self._loc(node) or self._current_loc

    def _add(self, code: ErrorCode, message: str, node=None, hint=None):
        """Record a structured type diagnostic and its formatted string."""
        loc = self._here(node)
        err = AuraError(code, ErrorSeverity.ERROR, message, loc, hint)
        self.diagnostics.append(err)
        self.errors.append(str(err))

    def _add_warning(self, code: ErrorCode, message: str, node=None, hint=None):
        """Record a non-fatal type warning (does not fail the check)."""
        loc = self._here(node)
        self.diagnostics.append(
            AuraError(code, ErrorSeverity.WARNING, message, loc, hint))

    # -- public API ---------------------------------------------------------

    def check(self, node) -> bool:
        """Check an AST node. Returns True when no errors were found."""
        try:
            self.visit(node)
        except TypeError as exc:  # single hard failure
            self.errors.append(str(exc))
        return len(self.errors) == 0

    def check_program(self, program) -> bool:
        """Check a parsed Program and return True if it type-checks."""
        self.errors = []
        self.diagnostics = []
        self._current_loc = None
        # Reset per-program state so a reused checker (the REPL) does not carry
        # stale class types from a previous chunk.
        self.classes = {}
        self.functions = {}
        self.context = {}
        # Names usable as a generic constraint. Collected up front so a
        # constraint may reference a class declared later in the file.
        declared, enums = self._collect_declarations(program)
        self._declared_types = declared
        self._enum_decls = enums
        self._enum_typed = {}
        try:
            for stmt in getattr(program, 'statements', []):
                self.visit(stmt)
            # Resolve inheritance once every class is registered, so a base
            # declared later in the file is still linked.
            self._link_bases(program)
        except TypeError as exc:
            self.errors.append(str(exc))
        return len(self.errors) == 0

    @staticmethod
    def _collect_declarations(program):
        """Collect class/trait names and enum declarations in one pass.

        Both sets are needed before visiting the program and both require a
        full traversal, so they are gathered together.
        """
        names: set = set()
        decls: dict = {}

        def walk(node):
            if node is None:
                return
            if isinstance(node, (list, tuple)):
                for item in node:
                    walk(item)
                return
            if isinstance(node, (ClassDecl, TraitDecl)):
                names.add(node.name)
                body = getattr(node, 'body', None) or getattr(node, 'members', None) or []
                for member in body:
                    walk(member)
            elif isinstance(node, EnumDecl):
                decls[node.name] = node
                for value in vars(node).values():
                    if isinstance(value, (Node, list, tuple)):
                        walk(value)
            elif isinstance(node, Module):
                for member in getattr(node, 'members', []) or []:
                    walk(member)
            elif isinstance(node, Node):
                for value in vars(node).values():
                    if isinstance(value, (Node, list, tuple)):
                        walk(value)

        walk(program)
        return names, decls

    def _check_type_constraints(self, node, owner_kind):
        """Validate generic constraints on a class/trait/function declaration.

        A constraint must name a builtin type or a class/trait declared in the
        same program; anything else is a typo that would silently erase.
        """
        constraints = getattr(node, 'type_constraints', None) or {}
        if not constraints:
            return
        params = set(getattr(node, 'type_params', None) or [])
        for name, constraint in constraints.items():
            if name not in params:
                self._add(
                    ErrorCode.UNKNOWN_TYPE_CONSTRAINT,
                    f"{owner_kind} '{node.name}': constraint applies to "
                    f"'{name}', which is not one of its type parameters",
                    node,
                    hint="declare the parameter first, e.g. '[T: Comparable]'",
                )
                continue
            base = str(constraint).strip().strip('[]')
            # A constraint may be a union (`Comparable | Hashable`) or a
            # parameterised type (`Iterable[T]`); check each component name.
            for part in base.replace('|', ',').split(','):
                token = part.strip()
                if not token:
                    continue
                token = token.split('[')[0].strip()
                if (token in self._declared_types
                        or token in self.inference.builtin_types
                        or token in self.inference.BUILTIN_NAMES):
                    continue
                self._add(
                    ErrorCode.UNKNOWN_TYPE_CONSTRAINT,
                    f"{owner_kind} '{node.name}': type parameter '{name}' has "
                    f"unknown constraint '{token}'",
                    node,
                    hint="use a builtin type or a class/trait declared here",
                )

    # -- traversal ----------------------------------------------------------

    def visit(self, node):
        if node is None:
            return

        # Track the innermost statement location for expression diagnostics.
        if isinstance(node, Stmt):
            loc = getattr(node, 'location', None)
            if loc is not None:
                self._current_loc = loc

        if isinstance(node, Program):
            for stmt in node.statements:
                self.visit(stmt)
        elif isinstance(node, Module):
            for member in node.members:
                self.visit(member)
        elif isinstance(node, VarDecl):
            self._check_var_decl(node)
        elif isinstance(node, ConstDecl):
            self._check_const_decl(node)
        elif isinstance(node, FunctionDecl):
            self._check_function_decl(node)
        elif isinstance(node, ClassDecl):
            self._check_class_decl(node)
        elif isinstance(node, (EnumDecl, TypeDecl)):
            pass
        elif isinstance(node, TraitDecl):
            self._check_type_constraints(node, "Trait")
        elif isinstance(node, (IfStmt, UnlessStmt)):
            self._check_if(node)
        elif isinstance(node, GuardStmt):
            self.visit(node.condition)
            for stmt in (node.else_body or []):
                self.visit(stmt)
        elif isinstance(node, WhileStmt):
            self._check_condition(node.condition, "while")
            for stmt in node.body:
                self.visit(stmt)
        elif isinstance(node, UntilStmt):
            self._check_condition(node.condition, "until")
            for stmt in node.body:
                self.visit(stmt)
        elif isinstance(node, ForStmt):
            self._check_for(node)
        elif isinstance(node, LoopStmt):
            for stmt in node.body:
                self.visit(stmt)
        elif isinstance(node, MatchStmt):
            self._check_match_exhaustive(node)
            for case in node.cases:
                if case.guard is not None:
                    self._check_condition(case.guard, "match guard")
                for stmt in case.body:
                    self.visit(stmt)
        elif isinstance(node, TryStmt):
            self._check_try(node)
        elif isinstance(node, WithStmt):
            for _, var_name in node.items:
                if var_name:
                    self.context[var_name] = AnyType()
            for stmt in node.body:
                self.visit(stmt)
        elif isinstance(node, ReturnStmt):
            self._check_return(node)
        elif isinstance(node, ExprStmt):
            self._check_expr_stmt(node)
        elif isinstance(node, AssertStmt):
            self._check_condition(node.condition, "assert")
        elif isinstance(node, ThrowStmt):
            pass
        else:
            # Unknown statement: try to infer its expressions conservatively.
            self._visit_children(node)

    def _visit_children(self, node):
        if isinstance(node, Node):
            for value in vars(node).values():
                if isinstance(value, list):
                    for item in value:
                        if isinstance(item, Node):
                            self.visit(item)
                elif isinstance(value, Node):
                    self.visit(value)

    # -- declarations -------------------------------------------------------

    def _check_var_decl(self, node):
        expr_type = self.inference.infer(node.value) if node.value else NoneType()
        self._check_expr(node.value)
        if node.type_annotation is not None:
            declared = self._parse_type_annotation(node.type_annotation)
            if not declared.is_compatible(expr_type):
                self._add(
                    ErrorCode.TYPE_MISMATCH,
                    f"Variable '{node.name}': expected {declared}, got {expr_type}",
                    node,
                )
                self.context[node.name] = declared
                return
        self.context[node.name] = expr_type

    def _check_const_decl(self, node):
        expr_type = self.inference.infer(node.value) if node.value else NoneType()
        self._check_expr(node.value)
        if node.type_annotation is not None:
            declared = self._parse_type_annotation(node.type_annotation)
            if not declared.is_compatible(expr_type):
                self._add(
                    ErrorCode.TYPE_MISMATCH,
                    f"Constant '{node.name}': expected {declared}, got {expr_type}",
                    node,
                )
        self.context[node.name] = expr_type

    def _check_function_decl(self, node):
        # Register generic type parameters so `T` resolves in annotations.
        old_type_params = self._type_params
        self._type_params = dict(old_type_params)
        for tp in getattr(node, 'type_params', None) or []:
            self._type_params[tp] = TypeVariable(tp)
        self._check_type_constraints(node, "Function")

        param_types = []
        variadic = False
        required = 0
        for param in node.params:
            if getattr(param, 'is_variadic', False) or getattr(param, 'is_kwonly', False):
                variadic = True
            if param.name == '*':
                # Bare `*` separates positional from keyword-only params; the
                # ones after it cannot be passed positionally.
                continue
            if param.is_kwonly:
                continue
            if param.type_annotation is not None:
                param_types.append(self._parse_type_annotation(param.type_annotation))
            else:
                param_types.append(AnyType())
            if not param.is_variadic and param.default is None:
                required += 1

        return_type = AnyType()
        if node.return_type is not None:
            return_type = self._parse_type_annotation(node.return_type)

        self.functions[node.name] = FunctionType(
            param_types, return_type, getattr(node, 'is_async', False),
            variadic=variadic,
            type_params=list(getattr(node, 'type_params', None) or []),
            required_args=required,
        )

        old_context = dict(self.context)
        for param in node.params:
            if param.name == '*':
                continue
            if param.type_annotation is not None:
                self.context[param.name] = self._parse_type_annotation(param.type_annotation)
                self._record_enum_annotation(param.name, param.type_annotation)
            else:
                self.context[param.name] = AnyType()

        self._return_stack.append(return_type)
        for stmt in (node.body or []):
            self.visit(stmt)
        self._return_stack.pop()
        self.context = old_context
        self._type_params = old_type_params

    def _check_class_decl(self, node):
        type_params = list(getattr(node, 'type_params', None) or [])
        class_type = ClassType(node.name)
        class_type.type_params = type_params
        self._check_type_constraints(node, "Class")

        # Register generic parameters while reading field/method signatures.
        old_type_params = self._type_params
        self._type_params = dict(old_type_params)
        for tp in type_params:
            self._type_params[tp] = TypeVariable(tp)

        for item in (node.body or []):
            if isinstance(item, VarDecl):
                field_type = self._parse_type_annotation(item.type_annotation) \
                    if item.type_annotation is not None else AnyType()
                class_type.fields[item.name] = field_type
            elif isinstance(item, Method):
                class_type.methods[item.name] = self._method_to_function_type(item)
        # Header fields are instance fields too, so they participate in
        # inherited-field lookups exactly like body-declared fields.
        for entry in getattr(node, 'header_fields', None) or []:
            param, _visibility, _mutable = entry
            field_type = self._parse_type_annotation(param.type_annotation) \
                if param.type_annotation is not None else AnyType()
            class_type.fields[param.name] = field_type
        # Resolve base classes. Bases are resolved against `self.classes`,
        # which may not yet contain a class declared later in the file, so this
        # runs again after the whole program is registered (see `_link_bases`).
        for base_name in self._base_class_names(node):
            base_type = self.classes.get(base_name)
            if base_type is not None:
                class_type.bases.append(base_type)
        self.classes[node.name] = class_type

        # Every declared type parameter should be referenced, otherwise it is
        # dead syntax that erases silently.
        if type_params:
            used = set()
            for ftype in class_type.fields.values():
                used |= self._type_variables_in(ftype)
            for mtype in class_type.methods.values():
                for ptype in mtype.param_types:
                    used |= self._type_variables_in(ptype)
                used |= self._type_variables_in(mtype.return_type)
            for tp in type_params:
                if tp not in used:
                    self._add_warning(
                        ErrorCode.UNUSED_TYPE_PARAMETER,
                        f"Class '{node.name}': type parameter '{tp}' is never used",
                        node,
                    )

        # Check method bodies in a fresh scope.
        old_context = dict(self.context)
        for item in (node.body or []):
            if isinstance(item, Method):
                self._check_function_decl(item)
        self.context = old_context
        self._type_params = old_type_params

    @staticmethod
    def _base_class_names(node) -> list:
        """Split a ClassDecl/TraitDecl `base_class` string into bare names.

        Dotted bases (`pkg.Base`) contribute their final component, which is
        the name registered in `self.classes`.
        """
        base_class = getattr(node, 'base_class', None)
        if not base_class:
            return []
        names = []
        for part in str(base_class).split(','):
            name = part.strip()
            if name:
                names.append(name.split('.')[-1])
        return names

    def _link_bases(self, program):
        """Re-resolve every class's bases now that all classes are registered.

        `_check_class_decl` resolves bases as it walks, so a base declared later
        in the file would be missing. This second pass fills those in.
        """
        for node in getattr(program, 'statements', []):
            self._link_bases_in(node)

    def _link_bases_in(self, node):
        if node is None:
            return
        if isinstance(node, (list, tuple)):
            for item in node:
                self._link_bases_in(item)
            return
        if isinstance(node, ClassDecl):
            class_type = self.classes.get(node.name)
            if class_type is not None:
                resolved = []
                for base_name in self._base_class_names(node):
                    base_type = self.classes.get(base_name)
                    if base_type is not None and base_type not in resolved:
                        resolved.append(base_type)
                class_type.bases = resolved
        if isinstance(node, Node):
            for value in vars(node).values():
                if isinstance(value, (Node, list, tuple)):
                    self._link_bases_in(value)

    @staticmethod
    def _type_variables_in(tp) -> set:
        """Collect TypeVariable names nested in a type."""
        if tp is None:
            return set()
        if isinstance(tp, TypeVariable):
            return {tp.name}
        if isinstance(tp, (ListType, SetType)):
            return TypeChecker._type_variables_in(tp.element_type)
        if isinstance(tp, DictType):
            return (TypeChecker._type_variables_in(tp.key_type)
                    | TypeChecker._type_variables_in(tp.value_type))
        if isinstance(tp, TupleType):
            result = set()
            for sub in tp.element_types:
                result |= TypeChecker._type_variables_in(sub)
            return result
        if isinstance(tp, UnionType):
            result = set()
            for sub in tp.types:
                result |= TypeChecker._type_variables_in(sub)
            return result
        if isinstance(tp, FunctionType):
            result = TypeChecker._type_variables_in(tp.return_type)
            for sub in tp.param_types:
                result |= TypeChecker._type_variables_in(sub)
            return result
        if isinstance(tp, ClassType):
            # A generic class type used as a field, e.g. Box[T].
            return set(tp.type_params)
        return set()

    # -- statements ---------------------------------------------------------

    def _check_if(self, node):
        self._check_condition(node.condition, "if")
        # `IfStmt` uses then_body; `UnlessStmt` uses body (and its condition is
        # inverted, but narrowing is a best-effort optimization either way).
        then_body = getattr(node, 'then_body', None)
        if then_body is None:
            then_body = getattr(node, 'body', None) or []
        then_narrow, else_narrow = self._narrowings(node.condition)
        self._with_narrowing(then_narrow, then_body)
        self._with_narrowing(else_narrow, node.else_body or [])

    def _with_narrowing(self, narrowed, statements):
        saved = {name: self.context.get(name, _MISSING) for name in narrowed}
        self.context.update(narrowed)
        try:
            for stmt in statements:
                self.visit(stmt)
        finally:
            for name, old in saved.items():
                if old is _MISSING:
                    self.context.pop(name, None)
                else:
                    self.context[name] = old

    def _narrowings(self, condition):
        """Compute type-narrowed bindings for the then/else branches.

        Handles null checks (`x != null`, `x == null`) and `is` type tests
        (`x is int`). Returns ``(then_bindings, else_bindings)``.
        """
        then_narrow = {}
        else_narrow = {}
        if not isinstance(condition, BinaryOp):
            return then_narrow, else_narrow

        op = condition.op
        left, right = condition.left, condition.right

        if not isinstance(left, Identifier):
            return then_narrow, else_narrow

        current = self.context.get(left.name)

        # Null comparisons: x != null / x == null (and `is`/`is not`).
        is_null_test = isinstance(right, NoneLiteral)
        is_not = op in ('!=', 'is not')
        is_eq = op in ('==', 'is')

        if is_null_test and (is_not or is_eq):
            non_null = self.inference._strip_none(current) if current else AnyType()
            if is_not:
                then_narrow[left.name] = non_null
                else_narrow[left.name] = NoneType()
            else:
                then_narrow[left.name] = NoneType()
                else_narrow[left.name] = non_null
            return then_narrow, else_narrow

        # Type tests: `x is int` narrows to the tested type.
        if op in ('is', 'is not') and isinstance(right, Identifier):
            tested = self._parse_type_annotation(right.name)
            if not isinstance(tested, AnyType):
                if op == 'is':
                    then_narrow[left.name] = tested
                else:
                    else_narrow[left.name] = tested
            return then_narrow, else_narrow

        return then_narrow, else_narrow

    def _check_for(self, node):
        iter_type = self.inference.infer(node.iterable)
        if isinstance(node.pattern, IdentifierPattern):
            if isinstance(iter_type, (ListType, SetType)):
                self.context[node.pattern.name] = iter_type.element_type
            elif isinstance(iter_type, DictType):
                self.context[node.pattern.name] = iter_type.key_type
            else:
                self.context[node.pattern.name] = AnyType()
        for stmt in node.body:
            self.visit(stmt)

    def _check_try(self, node):
        for stmt in node.try_body:
            self.visit(stmt)
        for catch in node.catch_clauses:
            if catch.var_name:
                self.context[catch.var_name] = AnyType()
            for stmt in catch.body:
                self.visit(stmt)
        for stmt in (node.finally_body or []):
            self.visit(stmt)

    # -- match exhaustiveness ----------------------------------------------

    def _check_match_exhaustive(self, node):
        """Warn when a `match` over a known finite domain has no catch-all.

        Aura is gradually typed, so only subjects with a *known* finite domain
        are checked: ``bool`` (both literals) and enum values (all members).
        For any other value a bare ``case _``/``case name`` is required,
        because a `match` with no fallback silently does nothing for the
        unmatched values. This is a warning, so it never fails a build.
        """
        # A "catch-all" is a wildcard or a bare identifier binding, with no
        # guard, since a guard may reject the value and fall through.
        catch_all = False
        covered_literals = set()
        covered_enums = set()
        for case in node.cases:
            pattern = case.pattern
            if case.guard is None and self._is_catch_all(pattern):
                catch_all = True
            for lit in self._literal_values(pattern):
                covered_literals.add(lit)
            for name in self._constructor_names(pattern):
                covered_enums.add(name)
        if catch_all:
            return

        subject_type = self._subject_type(node.expr)
        subject_name = type(subject_type).__name__

        if subject_name == 'BoolType':
            missing = {True, False} - covered_literals
            if missing:
                rendered = ', '.join('true' if m else 'false' for m in sorted(missing))
                self._add_warning(
                    ErrorCode.NON_EXHAUSTIVE_MATCH,
                    f"'match' over bool is not exhaustive: no case handles {rendered}",
                    node,
                    hint="add the missing cases, or a 'case _' fallback",
                )
            return

        enum_members = self._enum_members_for(node.expr, subject_type)
        if enum_members is not None:
            missing = enum_members - covered_enums
            if missing:
                rendered = ', '.join(sorted(missing))
                self._add_warning(
                    ErrorCode.NON_EXHAUSTIVE_MATCH,
                    f"'match' over enum is not exhaustive: no case handles {rendered}",
                    node,
                    hint="add the missing cases, or a 'case _' fallback",
                )
            return

        if isinstance(subject_type, (IntType, StrType)):
            self._add_warning(
                ErrorCode.NON_EXHAUSTIVE_MATCH,
                f"'match' over {subject_type} is not exhaustive: no case handles "
                f"unlisted values",
                node,
                hint="add a 'case _' or 'case name' fallback",
            )

    def _record_enum_annotation(self, name, annotation):
        """Remember that ``name`` is declared with an enum type annotation.

        Recorded independently of full type resolution, because an enum name is
        not a registered class type: the resolved type would be ``Any`` and
        exhaustiveness could not apply.
        """
        text = annotation if isinstance(annotation, str) else getattr(annotation, 'name', None)
        if not isinstance(text, str):
            return
        base = text.strip().strip('?').split('[')[0].strip()
        if base in self._enum_decls:
            self._enum_typed[name] = base

    def _expr_type(self, expr):
        """Resolve an expression's type, preferring the checker context.

        ``TypeInference`` is context-free, so a bare variable infers as ``Any``
        even when its declared type is a class. The checker's ``context`` holds
        the real type, which inheritance-aware method resolution needs.
        """
        if isinstance(expr, Identifier) and expr.name in self.context:
            return self.context[expr.name]
        return self.inference.infer(expr)

    def _subject_type(self, expr):
        """Resolve the type of a match subject, preferring the checker context.

        ``TypeInference`` is context-free, so a bare variable would infer as
        ``Any``. The checker's ``context`` holds the declared/param types, which
        is what exhaustiveness analysis needs.
        """
        if isinstance(expr, Identifier) and expr.name in self.context:
            return self.context[expr.name]
        return self.inference.infer(expr)

    @staticmethod
    def _is_catch_all(pattern):
        """True for a wildcard or a bare identifier binding pattern."""
        if isinstance(pattern, WildcardPattern):
            return True
        if isinstance(pattern, IdentifierPattern):
            return True
        if isinstance(pattern, AsPattern):
            return TypeChecker._is_catch_all(pattern.pattern)
        if isinstance(pattern, OrPattern):
            return any(TypeChecker._is_catch_all(p) for p in pattern.patterns)
        return False

    @staticmethod
    def _literal_values(pattern):
        """Collect literal values a pattern matches (for bool coverage)."""
        if isinstance(pattern, OrPattern):
            values = set()
            for sub in pattern.patterns:
                values |= TypeChecker._literal_values(sub)
            return values
        if isinstance(pattern, LiteralPattern):
            value = pattern.value
            if isinstance(value, BoolLiteral):
                return {bool(value.value)}
            if value is True or value is False:
                return {value}
        return set()

    @staticmethod
    def _constructor_names(pattern):
        """Collect enum/constructor member names a pattern matches.

        A dotted member pattern (`Color.RED`) covers the ``RED`` member; a
        constructor pattern (`Some(x)`) covers ``Some``.
        """
        if isinstance(pattern, OrPattern):
            names = set()
            for sub in pattern.patterns:
                names |= TypeChecker._constructor_names(sub)
            return names
        if isinstance(pattern, ConstructorPattern):
            return {pattern.name}
        if isinstance(pattern, MemberPattern):
            member = getattr(pattern.expr, 'member', None)
            if isinstance(member, str):
                return {member}
        return set()

    def _enum_members_for(self, expr, subject_type=None):
        """Return enum member names when ``expr`` is a known enum value."""
        enum_name = None
        if isinstance(expr, MemberExpr) and isinstance(expr.obj, Identifier):
            enum_name = expr.obj.name
        elif isinstance(expr, Identifier) and expr.name in self._enum_typed:
            enum_name = self._enum_typed[expr.name]
        elif isinstance(expr, Identifier) and expr.name in self.context:
            declared = self.context[expr.name]
            enum_name = getattr(declared, 'name', None)
        elif subject_type is not None:
            enum_name = getattr(subject_type, 'name', None)
        if enum_name is None:
            return None
        decl = self._enum_decls.get(enum_name)
        if decl is not None:
            return {name for name, _value in decl.members}
        return None

    def _check_return(self, node):
        self._check_expr(node.value)
        if not self._return_stack:
            return
        expected = self._return_stack[-1]
        if node.value is not None and not isinstance(expected, AnyType):
            actual = self.inference.infer(node.value)
            if not expected.is_compatible(actual):
                self._add(
                    ErrorCode.TYPE_MISMATCH,
                    f"Return type mismatch: expected {expected}, got {actual}",
                    node,
                )

    def _check_expr_stmt(self, node):
        self._check_expr(node.expr)

    def _check_expr(self, node):
        """Recursively validate expressions and their sub-expressions."""
        if node is None:
            return

        if isinstance(node, BinaryOp):
            self._check_binary_op(node)
            self._check_expr(node.left)
            self._check_expr(node.right)
        elif isinstance(node, UnaryOp):
            self._check_expr(node.operand)
        elif isinstance(node, CallExpr):
            self._check_call_expr(node)
        elif isinstance(node, MemberExpr):
            self._check_expr(node.obj)
        elif isinstance(node, IndexExpr):
            self._check_expr(node.obj)
            self._check_expr(node.index)
        elif isinstance(node, CondExpr):
            self._check_condition(node.condition, "ternary")
            self._check_expr(node.true_expr)
            self._check_expr(node.false_expr)
        elif isinstance(node, CoalesceExpr):
            self._check_expr(node.value)
            self._check_expr(node.default)
        elif isinstance(node, SafeNavExpr):
            self._check_expr(node.obj)
        elif isinstance(node, (ListLiteral, SetLiteral, TupleLiteral)):
            for e in node.elements:
                self._check_expr(e)
        elif isinstance(node, DictLiteral):
            for pair in node.pairs:
                if isinstance(pair, tuple):
                    self._check_expr(pair[0])
                    self._check_expr(pair[1])
                else:
                    self._check_expr(pair)
        elif isinstance(node, ComprehensionExpr):
            self._check_expr(node.expr)
        elif isinstance(node, PipeExpr):
            self._check_expr(node.left)
            self._check_expr(node.right)
        elif isinstance(node, SpreadExpr):
            self._check_expr(node.expr)
        elif isinstance(node, FStringLiteral):
            for part in node.parts:
                if not isinstance(part, str):
                    self._check_expr(part[0])

    def _check_condition(self, condition, context):
        cond_type = self.inference.infer(condition)
        if isinstance(cond_type, (IntType, FloatType, StrType, ListType,
                                  DictType, SetType, TupleType, NoneType)):
            self._add(
                ErrorCode.TYPE_MISMATCH,
                f"{context} condition must be Bool, got {cond_type}",
                condition,
            )
        self._check_expr(condition)

    def _check_binary_op(self, node):
        op = getattr(node, 'op', getattr(node, 'operator', None))
        if op not in self._COMPAT_OPS:
            return
        left = self.inference.infer(node.left)
        right = self.inference.infer(node.right)
        if isinstance(left, AnyType) or isinstance(right, AnyType):
            return

        numeric = (IntType, FloatType)
        if op == '+':
            str_ok = isinstance(left, StrType) and isinstance(right, StrType)
            num_ok = isinstance(left, numeric) and isinstance(right, numeric)
            list_ok = isinstance(left, ListType) and isinstance(right, ListType)
            if not (str_ok or num_ok or list_ok):
                self._add(
                    ErrorCode.INCOMPATIBLE_OPERANDS,
                    f"Operator '+' cannot combine {left} and {right}",
                    node,
                )
        elif op in ('-', '*', '/', '%', '**'):
            if not (isinstance(left, numeric) and isinstance(right, numeric)):
                # `*` also supports str/list repetition.
                if op == '*' and ((isinstance(left, StrType) and isinstance(right, IntType))
                                  or (isinstance(left, IntType) and isinstance(right, StrType))):
                    return
                self._add(
                    ErrorCode.INCOMPATIBLE_OPERANDS,
                    f"Operator '{op}' requires numbers, got {left} and {right}",
                    node,
                )
        elif op in ('<', '>', '<=', '>='):
            if not ((isinstance(left, numeric) and isinstance(right, numeric))
                    or (isinstance(left, StrType) and isinstance(right, StrType))):
                self._add(
                    ErrorCode.INCOMPATIBLE_OPERANDS,
                    f"Operator '{op}' cannot compare {left} and {right}",
                    node,
                )
        elif op in ('&', '|', '^', '<<', '>>'):
            int_like = (IntType, BoolType)
            if not (isinstance(left, int_like) and isinstance(right, int_like)):
                self._add(
                    ErrorCode.INCOMPATIBLE_OPERANDS,
                    f"Bitwise operator '{op}' requires integers, got {left} and {right}",
                    node,
                )

    def _check_call_expr(self, node):
        func_type = None
        func_name = None
        if isinstance(node.func, Identifier) and node.func.name in self.functions:
            func_type = self.functions[node.func.name]
            func_name = node.func.name
        elif isinstance(node.func, MemberExpr):
            # Resolve a method on a known class instance. The checker's
            # `context` holds the declared/param type (which can be a
            # ClassType); the context-free inference would return Any here and
            # the inheritance lookup would never run.
            obj_type = self._expr_type(node.func.obj)
            if isinstance(obj_type, ClassType):
                func_type = obj_type.get_method_type(node.func.member)
                func_name = node.func.member

        if func_type is not None:
            expected = len(func_type.param_types)
            owner = f"Function '{func_name}'" if isinstance(node.func, Identifier) \
                else f"Method '{func_name}'"
            # Too many positional arguments, ignoring keyword arguments (which
            # may be extras) and variadics (which absorb the rest).
            if len(node.args) > expected and not func_type.variadic and not node.kwargs:
                self._add(
                    ErrorCode.WRONG_ARGUMENT_COUNT,
                    f"{owner} expects {expected} argument(s), got {len(node.args)}",
                    node,
                )
            # Too few: a required parameter is missing. Keyword arguments are
            # treated as satisfying the requirement, so this stays quiet when
            # the call is by name.
            required = func_type.required_args
            if (required and not node.kwargs and not func_type.variadic
                    and len(node.args) < required):
                self._add(
                    ErrorCode.WRONG_ARGUMENT_COUNT,
                    f"{owner} expects at least {required} argument(s), got {len(node.args)}",
                    node,
                )
            # Argument type checking against declared parameter types.
            for index, arg in enumerate(node.args):
                if index >= len(func_type.param_types):
                    break
                declared = func_type.param_types[index]
                if isinstance(declared, (AnyType, TypeVariable)) or declared is None:
                    continue
                actual = self.inference.infer(arg)
                if isinstance(actual, AnyType):
                    continue
                if not declared.is_compatible(actual):
                    owner = f"Function '{func_name}'" if isinstance(node.func, Identifier) \
                        else f"Method '{func_name}'"
                    self._add(
                        ErrorCode.WRONG_ARGUMENT_TYPE,
                        f"{owner} argument {index + 1}: expected {declared}, got {actual}",
                        node,
                    )
        for arg in node.args:
            self._check_expr(arg)
        for value in node.kwargs.values():
            self._check_expr(value)

    # -- type annotations ---------------------------------------------------

    def _parse_type_annotation(self, annotation) -> Type:
        """Convert a parsed Aura type annotation into a Type instance."""
        if annotation is None:
            return AnyType()

        # Annotations may be plain strings from the parser or AST type nodes.
        if isinstance(annotation, str):
            text = annotation.strip()
            if text in self._type_params:
                return self._type_params[text]
            if text in self.inference.BUILTIN_NAMES:
                return self.inference.BUILTIN_NAMES[text]
            if text.endswith('?') and len(text) > 1:
                return UnionType({self._parse_type_annotation(text[:-1]), NoneType()})
            if text.startswith('(') and '->' in text:
                return FunctionType()
            if text.startswith('[') and text.endswith(']'):
                return ListType(self._parse_type_annotation(text[1:-1]))
            # `List[T]`, `Set[T]`, `Dict[K, V]` emitted by the parser.
            if text.startswith('List[') and text.endswith(']'):
                return ListType(self._parse_type_annotation(text[5:-1]))
            if text.startswith('Set[') and text.endswith(']'):
                return SetType(self._parse_type_annotation(text[4:-1]))
            if text.startswith('Dict[') and text.endswith(']'):
                inner = text[5:-1]
                parts = [p.strip() for p in inner.split(',', 1)]
                if len(parts) == 2:
                    return DictType(self._parse_type_annotation(parts[0]),
                                    self._parse_type_annotation(parts[1]))
                return DictType()
            if text.startswith('{') and text.endswith('}'):
                inner = text[1:-1]
                return DictType() if ':' in inner else SetType()
            if '|' in text:
                parts = [p.strip() for p in text.split('|')]
                return UnionType({self._parse_type_annotation(p) for p in parts})
            if text in self.classes:
                return self.classes[text]
            return AnyType()

        # AST type nodes (used when types are constructed programmatically).
        if isinstance(annotation, SimpleType):
            return self._parse_type_annotation(annotation.name)
        if isinstance(annotation, OptionalType):
            return UnionType({self._parse_type_annotation(annotation.base_type), NoneType()})
        if isinstance(annotation, AstUnionType):
            return UnionType({self._parse_type_annotation(t) for t in annotation.types})
        if isinstance(annotation, GenericType):
            base = self._parse_type_annotation(annotation.name)
            if isinstance(base, (ListType, SetType)) and annotation.type_args:
                return type(base)(self._parse_type_annotation(annotation.type_args[0]))
            return AnyType()
        if isinstance(annotation, StructuralType):
            return DictType()
        return AnyType()

    def _method_to_function_type(self, method) -> FunctionType:
        param_types = []
        required = 0
        for param in method.params:
            if param.name in ('self', 'cls', '*'):
                continue
            if getattr(param, 'is_kwonly', False):
                continue
            if param.type_annotation is not None:
                param_types.append(self._parse_type_annotation(param.type_annotation))
            else:
                param_types.append(AnyType())
            if not getattr(param, 'is_variadic', False) and param.default is None:
                required += 1
        return_type = AnyType()
        if method.return_type is not None:
            return_type = self._parse_type_annotation(method.return_type)
        return FunctionType(param_types, return_type, required_args=required)


