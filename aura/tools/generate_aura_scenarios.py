import os
from pathlib import Path

# Paths
SUCCESS_DIR = Path("tests/success_tests_aura")
SYNTAX_DIR = Path("tests/syntax_checker_aura")

os.makedirs(SUCCESS_DIR, exist_ok=True)
os.makedirs(SYNTAX_DIR, exist_ok=True)

def write_aura(path, content):
    with open(path, "w") as f:
        f.write(content)

# Templates and Snippets
types = ["int", "float", "str", "bool", "none", "list[int]", "dict[str, any]", "tuple[int, str]"]
values = ["42", "3.14", "\"Aura\"", "true", "null", "[1, 2, 3]", "{\"a\": 1}", "(10, \"bin\")"]

def gen_success_basics():
    for i in range(50):
        t = types[i % len(types)]
        v = values[i % len(values)]
        content = f"// Basic Type Test {i}\nlet x: {t} = {v};\nlet mut y = {v};\nconst Z = {v};\nprint(x);\nprint(y);\nprint(Z);"
        write_aura(SUCCESS_DIR / f"basics_{i}.aura", content)

def gen_success_control_flow():
    for i in range(100):
        indent = "    "
        content = f"// Control Flow Test {i}\nlet x = {i};\n"
        if i % 3 == 0:
            content += "if x % 2 == 0 { print(\"even\"); } else { print(\"odd\"); }\n"
        elif i % 3 == 1:
            content += "unless x > 100 { print(\"small\"); }\n"
        else:
            content += "guard x >= 0 else { return; }\nprint(\"positive\");\n"
        
        content += "let mut j = 0;\nwhile j < 5 { print(j); j = j + 1; }\n"
        content += "for k in 0..3 { print(k); }\n"
        write_aura(SUCCESS_DIR / f"control_{i}.aura", content)

def gen_success_functions():
    for i in range(100):
        content = f"// Function Test {i}\ndef test_func_{i}(a: int, b: int) -> int {{ return a + b + {i}; }}\n"
        content += f"let lambda_{i} = (x) => x * {i};\n"
        content += f"print(test_func_{i}(1, 2));\n"
        content += f"print(lambda_{i}(10));\n"
        write_aura(SUCCESS_DIR / f"func_{i}.aura", content)

def gen_success_classes():
    for i in range(100):
        content = f"// Class Test {i}\nclass Base_{i} {{ let val: int = {i} }}\n"
        content += f"class Sub_{i} extends Base_{i} {{\n    let extra: str = \"hi\"\n    def show(self) {{ print(self.val); print(self.extra); }}\n}}\n"
        content += f"let obj_{i} = Sub_{i}(val: {i * 2}, extra: \"test_{i}\");\n"
        content += f"obj_{i}.show();\n"
        write_aura(SUCCESS_DIR / f"class_{i}.aura", content)

def gen_success_patterns():
    for i in range(100):
        content = f"// Pattern Match Test {i}\nlet data_{i} = {values[i % len(values)]};\n"
        content += f"match data_{i} {{\n"
        content += f"    case {values[i % len(values)]} {{ print(\"exact\"); }}\n"
        content += f"    case _ {{ print(\"default\"); }}\n"
        content += "}\n"
        write_aura(SUCCESS_DIR / f"pattern_{i}.aura", content)

def gen_success_advanced():
    for i in range(100):
        content = f"// Advanced Features Test {i}\n"
        content += f"let x_{i}: int? = {i}\n"
        content += f"let res_{i} = x_{i} ?? 0\n"
        content += f"let mapped_{i} = [x * 2 for x in 1..5 if x > 2]\n"
        content += f"print(res_{i})\n"
        content += f"print(mapped_{i})\n"
        write_aura(SUCCESS_DIR / f"adv_{i}.aura", content)

def gen_syntax_tests():
    # Valid but tricky syntax
    for i in range(50):
        content = f"// Tricky Syntax {i}\nlet complicated = (1 + 2) * 3 / 4 % 5 |> ((x) => x + 1);\n"
        content += "match [1, 2, 3] { case [a, b, *rest] { print(rest); } }\n"
        write_aura(SYNTAX_DIR / f"tricky_{i}.aura", content)

if __name__ == "__main__":
    gen_success_basics() # 50
    gen_success_control_flow() # 100
    gen_success_functions() # 100
    gen_success_classes() # 100
    gen_success_patterns() # 100
    gen_success_advanced() # 100
    gen_syntax_tests() # 50
    print("Generated over 600 Aura scenarios.")
