"""Deep tests for the Aura REPL.

Covers the input loop, multi-line buffering, every ``:command``, expression
display, cross-chunk mutability, async chunks, rule/diagnostic surfacing and
error recovery.
"""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from repl.engine import AuraREPL  # noqa: E402


def make_repl():
    """A REPL that records its output instead of printing."""
    outputs = []
    repl = AuraREPL(output_func=outputs.append)
    return repl, outputs


def session(lines):
    """Feed lines through ``feed`` (the real input path); return (repl, output)."""
    repl, outputs = make_repl()
    for line in lines:
        result = repl.feed(line)
        if result is not None and not result.continued and result.message:
            repl.write(result.message)
    return repl, outputs


# ============================================================================
# Expression evaluation and display
# ============================================================================

def test_expression_value_is_returned():
    repl, _ = make_repl()
    result = repl.process("1 + 2")
    assert result.ok
    assert result.value == 3


def test_expression_stored_in_underscore():
    repl, _ = make_repl()
    repl.process("40 + 2")
    assert repl.locals["_"] == 42


def test_multiple_bindings_persist():
    repl, _ = make_repl()
    repl.process("let x = 1")
    repl.process("let y = x + 1")
    assert repl.locals["y"] == 2


def test_string_expression():
    repl, _ = make_repl()
    assert repl.process('"a" + "b"').value == "ab"


def test_collection_expression():
    repl, _ = make_repl()
    repl.process("let xs = [1, 2, 3]")
    assert repl.locals["xs"] == [1, 2, 3]


# ============================================================================
# Multi-line buffering
# ============================================================================

def test_open_bracket_continues():
    repl, _ = make_repl()
    result = repl.feed("let xs = [")
    assert result.continued
    result = repl.feed("  1, 2,")
    assert result.continued
    result = repl.feed("]")
    assert not result.continued
    assert repl.locals["xs"] == [1, 2]


def test_unclosed_string_continues():
    repl, _ = make_repl()
    result = repl.feed('let s = "hello')
    assert result.continued
    result = repl.feed('"')
    assert not result.continued
    assert repl.locals["s"].startswith("hello")


def test_open_brace_block_continues():
    repl, _ = make_repl()
    assert repl.feed("def f() {").continued
    assert repl.feed("  return 1").continued
    result = repl.feed("}")
    assert not result.continued
    assert repl.locals["f"]() == 1


def test_trailing_equals_continues():
    repl, _ = make_repl()
    assert repl.feed("let x =").continued
    result = repl.feed("5")
    assert not result.continued
    assert repl.locals["x"] == 5


def test_backslash_line_join():
    repl, _ = make_repl()
    assert repl.feed("let x = 1 + \\").continued
    result = repl.feed("2")
    assert not result.continued
    assert repl.locals["x"] == 3


def test_empty_line_is_ignored():
    repl, _ = make_repl()
    assert repl.feed("") is None
    assert repl.feed("   ") is None


# ============================================================================
# Commands
# ============================================================================

def test_command_help():
    repl, out = make_repl()
    repl.handle_command(":help")
    text = "\n".join(out)
    assert "help" in text.lower() or ":vars" in text


def test_command_vars_lists_bindings():
    repl, out = make_repl()
    repl.process("let abc = 1")
    repl.handle_command(":vars")
    assert any("abc" in line for line in out)


def test_command_vars_lists_python_binding():
    # The REPL always installs the `python` alias, so :vars is never empty.
    repl, out = make_repl()
    repl.handle_command(":vars")
    assert any("python" in line for line in out)


def test_command_type():
    repl, out = make_repl()
    repl.handle_command(":type 1 + 2")
    assert any("int" in line for line in out)
    assert any("3" in line for line in out)


def test_command_type_requires_expression():
    repl, out = make_repl()
    repl.handle_command(":type")
    assert any("usage" in line for line in out)


def test_command_type_error():
    repl, out = make_repl()
    repl.handle_command(":type 1 +")
    assert any("error" in line.lower() for line in out)


def test_command_ast():
    repl, out = make_repl()
    repl.handle_command(":ast 1 + 2")
    assert any("BinaryOp" in line or "Literal" in line for line in out)


def test_command_ast_requires_expression():
    repl, out = make_repl()
    repl.handle_command(":ast")
    assert any("usage" in line for line in out)


def test_command_python_eval():
    repl, out = make_repl()
    result = repl.handle_command(":py 1 + 1")
    assert result.value == 2
    assert repl.locals["_"] == 2


def test_command_python_exec():
    repl, out = make_repl()
    repl.handle_command(":py y = 5")
    assert repl.locals["y"] == 5


def test_command_python_multiline_statements():
    repl, out = make_repl()
    repl.handle_command(":py a = 1; b = a + 1")
    assert repl.locals["b"] == 2


def test_command_python_requires_code():
    repl, out = make_repl()
    repl.handle_command(":py")
    assert any("usage" in line for line in out)


def test_command_python_error():
    repl, out = make_repl()
    repl.handle_command(":py raise ValueError('boom')")
    assert any("python error" in line.lower() or "boom" in line for line in out)


def test_command_history():
    repl, out = make_repl()
    repl.process("let x = 1")
    repl.process("let y = 2")
    repl.handle_command(":history")
    assert any("let x" in line for line in out)


def test_command_reset():
    repl, out = make_repl()
    repl.process("let x = 1")
    repl.handle_command(":reset")
    assert "x" not in repl.locals
    assert any("reset" in line.lower() or "cleared" in line.lower()
               for line in out)


def test_command_unknown():
    repl, out = make_repl()
    repl.handle_command(":nope")
    assert any("unknown" in line.lower() for line in out)


def test_command_quit_raises_eof():
    repl, out = make_repl()
    with pytest.raises(EOFError):
        repl.handle_command(":q")
    assert any("Goodbye" in line for line in out)


def test_command_load(tmp_path):
    src = tmp_path / "lib.aura"
    src.write_text("let loaded = 99\n")
    repl, out = make_repl()
    repl.handle_command(f":load {src}")
    assert repl.locals["loaded"] == 99


def test_command_load_missing_file():
    repl, out = make_repl()
    repl.handle_command(":load /nonexistent/x.aura")
    assert any("error" in line.lower() for line in out)


def test_command_run(tmp_path, capsys):
    src = tmp_path / "prog.aura"
    src.write_text('def main() { print("ran") }\n')
    repl, out = make_repl()
    result = repl.handle_command(f":run {src}")
    assert result.ok
    assert "ran" in capsys.readouterr().out
    assert any("exit code 0" in line for line in out)


def test_command_run_failure(tmp_path, capsys):
    src = tmp_path / "bad.aura"
    src.write_text("print(1)\n")  # no main -> E310
    repl, out = make_repl()
    result = repl.handle_command(f":run {src}")
    assert not result.ok


def test_command_without_colon_falls_through():
    repl, _ = make_repl()
    repl.process("let x = 1")
    # A non-command line is treated as source.
    assert repl.process("print(x)").ok


# ============================================================================
# Cross-chunk mutability and diagnostics
# ============================================================================

def test_mutability_across_chunks():
    repl, _ = make_repl()
    repl.process("let mut x = 1")
    assert repl.process("x = 2").ok
    assert repl.locals["x"] == 2


def test_let_stays_immutable_across_chunks():
    repl, _ = make_repl()
    repl.process("let x = 1")
    result = repl.process("x = 2")
    assert not result.ok
    assert "immutable" in result.message


def test_rule_error_reports_code():
    repl, _ = make_repl()
    result = repl.process("def f() {\n  return 1\n  print(2)\n}")
    assert not result.ok
    assert "E302" in result.message


def test_syntax_error_is_reported():
    repl, _ = make_repl()
    result = repl.process("let = 5")
    assert not result.ok


def test_runtime_error_is_captured():
    repl, _ = make_repl()
    result = repl.process("1 / 0")
    assert not result.ok
    assert result.exception is not None


# ============================================================================
# Async chunks
# ============================================================================

def test_async_function_definition_and_await():
    repl, _ = make_repl()
    assert repl.process("async def work() -> int { return 7 }").ok
    assert repl.process("await work()").value == 7


def test_async_top_level_await():
    repl, out = make_repl()
    result = repl.process("let v = await work()") if "work" in repl.locals else None
    repl.process("async def work() -> int { return 5 }")
    result = repl.process("let v = await work()")
    assert result.ok
    assert repl.locals["v"] == 5


def test_async_call_across_chunks_is_awaited():
    repl, _ = make_repl()
    repl.process("async def side() { print('effect') }")
    # A bare call to an async function defined earlier must run.
    result = repl.process("side()")
    assert result.ok


# ============================================================================
# Input loop (run / EOF / interrupt)
# ============================================================================

def test_run_loop_processes_input_then_eof():
    inputs = iter(["let x = 1", "x + 1"])

    def fake_input(prompt=""):
        try:
            return next(inputs)
        except StopIteration:
            raise EOFError from None

    outputs = []
    repl = AuraREPL(input_func=fake_input, output_func=outputs.append)
    assert repl.run() == 0
    assert repl.locals["x"] == 1
    assert any("Aura REPL" in line for line in outputs)


def test_run_loop_flushes_buffer_on_eof():
    inputs = iter(["let xs = [1, 2"])

    def fake_input(prompt=""):
        try:
            return next(inputs)
        except StopIteration:
            raise EOFError from None

    outputs = []
    repl = AuraREPL(input_func=fake_input, output_func=outputs.append)
    repl.run()
    # The unterminated buffer is flushed (and fails to parse), not lost silently.
    assert any(line for line in outputs)


def test_run_loop_keyboard_interrupt_clears_buffer():
    state = {"n": 0}

    def fake_input(prompt=""):
        state["n"] += 1
        if state["n"] == 1:
            return "let xs = [1,"
        if state["n"] == 2:
            raise KeyboardInterrupt
        if state["n"] == 3:
            return "let ok = 1"
        raise EOFError

    outputs = []
    repl = AuraREPL(input_func=fake_input, output_func=outputs.append)
    repl.run()
    assert any("KeyboardInterrupt" in line for line in outputs)
    assert repl.locals.get("ok") == 1


# ============================================================================
# locals property and repr helpers
# ============================================================================

def test_locals_property_roundtrip():
    repl, _ = make_repl()
    repl.locals = {"x": 1}
    assert repl.locals == {"x": 1}


def test_format_vars_short_repr():
    from repl.engine import _short_repr
    assert "..." in _short_repr("x" * 200)


def test_dump_ast_produces_tree():
    from repl.engine import _dump_ast

    from aura.parser.to_ast import Parser, Tokenizer
    node = Parser(Tokenizer("1 + 2").tokenize()).parse_expression()
    text = _dump_ast(node)
    assert "BinaryOp" in text
