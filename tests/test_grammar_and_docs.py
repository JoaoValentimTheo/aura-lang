"""
Grammar compliance and documentation code-block tests.

1. grammar_compliance: for each grammar production in docs/language-reference/grammar.md,
   at least 1 accepted example + 1 rejected example (removed syntax).

2. docs_code_blocks: extract every fenced .aura block from docs/ and README.md,
   run `aura check` on each. Blocks marked ````aura skip`` are expected to fail.
"""

from __future__ import annotations

import glob
import re
import textwrap
from pathlib import Path

import pytest

from aura.parser.to_ast import Parser, Tokenizer
from aura.transpiler.transformer import Transformer
from aura.transpiler.semantics import MutabilityChecker

ROOT = Path(__file__).resolve().parent.parent


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _parse_source(source: str, label: str = "<test>"):
    """Parse an Aura source string. Returns the AST Program."""
    return Parser(Tokenizer(source).tokenize()).parse()


def _check_source(source: str, label: str = "<test>") -> None:
    """Parse + type-check an Aura source string. Raises on failure."""
    program = _parse_source(source, label)
    checker = MutabilityChecker()
    ok = checker.check_program(program)
    if not ok:
        msgs = "; ".join(checker.errors[:3])
        raise AssertionError(f"mutability check failed: {msgs}")
    # Also verify it transpiles to valid Python
    import ast as pyast
    pyast.parse(Transformer().transform(program))


def _should_reject(source: str, label: str = "<test>") -> None:
    """Parse should raise for removed/invalid syntax."""
    with pytest.raises(Exception):
        _parse_source(source, label)


# ---------------------------------------------------------------------------
# 1. Grammar compliance tests
# ---------------------------------------------------------------------------

# Accepted examples: one per grammar production area
GRAMMAR_ACCEPTED = {
    # Lexical
    "line_comment": '// this is a comment',
    "block_comment": '/* multi\nline comment */',
    "identifier": 'let foo_bar = 1',
    "int_literal": 'let x = 42',
    "hex_literal": 'let x = 0xFF',
    "octal_literal": 'let x = 0o77',
    "binary_literal": 'let x = 0b1010',
    "float_literal": 'let x = 3.14',
    "float_scientific": 'let x = 1.5e10',
    "float_no_int": 'let x = .5',
    "bool_literal": 'let x = true',
    "none_literal": 'let x = none',
    "string_single": 'let x = "hello"',
    "string_double": "let x = 'hello'",
    "string_triple": 'let x = """multi\nline"""',
    "string_raw": r'let x = r"\n is literal"',
    "string_fstring": 'let x = f"value is {42}"',
    "string_escape": 'let x = "line\\none"',
    "string_prefix_rb": r'let x = rb"bytes"',

    # Declarations
    "var_let": 'let x = 1',
    "var_let_mut": 'let mut x = 1',
    "var_typed": 'let x: int = 1',
    "const_decl": 'const MAX = 100',
    "var_destructure": 'let (a, b) = (1, 2)',

    # Functions
    "function_basic": 'def add(a: int, b: int) -> int { return a + b }',
    "function_no_params": 'def main() { print("hi") }',
    "function_default": 'def greet(name: str = "world") { print(name) }',
    "function_varargs": 'def f(*args) { }',
    "function_kwargs": 'def f(**kwargs) { }',
    "function_async": 'async def fetch() { }',
    "function_generics": 'def identity[T](x: T) -> T { return x }',

    # Classes
    "class_basic": 'class Point { public let x: float = 0.0\n public let y: float = 0.0 }',
    "class_header_fields": 'class User(private name: str, mut age: int = 0) { }',
    "class_extends": 'class Admin extends User { }',
    "class_abstract": 'abstract class Shape { public abstract def area() -> float }',
    "class_trait": 'trait Drawable { public def draw() -> void }',
    "class_trait_extends": 'trait A extends B { }',
    "class_member_public": 'class C { public let x: int = 0 }',
    "class_member_private": 'class C { private let x: int = 0 }',
    "class_member_protected": 'class C { protected let x: int = 0 }',
    "class_static_method": 'class C { static def create() -> C { } }',
    "class_const_field": 'class C { const PI = 3.14 }',
    "class_nested": 'class Outer { class Inner { } }',

    # Enums
    "enum_basic": 'enum Color { Red, Green, Blue }',
    "enum_values": 'enum Status { Active = 1, Disabled = 0 }',

    # Type alias
    "type_alias": 'type UserId = int',

    # Module
    "module_decl": 'module MyLib { public def hello() { } }',

    # Imports
    "import_simple": 'import stdlib.math',
    "import_as": 'import stdlib.math as m',
    "import_brace": 'import stdlib.math { sqrt, pi }',
    "from_import": 'from stdlib.math import sqrt',
    "from_import_star": 'from stdlib.math import *',
    "import_python": 'import os',

    # Control flow
    "if_else": 'if true { print("yes") } else { print("no") }',
    "if_elseif": 'if false { } else if true { }',
    "unless": 'unless false { print("ok") }',
    "guard": 'guard true else { return }',
    "while_loop": 'while true { break }',
    "until_loop": 'until false { break }',
    "for_in": 'for i in range(10) { print(i) }',
    "for_step": 'for i in 0..10 step 2 { print(i) }',
    "loop_infinite": 'loop { break }',
    "labeled_loop": 'outer: for i in range(10) { break outer }',

    # Match
    "match_basic": 'match 1 { case 1 { print("one") } case _ { print("other") } }',
    "match_guard": 'match x { case n if n > 100 { print("big") } case _ { } }',
    "match_arrow": 'let y = match x { case 1 -> "one" case _ -> "other" }',

    # Try
    "try_catch": 'try { risky() } catch e { handle(e) }',
    "try_finally": 'try { } catch { } finally { cleanup() }',
    "try_typed": 'try { } catch ValueError as e { }',

    # With
    "with_stmt": 'with open("f") as fh { let line = fh.read() }',

    # Expressions
    "ternary": 'let x = true ? 1 : 2',
    "pipe": 'let y = x |> f |> g',
    "elvis": 'let x = a ?: b',
    "coalesce": 'let x = a ?? b',
    "range": 'let r = 0..10',
    "range_exclusive": 'let r = 0..<10',
    "safe_nav": 'let x = obj?.name',
    "safe_index": 'let x = arr?[0]',
    "lambda_single": 'let f = (x) => x + 1',
    "lambda_block": 'let f = (x) => { return x + 1 }',
    "lambda_multi": 'let f = (x, y) => x + y',
    "spread_call": 'f(*args)',
    "kw_spread_call": 'f(**kwargs)',
    "ellipsis_spread": 'f(...items)',

    # Collections
    "list_literal": 'let x = [1, 2, 3]',
    "dict_literal": 'let x = {"a": 1, "b": 2}',
    "set_literal": 'let x = {1, 2, 3}',
    "tuple_literal": 'let x = (1, 2)',
    "list_comp": 'let evens = [x for x in range(10) if x % 2 == 0]',
    "dict_comp": 'let d = {k: v for k, v in items}',
    "set_comp": 'let s = {x for x in range(10)}',
    "gen_expr": 'let g = (x * x for x in range(10))',

    # Operators
    "bitwise_and": 'let x = 5 & 3',
    "bitwise_or": 'let x = 5 | 3',
    "bitwise_xor": 'let x = 5 ^ 3',
    "shift_left": 'let x = 1 << 4',
    "shift_right": 'let x = 128 >> 4',
    "power": 'let x = 2 ** 10',
    "modulo": 'let x = 10 % 3',
    "floor_div_comment": '// this is always a comment, not division',
    "not_keyword": 'let x = not true',
    "and_keyword": 'let x = true and false',
    "or_keyword": 'let x = true or false',
    "is_none": 'let x = a is none',
    "is_not_none": 'let x = a is not none',
    "in_operator": 'let x = 1 in [1, 2, 3]',
    "not_in": 'let x = 1 not in [1, 2, 3]',

    # Type annotations
    "type_union": 'let x: int | str = 1',
    "type_optional": 'let x: int? = none',
    "type_list": 'let x: [int] = []',
    "type_dict": 'let x: {str: int} = {}',
    "type_function": 'type Fn = (int, str) -> bool',
    "type_structural": 'type U = {name: str, age: int}',
    "as_cast": 'let x = val as int',

    # Assertions
    "assert_basic": 'assert true',
    "assert_message": 'assert x > 0, "must be positive"',

    # Return / throw / break / continue
    "return_val": 'def f() -> int { return 42 }',
    "throw_val": 'throw ValueError("bad")',
    "break_label": 'outer: loop { break outer }',
    "continue_label": 'outer: for i in range(10) { continue outer }',

    # Yield / spawn
    "yield_expr": 'def gen() { yield 1 }',
    "spawn_stmt": 'spawn thread { print("bg") }',

    # Decorators
    "decorator_simple": '@debug\ndef f() { }',
    "decorator_args": '@deprecated("use new_func")\ndef old() { }',

    # Augmented assignment
    "augmented_add": 'let mut x = 1\nx += 1',
    "augmented_sub": 'let mut x = 1\nx -= 1',
    "augmented_mul": 'let mut x = 1\nx *= 2',
    "augmented_pow": 'let mut x = 2\nx **= 3',
    "augmented_null": 'let mut x = none\nx ??= 1',

    # String operations
    "fstring_format": 'let x = f"{3.14:.2f}"',
    "fstring_nested": "let x = f'result: {f\"{42}\"}'",

    # Comprehension with multiple if
    "multi_if_comp": 'let x = [n for n in range(100) if n > 0 if n < 10]',
}

# Rejected examples: removed/invalid syntax
GRAMMAR_REJECTED = {
    "old_class_inheritance": 'class Admin(User) { }',
    "old_implements": 'class C implements Drawable { }',
    "old_fn_syntax": 'fn(x) { return x }',
    "old_if_then": 'if true then print("yes")',
    "double_slash_in_expr": 'let x = 5 / / 2',
}


@pytest.mark.parametrize("label,source", sorted(GRAMMAR_ACCEPTED.items()),
                         ids=[k for k in sorted(GRAMMAR_ACCEPTED)])
def test_grammar_accepted(label: str, source: str) -> None:
    """Each accepted grammar example must parse and type-check."""
    _check_source(source, label=label)


@pytest.mark.parametrize("label,source", sorted(GRAMMAR_REJECTED.items()),
                         ids=[k for k in sorted(GRAMMAR_REJECTED)])
def test_grammar_rejected(label: str, source: str) -> None:
    """Each rejected grammar example must fail to parse."""
    _should_reject(source, label=label)


# ---------------------------------------------------------------------------
# 2. Documentation code-block tests
# ---------------------------------------------------------------------------

def _extract_aura_blocks(md_path: Path) -> list[tuple[int, str, bool]]:
    """Extract fenced .aura code blocks from a Markdown file.

    Returns list of (line_number, code, skip_flag).
    skip_flag is True if the block is tagged ````aura skip``.
    """
    text = md_path.read_text(encoding="utf-8")
    blocks: list[tuple[int, str, bool]] = []
    in_block = False
    skip = False
    start = 0
    lines: list[str] = []
    for i, line in enumerate(text.splitlines(), 1):
        if not in_block:
            m = re.match(r'^```aura(?:\s+skip)?\s*$', line)
            if m:
                in_block = True
                skip = "skip" in line
                start = i
                lines = []
        else:
            if line.strip() == "```":
                in_block = False
                blocks.append((start, "\n".join(lines), skip))
            else:
                lines.append(line)
    return blocks


def _find_docs_with_aura() -> list[Path]:
    """Find all .md files under docs/ and README.md that contain ```aura blocks."""
    paths = []
    for md in [ROOT / "README.md"] + sorted((ROOT / "docs").rglob("*.md")):
        if md.read_text(encoding="utf-8").find("```aura") != -1:
            paths.append(md)
    return paths


@pytest.fixture(params=_find_docs_with_aura(), ids=lambda p: str(p.relative_to(ROOT)))
def doc_aura_blocks(request) -> list[tuple[int, str, bool]]:
    return _extract_aura_blocks(request.param)


def test_docs_code_blocks_compile(doc_aura_blocks: list[tuple[int, str, bool]]) -> None:
    """Every ```aura block in docs must pass aura check (unless tagged skip)."""
    errors = []
    for lineno, code, skip in doc_aura_blocks:
        if skip:
            continue
        try:
            _check_source(code, label=f"<doc line {lineno}>")
        except Exception as exc:
            errors.append(f"  line {lineno}: {exc}")
    if errors:
        pytest.fail("Documentation code blocks that failed aura check:\n" + "\n".join(errors))


# ---------------------------------------------------------------------------
# 3. Smoke: the full example corpus still transpiles
# ---------------------------------------------------------------------------

def test_example_corpus_transpiles() -> None:
    """Every .aura file in examples/ must parse and transpile."""
    aura_files = sorted(glob.glob(str(ROOT / "examples" / "*.aura")))
    assert aura_files, "no .aura files found in examples/"
    errors = []
    for path in aura_files:
        try:
            program = Parser(Tokenizer(Path(path).read_text()).tokenize()).parse()
            checker = MutabilityChecker()
            checker.check_program(program)
            import ast as pyast
            pyast.parse(Transformer().transform(program))
        except Exception as exc:
            errors.append(f"  {Path(path).name}: {exc}")
    if errors:
        pytest.fail("Example files that failed:\n" + "\n".join(errors))
