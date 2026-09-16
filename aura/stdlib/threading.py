"""Aura Standard Library - Threading module.

Native OS threads, wrapping Python's :mod:`threading`. Use this for I/O-bound
work that would otherwise block; for CPU-bound work prefer processes.

Aura's globals are shared across threads (like Python), so synchronize access
to shared state with the lock/event primitives here.

Example::

    import stdlib.threading as threading

    let mut total = 0
    let lock = threading.lock()

    def worker(n) {
      let mut i = 0
      while i < n {
        lock.acquire()
        total += 1
        lock.release()
        i += 1
      }
    }

    let t = threading.spawn((x) => worker(x), 100)
    t.join()
    print(total)
"""

import threading as _threading


class _Lock:
    """A mutex. Usable as a context manager: ``with lock { ... }``."""

    def __init__(self):
        self._lock = _threading.Lock()

    def acquire(self, blocking=True, timeout=-1):
        if timeout is None or timeout < 0:
            return self._lock.acquire(blocking)
        return self._lock.acquire(blocking, timeout)

    def release(self):
        self._lock.release()

    def locked(self):
        return self._lock.locked()

    def __enter__(self):
        self._lock.acquire()
        return self

    def __exit__(self, exc_type, exc, tb):
        self._lock.release()


class _RLock:
    """A reentrant mutex (the same thread may acquire it repeatedly)."""

    def __init__(self):
        self._lock = _threading.RLock()

    def acquire(self, blocking=True, timeout=-1):
        if timeout is None or timeout < 0:
            return self._lock.acquire(blocking)
        return self._lock.acquire(blocking, timeout)

    def release(self):
        self._lock.release()

    def __enter__(self):
        self._lock.acquire()
        return self

    def __exit__(self, exc_type, exc, tb):
        self._lock.release()


class Thread:
    """A handle to a running thread."""

    def __init__(self, target, args=(), kwargs=None, daemon=False, name=None):
        self._thread = _threading.Thread(
            target=target, args=tuple(args), kwargs=dict(kwargs or {}),
            daemon=daemon, name=name,
        )

    def start(self):
        self._thread.start()
        return self

    def join(self, timeout=None):
        self._thread.join(timeout)
        return self

    def is_alive(self):
        return self._thread.is_alive()

    @property
    def name(self):
        return self._thread.name

    @property
    def ident(self):
        return self._thread.ident


def spawn(target, *args, daemon=False, name=None):
    """Create and start a thread running ``target(*args)``."""
    return Thread(target, args, daemon=daemon, name=name).start()


def thread(target, args=(), kwargs=None, daemon=False, name=None):
    """Create a thread without starting it (call ``.start()``)."""
    return Thread(target, args, kwargs, daemon=daemon, name=name)


def lock():
    """Create a non-reentrant mutex."""
    return _Lock()


def rlock():
    """Create a reentrant mutex."""
    return _RLock()


def event():
    """Create an event for signalling between threads."""
    return _threading.Event()


def condition(lock_obj=None):
    """Create a condition variable, optionally wrapping ``lock_obj``."""
    inner = lock_obj._lock if isinstance(lock_obj, (_Lock, _RLock)) else lock_obj
    return _threading.Condition(inner)


def semaphore(value=1):
    """Create a semaphore with an initial permit count."""
    return _threading.Semaphore(value)


def barrier(parties):
    """Create a barrier for ``parties`` threads."""
    return _threading.Barrier(parties)


def local():
    """Create thread-local storage."""
    return _threading.local()


def current_name():
    """Name of the current thread."""
    return _threading.current_thread().name


def active_count():
    """Number of live thread objects."""
    return _threading.active_count()


def enumerate_threads():
    """List of live thread objects."""
    return _threading.enumerate()


def main_thread():
    """The main thread object."""
    return _threading.main_thread()


def get_ident():
    """Identifier of the current thread."""
    return _threading.get_ident()


def stack_size(size=None):
    """Get (or set, when ``size`` is given) the thread stack size."""
    if size is None:
        return _threading.stack_size()
    return _threading.stack_size(size)


def run_many(targets):
    """Start every callable in ``targets`` and wait for all of them.

    Returns the list of threads in the order given.
    """
    threads = [spawn(t) for t in targets]
    for t in threads:
        t.join()
    return threads


def map_concurrent(fn, items, workers=4):
    """Map ``fn`` over ``items`` using a bounded pool of ``workers`` threads."""
    from concurrent.futures import ThreadPoolExecutor

    with ThreadPoolExecutor(max_workers=workers) as pool:
        return list(pool.map(fn, items))
