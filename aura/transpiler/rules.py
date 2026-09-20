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

from pathlib import Path

from aura.transpiler.ast import (
    AssertStmt,
    BinaryOp,
    BreakStmt,
    CallExpr,
    ClassDecl,
    ConstDecl,
    ContinueStmt,
    EnumDecl,
    ExprStmt,
    ForStmt,
    FunctionDecl,
    GuardStmt,
    Identifier,
    IfStmt,
    IndexExpr,
    LambdaExpr,
    ListLiteral,
    LoopStmt,
    MatchStmt,
    MemberExpr,
    Method,
    Module,
    Node,
    Program,
    ReturnStmt,
    SafeNavExpr,
    SpreadExpr,
    Stmt,
    ThrowStmt,
    TraitDecl,
    TryStmt,
    TupleLiteral,
    TypeDecl,
    UnaryOp,
    UnlessStmt,
    UntilStmt,
    VarDecl,
    WhileStmt,
    WithStmt,
    aura_method_name,
)
from aura.transpiler.errors import ErrorCode, ErrorCollector

ASSIGNMENT_OPS = frozenset({
    '=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>=', '??=',
})

_TERMINATORS = (ReturnStmt, ThrowStmt, BreakStmt, ContinueStmt)


# Exception roots a class may `extends` without declaring them: Aura's own
# `Error` (aliased to Python's `Exception` by the prelude) plus the Python
# exception names the language exposes.
_BUILTIN_BASE_NAMES = frozenset({
    'Error', 'Exception', 'BaseException',
    'ValueError', 'TypeError', 'KeyError', 'IndexError', 'RuntimeError',
    'AttributeError', 'NameError', 'ZeroDivisionError', 'NotImplementedError',
    'FileNotFoundError', 'IOError', 'OSError', 'PermissionError',
    'StopIteration', 'AssertionError', 'ArithmeticError', 'OverflowError',
    'UnicodeError', 'ImportError', 'LookupError', 'EOFError', 'TimeoutError',
})

class RuleChecker:
    """Check structural language rules over a parsed program."""

    def __init__(self, collector=None):
        self.collector = collector or ErrorCollector()
        # One set of names per lexical scope body (module, function, block).
        self._scope_stack = []
        self._loop_depth = 0
        self._function_depth = 0
        # Labels of enclosing loops and static-method depth. `check_program`
        # resets them too; defaulting here keeps a direct `visit()` safe.
        self._loop_labels = []
        self._static_method_depth = 0
        self._async_depth = 0
        self._class_depth = 0
        # `guard cond else { return }` at the top level is Aura's idiomatic
        # "bail out of the program" form. A bare top-level `return` is only
        # legal in that position, so track it while walking a guard body.
        self._in_guard_else_depth = 0
        # Visibility enforcement: a registry of every class's members and bases,
        # plus the stack of class names enclosing the code being checked, and a
        # parallel scope stack mapping local names to the class they were
        # instantiated from (``let c = Point(...)`` -> ``c: Point``) so external
        # member access can be resolved structurally. Runtime name mangling
        # remains the authoritative safety net for anything this cannot prove.
        self._classes = {}
        self._class_stack = []
        self._instance_scopes = []
        self._current_loc = None

    # -- public API ---------------------------------------------------------

    def check_program(self, program, require_main=False) -> bool:
        """Check ``program``; return True when no rule was violated.

        ``require_main`` is set by entry points that execute a file directly
        (``aura run``); it enforces the program entry point described in the
        language reference. Imported modules leave it off, so a library file
        needs no ``main``.
        """
        self._scope_stack = [set()]
        self._loop_depth = 0
        self._function_depth = 0
        self._async_depth = 0
        self._class_depth = 0
        self._in_guard_else_depth = 0
        self._class_stack = []
        self._instance_scopes = [{}]
        self._loop_labels = []
        # Depth of enclosing static methods (which have no `self`/`cls`).
        self._static_method_depth = 0
        # First pass: register every class's members and bases so member
        # access can be resolved (including inherited members) in any order.
        self._classes = {}
        self._register_classes(program)
        if require_main:
            self._check_main(program)
        # The structural checks below each need a full traversal; collect the
        # nodes they care about once instead of four separate walks.
        modules, classes, calls = self._structural_nodes(program)
        # A `main` inside a `module` body never runs, in any file.
        self._check_no_main_in_modules(program, modules)
        # A bare `export Name` must resolve to a sibling source or a local
        # declaration.
        self._check_reexports(program, modules)
        # `extends` must name a real base, without duplicates or cycles, and a
        # trait or still-abstract class must not be instantiated.
        self._check_inheritance(program, classes)
        self._check_instantiations(program, calls)
        for stmt in getattr(program, 'statements', []) or []:
            self.visit(stmt)
        return not self.collector.has_errors()

    def _check_main(self, program):
        """Enforce the program entry point for directly-executed files.

        A program must declare a top-level ``def main()`` (sync or async). It
        may accept a single ``args`` parameter, which receives the command-line
        arguments; any other signature is rejected. ``main`` is invoked by the
        runtime, never by the programmer, so a bare trailing ``main()`` call is
        unnecessary.

        A ``main`` declared inside a ``module`` body is always an error (E312):
        a module is a library namespace, and the runtime only ever calls the
        entry file's top-level ``main``.
        """
        main = None
        for stmt in getattr(program, 'statements', []) or []:
            if isinstance(stmt, FunctionDecl) and stmt.name == 'main':
                main = stmt
                break
        if main is None:
            stmts = getattr(program, 'statements', []) or []
            loc = self._loc(stmts[0]) if stmts else None
            self.collector.add(
                ErrorCode.MISSING_MAIN,
                "program has no 'main' function",
                location=loc,
                hint="declare 'def main() { ... }' (an entry file is executed "
                     "from 'main')",
            )
            return
        params = list(getattr(main, 'params', None) or [])
        if not params:
            return
        if len(params) == 1:
            param = params[0]
            name = getattr(param, 'name', None)
            if name == 'args' and not getattr(param, 'is_variadic', False) \
                    and not getattr(param, 'is_kwonly', False):
                return
        self.collector.add(
            ErrorCode.INVALID_MAIN,
            "'main' must take no parameters, or a single 'args' parameter",
            location=self._loc(main),
            hint="use 'def main()' or 'def main(args: [string])'",
        )

    @staticmethod
    def _walk(node):
        """Yield every AST node reachable from ``node`` (depth-first)."""
        stack = [node]
        while stack:
            current = stack.pop()
            if current is None:
                continue
            if isinstance(current, (list, tuple)):
                stack.extend(current)
                continue
            yield current
            if isinstance(current, Node):
                for value in vars(current).values():
                    if isinstance(value, (list, tuple)):
                        stack.extend(value)
                    elif isinstance(value, Node):
                        stack.append(value)

    def _structural_nodes(self, program):
        """Collect the node kinds the structural checks need in one walk.

        `_check_no_main_in_modules`, `_check_reexports`, `_check_inheritance`
        and `_check_instantiations` used to each traverse the whole program,
        which was measurable on large inputs. A single pass gathers every
        module, class/trait and call node they care about.
        """
        modules = []
        classes = []
        calls = []
        for node in self._walk(program):
            if isinstance(node, Module):
                modules.append(node)
            elif isinstance(node, (ClassDecl, TraitDecl)):
                classes.append(node)
            elif isinstance(node, CallExpr):
                calls.append(node)
        return modules, classes, calls

    def _check_no_main_in_modules(self, program, modules=None):
        """Report a `main` declared inside a module body (E312).

        The runtime calls only the entry file's top-level `main`; a `main` in a
        module namespace would never run, so it is almost certainly a mistake
        (a module is a library, not a program).
        """
        if modules is None:
            modules = (n for n in self._walk(program) if isinstance(n, Module))
        for node in modules:
            for member in node.members:
                if isinstance(member, FunctionDecl) and member.name == 'main':
                    self.collector.add(
                        ErrorCode.MAIN_IN_MODULE,
                        f"'main' is declared inside module '{node.name}'",
                        location=self._loc(member),
                        hint="'main' belongs to the entry file; modules "
                             "expose named functions instead",
                    )

    def _check_inheritance(self, program, classes=None):
        """Validate every `extends` clause (E314/E315/E316).

        Runs after registration so a base declared later in the file resolves.
        Catches what would otherwise be a confusing runtime `NameError` or
        `TypeError`:

        * a base class that does not exist (E314);
        * a repeated base (`class C extends A, A`, a Python `TypeError`) and a
          cycle (`class A extends B` / `class B extends A`) (E315);
        * instantiating a trait or a class with unimplemented abstract methods
          (E316).
        """
        # `_declared_types` holds every class/trait/enum/type name in the
        # program; a base outside it is unknown. Builtin exception roots are
        # always available (the `Error` prelude aliases `Error` to Python's
        # `Exception`), as are the Python exception names Aura exposes.
        known = set(self._classes) | set(getattr(self, '_declared_types', ()) or ())
        known |= _BUILTIN_BASE_NAMES
        if classes is None:
            classes = (n for n in self._walk(program)
                       if isinstance(n, (ClassDecl, TraitDecl)))
        for node in classes:
            info = self._classes.get(node.name) or {}
            bases = [b.strip() for b in str(node.base_class or '').split(',')
                     if b.strip()]
            for base in bases:
                bare = base.split('.')[-1]
                # A dotted base (`App.Base`, `pkg.Base`) resolves through its
                # head, which may be an imported module or another namespace;
                # only a plain name can be checked against local declarations.
                if '.' in base:
                    continue
                if bare not in known:
                    self.collector.add(
                        ErrorCode.UNKNOWN_BASE_CLASS,
                        f"'{node.name}' extends '{base}', which is not defined",
                        location=self._loc(node),
                        hint=f"declare '{bare}', import it, or fix the name",
                    )
            seen = set()
            for base in bases:
                bare = base.split('.')[-1]
                if bare in seen:
                    self.collector.add(
                        ErrorCode.INVALID_INHERITANCE,
                        f"'{node.name}' lists base '{bare}' more than once",
                        location=self._loc(node),
                        hint="remove the duplicate base",
                    )
                seen.add(bare)
            # `name` itself may not appear among its own bases (direct cycle).
            if node.name in seen:
                self.collector.add(
                    ErrorCode.INVALID_INHERITANCE,
                    f"'{node.name}' cannot extend itself",
                    location=self._loc(node),
                )

        # Indirect cycles: walk each class's base chain.
        for name in self._classes:
            visited = []
            current = name
            while current is not None:
                if current in visited:
                    cycle = ' -> '.join(visited[visited.index(current):] + [current])
                    self.collector.add(
                        ErrorCode.INVALID_INHERITANCE,
                        f"circular inheritance: {cycle}",
                        location=self._loc(
                            self._first_class_node(program, name)) or self._loc(program),
                        hint="break the cycle",
                    )
                    break
                visited.append(current)
                info = self._classes.get(current) or {}
                bases = info.get('bases') or []
                current = bases[0].split('.')[-1] if bases else None

    @staticmethod
    def _first_class_node(program, name):
        """Return the ClassDecl/TraitDecl named ``name``, or None."""
        stack = [program]
        while stack:
            node = stack.pop()
            if node is None:
                continue
            if isinstance(node, (list, tuple)):
                stack.extend(node)
                continue
            if isinstance(node, (ClassDecl, TraitDecl)) and node.name == name:
                return node
            if isinstance(node, Node):
                for value in vars(node).values():
                    if isinstance(value, (list, tuple)):
                        stack.extend(value)
                    elif isinstance(value, Node):
                        stack.append(value)
        return None

    def _check_instantiations(self, program, calls=None):
        """Reject instantiating a trait or a class with unimplemented abstracts.

        `T()` where `T` is a trait, and `A()` where `A` still has an abstract
        inherited method, are both Python `TypeError`s at runtime; reporting
        them here names the missing method at the call site (E316).
        """
        if calls is None:
            calls = (n for n in self._walk(program) if isinstance(n, CallExpr))
        for node in calls:
            func = node.func
            if not isinstance(func, Identifier):
                continue
            info = self._classes.get(func.name)
            if info is None:
                continue
            if info.get('is_trait'):
                self.collector.add(
                    ErrorCode.INSTANTIATE_ABSTRACT,
                    f"'{func.name}' is a trait and cannot be instantiated",
                    location=self._loc(node),
                    hint="extend it with a class and instantiate the class",
                )
                continue
            if info.get('is_abstract'):
                self.collector.add(
                    ErrorCode.INSTANTIATE_ABSTRACT,
                    f"'{func.name}' is abstract and cannot be instantiated",
                    location=self._loc(node),
                    hint="extend it with a concrete class and instantiate the class",
                )
                continue
            missing = self._unimplemented_abstracts(func.name)
            if missing:
                self.collector.add(
                    ErrorCode.INSTANTIATE_ABSTRACT,
                    f"'{func.name}' does not implement {', '.join(sorted(missing))}",
                    location=self._loc(node),
                    hint="implement the abstract method(s) before instantiating",
                )

    def _unimplemented_abstracts(self, class_name):
        """Names of abstract methods still unimplemented for ``class_name``.

        The obligation is transitive: an abstract method declared by any
        ancestor must be implemented somewhere on the path down to this class.
        A concrete method of the same name — declared on this class or inherited
        from any ancestor that overrides the abstract declaration — satisfies
        it. Comparing the *whole* transitive abstract set against the whole set
        of concrete implementations is what makes a chain of abstract classes
        work: an intermediate `abstract class B extends A` legitimately leaves
        A's method open, and a concrete `C extends B` that implements it must
        not be flagged because B did not.
        """
        abstracts = self._collect_abstracts(class_name, set())
        if not abstracts:
            return set()
        implemented = self._collect_concrete(class_name, set())
        return {name for name in abstracts if name not in implemented}

    def _collect_concrete(self, class_name, seen):
        """Names of concrete (implemented) methods on ``class_name`` and its
        ancestors, transitively. Only methods count: a field cannot satisfy an
        abstract method, even when the names collide across a hierarchy."""
        if class_name in seen:
            return set()
        seen.add(class_name)
        info = self._classes.get(class_name) or {}
        result = set(info.get('methods') or ())
        for base in info.get('bases') or []:
            result |= self._collect_concrete(base.split('.')[-1], seen)
        return result

    def _check_reexports(self, program, modules=None):
        """Report a `export Name` that cannot be resolved (E313).

        The transformer resolves a bare re-export against a sibling source
        file; checking it here means `aura check` reports the problem instead
        of only the later transpile step.
        """
        from aura.transpiler.modules import resolve_reexport

        source_path = getattr(program, 'source_path', None)
        base_dir = None if source_path is None else Path(source_path).resolve().parent
        if modules is None:
            modules = (n for n in self._walk(program) if isinstance(n, Module))
        for node in modules:
            declared = {getattr(m, 'name', None) for m in node.members}
            for item in getattr(node, 'reexports', None) or []:
                for name in item.names:
                    if name in declared:
                        continue
                    if base_dir is None:
                        continue  # no file context (REPL): cannot resolve
                    if resolve_reexport(name, base_dir, item.source) is None:
                        where = (f' from "{item.source}"' if item.source
                                 else "")
                        self.collector.add(
                            ErrorCode.UNRESOLVED_REEXPORT,
                            f"module '{node.name}' exports '{name}'{where}, "
                            f"but no sibling source defines it",
                            location=self._loc(item) or self._loc(node),
                            hint=f"add '{name.lower()}.aura' next to this file, "
                                 f"or declare '{name}' here",
                        )

    def _register_classes(self, node):
        """Collect class/trait member maps and base names, recursively."""
        if node is None:
            return
        if isinstance(node, (list, tuple)):
            for item in node:
                self._register_classes(item)
            return
        if isinstance(node, (ClassDecl, TraitDecl)):
            is_trait = isinstance(node, TraitDecl)
            is_abstract = bool(getattr(node, 'is_abstract', False))
            members = {}
            abstracts = {}
            method_names = set()
            body = getattr(node, 'body', None) or getattr(node, 'members', None) or []
            # Header fields (`class User(private name: str)`) are instance
            # fields: register them so visibility checks and inherited-member
            # lookups see them like any body-declared field.
            for entry in getattr(node, 'header_fields', None) or []:
                param, visibility, _mutable = entry
                members[param.name] = (visibility, False)
            for member in body:
                name = getattr(member, 'name', None)
                if name:
                    # (visibility, is_const): a class-level `const` may never be
                    # assigned through a member expression.
                    members[name] = (getattr(member, 'visibility', None),
                                     isinstance(member, ConstDecl))
                # A method with no body is a pure signature: a trait method is
                # always one, and `abstract def` declares one in a class.
                # Subclasses must provide the implementation.
                if isinstance(member, Method) and (
                        getattr(member, 'is_abstract', False)
                        or (is_trait and not member.body)):
                    abstracts[name] = node.name
                elif isinstance(member, Method):
                    method_names.add(name)
            bases = []
            if node.base_class:
                bases = [b.strip() for b in str(node.base_class).split(',') if b.strip()]
            self._classes[node.name] = {'members': members, 'bases': bases,
                                        'is_trait': is_trait, 'abstracts': abstracts,
                                        'methods': method_names,
                                        'is_abstract': is_abstract,
                                        'is_module': False,
                                        'exports': None}
            # Recurse into nested classes.
            for member in body:
                self._register_classes(member)
        elif isinstance(node, Module):
            # A module is a namespace like a class, but membership is governed
            # by `export` rather than a visibility modifier: a name that is not
            # exported is invisible outside the module (E308).
            exports = set(getattr(node, 'exports', None) or ())
            members = {}
            for member in node.members:
                name = getattr(member, 'name', None)
                if not name:
                    continue
                is_const = isinstance(member, ConstDecl)
                members[name] = ('public' if name in exports else 'private',
                                 is_const)
            # A bare `export Name` re-exports a symbol from a sibling file or
            # another namespace; it is public by definition.
            for item in getattr(node, 'reexports', None) or []:
                for name in item.names:
                    members.setdefault(name, ('public', False))
            # Nested modules register under their dotted path too, so a member
            # access through the outer namespace resolves.
            self._classes[node.name] = {
                'members': members, 'bases': [], 'is_trait': False,
                'abstracts': {}, 'is_module': True, 'exports': exports,
            }
            for member in node.members:
                self._register_classes(member)
        elif isinstance(node, Node):
            for value in vars(node).values():
                if isinstance(value, (Node, list, tuple)):
                    self._register_classes(value)

    @property
    def errors(self):
        return [str(e) for e in self.collector.errors]

    @staticmethod
    def _loc(node):
        """Return a node's SourceLocation, or None.

        Prefers the structured ``location`` attached by the parser; falls back
        to a ``location``-free result when only a bare ``line`` exists (the
        collector then reports a line-only diagnostic).
        """
        if node is None:
            return None
        loc = getattr(node, 'location', None)
        if loc is not None:
            return loc
        line = getattr(node, 'line', None)
        if line:
            from aura.transpiler.ast import SourceLocation
            return SourceLocation(line=line)
        return None

    def _here(self, node=None):
        """Location for a diagnostic: the node's, else the current statement's."""
        return self._loc(node) or self._current_loc

    # -- scope helpers ------------------------------------------------------

    def _declare(self, name, node=None):
        if not isinstance(name, str) or not name:
            return
        if self._scope_stack and name in self._scope_stack[-1]:
            self.collector.add(
                ErrorCode.DUPLICATE_DEFINITION,
                f"'{name}' is already defined in this scope",
                location=self._loc(node),
                hint=f"rename or remove one of the '{name}' declarations",
            )
        if self._scope_stack:
            self._scope_stack[-1].add(name)

    def _push_scope(self):
        self._scope_stack.append(set())
        self._instance_scopes.append({})

    def _pop_scope(self):
        if len(self._scope_stack) > 1:
            self._scope_stack.pop()
            self._instance_scopes.pop()

    # -- traversal ----------------------------------------------------------

    def visit(self, node):
        if node is None:
            return
        if isinstance(node, (list, tuple)):
            for item in node:
                self.visit(item)
            return

        # Remember the innermost statement's location so expression-level
        # diagnostics (which have no location of their own) can point at it.
        if isinstance(node, Stmt):
            loc = getattr(node, 'location', None)
            if loc is not None:
                self._current_loc = loc

        if isinstance(node, Program):
            for stmt in node.statements:
                self.visit(stmt)
        elif isinstance(node, Module):
            self._push_scope()
            self._module_depth = getattr(self, '_module_depth', 0) + 1
            try:
                for member in node.members:
                    self.visit(member)
            finally:
                self._module_depth -= 1
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
        elif isinstance(node, (EnumDecl, TypeDecl)):
            self._declare(node.name, node)
        elif isinstance(node, TraitDecl):
            self._declare(node.name, node)
            # Trait members also require explicit visibility, and trait method
            # bodies (when present) are real function bodies.
            self._check_duplicate_members(node)
            self._class_stack.append(node.name)
            self._class_depth += 1
            try:
                for member in node.members or []:
                    self._check_member_visibility(member, node.name)
                    if isinstance(member, Method):
                        self._visit_method(member)
                    else:
                        self.visit(member)
            finally:
                self._class_depth -= 1
                self._class_stack.pop()
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
            self._visit_loop_body(node.body, getattr(node, 'label', None))
        elif isinstance(node, ForStmt):
            self.visit(node.iterable)
            self._push_scope()
            self._declare_pattern(node.pattern)
            self._visit_loop_body(node.body, getattr(node, 'label', None))
            self._pop_scope()
        elif isinstance(node, LoopStmt):
            self._visit_loop_body(node.body, getattr(node, 'label', None))
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
                    location=self._loc(node),
                    hint="move it inside a 'def', or use 'guard cond else { return }' to exit the program",
                )
            self.visit(node.value)
        elif isinstance(node, (BreakStmt, ContinueStmt)):
            keyword = 'break' if isinstance(node, BreakStmt) else 'continue'
            if self._loop_depth == 0:
                self.collector.add(
                    ErrorCode.INVALID_SYNTAX,
                    f"'{keyword}' outside of a loop",
                    location=self._loc(node),
                    hint="move it inside a loop or remove it",
                )
            else:
                label = getattr(node, 'label', None)
                if label and label not in self._loop_labels:
                    self.collector.add(
                        ErrorCode.UNKNOWN_LABEL,
                        f"'{keyword} {label}' does not match any enclosing "
                        f"labeled loop",
                        location=self._loc(node),
                        hint="check the label, or drop it to affect the "
                             "innermost loop",
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
                        location=self._loc(node),
                        hint="mark the enclosing function 'async def'",
                    )
            elif node.op == 'yield':
                # `yield` turns the enclosing function into a generator. At
                # module level there is no function to convert, so Python
                # rejects the emitted `yield` with "'yield' outside function".
                # Reject it here, with a position, instead of emitting Python
                # that cannot compile.
                if self._function_depth == 0:
                    self.collector.add(
                        ErrorCode.INVALID_SYNTAX,
                        "'yield' outside of a function",
                        location=self._loc(node),
                        hint="move it into a 'def'; a 'yield' makes that "
                             "function a generator",
                    )
            self.visit(node.operand)
        elif isinstance(node, MemberExpr):
            self._visit_member_access(node.obj, node.member, node)
        elif isinstance(node, CallExpr):
            if isinstance(node.func, MemberExpr):
                self._visit_member_access(node.func.obj, node.func.member, node.func)
                self.visit(node.func.obj)
            else:
                self.visit(node.func)
            for arg in node.args or []:
                self.visit(arg)
            for value in (node.kwargs or {}).values():
                self.visit(value)
        elif isinstance(node, SafeNavExpr):
            if not getattr(node, 'is_index', False):
                self._visit_member_access(node.obj, node.member_or_index, node)
                self.visit(node.obj)
            else:
                self.visit(node.obj)
                self.visit(node.member_or_index)
        elif isinstance(node, Identifier):
            if node.name == 'self' and self._class_depth == 0:
                self.collector.add(
                    ErrorCode.INVALID_SYNTAX,
                    "'self' used outside of a class method",
                    location=self._here(node),
                    hint="use it inside a method defined in a class body",
                )
            elif node.name in ('self', 'cls') and self._static_method_depth:
                # A static method has no instance or class binding, so
                # `self`/`cls` would be an undefined name at runtime.
                self.collector.add(
                    ErrorCode.SELF_IN_STATIC,
                    f"'{node.name}' is not available in a static method",
                    location=self._here(node),
                    hint="take it as a parameter, or drop 'static' to make it "
                         "an instance/class method",
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
            self._declare(name, node)
            klass = self._instantiated_class(value)
            if klass and self._instance_scopes:
                self._instance_scopes[-1][name] = klass

    def _visit_const_decl(self, node):
        if node.value is None:
            self.collector.add(
                ErrorCode.INVALID_SYNTAX,
                f"constant '{node.name}' must be initialised",
                location=self._loc(node),
                hint="write 'const NAME = value'",
            )
        self.visit(node.value)
        self._declare(node.name, node)

    def _visit_function(self, node):
        self._declare(node.name, node)
        self._check_params(node.params)
        for param in node.params or []:
            self.visit(getattr(param, 'default', None))

        self._push_scope()
        for param in node.params or []:
            if getattr(param, 'name', None) and param.name != '*':
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
        self._declare(node.name, node)
        self._check_abstract_implemented(node)
        self._check_duplicate_members(node)
        self._push_scope()
        self._class_depth += 1
        self._class_stack.append(node.name)
        try:
            # A header field and a body field may not share a name: both would
            # define the same storage, and the constructor would assign twice.
            body_names = {m.name for m in (node.body or [])
                          if getattr(m, 'name', None)}
            for entry in getattr(node, 'header_fields', None) or []:
                param, _visibility, _mutable = entry
                if param.name in body_names:
                    self.collector.add(
                        ErrorCode.DUPLICATE_DEFINITION,
                        f"header field '{param.name}' in class '{node.name}' is "
                        f"also declared in the body",
                        location=self._loc(param) or self._loc(node),
                        hint="declare each field once",
                    )
                # Check the default expression like any other expression.
                self.visit(getattr(param, 'default', None))
            for member in node.body or []:
                self._check_member_visibility(member, node.name)
                if (isinstance(member, Method)
                        and getattr(member, 'is_abstract', False)
                        and not getattr(node, 'is_abstract', False)):
                    # An `abstract def` promises a subclass will provide the
                    # body, but a concrete class can be instantiated directly,
                    # so the promise cannot be kept. The method must either get
                    # a body or move to an `abstract class`.
                    self.collector.add(
                        ErrorCode.UNIMPLEMENTED_ABSTRACT,
                        f"abstract method '{aura_method_name(member.name)}' "
                        f"requires class '{node.name}' to be abstract",
                        location=self._loc(member) or self._loc(node),
                        hint=f"add 'abstract' to 'class {node.name}' or give "
                             f"'{aura_method_name(member.name)}' a body",
                    )
                self.visit(member)
        finally:
            self._class_stack.pop()
            self._class_depth -= 1
            self._pop_scope()

    def _check_duplicate_members(self, node):
        """Reject two members of a class/trait sharing a name (E301).

        Fields, methods, constants and nested classes all live in one namespace
        on the generated Python class, so a repeated name silently overwrites
        the earlier definition: `class C { def f() {...} def f() {...} }` keeps
        only the second `f`, and `class C { let x = 1  def x() {} }` turns the
        field into a method. Neither is caught by the plain scope check because
        methods do not declare themselves there, so the collision is reported
        here with the name and kind of the later declaration.
        """
        body = list(getattr(node, 'body', None)
                    or getattr(node, 'members', None) or [])
        seen = {}
        for entry in getattr(node, 'header_fields', None) or []:
            param, _visibility, _mutable = entry
            seen.setdefault(param.name, ('header field', param))
        for member in body:
            name = getattr(member, 'name', None)
            if not name or name == '*':
                continue
            if isinstance(member, Method):
                kind = 'method'
            elif isinstance(member, ConstDecl):
                kind = 'constant'
            elif isinstance(member, (ClassDecl, TraitDecl)):
                kind = 'nested class'
            else:
                kind = 'field'
            if name in seen:
                first_kind, first = seen[name]
                # Fields, constants and nested classes all declare themselves
                # in the class scope, so `_declare` already reports a collision
                # between two of them. Methods do not, which is the gap this
                # check closes: report only when a method is involved.
                if kind == 'method' or first_kind == 'method':
                    display = aura_method_name(name)
                    self.collector.add(
                        ErrorCode.DUPLICATE_DEFINITION,
                        f"{kind} '{display}' is already declared as a "
                        f"{first_kind} in '{node.name}'",
                        location=self._loc(member) or self._loc(node),
                        hint=f"rename one of the two '{display}' members",
                    )
                continue
            seen[name] = (kind, member)

    def _check_abstract_implemented(self, node):
        """Check abstract-method obligations for a class.

        An `abstract class` may defer implementation, so it is exempt. A
        concrete class must implement every abstract method it inherits (from a
        trait or an abstract class); otherwise instantiating it would fail at
        runtime. An `abstract def` may only appear in an abstract class.
        """
        if not isinstance(node, ClassDecl):
            return
        if getattr(node, 'is_abstract', False):
            return
        abstracts = self._collect_abstracts(node.name, set())
        if not abstracts:
            return
        # An `abstract def` declared by this concrete class is reported by the
        # member check with a pointed message ("requires class to be abstract"),
        # so skip it here to avoid a second, generic diagnostic for the same
        # method.
        own_abstracts = {
            m.name for m in (node.body or [])
            if isinstance(m, Method) and getattr(m, 'is_abstract', False)
        }
        implemented = self._collect_concrete(node.name, set())
        for name, decl_class in sorted(abstracts.items()):
            if name not in implemented and name not in own_abstracts:
                self.collector.add(
                    ErrorCode.UNIMPLEMENTED_ABSTRACT,
                    f"'{node.name}' must implement abstract method '{name}' "
                    f"declared by '{decl_class}'",
                    location=self._loc(node),
                    hint=f"add 'public def {name}(...)' to '{node.name}', "
                         f"or mark '{node.name}' abstract",
                )

    def _collect_abstracts(self, class_name, seen):
        """Map every abstract method name to its declaring trait or abstract
        class, following bases transitively."""
        if class_name in seen:
            return {}
        seen.add(class_name)
        info = self._classes.get(class_name)
        if not info:
            return {}
        result = dict(info.get('abstracts') or {})
        for base in info.get('bases', []):
            # A dotted base (`pkg.Base`) is registered by its bare class name,
            # so normalize the same way `_collect_concrete` does.
            for name, decl in self._collect_abstracts(
                    base.split('.')[-1], seen).items():
                result.setdefault(name, decl)
        return result

    def _check_member_visibility(self, member, class_name):
        """Every class/trait member must declare its visibility explicitly."""
        if isinstance(member, (VarDecl, ConstDecl, Method)):
            if getattr(member, 'visibility', None) is None:
                if isinstance(member, Method):
                    kind = 'method'
                elif isinstance(member, ConstDecl):
                    kind = 'constant'
                else:
                    kind = 'field'
                self.collector.add(
                    ErrorCode.MISSING_VISIBILITY,
                    f"{kind} '{member.name}' in class '{class_name}' has no "
                    f"visibility modifier",
                    location=self._loc(member),
                    hint="prefix it with 'public', 'private' or 'protected'",
                )
        elif isinstance(member, ClassDecl):
            if getattr(member, 'visibility', None) is None:
                self.collector.add(
                    ErrorCode.MISSING_VISIBILITY,
                    f"nested class '{member.name}' in class '{class_name}' has "
                    f"no visibility modifier",
                    location=self._loc(member),
                    hint="prefix it with 'public', 'private' or 'protected'",
                )

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
        is_static = bool(getattr(node, 'is_static', False))
        if is_static:
            self._static_method_depth += 1
        self._function_depth += 1
        try:
            body = getattr(node, 'body', None)
            if isinstance(body, list):
                self._visit_body(body)
            elif body is not None:
                self.visit(body)
        finally:
            self._function_depth -= 1
            if is_static:
                self._static_method_depth -= 1
            self._pop_scope()

    def _instantiated_class(self, value):
        """Return the class a variable is instantiated from, or None.

        Only the simple, structural form ``Class(...)`` is recognised; anything
        dynamic (a function call, a conditional, ...) stays None and is left to
        runtime mangling.
        """
        if not isinstance(value, CallExpr):
            return None
        func = value.func
        if not isinstance(func, Identifier) or not func.name:
            return None
        return func.name if func.name in self._classes else None

    def _object_class(self, obj):
        """Resolve the class of an access object, or None if unknown.

        ``self``/``cls`` resolve to the innermost enclosing class; ``super``
        resolves to that class's first base, so `super.member` is checked
        against the parent's members. A bare identifier resolves through the
        instance map when it is known to hold an instantiated class.
        """
        if isinstance(obj, Identifier):
            name = obj.name
            if name in ('self', 'cls') and self._class_stack:
                return self._class_stack[-1]
            if name == 'super' and self._class_stack:
                bases = self._classes.get(
                    self._class_stack[-1], {}).get('bases') or []
                return bases[0] if bases else None
            for scope in reversed(self._instance_scopes):
                if name in scope:
                    return scope[name]
            # A bare class name denotes the class itself (static access), so
            # `C.K` resolves against C even though no instance is instantiated.
            if name in self._classes:
                return name
        # A direct instantiation `C().x` denotes an instance of C. Resolving it
        # keeps visibility enforcement consistent with the `let c = C(); c.x`
        # form, which would otherwise be the only checked spelling.
        return self._instantiated_class(obj)

    def _visit_member_access(self, obj, member, node):
        """Visit an ``obj.member`` access, visiting both sides and enforcing
        visibility when the member belongs to a known class."""
        self.visit(obj)
        if not isinstance(member, str):
            return
        owner_class = self._object_class(obj)
        if owner_class is None:
            return
        # `super.member` on an abstract (body-less) method has no
        # implementation to run; the call would raise at runtime.
        if isinstance(obj, Identifier) and obj.name == 'super':
            base_info = self._classes.get(owner_class) or {}
            if member in (base_info.get('abstracts') or {}):
                self.collector.add(
                    ErrorCode.ABSTRACT_SUPER_CALL,
                    f"cannot call 'super.{member}': it is abstract in "
                    f"'{owner_class}' and has no implementation",
                    location=self._here(node),
                    hint="implement the method in this class instead of "
                         "calling the parent",
                )
        vis, decl_class = self._lookup_member(owner_class, member, set())
        if vis is None or vis == 'public':
            return
        is_module = bool(self._classes.get(decl_class, {}).get('is_module'))
        internal = self._is_internal_access(obj) and not is_module
        if internal:
            if vis == 'private' and not self._private_visible_here(decl_class):
                self.collector.add(
                    ErrorCode.INACCESSIBLE_MEMBER,
                    f"'{member}' is private to '{decl_class}' and cannot be "
                    f"accessed from a subclass",
                    location=self._here(node),
                    hint="access it through a public method or getter",
                )
            return
        if is_module:
            self.collector.add(
                ErrorCode.INACCESSIBLE_MEMBER,
                f"'{member}' is not exported from module '{decl_class}'",
                location=self._here(node),
                hint=f"add 'export' to {member} in module {decl_class}",
            )
            return
        self.collector.add(
            ErrorCode.INACCESSIBLE_MEMBER,
            f"'{member}' is {vis} in '{decl_class}' and cannot be accessed "
            f"from outside the class",
            location=self._here(node),
            hint="use a public getter/setter or a public method",
        )

    def _is_internal_access(self, obj):
        """True when ``obj`` is ``self``/``cls``/``super`` within a class body.

        ``super`` counts as internal because a parent call happens inside the
        subclass: protected members are reachable, private ones are not.
        """
        return isinstance(obj, Identifier) \
            and obj.name in ('self', 'cls', 'super') \
            and bool(self._class_stack)

    def _lookup_member(self, class_name, name, seen):
        """Resolve ``(visibility, declaring_class)`` for a member, walking the
        base-class chain. Returns ``(None, None)`` when the member is unknown."""
        vis, _is_const, decl = self._lookup_member_info(class_name, name, seen)
        return vis, decl

    def _lookup_member_info(self, class_name, name, seen):
        """Resolve ``(visibility, is_const, declaring_class)`` for a member.

        Walks the base-class chain; returns ``(None, False, None)`` when the
        member is unknown.
        """
        if class_name in seen:
            return None, False, None
        seen.add(class_name)
        info = self._classes.get(class_name)
        if not info:
            return None, False, None
        if name in info['members']:
            vis, is_const = info['members'][name]
            return vis, is_const, class_name
        for base in info['bases']:
            vis, is_const, decl = self._lookup_member_info(base, name, seen)
            if vis is not None:
                return vis, is_const, decl
        return None, False, None

    def _private_visible_here(self, decl_class):
        """True when the private member's declaring class is exactly the class
        whose method body is being checked. A private member is only visible
        inside its own class, never in a subclass."""
        return self._class_stack and self._class_stack[-1] == decl_class

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
                    location=self._here(param),
                    hint="give each parameter a unique name",
                )
            seen.add(name)

    # -- statements ---------------------------------------------------------

    def _visit_conditional(self, node):
        self.visit(node.condition)
        self._visit_body(getattr(node, 'then_body', None) or getattr(node, 'body', None))
        if getattr(node, 'else_body', None):
            self._visit_body(node.else_body)

    def _visit_loop_body(self, body, label=None):
        self._loop_depth += 1
        if label:
            self._loop_labels.append(label)
        try:
            self._visit_body(body)
        finally:
            if label:
                self._loop_labels.pop()
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
                        location=self._loc(stmt),
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
                    location=self._here(node),
                    hint="assign to a variable, member or index",
                )
            elif isinstance(node.left, MemberExpr):
                self._check_const_member_assignment(node.left)
            self.visit(node.right)
            return
        self.visit(node.left)
        self.visit(node.right)

    def _check_const_member_assignment(self, target):
        """Reject assigning through a member access to an immutable member.

        Covers two cases that would otherwise silently succeed at runtime:

        * a class-level ``const`` (`C.K = 2`, `self.K = 2`) — constants never
          change;
        * any module member (`M.count = 99`) — a module namespace is not
          writable from outside; mutate module state through an exported
          function instead.
        """
        if not isinstance(target.member, str):
            return
        owner_class = self._object_class(target.obj)
        if owner_class is None:
            return
        info = self._classes.get(owner_class) or {}
        vis, is_const, decl_class = self._lookup_member_info(
            owner_class, target.member, set())

        if info.get('is_module') and vis is not None and not self._is_module_body():
            self.collector.add(
                ErrorCode.REASSIGN_IMMUTABLE,
                f"cannot assign to module member '{owner_class}.{target.member}'; "
                f"module state is not writable from outside",
                location=self._here(target),
                hint="mutate it through an exported function",
            )
            return
        if not is_const:
            return
        self.collector.add(
            ErrorCode.REASSIGN_IMMUTABLE,
            f"cannot assign to constant '{target.member}' of '{decl_class}'; "
            f"constants never change",
            location=self._here(target),
            hint="declare it with 'let mut' if it must change",
        )

    def _is_module_body(self):
        """True when the current visit is inside a module body or a module
        function, where assigning to module state is allowed."""
        return bool(getattr(self, '_module_depth', 0))

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
        from aura.transpiler.pattern_utils import pattern_names
        return pattern_names(pattern)

    def _target_names(self, name):
        from aura.transpiler.pattern_utils import target_names
        return target_names(name)
