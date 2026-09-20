"""Aura REPL engine.

The REPL shares the real parser, transformer and semantic rules with the rest
of the toolchain, so what you type behaves the way ``aura check`` treats it.
The same three checkers run on every chunk — type checks, structural rules and
mutability — so a type error or a visibility violation is reported instead of
executing. It supports:

* persistent state across lines (variables, functions, classes, imports);
* value echoing — a bare expression prints the resulting value, and the result
  is also stored in ``_``;
* automatic multi-line entry: input continues while brackets are unbalanced or
  the line ends with a continuation token;
* semantic rule enforcement, except the entry-point rule: an interactive chunk
  is a fragment, so ``main`` is not required (it is required by ``aura run``);
* built-in commands (``:help``, ``:vars``, ``:type``, ``:ast``, ``:py``,
  ``:load``, ``:run``, ``:reset``, ``:history``);
* direct Python execution through ``:py`` and the ``python`` bridge;
* history persistence across sessions (``~/.aura_history``);
* tab completion for commands and keywords.

The engine is importable and testable without touching ``stdin``/``stdout`` by
injecting ``input_func``/``output_func``.
"""

from __future__ import annotations

import contextlib
import pathlib

from aura.parser.to_ast import Parser, Tokenizer, parse_file
from aura.transpiler.ast import Program
from aura.transpiler.rules import RuleChecker
from aura.transpiler.semantics import MutabilityChecker
from aura.transpiler.transformer import Transformer
from aura.transpiler.types import TypeChecker

_HISTORY_FILE = pathlib.Path.home() / '.aura_history'
_MAX_HISTORY = 1000


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
        # Mutability of bindings declared in previous chunks, so a `let` from an
        # earlier line stays immutable across the whole session.
        self._bindings = {}
        # Names of async functions defined in earlier chunks, so a bare
        # `main()` call is awaited across chunk boundaries.
        self._async_names = set()
        self._install_python_alias()
        self._load_history()
        self._setup_completion()

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

    def _load_history(self):
        """Load command history from ~/.aura_history."""
        if _HISTORY_FILE.exists():
            try:
                lines = _HISTORY_FILE.read_text(encoding='utf-8').splitlines()
                self._history = lines[-_MAX_HISTORY:]
            except Exception:
                self._history = []

    def _save_history(self):
        """Save command history to ~/.aura_history."""
        try:
            _HISTORY_FILE.parent.mkdir(parents=True, exist_ok=True)
            _HISTORY_FILE.write_text(
                '\n'.join(self._history[-_MAX_HISTORY:]) + '\n',
                encoding='utf-8',
            )
        except Exception:
            pass

    def _setup_completion(self):
        """Set up tab completion for commands and keywords."""
        try:
            import readline
            self._commands = [
                ':help', ':quit', ':exit', ':vars', ':type', ':ast',
                ':py', ':load', ':run', ':reset', ':history',
            ]
            self._keywords = [
                'def', 'class', 'let', 'mut', 'if', 'else', 'for', 'while',
                'return', 'throw', 'catch', 'finally', 'import', 'from',
                'as', 'async', 'await', 'match', 'case', 'and', 'or', 'not',
                'is', 'in', 'pub', 'priv', 'prot', 'static', 'abstract',
            ]
            readline.set_completer(self._complete)
            readline.parse_and_bind('tab: complete')
        except ImportError:
            pass

    def _complete(self, text, state):
        """Tab completion handler."""
        if not text:
            return None
        # Complete commands (starting with :)
        if text.startswith(':'):
            options = [cmd for cmd in self._commands if cmd.startswith(text)]
        else:
            # Complete keywords and defined names
            options = [kw for kw in self._keywords if kw.startswith(text)]
            options += [name for name in self.namespace
                       if name.startswith(text) and name not in options]
        if state < len(options):
            return options[state]
        return None

    def write(self, text=""):
        self._output(text)

    # -- public entry point -------------------------------------------------

    def run(self):
        self.write("Aura REPL v0.2 (type ':help' for help, ':q' to quit)")
        while True:
            prompt = self.cont_prompt if self.buffer else self.prompt
            try:
                line = self._input(prompt)
            except EOFError:
                if self.buffer:
                    # Discard buffer on Ctrl+D instead of auto-executing
                    self.write("\n(buffer discarded)")
                    self.buffer = ""
                    continue
                self.write("")
                break
            except KeyboardInterrupt:
                # Clear current line/buffer on Ctrl+C
                self.write("\n(buffer discarded)")
                self.buffer = ""
                continue

            result = self.feed(line)
            if result is not None and not result.continued and result.message:
                self.write(result.message)
        self._save_history()
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
        in_line_comment = False
        in_block_comment = False
        i = 0
        text = self.buffer
        while i < len(text):
            ch = text[i]
            nxt = text[i + 1] if i + 1 < len(text) else ''
            if in_line_comment:
                if ch == '\n':
                    in_line_comment = False
                i += 1
                continue
            if in_block_comment:
                if ch == '*' and nxt == '/':
                    in_block_comment = False
                    i += 2
                    continue
                i += 1
                continue
            if escaped:
                escaped = False
                i += 1
                continue
            if ch == '\\':
                escaped = True
                i += 1
                continue
            if in_string:
                if ch == in_string:
                    in_string = None
                i += 1
                continue
            if ch == '/' and nxt == '/':
                in_line_comment = True
                i += 2
                continue
            if ch == '/' and nxt == '*':
                in_block_comment = True
                i += 2
                continue
            if ch in ('"', "'"):
                in_string = ch
            elif ch in '([{':
                depth += 1
            elif ch in ')]}':
                depth -= 1
            i += 1

        if depth > 0 or in_string is not None or in_block_comment:
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
            self._bindings = {}
            self._async_names = set()
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
            self.write(f"{type(value).__name__}  (value: {value!r})")
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
        """Execute raw Python inside the REPL namespace.

        .. warning::
            This bypasses Aura's type safety, mutability, and visibility
            rules. Use ``:py`` only when you need to inspect Python objects
            directly or run host-side diagnostics.
        """
        if not code.strip():
            self.write("usage: :py <python code>")
            return ReplResult(ok=False)
        try:
            try:
                compiled = compile(code, '<repl:py>', 'eval')
            except SyntaxError:
                # Not an expression: run it as statements instead.
                exec(compile(code, '<repl:py>', 'exec'), self.namespace)
                self.write("(executed)")
                return ReplResult()
            value = eval(compiled, self.namespace)
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
        except Exception as exc:
            self.write(f"error: {exc}")
            return ReplResult(ok=False, exception=exc)

        # Enforce the same rules the CLI applies to a program.
        rule_error = self._check_rules(program)
        if rule_error:
            return ReplResult(ok=False, message=rule_error)

        try:
            value, printed = self._execute(program)
            self._bindings = MutabilityChecker().collect_bindings(
                program, self._bindings)
            if printed:
                self.write(printed)
            self.write(f"loaded {path.strip()}")
            return ReplResult(value=value)
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
            message = f"Syntax error: {exc}"
            return ReplResult(ok=False, message=message, exception=exc)

        rule_error = self._check_rules(program)
        if rule_error:
            return ReplResult(ok=False, message=rule_error)

        try:
            value, printed = self._execute(program)
        except SystemExit:
            raise
        except Exception as exc:
            self.write(f"Runtime error: {type(exc).__name__}: {exc}")
            return ReplResult(ok=False, message=None, exception=exc)

        # Only record the chunk's bindings once it executed successfully, so a
        # failed assignment does not change the session's mutability view.
        self._bindings = MutabilityChecker().collect_bindings(
            program, self._bindings)
        self._record_async_names(program)

        if value is not None:
            self.namespace['_'] = value
        if printed:
            self.write(printed)
            return ReplResult(value=value, message=None)
        return ReplResult(value=value)

    def _record_async_names(self, program):
        """Remember async functions so later chunks await their bare calls."""
        from aura.transpiler.ast import FunctionDecl
        for stmt in getattr(program, 'statements', []):
            if isinstance(stmt, FunctionDecl) and getattr(stmt, 'is_async', False):
                self._async_names.add(stmt.name)

    @staticmethod
    def _tail_calls_async(program, async_names):
        """True when the final expression is a bare call to an async function."""
        from aura.transpiler.ast import CallExpr, ExprStmt, Identifier
        statements = getattr(program, 'statements', [])
        if not statements:
            return False
        last = statements[-1]
        if not isinstance(last, ExprStmt) or not isinstance(last.expr, CallExpr):
            return False
        func = last.expr.func
        return isinstance(func, Identifier) and func.name in async_names

    @staticmethod
    def _stmt_calls_async(stmt, async_names):
        """True when ``stmt`` is a bare call to a known async function."""
        from aura.transpiler.ast import CallExpr, ExprStmt, Identifier
        if not isinstance(stmt, ExprStmt) or not isinstance(stmt.expr, CallExpr):
            return False
        func = stmt.expr.func
        return isinstance(func, Identifier) and func.name in async_names

    def _execute(self, program):
        """Execute statements; echo the last bare expression.

        Returns ``(value, printed_text)``. Functions/classes/imports execute as
        statements; a trailing expression statement is evaluated separately so
        its value can be displayed. Async chunks are driven to completion, so
        top-level `await` behaves exactly as it does under `aura run`.
        """
        statements = program.statements
        if not statements:
            return None, None

        from aura.cli import _await_top_level_async_calls
        is_async = _await_top_level_async_calls(program)
        if not is_async and self._tail_calls_async(program, self._async_names):
            is_async = True
        if is_async:
            return self._execute_async(program)

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

    def _execute_async(self, program):
        """Execute an async chunk, like ``aura run`` but persistent.

        Definitions must survive the chunk, so they are executed at module
        scope. Only statements that actually contain a top-level ``await`` are
        run inside a coroutine and driven to completion.
        """
        statements = program.statements
        tail_expr = None
        body = statements
        last = statements[-1]
        if _is_bare_expression(last):
            tail_expr = last.expr
            body = statements[:-1]

        for stmt in body:
            code = self.transformer.transform(stmt)
            if not code.strip():
                continue
            needs_await = self._needs_await(code) or self._stmt_calls_async(
                stmt, self._async_names)
            if needs_await:
                # Bindings created by an awaited statement must survive the
                # chunk, so the coroutine writes them into the session
                # namespace via `global`.
                if self._stmt_calls_async(stmt, self._async_names) \
                        and not code.lstrip().startswith('await '):
                    code = "await " + code
                names = self._assigned_names(stmt)
                self._run_awaiting(code, persist=names)
            else:
                exec(compile(code, '<repl>', 'exec'), self.namespace)

        if tail_expr is None:
            return None, None

        expr_code = self.transformer.transform(tail_expr)
        # A bare call to an async function defined in an earlier chunk must be
        # awaited explicitly; top-level `await` is already in the expression.
        if (self._tail_calls_async(program, self._async_names)
                and not expr_code.lstrip().startswith('await ')):
            expr_code = f"await {expr_code}"
        value = self._run_awaiting(f"return {expr_code}", is_expr=True)
        return value, repr(value)

    @staticmethod
    def _needs_await(code):
        try:
            compile(code, '<repl>', 'exec')
            return False
        except SyntaxError as exc:
            return 'await' in (exc.msg or '')

    @staticmethod
    def _assigned_names(stmt):
        """Top-level names bound or assigned by ``stmt``."""
        from aura.transpiler.ast import (
            BinaryOp,
            ConstDecl,
            ExprStmt,
            Identifier,
            ListLiteral,
            TupleLiteral,
            VarDecl,
        )
        names = []

        if isinstance(stmt, VarDecl):
            target = stmt.name
            if isinstance(target, str) and target[:1] in '([' and target[-1:] in ')]':
                inner = target[1:-1]
                for part in inner.replace('*', ' ').split(','):
                    token = part.strip().strip('()[]{}')
                    if token and token.isidentifier():
                        names.append(token)
            elif isinstance(target, str) and target.isidentifier():
                names.append(target)
        elif isinstance(stmt, ConstDecl):
            names.append(stmt.name)
        elif isinstance(stmt, ExprStmt) and isinstance(stmt.expr, BinaryOp) \
                and stmt.expr.op == '=':
            target = stmt.expr.left
            if isinstance(target, Identifier):
                names.append(target.name)
            elif isinstance(target, (TupleLiteral, ListLiteral)):
                for el in target.elements:
                    if isinstance(el, Identifier):
                        names.append(el.name)
        return [n for n in names if n]

    def _run_awaiting(self, code, is_expr=False, persist=None):
        """Run ``code`` inside a coroutine and return its result.

        ``persist`` lists names whose assignments must be written to the session
        namespace instead of staying local to the coroutine.
        """
        import asyncio

        if is_expr:
            wrapper = f"async def _aura_repl_await():\n    {code}\n"
        else:
            lines = code.split("\n")
            indented = "\n".join(("    " + line if line.strip() else line)
                                 for line in lines)
            global_decl = ""
            if persist:
                global_decl = "    global " + ", ".join(sorted(set(persist))) + "\n"
            wrapper = f"async def _aura_repl_await():\n{global_decl}{indented}\n"
        exec(compile(wrapper, '<repl>', 'exec'), self.namespace)
        try:
            return asyncio.run(self.namespace['_aura_repl_await']())
        finally:
            self.namespace.pop('_aura_repl_await', None)

    # -- parsing helpers ----------------------------------------------------

    def _parse_program(self, source):
        tokens = Tokenizer(source).tokenize()
        return Parser(tokens).parse()

    def _parse_expression(self, source):
        tokens = Tokenizer(source).tokenize()
        return Parser(tokens).parse_expression()

    def _check_rules(self, program):
        """Run Aura's rules over one chunk; return an error string or None.

        The same three checkers that ``aura check`` runs are applied here:
        type checks (``TypeChecker``), structural rules (break/return/await/self
        placement, duplicate declarations, invalid assignment targets,
        unreachable code, visibility, abstract methods) and mutability rules.
        For mutability, bindings declared in earlier chunks seed the checker so
        a `let` stays immutable for the whole session. The entry-point rule
        (``main``) is deliberately *not* enforced: a REPL chunk is a script
        fragment, not a program.
        """
        type_checker = TypeChecker()
        try:
            if not type_checker.check_program(program):
                return self._first_error(getattr(type_checker, 'diagnostics', [])
                                         or type_checker.errors)

            checker = MutabilityChecker()
            if not checker.check_program(program, initial_bindings=self._bindings):
                return self._first_error(getattr(checker, 'diagnostics', [])
                                         or checker.errors)

            rule_checker = RuleChecker()
            if not rule_checker.check_program(program):
                return self._first_error(rule_checker.collector.errors)
        except RecursionError:
            return "input is nested too deeply"
        return None

    @staticmethod
    def _first_error(errors):
        """Render the first diagnostic as ``[CODE] message`` when available."""
        if not errors:
            return "rule violation"
        first = errors[0]
        code = getattr(getattr(first, 'code', None), 'value', None)
        message = getattr(first, 'message', None)
        if code and message:
            return f"[{code}] {message}"
        text = str(first)
        return text.splitlines()[1].strip() if "\n" in text else text

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


def _is_bare_expression(stmt):
    from aura.transpiler.ast import BinaryOp, ExprStmt, TupleLiteral
    if not isinstance(stmt, ExprStmt):
        return False
    expr = stmt.expr
    # Assignments are statements, not values to echo.
    if isinstance(expr, BinaryOp) and expr.op in (
        '=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>=', '??='
    ):
        return False
    # A bare tuple used for multi-assignment is not a value either.
    return not isinstance(expr, TupleLiteral)


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
    with contextlib.suppress(EOFError):
        AuraREPL().run()
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
