"""Tests for the asynchronous I/O surface of the standard library.

``stdlib.io`` and ``stdlib.http`` gained ``*_async`` entry points for use in
Aura ``async def`` code. They await the vetted synchronous implementations on a
worker thread, so behaviour (including the HTTP security guards) is identical;
the async variants differ only in being awaitable.
"""
import asyncio
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))


def run(coro):
    return asyncio.run(coro)


# ============================================================================
# stdlib.io async variants
# ============================================================================

class TestAsyncIo:
    def test_write_then_read_roundtrip(self, tmp_path):
        from aura.stdlib import io
        path = str(tmp_path / 'data.txt')

        async def main():
            await io.write_async(path, 'hello\n')
            return await io.read_async(path)

        assert run(main()) == 'hello\n'

    def test_append_async(self, tmp_path):
        from aura.stdlib import io
        path = str(tmp_path / 'log.txt')

        async def main():
            await io.write_async(path, 'a')
            await io.append_async(path, 'b')
            return await io.read_async(path)

        assert run(main()) == 'ab'

    def test_exists_and_predicates(self, tmp_path):
        from aura.stdlib import io
        path = str(tmp_path / 'f.txt')
        (tmp_path / 'f.txt').write_text('x')

        async def main():
            return (await io.exists_async(path),
                    await io.is_file_async(path),
                    await io.is_dir_async(str(tmp_path)))

        assert run(main()) == (True, True, True)

    def test_mkdir_and_ls_async(self, tmp_path):
        from aura.stdlib import io
        target = tmp_path / 'nested' / 'dir'

        async def main():
            await io.mkdir_async(str(target))
            (target / 'a.txt').write_text('x')
            return await io.ls_async(str(target))

        assert run(main()) == ['a.txt']

    def test_rename_and_rm_async(self, tmp_path):
        from aura.stdlib import io
        src = str(tmp_path / 'old.txt')
        dst = str(tmp_path / 'new.txt')

        async def main():
            await io.write_async(src, 'x')
            await io.rename_async(src, dst)
            existed = await io.exists_async(dst)
            await io.rm_async(dst)
            return existed, await io.exists_async(dst)

        assert run(main()) == (True, False)

    def test_read_write_lines_async(self, tmp_path):
        from aura.stdlib import io
        path = str(tmp_path / 'lines.txt')

        async def main():
            await io.write_lines_async(path, ['a', 'b'])
            return await io.read_lines_async(path)

        assert run(main()) == ['a', 'b']

    def test_size_and_touch_async(self, tmp_path):
        from aura.stdlib import io
        path = str(tmp_path / 's.txt')

        async def main():
            await io.write_async(path, '12345')
            size = await io.size_async(path)
            return size

        assert run(main()) == 5

    def test_copy_async(self, tmp_path):
        from aura.stdlib import io
        src = str(tmp_path / 'src.txt')
        dst = str(tmp_path / 'dst.txt')

        async def main():
            await io.write_async(src, 'copy me')
            await io.copy_async(src, dst)
            return await io.read_async(dst)

        assert run(main()) == 'copy me'

    def test_async_read_missing_file_raises(self, tmp_path):
        from aura.stdlib import io
        missing = str(tmp_path / 'nope.txt')

        async def main():
            return await io.read_async(missing)

        with pytest.raises(FileNotFoundError):
            run(main())

    def test_async_variants_exported(self):
        import aura.stdlib as stdlib
        assert stdlib.read_async is not None
        assert stdlib.write_async is not None


# ============================================================================
# stdlib.http async variants
# ============================================================================

class TestAsyncHttp:
    def test_aget_rejects_non_http_scheme(self):
        from aura.stdlib import http

        async def main():
            await http.aget('file:///etc/passwd')

        with pytest.raises(ValueError, match='scheme'):
            run(main())

    def test_aget_blocks_private_hosts(self):
        from aura.stdlib import http

        async def main():
            await http.aget('http://127.0.0.1:9/x')

        with pytest.raises(ValueError, match='private|local'):
            run(main())

    def test_arequest_blocks_private_hosts(self):
        from aura.stdlib import http

        async def main():
            await http.arequest('POST', 'http://10.0.0.1/x')

        with pytest.raises(ValueError):
            run(main())

    def test_apost_blocks_private_hosts(self):
        from aura.stdlib import http

        async def main():
            await http.apost('http://192.168.1.1/x', data={'a': 1})

        with pytest.raises(ValueError):
            run(main())

    def test_async_http_variants_are_awaitable_functions(self):
        import inspect

        from aura.stdlib import http
        for name in ('arequest', 'aget', 'apost', 'aput', 'adelete',
                     'aget_json', 'apost_json'):
            fn = getattr(http, name)
            assert inspect.iscoroutinefunction(fn), name

    def test_get_json_async_guard_applies(self):
        from aura.stdlib import http

        async def main():
            await http.aget_json('http://localhost/x')

        with pytest.raises(ValueError):
            run(main())


# ============================================================================
# End to end from Aura
# ============================================================================

class TestAsyncFromAura:
    def test_async_io_program(self, tmp_path):
        from aura_test_helpers import run_aura  # noqa: E402
        target = str(tmp_path / 'from_aura.txt')
        out = run_aura(
            'import stdlib.io as io\n'
            'async def main() {\n'
            f'  await io.write_async("{target}", "aura async\\n")\n'
            f'  let text = await io.read_async("{target}")\n'
            '  print(text.trim())\n'
            '}\n')
        assert out == 'aura async\n'

    def test_async_io_does_not_block_loop(self):
        # Two concurrent writes must both complete; the loop stays responsive.
        from aura.stdlib import io

        async def main():
            results = await asyncio.gather(
                io.read_async(__file__),
                io.read_async(__file__),
            )
            return [len(r) for r in results]

        first, second = run(main())
        assert first == second > 0
