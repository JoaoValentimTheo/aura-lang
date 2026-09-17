"""Aura Standard Library - Asyncio module.

Coroutines and structured concurrency, wrapping Python's :mod:`asyncio`. Aura
already has `async def` and `await` at the language level; this module provides
the task/gather/queue primitives.

Example::

    import stdlib.asyncio as aio

    async def work(n) {
      await aio.sleep(0.01)
      return n * 2
    }

    async def main() {
      let results = await aio.gather(work(1), work(2), work(3))
      print(results)
    }

    aio.run(main())
"""

import asyncio as _asyncio


def run(coro):
    """Run a coroutine to completion and return its result.

    Raises a clear error if called from inside an already-running event loop.
    """
    try:
        _asyncio.get_running_loop()
    except RuntimeError:
        return _asyncio.run(coro)
    raise RuntimeError(
        "aio.run() cannot be called from a running event loop; "
        "use 'await' instead"
    )


def sleep(seconds):
    """Suspend the current coroutine for ``seconds`` (await this)."""
    return _asyncio.sleep(seconds)


def create_task(coro, name=None):
    """Schedule ``coro`` as a task on the running loop."""
    return _asyncio.create_task(coro, name=name)


def gather(*coros, return_exceptions=False):
    """Await several coroutines concurrently and return their results."""
    return _asyncio.gather(*coros, return_exceptions=return_exceptions)


def wait_for(coro, timeout):
    """Await ``coro`` but cancel it if it exceeds ``timeout`` seconds."""
    return _asyncio.wait_for(coro, timeout)


def wait(*coros, timeout=None, return_when='ALL_COMPLETED'):
    """Wait for coroutines, returning ``(done, pending)``.

    Raw coroutines are wrapped in tasks first: Python's ``asyncio.wait``
    rejects bare coroutines and would otherwise raise ``TypeError``.
    """
    modes = {
        'ALL_COMPLETED': _asyncio.ALL_COMPLETED,
        'FIRST_COMPLETED': _asyncio.FIRST_COMPLETED,
        'FIRST_EXCEPTION': _asyncio.FIRST_EXCEPTION,
    }
    mode = modes.get(return_when, _asyncio.ALL_COMPLETED)
    tasks = [_asyncio.ensure_future(c) for c in coros]
    return _asyncio.wait(tasks, timeout=timeout, return_when=mode)


def as_completed(coros, timeout=None):
    """Yield results as each coroutine completes (async iterator)."""
    tasks = [_asyncio.ensure_future(c) for c in coros]
    return _asyncio.as_completed(tasks, timeout=timeout)


def queue(maxsize=0):
    """Create an async queue."""
    return _asyncio.Queue(maxsize=maxsize)


def lock():
    """Create an async lock."""
    return _asyncio.Lock()


def event():
    """Create an async event."""
    return _asyncio.Event()


def semaphore(value=1):
    """Create an async semaphore."""
    return _asyncio.Semaphore(value)


def current_task():
    """The currently running task, or ``None``."""
    return _asyncio.current_task()


def all_tasks():
    """All tasks that are not yet finished on the running loop."""
    return _asyncio.all_tasks()


def is_running():
    """True when called from inside a running event loop."""
    try:
        _asyncio.get_running_loop()
        return True
    except RuntimeError:
        return False


def new_event_loop():
    """Create a fresh event loop."""
    return _asyncio.new_event_loop()


def set_event_loop(loop):
    """Install ``loop`` as the current event loop."""
    _asyncio.set_event_loop(loop)


def get_event_loop():
    """Return the current event loop."""
    return _asyncio.get_event_loop()
