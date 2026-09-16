import os
import pytest
import sys
import io
import textwrap
from pathlib import Path
from contextlib import redirect_stdout

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from parser.to_ast import Tokenizer, Parser
from transpiler.transformer import Transformer
from tools.stochastic_aura_gen import AuraGenerator

# The generator is deterministic per seed. Each case takes a few seconds, so
# the default run covers a representative sample. Set AURA_FUZZ_SEEDS to run a
# larger campaign (e.g. AURA_FUZZ_SEEDS=100000).
FUZZ_SEEDS = int(os.environ.get("AURA_FUZZ_SEEDS", "200"))


@pytest.mark.parametrize("seed", range(FUZZ_SEEDS))
def test_stochastic_aura(seed):
    # 1. Generate unique Aura code and expected output
    gen = AuraGenerator(seed)
    aura_code, expected_output = gen.generate()
    
    try:
        # 2. Transpile to Python
        tokens = Tokenizer(aura_code).tokenize()
        ast = Parser(tokens).parse()
        py_code = Transformer().transform(ast)
        
        # 3. Prepare execution environment
        wrapped = f"def wrapper():\n{textwrap.indent(py_code, '    ')}\nwrapper()"
        
        env = {
            "filter": lambda c, f: list(filter(f, c)),
            "map": lambda c, f: list(map(f, c)),
            "print": print, # Use real print to capture
            "range": range
        }
        
        # 4. Execute and capture output
        f = io.StringIO()
        with redirect_stdout(f):
            exec(wrapped, env)
        actual_output = f.getvalue()
        
        # 5. Verify
        assert actual_output == expected_output
        
    except Exception as e:
        # Include Aura code in error message for easier debugging
        print(f"\n--- FAILED SEED: {seed} ---")
        print("--- AURA CODE ---")
        print(aura_code)
        print("--- ERROR ---")
        raise e
