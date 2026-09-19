"""Final combinatorial batch - 7e."""
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

CMP_ALL = [
    ("0==0", "def main() { let z = 0 == 0 }"),
    ("0==1", "def main() { let z = 0 == 1 }"),
    ("0==2", "def main() { let z = 0 == 2 }"),
    ("0==3", "def main() { let z = 0 == 3 }"),
    ("0==5", "def main() { let z = 0 == 5 }"),
    ("0==10", "def main() { let z = 0 == 10 }"),
    ("0!=0", "def main() { let z = 0 != 0 }"),
    ("0!=1", "def main() { let z = 0 != 1 }"),
    ("0!=2", "def main() { let z = 0 != 2 }"),
    ("0!=3", "def main() { let z = 0 != 3 }"),
    ("0!=5", "def main() { let z = 0 != 5 }"),
    ("0!=10", "def main() { let z = 0 != 10 }"),
    ("0<0", "def main() { let z = 0 < 0 }"),
    ("0<1", "def main() { let z = 0 < 1 }"),
    ("0<2", "def main() { let z = 0 < 2 }"),
    ("0<3", "def main() { let z = 0 < 3 }"),
    ("0<5", "def main() { let z = 0 < 5 }"),
    ("0<10", "def main() { let z = 0 < 10 }"),
    ("0>0", "def main() { let z = 0 > 0 }"),
    ("0>1", "def main() { let z = 0 > 1 }"),
    ("0>2", "def main() { let z = 0 > 2 }"),
    ("0>3", "def main() { let z = 0 > 3 }"),
    ("0>5", "def main() { let z = 0 > 5 }"),
    ("0>10", "def main() { let z = 0 > 10 }"),
    ("0<=0", "def main() { let z = 0 <= 0 }"),
    ("0<=1", "def main() { let z = 0 <= 1 }"),
    ("0<=2", "def main() { let z = 0 <= 2 }"),
    ("0<=3", "def main() { let z = 0 <= 3 }"),
    ("0<=5", "def main() { let z = 0 <= 5 }"),
    ("0<=10", "def main() { let z = 0 <= 10 }"),
    ("0>=0", "def main() { let z = 0 >= 0 }"),
    ("0>=1", "def main() { let z = 0 >= 1 }"),
    ("0>=2", "def main() { let z = 0 >= 2 }"),
    ("0>=3", "def main() { let z = 0 >= 3 }"),
    ("0>=5", "def main() { let z = 0 >= 5 }"),
    ("0>=10", "def main() { let z = 0 >= 10 }"),
    ("1==0", "def main() { let z = 1 == 0 }"),
    ("1==1", "def main() { let z = 1 == 1 }"),
    ("1==2", "def main() { let z = 1 == 2 }"),
    ("1==3", "def main() { let z = 1 == 3 }"),
    ("1==5", "def main() { let z = 1 == 5 }"),
    ("1==10", "def main() { let z = 1 == 10 }"),
    ("1!=0", "def main() { let z = 1 != 0 }"),
    ("1!=1", "def main() { let z = 1 != 1 }"),
    ("1!=2", "def main() { let z = 1 != 2 }"),
    ("1!=3", "def main() { let z = 1 != 3 }"),
    ("1!=5", "def main() { let z = 1 != 5 }"),
    ("1!=10", "def main() { let z = 1 != 10 }"),
    ("1<0", "def main() { let z = 1 < 0 }"),
    ("1<1", "def main() { let z = 1 < 1 }"),
    ("1<2", "def main() { let z = 1 < 2 }"),
    ("1<3", "def main() { let z = 1 < 3 }"),
    ("1<5", "def main() { let z = 1 < 5 }"),
    ("1<10", "def main() { let z = 1 < 10 }"),
    ("1>0", "def main() { let z = 1 > 0 }"),
    ("1>1", "def main() { let z = 1 > 1 }"),
    ("1>2", "def main() { let z = 1 > 2 }"),
    ("1>3", "def main() { let z = 1 > 3 }"),
    ("1>5", "def main() { let z = 1 > 5 }"),
    ("1>10", "def main() { let z = 1 > 10 }"),
    ("1<=0", "def main() { let z = 1 <= 0 }"),
    ("1<=1", "def main() { let z = 1 <= 1 }"),
    ("1<=2", "def main() { let z = 1 <= 2 }"),
    ("1<=3", "def main() { let z = 1 <= 3 }"),
    ("1<=5", "def main() { let z = 1 <= 5 }"),
    ("1<=10", "def main() { let z = 1 <= 10 }"),
    ("1>=0", "def main() { let z = 1 >= 0 }"),
    ("1>=1", "def main() { let z = 1 >= 1 }"),
    ("1>=2", "def main() { let z = 1 >= 2 }"),
    ("1>=3", "def main() { let z = 1 >= 3 }"),
    ("1>=5", "def main() { let z = 1 >= 5 }"),
    ("1>=10", "def main() { let z = 1 >= 10 }"),
    ("2==0", "def main() { let z = 2 == 0 }"),
    ("2==1", "def main() { let z = 2 == 1 }"),
    ("2==2", "def main() { let z = 2 == 2 }"),
    ("2==3", "def main() { let z = 2 == 3 }"),
    ("2==5", "def main() { let z = 2 == 5 }"),
    ("2==10", "def main() { let z = 2 == 10 }"),
    ("2!=0", "def main() { let z = 2 != 0 }"),
    ("2!=1", "def main() { let z = 2 != 1 }"),
    ("2!=2", "def main() { let z = 2 != 2 }"),
    ("2!=3", "def main() { let z = 2 != 3 }"),
    ("2!=5", "def main() { let z = 2 != 5 }"),
    ("2!=10", "def main() { let z = 2 != 10 }"),
    ("2<0", "def main() { let z = 2 < 0 }"),
    ("2<1", "def main() { let z = 2 < 1 }"),
    ("2<2", "def main() { let z = 2 < 2 }"),
    ("2<3", "def main() { let z = 2 < 3 }"),
    ("2<5", "def main() { let z = 2 < 5 }"),
    ("2<10", "def main() { let z = 2 < 10 }"),
    ("2>0", "def main() { let z = 2 > 0 }"),
    ("2>1", "def main() { let z = 2 > 1 }"),
    ("2>2", "def main() { let z = 2 > 2 }"),
    ("2>3", "def main() { let z = 2 > 3 }"),
    ("2>5", "def main() { let z = 2 > 5 }"),
    ("2>10", "def main() { let z = 2 > 10 }"),
    ("2<=0", "def main() { let z = 2 <= 0 }"),
    ("2<=1", "def main() { let z = 2 <= 1 }"),
    ("2<=2", "def main() { let z = 2 <= 2 }"),
    ("2<=3", "def main() { let z = 2 <= 3 }"),
    ("2<=5", "def main() { let z = 2 <= 5 }"),
    ("2<=10", "def main() { let z = 2 <= 10 }"),
    ("2>=0", "def main() { let z = 2 >= 0 }"),
    ("2>=1", "def main() { let z = 2 >= 1 }"),
    ("2>=2", "def main() { let z = 2 >= 2 }"),
    ("2>=3", "def main() { let z = 2 >= 3 }"),
    ("2>=5", "def main() { let z = 2 >= 5 }"),
    ("2>=10", "def main() { let z = 2 >= 10 }"),
    ("3==0", "def main() { let z = 3 == 0 }"),
    ("3==1", "def main() { let z = 3 == 1 }"),
    ("3==2", "def main() { let z = 3 == 2 }"),
    ("3==3", "def main() { let z = 3 == 3 }"),
    ("3==5", "def main() { let z = 3 == 5 }"),
    ("3==10", "def main() { let z = 3 == 10 }"),
    ("3!=0", "def main() { let z = 3 != 0 }"),
    ("3!=1", "def main() { let z = 3 != 1 }"),
    ("3!=2", "def main() { let z = 3 != 2 }"),
    ("3!=3", "def main() { let z = 3 != 3 }"),
    ("3!=5", "def main() { let z = 3 != 5 }"),
    ("3!=10", "def main() { let z = 3 != 10 }"),
    ("3<0", "def main() { let z = 3 < 0 }"),
    ("3<1", "def main() { let z = 3 < 1 }"),
    ("3<2", "def main() { let z = 3 < 2 }"),
    ("3<3", "def main() { let z = 3 < 3 }"),
    ("3<5", "def main() { let z = 3 < 5 }"),
    ("3<10", "def main() { let z = 3 < 10 }"),
    ("3>0", "def main() { let z = 3 > 0 }"),
    ("3>1", "def main() { let z = 3 > 1 }"),
    ("3>2", "def main() { let z = 3 > 2 }"),
    ("3>3", "def main() { let z = 3 > 3 }"),
    ("3>5", "def main() { let z = 3 > 5 }"),
    ("3>10", "def main() { let z = 3 > 10 }"),
    ("3<=0", "def main() { let z = 3 <= 0 }"),
    ("3<=1", "def main() { let z = 3 <= 1 }"),
    ("3<=2", "def main() { let z = 3 <= 2 }"),
    ("3<=3", "def main() { let z = 3 <= 3 }"),
    ("3<=5", "def main() { let z = 3 <= 5 }"),
    ("3<=10", "def main() { let z = 3 <= 10 }"),
    ("3>=0", "def main() { let z = 3 >= 0 }"),
    ("3>=1", "def main() { let z = 3 >= 1 }"),
    ("3>=2", "def main() { let z = 3 >= 2 }"),
    ("3>=3", "def main() { let z = 3 >= 3 }"),
    ("3>=5", "def main() { let z = 3 >= 5 }"),
    ("3>=10", "def main() { let z = 3 >= 10 }"),
    ("5==0", "def main() { let z = 5 == 0 }"),
    ("5==1", "def main() { let z = 5 == 1 }"),
    ("5==2", "def main() { let z = 5 == 2 }"),
    ("5==3", "def main() { let z = 5 == 3 }"),
    ("5==5", "def main() { let z = 5 == 5 }"),
    ("5==10", "def main() { let z = 5 == 10 }"),
    ("5!=0", "def main() { let z = 5 != 0 }"),
    ("5!=1", "def main() { let z = 5 != 1 }"),
    ("5!=2", "def main() { let z = 5 != 2 }"),
    ("5!=3", "def main() { let z = 5 != 3 }"),
    ("5!=5", "def main() { let z = 5 != 5 }"),
    ("5!=10", "def main() { let z = 5 != 10 }"),
    ("5<0", "def main() { let z = 5 < 0 }"),
    ("5<1", "def main() { let z = 5 < 1 }"),
    ("5<2", "def main() { let z = 5 < 2 }"),
    ("5<3", "def main() { let z = 5 < 3 }"),
    ("5<5", "def main() { let z = 5 < 5 }"),
    ("5<10", "def main() { let z = 5 < 10 }"),
    ("5>0", "def main() { let z = 5 > 0 }"),
    ("5>1", "def main() { let z = 5 > 1 }"),
    ("5>2", "def main() { let z = 5 > 2 }"),
    ("5>3", "def main() { let z = 5 > 3 }"),
    ("5>5", "def main() { let z = 5 > 5 }"),
    ("5>10", "def main() { let z = 5 > 10 }"),
    ("5<=0", "def main() { let z = 5 <= 0 }"),
    ("5<=1", "def main() { let z = 5 <= 1 }"),
    ("5<=2", "def main() { let z = 5 <= 2 }"),
    ("5<=3", "def main() { let z = 5 <= 3 }"),
    ("5<=5", "def main() { let z = 5 <= 5 }"),
    ("5<=10", "def main() { let z = 5 <= 10 }"),
    ("5>=0", "def main() { let z = 5 >= 0 }"),
    ("5>=1", "def main() { let z = 5 >= 1 }"),
    ("5>=2", "def main() { let z = 5 >= 2 }"),
    ("5>=3", "def main() { let z = 5 >= 3 }"),
    ("5>=5", "def main() { let z = 5 >= 5 }"),
    ("5>=10", "def main() { let z = 5 >= 10 }"),
    ("10==0", "def main() { let z = 10 == 0 }"),
    ("10==1", "def main() { let z = 10 == 1 }"),
    ("10==2", "def main() { let z = 10 == 2 }"),
    ("10==3", "def main() { let z = 10 == 3 }"),
    ("10==5", "def main() { let z = 10 == 5 }"),
    ("10==10", "def main() { let z = 10 == 10 }"),
    ("10!=0", "def main() { let z = 10 != 0 }"),
    ("10!=1", "def main() { let z = 10 != 1 }"),
    ("10!=2", "def main() { let z = 10 != 2 }"),
    ("10!=3", "def main() { let z = 10 != 3 }"),
    ("10!=5", "def main() { let z = 10 != 5 }"),
    ("10!=10", "def main() { let z = 10 != 10 }"),
    ("10<0", "def main() { let z = 10 < 0 }"),
    ("10<1", "def main() { let z = 10 < 1 }"),
    ("10<2", "def main() { let z = 10 < 2 }"),
    ("10<3", "def main() { let z = 10 < 3 }"),
    ("10<5", "def main() { let z = 10 < 5 }"),
    ("10<10", "def main() { let z = 10 < 10 }"),
    ("10>0", "def main() { let z = 10 > 0 }"),
    ("10>1", "def main() { let z = 10 > 1 }"),
    ("10>2", "def main() { let z = 10 > 2 }"),
    ("10>3", "def main() { let z = 10 > 3 }"),
    ("10>5", "def main() { let z = 10 > 5 }"),
    ("10>10", "def main() { let z = 10 > 10 }"),
    ("10<=0", "def main() { let z = 10 <= 0 }"),
    ("10<=1", "def main() { let z = 10 <= 1 }"),
    ("10<=2", "def main() { let z = 10 <= 2 }"),
    ("10<=3", "def main() { let z = 10 <= 3 }"),
    ("10<=5", "def main() { let z = 10 <= 5 }"),
    ("10<=10", "def main() { let z = 10 <= 10 }"),
    ("10>=0", "def main() { let z = 10 >= 0 }"),
    ("10>=1", "def main() { let z = 10 >= 1 }"),
    ("10>=2", "def main() { let z = 10 >= 2 }"),
    ("10>=3", "def main() { let z = 10 >= 3 }"),
    ("10>=5", "def main() { let z = 10 >= 5 }"),
    ("10>=10", "def main() { let z = 10 >= 10 }"),
]

@pytest.mark.parametrize("name,src", CMP_ALL, ids=[i[0] for i in CMP_ALL])
def test_cmp_all(name, src):
    _compile(src)

BIT_ALL = [
    ("0&0", "def main() { let z = 0 & 0 }"),
    ("0&1", "def main() { let z = 0 & 1 }"),
    ("0&3", "def main() { let z = 0 & 3 }"),
    ("0&7", "def main() { let z = 0 & 7 }"),
    ("0&15", "def main() { let z = 0 & 15 }"),
    ("0|0", "def main() { let z = 0 | 0 }"),
    ("0|1", "def main() { let z = 0 | 1 }"),
    ("0|3", "def main() { let z = 0 | 3 }"),
    ("0|7", "def main() { let z = 0 | 7 }"),
    ("0|15", "def main() { let z = 0 | 15 }"),
    ("0^0", "def main() { let z = 0 ^ 0 }"),
    ("0^1", "def main() { let z = 0 ^ 1 }"),
    ("0^3", "def main() { let z = 0 ^ 3 }"),
    ("0^7", "def main() { let z = 0 ^ 7 }"),
    ("0^15", "def main() { let z = 0 ^ 15 }"),
    ("1&0", "def main() { let z = 1 & 0 }"),
    ("1&1", "def main() { let z = 1 & 1 }"),
    ("1&3", "def main() { let z = 1 & 3 }"),
    ("1&7", "def main() { let z = 1 & 7 }"),
    ("1&15", "def main() { let z = 1 & 15 }"),
    ("1|0", "def main() { let z = 1 | 0 }"),
    ("1|1", "def main() { let z = 1 | 1 }"),
    ("1|3", "def main() { let z = 1 | 3 }"),
    ("1|7", "def main() { let z = 1 | 7 }"),
    ("1|15", "def main() { let z = 1 | 15 }"),
    ("1^0", "def main() { let z = 1 ^ 0 }"),
    ("1^1", "def main() { let z = 1 ^ 1 }"),
    ("1^3", "def main() { let z = 1 ^ 3 }"),
    ("1^7", "def main() { let z = 1 ^ 7 }"),
    ("1^15", "def main() { let z = 1 ^ 15 }"),
    ("3&0", "def main() { let z = 3 & 0 }"),
    ("3&1", "def main() { let z = 3 & 1 }"),
    ("3&3", "def main() { let z = 3 & 3 }"),
    ("3&7", "def main() { let z = 3 & 7 }"),
    ("3&15", "def main() { let z = 3 & 15 }"),
    ("3|0", "def main() { let z = 3 | 0 }"),
    ("3|1", "def main() { let z = 3 | 1 }"),
    ("3|3", "def main() { let z = 3 | 3 }"),
    ("3|7", "def main() { let z = 3 | 7 }"),
    ("3|15", "def main() { let z = 3 | 15 }"),
    ("3^0", "def main() { let z = 3 ^ 0 }"),
    ("3^1", "def main() { let z = 3 ^ 1 }"),
    ("3^3", "def main() { let z = 3 ^ 3 }"),
    ("3^7", "def main() { let z = 3 ^ 7 }"),
    ("3^15", "def main() { let z = 3 ^ 15 }"),
    ("7&0", "def main() { let z = 7 & 0 }"),
    ("7&1", "def main() { let z = 7 & 1 }"),
    ("7&3", "def main() { let z = 7 & 3 }"),
    ("7&7", "def main() { let z = 7 & 7 }"),
    ("7&15", "def main() { let z = 7 & 15 }"),
    ("7|0", "def main() { let z = 7 | 0 }"),
    ("7|1", "def main() { let z = 7 | 1 }"),
    ("7|3", "def main() { let z = 7 | 3 }"),
    ("7|7", "def main() { let z = 7 | 7 }"),
    ("7|15", "def main() { let z = 7 | 15 }"),
    ("7^0", "def main() { let z = 7 ^ 0 }"),
    ("7^1", "def main() { let z = 7 ^ 1 }"),
    ("7^3", "def main() { let z = 7 ^ 3 }"),
    ("7^7", "def main() { let z = 7 ^ 7 }"),
    ("7^15", "def main() { let z = 7 ^ 15 }"),
    ("15&0", "def main() { let z = 15 & 0 }"),
    ("15&1", "def main() { let z = 15 & 1 }"),
    ("15&3", "def main() { let z = 15 & 3 }"),
    ("15&7", "def main() { let z = 15 & 7 }"),
    ("15&15", "def main() { let z = 15 & 15 }"),
    ("15|0", "def main() { let z = 15 | 0 }"),
    ("15|1", "def main() { let z = 15 | 1 }"),
    ("15|3", "def main() { let z = 15 | 3 }"),
    ("15|7", "def main() { let z = 15 | 7 }"),
    ("15|15", "def main() { let z = 15 | 15 }"),
    ("15^0", "def main() { let z = 15 ^ 0 }"),
    ("15^1", "def main() { let z = 15 ^ 1 }"),
    ("15^3", "def main() { let z = 15 ^ 3 }"),
    ("15^7", "def main() { let z = 15 ^ 7 }"),
    ("15^15", "def main() { let z = 15 ^ 15 }"),
]

@pytest.mark.parametrize("name,src", BIT_ALL, ids=[i[0] for i in BIT_ALL])
def test_bit_all(name, src):
    _compile(src)

SHIFT_ALL = [
    ("0<<0", "def main() { let z = 0 << 0 }"),
    ("0<<1", "def main() { let z = 0 << 1 }"),
    ("0<<2", "def main() { let z = 0 << 2 }"),
    ("0<<3", "def main() { let z = 0 << 3 }"),
    ("0<<4", "def main() { let z = 0 << 4 }"),
    ("0<<5", "def main() { let z = 0 << 5 }"),
    ("0<<8", "def main() { let z = 0 << 8 }"),
    ("0>>0", "def main() { let z = 0 >> 0 }"),
    ("0>>1", "def main() { let z = 0 >> 1 }"),
    ("0>>2", "def main() { let z = 0 >> 2 }"),
    ("0>>3", "def main() { let z = 0 >> 3 }"),
    ("0>>4", "def main() { let z = 0 >> 4 }"),
    ("0>>5", "def main() { let z = 0 >> 5 }"),
    ("0>>8", "def main() { let z = 0 >> 8 }"),
    ("1<<0", "def main() { let z = 1 << 0 }"),
    ("1<<1", "def main() { let z = 1 << 1 }"),
    ("1<<2", "def main() { let z = 1 << 2 }"),
    ("1<<3", "def main() { let z = 1 << 3 }"),
    ("1<<4", "def main() { let z = 1 << 4 }"),
    ("1<<5", "def main() { let z = 1 << 5 }"),
    ("1<<8", "def main() { let z = 1 << 8 }"),
    ("1>>0", "def main() { let z = 1 >> 0 }"),
    ("1>>1", "def main() { let z = 1 >> 1 }"),
    ("1>>2", "def main() { let z = 1 >> 2 }"),
    ("1>>3", "def main() { let z = 1 >> 3 }"),
    ("1>>4", "def main() { let z = 1 >> 4 }"),
    ("1>>5", "def main() { let z = 1 >> 5 }"),
    ("1>>8", "def main() { let z = 1 >> 8 }"),
    ("2<<0", "def main() { let z = 2 << 0 }"),
    ("2<<1", "def main() { let z = 2 << 1 }"),
    ("2<<2", "def main() { let z = 2 << 2 }"),
    ("2<<3", "def main() { let z = 2 << 3 }"),
    ("2<<4", "def main() { let z = 2 << 4 }"),
    ("2<<5", "def main() { let z = 2 << 5 }"),
    ("2<<8", "def main() { let z = 2 << 8 }"),
    ("2>>0", "def main() { let z = 2 >> 0 }"),
    ("2>>1", "def main() { let z = 2 >> 1 }"),
    ("2>>2", "def main() { let z = 2 >> 2 }"),
    ("2>>3", "def main() { let z = 2 >> 3 }"),
    ("2>>4", "def main() { let z = 2 >> 4 }"),
    ("2>>5", "def main() { let z = 2 >> 5 }"),
    ("2>>8", "def main() { let z = 2 >> 8 }"),
    ("3<<0", "def main() { let z = 3 << 0 }"),
    ("3<<1", "def main() { let z = 3 << 1 }"),
    ("3<<2", "def main() { let z = 3 << 2 }"),
    ("3<<3", "def main() { let z = 3 << 3 }"),
    ("3<<4", "def main() { let z = 3 << 4 }"),
    ("3<<5", "def main() { let z = 3 << 5 }"),
    ("3<<8", "def main() { let z = 3 << 8 }"),
    ("3>>0", "def main() { let z = 3 >> 0 }"),
    ("3>>1", "def main() { let z = 3 >> 1 }"),
    ("3>>2", "def main() { let z = 3 >> 2 }"),
    ("3>>3", "def main() { let z = 3 >> 3 }"),
    ("3>>4", "def main() { let z = 3 >> 4 }"),
    ("3>>5", "def main() { let z = 3 >> 5 }"),
    ("3>>8", "def main() { let z = 3 >> 8 }"),
    ("4<<0", "def main() { let z = 4 << 0 }"),
    ("4<<1", "def main() { let z = 4 << 1 }"),
    ("4<<2", "def main() { let z = 4 << 2 }"),
    ("4<<3", "def main() { let z = 4 << 3 }"),
    ("4<<4", "def main() { let z = 4 << 4 }"),
    ("4<<5", "def main() { let z = 4 << 5 }"),
    ("4<<8", "def main() { let z = 4 << 8 }"),
    ("4>>0", "def main() { let z = 4 >> 0 }"),
    ("4>>1", "def main() { let z = 4 >> 1 }"),
    ("4>>2", "def main() { let z = 4 >> 2 }"),
    ("4>>3", "def main() { let z = 4 >> 3 }"),
    ("4>>4", "def main() { let z = 4 >> 4 }"),
    ("4>>5", "def main() { let z = 4 >> 5 }"),
    ("4>>8", "def main() { let z = 4 >> 8 }"),
    ("5<<0", "def main() { let z = 5 << 0 }"),
    ("5<<1", "def main() { let z = 5 << 1 }"),
    ("5<<2", "def main() { let z = 5 << 2 }"),
    ("5<<3", "def main() { let z = 5 << 3 }"),
    ("5<<4", "def main() { let z = 5 << 4 }"),
    ("5<<5", "def main() { let z = 5 << 5 }"),
    ("5<<8", "def main() { let z = 5 << 8 }"),
    ("5>>0", "def main() { let z = 5 >> 0 }"),
    ("5>>1", "def main() { let z = 5 >> 1 }"),
    ("5>>2", "def main() { let z = 5 >> 2 }"),
    ("5>>3", "def main() { let z = 5 >> 3 }"),
    ("5>>4", "def main() { let z = 5 >> 4 }"),
    ("5>>5", "def main() { let z = 5 >> 5 }"),
    ("5>>8", "def main() { let z = 5 >> 8 }"),
    ("8<<0", "def main() { let z = 8 << 0 }"),
    ("8<<1", "def main() { let z = 8 << 1 }"),
    ("8<<2", "def main() { let z = 8 << 2 }"),
    ("8<<3", "def main() { let z = 8 << 3 }"),
    ("8<<4", "def main() { let z = 8 << 4 }"),
    ("8<<5", "def main() { let z = 8 << 5 }"),
    ("8<<8", "def main() { let z = 8 << 8 }"),
    ("8>>0", "def main() { let z = 8 >> 0 }"),
    ("8>>1", "def main() { let z = 8 >> 1 }"),
    ("8>>2", "def main() { let z = 8 >> 2 }"),
    ("8>>3", "def main() { let z = 8 >> 3 }"),
    ("8>>4", "def main() { let z = 8 >> 4 }"),
    ("8>>5", "def main() { let z = 8 >> 5 }"),
    ("8>>8", "def main() { let z = 8 >> 8 }"),
]

@pytest.mark.parametrize("name,src", SHIFT_ALL, ids=[i[0] for i in SHIFT_ALL])
def test_shift_all(name, src):
    _compile(src)

POWER_ALL = [
    ("0**0", "def main() { let z = 0 ** 0 }"),
    ("0**1", "def main() { let z = 0 ** 1 }"),
    ("0**2", "def main() { let z = 0 ** 2 }"),
    ("0**3", "def main() { let z = 0 ** 3 }"),
    ("0**4", "def main() { let z = 0 ** 4 }"),
    ("0**5", "def main() { let z = 0 ** 5 }"),
    ("1**0", "def main() { let z = 1 ** 0 }"),
    ("1**1", "def main() { let z = 1 ** 1 }"),
    ("1**2", "def main() { let z = 1 ** 2 }"),
    ("1**3", "def main() { let z = 1 ** 3 }"),
    ("1**4", "def main() { let z = 1 ** 4 }"),
    ("1**5", "def main() { let z = 1 ** 5 }"),
    ("2**0", "def main() { let z = 2 ** 0 }"),
    ("2**1", "def main() { let z = 2 ** 1 }"),
    ("2**2", "def main() { let z = 2 ** 2 }"),
    ("2**3", "def main() { let z = 2 ** 3 }"),
    ("2**4", "def main() { let z = 2 ** 4 }"),
    ("2**5", "def main() { let z = 2 ** 5 }"),
    ("3**0", "def main() { let z = 3 ** 0 }"),
    ("3**1", "def main() { let z = 3 ** 1 }"),
    ("3**2", "def main() { let z = 3 ** 2 }"),
    ("3**3", "def main() { let z = 3 ** 3 }"),
    ("3**4", "def main() { let z = 3 ** 4 }"),
    ("3**5", "def main() { let z = 3 ** 5 }"),
    ("5**0", "def main() { let z = 5 ** 0 }"),
    ("5**1", "def main() { let z = 5 ** 1 }"),
    ("5**2", "def main() { let z = 5 ** 2 }"),
    ("5**3", "def main() { let z = 5 ** 3 }"),
    ("5**4", "def main() { let z = 5 ** 4 }"),
    ("5**5", "def main() { let z = 5 ** 5 }"),
    ("10**0", "def main() { let z = 10 ** 0 }"),
    ("10**1", "def main() { let z = 10 ** 1 }"),
    ("10**2", "def main() { let z = 10 ** 2 }"),
    ("10**3", "def main() { let z = 10 ** 3 }"),
    ("10**4", "def main() { let z = 10 ** 4 }"),
    ("10**5", "def main() { let z = 10 ** 5 }"),
]

@pytest.mark.parametrize("name,src", POWER_ALL, ids=[i[0] for i in POWER_ALL])
def test_power_all(name, src):
    _compile(src)

def test_neg_depth_1():
    _compile("def main() { let x = -5 }")

def test_neg_depth_2():
    _compile("def main() { let x = --5 }")

def test_neg_depth_3():
    _compile("def main() { let x = ---5 }")

def test_neg_depth_4():
    _compile("def main() { let x = ----5 }")

def test_neg_depth_5():
    _compile("def main() { let x = -----5 }")

def test_neg_depth_6():
    _compile("def main() { let x = ------5 }")

def test_neg_depth_7():
    _compile("def main() { let x = -------5 }")

def test_neg_depth_8():
    _compile("def main() { let x = --------5 }")

def test_neg_depth_9():
    _compile("def main() { let x = ---------5 }")

def test_neg_depth_10():
    _compile("def main() { let x = ----------5 }")

def test_str_concat_1():
    _compile('def main() { let x = "A" }')

def test_str_concat_2():
    _compile('def main() { let x = "A" + "B" }')

def test_str_concat_3():
    _compile('def main() { let x = "A" + "B" + "C" }')

def test_str_concat_4():
    _compile('def main() { let x = \"A\"+"B"+"C"+"D" }')

def test_str_concat_5():
    _compile('def main() { let x = \"A\"+"B"+"C"+"D"+"E" }')

def test_str_concat_6():
    _compile('def main() { let x = \"A\"+"B"+"C"+"D"+"E"+"F" }')

def test_str_concat_7():
    _compile('def main() { let x = \"A\"+"B"+"C"+"D"+"E"+"F"+"G" }')

def test_str_concat_8():
    _compile('def main() { let x = \"A\"+"B"+"C"+"D"+"E"+"F"+"G"+"H" }')

def test_str_concat_9():
    _compile('def main() { let x = \"A\"+"B"+"C"+"D"+"E"+"F"+"G"+"H"+"I" }')

def test_str_concat_10():
    _compile('def main() { let x = \"A\"+"B"+"C"+"D"+"E"+"F"+"G"+"H"+"I"+"J" }')

def test_list_concat_1():
    _compile("def main() { let x = [1] }")

def test_list_concat_2():
    _compile("def main() { let x = [1]+[1] }")

def test_list_concat_3():
    _compile("def main() { let x = [1]+[1]+[1] }")

def test_list_concat_4():
    _compile("def main() { let x = [1]+[1]+[1]+[1] }")

def test_list_concat_5():
    _compile("def main() { let x = [1]+[1]+[1]+[1]+[1] }")

def test_list_concat_6():
    _compile("def main() { let x = [1]+[1]+[1]+[1]+[1]+[1] }")

def test_list_concat_7():
    _compile("def main() { let x = [1]+[1]+[1]+[1]+[1]+[1]+[1] }")

def test_list_mul_1():
    _compile("def main() { let x = [1, 2] * 1 }")

def test_list_mul_2():
    _compile("def main() { let x = [1, 2] * 2 }")

def test_list_mul_3():
    _compile("def main() { let x = [1, 2] * 3 }")

def test_list_mul_4():
    _compile("def main() { let x = [1, 2] * 4 }")

def test_list_mul_5():
    _compile("def main() { let x = [1, 2] * 5 }")

def test_list_mul_6():
    _compile("def main() { let x = [1, 2] * 6 }")

def test_list_mul_7():
    _compile("def main() { let x = [1, 2] * 7 }")

def test_list_mul_8():
    _compile("def main() { let x = [1, 2] * 8 }")

def test_list_mul_9():
    _compile("def main() { let x = [1, 2] * 9 }")

def test_list_mul_10():
    _compile("def main() { let x = [1, 2] * 10 }")

def test_str_mul_1():
    _compile("def main() { let x = \"ab\" * 1 }")

def test_str_mul_2():
    _compile("def main() { let x = \"ab\" * 2 }")

def test_str_mul_3():
    _compile("def main() { let x = \"ab\" * 3 }")

def test_str_mul_4():
    _compile("def main() { let x = \"ab\" * 4 }")

def test_str_mul_5():
    _compile("def main() { let x = \"ab\" * 5 }")

def test_str_mul_6():
    _compile("def main() { let x = \"ab\" * 6 }")

def test_str_mul_7():
    _compile("def main() { let x = \"ab\" * 7 }")

def test_str_mul_8():
    _compile("def main() { let x = \"ab\" * 8 }")

def test_str_mul_9():
    _compile("def main() { let x = \"ab\" * 9 }")

def test_str_mul_10():
    _compile("def main() { let x = \"ab\" * 10 }")

def test_for_sum():
    _compile("def main() { let mut acc = 0\nfor i in 0..10 { acc = acc + i } }")

def test_for_product():
    _compile("def main() { let mut acc = 1\nfor i in 1..6 { acc = acc * i } }")

def test_for_count_even():
    _compile("def main() { let mut count = 0\nfor i in 0..20 { if i % 2 == 0 { count = count + 1 } } }")

HIERARCHY_CASES = [
    ("two_level", "class A {}\nclass B extends A {}"),
    ("three_level", "class A {}\nclass B extends A {}\nclass C extends B {}"),
    ("sibling", "class Base {}\nclass Left extends Base {}\nclass Right extends Base {}"),
]

@pytest.mark.parametrize("name,src", HIERARCHY_CASES, ids=[i[0] for i in HIERARCHY_CASES])
def test_hierarchies(name, src):
    _compile(src)

MATCH_EXPR_CASES = [
    ("match_literal", "match 3 {\n  case 1 { \"one\" }\n  case 2 { \"two\" }\n  case 3 { \"three\" }\n  case _ { \"other\" }\n}"),
]

@pytest.mark.parametrize("name,src", MATCH_EXPR_CASES, ids=[i[0] for i in MATCH_EXPR_CASES])
def test_match_exprs(name, src):
    _compile(src)

def test_enum_with_method():
    _compile("enum Color { Red, Green, Blue }")

LC_CASES = [
    ("lc_simple", "def main() { let x = [i for i in [1, 2, 3, 4, 5]] }"),
    ("lc_transform", "def main() { let x = [i * 2 for i in [1, 2, 3]] }"),
    ("lc_filter", "def main() { let x = [i for i in [1, 2, 3, 4, 5] if i > 2] }"),
    ("lc_transform_filter", "def main() { let x = [i * 2 for i in [1, 2, 3, 4, 5] if i > 2] }"),
]

@pytest.mark.parametrize("name,src", LC_CASES, ids=[i[0] for i in LC_CASES])
def test_list_comprehensions(name, src):
    _compile(src)

DEFAULT_PARAM_CASES = [
    ("int_default", "def f(x: int = 0) -> int { return x }"),
    ("str_default", "def f(x: str = \"hello\") -> str { return x }"),
    ("bool_default", "def f(x: bool = true) -> bool { return x }"),
    ("multi_default", "def f(a: int = 0, b: str = \"x\", c: bool = false) {}"),
    ("varargs_only", "def f(*args) {}"),
    ("kwargs_only", "def f(**kwargs) {}"),
    ("mixed_varargs", "def f(x: int, *args) {}"),
    ("mixed_kwargs", "def f(x: int, **kwargs) {}"),
    ("all_combined", "def f(x: int = 0, *args, y: str = \"hi\", **kwargs) {}"),
]

@pytest.mark.parametrize("name,src", DEFAULT_PARAM_CASES, ids=[i[0] for i in DEFAULT_PARAM_CASES])
def test_default_params(name, src):
    _compile(src)

ASYNC_CASES = [
    ("async_simple", "async def f() { return 1 }"),
    ("async_with_await", "async def f() { return await g() }"),
    ("async_method", "class C { async def m() { return 1 } }"),
]

@pytest.mark.parametrize("name,src", ASYNC_CASES, ids=[i[0] for i in ASYNC_CASES])
def test_async_funcs(name, src):
    _compile(src)

GEN_CASES = [
    ("yield_simple", "def f() { yield 1\nyield 2\nyield 3 }"),
    ("yield_from", "def f() { yield 1\nyield 2\nyield 3 }"),
]

@pytest.mark.parametrize("name,src", GEN_CASES, ids=[i[0] for i in GEN_CASES])
def test_generators(name, src):
    _compile(src)

TRIPLE_CASES = [
    ("triple_arith", "def main() { let x = ((1 + 2) * 3) - ((4 * 5) + (6 / 7)) }"),
    ("triple_bool", "def main() { let x = (true and false) or (not (true or false)) }"),
    ("triple_cmp", "def main() { let x = (1 < 2) and (3 < 4) and (5 < 6) }"),
    ("triple_call", "def main() { let x = max(min(1, 2), max(3, 4)) }"),
]

@pytest.mark.parametrize("name,src", TRIPLE_CASES, ids=[i[0] for i in TRIPLE_CASES])
def test_triple_nested(name, src):
    _compile(src)