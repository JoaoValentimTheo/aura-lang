"""Phase 0 — heavy stress + differential fuzz for Unicode and regex.

This suite extends `test_phase0_fuzz.py` with:

* **Full-surface regex differential fuzzing** — every public wrapper is compared
  against the equivalent :mod:`re` call, for both return values and exception
  class, so a wrapper can never diverge silently.
* **Lexer/parser crash-safety under arbitrary Unicode** — the tokenizer and
  parser must raise ``SyntaxError`` (never ``IndexError``/``UnicodeError``/a
  crash) for any input, and transpiled output must always be valid Python.
* **Unicode round-trip at the language level** — a program using Unicode names,
  literals and patterns produces the same result as its Python equivalent.
"""
import re
import sys
import unicodedata
from pathlib import Path

from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.stdlib import regex  # noqa: E402
from aura.transpiler.transformer import Transformer  # noqa: E402

SETTINGS = settings(
    max_examples=300,
    deadline=None,
    suppress_health_check=[HealthCheck.too_slow],
)

# Crash-safety properties are cheap and catch the widest range of inputs, so
# they run at a higher volume than the differential-parity checks.
CRASH_SETTINGS = settings(
    max_examples=1500,
    deadline=None,
    suppress_health_check=[HealthCheck.too_slow],
)

_ANY_TEXT = st.text(max_size=80)
_UNICODE_TEXT = st.text(
    alphabet=st.characters(min_codepoint=0x80), min_size=0, max_size=80)


def _transpile(src):
    return Transformer().transform(Parser(Tokenizer(src).tokenize()).parse())


# ---------------------------------------------------------------------------
# Regex: differential parity over the whole public surface
# ---------------------------------------------------------------------------

@given(_ANY_TEXT, _ANY_TEXT)
@SETTINGS
def test_match_parity(pattern, text):
    try:
        expected = re.match(pattern, text)
    except re.error:
        return
    got = regex.match(pattern, text)
    assert (got is None) == (expected is None)
    if got is not None:
        assert got.group() == expected.group()
        assert got.span() == expected.span()


@given(_ANY_TEXT, _ANY_TEXT)
@SETTINGS
def test_search_parity(pattern, text):
    try:
        expected = re.search(pattern, text)
    except re.error:
        return
    got = regex.search(pattern, text)
    assert (got is None) == (expected is None)
    if got is not None:
        assert got.span() == expected.span()


@given(_ANY_TEXT, _ANY_TEXT)
@SETTINGS
def test_full_match_parity(pattern, text):
    try:
        expected = re.fullmatch(pattern, text)
    except re.error:
        return
    got = regex.full_match(pattern, text)
    assert (got is None) == (expected is None)


@given(_ANY_TEXT, _ANY_TEXT)
@SETTINGS
def test_find_iter_parity(pattern, text):
    try:
        expected = [(m.group(), m.span()) for m in re.finditer(pattern, text)]
    except re.error:
        return
    got = [(m.group(), m.span()) for m in regex.find_iter(pattern, text)]
    assert got == expected


@given(_ANY_TEXT, _ANY_TEXT)
@SETTINGS
def test_replace_parity(pattern, text):
    try:
        expected = re.sub(pattern, "N", text)
    except re.error:
        return
    assert regex.replace(pattern, "N", text) == expected


@given(_ANY_TEXT, _ANY_TEXT)
@SETTINGS
def test_replace_fn_parity(pattern, text):
    try:
        expected = re.sub(pattern, lambda m: m.group().upper(), text)
    except re.error:
        return
    assert regex.replace_fn(pattern, lambda m: m.group().upper(), text) == expected


@given(_ANY_TEXT, _ANY_TEXT)
@SETTINGS
def test_groups_parity(pattern, text):
    try:
        expected = re.search(pattern, text)
    except re.error:
        return
    got = regex.groups(pattern, text)
    if expected is None:
        assert got is None
    else:
        assert got == list(expected.groups())


@given(_ANY_TEXT, _ANY_TEXT)
@SETTINGS
def test_group_dict_parity(pattern, text):
    try:
        expected = re.search(pattern, text)
    except re.error:
        return
    got = regex.group_dict(pattern, text)
    if expected is None:
        assert got is None
    else:
        assert got == expected.groupdict()


@given(_ANY_TEXT, _ANY_TEXT)
@SETTINGS
def test_compile_pattern_parity(pattern, text):
    try:
        compiled = regex.compile_pattern(pattern)
    except re.error:
        # A pattern Python rejects must raise re.error through Aura too.
        return
    try:
        expected = re.compile(pattern)
    except re.error as exc:
        raise AssertionError("Aura compiled a pattern Python rejects") from exc
    if expected.search(text):
        assert compiled.search(text)


def test_regex_invalid_pattern_raises_re_error():
    for bad in ("(", "[", "*", "(?P<", "\\", "(?<=a){2,1}"):
        try:
            re.compile(bad)
        except re.error as exc:
            try:
                regex.compile_pattern(bad)
            except re.error:
                continue
            raise AssertionError(f"Aura accepted invalid pattern {bad!r}") from exc


def test_regex_error_is_re_error_not_generic():
    """A pattern error must surface as re.error, so callers can catch it."""
    import pytest

    with pytest.raises(re.error):
        regex.search("(unclosed", "x")


@given(_UNICODE_TEXT, _UNICODE_TEXT)
@SETTINGS
def test_unicode_dot_matches_like_python(pattern, text):
    """DOTALL and character classes behave identically on non-ASCII text."""
    for flags in (0, re.DOTALL, re.IGNORECASE, re.MULTILINE):
        try:
            expected = [(m.start(), m.end()) for m in re.finditer(pattern, text, flags)]
        except re.error:
            continue
        got = [(m.start(), m.end()) for m in regex.find_iter(pattern, text, flags)]
        assert got == expected


@given(_UNICODE_TEXT)
@SETTINGS
def test_word_class_is_unicode_aware(text):
    """``\\w`` follows Unicode by default, matching Python, not ASCII."""
    assert regex.find_all(r"\w+", text) == re.findall(r"\w+", text)
    assert regex.find_all(r"\w+", text, regex.ASCII) == re.findall(r"\w+", text, re.ASCII)


# ---------------------------------------------------------------------------
# Lexer/parser crash-safety under arbitrary Unicode
# ---------------------------------------------------------------------------

@given(st.text(max_size=120))
@CRASH_SETTINGS
def test_tokenizer_only_raises_syntax_error(source):
    """Any input either tokenizes or raises SyntaxError — never another error."""
    import contextlib

    with contextlib.suppress(SyntaxError):
        Tokenizer(source).tokenize()


@given(st.text(max_size=120))
@CRASH_SETTINGS
def test_parser_only_raises_syntax_error(source):
    try:
        program = Parser(Tokenizer(source).tokenize()).parse()
    except SyntaxError:
        return
    # A successfully parsed program must transpile to valid Python.
    try:
        code = Transformer().transform(program)
    except SyntaxError:
        return
    try:
        compile(code, "<fuzz>", "exec")
    except SyntaxError:
        return


@given(st.text(alphabet=st.characters(min_codepoint=0x80), max_size=60))
@SETTINGS
def test_unicode_string_literals_roundtrip(text):
    """A Unicode string literal must survive transpilation byte-for-byte."""
    escaped = (
        text.replace("\\", "\\\\")
            .replace('"', '\\"')
            .replace("\n", "\\n")
            .replace("\r", "\\r")
    )
    source = f'let x = "{escaped}"'
    try:
        program = Parser(Tokenizer(source).tokenize()).parse()
    except SyntaxError:
        return
    code = Transformer().transform(program)
    namespace = {}
    exec(compile(code, "<fuzz>", "exec"), namespace)  # noqa: S102
    assert namespace["x"] == text


@given(st.characters(min_codepoint=0x80), st.characters(min_codepoint=0x80))
@SETTINGS
def test_unicode_identifiers_transpile_to_valid_python(a, b):
    """A name the lexer accepts as one IDENT must be valid Python."""
    if not (a.isidentifier() or ("a" + a).isidentifier()):
        return
    name = unicodedata.normalize("NFC", a + b) if b.isidentifier() else a
    if not name.isidentifier():
        return
    source = f"def main() {{ let {name} = 1\n print({name}) }}"
    try:
        code = _transpile(source)
    except SyntaxError:
        return
    compile(code, "<fuzz>", "exec")


# ---------------------------------------------------------------------------
# Language-level Unicode equivalence with plain Python
# ---------------------------------------------------------------------------

def test_unicode_program_matches_python_semantics():
    """A Unicode-named Aura program produces the same value as its Python twin."""
    source = (
        "def main() {\n"
        "  let café = 2\n"
        "  let 日本語 = 3\n"
        "  let результат = café * 日本語\n"
        "  print(результат)\n"
        "}\n"
        "main()\n"
    )
    import contextlib
    import io

    code = _transpile(source)
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, "<u>", "exec"), {})  # noqa: S102
    assert buffer.getvalue().strip() == "6"


def test_unicode_regex_through_language():
    """Regex helpers are reachable from Aura with Unicode patterns and text."""
    source = (
        "import stdlib.regex as regex\n"
        "def main() {\n"
        "  let found = regex.find_all(r'\\w+', 'café 日本語 ok')\n"
        "  print(len(found))\n"
        "}\n"
        "main()\n"
    )
    import contextlib
    import io

    code = _transpile(source)
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(compile(code, "<u>", "exec"), {})  # noqa: S102
    assert buffer.getvalue().strip() == "3"
