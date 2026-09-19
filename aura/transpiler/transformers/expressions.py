"""Expression transformers: convert AST expression nodes to Python code."""
import keyword

from aura.transpiler.ast import *


def catch_type_name(exc_type):
    """Map an Aura catch type to its Python spelling.

    Aura's exception root is ``Error``; Python's is ``Exception``. Mapping it
    here means ``catch Error`` works whether or not the program also triggers
    the ``Error = Exception`` prelude alias. A missing type means "catch
    everything".
    """
    if not exc_type:
        return 'Exception'
    if exc_type == 'Error':
        return 'Exception'
    return exc_type


def py_safe_name(name):
    """Return a Python-safe spelling of an Aura identifier.

    Aura allows identifiers that are reserved words in Python (``raise``,
    ``class``, ``lambda``, ...). Emitting them verbatim produces invalid
    Python, so such names get a trailing underscore. The mapping is a pure
    function of the name, which keeps declarations and every use consistent
    without needing scope tracking.
    """
    if not name.isidentifier():
        return name
    if name in ("True", "False", "None"):
        # These spell Python's literals and are already valid as emitted.
        return name
    if keyword.iskeyword(name):
        return f"{name}_"
    return name


def mangle_member(owner, name, visibility):
    """Return the runtime name a member is stored/called under.

    ``owner`` is the class that *declares* the member. Mangling is owner-aware
    so a private member keeps the defining class's prefix even when referenced
    from a subclass (Python's own ``__name`` mangling would use the *current*
    class and break the lookup).

    * ``public``    -> ``name``
    * ``protected`` -> ``_name``
    * ``private``   -> ``_{owner}__name``

    The name is additionally spelled safely for Python (a keyword like
    ``raise`` becomes ``raise_``) so the attribute is always valid.
    """
    if visibility == 'private':
        return f"_{owner}__{py_safe_name(name)}"
    if visibility == 'protected':
        return f"_{py_safe_name(name)}"
    return py_safe_name(name)


class ExpressionTransformer:
    def __init__(self):
        self.member_visibilities = {} # member_name -> (visibility, owner)
        self.known_method_names = set() # every method name defined in any class
        self._lambda_counter = 0
        self.hoisted_functions = []  # generated module-level defs
        # Set when an adaptive `...value` call needs the `_aura_call` helper.
        self._needs_aura_call = False
        # Prelude needs recorded *during* transformation, so the transformer
        # does not need a second full AST walk to decide what to inject.
        self.seen_identifiers = set()
        self.used_decorators = []
        self.has_dict = False
        # Set when `??`/`?:` are used and the coalescing prelude is required.
        self.uses_coalesce = False
        # Set when an open-ended range (`0..`) is used; needs `import itertools`.
        self.uses_infinite_range = False
        # Module-scope resolution. A module transpiles to a class whose data
        # members (const/let/static) live on the class, so a bare reference to
        # one inside a module function must become `Module.name`. Each stack
        # frame maps a visible module-level name to its owning module path.
        self._module_scopes = []
        # module_name -> {member_name: mangled_name} for non-exported members,
        # so a private member declared in a module is renamed everywhere the
        # module's own code refers to it.
        self._module_privates = {}
        # Names bound locally (params, locals, loop vars) in the current
        # function, which shadow module members and must not be rewritten.
        self._local_scopes = []
        # Compile-time macros (AuraMacroFactory). A bare-identifier call whose
        # name is registered here is expanded to its replacement AST before any
        # ordinary call codegen. Defaults to the built-in registry.
        from aura.transpiler.macro_factory import default_registry
        self.macro_registry = default_registry()
        # Guards a macro whose expansion contains another call to itself.
        self._macro_depth = 0

    # Aura string/collection methods with a direct Python method equivalent.
    # Only applied to *calls* on members that are not user-defined methods.
    METHOD_ALIASES = {
        'starts_with': 'startswith',
        'ends_with': 'endswith',
        'to_upper': 'upper',
        'to_lower': 'lower',
        'trim': 'strip',
        'trim_left': 'lstrip',
        'trim_right': 'rstrip',
        'capitalize_words': 'title',
        'index_of': 'find',
        'last_index_of': 'rfind',
        'is_alpha': 'isalpha',
        'is_alphanumeric': 'isalnum',
        'is_digit': 'isdigit',
        'is_numeric': 'isnumeric',
        'is_space': 'isspace',
        'is_lower': 'islower',
        'is_upper': 'isupper',
        'to_title': 'title',
    }

    def transform(self, node):
        if node is None:
            return "None"

        method_name = f"transform_{node.__class__.__name__}"
        method = getattr(self, method_name, None)
        if method:
            return method(node)

        raise NotImplementedError(f"No transformer for {node.__class__.__name__}")

    # ========== Literals ==========
    def transform_IntLiteral(self, node):
        return str(node.value)

    def transform_FloatLiteral(self, node):
        return str(node.value)

    def transform_StrLiteral(self, node):
        raw = getattr(node, 'raw_literal', None)
        if raw is not None:
            return raw
        return repr(node.value)

    def transform_FStringLiteral(self, node):
        pieces = []
        quote = getattr(node, 'quote', '"')
        for part in node.parts:
            if isinstance(part, str):
                pieces.append(self._escape_fstring_text(part, quote))
            else:
                expr, fmt_spec = part
                pieces.append("{" + self.transform(expr) + fmt_spec + "}")
        return f'f{quote}{"".join(pieces)}{quote}'

    @staticmethod
    def _escape_fstring_text(text, quote='"'):
        """Escape literal f-string text for the chosen output quote.

        The tokenizer preserves the source's escape sequences verbatim, so
        existing backslashes must be kept as-is; only real control characters,
        the output delimiter and braces need handling. Doubling every
        backslash (as an earlier version did) corrupted `\\'` inside
        single-quoted f-strings.
        """
        out = text.replace('{', '{{').replace('}', '}}')
        out = (out.replace('\n', '\\n')
                  .replace('\r', '\\r')
                  .replace('\t', '\\t'))
        # Escape an unescaped output delimiter. A delimiter preceded by an odd
        # number of backslashes is already escaped.
        if quote in out:
            chars = []
            backslashes = 0
            for ch in out:
                if ch == '\\':
                    backslashes += 1
                    chars.append(ch)
                    continue
                if ch == quote and backslashes % 2 == 0:
                    chars.append('\\')
                chars.append(ch)
                backslashes = 0
            out = ''.join(chars)
        return out

    def transform_BoolLiteral(self, node):
        return "True" if node.value else "False"

    def transform_NoneLiteral(self, node):
        return "None"

    def transform_ListLiteral(self, node):
        items = [self.transform(e) for e in node.elements]
        return f"[{', '.join(items)}]"

    def transform_DictLiteral(self, node):
        self.has_dict = True
        items = []
        for item in node.pairs:
            if isinstance(item, tuple):
                key, value = item
                k = self.transform(key)
                v = self.transform(value)
                items.append(f"{k}: {v}")
            elif isinstance(item, SpreadExpr):
                # **expr
                expr = self.transform(item.expr)
                items.append(f"**{expr}")
            else:
                # Fallback?
                pass

        return "AuraDict({" + ", ".join(items) + "})"

    def transform_SetLiteral(self, node):
        parts = []
        for e in node.elements:
            if isinstance(e, SpreadExpr):
                parts.append(f"*{self.transform(e.expr)}")
            else:
                parts.append(self.transform(e))
        return "{" + ", ".join(parts) + "}"

    def transform_TupleLiteral(self, node):
        items = [self.transform(e) for e in node.elements]
        if len(items) == 1:
            return f"({items[0]},)"
        return f"({', '.join(items)})"

    # ========== Identifiers & Variables ==========
    def transform_Identifier(self, node):
        self.seen_identifiers.add(node.name)
        # A bare reference to a module-level data member resolves through the
        # module class, mirroring how `Module.member` is spelled from outside.
        if self._module_scopes and not self._is_shadowed(node.name):
            for scope in reversed(self._module_scopes):
                owner = scope.get(node.name)
                if owner is not None:
                    # A private (non-exported) member has a mangled runtime
                    # name; an internal reference must use it.
                    mangled = self._module_privates.get(owner, {}).get(node.name)
                    if mangled is not None:
                        return f"{owner}.{mangled}"
                    return f"{owner}.{py_safe_name(node.name)}"
        return py_safe_name(node.name)

    def _is_shadowed(self, name):
        """True when ``name`` is bound locally in any enclosing function."""
        return any(name in scope for scope in self._local_scopes)

    # ========== Expressions ==========
    def transform_BinaryOp(self, node):
        left = self.transform(node.left)
        right = self.transform(node.right)

        if node.op == '??':
            self.uses_coalesce = True
            return f"_aura_null_coalesce({left}, {right})"
        elif node.op == '?:':
            self.uses_coalesce = True
            return f"_aura_elvis({left}, {right})"
        elif node.op == '?.':
            # Safe navigation: left?.right
            # Right is expected to be an identifier (member identifier).
            # But the parser returns an Expression (Identifier).
            # We want: (left.right if left is not None else None)
            # BUT if right is NOT just an identifier, e.g. a method call?
            # user?.get_address() -> (user.get_address() if user is not None else None)
            # The parser parsed RHS as expression.
            # In BinaryOp, RHS is just the expression node.
            # If we simply emit left.right, it works if expression stringification handles it.
            # Wait!
            # If the parser parsed `user?.address` as BinaryOp(user, '?.', address).
            # The `right` transformation (self.transform(node.right)) simply returns "address".
            # So `left`="user", `right`="address".
            # Result: `(user.address if user is not None else None)`.
            # Note: We must ensure we emit `.` between left and right.
            # In normal member access `.` is op.
            # Here `?.` is op.
            return f"({left}.{right} if {left} is not None else None)"
        elif node.op == 'as':
            # Cast: left as right -> right(left)
            # right is usually identifier (from parse_type -> Identifier)
            # e.g. int -> int(left)
            return f"{right}({left})"

        op_map = {
            '+': '+', '-': '-', '*': '*', '/': '/', '%': '%', '**': '**',
            '&': '&', '|': '|', '^': '^', '<<': '<<', '>>': '>>',
            'and': 'and', 'or': 'or',
            '==': '==', '!=': '!=', '<': '<', '>': '>', '<=': '<=', '>=': '>=',
            'is': 'is', 'is not': 'is not', 'in': 'in', 'not in': 'not in',
        }
        py_op = op_map.get(node.op, node.op)
        return f"({left} {py_op} {right})"

    # ========== Unary Operations ==========
    def transform_UnaryOp(self, node):
        if node.op == 'yield':
            # Python rejects a `yield` outside a function, so emitting one at
            # module level would produce code that cannot compile. The rule
            # checker reports this as E004; this guard keeps the transformer's
            # own invariant ("output is always valid Python") even when it is
            # invoked directly, without the checker. A standalone transformer
            # (no statement transformer attached) is treated as a fragment and
            # left alone, which keeps `transform_UnaryOp` unit-testable.
            stmt = getattr(self, 'stmt_transformer', None)
            if stmt is not None and not getattr(stmt, 'function_scopes', None):
                raise SyntaxError(
                    "'yield' outside of a function; 'yield' is only valid "
                    "inside a 'def' (it makes that function a generator)")
            if node.operand is None:
                return "yield"
            operand = self.transform(node.operand)
            # A tuple already renders with its own parentheses, so it needs no
            # extra wrapper; a bare `yield a, b` would otherwise yield a tuple
            # at a surprising precedence.
            return f"yield {operand}"
        operand = self.transform(node.operand)
        op_map = {
            '-': '-', '+': '+', 'not': 'not', '~': '~', 'await': 'await'
        }
        py_op = op_map.get(node.op, node.op)
        return f"({py_op} {operand})"

    # ========== Function Calls ==========
    def _maybe_expand_macro(self, node, render=True):
        """Expand ``node`` when it calls a registered compile-time macro.

        Returns the rendered Python for the expansion, or ``None`` when the
        call is not a macro invocation. Keyword arguments are passed through to
        the macro implementation; operands are *not* pre-transformed, so a
        macro sees raw AST and may choose to quote an argument (inspect it)
        rather than evaluate it.

        With ``render=False`` the replacement AST node is returned instead of
        its Python string, which lets the statement transformer inspect the
        expansion (e.g. to emit a block's statements directly).
        """
        registry = getattr(self, 'macro_registry', None)
        if registry is None or not len(registry):
            return None
        if not isinstance(node.func, Identifier):
            return None
        if node.func.name not in registry:
            return None
        # A name the program declares itself (function, class, import) shadows
        # the built-in macro of the same name, so `def swap(...)` is a normal
        # call rather than an expansion.
        if node.func.name in getattr(self, 'user_declared_names', set()):
            return None
        if self._is_shadowed(node.func.name):
            return None
        if self._macro_depth > 100:
            from aura.transpiler.macro_factory import MacroError
            raise MacroError(
                f"macro '{node.func.name}' expanded too deeply (recursive?)")
        from aura.transpiler.macro_factory import MacroError
        try:
            self._macro_depth += 1
            replacement = registry.expand_call(node)
        except MacroError:
            raise
        finally:
            self._macro_depth -= 1
        if replacement is None:
            return None
        if not render:
            return replacement
        return self.transform(replacement)

    def transform_CallExpr(self, node):
        # Compile-time macro expansion (AuraMacroFactory). A registered bare
        # name is replaced by its expansion before ordinary call codegen.
        expansion = self._maybe_expand_macro(node)
        if expansion is not None:
            return expansion

        # Aura method conveniences: str.length(), str.is_empty(), str.contains(x),
        # str.slice(a, b), plus the alias table above.
        if isinstance(node.func, MemberExpr):
            member = node.func.member
            # Adaptive `...value` as the sole argument of a member call:
            # resolve dict-vs-iterable at runtime, same as a bare-name call.
            adaptive = [a for a in node.args if isinstance(a, SpreadExpr)
                        and a.is_dict is None]
            if len(adaptive) == 1 and len(node.args) == 1 and not node.kwargs:
                target = self.transform(node.func)
                value = self.transform(adaptive[0].expr)
                self._needs_aura_call = True
                return f"_aura_call({target}, {value})"
            # Aura protocol methods keep their bare name at the call site
            # (`obj.str()`, `obj.eq(other)`) but are defined and called by their
            # Python dunder name (`__str__`, `__eq__`). Constructors are always
            # `__init__`.
            dunder = python_method_name(member)
            known_member = (dunder in self.member_visibilities
                            or dunder in self.known_method_names
                            or dunder == '__init__')
            obj = self._render_object(node.func.obj)
            args = [self.transform(a) for a in node.args]
            kwargs = [f"{k}={self.transform(v)}" for k, v in node.kwargs.items()]

            if known_member:
                all_args = args + kwargs
                info = self.member_visibilities.get(dunder)
                vis, owner = (info if info else ('public', None))
                dunder = mangle_member(owner, dunder, vis)
                return f"{obj}.{dunder}({', '.join(all_args)})"

            if member == 'length' and not args:
                return f"len({obj})"
            if member in ('size', 'len') and not args:
                # Collection length convenience, Kof-style (`list.size()`) and
                # Python-style (`list.len()`); both mean `len(list)`. A user
                # method with the same name is a `known_member` and wins above.
                return f"len({obj})"
            if member == 'is_empty' and not args:
                return f"(not {obj})"
            if member == 'contains' and len(args) == 1:
                return f"({args[0]} in {obj})"
            if member == 'add' and len(args) == 1:
                # `list.add(x)` appends, matching Kof's List API. A user class
                # that declares `add` is a `known_member` and keeps its method.
                return f"{obj}.append({args[0]})"
            if member == 'slice':
                if len(args) == 1:
                    return f"{obj}[{args[0]}:]"
                if len(args) == 2:
                    return f"{obj}[{args[0]}:{args[1]}]"
            if member == 'char_at' and len(args) == 1:
                return f"{obj}[{args[0]}]"
            if member in self.METHOD_ALIASES:
                py_member = self.METHOD_ALIASES[member]
                all_args = args + kwargs
                return f"{obj}.{py_member}({', '.join(all_args)})"

            # Unknown member (e.g. a native Python module/object method):
            # emit the name verbatim.
            all_args = args + kwargs
            return f"{obj}.{member}({', '.join(all_args)})"

        func = self.transform(node.func)

        # Aura spells a parent constructor call as `super(args)`, which maps to
        # Python's `super().__init__(args)`. Without this, `super(msg)` would be
        # emitted verbatim and fail at runtime ("super() argument 1 must be a
        # type"), since Python's `super` takes no constructor arguments here.
        if isinstance(node.func, Identifier) and node.func.name == 'super':
            args = self._render_call_args(node)
            return f"super().__init__({', '.join(args)})"

        # Adaptive `...value` as the single argument: unpack as keyword
        # arguments when the value is a mapping, otherwise as positional
        # arguments. This is resolved at runtime by the `_aura_call` helper.
        adaptive = [a for a in node.args if isinstance(a, SpreadExpr)
                    and a.is_dict is None]
        if len(adaptive) == 1 and len(node.args) == 1 and not node.kwargs:
            value = self.transform(adaptive[0].expr)
            self._needs_aura_call = True
            return f"_aura_call({func}, {value})"

        # A call to a private (non-exported) module member from inside the
        # module must use its mangled runtime name.
        if isinstance(node.func, Identifier) and not self._is_shadowed(node.func.name):
            for owner, privates in reversed(list(self._module_privates.items())):
                mangled = privates.get(node.func.name)
                if mangled is not None:
                    target = f"{owner}.{mangled}"
                    all_args = self._render_call_args(node)
                    return f"{target}({', '.join(all_args)})"

        all_args = self._render_call_args(node)
        return f"{func}({', '.join(all_args)})"

    def _render_call_args(self, node):
        """Render positional, keyword and spread call arguments.

        Adaptive spreads (``...value``) that are not the sole argument fall
        back to positional unpacking; use ``**value`` to force keyword
        unpacking.
        """
        rendered = []
        for arg in node.args:
            if isinstance(arg, SpreadExpr):
                prefix = '*' if arg.is_dict is False or arg.is_dict is None else '**'
                rendered.append(f"{prefix}{self.transform(arg.expr)}")
            else:
                rendered.append(self.transform(arg))
        for key, value in node.kwargs.items():
            rendered.append(f"{py_safe_name(key)}={self.transform(value)}")
        return rendered

    # ========== Indexing & Member Access ==========
    def transform_IndexExpr(self, node):
        obj = self.transform(node.obj)
        # A range used as an index is a slice, not a key: `a[0..<3]` means
        # `a[0:3]` (the exclusive `..<` bound becomes the Python stop, and the
        # inclusive `..` bound becomes `stop + 1`). Emitting `a[range(...)]`
        # would raise TypeError at runtime.
        if isinstance(node.index, RangeExpr):
            index = self._range_as_slice(node.index)
            return f"{obj}[{index}]"
        index = self.transform(node.index)
        return f"{obj}[{index}]"

    def _range_as_slice(self, node):
        """Render a range expression as a Python slice fragment."""
        start = self.transform(node.start) if node.start is not None else ""
        if node.end is None:
            stop = ""  # `a[1..]` -> `a[1:]`
        else:
            stop = self.transform(node.end)
            if not node.exclusive:
                # Inclusive upper bound: include the end element.
                stop = f"({stop}) + 1"
        if node.step is not None:
            step = self.transform(node.step)
            return f"{start}:{stop}:{step}"
        return f"{start}:{stop}"

    def transform_SliceExpr(self, node):
        obj = self.transform(node.obj)
        start = self.transform(node.start) if node.start is not None else ""
        stop = self.transform(node.stop) if node.stop is not None else ""
        if node.step is not None:
            step = self.transform(node.step)
            return f"{obj}[{start}:{stop}:{step}]"
        return f"{obj}[{start}:{stop}]"

    def _render_object(self, node):
        """Render an expression used as the object of a member access.

        Aura's zero-argument `super` (as in `super.new(...)` / `super.hi()`)
        maps to Python's `super()`, which requires the call. A bare `super`
        identifier would otherwise be emitted as the built-in class, producing
        `type object 'super' has no attribute ...`.
        """
        if isinstance(node, Identifier) and node.name == 'super':
            return 'super()'
        return self.transform(node)

    def transform_MemberExpr(self, node):
        obj = self._render_object(node.obj)
        member = node.member
        # Only rewrite a bare member reference when it is a known protocol
        # *method*; a field that happens to be named `len`/`str`/... is left
        # alone.
        dunder = python_method_name(member)
        if dunder != member and dunder in self.known_method_names:
            member = dunder
        info = self.member_visibilities.get(member)
        vis, owner = (info if info else ('public', None))
        member = mangle_member(owner, member, vis)
        return f"{obj}.{member}"

    # ========== Null-Safe Operations ==========
    def transform_SafeNavExpr(self, node):
        # For Python, translate to: (obj.member if obj is not None else None)
        obj = self.transform(node.obj)
        if node.is_index:
            index = self.transform(node.member_or_index)
            return f"({obj}[{index}] if {obj} is not None else None)"
        else:
            member = node.member_or_index
            dunder = python_method_name(member)
            if dunder != member and dunder in self.known_method_names:
                member = dunder
            info = self.member_visibilities.get(member)
            vis, owner = (info if info else ('public', None))
            member = mangle_member(owner, member, vis)
            return f"({obj}.{member} if {obj} is not None else None)"

    # ========== Pipe Operator ==========
    def transform_PipeExpr(self, node):
        # left |> right: transpile as right(left)
        left = self.transform(node.left)
        # right might be a function call or identifier
        if isinstance(node.right, CallExpr):
            # Insert left as first arg
            call = node.right
            new_args = [node.left] + call.args
            new_call = CallExpr(call.func, new_args, call.kwargs)
            return self.transform(new_call)
        else:
            # Simple function: apply it
            right = self.transform(node.right)
            return f"{right}({left})"

    # ========== Ternary & Coalescing ==========
    def transform_CondExpr(self, node):
        cond = self.transform(node.condition)
        true_expr = self.transform(node.true_expr)
        false_expr = self.transform(node.false_expr)
        return f"({true_expr} if {cond} else {false_expr})"

    def transform_CoalesceExpr(self, node):
        # value ?? default → value if value is not None else default
        self.uses_coalesce = True
        value = self.transform(node.value)
        default = self.transform(node.default)
        return f"_aura_null_coalesce({value}, {default})"

    def transform_IfStmt(self, node):
        # Handle if-expression: if cond { expr } else { expr }
        # This requires extracting the value from the block.
        cond = self.transform(node.condition)

        def extract_value(stmts):
            if not stmts: return "None"
            # Return last statement if it's an expression or ExprStmt
            last = stmts[-1]
            if isinstance(last, ReturnStmt):
                return self.transform(last.value)
            elif isinstance(last, ExprStmt):
                return self.transform(last.expr)
            elif isinstance(last, Expr):
                return self.transform(last)
            else:
                # Fallback for complex blocks in expression position
                # Ideally this would wrap in a lambda, but for simple cases:
                return "None"

        true_val = extract_value(node.then_body)
        false_val = extract_value(node.else_body) if node.else_body else "None"

        return f"({true_val} if {cond} else {false_val})"

    # ========== Range Expressions ==========
    def transform_RangeExpr(self, node):
        start = self.transform(node.start)
        if node.end is None:
            # Infinite range (`0..`): an unbounded counter. The transformer
            # injects `import itertools` for this form.
            self.uses_infinite_range = True
            return f"itertools.count({start})"
        end = self.transform(node.end)
        end_val = f"{end}" if node.exclusive else f"{end} + 1"

        if node.step:
            step = self.transform(node.step)
            return f"range({start}, {end_val}, {step})"
        return f"range({start}, {end_val})"

    # ========== Lambda ==========
    def _render_param(self, param):
        """Render one lambda parameter, honouring `*args`/`**kwargs`/bare `*`."""
        if param.name == '*' and not param.is_variadic and not param.is_kwonly:
            return '*'
        if param.is_kwonly:
            return f"**{py_safe_name(param.name)}"
        if param.is_variadic:
            return f"*{py_safe_name(param.name)}"
        if param.default:
            default = self.transform(param.default)
            return f"{py_safe_name(param.name)}={default}"
        return py_safe_name(param.name)

    def transform_LambdaExpr(self, node):
        params = ", ".join(self._render_param(p) for p in node.params)
        if isinstance(node.body, BlockExpr):
            # Block lambdas may contain statements/return, which cannot live
            # inside a Python lambda expression. Hoist a real function instead.
            return self._hoist_block_lambda(node.params, node.body)
        body = self.transform(node.body)
        return f"(lambda {params}: {body})"

    def _make_stmt_transformer(self):
        """Create a statement transformer that shares expression/class state.

        Hoisted lambdas and block expressions must see the surrounding class
        context (member visibility, known method names) so aliases such as
        ``length()`` keep resolving correctly inside them.
        """
        from aura.transpiler.transformers.statements import StatementTransformer
        stmt_transformer = StatementTransformer()
        stmt_transformer.indent_level = 0
        stmt_transformer.expr_transformer = self
        # Share class context when available.
        owner = getattr(self, 'stmt_transformer', None)
        if owner is not None:
            stmt_transformer.class_members = owner.class_members
            stmt_transformer.in_class_scope = owner.in_class_scope
        return stmt_transformer

    def _hoist_block_lambda(self, params, block):
        stmt_transformer = self._make_stmt_transformer()
        # Share the enclosing function-scope stack so captured mutable locals
        # can be declared `nonlocal` inside the hoisted function.
        owner = getattr(self, 'stmt_transformer', None)
        if owner is not None:
            stmt_transformer.function_scopes = owner.function_scopes

        self._lambda_counter += 1
        fname = f"_aura_lambda_{self._lambda_counter}"
        param_str = ", ".join(self._render_param(p) for p in params)

        nonlocal_line = self._nonlocal_declaration(block.statements, {p.name for p in params})

        body_code = stmt_transformer._block(block.statements)
        if not body_code.strip():
            body_code = "    pass"
        if nonlocal_line:
            body_code = nonlocal_line + "\n" + body_code
        self.hoisted_functions.append(f"def {fname}({param_str}):\n{body_code}")
        return fname

    def _nonlocal_declaration(self, statements, excluded):
        """Return a `nonlocal ...` line for names mutated in a closure.

        A name needs `nonlocal` when it is assigned inside the closure but not
        declared there (params, local `let`/`const`, nested defs) and it exists
        in an enclosing function scope. Module-level names are omitted because
        `nonlocal` would be a syntax error for them.
        """
        owner = getattr(self, 'stmt_transformer', None)
        if owner is None or not owner.function_scopes:
            return ""
        enclosing = set()
        for scope in owner.function_scopes:
            enclosing |= scope
        assigned = set()
        declared = set(excluded)
        _collect_closure_names(statements, assigned, declared)
        captured = (assigned - declared) & enclosing
        if not captured:
            return ""
        return "    nonlocal " + ", ".join(sorted(captured))

    # ========== Comprehensions ==========
    def transform_ComprehensionExpr(self, node):
        term = ""
        if node.expr_type == 'dict' and isinstance(node.expr, tuple):
             k = self.transform(node.expr[0])
             v = self.transform(node.expr[1])
             term = f"{k}: {v}"
        else:
             term = self.transform(node.expr)

        generator_parts = []
        for pattern, iterable, filters in node.comprehensions:
            pat_str = self.transform(pattern)
            iter_str = self.transform(iterable)

            # `for (k, v) in some_dict` needs `.items()` in Python. Only add it
            # when the iterable is provably dict-shaped; an arbitrary 2-tuple
            # pattern over a list of pairs must be left untouched, otherwise
            # `[k for (k, v) in pairs]` would become `pairs.items()`.
            if (isinstance(pattern, TupleLiteral) and len(pattern.elements) == 2
                    and self._needs_items(iterable, iter_str)):
                iter_str += ".items()"

            part = f"for {pat_str} in {iter_str}"

            for cond in filters:
                part += f" if {self.transform(cond)}"
            generator_parts.append(part)

        generators = " ".join(generator_parts)

        if node.expr_type == 'list':
            return f"[{term} {generators}]"
        elif node.expr_type == 'set' or node.expr_type == 'dict':
            return f"{{{term} {generators}}}"
        elif node.expr_type == 'generator':
            return f"({term} {generators})"
        else:
            raise NotImplementedError(f"Unknown comprehension type: {node.expr_type}")

    @staticmethod
    def _needs_items(iterable, rendered):
        """True when a `for (k, v) in X` iterable must be `.items()`-ed.

        Only provably dict-shaped iterables qualify: a dict/set literal, an
        `AuraDict(...)`/`dict(...)` call, or an expression that already yields
        pairs via `.items()`/`.keys()`. Everything else (lists of pairs,
        tuples, unknown names) is left alone so the pattern just unpacks it.
        """
        if isinstance(iterable, DictLiteral):
            return True
        text = (rendered or '').strip()
        if text.endswith('.items()') or text.endswith('.keys()'):
            return False
        return bool(text.startswith('AuraDict(') or text.startswith('dict('))

    # ========== Spread ==========
    def transform_SpreadExpr(self, node):
        expr = self.transform(node.expr)
        if node.is_dict:
            return f"**{expr}"
        else:
            return f"*{expr}"

    # ========== Match Expression ==========
    def transform_MatchExpr(self, node):
        """Hoist `match` in expression position into a helper function.

        Each case body's final expression (or `return` value) becomes the value
        returned by the helper, so `let x = match n { ... }` works.
        """
        stmt_transformer = self._make_stmt_transformer()
        # Case bodies live one level inside the helper function.
        stmt_transformer.indent_level = 1

        self._lambda_counter += 1
        fname = f"_aura_match_{self._lambda_counter}"
        subject = f"_aura_subject_{self._lambda_counter}"

        lines = [f"def {fname}({subject}):", f"    match {subject}:"]
        has_catch_all = False
        for case in node.cases:
            pattern = stmt_transformer._transform_pattern(case.pattern)
            if isinstance(case.pattern, WildcardPattern) and case.guard is None:
                has_catch_all = True
            if case.guard is not None:
                guard = self.transform(case.guard)
                lines.append(f"        case {pattern} if {guard}:")
            else:
                lines.append(f"        case {pattern}:")

            tail, stmts_to_run = self._case_value(case.body)
            if stmts_to_run:
                body_code = stmt_transformer._block(stmts_to_run)
                if body_code.strip():
                    lines.append(body_code)
            lines.append(f"            return {tail}")

        if not has_catch_all:
            lines.append("        case _:")
            lines.append("            return None")

        self.hoisted_functions.append("\n".join(lines))
        subject_code = self.transform(node.expr)
        return f"{fname}({subject_code})"

    def _case_value(self, body):
        """Return (tail_expression, statements_to_run) for a case body."""
        if not body:
            return "None", []
        statements = list(body)
        last = statements[-1]
        if last.__class__.__name__ == 'ExprStmt':
            return self.transform(last.expr), statements[:-1]
        if last.__class__.__name__ == 'ReturnStmt':
            tail = self.transform(last.value) if last.value else "None"
            return tail, statements[:-1]
        return "None", statements

    # ========== Try Expression ==========
    def transform_TryExpr(self, node):
        """Hoist `try` in expression position into a helper function."""
        stmt_transformer = self._make_stmt_transformer()
        stmt_transformer.indent_level = 1

        self._lambda_counter += 1
        fname = f"_aura_try_{self._lambda_counter}"

        lines = [f"def {fname}():"]
        lines.append("    try:")
        tail, stmts_to_run = self._case_value(node.try_body)
        if stmts_to_run:
            body_code = stmt_transformer._block(stmts_to_run)
            if body_code.strip():
                lines.append(body_code)
        lines.append(f"        return {tail}")

        for catch in node.catch_clauses:
            exc_type = catch_type_name(catch.exception_type)
            var_name = f" as {catch.var_name}" if catch.var_name else ""
            lines.append(f"    except {exc_type}{var_name}:")
            catch_tail, catch_stmts = self._case_value(catch.body)
            if catch_stmts:
                body_code = stmt_transformer._block(catch_stmts)
                if body_code.strip():
                    lines.append(body_code)
            lines.append(f"        return {catch_tail}")

        if node.finally_body is not None:
            lines.append("    finally:")
            body_code = stmt_transformer._block(node.finally_body)
            if body_code.strip():
                lines.append(body_code)

        self.hoisted_functions.append("\n".join(lines))
        return f"{fname}()"

    # ========== Block Expression ==========
    def transform_BlockExpr(self, node):
        # Turn `{ stmt; stmt; last_expr }` into a hoisted zero-arg function
        # that runs the statements and returns the final expression value.
        stmt_transformer = self._make_stmt_transformer()

        if not node.statements:
            return "None"

        statements = list(node.statements)
        last = statements[-1]

        # The final expression becomes the return value.
        if last.__class__.__name__ == 'ExprStmt':
            tail = self.transform(last.expr)
            stmts_to_run = statements[:-1]
        elif last.__class__.__name__ == 'ReturnStmt':
            tail = self.transform(last.value) if last.value else "None"
            stmts_to_run = statements[:-1]
        elif isinstance(last, Expr):
            # A macro may end its block with a bare expression (the value to
            # yield); treat it as the block's result rather than dropping it.
            tail = self.transform(last)
            stmts_to_run = statements[:-1]
        else:
            tail = "None"
            stmts_to_run = statements

        body_lines = []
        for stmt in stmts_to_run:
            code = stmt_transformer.transform(stmt)
            if code:
                for line in code.split('\n'):
                    if line.strip():
                        body_lines.append("    " + line)
        body_lines.append(f"    return {tail}")
        body = "\n".join(body_lines)

        self._lambda_counter += 1
        fname = f"_aura_block_{self._lambda_counter}"
        self.hoisted_functions.append(f"def {fname}():\n{body}")
        return f"{fname}()"

    # ========== Helper: transform pattern for comprehensions ==========
    def _transform_pattern(self, pattern):
        if isinstance(pattern, IdentifierPattern):
            return pattern.name
        elif isinstance(pattern, ListPattern):
            pats = [self._transform_pattern(p) for p in pattern.patterns]
            if pattern.rest_pattern:
                pats.append(f"*{pattern.rest_pattern.name}")
            return f"[{', '.join(pats)}]"
        elif isinstance(pattern, DictPattern):
            fields = []
            for name, pat in pattern.field_patterns.items():
                fields.append(f"{name}={self._transform_pattern(pat)}")
            if pattern.rest_pattern:
                fields.append(f"**{pattern.rest_pattern.name}")
            return f"{{{', '.join(fields)}}}"
        else:
            return "_"  # Wildcard/unknown

    # ========== Patterns (Shared with StatementTransformer) ==========
    def transform_IdentifierPattern(self, node):
        return py_safe_name(node.name)

    def transform_LiteralPattern(self, node):
        return self.transform(node.value)

    def transform_AsPattern(self, node):
        pat = self.transform(node.pattern)
        return f"{pat} as {py_safe_name(node.binding_name)}"

    def transform_WildcardPattern(self, node):
        return "_"

    def transform_ListPattern(self, node):
        pats = [self.transform(p) for p in node.patterns]
        if node.rest_pattern:
            pats.append(f"*{py_safe_name(node.rest_pattern.name)}")
        return f"[{', '.join(pats)}]"

    def transform_DictPattern(self, node):
        fields = []
        for name, pat in node.field_patterns.items():
             fields.append(f'"{name}": {self.transform(pat)}')
        if node.rest_pattern:
             fields.append(f"**{py_safe_name(node.rest_pattern.name)}")
        return f"{{{', '.join(fields)}}}"

    def transform_ConstructorPattern(self, node):
        name = node.name
        args = [self.transform(p) for p in node.subpatterns]
        return f"{name}({', '.join(args)})"

    def transform_OrPattern(self, node):
        pats = [self.transform(p) for p in node.patterns]
        return f"({' | '.join(pats)})"

    def transform_MemberPattern(self, node):
        """Render a dotted member pattern (`Color.RED`) as a value pattern."""
        return self.transform(node.expr)


# ============================================================================
# Closure analysis helpers (used to emit `nonlocal` in hoisted lambdas)
# ============================================================================

def _closure_target_names(target):
    """Yield names bound by an assignment target node."""
    from aura.transpiler.ast import (
        Identifier,
        IndexExpr,
        ListLiteral,
        MemberExpr,
        SpreadExpr,
        TupleLiteral,
    )
    if isinstance(target, Identifier):
        yield target.name
    elif isinstance(target, (TupleLiteral, ListLiteral)):
        for el in target.elements:
            yield from _closure_target_names(el)
    elif isinstance(target, SpreadExpr):
        yield from _closure_target_names(target.expr)
    # Member/index assignments mutate an object, not a local binding.
    elif isinstance(target, (MemberExpr, IndexExpr)):
        return


def _collect_closure_names(statements, assigned, declared):
    """Populate `assigned`/`declared` for a closure body.

    Nested functions/lambdas have their own scope, so their bodies are not
    descended into for *assignments*; their names still count as declared.
    """
    from aura.transpiler.ast import (
        BinaryOp,
        BlockExpr,
        ClassDecl,
        ConstDecl,
        ExprStmt,
        ForStmt,
        FunctionDecl,
        GuardStmt,
        IfStmt,
        LambdaExpr,
        ListLiteral,
        LoopStmt,
        MatchStmt,
        Node,
        ReturnStmt,
        TryStmt,
        TupleLiteral,
        UnlessStmt,
        UntilStmt,
        VarDecl,
        WhileStmt,
        WithStmt,
    )

    ASSIGN_OPS = {
        '=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>=', '??=',
    }

    def walk(node):
        if node is None:
            return
        if isinstance(node, list):
            for item in node:
                walk(item)
            return
        if isinstance(node, VarDecl):
            for n in _decl_names(node.name):
                declared.add(n)
            walk(node.value)
        elif isinstance(node, ConstDecl):
            declared.add(node.name)
            walk(node.value)
        elif isinstance(node, (FunctionDecl, ClassDecl)):
            declared.add(node.name)
        elif isinstance(node, LambdaExpr):
            # A nested lambda's assignments belong to its own scope.
            for p in node.params:
                declared.add(p.name)
            walk(node.body)
        elif isinstance(node, ExprStmt):
            e = node.expr
            if isinstance(e, BinaryOp) and e.op in ASSIGN_OPS:
                for n in _closure_target_names(e.left):
                    assigned.add(n)
                walk(e.right)
            elif isinstance(e, (TupleLiteral, ListLiteral)) and any(
                isinstance(el, BinaryOp) and el.op in ASSIGN_OPS for el in e.elements
            ):
                for el in e.elements:
                    if isinstance(el, BinaryOp) and el.op in ASSIGN_OPS:
                        for n in _closure_target_names(el.left):
                            assigned.add(n)
                        walk(el.right)
                    else:
                        for n in _closure_target_names(el):
                            assigned.add(n)
            else:
                walk(e)
        elif isinstance(node, ForStmt):
            for n in _pattern_names(node.pattern):
                declared.add(n)
            walk(node.iterable)
            walk(node.body)
        elif isinstance(node, WithStmt):
            for expr, var in node.items:
                walk(expr)
                if var:
                    declared.add(var)
            walk(node.body)
        elif isinstance(node, TryStmt):
            walk(node.try_body)
            for clause in (node.catch_clauses or []):
                if clause.var_name:
                    declared.add(clause.var_name)
                walk(clause.body)
            walk(node.finally_body)
        elif isinstance(node, MatchStmt):
            walk(node.expr)
            for case in node.cases:
                walk(case.body)
        elif isinstance(node, (IfStmt, UnlessStmt)):
            body = getattr(node, 'then_body', None) or getattr(node, 'body', None)
            walk(body)
            walk(node.else_body)
        elif isinstance(node, (WhileStmt, UntilStmt, LoopStmt, GuardStmt)):
            for attr in ('body', 'else_body', 'condition'):
                walk(getattr(node, attr, None))
        elif isinstance(node, ReturnStmt):
            walk(node.value)
        elif isinstance(node, BlockExpr):
            walk(node.statements)
        elif isinstance(node, Node):
            for value in vars(node).values():
                if isinstance(value, (Node, list)):
                    walk(value)

    walk(statements)


def _decl_names(name):
    from aura.transpiler.transformers.statements import StatementTransformer
    return StatementTransformer._declared_names(name)


def _pattern_names(pattern):
    from aura.transpiler.ast import (
        Identifier,
        IdentifierPattern,
        ListLiteral,
        ListPattern,
        SpreadExpr,
        TupleLiteral,
    )
    if pattern is None:
        return []
    if isinstance(pattern, IdentifierPattern):
        return [pattern.name]
    if isinstance(pattern, Identifier):
        return [pattern.name]
    if isinstance(pattern, (TupleLiteral, ListLiteral)):
        names = []
        for el in pattern.elements:
            names.extend(_pattern_names(el))
        return names
    if isinstance(pattern, ListPattern):
        names = []
        for sub in pattern.patterns:
            names.extend(_pattern_names(sub))
        if pattern.rest_pattern is not None:
            names.extend(_pattern_names(pattern.rest_pattern))
        return names
    if isinstance(pattern, SpreadExpr):
        return _pattern_names(pattern.expr)
    return []
