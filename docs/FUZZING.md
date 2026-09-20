---
layout: default
title: "Fuzzing"
nav_order: 13
---

# Fuzzing

Reproducible fuzzing and stress campaigns for the parser, transpiler, and
runtime.

---

## Campaign runner

The fuzz corpus is driven by the `AURA_FUZZ_SEEDS` environment variable.
A larger value runs more generated programs.

```bash
# Default CI campaign (fast)
AURA_FUZZ_SEEDS=200 python -m pytest tests/test_fuzz_parser.py tests/test_fuzz_security.py tests/test_phase0_fuzz.py tests/test_stress_raw.py -q

# Large local campaign
AURA_FUZZ_SEEDS=5000 python -m pytest tests/test_fuzz_parser.py tests/test_fuzz_security.py tests/test_phase0_fuzz.py tests/test_stress_raw.py -q
```

| Environment variable | Default | Meaning |
|----------------------|---------|---------|
| `AURA_FUZZ_SEEDS` | 200 | Number of generated programs per fuzz suite |

---

## What is exercised

| Suite | Focus |
|-------|-------|
| `tests/test_fuzz_parser.py` | Tokenizer and parser: string literals (all prefixes, escapes), integer bases, operators, malformed input |
| `tests/test_fuzz_security.py` | Structured fuzzing of valid-ish Aura fragments; parse and transpile under adversarial input |
| `tests/test_phase0_fuzz.py` | Unicode support and regex parity (differential against Python `re`) |
| `tests/test_stress_raw.py` | Deep nesting, large literal matrices, long programs, numeric edge cases |

Each generated program is parsed, rule-checked, type-checked, and transpiled;
the output is verified to be valid Python (`ast.parse`). A parse or transpile
crash fails the test.

---

## Running a campaign

```bash
# Recommended: 5000 seeds across all fuzz suites
AURA_FUZZ_SEEDS=5000 python -m pytest \
  tests/test_fuzz_parser.py \
  tests/test_fuzz_security.py \
  tests/test_phase0_fuzz.py \
  tests/test_stress_raw.py -q
```

When a crash is found, reduce it to a minimal `.aura` reproducer and file an
issue labeled `fuzz` (plus `bug`). Do not commit the crashing input as a test
without a fix; the reporter is responsible for the minimal repro.

---

## Campaign log

| Date | Seeds | Suites | Result |
|------|-------|--------|--------|
| 2026-09-20 | 1000 | parser, security, phase0 | 269 passed, 0 crashes |
| 2026-09-20 | 5000 | parser, security, phase0, stress | 559 passed, 0 crashes |
| 2026-09-20 | — | stress_raw (default) | 290 passed, 0 crashes |

**Result:** no crashes found as of 2026-09-20 (v0.2.0a8). No fuzz issues are
open.

---

## Design notes

- The generator is deterministic per seed (`aura/tools/stochastic_aura_gen.py`),
  so a failing seed reproduces exactly.
- Fuzzing targets the *front end* (lexer, parser, checker, transformer). Code
  execution is exercised by the differential and integration suites instead.
- Regex parity is checked differentially against Python's `re` module, so a
  divergence is a real bug, not an approximation.
