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


def build_source_map(ast):
    """Map generated-Python line numbers to Aura source lines.

    Transpiles each top-level statement independently; the number of lines in
    the result gives the span occupied by that statement in the full output.
    Returns ``{generated_line: aura_line}`` (1-indexed generated lines).
    """
    source_map = {}
    generated_line = 1
    # Prelude length: transpile the whole program and subtract the body lines.
    for stmt in getattr(ast, 'statements', []):
        try:
            code = Transformer().transform(stmt)
        except Exception:
            continue
        aura_line = getattr(stmt, 'line', None)
        line_count = max(1, len(code.split('\n')))
        if aura_line is not None:
            for offset in range(line_count):
                source_map[generated_line + offset] = aura_line
        generated_line += line_count
    return source_map


def _offset_for_prelude(ast, full_code):
    """Compute how many prelude lines precede the first program statement."""
    body_lines = 0
    for stmt in getattr(ast, 'statements', []):
        try:
            body_lines += max(1, len(Transformer().transform(stmt).split('\n')))
        except Exception:
            continue
    return max(0, len(full_code.split('\n')) - body_lines)


def run(path, trace=False, show_code=False):
    ast = parse_file(path)
    code = Transformer().transform(ast)

    if show_code:
        for number, line in enumerate(code.split('\n'), 1):
            print(f"{number:4d} | {line}")

    source_lines = Path(path).read_text(encoding='utf-8').split('\n')
    offset = _offset_for_prelude(ast, code)
    local_map = build_source_map(ast)

    def aura_line(generated_line):
        mapped = local_map.get(generated_line - offset)
        if mapped is None:
            return None
        return mapped

    namespace = {'__name__': '__aura__'}

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

        sys.settrace(tracer)
        try:
            exec(compile(code, path, 'exec'), namespace)
        finally:
            sys.settrace(None)
        return 0

    try:
        exec(compile(code, path, 'exec'), namespace)
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