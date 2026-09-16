"""CLI for Aura transpiler - Phase 3 with type checking, formatting, linting."""
import argparse
import sys
import json
import time as _time
from pathlib import Path

from aura.runtime import install_runtime_aliases

# Generated code uses `import stdlib...`; map those to `aura.stdlib...`.
install_runtime_aliases()

from aura.parser.to_ast import parse_file
from aura.transpiler.transformer import Transformer
from aura.transpiler.types import TypeChecker, TypeInference
from aura.transpiler.errors import ErrorCollector, ErrorCode, ErrorSeverity
from aura.transpiler.semantics import MutabilityChecker


def _mutability_errors(ast):
    """Return the semantic mutability violations for ``ast`` (possibly empty).

    Aura's ``let`` bindings are immutable and ``const`` bindings can never be
    reassigned; both require an explicit ``let mut`` to be reassigned. This is
    enforced for every entry point (transpile, run, test, check) so the rule is
    actually effective rather than documentation-only.
    """
    checker = MutabilityChecker()
    if checker.check_program(ast):
        return []
    return list(checker.errors)


def _rule_errors(ast):
    """Return structural rule violations for ``ast`` (possibly empty).

    These are the rules that do not depend on type inference: ``return`` and
    ``break``/``continue`` placement, ``await`` outside async, ``self`` outside
    a class, duplicate declarations, unreachable code and invalid assignment
    targets. Enforced for ``check`` and ``run``.
    """
    from aura.transpiler.rules import RuleChecker
    checker = RuleChecker()
    if checker.check_program(ast):
        return []
    return [str(e) for e in checker.collector.errors]


def _print_semantic_errors(path, mutability, rules):
    for error in mutability:
        print(f"{path}: semantic error: {error}", file=sys.stderr)
    for error in rules:
        print(f"{path}: rule error: {error}", file=sys.stderr)


def _install_aura_imports(script_path: str):
    """Enable `import sibling_module` for Aura files next to the script."""
    try:
        from aura.transpiler.importer import install_aura_import_hook
        script_dir = Path(script_path).resolve().parent
        install_aura_import_hook([script_dir, Path.cwd()])
    except Exception:
        # The hook is best-effort; Python imports still work without it.
        pass


def cmd_transpile(path: str, output: str = None, verbose: bool = False) -> int:
    """Transpile Aura file to Python."""
    try:
        ast = parse_file(path)
    except FileNotFoundError:
        print(f"Error: File not found: {path}", file=sys.stderr)
        return 2
    except Exception as e:
        print(f"Error parsing {path}: {e}", file=sys.stderr)
        return 2

    mutability = _mutability_errors(ast)
    rules = _rule_errors(ast)
    if mutability or rules:
        _print_semantic_errors(path, mutability, rules)
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
    mutability = _mutability_errors(ast)
    rules = _rule_errors(ast)
    success = type_ok and not mutability and not rules

    for error in checker.errors:
        print(f"{path}: type error: {error}", file=sys.stderr)
    _print_semantic_errors(path, mutability, rules)

    if verbose:
        print(f"# Inferred {len(checker.context)} top-level binding(s)", file=sys.stderr)
        for name, t in sorted(checker.context.items()):
            print(f"#   {name}: {t}", file=sys.stderr)

    if success:
        print(f"OK {path}: type check passed")
        return 0
    total = len(checker.errors) + len(mutability) + len(rules)
    print(f"FAIL {path}: {total} issue(s)", file=sys.stderr)
    return 1


def cmd_format(path: str, output: str = None, width: int = 100) -> int:
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
        lines = source.split('\n')
        
        for i, line in enumerate(lines, 1):
            # Check line length
            if len(line) > 100:
                errors.add_warning(
                    ErrorCode.INVALID_SYNTAX,
                    f"Line {i} is {len(line)} characters (max 100 recommended)",
                    hint="Consider breaking into multiple lines"
                )
            
            # Check trailing whitespace
            if line.endswith(' ') or line.endswith('\t'):
                errors.add_warning(
                    ErrorCode.INVALID_SYNTAX,
                    f"Line {i} has trailing whitespace"
                )
            
            # Check naming conventions
            if line.strip().startswith('let '):
                var_name = line.strip().split()[1].split('=')[0]
                if var_name.isupper():
                    errors.add_warning(
                        ErrorCode.INVALID_SYNTAX,
                        f"Variable '{var_name}' should be snake_case, not UPPER_CASE"
                    )
        
        # Check for common style issues
        if 'def  ' in source:
            errors.add_warning(
                ErrorCode.INVALID_SYNTAX,
                "Multiple spaces after 'def' keyword",
                hint="Use a single space: 'def name'"
            )
        
        if errors.errors:
            print(errors.format())
            return 0 if errors.warning_count() > 0 else 0
        else:
            print(f"✓ {path}: no style issues")
            return 0
    except Exception as e:
        print(f"Error linting {path}: {e}", file=sys.stderr)
        return 2


def _await_top_level_async_calls(ast):
    """Wrap bare top-level calls to async functions in `await`.

    Aura allows `main()` as the last statement of an async program. In
    generated Python that is a bare coroutine that is never awaited, so the
    program silently does nothing. Rewriting `main()` to `await main()` lets
    the async wrapper in cmd_run drive it to completion.

    Returns ``True`` when the program is asynchronous (any ``async def`` or
    ``await`` expression), so ``cmd_run`` can decide whether to wrap the whole
    program in a coroutine. Detection is AST-based, not a substring search, so
    a literal ``"await"`` inside a string does not trigger it.
    """
    from aura.transpiler.ast import (
        ExprStmt, UnaryOp, CallExpr, Identifier, FunctionDecl, Node,
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

    if async_names:
        for stmt in ast.statements:
            if not isinstance(stmt, ExprStmt) or not isinstance(stmt.expr, CallExpr):
                continue
            func = stmt.expr.func
            if isinstance(func, Identifier) and func.name in async_names:
                stmt.expr = UnaryOp('await', stmt.expr)

    return has_async


def cmd_run(path: str, verbose: bool = False) -> int:
    """Run Aura file by transpiling and executing."""
    try:
        ast = parse_file(path)
    except FileNotFoundError:
        print(f"Error: File not found: {path}", file=sys.stderr)
        return 2
    except Exception as e:
        print(f"Error parsing {path}: {e}", file=sys.stderr)
        return 2

    mutability = _mutability_errors(ast)
    rules = _rule_errors(ast)
    if mutability or rules:
        _print_semantic_errors(path, mutability, rules)
        return 2

    has_async = _await_top_level_async_calls(ast)

    try:
        t = Transformer()
        code = t.transform(ast)

        if verbose:
            print(f"# Generated Python code:", file=sys.stderr)
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

            namespace = {'__name__': '__aura__'}
            _install_aura_imports(path)
            exec(compile(wrapper, path, 'exec'), namespace)
            asyncio.run(namespace['_aura_main']())
        else:
            _install_aura_imports(path)
            exec(compile(code, path, 'exec'), {'__name__': '__aura__'})
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


def cmd_test(path: str = ".", verbose: bool = False, pattern: str = "*.aura") -> int:
    """Run .aura test files and report pass/fail."""
    import glob
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
                err_msg = result.stderr.strip().split('\n')[-1] if result.stderr else "exit code non-zero"
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


def cmd_add(package: str, version: str = None, no_install: bool = False) -> int:
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


def cmd_version(bump: str = None) -> int:
    """Print the version, or bump/modify and publish it to the metadata."""
    from aura.tools.release import bump as bump_version, get_version, set_version
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
        return cmd_run(args.path, args.verbose)
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