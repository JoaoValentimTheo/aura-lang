# Aura Training Corpus

Structured material for learning Aura — both for humans and language models.
Modeled after [Kof's training corpus](https://koflang.github.io/docs).

```
training/
├── language/          # Core syntax and semantics
│   ├── 00-overview.md
│   ├── 01-lexical.md
│   ├── 02-types.md
│   ├── 03-expressions.md
│   ├── 04-statements.md
│   ├── 05-functions.md
│   ├── 06-classes.md
│   ├── 07-modules.md
│   ├── 08-patterns.md
│   └── 09-errors.md
├── reference/         # Quick-reference cards
│   ├── operators.md
│   ├── keywords.md
│   ├── builtins.md
│   └── stdlib.md
├── patterns/          # Idiomatic Aura (AUP)
│   ├── option.md
│   ├── builder.md
│   ├── strategy.md
│   ├── pipeline.md
│   ├── error-handling.md
│   ├── memoization.md
│   ├── observer.md
│   ├── resource.md
│   ├── worker-pool.md
│   └── crypto.md
├── anti-patterns/     # What NOT to do
│   ├── mutable-default.md
│   ├── bare-except.md
│   ├── sentinel-values.md
│   ├── god-class.md
│   └── stringly-typed.md
└── examples/          # Runnable code with annotations
    ├── hello.aura
    ├── fibonacci.aura
    ├── classes.aura
    └── error-handling.aura
```

## Design Principles

1. **One spelling per construct.** No synonyms, no aliases.
2. **Evidence-based.** Every rule points to source code or a test.
3. **Machine-readable.** Consistent structure for LLM training.
4. **Human-first.** Written for developers, not robots.

## Usage

```bash
# Read the language overview
cat training/language/00-overview.md

# Study a pattern
cat training/patterns/pipeline.md

# Avoid anti-patterns
cat training/anti-patterns/sentinel-values.md
```
