"""Aura REPL engine.

The REPL shares the real parser, transformer and semantic rules with the rest
of the toolchain, so what you type behaves exactly like what ``aura run``
executes. It supports:

* persistent state across lines (variables, functions, classes, imports);
* value echoing — a bare expression prints the resulting value, and the result
  is also stored in ``_``;
* automatic multi-line entry: input continues while brackets are unbalanced or
  the line ends with a continuation token;
* semantic rule enforcement (mutability is relaxed to *mutable by default* for
  interactive bindings, but structural rules still apply);
* built-in commands (``:help``, ``:vars``, ``:type``, ``:ast``, ``:py``,
  ``:load``, ``:run``, ``:reset``, ``:history``);
* direct Python execution through ``:py`` and the ``python`` bridge.

The engine is importable and testable without touching ``stdin``/``stdout`` by
injecting ``input_func``/``output_func``.
"""

from __future__ import annotations

from aura.parser.to_ast import Tokenizer, Parser, parse_file
from aura.transpiler.ast import Program
from aura.transpiler.transformer import Transformer
from aura.transpiler.rules import RuleChecker


BANNER = (
    "Aura REPL v0.4  •  {}  •  ':help' for commands, ':q' to quit\n"
).format("transpile to Python")


class ReplResult:
    """Outcome of processing one chunk of input."""

    __slots__ = ('ok', 'value', 'continued', 'message', 'exception')

    def __init__(self, ok=True, value=None, continued=False, message=None,
                 exception=None):
        self.ok = ok
        self.value = value
        self.continued = continued
        self.message = message
        self.exception = exception


class AuraREPL:
    """Interactive Aura read-eval-print loop."""

    def __init__(self, input_func=None, output_func=None):
        self.namespace = {'__name__': '__aura_repl__', '__builtins__': __builtins__}
        self.transformer = Transformer()
        self.buffer = ""
        self._history = []
        self._counter = 0
        self.prompt = "aura> "
        self.cont_prompt = "....> "
        self._input = input_func or input
        self._output = output_func or (lambda text: print(text))
        self._install_python_alias()

    @property
    def locals(self):
        """Backwards-compatible alias for the session namespace."""
        return self.namespace

    @locals.setter
    def locals(self, value):
        self.namespace = value

    # -- plumbing -----------------------------------------------------------

    def _install_python_alias(self):
        """Make `import python` available inside the REPL namespace."""
        from aura.runtime import install_runtime_aliases
        install_runtime_aliases()
        try:
            import python  # noqa: F401  (registered alias)
            self.namespace['python'] = python
        except Exception:
            pass

    def write(self, text=""):
        self._output(text)

    # -- public entry point -------------------------------------------------

    def run(self):
        self.write("Aura REPL v0.4 (type ':help' for help, ':q' to quit)")
        while True:
            prompt = self.cont_prompt if self.buffer else self.prompt
            try:
                line = self._input(prompt)
            except EOFError:
                if self.buffer:
                    # Flush whatever is buffered before exiting.
                    self.process_buffer()
                    self.buffer = ""
                    continue
                self.write("")
                break
            except KeyboardInterrupt:
                self.write("\nKeyboardInterrupt (buffer cleared)")
                self.buffer = ""
                continue

            result = self.feed(line)
            if result is not None and not result.continued and result.message:
                self.write(result.message)
        return 0

    def feed(self, line):
        """Feed one line of input, handling buffering and command dispatch.

        Returns a :class:`ReplResult`, or ``None`` for an empty line while not
        in a continuation.
        """
        if not self.buffer and line.startswith(':'):
            return self.handle_command(line.strip())

        if not self.buffer and not line.strip():
            return None

        self.buffer += line + "\n"

        if self._is_incomplete():
            return ReplResult(continued=True)

        text = self.buffer
        self.buffer = ""
        return self.process(text)

    # -- continuation detection --------------------------------------------

    def _is_incomplete(self):
        """True while the buffered source cannot be parsed yet.

        The primary signal is structural: unbalanced brackets or an unclosed
        string keep the buffer open. A trailing continuation token also keeps
        it open, but only when the buffer does not already parse cleanly, so
        ``let x = `` continues while a complete chunk is executed immediately.
        """
        depth = 0
        in_string = None
        escaped = False
        for ch in self.buffer:
            if escaped:
                escaped = False
                continue
            if ch == '\\':
                escaped = True
                continue
            if in_string:
                if ch == in_string:
                    in_string = None
                continue
            if ch in ('"', "'"):
                in_string = ch
            elif ch in '([{':
                depth += 1
            elif ch in ')]}':
                depth -= 1

        if depth > 0 or in_string is not None:
            return True

        stripped = self.buffer.rstrip()
        if not stripped:
            return False
        if stripped.endswith('\\'):
            self.buffer = stripped[:-1] + "\n"
            return True

        # When it already parses, it is complete regardless of trailing tokens.
        if self._parses_cleanly():
            return False

        open_ending = ('=', '+', '-', '*', '/', '%', '|', '&', ',', '->',
                       '=>', 'and', 'or', 'not', 'in', 'is')
        return any(stripped.endswith(tok) for tok in open_ending)

    def _parses_cleanly(self):
        try:
            self._parse_program(self.buffer)
            return True
        except Exception:
            return False

    # -- command dispatch ---------------------------------------------------

    def handle_command(self, command):
        parts = command.split(maxsplit=1)
        name = parts[0]
        arg = parts[1] if len(parts) > 1 else ""

        if name in (':q', ':quit', ':exit'):
            self.write("Goodbye!")
            raise EOFError
        if name == ':help':
            self.write(HELP_TEXT)
            return ReplResult()
        if name in (':vars', ':v'):
            self.write(self._format_vars())
            return ReplResult()
        if name == ':reset':
            self.namespace = {'__name__': '__aura_repl__',
                              '__builtins__': __builtins__}
            self._install_python_alias()
            self.write("namespace reset")
            return ReplResult()
        if name == ':history':
            for i, entry in enumerate(self._history, 1):
                self.write(f"{i:3d}  {entry}")
            return ReplResult()
        if name == ':type':
            return self.handle_type(arg)
        if name == ':ast':
            return self.handle_ast(arg)
        if name == ':py':
            return self.handle_python(arg)
        if name == ':load':
            return self.handle_load(arg)
        if name == ':run':
            return self.handle_run(arg)
        self.write(f"unknown command: {name} (try ':help')")
        return ReplResult(ok=False)

    def handle_type(self, expr):
        if not expr.strip():
            self.write("usage: :type <expression>")
            return ReplResult(ok=False)
        try:
            ast = self._parse_expression(expr)
            value = eval(compile(self.transformer.transform(ast), '<repl>', 'eval'),
                         self.namespace)
            from aura.transpiler.types import TypeInference
            inferred = TypeInference().infer(ast) if hasattr(TypeInference(), 'infer') else None
            self.write(f"{type(value).__name__}  (value: {value!r})")
            _ = inferred
            return ReplResult(value=value)
        except Exception as exc:
            self.write(f"error: {exc}")
            return ReplResult(ok=False, exception=exc)

    def handle_ast(self, expr):
        if not expr.strip():
            self.write("usage: :ast <code>")
            return ReplResult(ok=False)
        try:
            node = self._parse_program(expr)
            for stmt in node.statements:
                self.write(_dump_ast(stmt))
            return ReplResult()
        except Exception as exc:
            self.write(f"error: {exc}")
            return ReplResult(ok=False, exception=exc)

    def handle_python(self, code):
        """Execute raw Python inside the REPL namespace."""
        if not code.strip():
            self.write("usage: :py <python code>")
            return ReplResult(ok=False)
        try:
            compiled = compile(code, '<repl:py>', 'eval')
            try:
                value = eval(compiled, self.namespace)
            except SyntaxError:
                exec(compile(code, '<repl:py>', 'exec'), self.namespace)
                self.write("(executed)")
                return ReplResult()
            self.namespace['_'] = value
            self.write(repr(value))
            return ReplResult(value=value)
        except Exception as exc:
            self.write(f"python error: {type(exc).__name__}: {exc}")
            return ReplResult(ok=False, exception=exc)

    def handle_load(self, path):
        if not path.strip():
            self.write("usage: :load <file.aura>")
            return ReplResult(ok=False)
        try:
            program = parse_file(path.strip())
            code = self.transformer.transform(program)
            exec(compile(code, path.strip(), 'exec'), self.namespace)
            self.write(f"loaded {path.strip()}")
            return ReplResult()
        except Exception as exc:
            self.write(f"error: {exc}")
            return ReplResult(ok=False, exception=exc)

    def handle_run(self, path):
        if not path.strip():
            self.write("usage: :run <file.aura>")
            return ReplResult(ok=False)
        try:
            from aura.cli import cmd_run
            code = cmd_run(path.strip())
            self.write(f"(exit code {code})")
            return ReplResult(ok=code == 0)
        except Exception as exc:
            self.write(f"error: {exc}")
            return ReplResult(ok=False, exception=exc)

    # -- core evaluation ----------------------------------------------------

    def process_buffer(self):
        """Execute the current buffer (kept for backwards compatibility)."""
        text = self.buffer
        self.buffer = ""
        return self.process(text)

    def process(self, source):
        """Parse, transform and execute ``source`` in the persistent namespace."""
        self._history.append(source.strip())
        try:
            program = self._parse_program(source)
        except Exception as exc:
            self.write(f"Syntax error: {exc}")
            return ReplResult(ok=False, message=f"Syntax error: {exc}", exception=exc)

        rule_error = self._check_rules(program)
        if rule_error:
            self.write(rule_error)
            return ReplResult(ok=False, message=rule_error)

        try:
            value, printed = self._execute(program)
        except SystemExit:
            raise
        except Exception as exc:
            self.write(f"Runtime error: {type(exc).__name__}: {exc}")
            return ReplResult(ok=False, message=None, exception=exc)

        if value is not None:
            self.namespace['_'] = value
        if printed:
            self.write(printed)
            return ReplResult(value=value, message=None)
        return ReplResult(value=value)

    def _execute(self, program):
        """Execute statements; echo the last bare expression.

        Returns ``(value, printed_text)``. Functions/classes/imports execute as
        statements; a trailing expression statement is evaluated separately so
        its value can be displayed.
        """
        statements = program.statements
        if not statements:
            return None, None

        tail_expr = None
        body = statements
        last = statements[-1]
        if _is_bare_expression(last):
            tail_expr = last.expr
            body = statements[:-1]

        if body:
            code = self.transformer.transform(Program(body))
            if code.strip():
                exec(compile(code, '<repl>', 'exec'), self.namespace)

        if tail_expr is None:
            return None, None

        expr_code = self.transformer.transform(tail_expr)
        value = eval(compile(expr_code, '<repl>', 'eval'), self.namespace)
        return value, repr(value)

    # -- parsing helpers ----------------------------------------------------

    def _parse_program(self, source):
        tokens = Tokenizer(source).tokenize()
        return Parser(tokens).parse()

    def _parse_expression(self, source):
        tokens = Tokenizer(source).tokenize()
        return Parser(tokens).parse_expression()

    def _check_rules(self, program):
        """Run structural rules; returns an error string or None.

        Mutability is intentionally *not* enforced here: interactive bindings
        are treated as mutable so experimentation is frictionless. Structural
        rules (break/return/await/self placement and invalid targets) still
        apply because they catch real mistakes.
        """
        checker = RuleChecker()
        if checker.check_program(_RelaxedProgram(program)):
            return None
        first = str(checker.collector.errors[0]) if checker.collector.errors else "rule violation"
        return first.splitlines()[1].strip() if "\n" in first else first

    def _format_vars(self):
        skip = {'__name__', '__builtins__', '__doc__', '__package__', '__loader__',
                '__spec__'}
        rows = []
        for name, value in sorted(self.namespace.items()):
            if name in skip or name.startswith('__'):
                continue
            rows.append(f"  {name} = {_short_repr(value)}")
        if not rows:
            return "(no bindings)"
        return "\n".join(rows)


class _RelaxedProgram:
    """Wrapper that reports all top-level `let` bindings as mutable.

    The REPL lets you reassign anything you typed, but the structural rules
    (break/return placement etc.) are still checked by :class:`RuleChecker`.
    """

    def __init__(self, program):
        self.statements = [_relax_stmt(s) for s in program.statements]


def _relax_stmt(stmt):
    from aura.transpiler.ast import VarDecl
    if isinstance(stmt, VarDecl):
        stmt.mutable = True
    return stmt


def _is_bare_expression(stmt):
    from aura.transpiler.ast import ExprStmt, BinaryOp, TupleLiteral
    if not isinstance(stmt, ExprStmt):
        return False
    expr = stmt.expr
    # Assignments are statements, not values to echo.
    if isinstance(expr, BinaryOp) and expr.op in (
        '=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>=', '??='
    ):
        return False
    # A bare tuple used for multi-assignment is not a value either.
    if isinstance(expr, TupleLiteral):
        return False
    return True


def _short_repr(value, limit=80):
    text = repr(value)
    if len(text) > limit:
        return text[:limit - 3] + '...'
    return text


def _dump_ast(node, indent=0):
    """Render an AST node as an indented tree."""
    pad = "  " * indent
    lines = [f"{pad}{type(node).__name__}"]
    for key, value in vars(node).items():
        if key in ('line', 'column'):
            continue
        if hasattr(value, '__dict__') and value.__class__.__module__.startswith('aura'):
            lines.append(f"{pad}  {key}:")
            lines.append(_dump_ast(value, indent + 2))
        elif isinstance(value, list) and value and hasattr(value[0], '__dict__'):
            lines.append(f"{pad}  {key}:")
            for item in value:
                lines.append(_dump_ast(item, indent + 2))
        else:
            lines.append(f"{pad}  {key}: {value!r}")
    return "\n".join(lines)


HELP_TEXT = """Aura REPL commands
  :help                 show this help
  :vars                 list bindings in the current session
  :type <expr>          evaluate and print the value/type of an expression
  :ast <code>           parse and print the syntax tree
  :py <code>            run raw Python in the session namespace
  :load <file.aura>     execute an Aura file into this session
  :run <file.aura>      run an Aura file as a separate program
  :history              show previously entered chunks
  :reset                clear all bindings
  :q :quit :exit        leave the REPL

You can also write Aura directly. A bare expression prints its value; the
result is kept in `_`. Multi-line input continues automatically while brackets
are open or the line ends with a continuation token. `import python` exposes
the Python interop bridge."""


def main():
    """Console entry point for `python -m aura.repl.engine`."""
    try:
        AuraREPL().run()
    except EOFError:
        pass
    return 0


if __name__ == '__main__':
    raise SystemExit(main())