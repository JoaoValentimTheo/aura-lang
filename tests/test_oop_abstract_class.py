"""Abstract classes: `abstract class` + `abstract def` (syntax freeze).

Aura's class OOP is frozen with two explicit spellings:

* `abstract class C { ... }` marks a class that may not be instantiated and may
  defer methods to a subclass.
* `abstract def m(...)` declares a signature with no body; a concrete subclass
  must implement it.

A trait stays a pure contract (body-less methods, no state); an abstract class
is a real class that may carry fields, concrete methods and a constructor. The
parser rejects an `abstract def` with a body, a header field mixed with a manual
`new`, and `abstract` on anything that is not a class or a method.
"""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT))

from aura_test_helpers import parse, run_aura, transpile  # noqa: E402

from aura.parser.to_ast import Parser, Tokenizer  # noqa: E402
from aura.transpiler.rules import RuleChecker  # noqa: E402


def rule_codes(source):
    program = Parser(Tokenizer(source).tokenize()).parse()
    checker = RuleChecker()
    checker.check_program(program)
    return [e.code.value for e in checker.collector.errors]


# ============================================================================
# Grammar
# ============================================================================

class TestAbstractGrammar:
    def test_abstract_class_parses(self):
        decl = parse("abstract class Shape {\n}\n").statements[0]
        assert decl.is_abstract is True

    def test_concrete_class_is_not_abstract(self):
        decl = parse("class Shape {\n}\n").statements[0]
        assert decl.is_abstract is False

    def test_abstract_def_is_bodyless(self):
        decl = parse(
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n").statements[0]
        method = decl.body[0]
        assert method.is_abstract is True
        assert method.body is None

    def test_abstract_def_may_end_with_semicolon(self):
        decl = parse(
            "abstract class Shape {\n"
            "  public abstract def area() -> float;\n"
            "}\n").statements[0]
        assert decl.body[0].is_abstract is True

    def test_modifier_order_is_flexible(self):
        decl = parse(
            "abstract class Shape {\n"
            "  abstract public def area() -> float\n"
            "}\n").statements[0]
        assert decl.body[0].is_abstract is True
        assert decl.body[0].visibility == 'public'

    def test_abstract_def_with_block_body_is_rejected(self):
        with pytest.raises(SyntaxError, match='cannot have a body'):
            parse("abstract class C { public abstract def m() {} }")

    def test_abstract_def_with_expression_body_is_rejected(self):
        with pytest.raises(SyntaxError, match='cannot have an expression body'):
            parse("abstract class C { public abstract def m() = 1 }")

    def test_abstract_on_a_field_is_rejected(self):
        with pytest.raises(SyntaxError, match="'abstract' applies to a class or a method"):
            parse("abstract let x = 1")

    def test_abstract_on_class_is_kept_in_ast(self):
        program = parse("abstract class Shape {\n}\n")
        assert program.statements[0].is_abstract is True

    def test_abstract_method_keeps_return_type(self):
        decl = parse(
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n").statements[0]
        assert decl.body[0].return_type is not None

    def test_override_is_rejected_implicit_override(self):
        # Overriding is implicit; `override` would otherwise be silently
        # parsed as a field named `override`.
        with pytest.raises(SyntaxError, match="'override' is not Aura"):
            parse("class C extends B { override def m() { } }")

    def test_abstract_is_rejected_in_a_trait(self):
        # A trait method without a body is already abstract.
        with pytest.raises(SyntaxError, match="not used in a trait"):
            parse("trait T { public abstract def f() -> int }")

    def test_abstract_class_nested_in_class(self):
        decl = parse(
            "abstract class Outer {\n"
            "  public abstract class Inner {\n"
            "    public abstract def f() -> int\n"
            "  }\n"
            "}\n").statements[0]
        assert decl.is_abstract is True
        inner = decl.body[0]
        assert inner.is_abstract is True
        assert inner.body[0].is_abstract is True

    @pytest.mark.parametrize("word", ['final', 'open', 'sealed', 'override', 'implements'])
    def test_foreign_member_modifier_is_rejected(self, word):
        # A word Aura does not have as a modifier must not be silently parsed
        # as a field named after it.
        with pytest.raises(SyntaxError):
            parse(f"class C {{ public {word} def m() {{ }} }}")

    def test_async_method_in_class_is_still_accepted(self):
        decl = parse(
            "class C {\n"
            "  public async def fetch() -> int { return 1 }\n"
            "}\n").statements[0]
        assert decl.body[0].is_async is True


# ============================================================================
# E316 - abstract class cannot be instantiated
# ============================================================================

class TestAbstractInstantiation:
    def test_instantiating_abstract_class_is_rejected(self):
        src = (
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "let s = Shape()\n"
        )
        assert 'E316' in rule_codes(src)

    def test_instantiating_abstract_class_message(self):
        src = (
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "Shape()\n"
        )
        program = Parser(Tokenizer(src).tokenize()).parse()
        checker = RuleChecker()
        checker.check_program(program)
        message = str(checker.collector.errors[0])
        assert "'Shape'" in message
        assert 'abstract' in message

    def test_instantiating_concrete_subclass_is_accepted(self):
        src = (
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "class Square extends Shape {\n"
            "  public def area() -> float { return 4.0 }\n"
            "}\n"
            "let s = Square()\n"
        )
        assert 'E316' not in rule_codes(src)
        assert 'E309' not in rule_codes(src)

    def test_runtime_guards_against_abstract_instantiation(self):
        src = (
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "Shape()\n"
        )
        with pytest.raises(TypeError):
            run_aura(src)


# ============================================================================
# E309 - concrete subclass must implement abstract methods
# ============================================================================

class TestAbstractImplementation:
    def test_unimplemented_abstract_method_is_rejected(self):
        src = (
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "class Bad extends Shape {\n"
            "  public let s: float = 1.0\n"
            "}\n"
        )
        assert 'E309' in rule_codes(src)

    def test_message_names_class_and_method(self):
        src = (
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "class Bad extends Shape {\n"
            "  public let s: float = 1.0\n"
            "}\n"
        )
        program = Parser(Tokenizer(src).tokenize()).parse()
        checker = RuleChecker()
        checker.check_program(program)
        message = str(checker.collector.errors[0])
        assert "'Bad'" in message
        assert "'area'" in message
        assert "'Shape'" in message

    def test_implemented_abstract_method_is_accepted(self):
        src = (
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "class Square extends Shape {\n"
            "  public def area() -> float { return 4.0 }\n"
            "}\n"
        )
        assert rule_codes(src) == []

    def test_abstract_class_may_defer(self):
        # An abstract subclass is allowed to leave the method open.
        src = (
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "abstract class Poly extends Shape {\n"
            "  public abstract def sides() -> int\n"
            "}\n"
        )
        assert rule_codes(src) == []

    def test_chain_of_abstract_classes_discharged_by_leaf(self):
        src = (
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "abstract class Poly extends Shape {\n"
            "  public abstract def sides() -> int\n"
            "}\n"
            "class Square extends Poly {\n"
            "  public def area() -> float { return 4.0 }\n"
            "  public def sides() -> int { return 4 }\n"
            "}\n"
        )
        assert rule_codes(src) == []

    def test_chain_missing_leaf_method_is_rejected(self):
        src = (
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "abstract class Poly extends Shape {\n"
            "  public abstract def sides() -> int\n"
            "}\n"
            "class Square extends Poly {\n"
            "  public def sides() -> int { return 4 }\n"
            "}\n"
        )
        assert 'E309' in rule_codes(src)

    def test_inherited_concrete_method_satisfies_abstract(self):
        src = (
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "abstract class Base extends Shape {\n"
            "  public def area() -> float { return 0.0 }\n"
            "}\n"
            "class Circle extends Base {\n"
            "}\n"
        )
        assert rule_codes(src) == []

    def test_field_does_not_satisfy_abstract_method(self):
        # A field named like the abstract method is not an implementation.
        src = (
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "class Bad extends Shape {\n"
            "  public let area: float = 1.0\n"
            "}\n"
        )
        assert 'E309' in rule_codes(src)

    def test_abstract_def_in_concrete_class_is_rejected(self):
        src = "class C {\n  public abstract def m() -> int\n}\n"
        assert 'E309' in rule_codes(src)


# ============================================================================
# Interaction with traits and `extends`
# ============================================================================

class TestAbstractAndTraits:
    def test_abstract_class_can_implement_trait_partially(self):
        src = (
            "trait Drawable {\n"
            "  public def draw() -> void\n"
            "  public def bounds() -> float\n"
            "}\n"
            "abstract class Base extends Drawable {\n"
            "  public def draw() -> void { print('x') }\n"
            "}\n"
        )
        assert rule_codes(src) == []

    def test_concrete_class_must_finish_trait_obligations(self):
        src = (
            "trait Drawable {\n"
            "  public def draw() -> void\n"
            "  public def bounds() -> float\n"
            "}\n"
            "abstract class Base extends Drawable {\n"
            "  public def draw() -> void { print('x') }\n"
            "}\n"
            "class Circle extends Base {\n"
            "}\n"
        )
        assert 'E309' in rule_codes(src)

    def test_abstract_methods_from_trait_and_class_combine(self):
        src = (
            "trait Drawable {\n"
            "  public def draw() -> void\n"
            "}\n"
            "abstract class Shape extends Drawable {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "class Square extends Shape {\n"
            "  public def draw() -> void { print('x') }\n"
            "  public def area() -> float { return 4.0 }\n"
            "  public def sides() -> int { return 4 }\n"
            "}\n"
        )
        assert rule_codes(src) == []


# ============================================================================
# Code generation
# ============================================================================

class TestAbstractCodegen:
    def test_abstract_class_is_an_abc(self):
        code = transpile(
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n")
        assert '_aura_abc.ABC' in code

    def test_abstract_method_is_marked(self):
        code = transpile(
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n")
        assert '@_aura_abc.abstractmethod' in code

    def test_concrete_class_has_no_abc_base(self):
        code = transpile("class Shape {\n}\n")
        assert '_aura_abc.ABC' not in code

    def test_abstract_class_with_fields_and_concrete_method(self):
        code = transpile(
            "abstract class Shape {\n"
            "  public let name: str = 'x'\n"
            "  public def describe() -> str { return self.name }\n"
            "  public abstract def area() -> float\n"
            "}\n")
        assert 'def describe' in code
        assert 'def area' in code
        assert '@_aura_abc.abstractmethod' in code

    def test_generated_python_is_valid(self):
        import ast as py_ast
        code = transpile(
            "abstract class Shape {\n"
            "  public let name: str = 'x'\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "class Square extends Shape {\n"
            "  public def area() -> float { return 4.0 }\n"
            "}\n")
        py_ast.parse(code)


# ============================================================================
# Runtime behaviour
# ============================================================================

class TestAbstractRuntime:
    def test_concrete_subclass_runs_abstract_method(self):
        out = run_aura(
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "  public def describe() -> str { return 'a shape' }\n"
            "}\n"
            "class Square extends Shape {\n"
            "  public def area() -> float { return 4.0 }\n"
            "}\n"
            "def main() {\n"
            "  print(Square().area())\n"
            "  print(Square().describe())\n"
            "}\n")
        assert out == '4.0\na shape\n'

    def test_virtual_dispatch_through_abstract_base(self):
        out = run_aura(
            "abstract class Shape {\n"
            "  public abstract def area() -> float\n"
            "}\n"
            "class Square extends Shape {\n"
            "  public def area() -> float { return 4.0 }\n"
            "}\n"
            "class Circle extends Shape {\n"
            "  public def area() -> float { return 1.0 }\n"
            "}\n"
            "def total(a: Shape) -> float { return a.area() }\n"
            "def main() {\n"
            "  print(total(Square()))\n"
            "  print(total(Circle()))\n"
            "}\n")
        assert out == '4.0\n1.0\n'


# ============================================================================
# Header + manual `new` (frozen: one constructor style per class)
# ============================================================================

class TestHeaderAndNew:
    def test_header_plus_manual_new_is_rejected(self):
        with pytest.raises(SyntaxError, match='header fields and a manual'):
            parse(
                'class U(name: str) {\n'
                '  public def new(name: str) { self.name = name }\n'
                '}\n')

    def test_body_fields_with_manual_new_is_accepted(self):
        src = (
            "class U {\n"
            "  public let name: str = ''\n"
            "  public def new(name: str) { self.name = name }\n"
            "}\n"
        )
        assert rule_codes(src) == []

    def test_header_alone_is_accepted(self):
        src = "class U(name: str) {\n}\n"
        assert rule_codes(src) == []
