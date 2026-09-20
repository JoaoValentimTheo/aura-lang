---
title: "05 — Functions"
---

# Functions

`def` is the **only** function keyword. `fn` is rejected with a pointed message; `lambda`, `fun`, and `func` are not Aura.

See `aura/parser/to_ast.py:1156-1176` (`parse_function_decl`), `aura/transpiler/transformers/statements.py:319-391`.

## Declaration

```aura
def greet(name) -> str { return "Hello, " + name }
def square(x: int) -> int = x * x     // expression body
def double(x) = x * 2                 // no return type
```

An expression body is desugared to `return <expr>` at parse time.

## Parameters

```aura
def greet(name, greeting = "hello") { return greeting + ", " + name }
greet("Alice")            // "Hello, Alice"
greet("Bob", "Hi")        // "Hi, Bob"
```

Two forms per position: annotated (`name: type`) or bare, both with optional default `= expr`.

### Keyword Arguments

At a call site, `name: expr` or `name = expr` becomes a Python keyword argument:

```aura
create_user(name: "Alice", age: 30, email: "a@x.com")
```

### Variadic Parameters

```aura
def sum_all(*numbers) -> int { return sum(numbers) }
def log(level, **context) { print(f"[{level}] {context}") }
def f(a, *, b) { return a + b }   // b is keyword-only
```

### Spread at the Call Site

```aura
add(*nums)      // positional spread
add(**kw)       // keyword spread
add(...nums)    // adaptive: dict → kwargs, else positional
```

## Return

```aura
def swap(a, b) { return b, a }   // tuple
let x, y = swap(1, 2)           // x = 2, y = 1
return                            // bare, no value
```

Comma-separated values become a tuple. A bare `return` yields `None`.

## Generics

```aura
def id[T](x: T) -> T { return x }
def smallest[T: Comparable](items: [T]) -> T { return items[0] }
def map[T, R](f: (T) -> R, items: [T]) -> [R] {
  return [f(i) for i in items]
}
```

Type parameters use **square brackets** and precede the parameter list. Constraints follow `:` on the parameter. Generic parameters are erased at transpile time.

## Recursion

```aura
def factorial(n) -> int {
  if n <= 1 { return 1 }
  return n * factorial(n - 1)
}
```

No tail-call optimization is guaranteed; deep recursion is bounded by the CPython stack.

## Nesting

```aura
def main() {
  def helper(x: int) -> int { return x + 1 }
  print(helper(2))     // 3
}
```

Inner `def` is emitted at the point of declaration inside the enclosing Python function.

## `async` and Decorators

```aura
async def fetch_data(url) -> str {
  let response = await http_get(url)
  return response.body
}

@memoize
def fib(n) -> int { return n if n < 2 else fib(n - 1) + fib(n - 2) }
```

Built-in decorators: `@debug`, `@timeit`, `@memoize`, `@cache(maxsize=...)`, `@must_return`, `@deprecated(...)`. Class decorators: `@property`, `@staticmethod`, `@classmethod`.

## Entry Point (`main`)

```aura
def main() { print("hello") }
def main(args: [string]) { print(f"{args.length()} argument(s)") }
```

An entry file must declare a top-level `main`. Missing → **E310**. Wrong signature → **E311**. Inside a module → **E312**.

## Visibility (Class Members)

| Modifier | Meaning |
|----------|---------|
| `public` | accessible from anywhere |
| `private` | accessible only inside the declaring class (mangled) |
| `protected` | accessible inside the declaring class and its subclasses |

Omitting visibility on a class member is **E307**.

## Gotcha

- `def` is the only keyword. `fn`, `fun`, `func`, `function` are all rejected.
- `main` takes no parameters or a single `args` parameter. Other signatures are E311.
- A `main` inside a `module` body is E312.
- No overloading: a repeated name in the same class body is E301.
