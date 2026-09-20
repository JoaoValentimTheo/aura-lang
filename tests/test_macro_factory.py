"""Phase 1 — AuraMacroFactory (compile-time macros) tests.

Covers the engine in ``aura.transpiler.macro_factory`` directly and its
integration with the transformer: registration, arity validation, hygienic
expansion, quoting, and the built-in macros used from Aura source.
"""

import contextlib
import io
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Parser, Tokenizer  # noqa: E402
from transpiler.ast import (  # noqa: E402
    CallExpr,
    Identifier,
    IntLiteral,
    ReturnStmt,
)
from transpiler.macro_factory import (  # noqa: E402
    MacroError,
    MacroRegistry,
    Quote,
    default_registry,
    gensym,
)
from transpiler.transformer import Transformer  # noqa: E402


def transpile(source: str, registry=None) -> str:
    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    transformer = Transformer()
    if registry is not None:
        transformer.expr_transformer.macro_registry = registry
    return transformer.transform(program)


def run_aura(source: str, registry=None):
    """Transpile, execute, and return ``(stdout, generated_python)``."""
    code = transpile(source, registry=registry)
    namespace = {"__name__": "__aura_macro__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, namespace)  # noqa: S102 - test harness
    return buffer.getvalue().strip(), code


# ============================================================================
# Engine: registry and quoting
# ============================================================================

def test_registry_contains_and_names():
    registry = MacroRegistry()

    def impl(args, kwargs):
        return args[0]

    registry.register("one", impl, min_args=1, max_args=1)
    assert "one" in registry
    assert registry.names() == ("one",)
    assert len(registry) == 1
    assert "missing" not in registry


def test_registry_rejects_duplicate_names():
    registry = MacroRegistry()
    registry.register("dup", lambda a, k: a[0])
    with pytest.raises(MacroError, match="already registered"):
        registry.register("dup", lambda a, k: a[0])


def test_registry_rejects_bad_names_and_impls():
    registry = MacroRegistry()
    with pytest.raises(MacroError, match="not an identifier"):
        registry.register("not a name", lambda a, k: a[0])
    with pytest.raises(MacroError, match="not callable"):
        registry.register("ok", 42)


def test_macro_decorator_form():
    registry = MacroRegistry()

    @registry.macro("twice", min_args=1, max_args=1)
    def twice(args, kwargs):
        return Quote.binary("*", args[0], Quote.int_literal(2))

    assert "twice" in registry


def test_quote_validates_inputs():
    with pytest.raises(MacroError, match="valid identifier"):
        Quote.identifier("1bad")
    with pytest.raises(MacroError, match="int literal"):
        Quote.int_literal("x")
    with pytest.raises(MacroError, match="string literal"):
        Quote.str_literal(5)
    with pytest.raises(MacroError, match="call target"):
        Quote.call(42)


def test_gensym_is_unique_and_valid():
    names = {gensym("t") for _ in range(50)}
    assert len(names) == 50
    assert all(n.isidentifier() for n in names)


def test_expand_call_ignores_member_calls():
    registry = MacroRegistry()
    registry.register("m", lambda a, k: a[0])
    call = CallExpr(Identifier("other"), [IntLiteral(1)])
    assert registry.expand_call(call) is None


# ============================================================================
# Engine: arity and errors
# ============================================================================

def test_arity_too_few():
    registry = default_registry()
    call = CallExpr(Identifier("assert_eq"), [IntLiteral(1)])
    with pytest.raises(MacroError, match="expects 2 argument"):
        registry.expand_call(call)


def test_arity_too_many():
    registry = MacroRegistry()
    registry.register("one", lambda a, k: a[0], min_args=1, max_args=1)
    call = CallExpr(Identifier("one"), [IntLiteral(1), IntLiteral(2)])
    with pytest.raises(MacroError, match="got 2"):
        registry.expand_call(call)


def test_kwargs_rejected_when_not_allowed():
    registry = MacroRegistry()
    registry.register("one", lambda a, k: a[0], min_args=1, max_args=1)
    call = CallExpr(Identifier("one"), [IntLiteral(1)], kwargs={"x": IntLiteral(2)})
    with pytest.raises(MacroError, match="does not accept keyword"):
        registry.expand_call(call)


def test_macro_must_return_expression():
    registry = MacroRegistry()
    registry.register("bad", lambda a, k: "a string", min_args=0)
    call = CallExpr(Identifier("bad"), [])
    with pytest.raises(MacroError, match="must return an expression"):
        registry.expand_call(call)


# ============================================================================
# Built-in macros (engine level)
# ============================================================================

def test_default_registry_has_builtins():
    names = default_registry().names()
    assert names == (
        "assert_eq", "assert_ne", "debug_value", "discard", "identity",
        "once", "retry", "static_assert", "stringify", "swap", "todo",
        "unreachable")


def test_default_registry_is_fresh_each_call():
    a = default_registry()
    b = default_registry()
    a.unregister("identity")
    assert "identity" in b


# ============================================================================
# Integration: built-ins from Aura source
# ============================================================================

def test_identity_macro():
    source = "def main() { print(identity(41) + 1) }\nmain()"
    out, _ = run_aura(source)
    assert out == "42"


def test_discard_macro_evaluates_operand():
    source = (
        "def main() { discard(compute())\n print('done') }\n"
        "def compute() { print('side'); return 1 }\nmain()"
    )
    out, _ = run_aura(source)
    assert out == "side\ndone"


def test_assert_eq_passes():
    source = "def main() { assert_eq(2 * 3, 6)\n print('ok') }\nmain()"
    out, _ = run_aura(source)
    assert out == "ok"


def test_assert_eq_failure_shows_both_values():
    source = "def main() { assert_eq(1, 2) }\nmain()"
    with pytest.raises(AssertionError, match="1 != 2"):
        run_aura(source)


def test_assert_eq_evaluates_operands_once():
    """A side-effecting operand must run a single time in the expansion."""
    source = (
        "let mut calls = 0\n"
        "def bump() { calls += 1\n return calls }\n"
        "def main() { assert_eq(bump(), 1)\n print(calls) }\n"
        "main()"
    )
    out, _ = run_aura(source)
    assert out == "1"


def test_todo_macro_raises():
    source = "def main() { todo('later') }\nmain()"
    with pytest.raises(Exception, match="later"):
        run_aura(source)


def test_todo_macro_default_message():
    source = "def main() { todo() }\nmain()"
    with pytest.raises(Exception, match="not implemented"):
        run_aura(source)


def test_macro_arity_error_surfaces():
    source = "def main() { assert_eq(1) }\nmain()"
    with pytest.raises(MacroError, match="expects 2 argument"):
        transpile(source)


# ============================================================================
# Integration: user-defined macros via a custom registry
# ============================================================================

def test_user_macro_expands_at_compile_time():
    registry = MacroRegistry()

    @registry.macro("double", min_args=1, max_args=1)
    def double(args, kwargs):
        return Quote.binary("*", args[0], Quote.int_literal(2))

    source = "def main() { print(double(21)) }\nmain()"
    out, code = run_aura(source, registry=registry)
    assert out == "42"
    # The macro is gone from the emitted Python: only the expansion remains.
    assert "double" not in code
    assert "* 2" in code


def test_user_macro_is_hygienic():
    """A macro binding named like a call-site variable must not collide."""
    registry = MacroRegistry()

    @registry.macro("shadowy", min_args=1, max_args=1)
    def shadowy(args, kwargs):
        """Bind a hygienic name, return the operand, and leave no collision."""
        from transpiler.ast import BlockExpr, ExprStmt, VarDecl
        hidden = gensym("hidden")
        return BlockExpr([
            VarDecl(hidden, False, value=Quote.int_literal(999)),
            ExprStmt(args[0]),
        ])

    # The call-site `hidden` must keep its own value; the macro's hygienic
    # `hidden` lives only inside the expansion.
    source = (
        "def main() { let hidden = 100\n print(shadowy(hidden)) }\nmain()"
    )
    out, _ = run_aura(source, registry=registry)
    assert out == "100"


def test_unknown_macro_name_is_a_normal_call():
    """An unregistered name must still compile as an ordinary call."""
    registry = MacroRegistry()  # empty: nothing expands
    source = "def not_a_macro(x) { return x + 1 }\ndef main() { print(not_a_macro(1)) }\nmain()"
    out, _ = run_aura(source, registry=registry)
    assert out == "2"


def test_macro_registry_is_reset_between_programs():
    """A reused Transformer must not leak macro state across programs."""
    registry = MacroRegistry()

    @registry.macro("inline", min_args=1, max_args=1)
    def inline(args, kwargs):
        return args[0]

    transformer = Transformer()
    transformer.expr_transformer.macro_registry = registry

    def compile_with(t):
        tokens = Tokenizer("def main() { print(inline(7)) }\nmain()").tokenize()
        return t.transform(Parser(tokens).parse())

    first = compile_with(transformer)
    assert "inline" not in first
    second = compile_with(transformer)
    assert "inline" not in second


# ============================================================================
# Introspection helpers (a macro inspects operands, not only pastes them)
# ============================================================================

def test_literal_value_reads_each_literal_kind():
    from transpiler.ast import BoolLiteral, FloatLiteral, NoneLiteral, StrLiteral
    from transpiler.macro_factory import _NOT_A_LITERAL, literal_value

    assert literal_value(IntLiteral(3)) == 3
    assert literal_value(FloatLiteral(1.5)) == 1.5
    assert literal_value(StrLiteral("s")) == "s"
    assert literal_value(BoolLiteral(True)) is True
    assert literal_value(NoneLiteral()) is None
    assert literal_value(Identifier("x")) is _NOT_A_LITERAL


def test_name_of_identifies_names():
    from transpiler.ast import MemberExpr
    from transpiler.macro_factory import name_of

    assert name_of(Identifier("foo")) == "foo"
    assert name_of(MemberExpr(Identifier("a"), "b")) == "a.b"
    assert name_of(IntLiteral(1)) is None


def test_is_identifier_named():
    from transpiler.macro_factory import is_identifier_named

    assert is_identifier_named(Identifier("x"), "x")
    assert not is_identifier_named(Identifier("y"), "x")
    assert not is_identifier_named(IntLiteral(1), "1")


def test_contains_identifier_walks_nested_nodes():
    from transpiler.ast import BinaryOp
    from transpiler.macro_factory import contains_identifier

    tree = BinaryOp(Identifier("a"), "+", BinaryOp(Identifier("b"), "*", IntLiteral(2)))
    assert contains_identifier(tree, "a")
    assert contains_identifier(tree, "b")
    assert not contains_identifier(tree, "c")


def test_macro_can_branch_on_a_literal_operand():
    """A macro may choose its expansion from the shape of the operand."""
    registry = MacroRegistry()

    @registry.macro("scale", min_args=1, max_args=1)
    def scale(args, kwargs):
        from transpiler.macro_factory import _NOT_A_LITERAL, literal_value
        value = literal_value(args[0])
        if value is not _NOT_A_LITERAL:
            return Quote.int_literal(value * 10)
        return Quote.binary("*", args[0], Quote.int_literal(10))

    source = "def main() { let x = 4\n print(scale(3), scale(x)) }\nmain()"
    out, _ = run_aura(source, registry=registry)
    assert out == "30 40"


# ============================================================================
# Statement-producing macros (an expansion may be more than one expression)
# ============================================================================

def test_macro_can_emit_statements_then_a_value():
    registry = MacroRegistry()

    @registry.macro("twice_eval", min_args=1, max_args=1)
    def twice_eval(args, kwargs):
        once = gensym("v")
        twice = gensym("w")
        return Quote.block(
            Quote.var(once, args[0]),
            Quote.var(twice, args[0]),
            Quote.binary("+", Identifier(once), Identifier(twice)),
        )

    source = (
        "let mut calls = 0\n"
        "def bump() { calls += 1\n return calls }\n"
        "def main() { print(twice_eval(bump()), calls) }\nmain()"
    )
    out, code = run_aura(source, registry=registry)
    assert out == "3 2"


def test_macro_can_define_a_helper_function():
    registry = MacroRegistry()

    @registry.macro("with_helper", min_args=1, max_args=1)
    def with_helper(args, kwargs):
        helper = gensym("helper")
        return Quote.block(
            Quote.function(helper, [], [ReturnStmt(args[0])]),
            Quote.call(helper),
        )

    source = "def main() { print(with_helper(21 * 2)) }\nmain()"
    out, _ = run_aura(source, registry=registry)
    assert out == "42"


def test_quote_validates_statement_inputs():
    with pytest.raises(MacroError, match="expression statement"):
        Quote.expr_stmt("not a node")
    with pytest.raises(MacroError, match="block"):
        Quote.block()
    with pytest.raises(MacroError, match="statements or expressions"):
        Quote.block("nope")


# ============================================================================
# Built-ins added by the expanded factory
# ============================================================================

def test_assert_ne_passes_and_fails():
    source = "def main() { assert_ne(1, 2)\n print('ok') }\nmain()"
    out, _ = run_aura(source)
    assert out == "ok"
    with pytest.raises(AssertionError, match="1 == 1"):
        run_aura("def main() { assert_ne(1, 1) }\nmain()")


def test_unreachable_macro_raises():
    with pytest.raises(Exception, match="should not get here"):
        run_aura("def main() { unreachable('should not get here') }\nmain()")


def test_static_assert_true_is_a_no_op():
    source = "def main() { static_assert(true)\n print('ok') }\nmain()"
    out, _ = run_aura(source)
    assert out == "ok"


def test_static_assert_false_fails_at_compile_time():
    with pytest.raises(MacroError, match="static assertion failed"):
        transpile("def main() { static_assert(false, 'nope') }\nmain()")


def test_static_assert_rejects_non_literal():
    with pytest.raises(MacroError, match="literal condition"):
        transpile("def main() { let x = 1\n static_assert(x) }\nmain()")


def test_stringify_literal_folds_at_compile_time():
    out, code = run_aura("def main() { print(stringify(42)) }\nmain()")
    assert out == "42"
    assert "'42'" in code


def test_stringify_non_literal_emits_str_call():
    out, code = run_aura("def main() { let x = 5\n print(stringify(x)) }\nmain()")
    assert out == "5"
    assert "str(x)" in code


def test_debug_value_prints_once_and_returns_value():
    source = (
        "let mut calls = 0\n"
        "def bump() { calls += 1\n return 7 }\n"
        "def main() { print(debug_value(bump()))\n print(calls) }\nmain()"
    )
    out, _ = run_aura(source)
    assert out == "bump = 7\n7\n1"


def test_swap_exchanges_two_locals():
    source = (
        "def main() { let mut a = 1\n let mut b = 2\n"
        " swap(a, b)\n print(a, b) }\nmain()"
    )
    out, _ = run_aura(source)
    assert out == "2 1"


def test_swap_uses_a_hygienic_temporary():
    """The expansion must not leak its temporaries into the emitted Python."""
    code = transpile(
        "def main() { let mut a = 1\n let mut b = 2\n swap(a, b) }\nmain()")
    assert "__aura_swap_" in code
    # The call-site names are untouched.
    assert "\n    a = 1" in code and "\n    b = 2" in code
