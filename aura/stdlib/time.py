"""Aura Standard Library - Time module."""

import datetime as _dt
import time as _time


def now():
    """Current timestamp (seconds since epoch)."""
    return _time.time()


def now_ms():
    """Current timestamp in milliseconds."""
    return _time.time() * 1000


def sleep(seconds):
    """Sleep for given seconds."""
    _time.sleep(seconds)


def clock():
    """CPU time (seconds) used by the process."""
    return _time.process_time()


def monotonic():
    """Monotonic clock (seconds), not affected by system clock changes."""
    return _time.monotonic()


def perf_counter():
    """High-resolution performance counter."""
    return _time.perf_counter()


def strftime(fmt):
    """Current time formatted by strftime format string."""
    return _dt.datetime.now().strftime(fmt)


def parse(date_string, fmt="%Y-%m-%d %H:%M:%S"):
    """Parse a date string into a datetime object."""
    return _dt.datetime.strptime(date_string, fmt)


def iso():
    """Current time as ISO 8601 string."""
    return _dt.datetime.now().isoformat()


def timestamp(dt_obj=None):
    """Convert datetime to timestamp. Default: now."""
    if dt_obj is None:
        dt_obj = _dt.datetime.now()
    return dt_obj.timestamp()


def elapsed(start):
    """Elapsed time since start timestamp."""
    return _time.time() - start
