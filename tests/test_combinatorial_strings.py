"""Combinatorial string tests - batch 7b."""
from __future__ import annotations
import pytest
from aura.parser.to_ast import Tokenizer, Parser
from aura.transpiler.transformer import Transformer


def _compile(src):
    tokens = Tokenizer(src).tokenize()
    tree = Parser(tokens).parse()
    code = Transformer().transform(tree)
    compile(code, "<test>", "exec")
    return code

LENGTH_CASES = [
    ('""',),
    ('"a"',),
    ('"ab"',),
    ('"abc"',),
    ('"hello"',),
    ('"hello world"',),
    ('"HELLO"',),
    ('"0"',),
    ('"1"',),
    ('"-1"',),
]

@pytest.mark.parametrize("src", LENGTH_CASES)
def test_length(src):
    _compile(f"let x = {src}.length")

TRIM_CASES = [
    ('"  a  "',),
    ('"  hello  "',),
    ('"\\t\\nx\\n\\t"',),
    ('"x"',),
    ('"   "',),
]

@pytest.mark.parametrize("src", TRIM_CASES)
def test_trim(src):
    _compile(f"let x = {src}.trim()")

@pytest.mark.parametrize("src", TRIM_CASES)
def test_trim_left(src):
    _compile(f"let x = {src}.trim_left()")

@pytest.mark.parametrize("src", TRIM_CASES)
def test_trim_right(src):
    _compile(f"let x = {src}.trim_right()")

CONTAINS_CASES = [
    ('""', '""'),
    ('""', '"a"'),
    ('""', '"e"'),
    ('""', '"o"'),
    ('""', '"l"'),
    ('""', '"x"'),
    ('"a"', '""'),
    ('"a"', '"a"'),
    ('"a"', '"e"'),
    ('"a"', '"o"'),
    ('"a"', '"l"'),
    ('"a"', '"x"'),
    ('"ab"', '""'),
    ('"ab"', '"a"'),
    ('"ab"', '"e"'),
    ('"ab"', '"o"'),
    ('"ab"', '"l"'),
    ('"ab"', '"x"'),
    ('"abc"', '""'),
    ('"abc"', '"a"'),
    ('"abc"', '"e"'),
    ('"abc"', '"o"'),
    ('"abc"', '"l"'),
    ('"abc"', '"x"'),
    ('"hello"', '""'),
    ('"hello"', '"a"'),
    ('"hello"', '"e"'),
    ('"hello"', '"o"'),
    ('"hello"', '"l"'),
    ('"hello"', '"x"'),
    ('"hello world"', '""'),
    ('"hello world"', '"a"'),
    ('"hello world"', '"e"'),
    ('"hello world"', '"o"'),
    ('"hello world"', '"l"'),
    ('"hello world"', '"x"'),
    ('"HELLO"', '""'),
    ('"HELLO"', '"a"'),
    ('"HELLO"', '"e"'),
    ('"HELLO"', '"o"'),
    ('"HELLO"', '"l"'),
    ('"HELLO"', '"x"'),
    ('"0"', '""'),
    ('"0"', '"a"'),
    ('"0"', '"e"'),
    ('"0"', '"o"'),
    ('"0"', '"l"'),
    ('"0"', '"x"'),
    ('"1"', '""'),
    ('"1"', '"a"'),
    ('"1"', '"e"'),
    ('"1"', '"o"'),
    ('"1"', '"l"'),
    ('"1"', '"x"'),
    ('"-1"', '""'),
    ('"-1"', '"a"'),
    ('"-1"', '"e"'),
    ('"-1"', '"o"'),
    ('"-1"', '"l"'),
    ('"-1"', '"x"'),
]

@pytest.mark.parametrize("haystack,needle", CONTAINS_CASES)
def test_contains(haystack, needle):
    _compile(f"let x = {haystack}.contains({needle})")

STARTS_WITH_CASES = [
    ('""', '""'),
    ('""', '"h"'),
    ('""', '"a"'),
    ('""', '"he"'),
    ('""', '"hello"'),
    ('"a"', '""'),
    ('"a"', '"h"'),
    ('"a"', '"a"'),
    ('"a"', '"he"'),
    ('"a"', '"hello"'),
    ('"ab"', '""'),
    ('"ab"', '"h"'),
    ('"ab"', '"a"'),
    ('"ab"', '"he"'),
    ('"ab"', '"hello"'),
    ('"abc"', '""'),
    ('"abc"', '"h"'),
    ('"abc"', '"a"'),
    ('"abc"', '"he"'),
    ('"abc"', '"hello"'),
    ('"hello"', '""'),
    ('"hello"', '"h"'),
    ('"hello"', '"a"'),
    ('"hello"', '"he"'),
    ('"hello"', '"hello"'),
    ('"hello world"', '""'),
    ('"hello world"', '"h"'),
    ('"hello world"', '"a"'),
    ('"hello world"', '"he"'),
    ('"hello world"', '"hello"'),
    ('"HELLO"', '""'),
    ('"HELLO"', '"h"'),
    ('"HELLO"', '"a"'),
    ('"HELLO"', '"he"'),
    ('"HELLO"', '"hello"'),
    ('"0"', '""'),
    ('"0"', '"h"'),
    ('"0"', '"a"'),
    ('"0"', '"he"'),
    ('"0"', '"hello"'),
    ('"1"', '""'),
    ('"1"', '"h"'),
    ('"1"', '"a"'),
    ('"1"', '"he"'),
    ('"1"', '"hello"'),
    ('"-1"', '""'),
    ('"-1"', '"h"'),
    ('"-1"', '"a"'),
    ('"-1"', '"he"'),
    ('"-1"', '"hello"'),
]

@pytest.mark.parametrize("s,prefix", STARTS_WITH_CASES)
def test_starts_with(s, prefix):
    _compile(f"let x = {s}.starts_with({prefix})")

ENDS_WITH_CASES = [
    ('""', '""'),
    ('""', '"o"'),
    ('""', '"a"'),
    ('""', '"lo"'),
    ('""', '"hello"'),
    ('"a"', '""'),
    ('"a"', '"o"'),
    ('"a"', '"a"'),
    ('"a"', '"lo"'),
    ('"a"', '"hello"'),
    ('"ab"', '""'),
    ('"ab"', '"o"'),
    ('"ab"', '"a"'),
    ('"ab"', '"lo"'),
    ('"ab"', '"hello"'),
    ('"abc"', '""'),
    ('"abc"', '"o"'),
    ('"abc"', '"a"'),
    ('"abc"', '"lo"'),
    ('"abc"', '"hello"'),
    ('"hello"', '""'),
    ('"hello"', '"o"'),
    ('"hello"', '"a"'),
    ('"hello"', '"lo"'),
    ('"hello"', '"hello"'),
    ('"hello world"', '""'),
    ('"hello world"', '"o"'),
    ('"hello world"', '"a"'),
    ('"hello world"', '"lo"'),
    ('"hello world"', '"hello"'),
    ('"HELLO"', '""'),
    ('"HELLO"', '"o"'),
    ('"HELLO"', '"a"'),
    ('"HELLO"', '"lo"'),
    ('"HELLO"', '"hello"'),
    ('"0"', '""'),
    ('"0"', '"o"'),
    ('"0"', '"a"'),
    ('"0"', '"lo"'),
    ('"0"', '"hello"'),
    ('"1"', '""'),
    ('"1"', '"o"'),
    ('"1"', '"a"'),
    ('"1"', '"lo"'),
    ('"1"', '"hello"'),
    ('"-1"', '""'),
    ('"-1"', '"o"'),
    ('"-1"', '"a"'),
    ('"-1"', '"lo"'),
    ('"-1"', '"hello"'),
]

@pytest.mark.parametrize("s,suffix", ENDS_WITH_CASES)
def test_ends_with(s, suffix):
    _compile(f"let x = {s}.ends_with({suffix})")

REPEAT_CASES = [
    ('"a"', 0),
    ('"a"', 1),
    ('"a"', 2),
    ('"a"', 3),
    ('"a"', 5),
    ('"ab"', 0),
    ('"ab"', 1),
    ('"ab"', 2),
    ('"ab"', 3),
    ('"ab"', 5),
    ('"hello"', 0),
    ('"hello"', 1),
    ('"hello"', 2),
    ('"hello"', 3),
    ('"hello"', 5),
    ('""', 0),
    ('""', 1),
    ('""', 2),
    ('""', 3),
    ('""', 5),
]

@pytest.mark.parametrize("src,n", REPEAT_CASES)
def test_repeat(src, n):
    _compile(f"let x = {src}.repeat_string({n})")

REPLACE_CASES = [
    ('"hello"', '"l"', '"r"'),
    ('"aaa"', '"a"', '"b"'),
    ('"hello"', '"x"', '"y"'),
    ('"abcabc"', '"a"', '"z"'),
    ('"hello"', '"ll"', '"r"'),
    ('"test"', '"t"', '"T"'),
    ('"123"', '"2"', '"@"'),
    ('"aabb"', '"a"', '"x"'),
]

@pytest.mark.parametrize("src,old,new", REPLACE_CASES)
def test_replace(src, old, new):
    _compile(f"let x = {src}.replace({old}, {new})")

REVERSE_CASES = [
    ('""',),
    ('"a"',),
    ('"ab"',),
    ('"abc"',),
    ('"hello"',),
    ('"hello world"',),
    ('"HELLO"',),
    ('"0"',),
    ('"1"',),
    ('"-1"',),
]

@pytest.mark.parametrize("src", REVERSE_CASES)
def test_reverse(src):
    _compile(f"let x = {src}.reverse()")

IS_ALPHA_CASES = [
    ('"abc"',),
    ('"ABC"',),
    ('"abc123"',),
    ('"hello world"',),
    ('""',),
    ('"123"',),
]

@pytest.mark.parametrize("src", IS_ALPHA_CASES)
def test_is_alpha(src):
    _compile(f"let x = {src}.is_alpha()")

IS_DIGIT_CASES = [
    ('"123"',),
    ('"0"',),
    ('"12a"',),
    ('"abc"',),
    ('""',),
    ('"-1"',),
]

@pytest.mark.parametrize("src", IS_DIGIT_CASES)
def test_is_digit(src):
    _compile(f"let x = {src}.is_digit()")

IS_UPPER_CASES = [
    ('"HELLO"',),
    ('"hello"',),
    ('"Hello"',),
    ('""',),
    ('"ABC123"',),
]

@pytest.mark.parametrize("src", IS_UPPER_CASES)
def test_is_upper(src):
    _compile(f"let x = {src}.is_upper()")

IS_LOWER_CASES = [
    ('"hello"',),
    ('"HELLO"',),
    ('"Hello"',),
    ('""',),
    ('"abc123"',),
]

@pytest.mark.parametrize("src", IS_LOWER_CASES)
def test_is_lower(src):
    _compile(f"let x = {src}.is_lower()")

IS_EMPTY_CASES = [
    ('""',),
    ('"a"',),
    ('"ab"',),
    ('"abc"',),
    ('"hello"',),
    ('"hello world"',),
    ('"HELLO"',),
    ('"0"',),
    ('"1"',),
    ('"-1"',),
]

@pytest.mark.parametrize("src", IS_EMPTY_CASES)
def test_is_empty(src):
    _compile(f"let x = {src}.is_empty()")

IS_ALPHANUMERIC_CASES = [
    ('"abc123"',),
    ('"ABC"',),
    ('"hello"',),
    ('""',),
    ('"12345"',),
]

@pytest.mark.parametrize("src", IS_ALPHANUMERIC_CASES)
def test_is_alphanumeric(src):
    _compile(f"let x = {src}.is_alphanumeric()")

IS_BLANK_CASES = [
    ('"   "',),
    ('"x"',),
    ('""',),
    ('" a "',),
]

@pytest.mark.parametrize("src", IS_BLANK_CASES)
def test_is_blank(src):
    _compile(f"let x = {src}.is_blank()")

SPLIT_CASES = [
    ('"a,b,c"', '","'),
    ('"hello world"', '" "'),
    ('"abc"', '","'),
    ('"x"', '"-"'),
    ('"hello"', '"l"'),
]

@pytest.mark.parametrize("src,sep", SPLIT_CASES)
def test_split(src, sep):
    _compile(f"let x = {src}.split({sep})")

CHAR_AT_CASES = [
    ('"hello"', 0),
    ('"hello"', 1),
    ('"hello"', 4),
    ('"a"', 0),
    ('"abc"', 2),
]

@pytest.mark.parametrize("src,i", CHAR_AT_CASES)
def test_char_at(src, i):
    _compile(f"let x = {src}.char_at({i})")

INDEX_OF_CASES = [
    ('"hello"', '"h"'),
    ('"hello"', '"e"'),
    ('"hello"', '"o"'),
    ('"hello"', '"x"'),
    ('"hello"', '"hello"'),
    ('"hello"', '"llo"'),
]

@pytest.mark.parametrize("src,sub", INDEX_OF_CASES)
def test_index_of(src, sub):
    _compile(f"let x = {src}.index_of({sub})")

UNICODE_AT_CASES = [
    ('"hello"', 0),
    ('"hello"', 1),
    ('"hello"', 4),
    ('"A"', 0),
    ('"café"', 3),
]

@pytest.mark.parametrize("src,i", UNICODE_AT_CASES)
def test_unicode_at(src, i):
    _compile(f"let x = {src}.unicode_at({i})")
