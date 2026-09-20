"""Semantic checks that are independent of type inference.

Currently this implements Aura's mutability rules:

* ``let name = ...`` declares an immutable binding; it may be assigned only
  once, at its declaration.
* ``let mut name = ...`` declares a mutable binding and may be reassigned.
* ``const NAME = ...`` is immutable and must be initialised.
* ``for`` loop variables and ``with`` bindings are rebindable by the loop /
  context manager machinery, so they behave as mutable.
* Member assignments (``self.x = ...``, ``obj.field = ...``) are not affected:
  they mutate an object, not a local binding.

The checker is intentionally conservative: it only reports a reassignment when
it can identify a plain local binding in the current lexical scope, so it never
produces false positives for shadowing in nested scopes.
"""

from aura.transpiler.ast import (
    BinaryOp,
    BlockExpr,
    ClassDecl,
    ConstDecl,
    EnumDecl,
    ExprStmt,
    ForStmt,
    FunctionDecl,
    GuardStmt,
    Identifier,
    IdentifierPattern,
    IfStmt,
    IndexExpr,
    LambdaExpr,
    ListLiteral,
    ListPattern,
    LoopStmt,
    MatchStmt,
    MemberExpr,
    Module,
    Node,
    Program,
    ReturnStmt,
    SpreadExpr,
    Stmt,
    TraitDecl,
    TryStmt,
    TupleLiteral,
    TypeDecl,
    UnlessStmt,
    UntilStmt,
    VarDecl,
    WhileStmt,
    WithStmt,
)

ASSIGNMENT_OPS = frozenset({
    '=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>=', '??=',
})


class _Scope:
    __slots__ = ('bindings', 'parent')

    def __init__(self, parent=None):
        # name -> True when mutable, False when immutable.
        self.bindings = {}
        self.parent = parent

    def declare(self, name, mutable):
        if not isinstance(name, str) or not name:
            return
        # The closest declaration wins; re-declaration in the same scope
        # updates mutability (Aura lets a later declaration shadow).
        self.bindings[name] = mutable

    def lookup(self, name):
        scope = self
        while scope is not None:
            if name in scope.bindings:
                return scope.bindings[name]
            scope = scope.parent
        return None


class MutabilityChecker:
    """Walk an AST and collect immutability violations."""

    def __init__(self):
        self.errors = []
        # Structured diagnostics (code + location); ``errors`` mirrors these.
        self.diagnostics = []
        self.violations = []
        self._scope = _Scope()
        self._current_loc = None

    # -- public API ---------------------------------------------------------

    def check_program(self, program, initial_bindings=None) -> bool:
        """Check ``program``; return True when no rule was violated.

        ``initial_bindings`` maps names to their mutability (``True`` for
        mutable) as established by an enclosing session. It is used by the REPL
        so bindings declared in earlier chunks keep their mutability.
        """
        self.errors = []
        self.diagnostics = []
        self.violations = []
        self._scope = _Scope()
        self._current_loc = None
        # Function-local names declared later in the body, and those already
        # declared; used to report a use-before-declaration (E319).
        self._pending_locals: set = set()
        self._seen_locals: set = set()
        if initial_bindings:
            for name, mutable in initial_bindings.items():
                self._scope.declare(name, bool(mutable))
        for stmt in getattr(program, 'statements', []):
            self.visit(stmt)
        return not self.errors

    def collect_bindings(self, program, seed=None):
        """Return ``{name: mutable}`` for top-level bindings in ``program``.

        Existing entries in ``seed`` are preserved unless the program
        re-declares the name, mirroring the checker's shadowing semantics. Used
        by the REPL to carry binding mutability across chunks.
        """
        bindings = dict(seed or {})
        for stmt in getattr(program, 'statements', []):
            if isinstance(stmt, VarDecl):
                for name in self._target_names(stmt.name):
                    bindings[name] = bool(getattr(stmt, 'mutable', False))
            elif isinstance(stmt, ConstDecl):
                bindings[stmt.name] = False
        return bindings

    # -- traversal ----------------------------------------------------------

    def visit(self, node, mutable_binding=False):
        if node is None:
            return
        if isinstance(node, (list, tuple)):
            for item in node:
                self.visit(item)
            return

        # Track the innermost statement location for expression diagnostics.
        if isinstance(node, Stmt):
            loc = getattr(node, 'location', None)
            if loc is not None:
                self._current_loc = loc

        if isinstance(node, VarDecl):
            self._visit_var_decl(node)
        elif isinstance(node, ConstDecl):
            self._visit_const_decl(node)
        elif isinstance(node, FunctionDecl):
            self._visit_function(node)
        elif isinstance(node, ClassDecl):
            self._visit_class(node)
        elif isinstance(node, (EnumDecl, TypeDecl, TraitDecl)):
            pass
        elif isinstance(node, (IfStmt, UnlessStmt)):
            self.visit(node.then_body if hasattr(node, 'then_body') else node.body)
            self.visit(node.else_body)
            self.visit(node.condition)
        elif isinstance(node, GuardStmt):
            self.visit(node.condition)
            self.visit(node.else_body)
        elif isinstance(node, (WhileStmt, UntilStmt)):
            self.visit(node.condition)
            self.visit(node.body)
        elif isinstance(node, ForStmt):
            self._visit_for(node)
        elif isinstance(node, LoopStmt):
            self.visit(node.body)
        elif isinstance(node, MatchStmt):
            self.visit(node.expr)
            for case in node.cases:
                self._with_scope()
                self._bind_pattern(case.pattern, mutable=True)
                self.visit(case.guard)
                self.visit(case.body)
                self._pop_scope()
        elif isinstance(node, TryStmt):
            self.visit(node.try_body)
            for clause in (node.catch_clauses or []):
                self._with_scope()
                if clause.var_name:
                    self._scope.declare(clause.var_name, True)
                self.visit(clause.body)
                self._pop_scope()
            self.visit(node.finally_body)
        elif isinstance(node, WithStmt):
            for expr, var_name in node.items:
                self.visit(expr)
                if var_name:
                    # The with-target is rebound on entry; treat as mutable.
                    self._scope.declare(var_name, True)
            self.visit(node.body)
        elif isinstance(node, ReturnStmt):
            self.visit(node.value)
        elif isinstance(node, ExprStmt):
            self._visit_expr_stmt(node)
        elif isinstance(node, LambdaExpr):
            self._with_scope()
            for param in (node.params or []):
                self._scope.declare(self._param_name(param), True)
            self.visit(node.body)
            self._pop_scope()
        elif isinstance(node, Module):
            self.visit(node.members)
        elif isinstance(node, Program):
            for stmt in node.statements:
                self.visit(stmt)
        elif isinstance(node, BlockExpr):
            self._with_scope()
            self.visit(node.statements)
            self._pop_scope()
        elif isinstance(node, Identifier):
            self._check_use_before_declaration(node)
        elif isinstance(node, Node):
            self._visit_children(node)

    def _seen_locals_get(self):
        """The set of already-declared locals, created on first use.

        `check_program` seeds it; this keeps a direct `visit()` call safe
        without requiring the caller to initialise the checker's state.
        """
        seen = getattr(self, '_seen_locals', None)
        if seen is None:
            seen = set()
            self._seen_locals = seen
        return seen

    def _check_use_before_declaration(self, node):
        """Report a read of a function-local name before its `let`/`const`.

        Only fires inside a function body and only for a name that *this*
        function declares later in its own statement list, so a module-level,
        outer-scope or imported name is never reported.
        """
        name = node.name
        pending = getattr(self, '_pending_locals', None)
        if name in self._seen_locals_get() or name not in (pending or ()):
            return
        from aura.transpiler.errors import (
            AuraError,
            ErrorCode,
            ErrorSeverity,
        )
        loc = getattr(node, 'location', None) or self._current_loc
        err = AuraError(
            ErrorCode.USED_BEFORE_DECLARED,
            ErrorSeverity.ERROR,
            f"'{name}' is used before it is declared",
            loc,
            hint=f"move the 'let {name} = ...' above this line",
        )
        self.diagnostics.append(err)
        self.errors.append(str(err))

    def _visit_children(self, node):
        for value in vars(node).values():
            if isinstance(value, list):
                for item in value:
                    if isinstance(item, Node):
                        self.visit(item)
            elif isinstance(value, Node):
                self.visit(value)

    # -- declarations -------------------------------------------------------

    def _visit_var_decl(self, node):
        if node.value is not None:
            self.visit(node.value)
        for name in self._target_names(node.name):
            self._scope.declare(name, bool(getattr(node, 'mutable', False)))
            self._seen_locals_get().add(name)

    def _visit_const_decl(self, node):
        self.visit(node.value)
        self._scope.declare(node.name, False)
        self._seen_locals_get().add(node.name)

    def _visit_function(self, node):
        # Default values are evaluated in the *enclosing* scope.
        for param in (node.params or []):
            if getattr(param, 'default', None) is not None:
                self.visit(param.default)
        self._with_scope()
        declared = set()
        for param in (node.params or []):
            name = self._param_name(param)
            self._scope.declare(name, True)
            declared.add(name)
        body = node.body
        if isinstance(body, list):
            # Names this function declares somewhere in its own body. A read
            # of one before its declaration is a mistake (Python would raise
            # `UnboundLocalError`); parameters and earlier declarations are
            # excluded. This only fires for a name declared *later in the same
            # function*, so a module-level or outer-scope name is never
            # reported.
            pending = self._locals_declared_in(body) - declared
            old_pending, old_seen = self._pending_locals, self._seen_locals
            self._pending_locals = pending
            self._seen_locals = set(declared)
            try:
                self.visit(body)
            finally:
                self._pending_locals, self._seen_locals = old_pending, old_seen
        elif body is not None:
            # Expression-bodied function: names are read-only there.
            self.visit(body)
        self._pop_scope()

    def _locals_declared_in(self, statements):
        """Names bound by a `let`/`const` at a body's own nesting level.

        Only `let`/`const` count. A `for` target is intentionally excluded: it
        is scoped to its own loop (and a comprehension's target to the
        comprehension), so a read elsewhere in the body is not a
        use-before-declaration. Only declarations at this level are collected,
        so a name declared in a nested block is left to that block's scope.
        """
        names = set()
        for stmt in statements or []:
            if isinstance(stmt, VarDecl):
                for name in self._target_names(stmt.name):
                    names.add(name)
            elif isinstance(stmt, ConstDecl):
                names.add(stmt.name)
        return names

    def _visit_class(self, node):
        self._with_scope()
        self.visit(node.body)
        self._pop_scope()

    def _visit_for(self, node):
        self.visit(node.iterable)
        self._with_scope()
        self._bind_pattern(node.pattern, mutable=True)
        self.visit(node.body)
        self._pop_scope()

    # -- assignments --------------------------------------------------------

    def _visit_expr_stmt(self, node):
        expr = node.expr
        if isinstance(expr, BinaryOp) and expr.op in ASSIGNMENT_OPS:
            self._check_assignment(expr)
            return
        if isinstance(expr, TupleLiteral) and self._looks_like_multi_assign(expr):
            self._check_multi_assignment(expr)
            return
        self.visit(expr)

    @staticmethod
    def _looks_like_multi_assign(node):
        return any(
            isinstance(el, BinaryOp) and el.op in ASSIGNMENT_OPS
            for el in node.elements
        )

    def _check_assignment(self, node):
        target = node.left
        if isinstance(target, Identifier):
            self._record_reassignment(target.name, target)
        elif isinstance(target, (TupleLiteral, ListLiteral)):
            for el in target.elements:
                self._collect_assign_targets(el)
        # Member/index assignments mutate an object, not a binding.
        self.visit(node.right)

    def _check_multi_assignment(self, node):
        # `a, b = 1, 2` parses as TupleLiteral([Identifier a,
        # BinaryOp('=', Identifier b, ...), ...]); identifiers before the
        # BinaryOp are also targets.
        for el in node.elements:
            if isinstance(el, BinaryOp) and el.op in ASSIGNMENT_OPS:
                self._collect_assign_targets(el.left)
                self.visit(el.right)
            else:
                self._collect_assign_targets(el)

    def _collect_assign_targets(self, node):
        if isinstance(node, Identifier):
            self._record_reassignment(node.name, node)
        elif isinstance(node, (TupleLiteral, ListLiteral)):
            for el in node.elements:
                self._collect_assign_targets(el)
        elif isinstance(node, SpreadExpr):
            self._collect_assign_targets(node.expr)
        elif isinstance(node, (MemberExpr, IndexExpr)):
            # Object mutation, not a binding reassignment.
            self.visit(node)
        else:
            self.visit(node)

    def _record_reassignment(self, name, node):
        mutable = self._scope.lookup(name)
        if mutable is False:
            from aura.transpiler.errors import (
                AuraError,
                ErrorCode,
                ErrorSeverity,
            )
            loc = getattr(node, 'location', None)
            if loc is None:
                loc = self._current_loc
            if loc is None:
                line = getattr(node, 'line', None)
                if line:
                    from aura.transpiler.ast import SourceLocation
                    loc = SourceLocation(line=line)
            err = AuraError(
                ErrorCode.REASSIGN_IMMUTABLE,
                ErrorSeverity.ERROR,
                f"Cannot reassign immutable binding '{name}'; "
                f"declare it with 'let mut {name}' or use 'const' only for "
                f"values that never change",
                loc,
                hint=f"write 'let mut {name}' at its declaration",
            )
            self.violations.append(name)
            self.diagnostics.append(err)
            self.errors.append(str(err))

    # -- patterns -----------------------------------------------------------

    def _bind_pattern(self, pattern, mutable):
        for name in self._pattern_names(pattern):
            self._scope.declare(name, mutable)

    def _pattern_names(self, pattern):
        from aura.transpiler.pattern_utils import pattern_names
        return pattern_names(pattern)

    @staticmethod
    def _param_name(param):
        return getattr(param, 'name', None)

    def _target_names(self, name):
        """Parse a declaration target string into bound names.

        Plain identifiers return ``[name]``. Tuple targets such as
        ``"(a, b)"`` and destructuring patterns return each component name.
        """
        from aura.transpiler.pattern_utils import target_names
        return target_names(name)

    @staticmethod
    def _split_names(inner):
        names = []
        for part in inner.replace('*', ' ').split(','):
            token = part.strip().strip('()[]{}')
            if token and token.isidentifier():
                names.append(token)
        return names

    # -- scopes -------------------------------------------------------------

    def _with_scope(self):
        self._scope = _Scope(self._scope)

    def _pop_scope(self):
        if self._scope.parent is not None:
            self._scope = self._scope.parent
