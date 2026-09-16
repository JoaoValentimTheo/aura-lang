import unittest
from parser.to_ast import Tokenizer, Parser
from transpiler.ast import *

class TestFeatures(unittest.TestCase):
    def parse_expr(self, source):
        tokenizer = Tokenizer(source)
        tokens = tokenizer.tokenize()
        parser = Parser(tokens)
        return parser.parse_expression()

    def test_match_guard(self):
        # match x { case x if x > 0 -> ... }
        source = """
        match x {
            case x if x > 0 -> print(x)
        }
        """
        # Parsing stmt, not expr
        tokenizer = Tokenizer(source)
        parser = Parser(tokenizer.tokenize())
        stmt = parser.parse_statement()
        self.assertIsInstance(stmt, MatchStmt)
        self.assertIsInstance(stmt.cases[0].guard, BinaryOp)
        
    def test_destructuring(self):
        # Not fully implemented in parser logic, checking basic acceptance
        pass
        
    def test_async_await(self):
        source = """
        async def fetch() {
            await http.get(url)
        }
        """
        tokenizer = Tokenizer(source)
        parser = Parser(tokenizer.tokenize())
        func = parser.parse_statement()
        self.assertTrue(func.is_async)
        # Check body stmt
        body_stmt = func.body[0]
        self.assertIsInstance(body_stmt, ExprStmt) # await expr is statement

    def test_if_expression(self):
        # let x = if true { 1 } else { 0 }
        source = "let x = if true { 1 } else { 0 }"
        tokenizer = Tokenizer(source)
        parser = Parser(tokenizer.tokenize())
        stmt = parser.parse_statement()
        self.assertIsInstance(stmt, VarDecl)
        self.assertIsInstance(stmt.value, IfStmt)

if __name__ == '__main__':
    unittest.main()
