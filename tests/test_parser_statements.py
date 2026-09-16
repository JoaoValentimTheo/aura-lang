import unittest
from parser.to_ast import Tokenizer, Parser
from transpiler.ast import *

class TestParserStatements(unittest.TestCase):
    def parse(self, source):
        tokenizer = Tokenizer(source)
        tokens = tokenizer.tokenize()
        parser = Parser(tokens)
        return parser.parse()

    def test_var_decl(self):
        prog = self.parse("let x = 1;")
        stmt = prog.statements[0]
        self.assertIsInstance(stmt, VarDecl)
        self.assertEqual(stmt.name, 'x')
        self.assertFalse(stmt.mutable)
        self.assertIsInstance(stmt.value, IntLiteral)

    def test_var_decl_mut_type(self):
        prog = self.parse("let mut x: Int = 1;")
        stmt = prog.statements[0]
        self.assertIsInstance(stmt, VarDecl)
        self.assertTrue(stmt.mutable)
        self.assertEqual(stmt.type_annotation, 'Int')

    def test_const_decl(self):
        prog = self.parse("const PI = 3.14;")
        stmt = prog.statements[0]
        self.assertIsInstance(stmt, ConstDecl)
        self.assertEqual(stmt.name, 'PI')
        
    def test_function_decl(self):
        source = """
        def add(a: Int, b: Int) -> Int {
            return a + b
        }
        """
        prog = self.parse(source)
        func = prog.statements[0]
        self.assertIsInstance(func, FunctionDecl)
        self.assertEqual(func.name, 'add')
        self.assertEqual(len(func.params), 2)
        self.assertEqual(func.return_type, 'Int')
        
    def test_class_decl(self):
        source = """
        class Point {
            x: Int
            y: Int = 0
            
            def new(x, y) {
                self.x = x
                self.y = y
            }
        }
        """
        prog = self.parse(source)
        cls = prog.statements[0]
        self.assertIsInstance(cls, ClassDecl)
        self.assertEqual(cls.name, 'Point')
        # x and y are fields (VarDecl), new is Method
        self.assertEqual(len(cls.body), 3) 
        
    def test_if_stmt(self):
        source = "if x > 0 { print(x) } else { print(0) }"
        prog = self.parse(source)
        stmt = prog.statements[0]
        self.assertIsInstance(stmt, IfStmt)
        
    def test_while_stmt(self):
        source = "while true { break }"
        prog = self.parse(source)
        stmt = prog.statements[0]
        self.assertIsInstance(stmt, WhileStmt)
        
    def test_for_stmt(self):
        source = "for x in items { print(x) }"
        prog = self.parse(source)
        stmt = prog.statements[0]
        self.assertIsInstance(stmt, ForStmt)
        self.assertEqual(stmt.pattern.name, 'x')

if __name__ == '__main__':
    unittest.main()
