"""A lightweight step tracer for Aura.

Aura executes transpiled Python, so debugging happens at two levels:

* ``aura debug file.aura`` runs the program and, on an uncaught exception,
  prints a post-mortem that maps the failing generated-Python line back to the
  Aura statement that produced it (built by transpiling each top-level
  statement separately to learn its line span).
* ``aura debug file.aura --trace`` prints every generated line as it executes,
  annotated with the Aura line range it came from.

This is a trace debugger, not a full breakpoint debugger; it is dependency
free and works anywhere the `aura` command does.
"""

import sys
from pathlib import Path

from aura.parser.to_ast import parse_file
from aura.transpiler.transformer import Transformer


def _statement_spans(ast):
    """Transpile each top-level statement once.

    Returns ``(spans, body_lines)`` where ``spans`` is a list of
    ``(aura_line, line_count)`` in source order and ``body_lines`` is the total
    number of generated body lines. Transpiling each statement a single time
    keeps the debugger from doing redundant work.
    """
    spans = []
    body_lines = 0
    for stmt in getattr(ast, 'statements', []):
        try:
            code = Transformer().transform(stmt)
        except Exception:
            continue
        line_count = max(1, len(code.split('\n')))
        spans.append((getattr(stmt, 'line', None), line_count))
        body_lines += line_count
    return spans, body_lines


def build_source_map(ast):
    """Map generated-Python line numbers to Aura source lines.

    Returns ``{generated_line: aura_line}`` (1-indexed generated lines).
    """
    source_map = {}
    generated_line = 1
    spans, _ = _statement_spans(ast)
    for aura_line, line_count in spans:
        if aura_line is not None:
            for offset in range(line_count):
                source_map[generated_line + offset] = aura_line
        generated_line += line_count
    return source_map


def run(path, trace=False, show_code=False):
    ast = parse_file(path)

    # Entry files declare `main`; the runtime invokes it. Mirror `aura run` so
    # the debugger actually executes the program.
    from aura.cli import _prepare_entrypoint
    has_async, invoke_code = _prepare_entrypoint(ast)

    code = Transformer().transform(ast)
    if invoke_code:
        code = code + "\n" + invoke_code

    if show_code:
        for number, line in enumerate(code.split('\n'), 1):
            print(f"{number:4d} | {line}")

    source_lines = Path(path).read_text(encoding='utf-8').split('\n')
    spans, body_lines = _statement_spans(ast)
    local_map = {}
    generated_line = 1
    for aura_line, line_count in spans:
        if aura_line is not None:
            for span_offset in range(line_count):
                local_map[generated_line + span_offset] = aura_line
        generated_line += line_count
    offset = max(0, len(code.split('\n')) - body_lines)

    def aura_line(generated_line):
        mapped = local_map.get(generated_line - offset)
        if mapped is None:
            return None
        return mapped

    namespace = {'__name__': '__aura__'}

    def _exec(source):
        if has_async:
            import asyncio
            indented = "\n".join(
                ("    " + line if line.strip() else line)
                for line in source.split("\n"))
            wrapper = "async def _aura_debug_main():\n" + indented + "\n"
            exec(compile(wrapper, path, 'exec'), namespace)
            asyncio.run(namespace['_aura_debug_main']())
        else:
            exec(compile(source, path, 'exec'), namespace)

    if trace:
        def tracer(frame, event, arg):
            if frame.f_code.co_filename != path:
                return None
            if event == 'line':
                aline = aura_line(frame.f_lineno)
                context = ''
                if aline and 0 < aline <= len(source_lines):
                    context = source_lines[aline - 1].strip()
                print(f"  {path}:{aline or '?'}  {context}")
            return tracer

        # Save and restore the previous tracer instead of clearing it: tools
        # like coverage (which drives tracing via `sys.settrace` on Python
        # <= 3.12) would otherwise be silently disabled for the rest of the
        # process.
        previous_trace = sys.gettrace()
        previous_profile = sys.getprofile()
        sys.settrace(tracer)
        try:
            _exec(code)
        finally:
            sys.settrace(previous_trace)
            if previous_profile is not None:
                sys.setprofile(previous_profile)
        return 0

    try:
        _exec(code)
        return 0
    except SystemExit as exc:
        return exc.code if isinstance(exc.code, int) else 0
    except Exception as exc:
        print(f"Uncaught {type(exc).__name__}: {exc}", file=sys.stderr)
        tb = exc.__traceback__
        last_aura = None
        while tb is not None:
            if tb.tb_frame.f_code.co_filename == path:
                last_aura = aura_line(tb.tb_lineno)
            tb = tb.tb_next
        if last_aura and 0 < last_aura <= len(source_lines):
            print(f"  at {path}:{last_aura}: {source_lines[last_aura - 1].strip()}",
                  file=sys.stderr)
        return 1
