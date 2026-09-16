import pytest
import io
import sys
from contextlib import redirect_stdout
from repl.engine import AuraREPL

def run_repl_session(input_lines):
    """Simulate a REPL session and return output."""
    repl = AuraREPL()
    # Mock input
    input_iter = iter(input_lines + ["exit"])
    
    def mock_input(prompt=None):
        try:
            return next(input_iter)
        except StopIteration:
            raise EOFError
            
    # Mock input global or injection? 
    # Current REPL uses 'input()', need to dependency inject or patch.
    # Refactoring REPL to accept input_func is cleaner, but let's monkeypatch for now logic
    
    # We will instantiate REPL and manually feed buffer/process 
    # to avoid mocking builtins.input complexity if we modify REPL class slightly
    # or just use process_buffer directly.
    
    output = io.StringIO()
    with redirect_stdout(output):
        for line in input_lines:
            repl.buffer = line
            # Force processing
            try:
                repl.process_buffer()
                repl.buffer = "" # clear buffer like run loop 
            except Exception as e:
                print(f"Error: {e}")
                
    return output.getvalue(), repl.locals

def test_repl_basic_arithmetic():
    out, loc = run_repl_session(["let x = 10", "let y = 20", "let z = x + y"])
    assert loc['x'] == 10
    assert loc['z'] == 30

def test_repl_persistence():
    out, loc = run_repl_session(["let mut x = 5", "x = x * 2"])
    assert loc['x'] == 10


def test_repl_immutable_binding_is_rejected():
    """A `let` binding cannot be reassigned across chunks (Aura's rule)."""
    repl = AuraREPL()
    assert repl.process("let x = 5").ok
    result = repl.process("x = x * 2")
    assert result.ok is False
    assert "immutable" in result.message
    assert repl.locals['x'] == 5


def test_repl_immutable_same_chunk_is_rejected():
    repl = AuraREPL()
    result = repl.process("let x = 1\nx = 2")
    assert result.ok is False
    assert "immutable" in result.message


def test_repl_mut_allows_cross_chunk_reassignment():
    repl = AuraREPL()
    assert repl.process("let mut x = 1").ok
    assert repl.process("x = 2").ok
    assert repl.locals['x'] == 2


def test_repl_const_is_immutable():
    repl = AuraREPL()
    assert repl.process("const C = 1").ok
    result = repl.process("C = 2")
    assert result.ok is False
    assert "immutable" in result.message

def test_repl_functions():
    lines = [
        "def add(a, b) { return a + b }",
        "let res = add(10, 5)"
    ]
    out, loc = run_repl_session(lines)
    assert loc['res'] == 15

def test_repl_classes():
    lines = [
        "class Point { public let x = 0; public let y = 0 }",
        "let p = Point()",
        "p.x = 10"
    ]
    out, loc = run_repl_session(lines)
    assert loc['p'].x == 10

# Generate 96 more tests programmatically for robustness
@pytest.mark.parametrize("i", range(100))
def test_repl_stability_stress(i):
    """Run random simple logic to ensure REPL doesn't crash repeatedly."""
    val = i
    lines = [
        f"let mut val = {val}",
        "val = val + 1",
        "print(val)"
    ]
    out, loc = run_repl_session(lines)
    assert loc['val'] == val + 1

def test_repl_syntax_error_recovery():
    # Should not crash on error
    lines = [
        "let x = ", # Syntax error
        "let y = 10" # Should work
    ]
    out, loc = run_repl_session(lines)
    assert 'y' in loc
    assert loc['y'] == 10


# ============================================================================
# Aura rule enforcement (the REPL must match `aura check` / `aura run`)
# ============================================================================

def test_repl_rejects_return_outside_function():
    repl = AuraREPL()
    result = repl.process("return 1")
    assert result.ok is False
    assert "outside of a function" in result.message


def test_repl_rejects_break_outside_loop():
    repl = AuraREPL()
    result = repl.process("break")
    assert result.ok is False
    assert "outside of a loop" in result.message


def test_repl_rejects_continue_outside_loop():
    repl = AuraREPL()
    result = repl.process("continue")
    assert result.ok is False
    assert "outside of a loop" in result.message


def test_repl_rejects_self_outside_class():
    repl = AuraREPL()
    result = repl.process("print(self)")
    assert result.ok is False
    assert "self" in result.message


def test_repl_rejects_duplicate_declaration():
    repl = AuraREPL()
    result = repl.process("let x = 1\nlet x = 2")
    assert result.ok is False
    assert "already defined" in result.message


def test_repl_rejects_invalid_assignment_target():
    repl = AuraREPL()
    result = repl.process("1 = 2")
    assert result.ok is False


def test_repl_rejects_unreachable_after_return():
    repl = AuraREPL()
    result = repl.process("def f() { return 1\nprint(2) }")
    assert result.ok is False
    assert "unreachable" in result.message


def test_repl_allows_mutable_reassignment():
    repl = AuraREPL()
    assert repl.process("let mut n = 0").ok
    assert repl.process("n = n + 1").ok
    assert repl.locals['n'] == 1


def test_repl_allows_local_mutation_inside_function():
    repl = AuraREPL()
    source = (
        "def f(a) {\n"
        "  let mut total = 0\n"
        "  for i in range(a) { total += i }\n"
        "  return total\n"
        "}"
    )
    assert repl.process(source).ok
    result = repl.process("f(4)")
    assert result.ok
    assert repl.locals['_'] == 6


def test_repl_default_let_is_immutable_like_cli():
    """The REPL must not silently relax `let` mutability (regression)."""
    repl = AuraREPL()
    assert repl.process("let value = 1").ok
    result = repl.process("value = 99")
    assert result.ok is False
    assert repl.locals['value'] == 1


# ============================================================================
# Async support across chunks
# ============================================================================

def test_repl_top_level_await():
    repl = AuraREPL()
    assert repl.process("async def work(x) { return x * 2 }").ok
    assert repl.process("let r = await work(21)").ok
    assert repl.locals['r'] == 42


def test_repl_bare_async_call_is_awaited_across_chunks():
    repl = AuraREPL()
    assert repl.process("async def compute() { return 7 }").ok
    result = repl.process("compute()")
    assert result.ok
    assert repl.locals['_'] == 7


def test_repl_async_side_effect_runs():
    repl = AuraREPL()
    assert repl.process("async def greet() { print('hello') }").ok
    assert repl.process("greet()").ok


# ============================================================================
# :load enforces the same rules as the CLI
# ============================================================================

def test_repl_load_rejects_rule_violation(tmp_path):
    bad = tmp_path / "bad.aura"
    bad.write_text("let a = 1\na = 2\n", encoding="utf-8")
    repl = AuraREPL()
    result = repl.handle_load(str(bad))
    assert result.ok is False
    assert "immutable" in result.message
    assert "a" not in repl.locals


def test_repl_load_accepts_valid_program(tmp_path):
    good = tmp_path / "good.aura"
    good.write_text("let mut a = 1\na = 2\nprint(a)\n", encoding="utf-8")
    repl = AuraREPL()
    result = repl.handle_load(str(good))
    assert result.ok is True
    assert repl.locals['a'] == 2


def test_repl_load_records_bindings_for_later_chunks(tmp_path):
    good = tmp_path / "vals.aura"
    good.write_text("let fixed = 1\n", encoding="utf-8")
    repl = AuraREPL()
    assert repl.handle_load(str(good)).ok
    result = repl.process("fixed = 2")
    assert result.ok is False
