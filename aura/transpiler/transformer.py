"""Main transpiler that orchestrates AST → Python code transformation."""
from aura.transpiler.ast import *
from aura.transpiler.macros import (
    DICT_PRELUDE,
    ENUM_PRELUDE,
    ERROR_PRELUDE,
    LABEL_PRELUDE,
    OOP_PRELUDE,
    PRELUDE,
    SPREAD_PRELUDE,
    STDLIB_PRELUDE,
    UNSET_PRELUDE,
    error_prelude_needed,
    prelude_needed,
    stdlib_prelude_needed,
)
from aura.transpiler.transformers.expressions import ExpressionTransformer
from aura.transpiler.transformers.statements import StatementTransformer


class Transformer:
    def __init__(self):
        self.expr_transformer = ExpressionTransformer()
        self.stmt_transformer = StatementTransformer()
        self.stmt_transformer.expr_transformer = self.expr_transformer
        self.expr_transformer.stmt_transformer = self.stmt_transformer

    def transform(self, node):
        if isinstance(node, Program):
            return self._transform_program(node)
        elif isinstance(node, (ImportStmt, FromImport)):
            return self.stmt_transformer.transform(node)
        elif isinstance(node, Module):
            return self._transform_module(node)
        elif isinstance(node, Stmt):
            return self.stmt_transformer.transform(node)
        elif isinstance(node, Expr):
            return self.expr_transformer.transform(node)
        else:
            # Let the expression transformer raise its NotImplementedError so
            # the failing node type is named in the message.
            return self.expr_transformer.transform(node)

    def _transform_program(self, program):
        expr = self.expr_transformer
        stmt = self.stmt_transformer
        expr.hoisted_functions = []
        expr._needs_aura_call = False
        expr.seen_identifiers = set()
        expr.used_decorators = []
        expr.has_dict = False
        # Reset cross-program state so a reused Transformer is deterministic
        # (important for the REPL, which reuses one instance across chunks).
        expr.member_visibilities = {}
        expr.known_method_names = set()
        expr._lambda_counter = 0
        stmt.class_members = {}
        stmt.indent_level = 0
        stmt.in_class_scope = False
        stmt.function_scopes = []
        stmt.uses_aura_unset = False
        stmt.uses_oop_prelude = False
        stmt.has_enum = False
        stmt.has_label = False
        stmt.module_bindings = set()
        stmt._global_assignments = []
        # Source file path, used to resolve a module's sibling re-exports.
        stmt.source_path = getattr(program, 'source_path', None)
        stmt._module_reexport_imports = []
        # Collect module-level binding names so functions that assign to them
        # get a `global` declaration in the generated Python.
        from aura.transpiler.ast import ConstDecl, VarDecl
        for statement in program.statements:
            if isinstance(statement, VarDecl):
                for name in stmt._declared_names(statement.name):
                    stmt.module_bindings.add(name)
            elif isinstance(statement, ConstDecl):
                stmt.module_bindings.add(statement.name)
        lines = []
        for statement in program.statements:
            code = self.transform(statement)
            if code and code.strip():
                lines.append(code)
        body = "\n".join(lines)

        # Prelude flags were recorded while transforming, so there is no
        # separate AST scan.
        preludes = []
        if error_prelude_needed(expr.seen_identifiers):
            # Aura's exception root is `Error`; alias it to Python's Exception
            # so `class MyError extends Error` and `catch Error` work without
            # an explicit import.
            preludes.append(ERROR_PRELUDE)
        if prelude_needed(expr.used_decorators):
            preludes.append(PRELUDE)
        if stdlib_prelude_needed(expr.seen_identifiers):
            preludes.append(STDLIB_PRELUDE)
        if expr.has_dict:
            preludes.append(DICT_PRELUDE)
        if stmt.has_enum:
            preludes.append(ENUM_PRELUDE)
        if stmt.has_label:
            preludes.append(LABEL_PRELUDE)
        if expr._needs_aura_call:
            preludes.append(SPREAD_PRELUDE)
        if stmt.uses_aura_unset:
            preludes.append(UNSET_PRELUDE)
        if stmt.uses_oop_prelude:
            preludes.append(OOP_PRELUDE)

        hoisted = expr.hoisted_functions
        if hoisted:
            preludes.append("\n".join(hoisted))
        # A facade module's sibling imports are hoisted above the class body.
        if stmt._module_reexport_imports:
            preludes.append("\n".join(stmt._module_reexport_imports))

        if preludes:
            return "\n".join(preludes) + "\n" + body
        return body

    def _transform_module(self, node):
        return self.stmt_transformer.transform(node)


