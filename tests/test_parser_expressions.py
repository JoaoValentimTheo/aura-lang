import unittest
from parser.to_ast import Tokenizer, Parser
from transpiler.ast import *

class TestParserExpressions(unittest.TestCase):
    def parse_expr(self, source):
        tokenizer = Tokenizer(source)
        tokens = tokenizer.tokenize()
        parser = Parser(tokens)
        return parser.parse_expression()

    def test_binary_ops(self):
        expr = self.parse_expr("1 + 2")
        self.assertIsInstance(expr, BinaryOp)
        self.assertEqual(expr.op, '+')
        
    def test_precedence(self):
        # 1 + 2 * 3 -> 1 + (2 * 3)
        expr = self.parse_expr("1 + 2 * 3")
        self.assertIsInstance(expr, BinaryOp)
        self.assertEqual(expr.op, '+')
        self.assertIsInstance(expr.right, BinaryOp)
        self.assertEqual(expr.right.op, '*')
        
    def test_pipe_operator(self):
        # x |> f |> g -> (x |> f) |> g
        expr = self.parse_expr("x |> f |> g")
        self.assertIsInstance(expr, PipeExpr)
        self.assertIsInstance(expr.left, PipeExpr)
        
    def test_literals(self):
        self.assertIsInstance(self.parse_expr("123"), IntLiteral)
        self.assertIsInstance(self.parse_expr("1.5"), FloatLiteral)
        self.assertIsInstance(self.parse_expr('"s"'), StrLiteral)
        self.assertIsInstance(self.parse_expr("true"), BoolLiteral)
        self.assertIsInstance(self.parse_expr("none"), NoneLiteral)
        
    def test_collections(self):
        list_expr = self.parse_expr("[1, 2]")
        self.assertIsInstance(list_expr, ListLiteral)
        self.assertEqual(len(list_expr.elements), 2)
        
        dict_expr = self.parse_expr("{a: 1, b: 2}")
        self.assertIsInstance(dict_expr, DictLiteral)
        
    def test_comprehension(self):
        comp = self.parse_expr("[x for x in list]")
        self.assertIsInstance(comp, ComprehensionExpr)
        
    def test_lambda(self):
        lam = self.parse_expr("(x) => x + 1")
        self.assertIsInstance(lam, LambdaExpr)
        self.assertEqual(len(lam.params), 1)

if __name__ == '__main__':
    unittest.main()
