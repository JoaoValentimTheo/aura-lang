# Aura

A programming language that transpiles to Python.

## Installation

Install the toolchain from a wheel or sdist; this provides the `aura` command:

```bash
pip install aura-language            # once published on PyPI
# or from a checkout:
pip install .
aura run examples/hello.aura
```

Installed commands (`aura <cmd>`):

| Command | Description |
|---------|-------------|
| `aura run <file>` | Transpile and execute |
| `aura transpile <file> [-o out.py]` | Convert Aura to Python |
| `aura check <file>` | Type and mutability checks |
| `aura format <file>` | Format source |
| `aura lint <file>` | Style warnings |
| `aura test <dir>` | Run `.aura` files |
| `aura repl` | Interactive REPL |
| `aura init [name]` | Create `aura.toml` and `src/main.aura` |
| `aura add <pkg>` | Add and install a dependency |
| `aura install` | Install dependencies from `aura.toml` |
| `aura deps` | List declared dependencies |
| `aura version [bump]` | Show or bump the version |
| `aura debug <file> [-t]` | Run under the trace debugger |
| `aura lsp` | Start the language server (stdio) |

Aura programs can import any Python standard-library module or installed PyPI
package, plus local `.aura` modules and packages. The standard library also
ships `stdlib.regex`, `stdlib.os` and `stdlib.http`.

## Quick Start (from source, no install)

```bash
git clone https://github.com/JoaoValentimTheo/aura-lang.git
cd aura-lang
python3 main.py run examples/hello.aura
```

## Examples

### Hello World

```aura
print("Hello, Aura!")
```

### Fibonacci

```aura
def fibonacci(n) -> int {
  let mut a = 0
  let mut b = 1
  for i in range(n) {
    let next = a + b
    a = b
    b = next
  }
  return a
}

print(fibonacci(10))
```

### Prime Checker

```aura
def is_prime(n) -> bool {
  if n < 2 { return false }
  if n == 2 { return true }
  if n % 2 == 0 { return false }

  let i = 3
  while i * i <= n {
    if n % i == 0 { return false }
    i += 2
  }
  return true
}

for n in [2, 3, 5, 7, 11, 13, 17, 19, 23, 29] {
  print(f"{n} is prime: {is_prime(n)}")
}
```

### Classes

```aura
class BankAccount {
  let owner: str = ""
  let balance: float = 0.0

  def new(owner: str, balance: float = 0.0) {
    self.owner = owner
    self.balance = balance
  }

  def deposit(amount: float) {
    if amount > 0.0 {
      self.balance += amount
      print(f"Deposited {amount}, balance: {self.balance}")
    }
  }

  def withdraw(amount: float) -> bool {
    if amount <= self.balance {
      self.balance -= amount
      print(f"Withdrew {amount}, balance: {self.balance}")
      return true
    }
    print("Insufficient funds")
    return false
  }

  @property
  def is_empty() -> bool {
    return self.balance == 0.0
  }
}

let account = BankAccount("Alice", 1000.0)
account.deposit(500.0)
account.withdraw(200.0)
print(f"Empty? {account.is_empty}")
```

### Pattern Matching

```aura
match value {
  case 0 { print("zero") }
  case 1 { print("one") }
  case n if n > 100 { print("big") }
  case _ { print("other") }
}
```

### Functional Pipelines

```aura
let result = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
  |> filter((x) => x % 2 == 0)
  |> map((x) => x * x)
  |> reduce((acc, x) => acc + x, 0)

print(f"Sum of even squares: {result}")
```

### Error Handling

```aura
def safe_divide(a, b) -> float {
  if b == 0 {
    throw ValueError("division by zero")
  }
  return a / b
}

try {
  print(safe_divide(10, 2))
  print(safe_divide(1, 0))
} catch error {
  print(f"Caught: {error}")
} finally {
  print("cleanup complete")
}
```

### Macros

```aura
@debug
def multiply(a, b) -> int {
  return a * b
}

@timeit
def sum_to(n) -> int {
  let mut total = 0
  for i in range(n) {
    total += i
  }
  return total
}

@memoize
def fib(n) -> int {
  if n < 2 { return n }
  return fib(n - 1) + fib(n - 2)
}
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `python3 main.py transpile <file.aura>` | Transpile to Python (stdout) |
| `python3 main.py transpile <file.aura> -o <out.py>` | Transpile to file |
| `python3 main.py check <file.aura>` | Type check |
| `python3 main.py format <file.aura>` | Format code |
| `python3 main.py lint <file.aura>` | Lint code |
| `python3 main.py run <file.aura>` | Run Aura file |
| `python3 main.py run <file.aura> -v` | Run with Python output |
| `python3 main.py repl` | Interactive REPL |

## Documentation

- [Language Reference](docs/LANGUAGE.md) / [Referencia da Linguagem](docs/LANGUAGE_PT.md)
- [Type System](docs/TYPES.md) / [Sistema de Tipos](docs/TYPES_PT.md)
- [Architecture](docs/DESIGN.md)
- [Audit Report](docs/AUDIT.md)
- [Documentation Index](docs/README.md)

## Testing

```bash
python3 -m pytest tests/ -v          # full suite
python3 tests/test_runtime.py        # transpile + execute programs
python3 tests/test_regressions.py    # audit regression coverage
```

Set `AURA_FUZZ_SEEDS=100000` to run the stochastic fuzzer beyond its
default sample of 200 seeds.

## License

MIT
