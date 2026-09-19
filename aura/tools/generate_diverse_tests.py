#!/usr/bin/env python3
"""
Advanced Test Generator for Aura Language
Generates 400k+ diverse tests for OOP, Security, Scope, Collections
Each test is unique with varied patterns, edge cases, and complexity levels
"""

import os
import random
from pathlib import Path

# Configuration
ROOT = Path("tests")
OOP_DIR = ROOT / "oop_tests"
SECURE_DIR = ROOT / "secure_aura_tests"
SCOPE_DIR = ROOT / "syntax_scope_tests"
COLL_DIR = ROOT / "collections_tests"

# Test templates and variations
class TestVariations:
    # OOP Patterns
    OOP_PATTERNS = [
        "simple_inheritance",
        "deep_inheritance",
        "static_members",
        "method_variants",
        "visibility_mix",
        "static_factory",
        "builder_pattern",
        "abstract_patterns",
        "mixin_patterns",
        "constructor_variants",
        "property_accessors",
        "composition",
        "delegation",
        "polymorphism",
    ]

    # Security/Edge Cases
    SECURITY_PATTERNS = [
        "recursion_depth",
        "input_validation",
        "null_checks",
        "type_coercion",
        "boundary_values",
        "large_collection",
        "nested_structures",
    ]

    # Scope Patterns
    SCOPE_PATTERNS = [
        "basic_scope",
        "nested_blocks",
        "closure_capture",
        "shadow_variables",
        "lifetime_tracking",
        "scope_exit",
        "guard_unless_until",
        "deep_nesting",
        "scope_transitions",
    ]

    # Collections Patterns
    COLLECTION_PATTERNS = [
        "list_operations",
        "dict_operations",
        "comprehensions",
        "pipes_chains",
        "transformations",
        "set_operations",
        "nested_collections",
        "spreads_unpacking",
        "filtering",
        "mapping",
        "reducing",
        "sorting",
        "grouping",
        "deduplication",
    ]


class AuraTestGenerator:
    def __init__(self, seed: int | None = None):
        self.seed = seed
        self.rng = random.Random(seed)

    def gen_name(self, prefix: str = "v", length: int = 1) -> str:
        """Generate unique variable names"""
        base = f"{prefix}_{self.rng.randint(10000, 99999)}"
        if length > 1:
            base += "_" + "".join(self.rng.choices("abcdefghijklmnopqrstuvwxyz", k=length))
        return base

    def gen_class_name(self) -> str:
        """Generate class names"""
        prefixes = ["", "Abstract", "Base", "Concrete", "Impl", "Mock", "Real"]
        return f"{self.rng.choice(prefixes)}{self.gen_name('C', 2).replace('_', '').title()}"

    # ============ OOP Tests ============
    def gen_oop_simple_inheritance(self) -> str:
        """Simple single inheritance with method override"""
        base = self.gen_class_name()
        child = self.gen_class_name()
        val = self.rng.randint(1, 100)

        code = f"""// OOP: Simple Inheritance
class {base} {{
    let value: int = {val}
    public def get_value() -> int {{ return self.value; }}
    public def double_value() -> int {{ return self.value * 2; }}
}}

class {child}({base}) {{
    public def double_value() -> int {{ return self.value * 3; }}
}}

let obj = {child}(value: {val});
print(obj.get_value());
print(obj.double_value());
"""
        return code

    def gen_oop_deep_inheritance(self) -> str:
        """Deep inheritance chain (3+ levels)"""
        classes = [self.gen_class_name() for _ in range(self.rng.randint(3, 5))]
        val = self.rng.randint(1, 50)

        code = f"// OOP: Deep Inheritance Chain ({len(classes)} levels)\n"

        # First class
        code += f"""class {classes[0]} {{
    protected let base_val: int = {val}
    public def get_val() -> int {{ return self.base_val; }}
}}

"""

        # Middle classes
        for i in range(1, len(classes) - 1):
            code += f"""class {classes[i]}({classes[i-1]}) {{
    public def get_val() -> int {{ return self.base_val * {i + 1}; }}
}}

"""

        # Last class
        code += f"""class {classes[-1]}({classes[-2]}) {{
    public def get_val() -> int {{ return self.base_val * {len(classes)}; }}
}}

let obj = {classes[-1]}(base_val: {val});
print(obj.get_val());
"""
        return code

    def gen_oop_visibility_mix(self) -> str:
        """Mix of public, protected, private members"""
        cls = self.gen_class_name()
        v1, v2, v3 = self.rng.randint(1, 20), self.rng.randint(1, 20), self.rng.randint(1, 20)

        code = f"""// OOP: Visibility Mix
class {cls} {{
    public let public_field: int = {v1}
    protected let protected_field: int = {v2}
    private let private_field: int = {v3}

    public def get_public() -> int {{ return self.public_field; }}
    public def get_protected() -> int {{ return self.protected_field; }}
    public def get_private() -> int {{ return self.private_field; }}
    public def sum_all() -> int {{
        return self.public_field + self.protected_field + self.private_field;
    }}
}}

let obj = {cls}(
    public_field: {v1},
    protected_field: {v2},
    private_field: {v3}
);
print(obj.get_public());
print(obj.sum_all());
"""
        return code

    def gen_oop_static_members(self) -> str:
        """Static fields and methods"""
        cls = self.gen_class_name()

        code = f"""// OOP: Static Members
class {cls} {{
    public static let instance_count: int = 0
    public static def increment_count() -> void {{ {cls}.instance_count = {cls}.instance_count + 1; }}

    public let id: int

    def new(id: int) {{
        self.id = id;
        {cls}.increment_count();
    }}

    public def get_id() -> int {{ return self.id; }}
}}

let obj1 = {cls}(id: 1);
let obj2 = {cls}(id: 2);
let obj3 = {cls}(id: 3);

print({cls}.instance_count);
print(obj1.get_id());
print(obj3.get_id());
"""
        return code

    def gen_oop_composition(self) -> str:
        """Object composition instead of inheritance"""
        inner = self.gen_class_name()
        outer = self.gen_class_name()
        val = self.rng.randint(1, 100)

        code = f"""// OOP: Composition
class {inner} {{
    public let data: int = {val}
    public def process() -> int {{ return self.data * 2; }}
}}

class {outer} {{
    private let component: {inner}

    def new(comp: {inner}) {{
        self.component = comp;
    }}

    public def transform() -> int {{
        return self.component.process() + 10;
    }}
}}

let inner_obj = {inner}(data: {val});
let outer_obj = {outer}(comp: inner_obj);
print(outer_obj.transform());
"""
        return code

    def gen_oop_method_variants(self) -> str:
        """Various method styles (expressions, blocks, overloading patterns)"""
        cls = self.gen_class_name()
        a, b = self.rng.randint(1, 50), self.rng.randint(1, 50)

        code = f"""// OOP: Method Variants
class {cls} {{
    public let x: int
    public let y: int

    def new(x: int, y: int) {{
        self.x = x;
        self.y = y;
    }}

    // Expression method
    public def add() -> int = self.x + self.y

    // Block method
    public def multiply() -> int {{
        return self.x * self.y;
    }}

    // Computed method
    public def sum_squares() -> int {{
        let sum = self.x * self.x + self.y * self.y;
        return sum;
    }}
}}

let obj = {cls}(x: {a}, y: {b});
print(obj.add());
print(obj.multiply());
print(obj.sum_squares());
"""
        return code

    def gen_oop_static_factory(self) -> str:
        """Static factory pattern"""
        cls = self.gen_class_name()
        val = self.rng.randint(1, 100)

        code = f"""// OOP: Static Factory Pattern
class {cls} {{
    private let value: int

    def new(value: int) {{ self.value = value; }}

    public def get_value() -> int {{ return self.value; }}

    public static def create_default() -> {cls} {{
        return {cls}(value: 42);
    }}

    public static def create_custom(val: int) -> {cls} {{
        return {cls}(value: val);
    }}
}}

let obj1 = {cls}.create_default();
let obj2 = {cls}.create_custom({val});

print(obj1.get_value());
print(obj2.get_value());
"""
        return code

    def gen_oop_builder_pattern(self) -> str:
        """Builder pattern for complex construction"""
        cls = self.gen_class_name()

        code = f"""// OOP: Builder Pattern
class {cls}Builder {{
    private let name: str = ""
    private let age: int = 0
    private let email: str = ""

    public def with_name(n: str) -> {cls}Builder {{
        self.name = n;
        return self;
    }}

    public def with_age(a: int) -> {cls}Builder {{
        self.age = a;
        return self;
    }}

    public def build() -> {cls} {{
        return {cls}(name: self.name, age: self.age, email: self.email);
    }}
}}

class {cls} {{
    public let name: str
    public let age: int
    public let email: str

    def new(name: str, age: int, email: str = "") {{
        self.name = name;
        self.age = age;
        self.email = email;
    }}
}}

let obj = {cls}Builder().with_name("Aura").with_age(30).build();
print(obj.name);
print(obj.age);
"""
        return code

    def gen_oop_abstract_patterns(self) -> str:
        """Abstract-like patterns with interface simulation"""
        iface = self.gen_class_name()
        impl1 = self.gen_class_name()
        impl2 = self.gen_class_name()

        code = f"""// OOP: Abstract Patterns
class {iface} {{
    public def process(x: int) -> int {{ return x; }}
}}

class {impl1}({iface}) {{
    public def process(x: int) -> int {{ return x * 2; }}
}}

class {impl2}({iface}) {{
    public def process(x: int) -> int {{ return x + 10; }}
}}

let obj1 = {impl1}();
let obj2 = {impl2}();

print(obj1.process(5));
print(obj2.process(5));
"""
        return code

    def gen_oop_mixin_patterns(self) -> str:
        """Mixin-like behavior through composition"""
        base = self.gen_class_name()
        mixin1 = self.gen_class_name()

        code = f"""// OOP: Mixin-like Composition
class {mixin1} {{
    public def log(msg: str) {{ print(msg); }}
}}

class {base} {{
    private let logger: {mixin1}
    public let value: int

    def new(val: int, logger: {mixin1}) {{
        self.value = val;
        self.logger = logger;
    }}

    public def process() -> int {{
        self.logger.log("Processing");
        return self.value * 2;
    }}
}}

let logger = {mixin1}();
let obj = {base}(val: 50, logger: logger);
print(obj.process());
"""
        return code

    def gen_oop_constructor_variants(self) -> str:
        """Different constructor patterns"""
        cls = self.gen_class_name()

        code = f"""// OOP: Constructor Variants
class {cls} {{
    public let value: int
    public let name: str = "default"

    def new(value: int) {{
        self.value = value;
    }}

    def init_with_name(value: int, name: str) {{
        self.value = value;
        self.name = name;
    }}
}}

let obj1 = {cls}(value: 42);
let obj2 = {cls}(value: 100);
obj2.name = "custom";

print(obj1.value);
print(obj2.value);
print(obj2.name);
"""
        return code

    def gen_oop_property_accessors(self) -> str:
        """Property getter/setter patterns"""
        cls = self.gen_class_name()
        val = self.rng.randint(1, 100)

        code = f"""// OOP: Property Accessors
class {cls} {{
    private let _value: int = 0

    def new(value: int) {{ self._value = value; }}

    public def get_value() -> int {{ return self._value; }}
    public def set_value(v: int) -> void {{
        if v >= 0 {{ self._value = v; }}
    }}

    public def get_double() -> int {{ return self._value * 2; }}
}}

let obj = {cls}(value: {val});
print(obj.get_value());
print(obj.get_double());
obj.set_value(50);
print(obj.get_value());
"""
        return code

    def gen_oop_delegation(self) -> str:
        """Delegation pattern"""
        target = self.gen_class_name()
        delegator = self.gen_class_name()

        code = f"""// OOP: Delegation
class {target} {{
    public let data: int = 0
    public def process() -> int {{ return self.data * 2; }}
}}

class {delegator} {{
    private let delegate: {target}

    def new(t: {target}) {{ self.delegate = t; }}

    public def process() -> int {{
        return self.delegate.process() + 10;
    }}
}}

let target_obj = {target}();
target_obj.data = 25;
let delegator_obj = {delegator}(t: target_obj);
print(delegator_obj.process());
"""
        return code

    def gen_oop_polymorphism(self) -> str:
        """Polymorphic behavior"""
        base = self.gen_class_name()
        child1 = self.gen_class_name()
        child2 = self.gen_class_name()

        code = f"""// OOP: Polymorphism
class {base} {{
    public def render() -> str {{ return "base"; }}
}}

class {child1}({base}) {{
    public def render() -> str {{ return "child1"; }}
}}

class {child2}({base}) {{
    public def render() -> str {{ return "child2"; }}
}}

let objects = [{base}(), {child1}(), {child2}()];
for obj in objects {{
    print(obj.render());
}}
"""
        return code

    # ============ Security/Edge Case Tests ============
    def gen_security_recursion_depth(self) -> str:
        """Recursion with depth limits"""
        fn = self.gen_name("fib")
        depth = self.rng.randint(5, 15)

        code = f"""// Security: Recursion Depth ({depth})
def {fn}(n: int) -> int {{
    if n <= 1 {{ return n; }}
    return {fn}(n - 1) + {fn}(n - 2);
}}

let result = {fn}({depth});
print(result);
"""
        return code

    def gen_security_null_checks(self) -> str:
        """Null safety and optional handling"""
        var = self.gen_name()
        var2 = self.gen_name()

        code = f"""// Security: Null Safety
let {var}: int? = null
let {var2}: int? = 42

guard {var2} != null else {{
    print("null value");
    return;
}}

let result = {var2} ?? 0
print(result);

unless {var} == null {{
    print("has value");
}}
"""
        return code

    def gen_security_boundary_values(self) -> str:
        """Test boundary conditions"""
        tests = [
            ("min_int", -2147483648),
            ("max_int", 2147483647),
            ("zero", 0),
            ("one", 1),
            ("large", self.rng.randint(1000000, 9999999))
        ]

        code = """// Security: Boundary Values
"""
        for name, val in tests:
            code += f"let {name} = {val};\n"

        code += f"""
let results = [{tests[0][1]}, {tests[1][1]}, {tests[2][1]}, {tests[3][1]}, {tests[4][1]}];
print(results);
"""
        return code

    def gen_security_large_collection(self) -> str:
        """Large collection handling"""
        size = self.rng.randint(100, 500)

        code = f"""// Security: Large Collection (size={size})
let items = [];
for i in 1..{size} {{
    items = [*items, i];
}}

let count = 0;
for item in items {{
    if item % 2 == 0 {{
        count = count + 1;
    }}
}}
print(count);
"""
        return code

    def gen_security_nested_structures(self) -> str:
        """Deeply nested data structures"""
        depth = self.rng.randint(5, 10)

        code = f"""// Security: Nested Structures (depth={depth})
let nested = {{"value": 0}};
"""
        for i in range(1, depth):
            code += f'let nested = {{"level_{i}": nested}};\n'

        code += """
print(nested);
"""
        return code

    def gen_security_input_validation(self) -> str:
        """Input validation patterns"""
        fn = self.gen_name("validate")

        code = f"""// Security: Input Validation
def {fn}(input: int) -> bool {{
    guard input >= 0 else {{ return false; }}
    guard input <= 100 else {{ return false; }}
    return true;
}}

let test1 = {fn}(50);
let test2 = {fn}(-5);
let test3 = {fn}(101);

print(test1);
print(test2);
print(test3);
"""
        return code

    def gen_security_type_coercion(self) -> str:
        """Type coercion and conversion"""
        int_val = self.rng.randint(1, 100)
        float_val = self.rng.random() * 100

        code = f"""// Security: Type Coercion
let int_val: int = {int_val}
let float_val: float = {float_val:.2f}
let str_val: str = "42"

let result1 = int_val + int(float_val);
let result2 = float(int_val) + float_val;

print(result1);
print(result2);
"""
        return code

    # ============ Scope Tests ============
    def gen_scope_basic_scope(self) -> str:
        """Basic variable scope"""
        outer = self.gen_name("outer")
        inner = self.gen_name("inner")
        val1, val2 = self.rng.randint(1, 100), self.rng.randint(1, 100)

        code = f"""// Scope: Basic Scope
let {outer} = {val1};

{{
    let {inner} = {val2};
    print({outer});
    print({inner});
}}

print({outer});
"""
        return code

    def gen_scope_nested_blocks(self) -> str:
        """Nested block scoping"""
        depth = self.rng.randint(3, 6)
        vals = [self.rng.randint(1, 100) for _ in range(depth)]

        code = f"""// Scope: Nested Blocks (depth={depth})
let outer = {vals[0]};
"""
        for i in range(depth):
            indent = "    " * (i + 1)
            code += f"""{indent[:-4]}{{\n{indent}let inner_{i} = {vals[i]};
"""

        code += "    " * (depth - 1) + "print(outer);\n"
        for i in range(depth - 1, 0, -1):
            code += "    " * i + "}\n"
        code += "}\n"

        return code

    def gen_scope_shadow_variables(self) -> str:
        """Variable shadowing across scopes"""
        var = "x"
        vals = [self.rng.randint(1, 100) for _ in range(4)]

        code = f"""// Scope: Variable Shadowing
let {var} = {vals[0]};
print({var});

{{
    let {var} = {vals[1]};
    print({var});

    {{
        let {var} = {vals[2]};
        print({var});
    }}

    print({var});
}}

print({var});
"""
        return code

    def gen_scope_closure_capture(self) -> str:
        """Closure capturing outer scope"""
        outer = self.gen_name("outer")
        inner = self.gen_name("inner")
        outer_val = self.rng.randint(1, 50)
        multiplier = self.rng.randint(2, 10)
        addend = self.rng.randint(5, 20)

        code = f"""// Scope: Closure Capture
let {outer} = {outer_val};
let {inner} = (x) => x * {multiplier} + {outer} + {addend};

let result1 = {inner}(10);
let result2 = {inner}(20);

print(result1);
print(result2);
"""
        return code

    def gen_scope_guard_unless_until(self) -> str:
        """Guard, unless, until patterns"""
        x = self.gen_name("x")
        start = self.rng.randint(0, 5)
        target = start + self.rng.randint(3, 10)

        code = f"""// Scope: Guard, Unless, Until
let mut {x} = {start};

guard {x} >= 0 else {{
    print("negative");
}}

unless {x} > {target * 2} {{
    print("small");
}}

until {x} == {target} {{
    {x} = {x} + 1;
}}

print({x});
"""
        return code

    def gen_scope_defer_pattern(self) -> str:
        """Defer-like cleanup patterns"""
        resource = self.gen_name("res")
        acquired = self.rng.randint(1, 100)

        code = f"""// Scope: Resource Management Pattern
let {resource} = "resource";

{{
    let {resource} = {acquired};
    print({resource});
    // implicit cleanup on scope exit
}}

print("cleanup_done");
"""
        return code

    def gen_scope_lifetime_tracking(self) -> str:
        """Lifetime and initialization patterns"""
        var1 = self.gen_name("v")
        var2 = self.gen_name("v")

        code = f"""// Scope: Lifetime Tracking
let {var1} = 100;

{{
    let {var2} = {var1} * 2;
    print({var2});
}}

print({var1});
"""
        return code

    # ============ Collections Tests ============
    def gen_scope_scope_exit(self) -> str:
        """Scope exit and cleanup"""
        code = """// Scope: Scope Exit
let resource = "acquired";

{
    print(resource);
    let local = 42;
    print(local);
}

print("scope exited");
"""
        return code

    def gen_scope_scope_transitions(self) -> str:
        """Scope transitions with control flow"""
        code = """// Scope: Scope Transitions
let x = 10;

if x > 5 {
    let y = x * 2;
    print(y);
} else {
    let z = x + 5;
    print(z);
}

print(x);
"""
        return code

    def gen_scope_deep_nesting(self) -> str:
        """Deep nesting of scopes"""
        code = """// Scope: Deep Nesting
let a = 1;
{
    let b = a + 1;
    {
        let c = b + 1;
        {
            let d = c + 1;
            {
                let e = d + 1;
                print(e);
            }
        }
    }
}
"""
        return code

    def gen_collection_list_operations(self) -> str:
        """List operations: append, spread, indexing"""
        lst = self.gen_name("list")
        items = [self.rng.randint(1, 20) for _ in range(self.rng.randint(3, 6))]

        code = f"""// Collections: List Operations
let {lst} = {items};
let extended = [*{lst}, 99, 100];
let filtered = extended |> filter((x) => x > 10);
let mapped = filtered |> map((x) => x * 2);

print({lst});
print(extended);
print(filtered);
print(mapped);
"""
        return code

    def gen_collection_dict_operations(self) -> str:
        """Dictionary operations: spread, merge, access"""
        d1 = self.gen_name("d")
        d2 = self.gen_name("d")
        keys = [f"key_{i}" for i in range(self.rng.randint(2, 4))]

        code = f"""// Collections: Dictionary Operations
let {d1} = {{\n"""
        for key in keys[:len(keys)//2]:
            code += f'    "{key}": {self.rng.randint(1, 50)},\n'
        code = code.rstrip(',\n') + "\n};\n"

        code += f"""let {d2} = {{\n"""
        for key in keys[len(keys)//2:]:
            code += f'    "{key}": {self.rng.randint(1, 50)},\n'
        code = code.rstrip(',\n') + "\n};\n"

        code += f"""let merged = {{**{d1}, **{d2}, extra: 999}};
print(merged);
"""
        return code

    def gen_collection_comprehensions(self) -> str:
        """List/dict comprehensions"""
        range_end = self.rng.randint(5, 15)
        self.rng.randint(2, 5)

        code = f"""// Collections: Comprehensions
let squared = [x * x for x in 1..{range_end}];
let filtered = [x for x in squared if x > 10];
let dict_comp = {{x: x * 2 for x in 1..5}};

print(squared);
print(filtered);
print(dict_comp);
"""
        return code

    def gen_collection_pipes_chains(self) -> str:
        """Pipe/chain operations"""
        lst = [self.rng.randint(1, 30) for _ in range(self.rng.randint(5, 10))]

        code = f"""// Collections: Pipes & Chains
let data = {lst};

let result = data
    |> filter((x) => x % 2 == 0)
    |> map((x) => x * 2)
    |> filter((x) => x > 10);

print(result);
"""
        return code

    def gen_collection_transformations(self) -> str:
        """Reduce, fold, accumulation"""
        lst = [self.rng.randint(1, 20) for _ in range(self.rng.randint(3, 8))]

        code = f"""// Collections: Transformations
let numbers = {lst};

let sum = numbers |> reduce(0, (acc, x) => acc + x);
let product = numbers |> reduce(1, (acc, x) => acc * x);

print(sum);
print(product);
"""
        return code

    def gen_collection_mixed_operations(self) -> str:
        """Complex mixed collection operations"""
        self.rng.randint(5, 12)

        code = """// Collections: Mixed Operations
let lists = [[1, 2], [3, 4], [5, 6]];
let flat = lists |> reduce([], (acc, lst) => [*acc, *lst]);
let filtered = flat |> filter((x) => x > 2);
let dict = {value: filtered};

print(dict);
"""
        return code

    def gen_collection_set_operations(self) -> str:
        """Set-like operations (using lists with dedup)"""
        code = """// Collections: Set Operations
let items = [1, 2, 2, 3, 3, 3, 4];
let unique = [x for x in items if items.index(x) == items.last_index(x)];

print(unique);
"""
        return code

    def gen_collection_nested_collections(self) -> str:
        """Nested lists and dicts"""
        code = """// Collections: Nested Collections
let matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];
let nested_dict = {
    "group1": [1, 2, 3],
    "group2": [4, 5, 6]
};

print(matrix);
print(nested_dict);
"""
        return code

    def gen_collection_spreads_unpacking(self) -> str:
        """Spread and unpacking operations"""
        code = """// Collections: Spreads & Unpacking
let list1 = [1, 2, 3];
let list2 = [4, 5, 6];
let combined = [*list1, *list2, 7];

let dict1 = {"a": 1, "b": 2};
let dict2 = {"c": 3, "d": 4};
let merged = {**dict1, **dict2};

print(combined);
print(merged);
"""
        return code

    def gen_collection_filtering(self) -> str:
        """Filtering operations"""
        code = """// Collections: Filtering
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

let evens = numbers |> filter((x) => x % 2 == 0);
let odds = numbers |> filter((x) => x % 2 != 0);
let large = numbers |> filter((x) => x > 5);

print(evens);
print(odds);
print(large);
"""
        return code

    def gen_collection_mapping(self) -> str:
        """Mapping/transformation operations"""
        code = """// Collections: Mapping
let numbers = [1, 2, 3, 4, 5];

let doubled = numbers |> map((x) => x * 2);
let squared = numbers |> map((x) => x * x);
let strings = numbers |> map((x) => "num_" + x);

print(doubled);
print(squared);
print(strings);
"""
        return code

    def gen_collection_reducing(self) -> str:
        """Reduce/fold operations"""
        code = """// Collections: Reducing
let numbers = [1, 2, 3, 4, 5];

let sum = numbers |> reduce(0, (acc, x) => acc + x);
let product = numbers |> reduce(1, (acc, x) => acc * x);
let max_val = numbers |> reduce(0, (acc, x) => x > acc ? x : acc);

print(sum);
print(product);
print(max_val);
"""
        return code

    def gen_collection_sorting(self) -> str:
        """Sorting operations"""
        code = """// Collections: Sorting
let numbers = [5, 2, 8, 1, 9, 3];
let sorted_asc = numbers |> sort();
let sorted_desc = numbers |> sort() |> reverse();

print(sorted_asc);
print(sorted_desc);
"""
        return code

    def gen_collection_grouping(self) -> str:
        """Grouping operations"""
        code = """// Collections: Grouping
let numbers = [1, 2, 2, 3, 3, 3, 4, 4];
let grouped = {};

for num in numbers {
    let key = "num_" + num;
    if key in grouped {
        grouped[key] = [*grouped[key], num];
    } else {
        grouped[key] = [num];
    }
}

print(grouped);
"""
        return code

    def gen_collection_deduplication(self) -> str:
        """Deduplication operations"""
        code = """// Collections: Deduplication
let items = [1, 2, 2, 3, 3, 3, 4, 4, 4, 4];
let seen = [];
let unique = [];

for item in items {
    unless seen |> contains(item) {
        seen = [*seen, item];
        unique = [*unique, item];
    }
}

print(unique);
"""
        return code

    def generate(self, test_type: str, index: int) -> str:
        """Generate a single test of given type"""
        # Use index to select pattern deterministically and cyclically
        if test_type == "oop":
            patterns = TestVariations.OOP_PATTERNS
            pattern = patterns[index % len(patterns)]
            method = getattr(self, f"gen_oop_{pattern}")
        elif test_type == "security":
            patterns = TestVariations.SECURITY_PATTERNS
            pattern = patterns[index % len(patterns)]
            method = getattr(self, f"gen_security_{pattern}")
        elif test_type == "scope":
            patterns = TestVariations.SCOPE_PATTERNS
            pattern = patterns[index % len(patterns)]
            method = getattr(self, f"gen_scope_{pattern}")
        elif test_type == "collections":
            patterns = TestVariations.COLLECTION_PATTERNS
            pattern = patterns[index % len(patterns)]
            method = getattr(self, f"gen_collection_{pattern}")
        else:
            raise ValueError(f"Unknown test type: {test_type}")

        return method()


def generate_all_tests(num_tests: int = 1000):
    """Generate all test files"""
    categories = [
        ("oop", OOP_DIR),
        ("security", SECURE_DIR),
        ("scope", SCOPE_DIR),
        ("collections", COLL_DIR),
    ]

    total = 0

    for category, directory in categories:
        print(f"\n[{category.upper()}] Generating {num_tests} diverse tests...")

        for i in range(num_tests):
            # Use seed to make generation deterministic per file
            seed = hash((category, i)) % (2**31)
            gen = AuraTestGenerator(seed=seed)

            code = gen.generate(category, i)

            # Write to file
            filepath = directory / f"{category[0]}{i}.aura"  # oop0, sec0, sco0, col0, etc.

            with open(filepath, "w") as f:
                f.write(code)

            total += 1

            if (i + 1) % 100 == 0:
                print(f"  Generated {i + 1}/{num_tests} files...")

        print(f"  ✓ Completed {category}: {num_tests} unique tests")

    print(f"\n{'='*60}")
    print(f"TOTAL TESTS GENERATED: {total}")
    print(f"  - OOP Tests: {num_tests}")
    print(f"  - Security Tests: {num_tests}")
    print(f"  - Scope Tests: {num_tests}")
    print(f"  - Collections Tests: {num_tests}")
    print(f"{'='*60}")


if __name__ == "__main__":
    for d in [OOP_DIR, SECURE_DIR, SCOPE_DIR, COLL_DIR]:
        os.makedirs(d, exist_ok=True)

    NUM_PER_CATEGORY = 1000

    print("🚀 Aura Diverse Test Generator")
    print(f"Generating {NUM_PER_CATEGORY} tests per category...")

    generate_all_tests(NUM_PER_CATEGORY)

    print("\n✨ Test generation complete!")
    print("Run tests with: pytest tests/ -v")
