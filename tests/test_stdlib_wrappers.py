"""Coverage for the thin stdlib wrapper modules.

Each Aura stdlib module delegates to a Python module. These tests exercise the
public surface directly so a wrapper can never silently drift from the Python
API it advertises, and so hidden branches (defaults, error paths, option
handling) are covered.
"""
import math as _math
import os as _os
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))


# ============================================================================
# stdlib.math
# ============================================================================

class TestMath:
    def test_constants_and_lowercase_aliases(self):
        from aura.stdlib import math as m
        assert m.PI == m.pi == _math.pi
        assert m.E == m.e == _math.e
        assert m.TAU == m.tau == _math.tau
        assert m.INF == m.inf == _math.inf
        assert _math.isnan(m.NAN) and _math.isnan(m.nan)

    def test_basic_functions(self):
        from aura.stdlib import math as m
        assert m.abs(-3) == 3
        assert m.min(3, 1, 2) == 1
        assert m.max(3, 1, 2) == 3
        assert m.round(2.6) == 3
        assert m.round(2.345, 2) == 2.35
        assert m.floor(2.9) == 2
        assert m.ceil(2.1) == 3
        assert m.sqrt(9) == 3
        assert m.pow(2, 10) == 1024

    def test_logarithms_and_exp(self):
        from aura.stdlib import math as m
        assert m.log(_math.e) == pytest.approx(1.0)
        assert m.log(100, 10) == pytest.approx(2.0)
        assert m.log10(1000) == pytest.approx(3.0)
        assert m.log2(8) == pytest.approx(3.0)
        assert m.exp(0) == pytest.approx(1.0)

    def test_trigonometry(self):
        from aura.stdlib import math as m
        assert m.sin(0) == pytest.approx(0.0)
        assert m.cos(0) == pytest.approx(1.0)
        assert m.tan(0) == pytest.approx(0.0)
        assert m.asin(0) == pytest.approx(0.0)
        assert m.acos(1) == pytest.approx(0.0)
        assert m.atan(0) == pytest.approx(0.0)
        assert m.atan2(0, 1) == pytest.approx(0.0)
        assert m.sinh(0) == pytest.approx(0.0)
        assert m.cosh(0) == pytest.approx(1.0)
        assert m.tanh(0) == pytest.approx(0.0)

    def test_angle_conversion(self):
        from aura.stdlib import math as m
        assert m.degrees(_math.pi) == pytest.approx(180.0)
        assert m.radians(180) == pytest.approx(_math.pi)

    def test_gcd_lcm_and_empty_identity(self):
        from aura.stdlib import math as m
        assert m.gcd() == 0
        assert m.gcd(12, 18, 24) == 6
        assert m.lcm() == 1
        assert m.lcm(2, 3, 4) == 12

    def test_combinatorics(self):
        from aura.stdlib import math as m
        assert m.factorial(5) == 120
        assert m.comb(5, 2) == 10
        assert m.perm(5, 2) == 20
        assert m.perm(5) == 120

    def test_predicates(self):
        from aura.stdlib import math as m
        assert m.is_finite(1.0) is True
        assert m.is_infinite(_math.inf) is True
        assert m.is_nan(_math.nan) is True

    def test_float_helpers(self):
        from aura.stdlib import math as m
        assert m.copysign(3, -1) == -3
        assert m.fabs(-2.5) == 2.5
        assert m.fmod(7, 3) == pytest.approx(1.0)
        assert m.fsum([0.1, 0.2]) == pytest.approx(0.3)
        assert m.prod([2, 3, 4]) == 24
        assert m.prod([1, 2], start=10) == 20
        assert m.remainder(7, 3) == pytest.approx(1.0)
        assert m.dist((0, 0), (3, 4)) == pytest.approx(5.0)
        assert m.hypot(3, 4) == pytest.approx(5.0)


# ============================================================================
# stdlib.os
# ============================================================================

class TestOs:
    def test_env_get_set_unset(self, monkeypatch):
        from aura.stdlib import os as a
        monkeypatch.delenv('AURA_WRAPPER_TEST', raising=False)
        assert a.get_env('AURA_WRAPPER_TEST') is None
        assert a.get_env('AURA_WRAPPER_TEST', 'fallback') == 'fallback'
        a.set_env('AURA_WRAPPER_TEST', 42)
        assert a.get_env('AURA_WRAPPER_TEST') == '42'
        a.unset_env('AURA_WRAPPER_TEST')
        assert a.get_env('AURA_WRAPPER_TEST') is None

    def test_env_allowlist_and_full(self, monkeypatch):
        from aura.stdlib import os as a
        monkeypatch.setenv('AURA_ONE', '1')
        monkeypatch.setenv('AURA_TWO', '2')
        assert a.env(allowlist=['AURA_ONE']) == {'AURA_ONE': '1'}
        assert a.env(allowlist=[]) == {}
        full = a.env()
        assert full['AURA_ONE'] == '1' and full['AURA_TWO'] == '2'

    def test_cwd_home_tempdir(self):
        from aura.stdlib import os as a
        assert a.cwd() == _os.getcwd()
        assert a.home() == str(Path.home())
        assert isinstance(a.temp_dir(), str)

    def test_chdir_roundtrip(self, tmp_path):
        from aura.stdlib import os as a
        original = a.cwd()
        try:
            a.chdir(str(tmp_path))
            assert a.cwd() == str(tmp_path)
        finally:
            a.chdir(original)

    def test_path_helpers(self, tmp_path):
        from aura.stdlib import os as a
        f = tmp_path / 'file.txt'
        f.write_text('x')
        assert a.path_join('a', 'b') == _os.path.join('a', 'b')
        assert a.path_exists(str(f)) is True
        assert a.path_is_file(str(f)) is True
        assert a.path_is_dir(str(tmp_path)) is True
        assert a.path_basename(str(f)) == 'file.txt'
        assert a.path_dirname(str(f)) == str(tmp_path)
        assert a.path_split(str(f)) == list(_os.path.split(str(f)))
        assert a.path_splitext(str(f)) == list(_os.path.splitext(str(f)))
        assert a.path_normpath('a/../b') == 'b'
        assert a.path_abspath('x') == _os.path.abspath('x')

    def test_platform_metadata(self):
        from aura.stdlib import os as a
        assert a.sep() == _os.sep
        assert a.linesep() == _os.linesep
        assert a.name() == _os.name
        assert a.platform() == sys.platform
        assert isinstance(a.pid(), int)

    def test_directory_operations(self, tmp_path):
        from aura.stdlib import os as a
        d = tmp_path / 'nested' / 'deep'
        a.makedirs(str(d))
        assert d.is_dir()
        (d / 'f.txt').write_text('hello')
        assert 'f.txt' in a.listdir(str(d))
        walk = a.walk(str(d))
        assert any('f.txt' in files for _, _, files in walk)
        assert a.get_size(str(d / 'f.txt')) == 5

    def test_remove_and_rename(self, tmp_path):
        from aura.stdlib import os as a
        src = tmp_path / 'a.txt'
        src.write_text('data')
        dst = tmp_path / 'b.txt'
        a.rename(str(src), str(dst))
        assert dst.is_file() and not src.exists()
        a.remove(str(dst))
        assert not dst.exists()


# ============================================================================
# stdlib.io
# ============================================================================

class TestIo:
    def test_read_write_append(self, tmp_path):
        from aura.stdlib import io as a
        p = str(tmp_path / 'f.txt')
        a.write(p, 'one\n')
        a.append(p, 'two\n')
        assert a.read(p) == 'one\ntwo\n'

    def test_predicates_and_extension(self, tmp_path):
        from aura.stdlib import io as a
        f = tmp_path / 'x.md'
        f.write_text('x')
        assert a.exists(str(f)) is True
        assert a.is_file(str(f)) is True
        assert a.is_dir(str(tmp_path)) is True
        assert a.extension(str(f)) == 'md'
        assert a.basename(str(f)) == 'x.md'
        assert a.dirname(str(f)) == str(tmp_path)
        assert a.join(str(tmp_path), 'a', 'b') == str(tmp_path / 'a' / 'b')

    def test_lines_roundtrip(self, tmp_path):
        from aura.stdlib import io as a
        p = str(tmp_path / 'lines.txt')
        a.write_lines(p, ['a', 'b', ''])
        assert a.read_lines(p) == ['a', 'b', '']

    def test_ls_mkdir_rm_rename_size_touch(self, tmp_path):
        from aura.stdlib import io as a
        d = tmp_path / 'dir'
        a.mkdir(str(d))
        a.touch(str(d / 'z.txt'))
        assert a.ls(str(d)) == ['z.txt']
        a.rename(str(d / 'z.txt'), str(d / 'y.txt'))
        assert a.size(str(d / 'y.txt')) == 0
        a.rm(str(d / 'y.txt'))
        assert a.ls(str(d)) == []

    def test_copy(self, tmp_path):
        from aura.stdlib import io as a
        src = tmp_path / 'src.txt'
        src.write_text('payload')
        dst = tmp_path / 'dst.txt'
        a.copy(str(src), str(dst))
        assert dst.read_text() == 'payload'


# ============================================================================
# stdlib.time
# ============================================================================

class TestTime:
    def test_clock_functions(self):
        from aura.stdlib import time as t
        assert t.now() > 0
        assert t.now_ms() > 0
        assert t.clock() >= 0
        assert t.monotonic() >= 0
        assert t.perf_counter() >= 0

    def test_sleep_advances_monotonic(self):
        from aura.stdlib import time as t
        start = t.monotonic()
        t.sleep(0.01)
        assert t.monotonic() >= start

    def test_format_parse_iso(self):
        from aura.stdlib import time as t
        parsed = t.parse('2020-01-02 03:04:05')
        assert (parsed.year, parsed.month, parsed.day) == (2020, 1, 2)
        assert t.strftime('%Y') == str(t.parse(t.strftime('%Y-%m-%d %H:%M:%S')).year)
        assert 'T' in t.iso()

    def test_timestamp_default_and_explicit(self):
        from aura.stdlib import time as t
        assert t.timestamp() > 0
        parsed = t.parse('2020-01-02 03:04:05')
        assert t.timestamp(parsed) == pytest.approx(parsed.timestamp())

    def test_elapsed(self):
        from aura.stdlib import time as t
        start = t.now()
        assert t.elapsed(start) >= 0


# ============================================================================
# stdlib.regex
# ============================================================================

class TestRegex:
    def test_matching_functions(self):
        from aura.stdlib import regex as r
        assert r.match(r'\d+', '123abc').group() == '123'
        assert r.match(r'\d+', 'abc') is None
        assert r.full_match(r'\d+', '123').group() == '123'
        assert r.full_match(r'\d+', '123a') is None
        assert r.search(r'\d+', 'abc123').group() == '123'
        assert r.search(r'\d+', 'abc') is None

    def test_find_split_replace(self):
        from aura.stdlib import regex as r
        assert r.find_all(r'\d+', 'a1b22c333') == ['1', '22', '333']
        assert [m.group() for m in r.find_iter(r'\d', 'a1b2')] == ['1', '2']
        assert r.split(r',', 'a,b,c') == ['a', 'b', 'c']
        assert r.split(r',', 'a,b,c', limit=1) == ['a', 'b,c']
        assert r.replace(r'\d', '#', 'a1b2') == 'a#b#'
        assert r.replace(r'\d', '#', 'a1b2', count=1) == 'a#b2'
        assert r.replace_fn(r'\d', lambda m: f'[{m.group()}]', 'a1') == 'a[1]'

    def test_groups_escape_compile(self):
        from aura.stdlib import regex as r
        assert r.groups(r'(\d)(\d)', 'ab12') == ['1', '2']
        assert r.groups(r'(\d)', 'abc') is None
        assert r.escape('a.b') == 'a\\.b'
        compiled = r.compile_pattern(r'\d+')
        assert compiled.search('x9').group() == '9'

    def test_flags_are_exposed(self):
        import re

        from aura.stdlib import regex as r
        assert r.IGNORECASE == re.IGNORECASE
        assert r.MULTILINE == re.MULTILINE
        assert r.DOTALL == re.DOTALL
        assert r.VERBOSE == re.VERBOSE
        assert r.ASCII == re.ASCII


# ============================================================================
# stdlib.threading - primitives not covered elsewhere
# ============================================================================

class TestThreadingExtras:
    def test_rlock_reentrant(self):
        from aura.stdlib import threading as th
        lock = th.rlock()
        lock.acquire()
        lock.acquire()
        lock.release()
        lock.release()

    def test_lock_context_manager_and_locked(self):
        from aura.stdlib import threading as th
        lock = th.lock()
        with lock:
            assert lock.locked() is True
        assert lock.locked() is False

    def test_lock_timeout_and_nonblocking(self):
        from aura.stdlib import threading as th
        lock = th.lock()
        lock.acquire()
        assert lock.acquire(blocking=False) is False
        assert lock.acquire(timeout=0.01) is False
        lock.release()

    def test_thread_handle_properties(self):
        from aura.stdlib import threading as th
        t = th.thread(lambda: None, name='worker')
        assert t.name == 'worker'
        t.start()
        t.join()
        assert t.is_alive() is False
        assert t.ident is not None

    def test_condition_event_semaphore_barrier(self):
        from aura.stdlib import threading as th
        inner = th.lock()
        cond = th.condition(inner)
        assert cond is not None
        cond_raw = th.condition()
        assert cond_raw is not None
        assert th.event() is not None
        sem = th.semaphore(2)
        assert sem.acquire() is True and sem.acquire() is True
        assert th.barrier(2) is not None
        assert th.local() is not None

    def test_introspection_helpers(self):
        from aura.stdlib import threading as th
        assert th.current_name() == 'MainThread'
        assert th.active_count() >= 1
        assert any(t is th.main_thread() for t in th.enumerate_threads())
        assert isinstance(th.get_ident(), int)

    def test_stack_size_get_and_default(self):
        from aura.stdlib import threading as th
        assert isinstance(th.stack_size(), int)


# ============================================================================
# stdlib.asyncio - primitives not covered elsewhere
# ============================================================================

class TestAsyncioExtras:
    def test_is_running_false_outside_loop(self):
        from aura.stdlib import asyncio as aio
        assert aio.is_running() is False

    def test_run_rejects_nested_loop(self):
        from aura.stdlib import asyncio as aio

        async def inner():
            raise AssertionError('should not run')

        async def outer():
            coro = inner()
            try:
                aio.run(coro)
            finally:
                # `aio.run` refuses before awaiting; close it to avoid a
                # "coroutine was never awaited" warning.
                coro.close()

        with pytest.raises(RuntimeError):
            aio.run(outer())

    def test_wait_wraps_coroutines(self):
        from aura.stdlib import asyncio as aio

        async def work(n):
            return n

        async def main():
            done, pending = await aio.wait(work(1), work(2))
            return sorted(t.result() for t in done), len(pending)

        results, pending = aio.run(main())
        assert results == [1, 2] and pending == 0

    def test_as_completed_and_create_task(self):
        from aura.stdlib import asyncio as aio

        async def work(n):
            return n * 2

        async def main():
            task = aio.create_task(work(3))
            collected = []
            for fut in aio.as_completed([work(1), work(2)]):
                collected.append(await fut)
            return await task, sorted(collected)

        task_result, collected = aio.run(main())
        assert task_result == 6 and collected == [2, 4]

    def test_wait_for_timeout(self):
        import asyncio as _std_asyncio

        from aura.stdlib import asyncio as aio

        async def slow():
            await aio.sleep(1)

        async def main():
            # `asyncio.TimeoutError` is an alias of the builtin TimeoutError
            # only from 3.11; accept either so the test is version-agnostic.
            with pytest.raises((TimeoutError, _std_asyncio.TimeoutError)):
                await aio.wait_for(slow(), 0.01)

        aio.run(main())

    def test_primitives_and_task_introspection(self):
        from aura.stdlib import asyncio as aio

        async def main():
            q = aio.queue(maxsize=1)
            await q.put(5)
            assert await q.get() == 5
            async with aio.lock():
                pass
            ev = aio.event()
            ev.set()
            assert ev.is_set()
            sem = aio.semaphore(1)
            async with sem:
                pass
            current = aio.current_task()
            assert current in aio.all_tasks()
            return True

        assert aio.run(main()) is True

    def test_event_loop_lifecycle(self):
        from aura.stdlib import asyncio as aio
        loop = aio.new_event_loop()
        try:
            aio.set_event_loop(loop)
            assert aio.get_event_loop() is loop
        finally:
            loop.close()


# ============================================================================
# stdlib.python - explicit Python interop bridge
# ============================================================================

class TestPythonBridge:
    def test_import_module_and_load(self):
        from aura.stdlib import python as p
        mod = p.import_module('math')
        assert mod.sqrt(9) == 3
        proxy = p.load('math')
        assert proxy.sqrt(16) == 4

    def test_import_module_rejects_bad_names(self):
        from aura.stdlib import python as p
        with pytest.raises(TypeError):
            p.import_module('')
        with pytest.raises(TypeError):
            p.import_module(123)

    def test_reload(self):
        from aura.stdlib import python as p
        mod = p.import_module('math')
        assert p.reload(mod) is mod
        proxy = p.load('math')
        assert p.reload(proxy) is mod
        with pytest.raises(TypeError):
            p.reload('not a module')

    def test_is_available(self):
        from aura.stdlib import python as p
        assert p.is_available('math') is True
        assert p.is_available('definitely_not_a_real_module_xyz') is False

    def test_eval_and_exec(self):
        from aura.stdlib import python as p
        assert p.eval('1 + 2') == 3
        with pytest.raises(TypeError):
            p.eval(5)
        namespace = p.exec_code('answer = 42')
        assert namespace['answer'] == 42
        with pytest.raises(TypeError):
            p.exec_code(5)

    def test_compile_source_and_call(self):
        from aura.stdlib import python as p
        code = p.compile_source('x = 1', filename='<t>')
        assert code is not None
        assert p.call(lambda a, b: a + b, 2, 3) == 5
        with pytest.raises(TypeError):
            p.call(5)

    def test_attribute_helpers(self):
        from aura.stdlib import python as p

        class Obj:
            value = 10

        obj = Obj()
        assert p.getattr(obj, 'value') == 10
        assert p.getattr(obj, 'missing', 'dflt') == 'dflt'
        assert p.hasattr(obj, 'value') is True
        assert 'value' in p.dir(obj)
        p.setattr(obj, 'other', 7)
        assert obj.other == 7

    def test_type_predicates(self):
        import types as _types

        from aura.stdlib import python as p
        assert p.type_name(1) == 'builtins.int'
        assert p.is_module(p.import_module('math')) is True
        assert p.is_module(p.load('math')) is True
        assert p.is_module(1) is False
        assert p.is_callable(len) is True
        assert p.is_class(int) is True
        assert p.is_instance(1, int) is True
        assert p.is_instance('x', int) is False
        assert isinstance(p.py_builtins, _types.ModuleType)
        assert isinstance(p.builtins, _types.ModuleType)

    def test_to_aura_and_to_python(self):
        from aura.stdlib import python as p
        proxy = p.to_aura(p.import_module('math'))
        assert isinstance(proxy, p.ModuleProxy)
        assert p.to_python(proxy) is p.import_module('math')
        assert p.to_python(5) == 5
        converted = p.to_aura({'a': [1, (2, 3)], 's': {4}})
        assert converted['a'][0] == 1
        assert converted['a'][1] == (2, 3)
        assert converted['s'] == {4}
        assert p.to_aura(42) == 42

    def test_interpreter_and_path_helpers(self):
        from aura.stdlib import python as p
        assert p.interpreter_version().major == sys.version_info.major
        path = p.add_path('/tmp/aura-nonexistent-path-xyz')
        assert path in sys.path
        assert p.add_path(path) == path  # idempotent
        assert isinstance(p.site_packages(), list)
        assert 'math' in p.modules()

    def test_module_proxy_protocol(self):
        from aura.stdlib import python as p
        proxy = p.load('os')
        assert p.is_module(proxy.path) is True  # nested module proxied
        assert proxy == proxy
        assert proxy == p.import_module('os')
        assert hash(proxy) == hash(p.import_module('os'))
        assert 'name' in dir(proxy)
        assert 'ModuleProxy' in repr(proxy)


# ============================================================================
# stdlib.http - offline helpers only (no network)
# ============================================================================

class TestHttpOffline:
    def test_url_helpers(self):
        from aura.stdlib import http as h
        assert h.quote('a b') == 'a%20b'
        assert h.unquote('a%20b') == 'a b'
        assert h.build_url('http://x') == 'http://x'
        assert h.build_url('http://x', {'a': 1}) == 'http://x?a=1'
        assert h.build_url('http://x?y=1', {'a': 2}) == 'http://x?y=1&a=2'

    def test_json_headers(self):
        from aura.stdlib import http as h
        assert h._json_headers(None, None) == {}
        assert h._json_headers({'X': '1'}, None) == {'X': '1'}
        headers = h._json_headers({'X': '1'}, {'a': 1})
        assert headers['Content-Type'] == 'application/json'
        assert headers['X'] == '1'

    def test_read_limited_unlimited_and_capped(self):
        import io as _io

        from aura.stdlib import http as h
        assert h._read_limited(_io.BytesIO(b'abcdef'), 0) == b'abcdef'
        assert h._read_limited(_io.BytesIO(b'abcdef'), None) == b'abcdef'
        assert h._read_limited(_io.BytesIO(b'abcdef'), 3) == b'abc'

    def test_max_bytes_env_override(self, monkeypatch):
        from aura.stdlib import http as h
        monkeypatch.delenv('AURA_HTTP_MAX_BYTES', raising=False)
        assert h._max_bytes() == h._DEFAULT_MAX_BYTES
        monkeypatch.setenv('AURA_HTTP_MAX_BYTES', '1024')
        assert h._max_bytes() == 1024
        monkeypatch.setenv('AURA_HTTP_MAX_BYTES', 'not-a-number')
        assert h._max_bytes() == h._DEFAULT_MAX_BYTES

    def test_allow_private_flag(self, monkeypatch):
        from aura.stdlib import http as h
        monkeypatch.delenv('AURA_HTTP_ALLOW_PRIVATE', raising=False)
        assert h._allow_private() is False
        monkeypatch.setenv('AURA_HTTP_ALLOW_PRIVATE', '1')
        assert h._allow_private() is True

    def test_is_blocked_host(self):
        from aura.stdlib import http as h
        assert h._is_blocked_host('127.0.0.1') is True
        assert h._is_blocked_host('192.168.1.1') is True
        assert h._is_blocked_host('8.8.8.8') is False

    def test_response_shape(self):
        from aura.stdlib import http as h
        resp = h._response(200, 'body', {'H': '1'}, True, 'http://x')
        assert resp.status == 200
        assert resp['body'] == 'body'
        assert resp.ok is True

    def test_as_aura_recursive(self):
        from aura.stdlib import http as h
        from aura.stdlib.collections import AuraDict
        converted = h._as_aura({'a': [{'b': 1}], 'c': 2})
        assert isinstance(converted, AuraDict)
        assert isinstance(converted['a'][0], AuraDict)
        assert converted['a'][0].b == 1
        assert h._as_aura(5) == 5

    def test_validate_url_rejects_bad_inputs(self):
        from aura.stdlib import http as h
        with pytest.raises(ValueError):
            h._validate_url('')
        with pytest.raises(ValueError):
            h._validate_url(123)
        with pytest.raises(ValueError):
            h._validate_url('ftp://example.com')
        with pytest.raises(ValueError):
            h._validate_url('http://')

    def test_use_requests_returns_bool(self):
        from aura.stdlib import http as h
        assert isinstance(h._use_requests(), bool)
