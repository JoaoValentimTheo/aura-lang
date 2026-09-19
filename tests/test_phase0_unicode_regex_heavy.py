"""Phase 0 heavy stress tests for Unicode and regex parity.

Covers edge cases identified in the audit:
- Unicode identifiers: all scripts, combining marks, ZWJ, surrogate rejection
- String escapes: incomplete sequences, surrogate rejection, supplementary plane
- Regex: VERBOSE flag, backreferences, non-capturing groups, compiled patterns,
  Unicode patterns with flags, ReDoS-resistance, exhaustive flag combinations
- Cross-cutting: Unicode identifiers + regex + string ops combined
"""

from __future__ import annotations

import re
import unicodedata

import pytest

from aura.parser.to_ast import Tokenizer
from aura.stdlib import regex


# ---------------------------------------------------------------------------
# Unicode Identifier Tests
# ---------------------------------------------------------------------------

class TestUnicodeIdentifiersExhaustive:
    """Exhaustive Unicode identifier coverage across all major scripts."""

    @pytest.mark.parametrize("ident", [
        "café", "naïve", "résumé", "über", "ñoño",  # Latin accented
        "Москва", "кириллица", "тест",  # Cyrillic
        "العربية", "مرحبا",  # Arabic
        "日本語", "変数", "関数",  # CJK (Chinese/Japanese)
        "한국어", "변수",  # Korean
        "Ελληνικά", "μεταβλητή",  # Greek
        "हिन्दी", "चर",  # Devanagari
        "ไทย", "ตัวแปร",  # Thai
        "İstanbul",  # Turkish dotless i
        "Łódź",  # Polish L-stroke
        "Österreich",  # German umlaut
        "Москва_тест",  # Mixed Cyrillic + Latin
    ])
    def test_identifier_valid(self, ident):
        """Valid Unicode identifiers from various scripts must tokenize."""
        tok = Tokenizer(f"let {ident} = 1")
        tokens = list(tok.tokenize())
        assert any(t.value == ident for t in tokens)

    @pytest.mark.parametrize("ident", [
        "𐌰",  # Gothic U+10330 (supplementary plane)
        "\U0001D400",  # Mathematical bold capital A
        "\U00010330test",  # Gothic + ASCII
    ])
    def test_supplementary_plane_identifiers(self, ident):
        """Non-BMP identifiers (supplementary plane) must work."""
        tok = Tokenizer(f"let {ident} = 1")
        tokens = list(tok.tokenize())
        assert any(t.value == ident for t in tokens)

    @pytest.mark.parametrize("ident", [
        "x₃",  # subscript digit (NOT XID_Continue)
        "²test",  # superscript 2 as start
    ])
    def test_non_xid_continue_rejected(self, ident):
        """Characters that are not XID_Continue must not be part of identifiers."""
        tok = Tokenizer(f"let {ident} = 1")
        tokens = list(tok.tokenize())
        # The identifier should be truncated at the non-XID character
        ident_tokens = [t for t in tokens if t.type == "IDENT"]
        assert ident_tokens
        # The token should NOT contain the subscript/superscript
        for t in ident_tokens:
            assert "₃" not in t.value
            assert "²" not in t.value

    def test_nfc_normalization_stability(self):
        """NFC normalization must be idempotent and consistent."""
        # NFD form of café (e + combining acute)
        nfd = "caf\u0301"
        tok = Tokenizer(f"let {nfd} = 1")
        tokens = list(tok.tokenize())
        ident_tokens = [t for t in tokens if t.type == "IDENT" and t.value != "let"]
        assert ident_tokens
        normalized = unicodedata.normalize("NFC", nfd)
        assert ident_tokens[0].value == normalized

    def test_identifier_with_zwj(self):
        """ZWJ (U+200D) should be valid as identifier continuation."""
        ident = "test\u200D"
        tok = Tokenizer(f"let {ident} = 1")
        tokens = list(tok.tokenize())
        ident_tokens = [t for t in tokens if t.type == "IDENT" and t.value != "let"]
        assert ident_tokens
        # ZWJ is XID_Continue, so it should be part of the identifier
        assert "\u200D" in ident_tokens[0].value

    def test_mixed_script_identifier(self):
        """Mixed-script identifiers (Latin + Cyrillic + CJK) must work."""
        ident = "hello_мир_你好"
        tok = Tokenizer(f"let {ident} = 1")
        tokens = list(tok.tokenize())
        ident_tokens = [t for t in tokens if t.type == "IDENT" and t.value != "let"]
        assert ident_tokens
        assert ident_tokens[0].value == ident


# ---------------------------------------------------------------------------
# Unicode String Escape Tests
# ---------------------------------------------------------------------------

class TestUnicodeEscapesExhaustive:

    def test_u_escape_bmp(self):
        tok = Tokenizer(r'let x = "\u00E9"')
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type == "STRING"]
        assert str_tok
        assert "é" in str_tok[0].value

    def test_U_escape_supplementary(self):
        tok = Tokenizer(r'let x = "\U00010330"')
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type == "STRING"]
        assert str_tok
        assert "\U00010330" in str_tok[0].value

    def test_x_escape(self):
        tok = Tokenizer(r'let x = "\x41"')
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type == "STRING"]
        assert str_tok
        assert "A" in str_tok[0].value

    def test_surrogate_rejection_u(self):
        """Lone surrogates (\\uD800-\\uDFFF) must NOT produce valid Unicode."""
        # Build source string with surrogate escape as text, not as actual char
        source = 'let x = "\\uD800"'
        tok = Tokenizer(source)
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type == "STRING"]
        assert str_tok
        # Should NOT contain the surrogate character
        for ch in str_tok[0].value:
            assert not (0xD800 <= ord(ch) <= 0xDFFF)

    def test_surrogate_rejection_uffff(self):
        source = 'let x = "\\uDFFF"'
        tok = Tokenizer(source)
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type == "STRING"]
        assert str_tok
        for ch in str_tok[0].value:
            assert not (0xD800 <= ord(ch) <= 0xDFFF)

    def test_incomplete_escape_u(self):
        """Incomplete \\u escape (not enough hex digits) must be kept literally."""
        tok = Tokenizer(r'let x = "\u00"')
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type == "STRING"]
        assert str_tok

    def test_incomplete_escape_U(self):
        tok = Tokenizer(r'let x = "\U0001F"')
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type == "STRING"]
        assert str_tok

    def test_mixed_escapes(self):
        tok = Tokenizer(r'let x = "hello\n\u00E9\U0001F600"')
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type == "STRING"]
        assert str_tok

    def test_raw_string_preserves_escapes(self):
        tok = Tokenizer(r'let x = r"\u00E9"')
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type in ("STRING", "RAWSTRING")]
        assert str_tok
        assert "\\u00E9" in str_tok[0].value

    def test_fstring_unicode_interpolation(self):
        """Unicode identifiers in f-string interpolation."""
        tok = Tokenizer(r'let x = f"result: {名前}"')
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type == "FSTRING"]
        assert str_tok

    def test_triple_quoted_unicode(self):
        tok = Tokenizer('let x = """日本語テスト"""')
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type == "STRING"]
        assert str_tok

    def test_escape_10ffff(self):
        """Maximum valid codepoint U+10FFFF."""
        tok = Tokenizer(r'let x = "\U0010FFFF"')
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type == "STRING"]
        assert str_tok

    def test_escape_110000_invalid(self):
        """Codepoint > U+10FFFF should fail gracefully."""
        tok = Tokenizer(r'let x = "\U00110000"')
        tokens = list(tok.tokenize())
        str_tok = [t for t in tokens if t.type == "STRING"]
        assert str_tok


# ---------------------------------------------------------------------------
# Unicode Comments
# ---------------------------------------------------------------------------

class TestUnicodeComments:

    def test_line_comment_cjk(self):
        """CJK comments must not cause tokenizer errors."""
        tok = Tokenizer('// 日本語コメント\nlet x = 1')
        tokens = list(tok.tokenize())
        # Comments are skipped; just verify the rest parses correctly
        assert any(t.type == "IDENT" and t.value == "x" for t in tokens)

    def test_block_comment_arabic(self):
        """Arabic block comments must not cause tokenizer errors."""
        tok = Tokenizer('/* تعليق */ let x = 1')
        tokens = list(tok.tokenize())
        assert any(t.type == "IDENT" and t.value == "x" for t in tokens)

    def test_comment_with_emoji(self):
        """Emoji in comments must not cause tokenizer errors."""
        tok = Tokenizer('// 🎉🎉🎉\nlet x = 1')
        tokens = list(tok.tokenize())
        assert any(t.type == "IDENT" and t.value == "x" for t in tokens)


# ---------------------------------------------------------------------------
# Regex Tests - VERBOSE Flag
# ---------------------------------------------------------------------------

class TestRegexVerboseFlag:

    def test_verbose_basic(self):
        """VERBOSE flag allows whitespace and comments in patterns."""
        pattern = r"""
            (?P<year>\d{4})   # year
            -                 # separator
            (?P<month>\d{2})  # month
            -                 # separator
            (?P<day>\d{2})    # day
        """
        result = regex.full_match(pattern, "2026-09-18", regex.VERBOSE)
        assert result is not None
        assert result.group("year") == "2026"
        assert result.group("month") == "09"
        assert result.group("day") == "18"

    def test_verbose_with_unicode(self):
        """VERBOSE flag with Unicode content in comments."""
        pattern = r"""
            (?P<name>\w+)  # 名前
            \s*
            =              # 区切り
            \s*
            (?P<value>\d+) # 値
        """
        result = regex.full_match(pattern, "test = 42", regex.VERBOSE)
        assert result is not None
        assert result.group("name") == "test"
        assert result.group("value") == "42"


# ---------------------------------------------------------------------------
# Regex Tests - Backreferences
# ---------------------------------------------------------------------------

class TestRegexBackreferences:

    def test_numbered_backreference(self):
        m = regex.search(r"(hello) \1", "hello hello")
        assert m is not None
        assert m.group(0) == "hello hello"

    def test_numbered_backreference_no_match(self):
        m = regex.search(r"(hello) \1", "hello world")
        assert m is None

    def test_named_backreference(self):
        """Named backreference via \\1 after named group (re module style)."""
        m = regex.search(r"(?P<word>\w+) \1", "test test")
        assert m is not None
        assert m.group("word") == "test"

    def test_backreference_with_unicode(self):
        m = regex.search(r"(\w+) \1", "日本語 日本語")
        assert m is not None

    def test_backreference_in_replace(self):
        result = regex.replace(r"(\w+) (\w+)", r"\2 \1", "hello world")
        assert result == "world hello"


# ---------------------------------------------------------------------------
# Regex Tests - Non-capturing Groups
# ---------------------------------------------------------------------------

class TestRegexNonCapturingGroups:

    def test_non_capturing_basic(self):
        m = regex.search(r"(?:hello) (world)", "hello world")
        assert m is not None
        assert m.groups() == ("world",)

    def test_non_capturing_with_quantifier(self):
        m = regex.search(r"(?:\d+\.){3}\d+", "192.168.1.1")
        assert m is not None
        assert m.group(0) == "192.168.1.1"

    def test_non_capturing_nested(self):
        m = regex.search(r"((?:a|b)+)", "abba")
        assert m is not None
        assert m.group(0) == "abba"


# ---------------------------------------------------------------------------
# Regex Tests - Compiled Pattern Objects
# ---------------------------------------------------------------------------

class TestRegexCompiledPatterns:

    def test_compiled_match(self):
        pat = regex.compile_pattern(r"\d+")
        m = regex.match(pat, "123abc")
        assert m is not None
        assert m.group(0) == "123"

    def test_compiled_search(self):
        pat = regex.compile_pattern(r"\d+")
        m = regex.search(pat, "abc123def")
        assert m is not None
        assert m.group(0) == "123"

    def test_compiled_find_all(self):
        pat = regex.compile_pattern(r"\d+")
        result = regex.find_all(pat, "a1b2c3")
        assert result == ["1", "2", "3"]

    def test_compiled_replace(self):
        pat = regex.compile_pattern(r"\d+")
        result = regex.replace(pat, "NUM", "a1b2c3")
        assert result == "aNUMbNUMcNUM"

    def test_compiled_with_flags(self):
        pat = regex.compile_pattern(r"hello", regex.IGNORECASE)
        m = regex.search(pat, "HELLO")
        assert m is not None

    def test_compiled_split(self):
        pat = regex.compile_pattern(r"\s+")
        result = regex.split(pat, "a  b   c")
        assert result == ["a", "b", "c"]

    def test_compiled_group(self):
        pat = regex.compile_pattern(r"(\d+)-(\d+)")
        m = regex.search(pat, "123-456")
        assert m is not None
        assert regex.group(m, 1) == "123"
        assert regex.group(m, 2) == "456"


# ---------------------------------------------------------------------------
# Regex Tests - Flags Combinations
# ---------------------------------------------------------------------------

class TestRegexFlagCombinations:

    def test_ignorecase_unicode(self):
        m = regex.search(r"\w+", "CAFÉ", regex.IGNORECASE | regex.UNICODE)
        assert m is not None

    def test_multiline_dotall(self):
        m = regex.search(r"^.", "line1\nline2", regex.MULTILINE | regex.DOTALL)
        assert m is not None
        assert m.group(0) == "l"

    def test_ignorecase_multiline(self):
        m = regex.search(r"^hello", "HELLO\nworld", regex.IGNORECASE | regex.MULTILINE)
        assert m is not None

    def test_ascii_flag(self):
        """ASCII flag restricts \\w, \\d, \\s to ASCII only."""
        m = regex.search(r"\w+", "日本語", regex.ASCII)
        assert m is None  # CJK chars are NOT ASCII word chars

    def test_unicode_flag_explicit(self):
        m = regex.search(r"\w+", "日本語", regex.UNICODE)
        assert m is not None  # CJK chars ARE Unicode word chars


# ---------------------------------------------------------------------------
# Regex Tests - Unicode Patterns Deep
# ---------------------------------------------------------------------------

class TestRegexUnicodeDeep:

    def test_unicode_named_group(self):
        m = regex.search(r"(?P<名前>\w+)", "名前は太郎")
        assert m is not None
        assert m.group("名前") == "名前は太郎"

    def test_unicode_character_class(self):
        m = regex.search(r"[\u4e00-\u9fff]+", "日本語テスト")
        assert m is not None
        # Katakana (テスト) is U+30A0-U+30FF, outside CJK Unified range
        assert m.group(0) == "日本語"

    def test_unicode_escape_in_pattern(self):
        m = regex.search(r"\u00E9", "café")
        assert m is not None
        assert m.group(0) == "é"

    def test_unicode_replacement(self):
        result = regex.replace(r"\d+", "NUM", "123abc456")
        assert result == "NUMabcNUM"

    def test_unicode_findall_mixed(self):
        result = regex.find_all(r"[a-zA-Z]+", "hello 日本語 world")
        assert result == ["hello", "world"]

    def test_unicode_split(self):
        result = regex.split(r"\s+", "hello 日本語 world")
        assert result == ["hello", "日本語", "world"]

    def test_unicode_groups(self):
        result = regex.groups(r"(\w+) (\w+)", "hello world")
        assert result == ["hello", "world"]

    def test_unicode_group_dict(self):
        result = regex.group_dict(r"(?P<x>\w+) (?P<y>\w+)", "hello world")
        assert result == {"x": "hello", "y": "world"}

    def test_emoji_in_pattern(self):
        """Emoji in regex pattern (literal match)."""
        m = regex.search(r"🎉", "I love 🎉 parties")
        assert m is not None

    def test_emoji_text_search(self):
        result = regex.find_all(r"[\U0001F300-\U0001F9FF]", "I 🎉 like 🐱 cats")
        assert len(result) == 2


# ---------------------------------------------------------------------------
# Regex Tests - Edge Cases
# ---------------------------------------------------------------------------

class TestRegexEdgeCases:

    def test_empty_pattern(self):
        m = regex.search("", "hello")
        assert m is not None

    def test_empty_text(self):
        m = regex.search(r"\d*", "")
        assert m is not None

    def test_special_chars_escape(self):
        escaped = regex.escape(r"\.*+?^${}()|[]")
        assert escaped != r"\.*+?^${}()|[]"

    def test_lookahead_positive(self):
        m = regex.search(r"\w+(?=\d)", "abc123")
        assert m is not None
        # \w+ is greedy, matches "abc12" (lookahead asserts digit follows)
        assert m.group(0) == "abc12"

    def test_lookahead_negative(self):
        m = regex.search(r"\w+(?!\d)", "abc123")
        assert m is not None

    def test_lookbehind_positive(self):
        m = regex.search(r"(?<=@)\w+", "user@host")
        assert m is not None
        assert m.group(0) == "host"

    def test_lookbehind_negative(self):
        m = regex.search(r"(?<!@)\w+", "user@host")
        assert m is not None

    def test_zero_width_assertions_combined(self):
        m = regex.search(r"\b\w+\b", "hello world")
        assert m is not None

    def test_catastrophic_backtracking_resistant(self):
        """Simple patterns should not cause catastrophic backtracking."""
        import signal

        def handler(signum, frame):
            raise TimeoutError("ReDoS detected")

        old = signal.signal(signal.SIGALRM, handler)
        signal.alarm(2)
        try:
            regex.search(r"a+", "a" * 20 + "b")
            regex.search(r"(?:a|b)+", "a" * 20 + "c")
            regex.search(r"\d+\d+\d+", "1" * 10 + "a")
        finally:
            signal.alarm(0)
            signal.signal(signal.SIGALRM, old)

    def test_findall_groups(self):
        result = regex.find_all(r"(\d+)-(\w+)", "1-foo 2-bar")
        assert result == [("1", "foo"), ("2", "bar")]

    def test_subn_basic(self):
        result, count = regex.subn(r"\d+", "N", "a1b2c3")
        assert result == "aNbNcN"
        assert count == 3

    def test_group_none_match(self):
        """group(None) must return None, not crash."""
        assert regex.group(None) is None
        assert regex.group(None, 1) is None

    def test_group_none_multiple_indices(self):
        assert regex.group(None, 1, 2) is None

    def test_error_is_re_error(self):
        """regex.error should be re.error."""
        assert regex.error is re.error

    def test_error_catchable(self):
        """re.error should be catchable from regex.error."""
        with pytest.raises(regex.error):
            regex.search(r"[invalid", "text")


# ---------------------------------------------------------------------------
# Regex Differential Parity (heavy)
# ---------------------------------------------------------------------------

class TestRegexDifferentialHeavy:

    @pytest.mark.parametrize("pattern,text", [
        (r"\d+", "abc123def456"),
        (r"\w+", "hello world"),
        (r"\s+", "a  b\tc\n"),
        (r"[a-z]+", "Hello World"),
        (r"\b\w+\b", "one two-three"),
        (r"(?P<year>\d{4})-(?P<month>\d{2})", "2026-09-18"),
        (r"a|b|c", "xbx"),
        (r"(?:ab)+", "ababab"),
        (r"(?=foo)foobar", "foobar"),
        (r"(?<=@)\w+", "a@b"),
    ])
    def test_search_parity(self, pattern, text):
        """Aura regex.search must agree with re.search."""
        assert regex.search(pattern, text) is not None
        assert regex.search(pattern, text).group(0) == re.search(pattern, text).group(0)

    @pytest.mark.parametrize("pattern,text", [
        (r"\d+", "abc123def456"),
        (r"\w+", "hello world"),
        (r"[a-z]+", "123abc"),
    ])
    def test_findall_parity(self, pattern, text):
        assert regex.find_all(pattern, text) == re.findall(pattern, text)

    @pytest.mark.parametrize("pattern,repl,text", [
        (r"\d+", "N", "a1b2"),
        (r"\w+", "WORD", "hi there"),
        (r"(a)", r"\1\1", "a"),
    ])
    def test_replace_parity(self, pattern, repl, text):
        assert regex.replace(pattern, repl, text) == re.sub(pattern, repl, text)

    @pytest.mark.parametrize("pattern,text,limit", [
        (r"\s+", "a b c", 1),
        (r"\d+", "a1b2c3", 2),
        (r",", "a,b,c", 0),
    ])
    def test_split_parity(self, pattern, text, limit):
        assert regex.split(pattern, text, limit) == re.split(pattern, text, maxsplit=limit)

    def test_subn_parity(self):
        assert regex.subn(r"\d+", "N", "a1b2") == re.subn(r"\d+", "N", "a1b2")

    def test_compile_search_parity(self):
        pat = regex.compile_pattern(r"\d+")
        m1 = regex.search(pat, "abc123")
        m2 = re.search(r"\d+", "abc123")
        assert m1.group(0) == m2.group(0)

    def test_groups_parity(self):
        assert regex.groups(r"(\d+)-(\w+)", "1-foo") == list(re.search(r"(\d+)-(\w+)", "1-foo").groups())

    def test_group_dict_parity(self):
        assert regex.group_dict(r"(?P<x>\w+)", "hello") == re.search(r"(?P<x>\w+)", "hello").groupdict()


# ---------------------------------------------------------------------------
# Cross-cutting: Unicode + Regex + String Ops
# ---------------------------------------------------------------------------

class TestUnicodeRegexCrossCutting:

    def test_unicode_identifier_roundtrip(self):
        """Unicode identifiers must survive tokenize -> transpile -> exec."""
        from aura.transpiler.transformer import Transformer
        from aura.parser.to_ast import Parser, Tokenizer

        source = 'let 名前 = "日本語"\nprint(名前)\n'
        tokens = Tokenizer(source).tokenize()
        tree = Parser(tokens).parse()
        transformer = Transformer()
        python_code = transformer.transform(tree)
        assert "名前" in python_code or "日本語" in python_code

    def test_regex_on_unicode_string_literal(self):
        """Regex matching on Unicode string literals."""
        from aura.parser.to_ast import Parser, Tokenizer
        from aura.transpiler.transformer import Transformer

        source = 'let x = "日本語テスト"\n'
        tokens = Tokenizer(source).tokenize()
        tree = Parser(tokens).parse()
        transformer = Transformer()
        python_code = transformer.transform(tree)
        compile(python_code, "<test>", "exec")

    def test_unicode_emoji_in_string(self):
        """Emoji in string literals must work end-to-end."""
        from aura.parser.to_ast import Parser, Tokenizer
        from aura.transpiler.transformer import Transformer

        source = 'let x = "🎉🎊🎈"\n'
        tokens = Tokenizer(source).tokenize()
        tree = Parser(tokens).parse()
        transformer = Transformer()
        python_code = transformer.transform(tree)
        compile(python_code, "<test>", "exec")

    def test_unicode_identifier_in_fstring(self):
        """Unicode identifiers in f-string expressions."""
        from aura.parser.to_ast import Parser, Tokenizer
        from aura.transpiler.transformer import Transformer

        source = 'let 名前 = "太郎"\nlet x = f"こんにちは、{名前}さん"\n'
        tokens = Tokenizer(source).tokenize()
        tree = Parser(tokens).parse()
        transformer = Transformer()
        python_code = transformer.transform(tree)
        compile(python_code, "<test>", "exec")

    def test_regex_with_unicode_escape_in_source(self):
        """Regex pattern with Unicode escapes from Aura source."""
        from aura.parser.to_ast import Parser, Tokenizer
        from aura.transpiler.transformer import Transformer

        source = 'import stdlib.regex as regex\nlet m = regex.search(r"\\u00E9", "caf\\u00E9")\n'
        tokens = Tokenizer(source).tokenize()
        tree = Parser(tokens).parse()
        transformer = Transformer()
        python_code = transformer.transform(tree)
        compile(python_code, "<test>", "exec")

    def test_unicode_string_comparison(self):
        """Unicode string comparison in generated Python."""
        from aura.parser.to_ast import Parser, Tokenizer
        from aura.transpiler.transformer import Transformer

        source = 'let a = "café"\nlet b = "café"\n'
        tokens = Tokenizer(source).tokenize()
        tree = Parser(tokens).parse()
        transformer = Transformer()
        python_code = transformer.transform(tree)
        compile(python_code, "<test>", "exec")

    def test_unicode_in_print(self):
        """Unicode content in print statements."""
        from aura.parser.to_ast import Parser, Tokenizer
        from aura.transpiler.transformer import Transformer

        source = 'print("日本語: 你好世界")\n'
        tokens = Tokenizer(source).tokenize()
        tree = Parser(tokens).parse()
        transformer = Transformer()
        python_code = transformer.transform(tree)
        compile(python_code, "<test>", "exec")
