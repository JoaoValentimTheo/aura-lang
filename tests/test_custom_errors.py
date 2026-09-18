"""Tests for Aura's custom exception hierarchy.

Aura spells the exception root ``Error`` (aliased to Python's ``Exception``),
so a program can define its own error types with either supported inheritance
syntax — ``class MyError extends Error`` or ``class MyError extends Error`` — and
``throw``/``catch`` them just like the builtins. ``super(args)`` maps to
``super().__init__(args)`` for message-passing constructors.
"""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura_test_helpers import run_aura, transpile  # noqa: E402

# ============================================================================
# Syntax: `extends` is the only inheritance spelling
# ============================================================================

class TestErrorSyntax:
    def test_extends_error_parses(self):
        code = transpile(
            'class MyError extends Error {\n'
            '  public def new(msg: str) { super(msg) }\n'
            '}\n'
        )
        assert 'Error = Exception' in code
        # The generated Python uses the parenthesised form.
        assert 'class MyError(Error):' in code

    def test_parenthesised_base_is_rejected(self):
        # Inheritance is spelled with `extends` only. `(` after a class name
        # introduces header fields, so a bare name there is an error.
        with pytest.raises(SyntaxError):
            transpile('class MyError(Error) { }\n')

    def test_implements_is_rejected(self):
        with pytest.raises(SyntaxError):
            transpile('class C implements T { }\n')

    def test_super_maps_to_init(self):
        code = transpile(
            'class E extends Error {\n'
            '  public def new(msg: str) { super(msg) }\n'
            '}\n'
        )
        assert 'super().__init__(msg)' in code
        assert 'super(msg)' not in code

    def test_error_prelude_only_when_used(self):
        assert 'Error = Exception' not in transpile('def main() { print(1) }\n')
        assert 'Error = Exception' in transpile('class E extends Error { }\n')

    def test_user_class_inheritance_unaffected(self):
        code = transpile(
            'class Base { public def v() -> int { return 1 } }\n'
            'class Child extends Base { }\n'
        )
        assert 'Error = Exception' not in code
        assert 'class Child(Base):' in code


# ============================================================================
# Runtime behaviour
# ============================================================================

class TestErrorRuntime:
    def test_throw_and_catch_custom_error(self):
        out = run_aura(
            'class MyError extends Error {\n'
            '  public def new(msg: str) { super(msg) }\n'
            '}\n'
            'def main() {\n'
            '  try { throw MyError("boom") }\n'
            '  catch e { print("caught: " + str(e)) }\n'
            '}\n'
        )
        assert out == 'caught: boom\n'

    def test_typed_catch_matches_subclass(self):
        out = run_aura(
            'class AppError extends Error {\n'
            '  public def new(msg: str) { super(msg) }\n'
            '}\n'
            'class NotFoundError extends AppError {\n'
            '  public def new(msg: str) { super(msg) }\n'
            '}\n'
            'def main() {\n'
            '  try { throw NotFoundError("missing") }\n'
            '  catch NotFoundError { print("nf") }\n'
            '  catch AppError { print("app") }\n'
            '}\n'
        )
        assert out == 'nf\n'

    def test_typed_catch_parent_catches_child(self):
        out = run_aura(
            'class AppError extends Error {\n'
            '  public def new(msg: str) { super(msg) }\n'
            '}\n'
            'class NotFoundError extends AppError {\n'
            '  public def new(msg: str) { super(msg) }\n'
            '}\n'
            'def main() {\n'
            '  try { throw NotFoundError("missing") }\n'
            '  catch AppError { print("parent") }\n'
            '}\n'
        )
        assert out == 'parent\n'

    def test_catch_error_root_catches_everything(self):
        out = run_aura(
            'class MyError extends Error { }\n'
            'def main() {\n'
            '  try { throw MyError() } catch Error { print("root") }\n'
            '}\n'
        )
        assert out == 'root\n'

    def test_custom_error_message_survives(self):
        out = run_aura(
            'class ValidationError extends Error {\n'
            '  public def new(msg: str) { super(msg) }\n'
            '}\n'
            'def check(n: int) {\n'
            '  if n < 0 { throw ValidationError("negative") }\n'
            '  return n\n'
            '}\n'
            'def main() {\n'
            '  try { check(-1) }\n'
            '  catch ValidationError { print("rejected") }\n'
            '}\n'
        )
        assert out == 'rejected\n'

    def test_custom_error_is_real_exception(self):
        out = run_aura(
            'import python\n'
            'class MyError extends Error {\n'
            '  public def new(msg: str) { super(msg) }\n'
            '}\n'
            'def main() {\n'
            '  let e = MyError("x")\n'
            '  print(python.is_instance(e, Error))\n'
            '}\n'
        )
        assert out == 'True\n'

    def test_finally_runs_on_custom_error(self):
        out = run_aura(
            'class MyError extends Error { }\n'
            'def main() {\n'
            '  try { throw MyError() }\n'
            '  catch MyError { print("caught") }\n'
            '  finally { print("cleanup") }\n'
            '}\n'
        )
        assert out == 'caught\ncleanup\n'


# ============================================================================
# Error messages and diagnostics
# ============================================================================

class TestErrorDiagnostics:
    def test_extends_unknown_base_still_transpiles(self):
        # A non-Error base must not trigger the Error prelude.
        code = transpile('class C extends Base { }\n')
        assert 'Error = Exception' not in code

    def test_uncaught_custom_error_reports_type(self, tmp_path):
        import subprocess
        src = tmp_path / 'boom.aura'
        src.write_text(
            'class MyError extends Error {\n'
            '  public def new(msg: str) { super(msg) }\n'
            '}\n'
            'def main() { throw MyError("fatal") }\n'
        )
        result = subprocess.run(
            [sys.executable, str(ROOT / 'main.py'), 'run', str(src)],
            capture_output=True, text=True,
        )
        assert result.returncode == 1
        assert 'MyError' in result.stderr
