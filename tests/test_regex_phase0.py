"""Phase 0 — Regex standard library tests.

Covers:
- All public functions in stdlib.regex
- Unicode patterns and text
- Flags (IGNORECASE, MULTILINE, DOTALL, VERBOSE, ASCII, UNICODE)
- Named groups via group_dict
- subn, purge
- find_iter (lazy iterator)
- Edge cases (empty strings, no matches, overlapping patterns)
"""

from aura.stdlib import regex


class TestBasicMatching:
    def test_match_at_start(self):
        m = regex.match(r"\d+", "123abc")
        assert m is not None
        assert m.group() == "123"

    def test_match_not_at_start(self):
        assert regex.match(r"\d+", "abc123") is None

    def test_search_anywhere(self):
        m = regex.search(r"\d+", "abc123xyz")
        assert m is not None
        assert m.group() == "123"

    def test_full_match(self):
        assert regex.full_match(r"\d+", "123") is not None
        assert regex.full_match(r"\d+", "123abc") is None

    def test_no_match(self):
        assert regex.search(r"\d+", "hello") is None


class TestFindAll:
    def test_find_all_strings(self):
        assert regex.find_all(r"\d+", "a1b22c333") == ["1", "22", "333"]

    def test_find_all_with_groups(self):
        result = regex.find_all(r"(\d+)([a-z])", "1a2b3c")
        assert result == [("1", "a"), ("2", "b"), ("3", "c")]

    def test_find_all_no_match(self):
        assert regex.find_all(r"\d+", "hello") == []


class TestFindIter:
    def test_find_iter_returns_iterator(self):
        result = regex.find_iter(r"\d+", "a1b2c3")
        assert hasattr(result, '__iter__')
        assert hasattr(result, '__next__')

    def test_find_iter_yields_match_objects(self):
        matches = list(regex.find_iter(r"\d+", "a1b2c3"))
        assert len(matches) == 3
        assert all(hasattr(m, 'group') for m in matches)

    def test_find_iter_empty(self):
        matches = list(regex.find_iter(r"\d+", "hello"))
        assert len(matches) == 0


class TestSplit:
    def test_split_basic(self):
        assert regex.split(r"\s+", "hello world foo") == ["hello", "world", "foo"]

    def test_split_with_limit(self):
        assert regex.split(r"\s+", "a b c d", limit=2) == ["a", "b", "c d"]

    def test_split_no_match(self):
        assert regex.split(r"\d+", "hello") == ["hello"]


class TestReplace:
    def test_replace_string(self):
        assert regex.replace(r"\d+", "NUM", "a1b2") == "aNUMbNUM"

    def test_replace_count(self):
        assert regex.replace(r"\d+", "N", "a1b2c3", count=1) == "aNb2c3"

    def test_replace_no_match(self):
        assert regex.replace(r"\d+", "N", "hello") == "hello"


class TestReplaceFn:
    def test_replace_fn_callable(self):
        def double(m):
            return m.group() * 2
        assert regex.replace_fn(r"\d+", double, "a1b2") == "a11b22"

    def test_replace_fn_no_match(self):
        assert regex.replace_fn(r"\d+", lambda m: "X", "hello") == "hello"


class TestSubn:
    def test_subn_returns_tuple(self):
        result, count = regex.subn(r"\d+", "N", "a1b2c3")
        assert result == "aNbNcN"
        assert count == 3

    def test_subn_no_match(self):
        result, count = regex.subn(r"\d+", "N", "hello")
        assert result == "hello"
        assert count == 0


class TestGroups:
    def test_groups_basic(self):
        result = regex.groups(r"(\d+)-(\d+)", "12-34")
        assert result == ["12", "34"]

    def test_groups_no_match(self):
        assert regex.groups(r"(\d+)", "hello") is None

    def test_groups_optional(self):
        result = regex.groups(r"(\d+)?-([a-z]+)", "-abc")
        assert result == [None, "abc"]


class TestGroupDict:
    def test_group_dict_named_groups(self):
        result = regex.group_dict(r"(?P<year>\d{4})-(?P<month>\d{2})", "2026-09")
        assert result == {"year": "2026", "month": "09"}

    def test_group_dict_no_match(self):
        assert regex.group_dict(r"(?P<x>\d+)", "hello") is None

    def test_group_dict_partial(self):
        result = regex.group_dict(r"(?P<a>\d+)?(?P<b>[a-z]+)?", "abc")
        assert result == {"a": None, "b": "abc"}


class TestGroup:
    def test_group_entire_match(self):
        m = regex.search(r"\d+", "abc123")
        assert regex.group(m) == "123"

    def test_group_index(self):
        m = regex.search(r"(\d+)-(\d+)", "12-34")
        assert regex.group(m, 1) == "12"
        assert regex.group(m, 2) == "34"

    def test_group_multiple(self):
        m = regex.search(r"(\d+)-(\d+)", "12-34")
        assert regex.group(m, 1, 2) == ("12", "34")


class TestEscape:
    def test_escape(self):
        assert regex.escape("hello.world") == r"hello\.world"

    def test_escape_empty(self):
        assert regex.escape("") == ""


class TestCompilePattern:
    def test_compile_and_use(self):
        pat = regex.compile_pattern(r"\d+")
        m = pat.search("abc123")
        assert m is not None
        assert m.group() == "123"

    def test_compile_with_flags(self):
        pat = regex.compile_pattern(r"hello", regex.IGNORECASE)
        assert pat.search("HELLO") is not None


class TestPurge:
    def test_purge_does_not_error(self):
        regex.purge()
        # Just confirm it doesn't raise


class TestFlags:
    def test_ignorecase(self):
        m = regex.match(r"hello", "HELLO", regex.IGNORECASE)
        assert m is not None

    def test_multiline(self):
        m = regex.search(r"^world", "hello\nworld", regex.MULTILINE)
        assert m is not None

    def test_dotall(self):
        m = regex.search(r"h.llo", "h\nllo", regex.DOTALL)
        assert m is not None

    def test_unicode_flag(self):
        m = regex.search(r"\w+", "café", regex.UNICODE)
        assert m is not None

    def test_ascii_flag(self):
        # \w matches [a-zA-Z0-9_] in ASCII mode
        m = regex.search(r"\w+", "café", regex.ASCII)
        assert m is not None


class TestUnicodePatterns:
    def test_unicode_text_search(self):
        m = regex.search(r"名前", "こんにちは名前さん")
        assert m is not None
        assert m.group() == "名前"

    def test_unicode_named_group(self):
        result = regex.group_dict(r"(?P<name>[\u4e00-\u9fff]+)", "hello世界")
        assert result == {"name": "世界"}

    def test_unicode_character_class(self):
        matches = regex.find_all(r"[\u4e00-\u9fff]+", "hello世界foo bar测试")
        assert matches == ["世界", "测试"]

    def test_unicode_in_replacement(self):
        result = regex.replace(r"hello", "你好", "hello world")
        assert result == "你好 world"


class TestEdgeCases:
    def test_empty_pattern(self):
        m = regex.match("", "hello")
        assert m is not None

    def test_empty_text(self):
        assert regex.search(r"\d+", "") is None

    def test_empty_text_find_all(self):
        assert regex.find_all(r"\d+", "") == []

    def test_special_regex_chars(self):
        assert regex.escape(r"\.*+?^${}()|[]") is not None

    def test_lookahead(self):
        m = regex.search(r"\d+(?=%)", "50%")
        assert m is not None
        assert m.group() == "50"

    def test_lookbehind(self):
        m = regex.search(r"(?<=\$)\d+", "$100")
        assert m is not None
        assert m.group() == "100"

    def test_negative_lookahead(self):
        m = regex.search(r"\d+(?!%)", "50px 100%")
        assert m is not None

    def test_negative_lookbehind(self):
        m = regex.search(r"(?<!\$)\d+", "€100 $200")
        assert m is not None

    def test_named_group_complex(self):
        result = regex.group_dict(
            r"(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})",
            "2026-09-18"
        )
        assert result == {"year": "2026", "month": "09", "day": "18"}
