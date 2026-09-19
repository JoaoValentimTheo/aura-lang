"""Heavy combinatorial tests - batch 7d."""
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

FLOAT_ARITH = [
    ("0.0+0.0", "def main() { let x = 0.0 + 0.0 }"),
    ("0.0+1.0", "def main() { let x = 0.0 + 1.0 }"),
    ("0.0+3.14", "def main() { let x = 0.0 + 3.14 }"),
    ("0.0+2.71", "def main() { let x = 0.0 + 2.71 }"),
    ("0.0+0.5", "def main() { let x = 0.0 + 0.5 }"),
    ("0.0+10.0", "def main() { let x = 0.0 + 10.0 }"),
    ("0.0+-1.0", "def main() { let x = 0.0 + -1.0 }"),
    ("0.0+-3.14", "def main() { let x = 0.0 + -3.14 }"),
    ("0.0-0.0", "def main() { let x = 0.0 - 0.0 }"),
    ("0.0-1.0", "def main() { let x = 0.0 - 1.0 }"),
    ("0.0-3.14", "def main() { let x = 0.0 - 3.14 }"),
    ("0.0-2.71", "def main() { let x = 0.0 - 2.71 }"),
    ("0.0-0.5", "def main() { let x = 0.0 - 0.5 }"),
    ("0.0-10.0", "def main() { let x = 0.0 - 10.0 }"),
    ("0.0--1.0", "def main() { let x = 0.0 - -1.0 }"),
    ("0.0--3.14", "def main() { let x = 0.0 - -3.14 }"),
    ("0.0*0.0", "def main() { let x = 0.0 * 0.0 }"),
    ("0.0*1.0", "def main() { let x = 0.0 * 1.0 }"),
    ("0.0*3.14", "def main() { let x = 0.0 * 3.14 }"),
    ("0.0*2.71", "def main() { let x = 0.0 * 2.71 }"),
    ("0.0*0.5", "def main() { let x = 0.0 * 0.5 }"),
    ("0.0*10.0", "def main() { let x = 0.0 * 10.0 }"),
    ("0.0*-1.0", "def main() { let x = 0.0 * -1.0 }"),
    ("0.0*-3.14", "def main() { let x = 0.0 * -3.14 }"),
    ("1.0+0.0", "def main() { let x = 1.0 + 0.0 }"),
    ("1.0+1.0", "def main() { let x = 1.0 + 1.0 }"),
    ("1.0+3.14", "def main() { let x = 1.0 + 3.14 }"),
    ("1.0+2.71", "def main() { let x = 1.0 + 2.71 }"),
    ("1.0+0.5", "def main() { let x = 1.0 + 0.5 }"),
    ("1.0+10.0", "def main() { let x = 1.0 + 10.0 }"),
    ("1.0+-1.0", "def main() { let x = 1.0 + -1.0 }"),
    ("1.0+-3.14", "def main() { let x = 1.0 + -3.14 }"),
    ("1.0-0.0", "def main() { let x = 1.0 - 0.0 }"),
    ("1.0-1.0", "def main() { let x = 1.0 - 1.0 }"),
    ("1.0-3.14", "def main() { let x = 1.0 - 3.14 }"),
    ("1.0-2.71", "def main() { let x = 1.0 - 2.71 }"),
    ("1.0-0.5", "def main() { let x = 1.0 - 0.5 }"),
    ("1.0-10.0", "def main() { let x = 1.0 - 10.0 }"),
    ("1.0--1.0", "def main() { let x = 1.0 - -1.0 }"),
    ("1.0--3.14", "def main() { let x = 1.0 - -3.14 }"),
    ("1.0*0.0", "def main() { let x = 1.0 * 0.0 }"),
    ("1.0*1.0", "def main() { let x = 1.0 * 1.0 }"),
    ("1.0*3.14", "def main() { let x = 1.0 * 3.14 }"),
    ("1.0*2.71", "def main() { let x = 1.0 * 2.71 }"),
    ("1.0*0.5", "def main() { let x = 1.0 * 0.5 }"),
    ("1.0*10.0", "def main() { let x = 1.0 * 10.0 }"),
    ("1.0*-1.0", "def main() { let x = 1.0 * -1.0 }"),
    ("1.0*-3.14", "def main() { let x = 1.0 * -3.14 }"),
    ("3.14+0.0", "def main() { let x = 3.14 + 0.0 }"),
    ("3.14+1.0", "def main() { let x = 3.14 + 1.0 }"),
    ("3.14+3.14", "def main() { let x = 3.14 + 3.14 }"),
    ("3.14+2.71", "def main() { let x = 3.14 + 2.71 }"),
    ("3.14+0.5", "def main() { let x = 3.14 + 0.5 }"),
    ("3.14+10.0", "def main() { let x = 3.14 + 10.0 }"),
    ("3.14+-1.0", "def main() { let x = 3.14 + -1.0 }"),
    ("3.14+-3.14", "def main() { let x = 3.14 + -3.14 }"),
    ("3.14-0.0", "def main() { let x = 3.14 - 0.0 }"),
    ("3.14-1.0", "def main() { let x = 3.14 - 1.0 }"),
    ("3.14-3.14", "def main() { let x = 3.14 - 3.14 }"),
    ("3.14-2.71", "def main() { let x = 3.14 - 2.71 }"),
    ("3.14-0.5", "def main() { let x = 3.14 - 0.5 }"),
    ("3.14-10.0", "def main() { let x = 3.14 - 10.0 }"),
    ("3.14--1.0", "def main() { let x = 3.14 - -1.0 }"),
    ("3.14--3.14", "def main() { let x = 3.14 - -3.14 }"),
    ("3.14*0.0", "def main() { let x = 3.14 * 0.0 }"),
    ("3.14*1.0", "def main() { let x = 3.14 * 1.0 }"),
    ("3.14*3.14", "def main() { let x = 3.14 * 3.14 }"),
    ("3.14*2.71", "def main() { let x = 3.14 * 2.71 }"),
    ("3.14*0.5", "def main() { let x = 3.14 * 0.5 }"),
    ("3.14*10.0", "def main() { let x = 3.14 * 10.0 }"),
    ("3.14*-1.0", "def main() { let x = 3.14 * -1.0 }"),
    ("3.14*-3.14", "def main() { let x = 3.14 * -3.14 }"),
    ("2.71+0.0", "def main() { let x = 2.71 + 0.0 }"),
    ("2.71+1.0", "def main() { let x = 2.71 + 1.0 }"),
    ("2.71+3.14", "def main() { let x = 2.71 + 3.14 }"),
    ("2.71+2.71", "def main() { let x = 2.71 + 2.71 }"),
    ("2.71+0.5", "def main() { let x = 2.71 + 0.5 }"),
    ("2.71+10.0", "def main() { let x = 2.71 + 10.0 }"),
    ("2.71+-1.0", "def main() { let x = 2.71 + -1.0 }"),
    ("2.71+-3.14", "def main() { let x = 2.71 + -3.14 }"),
    ("2.71-0.0", "def main() { let x = 2.71 - 0.0 }"),
    ("2.71-1.0", "def main() { let x = 2.71 - 1.0 }"),
    ("2.71-3.14", "def main() { let x = 2.71 - 3.14 }"),
    ("2.71-2.71", "def main() { let x = 2.71 - 2.71 }"),
    ("2.71-0.5", "def main() { let x = 2.71 - 0.5 }"),
    ("2.71-10.0", "def main() { let x = 2.71 - 10.0 }"),
    ("2.71--1.0", "def main() { let x = 2.71 - -1.0 }"),
    ("2.71--3.14", "def main() { let x = 2.71 - -3.14 }"),
    ("2.71*0.0", "def main() { let x = 2.71 * 0.0 }"),
    ("2.71*1.0", "def main() { let x = 2.71 * 1.0 }"),
    ("2.71*3.14", "def main() { let x = 2.71 * 3.14 }"),
    ("2.71*2.71", "def main() { let x = 2.71 * 2.71 }"),
    ("2.71*0.5", "def main() { let x = 2.71 * 0.5 }"),
    ("2.71*10.0", "def main() { let x = 2.71 * 10.0 }"),
    ("2.71*-1.0", "def main() { let x = 2.71 * -1.0 }"),
    ("2.71*-3.14", "def main() { let x = 2.71 * -3.14 }"),
    ("0.5+0.0", "def main() { let x = 0.5 + 0.0 }"),
    ("0.5+1.0", "def main() { let x = 0.5 + 1.0 }"),
    ("0.5+3.14", "def main() { let x = 0.5 + 3.14 }"),
    ("0.5+2.71", "def main() { let x = 0.5 + 2.71 }"),
    ("0.5+0.5", "def main() { let x = 0.5 + 0.5 }"),
    ("0.5+10.0", "def main() { let x = 0.5 + 10.0 }"),
    ("0.5+-1.0", "def main() { let x = 0.5 + -1.0 }"),
    ("0.5+-3.14", "def main() { let x = 0.5 + -3.14 }"),
    ("0.5-0.0", "def main() { let x = 0.5 - 0.0 }"),
    ("0.5-1.0", "def main() { let x = 0.5 - 1.0 }"),
    ("0.5-3.14", "def main() { let x = 0.5 - 3.14 }"),
    ("0.5-2.71", "def main() { let x = 0.5 - 2.71 }"),
    ("0.5-0.5", "def main() { let x = 0.5 - 0.5 }"),
    ("0.5-10.0", "def main() { let x = 0.5 - 10.0 }"),
    ("0.5--1.0", "def main() { let x = 0.5 - -1.0 }"),
    ("0.5--3.14", "def main() { let x = 0.5 - -3.14 }"),
    ("0.5*0.0", "def main() { let x = 0.5 * 0.0 }"),
    ("0.5*1.0", "def main() { let x = 0.5 * 1.0 }"),
    ("0.5*3.14", "def main() { let x = 0.5 * 3.14 }"),
    ("0.5*2.71", "def main() { let x = 0.5 * 2.71 }"),
    ("0.5*0.5", "def main() { let x = 0.5 * 0.5 }"),
    ("0.5*10.0", "def main() { let x = 0.5 * 10.0 }"),
    ("0.5*-1.0", "def main() { let x = 0.5 * -1.0 }"),
    ("0.5*-3.14", "def main() { let x = 0.5 * -3.14 }"),
    ("10.0+0.0", "def main() { let x = 10.0 + 0.0 }"),
    ("10.0+1.0", "def main() { let x = 10.0 + 1.0 }"),
    ("10.0+3.14", "def main() { let x = 10.0 + 3.14 }"),
    ("10.0+2.71", "def main() { let x = 10.0 + 2.71 }"),
    ("10.0+0.5", "def main() { let x = 10.0 + 0.5 }"),
    ("10.0+10.0", "def main() { let x = 10.0 + 10.0 }"),
    ("10.0+-1.0", "def main() { let x = 10.0 + -1.0 }"),
    ("10.0+-3.14", "def main() { let x = 10.0 + -3.14 }"),
    ("10.0-0.0", "def main() { let x = 10.0 - 0.0 }"),
    ("10.0-1.0", "def main() { let x = 10.0 - 1.0 }"),
    ("10.0-3.14", "def main() { let x = 10.0 - 3.14 }"),
    ("10.0-2.71", "def main() { let x = 10.0 - 2.71 }"),
    ("10.0-0.5", "def main() { let x = 10.0 - 0.5 }"),
    ("10.0-10.0", "def main() { let x = 10.0 - 10.0 }"),
    ("10.0--1.0", "def main() { let x = 10.0 - -1.0 }"),
    ("10.0--3.14", "def main() { let x = 10.0 - -3.14 }"),
    ("10.0*0.0", "def main() { let x = 10.0 * 0.0 }"),
    ("10.0*1.0", "def main() { let x = 10.0 * 1.0 }"),
    ("10.0*3.14", "def main() { let x = 10.0 * 3.14 }"),
    ("10.0*2.71", "def main() { let x = 10.0 * 2.71 }"),
    ("10.0*0.5", "def main() { let x = 10.0 * 0.5 }"),
    ("10.0*10.0", "def main() { let x = 10.0 * 10.0 }"),
    ("10.0*-1.0", "def main() { let x = 10.0 * -1.0 }"),
    ("10.0*-3.14", "def main() { let x = 10.0 * -3.14 }"),
    ("-1.0+0.0", "def main() { let x = -1.0 + 0.0 }"),
    ("-1.0+1.0", "def main() { let x = -1.0 + 1.0 }"),
    ("-1.0+3.14", "def main() { let x = -1.0 + 3.14 }"),
    ("-1.0+2.71", "def main() { let x = -1.0 + 2.71 }"),
    ("-1.0+0.5", "def main() { let x = -1.0 + 0.5 }"),
    ("-1.0+10.0", "def main() { let x = -1.0 + 10.0 }"),
    ("-1.0+-1.0", "def main() { let x = -1.0 + -1.0 }"),
    ("-1.0+-3.14", "def main() { let x = -1.0 + -3.14 }"),
    ("-1.0-0.0", "def main() { let x = -1.0 - 0.0 }"),
    ("-1.0-1.0", "def main() { let x = -1.0 - 1.0 }"),
    ("-1.0-3.14", "def main() { let x = -1.0 - 3.14 }"),
    ("-1.0-2.71", "def main() { let x = -1.0 - 2.71 }"),
    ("-1.0-0.5", "def main() { let x = -1.0 - 0.5 }"),
    ("-1.0-10.0", "def main() { let x = -1.0 - 10.0 }"),
    ("-1.0--1.0", "def main() { let x = -1.0 - -1.0 }"),
    ("-1.0--3.14", "def main() { let x = -1.0 - -3.14 }"),
    ("-1.0*0.0", "def main() { let x = -1.0 * 0.0 }"),
    ("-1.0*1.0", "def main() { let x = -1.0 * 1.0 }"),
    ("-1.0*3.14", "def main() { let x = -1.0 * 3.14 }"),
    ("-1.0*2.71", "def main() { let x = -1.0 * 2.71 }"),
    ("-1.0*0.5", "def main() { let x = -1.0 * 0.5 }"),
    ("-1.0*10.0", "def main() { let x = -1.0 * 10.0 }"),
    ("-1.0*-1.0", "def main() { let x = -1.0 * -1.0 }"),
    ("-1.0*-3.14", "def main() { let x = -1.0 * -3.14 }"),
    ("-3.14+0.0", "def main() { let x = -3.14 + 0.0 }"),
    ("-3.14+1.0", "def main() { let x = -3.14 + 1.0 }"),
    ("-3.14+3.14", "def main() { let x = -3.14 + 3.14 }"),
    ("-3.14+2.71", "def main() { let x = -3.14 + 2.71 }"),
    ("-3.14+0.5", "def main() { let x = -3.14 + 0.5 }"),
    ("-3.14+10.0", "def main() { let x = -3.14 + 10.0 }"),
    ("-3.14+-1.0", "def main() { let x = -3.14 + -1.0 }"),
    ("-3.14+-3.14", "def main() { let x = -3.14 + -3.14 }"),
    ("-3.14-0.0", "def main() { let x = -3.14 - 0.0 }"),
    ("-3.14-1.0", "def main() { let x = -3.14 - 1.0 }"),
    ("-3.14-3.14", "def main() { let x = -3.14 - 3.14 }"),
    ("-3.14-2.71", "def main() { let x = -3.14 - 2.71 }"),
    ("-3.14-0.5", "def main() { let x = -3.14 - 0.5 }"),
    ("-3.14-10.0", "def main() { let x = -3.14 - 10.0 }"),
    ("-3.14--1.0", "def main() { let x = -3.14 - -1.0 }"),
    ("-3.14--3.14", "def main() { let x = -3.14 - -3.14 }"),
    ("-3.14*0.0", "def main() { let x = -3.14 * 0.0 }"),
    ("-3.14*1.0", "def main() { let x = -3.14 * 1.0 }"),
    ("-3.14*3.14", "def main() { let x = -3.14 * 3.14 }"),
    ("-3.14*2.71", "def main() { let x = -3.14 * 2.71 }"),
    ("-3.14*0.5", "def main() { let x = -3.14 * 0.5 }"),
    ("-3.14*10.0", "def main() { let x = -3.14 * 10.0 }"),
    ("-3.14*-1.0", "def main() { let x = -3.14 * -1.0 }"),
    ("-3.14*-3.14", "def main() { let x = -3.14 * -3.14 }"),
]

@pytest.mark.parametrize("name,src", FLOAT_ARITH, ids=[i[0] for i in FLOAT_ARITH])
def test_float_arith(name, src):
    _compile(src)

BOOL_COMBOS = [
    ("true_and_true", "def main() { let x = true and true }"),
    ("true_and_false", "def main() { let x = true and false }"),
    ("true_or_true", "def main() { let x = true or true }"),
    ("true_or_false", "def main() { let x = true or false }"),
    ("false_and_true", "def main() { let x = false and true }"),
    ("false_and_false", "def main() { let x = false and false }"),
    ("false_or_true", "def main() { let x = false or true }"),
    ("false_or_false", "def main() { let x = false or false }"),
]

@pytest.mark.parametrize("name,src", BOOL_COMBOS, ids=[i[0] for i in BOOL_COMBOS])
def test_bool_combos(name, src):
    _compile(src)

UNARY_CASES = [
    ("neg_int", "def main() { let x = -5 }"),
    ("neg_float", "def main() { let x = -3.14 }"),
    ("not_true", "def main() { let x = not true }"),
    ("not_false", "def main() { let x = not false }"),
    ("double_neg", "def main() { let x = --5 }"),
    ("triple_neg", "def main() { let x = ---5 }"),
]

@pytest.mark.parametrize("name,src", UNARY_CASES, ids=[i[0] for i in UNARY_CASES])
def test_unary(name, src):
    _compile(src)

TERNARY_CASES = [
    ("simple_true", "def main() { let x = true ? 1 : 2 }"),
    ("simple_false", "def main() { let x = false ? 1 : 2 }"),
    ("cmp_lt", "def main() { let x = 1 < 2 ? 10 : 20 }"),
    ("nested", "def main() { let x = true ? (false ? 1 : 2) : 3 }"),
]

@pytest.mark.parametrize("name,src", TERNARY_CASES, ids=[i[0] for i in TERNARY_CASES])
def test_ternary(name, src):
    _compile(src)

PIPE_CASES = [
    ("pipe_1", "def main() { let x = 5 |> (n) => n + 1 }"),
    ("pipe_2", "def main() { let x = 5 |> (n) => n * 2 |> (n) => n + 1 }"),
]

@pytest.mark.parametrize("name,src", PIPE_CASES, ids=[i[0] for i in PIPE_CASES])
def test_pipe(name, src):
    _compile(src)

RANGE_CASES = [
    ("range_0_1", "def main() { let x = 0..1 }"),
    ("range_0_5", "def main() { let x = 0..5 }"),
    ("range_1_10", "def main() { let x = 1..10 }"),
    ("range_-5_5", "def main() { let x = -5..5 }"),
]

@pytest.mark.parametrize("name,src", RANGE_CASES, ids=[i[0] for i in RANGE_CASES])
def test_range(name, src):
    _compile(src)

def test_list_size_0():
    _compile("def main() { let x = [] }")

def test_list_size_1():
    _compile("def main() { let x = [0] }")

def test_list_size_2():
    _compile("def main() { let x = [0, 1] }")

def test_list_size_3():
    _compile("def main() { let x = [0, 1, 2] }")

def test_list_size_4():
    _compile("def main() { let x = [0, 1, 2, 3] }")

def test_list_size_5():
    _compile("def main() { let x = [0, 1, 2, 3, 4] }")

def test_list_size_6():
    _compile("def main() { let x = [0, 1, 2, 3, 4, 5] }")

def test_list_size_7():
    _compile("def main() { let x = [0, 1, 2, 3, 4, 5, 6] }")

def test_list_size_8():
    _compile("def main() { let x = [0, 1, 2, 3, 4, 5, 6, 7] }")

def test_list_size_9():
    _compile("def main() { let x = [0, 1, 2, 3, 4, 5, 6, 7, 8] }")

def test_list_size_10():
    _compile("def main() { let x = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] }")

def test_dict_size_0():
    _compile("def main() { let x = {} }")

def test_dict_size_1():
    _compile("def main() { let x = {\"a\": 0} }")

def test_dict_size_2():
    _compile("def main() { let x = {\"a\": 0, \"b\": 1} }")

def test_dict_size_3():
    _compile("def main() { let x = {\"a\": 0, \"b\": 1, \"c\": 2} }")

def test_dict_size_4():
    _compile("def main() { let x = {\"a\": 0, \"b\": 1, \"c\": 2, \"d\": 3} }")

def test_dict_size_5():
    _compile("def main() { let x = {\"a\": 0, \"b\": 1, \"c\": 2, \"d\": 3, \"e\": 4} }")

def test_dict_size_6():
    _compile("def main() { let x = {\"a\": 0, \"b\": 1, \"c\": 2, \"d\": 3, \"e\": 4, \"f\": 5} }")

def test_dict_size_7():
    _compile("def main() { let x = {\"a\": 0, \"b\": 1, \"c\": 2, \"d\": 3, \"e\": 4, \"f\": 5, \"g\": 6} }")

def test_dict_size_8():
    _compile("def main() { let x = {\"a\": 0, \"b\": 1, \"c\": 2, \"d\": 3, \"e\": 4, \"f\": 5, \"g\": 6, \"h\": 7} }")

def test_dict_size_9():
    _compile("def main() { let x = {\"a\": 0, \"b\": 1, \"c\": 2, \"d\": 3, \"e\": 4, \"f\": 5, \"g\": 6, \"h\": 7, \"i\": 8} }")

def test_dict_size_10():
    _compile("def main() { let x = {\"a\": 0, \"b\": 1, \"c\": 2, \"d\": 3, \"e\": 4, \"f\": 5, \"g\": 6, \"h\": 7, \"i\": 8, \"j\": 9} }")

FOR_COLL_CASES = [
    ("list_literal", "def main() { let mut sum = 0\nfor i in [1, 2, 3, 4, 5] { sum = sum + 1 } }"),
    ("range_0_10", "def main() { let mut sum = 0\nfor i in 0..10 { sum = sum + 1 } }"),
]

@pytest.mark.parametrize("name,src", FOR_COLL_CASES, ids=[i[0] for i in FOR_COLL_CASES])
def test_for_collections(name, src):
    _compile(src)

def test_nested_loop_1():
    _compile("def main() { let mut k = 0\nfor j in 0..3 { k = k + 1 } }")

def test_nested_loop_2():
    _compile("def main() { let mut k = 0\nfor j in 0..3 { k = k + 1 }\nfor j in 0..3 { k = k + 1 } }")

def test_nested_loop_3():
    _compile("def main() { let mut k = 0\nfor j in 0..3 { k = k + 1 }\nfor j in 0..3 { k = k + 1 }\nfor j in 0..3 { k = k + 1 } }")

def test_nested_loop_4():
    _compile("def main() { let mut k = 0\nfor j in 0..3 { k = k + 1 }\nfor j in 0..3 { k = k + 1 }\nfor j in 0..3 { k = k + 1 }\nfor j in 0..3 { k = k + 1 } }")

def test_nested_loop_5():
    _compile("def main() { let mut k = 0\nfor j in 0..3 { k = k + 1 }\nfor j in 0..3 { k = k + 1 }\nfor j in 0..3 { k = k + 1 }\nfor j in 0..3 { k = k + 1 }\nfor j in 0..3 { k = k + 1 } }")

def test_class_fields_1():
    _compile("class C {\n    public x0: int\n}")

def test_class_fields_2():
    _compile("class C {\n    public x0: int\n    public x1: int\n}")

def test_class_fields_3():
    _compile("class C {\n    public x0: int\n    public x1: int\n    public x2: int\n}")

def test_class_fields_4():
    _compile("class C {\n    public x0: int\n    public x1: int\n    public x2: int\n    public x3: int\n}")

def test_class_fields_5():
    _compile("class C {\n    public x0: int\n    public x1: int\n    public x2: int\n    public x3: int\n    public x4: int\n}")

def test_class_fields_6():
    _compile("class C {\n    public x0: int\n    public x1: int\n    public x2: int\n    public x3: int\n    public x4: int\n    public x5: int\n}")

def test_class_fields_7():
    _compile("class C {\n    public x0: int\n    public x1: int\n    public x2: int\n    public x3: int\n    public x4: int\n    public x5: int\n    public x6: int\n}")

def test_module_funcs_1():
    _compile("module M {\n    public def f0() -> int { return 0 }\n}")

def test_module_funcs_2():
    _compile("module M {\n    public def f0() -> int { return 0 }\n    public def f1() -> int { return 1 }\n}")

def test_module_funcs_3():
    _compile("module M {\n    public def f0() -> int { return 0 }\n    public def f1() -> int { return 1 }\n    public def f2() -> int { return 2 }\n}")

def test_module_funcs_4():
    _compile("module M {\n    public def f0() -> int { return 0 }\n    public def f1() -> int { return 1 }\n    public def f2() -> int { return 2 }\n    public def f3() -> int { return 3 }\n}")

def test_module_funcs_5():
    _compile("module M {\n    public def f0() -> int { return 0 }\n    public def f1() -> int { return 1 }\n    public def f2() -> int { return 2 }\n    public def f3() -> int { return 3 }\n    public def f4() -> int { return 4 }\n}")

def test_module_funcs_6():
    _compile("module M {\n    public def f0() -> int { return 0 }\n    public def f1() -> int { return 1 }\n    public def f2() -> int { return 2 }\n    public def f3() -> int { return 3 }\n    public def f4() -> int { return 4 }\n    public def f5() -> int { return 5 }\n}")

def test_module_funcs_7():
    _compile("module M {\n    public def f0() -> int { return 0 }\n    public def f1() -> int { return 1 }\n    public def f2() -> int { return 2 }\n    public def f3() -> int { return 3 }\n    public def f4() -> int { return 4 }\n    public def f5() -> int { return 5 }\n    public def f6() -> int { return 6 }\n}")

def test_match_1_cases():
    _compile("def main() { let x = 0\nmatch x {\n  case 0 { 0 }\n  case _ { -1 }\n} }")

def test_match_2_cases():
    _compile("def main() { let x = 1\nmatch x {\n  case 0 { 0 }\n  case 1 { 1 }\n  case _ { -1 }\n} }")

def test_match_3_cases():
    _compile("def main() { let x = 1\nmatch x {\n  case 0 { 0 }\n  case 1 { 1 }\n  case 2 { 2 }\n  case _ { -1 }\n} }")

def test_match_5_cases():
    _compile("def main() { let x = 2\nmatch x {\n  case 0 { 0 }\n  case 1 { 1 }\n  case 2 { 2 }\n  case 3 { 3 }\n  case 4 { 4 }\n  case _ { -1 }\n} }")

def test_match_8_cases():
    _compile("def main() { let x = 4\nmatch x {\n  case 0 { 0 }\n  case 1 { 1 }\n  case 2 { 2 }\n  case 3 { 3 }\n  case 4 { 4 }\n  case 5 { 5 }\n  case 6 { 6 }\n  case 7 { 7 }\n  case _ { -1 }\n} }")

def test_match_10_cases():
    _compile("def main() { let x = 5\nmatch x {\n  case 0 { 0 }\n  case 1 { 1 }\n  case 2 { 2 }\n  case 3 { 3 }\n  case 4 { 4 }\n  case 5 { 5 }\n  case 6 { 6 }\n  case 7 { 7 }\n  case 8 { 8 }\n  case 9 { 9 }\n  case _ { -1 }\n} }")

def test_try_depth_1():
    _compile("def main() { try { let x = 1 } catch e { print(e) } }")

def test_try_depth_2():
    _compile("def main() { try { let x = 1 } catch e { print(e) }try { let x = 1 } catch e { print(e) } }")

def test_try_depth_3():
    _compile("def main() { try { let x = 1 } catch e { print(e) }try { let x = 1 } catch e { print(e) }try { let x = 1 } catch e { print(e) } }")

def test_enum_2_values():
    _compile("enum E { V0, V1 }")

def test_enum_3_values():
    _compile("enum E { V0, V1, V2 }")

def test_enum_5_values():
    _compile("enum E { V0, V1, V2, V3, V4 }")

def test_enum_8_values():
    _compile("enum E { V0, V1, V2, V3, V4, V5, V6, V7 }")

GUARD_COND_CASES = [
    ("x_gt_0", "def f(x: int) -> int { guard x > 0 else { return 0 } return x }"),
    ("x_lt_10", "def f(x: int) -> int { guard x < 10 else { return 0 } return x }"),
    ("x_eq_5", "def f(x: int) -> int { guard x == 5 else { return 0 } return x }"),
    ("x_ne_0", "def f(x: int) -> int { guard x != 0 else { return 0 } return x }"),
    ("x_gt=_1", "def f(x: int) -> int { guard x >= 1 else { return 0 } return x }"),
]

@pytest.mark.parametrize("name,src", GUARD_COND_CASES, ids=[i[0] for i in GUARD_COND_CASES])
def test_guard_conditions(name, src):
    _compile(src)

TYPE_PROG_CASES = [
    ("type_id", "type ID = int"),
    ("type_name", "type Name = str"),
]

@pytest.mark.parametrize("name,src", TYPE_PROG_CASES, ids=[i[0] for i in TYPE_PROG_CASES])
def test_type_programs(name, src):
    _compile(src)

ABSTRACT_PROGRAMS = [
    ("abstract_single", "abstract class Shape { abstract def area() -> float {} }"),
]

@pytest.mark.parametrize("name,src", ABSTRACT_PROGRAMS, ids=[i[0] for i in ABSTRACT_PROGRAMS])
def test_abstract_programs(name, src):
    _compile(src)

TRAIT_PROGRAMS = [
    ("trait_simple", "trait Drawable { public def draw() {} }"),
]

@pytest.mark.parametrize("name,src", TRAIT_PROGRAMS, ids=[i[0] for i in TRAIT_PROGRAMS])
def test_trait_programs(name, src):
    _compile(src)