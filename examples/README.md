# Aura Examples

Runnable Aura programs demonstrating the language.

| File | What it shows |
|------|---------------|
| `hello.aura` | Basic output |
| `fibonacci.aura` | Loops and variables |
| `prime_checker.aura` | Functions, conditionals, loops |
| `classes.aura` | Classes, constructors, inheritance, properties |
| `functional.aura` | Lambdas, pipe operator, comprehensions |
| `pattern_matching.aura` | `match` with guards and destructuring |
| `error_handling.aura` | `try`/`catch`/`finally`, `guard`, `throw` |
| `macros.aura` | Built-in decorators (`@debug`, `@timeit`, `@memoize`, `@cache`) |
| `tour.aura` | A tour of the whole language |

Run any example with:

```bash
python3 main.py run examples/tour.aura
```

## Style notes

Aura follows a single, explicit syntax:

- Functions use `def`; `fn` does not exist.
- Inheritance uses parentheses: `class Dog(Animal) { ... }`.
- Generics use square brackets: `class Box[T] { ... }`.
- Logical negation is `not`; `!` is not part of the language.
- `unless`, `until`, `loop` and `guard` are first-class statements.

## Testing

The project is verified by a large automated test suite:

- `tests/test_massive_aura_files.py` - generated language constructs
- `tests/test_massive_generated.py` - generated robustness cases
- `tests/test_massive_stress.py` - 100,000 stress cases
- `tests/test_massive_aura_v2.py` - stochastic fuzzing (200 seeds by default;
  set `AURA_FUZZ_SEEDS` for a longer campaign)
- `tests/test_runtime.py` and `tests/test_regressions.py` - end-to-end checks