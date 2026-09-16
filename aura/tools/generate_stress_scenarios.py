import os
from pathlib import Path

# Paths
SPEED_DIR = Path("tests/speed_aura_tests")
SECURE_DIR = Path("tests/secure_aura_tests")

os.makedirs(SPEED_DIR, exist_ok=True)
os.makedirs(SECURE_DIR, exist_ok=True)

def write_aura(path, content):
    with open(path, "w") as f:
        f.write(content)

def gen_speed_scenarios():
    for i in range(1000):
        # Template for performance: Arithmetic loops and pattern matching
        content = f"""// Speed Test {i}
let limit = 1000; // This will be multiplied by stress_level in test
let mut sum = 0;
for k in 0..limit {{
    sum = sum + k;
    match k % 3 {{
        case 0 {{ sum = sum + 1; }}
        case 1 {{ sum = sum * 1; }}
        case _ {{ sum = sum - 1; }}
    }}
}}
print(sum);
"""
        write_aura(SPEED_DIR / f"speed_{i}.aura", content)

def gen_secure_scenarios():
    for i in range(1000):
        if i % 2 == 0:
            # Recursion stress
            content = f"""// Security Test {i} - Recursion
def deep_recursion(n: int) -> int {{
    if n <= 0 {{ return 0; }}
    return 1 + deep_recursion(n - 1);
}}
print(deep_recursion(100)); // 100 * stress_level
"""
        else:
            # Indexing/Safety stress
            content = f"""// Security Test {i} - Bounds
let data = [1, 2, 3, 4, 5];
let mut j = 0;
while j < 100 {{ // 100 * stress_level
    let idx = j % 5;
    print(data[idx]);
    j = j + 1;
}}
"""
        write_aura(SECURE_DIR / f"secure_{i}.aura", content)

if __name__ == "__main__":
    gen_speed_scenarios()
    gen_secure_scenarios()
    print("Generated 2,000 Stress Scenarios (1000 Speed, 1000 Secure).")
