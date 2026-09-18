"""Runtime execution tests for Aura.

Unlike the AST-level tests, these tests transpile *real Aura source* and
execute the generated Python, asserting on the actual program output. They
cover the language features that were previously broken: the pipe operator,
lambda forms, classes/constructors, properties, decorators/macros, pattern
matching, exceptions, imports and comprehensions.
"""
import io
import sys
import contextlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import parse_file  # noqa: E402
from transpiler.transformer import Transformer  # noqa: E402


def run_aura(source: str):
    """Transpile Aura source, execute it, and return captured stdout."""
    from parser.to_ast import Tokenizer, Parser

    tokens = Tokenizer(source).tokenize()
    program = Parser(tokens).parse()
    code = Transformer().transform(program)

    namespace = {"__name__": "__aura_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, namespace)
    return buffer.getvalue(), code, namespace


def run_aura_file(path):
    program = parse_file(str(path))
    code = Transformer().transform(program)
    namespace = {"__name__": "__aura_test__"}
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        exec(code, namespace)
    return buffer.getvalue(), code, namespace


# ============================================================================
# Pipe operator (data-first)
# ============================================================================

def test_pipe_map():
    out, _, _ = run_aura("print([1, 2, 3] |> map((x) => x * 2))")
    assert out.strip() == "[2, 4, 6]"


def test_pipe_filter():
    out, _, _ = run_aura("print([1, 2, 3, 4] |> filter((x) => x % 2 == 0))")
    assert out.strip() == "[2, 4]"


def test_pipe_reduce():
    out, _, _ = run_aura("print([1, 2, 3, 4] |> reduce((a, b) => a + b, 0))")
    assert out.strip() == "10"


def test_pipe_chain():
    source = (
        "print([1, 2, 3, 4, 5]"
        " |> map((x) => x * 2)"
        " |> filter((x) => x > 4)"
        " |> reduce((a, b) => a + b, 0))"
    )
    out, _, _ = run_aura(source)
    assert out.strip() == "24"


def test_pipe_plain_function():
    out, _, _ = run_aura("def inc(x) { return x + 1 }\nprint(5 |> inc)")
    assert out.strip() == "6"


# ============================================================================
# Lambda forms
# ============================================================================

def test_lambda_parenthesized():
    out, _, _ = run_aura("let f = (x) => x * 2\nprint(f(5))")
    assert out.strip() == "10"


def test_lambda_multi_param():
    out, _, _ = run_aura("let add = (a, b) => a + b\nprint(add(3, 4))")
    assert out.strip() == "7"


def test_lambda_no_parens():
    out, _, _ = run_aura("let sq = x => x * x\nprint(sq(6))")
    assert out.strip() == "36"


def test_lambda_zero_arg():
    out, _, _ = run_aura("let f = () => 42\nprint(f())")
    assert out.strip() == "42"


def test_lambda_block_body():
    source = """
    let f = (x) => {
      let doubled = x * 2
      return doubled + 1
    }
    print(f(10))
    """
    out, _, _ = run_aura(source)
    assert out.strip() == "21"


def test_lambda_closure():
    source = """
    let make_adder = (n) => (x) => x + n
    let add10 = make_adder(10)
    print(add10(5))
    """
    out, _, _ = run_aura(source)
    assert out.strip() == "15"


# ============================================================================
# Classes
# ============================================================================

def test_class_constructor_new():
    source = """
    class Point {
      let x: int
      let y: int
      def new(x: int, y: int) {
        self.x = x
        self.y = y
      }
    }
    let p = Point(3, 4)
    print(p.x, p.y)
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "3 4"
    assert "def __init__(self, x, y):" in code
    assert "class Point" in code


def test_class_field_defaults():
    source = """
    class Counter {
      let count: int = 0
      def increment() {
        self.count += 1
      }
    }
    let c = Counter()
    c.increment()
    c.increment()
    print(c.count)
    """
    out, _, _ = run_aura(source)
    assert out.strip() == "2"


def test_class_inheritance_and_override():
    source = """
    class Animal {
      let name: str = ""
      def new(name: str) { self.name = name }
      def speak() -> str { return "..." }
    }
    class Dog extends Animal {
      def speak() -> str { return self.name + " says woof" }
    }
    let d = Dog("Rex")
    print(d.speak())
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "Rex says woof"
    assert "class Dog(Animal):" in code


def test_class_method_self_not_duplicated():
    """An explicit `self` parameter must not produce `def m(self, self)`."""
    source = """
    class A {
      def greet(self) -> str { return "hi" }
    }
    print(A().greet())
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "hi"
    assert "def greet(self):" in code


def test_property():
    source = """
    class Rect {
      let w: int
      let h: int
      def new(w: int, h: int) { self.w = w; self.h = h }
      @property
      def area() -> int { return self.w * self.h }
    }
    let r = Rect(4, 5)
    print(r.area)
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "20"
    assert "@property" in code


def test_staticmethod():
    source = """
    class MathUtil {
      @staticmethod
      def max(a, b) -> int {
        if a > b { return a }
        return b
      }
    }
    print(MathUtil.max(3, 9))
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "9"
    assert "@staticmethod" in code


def test_classmethod():
    source = """
    class Factory {
      let kind: str = "base"
      def new(kind: str) { self.kind = kind }
      @classmethod
      def create(cls, kind) {
        return cls(kind)
      }
    }
    let f = Factory.create("custom")
    print(f.kind)
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "custom"
    assert "@classmethod" in code
    assert "def create(cls, kind):" in code


# ============================================================================
# Control flow
# ============================================================================

def test_match_with_guards():
    source = """
    def classify(n) -> str {
      match n {
        case 0 { return "zero" }
        case x if x > 100 { return "big" }
        case x if x > 0 { return "positive" }
        case _ { return "other" }
      }
      return "?"
    }
    print(classify(0), classify(200), classify(5), classify(-1))
    """
    out, _, _ = run_aura(source)
    assert out.strip() == "zero big positive other"


def test_guard_statement():
    source = """
    def f(n) {
      guard n > 0 else { return "invalid" }
      return "ok"
    }
    print(f(5), f(-1))
    """
    out, _, _ = run_aura(source)
    assert out.strip() == "ok invalid"


def test_for_range_step_and_while():
    source = """
    let total = 0
    for i in range(0, 10, 2) { total += i }
    let n = 3
    while n > 0 { n -= 1 }
    print(total, n)
    """
    out, _, _ = run_aura(source)
    assert out.strip() == "20 0"


# ============================================================================
# Exceptions
# ============================================================================

def test_try_catch_bare():
    source = """
    try {
      throw ValueError("boom")
    } catch e {
      print("caught", e)
    }
    """
    out, _, _ = run_aura(source)
    assert "caught boom" in out


def test_try_catch_typed():
    source = """
    try {
      let x = [1]
      print(x[5])
    } catch IndexError {
      print("index error")
    }
    """
    out, _, _ = run_aura(source)
    assert out.strip() == "index error"


def test_try_finally():
    source = """
    try {
      print("try")
    } catch e {
      print("catch")
    } finally {
      print("finally")
    }
    """
    out, _, _ = run_aura(source)
    assert out.strip() == "try\nfinally"


# ============================================================================
# Collections & comprehensions
# ============================================================================

def test_list_comprehension():
    out, _, _ = run_aura("print([x * x for x in range(5)])")
    assert out.strip() == "[0, 1, 4, 9, 16]"


def test_list_comprehension_filter():
    out, _, _ = run_aura("print([x for x in range(10) if x % 2 == 0])")
    assert out.strip() == "[0, 2, 4, 6, 8]"


def test_dict_literal_and_attribute_access():
    source = """
    let user = {name: "Alice", age: 30}
    print(user["name"], user.age)
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "Alice 30"
    assert "AuraDict" in code


def test_set_literal():
    out, _, _ = run_aura("print(len({1, 2, 2, 3, 3, 3}))")
    assert out.strip() == "3"


# ============================================================================
# Imports
# ============================================================================

def test_from_import_stdlib():
    source = """
    from stdlib.math import sqrt
    print(sqrt(16))
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "4.0"
    assert "from stdlib.math import sqrt" in code


def test_import_braces():
    source = """
    import stdlib.math { sqrt, PI }
    print(sqrt(9))
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "3.0"
    assert "from stdlib.math import sqrt, PI" in code


def test_import_as_alias():
    source = """
    import stdlib.math as m
    print(m.sqrt(25))
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "5.0"
    assert "import stdlib.math as m" in code


# ============================================================================
# Macros / decorators
# ============================================================================

def test_memoize():
    source = """
    @memoize
    def fib(n) -> int {
      if n < 2 { return n }
      return fib(n - 1) + fib(n - 2)
    }
    print(fib(25))
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "75025"
    assert "def memoize" in code


def test_timeit_reports():
    source = """
    @timeit
    def work() -> int { return 1 }
    print(work())
    """
    out, _, _ = run_aura(source)
    assert "work took" in out
    assert out.strip().endswith("1")


def test_debug():
    source = """
    @debug
    def f(x) -> int { return x + 1 }
    print(f(1))
    """
    out, _, _ = run_aura(source)
    assert "DEBUG: enter f" in out
    assert "DEBUG: exit f -> 2" in out


def test_must_return():
    source = """
    @must_return
    def f() -> int { return 7 }
    print(f())
    """
    out, _, _ = run_aura(source)
    assert out.strip() == "7"


def test_cache_with_args():
    source = """
    @cache(maxsize=32)
    def square(n) -> int { return n * n }
    print(square(9))
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "81"
    assert "@cache(maxsize=32)" in code


# ============================================================================
# End-to-end example files
# ============================================================================

def test_all_examples_run():
    examples = sorted(Path(__file__).parent.parent.glob("examples/*.aura"))
    assert examples, "expected example files"
    for example in examples:
        out, code, _ = run_aura_file(example)
        assert code.strip(), f"{example.name} produced no code"


def test_examples_dir_has_core_set():
    examples = {p.name for p in Path(__file__).parent.parent.glob("examples/*.aura")}
    for expected in {
        "hello.aura",
        "fibonacci.aura",
        "prime_checker.aura",
        "classes.aura",
        "functional.aura",
        "pattern_matching.aura",
        "error_handling.aura",
        "macros.aura",
        "tour.aura",
    }:
        assert expected in examples, f"missing example {expected}"


# ============================================================================
# Regression: imports, block expressions, shared transformer state
# ============================================================================

def test_import_braces_with_alias():
    source = """
    import stdlib.math { sqrt as sq }
    print(sq(9))
    """
    out, code, _ = run_aura(source)
    assert out.strip() == "3.0"
    assert "import sq" in code or "as sq" in code


def test_from_import_with_alias():
    source = """
    from stdlib.math import sqrt as sq, PI
    print(sq(16))
    print(PI)
    """
    out, _, _ = run_aura(source)
    assert out.splitlines() == ["4.0", "3.141592653589793"]


def test_block_expr_with_assignments():
    source = """
    val = {
      a = 1
      b = a + 2
      b * 10
    }
    print(val)
    """
    out, _, _ = run_aura(source)
    assert out.strip() == "30"


def test_block_expr_uses_shared_transformer_state():
    # The inner block reads a method defined in an outer class scope; if the
    # block expression used a fresh ExpressionTransformer its member
    # visibility table would be lost and ``length()`` alias handling would
    # break. This guards against the shared-state regression.
    source = """
    data = [1, 2, 3, 4]
    total = {
      doubled = data |> map((x) => x * 2)
      reduced = doubled |> reduce((a, b) => a + b, 0)
      reduced
    }
    print(total)
    """
    out, _, _ = run_aura(source)
    assert out.strip() == "20"


def test_multiline_method_body_indentation():
    source = """
    class Calc {
      def new(base) {
        self.base = base
      }
      def add(x) {
        tmp = x + 1
        return self.base + tmp
      }
    }
    c = Calc(10)
    print(c.add(5))
    """
    out, _, _ = run_aura(source)
    assert out.strip() == "16"


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
    print(f"\n{len(tests) - failed}/{len(tests)} runtime tests passed")
    sys.exit(1 if failed else 0)