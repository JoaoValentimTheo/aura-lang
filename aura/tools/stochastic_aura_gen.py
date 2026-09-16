import random
import io

class AuraGenerator:
    def __init__(self, seed):
        self.rng = random.Random(seed)
        self.output = io.StringIO()
        self.expected_output = []
        self.indent_level = 0
        self.vars = []
        self.classes = []

    def _indent(self, delta=0):
        return "    " * (self.indent_level + delta)

    def write(self, text, indent=False):
        if indent:
            self.output.write(self._indent())
        self.output.write(text)

    def writeln(self, text="", indent=True):
        if indent:
            self.output.write(self._indent())
        self.output.write(text + "\n")

    def gen_name(self, prefix="v"):
        return f"{prefix}_{self.rng.randint(0, 1000000)}"

    def generate(self):
        # 400k tests need to be fast. We'll generate a mix of:
        # 1. Variable declarations and arithmetic
        # 2. Control flow (if, unless, until, guard)
        # 3. Collection operations (list, dict, set, spread, pipe)
        # 4. OOP (classes, methods, visibility)
        
        # Decide category based on seed to ensure even distribution
        category = self.rng.choices(['syntax', 'collections', 'oop', 'integration'], weights=[25, 25, 25, 25])[0]
        
        if category == 'syntax':
            self.gen_syntax_test()
        elif category == 'collections':
            self.gen_collections_test()
        elif category == 'oop':
            self.gen_oop_test()
        else:
            self.gen_integration_test()
            
        return self.output.getvalue(), "".join(self.expected_output)

    def gen_syntax_test(self):
        self.writeln("// Stochastic Syntax Test (Zen Mode)")
        var_name = self.gen_name()
        val = self.rng.randint(-50, 50)
        # Random visibility/static for global var
        vis = self.rng.choice(['public', 'private', 'protected', 'static', ''])
        decl = f"let mut {var_name} = {val};"
        if vis: decl = f"{vis} {decl}"
        self.writeln(decl)
        
        # Nested structures (limit depth to avoid complexity explosion)
        depth = self.rng.randint(2, 4)
        for d in range(depth):
            # branching: if, while, match, block
            branch = self.rng.choices(['if', 'while', 'match', 'block'], weights=[40, 20, 20, 20])[0]
            if branch == 'if':
                cond_str = self.rng.choice(['>= 0', '< 100', '!= 42'])
                passed = False
                if cond_str == '>= 0': passed = val >= 0
                elif cond_str == '< 100': passed = val < 100
                elif cond_str == '!= 42': passed = val != 42
                
                self.writeln(f"if {var_name} {cond_str} {{")
                self.indent_level += 1
                op = self.rng.choice(['+', '-'])
                operand = self.rng.randint(1, 5)
                self.writeln(f"{var_name} = {var_name} {op} {operand};")
                if passed:
                    if op == '+': val += operand
                    else: val -= operand
                self.indent_level -= 1
                self.writeln("}")
            elif branch == 'while':
                # Small, safe loop (was until)
                # while var < limit { ... }
                # logic: if var < limit, increment until limit.
                limit = val + self.rng.randint(1, 4)
                
                # We need a condition that is initially true (sometimes) and eventually false
                # Let's say: while var_name < limit
                
                self.writeln(f"while {var_name} < {limit} {{")
                self.indent_level += 1
                self.writeln(f"{var_name} = {var_name} + 1;")
                
                # Logic update:
                if val < limit:
                    val = limit
                    
                self.indent_level -= 1
                self.writeln("}")

            elif branch == 'match':
                self.writeln(f"match {var_name} {{")
                self.indent_level += 1
                self.writeln(f"case {val} {{ {var_name} = {var_name} + 1; }}")
                self.writeln(f"case _ {{ {var_name} = {var_name} - 1; }}")
                val += 1
                self.indent_level -= 1
                self.writeln("}")
            else:
                self.writeln("{")
                self.indent_level += 1
                inner_var = self.gen_name("y")
                self.writeln(f"let {inner_var} = {var_name};")
                self.indent_level -= 1
                self.writeln("}")
            
        self.writeln(f"print({var_name});")
        self.expected_output.append(str(val) + "\n")

    def gen_collections_test(self):
        self.writeln("// Stochastic Collections Test")
        size = self.rng.randint(3, 8)
        items = [self.rng.randint(1, 20) for _ in range(size)]
        list_name = self.gen_name("l")
        self.writeln(f"let {list_name} = {items};")
        
        # Complex pipe & comprehension
        self.writeln(f"let result = {list_name}")
        self.writeln("    |> filter((x) => x > 5)", indent=True)
        items = [x for x in items if x > 5]
        self.writeln("    |> map((x) => x * 2);", indent=True)
        items = [x * 2 for x in items]
        
        self.writeln("print(result);")
        self.expected_output.append(str(items) + "\n")
        
        # Dict with comprehension (inclusive range)
        self.writeln("let d = { x: x * x for x in 1..4 };")
        # 1..4 in Aura is range(1, 5) in Python
        self.expected_output.append(str({x: x*x for x in range(1, 5)}) + "\n")
        self.writeln("print(d);")

    def gen_oop_test(self):
        self.writeln("// Stochastic OOP Test")
        base = self.gen_name("B")
        self.writeln(f"class {base} {{")
        self.writeln("    protected let p = 10")
        self.writeln("    public server def get_p() = self.p") # 'server' ignored but valid? No, only standard mods.
        # Use standard mods
        self.writeln("    public def get_p() = self.p")
        self.writeln("    static def factory() { return 100; }")
        self.writeln("}")
        
        mid = self.gen_name("M")
        self.writeln(f"class {mid}({base}) {{")
        self.writeln("    public def get_p2() = self.p * 2")
        self.writeln("}")
        
        leaf = self.gen_name("L")
        self.writeln(f"class {leaf}({mid}) {{")
        self.writeln("    private let secret = 100")
        self.writeln("    public def total() = self.get_p2() + self.secret")
        self.writeln("}")
        
        obj = self.gen_name("o")
        self.writeln(f"let {obj} = {leaf}();")
        self.writeln(f"print({obj}.total());")
        self.writeln(f"print({base}.factory());") 
        self.expected_output.append("120\n")
        self.expected_output.append("100\n")

    def gen_integration_test(self):
        self.writeln("// Stochastic Integration Test")
        # Mixture of logic and OOP
        self.writeln("class Counter {")
        self.writeln("    public mut let count = 0")
        self.writeln("    public def inc(amt) { self.count = self.count + amt; }")
        self.writeln("}")
        
        c = self.gen_name("c")
        self.writeln(f"let {c} = Counter();")
        total = 0
        
        reps = self.rng.randint(3, 10)
        # Aura 1..reps is inclusive
        self.writeln(f"for i in 1..{reps} {{")
        self.indent_level += 1
        self.writeln(f"{c}.inc(i);")
        total += sum(range(1, reps + 1))
        self.indent_level -= 1
        self.writeln("}")
        
        self.writeln(f"print({c}.count);")
        self.expected_output.append(str(total) + "\n")

if __name__ == "__main__":
    # Test one
    gen = AuraGenerator(42)
    code, expected = gen.generate()
    print("--- CODE ---")
    print(code)
    print("--- EXPECTED ---")
    print(expected)
