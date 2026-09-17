"""Tests for ``aura.stdlib.testing`` and its integration with ``aura test``.

The module is Aura's dependency-free assertion framework. These tests pin every
matcher's pass/fail boundary, the runner's reporting and exit behaviour, and
the CLI's ability to summarise a failing test file.
"""
import io
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.stdlib import testing as T  # noqa: E402


@pytest.fixture(autouse=True)
def _clean_registry():
    """Each test starts and ends with an empty registry."""
    T.clear()
    yield
    T.clear()


# ============================================================================
# Registration
# ============================================================================

class TestRegistration:
    def test_test_registers_in_order(self):
        T.test("a", lambda: None)
        T.test("b", lambda: None)
        assert T.tests() == ["a", "b"]

    def test_case_is_an_alias(self):
        T.case("aliased", lambda: None)
        assert T.tests() == ["aliased"]

    def test_test_rejects_empty_name(self):
        with pytest.raises(TypeError):
            T.test("", lambda: None)

    def test_test_rejects_non_callable(self):
        with pytest.raises(TypeError):
            T.test("bad", 42)

    def test_skip_is_recorded(self):
        T.skip("later", "not yet")
        assert T.tests() == ["later"]

    def test_clear_empties_registry(self):
        T.test("x", lambda: None)
        T.clear()
        assert T.tests() == []


# ============================================================================
# Matchers: success and failure boundaries
# ============================================================================

class TestMatchers:
    def test_equal(self):
        assert T.equal(2, 2) == 2
        with pytest.raises(AssertionError) as exc:
            T.equal(2, 3)
        assert "expected 3, got 2" in str(exc.value)

    def test_equal_with_label(self):
        with pytest.raises(AssertionError) as exc:
            T.equal(1, 2, "count")
        assert str(exc.value).startswith("count: ")

    def test_not_equal(self):
        assert T.not_equal(1, 2) == 1
        with pytest.raises(AssertionError):
            T.not_equal(1, 1)

    def test_is_true_and_false(self):
        assert T.is_true(1) == 1
        assert T.is_false(0) == 0
        with pytest.raises(AssertionError):
            T.is_true(0)
        with pytest.raises(AssertionError):
            T.is_false(1)

    def test_is_none_variants(self):
        assert T.is_none(None) is None
        assert T.is_not_none(5) == 5
        with pytest.raises(AssertionError):
            T.is_none(5)
        with pytest.raises(AssertionError):
            T.is_not_none(None)

    def test_contains_on_string_list_and_dict(self):
        assert T.contains("hello", "ell") == "hello"
        assert T.contains([1, 2], 2) == [1, 2]
        assert T.contains({"k": 1}, "k") == {"k": 1}
        with pytest.raises(AssertionError):
            T.contains("hello", "z")

    def test_not_contains(self):
        assert T.not_contains("hello", "z") == "hello"
        with pytest.raises(AssertionError):
            T.not_contains("hello", "ell")

    def test_starts_and_ends_with(self):
        assert T.starts_with("hello", "he") == "hello"
        assert T.ends_with("hello", "lo") == "hello"
        with pytest.raises(AssertionError):
            T.starts_with("hello", "x")
        with pytest.raises(AssertionError):
            T.ends_with("hello", "x")

    def test_raises_any(self):
        def boom():
            raise ValueError("nope")
        exc = T.raises(boom)
        assert isinstance(exc, ValueError)

    def test_raises_specific_class(self):
        def boom():
            raise ValueError("nope")
        assert isinstance(T.raises(boom, ValueError), ValueError)
        with pytest.raises(AssertionError) as exc:
            T.raises(boom, TypeError)
        assert "expected TypeError" in str(exc.value)

    def test_raises_by_name(self):
        def boom():
            raise KeyError("k")
        assert T.raises(boom, "KeyError") is not None
        with pytest.raises(AssertionError):
            T.raises(boom, "ValueError")

    def test_raises_when_nothing_raised(self):
        with pytest.raises(AssertionError) as exc:
            T.raises(lambda: None)
        assert "none was raised" in str(exc.value)

    def test_raises_rejects_non_callable(self):
        with pytest.raises(TypeError):
            T.raises(42)

    def test_ordering_matchers(self):
        assert T.greater(3, 2) == 3
        assert T.greater_or_equal(2, 2) == 2
        assert T.less(1, 2) == 1
        assert T.less_or_equal(2, 2) == 2
        with pytest.raises(AssertionError):
            T.greater(1, 2)
        with pytest.raises(AssertionError):
            T.less(3, 2)

    def test_approx_equal(self):
        assert T.approx_equal(0.1 + 0.2, 0.3) <= 0.3001
        with pytest.raises(AssertionError):
            T.approx_equal(0.1, 0.2, tolerance=1e-9)

    def test_in_range_inclusive(self):
        assert T.in_range(0, 0, 10) == 0
        assert T.in_range(10, 0, 10) == 10
        with pytest.raises(AssertionError):
            T.in_range(11, 0, 10)

    def test_has_length_and_is_empty(self):
        assert T.has_length([1, 2], 2) == [1, 2]
        assert T.is_empty([]) == []
        with pytest.raises(AssertionError):
            T.has_length([1], 2)
        with pytest.raises(AssertionError):
            T.is_empty([1])

    def test_fail_unconditionally(self):
        with pytest.raises(AssertionError) as exc:
            T.fail("gave up")
        assert "gave up" in str(exc.value)


# ============================================================================
# Runner
# ============================================================================

class TestRunner:
    def test_all_pass_returns_zero(self):
        T.test("one", lambda: T.equal(1, 1))
        T.test("two", lambda: T.is_true(True))
        out = io.StringIO()
        assert T.run_all(stream=out) == 0
        assert "2/2 passed" in out.getvalue()

    def test_failure_count_and_raises(self):
        T.test("ok", lambda: None)
        T.test("bad", lambda: T.equal(1, 2))
        out = io.StringIO()
        with pytest.raises(T.TestFailure) as exc:
            T.run_all(stream=out)
        assert exc.value.failed == 1
        assert "1/2 passed, 1 failed" in out.getvalue()

    def test_raise_on_failure_can_be_disabled(self):
        T.test("bad", lambda: T.fail("x"))
        out = io.StringIO()
        assert T.run_all(stream=out, raise_on_failure=False) == 1

    def test_skipped_reported_and_not_failed(self):
        T.test("ok", lambda: None)
        T.skip("todo", "later")
        out = io.StringIO()
        assert T.run_all(stream=out) == 0
        text = out.getvalue()
        assert "SKIP todo (later)" in text
        assert "0 failed, 1 skipped" in text

    def test_failure_names_assertion_source(self, tmp_path):
        # The reported location must be the *test body*, not this module.
        src = tmp_path / "body.py"
        src.write_text("def body():\n    raise AssertionError('boom')\n")
        namespace = {}
        exec(compile(src.read_text(), str(src), "exec"), namespace)
        T.test("located", namespace["body"])
        out = io.StringIO()
        with pytest.raises(T.TestFailure):
            T.run_all(stream=out)
        assert str(src) in out.getvalue()

    def test_verbose_false_suppresses_per_test_lines(self):
        T.test("ok", lambda: None)
        out = io.StringIO()
        T.run_all(verbose=False, stream=out)
        assert "OK" not in out.getvalue()
        assert "1/1 passed" in out.getvalue()

    def test_empty_suite_passes(self):
        out = io.StringIO()
        assert T.run_all(stream=out) == 0
        assert "0/0 passed" in out.getvalue()


# ============================================================================
# Integration: Aura program + `aura test`
# ============================================================================

class TestAuraIntegration:
    def test_run_all_from_aura_passes(self, tmp_path):
        from aura_test_helpers import run_aura

        out = run_aura(
            'import stdlib.testing as t\n'
            't.test("add", () => { t.equal(1 + 1, 2) })\n'
            'def main() { t.run_all() }\n')
        assert '1/1 passed' in out

    def test_failing_suite_exits_non_zero(self, tmp_path):
        from aura.cli import cmd_test

        src = tmp_path / "suite.aura"
        src.write_text(
            'import stdlib.testing as t\n'
            't.test("bad", () => { t.equal(1, 2) })\n'
            'def main() { t.run_all() }\n')
        assert cmd_test(str(src)) == 1

    def test_passing_suite_exits_zero(self, tmp_path):
        from aura.cli import cmd_test

        src = tmp_path / "suite.aura"
        src.write_text(
            'import stdlib.testing as t\n'
            't.test("ok", () => { t.equal(2, 2) })\n'
            'def main() { t.run_all() }\n')
        assert cmd_test(str(src)) == 0

    def test_cmd_test_summarises_failure(self, tmp_path, capsys):
        from aura.cli import cmd_test

        src = tmp_path / "suite.aura"
        src.write_text(
            'import stdlib.testing as t\n'
            't.test("bad", () => { t.equal(1, 2) })\n'
            'def main() { t.run_all() }\n')
        cmd_test(str(src))
        out = capsys.readouterr().out
        assert "1 failed" in out
        assert "FAIL bad" in out
