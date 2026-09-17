"""Exhaustive tests for the ``collections`` and ``string`` stdlib modules."""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura.stdlib import collections as C  # noqa: E402
from aura.stdlib import string as S  # noqa: E402

# ============================================================================
# collections: AuraDict
# ============================================================================

def test_auradict_attribute_read():
    d = C.AuraDict({"name": "a", "age": 3})
    assert d.name == "a"
    assert d["age"] == 3


def test_auradict_missing_attribute_raises_attribute_error():
    d = C.AuraDict({})
    with pytest.raises(AttributeError):
        _ = d.nope


def test_auradict_dunder_attribute_raises_attribute_error():
    d = C.AuraDict({})
    with pytest.raises(AttributeError):
        _ = d.__deepcopy__


def test_auradict_attribute_set():
    d = C.AuraDict({})
    d.x = 5
    assert d["x"] == 5


def test_auradict_get_attribute():
    assert C.AuraDict({"k": 1}).get("k") == 1
    assert C.AuraDict({}).get("k", 9) == 9


# ============================================================================
# collections: list_*
# ============================================================================

def test_list_map():
    assert C.list_map(lambda x: x * 2, [1, 2, 3]) == [2, 4, 6]


def test_list_filter():
    assert C.list_filter(lambda x: x % 2 == 0, [1, 2, 3, 4]) == [2, 4]


def test_list_reduce_without_initial():
    assert C.list_reduce(lambda a, b: a + b, [1, 2, 3]) == 6


def test_list_reduce_with_initial():
    assert C.list_reduce(lambda a, b: a + b, [1, 2, 3], 10) == 16


def test_list_reduce_empty_without_initial_raises():
    with pytest.raises(TypeError):
        C.list_reduce(lambda a, b: a + b, [])


def test_list_find_present_and_absent():
    assert C.list_find(lambda x: x > 2, [1, 2, 3, 4]) == 3
    assert C.list_find(lambda x: x > 9, [1, 2]) is None


def test_list_any_and_all():
    assert C.list_any(lambda x: x > 2, [1, 3]) is True
    assert C.list_any(lambda x: x > 9, [1, 3]) is False
    assert C.list_all(lambda x: x > 0, [1, 3]) is True
    assert C.list_all(lambda x: x > 1, [1, 3]) is False


def test_list_take_and_drop():
    assert C.list_take(2, [1, 2, 3, 4]) == [1, 2]
    assert C.list_take(0, [1, 2]) == []
    assert C.list_take(9, [1, 2]) == [1, 2]
    assert C.list_drop(2, [1, 2, 3, 4]) == [3, 4]
    assert C.list_drop(0, [1, 2]) == [1, 2]
    assert C.list_drop(9, [1, 2]) == []


def test_list_zip():
    assert C.list_zip([1, 2], [3, 4]) == [(1, 3), (2, 4)]


def test_list_flatten():
    assert C.list_flatten([[1, 2], [3], []]) == [1, 2, 3]


def test_list_unique_preserves_order():
    assert C.list_unique([3, 1, 3, 2, 1]) == [3, 1, 2]


def test_list_sort():
    assert C.list_sort([3, 1, 2]) == [1, 2, 3]
    assert C.list_sort([3, 1, 2], reverse=True) == [3, 2, 1]
    assert C.list_sort(["bb", "a"], key=len) == ["a", "bb"]


def test_list_reverse():
    assert C.list_reverse([1, 2, 3]) == [3, 2, 1]


def test_list_chunk():
    assert C.list_chunk(2, [1, 2, 3, 4, 5]) == [[1, 2], [3, 4], [5]]


def test_list_chunk_invalid_size_raises():
    with pytest.raises(ValueError):
        C.list_chunk(0, [1, 2])


def test_set_union_empty():
    assert C.set_union() == set()


def test_set_intersection_single_and_empty():
    assert C.set_intersection() == set()
    assert C.set_intersection({1, 2}) == {1, 2}


def test_set_intersection_returns_fresh_set():
    a = {1, 2}
    result = C.set_intersection(a, {2, 3})
    result.add(99)
    assert 99 not in a


def test_set_difference_single_and_fresh():
    assert C.set_difference({1, 2}) == {1, 2}
    a = {1, 2, 3}
    result = C.set_difference(a, {2})
    result.add(99)
    assert 99 not in a


def test_data_first_reduce_without_initial():
    assert C.reduce([1, 2, 3], lambda a, b: a + b) == 6


def test_data_first_take_drop_zero():
    assert C.take([1, 2], 0) == []
    assert C.drop([1, 2], 0) == [1, 2]


# ============================================================================
# collections: dict_*
# ============================================================================

def test_dict_get():
    assert C.dict_get({"a": 1}, "a") == 1
    assert C.dict_get({}, "a", 7) == 7


def test_dict_keys_values_items():
    d = {"a": 1, "b": 2}
    assert sorted(C.dict_keys(d)) == ["a", "b"]
    assert sorted(C.dict_values(d)) == [1, 2]
    assert sorted(C.dict_items(d)) == [("a", 1), ("b", 2)]


def test_dict_merge():
    assert C.dict_merge({"a": 1}, {"b": 2}, {"a": 3}) == {"a": 3, "b": 2}


def test_dict_filter():
    assert C.dict_filter(lambda k, v: v > 1, {"a": 1, "b": 2}) == {"b": 2}


def test_dict_map():
    assert C.dict_map(lambda v: v * 10, {"a": 1, "b": 2}) == {"a": 10, "b": 20}


# ============================================================================
# collections: set_*
# ============================================================================

def test_set_union():
    assert C.set_union({1, 2}, {2, 3}) == {1, 2, 3}


def test_set_intersection():
    assert C.set_intersection({1, 2, 3}, {2, 3, 4}) == {2, 3}


def test_set_difference():
    assert C.set_difference({1, 2, 3}, {2}) == {1, 3}


# ============================================================================
# collections: data-first pipe helpers
# ============================================================================

def test_data_first_map_filter_reduce():
    assert C.map([1, 2, 3], lambda x: x + 1) == [2, 3, 4]
    assert C.filter([1, 2, 3, 4], lambda x: x % 2 == 0) == [2, 4]
    assert C.reduce([1, 2, 3], lambda a, b: a + b, 0) == 6


def test_data_first_take_drop():
    assert C.take([1, 2, 3, 4], 2) == [1, 2]
    assert C.drop([1, 2, 3, 4], 2) == [3, 4]


# ============================================================================
# string: case and trim
# ============================================================================

@pytest.mark.parametrize("fn,arg,expected", [
    (S.upper, "abc", "ABC"),
    (S.lower, "ABC", "abc"),
    (S.title, "hello world", "Hello World"),
    (S.capitalize, "hello", "Hello"),
    (S.reverse, "abc", "cba"),
    (S.trim, "  hi  ", "hi"),
    (S.trim_left, "  hi  ", "hi  "),
    (S.trim_right, "  hi  ", "  hi"),
])
def test_string_case_and_trim(fn, arg, expected):
    assert fn(arg) == expected


# ============================================================================
# string: padding and repeat
# ============================================================================

def test_pad_left_and_right():
    assert S.pad_left("7", 3, "0") == "007"
    assert S.pad_right("7", 3, "0") == "700"
    assert S.pad("7", 3) == " 7 "


def test_pad_noop_when_long_enough():
    assert S.pad_left("abcd", 2) == "abcd"


def test_pad_invalid_fill_raises():
    with pytest.raises(ValueError):
        S.pad_left("x", 5, "ab")


def test_repeat_string():
    assert S.repeat_string("ab", 3) == "ababab"
    assert S.repeat_string("ab", 0) == ""


# ============================================================================
# string: split / join
# ============================================================================

def test_split_default_and_separator():
    assert S.split("a b c") == ["a", "b", "c"]
    assert S.split("a,b,c", ",") == ["a", "b", "c"]


def test_split_with_limit():
    assert S.split("a,b,c", ",", 1) == ["a", "b,c"]


def test_join_default_and_separator():
    assert S.join(["a", "b"]) == "ab"
    assert S.join(["a", "b"], "-") == "a-b"


# ============================================================================
# string: predicates and search
# ============================================================================

def test_starts_ends_contains():
    assert S.starts_with("hello", "he")
    assert not S.starts_with("hello", "lo")
    assert S.ends_with("hello", "lo")
    assert S.contains("hello", "ell")
    assert not S.contains("hello", "xyz")


def test_index_of():
    assert S.index_of("hello", "l") == 2
    assert S.index_of("hello", "z") == -1


def test_last_index_of():
    assert S.last_index_of("hello", "l") == 3
    assert S.last_index_of("hello", "z") == -1


def test_replace():
    assert S.replace("aXbXc", "X", "-") == "a-b-c"
    assert S.replace("aaa", "a", "b", 2) == "bba"


# ============================================================================
# string: slicing
# ============================================================================

def test_slice_string():
    assert S.slice_string("hello", 1, 3) == "el"
    assert S.slice_string("hello", 2) == "llo"


def test_substring():
    assert S.substring("hello", 1, 3) == "ell"


def test_char_at():
    assert S.char_at("hello", 1) == "e"
    # Out of range yields the `none` sentinel, not a slice.
    assert S.char_at("hello", 99) is None


def test_format_string():
    assert S.format_string("{} + {} = {}", 1, 2, 3) == "1 + 2 = 3"
    assert S.format_string("{a}-{b}", a="x", b="y") == "x-y"


# ============================================================================
# string: tests and length
# ============================================================================

def test_is_empty_and_blank():
    assert S.is_empty("")
    assert not S.is_empty("x")
    assert S.is_blank("   ")
    assert not S.is_blank(" x ")


def test_length():
    assert S.length("hello") == 5


@pytest.mark.parametrize("fn,arg,expected", [
    (S.is_alpha, "abc", True),
    (S.is_alpha, "ab1", False),
    (S.is_alphanumeric, "ab1", True),
    (S.is_digit, "123", True),
    (S.is_digit, "12a", False),
    (S.is_space, "  ", True),
    (S.is_lower, "abc", True),
    (S.is_lower, "Abc", False),
    (S.is_upper, "ABC", True),
    (S.is_upper, "ABc", False),
    (S.is_numeric, "3.14", True),
    (S.is_numeric, "x", False),
])
def test_string_predicates(fn, arg, expected):
    assert fn(arg) is expected


def test_is_numeric_empty_and_special():
    assert S.is_numeric("") is False
    assert S.is_numeric("nan") is False


# ============================================================================
# string: encoding helpers
# ============================================================================

def test_bytes_roundtrip():
    b = S.bytes_from_string("café")
    assert S.string_from_bytes(b) == "café"


def test_unicode_at_and_char_from_code():
    assert S.unicode_at("café", 3) == ord("é")
    assert S.char_from_code(233) == "é"


# ============================================================================
# string: lines and codes
# ============================================================================

def test_lines_and_unlines():
    assert S.lines("a\nb\nc") == ["a", "b", "c"]
    assert S.unlines(["a", "b"]) == "a\nb"


def test_codes_and_from_codes():
    assert S.codes("abc") == [97, 98, 99]
    assert S.from_codes([97, 98, 99]) == "abc"
