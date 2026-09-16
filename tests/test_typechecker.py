"""Unit tests for the Aura type system: inference, checking and narrowing."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Tokenizer, Parser  # noqa: E402
from transpiler.types import (  # noqa: E402
    TypeChecker, TypeInference,
    IntType, FloatType, StrType, BoolType, NoneType, AnyType,
    ListType, DictType, SetType, TupleType, UnionType,
)


def check_source(source: str):
    """Parse and type-check ``source``; return (ok, errors)."""
    program = Parser(Tokenizer(source).tokenize()).parse()
    checker = TypeChecker()
    ok = checker.check_program(program)
    return ok, checker.errors


def infer(source: str):
    """Infer the type of the value in `let x = <expr>` style source."""
    program = Parser(Tokenizer(source).tokenize()).parse()
    return TypeInference().infer(program.statements[0].value)


# ============================================================================
# Inference
# ============================================================================

def test_infer_int_literal():
    assert infer("let x = 42") == IntType()


def test_infer_float_literal():
    assert infer("let x = 3.14") == FloatType()


def test_infer_string_literal():
    assert infer('let x = "hello"') == StrType()


def test_infer_bool_literal():
    assert infer("let x = true") == BoolType()


def test_infer_none_literal():
    assert infer("let x = none") == NoneType()


def test_infer_list_of_ints():
    t = infer("let x = [1, 2, 3]")
    assert isinstance(t, ListType)
    assert t.element_type == IntType()


def test_infer_empty_list():
    t = infer("let x = []")
    assert isinstance(t, ListType)
    assert isinstance(t.element_type, AnyType)


def test_infer_dict():
    t = infer('let x = {name: "Alice"}')
    assert isinstance(t, DictType)
    assert t.value_type == StrType()


def test_infer_set():
    t = infer("let x = {1, 2, 3}")
    assert isinstance(t, SetType)
    assert t.element_type == IntType()


def test_infer_tuple():
    t = infer('let x = (1, "a")')
    assert isinstance(t, TupleType)
    assert t.element_types == [IntType(), StrType()]


def test_infer_int_addition():
    assert infer("let x = 1 + 2") == IntType()


def test_infer_float_addition():
    assert infer("let x = 1 + 2.0") == FloatType()


def test_infer_string_concatenation():
    assert infer('let x = "a" + "b"') == StrType()


def test_infer_comparison_is_bool():
    assert infer("let x = 1 < 2") == BoolType()


def test_infer_logical_is_bool():
    assert infer("let x = true and false") == BoolType()


def test_infer_unary_not_is_bool():
    assert infer("let x = not true") == BoolType()


def test_infer_range_is_list_of_int():
    t = infer("let x = 1..10")
    assert isinstance(t, ListType)
    assert t.element_type == IntType()


def test_infer_fstring_is_str():
    assert infer('let x = f"a{1}b"') == StrType()


# ============================================================================
# Checking: variable declarations
# ============================================================================

def test_valid_int_declaration():
    ok, errors = check_source("let x: int = 5")
    assert ok and errors == []


def test_invalid_int_declaration():
    ok, errors = check_source('let x: int = "hello"')
    assert not ok
    assert any("expected Int" in e and "got String" in e for e in errors)


def test_valid_str_declaration():
    ok, _ = check_source('let s: str = "hi"')
    assert ok


def test_invalid_str_declaration():
    ok, _ = check_source("let s: str = 42")
    assert not ok


def test_valid_float_declaration():
    ok, _ = check_source("let f: float = 3.14")
    assert ok


def test_valid_list_declaration():
    ok, _ = check_source("let xs: [int] = [1, 2, 3]")
    assert ok


def test_invalid_list_declaration():
    ok, _ = check_source("let xs: [int] = 5")
    assert not ok


def test_valid_union_declaration():
    ok, _ = check_source('let x: int | str = "a"')
    assert ok


def test_valid_optional_declaration():
    ok, _ = check_source("let x: str? = none")
    assert ok


def test_any_accepts_everything():
    ok, _ = check_source("let x: any = 5")
    assert ok
    ok, _ = check_source('let x: any = "s"')
    assert ok


def test_unknown_type_is_lenient():
    ok, _ = check_source("let x: MyClass = foo()")
    assert ok


def test_const_type_mismatch():
    ok, errors = check_source('const X: int = "nope"')
    assert not ok
    assert any("Constant 'X'" in e for e in errors)


# ============================================================================
# Checking: binary operators
# ============================================================================

def test_valid_number_addition():
    ok, _ = check_source("let x = 1 + 2")
    assert ok


def test_invalid_int_plus_string():
    ok, errors = check_source('let x = 1 + "a"')
    assert not ok
    assert any("cannot combine" in e for e in errors)


def test_invalid_string_subtraction():
    ok, errors = check_source('let x = "a" - "b"')
    assert not ok
    assert any("requires numbers" in e for e in errors)


def test_valid_string_repetition():
    ok, _ = check_source('let x = "ab" * 3')
    assert ok


def test_invalid_comparison():
    ok, _ = check_source('let x = 1 < "a"')
    assert not ok


def test_valid_string_comparison():
    ok, _ = check_source('let x = "a" < "b"')
    assert ok


def test_invalid_bitwise_on_strings():
    ok, _ = check_source('let x = "a" & "b"')
    assert not ok


def test_valid_bitwise_on_ints():
    ok, _ = check_source("let x = 5 & 3")
    assert ok


# ============================================================================
# Checking: control flow
# ============================================================================

def test_valid_if_condition():
    ok, _ = check_source('if 1 == 1 { print("ok") }')
    assert ok


def test_invalid_if_condition():
    ok, errors = check_source('if 5 { print("ok") }')
    assert not ok
    assert any("if condition must be Bool" in e for e in errors)


def test_valid_while_condition():
    ok, _ = check_source("let mut i = 0\nwhile i < 3 { i += 1 }")
    assert ok


def test_invalid_while_condition():
    ok, _ = check_source('while "x" { print("no") }')
    assert not ok


# ============================================================================
# Checking: functions and returns
# ============================================================================

def test_valid_return_type():
    ok, _ = check_source("def f() -> int { return 5 }")
    assert ok


def test_invalid_return_type():
    ok, errors = check_source('def f() -> int { return "x" }')
    assert not ok
    assert any("Return type mismatch" in e for e in errors)


def test_function_arity_too_many():
    ok, errors = check_source("def f(a) { return a }\nf(1, 2)")
    assert not ok
    assert any("expects 1 argument" in e for e in errors)


def test_function_arity_ok():
    ok, _ = check_source("def f(a, b) { return a }\nf(1, 2)")
    assert ok


# ============================================================================
# Checking: narrowing
# ============================================================================

def test_narrow_not_null_then():
    source = "let x: int | none = get()\nif x != none { let y: int = x }"
    ok, errors = check_source(source)
    assert ok, errors


def test_narrow_eq_null_else():
    source = 'let x: int | none = get()\nif x == none { print("n") } else { let y: int = x }'
    ok, errors = check_source(source)
    assert ok, errors


def test_narrow_is_type():
    source = "def f(v) { if v is int { let y: int = v } }"
    ok, errors = check_source(source)
    assert ok, errors


# ============================================================================
# Type compatibility helpers
# ============================================================================

def test_any_is_compatible_with_all():
    assert IntType().is_compatible(AnyType())
    assert AnyType().is_compatible(IntType())


def test_distinct_primitives_incompatible():
    assert not IntType().is_compatible(StrType())
    assert not StrType().is_compatible(BoolType())


def test_union_compatibility():
    union = UnionType({IntType(), StrType()})
    assert union.is_compatible(IntType())
    assert union.is_compatible(StrType())
    assert not union.is_compatible(BoolType())


if __name__ == "__main__":
    import traceback

    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    failed = 0
    for test in tests:
        try:
            test()
            print(f"PASS {test.__name__}")
        except Exception:
            failed += 1
            print(f"FAIL {test.__name__}")
            traceback.print_exc()
    print(f"\n{len(tests) - failed}/{len(tests)} type checker tests passed")
    sys.exit(1 if failed else 0)