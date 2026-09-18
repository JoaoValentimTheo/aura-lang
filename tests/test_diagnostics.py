"""Diagnostics standardisation tests.

``docs/ERRORS.md`` is the single source of truth for diagnostic codes; these
tests assert that ``aura/transpiler/errors.py`` mirrors it exactly, that
warnings use the ``W`` namespace, and that diagnostics carry real locations.
"""
import re
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler import errors as errors_mod  # noqa: E402
from aura.transpiler.errors import (  # noqa: E402
    ErrorCode,
    ErrorCollector,
)
from aura.transpiler.rules import RuleChecker  # noqa: E402

ERRORS_DOC = ROOT / "docs" / "ERRORS.md"


def documented_codes():
    """Return the set of active codes listed in ERRORS.md tables.

    The trailing "Removed codes" section is excluded: it documents codes that
    must *not* exist in the enum.
    """
    text = ERRORS_DOC.read_text(encoding="utf-8")
    text = text.split("## Removed codes", 1)[0]
    return set(re.findall(r"`([EW]\d{3})`", text))


def parse(source, require_main=False):
    prog = Parser(Tokenizer(source).tokenize()).parse()
    checker = RuleChecker()
    checker.check_program(prog, require_main=require_main)
    return checker.collector.errors


# ============================================================================
# docs/ERRORS.md <-> errors.py stay in sync
# ============================================================================

def test_errors_doc_exists():
    assert ERRORS_DOC.is_file()


def test_every_enum_code_is_documented():
    enum_codes = {c.value for c in ErrorCode}
    doc_codes = documented_codes()
    missing = enum_codes - doc_codes
    assert not missing, f"codes in errors.py but not docs/ERRORS.md: {sorted(missing)}"


def test_documented_codes_exist_in_enum():
    enum_codes = {c.value for c in ErrorCode}
    doc_codes = documented_codes()
    # W2xx range is documented as reserved, not defined.
    reserved = {c for c in doc_codes if c.startswith("W2")}
    extra = doc_codes - enum_codes - reserved
    assert not extra, f"codes in docs/ERRORS.md but not errors.py: {sorted(extra)}"


def test_removed_codes_absent():
    for name in ("DIVISION_BY_ZERO", "NULL_POINTER", "INDEX_OUT_OF_BOUNDS",
                 "KEY_ERROR", "INFINITE_LOOP", "MISSING_RETURN",
                 # Documented but never emitted; removed to keep the catalogue
                 # honest (see docs/ERRORS.md, "Removed codes").
                 "SYNTAX_ERROR", "UNEXPECTED_TOKEN", "UNEXPECTED_EOF",
                 "UNDEFINED_VARIABLE", "UNDEFINED_FUNCTION", "UNDEFINED_CLASS",
                 "CANNOT_CALL_NON_FUNCTION", "IO_ERROR", "CONFIGURATION_ERROR",
                 "UNUSED_VARIABLE", "UNUSED_IMPORT"):
        assert not hasattr(ErrorCode, name), f"{name} should be removed"


def test_code_numbering_scheme():
    for code in ErrorCode:
        assert re.fullmatch(r"[EW]\d{3}", code.value), code
        prefix = code.value[0]
        assert prefix in "EW"


# ============================================================================
# Warnings use the W namespace
# ============================================================================

def test_add_warning_rejects_error_codes():
    collector = ErrorCollector("x.aura")
    with pytest.raises(ValueError):
        collector.add_warning(ErrorCode.TYPE_MISMATCH, "nope")


def test_add_warning_accepts_w_codes():
    collector = ErrorCollector("x.aura")
    collector.add_warning(ErrorCode.LINE_TOO_LONG, "too long")
    assert collector.warning_count() == 1
    assert not collector.has_errors()


def test_all_w_codes_are_warnings_by_convention():
    for code in ErrorCode:
        if code.value.startswith("W"):
            assert code.value[1] in "012"


# ============================================================================
# Diagnostics carry real locations
# ============================================================================

def test_parser_error_reports_line_and_column():
    tokens = Tokenizer("let x = 1\nlet = 2\n").tokenize()
    parser = Parser(tokens, filename="m.aura")
    with pytest.raises(SyntaxError) as info:
        parser.parse()
    assert info.value.line == 2
    assert info.value.column >= 1
    assert info.value.filename == "m.aura"


def test_rule_error_carries_location():
    errors = parse("def main() {\n  return 1\n  print(2)\n}")
    unreachable = [e for e in errors if e.code is ErrorCode.UNREACHABLE_CODE]
    assert unreachable
    loc = unreachable[0].location
    assert loc is not None
    assert loc.line == 3


def test_visibility_error_carries_location():
    errors = parse("class C {\n  let x = 1\n}")
    miss = [e for e in errors if e.code is ErrorCode.MISSING_VISIBILITY]
    assert miss
    assert miss[0].location is not None
    assert miss[0].location.line == 2


def test_duplicate_error_carries_location():
    errors = parse("def main() {}\nlet x = 1\nlet x = 2\n")
    dup = [e for e in errors if e.code is ErrorCode.DUPLICATE_DEFINITION]
    assert dup
    # 'let x' is on line 2; the 'def' on line 1 must not be flagged.
    assert all(e.location is None or e.location.line >= 2 for e in dup)


def test_main_error_carries_location():
    errors = parse("def main(a, b) {}", require_main=True)
    bad = [e for e in errors if e.code is ErrorCode.INVALID_MAIN]
    assert bad
    assert bad[0].location is not None


def test_error_format_includes_location_prefix():
    collector = ErrorCollector("f.aura")
    from aura.transpiler.ast import SourceLocation
    collector.add(ErrorCode.TYPE_MISMATCH, "boom",
                  location=SourceLocation("f.aura", 3, 5, 2))
    text = collector.format()
    assert "f.aura:3:5" in text
    assert "[E101]" in text


def test_warning_format_includes_code_and_location():
    collector = ErrorCollector("f.aura")
    from aura.transpiler.ast import SourceLocation
    collector.add_warning(ErrorCode.TRAILING_WHITESPACE, "trailing",
                          location=SourceLocation("f.aura", 2, 9, 1))
    text = collector.format()
    assert "[W002]" in text
    assert "f.aura:2:9" in text
    assert "1 warning(s)" in text


def test_code_area_classification():
    assert errors_mod.code_area(ErrorCode.TYPE_MISMATCH) == "type"
    assert errors_mod.code_area(ErrorCode.MISSING_MAIN) == "semantic"
    assert errors_mod.code_area(ErrorCode.LINE_TOO_LONG) == "style warning"
    assert errors_mod.code_area(ErrorCode.UNUSED_TYPE_PARAMETER) == "warning"
    assert errors_mod.code_area(ErrorCode.FATAL) == "fatal"
