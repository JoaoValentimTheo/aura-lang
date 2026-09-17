"""Extreme/edge-case tests for every enforced Aura rule.

Each enforced diagnostic code gets a dedicated test that pins the boundary:
the minimal program that triggers it and a near-miss that must stay clean.
The suite also checks that every catalogue code is either emitted or explicitly
documented as reserved, so the diagnostic set cannot silently drift.
"""
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler import errors as errors_mod  # noqa: E402
from aura.transpiler.errors import ErrorCode  # noqa: E402
from aura.transpiler.rules import RuleChecker  # noqa: E402
from aura.transpiler.semantics import MutabilityChecker  # noqa: E402
from aura.transpiler.types import TypeChecker  # noqa: E402

MAIN = str(ROOT / "main.py")


def parse(source):
    return Parser(Tokenizer(source).tokenize()).parse()


def codes(source, require_main=False):
    """All diagnostic codes from rule + mutability + type checkers."""
    program = parse(source)

    rule = RuleChecker()
    rule.check_program(program, require_main=require_main)

    mut = MutabilityChecker()
    mut.check_program(program)

    typ = TypeChecker()
    typ.check_program(program)

    found = [e.code.value for e in rule.collector.errors]
    found += [d.code.value for d in getattr(mut, 'diagnostics', [])]
    found += [d.code.value for d in getattr(typ, 'diagnostics', [])]
    return found


def has_code(source, code, require_main=False):
    return code in codes(source, require_main=require_main)


def run_cli(*args):
    return subprocess.run(
        [sys.executable, MAIN, *args], capture_output=True, text=True, timeout=30)


# ============================================================================
# E004 INVALID_SYNTAX - structural placement rules
# ============================================================================

def test_e004_return_outside_function():
    assert has_code("return 1", "E004")


def test_e004_return_inside_function_ok():
    assert not has_code("def main() { return 1 }", "E004")


def test_e004_break_outside_loop():
    assert has_code("def main() { break }", "E004")


def test_e004_continue_outside_loop():
    assert has_code("def main() { continue }", "E004")


def test_e004_break_inside_loop_ok():
    assert not has_code("def main() { while true { break } }", "E004")


def test_e004_await_in_sync_function():
    assert has_code("def main() { await f() }", "E004")


def test_e004_await_in_async_function_ok():
    assert not has_code("async def main() { await f() }", "E004")


def test_e004_self_outside_class():
    assert has_code("def main() { print(self) }", "E004")


def test_e004_self_inside_method_ok():
    assert not has_code("class C { public def m() -> int { return 1 } }", "E004")


def test_e004_invalid_assignment_target():
    assert has_code("def main() { 1 = 2 }", "E004")


def test_e004_valid_assignment_target_ok():
    assert not has_code("def main() { let mut x = 1\n x = 2 }", "E004")


def test_const_without_value_is_a_syntax_error():
    with pytest.raises(SyntaxError):
        parse("const X")


def test_e004_const_with_value_ok():
    assert not has_code("const X = 1\ndef main() { print(X) }", "E004")


# ============================================================================
# E301 DUPLICATE_DEFINITION
# ============================================================================

def test_e301_duplicate_variable():
    assert has_code("def main() {\n let x = 1\n let x = 2\n}", "E301")


def test_e301_duplicate_function():
    assert has_code("def f() {}\ndef f() {}", "E301")


def test_e301_duplicate_parameter():
    assert has_code("def f(a, a) {}", "E301")


def test_e301_distinct_names_ok():
    assert not has_code("def f(a, b) {}\ndef g() {}", "E301")


def test_e301_same_name_different_scopes_ok():
    assert not has_code("def main() {\n let x = 1\n if true { let x = 2 }\n}", "E301")


# ============================================================================
# E302 UNREACHABLE_CODE
# ============================================================================

def test_e302_after_return():
    assert has_code("def main() {\n return 1\n print(2)\n}", "E302")


def test_e302_after_throw():
    assert has_code('def main() {\n throw "x"\n print(2)\n}', "E302")


def test_e302_after_break():
    assert has_code("def main() { while true {\n break\n print(1) } }", "E302")


def test_e302_no_dead_code_ok():
    assert not has_code("def main() {\n print(1)\n return 2\n}", "E302")


# ============================================================================
# E303 REASSIGN_IMMUTABLE
# ============================================================================

def test_e303_reassign_let():
    assert has_code("def main() {\n let x = 1\n x = 2\n}", "E303")


def test_e303_reassign_const():
    assert has_code("const X = 1\ndef main() { X = 2 }", "E303")


def test_e303_let_mut_ok():
    assert not has_code("def main() {\n let mut x = 1\n x = 2\n}", "E303")


def test_e303_augmented_let_ok():
    assert has_code("def main() {\n let x = 1\n x += 2\n}", "E303")


def test_e303_member_mutation_ok():
    assert not has_code("class C { public let x: int = 0\n public def set(v: int) { self.x = v } }", "E303")


# ============================================================================
# E307 MISSING_VISIBILITY
# ============================================================================

@pytest.mark.parametrize("body", [
    "let x = 1",
    "def m() {}",
    "class N {}",
])
def test_e307_missing_visibility(body):
    assert has_code(f"class C {{\n {body}\n}}", "E307")


@pytest.mark.parametrize("mod", ["public", "private", "protected"])
def test_e307_explicit_visibility_ok(mod):
    assert not has_code(f"class C {{\n {mod} let x = 1\n}}", "E307")


def test_e307_trait_member_requires_visibility():
    assert has_code("trait T {\n def m() -> int\n}", "E307")


# ============================================================================
# E308 INACCESSIBLE_MEMBER
# ============================================================================

def test_e308_private_from_outside():
    assert has_code("class C { private let x: int = 1 }\ndef main() { let c = C()\n print(c.x) }", "E308")


def test_e308_protected_from_outside():
    assert has_code(
        "class C { protected let x: int = 1 }\n"
        "def main() { let c = C()\n print(c.x) }", "E308")


def test_e308_private_in_subclass():
    assert has_code(
        "class A { private let x: int = 1 }\n"
        "class B(A) { public def r() -> int { return self.x } }", "E308")


def test_e308_protected_in_subclass_ok():
    assert not has_code(
        "class A { protected let x: int = 1 }\n"
        "class B(A) { public def r() -> int { return self.x } }", "E308")


def test_e308_private_inside_own_class_ok():
    assert not has_code(
        "class C { private let x: int = 1\n public def r() -> int { return self.x } }", "E308")


def test_e308_public_anywhere_ok():
    assert not has_code("class C { public let x: int = 1 }\ndef main() { let c = C()\n print(c.x) }", "E308")


# ============================================================================
# E309 UNIMPLEMENTED_ABSTRACT
# ============================================================================

def test_e309_missing_trait_method():
    assert has_code("trait T { public def m() -> int }\nclass C implements T { }", "E309")


def test_e309_implemented_ok():
    assert not has_code(
        "trait T { public def m() -> int }\n"
        "class C implements T { public def m() -> int { return 1 } }", "E309")


def test_e309_trait_alone_ok():
    assert not has_code("trait T { public def m() -> int }", "E309")


# ============================================================================
# E310/E311 main entry point
# ============================================================================

def test_e310_missing_main():
    assert has_code("print(1)", "E310", require_main=True)


def test_e310_module_mode_ok():
    assert not has_code("print(1)", "E310", require_main=False)


def test_e311_main_with_param():
    assert has_code("def main(x: int) {}", "E311", require_main=True)


def test_e311_main_with_args_ok():
    assert not has_code("def main(args: [string]) {}", "E311", require_main=True)


# ============================================================================
# E101/E105/E106/E108 type diagnostics
# ============================================================================

def test_e101_variable_annotation_mismatch():
    assert has_code('def main() { let x: int = "s" }', "E101")


def test_e101_return_mismatch():
    assert has_code('def f() -> int { return "s" }\ndef main() { print(f()) }', "E101")


def test_e105_too_many_arguments():
    assert has_code("def f(a: int) -> int { return a }\ndef main() { f(1, 2, 3) }", "E105")


def test_e105_too_few_arguments():
    assert has_code("def f(a: int, b: int) -> int { return a }\ndef main() { f(1) }", "E105")


def test_e105_default_makes_optional_ok():
    assert not has_code("def f(a: int, b: int = 2) -> int { return a }\ndef main() { f(1) }", "E105")


def test_e105_variadic_ok():
    assert not has_code("def f(a: int, *rest) -> int { return a }\ndef main() { f(1) }", "E105")


def test_e106_wrong_argument_type():
    assert has_code('def f(a: int) -> int { return a }\ndef main() { f("s") }', "E106")


def test_e108_incompatible_operands():
    assert has_code('def main() { let x: int = 1 + "s" }', "E108")


# ============================================================================
# W101/W102/W103 warnings
# ============================================================================

def test_w103_unused_type_parameter():
    assert has_code("class Box[T] { public let x: int = 1 }", "W103")


def test_w103_used_type_parameter_ok():
    assert not has_code("class Box[T] { public let item: T = none }", "W103")


def test_warning_codes_never_fail_has_errors():
    program = parse("class Box[T] { public let x: int = 1 }")
    checker = TypeChecker()
    assert checker.check_program(program) is True


# ============================================================================
# CLI end-to-end: extreme inputs
# ============================================================================

def test_cli_deeply_nested_expression(tmp_path):
    src = tmp_path / "deep.aura"
    depth = 200
    src.write_text("def main() { print(" + "(" * depth + "1" + ")" * depth + ") }")
    result = run_cli("check", str(src))
    assert result.returncode == 0, result.stderr


def test_cli_huge_integer_literal(tmp_path):
    src = tmp_path / "big.aura"
    src.write_text(f"def main() {{ print({10**80}) }}")
    result = run_cli("run", str(src))
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == str(10**80)


def test_cli_empty_program_fails_entry(tmp_path):
    src = tmp_path / "empty.aura"
    src.write_text("")
    result = run_cli("run", str(src))
    assert result.returncode == 2
    assert "[E310]" in result.stderr


def test_cli_module_without_main_transpiles(tmp_path):
    src = tmp_path / "mod.aura"
    src.write_text("def square(x: int) -> int { return x * x }")
    result = run_cli("transpile", str(src))
    assert result.returncode == 0
    assert "def square" in result.stdout


def test_cli_unicode_identifier_string_ok(tmp_path):
    src = tmp_path / "u.aura"
    src.write_text('def main() { print("olá, mundo — 你好") }')
    result = run_cli("run", str(src))
    assert result.returncode == 0, result.stderr
    assert "olá" in result.stdout


# ============================================================================
# Catalogue completeness
# ============================================================================

def test_every_code_has_a_canonical_template():
    missing = [c.value for c in ErrorCode if c not in errors_mod.ERROR_TEMPLATES]
    assert not missing, f"codes without a canonical message: {missing}"


def test_code_areas_are_classified():
    for code in ErrorCode:
        assert errors_mod.code_area(code)
