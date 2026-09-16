"""Static rule checking for Aura.

The mutability rules live in :mod:`aura.transpiler.semantics`. This module adds
the remaining *structural* rules that make Aura's "strict by default" contract
real, independent of type inference:

* duplicate declarations in the same lexical scope;
* ``break`` / ``continue`` outside a loop;
* ``return`` outside a function;
* ``await`` outside an ``async`` function or top level of an async program;
* ``self`` used outside a class body;
* unreachable statements after ``return`` / ``throw`` / ``break`` / ``continue``;
* assignments whose target is not assignable (literal, call, ...);
* duplicate parameter names in a function signature.

Every rule emits a message through an :class:`~aura.transpiler.errors.ErrorCollector`
so the CLI and the REPL share one reporting format. The checker is deliberately
conservative: it never reports a problem it cannot prove, so it stays quiet on
dynamically shaped code.
"""

from aura.transpiler.errors import ErrorCollector, ErrorCode
from aura.transpiler.ast import (
    Node, Program, Module,
    VarDecl, ConstDecl, FunctionDecl, ClassDecl, EnumDecl, TypeDecl, TraitDecl,
    IfStmt, UnlessStmt, GuardStmt, WhileStmt, UntilStmt, ForStmt, LoopStmt,
    MatchStmt, TryStmt, WithStmt, ReturnStmt, ExprStmt, ThrowStmt,
    AssertStmt, BreakStmt, ContinueStmt, Method,
    Identifier, MemberExpr, IndexExpr, BinaryOp, TupleLiteral, ListLiteral,
    SpreadExpr, CallExpr, UnaryOp, LambdaExpr, BlockExpr,
    IdentifierPattern, ListPattern,
)

ASSIGNMENT_OPS = frozenset({
    '=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>=', '??=',
})

_TERMINATORS = (ReturnStmt, ThrowStmt, BreakStmt, ContinueStmt)


class RuleChecker:
    """Check structural language rules over a parsed program."""

    def __init__(self, collector=None):
        self.collector = collector or ErrorCollector()
        # One set of names per lexical scope body (module, function, block).
        self._scope_stack = []
        self._loop_depth = 0
        self._function_depth = 0
        self._async_depth = 0
        self._class_depth = 0
        # `guard cond else { return }` at the top level is Aura's idiomatic
        # "bail out of the program" form. A bare top-level `return` is only
        # legal in that position, so track it while walking a guard body.
        self._in_guard_else_depth = 0

    # -- public API ---------------------------------------------------------

    def check_program(self, program) -> bool:
        """Check ``program``; return True when no rule was violated."""
        self._scope_stack = [set()]
        self._loop_depth = 0
        self._function_depth = 0
        self._async_depth = 0
        self._class_depth = 0
        self._in_guard_else_depth = 0
        for stmt in getattr(program, 'statements', []) or []:
            self.visit(stmt)
        return not self.collector.has_errors()

    @property
    def errors(self):
        return [str(e) for e in self.collector.errors]

    # -- scope helpers ------------------------------------------------------

    def _declare(self, name):
        if not isinstance(name, str) or not name:
            return
        if self._scope_stack and name in self._scope_stack[-1]:
            self.collector.add(
                ErrorCode.DUPLICATE_DEFINITION,
                f"'{name}' is already defined in this scope",
                hint=f"rename or remove one of the '{name}' declarations",
            )
        if self._scope_stack:
            self._scope_stack[-1].add(name)

    def _push_scope(self):
        self._scope_stack.append(set())

    def _pop_scope(self):
        if len(self._scope_stack) > 1:
            self._scope_stack.pop()

    # -- traversal ----------------------------------------------------------

    def visit(self, node):
        if node is None:
            return
        if isinstance(node, (list, tuple)):
            for item in node:
                self.visit(item)
            return

        if isinstance(node, Program):
            for stmt in node.statements:
                self.visit(stmt)
        elif isinstance(node, Module):
            self._push_scope()
            for member in node.members:
                self.visit(member)
            self._pop_scope()
        elif isinstance(node, VarDecl):
            self._visit_var_decl(node)
        elif isinstance(node, ConstDecl):
            self._visit_const_decl(node)
        elif isinstance(node, FunctionDecl):
            self._visit_function(node)
        elif isinstance(node, Method):
            self._visit_method(node)
        elif isinstance(node, ClassDecl):
            self._visit_class(node)
        elif isinstance(node, EnumDecl):
            self._declare(node.name)
        elif isinstance(node, TypeDecl):
            self._declare(node.name)
        elif isinstance(node, TraitDecl):
            self._declare(node.name)
            # Trait method bodies (when present) are real function bodies.
            for member in node.members or []:
                if isinstance(member, Method):
                    self._visit_method(member)
                else:
                    self.visit(member)
        elif isinstance(node, (IfStmt, UnlessStmt)):
            self._visit_conditional(node)
        elif isinstance(node, GuardStmt):
            self.visit(node.condition)
            self._in_guard_else_depth += 1
            try:
                for stmt in (node.else_body or []):
                    self.visit(stmt)
            finally:
                self._in_guard_else_depth -= 1
        elif isinstance(node, (WhileStmt, UntilStmt)):
            self.visit(node.condition)
            self._visit_loop_body(node.body)
        elif isinstance(node, ForStmt):
            self.visit(node.iterable)
            self._push_scope()
            self._declare_pattern(node.pattern)
            self._visit_loop_body(node.body)
            self._pop_scope()
        elif isinstance(node, LoopStmt):
            self._visit_loop_body(node.body)
        elif isinstance(node, MatchStmt):
            self.visit(node.expr)
            for case in node.cases:
                self._push_scope()
                self._declare_pattern(case.pattern)
                self.visit(case.guard)
                self._visit_body(case.body)
                self._pop_scope()
        elif isinstance(node, TryStmt):
            self._visit_body(node.try_body)
            for clause in (node.catch_clauses or []):
                self._push_scope()
                if clause.var_name:
                    self._declare(clause.var_name)
                self._visit_body(clause.body)
                self._pop_scope()
            if node.finally_body:
                self._visit_body(node.finally_body)
        elif isinstance(node, WithStmt):
            for expr, var_name in node.items:
                self.visit(expr)
                if var_name:
                    self._declare(var_name)
            self._visit_body(node.body)
        elif isinstance(node, ReturnStmt):
            if self._function_depth == 0 and self._in_guard_else_depth == 0:
                self.collector.add(
                    ErrorCode.INVALID_SYNTAX,
                    "'return' outside of a function",
                    hint="move it inside a 'def', or use 'guard cond else { return }' to exit the program",
                )
            self.visit(node.value)
        elif isinstance(node, (BreakStmt, ContinueStmt)):
            if self._loop_depth == 0:
                keyword = 'break' if isinstance(node, BreakStmt) else 'continue'
                self.collector.add(
                    ErrorCode.INVALID_SYNTAX,
                    f"'{keyword}' outside of a loop",
                    hint="move it inside a loop or remove it",
                )
        elif isinstance(node, ThrowStmt):
            self.visit(node.value)
        elif isinstance(node, AssertStmt):
            self.visit(node.condition)
            self.visit(node.message)
        elif isinstance(node, ExprStmt):
            self.visit(node.expr)
        elif isinstance(node, LambdaExpr):
            self._visit_lambda(node)
        elif isinstance(node, (BinaryOp,)):
            self._visit_binary(node)
        elif isinstance(node, UnaryOp):
            if node.op == 'await':
                # Top-level `await` is valid: the CLI wraps async programs in a
                # coroutine. Inside a function, the enclosing `def` must be
                # `async`.
                if self._function_depth > 0 and self._async_depth == 0:
                    self.collector.add(
                        ErrorCode.INVALID_SYNTAX,
                        "'await' outside of an async function",
                        hint="mark the enclosing function 'async def'",
                    )
            self.visit(node.operand)
        elif isinstance(node, Identifier):
            if node.name == 'self' and self._class_depth == 0:
                self.collector.add(
                    ErrorCode.INVALID_SYNTAX,
                    "'self' used outside of a class method",
                    hint="use it inside a method defined in a class body",
                )
        elif isinstance(node, Node):
            for value in vars(node).values():
                if isinstance(value, (Node, list, tuple)):
                    self.visit(value)

    # -- declarations -------------------------------------------------------

    def _visit_var_decl(self, node):
        value = getattr(node, 'value', None)
        if value is not None:
            self.visit(value)
        for name in self._target_names(node.name):
            self._declare(name)

    def _visit_const_decl(self, node):
        if node.value is None:
            self.collector.add(
                ErrorCode.INVALID_SYNTAX,
                f"constant '{node.name}' must be initialised",
                hint="write 'const NAME = value'",
            )
        self.visit(node.value)
        self._declare(node.name)

    def _visit_function(self, node):
        self._declare(node.name)
        self._check_params(node.params)
        for param in node.params or []:
            self.visit(getattr(param, 'default', None))

        self._push_scope()
        for param in node.params or []:
            if getattr(param, 'name', None):
                if param.name != '*':
                    self._scope_stack[-1].add(param.name)
        self._function_depth += 1
        if getattr(node, 'is_async', False):
            self._async_depth += 1
        try:
            body = node.body
            if isinstance(body, list):
                self._visit_body(body)
            elif body is not None:
                self.visit(body)
        finally:
            if getattr(node, 'is_async', False):
                self._async_depth -= 1
            self._function_depth -= 1
            self._pop_scope()

    def _visit_class(self, node):
        self._declare(node.name)
        self._push_scope()
        self._class_depth += 1
        try:
            for member in node.body or []:
                self.visit(member)
        finally:
            self._class_depth -= 1
            self._pop_scope()

    def _visit_method(self, node):
        """Check a class/trait method body as a function body."""
        self._check_params(getattr(node, 'params', None))
        for param in (getattr(node, 'params', None) or []):
            self.visit(getattr(param, 'default', None))

        self._push_scope()
        for param in (getattr(node, 'params', None) or []):
            name = getattr(param, 'name', None)
            if name and name != '*':
                self._scope_stack[-1].add(name)
        self._function_depth += 1
        try:
            body = getattr(node, 'body', None)
            if isinstance(body, list):
                self._visit_body(body)
            elif body is not None:
                self.visit(body)
        finally:
            self._function_depth -= 1
            self._pop_scope()

    def _visit_lambda(self, node):
        self._check_params(node.params)
        self._push_scope()
        for param in node.params or []:
            if getattr(param, 'name', None):
                self._scope_stack[-1].add(param.name)
        self._function_depth += 1
        try:
            self.visit(node.body)
        finally:
            self._function_depth -= 1
            self._pop_scope()

    def _check_params(self, params):
        seen = set()
        for param in params or []:
            name = getattr(param, 'name', None)
            if not name or name == '*':
                continue
            if name in seen:
                self.collector.add(
                    ErrorCode.DUPLICATE_DEFINITION,
                    f"duplicate parameter '{name}'",
                    hint="give each parameter a unique name",
                )
            seen.add(name)

    # -- statements ---------------------------------------------------------

    def _visit_conditional(self, node):
        self.visit(node.condition)
        self._visit_body(getattr(node, 'then_body', None) or getattr(node, 'body', None))
        if getattr(node, 'else_body', None):
            self._visit_body(node.else_body)

    def _visit_loop_body(self, body):
        self._loop_depth += 1
        try:
            self._visit_body(body)
        finally:
            self._loop_depth -= 1

    def _visit_body(self, statements):
        if not statements:
            return
        self._push_scope()
        try:
            terminated = False
            for stmt in statements:
                if terminated:
                    self.collector.add(
                        ErrorCode.UNREACHABLE_CODE,
                        "unreachable statement after a terminating statement",
                        hint="remove the dead code or move it before the terminator",
                    )
                    terminated = False  # report once per region
                self.visit(stmt)
                if isinstance(stmt, _TERMINATORS):
                    terminated = True
        finally:
            self._pop_scope()

    # -- expressions --------------------------------------------------------

    def _visit_binary(self, node):
        if node.op in ASSIGNMENT_OPS:
            if not self._is_assignable(node.left):
                self.collector.add(
                    ErrorCode.INVALID_SYNTAX,
                    f"invalid assignment target for '{node.op}'",
                    hint="assign to a variable, member or index",
                )
            self.visit(node.right)
            return
        self.visit(node.left)
        self.visit(node.right)

    def _is_assignable(self, target):
        if isinstance(target, (Identifier, MemberExpr, IndexExpr)):
            return True
        if isinstance(target, (TupleLiteral, ListLiteral)):
            return all(self._is_assignable(el) for el in target.elements)
        if isinstance(target, SpreadExpr):
            return self._is_assignable(target.expr)
        return False

    # -- pattern / target helpers -------------------------------------------

    def _declare_pattern(self, pattern):
        for name in self._pattern_names(pattern):
            self._declare(name)

    def _pattern_names(self, pattern):
        if pattern is None:
            return []
        if isinstance(pattern, IdentifierPattern):
            return [pattern.name]
        if isinstance(pattern, Identifier):
            return [pattern.name]
        if isinstance(pattern, (TupleLiteral, ListLiteral)):
            names = []
            for el in pattern.elements:
                names.extend(self._pattern_names(el))
            return names
        if isinstance(pattern, ListPattern):
            names = []
            for sub in pattern.patterns:
                names.extend(self._pattern_names(sub))
            if getattr(pattern, 'rest_pattern', None) is not None:
                names.extend(self._pattern_names(pattern.rest_pattern))
            return names
        if isinstance(pattern, SpreadExpr):
            return self._pattern_names(pattern.expr)
        return []

    def _target_names(self, name):
        if not name:
            return []
        if isinstance(name, str) and name[:1] in '([' and name[-1:] in ')]':
            names = []
            for part in name[1:-1].replace('*', ' ').split(','):
                token = part.strip().strip('()[]{}')
                if token and token.isidentifier():
                    names.append(token)
            return names
        return [name] if isinstance(name, str) else []