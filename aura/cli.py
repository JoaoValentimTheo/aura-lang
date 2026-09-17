"""CLI for Aura transpiler - Phase 3 with type checking, formatting, linting."""
import argparse
import sys
import time as _time
from pathlib import Path

from aura.runtime import install_runtime_aliases

# Generated code uses `import stdlib...`; map those to `aura.stdlib...`.
install_runtime_aliases()

from aura.parser.to_ast import parse_file
from aura.transpiler.errors import ErrorCode, ErrorCollector
from aura.transpiler.semantics import MutabilityChecker
from aura.transpiler.transformer import Transformer
from aura.transpiler.types import TypeChecker


def _mutability_diagnostics(ast):
    """Return structured mutability diagnostics for ``ast``.

    Aura's ``let`` bindings are immutable and ``const`` bindings can never be
    reassigned; both require an explicit ``let mut`` to be reassigned. This is
    enforced for every entry point (transpile, run, test, check) so the rule is
    actually effective rather than documentation-only.
    """
    checker = MutabilityChecker()
    try:
        if checker.check_program(ast):
            return []
    except RecursionError:
        return []
    return list(getattr(checker, 'diagnostics', []))


def _rule_diagnostics(ast, require_main=False):
    """Return structured rule diagnostics for ``ast``.

    ``require_main`` enforces the program entry point (a top-level ``def
    main``) for files executed directly; imported modules leave it off.
    """
    from aura.transpiler.rules import RuleChecker
    checker = RuleChecker()
    try:
        checker.check_program(ast, require_main=require_main)
    except RecursionError:
        return []
    return list(checker.collector.errors)


def _report_diagnostic(diag, fallback_path=None):
    """Print one diagnostic to stderr.

    The formatted diagnostic already carries ``file:line:column: SEVERITY
    [code]``; when a location is missing we prefix the path so the user still
    knows which file is at fault.
    """
    text = str(diag)
    if getattr(diag, 'location', None) is None and fallback_path:
        print(f"{fallback_path}: {text}", file=sys.stderr)
    else:
        print(text, file=sys.stderr)


def _report_all(path, *groups):
    """Report every diagnostic in ``groups`` (each an iterable of diagnostics)."""
    total = 0
    for group in groups:
        for diag in group:
            _report_diagnostic(diag, fallback_path=path)
            total += 1
    return total


def _install_aura_imports(script_path: str):
    """Enable `import sibling_module` for Aura files next to the script."""
    try:
        from aura.transpiler.importer import install_aura_import_hook
        script_dir = Path(script_path).resolve().parent
        install_aura_import_hook([script_dir, Path.cwd()])
    except Exception:
        # The hook is best-effort; Python imports still work without it.
        pass


def cmd_transpile(path: str, output: str | None = None, verbose: bool = False) -> int:
    """Transpile Aura file to Python."""
    try:
        ast = parse_file(path)
    except FileNotFoundError:
        print(f"Error: File not found: {path}", file=sys.stderr)
        return 2
    except Exception as e:
        print(f"Error parsing {path}: {e}", file=sys.stderr)
        return 2

    mutability = _mutability_diagnostics(ast)
    rules = _rule_diagnostics(ast)
    if mutability or rules:
        _report_all(path, mutability, rules)
        return 2

    try:
        t = Transformer()
        code = t.transform(ast)

        if output:
            Path(output).write_text(code)
            print(f"Transpiled to: {output}")
        else:
            print(code)

        if verbose:
            print(f"# AST: {ast}", file=sys.stderr)

        return 0
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(f"Error transpiling {path}: {e}", file=sys.stderr)
        return 2


def cmd_check(path: str, verbose: bool = False) -> int:
    """Type check Aura file without transpiling."""
    try:
        ast = parse_file(path)
    except FileNotFoundError:
        print(f"Error: File not found: {path}", file=sys.stderr)
        return 2
    except Exception as e:
        print(f"Error parsing {path}: {e}", file=sys.stderr)
        return 2

    checker = TypeChecker()
    type_ok = checker.check_program(ast)
    mutability = _mutability_diagnostics(ast)
    rules = _rule_diagnostics(ast)
    success = type_ok and not mutability and not rules

    total = _report_all(path, checker.diagnostics, mutability, rules)

    if verbose:
        print(f"# Inferred {len(checker.context)} top-level binding(s)", file=sys.stderr)
        for name, t in sorted(checker.context.items()):
            print(f"#   {name}: {t}", file=sys.stderr)

    if success:
        print(f"OK {path}: type check passed")
        return 0
    print(f"FAIL {path}: {total} issue(s)", file=sys.stderr)
    return 1


def cmd_format(path: str, output: str | None = None, width: int = 100) -> int:
    """Format Aura source code."""
    try:
        source = Path(path).read_text()
    except FileNotFoundError:
        print(f"Error: File not found: {path}", file=sys.stderr)
        return 2

    try:
        from aura.tools.formatter import format_aura
        formatted = format_aura(source, width=width)

        if output:
            Path(output).write_text(formatted)
            print(f"Formatted to: {output}")
        else:
            print(formatted)

        return 0
    except Exception as e:
        print(f"Error formatting {path}: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        return 2


def cmd_lint(path: str) -> int:
    """Lint Aura source code (style warnings)."""
    try:
        source = Path(path).read_text()
    except FileNotFoundError:
        print(f"Error: File not found: {path}", file=sys.stderr)
        return 2

    errors = ErrorCollector(path)

    try:
        from aura.transpiler.ast import SourceLocation

        lines = source.split('\n')

        for i, line in enumerate(lines, 1):
            # Check line length
            if len(line) > 100:
                errors.add_warning(
                    ErrorCode.LINE_TOO_LONG,
                    f"Line {i} is {len(line)} characters (max 100 recommended)",
                    location=SourceLocation(path, i, 101, len(line) - 100),
                    hint="Consider breaking into multiple lines"
                )

            # Check trailing whitespace
            if line.endswith(' ') or line.endswith('\t'):
                trimmed = line.rstrip()
                errors.add_warning(
                    ErrorCode.TRAILING_WHITESPACE,
                    f"Line {i} has trailing whitespace",
                    location=SourceLocation(path, i, len(trimmed) + 1,
                                            len(line) - len(trimmed)),
                )

            # Check naming conventions
            if line.strip().startswith('let '):
                var_name = line.strip().split()[1].split('=')[0]
                if var_name.isupper():
                    col = line.index(var_name) + 1
                    errors.add_warning(
                        ErrorCode.NAMING_CONVENTION,
                        f"Variable '{var_name}' should be snake_case, not UPPER_CASE",
                        location=SourceLocation(path, i, col, len(var_name)),
                        hint="use lower_snake_case for variables",
                    )

        # Check for common style issues
        idx = source.find('def  ')
        if idx != -1:
            line = source.count('\n', 0, idx) + 1
            col = idx - source.rfind('\n', 0, idx)
            errors.add_warning(
                ErrorCode.SPACING,
                "Multiple spaces after 'def' keyword",
                location=SourceLocation(path, line, col, 5),
                hint="Use a single space: 'def name'"
            )

        if errors.errors:
            print(errors.format())
            return 1
        else:
            print(f"✓ {path}: no style issues")
            return 0
    except Exception as e:
        print(f"Error linting {path}: {e}", file=sys.stderr)
        return 2


def _find_program_main(ast):
    """Return the top-level ``main`` FunctionDecl, or None."""
    from aura.transpiler.ast import FunctionDecl

    for stmt in ast.statements:
        if isinstance(stmt, FunctionDecl) and stmt.name == 'main':
            return stmt
    return None


def _main_is_called(ast):
    """True when the file already calls ``main()`` at the top level.

    A program normally never calls ``main`` itself (the runtime does). This
    only detects the explicit form so an existing program is not run twice.
    """
    from aura.transpiler.ast import CallExpr, ExprStmt, Identifier

    for stmt in ast.statements:
        if not isinstance(stmt, ExprStmt) or not isinstance(stmt.expr, CallExpr):
            continue
        func = stmt.expr.func
        if isinstance(func, Identifier) and func.name == 'main':
            return True
    return False


def _prepare_entrypoint(ast):
    """Return ``(has_async, invoke_code)`` for running ``ast`` as a program.

    ``has_async`` is True when the file declares or awaits async code, so
    ``cmd_run`` can wrap the whole program in a coroutine. ``invoke_code`` is
    the Python snippet that calls the program's ``main`` (with command-line
    arguments when the signature asks for them) and propagates an integer
    return value as the process exit code. It is empty when the file already
    calls ``main`` explicitly, so nothing is invoked twice.

    A bare top-level call to any async function is rewritten to ``await`` so a
    coroutine is not left un-awaited; detection is AST-based, not a substring
    search, so a literal ``"await"`` inside a string does not trigger it.
    """
    from aura.transpiler.ast import (
        CallExpr,
        ExprStmt,
        FunctionDecl,
        Identifier,
        Node,
        UnaryOp,
    )

    async_names = set()
    has_async = False

    def scan(value):
        nonlocal has_async
        if value is None or has_async:
            return
        if isinstance(value, FunctionDecl) and getattr(value, 'is_async', False):
            has_async = True
            return
        if isinstance(value, UnaryOp) and value.op == 'await':
            has_async = True
            return
        if isinstance(value, dict):
            for item in value.values():
                scan(item)
        elif isinstance(value, (list, tuple)):
            for item in value:
                scan(item)
        elif isinstance(value, Node):
            for item in vars(value).values():
                scan(item)

    for stmt in ast.statements:
        fn = getattr(stmt, 'name', None)
        if fn is not None and getattr(stmt, 'is_async', False):
            async_names.add(fn)
    scan(ast)

    # An explicit top-level call to an async function must be awaited.
    if async_names:
        for stmt in ast.statements:
            if not isinstance(stmt, ExprStmt) or not isinstance(stmt.expr, CallExpr):
                continue
            func = stmt.expr.func
            if isinstance(func, Identifier) and func.name in async_names:
                stmt.expr = UnaryOp('await', stmt.expr)

    main = _find_program_main(ast)
    if main is None or _main_is_called(ast):
        return has_async, ""

    main_async = bool(getattr(main, 'is_async', False))
    params = list(getattr(main, 'params', None) or [])
    takes_args = bool(params)
    call = "main(_aura_argv)" if takes_args else "main()"
    if main_async:
        call = "await " + call
    snippet = (
        f"_aura_rc = {call}\n"
        "if _aura_rc is not None:\n"
        "    raise SystemExit(_aura_rc)\n"
    )
    return has_async, snippet


def _await_top_level_async_calls(ast):
    """Backwards-compatible helper: return only whether ``ast`` is async.

    Kept for the REPL, which needs the async detection and the await rewrite
    but performs its own execution.
    """
    return _prepare_entrypoint(ast)[0]


def cmd_run(path: str, verbose: bool = False, program_args=None) -> int:
    """Run Aura file by transpiling and executing.

    ``program_args`` are forwarded to the program's ``main(args)`` when it
    declares an ``args`` parameter.
    """
    try:
        ast = parse_file(path)
    except FileNotFoundError:
        print(f"Error: File not found: {path}", file=sys.stderr)
        return 2
    except Exception as e:
        print(f"Error parsing {path}: {e}", file=sys.stderr)
        return 2

    mutability = _mutability_diagnostics(ast)
    rules = _rule_diagnostics(ast, require_main=True)
    if mutability or rules:
        _report_all(path, mutability, rules)
        return 2

    has_async, invoke_code = _prepare_entrypoint(ast)

    try:
        t = Transformer()
        code = t.transform(ast)
        if invoke_code:
            code = code + "\n" + invoke_code

        if verbose:
            print("# Generated Python code:", file=sys.stderr)
            print(f"# {'-'*60}", file=sys.stderr)
            for i, line in enumerate(code.split('\n'), 1):
                print(f"# {i:3d} | {line}", file=sys.stderr)
            print(f"# {'-'*60}", file=sys.stderr)

        # Async programs (async def or top-level await) are executed inside a
        # coroutine so `await` is legal and coroutines are driven to
        # completion exactly once.
        if has_async:
            import asyncio

            indented = "\n".join(
                ("    " + line if line.strip() else line)
                for line in code.split("\n")
            )
            wrapper = "async def _aura_main():\n" + indented + "\n"

            namespace: dict = {'__name__': '__aura__',
                               '_aura_argv': list(program_args or [])}
            _install_aura_imports(path)
            exec(compile(wrapper, path, 'exec'), namespace)
            asyncio.run(namespace['_aura_main']())
        else:
            _install_aura_imports(path)
            namespace = {'__name__': '__aura__',
                         '_aura_argv': list(program_args or [])}
            exec(compile(code, path, 'exec'), namespace)
        return 0
    except SystemExit as e:
        # A bare `return` in a top-level guard becomes `raise SystemExit()`.
        # That is a *successful* early exit, so treat a missing/None code as 0.
        if e.code is None:
            return 0
        return e.code if isinstance(e.code, int) else 1
    except Exception as e:
        print(f"Runtime error: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        return 1


def _last_stderr_line(stderr: str) -> str:
    """Return the most informative last line of an error stream."""
    if not stderr:
        return "exit code non-zero"
    lines = [line for line in stderr.strip().split('\n') if line.strip()]
    return lines[-1] if lines else "exit code non-zero"


def _test_failure_summary(stdout: str, stderr: str):
    """Extract a ``stdlib.testing`` failure summary from a test run.

    A file that uses ``import stdlib.testing`` prints ``N/M passed, K failed``
    plus ``FAIL <name>: ...`` lines to stdout and lets ``TestFailure`` produce
    the non-zero exit. Surfacing that summary is far more useful than the
    generic traceback line.
    """
    combined = f"{stdout}\n{stderr}"
    if 'stdlib.testing' not in combined and 'tests failed' not in combined:
        return None
    summary = None
    details = []
    for line in stdout.split('\n'):
        stripped = line.strip()
        if stripped.startswith('FAIL '):
            details.append(stripped)
        elif 'passed,' in stripped and 'failed' in stripped:
            summary = stripped
    if summary is None:
        return None
    if details:
        return summary + ' — ' + '; '.join(details)
    return summary


def cmd_test(path: str = ".", verbose: bool = False, pattern: str = "*.aura") -> int:
    """Run .aura test files and report pass/fail."""
    import subprocess

    root = Path(path)
    if root.is_file():
        files = [root]
    else:
        # Find all .aura files recursively
        files = sorted(root.rglob(pattern))
        if not files:
            print(f"No {pattern} files found in {path}", file=sys.stderr)
            return 2

    passed = 0
    failed = 0
    errors = []
    t0 = _time.time()

    for f in files:
        if verbose:
            print(f"  RUN  {f}", file=sys.stderr)
        try:
            # Use subprocess to isolate each test run. Invoke the installed
            # module (`python -m aura.cli`) so this works both from a source
            # checkout and from a pip install, and run in the file's directory
            # so sibling Aura modules resolve.
            resolved = Path(f).resolve()
            test_cwd = str(resolved.parent)
            result = subprocess.run(
                [sys.executable, "-m", "aura.cli", "run", str(resolved)],
                capture_output=True, text=True, timeout=30,
                cwd=test_cwd,
            )
            if result.returncode == 0:
                passed += 1
                if verbose:
                    print(f"  OK   {f}", file=sys.stderr)
            else:
                failed += 1
                err_msg = (_test_failure_summary(result.stdout, result.stderr)
                           or _last_stderr_line(result.stderr))
                errors.append((f, err_msg))
                if verbose:
                    print(f"  FAIL {f}: {err_msg}", file=sys.stderr)
        except subprocess.TimeoutExpired:
            failed += 1
            errors.append((f, "timeout (30s)"))
            if verbose:
                print(f"  FAIL {f}: timeout", file=sys.stderr)
        except Exception as e:
            failed += 1
            errors.append((f, str(e)))
            if verbose:
                print(f"  FAIL {f}: {e}", file=sys.stderr)

    elapsed = _time.time() - t0
    total = passed + failed
    print(f"\n{passed}/{total} passed, {failed} failed ({elapsed:.2f}s)")

    if errors:
        print("\nFailures:")
        for f, msg in errors[:20]:
            print(f"  {f}: {msg}")

    return 1 if failed else 0


def cmd_add(package: str, version: str | None = None, no_install: bool = False) -> int:
    """Add a Python dependency to aura.toml and install it."""
    from aura.tools.deps import add_package
    return add_package(package, version, install=not no_install)


def cmd_install(upgrade: bool = False) -> int:
    """Install all dependencies declared in aura.toml."""
    from aura.tools.deps import install_dependencies
    return install_dependencies(upgrade=upgrade)


def cmd_deps() -> int:
    """List declared dependencies."""
    from aura.tools.deps import list_dependencies
    return list_dependencies()


def cmd_init(name: str = "app") -> int:
    """Create a starter aura.toml and src/main.aura."""
    from aura.tools.deps import init_project
    return init_project(name)


def cmd_version(bump: str | None = None) -> int:
    """Print the version, or bump/modify and publish it to the metadata."""
    from aura.tools.release import bump as bump_version
    from aura.tools.release import get_version, set_version
    if bump:
        if bump in ('major', 'minor', 'patch'):
            print(bump_version(bump))
        else:
            print(set_version(bump))
    else:
        print(get_version())
    return 0


def cmd_debug(path: str, trace: bool = False, show_code: bool = False) -> int:
    """Run an Aura file under the lightweight trace debugger."""
    from aura.tools.debugger import run as debug_run
    return debug_run(path, trace=trace, show_code=show_code)


def cmd_lsp() -> int:
    """Start the Aura language server (JSON-RPC over stdio)."""
    from aura.lsp.server import main as lsp_main
    return lsp_main()


def cmd_repl() -> int:
    """Interactive REPL for Aura."""
    from aura.repl.engine import AuraREPL
    try:
        return AuraREPL().run()
    except EOFError:
        return 0


def main(argv=None):
    argv = argv or sys.argv[1:]

    p = argparse.ArgumentParser(
        prog='aura',
        description='Aura transpiler - Convert Aura source to Python',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  aura transpile file.aura              Transpile to stdout
  aura transpile file.aura -o out.py    Transpile to file
  aura check file.aura                  Type check only
  aura format file.aura                 Format source code
  aura lint file.aura                   Check style warnings
  aura run file.aura                    Run Aura file
  aura run file.aura -v                 Run with Python code output
  aura test tests/                      Run all .aura tests in directory
  aura test -v                          Verbose test output
  aura repl                             Start interactive REPL
        """
    )

    sub = p.add_subparsers(dest='cmd')

    # transpile command
    transp = sub.add_parser('transpile', help='Transpile Aura file to Python')
    transp.add_argument('path', help='Source file (.aura)')
    transp.add_argument('-o', '--output', help='Output file (.py)')
    transp.add_argument('-v', '--verbose', action='store_true', help='Show AST')

    # check command
    check = sub.add_parser('check', help='Type check without transpiling')
    check.add_argument('path', help='Source file (.aura)')
    check.add_argument('-v', '--verbose', action='store_true', help='Show inferred types')

    # format command
    fmt = sub.add_parser('format', help='Format source code')
    fmt.add_argument('path', help='Source file (.aura)')
    fmt.add_argument('-o', '--output', help='Output file')
    fmt.add_argument('--width', type=int, default=100, help='Max line width (default: 100)')

    # lint command
    lnt = sub.add_parser('lint', help='Check style and conventions')
    lnt.add_argument('path', help='Source file (.aura)')

    # run command
    run = sub.add_parser('run', help='Run Aura file')
    run.add_argument('path', help='Source file (.aura)')
    run.add_argument('-v', '--verbose', action='store_true', help='Show generated Python code')
    run.add_argument('args', nargs=argparse.REMAINDER,
                     help='Arguments passed to the program as main(args)')

    # repl command
    sub.add_parser('repl', help='Start interactive REPL')

    # test command
    test = sub.add_parser('test', help='Run .aura test files')
    test.add_argument('path', nargs='?', default='.', help='Directory or file to test (default: .)')
    test.add_argument('-v', '--verbose', action='store_true', help='Show each test result')
    test.add_argument('-p', '--pattern', default='*.aura', help='File glob pattern (default: *.aura)')

    # init command
    init = sub.add_parser('init', help='Create a starter aura.toml and project')
    init.add_argument('name', nargs='?', default='app', help='Project name')

    # add command
    add = sub.add_parser('add', help='Add a Python dependency and install it')
    add.add_argument('package', help='Package name, optionally with a specifier')
    add.add_argument('-V', '--version', help='Version or specifier, e.g. 1.2.3 or ">=2.0"')
    add.add_argument('--no-install', action='store_true', help='Only record the dependency')

    # install command
    install = sub.add_parser('install', help='Install dependencies from aura.toml')
    install.add_argument('--upgrade', action='store_true', help='Upgrade to latest versions')

    # deps command
    sub.add_parser('deps', help='List declared dependencies')

    # version command
    version = sub.add_parser('version', help='Show or bump the version')
    version.add_argument('bump', nargs='?', default=None,
                         help="'major', 'minor', 'patch', or an explicit version")

    # debug command
    debug = sub.add_parser('debug', help='Run a file under the trace debugger')
    debug.add_argument('path', help='Source file (.aura)')
    debug.add_argument('-t', '--trace', action='store_true', help='Trace executed lines')
    debug.add_argument('-c', '--show-code', action='store_true', help='Print generated Python')

    # lsp command
    sub.add_parser('lsp', help='Start the language server (stdio)')

    args = p.parse_args(argv)

    if args.cmd == 'transpile':
        return cmd_transpile(args.path, args.output, args.verbose)
    elif args.cmd == 'check':
        return cmd_check(args.path, args.verbose)
    elif args.cmd == 'format':
        return cmd_format(args.path, args.output, args.width)
    elif args.cmd == 'lint':
        return cmd_lint(args.path)
    elif args.cmd == 'run':
        return cmd_run(args.path, args.verbose, args.args)
    elif args.cmd == 'test':
        return cmd_test(args.path, args.verbose, args.pattern)
    elif args.cmd == 'repl':
        return cmd_repl()
    elif args.cmd == 'init':
        return cmd_init(args.name)
    elif args.cmd == 'add':
        return cmd_add(args.package, args.version, args.no_install)
    elif args.cmd == 'install':
        return cmd_install(args.upgrade)
    elif args.cmd == 'deps':
        return cmd_deps()
    elif args.cmd == 'version':
        return cmd_version(args.bump)
    elif args.cmd == 'debug':
        return cmd_debug(args.path, args.trace, args.show_code)
    elif args.cmd == 'lsp':
        return cmd_lsp()
    else:
        p.print_help()
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
