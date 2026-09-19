"""Full program combinatorial tests - batch 7c."""
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

ARITH_PROGRAMS = [
    ("0+0", "def main() { let x = 0 + 0 }"),
    ("0+1", "def main() { let x = 0 + 1 }"),
    ("0+2", "def main() { let x = 0 + 2 }"),
    ("0+3", "def main() { let x = 0 + 3 }"),
    ("0+5", "def main() { let x = 0 + 5 }"),
    ("0+10", "def main() { let x = 0 + 10 }"),
    ("0+42", "def main() { let x = 0 + 42 }"),
    ("0+100", "def main() { let x = 0 + 100 }"),
    ("0+-1", "def main() { let x = 0 + -1 }"),
    ("0+-5", "def main() { let x = 0 + -5 }"),
    ("0-0", "def main() { let x = 0 - 0 }"),
    ("0-1", "def main() { let x = 0 - 1 }"),
    ("0-2", "def main() { let x = 0 - 2 }"),
    ("0-3", "def main() { let x = 0 - 3 }"),
    ("0-5", "def main() { let x = 0 - 5 }"),
    ("0-10", "def main() { let x = 0 - 10 }"),
    ("0-42", "def main() { let x = 0 - 42 }"),
    ("0-100", "def main() { let x = 0 - 100 }"),
    ("0--1", "def main() { let x = 0 - -1 }"),
    ("0--5", "def main() { let x = 0 - -5 }"),
    ("0*0", "def main() { let x = 0 * 0 }"),
    ("0*1", "def main() { let x = 0 * 1 }"),
    ("0*2", "def main() { let x = 0 * 2 }"),
    ("0*3", "def main() { let x = 0 * 3 }"),
    ("0*5", "def main() { let x = 0 * 5 }"),
    ("0*10", "def main() { let x = 0 * 10 }"),
    ("0*42", "def main() { let x = 0 * 42 }"),
    ("0*100", "def main() { let x = 0 * 100 }"),
    ("0*-1", "def main() { let x = 0 * -1 }"),
    ("0*-5", "def main() { let x = 0 * -5 }"),
    ("0%0", "def main() { let x = 0 % 0 }"),
    ("0%1", "def main() { let x = 0 % 1 }"),
    ("0%2", "def main() { let x = 0 % 2 }"),
    ("0%3", "def main() { let x = 0 % 3 }"),
    ("0%5", "def main() { let x = 0 % 5 }"),
    ("0%10", "def main() { let x = 0 % 10 }"),
    ("0%42", "def main() { let x = 0 % 42 }"),
    ("0%100", "def main() { let x = 0 % 100 }"),
    ("0%-1", "def main() { let x = 0 % -1 }"),
    ("0%-5", "def main() { let x = 0 % -5 }"),
    ("1+0", "def main() { let x = 1 + 0 }"),
    ("1+1", "def main() { let x = 1 + 1 }"),
    ("1+2", "def main() { let x = 1 + 2 }"),
    ("1+3", "def main() { let x = 1 + 3 }"),
    ("1+5", "def main() { let x = 1 + 5 }"),
    ("1+10", "def main() { let x = 1 + 10 }"),
    ("1+42", "def main() { let x = 1 + 42 }"),
    ("1+100", "def main() { let x = 1 + 100 }"),
    ("1+-1", "def main() { let x = 1 + -1 }"),
    ("1+-5", "def main() { let x = 1 + -5 }"),
    ("1-0", "def main() { let x = 1 - 0 }"),
    ("1-1", "def main() { let x = 1 - 1 }"),
    ("1-2", "def main() { let x = 1 - 2 }"),
    ("1-3", "def main() { let x = 1 - 3 }"),
    ("1-5", "def main() { let x = 1 - 5 }"),
    ("1-10", "def main() { let x = 1 - 10 }"),
    ("1-42", "def main() { let x = 1 - 42 }"),
    ("1-100", "def main() { let x = 1 - 100 }"),
    ("1--1", "def main() { let x = 1 - -1 }"),
    ("1--5", "def main() { let x = 1 - -5 }"),
    ("1*0", "def main() { let x = 1 * 0 }"),
    ("1*1", "def main() { let x = 1 * 1 }"),
    ("1*2", "def main() { let x = 1 * 2 }"),
    ("1*3", "def main() { let x = 1 * 3 }"),
    ("1*5", "def main() { let x = 1 * 5 }"),
    ("1*10", "def main() { let x = 1 * 10 }"),
    ("1*42", "def main() { let x = 1 * 42 }"),
    ("1*100", "def main() { let x = 1 * 100 }"),
    ("1*-1", "def main() { let x = 1 * -1 }"),
    ("1*-5", "def main() { let x = 1 * -5 }"),
    ("1%0", "def main() { let x = 1 % 0 }"),
    ("1%1", "def main() { let x = 1 % 1 }"),
    ("1%2", "def main() { let x = 1 % 2 }"),
    ("1%3", "def main() { let x = 1 % 3 }"),
    ("1%5", "def main() { let x = 1 % 5 }"),
    ("1%10", "def main() { let x = 1 % 10 }"),
    ("1%42", "def main() { let x = 1 % 42 }"),
    ("1%100", "def main() { let x = 1 % 100 }"),
    ("1%-1", "def main() { let x = 1 % -1 }"),
    ("1%-5", "def main() { let x = 1 % -5 }"),
    ("2+0", "def main() { let x = 2 + 0 }"),
    ("2+1", "def main() { let x = 2 + 1 }"),
    ("2+2", "def main() { let x = 2 + 2 }"),
    ("2+3", "def main() { let x = 2 + 3 }"),
    ("2+5", "def main() { let x = 2 + 5 }"),
    ("2+10", "def main() { let x = 2 + 10 }"),
    ("2+42", "def main() { let x = 2 + 42 }"),
    ("2+100", "def main() { let x = 2 + 100 }"),
    ("2+-1", "def main() { let x = 2 + -1 }"),
    ("2+-5", "def main() { let x = 2 + -5 }"),
    ("2-0", "def main() { let x = 2 - 0 }"),
    ("2-1", "def main() { let x = 2 - 1 }"),
    ("2-2", "def main() { let x = 2 - 2 }"),
    ("2-3", "def main() { let x = 2 - 3 }"),
    ("2-5", "def main() { let x = 2 - 5 }"),
    ("2-10", "def main() { let x = 2 - 10 }"),
    ("2-42", "def main() { let x = 2 - 42 }"),
    ("2-100", "def main() { let x = 2 - 100 }"),
    ("2--1", "def main() { let x = 2 - -1 }"),
    ("2--5", "def main() { let x = 2 - -5 }"),
    ("2*0", "def main() { let x = 2 * 0 }"),
    ("2*1", "def main() { let x = 2 * 1 }"),
    ("2*2", "def main() { let x = 2 * 2 }"),
    ("2*3", "def main() { let x = 2 * 3 }"),
    ("2*5", "def main() { let x = 2 * 5 }"),
    ("2*10", "def main() { let x = 2 * 10 }"),
    ("2*42", "def main() { let x = 2 * 42 }"),
    ("2*100", "def main() { let x = 2 * 100 }"),
    ("2*-1", "def main() { let x = 2 * -1 }"),
    ("2*-5", "def main() { let x = 2 * -5 }"),
    ("2%0", "def main() { let x = 2 % 0 }"),
    ("2%1", "def main() { let x = 2 % 1 }"),
    ("2%2", "def main() { let x = 2 % 2 }"),
    ("2%3", "def main() { let x = 2 % 3 }"),
    ("2%5", "def main() { let x = 2 % 5 }"),
    ("2%10", "def main() { let x = 2 % 10 }"),
    ("2%42", "def main() { let x = 2 % 42 }"),
    ("2%100", "def main() { let x = 2 % 100 }"),
    ("2%-1", "def main() { let x = 2 % -1 }"),
    ("2%-5", "def main() { let x = 2 % -5 }"),
    ("3+0", "def main() { let x = 3 + 0 }"),
    ("3+1", "def main() { let x = 3 + 1 }"),
    ("3+2", "def main() { let x = 3 + 2 }"),
    ("3+3", "def main() { let x = 3 + 3 }"),
    ("3+5", "def main() { let x = 3 + 5 }"),
    ("3+10", "def main() { let x = 3 + 10 }"),
    ("3+42", "def main() { let x = 3 + 42 }"),
    ("3+100", "def main() { let x = 3 + 100 }"),
    ("3+-1", "def main() { let x = 3 + -1 }"),
    ("3+-5", "def main() { let x = 3 + -5 }"),
    ("3-0", "def main() { let x = 3 - 0 }"),
    ("3-1", "def main() { let x = 3 - 1 }"),
    ("3-2", "def main() { let x = 3 - 2 }"),
    ("3-3", "def main() { let x = 3 - 3 }"),
    ("3-5", "def main() { let x = 3 - 5 }"),
    ("3-10", "def main() { let x = 3 - 10 }"),
    ("3-42", "def main() { let x = 3 - 42 }"),
    ("3-100", "def main() { let x = 3 - 100 }"),
    ("3--1", "def main() { let x = 3 - -1 }"),
    ("3--5", "def main() { let x = 3 - -5 }"),
    ("3*0", "def main() { let x = 3 * 0 }"),
    ("3*1", "def main() { let x = 3 * 1 }"),
    ("3*2", "def main() { let x = 3 * 2 }"),
    ("3*3", "def main() { let x = 3 * 3 }"),
    ("3*5", "def main() { let x = 3 * 5 }"),
    ("3*10", "def main() { let x = 3 * 10 }"),
    ("3*42", "def main() { let x = 3 * 42 }"),
    ("3*100", "def main() { let x = 3 * 100 }"),
    ("3*-1", "def main() { let x = 3 * -1 }"),
    ("3*-5", "def main() { let x = 3 * -5 }"),
    ("3%0", "def main() { let x = 3 % 0 }"),
    ("3%1", "def main() { let x = 3 % 1 }"),
    ("3%2", "def main() { let x = 3 % 2 }"),
    ("3%3", "def main() { let x = 3 % 3 }"),
    ("3%5", "def main() { let x = 3 % 5 }"),
    ("3%10", "def main() { let x = 3 % 10 }"),
    ("3%42", "def main() { let x = 3 % 42 }"),
    ("3%100", "def main() { let x = 3 % 100 }"),
    ("3%-1", "def main() { let x = 3 % -1 }"),
    ("3%-5", "def main() { let x = 3 % -5 }"),
    ("5+0", "def main() { let x = 5 + 0 }"),
    ("5+1", "def main() { let x = 5 + 1 }"),
    ("5+2", "def main() { let x = 5 + 2 }"),
    ("5+3", "def main() { let x = 5 + 3 }"),
    ("5+5", "def main() { let x = 5 + 5 }"),
    ("5+10", "def main() { let x = 5 + 10 }"),
    ("5+42", "def main() { let x = 5 + 42 }"),
    ("5+100", "def main() { let x = 5 + 100 }"),
    ("5+-1", "def main() { let x = 5 + -1 }"),
    ("5+-5", "def main() { let x = 5 + -5 }"),
    ("5-0", "def main() { let x = 5 - 0 }"),
    ("5-1", "def main() { let x = 5 - 1 }"),
    ("5-2", "def main() { let x = 5 - 2 }"),
    ("5-3", "def main() { let x = 5 - 3 }"),
    ("5-5", "def main() { let x = 5 - 5 }"),
    ("5-10", "def main() { let x = 5 - 10 }"),
    ("5-42", "def main() { let x = 5 - 42 }"),
    ("5-100", "def main() { let x = 5 - 100 }"),
    ("5--1", "def main() { let x = 5 - -1 }"),
    ("5--5", "def main() { let x = 5 - -5 }"),
    ("5*0", "def main() { let x = 5 * 0 }"),
    ("5*1", "def main() { let x = 5 * 1 }"),
    ("5*2", "def main() { let x = 5 * 2 }"),
    ("5*3", "def main() { let x = 5 * 3 }"),
    ("5*5", "def main() { let x = 5 * 5 }"),
    ("5*10", "def main() { let x = 5 * 10 }"),
    ("5*42", "def main() { let x = 5 * 42 }"),
    ("5*100", "def main() { let x = 5 * 100 }"),
    ("5*-1", "def main() { let x = 5 * -1 }"),
    ("5*-5", "def main() { let x = 5 * -5 }"),
    ("5%0", "def main() { let x = 5 % 0 }"),
    ("5%1", "def main() { let x = 5 % 1 }"),
    ("5%2", "def main() { let x = 5 % 2 }"),
    ("5%3", "def main() { let x = 5 % 3 }"),
    ("5%5", "def main() { let x = 5 % 5 }"),
    ("5%10", "def main() { let x = 5 % 10 }"),
    ("5%42", "def main() { let x = 5 % 42 }"),
    ("5%100", "def main() { let x = 5 % 100 }"),
    ("5%-1", "def main() { let x = 5 % -1 }"),
    ("5%-5", "def main() { let x = 5 % -5 }"),
    ("10+0", "def main() { let x = 10 + 0 }"),
    ("10+1", "def main() { let x = 10 + 1 }"),
    ("10+2", "def main() { let x = 10 + 2 }"),
    ("10+3", "def main() { let x = 10 + 3 }"),
    ("10+5", "def main() { let x = 10 + 5 }"),
    ("10+10", "def main() { let x = 10 + 10 }"),
    ("10+42", "def main() { let x = 10 + 42 }"),
    ("10+100", "def main() { let x = 10 + 100 }"),
    ("10+-1", "def main() { let x = 10 + -1 }"),
    ("10+-5", "def main() { let x = 10 + -5 }"),
    ("10-0", "def main() { let x = 10 - 0 }"),
    ("10-1", "def main() { let x = 10 - 1 }"),
    ("10-2", "def main() { let x = 10 - 2 }"),
    ("10-3", "def main() { let x = 10 - 3 }"),
    ("10-5", "def main() { let x = 10 - 5 }"),
    ("10-10", "def main() { let x = 10 - 10 }"),
    ("10-42", "def main() { let x = 10 - 42 }"),
    ("10-100", "def main() { let x = 10 - 100 }"),
    ("10--1", "def main() { let x = 10 - -1 }"),
    ("10--5", "def main() { let x = 10 - -5 }"),
    ("10*0", "def main() { let x = 10 * 0 }"),
    ("10*1", "def main() { let x = 10 * 1 }"),
    ("10*2", "def main() { let x = 10 * 2 }"),
    ("10*3", "def main() { let x = 10 * 3 }"),
    ("10*5", "def main() { let x = 10 * 5 }"),
    ("10*10", "def main() { let x = 10 * 10 }"),
    ("10*42", "def main() { let x = 10 * 42 }"),
    ("10*100", "def main() { let x = 10 * 100 }"),
    ("10*-1", "def main() { let x = 10 * -1 }"),
    ("10*-5", "def main() { let x = 10 * -5 }"),
    ("10%0", "def main() { let x = 10 % 0 }"),
    ("10%1", "def main() { let x = 10 % 1 }"),
    ("10%2", "def main() { let x = 10 % 2 }"),
    ("10%3", "def main() { let x = 10 % 3 }"),
    ("10%5", "def main() { let x = 10 % 5 }"),
    ("10%10", "def main() { let x = 10 % 10 }"),
    ("10%42", "def main() { let x = 10 % 42 }"),
    ("10%100", "def main() { let x = 10 % 100 }"),
    ("10%-1", "def main() { let x = 10 % -1 }"),
    ("10%-5", "def main() { let x = 10 % -5 }"),
    ("42+0", "def main() { let x = 42 + 0 }"),
    ("42+1", "def main() { let x = 42 + 1 }"),
    ("42+2", "def main() { let x = 42 + 2 }"),
    ("42+3", "def main() { let x = 42 + 3 }"),
    ("42+5", "def main() { let x = 42 + 5 }"),
    ("42+10", "def main() { let x = 42 + 10 }"),
    ("42+42", "def main() { let x = 42 + 42 }"),
    ("42+100", "def main() { let x = 42 + 100 }"),
    ("42+-1", "def main() { let x = 42 + -1 }"),
    ("42+-5", "def main() { let x = 42 + -5 }"),
    ("42-0", "def main() { let x = 42 - 0 }"),
    ("42-1", "def main() { let x = 42 - 1 }"),
    ("42-2", "def main() { let x = 42 - 2 }"),
    ("42-3", "def main() { let x = 42 - 3 }"),
    ("42-5", "def main() { let x = 42 - 5 }"),
    ("42-10", "def main() { let x = 42 - 10 }"),
    ("42-42", "def main() { let x = 42 - 42 }"),
    ("42-100", "def main() { let x = 42 - 100 }"),
    ("42--1", "def main() { let x = 42 - -1 }"),
    ("42--5", "def main() { let x = 42 - -5 }"),
    ("42*0", "def main() { let x = 42 * 0 }"),
    ("42*1", "def main() { let x = 42 * 1 }"),
    ("42*2", "def main() { let x = 42 * 2 }"),
    ("42*3", "def main() { let x = 42 * 3 }"),
    ("42*5", "def main() { let x = 42 * 5 }"),
    ("42*10", "def main() { let x = 42 * 10 }"),
    ("42*42", "def main() { let x = 42 * 42 }"),
    ("42*100", "def main() { let x = 42 * 100 }"),
    ("42*-1", "def main() { let x = 42 * -1 }"),
    ("42*-5", "def main() { let x = 42 * -5 }"),
    ("42%0", "def main() { let x = 42 % 0 }"),
    ("42%1", "def main() { let x = 42 % 1 }"),
    ("42%2", "def main() { let x = 42 % 2 }"),
    ("42%3", "def main() { let x = 42 % 3 }"),
    ("42%5", "def main() { let x = 42 % 5 }"),
    ("42%10", "def main() { let x = 42 % 10 }"),
    ("42%42", "def main() { let x = 42 % 42 }"),
    ("42%100", "def main() { let x = 42 % 100 }"),
    ("42%-1", "def main() { let x = 42 % -1 }"),
    ("42%-5", "def main() { let x = 42 % -5 }"),
    ("100+0", "def main() { let x = 100 + 0 }"),
    ("100+1", "def main() { let x = 100 + 1 }"),
    ("100+2", "def main() { let x = 100 + 2 }"),
    ("100+3", "def main() { let x = 100 + 3 }"),
    ("100+5", "def main() { let x = 100 + 5 }"),
    ("100+10", "def main() { let x = 100 + 10 }"),
    ("100+42", "def main() { let x = 100 + 42 }"),
    ("100+100", "def main() { let x = 100 + 100 }"),
    ("100+-1", "def main() { let x = 100 + -1 }"),
    ("100+-5", "def main() { let x = 100 + -5 }"),
    ("100-0", "def main() { let x = 100 - 0 }"),
    ("100-1", "def main() { let x = 100 - 1 }"),
    ("100-2", "def main() { let x = 100 - 2 }"),
    ("100-3", "def main() { let x = 100 - 3 }"),
    ("100-5", "def main() { let x = 100 - 5 }"),
    ("100-10", "def main() { let x = 100 - 10 }"),
    ("100-42", "def main() { let x = 100 - 42 }"),
    ("100-100", "def main() { let x = 100 - 100 }"),
    ("100--1", "def main() { let x = 100 - -1 }"),
    ("100--5", "def main() { let x = 100 - -5 }"),
    ("100*0", "def main() { let x = 100 * 0 }"),
    ("100*1", "def main() { let x = 100 * 1 }"),
    ("100*2", "def main() { let x = 100 * 2 }"),
    ("100*3", "def main() { let x = 100 * 3 }"),
    ("100*5", "def main() { let x = 100 * 5 }"),
    ("100*10", "def main() { let x = 100 * 10 }"),
    ("100*42", "def main() { let x = 100 * 42 }"),
    ("100*100", "def main() { let x = 100 * 100 }"),
    ("100*-1", "def main() { let x = 100 * -1 }"),
    ("100*-5", "def main() { let x = 100 * -5 }"),
    ("100%0", "def main() { let x = 100 % 0 }"),
    ("100%1", "def main() { let x = 100 % 1 }"),
    ("100%2", "def main() { let x = 100 % 2 }"),
    ("100%3", "def main() { let x = 100 % 3 }"),
    ("100%5", "def main() { let x = 100 % 5 }"),
    ("100%10", "def main() { let x = 100 % 10 }"),
    ("100%42", "def main() { let x = 100 % 42 }"),
    ("100%100", "def main() { let x = 100 % 100 }"),
    ("100%-1", "def main() { let x = 100 % -1 }"),
    ("100%-5", "def main() { let x = 100 % -5 }"),
    ("-1+0", "def main() { let x = -1 + 0 }"),
    ("-1+1", "def main() { let x = -1 + 1 }"),
    ("-1+2", "def main() { let x = -1 + 2 }"),
    ("-1+3", "def main() { let x = -1 + 3 }"),
    ("-1+5", "def main() { let x = -1 + 5 }"),
    ("-1+10", "def main() { let x = -1 + 10 }"),
    ("-1+42", "def main() { let x = -1 + 42 }"),
    ("-1+100", "def main() { let x = -1 + 100 }"),
    ("-1+-1", "def main() { let x = -1 + -1 }"),
    ("-1+-5", "def main() { let x = -1 + -5 }"),
    ("-1-0", "def main() { let x = -1 - 0 }"),
    ("-1-1", "def main() { let x = -1 - 1 }"),
    ("-1-2", "def main() { let x = -1 - 2 }"),
    ("-1-3", "def main() { let x = -1 - 3 }"),
    ("-1-5", "def main() { let x = -1 - 5 }"),
    ("-1-10", "def main() { let x = -1 - 10 }"),
    ("-1-42", "def main() { let x = -1 - 42 }"),
    ("-1-100", "def main() { let x = -1 - 100 }"),
    ("-1--1", "def main() { let x = -1 - -1 }"),
    ("-1--5", "def main() { let x = -1 - -5 }"),
    ("-1*0", "def main() { let x = -1 * 0 }"),
    ("-1*1", "def main() { let x = -1 * 1 }"),
    ("-1*2", "def main() { let x = -1 * 2 }"),
    ("-1*3", "def main() { let x = -1 * 3 }"),
    ("-1*5", "def main() { let x = -1 * 5 }"),
    ("-1*10", "def main() { let x = -1 * 10 }"),
    ("-1*42", "def main() { let x = -1 * 42 }"),
    ("-1*100", "def main() { let x = -1 * 100 }"),
    ("-1*-1", "def main() { let x = -1 * -1 }"),
    ("-1*-5", "def main() { let x = -1 * -5 }"),
    ("-1%0", "def main() { let x = -1 % 0 }"),
    ("-1%1", "def main() { let x = -1 % 1 }"),
    ("-1%2", "def main() { let x = -1 % 2 }"),
    ("-1%3", "def main() { let x = -1 % 3 }"),
    ("-1%5", "def main() { let x = -1 % 5 }"),
    ("-1%10", "def main() { let x = -1 % 10 }"),
    ("-1%42", "def main() { let x = -1 % 42 }"),
    ("-1%100", "def main() { let x = -1 % 100 }"),
    ("-1%-1", "def main() { let x = -1 % -1 }"),
    ("-1%-5", "def main() { let x = -1 % -5 }"),
    ("-5+0", "def main() { let x = -5 + 0 }"),
    ("-5+1", "def main() { let x = -5 + 1 }"),
    ("-5+2", "def main() { let x = -5 + 2 }"),
    ("-5+3", "def main() { let x = -5 + 3 }"),
    ("-5+5", "def main() { let x = -5 + 5 }"),
    ("-5+10", "def main() { let x = -5 + 10 }"),
    ("-5+42", "def main() { let x = -5 + 42 }"),
    ("-5+100", "def main() { let x = -5 + 100 }"),
    ("-5+-1", "def main() { let x = -5 + -1 }"),
    ("-5+-5", "def main() { let x = -5 + -5 }"),
    ("-5-0", "def main() { let x = -5 - 0 }"),
    ("-5-1", "def main() { let x = -5 - 1 }"),
    ("-5-2", "def main() { let x = -5 - 2 }"),
    ("-5-3", "def main() { let x = -5 - 3 }"),
    ("-5-5", "def main() { let x = -5 - 5 }"),
    ("-5-10", "def main() { let x = -5 - 10 }"),
    ("-5-42", "def main() { let x = -5 - 42 }"),
    ("-5-100", "def main() { let x = -5 - 100 }"),
    ("-5--1", "def main() { let x = -5 - -1 }"),
    ("-5--5", "def main() { let x = -5 - -5 }"),
    ("-5*0", "def main() { let x = -5 * 0 }"),
    ("-5*1", "def main() { let x = -5 * 1 }"),
    ("-5*2", "def main() { let x = -5 * 2 }"),
    ("-5*3", "def main() { let x = -5 * 3 }"),
    ("-5*5", "def main() { let x = -5 * 5 }"),
    ("-5*10", "def main() { let x = -5 * 10 }"),
    ("-5*42", "def main() { let x = -5 * 42 }"),
    ("-5*100", "def main() { let x = -5 * 100 }"),
    ("-5*-1", "def main() { let x = -5 * -1 }"),
    ("-5*-5", "def main() { let x = -5 * -5 }"),
    ("-5%0", "def main() { let x = -5 % 0 }"),
    ("-5%1", "def main() { let x = -5 % 1 }"),
    ("-5%2", "def main() { let x = -5 % 2 }"),
    ("-5%3", "def main() { let x = -5 % 3 }"),
    ("-5%5", "def main() { let x = -5 % 5 }"),
    ("-5%10", "def main() { let x = -5 % 10 }"),
    ("-5%42", "def main() { let x = -5 % 42 }"),
    ("-5%100", "def main() { let x = -5 % 100 }"),
    ("-5%-1", "def main() { let x = -5 % -1 }"),
    ("-5%-5", "def main() { let x = -5 % -5 }"),
]

@pytest.mark.parametrize("name,src", ARITH_PROGRAMS, ids=[i[0] for i in ARITH_PROGRAMS])
def test_arith_program(name, src):
    _compile(src)

FUNC_PARAM_PROGRAMS = [
    ("params_0", 'def f() -> int { return 0 }\ndef main() { let x = f() }'),
    ("params_1", 'def f(a0: int) -> int { return a0 }\ndef main() { let x = f(0) }'),
    ("params_2", 'def f(a0: int, a1: int) -> int { return a0 + a1 }\ndef main() { let x = f(0, 1) }'),
    ("params_3", 'def f(a0: int, a1: int, a2: int) -> int { return a0 + a1 + a2 }\ndef main() { let x = f(0, 1, 2) }'),
    ("params_5", 'def f(a0: int, a1: int, a2: int, a3: int, a4: int) -> int { return a0 + a1 + a2 + a3 + a4 }\ndef main() { let x = f(0, 1, 2, 3, 4) }'),
]

@pytest.mark.parametrize("name,src", FUNC_PARAM_PROGRAMS, ids=[i[0] for i in FUNC_PARAM_PROGRAMS])
def test_func_params(name, src):
    _compile(src)

CLASS_METHOD_PROGRAMS = [
    ("methods_0", 'class C {\n\n}'),
    ("methods_1", 'class C {\n    public def m0() -> int { return 0 }\n}'),
    ("methods_2", 'class C {\n    public def m0() -> int { return 0 }\n    public def m1() -> int { return 1 }\n}'),
    ("methods_3", 'class C {\n    public def m0() -> int { return 0 }\n    public def m1() -> int { return 1 }\n    public def m2() -> int { return 2 }\n}'),
    ("methods_5", 'class C {\n    public def m0() -> int { return 0 }\n    public def m1() -> int { return 1 }\n    public def m2() -> int { return 2 }\n    public def m3() -> int { return 3 }\n    public def m4() -> int { return 4 }\n}'),
]

@pytest.mark.parametrize("name,src", CLASS_METHOD_PROGRAMS, ids=[i[0] for i in CLASS_METHOD_PROGRAMS])
def test_class_methods(name, src):
    _compile(src)

FOR_RANGE_PROGRAMS = [
    ("range_0_5", 'def main() { let mut sum = 0\nfor i in 0..5 { sum = sum + i } }'),
    ("range_0_10", 'def main() { let mut sum = 0\nfor i in 0..10 { sum = sum + i } }'),
    ("range_1_3", 'def main() { let mut sum = 0\nfor i in 1..3 { sum = sum + i } }'),
    ("range_10_20", 'def main() { let mut sum = 0\nfor i in 10..20 { sum = sum + i } }'),
    ("range_-5_5", 'def main() { let mut sum = 0\nfor i in -5..5 { sum = sum + i } }'),
    ("range_0_100", 'def main() { let mut sum = 0\nfor i in 0..100 { sum = sum + i } }'),
]

@pytest.mark.parametrize("name,src", FOR_RANGE_PROGRAMS, ids=[i[0] for i in FOR_RANGE_PROGRAMS])
def test_for_range(name, src):
    _compile(src)

LIST_PROGRAMS = [
    ("list_0", 'def main() { let x = [1]\nlet y = x.length }'),
    ("list_1", 'def main() { let x = [1, 2]\nlet y = x.length }'),
    ("list_2", 'def main() { let x = [1, 2, 3]\nlet y = x.length }'),
    ("list_3", 'def main() { let x = [0, 0, 0]\nlet y = x.length }'),
    ("list_4", 'def main() { let x = [10, 20, 30, 40]\nlet y = x.length }'),
    ("list_5", 'def main() { let x = []\nlet y = x.length }'),
]

@pytest.mark.parametrize("name,src", LIST_PROGRAMS, ids=[i[0] for i in LIST_PROGRAMS])
def test_list_programs(name, src):
    _compile(src)

NESTED_IF_PROGRAMS = [
    ("depth_1", 'def main() {\nif true {\n  \n  let x = 1\n}\n}'),
    ("depth_2", 'def main() {\nif true {\n  \nif true {\n    \n  let x = 2\n  }}\n}'),
    ("depth_3", 'def main() {\nif true {\n  \nif true {\n    \nif true {\n      \n  let x = 3\n    }}}\n}'),
    ("depth_4", 'def main() {\nif true {\n  \nif true {\n    \nif true {\n      \nif true {\n        \n  let x = 4\n      }}}}\n}'),
    ("depth_5", 'def main() {\nif true {\n  \nif true {\n    \nif true {\n      \nif true {\n        \nif true {\n          \n  let x = 5\n        }}}}}\n}'),
]

@pytest.mark.parametrize("name,src", NESTED_IF_PROGRAMS, ids=[i[0] for i in NESTED_IF_PROGRAMS])
def test_nested_if(name, src):
    _compile(src)

MATCH_PROGRAMS = [
    ("cases_1", 'def main() { let x = 0\nmatch x {\n  case 0 { 0 }\n  case _ { -1 }\n} }'),
    ("cases_2", 'def main() { let x = 1\nmatch x {\n  case 0 { 0 }\n  case 1 { 1 }\n  case _ { -1 }\n} }'),
    ("cases_3", 'def main() { let x = 1\nmatch x {\n  case 0 { 0 }\n  case 1 { 1 }\n  case 2 { 2 }\n  case _ { -1 }\n} }'),
    ("cases_5", 'def main() { let x = 2\nmatch x {\n  case 0 { 0 }\n  case 1 { 1 }\n  case 2 { 2 }\n  case 3 { 3 }\n  case 4 { 4 }\n  case _ { -1 }\n} }'),
    ("cases_8", 'def main() { let x = 4\nmatch x {\n  case 0 { 0 }\n  case 1 { 1 }\n  case 2 { 2 }\n  case 3 { 3 }\n  case 4 { 4 }\n  case 5 { 5 }\n  case 6 { 6 }\n  case 7 { 7 }\n  case _ { -1 }\n} }'),
]

@pytest.mark.parametrize("name,src", MATCH_PROGRAMS, ids=[i[0] for i in MATCH_PROGRAMS])
def test_match_programs(name, src):
    _compile(src)

WHILE_PROGRAMS = [
    ("countdown", 'def main() {{ let mut i = 10\nwhile i > 0 {{ i = i - 1 }} }}'),
    ("accumulate", 'def main() {{ let mut sum = 0\nlet mut i = 1\nwhile i <= 10 {{ sum = sum + i\ni = i + 1 }} }}'),
    ("fibonacci_iter", 'def main() {{ let mut a = 0\nlet mut b = 1\nlet mut i = 0\nwhile i < 10 {{ let temp = a + b\na = b\nb = temp\ni = i + 1 }} }}'),
    ("nested_while", 'def main() {{ let mut i = 0\nwhile i < 5 {{ let mut j = 0\nwhile j < 3 {{ j = j + 1 }}\ni = i + 1 }} }}'),
]

@pytest.mark.parametrize("name,src", WHILE_PROGRAMS, ids=[i[0] for i in WHILE_PROGRAMS])
def test_while_programs(name, src):
    _compile(src)

LAMBDA_PROGRAMS = [
    ("double", 'def main() { let f = (x) => x * 2\nlet x = f(5) }'),
    ("add_one", 'def main() { let f = (x) => x + 1\nlet x = f(5) }'),
    ("negate", 'def main() { let f = (x) => -x\nlet x = f(5) }'),
    ("is_positive", 'def main() { let f = (x) => x > 0\nlet x = f(5) }'),
    ("identity", 'def main() { let f = (x) => x\nlet x = f(5) }'),
    ("const_42", 'def main() { let f = () => 42\nlet x = f(5) }'),
    ("add", 'def main() { let f = (a, b) => a + b\nlet x = f(5) }'),
    ("multiply", 'def main() { let f = (a, b) => a * b\nlet x = f(5) }'),
    ("max_ab", 'def main() { let f = (a, b) => a > b ? a : b\nlet x = f(5) }'),
    ("min_ab", 'def main() { let f = (a, b) => a < b ? a : b\nlet x = f(5) }'),
]

@pytest.mark.parametrize("name,src", LAMBDA_PROGRAMS, ids=[i[0] for i in LAMBDA_PROGRAMS])
def test_lambda_programs(name, src):
    _compile(src)

TRY_PROGRAMS = [
    ("simple_try", 'def main() { try { let x = 1 } catch e { print(e) } }'),
    ("try_finally", 'def main() { try { let x = 1 } finally { print(1) } }'),
    ("try_catch_finally", 'def main() { try { let x = 1 } catch e { print(e) } finally { print(2) } }'),
    ("nested_try", 'def main() { try { try { let x = 1 } catch e { print(e) } } catch e { print(e) } }'),
]

@pytest.mark.parametrize("name,src", TRY_PROGRAMS, ids=[i[0] for i in TRY_PROGRAMS])
def test_try_programs(name, src):
    _compile(src)

HIERARCHY_PROGRAMS = [
    ("single_class", 'class Animal { public name: str }\nclass Dog extends Animal {}'),
    ("three_deep", 'class A {}\nclass B extends A {}\nclass C extends B {}'),
    ("diamond", 'class Root {}\nclass Left extends Root {}\nclass Right extends Root {}'),
]

@pytest.mark.parametrize("name,src", HIERARCHY_PROGRAMS, ids=[i[0] for i in HIERARCHY_PROGRAMS])
def test_class_hierarchy(name, src):
    _compile(src)

ENUM_PROGRAMS = [
    ("simple_enum", 'enum Color { Red, Green, Blue }\ndef main() { let c = Color.Red }'),
    ("valued_enum", 'enum Status { Ok = 0, Error = 1 }\ndef main() { let s = Status.Ok }'),
    ("multi_enum", 'enum Dir { North, South, East, West }\ndef main() { let d = Dir.North }'),
]

@pytest.mark.parametrize("name,src", ENUM_PROGRAMS, ids=[i[0] for i in ENUM_PROGRAMS])
def test_enum_programs(name, src):
    _compile(src)

GUARD_PROGRAMS = [
    ("guard_positive", 'def f(x: int) -> int { guard x > 0 else { return 0 } return x }'),
    ("guard_string", 'def f(s: str) -> int { guard s != "" else { return 0 } return s.length }'),
    ("guard_list", 'def f(l: list) -> int { guard l.length > 0 else { return 0 } return l.length }'),
]

@pytest.mark.parametrize("name,src", GUARD_PROGRAMS, ids=[i[0] for i in GUARD_PROGRAMS])
def test_guard_programs(name, src):
    _compile(src)

MODULE_PROGRAMS = [
    ("empty_module", 'module M {}'),
    ("module_func", 'module M { public def helper() { 42 } }'),
    ("module_multi_func", 'module M { public def a() { 1 }\npublic def b() { 2 } }'),
]

@pytest.mark.parametrize("name,src", MODULE_PROGRAMS, ids=[i[0] for i in MODULE_PROGRAMS])
def test_module_programs(name, src):
    _compile(src)

RECURSIVE_PROGRAMS = [
    ("factorial", 'def fact(n: int) -> int { if n <= 1 { return 1 } return n * fact(n - 1) }'),
    ("fibonacci", 'def fib(n: int) -> int { if n <= 1 { return n } return fib(n - 1) + fib(n - 2) }'),
    ("power", 'def pow(b: int, e: int) -> int { if e <= 0 { return 1 } return b * pow(b, e - 1) }'),
    ("sum_to", 'def sum_to(n: int) -> int { if n <= 0 { return 0 } return n + sum_to(n - 1) }'),
]

@pytest.mark.parametrize("name,src", RECURSIVE_PROGRAMS, ids=[i[0] for i in RECURSIVE_PROGRAMS])
def test_recursive_programs(name, src):
    _compile(src)
