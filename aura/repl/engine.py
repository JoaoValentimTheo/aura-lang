import sys
import traceback
from aura.parser.to_ast import Tokenizer, Parser
from aura.transpiler.transformer import Transformer

class AuraREPL:
    def __init__(self):
        self.locals = {}
        self.transformer = Transformer()
        self.buffer = ""
        self.prompt = "aura> "
        self.cont_prompt = "....> "
        
    def run(self):
        print("Aura Programming Language REPL v1.0")
        print("Type 'exit' to quit.")
        
        while True:
            try:
                line = input(self.prompt if not self.buffer else self.cont_prompt)
                
                if line.strip() == "exit":
                    break
                
                if not line.strip() and not self.buffer:
                    continue
                
                self.buffer += line + "\n"
                
                # Heuristic: if line ends with { or ( or [ or is not complete statement, continue
                # Simpler: Try parse, if unexpected EOF, continue.
                
                try:
                    self.process_buffer()
                    self.buffer = ""
                except SyntaxError as e:
                    # Very basic multi-line support: if error is unexpected eof, wait for more
                    # For now, just print error to allow user to retry
                    print(f"Syntax Error: {e}")
                    self.buffer = "" # Reset on error
                except Exception as e:
                    print(f"Runtime Error: {e}")
                    # traceback.print_exc()
                    self.buffer = ""

            except KeyboardInterrupt:
                print("\nKeyboardInterrupt")
                self.buffer = ""
            except EOFError:
                break

    def process_buffer(self):
        # 1. Tokenize
        tokenizer = Tokenizer(self.buffer)
        tokens = tokenizer.tokenize()
        
        # 2. Parse (assume single statement or block)
        parser = Parser(tokens)
        ast_node = parser.parse() # Helper that parses list of statements
        
        if not ast_node: return
        
        # 3. Transpile
        # We need to handle expressions vs statements for REPL printing
        # The parser returns a Block of statements usually
        
        py_code = self.transformer.transform(ast_node)
        
        # 4. Execute
        # We want to print the result of the last expression if it's an expression
        # But our transformer wraps everything in valid python.
        # Check if last stmt is expression
        
        exec(py_code, {}, self.locals)
