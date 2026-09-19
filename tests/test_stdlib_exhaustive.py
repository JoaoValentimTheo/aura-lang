"""Stdlib exhaustive tests - batch 3."""
from __future__ import annotations

import itertools as pyit
import math
import pytest
from aura.stdlib import math as aura_math
from aura.stdlib import string as aura_string
from aura.stdlib import collections as aura_collections
from aura.stdlib import json as aura_json
from aura.stdlib import itertools as aura_itertools
from aura.stdlib import time as aura_time
from aura.stdlib import os as aura_os
from aura.stdlib import python as aura_python

# ---------------------------------------------------------------------------
# stdlib.math - unary
# ---------------------------------------------------------------------------
MATH_UNARY = [
    ("sqrt_4", aura_math.sqrt, 4, 2.0),
    ("sqrt_9", aura_math.sqrt, 9, 3.0),
    ("sqrt_0", aura_math.sqrt, 0, 0.0),
    ("sqrt_float", aura_math.sqrt, 2.25, 1.5),
    ("abs_pos", aura_math.abs, 5, 5),
    ("abs_neg", aura_math.abs, -5, 5),
    ("abs_zero", aura_math.abs, 0, 0),
    ("abs_float", aura_math.abs, -3.14, 3.14),
    ("ceil_12", aura_math.ceil, 1.2, 2),
    ("ceil_18", aura_math.ceil, 1.8, 2),
    ("ceil_int", aura_math.ceil, 5, 5),
    ("floor_18", aura_math.floor, 1.8, 1),
    ("floor_12", aura_math.floor, 1.2, 1),
    ("floor_int", aura_math.floor, 5, 5),
    ("sin_0", aura_math.sin, 0, 0.0),
    ("sin_half_pi", aura_math.sin, math.pi / 2, 1.0),
    ("cos_0", aura_math.cos, 0, 1.0),
    ("cos_pi", aura_math.cos, math.pi, -1.0),
    ("tan_0", aura_math.tan, 0, 0.0),
    ("factorial_5", aura_math.factorial, 5, 120),
    ("factorial_0", aura_math.factorial, 0, 1),
    ("factorial_1", aura_math.factorial, 1, 1),
    ("factorial_10", aura_math.factorial, 10, 3628800),
    ("is_finite", aura_math.is_finite, 1.0, True),
    ("is_finite_inf", aura_math.is_finite, float("inf"), False),
    ("is_nan", aura_math.is_nan, float("nan"), True),
    ("is_nan_num", aura_math.is_nan, 1.0, False),
    ("is_infinite", aura_math.is_infinite, float("inf"), True),
    ("is_infinite_num", aura_math.is_infinite, 1.0, False),
    ("degrees_pi", aura_math.degrees, math.pi, 180.0),
    ("degrees_0", aura_math.degrees, 0.0, 0.0),
    ("radians_180", aura_math.radians, 180, math.pi),
    ("radians_0", aura_math.radians, 0, 0.0),
    ("log2_8", aura_math.log2, 8, 3.0),
    ("log2_1", aura_math.log2, 1, 0.0),
    ("log10_100", aura_math.log10, 100, 2.0),
    ("log10_1", aura_math.log10, 1, 0.0),
    ("log_e", aura_math.log, math.e, 1.0),
    ("exp_0", aura_math.exp, 0, 1.0),
    ("exp_1", aura_math.exp, 1, math.e),
    ("fabs_neg", aura_math.fabs, -3.5, 3.5),
    ("fabs_pos", aura_math.fabs, 3.5, 3.5),
    ("cosh_0", aura_math.cosh, 0, 1.0),
    ("sinh_0", aura_math.sinh, 0, 0.0),
    ("tanh_0", aura_math.tanh, 0, 0.0),
    ("acos_1", aura_math.acos, 1, 0.0),
    ("asin_0", aura_math.asin, 0, 0.0),
    ("atan_0", aura_math.atan, 0, 0.0),
]

@pytest.mark.parametrize("name,fn,arg,expected", MATH_UNARY, ids=[i[0] for i in MATH_UNARY])
def test_math_unary(name, fn, arg, expected):
    result = fn(arg)
    if isinstance(expected, float) and math.isnan(expected):
        assert math.isnan(result)
    elif isinstance(expected, float):
        assert abs(result - expected) < 1e-10
    else:
        assert result == expected

# ---------------------------------------------------------------------------
# stdlib.math - binary
# ---------------------------------------------------------------------------
MATH_BINARY = [
    ("pow_2_10", aura_math.pow, 2, 10, 1024.0),
    ("pow_3_3", aura_math.pow, 3, 3, 27.0),
    ("pow_10_0", aura_math.pow, 10, 0, 1.0),
    ("min_3_1", aura_math.min, 3, 1, 1),
    ("min_neg", aura_math.min, -1, -5, -5),
    ("max_3_1", aura_math.max, 3, 1, 3),
    ("max_neg", aura_math.max, -1, -5, -1),
    ("gcd_12_8", aura_math.gcd, 12, 8, 4),
    ("gcd_coprime", aura_math.gcd, 7, 13, 1),
    ("gcd_0", aura_math.gcd, 0, 5, 5),
    ("hypot_3_4", aura_math.hypot, 3, 4, 5.0),
    ("hypot_5_12", aura_math.hypot, 5, 12, 13.0),
    ("atan2_1_0", aura_math.atan2, 1, 0, math.pi / 2),
    ("atan2_0_1", aura_math.atan2, 0, 1, 0.0),
    ("fmod", aura_math.fmod, 7.0, 2.0, 1.0),
    ("fmod_exact", aura_math.fmod, 10.0, 5.0, 0.0),
]

@pytest.mark.parametrize("name,fn,a,b,expected", MATH_BINARY, ids=[i[0] for i in MATH_BINARY])
def test_math_binary(name, fn, a, b, expected):
    result = fn(a, b)
    assert abs(result - expected) < 1e-10

# ---------------------------------------------------------------------------
# stdlib.string - 1 arg
# ---------------------------------------------------------------------------
STRING_1 = [
    ("length_hello", aura_string.length, "hello", 5),
    ("length_empty", aura_string.length, "", 0),
    ("length_unicode", aura_string.length, "\u65e5\u672c\u8a9e", 3),
    ("upper", aura_string.upper, "hello", "HELLO"),
    ("lower", aura_string.lower, "HELLO", "hello"),
    ("trim", aura_string.trim, "  hello  ", "hello"),
    ("trim_tabs", aura_string.trim, "\t\nhello\n\t", "hello"),
    ("trim_left", aura_string.trim_left, "  hello  ", "hello  "),
    ("trim_right", aura_string.trim_right, "  hello  ", "  hello"),
    ("reverse", aura_string.reverse, "hello", "olleh"),
    ("reverse_empty", aura_string.reverse, "", ""),
    ("is_empty_true", aura_string.is_empty, "", True),
    ("is_empty_false", aura_string.is_empty, "x", False),
    ("capitalize", aura_string.capitalize, "hello", "Hello"),
    ("capitalize_single", aura_string.capitalize, "a", "A"),
    ("title", aura_string.title, "hello world", "Hello World"),
    ("title_single", aura_string.title, "hello", "Hello"),
    ("is_alpha_true", aura_string.is_alpha, "hello", True),
    ("is_alpha_false", aura_string.is_alpha, "hello1", False),
    ("is_alpha_empty", aura_string.is_alpha, "", False),
    ("is_digit_true", aura_string.is_digit, "123", True),
    ("is_digit_false", aura_string.is_digit, "12a", False),
    ("is_digit_empty", aura_string.is_digit, "", False),
    ("is_upper_true", aura_string.is_upper, "HELLO", True),
    ("is_upper_false", aura_string.is_upper, "Hello", False),
    ("is_lower_true", aura_string.is_lower, "hello", True),
    ("is_lower_false", aura_string.is_lower, "Hello", False),
    ("is_blank_true", aura_string.is_blank, "   ", True),
    ("is_blank_false", aura_string.is_blank, " a ", False),
    ("is_blank_empty", aura_string.is_blank, "", True),
    ("is_alphanumeric", aura_string.is_alphanumeric, "abc123", True),
    ("is_alphanumeric_false", aura_string.is_alphanumeric, "abc!@#", False),
    ("is_numeric_true", aura_string.is_numeric, "123", True),
    ("is_numeric_false", aura_string.is_numeric, "12a", False),
    ("is_space_true", aura_string.is_space, "  ", True),
    ("is_space_false", aura_string.is_space, "a", False),
]

@pytest.mark.parametrize("name,fn,arg,expected", STRING_1, ids=[i[0] for i in STRING_1])
def test_string_1arg(name, fn, arg, expected):
    assert fn(arg) == expected

# ---------------------------------------------------------------------------
# stdlib.string - 2 args
# ---------------------------------------------------------------------------
STRING_2 = [
    ("contains_yes", aura_string.contains, "hello", "ell", True),
    ("contains_no", aura_string.contains, "hello", "xyz", False),
    ("contains_full", aura_string.contains, "hello", "hello", True),
    ("contains_empty_needle", aura_string.contains, "hello", "", True),
    ("starts_with_yes", aura_string.starts_with, "hello", "hel", True),
    ("starts_with_no", aura_string.starts_with, "hello", "xyz", False),
    ("starts_with_full", aura_string.starts_with, "hello", "hello", True),
    ("ends_with_yes", aura_string.ends_with, "hello", "llo", True),
    ("ends_with_no", aura_string.ends_with, "hello", "xyz", False),
    ("ends_with_full", aura_string.ends_with, "hello", "hello", True),
    ("split_space", aura_string.split, "hello world", " ", ["hello", "world"]),
    ("split_comma", aura_string.split, "a,b,c", ",", ["a", "b", "c"]),
    ("split_no_match", aura_string.split, "abc", ",", ["abc"]),
    ("char_at_0", aura_string.char_at, "hello", 0, "h"),
    ("char_at_1", aura_string.char_at, "hello", 1, "e"),
    ("char_at_last", aura_string.char_at, "hello", 4, "o"),
    ("index_of_found", aura_string.index_of, "hello", "ell", 1),
    ("index_of_not_found", aura_string.index_of, "hello", "xyz", -1),
    ("index_of_first", aura_string.index_of, "hello", "h", 0),
    ("last_index_of_found", aura_string.last_index_of, "hellohello", "ell", 6),
    ("last_index_of_not_found", aura_string.last_index_of, "hello", "xyz", -1),
    ("repeat_3", aura_string.repeat_string, "ab", 3, "ababab"),
    ("repeat_0", aura_string.repeat_string, "ab", 0, ""),
    ("repeat_1", aura_string.repeat_string, "ab", 1, "ab"),
]

@pytest.mark.parametrize("name,fn,a,b,expected", STRING_2, ids=[i[0] for i in STRING_2])
def test_string_2args(name, fn, a, b, expected):
    assert fn(a, b) == expected

# ---------------------------------------------------------------------------
# stdlib.string - 3 args
# ---------------------------------------------------------------------------
STRING_3 = [
    ("replace", aura_string.replace, "hello world", "world", "aura", "hello aura"),
    ("replace_all", aura_string.replace, "aaa", "a", "b", "bbb"),
    ("replace_none", aura_string.replace, "hello", "xyz", "a", "hello"),
    ("pad_left", aura_string.pad_left, "42", 5, "0", "00042"),
    ("pad_left_already", aura_string.pad_left, "hello", 3, "0", "hello"),
    ("pad_right", aura_string.pad_right, "hi", 5, ".", "hi..."),
    ("pad_right_already", aura_string.pad_right, "hello", 3, ".", "hello"),
    ("substring", aura_string.substring, "hello", 1, 3, "ell"),
    ("substring_full", aura_string.substring, "hello", 0, 5, "hello"),
    ("substring_single", aura_string.substring, "hello", 2, 1, "l"),
    ("slice_string", aura_string.slice_string, "hello", 1, 4, "ell"),
]

@pytest.mark.parametrize("name,fn,a,b,c,expected", STRING_3, ids=[i[0] for i in STRING_3])
def test_string_3args(name, fn, a, b, c, expected):
    assert fn(a, b, c) == expected

# ---------------------------------------------------------------------------
# stdlib.string - pad (no direction param)
# ---------------------------------------------------------------------------
def test_pad_default():
    assert aura_string.pad("hi", 5) == "  hi "

def test_pad_char():
    assert aura_string.pad("hi", 6, ".") == "..hi.."

def test_pad_already():
    assert aura_string.pad("hello", 3) == "hello"

# ---------------------------------------------------------------------------
# stdlib.string - join/special
# ---------------------------------------------------------------------------
def test_string_join():
    assert aura_string.join(["a", "b", "c"], ",") == "a,b,c"

def test_string_join_empty():
    assert aura_string.join(["a", "b"], "") == "ab"

def test_string_join_one():
    assert aura_string.join(["a"], ",") == "a"

def test_string_char_from_code():
    assert aura_string.char_from_code(65) == "A"

def test_string_from_codes():
    assert aura_string.from_codes([65, 66, 67]) == "ABC"

def test_string_codes():
    assert aura_string.codes("ABC") == [65, 66, 67]

def test_string_unicode_at():
    assert aura_string.unicode_at("café", 3) == 233

def test_string_lines():
    assert aura_string.lines("a\nb\nc") == ["a", "b", "c"]

def test_string_lines_single():
    assert aura_string.lines("hello") == ["hello"]

# ---------------------------------------------------------------------------
# stdlib.collections - list
# ---------------------------------------------------------------------------
def test_list_filter():
    assert aura_collections.list_filter(lambda x: x > 2, [1, 2, 3]) == [3]

def test_list_filter_multiple():
    assert aura_collections.list_filter(lambda x: x % 2 == 0, [1, 2, 3, 4, 5]) == [2, 4]

def test_list_filter_none():
    assert aura_collections.list_filter(lambda x: x > 10, [1, 2, 3]) == []

def test_list_map():
    assert aura_collections.list_map(lambda x: x * 2, [1, 2, 3]) == [2, 4, 6]

def test_list_map_str():
    assert aura_collections.list_map(lambda x: str(x), [1, 2, 3]) == ["1", "2", "3"]

def test_list_reduce():
    assert aura_collections.list_reduce(lambda a, b: a + b, [1, 2, 3], 0) == 6

def test_list_reduce_mul():
    assert aura_collections.list_reduce(lambda a, b: a * b, [2, 3, 4], 1) == 24

def test_list_flatten():
    assert aura_collections.list_flatten([1, [2, 3], 4]) == [1, 2, 3, 4]

def test_list_flatten_deep():
    assert aura_collections.list_flatten([1, [2, [3, [4]]]]) == [1, 2, 3, 4]

def test_list_reverse():
    assert aura_collections.list_reverse([1, 2, 3]) == [3, 2, 1]

def test_list_reverse_single():
    assert aura_collections.list_reverse([1]) == [1]

def test_list_sort():
    assert aura_collections.list_sort([3, 1, 2]) == [1, 2, 3]

def test_list_sort_strings():
    assert aura_collections.list_sort(["c", "a", "b"]) == ["a", "b", "c"]

def test_list_take():
    assert aura_collections.list_take(2, [1, 2, 3]) == [1, 2]

def test_list_take_zero():
    assert aura_collections.list_take(0, [1, 2, 3]) == []

def test_list_take_all():
    assert aura_collections.list_take(10, [1, 2]) == [1, 2]

def test_list_drop():
    assert aura_collections.list_drop(2, [1, 2, 3, 4]) == [3, 4]

def test_list_drop_zero():
    assert aura_collections.list_drop(0, [1, 2]) == [1, 2]

def test_list_drop_all():
    assert aura_collections.list_drop(10, [1, 2]) == []

def test_list_unique():
    assert aura_collections.list_unique([1, 2, 2, 3, 3]) == [1, 2, 3]

def test_list_unique_no_dups():
    assert aura_collections.list_unique([1, 2, 3]) == [1, 2, 3]

def test_list_chunk():
    assert aura_collections.list_chunk(2, [1, 2, 3, 4, 5]) == [[1, 2], [3, 4], [5]]

def test_list_chunk_exact():
    assert aura_collections.list_chunk(2, [1, 2, 3, 4]) == [[1, 2], [3, 4]]

def test_list_find():
    assert aura_collections.list_find(lambda x: x > 2, [1, 2, 3]) == 3

def test_list_find_none():
    assert aura_collections.list_find(lambda x: x > 10, [1, 2, 3]) is None

def test_list_any_true():
    assert aura_collections.list_any(lambda x: x > 2, [1, 2, 3]) == True

def test_list_any_false():
    assert aura_collections.list_any(lambda x: x > 5, [1, 2, 3]) == False

def test_list_all_true():
    assert aura_collections.list_all(lambda x: x % 2 == 0, [2, 4, 6]) == True

def test_list_all_false():
    assert aura_collections.list_all(lambda x: x % 2 == 0, [2, 3, 6]) == False

def test_list_zip():
    assert aura_collections.list_zip([1, 2], ["a", "b"]) == [(1, "a"), (2, "b")]

def test_list_zip_uneven():
    assert aura_collections.list_zip([1], ["a", "b"]) == [(1, "a")]

# ---------------------------------------------------------------------------
# stdlib.collections - dict
# ---------------------------------------------------------------------------
def test_dict_keys():
    assert sorted(aura_collections.dict_keys({"a": 1, "b": 2})) == ["a", "b"]

def test_dict_keys_empty():
    assert aura_collections.dict_keys({}) == []

def test_dict_values():
    assert sorted(aura_collections.dict_values({"a": 1, "b": 2})) == [1, 2]

def test_dict_values_empty():
    assert aura_collections.dict_values({}) == []

def test_dict_merge():
    assert aura_collections.dict_merge({"a": 1}, {"b": 2}) == {"a": 1, "b": 2}

def test_dict_merge_overlap():
    assert aura_collections.dict_merge({"a": 1, "b": 2}, {"b": 3}) == {"a": 1, "b": 3}

def test_dict_items():
    assert sorted(aura_collections.dict_items({"a": 1})) == [("a", 1)]

def test_dict_filter():
    result = aura_collections.dict_filter(lambda k, v: v > 1, {"a": 1, "b": 2, "c": 3})
    assert result == {"b": 2, "c": 3}

def test_dict_map():
    result = aura_collections.dict_map(lambda v: v * 10, {"a": 1, "b": 2})
    assert result == {"a": 10, "b": 20}

# ---------------------------------------------------------------------------
# stdlib.json
# ---------------------------------------------------------------------------
JSON_DUMPS = [
    ("string", aura_json.dumps, "hello", '"hello"'),
    ("int", aura_json.dumps, 42, "42"),
    ("float", aura_json.dumps, 3.14, "3.14"),
    ("bool_true", aura_json.dumps, True, "true"),
    ("bool_false", aura_json.dumps, False, "false"),
    ("none", aura_json.dumps, None, "null"),
    ("list", aura_json.dumps, [1, 2, 3], "[1, 2, 3]"),
    ("empty_list", aura_json.dumps, [], "[]"),
    ("empty_dict", aura_json.dumps, {}, "{}"),
]

@pytest.mark.parametrize("name,fn,arg,expected", JSON_DUMPS, ids=[i[0] for i in JSON_DUMPS])
def test_json_dumps(name, fn, arg, expected):
    assert fn(arg) == expected

JSON_LOADS = [
    ("string", aura_json.loads, '"hello"', "hello"),
    ("int", aura_json.loads, "42", 42),
    ("float", aura_json.loads, "3.14", 3.14),
    ("bool_true", aura_json.loads, "true", True),
    ("bool_false", aura_json.loads, "false", False),
    ("none", aura_json.loads, "null", None),
    ("list", aura_json.loads, "[1, 2, 3]", [1, 2, 3]),
    ("empty_list", aura_json.loads, "[]", []),
    ("empty_dict", aura_json.loads, "{}", {}),
    ("nested", aura_json.loads, '{"a": [1, 2]}', {"a": [1, 2]}),
]

@pytest.mark.parametrize("name,fn,arg,expected", JSON_LOADS, ids=[i[0] for i in JSON_LOADS])
def test_json_loads(name, fn, arg, expected):
    assert fn(arg) == expected

def test_json_roundtrip_dict():
    d = {"a": 1, "b": [2, 3], "c": "hello"}
    assert aura_json.loads(aura_json.dumps(d)) == d

def test_json_is_valid():
    assert aura_json.is_valid('{"a": 1}') == True

def test_json_is_valid_bad():
    assert aura_json.is_valid("not json") == False

def test_json_pretty():
    result = aura_json.pretty({"a": 1})
    assert isinstance(result, str)

def test_json_stringify():
    result = aura_json.stringify([1, 2, 3])
    assert isinstance(result, str)

# ---------------------------------------------------------------------------
# stdlib.itertools
# ---------------------------------------------------------------------------
def test_range_5():
    assert list(aura_itertools.range_iter(5)) == list(range(5))

def test_range_0():
    assert list(aura_itertools.range_iter(0)) == []

def test_range_step():
    assert list(aura_itertools.range_iter(0, 10, 2)) == list(range(0, 10, 2))

def test_range_negative_step():
    assert list(aura_itertools.range_iter(5, 0, -1)) == list(range(5, 0, -1))

def test_enumerate():
    assert list(aura_itertools.enumerate_iter(["a", "b"])) == [(0, "a"), (1, "b")]

def test_enumerate_start():
    assert list(aura_itertools.enumerate_iter(["a", "b"], 5)) == [(5, "a"), (6, "b")]

def test_zip_iter():
    assert list(aura_itertools.zip_longest([1, 2], ["a", "b"])) == [(1, "a"), (2, "b")]

def test_zip_uneven():
    assert list(aura_itertools.zip_longest([1], ["a", "b"]))[:1] == [(1, "a")]

def test_map_iter():
    result = list(aura_itertools.starmap(lambda x: x * 2, [(1,), (2,), (3,)]))
    assert result == [2, 4, 6]

def test_filter_iter():
    result = list(aura_itertools.filterfalse(lambda x: x <= 2, [1, 2, 3]))
    assert result == [3]

def test_chain_iter():
    assert list(aura_itertools.chain([1, 2], [3, 4])) == [1, 2, 3, 4]

def test_chain_empty():
    assert list(aura_itertools.chain([], [])) == []

def test_repeat_iter():
    assert list(aura_itertools.repeat(42, 3)) == [42, 42, 42]

def test_count_iter():
    assert list(pyit.islice(aura_itertools.count(0, 2), 3)) == [0, 2, 4]

def test_cycle_iter():
    result = list(pyit.islice(aura_itertools.cycle([1, 2]), 5))
    assert result == [1, 2, 1, 2, 1]

def test_dropwhile():
    assert list(aura_itertools.dropwhile(lambda x: x < 3, [1, 2, 3, 4])) == [3, 4]

def test_takewhile():
    assert list(aura_itertools.takewhile(lambda x: x < 3, [1, 2, 3, 4])) == [1, 2]

def test_islice_list():
    assert list(aura_itertools.islice([0, 1, 2, 3, 4], 2, 4)) == [2, 3]

def test_islice_range():
    assert list(aura_itertools.islice(aura_itertools.range_iter(10), 5)) == [0, 1, 2, 3, 4]

def test_pairwise():
    assert list(aura_itertools.pairwise([1, 2, 3])) == [(1, 2), (2, 3)]

def test_groupby():
    result = dict(aura_itertools.groupby([1, 1, 2, 2, 3]))
    assert sorted(result.keys()) == [1, 2, 3]

# ---------------------------------------------------------------------------
# stdlib.python
# ---------------------------------------------------------------------------
PYTHON_INST = [
    ("str", aura_python.is_instance, "hello", str, True),
    ("int", aura_python.is_instance, 42, int, True),
    ("float", aura_python.is_instance, 3.14, float, True),
    ("list", aura_python.is_instance, [], list, True),
    ("dict", aura_python.is_instance, {}, dict, True),
    ("bool", aura_python.is_instance, True, bool, True),
    ("wrong", aura_python.is_instance, "hello", int, False),
]

@pytest.mark.parametrize("name,fn,a,b,expected", PYTHON_INST, ids=[i[0] for i in PYTHON_INST])
def test_python_is_instance(name, fn, a, b, expected):
    assert fn(a, b) == expected

PYTHON_CALL = [
    ("callable_func", aura_python.is_callable, lambda: None, True),
    ("callable_int", aura_python.is_callable, 42, False),
    ("callable_str", aura_python.is_callable, "hello", False),
]

@pytest.mark.parametrize("name,fn,arg,expected", PYTHON_CALL, ids=[i[0] for i in PYTHON_CALL])
def test_python_is_callable(name, fn, arg, expected):
    assert fn(arg) == expected

PYTHON_TYPE = [
    ("int", aura_python.type_name, 42, "builtins.int"),
    ("str", aura_python.type_name, "hello", "builtins.str"),
    ("float", aura_python.type_name, 3.14, "builtins.float"),
    ("list", aura_python.type_name, [], "builtins.list"),
    ("dict", aura_python.type_name, {}, "builtins.dict"),
    ("bool", aura_python.type_name, True, "builtins.bool"),
    ("none", aura_python.type_name, None, "builtins.NoneType"),
]

@pytest.mark.parametrize("name,fn,arg,expected", PYTHON_TYPE, ids=[i[0] for i in PYTHON_TYPE])
def test_python_type_name(name, fn, arg, expected):
    assert fn(arg) == expected

def test_python_interpreter_version():
    result = aura_python.interpreter_version()
    assert result is not None

def test_python_is_class():
    assert aura_python.is_class(int) == True

def test_python_is_class_false():
    assert aura_python.is_class(42) == False

# ---------------------------------------------------------------------------
# stdlib.os
# ---------------------------------------------------------------------------
def test_os_cwd():
    result = aura_os.cwd()
    assert isinstance(result, str) and len(result) > 0

def test_os_pid():
    result = aura_os.pid()
    assert isinstance(result, int) and result > 0

def test_os_platform():
    result = aura_os.platform()
    assert isinstance(result, str)

def test_os_sep():
    result = aura_os.sep()
    assert isinstance(result, str) and len(result) == 1

def test_os_linesep():
    result = aura_os.linesep()
    assert isinstance(result, str)

def test_os_home():
    result = aura_os.home()
    assert isinstance(result, str) and len(result) > 0

def test_os_name():
    result = aura_os.name()
    assert isinstance(result, str)

def test_os_path_join():
    result = aura_os.path_join("a", "b")
    assert "a" in result and "b" in result

def test_os_path_basename():
    result = aura_os.path_basename("/a/b/c.txt")
    assert result == "c.txt"

def test_os_path_dirname():
    result = aura_os.path_dirname("/a/b/c.txt")
    assert "a" in result and "b" in result

def test_os_path_splitext():
    name, ext = aura_os.path_splitext("file.txt")
    assert name == "file" and ext == ".txt"

def test_os_get_env():
    result = aura_os.get_env("PATH")
    assert isinstance(result, str)

def test_os_set_env():
    aura_os.set_env("AURA_TEST_VAR", "test123")
    assert aura_os.get_env("AURA_TEST_VAR") == "test123"
    aura_os.unset_env("AURA_TEST_VAR")

def test_os_temp_dir():
    result = aura_os.temp_dir()
    assert isinstance(result, str) and len(result) > 0

# ---------------------------------------------------------------------------
# stdlib.time
# ---------------------------------------------------------------------------
def test_time_now():
    result = aura_time.now()
    assert isinstance(result, float) and result > 0

def test_time_now_ms():
    result = aura_time.now_ms()
    assert isinstance(result, float) and result > 0

def test_time_timestamp():
    result = aura_time.timestamp()
    assert isinstance(result, float) and result > 0

def test_time_monotonic():
    result = aura_time.monotonic()
    assert isinstance(result, float) and result >= 0

def test_time_perf_counter():
    result = aura_time.perf_counter()
    assert isinstance(result, float) and result >= 0

def test_time_clock():
    result = aura_time.clock()
    assert isinstance(result, float) and result >= 0

def test_time_elapsed():
    start = aura_time.perf_counter()
    result = aura_time.elapsed(start)
    assert isinstance(result, float) and result >= 0

def test_time_iso():
    result = aura_time.iso()
    assert isinstance(result, str) and "T" in result

def test_time_parse():
    result = aura_time.parse("2024-01-15 12:00:00")
    assert result is not None
