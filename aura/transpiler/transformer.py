"""Main transpiler that orchestrates AST → Python code transformation."""
from aura.transpiler.ast import *
from aura.transpiler.transformers.expressions import ExpressionTransformer
from aura.transpiler.transformers.statements import StatementTransformer
from aura.transpiler.macros import (
    PRELUDE,
    prelude_needed,
    STDLIB_PRELUDE,
    STDLIB_PRELUDE_NAMES,
    stdlib_prelude_needed,
    DICT_PRELUDE,
    ENUM_PRELUDE,
    LABEL_PRELUDE,
)


# Fast lookup table for the most common node transform dispatch.
_COMMON_STMT_TRANSFORMS = frozenset({
    'VarDecl', 'ConstDecl', 'FunctionDecl', 'ClassDecl',
    'IfStmt', 'ForStmt', 'WhileStmt', 'ReturnStmt',
    'ExprStmt', 'Import', 'FromImport',
})


class Transformer:
    def __init__(self):
        self.expr_transformer = ExpressionTransformer()
        self.stmt_transformer = StatementTransformer()
        self.stmt_transformer.expr_transformer = self.expr_transformer
        self.expr_transformer.stmt_transformer = self.stmt_transformer
        # Cache transform methods to avoid repeated getattr lookups.
        self._stmt_methods = {}
        self._expr_methods = {}
    
    def _get_stmt_method(self, name):
        """Cache-backed lookup for statement transform methods."""
        m = self._stmt_methods.get(name)
        if m is None:
            m = getattr(self.stmt_transformer, name, None)
            self._stmt_methods[name] = m
        return m

    def _get_expr_method(self, name):
        """Cache-backed lookup for expression transform methods."""
        m = self._expr_methods.get(name)
        if m is None:
            m = getattr(self.expr_transformer, name, None)
            self._expr_methods[name] = m
        return m

    def transform(self, node):
        if isinstance(node, Program):
            return self._transform_program(node)
        elif isinstance(node, (Import, ImportStmt, FromImport)):
            return self.stmt_transformer.transform(node)
        elif isinstance(node, Module):
            return self._transform_module(node)
        elif isinstance(node, Stmt):
            return self.stmt_transformer.transform(node)
        elif isinstance(node, Expr):
            return self.expr_transformer.transform(node)
        else:
            try:
                return self.expr_transformer.transform(node)
            except NotImplementedError:
                return self._transform_legacy(node)
    
    def _transform_program(self, program):
        self.expr_transformer.hoisted_functions = []
        lines = []
        for stmt in program.statements:
            code = self.transform(stmt)
            if code and code.strip():
                lines.append(code)
        body = "\n".join(lines)

        decorators, identifiers, has_dict, has_enum, has_label = self._scan_ast(program)

        preludes = []
        if prelude_needed(decorators):
            preludes.append(PRELUDE)
        if stdlib_prelude_needed(identifiers):
            preludes.append(STDLIB_PRELUDE)
        if has_dict:
            preludes.append(DICT_PRELUDE)
        if has_enum:
            preludes.append(ENUM_PRELUDE)
        if has_label:
            preludes.append(LABEL_PRELUDE)

        hoisted = self.expr_transformer.hoisted_functions
        if hoisted:
            preludes.append("\n".join(hoisted))

        if preludes:
            return "\n".join(preludes) + "\n" + body
        return body

    def _scan_ast(self, node):
        """Single-pass collection of decorator names, identifiers, dict/enum/label presence."""
        decorators = []
        identifiers = set()
        has_dict = False
        has_enum = False
        has_label = False

        def visit(value):
            nonlocal has_dict, has_enum, has_label
            if value is None:
                return
            if isinstance(value, Decorator):
                decorators.append(value.name)
            elif isinstance(value, Identifier):
                identifiers.add(value.name)
            elif isinstance(value, DictLiteral):
                has_dict = True
            elif isinstance(value, EnumDecl):
                has_enum = True
            elif isinstance(value, (ForStmt, WhileStmt, UntilStmt, LoopStmt)) and value.label:
                has_label = True
            elif isinstance(value, (BreakStmt, ContinueStmt)) and value.label:
                has_label = True
            elif isinstance(value, dict):
                for item in value.values():
                    visit(item)
                return
            elif isinstance(value, (list, tuple)):
                for item in value:
                    visit(item)
                return
            elif isinstance(value, Node):
                for item in vars(value).values():
                    visit(item)

        visit(node)
        return decorators, identifiers, has_dict, has_enum, has_label

    def _transform_module(self, node):
        return self.stmt_transformer.transform(node)
    
    def _transform_legacy(self, node):
        if hasattr(node, 'value') and isinstance(node.value, (Identifier, IntLiteral, FloatLiteral, StrLiteral)):
            return self.expr_transformer.transform(node.value)
        if isinstance(node, Identifier):
            return self.expr_transformer.transform(node)
        if isinstance(node, (IntLiteral, FloatLiteral, StrLiteral)):
            return self.expr_transformer.transform(node)
        raise NotImplementedError(f"Transformer for {node.__class__.__name__} not implemented")

