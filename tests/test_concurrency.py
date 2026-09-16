"""Tests for concurrency: the threading/asyncio stdlib and global-scope codegen.

Also covers the `global` emission for functions that assign to module-level
bindings, which is what makes Aura threading programs (shared state guarded by
locks) actually work.
"""
import contextlib
import io
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Tokenizer, Parser  # noqa: E402
from transpiler.transformer import Transformer  # noqa: E402


def transpile(source):
    return Transformer().transform(Parser(Tokenizer(source).tokenize()).parse())


def run_aura(source):
    code = transpile(source)
    namespace = {"__name__": "__aura_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, "<test>", "exec"), namespace)
    return buffer.getvalue(), namespace


def run_aura_async(source):
    """Run an Aura program with top-level await, like the CLI does."""
    import asyncio

    code = transpile(source)
    indented = "\n".join(
        ("    " + line if line.strip() else line) for line in code.split("\n")
    )
    wrapper = "async def _aura_main():\n" + indented + "\n"
    namespace = {"__name__": "__aura_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(wrapper, "<test>", "exec"), namespace)
        asyncio.run(namespace["_aura_main"]())
    return buffer.getvalue(), namespace


# ============================================================================
# global-scope codegen
# ============================================================================

def test_function_assigning_module_binding_gets_global():
    code = transpile("let mut total = 0\ndef inc() { total += 1 }")
    assert "global total" in code


def test_function_not_assigning_module_binding_has_no_global():
    code = transpile("let mut total = 0\ndef read() { return total }")
    assert "global " not in code


def test_module_assignment_works_at_runtime():
    out, ns = run_aura(
        "let mut total = 0\n"
        "def inc() { total += 1 }\n"
        "inc()\ninc()\n"
        "print(total)"
    )
    assert out == "2\n"
    assert ns["total"] == 2


def test_local_variable_assignment_is_not_global():
    code = transpile("def f() { let mut x = 1\nx = 2\nreturn x }")
    assert "global" not in code


# ============================================================================
# stdlib.threading
# ============================================================================

def test_threading_lock_and_spawn():
    out, _ = run_aura(
        "import stdlib.threading as threading\n"
        "let mut total = 0\n"
        "let lock = threading.lock()\n"
        "def worker(n) {\n"
        "  let mut i = 0\n"
        "  while i < n {\n"
        "    lock.acquire()\n"
        "    total += 1\n"
        "    lock.release()\n"
        "    i += 1\n"
        "  }\n"
        "}\n"
        "let threads = [threading.spawn((x) => worker(x), 100),"
        " threading.spawn((x) => worker(x), 100)]\n"
        "for t in threads { t.join() }\n"
        "print(total)"
    )
    assert out == "200\n"


def test_threading_run_many():
    out, _ = run_aura(
        "import stdlib.threading as threading\n"
        "let mut hits = 0\n"
        "let lock = threading.lock()\n"
        "def bump() {\n"
        "  lock.acquire()\n"
        "  hits += 1\n"
        "  lock.release()\n"
        "}\n"
        "threading.run_many([bump, bump, bump])\n"
        "print(hits)"
    )
    assert out == "3\n"


def test_threading_map_concurrent():
    out, _ = run_aura(
        "import stdlib.threading as threading\n"
        "def square(x) { return x * x }\n"
        "print(threading.map_concurrent((x) => square(x), [1, 2, 3, 4], workers: 2))"
    )
    assert out == "[1, 4, 9, 16]\n"


def test_threading_event():
    out, _ = run_aura(
        "import stdlib.threading as threading\n"
        "let ev = threading.event()\n"
        "let t = threading.spawn(() => { ev.wait()\nprint('released') })\n"
        "ev.set()\n"
        "t.join()"
    )
    assert out == "released\n"


def test_threading_module_api():
    from aura.stdlib import threading as aura_threading

    lock = aura_threading.lock()
    assert lock.acquire() is True
    assert lock.locked() is True
    lock.release()

    rlock = aura_threading.rlock()
    assert rlock.acquire() is True
    assert rlock.acquire() is True
    rlock.release()
    rlock.release()

    assert aura_threading.active_count() >= 1
    assert aura_threading.current_name()


# ============================================================================
# stdlib.asyncio
# ============================================================================

def test_asyncio_gather():
    out, _ = run_aura_async(
        "import stdlib.asyncio as aio\n"
        "async def work(n) { await aio.sleep(0.001)\nreturn n * 2 }\n"
        "async def main() {\n"
        "  let results = await aio.gather(work(1), work(2), work(3))\n"
        "  print(results)\n"
        "}\n"
        "await main()"
    )
    assert out == "[2, 4, 6]\n"


def test_asyncio_queue():
    out, _ = run_aura_async(
        "import stdlib.asyncio as aio\n"
        "async def main() {\n"
        "  let q = aio.queue()\n"
        "  await q.put(10)\n"
        "  await q.put(20)\n"
        "  print(await q.get())\n"
        "  print(await q.get())\n"
        "}\n"
        "await main()"
    )
    assert out == "10\n20\n"


def test_asyncio_run_from_sync_top_level():
    out, _ = run_aura(
        "import stdlib.asyncio as aio\n"
        "async def work() { return 7 }\n"
        "print(aio.run(work()))"
    )
    assert out == "7\n"


def test_asyncio_module_api():
    from aura.stdlib import asyncio as aio
    import asyncio as real_asyncio

    async def inner():
        assert aio.is_running() is True
        await aio.sleep(0)
        return 1

    assert aio.run(inner()) == 1
    assert aio.is_running() is False

    # run() must refuse to nest inside a running loop.
    async def outer():
        coro = inner()
        with pytest.raises(RuntimeError):
            aio.run(coro)
        coro.close()

    real_asyncio.run(outer())