import pytest
import sys
import io
import textwrap
from pathlib import Path

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Tokenizer, Parser
from transpiler.transformer import Transformer

def transpile_and_run(code):
    """Helper to transpile and run Aura code, returning local variables."""
    tokenizer = Tokenizer(code)
    tokens = tokenizer.tokenize()
    parser = Parser(tokens)
    ast = parser.parse()
    transformer = Transformer()
    py_code = transformer.transform(ast)
    
    # Execute
    loc = {}
    exec(py_code, loc)
    return loc

def test_private_visibility_enforcement():
    """Test that private members are not accessible from outside."""
    # 1. Test Valid Access
    valid_code = """
    class Secret {
        private let x = 10
        public def get_x() { return self.x }
    }
    let s = Secret()
    let val = s.get_x()
    """
    loc = transpile_and_run(valid_code)
    assert loc['val'] == 10
    
    # 2. Test Invalid Access (Runtime Error)
    invalid_code = """
    class Secret {
        private let x = 10
    }
    let s = Secret()
    let val = s.x  // Should fail
    """
    
    with pytest.raises(AttributeError):
        transpile_and_run(invalid_code)

def test_private_method_enforcement():
    """Test that private methods are not accessible."""
    invalid_code = """
    class Bank {
        private def transfer() { return "money" }
    }
    let b = Bank()
    b.transfer()
    """
    with pytest.raises(AttributeError):
        transpile_and_run(invalid_code)

def test_static_method_semantics():
    """Test static method decoration."""
    code = """
    class MathUtil {
        static def add(a, b) { return a + b }
    }
    let res = MathUtil.add(10, 5)
    """
    loc = transpile_and_run(code)
    assert loc['res'] == 15

def test_global_modifiers_parsing():
    """Test that global visibility modifiers parse and are compile-time only.

    Name mangling is only meaningful inside a class body, so a global
    ``private let y`` keeps its plain name rather than becoming ``__y``.
    """
    code = """
    public let x = 1
    private let y = 2
    static def foo() { return 3 }
    """
    loc = transpile_and_run(code)
    assert loc['x'] == 1
    # Global 'private' is metadata only: the name is not mangled.
    assert loc['y'] == 2
    
    # Inside a class, private members are still name-mangled.
    class_code = """
    class Account {
        private let balance = 100
        def get() { return self.balance }
    }
    let a = Account()
    let result = a.get()
    """
    loc2 = transpile_and_run(class_code)
    assert loc2['result'] == 100 
