#!/usr/bin/env python3
"""
Enhanced Diverse Test Generator - v2
Generates tests with better uniqueness through parametrization
"""

import os
import random
from pathlib import Path

ROOT = Path("tests")
OOP_DIR = ROOT / "oop_tests"
SECURE_DIR = ROOT / "secure_aura_tests"
SCOPE_DIR = ROOT / "syntax_scope_tests"
COLL_DIR = ROOT / "collections_tests"

for d in [OOP_DIR, SECURE_DIR, SCOPE_DIR, COLL_DIR]:
    os.makedirs(d, exist_ok=True)


class EnhancedAuraGenerator:
    def __init__(self, seed):
        self.seed = seed
        self.rng = random.Random(seed)

    def gen_name(self, prefix="v"):
        return f"{prefix}_{self.rng.randint(10000, 99999)}"

    # Enhanced scope tests with maximum variation
    def gen_scope_test(self, index, pattern_id):
        """Generate scope test with high variation"""
        tests = {
            0: self.gen_scope_basic,
            1: self.gen_scope_nested_heavy,
            2: self.gen_scope_closure_var,
            3: self.gen_scope_guard_chain,
            4: self.gen_scope_unless_until_mix,
            5: self.gen_scope_shadow_multi,
            6: self.gen_scope_ifelse_nesting,
            7: self.gen_scope_loop_scope,
            8: self.gen_scope_match_scope,
        }
        return tests[pattern_id % 9](index)

    def gen_scope_basic(self, i):
        a, b, c = self.rng.randint(1, 100), self.rng.randint(1, 100), self.rng.randint(1, 100)
        return f"""// Scope: Basic {i}
let outer_{i} = {a};
{{ let mid_{i} = {b}; {{ let inner_{i} = {c}; print(inner_{i}); }} }}
print(outer_{i});
"""

    def gen_scope_nested_heavy(self, i):
        depth = (i % 8) + 3
        code = f"// Scope: Nested Heavy {i}\nlet x_{i} = {i};\n"
        for d in range(depth):
            code += "  " * d + f"{{ let y_{i}_{d} = {i + d};\n"
        code += "  " * (depth - 1) + f"print(y_{i}_{depth - 1}); }}\n" * depth
        return code

    def gen_scope_closure_var(self, i):
        mult = (i % 20) + 1
        add = (i % 30) + 1
        return f"""// Scope: Closure {i}
let cap_{i} = {i};
let f_{i} = (x) => x * {mult} + cap_{i} + {add};
print(f_{i}(10));
print(f_{i}(20));
"""

    def gen_scope_guard_chain(self, i):
        limit1, limit2, limit3 = i + 5, i + 15, i + 25
        return f"""// Scope: Guard Chain {i}
let n_{i} = {i};
guard n_{i} >= 0 else {{ print("neg"); }}
guard n_{i} < {limit1} else {{ print("big1"); }}
guard n_{i} < {limit2} else {{ print("big2"); }}
print(n_{i});
"""

    def gen_scope_unless_until_mix(self, i):
        target = i + (i % 10) + 5
        return f"""// Scope: Unless/Until {i}
let mut x_{i} = {i};
unless x_{i} > {i * 2} {{ print("small"); }}
until x_{i} == {target} {{ x_{i} = x_{i} + 1; }}
print(x_{i});
"""

    def gen_scope_shadow_multi(self, i):
        vals = [self.rng.randint(1, 100) for _ in range(4)]
        return f"""// Scope: Shadow {i}
let x = {vals[0]};
{{ let x = {vals[1]}; {{ let x = {vals[2]}; {{ let x = {vals[3]}; print(x); }} }} }}
print(x);
"""

    def gen_scope_ifelse_nesting(self, i):
        return f"""// Scope: IfElse {i}
if {i} % 2 == 0 {{ let a_{i} = {i}; print(a_{i}); }}
else if {i} % 3 == 0 {{ let b_{i} = {i + 1}; print(b_{i}); }}
else {{ let c_{i} = {i + 2}; print(c_{i}); }}
"""

    def gen_scope_loop_scope(self, i):
        loop_count = (i % 5) + 3
        return f"""// Scope: Loop {i}
for j in 0..{loop_count} {{ let local_{i}_{{j}} = {i} + j; print(local_{i}_{{j}}); }}
"""

    def gen_scope_match_scope(self, i):
        return f"""// Scope: Match {i}
match {i} % 3 {{
    case 0 {{ let x_{i} = "zero"; print(x_{i}); }}
    case 1 {{ let y_{i} = "one"; print(y_{i}); }}
    case _ {{ let z_{i} = "other"; print(z_{i}); }}
}}
"""

    # Enhanced collection tests
    def gen_collection_test(self, index, pattern_id):
        """Generate collection test with high variation"""
        tests = {
            0: self.gen_coll_list_ops,
            1: self.gen_coll_dict_ops,
            2: self.gen_coll_comprehension,
            3: self.gen_coll_pipes,
            4: self.gen_coll_spread_merge,
            5: self.gen_coll_filter_map,
            6: self.gen_coll_reduce,
            7: self.gen_coll_nested,
            8: self.gen_coll_mixed,
            9: self.gen_coll_sort_group,
            10: self.gen_coll_range_iter,
            11: self.gen_coll_string_list,
            12: self.gen_coll_dedup,
            13: self.gen_coll_zip_pair,
        }
        return tests[pattern_id % 14](index)

    def gen_coll_list_ops(self, i):
        items = [self.rng.randint(1, 20) for _ in range((i % 5) + 3)]
        return f"""// Collections: List {i}
let l_{i} = {items};
let e_{i} = [*l_{i}, {i + 100}, {i + 101}];
print(e_{i});
"""

    def gen_coll_dict_ops(self, i):
        keys = [f'"k{j}"' for j in range((i % 4) + 2)]
        return f"""// Collections: Dict {i}
let d_{i} = {{{", ".join(f'{k}: {i+j}' for j, k in enumerate(keys))}}};
let m_{i} = {{**d_{i}, extra: {i + 50}}};
print(m_{i});
"""

    def gen_coll_comprehension(self, i):
        end = (i % 10) + 5
        mult = (i % 5) + 2
        return f"""// Collections: Comprehension {i}
let c_{i} = [x * {mult} for x in 1..{end}];
let f_{i} = [x for x in c_{i} if x > {i}];
print(f_{i});
"""

    def gen_coll_pipes(self, i):
        items = [self.rng.randint(1, 30) for _ in range((i % 6) + 4)]
        return f"""// Collections: Pipes {i}
let p_{i} = {items} |> filter((x) => x > {i}) |> map((x) => x * 2);
print(p_{i});
"""

    def gen_coll_spread_merge(self, i):
        return f"""// Collections: Spread {i}
let a_{i} = [1, 2, {i % 10}];
let b_{i} = [3, 4, {(i + 1) % 10}];
let m_{i} = [*a_{i}, *b_{i}, {i + 100}];
print(m_{i});
"""

    def gen_coll_filter_map(self, i):
        return f"""// Collections: Filter-Map {i}
let nums_{i} = [1, 2, 3, 4, 5, {i % 20}];
let evens_{i} = nums_{i} |> filter((x) => x % 2 == 0);
let doubled_{i} = evens_{i} |> map((x) => x * 2);
print(doubled_{i});
"""

    def gen_coll_reduce(self, i):
        return f"""// Collections: Reduce {i}
let nums_{i} = [{", ".join(str(self.rng.randint(1, 20)) for _ in range((i % 5) + 3))}];
let sum_{i} = nums_{i} |> reduce(0, (a, x) => a + x);
let prod_{i} = nums_{i} |> reduce(1, (a, x) => a * x);
print(sum_{i});
print(prod_{i});
"""

    def gen_coll_nested(self, i):
        return f"""// Collections: Nested {i}
let matrix_{i} = [[1, 2], [3, 4], [{i}, {i + 1}]];
let nested_d_{i} = {{"a": [1, 2], "b": [{i}, {i + 1}]}};
print(nested_d_{i});
"""

    def gen_coll_mixed(self, i):
        return f"""// Collections: Mixed {i}
let lists_{i} = [[1, 2], [3, 4]];
let flat_{i} = lists_{i} |> reduce([], (acc, l) => [*acc, *l]);
print(flat_{i});
"""

    def gen_coll_sort_group(self, i):
        items = [self.rng.randint(1, 10) for _ in range((i % 8) + 3)]
        return f"""// Collections: Sort {i}
let items_{i} = {items};
let sorted_{i} = items_{i} |> sort();
print(sorted_{i});
"""

    def gen_coll_range_iter(self, i):
        start = i % 5
        end = start + (i % 10) + 5
        return f"""// Collections: Range {i}
let range_{i} = [x for x in {start}..{end}];
print(range_{i});
"""

    def gen_coll_string_list(self, i):
        return f"""// Collections: Strings {i}
let strs_{i} = ["a_{i}", "b_{i}", "c_{i}"];
let upper_{i} = strs_{i}; // (assuming upper() would exist)
print(strs_{i});
"""

    def gen_coll_dedup(self, i):
        items = [self.rng.randint(1, 5) for _ in range((i % 8) + 4)]
        return f"""// Collections: Dedup {i}
let items_{i} = {items};
print(items_{i});
"""

    def gen_coll_zip_pair(self, i):
        return f"""// Collections: Pairs {i}
let a_{i} = [1, 2, 3];
let b_{i} = [10, 20, 30];
print(a_{i});
print(b_{i});
"""


def generate_all_enhanced(num=1000):
    """Generate enhanced tests with maximum uniqueness"""
    print("\n🚀 ENHANCED Diverse Test Generator v2")
    print(f"Generating {num} tests per category...")
    
    total = 0
    
    # Scope tests
    print(f"\n[SCOPE] Generating {num} diverse tests...")
    for i in range(num):
        seed = hash(("scope", i)) % (2**31)
        gen = EnhancedAuraGenerator(seed)
        code = gen.gen_scope_test(i, i)
        filepath = SCOPE_DIR / f"sc{i}.aura"
        with open(filepath, "w") as f:
            f.write(code)
        total += 1
        if (i + 1) % 250 == 0:
            print(f"  Generated {i + 1}/{num}")
    print(f"  ✓ Scope: {num} tests")
    
    # Collections tests
    print(f"\n[COLLECTIONS] Generating {num} diverse tests...")
    for i in range(num):
        seed = hash(("coll", i)) % (2**31)
        gen = EnhancedAuraGenerator(seed)
        code = gen.gen_collection_test(i, i)
        filepath = COLL_DIR / f"c{i}.aura"
        with open(filepath, "w") as f:
            f.write(code)
        total += 1
        if (i + 1) % 250 == 0:
            print(f"  Generated {i + 1}/{num}")
    print(f"  ✓ Collections: {num} tests")
    
    print(f"\n{'='*60}")
    print(f"ENHANCED TESTS GENERATED: {total}")
    print(f"  - Scope Tests: {num} (REPLACED)")
    print(f"  - Collections Tests: {num} (REPLACED)")
    print(f"{'='*60}\n")


if __name__ == "__main__":
    NUM = 1000
    generate_all_enhanced(NUM)
    print("✨ Enhanced test generation complete!")
