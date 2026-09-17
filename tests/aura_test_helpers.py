"""Shared helpers for Aura's professional test suites.

Kept import-light so both property-based and differential tests can use it
without pulling in the whole CLI.
"""
import contextlib
import io
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.transformer import Transformer  # noqa: E402


def parse(source):
    return Parser(Tokenizer(source).tokenize()).parse()


def parse_expr(source):
    return Parser(Tokenizer(source).tokenize()).parse_expression()


def transpile(source):
    return Transformer().transform(parse(source))


def run_aura(source):
    """Transpile and execute Aura source; return captured stdout.

    Entry files declare ``main``; the runtime invokes it, which this mirrors.
    """
    code = transpile(source)
    namespace = {"__name__": "__aura_prop__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, "<prop>", "exec"), namespace)
        main = namespace.get("main")
        if callable(main):
            main()
    return buffer.getvalue()


def run_aura_ns(source):
    """Like ``run_aura`` but also return the namespace."""
    code = transpile(source)
    namespace = {"__name__": "__aura_prop__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, "<prop>", "exec"), namespace)
        main = namespace.get("main")
        if callable(main):
            main()
    return buffer.getvalue(), namespace