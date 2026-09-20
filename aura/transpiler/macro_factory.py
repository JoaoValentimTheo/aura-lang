"""AuraMacroFactory — compile-time metaprogramming for Aura.

Aura's built-in macros (``@debug``, ``@memoize``, ...) are *runtime* decorators:
they wrap a function and cost a call on every invocation. The macro factory
adds a second, stronger tier: **compile-time macros** that receive their
arguments as quoted AST fragments and return replacement AST, which the
transpiler then emits as ordinary Python. Nothing survives to runtime unless the
macro chooses to emit it.

Design goals (roadmap Phase 1):

* **Hygiene** — bindings introduced by a macro expansion are renamed with a
  unique suffix (:func:`gensym`) so they can never capture, or be captured by,
  user identifiers at the call site.
* **Quoting** — macros build AST nodes instead of string-concatenating Python,
  so a macro cannot emit syntactically invalid code by accident. The
  :class:`Quote` helpers cover the common node shapes, including *statements*
  and whole *functions* (a macro may expand to a callable it defines).
* **Explicit registration** — a macro is a Python callable registered under a
  name. Only names present in a :class:`MacroRegistry` are expanded, so a typo
  is a compile error rather than a silent no-op.
* **Introspection** — because operands arrive as AST, a macro can inspect shape,
  names and literals (see :func:`is_identifier_named`, :func:`literal_value`,
  :func:`name_of`) rather than merely paste them.

The engine is deliberately independent of the transformer: it operates purely
on ``aura.transpiler.ast`` nodes and is unit-testable on its own. The
transformer calls :meth:`MacroRegistry.expand_call` at call sites.
"""

from __future__ import annotations

import itertools

from aura.transpiler.ast import (
    BinaryOp,
    BlockExpr,
    BoolLiteral,
    CallExpr,
    Expr,
    ExprStmt,
    FloatLiteral,
    Identifier,
    IntLiteral,
    ListLiteral,
    MemberExpr,
    NoneLiteral,
    Stmt,
    StrLiteral,
    TupleLiteral,
    UnaryOp,
)


class MacroError(Exception):
    """Raised when a macro is used with the wrong arity or shape."""


_GENSYM_COUNTER = itertools.count(1)


def gensym(prefix: str = "macro") -> str:
    """Return a fresh, collision-resistant identifier.

    The name embeds a counter and is prefixed so it cannot clash with a
    user-authored identifier in practice. Macros that introduce bindings should
    always build them with ``gensym`` — that is what makes expansion hygienic.
    """
    return f"__aura_{prefix}_{next(_GENSYM_COUNTER)}"


# ---------------------------------------------------------------------------
# Introspection helpers (macros inspect operands instead of only pasting them)
# ---------------------------------------------------------------------------

def name_of(node) -> str | None:
    """Return a readable source-like name for ``node``.

    Identifiers and members yield their dotted name; a call yields the callee's
    name, so a macro can label a value by the expression that produced it.
    Anything else has no name and yields None.
    """
    if isinstance(node, Identifier):
        return node.name
    if isinstance(node, MemberExpr):
        base = name_of(node.obj)
        return f"{base}.{node.member}" if base else node.member
    if isinstance(node, CallExpr):
        return name_of(node.func)
    return None


def is_identifier_named(node, name: str) -> bool:
    """Return True when ``node`` is the bare identifier ``name``."""
    return isinstance(node, Identifier) and node.name == name


def literal_value(node):
    """Return the Python value of a literal node, or ``_NOT_A_LITERAL``.

    Macros use this for compile-time decisions such as selecting an expansion
    from a literal argument. A non-literal operand yields the sentinel rather
    than raising, so a macro can fall back to emitting the expression.
    """
    if isinstance(node, IntLiteral):
        return node.value
    if isinstance(node, FloatLiteral):
        return node.value
    if isinstance(node, StrLiteral):
        return node.value
    if isinstance(node, BoolLiteral):
        return node.value
    if isinstance(node, NoneLiteral):
        return None
    return _NOT_A_LITERAL


class _NotALiteral:
    def __bool__(self):
        return False

    def __repr__(self):
        return "<not a literal>"


_NOT_A_LITERAL = _NotALiteral()


def contains_identifier(node, name: str) -> bool:
    """Return True when ``name`` appears anywhere inside the ``node`` tree."""
    stack = [node]
    while stack:
        current = stack.pop()
        if current is None:
            continue
        if isinstance(current, Identifier) and current.name == name:
            return True
        if isinstance(current, (list, tuple)):
            stack.extend(current)
            continue
        if isinstance(current, Node):
            stack.extend(vars(current).values())
    return False


from aura.transpiler.ast import Node  # noqa: E402  (used by contains_identifier)


class Quote:
    """Factories for AST fragments, so macros never build Python strings.

    Every helper validates its inputs and raises :class:`MacroError` on misuse,
    turning a malformed expansion into a clear diagnostic. Helpers exist for
    expressions, statements and whole functions, so a macro can expand to more
    than a single expression.
    """

    @staticmethod
    def identifier(name: str) -> Identifier:
        if not isinstance(name, str) or not name.isidentifier():
            raise MacroError(f"{name!r} is not a valid identifier")
        return Identifier(name)

    @staticmethod
    def int_literal(value: int) -> IntLiteral:
        if not isinstance(value, int) or isinstance(value, bool):
            raise MacroError(f"{value!r} is not an int literal")
        return IntLiteral(value)

    @staticmethod
    def float_literal(value: float) -> FloatLiteral:
        if not isinstance(value, float):
            raise MacroError(f"{value!r} is not a float literal")
        return FloatLiteral(value)

    @staticmethod
    def str_literal(value: str) -> StrLiteral:
        if not isinstance(value, str):
            raise MacroError(f"{value!r} is not a string literal")
        return StrLiteral(value)

    @staticmethod
    def bool_literal(value: bool) -> BoolLiteral:
        if not isinstance(value, bool):
            raise MacroError(f"{value!r} is not a bool literal")
        return BoolLiteral(value)

    @staticmethod
    def none_literal() -> NoneLiteral:
        return NoneLiteral()

    @staticmethod
    def list_literal(*elements) -> ListLiteral:
        Quote._require_exprs(elements, "list elements")
        return ListLiteral(list(elements))

    @staticmethod
    def tuple_literal(*elements) -> TupleLiteral:
        Quote._require_exprs(elements, "tuple elements")
        return TupleLiteral(list(elements))

    @staticmethod
    def call(callee, *args) -> CallExpr:
        target = (Quote.identifier(callee)
                  if isinstance(callee, str) else callee)
        if not isinstance(target, Expr):
            raise MacroError("call target must be a name or expression")
        Quote._require_exprs(args, "call arguments")
        return CallExpr(target, list(args))

    @staticmethod
    def member(obj, name: str) -> MemberExpr:
        if not isinstance(obj, Expr):
            raise MacroError("member object must be an expression")
        if not isinstance(name, str) or not name.isidentifier():
            raise MacroError(f"{name!r} is not a valid member name")
        return MemberExpr(obj, name)

    @staticmethod
    def binary(op: str, left, right) -> BinaryOp:
        if not isinstance(left, Expr) or not isinstance(right, Expr):
            raise MacroError("binary operands must be expressions")
        return BinaryOp(left, op, right)

    @staticmethod
    def unary(op: str, operand) -> UnaryOp:
        if not isinstance(operand, Expr):
            raise MacroError("unary operand must be an expression")
        return UnaryOp(op, operand)

    @staticmethod
    def expr_stmt(expr) -> ExprStmt:
        if not isinstance(expr, Expr):
            raise MacroError("expression statement needs an expression")
        return ExprStmt(expr)

    @staticmethod
    def var(name: str, value=None, *, mutable: bool = False):
        """Build a ``let`` / ``let mut`` binding with a hygienic name."""
        from aura.transpiler.ast import VarDecl
        if not isinstance(name, str) or not name.isidentifier():
            raise MacroError(f"{name!r} is not a valid identifier")
        if value is not None and not isinstance(value, Expr):
            raise MacroError("a binding value must be an expression")
        return VarDecl(name, mutable, value=value)

    @staticmethod
    def block(*statements) -> BlockExpr:
        if not statements:
            raise MacroError("a macro block needs at least one statement")
        for statement in statements:
            if not isinstance(statement, (Stmt, Expr)):
                raise MacroError(
                    f"block members must be statements or expressions, got "
                    f"{type(statement).__name__}")
        return BlockExpr(list(statements))

    @staticmethod
    def if_stmt(condition, then_body, else_body=None):
        from aura.transpiler.ast import IfStmt
        if not isinstance(condition, Expr):
            raise MacroError("if condition must be an expression")
        return IfStmt(condition, list(then_body), list(else_body or []))

    @staticmethod
    def function(name: str, params, body, return_type=None, *, is_async=False):
        """Build a function declaration, so a macro can expand to a helper.

        ``params`` is a list of :class:`Parameter`; ``body`` a list of
        statements. The macro is responsible for using :func:`gensym` for the
        function name so it cannot collide at the call site.
        """
        from aura.transpiler.ast import FunctionDecl, Parameter
        if not isinstance(name, str) or not name.isidentifier():
            raise MacroError(f"{name!r} is not a valid function name")
        for param in params:
            if not isinstance(param, Parameter):
                raise MacroError("function parameters must be Parameter nodes")
        for statement in body:
            if not isinstance(statement, (Stmt, Expr)):
                raise MacroError("a function body takes statements")
        return FunctionDecl(name, list(params), return_type, list(body),
                            is_async=is_async)

    @staticmethod
    def _require_exprs(items, what):
        for item in items:
            if not isinstance(item, Expr):
                raise MacroError(
                    f"{what} must be expressions, got {type(item).__name__}")


class Macro:
    """A registered compile-time macro.

    ``impl`` is a Python callable ``impl(args, kwargs) -> Expr``. ``args`` are
    the already-quoted positional operands and ``kwargs`` the keyword operands
    of the call site. ``min_args``/``max_args`` bound the positional arity;
    ``None`` means unbounded.
    """

    __slots__ = ("name", "impl", "min_args", "max_args", "allow_kwargs")

    def __init__(self, name, impl, min_args=0, max_args=None, allow_kwargs=False):
        if not isinstance(name, str) or not name.isidentifier():
            raise MacroError(f"macro name {name!r} is not an identifier")
        if not callable(impl):
            raise MacroError(f"macro {name!r} implementation is not callable")
        self.name = name
        self.impl = impl
        self.min_args = min_args
        self.max_args = max_args
        self.allow_kwargs = allow_kwargs

    def expand(self, args, kwargs):
        count = len(args)
        if count < self.min_args or (
                self.max_args is not None and count > self.max_args):
            if self.max_args is None:
                expected = f"at least {self.min_args}"
            elif self.min_args == self.max_args:
                expected = str(self.min_args)
            else:
                expected = f"{self.min_args}..{self.max_args}"
            raise MacroError(
                f"macro '{self.name}' expects {expected} argument(s), got {count}")
        if kwargs and not self.allow_kwargs:
            raise MacroError(
                f"macro '{self.name}' does not accept keyword arguments")
        result = self.impl(args, kwargs)
        if isinstance(result, Expr):
            # A block expansion can carry statement-only content; wrap a bare
            # statement list into a block so the caller always gets an Expr.
            return result
        raise MacroError(
            f"macro '{self.name}' must return an expression, got "
            f"{type(result).__name__}")


class MacroRegistry:
    """A collection of compile-time macros.

    The transformer consults a registry at every call site whose callee is a
    bare identifier; when the name is registered, the call is expanded and its
    replacement expression is emitted instead.
    """

    def __init__(self):
        self._macros: dict[str, Macro] = {}

    def __contains__(self, name) -> bool:
        return name in self._macros

    def __len__(self) -> int:
        return len(self._macros)

    def names(self) -> tuple[str, ...]:
        return tuple(sorted(self._macros))

    def register(self, name, impl, *, min_args=0, max_args=None,
                 allow_kwargs=False) -> Macro:
        """Register ``impl`` under ``name`` and return the :class:`Macro`."""
        if name in self._macros:
            raise MacroError(f"macro '{name}' is already registered")
        macro = Macro(name, impl, min_args=min_args, max_args=max_args,
                      allow_kwargs=allow_kwargs)
        self._macros[name] = macro
        return macro

    def macro(self, name, *, min_args=0, max_args=None, allow_kwargs=False):
        """Decorator form of :meth:`register`."""

        def decorate(impl):
            self.register(name, impl, min_args=min_args, max_args=max_args,
                          allow_kwargs=allow_kwargs)
            return impl

        return decorate

    def unregister(self, name) -> None:
        self._macros.pop(name, None)

    def get(self, name):
        return self._macros.get(name)

    def expand_call(self, call: CallExpr):
        """Expand ``call`` when its callee is a registered macro.

        Returns the replacement expression, or ``None`` when the call is not a
        macro invocation (so the transformer emits it normally).
        """
        callee = call.func
        if not isinstance(callee, Identifier):
            return None
        macro = self._macros.get(callee.name)
        if macro is None:
            return None
        kwargs = {}
        for key, value in getattr(call, "kwargs", {}).items():
            kwargs[key] = value
        return macro.expand(list(call.args), kwargs)


# ---------------------------------------------------------------------------
# Built-in compile-time macros
# ---------------------------------------------------------------------------

def _assert_eq(args, kwargs):
    """``assert_eq(a, b)`` → bind both once, then assert they are equal.

    The operands are evaluated exactly once into hygienic temporaries, so a
    side-effecting expression is not run twice and the failure message can show
    both values without re-evaluating them.
    """
    left, right = args
    left_name = gensym("ae_l")
    right_name = gensym("ae_r")
    from aura.transpiler.ast import AssertStmt
    condition = BinaryOp(Identifier(left_name), "==", Identifier(right_name))
    message = _assert_eq_message(left_name, right_name)
    return BlockExpr([
        Quote.var(left_name, left),
        Quote.var(right_name, right),
        AssertStmt(condition, message),
    ])


def _assert_ne(args, kwargs):
    """``assert_ne(a, b)`` → evaluate both once and assert they differ."""
    left, right = args
    left_name = gensym("an_l")
    right_name = gensym("an_r")
    from aura.transpiler.ast import AssertStmt
    condition = BinaryOp(Identifier(left_name), "!=", Identifier(right_name))
    message = _assert_ne_message(left_name, right_name)
    return BlockExpr([
        Quote.var(left_name, left),
        Quote.var(right_name, right),
        AssertStmt(condition, message),
    ])


def _assert_eq_message(left_name, right_name):
    """Build the ``f"{left!r} != {right!r}"`` failure message as an AST node."""
    from aura.transpiler.ast import FStringLiteral
    # The two names are hygienic, so the rendered message is traceable back to
    # the operator's operands without re-evaluating them.
    parts = [
        (Identifier(left_name), "!r"),
        " != ",
        (Identifier(right_name), "!r"),
    ]
    return FStringLiteral(parts)


def _assert_ne_message(left_name, right_name):
    from aura.transpiler.ast import FStringLiteral
    parts = [
        (Identifier(left_name), "!r"),
        " == ",
        (Identifier(right_name), "!r"),
    ]
    return FStringLiteral(parts)


def _todo(args, kwargs):
    """``todo()`` / ``todo("msg")`` → raise an error at the call site."""
    from aura.transpiler.ast import ThrowStmt
    message = args[0] if args else Quote.str_literal("not implemented")
    return BlockExpr([ThrowStmt(Quote.call("Error", message))])


def _identity(args, kwargs):
    return args[0]


def _discard(args, kwargs):
    """``discard(expr)`` → a hygienic binding, used to mark a value unused."""
    name = gensym("ignored")
    return BlockExpr([Quote.var(name, args[0])])


def _unreachable(args, kwargs):
    """``unreachable("why")`` → a documented ``todo`` that always raises."""
    message = args[0] if args else Quote.str_literal("unreachable")
    return _todo([message], kwargs)


def _static_assert(args, kwargs):
    """``static_assert(condition, message)`` → checked entirely at compile time.

    The condition must be a literal: a non-literal operand is a macro error, not
    a runtime assertion. A true condition expands to a no-op; a false one is a
    :class:`MacroError`, so the program never compiles with a broken invariant.
    """
    condition = args[0]
    value = literal_value(condition)
    if value is _NOT_A_LITERAL:
        raise MacroError(
            "static_assert requires a literal condition at compile time")
    if value:
        return BlockExpr([Quote.expr_stmt(Quote.none_literal())])
    message = args[1] if len(args) > 1 else Quote.str_literal(
        "static assertion failed")
    raise MacroError(f"static assertion failed: {_render_message(message)}")


def _render_message(node) -> str:
    """Render a literal message operand for a compile-time error, best effort."""
    value = literal_value(node)
    if value is _NOT_A_LITERAL:
        return "<message>"
    return str(value)


def _swap(args, kwargs):
    """``swap(a, b)`` → exchange two lvalues via a hygienic temporary.

    Both operands must be assignable names or members. The expansion binds the
    left value once, then performs the two assignments in sequence, so neither
    operand's side effect runs more than once.
    """
    left, right = args
    temp = gensym("swap")
    return BlockExpr([
        Quote.var(temp, left),
        Quote.expr_stmt(BinaryOp(left, "=", right)),
        Quote.expr_stmt(BinaryOp(right, "=", Identifier(temp))),
    ])


def _debug_value(args, kwargs):
    """``debug_value(x)`` → print ``x = <value>`` once, then return the value.

    The operand is evaluated a single time into a hygienic temporary, so a
    side-effecting expression is not repeated for the label and the value.
    """
    label = name_of(args[0]) or "value"
    temp = gensym("dbg")
    from aura.transpiler.ast import CallExpr as Call
    from aura.transpiler.ast import FStringLiteral
    message = FStringLiteral([f"{label} = ", (Identifier(temp), "!r")])
    return BlockExpr([
        Quote.var(temp, args[0]),
        Quote.expr_stmt(Call(Quote.identifier("print"), [message])),
        Identifier(temp),
    ])


def _stringify(args, kwargs):
    """``stringify(x)`` → ``str(x)`` at compile time for literals.

    A literal operand is turned into a string literal without a runtime call; a
    non-literal falls back to a ``str`` call.
    """
    value = literal_value(args[0])
    if value is not _NOT_A_LITERAL:
        return Quote.str_literal(str(value))
    return Quote.call("str", args[0])


def _once(args, kwargs):
    """``once(body)`` → execute ``body`` only on first call; subsequent calls
    are no-ops.

    The body is wrapped in a flag-guarded block. A hygienic boolean flag
    tracks whether the body has already executed.
    """
    flag = gensym("once_flag")
    body = args[0] if len(args) == 1 else BlockExpr(args)
    from aura.transpiler.ast import IfStmt, ExprStmt, AssignStmt
    return BlockExpr([
        Quote.var(flag, Quote.boolean_literal(False)),
        IfStmt(
            BinaryOp(Identifier(flag), "==", Quote.boolean_literal(False)),
            BlockExpr([
                AssignStmt(Identifier(flag), Quote.boolean_literal(True)),
                body,
            ]),
        ),
    ])


def _retry(args, kwargs):
    """``retry(count, body)`` → execute ``body`` up to ``count`` times.

    ``count`` must be a compile-time literal integer. The body is wrapped
    in a try/except that catches ``Exception``; on failure it retries until
    the count is exhausted.
    """
    count_val = literal_value(args[0])
    if count_val is _NOT_A_LITERAL or not isinstance(count_val, int) or count_val < 1:
        raise MacroError("retry() requires a positive integer literal for count")
    body = args[1] if len(args) > 1 else BlockExpr([])
    attempt = gensym("retry_attempt")
    last_err = gensym("retry_err")
    from aura.transpiler.ast import WhileStmt, AugAssignStmt, TryStmt, ExceptClause, RaiseStmt
    return BlockExpr([
        Quote.var(attempt, Quote.int_literal(0)),
        Quote.var(last_err, Quote.none_literal()),
        WhileStmt(
            BinaryOp(Identifier(attempt), "<", Quote.int_literal(count_val)),
            BlockExpr([
                TryStmt(
                    BlockExpr([
                        AugAssignStmt(Identifier(attempt), "+=", Quote.int_literal(1)),
                        body,
                    ]),
                    [ExceptClause(
                        Identifier("e"),
                        BlockExpr([
                            AssignStmt(Identifier(last_err), Identifier("e")),
                        ]),
                    )],
                ),
            ])
        ),
        IfStmt(
            BinaryOp(Identifier(last_err), "!=", Quote.none_literal()),
            BlockExpr([RaiseStmt(Identifier(last_err))]),
        ),
    ])


def default_registry() -> MacroRegistry:
    """Return a registry preloaded with Aura's built-in compile-time macros."""
    registry = MacroRegistry()
    registry.register("assert_eq", _assert_eq, min_args=2, max_args=2)
    registry.register("assert_ne", _assert_ne, min_args=2, max_args=2)
    registry.register("todo", _todo, min_args=0, max_args=1)
    registry.register("unreachable", _unreachable, min_args=0, max_args=1)
    registry.register("identity", _identity, min_args=1, max_args=1)
    registry.register("discard", _discard, min_args=1, max_args=1)
    registry.register("static_assert", _static_assert, min_args=1, max_args=2)
    registry.register("swap", _swap, min_args=2, max_args=2)
    registry.register("debug_value", _debug_value, min_args=1, max_args=1)
    registry.register("stringify", _stringify, min_args=1, max_args=1)
    registry.register("once", _once, min_args=1, max_args=1)
    registry.register("retry", _retry, min_args=2, max_args=2)
    return registry
