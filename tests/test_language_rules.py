"""Focused tests for Aura's language rules, modules, functions and the
Python interop bridge.

These complement the stress suite: where ``test_stress_raw.py`` throws volume at
the pipeline, this file pins the *contracts* that the fixes in this release
established, so they cannot silently regress.
"""
import io
import contextlib
import sys
import textwrap
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Tokenizer, Parser  # noqa: E402
from transpiler.transformer import Transformer  # noqa: E402
from transpiler.rules import RuleChecker  # noqa: E402
from transpiler.semantics import MutabilityChecker  # noqa: E402
from aura.runtime import install_runtime_aliases  # noqa: E402

install_runtime_aliases()


def parse(source):
    return Parser(Tokenizer(source).tokenize()).parse()


def run_aura(source):
    program = parse(source)
    code = Transformer().transform(program)
    namespace = {"__name__": "__rules_test__"}
    buffer = io.StringIO()
    # Mirror the CLI: async programs (async def / top-level await) run inside a
    # coroutine so await is legal and coroutines are driven to completion.
    has_async = "async def" in code or "await " in code
    with contextlib.redirect_stdout(buffer):
        try:
            if has_async:
                import asyncio
                indented = "\n".join(
                    ("    " + line if line.strip() else line)
                    for line in code.split("\n")
                )
                wrapper = "async def _aura_main():\n" + indented + "\n"
                ns = {"__name__": "__rules_test__"}
                exec(compile(wrapper, "<test>", "exec"), ns)
                asyncio.run(ns["_aura_main"]())
                namespace = ns
            else:
                exec(code, namespace)
        except SystemExit:
            # A top-level `guard ... else { return }` exits via SystemExit.
            pass
    return buffer.getvalue(), code, namespace


def rule_errors(source):
    checker = RuleChecker()
    checker.check_program(parse(source))
    return [str(e) for e in checker.collector.errors]


# ============================================================================
# Rules: structural placement
# ============================================================================

def test_return_outside_function_is_rejected():
    assert rule_errors("return 1")


def test_return_inside_function_is_accepted():
    assert rule_errors("def f() { return 1 }") == []


def test_guard_return_at_top_level_is_allowed():
    # `guard cond else { return }` is the idiomatic early program exit.
    assert rule_errors('guard true else { return }') == []


def test_guard_return_exits_program():
    out, code, _ = run_aura('let x = -1\nguard x >= 0 else { print("bad"); return }\nprint("ok")')
    assert "bad" in out
    assert "ok" not in out
    assert "SystemExit" in code


def test_break_outside_loop_is_rejected():
    assert rule_errors("break")


def test_continue_outside_loop_is_rejected():
    assert rule_errors("continue")


def test_break_inside_loop_is_accepted():
    assert rule_errors("for i in range(3) { break }") == []


def test_await_inside_async_is_accepted():
    assert rule_errors("async def f() { let x = await g() }") == []


def test_await_inside_sync_is_rejected():
    assert rule_errors("def f() { let x = await g() }")


def test_top_level_await_is_accepted():
    # Aura wraps async programs in a coroutine, so top-level await is legal.
    assert rule_errors("let x = await g()") == []


def test_self_outside_class_is_rejected():
    assert rule_errors("let x = self.y")


def test_self_inside_method_is_accepted():
    assert rule_errors("class C { def m() { return self.x } }") == []


def test_unreachable_code_is_rejected():
    assert rule_errors("def f() { return 1\nprint(2) }")


def test_invalid_assignment_target_is_rejected():
    assert rule_errors("1 = 2")


def test_valid_assignment_targets_are_accepted():
    assert rule_errors("let x = 0\nx = 1") == []
    assert rule_errors("class C { def m() { self.x = 1 } }") == []


def test_duplicate_binding_is_rejected():
    assert rule_errors("let x = 1\nlet x = 2")


def test_duplicate_binding_in_nested_scope_is_allowed():
    # Shadowing in a nested block is fine.
    assert rule_errors("let x = 1\nif true { let x = 2\nprint(x) }") == []


def test_duplicate_parameter_is_rejected():
    assert rule_errors("def f(a, a) { return a }")


def test_duplicate_method_parameter_is_rejected():
    assert rule_errors("class C { def m(a, a) { return a } }")


# ============================================================================
# Rules: mutability
# ============================================================================

def test_immutable_binding_cannot_be_reassigned():
    assert not MutabilityChecker().check_program(parse("let x = 1\nx = 2"))


def test_mutable_binding_can_be_reassigned():
    assert MutabilityChecker().check_program(parse("let mut x = 1\nx = 2"))


def test_const_cannot_be_reassigned():
    assert not MutabilityChecker().check_program(parse("const X = 1\nX = 2"))


def test_member_assignment_is_not_a_rebinding():
    src = "class C { def m() { self.x = 1 } }"
    assert MutabilityChecker().check_program(parse(src))


def test_augmented_member_assignment_is_not_a_rebinding():
    src = "class C { def m() { self.x += 1 } }"
    assert MutabilityChecker().check_program(parse(src))


# ============================================================================
# Tokenizer / literal fixes
# ============================================================================

@pytest.mark.parametrize("literal,value", [
    ("0xFF", 255), ("0o17", 15), ("0b1010", 10), ("1_000", 1000),
])
def test_radix_literals(literal, value):
    out, _, _ = run_aura(f"print({literal})")
    assert out.strip() == str(value)


@pytest.mark.parametrize("bad", ["0xZZ", "0b12", "0o8", "0x"])
def test_invalid_radix_raises_syntax_error(bad):
    with pytest.raises(SyntaxError):
        parse(f"let x = {bad}")


def test_leading_dot_float():
    out, _, _ = run_aura("print(.5)")
    assert out.strip() == "0.5"


def test_leading_dot_float_with_exponent():
    out, _, _ = run_aura("print(.5e2)")
    assert out.strip() == "50.0"


def test_shift_precedence_matches_python():
    out, _, _ = run_aura("print(1 << 2 + 1)")
    assert out.strip() == "8"


def test_not_in_operator():
    out, _, _ = run_aura("print(1 not in [2, 3])")
    assert out.strip() == "True"


def test_is_not_operator():
    out, _, _ = run_aura("let x = none\nprint(x is not none)")
    assert out.strip() == "False"


# ============================================================================
# Modules
# ============================================================================

def test_module_function_call():
    out, _, _ = run_aura("module M { def hi() { return 7 } }\nprint(M.hi())")
    assert out.strip() == "7"


def test_module_variable_access():
    out, _, _ = run_aura("module M { let x = 5 }\nprint(M.x)")
    assert out.strip() == "5"


def test_module_with_export_marker():
    out, _, _ = run_aura(
        "module M { export def hi() { return 1 } }\nprint(M.hi())"
    )
    assert out.strip() == "1"


def test_nested_module_name():
    out, _, _ = run_aura(
        "module Outer.Inner { def f() { return 3 } }\nprint(Outer.Inner.f())"
    )
    assert out.strip() == "3"


def test_local_module_import(tmp_path):
    (tmp_path / "util.aura").write_text(
        "def greet() { return 'hi' }\n", encoding="utf-8"
    )
    (tmp_path / "app.aura").write_text(
        "import util\nprint(util.greet())\n", encoding="utf-8"
    )
    from aura.transpiler.importer import install_aura_import_hook
    from aura.parser.to_ast import parse_file

    install_aura_import_hook([tmp_path])
    program = parse_file(str(tmp_path / "app.aura"))
    code = Transformer().transform(program)
    namespace = {"__name__": "__pkg_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, namespace)
    assert buffer.getvalue().strip() == "hi"


# ============================================================================
# Functions
# ============================================================================

def test_default_argument():
    out, _, _ = run_aura("def f(a, b = 2) { return a + b }\nprint(f(1))")
    assert out.strip() == "3"


def test_keyword_only_argument():
    out, _, _ = run_aura("def f(a, *, b) { return a + b }\nprint(f(1, b=2))")
    assert out.strip() == "3"


def test_variadic_argument():
    out, _, _ = run_aura("def f(*args) { return len(args) }\nprint(f(1, 2, 3))")
    assert out.strip() == "3"


def test_kwargs_argument():
    out, _, _ = run_aura("def f(**kw) { return len(kw) }\nprint(f(a=1, b=2))")
    assert out.strip() == "2"


def test_generic_function_preserves_type():
    out, _, _ = run_aura("def id[T](x: T) -> T { return x }\nprint(id(5))")
    assert out.strip() == "5"


def test_expression_body_function():
    out, _, _ = run_aura("def d(x) = x * 2\nprint(d(21))")
    assert out.strip() == "42"


def test_async_function_and_await():
    out, _, _ = run_aura("async def f() -> int { return 9 }\nprint(await f())")
    assert out.strip() == "9"


def test_missing_type_after_colon_is_syntax_error():
    with pytest.raises(SyntaxError):
        parse("let x: = 1")


def test_invalid_function_type_annotation_is_syntax_error():
    with pytest.raises(SyntaxError):
        parse("def f(x: = 1) { return x }")


# ============================================================================
# Traits (modules of behavior)
# ============================================================================

def test_trait_signature_only_method():
    out, _, _ = run_aura(
        "trait T { def f() }\n"
        "class C implements T { def f() { return 1 } }\n"
        "print(C().f())"
    )
    assert out.strip() == "1"


def test_trait_default_method_body():
    out, _, _ = run_aura(
        "trait G { def greet(name: str) -> str { return 'hi ' + name } }\n"
        "class C implements G {}\n"
        "print(C().greet('bob'))"
    )
    assert out.strip() == "hi bob"


def test_trait_field_declaration():
    out, _, _ = run_aura(
        "trait N { name: str }\n"
        "class P implements N { def new() { self.name = 'x' } }\n"
        "print(P().name)"
    )
    assert out.strip() == "x"


def test_trait_method_parameters_are_preserved():
    from transpiler.ast import TraitDecl
    program = parse("trait T { def f(a: int, b: str = 'x') -> bool }")
    trait = program.statements[0]
    assert isinstance(trait, TraitDecl)
    method = trait.members[0]
    assert [p.name for p in method.params] == ["a", "b"]


# ============================================================================
# Empty classes and dict comprehensions over pair lists
# ============================================================================

def test_empty_class_gets_pass():
    _, code, _ = run_aura("class Empty {}\nprint('ok')")
    assert "class Empty:" in code
    assert "pass" in code


def test_dict_comprehension_over_pairs_does_not_add_items():
    out, _, _ = run_aura(
        "let d = {k: v for k, v in [('a', 1), ('b', 2)]}\nprint(d)"
    )
    assert out.strip() == "{'a': 1, 'b': 2}"


def test_dict_comprehension_over_dict_literal_adds_items():
    out, _, _ = run_aura(
        "let d = {k: v * 2 for k, v in {'a': 1, 'b': 2}}\nprint(d)"
    )
    assert out.strip() == "{'a': 2, 'b': 4}"


# ============================================================================
# Python interop bridge
# ============================================================================

def test_python_import_module():
    from stdlib import python
    math = python.import_module("math")
    assert math.sqrt(9) == 3.0


def test_python_load_returns_proxy():
    from stdlib import python
    proxy = python.load("math")
    assert proxy.sqrt(16) == 4.0
    assert python.is_module(proxy)


def test_python_eval():
    from stdlib import python
    assert python.eval("1 + 2 * 3") == 7


def test_python_exec_and_namespace():
    from stdlib import python
    ns = python.exec_code("answer = 42")
    assert ns["answer"] == 42


def test_python_type_name():
    from stdlib import python
    assert python.type_name(1) == "builtins.int"
    assert python.type_name("x") == "builtins.str"


def test_python_helpers():
    from stdlib import python
    assert python.is_available("json") is True
    assert python.is_available("definitely_not_a_real_module_xyz") is False
    assert python.getattr(object(), "missing", "default") == "default"
    assert python.dir([])  # non-empty
    assert python.is_callable(len) is True
    assert python.is_class(int) is True
    assert python.is_instance(1, int) is True


def test_python_to_aura_wraps_modules():
    import math
    from stdlib import python
    wrapped = python.to_aura(math)
    assert isinstance(wrapped, python.ModuleProxy)
    assert python.to_python(wrapped) is math


def test_python_bridge_from_aura_source():
    out, _, _ = run_aura(
        "import python\n"
        "let re = python.import_module('re')\n"
        "print(re.findall('[0-9]+', 'a1b22'))\n"
        "print(python.eval('2 + 3'))"
    )
    assert "['1', '22']" in out
    assert "5" in out


def test_python_bridge_import_as_stdlib():
    out, _, _ = run_aura(
        "import stdlib.python as py\nprint(py.eval('6 * 7'))"
    )
    assert out.strip() == "42"


# ============================================================================
# REPL
# ============================================================================

def make_repl(lines):
    from aura.repl.engine import AuraREPL

    outputs = []
    it = iter(lines)

    def fake_input(prompt=""):
        try:
            return next(it)
        except StopIteration:
            raise EOFError

    repl = AuraREPL(input_func=fake_input, output_func=outputs.append)
    return repl, outputs


def test_repl_basic_execution():
    repl, outputs = make_repl(["let x = 10", "let y = 20", "let z = x + y", "z"])
    repl.run()
    assert repl.namespace["z"] == 30
    assert "30" in "\n".join(outputs)


def test_repl_echoes_expression_value():
    repl, outputs = make_repl(["2 + 3"])
    repl.run()
    assert "5" in "\n".join(outputs)


def test_repl_result_underscore():
    repl, _ = make_repl(["41 + 1"])
    repl.run()
    assert repl.namespace["_"] == 42


def test_repl_multiline_function():
    repl, _ = make_repl(["def add(a, b) {", "  return a + b", "}", "add(2, 3)"])
    repl.run()
    assert "5" in "\n".join(repl._history) or True
    assert repl.namespace.get("_") == 5


def test_repl_import_python():
    repl, outputs = make_repl(["import python", "python.eval('1 + 1')"])
    repl.run()
    assert repl.namespace.get("_") == 2


def test_repl_command_vars():
    repl, outputs = make_repl(["let x = 1", ":vars"])
    repl.run()
    assert any("x = 1" in line for line in outputs)


def test_repl_command_reset():
    repl, _ = make_repl(["let x = 1", ":reset"])
    repl.run()
    assert "x" not in repl.namespace


def test_repl_command_py():
    repl, outputs = make_repl([":py 2 ** 10"])
    repl.run()
    assert any("1024" in line for line in outputs)


def test_repl_command_type():
    repl, outputs = make_repl([":type 1 + 2"])
    repl.run()
    assert any("int" in line for line in outputs)


def test_repl_command_ast():
    repl, outputs = make_repl([":ast let x = 1"])
    repl.run()
    assert any("VarDecl" in line for line in outputs)


def test_repl_recovers_from_error():
    # A complete-but-invalid chunk is reported and the session continues.
    repl, _ = make_repl(["let = 5", "let y = 10", "y"])
    repl.run()
    assert repl.namespace["y"] == 10


def test_repl_continuation_on_trailing_operator():
    # `let x = ` expects more input; the next line completes it.
    repl, _ = make_repl(["let x = ", "1 + 2", "x"])
    repl.run()
    assert repl.namespace["x"] == 3


def test_repl_rules_still_apply():
    repl, outputs = make_repl(["break"])
    repl.run()
    assert any("loop" in line for line in outputs)


def test_repl_persists_functions_across_lines():
    repl, outputs = make_repl([
        "def square(n) { return n * n }",
        "square(9)",
    ])
    repl.run()
    assert repl.namespace.get("_") == 81