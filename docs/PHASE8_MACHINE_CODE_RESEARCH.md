# Phase 8 — Research Report: Machine Code Compilation for Aura

Date: 2026-09-18
Status: Research Complete — Recommendation: **NO-GO** (for now)

---

## Executive Summary

Aura currently transpiles to Python and executes via `exec()`. This report evaluates whether the transpiled Python output can be compiled to native machine code using existing tools. After analyzing six candidates, the recommendation is **NO-GO** for alpha 0.2.0, with Nuitka as the most viable future path.

---

## Candidate Tools

### 1. Nuitka

| Attribute | Details |
|-----------|---------|
| **What it does** | Compiles Python to optimized C/C++, then to machine code via C compiler |
| **Python support** | Full CPython 3.4–3.14 compatibility |
| **License** | AGPL-3.0 (commercial license available) |
| **GitHub stars** | 15,000+ |
| **Performance** | 2–4x over CPython (Pystone benchmark: 3.7x) |
| **Build dependency** | Requires C compiler (GCC/Clang/MSVC) |
| **Standalone mode** | Yes — produces self-contained executables |

**Pros:**
- Full Python compatibility — all Aura-generated Python will work
- No code changes needed — just pipe Aura's output through Nuitka
- Actively maintained (latest: v2.8.6)
- Lowest memory usage among all compilers studied

**Cons:**
- AGPL license for commercial use requires paid license
- Compilation is slow (minutes for large projects)
- Requires C compiler on the build machine
- Not a "compile to machine code" solution — it's "compile Python to C to machine code"

**Fit for Aura:** ★★★★★ — Best fit. Aura generates standard Python; Nuitka can compile it directly.

---

### 2. Cython

| Attribute | Details |
|-----------|---------|
| **What it does** | Translates Python (with optional type annotations) to C extensions |
| **Python support** | Full (superset of Python with `.pyx` syntax) |
| **License** | Apache-2.0 |
| **GitHub stars** | 8,000+ |
| **Performance** | Up to 40x for type-annotated code |
| **Build dependency** | Requires C compiler |

**Pros:**
- Apache-2.0 license (permissive)
- Mature, widely used (40M+ monthly PyPI downloads)
- Excellent performance with type annotations

**Cons:**
- Optimal performance requires Cython-specific syntax (`cpdef`, `cdef`)
- Without type annotations, performance is similar to CPython
- Not a standalone compiler — produces Python extension modules
- Aura's generated Python doesn't have Cython type annotations

**Fit for Aura:** ★★★☆☆ — Would work but requires significant modifications to Aura's code generator to emit Cython annotations.

---

### 3. Codon

| Attribute | Details |
|-----------|---------|
| **What it does** | AOT compiler for Python subsets using LLVM |
| **Python support** | Partial — most common features, not all dynamic features |
| **License** | MIT |
| **GitHub stars** | 5,000+ |
| **Performance** | 10–100x over CPython for numerical code |
| **Build dependency** | LLVM |

**Pros:**
- MIT license (very permissive)
- Dramatic speedups for numerical code
- Supports parallel execution (`@par`)

**Cons:**
- **Does not support full Python** — many dynamic features missing
- No `exec()`, limited metaprogramming, no runtime type introspection
- Aura's `python.eval()` and `python.exec_code()` would fail
- Longer compile times than C++
- Limited platform support

**Fit for Aura:** ★★☆☆☆ — Incompatible. Aura relies on full Python runtime features (exec, eval, dynamic imports) that Codon doesn't support.

---

### 4. mypyc

| Attribute | Details |
|-----------|---------|
| **What it does** | Compiles type-annotated Python to C extensions using mypy |
| **Python support** | Full (optional type annotations) |
| **License** | MIT |
| **GitHub stars** | 2,000+ |
| **Performance** | 1.5–5x over CPython |
| **Build dependency** | C compiler |

**Pros:**
- MIT license
- Uses standard Python type hints
- Good type inference

**Cons:**
- Requires type annotations for best performance
- Produces C extensions, not standalone executables
- Limited runtime type flexibility
- Aura's gradual typing doesn't map cleanly to mypyc's strict model

**Fit for Aura:** ★★★☆☆ — Could work for typed Aura subsets, but not for the full language.

---

### 5. Numba

| Attribute | Details |
|-----------|---------|
| **What it does** | JIT compiler for numerical Python using LLVM |
| **Python support** | Partial — numerical/scientific subset |
| **License** | BSD |
| **Performance** | 90%+ speedup for numerical loops |

**Pros:**
- BSD license
- Excellent for numerical code
- No compilation step needed (JIT)

**Cons:**
- **JIT only** — no AOT compilation
- Only works for numerical/scientific code
- Not a general-purpose compiler
- Requires `@jit` decorators

**Fit for Aura:** ★☆☆☆☆ — Not applicable. Numba is a JIT for numerical Python, not a general-purpose compiler.

---

### 6. PyPy

| Attribute | Details |
|-----------|---------|
| **What it does** | Alternative Python interpreter with JIT |
| **Python support** | Full CPython 3.10+ compatible |
| **License** | MIT |
| **Performance** | 10–100x for long-running programs |

**Pros:**
- Full Python compatibility
- Dramatic speedups for JIT-friendly code
- No code changes needed

**Cons:**
- **Not AOT** — requires PyPy interpreter to run
- Higher memory usage
- C extension compatibility issues
- Not a "compile to machine code" solution

**Fit for Aura:** ★★☆☆☆ — Would work as a runtime alternative, but not a compilation path.

---

## Comparison Matrix

| Tool | Full Python? | AOT? | License | Perf Gain | Aura Fit |
|------|-------------|------|---------|-----------|----------|
| **Nuitka** | Yes | Yes | AGPL/commercial | 2–4x | ★★★★★ |
| Cython | Yes* | Yes | Apache-2.0 | Up to 40x | ★★★☆☆ |
| Codon | No | Yes | MIT | 10–100x | ★★☆☆☆ |
| mypyc | Yes* | Yes | MIT | 1.5–5x | ★★★☆☆ |
| Numba | No | No (JIT) | BSD | 90%+ | ★☆☆☆☆ |
| PyPy | Yes | No (JIT) | MIT | 10–100x | ★★☆☆☆ |

*Cython/mypyc require type annotations for full performance

---

## Recommendation: NO-GO for Alpha 0.2.0

### Rationale

1. **Complexity budget**: Aura is still in alpha. Adding a compilation backend doubles the maintenance surface and introduces a C compiler dependency.

2. **The transpile-to-Python model is working**: The Phase 3 benchmarks show that the current pipeline is fast enough for development. Premature optimization at the compiler level would distract from language stability.

3. **License concerns**: The best option (Nuitka) uses AGPL. Commercial licensing adds friction for users.

4. **Build complexity**: All AOT options require a C compiler on the build machine. This is a significant UX regression for Aura users.

5. **Not blocking alpha 0.2.0**: The roadmap's goal is syntax freeze and documentation. Machine code compilation is a separate, future concern.

### When to Revisit

- After Aura reaches **stable 1.0** with a frozen syntax
- When **performance benchmarks** show the Python runtime is the bottleneck (not the transpiler)
- When Aura has **type annotations** that can be leveraged by mypyc/Cython
- When the **Nuitka AGPL** situation is evaluated against Aura's licensing goals

### Recommended Path (Future)

1. **Short-term (1.0)**: Add `aura compile` command that runs `python -m nuitka` on the transpiled output. This is a 10-line wrapper, not a compiler backend.

2. **Medium-term (2.0)**: Investigate emitting Cython-compatible code with type annotations in the transpiler, then using Cython for compilation.

3. **Long-term (3.0+)**: Consider a native code generation backend (LLVM IR) if the language stabilizes and performance requirements demand it.

---

## References

- Nuitka: https://nuitka.net
- Cython: https://cython.org
- Codon: https://github.com/exaloop/codon
- mypyc: https://github.com/mypyc/mypyc
- "An Empirical Study on Performance and Energy Usage of Compiled Python Code" (EASE 2025)
