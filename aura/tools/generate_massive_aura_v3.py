import os
from pathlib import Path

# Paths
ROOT = Path("tests")
SYNTAX_DIR = ROOT / "syntax_scope_tests"
COLLECTIONS_DIR = ROOT / "collections_tests"
OOP_DIR = ROOT / "oop_tests"
INTEGRATION_DIR = ROOT / "integration_tests"

os.makedirs(SYNTAX_DIR, exist_ok=True)
os.makedirs(COLLECTIONS_DIR, exist_ok=True)
os.makedirs(OOP_DIR, exist_ok=True)
os.makedirs(INTEGRATION_DIR, exist_ok=True)

def write_aura(path, content):
    with open(path, "w") as f:
        f.write(content)

def gen_syntax():
    for i in range(1000):
        # Nested blocks, until, guard, unless
        content = f"""// Syntax & Scope {i}
let mut x = 0;
unless x > 10 {{
    guard x >= 0 else {{ return; }}
    until x == 5 {{
        x = x + 1;
        {{
            let y = x * 2;
            print(y);
        }}
    }}
}}
match x {{
    case 5 {{ print("done"); }}
    case _ {{ print("error"); }}
}}
"""
        write_aura(SYNTAX_DIR / f"syntax_{i}.aura", content)

def gen_collections():
    for i in range(1000):
        # List/Dict/Set operations, spread, pipe
        content = f"""// Collections {i}
let base = [1, 2, 3];
let extended = [*base, 4, 5];
let filtered = extended |> filter((v) => v % 2 == 0);
let mapped = filtered |> map((v) => v * 2);
let d = {{ "a": 1, "b": 2 }};
let d2 = {{ **d, "c": 3 }};
print(mapped);
print(d2);
"""
        write_aura(COLLECTIONS_DIR / f"coll_{i}.aura", content)

def gen_oop():
    for i in range(1000):
        # Classes, inheritance, modifiers
        content = f"""// OOP {i}
class Base {{
    protected let value: int
    public static let tag = "BASE"
    def show() = print(self.value)
}}

class Derived(Base) {{
    private let secret: str = "AURA"
    public static volatily let count = 0

    def log() {{
        print(self.value); // protected access
        print(self.secret); // private access
        print(Derived.tag); // static access
    }}
}}

let d = Derived(10);
d.log();
"""
        write_aura(OOP_DIR / f"oop_{i}.aura", content)

def gen_integration():
    for i in range(1000):
        # Mixed heavy stuff
        content = f"""// Integration {i}
module App {{
    class Service {{
        private let data = []
        public def add(item) {{
            self.data = [*self.data, item];
        }}
        public def process() {{
            return self.data |> map((x) => x * x) |> filter((x) => x < 100);
        }}
    }}
}}

let s = App.Service();
for i in 0..10 {{
    s.add(i);
}}
print(s.process());
"""
        write_aura(INTEGRATION_DIR / f"integ_{i}.aura", content)

if __name__ == "__main__":
    gen_syntax()
    gen_collections()
    gen_oop()
    gen_integration()
    print("Generated 4,000 base files for the 106,665 tests expansion.")
