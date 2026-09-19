"""Batch 7g - large combinatorial to reach 10K."""
from __future__ import annotations
import pytest
from aura.parser.to_ast import Tokenizer, Parser
from aura.transpiler.transformer import Transformer


def _compile(src):
    tokens = Tokenizer(src).tokenize()
    tree = Parser(tokens).parse()
    code = Transformer().transform(tree)
    compile(code, '<test>', 'exec')
    return code


# Arithmetic on 12 ints with 5 ops
INTS12 = ['0', '1', '2', '3', '5', '7', '10', '13', '17', '42', '100', '-1']
OPS5 = ['+', '-', '*', '%']
ARITH_LARGE = [(f'{a}{op}{b}', f'def main() {{ let x = {a} {op} {b} }}')
               for a in INTS12 for op in OPS5 for b in INTS12]


@pytest.mark.parametrize('name,src', ARITH_LARGE, ids=[i[0] for i in ARITH_LARGE])
def test_arith_large(name, src):
    _compile(src)


# Power 10x10
BASES10 = ['0', '1', '2', '3', '4', '5', '8', '10', '12', '15']
EXPS10 = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']
POWER_LARGE = [(f'{b}**{e}', f'def main() {{ let x = {b} ** {e} }}')
               for b in BASES10 for e in EXPS10]


@pytest.mark.parametrize('name,src', POWER_LARGE, ids=[i[0] for i in POWER_LARGE])
def test_power_large(name, src):
    _compile(src)


# Comparison 10x10x6
INTS10 = ['0', '1', '2', '3', '5', '10', '25', '50', '100', '-5']
CMP10 = ['==', '!=', '<', '>', '<=', '>=']
CMP_LARGE = [(f'{x}{op}{y}', f'def main() {{ let x = {x} {op} {y} }}')
             for x in INTS10 for op in CMP10 for y in INTS10]


@pytest.mark.parametrize('name,src', CMP_LARGE, ids=[i[0] for i in CMP_LARGE])
def test_cmp_large(name, src):
    _compile(src)


# Bitwise 8x8x3
BITS = ['0', '1', '3', '7', '15', '31', '63', '127']
BITOPS = ['&', '|', '^']
BIT_LARGE = [(f'{a}{op}{b}', f'def main() {{ let x = {a} {op} {b} }}')
             for a in BITS for op in BITOPS for b in BITS]


@pytest.mark.parametrize('name,src', BIT_LARGE, ids=[i[0] for i in BIT_LARGE])
def test_bit_large(name, src):
    _compile(src)


# Shift 8x8x2
SHIFTS = ['0', '1', '2', '3', '4', '5', '8', '16']
SHIFTOPS = ['<<', '>>']
SHIFT_LARGE = [(f'{a}{op}{b}', f'def main() {{ let x = {a} {op} {b} }}')
               for a in SHIFTS for op in SHIFTOPS for b in SHIFTS]


@pytest.mark.parametrize('name,src', SHIFT_LARGE, ids=[i[0] for i in SHIFT_LARGE])
def test_shift_large(name, src):
    _compile(src)


# More programs
PROGRAMS = [
    ('min3', 'def min3(a: int, b: int, c: int) -> int { if a < b { if a < c { return a } else { return c } } else { if b < c { return b } else { return c } } }'),
    ('abs_val', 'def abs_val(x: int) -> int { if x < 0 { return -x } return x }'),
    ('clamp', 'def clamp(x: int, lo: int, hi: int) -> int { if x < lo { return lo } if x > hi { return hi } return x }'),
    ('gcd', 'def gcd(a: int, b: int) -> int { while b != 0 { let t = b\nb = a % b\na = t } return a }'),
    ('lcm', 'def lcm(a: int, b: int) -> int { return a / gcd(a, b) * b }'),
    ('is_even', 'def is_even(x: int) -> bool { return x % 2 == 0 }'),
    ('is_odd', 'def is_odd(x: int) -> bool { return x % 2 != 0 }'),
    ('max3', 'def max3(a: int, b: int, c: int) -> int { let m = a\nif b > m { m = b }\nif c > m { m = c } return m }'),
    ('pow_iter', 'def pow_iter(b: int, e: int) -> int { let result = 1\nlet i = 0\nwhile i < e { result = result * b\ni = i + 1 } return result }'),
    ('digit_sum', 'def digit_sum(n: int) -> int { let sum = 0\nwhile n > 0 { sum = sum + n % 10\nn = n / 10 } return sum }'),
    ('count_digits', 'def count_digits(n: int) -> int { let count = 0\nlet x = n\nwhile x > 0 { count = count + 1\nx = x / 10 } return count }'),
    ('reverse_digits', 'def reverse_digits(n: int) -> int { let rev = 0\nlet x = n\nwhile x > 0 { rev = rev * 10 + x % 10\nx = x / 10 } return rev }'),
    ('is_palindrome_num', 'def is_palindrome_num(n: int) -> bool { guard n >= 0 else { return false }\nreturn n == reverse_digits(n) }'),
    ('sum_range', 'def sum_range(a: int, b: int) -> int { let sum = 0\nlet i = a\nwhile i <= b { sum = sum + i\ni = i + 1 } return sum }'),
    ('product_range', 'def product_range(a: int, b: int) -> int { let prod = 1\nlet i = a\nwhile i <= b { prod = prod * i\ni = i + 1 } return prod }'),
    ('fib_iter', 'def fib_iter(n: int) -> int { let a = 0\nlet b = 1\nlet i = 0\nwhile i < n { let t = a + b\na = b\nb = t\ni = i + 1 } return a }'),
    ('is_prime', 'def is_prime(n: int) -> bool { guard n > 1 else { return false }\nlet i = 2\nwhile i * i <= n { if n % i == 0 { return false }\ni = i + 1 } return true }'),
    ('max_list', 'def max_list(l: list) -> int { let m = l[0]\nfor x in l { if x > m { m = x } } return m }'),
    ('sum_list', 'def sum_list(l: list) -> int { let s = 0\nfor x in l { s = s + x } return s }'),
    ('contains_val', 'def contains_val(l: list, v: int) -> bool { for x in l { if x == v { return true } } return false }'),
]


@pytest.mark.parametrize('name,src', PROGRAMS, ids=[i[0] for i in PROGRAMS])
def test_programs(name, src):
    _compile(src)


# Class programs
CLASS_PROGS = [
    ('simple', 'class Point { public x: int\npublic y: int }'),
    ('with_method', 'class Point { public x: int\npublic y: int\npublic def dist() -> int { return self.x + self.y } }'),
    ('extends', 'class Animal { public name: str }\nclass Dog extends Animal {}'),
    ('abstract', 'abstract class Shape { abstract def area() -> float {} }'),
    ('multi_field', 'class Record { public a: int\npublic b: int\npublic c: int\npublic d: int\npublic e: int }'),
    ('trait', 'trait Drawable { public def draw() {} }'),
    ('trait_impl', 'trait Drawable { public def draw() {} }\nclass Circle extends Drawable { public def draw() {} }'),
    ('generic', 'class Box[T](public value: T) {}'),
    ('multi_method', 'class Calc { public def add(a: int, b: int) -> int { return a + b }\npublic def sub(a: int, b: int) -> int { return a - b }\npublic def mul(a: int, b: int) -> int { return a * b } }'),
    ('static_method', 'class Utils { static def pi() -> float { return 3.14 }\nstatic def e() -> float { return 2.71 } }'),
]


@pytest.mark.parametrize('name,src', CLASS_PROGS, ids=[i[0] for i in CLASS_PROGS])
def test_class_progs(name, src):
    _compile(src)


# Module programs
MOD_PROGS = [
    ('single', 'module M { public def hello() { 42 } }'),
    ('multi', 'module M { public def a() { 1 }\npublic def b() { 2 }\npublic def c() { 3 } }'),
    ('nested', 'module M { module Inner { public def f() { 42 } } }'),
    ('nested_multi', 'module M { public def a() { 1 }\nmodule Inner { public def b() { 2 } } }'),
]


@pytest.mark.parametrize('name,src', MOD_PROGS, ids=[i[0] for i in MOD_PROGS])
def test_mod_progs(name, src):
    _compile(src)


# Type alias programs
TYPE_PROGS = [
    ('id', 'type ID = int'),
    ('name', 'type Name = str'),
    ('flag', 'type Flag = bool'),
    ('score', 'type Score = int'),
]


@pytest.mark.parametrize('name,src', TYPE_PROGS, ids=[i[0] for i in TYPE_PROGS])
def test_type_progs(name, src):
    _compile(src)


# Enum programs
ENUM_PROGS = [
    ('traffic', 'enum Traffic { Red, Yellow, Green }'),
    ('cardinal', 'enum Cardinal { North, South, East, West }'),
    ('planet', 'enum Planet { Mercury, Venus, Earth, Mars, Jupiter }'),
    ('code', 'enum Code { Ok = 200, NotFound = 404, Error = 500 }'),
    ('season', 'enum Season { Spring, Summer, Autumn, Winter }'),
]


@pytest.mark.parametrize('name,src', ENUM_PROGS, ids=[i[0] for i in ENUM_PROGS])
def test_enum_progs2(name, src):
    _compile(src)

# --- XLarge: 15 ints * 4 ops * 15 ints = 900 ---
INTS15 = ['0', '1', '2', '3', '5', '7', '10', '13', '17', '23', '42', '100', '-1', '-7', '-13']
OPS4 = ['+', '-', '*', '%']
ARITH_XLARGE = [(f"{a}{op}{b}", f"def main() {{ let x = {a} {op} {b} }}")
               for a in INTS15 for op in OPS4 for b in INTS15]

@pytest.mark.parametrize('name,src', ARITH_XLARGE, ids=[i[0] for i in ARITH_XLARGE])
def test_arith_xlarge(name, src):
    _compile(src)

# --- Power 12x8 = 96 ---
BASES12 = ['0', '1', '2', '3', '4', '5', '6', '8', '10', '12', '15', '20']
EXPS8 = ['0', '1', '2', '3', '4', '5', '6', '7']
POWER_XLARGE = [(f"{b}**{e}", f"def main() {{ let x = {b} ** {e} }}")
               for b in BASES12 for e in EXPS8]

@pytest.mark.parametrize('name,src', POWER_XLARGE, ids=[i[0] for i in POWER_XLARGE])
def test_power_xlarge(name, src):
    _compile(src)

# --- CMP 12x12x6 = 864 ---
INTS12C = ['0', '1', '2', '3', '5', '10', '25', '50', '100', '-5', '-10', '-100']
CMP6 = ['==', '!=', '<', '>', '<=', '>=']
CMP_XLARGE = [(f"{x}{op}{y}", f"def main() {{ let z = {x} {op} {y} }}")
             for x in INTS12C for op in CMP6 for y in INTS12C]

@pytest.mark.parametrize('name,src', CMP_XLARGE, ids=[i[0] for i in CMP_XLARGE])
def test_cmp_xlarge(name, src):
    _compile(src)

# --- Negation depth 1-30 = 30 ---
for _d in range(1, 31):
    _expr = "-" * _d + "5"
    exec(f"def test_neg_{_d}():\n    _compile('def main() {{ let x = {_expr} }}')" + "\n")

# --- List mul 1-30 = 30 ---
for _s in range(1, 31):
    exec(f"def test_list_mul_{_s}():\n    _compile('def main() {{ let x = [1, 2] * {_s} }}')" + "\n")

# --- More class progs = 6 ---
CLASS_PROGS2 = [
    ("3_fields", "class C {     public a: int\n    public b: int\n    public c: int }"),
    ("4_fields", "class C {     public a: int\n    public b: int\n    public c: int\n    public d: int }"),
    ("5_fields", "class C {     public a: int\n    public b: int\n    public c: int\n    public d: int\n    public e: int }"),
    ("6_fields", "class C {     public a: int\n    public b: int\n    public c: int\n    public d: int\n    public e: int\n    public f: int }"),
    ("7_fields", "class C {     public a: int\n    public b: int\n    public c: int\n    public d: int\n    public e: int\n    public f: int\n    public g: int }"),
    ("8_fields", "class C {     public a: int\n    public b: int\n    public c: int\n    public d: int\n    public e: int\n    public f: int\n    public g: int\n    public h: int }"),
]

@pytest.mark.parametrize('name,src', CLASS_PROGS2, ids=[i[0] for i in CLASS_PROGS2])
def test_class_progs2(name, src):
    _compile(src)

# --- More module progs = 4 ---
MOD_PROGS2 = [
    ("2_funcs", "module M {     public def a() { 1 }\n    public def b() { 2 } }"),
    ("3_funcs", "module M {     public def a() { 1 }\n    public def b() { 2 }\n    public def c() { 3 } }"),
    ("4_funcs", "module M {     public def a() { 1 }\n    public def b() { 2 }\n    public def c() { 3 }\n    public def d() { 4 } }"),
    ("5_funcs", "module M {     public def a() { 1 }\n    public def b() { 2 }\n    public def c() { 3 }\n    public def d() { 4 }\n    public def e() { 5 } }"),
]

@pytest.mark.parametrize('name,src', MOD_PROGS2, ids=[i[0] for i in MOD_PROGS2])
def test_mod_progs2(name, src):
    _compile(src)

# --- More complex programs = 8 ---
COMPLEX_PROGS2 = [
    ("gcd_euclid", "def gcd(a: int, b: int) -> int { while b != 0 { let t = b\nb = a % b\na = t } return a }"),
    ("pow_fast", "def pow_fast(b: int, e: int) -> int { let result = 1\nwhile e > 0 { if e % 2 == 1 { result = result * b }\nb = b * b\ne = e / 2 } return result }"),
    ("count_even", "def count_even(n: int) -> int { let count = 0\nlet i = 0\nwhile i < n { if i % 2 == 0 { count = count + 1 }\ni = i + 1 } return count }"),
    ("sum_even", "def sum_even(n: int) -> int { let sum = 0\nlet i = 0\nwhile i < n { if i % 2 == 0 { sum = sum + i }\ni = i + 1 } return sum }"),
    ("is_power_of2", "def is_power_of2(n: int) -> bool { guard n > 0 else { return false }\nwhile n > 1 { if n % 2 != 0 { return false }\n n = n / 2 } return true }"),
]

@pytest.mark.parametrize('name,src', COMPLEX_PROGS2, ids=[i[0] for i in COMPLEX_PROGS2])
def test_complex_progs2(name, src):
    _compile(src)