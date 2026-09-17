"""Regression tests for stdlib bugs fixed in the 0.1.0a10 hardening pass."""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))


# ---------------------------------------------------------------------------
# itertools.repeat
# ---------------------------------------------------------------------------

def test_itertools_repeat_without_times_is_infinite():
    from aura.stdlib import itertools as it
    seq = it.repeat("x")
    assert [next(seq) for _ in range(3)] == ["x", "x", "x"]


def test_itertools_repeat_with_times_is_bounded():
    from aura.stdlib import itertools as it
    assert list(it.repeat("y", 3)) == ["y", "y", "y"]
    assert list(it.repeat("z", 0)) == []


# ---------------------------------------------------------------------------
# asyncio.wait / as_completed with raw coroutines
# ---------------------------------------------------------------------------

def test_asyncio_wait_accepts_raw_coroutines():
    import asyncio

    from aura.stdlib import asyncio as aio

    async def work(n):
        await asyncio.sleep(0.001)
        return n

    async def main():
        done, pending = await aio.wait(work(1), work(2))
        return sorted(t.result() for t in done), len(pending)

    results, pending = asyncio.run(main())
    assert results == [1, 2]
    assert pending == 0


def test_asyncio_as_completed_accepts_raw_coroutines():
    import asyncio

    from aura.stdlib import asyncio as aio

    async def work(n):
        await asyncio.sleep(0.001)
        return n

    async def main():
        got = []
        for fut in aio.as_completed([work(3), work(4)]):
            got.append(await fut)
        return sorted(got)

    assert asyncio.run(main()) == [3, 4]


# ---------------------------------------------------------------------------
# hkdf_sha256 length bounds
# ---------------------------------------------------------------------------

def test_hkdf_max_length_is_accepted():
    from aura.stdlib import crypto
    max_len = 255 * 32
    assert len(crypto.hkdf_sha256("ikm", max_len)) == max_len


def test_hkdf_over_max_length_is_rejected():
    from aura.stdlib import crypto
    with pytest.raises(ValueError):
        crypto.hkdf_sha256("ikm", 255 * 32 + 1)


def test_hkdf_negative_length_is_rejected():
    from aura.stdlib import crypto
    with pytest.raises(ValueError):
        crypto.hkdf_sha256("ikm", -1)


def test_hkdf_is_deterministic():
    from aura.stdlib import crypto
    a = crypto.hkdf_sha256("ikm", 64, salt=b"s", info=b"i")
    b = crypto.hkdf_sha256("ikm", 64, salt=b"s", info=b"i")
    assert a == b and len(a) == 64


# ---------------------------------------------------------------------------
# stdlib.join shadowing resolved
# ---------------------------------------------------------------------------

def test_string_join_is_top_level_join():
    from aura import stdlib
    assert stdlib.join(["a", "b", "c"], "-") == "a-b-c"


def test_io_join_is_available_as_path_join():
    from aura import stdlib
    assert stdlib.path_join("a", "b", "c") == "a/b/c"


def test_path_join_matches_os_path_join():
    import os

    from aura import stdlib
    assert stdlib.path_join("x", "y") == os.path.join("x", "y")


# ---------------------------------------------------------------------------
# Newly exported constants
# ---------------------------------------------------------------------------

def test_inf_nan_exported():
    import math

    from aura import stdlib
    assert math.inf == stdlib.INF
    assert math.isnan(stdlib.NAN)


# ---------------------------------------------------------------------------
# The removed aliases are gone (kept clean)
# ---------------------------------------------------------------------------

def test_removed_redundant_aliases_are_absent():
    from aura.stdlib import collections
    for name in ("map_list", "filter_list", "reduce_list", "find_in_list",
                 "aura_dict", "list_from", "dict_from", "set_from"):
        assert not hasattr(collections, name), f"{name} should be removed"


def test_crypto_backend_is_exported():
    from aura import stdlib
    assert "crypto_backend" in stdlib.__all__
    assert hasattr(stdlib, "crypto_backend")
