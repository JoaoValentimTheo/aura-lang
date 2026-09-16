"""Tests for stdlib: json, time, io modules."""
import pytest
import sys
import os
import tempfile

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


class TestJsonStdlib:
    def test_loads(self):
        from stdlib.json import loads
        assert loads('{"a": 1}') == {"a": 1}

    def test_loads_list(self):
        from stdlib.json import loads
        assert loads('[1, 2, 3]') == [1, 2, 3]

    def test_dumps(self):
        from stdlib.json import dumps
        result = dumps({"a": 1})
        assert '"a"' in result
        assert '1' in result

    def test_pretty(self):
        from stdlib.json import pretty
        result = pretty({"key": "val"})
        assert '\n' in result
        assert '  "key"' in result

    def test_parse_alias(self):
        from stdlib.json import parse
        assert parse('{"x": 10}') == {"x": 10}

    def test_stringify_alias(self):
        from stdlib.json import stringify
        result = stringify([1, 2])
        assert result == '[1, 2]'

    def test_is_valid(self):
        from stdlib.json import is_valid
        assert is_valid('{"a": 1}') is True
        assert is_valid('not json') is False
        assert is_valid(None) is False

    def test_merge(self):
        from stdlib.json import merge
        import json as _json
        result = _json.loads(merge({"a": 1}, {"b": 2}))
        assert result == {"a": 1, "b": 2}

    def test_merge_overwrite(self):
        from stdlib.json import merge
        import json as _json
        result = _json.loads(merge({"a": 1}, {"a": 2}))
        assert result == {"a": 2}

    def test_load_file(self):
        from stdlib.json import load, dump
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            path = f.name
        try:
            dump({"hello": "world"}, path)
            assert load(path) == {"hello": "world"}
        finally:
            os.unlink(path)


class TestTimeStdlib:
    def test_now(self):
        from stdlib.time import now
        t = now()
        assert isinstance(t, float)
        assert t > 0

    def test_now_ms(self):
        from stdlib.time import now_ms
        t = now_ms()
        assert isinstance(t, float)
        assert t > 1_000_000

    def test_clock(self):
        from stdlib.time import clock
        t = clock()
        assert isinstance(t, float)

    def test_monotonic(self):
        from stdlib.time import monotonic
        t = monotonic()
        assert isinstance(t, float)

    def test_perf_counter(self):
        from stdlib.time import perf_counter
        t = perf_counter()
        assert isinstance(t, float)

    def test_iso(self):
        from stdlib.time import iso
        s = iso()
        assert 'T' in s

    def test_strftime(self):
        from stdlib.time import strftime
        s = strftime("%Y")
        assert s == str(__import__('datetime').datetime.now().year)

    def test_elapsed(self):
        from stdlib.time import now, elapsed
        t = now()
        e = elapsed(t)
        assert e >= 0
        assert e < 5


class TestIoStdlib:
    def test_read_write(self):
        from stdlib.io import read, write
        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False, mode='w') as f:
            path = f.name
        try:
            write(path, "hello aura")
            assert read(path) == "hello aura"
        finally:
            os.unlink(path)

    def test_append(self):
        from stdlib.io import write, append, read
        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False, mode='w') as f:
            path = f.name
        try:
            write(path, "line1\n")
            append(path, "line2\n")
            assert "line1" in read(path)
            assert "line2" in read(path)
        finally:
            os.unlink(path)

    def test_exists(self):
        from stdlib.io import exists
        assert exists(__file__) is True
        assert exists("/nonexistent_aura_xyz") is False

    def test_is_file(self):
        from stdlib.io import is_file
        assert is_file(__file__) is True

    def test_is_dir(self):
        from stdlib.io import is_dir
        assert is_dir(os.path.dirname(__file__)) is True

    def test_ls(self):
        from stdlib.io import ls
        entries = ls(os.path.dirname(__file__))
        assert isinstance(entries, list)
        assert len(entries) > 0

    def test_mkdir(self):
        from stdlib.io import mkdir, exists, rm
        d = tempfile.mkdtemp()
        sub = os.path.join(d, "aura_test_sub")
        mkdir(sub)
        assert exists(sub)
        os.rmdir(sub)
        os.rmdir(d)

    def test_basename(self):
        from stdlib.io import basename
        assert basename("/foo/bar.aura") == "bar.aura"

    def test_dirname(self):
        from stdlib.io import dirname
        assert dirname("/foo/bar.aura") == "/foo"

    def test_join(self):
        from stdlib.io import join
        result = join("/foo", "bar.aura")
        assert "foo" in result
        assert "bar.aura" in result

    def test_extension(self):
        from stdlib.io import extension
        assert extension("test.aura") == "aura"
        assert extension("test.py") == "py"

    def test_read_lines(self):
        from stdlib.io import write, read_lines
        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False, mode='w') as f:
            path = f.name
        try:
            write(path, "line1\nline2\nline3\n")
            lines = read_lines(path)
            assert lines == ["line1", "line2", "line3"]
        finally:
            os.unlink(path)

    def test_write_lines(self):
        from stdlib.io import write_lines, read
        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False, mode='w') as f:
            path = f.name
        try:
            write_lines(path, ["a", "b", "c"])
            content = read(path)
            assert "a\n" in content
            assert "c\n" in content
        finally:
            os.unlink(path)

    def test_size(self):
        from stdlib.io import write, size
        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False, mode='w') as f:
            path = f.name
        try:
            write(path, "hello")
            assert size(path) == 5
        finally:
            os.unlink(path)

    def test_touch(self):
        from stdlib.io import touch, exists, rm
        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False) as f:
            path = f.name
        os.unlink(path)
        touch(path)
        assert exists(path)
        os.unlink(path)

    def test_rm(self):
        from stdlib.io import write, rm, exists
        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False, mode='w') as f:
            path = f.name
        write(path, "delete me")
        rm(path)
        assert not exists(path)


class TestRegexStdlib:
    def test_find_all(self):
        from stdlib.regex import find_all
        assert find_all(r"\d+", "a1b22c333") == ["1", "22", "333"]

    def test_groups(self):
        from stdlib.regex import groups
        assert groups(r"(\w+)@(\w+)", "user@host") == ["user", "host"]
        assert groups(r"(\w+)@(\w+)", "no match") is None

    def test_replace_and_split(self):
        from stdlib.regex import replace, split
        assert replace(r"\s+", "_", "a b  c") == "a_b_c"
        assert split(r",\s*", "a, b,c") == ["a", "b", "c"]

    def test_search_and_match(self):
        from stdlib.regex import search, match, full_match
        assert search(r"\d+", "abc123").group() == "123"
        assert match(r"\d+", "abc123") is None
        assert full_match(r"\d+", "123") is not None

    def test_flags(self):
        from stdlib.regex import find_all, IGNORECASE
        assert find_all(r"a", "AaA", IGNORECASE) == ["A", "a", "A"]

    def test_escape(self):
        from stdlib.regex import escape
        assert escape("a.b") == r"a\.b"


class TestOsStdlib:
    def test_cwd_and_paths(self):
        from stdlib.os import cwd, path_join, path_splitext
        assert cwd()
        assert path_join("a", "b", "c.txt").endswith("c.txt")
        assert path_splitext("file.tar.gz") == ["file.tar", ".gz"]

    def test_env_roundtrip(self):
        from stdlib.os import get_env, set_env, unset_env
        set_env("AURA_TEST_VAR", "42")
        assert get_env("AURA_TEST_VAR") == "42"
        unset_env("AURA_TEST_VAR")
        assert get_env("AURA_TEST_VAR") is None
        assert get_env("AURA_TEST_MISSING", "d") == "d"

    def test_system_info(self):
        from stdlib.os import name, platform, pid, sep
        assert name() in ("posix", "nt", "java")
        assert platform()
        assert isinstance(pid(), int)
        assert sep() in ("/", "\\")


class TestHttpStdlib:
    def test_build_url_and_quote(self):
        from stdlib.http import build_url, quote, unquote
        assert build_url("http://x.com/s", {"q": "a", "n": 1}) == "http://x.com/s?q=a&n=1"
        assert build_url("http://x.com/s?a=1", {"b": 2}) == "http://x.com/s?a=1&b=2"
        assert build_url("http://x.com", None) == "http://x.com"
        assert unquote(quote("a b/c")) == "a b/c"

    def test_request_against_local_server(self):
        import http.server
        import json
        import threading

        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({"ok": True}).encode())

            def do_POST(self):
                length = int(self.headers.get('Content-Length', 0))
                body = self.rfile.read(length)
                self.send_response(201)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({"received": body.decode()}).encode())

        server = http.server.HTTPServer(('127.0.0.1', 0), Handler)
        port = server.server_address[1]
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            from stdlib.http import get, post, get_json, post_json
            base = f"http://127.0.0.1:{port}"

            response = get(base + "/hello")
            assert response.status == 200
            assert response.ok is True
            assert response["status"] == 200

            assert get_json(base + "/api").ok is True

            created = post(base + "/submit", data="payload")
            assert created.status == 201

            echoed = post_json(base + "/submit", {"a": 1})
            assert echoed["received"] == '{"a": 1}'
        finally:
            server.shutdown()
            server.server_close()

    def test_request_404_returns_response(self):
        import http.server
        import threading

        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                self.send_response(404)
                self.end_headers()
                self.wfile.write(b"nope")

        server = http.server.HTTPServer(('127.0.0.1', 0), Handler)
        port = server.server_address[1]
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            from stdlib.http import get
            response = get(f"http://127.0.0.1:{port}/missing")
            assert response.status == 404
            assert response.ok is False
        finally:
            server.shutdown()
            server.server_close()
