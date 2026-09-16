import unittest
from parser.to_ast import Tokenizer, Token

class TestTokenizer(unittest.TestCase):
    def check_tokens(self, source, expected_types):
        tokenizer = Tokenizer(source)
        tokens = tokenizer.tokenize()
        # Remove EOF check for simplicity in some tests unless needed
        self.assertEqual(len(tokens) - 1, len(expected_types)) 
        for i, (tok, exp) in enumerate(zip(tokens, expected_types)):
            self.assertEqual(tok.type, exp, f"Token {i} mismatch: {tok} != {exp}")

    def test_keywords_and_identifiers(self):
        source = "let const if else while for fn class match"
        tokenizer = Tokenizer(source)
        tokens = tokenizer.tokenize()
        keywords = ['IDENT'] * 9 
        self.check_tokens(source, keywords)
        
        # Verify values
        for t in tokens[:-1]:
            self.assertEqual(t.type, 'IDENT')
            
    def test_literals(self):
        self.check_tokens("123", ['INT'])
        self.check_tokens("3.14", ['FLOAT'])
        self.check_tokens('"string"', ['STRING'])
        self.check_tokens("'string'", ['STRING'])
        self.check_tokens("true false null", ['IDENT', 'IDENT', 'IDENT'])
        
    def test_operators(self):
        ops = "+ - * / % == != < > <= >= && || |> .. ?? ?:"
        tokenizer = Tokenizer(ops)
        tokens = tokenizer.tokenize()
        expected = ['OP'] * 17
        self.assertEqual(len(tokens) - 1, 17)
        
    def test_f_strings(self):
        # f-strings carry their quote style plus the raw inner text.
        source = 'f"Hello {name}"'
        tokenizer = Tokenizer(source)
        tokens = tokenizer.tokenize()
        self.assertEqual(tokens[0].type, 'FSTRING')
        quote, raw = tokens[0].value
        self.assertEqual(quote, '"')
        self.assertEqual(raw, 'Hello {name}')
        
    def test_float_vs_range(self):
        # 1..10 -> INT OP(..) INT
        # 1.5 -> FLOAT
        t1 = Tokenizer("1..10").tokenize()
        self.assertEqual(t1[0].type, 'INT')
        self.assertEqual(t1[1].type, 'OP')
        self.assertEqual(t1[1].value, '..')
        self.assertEqual(t1[2].type, 'INT')
        
        t2 = Tokenizer("1.5").tokenize()
        self.assertEqual(t2[0].type, 'FLOAT')
        
    def test_comments(self):
        source = """
        let x = 1 // Comment
        let y = 2
        """
        tokenizer = Tokenizer(source)
        tokens = tokenizer.tokenize()
        # let, x, =, 1, let, y, =, 2, EOF
        self.assertEqual(len(tokens), 9) 

if __name__ == '__main__':
    unittest.main()
