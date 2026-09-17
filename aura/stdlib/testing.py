"""Aura Standard Library - Testing module.

A tiny, dependency-free test framework for Aura programs. Tests register
themselves with :func:`test` (or :func:`case`) and are executed by
:func:`run_all`, which prints a report and returns the number of failures.

Usage from Aura::

    import stdlib.testing as t

    t.test("addition", () => {
      t.equal(1 + 1, 2)
    })

    t.test("split", () => {
      t.equal("a,b".split(","), ["a", "b"])
      t.contains("hello world", "world")
    })

    t.run_all()

A failing assertion raises ``AssertionError`` with a message that names the
expected and actual values, so ``aura test`` reports it as a failure. The
module is intentionally small: it favours clear messages and zero setup over
features like fixtures or parameterisation.
"""

import traceback as _traceback

__all__ = [
    'test',
    'case',
    'skip',
    'equal',
    'not_equal',
    'is_true',
    'is_false',
    'is_none',
    'is_not_none',
    'contains',
    'not_contains',
    'starts_with',
    'ends_with',
    'raises',
    'greater',
    'greater_or_equal',
    'less',
    'less_or_equal',
    'approx_equal',
    'in_range',
    'has_length',
    'is_empty',
    'fail',
    'run_all',
    'tests',
    'clear',
    'TestFailure',
]

# Registry of (name, function, skip_reason) in declaration order.
_REGISTRY: list = []


class TestFailure(AssertionError):
    """Raised by :func:`run_all` when one or more tests fail.

    Subclasses ``AssertionError`` so a program that lets it propagate exits
    non-zero under ``aura run`` (and therefore fails under ``aura test``),
    without callers needing any special handling.
    """

    def __init__(self, failed, total):
        self.failed = failed
        self.total = total
        super().__init__(f"{failed}/{total} tests failed")


def _repr(value):
    """Render a value for a failure message (Aura-friendly)."""
    if isinstance(value, str):
        return f'"{value}"'
    return repr(value)


def test(name, fn):
    """Register a test case. ``name`` is a label, ``fn`` a zero-arg callable."""
    if not isinstance(name, str) or not name:
        raise TypeError("test() expects a non-empty name")
    if not callable(fn):
        raise TypeError(f"test({name!r}) expects a callable body")
    _REGISTRY.append((name, fn, None))
    return fn


def case(name, fn):
    """Alias for :func:`test`, for readability in some code bases."""
    return test(name, fn)


def skip(name, reason="skipped"):
    """Register a test that is reported as skipped."""
    _REGISTRY.append((name, None, reason))


def clear():
    """Forget every registered test (useful between suites)."""
    _REGISTRY.clear()


def tests():
    """Return the registered test names in declaration order."""
    return [name for name, _fn, _reason in _REGISTRY]


# ============================================================================
# Assertions
# ============================================================================

def _fail(message):
    raise AssertionError(message)


def equal(actual, expected, label=None):
    """Assert ``actual == expected``."""
    if actual != expected:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected {_repr(expected)}, got {_repr(actual)}")
    return actual


def not_equal(actual, unexpected, label=None):
    """Assert ``actual != unexpected``."""
    if actual == unexpected:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}did not expect {_repr(unexpected)}")
    return actual


def is_true(value, label=None):
    """Assert ``value`` is truthy."""
    if not value:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected a truthy value, got {_repr(value)}")
    return value


def is_false(value, label=None):
    """Assert ``value`` is falsy."""
    if value:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected a falsy value, got {_repr(value)}")
    return value


def is_none(value, label=None):
    """Assert ``value is None``."""
    if value is not None:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected None, got {_repr(value)}")
    return value


def is_not_none(value, label=None):
    """Assert ``value is not None``."""
    if value is None:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected a value, got None")
    return value


def contains(haystack, needle, label=None):
    """Assert ``needle`` is in ``haystack`` (string, list, set or dict keys)."""
    if needle not in haystack:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}{_repr(haystack)} does not contain {_repr(needle)}")
    return haystack


def not_contains(haystack, needle, label=None):
    """Assert ``needle`` is not in ``haystack``."""
    if needle in haystack:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}{_repr(haystack)} unexpectedly contains {_repr(needle)}")
    return haystack


def starts_with(text, prefix, label=None):
    """Assert ``text`` starts with ``prefix``."""
    if not str(text).startswith(prefix):
        tag = f"{label}: " if label else ""
        _fail(f"{tag}{_repr(text)} does not start with {_repr(prefix)}")
    return text


def ends_with(text, suffix, label=None):
    """Assert ``text`` ends with ``suffix``."""
    if not str(text).endswith(suffix):
        tag = f"{label}: " if label else ""
        _fail(f"{tag}{_repr(text)} does not end with {_repr(suffix)}")
    return text


def raises(fn, error_type=None, label=None):
    """Assert calling ``fn`` raises. Returns the exception for inspection.

    ``error_type`` is an optional exception class (e.g. ``ValueError``) or its
    name as a string.
    """
    if not callable(fn):
        raise TypeError("raises() expects a callable")
    try:
        fn()
    except BaseException as exc:  # noqa: BLE001 - the point is to catch
        if error_type is None:
            return exc
        expected = error_type
        if isinstance(expected, str):
            if type(exc).__name__ != expected:
                prefix = f"{label}: " if label else ""
                _fail(f"{prefix}expected {expected}, got {type(exc).__name__}: {exc}")
        elif not isinstance(exc, expected):
            prefix = f"{label}: " if label else ""
            _fail(f"{prefix}expected {expected.__name__}, got "
                  f"{type(exc).__name__}: {exc}")
        return exc
    prefix = f"{label}: " if label else ""
    _fail(f"{prefix}expected an exception, but none was raised")


def greater(actual, threshold, label=None):
    """Assert ``actual > threshold``."""
    if not actual > threshold:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected > {_repr(threshold)}, got {_repr(actual)}")
    return actual


def greater_or_equal(actual, threshold, label=None):
    """Assert ``actual >= threshold``."""
    if not actual >= threshold:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected >= {_repr(threshold)}, got {_repr(actual)}")
    return actual


def less(actual, threshold, label=None):
    """Assert ``actual < threshold``."""
    if not actual < threshold:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected < {_repr(threshold)}, got {_repr(actual)}")
    return actual


def less_or_equal(actual, threshold, label=None):
    """Assert ``actual <= threshold``."""
    if not actual <= threshold:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected <= {_repr(threshold)}, got {_repr(actual)}")
    return actual


def approx_equal(actual, expected, tolerance=1e-9, label=None):
    """Assert ``abs(actual - expected) <= tolerance`` (for floats)."""
    if abs(actual - expected) > tolerance:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected {_repr(expected)} ± {tolerance}, "
              f"got {_repr(actual)}")
    return actual


def in_range(value, low, high, label=None):
    """Assert ``low <= value <= high`` (inclusive)."""
    if not low <= value <= high:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected {low}..{high}, got {_repr(value)}")
    return value


def has_length(value, expected, label=None):
    """Assert ``len(value) == expected``."""
    actual = len(value)
    if actual != expected:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected length {expected}, got {actual}")
    return value


def is_empty(value, label=None):
    """Assert ``len(value) == 0``."""
    if len(value) != 0:
        prefix = f"{label}: " if label else ""
        _fail(f"{prefix}expected an empty value, got length {len(value)}")
    return value


def fail(message="test failed"):
    """Fail unconditionally with ``message``."""
    _fail(str(message))


# ============================================================================
# Runner
# ============================================================================

def run_all(verbose=True, stream=None, raise_on_failure=True):
    """Run every registered test and print a summary.

    Returns the number of failures (0 means success). By default it also raises
    :class:`TestFailure` when any test failed, so an Aura program that simply
    calls ``t.run_all()`` exits non-zero and ``aura test`` marks the file as
    failed. Pass ``raise_on_failure=False`` to inspect the count instead.
    """
    import sys

    out = stream if stream is not None else sys.stdout
    passed = 0
    failed = 0
    skipped = 0
    failures = []

    for name, fn, reason in _REGISTRY:
        if fn is None:
            skipped += 1
            if verbose:
                print(f"  SKIP {name} ({reason})", file=out)
            continue
        try:
            fn()
        except BaseException as exc:  # noqa: BLE001 - report, do not abort
            failed += 1
            detail = f"{type(exc).__name__}: {exc}"
            failures.append((name, detail, exc))
            if verbose:
                print(f"  FAIL {name}: {detail}", file=out)
        else:
            passed += 1
            if verbose:
                print(f"  OK   {name}", file=out)

    total = passed + failed + skipped
    print(f"\n{passed}/{total} passed, {failed} failed, {skipped} skipped", file=out)
    if failures and verbose:
        print("\nFailures:", file=out)
        for name, detail, exc in failures[:20]:
            print(f"  {name}: {detail}", file=out)
            tb = getattr(exc, '__traceback__', None)
            if tb is not None:
                frames = _traceback.extract_tb(tb)
                # Point at the user's assertion, not this module's helpers: the
                # deepest frame that is not the testing module itself.
                _this_file = __file__
                for frame in reversed(frames):
                    if frame.filename == _this_file:
                        continue
                    if frame.filename:
                        print(f"      at {frame.filename}:{frame.lineno}", file=out)
                        break
    if failed and raise_on_failure:
        raise TestFailure(failed, total)
    return failed
