"""Parser fuzz and exhaustive tests — batch 1.

Heavy parametrization to stress the tokenizer and parser with
hundreds of input variations. No Hypothesis dependency needed.
"""
from __future__ import annotations

import pytest

from aura.parser.to_ast import Tokenizer, Parser, parse_file


def _tokenize(src):
    return list(Tokenizer(src).tokenize())


def _parse(src):
    return Parser(Tokenizer(src).tokenize()).parse()


# ---------------------------------------------------------------------------
# Tokenizer — identifiers
# ---------------------------------------------------------------------------

IDENT_SCRIPTS = [
    ("latin", "hello"),
    ("latin_upper", "Hello"),
    ("latin_underscore", "_name"),
    ("latin_digit", "x1"),
    ("cjk", "変数"),
    ("cyrillic", "переменная"),
    ("arabic", "متغير"),
    ("greek", "μεταβλητή"),
    ("devanagari", "चर"),
    ("thai", "ตัวแปร"),
    ("korean", "변수"),
    ("mixed", "hello_мир_你好"),
    ("turkish_dotless", "Istanbul"),
    ("polish_lstroke", "Lodz"),
    ("german_umlaut", "Uber"),
    ("supplementary_gothic", "\U00010330test"),
    ("supplementary_math", "\U0001D400"),
]


@pytest.mark.parametrize("name,ident", IDENT_SCRIPTS, ids=[i[0] for i in IDENT_SCRIPTS])
def test_identifier_tokenizes(name, ident):
    tokens = _tokenize(f"let {ident} = 1")
    idents = [t for t in tokens if t.type == "IDENT" and t.value != "let"]
    assert idents
    assert idents[0].value == ident


# ---------------------------------------------------------------------------
# Tokenizer — string literals
# ---------------------------------------------------------------------------

STRING_CASES = [
    ("empty", '""', ""),
    ("simple", '"hello"', "hello"),
    ("single_quote", "'world'", "world"),
    ("triple_double", '"""multi\nline"""', "multi\nline"),
    ("triple_single", "'''also\nmulti'''", "also\nmulti"),
    ("escape_newline", r'"a\nb"', "a\nb"),
    ("escape_tab", r'"a\tb"', "a\tb"),
    ("escape_backslash", r'"a\\b"', "a\\b"),
    ("escape_quote", r'"a\"b"', 'a"b'),
    ("hex_escape", r'"a\x41b"', "aAb"),
    ("unicode_bmp", r'"caf\u00e9"', "café"),
    ("unicode_supplementary", r'"gothic\U00010330"', "gothic\U00010330"),
    ("raw_string", r'r"no\nescape"', r"no\nescape"),
    ("fstring_simple", 'f"hello {1 + 2}"', None),
    ("fstring_nested", 'f"{"nested"}"', None),
    ("unicode_content", '"日本語"', "日本語"),
    ("emoji", '"🎉🎊🎈"', "🎉🎊🎈"),
    ("mixed_script", '"café 日本語 мир"', "café 日本語 мир"),
]


@pytest.mark.parametrize("name,src,expected", STRING_CASES,
                         ids=[i[0] for i in STRING_CASES])
def test_string_literal(name, src, expected):
    tokens = _tokenize(f"let x = {src}")
    str_tokens = [t for t in tokens if t.type in ("STRING", "RAWSTRING", "FSTRING")]
    assert str_tokens, f"No string token found in {tokens}"


# ---------------------------------------------------------------------------
# Tokenizer — integer literals
# ---------------------------------------------------------------------------

INT_CASES = [
    ("decimal", "42"),
    ("zero", "0"),
    ("underscore", "1_000_000"),
    ("hex_lower", "0xff"),
    ("hex_upper", "0XFF"),
    ("hex_underscore", "0xFF_FF"),
    ("octal", "0o77"),
    ("binary", "0b1010"),
    ("large", "999999999999999999"),
]


@pytest.mark.parametrize("name,lit", INT_CASES, ids=[i[0] for i in INT_CASES])
def test_integer_literal(name, lit):
    tokens = _tokenize(f"let x = {lit}")
    int_tokens = [t for t in tokens if t.type == "INT"]
    assert int_tokens


# ---------------------------------------------------------------------------
# Tokenizer — float literals
# ---------------------------------------------------------------------------

FLOAT_CASES = [
    ("simple", "3.14"),
    ("no_int", ".5"),
    ("scientific", "1e10"),
    ("scientific_neg", "1e-5"),
    ("scientific_plus", "1.5E+3"),
    ("underscore", "1_000.5"),
]


@pytest.mark.parametrize("name,lit", FLOAT_CASES, ids=[i[0] for i in FLOAT_CASES])
def test_float_literal(name, lit):
    tokens = _tokenize(f"let x = {lit}")
    float_tokens = [t for t in tokens if t.type == "FLOAT"]
    assert float_tokens


# ---------------------------------------------------------------------------
# Tokenizer — operators and punctuation
# ---------------------------------------------------------------------------

OPERATOR_CASES = [
    ("plus", "+"),
    ("minus", "-"),
    ("star", "*"),
    ("slash", "/"),
    ("percent", "%"),
    ("power", "**"),
    ("eq", "=="),
    ("neq", "!="),
    ("lt", "<"),
    ("gt", ">"),
    ("lte", "<="),
    ("gte", ">="),
    ("pipe", "|>"),
    ("ampersand", "&"),
    ("caret", "^"),
    ("tilde", "~"),
    ("assign", "="),
    ("plus_assign", "+="),
    ("minus_assign", "-="),
    ("star_assign", "*="),
    ("arrow", "->"),
    ("fat_arrow", "=>"),
    ("range", ".."),
    ("null_coalesce", "??"),
    ("spread", "..."),
    ("question", "?"),
    ("colon", ":"),
    ("semicolon", ";"),
    ("dot", "."),
    ("comma", ","),
    ("lparen", "("),
    ("rparen", ")"),
    ("lbracket", "["),
    ("rbracket", "]"),
    ("lbrace", "{"),
    ("rbrace", "}"),
]


@pytest.mark.parametrize("name,op", OPERATOR_CASES, ids=[i[0] for i in OPERATOR_CASES])
def test_operator_tokenizes(name, op):
    tokens = _tokenize(f"x {op} y")
    op_tokens = [t for t in tokens if t.type == "OP"]
    assert op_tokens


# ---------------------------------------------------------------------------
# Tokenizer — keywords
# ---------------------------------------------------------------------------

KEYWORDS = [
    "let", "mut", "const", "def", "new", "class", "trait", "enum",
    "extends", "if", "else", "match", "case", "for", "while", "loop",
    "return", "break", "continue", "throw", "try", "catch", "finally",
    "guard", "import", "export", "as", "from", "none", "true", "false",
    "this", "async", "await", "yield", "type", "where", "public",
    "private", "protected", "abstract", "static", "override",
]


@pytest.mark.parametrize("kw", KEYWORDS)
def test_keyword_tokenizes(kw):
    tokens = _tokenize(f"let {kw} = 1")
    ident_tokens = [t for t in tokens if t.type == "IDENT"]
    assert ident_tokens


# ---------------------------------------------------------------------------
# Tokenizer — comments
# ---------------------------------------------------------------------------

COMMENT_CASES = [
    ("line_simple", "// comment"),
    ("line_empty", "//"),
    ("line_unicode", "// 日本語コメント"),
    ("line_emoji", "// 🎉"),
    ("block_simple", "/* comment */"),
    ("block_multiline", "/* line1\nline2\nline3 */"),
    ("block_unicode", "/* تعليق */"),
    ("block_emoji", "/* 🎉🎉🎉 */"),
    ("line_before_code", "// comment\nlet x = 1"),
    ("block_before_code", "/* comment */ let x = 1"),
]


@pytest.mark.parametrize("name,src", COMMENT_CASES, ids=[i[0] for i in COMMENT_CASES])
def test_comments(name, src):
    tokens = _tokenize(src)
    assert tokens


# ---------------------------------------------------------------------------
# Parser — basic programs
# ---------------------------------------------------------------------------

PROGRAM_CASES = [
    ("empty", ""),
    ("single_var", "let x = 1"),
    ("single_const", "const PI = 3.14"),
    ("single_mut", "let mut x = 1"),
    ("multiple_vars", "let a = 1\nlet b = 2\nlet c = 3"),
    ("function_def", "def add(a: int, b: int) -> int {\n  return a + b\n}"),
    ("function_no_params", "def noop() {\n  pass\n}"),
    ("function_varargs", "def f(*args) {\n  pass\n}"),
    ("function_kwargs", "def f(**kwargs) {\n  pass\n}"),
    ("if_else", "if true {\n  let x = 1\n} else {\n  let x = 2\n}"),
    ("if_elif_else", "if false {\n  let x = 1\n} else if true {\n  let x = 2\n} else {\n  let x = 3\n}"),
    ("for_loop", "for i in [1, 2, 3] {\n  print(i)\n}"),
    ("while_loop", "while false {\n  break\n}"),
    ("loop_infinite", "loop {\n  break\n}"),
    ("match_simple", 'match x {\n  case 1 { "one" }\n  case _ { "other" }\n}'),
    ("match_guard", 'match x {\n  case n if n > 100 { "big" }\n  case _ { "small" }\n}'),
    ("try_catch", "try {\n  let x = 1\n} catch e {\n  print(e)\n}"),
    ("try_finally", "try {\n  let x = 1\n} finally {\n  print(\"done\")\n}"),
    ("class_simple", "class Point {\n  public x: int\n  public y: int\n}"),
    ("class_inherits", "class Dog extends Animal {\n  public name: str\n}"),
    ("class_header_fields", "class User(private name: str, mut age: int = 0) {\n}"),
    ("trait_simple", "trait Drawable {\n  public def draw()\n}"),
    ("enum_simple", "enum Color { Red, Green, Blue }"),
    ("enum_with_values", 'enum Status { Ok = 0, Error = 1, Unknown = 2 }'),
    ("lambda_simple", "let f = (x) => x + 1"),
    ("lambda_multi", "let f = (a, b) => a + b"),
    ("lambda_block", "let f = (x) => {\n  let y = x * 2\n  y\n}"),
    ("pipe_chain", "x |> f |> g |> h"),
    ("ternary", "let x = true ? 1 : 2"),
    ("fstring", 'let x = f"hello {name}"'),
    ("import_stdlib", 'import stdlib.math as math'),
    ("import_python", "import python"),
    ("from_import", "from stdlib.string import to_upper"),
    ("module_with_export", "module M {\n  export def helper() {\n    42\n  }\n}"),
    ("async_function", "async def fetch() {\n  await http.get(url)\n}"),
    ("generator", "def gen() {\n  yield 1\n  yield 2\n  yield 3\n}"),
    ("complex_nested", "def f() {\n  if true {\n    for x in [1] {\n      match x {\n        case 1 { return }\n        case _ { }\n      }\n    }\n  }\n}"),
]


@pytest.mark.parametrize("name,src", PROGRAM_CASES, ids=[i[0] for i in PROGRAM_CASES])
def test_program_parses(name, src):
    tree = _parse(src)
    assert tree is not None


# ---------------------------------------------------------------------------
# Parser — expressions
# ---------------------------------------------------------------------------

EXPR_CASES = [
    ("int_literal", "42"),
    ("float_literal", "3.14"),
    ("string_literal", '"hello"'),
    ("bool_true", "true"),
    ("bool_false", "false"),
    ("none_literal", "none"),
    ("identifier", "x"),
    ("binary_add", "1 + 2"),
    ("binary_sub", "1 - 2"),
    ("binary_mul", "2 * 3"),
    ("binary_div", "6 / 2"),
    ("binary_mod", "10 % 3"),
    ("binary_power", "2 ** 10"),
    ("binary_floor_div", "7 // 2"),
    ("binary_eq", "1 == 1"),
    ("binary_neq", "1 != 2"),
    ("binary_lt", "1 < 2"),
    ("binary_gt", "2 > 1"),
    ("binary_lte", "1 <= 1"),
    ("binary_gte", "2 >= 2"),
    ("binary_and", "true and false"),
    ("binary_or", "true or false"),
    ("unary_not", "not true"),
    ("unary_minus", "-5"),
    ("unary_plus", "+5"),
    ("unary_bitwise_not", "~0"),
    ("call_simple", "f()"),
    ("call_one_arg", "f(1)"),
    ("call_two_args", "f(1, 2)"),
    ("call_kwargs", 'f(name="test")'),
    ("call_spread", "f(*args)"),
    ("call_dict_spread", "f(**kwargs)"),
    ("member_access", "obj.field"),
    ("method_call", "obj.method()"),
    ("index_access", "arr[0]"),
    ("slice", "arr[1:3]"),
    ("ternary", "true ? 1 : 2"),
    ("parenthesized", "(1 + 2)"),
    ("list_literal", "[1, 2, 3]"),
    ("dict_literal", '{"a": 1, "b": 2}'),
    ("tuple_literal", "(1, 2, 3)"),
    ("set_literal", "{1, 2, 3}"),
    ("spread_in_list", "[*a, 3]"),
    ("spread_in_dict", '{"x": 1, **d}'),
    ("fstring_simple", 'f"hello"'),
    ("fstring_expr", 'f"{1 + 2}"'),
    ("fstring_format", 'f"{x:.2f}"'),
    ("lambda_simple", "(x) => x + 1"),
    ("lambda_multi", "(a, b) => a + b"),
    ("pipe", "x |> f"),
    ("pipe_chain", "x |> f |> g"),
    ("null_coalesce", "x ?? default"),
    ("range", "1..10"),
    ("range_step", "1..10..2"),
]


@pytest.mark.parametrize("name,src", EXPR_CASES, ids=[i[0] for i in EXPR_CASES])
def test_expression_parses(name, src):
    tree = _parse(f"let x = {src}")
    assert tree is not None


# ---------------------------------------------------------------------------
# Parser — string edge cases
# ---------------------------------------------------------------------------

STRING_EDGE_CASES = [
    ("backslash_at_end", r'"test\\"'),
    ("null_char", '"test\x00end"'),
    ("newline_in_triple", '"""line1\nline2"""'),
    ("tab_in_string", '"col1\tcol2"'),
    ("carriage_return", '"line1\rline2"'),
    ("mixed_quotes", """'he said "hello"'"""),
    ("nested_fstring", 'f"{f\\"{x}\\"}"'),
    ("fstring_braces", 'f"{{literal}}"'),
    ("long_string", '"' + "a" * 1000 + '"'),
    ("unicode_escaped", r'"\u0041\u0042\u0043"'),
    ("supplementary_escaped", r'"\U0001F600"'),
]


@pytest.mark.parametrize("name,src", STRING_EDGE_CASES, ids=[i[0] for i in STRING_EDGE_CASES])
def test_string_edge_cases(name, src):
    tokens = _tokenize(f"let x = {src}")
    assert tokens


# ---------------------------------------------------------------------------
# Parser — error recovery
# ---------------------------------------------------------------------------

ERROR_CASES = [
    ("missing_rparen", "f(1, 2"),
    ("missing_rbrace", "def f() { let x = 1"),
    ("missing_rbracket", "let x = [1, 2, 3"),
    ("double_equal", "let x = == 1"),
    ("missing_colon_type", "let x: = 1"),
    ("invalid_operator", "let x = 1 @ 2"),
    ("missing_value", "let x ="),
    ("double_let", "let let x = 1"),
]


@pytest.mark.parametrize("name,src", ERROR_CASES, ids=[i[0] for i in ERROR_CASES])
def test_parser_error_recovery(name, src):
    """Parser should raise SyntaxError, not crash."""
    with pytest.raises(SyntaxError):
        _parse(src)
