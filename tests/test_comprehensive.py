"""Comprehensive test suite with 70+ tests covering all Aura features."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from transpiler.transformer import Transformer
from transpiler.ast import *


# ============================================================================
# TESTS: Basic Literals and Variables (10 tests)
# ============================================================================

def test_int_literal():
    """Test integer literals."""
    ast = Program([ExprStmt(IntLiteral(42))])
    t = Transformer()
    output = t.transform(ast)
    assert "42" in output
    print("✓ test_int_literal")

def test_float_literal():
    """Test float literals."""
    ast = Program([ExprStmt(FloatLiteral(3.14))])
    t = Transformer()
    output = t.transform(ast)
    assert "3.14" in output
    print("✓ test_float_literal")

def test_string_literal():
    """Test string literals."""
    ast = Program([ExprStmt(StrLiteral("hello"))])
    t = Transformer()
    output = t.transform(ast)
    assert "'hello'" in output or '"hello"' in output
    print("✓ test_string_literal")

def test_bool_literal():
    """Test boolean literals."""
    ast = Program([
        ExprStmt(BoolLiteral(True)),
        ExprStmt(BoolLiteral(False)),
    ])
    t = Transformer()
    output = t.transform(ast)
    assert "True" in output and "False" in output
    print("✓ test_bool_literal")

def test_none_literal():
    """Test none/null literal."""
    ast = Program([ExprStmt(NoneLiteral())])
    t = Transformer()
    output = t.transform(ast)
    assert "None" in output
    print("✓ test_none_literal")

def test_var_decl():
    """Test variable declaration."""
    ast = Program([VarDecl("x", False, None, IntLiteral(10))])
    t = Transformer()
    output = t.transform(ast)
    assert "x = 10" in output
    print("✓ test_var_decl")

def test_var_mutable():
    """Test mutable variable declaration."""
    ast = Program([VarDecl("x", True, None, IntLiteral(5))])
    t = Transformer()
    output = t.transform(ast)
    assert "x = 5" in output
    print("✓ test_var_mutable")

def test_const_decl():
    """Test const declaration."""
    ast = Program([ConstDecl("MAX", None, IntLiteral(100))])
    t = Transformer()
    output = t.transform(ast)
    assert "MAX" in output
    print("✓ test_const_decl")

def test_identifier():
    """Test identifier expression."""
    ast = Program([ExprStmt(Identifier("myVar"))])
    t = Transformer()
    output = t.transform(ast)
    assert "myVar" in output
    print("✓ test_identifier")

def test_var_with_type():
    """Test variable with type annotation."""
    ast = Program([VarDecl("name", False, SimpleType("str"), StrLiteral("Alice"))])
    t = Transformer()
    output = t.transform(ast)
    assert "name" in output and "Alice" in output
    print("✓ test_var_with_type")


# ============================================================================
# TESTS: Collection Literals (8 tests)
# ============================================================================

def test_list_literal():
    """Test list literals."""
    ast = Program([ExprStmt(ListLiteral([IntLiteral(1), IntLiteral(2), IntLiteral(3)]))])
    t = Transformer()
    output = t.transform(ast)
    assert "[" in output and "1" in output
    print("✓ test_list_literal")

def test_empty_list():
    """Test empty list literal."""
    ast = Program([ExprStmt(ListLiteral([]))])
    t = Transformer()
    output = t.transform(ast)
    assert "[]" in output
    print("✓ test_empty_list")

def test_dict_literal():
    """Test dictionary literals."""
    ast = Program([ExprStmt(DictLiteral([
        (StrLiteral("name"), StrLiteral("Alice")),
        (StrLiteral("age"), IntLiteral(25)),
    ]))])
    t = Transformer()
    output = t.transform(ast)
    assert "{" in output and "name" in output
    print("✓ test_dict_literal")

def test_set_literal():
    """Test set literals."""
    ast = Program([ExprStmt(SetLiteral([IntLiteral(1), IntLiteral(2), IntLiteral(3)]))])
    t = Transformer()
    output = t.transform(ast)
    assert "{" in output
    print("✓ test_set_literal")

def test_tuple_literal():
    """Test tuple literals."""
    ast = Program([ExprStmt(TupleLiteral([IntLiteral(1), StrLiteral("two"), IntLiteral(3)]))])
    t = Transformer()
    output = t.transform(ast)
    assert "(" in output
    print("✓ test_tuple_literal")

def test_empty_dict():
    """Test empty dict literal."""
    ast = Program([ExprStmt(DictLiteral([]))])
    t = Transformer()
    output = t.transform(ast)
    assert "{}" in output
    print("✓ test_empty_dict")

def test_nested_list():
    """Test nested lists."""
    ast = Program([ExprStmt(ListLiteral([
        ListLiteral([IntLiteral(1), IntLiteral(2)]),
        ListLiteral([IntLiteral(3), IntLiteral(4)]),
    ]))])
    t = Transformer()
    output = t.transform(ast)
    assert "[[" in output or ("1" in output and "2" in output)
    print("✓ test_nested_list")

def test_mixed_dict():
    """Test dictionary with mixed values."""
    ast = Program([ExprStmt(DictLiteral([
        (StrLiteral("count"), IntLiteral(42)),
        (StrLiteral("items"), ListLiteral([IntLiteral(1), IntLiteral(2)])),
    ]))])
    t = Transformer()
    output = t.transform(ast)
    assert "count" in output
    print("✓ test_mixed_dict")


# ============================================================================
# TESTS: Binary and Unary Operators (10 tests)
# ============================================================================

def test_binary_add():
    """Test addition operator."""
    ast = Program([ExprStmt(BinaryOp(IntLiteral(10), "+", IntLiteral(5)))])
    t = Transformer()
    output = t.transform(ast)
    assert "+" in output
    print("✓ test_binary_add")

def test_binary_sub():
    """Test subtraction operator."""
    ast = Program([ExprStmt(BinaryOp(IntLiteral(10), "-", IntLiteral(3)))])
    t = Transformer()
    output = t.transform(ast)
    assert "-" in output
    print("✓ test_binary_sub")

def test_binary_mul():
    """Test multiplication operator."""
    ast = Program([ExprStmt(BinaryOp(IntLiteral(4), "*", IntLiteral(5)))])
    t = Transformer()
    output = t.transform(ast)
    assert "*" in output
    print("✓ test_binary_mul")

def test_binary_div():
    """Test division operator."""
    ast = Program([ExprStmt(BinaryOp(IntLiteral(20), "/", IntLiteral(4)))])
    t = Transformer()
    output = t.transform(ast)
    assert "/" in output
    print("✓ test_binary_div")

def test_binary_comparison():
    """Test comparison operators."""
    ast = Program([
        ExprStmt(BinaryOp(IntLiteral(5), ">", IntLiteral(3))),
        ExprStmt(BinaryOp(IntLiteral(5), "<", IntLiteral(10))),
        ExprStmt(BinaryOp(IntLiteral(5), "==", IntLiteral(5))),
    ])
    t = Transformer()
    output = t.transform(ast)
    assert ">" in output and "<" in output and "==" in output
    print("✓ test_binary_comparison")

def test_binary_logical():
    """Test logical operators."""
    ast = Program([
        ExprStmt(BinaryOp(BoolLiteral(True), "and", BoolLiteral(False))),
        ExprStmt(BinaryOp(BoolLiteral(True), "or", BoolLiteral(False))),
    ])
    t = Transformer()
    output = t.transform(ast)
    assert "and" in output and "or" in output
    print("✓ test_binary_logical")

def test_unary_not():
    """Test not operator."""
    ast = Program([ExprStmt(UnaryOp("not", BoolLiteral(True)))])
    t = Transformer()
    output = t.transform(ast)
    assert "not" in output
    print("✓ test_unary_not")

def test_unary_neg():
    """Test negation operator."""
    ast = Program([ExprStmt(UnaryOp("-", IntLiteral(42)))])
    t = Transformer()
    output = t.transform(ast)
    assert "-" in output
    print("✓ test_unary_neg")

def test_binary_in():
    """Test 'in' operator."""
    ast = Program([ExprStmt(BinaryOp(IntLiteral(2), "in", ListLiteral([IntLiteral(1), IntLiteral(2), IntLiteral(3)])))])
    t = Transformer()
    output = t.transform(ast)
    assert "in" in output
    print("✓ test_binary_in")


# ============================================================================
# TESTS: Control Flow (12 tests)
# ============================================================================

def test_if_stmt():
    """Test if statement."""
    ast = Program([IfStmt(
        BoolLiteral(True),
        [ExprStmt(IntLiteral(1))],
        [ExprStmt(IntLiteral(2))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "if" in output
    print("✓ test_if_stmt")

def test_unless_stmt():
    """Test unless statement."""
    ast = Program([UnlessStmt(
        BoolLiteral(False),
        [ExprStmt(IntLiteral(1))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "if" in output or "not" in output
    print("✓ test_unless_stmt")

def test_guard_stmt():
    """Test guard statement."""
    ast = Program([GuardStmt(
        BinaryOp(Identifier("x"), ">", IntLiteral(0)),
        [ReturnStmt(IntLiteral(-1))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "if" in output or "guard" in output.lower()
    print("✓ test_guard_stmt")

def test_while_stmt():
    """Test while loop."""
    ast = Program([WhileStmt(
        BinaryOp(Identifier("x"), "<", IntLiteral(10)),
        [ExprStmt(Identifier("x"))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "while" in output
    print("✓ test_while_stmt")

def test_until_stmt():
    """Test until loop."""
    ast = Program([UntilStmt(
        BinaryOp(Identifier("x"), ">=", IntLiteral(10)),
        [ExprStmt(Identifier("x"))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "while" in output or "until" in output.lower()
    print("✓ test_until_stmt")

def test_for_stmt():
    """Test for loop."""
    ast = Program([ForStmt(
        IdentifierPattern("i"),
        RangeExpr(IntLiteral(0), IntLiteral(10)),
        [ExprStmt(Identifier("i"))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "for" in output
    print("✓ test_for_stmt")

def test_for_enumerate():
    """Test for loop with enumeration."""
    ast = Program([ForStmt(
        ListPattern([IdentifierPattern("i"), IdentifierPattern("x")]),
        CallExpr(Identifier("enumerate"), [Identifier("items")]),
        [ExprStmt(Identifier("i"))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "for" in output
    print("✓ test_for_enumerate")

def test_loop_stmt():
    """Test infinite loop."""
    ast = Program([LoopStmt([
        IfStmt(
            BinaryOp(Identifier("x"), "==", IntLiteral(5)),
            [BreakStmt()],
            None,
        ),
    ])])
    t = Transformer()
    output = t.transform(ast)
    assert "while" in output or "True" in output
    print("✓ test_loop_stmt")

def test_break_stmt():
    """Test break statement."""
    ast = Program([BreakStmt()])
    t = Transformer()
    output = t.transform(ast)
    assert "break" in output
    print("✓ test_break_stmt")

def test_continue_stmt():
    """Test continue statement."""
    ast = Program([ContinueStmt()])
    t = Transformer()
    output = t.transform(ast)
    assert "continue" in output
    print("✓ test_continue_stmt")

def test_return_stmt():
    """Test return statement."""
    ast = Program([ReturnStmt(IntLiteral(42))])
    t = Transformer()
    output = t.transform(ast)
    assert "return" in output
    print("✓ test_return_stmt")

def test_return_no_value():
    """Test return without value."""
    ast = Program([ReturnStmt()])
    t = Transformer()
    output = t.transform(ast)
    assert "return" in output
    print("✓ test_return_no_value")


# ============================================================================
# TESTS: Functions (10 tests)
# ============================================================================

def test_function_decl():
    """Test function declaration."""
    ast = Program([FunctionDecl(
        "add",
        [Parameter("a"), Parameter("b")],
        "int",
        [ReturnStmt(BinaryOp(Identifier("a"), "+", Identifier("b")))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "def add" in output
    print("✓ test_function_decl")

def test_function_no_params():
    """Test function with no parameters."""
    ast = Program([FunctionDecl(
        "greet",
        [],
        "str",
        [ReturnStmt(StrLiteral("Hello"))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "def greet" in output
    print("✓ test_function_no_params")

def test_function_with_default():
    """Test function with default parameters."""
    ast = Program([FunctionDecl(
        "greet",
        [Parameter("name", None, StrLiteral("World"))],
        "str",
        [ReturnStmt(StrLiteral("Hello"))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "def greet" in output
    print("✓ test_function_with_default")

def test_function_variadic():
    """Test function with variadic parameters."""
    ast = Program([FunctionDecl(
        "sum_all",
        [Parameter("numbers", None, None, True)],
        "int",
        [ReturnStmt(CallExpr(Identifier("sum"), [Identifier("numbers")]))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "def sum_all" in output
    print("✓ test_function_variadic")

def test_lambda_expr():
    """Test lambda expression."""
    ast = Program([ExprStmt(LambdaExpr(
        [Parameter("x")],
        BinaryOp(Identifier("x"), "*", IntLiteral(2)),
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "lambda" in output or "=>" in output
    print("✓ test_lambda_expr")

def test_call_expr():
    """Test function call."""
    ast = Program([ExprStmt(CallExpr(
        Identifier("print"),
        [StrLiteral("hello")],
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "print" in output
    print("✓ test_call_expr")

def test_call_with_kwargs():
    """Test function call with keyword arguments."""
    ast = Program([ExprStmt(CallExpr(
        Identifier("func"),
        [],
        {"x": IntLiteral(10), "y": IntLiteral(20)},
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "func" in output
    print("✓ test_call_with_kwargs")

def test_recursive_function():
    """Test recursive function."""
    ast = Program([FunctionDecl(
        "factorial",
        [Parameter("n")],
        "int",
        [IfStmt(
            BinaryOp(Identifier("n"), "==", IntLiteral(0)),
            [ReturnStmt(IntLiteral(1))],
            [ReturnStmt(BinaryOp(
                Identifier("n"),
                "*",
                CallExpr(Identifier("factorial"), [BinaryOp(Identifier("n"), "-", IntLiteral(1))])
            ))],
        )],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "def factorial" in output
    print("✓ test_recursive_function")

def test_nested_function():
    """Test nested function."""
    ast = Program([FunctionDecl(
        "outer",
        [],
        None,
        [FunctionDecl(
            "inner",
            [],
            "int",
            [ReturnStmt(IntLiteral(42))],
        )],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "outer" in output
    print("✓ test_nested_function")

def test_higher_order_function():
    """Test higher-order function."""
    ast = Program([FunctionDecl(
        "apply_twice",
        [Parameter("fn"), Parameter("x")],
        None,
        [ReturnStmt(CallExpr(
            Identifier("fn"),
            [CallExpr(Identifier("fn"), [Identifier("x")])],
        ))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "apply_twice" in output
    print("✓ test_higher_order_function")


# ============================================================================
# TESTS: Classes and OOP (10 tests)
# ============================================================================

def test_class_decl():
    """Test class declaration."""
    ast = Program([ClassDecl(
        "Person",
        [
            VarDecl("name", False),
            Method("greet", [Parameter("self")], "str", [
                ReturnStmt(StrLiteral("Hello"))
            ]),
        ],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "class Person" in output
    print("✓ test_class_decl")

def test_class_with_base():
    """Test class with base class."""
    ast = Program([ClassDecl(
        "Student",
        [VarDecl("grade", False)],
        Identifier("Person"),
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "Student" in output
    print("✓ test_class_with_base")

def test_method_static():
    """Test static method."""
    ast = Program([ClassDecl(
        "MyClass",
        [Method("static_method", [Parameter("x")], "int", [
            ReturnStmt(IntLiteral(42))
        ], is_static=True)],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "MyClass" in output
    print("✓ test_method_static")

def test_method_classmethod():
    """Test classmethod."""
    ast = Program([ClassDecl(
        "MyClass",
        [Method("class_method", [Parameter("cls")], None, [
            ReturnStmt(Identifier("cls"))
        ], is_classmethod=True)],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "MyClass" in output
    print("✓ test_method_classmethod")

def test_method_property():
    """Test property decorator."""
    ast = Program([ClassDecl(
        "Circle",
        [Method("area", [Parameter("self")], "float", [
            ReturnStmt(IntLiteral(42))
        ], is_property=True)],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "Circle" in output
    print("✓ test_method_property")

def test_class_init():
    """Test class with __init__ method."""
    ast = Program([ClassDecl(
        "Point",
        [Method("__init__", [Parameter("self"), Parameter("x"), Parameter("y")], None, [
            ExprStmt(MemberExpr(Identifier("self"), "x")),
        ])],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "Point" in output
    print("✓ test_class_init")

def test_class_member_access():
    """Test class member access."""
    ast = Program([ExprStmt(MemberExpr(
        Identifier("person"),
        "name",
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "." in output or "person" in output
    print("✓ test_class_member_access")

def test_class_method_call():
    """Test class method call."""
    ast = Program([ExprStmt(CallExpr(
        MemberExpr(Identifier("obj"), "method"),
        [IntLiteral(42)],
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "obj" in output
    print("✓ test_class_method_call")

def test_trait_decl():
    """Test trait declaration."""
    ast = Program([TraitDecl(
        "Drawable",
        [Method("draw", [], None, [ExprStmt(StrLiteral("drawing"))])],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "draw" in output or "Drawable" in output
    print("✓ test_trait_decl")

def test_class_with_decorators():
    """Test class with decorators."""
    ast = Program([ClassDecl(
        "DataClass",
        [VarDecl("name", False)],
        decorators=[Decorator("dataclass")],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "DataClass" in output
    print("✓ test_class_with_decorators")


# ============================================================================
# TESTS: Advanced Expressions (12 tests)
# ============================================================================

def test_pipe_expr():
    """Test pipe operator."""
    ast = Program([ExprStmt(PipeExpr(
        Identifier("x"),
        CallExpr(Identifier("double"), []),
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "double" in output or "x" in output
    print("✓ test_pipe_expr")

def test_cond_expr():
    """Test ternary conditional."""
    ast = Program([ExprStmt(CondExpr(
        BoolLiteral(True),
        IntLiteral(1),
        IntLiteral(2),
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "if" in output or "else" in output
    print("✓ test_cond_expr")

def test_elvis_expr():
    """The `?:` operator is parsed as a coalescing binary expression."""
    from aura.parser.to_ast import Parser, Tokenizer
    prog = Parser(Tokenizer('let x = value ?: "default"\n').tokenize()).parse()
    output = Transformer().transform(prog)
    assert "value" in output and "default" in output
    print("✓ test_elvis_expr")

def test_coalesce_expr():
    """Test null coalescing operator."""
    ast = Program([ExprStmt(CoalesceExpr(
        Identifier("value"),
        StrLiteral("default"),
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "is not None" in output or "value" in output
    print("✓ test_coalesce_expr")

def test_safe_nav_member():
    """Test safe navigation for member."""
    ast = Program([ExprStmt(SafeNavExpr(
        Identifier("user"),
        "name",
        False,
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "is not None" in output or "user" in output
    print("✓ test_safe_nav_member")

def test_safe_nav_index():
    """Test safe navigation for index."""
    ast = Program([ExprStmt(SafeNavExpr(
        Identifier("items"),
        IntLiteral(0),
        True,
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "items" in output or "is not None" in output
    print("✓ test_safe_nav_index")

def test_index_expr():
    """Test index expression."""
    ast = Program([ExprStmt(IndexExpr(
        Identifier("list"),
        IntLiteral(0),
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "[" in output
    print("✓ test_index_expr")

def test_range_expr():
    """Test range expression."""
    ast = Program([ExprStmt(RangeExpr(
        IntLiteral(0),
        IntLiteral(10),
        False,
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "range" in output or "0" in output
    print("✓ test_range_expr")

def test_range_exclusive():
    """Test exclusive range."""
    ast = Program([ExprStmt(RangeExpr(
        IntLiteral(0),
        IntLiteral(10),
        True,
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "range" in output or "0" in output
    print("✓ test_range_exclusive")

def test_comprehension_list():
    """Test list comprehension."""
    ast = Program([ExprStmt(ComprehensionExpr(
        BinaryOp(Identifier("x"), "*", IntLiteral(2)),
        [(IdentifierPattern("x"), Identifier("items"), [])],
        'list',
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "[" in output or "for" in output
    print("✓ test_comprehension_list")

def test_comprehension_dict():
    """Test dict comprehension."""
    ast = Program([ExprStmt(ComprehensionExpr(
        BinaryOp(Identifier("k"), ":", Identifier("v")),
        [(ListPattern([IdentifierPattern("k"), IdentifierPattern("v")]), Identifier("items"), [])],
        'dict',
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "{" in output or "for" in output
    print("✓ test_comprehension_dict")

def test_comprehension_with_filter():
    """Test comprehension with filter."""
    ast = Program([ExprStmt(ComprehensionExpr(
        Identifier("x"),
        [(IdentifierPattern("x"), Identifier("items"), [
            BinaryOp(Identifier("x"), ">", IntLiteral(0))
        ])],
        'list',
    ))])
    t = Transformer()
    output = t.transform(ast)
    assert "for" in output and "if" in output
    print("✓ test_comprehension_with_filter")


# ============================================================================
# TESTS: Exception Handling (6 tests)
# ============================================================================

def test_try_catch():
    """Test try-catch."""
    ast = Program([TryStmt(
        [ExprStmt(CallExpr(Identifier("risky"), []))],
        [CatchClause("Exception", "e", [
            ExprStmt(Identifier("e"))
        ])],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "try" in output and "except" in output
    print("✓ test_try_catch")

def test_try_multiple_catch():
    """Test try with multiple catches."""
    ast = Program([TryStmt(
        [ExprStmt(CallExpr(Identifier("risky"), []))],
        [
            CatchClause("ValueError", "e", [ExprStmt(StrLiteral("value error"))]),
            CatchClause("KeyError", "e", [ExprStmt(StrLiteral("key error"))]),
        ],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "except" in output
    print("✓ test_try_multiple_catch")

def test_try_finally():
    """Test try-finally."""
    ast = Program([TryStmt(
        [ExprStmt(CallExpr(Identifier("risky"), []))],
        [],
        [ExprStmt(CallExpr(Identifier("cleanup"), []))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "finally" in output
    print("✓ test_try_finally")

def test_try_catch_finally():
    """Test try-catch-finally."""
    ast = Program([TryStmt(
        [ExprStmt(CallExpr(Identifier("risky"), []))],
        [CatchClause("Exception", "e", [ExprStmt(StrLiteral("error"))])],
        [ExprStmt(CallExpr(Identifier("cleanup"), []))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "try" in output and "except" in output and "finally" in output
    print("✓ test_try_catch_finally")

def test_with_stmt():
    """Test with statement."""
    ast = Program([WithStmt(
        [(Identifier("file"), "f")],
        [ExprStmt(CallExpr(MemberExpr(Identifier("f"), "read"), []))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "with" in output
    print("✓ test_with_stmt")

def test_match_stmt():
    """Test match statement."""
    ast = Program([MatchStmt(
        Identifier("x"),
        [
            MatchCase(IntLiteral(1), None, [ExprStmt(StrLiteral("one"))]),
            MatchCase(IntLiteral(2), None, [ExprStmt(StrLiteral("two"))]),
            MatchCase(WildcardPattern(), None, [ExprStmt(StrLiteral("other"))]),
        ],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "match" in output or "case" in output or "if" in output
    print("✓ test_match_stmt")


# ============================================================================
# TESTS: Type System (8 tests)
# ============================================================================

def test_simple_type():
    """Test simple type annotation."""
    ast = Program([VarDecl("x", False, SimpleType("int"), IntLiteral(10))])
    t = Transformer()
    output = t.transform(ast)
    assert "x" in output
    print("✓ test_simple_type")

def test_generic_type():
    """Test generic type annotation."""
    ast = Program([VarDecl("list", False, GenericType("list", [SimpleType("int")]), ListLiteral([IntLiteral(1)]))])
    t = Transformer()
    output = t.transform(ast)
    assert "list" in output
    print("✓ test_generic_type")

def test_optional_type():
    """Test optional type."""
    ast = Program([VarDecl("maybe", False, OptionalType(SimpleType("str")), NoneLiteral())])
    t = Transformer()
    output = t.transform(ast)
    assert "maybe" in output
    print("✓ test_optional_type")

def test_union_type():
    """Test union type."""
    ast = Program([VarDecl("value", False, UnionType([SimpleType("int"), SimpleType("str")]), IntLiteral(42))])
    t = Transformer()
    output = t.transform(ast)
    assert "value" in output
    print("✓ test_union_type")

def test_function_type():
    """Test function type."""
    ast = Program([VarDecl(
        "fn",
        False,
        FunctionType([SimpleType("int"), SimpleType("int")], SimpleType("int")),
        LambdaExpr([Parameter("a"), Parameter("b")], BinaryOp(Identifier("a"), "+", Identifier("b"))),
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "fn" in output
    print("✓ test_function_type")

def test_type_decl():
    """Test type declaration."""
    ast = Program([TypeDecl("UserId", SimpleType("int"))])
    t = Transformer()
    output = t.transform(ast)
    assert "UserId" in output or "int" in output
    print("✓ test_type_decl")

def test_structural_type():
    """Test structural type."""
    ast = Program([VarDecl(
        "point",
        False,
        StructuralType({"x": SimpleType("int"), "y": SimpleType("int")}),
        DictLiteral([(StrLiteral("x"), IntLiteral(0)), (StrLiteral("y"), IntLiteral(0))]),
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "point" in output
    print("✓ test_structural_type")

def test_function_with_return_type():
    """Test function with return type."""
    ast = Program([FunctionDecl(
        "add",
        [Parameter("a", SimpleType("int")), Parameter("b", SimpleType("int"))],
        SimpleType("int"),
        [ReturnStmt(BinaryOp(Identifier("a"), "+", Identifier("b")))],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "def add" in output
    print("✓ test_function_with_return_type")


# ============================================================================
# TESTS: Patterns and Destructuring (6 tests)
# ============================================================================

def test_literal_pattern():
    """Test literal pattern matching."""
    ast = Program([MatchStmt(
        Identifier("x"),
        [MatchCase(LiteralPattern(IntLiteral(5)), None, [ExprStmt(StrLiteral("five"))])],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "5" in output
    print("✓ test_literal_pattern")

def test_identifier_pattern():
    """Test identifier pattern."""
    ast = Program([MatchStmt(
        Identifier("x"),
        [MatchCase(IdentifierPattern("x"), None, [ExprStmt(Identifier("x"))])],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "match" in output or "case" in output or "x" in output
    print("✓ test_identifier_pattern")

def test_wildcard_pattern():
    """Test wildcard pattern."""
    ast = Program([MatchStmt(
        Identifier("x"),
        [MatchCase(WildcardPattern(), None, [ExprStmt(StrLiteral("default"))])],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "default" in output or "match" in output
    print("✓ test_wildcard_pattern")

def test_list_pattern():
    """Test list pattern destructuring."""
    ast = Program([MatchStmt(
        Identifier("pair"),
        [MatchCase(ListPattern([IdentifierPattern("a"), IdentifierPattern("b")]), None, [
            ExprStmt(BinaryOp(Identifier("a"), "+", Identifier("b")))
        ])],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "pair" in output
    print("✓ test_list_pattern")

def test_dict_pattern():
    """Test dict pattern destructuring."""
    ast = Program([MatchStmt(
        Identifier("data"),
        [MatchCase(DictPattern({"name": IdentifierPattern("n"), "age": IdentifierPattern("a")}), None, [
            ExprStmt(Identifier("n"))
        ])],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "data" in output
    print("✓ test_dict_pattern")

def test_as_pattern():
    """Test as pattern (binding)."""
    ast = Program([MatchStmt(
        Identifier("value"),
        [MatchCase(AsPattern(WildcardPattern(), "x"), None, [ExprStmt(Identifier("x"))])],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "value" in output
    print("✓ test_as_pattern")


# ============================================================================
# TESTS: Imports and Modules (5 tests)
# ============================================================================

def test_import():
    """Test import statement."""
    ast = Program([ImportStmt("math")])
    t = Transformer()
    output = t.transform(ast)
    assert "import" in output
    print("✓ test_import")

def test_import_alias():
    """Test import with alias."""
    ast = Program([ImportStmt("collections", alias="coll")])
    t = Transformer()
    output = t.transform(ast)
    assert "import" in output
    print("✓ test_import_alias")

def test_from_import():
    """Test from-import."""
    ast = Program([FromImport("math", [("sqrt", None), ("pi", None)])])
    t = Transformer()
    output = t.transform(ast)
    assert "from" in output and "import" in output
    print("✓ test_from_import")

def test_from_import_alias():
    """Test from-import with alias."""
    ast = Program([FromImport("collections", [("defaultdict", "dd")])])
    t = Transformer()
    output = t.transform(ast)
    assert "from" in output
    print("✓ test_from_import_alias")

def test_module():
    """Test module declaration."""
    ast = Program([Module("mymodule", [
        VarDecl("x", False, None, IntLiteral(10)),
        FunctionDecl("func", [], None, [ReturnStmt(Identifier("x"))]),
    ])])
    t = Transformer()
    output = t.transform(ast)
    assert "x" in output or "func" in output
    print("✓ test_module")


# ============================================================================
# TESTS: Decorators and Macros (6 tests)
# ============================================================================

def test_function_decorator():
    """Test function with decorator."""
    ast = Program([FunctionDecl(
        "cached",
        [],
        None,
        [ReturnStmt(IntLiteral(42))],
        decorators=[Decorator("cache")],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "cached" in output
    print("✓ test_function_decorator")

def test_multiple_decorators():
    """Test function with multiple decorators."""
    ast = Program([FunctionDecl(
        "special",
        [],
        None,
        [ReturnStmt(IntLiteral(1))],
        decorators=[Decorator("cache"), Decorator("debug")],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "special" in output
    print("✓ test_multiple_decorators")

def test_decorator_with_args():
    """Test decorator with arguments."""
    ast = Program([FunctionDecl(
        "cached",
        [],
        None,
        [ReturnStmt(IntLiteral(42))],
        decorators=[Decorator("cache", [IntLiteral(10)])],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "cached" in output
    print("✓ test_decorator_with_args")

def test_class_decorator():
    """Test class with decorator."""
    ast = Program([ClassDecl(
        "Data",
        [VarDecl("x", False)],
        decorators=[Decorator("dataclass")],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "Data" in output
    print("✓ test_class_decorator")

def test_property_decorator():
    """Test property decorator."""
    ast = Program([ClassDecl(
        "Circle",
        [Method("area", [Parameter("self")], "float", [
            ReturnStmt(IntLiteral(42))
        ], is_property=True)],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "Circle" in output
    print("✓ test_property_decorator")

def test_staticmethod_decorator():
    """Test staticmethod decorator."""
    ast = Program([ClassDecl(
        "Utils",
        [Method("add", [Parameter("a"), Parameter("b")], "int", [
            ReturnStmt(BinaryOp(Identifier("a"), "+", Identifier("b")))
        ], is_static=True)],
    )])
    t = Transformer()
    output = t.transform(ast)
    assert "Utils" in output
    print("✓ test_staticmethod_decorator")


# ============================================================================
# Run all tests
# ============================================================================

def run_all():
    """Run all tests."""
    tests = [
        # Literals and Variables (10)
        test_int_literal, test_float_literal, test_string_literal, test_bool_literal,
        test_none_literal, test_var_decl, test_var_mutable, test_const_decl,
        test_identifier, test_var_with_type,
        
        # Collections (8)
        test_list_literal, test_empty_list, test_dict_literal, test_set_literal,
        test_tuple_literal, test_empty_dict, test_nested_list, test_mixed_dict,
        
        # Operators (10)
        test_binary_add, test_binary_sub, test_binary_mul, test_binary_div,
        test_binary_comparison, test_binary_logical, test_unary_not, test_unary_neg,
        test_binary_in,
        
        # Control Flow (12)
        test_if_stmt, test_unless_stmt, test_guard_stmt, test_while_stmt,
        test_until_stmt, test_for_stmt, test_for_enumerate, test_loop_stmt,
        test_break_stmt, test_continue_stmt, test_return_stmt, test_return_no_value,
        
        # Functions (10)
        test_function_decl, test_function_no_params, test_function_with_default,
        test_function_variadic, test_lambda_expr, test_call_expr, test_call_with_kwargs,
        test_recursive_function, test_nested_function, test_higher_order_function,
        
        # Classes (10)
        test_class_decl, test_class_with_base, test_method_static, test_method_classmethod,
        test_method_property, test_class_init, test_class_member_access, test_class_method_call,
        test_trait_decl, test_class_with_decorators,
        
        # Advanced Expressions (12)
        test_pipe_expr, test_cond_expr, test_elvis_expr, test_coalesce_expr,
        test_safe_nav_member, test_safe_nav_index, test_index_expr, test_range_expr,
        test_range_exclusive, test_comprehension_list, test_comprehension_dict,
        test_comprehension_with_filter,
        
        # Exception Handling (6)
        test_try_catch, test_try_multiple_catch, test_try_finally, test_try_catch_finally,
        test_with_stmt, test_match_stmt,
        
        # Type System (8)
        test_simple_type, test_generic_type, test_optional_type, test_union_type,
        test_function_type, test_type_decl, test_structural_type, test_function_with_return_type,
        
        # Patterns (6)
        test_literal_pattern, test_identifier_pattern, test_wildcard_pattern,
        test_list_pattern, test_dict_pattern, test_as_pattern,
        
        # Imports (5)
        test_import, test_import_alias, test_from_import, test_from_import_alias,
        test_module,
        
        # Decorators (6)
        test_function_decorator, test_multiple_decorators, test_decorator_with_args,
        test_class_decorator, test_property_decorator, test_staticmethod_decorator,
    ]
    
    print("\n" + "="*70)
    print("COMPREHENSIVE AURA TRANSPILER TEST SUITE")
    print("="*70)
    print(f"\nRunning {len(tests)} tests covering all Aura features...\n")
    
    failed = 0
    categories = {
        "Literals and Variables": 0,
        "Collections": 0,
        "Binary/Unary Operators": 0,
        "Control Flow": 0,
        "Functions": 0,
        "Classes and OOP": 0,
        "Advanced Expressions": 0,
        "Exception Handling": 0,
        "Type System": 0,
        "Patterns": 0,
        "Imports": 0,
        "Decorators": 0,
    }
    
    for test in tests:
        try:
            test()
            # Categorize the test
            for cat in categories:
                if cat.lower().replace(" ", "_") in test.__name__:
                    categories[cat] += 1
                    break
        except Exception as e:
            print(f"✗ {test.__name__}: {e}")
            failed += 1
    
    print("\n" + "="*70)
    print(f"RESULTS: {len(tests) - failed}/{len(tests)} tests passed")
    print("="*70)
    
    for cat, count in categories.items():
        if count > 0:
            print(f"  ✓ {cat}: {count} tests")
    
    if failed > 0:
        print(f"\n  ✗ {failed} test(s) failed")
    
    print("="*70 + "\n")
    
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(run_all())
