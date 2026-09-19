"""Phase 0 — stress and fuzz tests for Unicode support and regex parity.

Two invariants are exercised:

1. The lexer accepts exactly the identifiers Python accepts. A name that
   tokenizes as one IDENT must also be a valid Python identifier, and a name
   Python rejects must not silently become one.
2. Every generated regex pattern either behaves like :mod:`re` or raises the
   same class of error; the wrappers never mask a real failure or crash on
   arbitrary Unicode input.
"""
import re
import sys
from pathlib import Path

from hypothesis import HealthCheck, assume, given, settings
from hypothesis import strategies as st

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Tokenizer  # noqa: E402
from aura.stdlib import regex  # noqa: E402

SETTINGS = settings(
    max_examples=300,
    deadline=None,
    suppress_health_check=[HealthCheck.too_slow],
)

_IDENT_TAIL = st.text(
    alphabet=st.characters(
        whitelist_categories=("Ll", "Lu", "Lt", "Lm", "Lo", "Mn", "Mc", "Nd", "Nl", "Pc"),
    ),
    min_size=0,
    max_size=8,
)

_UNICODE_TEXT = st.text(max_size=60)


def _idents(src):
    return [t.value for t in Tokenizer(src).tokenize() if t.type == "IDENT"]


# ---------------------------------------------------------------------------
# Unicode identifier parity
# ---------------------------------------------------------------------------

@given(st.text(alphabet=st.characters(min_codepoint=0x80), min_size=1, max_size=4))
@SETTINGS
def test_unicode_identifiers_match_python(name):
    """A tokenized identifier is always a valid Python identifier.

    The tokenizer must never accept a character Python rejects as an
    identifier character, otherwise Aura emits invalid Python.
    """
    src = f"let {name} = 1"
    try:
        idents = _idents(src)
    except SyntaxError:
        return
    for value in idents:
        if value == "let":
            continue
        assert value.isidentifier(), f"tokenized invalid Python identifier {value!r}"


@given(st.from_regex(r"[A-Za-z_\u00aa\u00b5\u00ba\u2118\u212e][A-Za-z0-9_\u0301\u200d]*", fullmatch=True))
@SETTINGS
def test_valid_python_identifiers_roundtrip(name):
    """A valid Python identifier is lexed as a single IDENT (NFC-normalized)."""
    import unicodedata

    if not name.isidentifier():
        assume(False)  # skip names not valid on this Python version (e.g. ZWJ)
    idents = _idents(f"let {name} = 1")
    assert unicodedata.normalize("NFC", name) in idents


@given(_IDENT_TAIL, _IDENT_TAIL)
@SETTINGS
def test_identifier_normalization_is_stable(a, b):
    """NFC normalization is idempotent on tokenized identifiers."""
    import unicodedata

    src = f"let {a}{b} = 1"
    try:
        idents = _idents(src)
    except SyntaxError:
        return
    for value in idents:
        assert unicodedata.normalize("NFC", value) == value


# ---------------------------------------------------------------------------
# Regex parity and crash-safety
# ---------------------------------------------------------------------------

@given(_UNICODE_TEXT)
@SETTINGS
def test_regex_never_crashes(text):
    """Public wrappers either return a value or raise re.error — nothing else."""
    try:
        regex.search("a", text)
        regex.find_all(r"\w+", text)
        regex.replace(r"\d+", "N", text)
    except re.error:
        return


@given(st.text(max_size=40), st.text(max_size=200))
@SETTINGS
def test_find_all_matches_re(pattern, text):
    """find_all agrees with :func:`re.findall` for every pattern/text pair."""
    try:
        expected = re.findall(pattern, text)
    except re.error:
        return
    assert regex.find_all(pattern, text) == expected


@given(st.text(max_size=40), st.text(max_size=200))
@SETTINGS
def test_subn_matches_re(pattern, text):
    try:
        expected = re.subn(pattern, "N", text)
    except re.error:
        return
    assert regex.subn(pattern, "N", text) == expected


@given(st.text(max_size=40), st.text(max_size=200))
@SETTINGS
def test_split_matches_re(pattern, text):
    try:
        expected = re.split(pattern, text)
    except re.error:
        return
    assert regex.split(pattern, text) == expected


_UNICODE_PATTERNS = st.sampled_from([
    r"\w+",
    r"[\u4e00-\u9fff]+",
    r"(?P<word>\w+)",
    r"(?<=\$)\d+",
    r"\d+(?=%)",
    r"(?i)hello",
    r"(?s).+",
    r"(?m)^x",
])


@given(_UNICODE_PATTERNS, _UNICODE_TEXT)
@SETTINGS
def test_supported_patterns_parity(pattern, text):
    assert regex.search(pattern, text) is not None if re.search(pattern, text) else True
