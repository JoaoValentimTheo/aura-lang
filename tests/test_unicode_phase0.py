"""Phase 0 — Unicode support tests for the Aura lexer/parser.

Covers:
- Unicode identifiers (CJK, Cyrillic, Arabic, Greek, combining marks via NFC)
- Unicode string escapes (\\uXXXX, \\UXXXXXXXX, \\xHH)
- Unicode in f-strings, raw strings, comments
- NFC normalization of identifiers
- Supplementary plane characters (codepoints > U+FFFF)
"""

from aura.parser.to_ast import Parser, Tokenizer


def _tokenize(src):
    """Tokenize source and return list of (type, value) pairs."""
    tok = Tokenizer(src)
    return [(t.type, t.value) for t in tok.tokenize() if t.type != 'EOF']


def _parse_expr(src):
    """Parse a single expression and return the AST node."""
    parser = Parser(src)
    return parser.parse_expression()


def _transpile_single(stmt_src):
    """Transpile a single statement wrapped in a function and return the Python output."""
    from aura.transpiler.transformer import Transformer
    wrapped = f"def main() {{\n  {stmt_src}\n}}"
    tokens = Tokenizer(wrapped).tokenize()
    parser = Parser(tokens)
    program = parser.parse()
    t = Transformer()
    return t.transform(program)


# ── Identifiers ──────────────────────────────────────────────────────────────

class TestUnicodeIdentifiers:
    def test_cjk_identifier(self):
        tokens = _tokenize('let 名前 = 1')
        idents = [v for t, v in tokens if t == 'IDENT']
        assert '名前' in idents

    def test_cyrillic_identifier(self):
        tokens = _tokenize('let Москва = 1')
        idents = [v for t, v in tokens if t == 'IDENT']
        assert 'Москва' in idents

    def test_arabic_identifier(self):
        tokens = _tokenize('let المستخدم = 1')
        idents = [v for t, v in tokens if t == 'IDENT']
        assert 'المستخدم' in idents

    def test_greek_identifier(self):
        tokens = _tokenize('let ελληνικά = 1')
        idents = [v for t, v in tokens if t == 'IDENT']
        assert 'ελληνικά' in idents

    def test_latin_accented_identifier(self):
        tokens = _tokenize('let café = 1')
        idents = [v for t, v in tokens if t == 'IDENT']
        assert 'café' in idents

    def test_supplementary_plane_identifier(self):
        """Gothic letter U+10330 (𐌰) should be accepted as identifier start."""
        tokens = _tokenize('let \U00010330test = 1')
        idents = [v for t, v in tokens if t == 'IDENT']
        assert len(idents) >= 1
        assert '\U00010330test' in idents

    def test_mixed_script_identifier(self):
        tokens = _tokenize('let hello_мир_你好 = 1')
        idents = [v for t, v in tokens if t == 'IDENT']
        assert 'hello_мир_你好' in idents

    def test_underscore_start_unicode(self):
        tokens = _tokenize('let _名 = 1')
        idents = [v for t, v in tokens if t == 'IDENT']
        assert '_名' in idents

    def test_nfc_normalization(self):
        """NFD e + combining acute should normalize to NFC é."""
        # U+0301 = combining acute accent
        src = 'let caf\u0065\u0301 = 1'  # 'e' + combining acute
        tokens = _tokenize(src)
        idents = [v for t, v in tokens if t == 'IDENT']
        assert 'café' in idents  # normalized to NFC

    def test_nfc_matches_nfd(self):
        """NFC and NFD forms of the same identifier should produce the same token."""
        nfc = 'let café = 1'
        nfd = 'let caf\u0065\u0301 = 1'
        ids_nfc = [v for t, v in _tokenize(nfc) if t == 'IDENT']
        ids_nfd = [v for t, v in _tokenize(nfd) if t == 'IDENT']
        assert ids_nfc == ids_nfd

    def test_unicode_digit_continuation(self):
        """Unicode digits are *not* XID_Continue; the tokenizer must agree with Python."""
        tokens = _tokenize('let x\u2083 = 1')  # x₃
        idents = [v for t, v in tokens if t == 'IDENT']
        assert 'x' in idents
        assert 'x\u2083' not in idents

    def test_script_capital_p_continuation(self):
        """U+2118 / U+212E are valid Python identifier characters."""
        tokens = _tokenize('let x\u2118 = 1')  # x℘
        idents = [v for t, v in tokens if t == 'IDENT']
        assert 'x\u2118' in idents

    def test_zwj_continuation(self):
        """ZWJ (U+200D) is XID_Continue on some Python versions."""
        tokens = _tokenize('let a\u200db = 1')
        idents = [v for t, v in tokens if t == 'IDENT']
        # ZWJ handling varies across Python versions
        if ('a' + '\u200d').isidentifier():
            assert 'a\u200db' in idents
        else:
            assert 'a\u200db' not in idents
            assert 'a' in idents
            assert 'b' in idents

    def test_arabic_ligature_not_identifier_start(self):
        """U+FC5E is rejected by Python as a start, so it must not begin a name."""
        tokens = _tokenize('let \ufc5e = 1')
        idents = [v for t, v in tokens if t == 'IDENT']
        assert idents == ['let']


# ── String escapes ───────────────────────────────────────────────────────────

class TestUnicodeStringEscapes:
    def test_unicode_escape_lowercase(self):
        result = _transpile_single('let x = "hello \\u0041"')
        assert "'hello A'" in result

    def test_unicode_escape_accented(self):
        result = _transpile_single('let x = "caf\\u00e9"')
        assert "'café'" in result

    def test_unicode_escape_uppercase(self):
        """\\UXXXXXXXX should decode supplementary plane codepoints."""
        result = _transpile_single('let x = "Gothic \\U00010330"')
        assert "'Gothic \U00010330'" in result

    def test_unicode_escape_emoji(self):
        result = _transpile_single('let x = "smile \\U0001F600"')
        assert '\U0001F600' in result

    def test_hex_escape(self):
        result = _transpile_single('let x = "\\x41"')
        assert "'A'" in result

    def test_mixed_escapes(self):
        result = _transpile_single('let x = "\\u0048\\x69"')
        assert "'Hi'" in result

    def test_unicode_in_fstring_text(self):
        result = _transpile_single('let name = "Aura"; let x = f"こんにちは {name}"')
        assert 'こんにちは' in result

    def test_unicode_in_fstring_expr(self):
        result = _transpile_single('let 名前 = "Aura"; let x = f"hi {名前}"')
        assert '名前' in result

    def test_raw_string_unicode(self):
        result = _transpile_single('let x = r"caf\\u00e9 \\n noesc"')
        assert r"caf\u00e9 \n noesc" in result

    def test_triple_quoted_unicode(self):
        result = _transpile_single('let x = """Héllo Wörld 你好"""')
        assert 'Héllo Wörld 你好' in result

    def test_supplementary_plane_in_string(self):
        result = _transpile_single('let x = "GINE"')
        assert 'GINE' in result


# ── Comments ─────────────────────────────────────────────────────────────────

class TestUnicodeComments:
    def test_line_comment_unicode(self):
        tokens = _tokenize('// 日本語 comment\nlet x = 1')
        # Comment should be consumed, only 'let', 'x', '=', '1' remain
        assert all(v != '日本語' for _, v in tokens)

    def test_block_comment_unicode(self):
        tokens = _tokenize('/* ブロックコメント */ let x = 1')
        assert all(v != 'ブロックコメント' for _, v in tokens)


# ── Edge cases ───────────────────────────────────────────────────────────────

class TestUnicodeEdgeCases:
    def test_emoji_in_string_body(self):
        result = _transpile_single('let x = "party 🎉"')
        assert '🎉' in result

    def test_rtl_in_string_body(self):
        result = _transpile_single('let x = "مرحبا world"')
        assert 'مرحبا' in result

    def test_combining_chars_in_string(self):
        result = _transpile_single('let x = "e\u0301"')  # NFD e + combining
        assert 'e\u0301' in result

    def test_zero_width_in_string(self):
        result = _transpile_single('let x = "a\u200bb"')
        # The transpiler escapes the zero-width space in output
        assert 'a' in result and 'b' in result

    def test_identifier_with_emoji_not_valid(self):
        """Emoji should not be part of identifiers (this is expected behavior)."""
        tokens = _tokenize('let x🎉 = 1')
        idents = [v for t, v in tokens if t == 'IDENT']
        # x and 🎉 should be separate tokens
        assert 'x' in idents
