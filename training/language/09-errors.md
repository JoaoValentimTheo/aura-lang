---
title: "09 — Error Handling"
---

# Error Handling

Aura uses exceptions for error recovery. `try`/`catch`/`finally` catches errors, `throw` raises them, `guard` validates preconditions, and `assert` checks invariants.

See `aura/parser/to_ast.py:parse_try_stmt`, `aura/transpiler/transformers/statements.py:transform_TryStmt`.

## `throw`

```aura
def safe_divide(a: float, b: float) -> float {
  if b == 0 { throw ValueError("division by zero") }
  return a / b
}
```

| Form | Compiles to |
|------|-------------|
| `throw ValueError("bad")` | `raise ValueError('bad')` |
| `throw "msg"` | `raise Exception("msg")` (wrapped) |

A non-string, non-exception expression is emitted as-is and may fail at runtime.

## `try` / `catch` / `finally`

```aura
try {
  let result = safe_divide(10, 0)
} catch ValueError as e {
  print(f"Caught: {e}")
} finally {
  print("cleanup complete")
}
```

`try` requires at least one `catch` or a `finally`. `finally` always runs.

### Catch Spellings

| Form | Meaning |
|------|---------|
| `catch { }` | catch every exception, no binding |
| `catch Type { }` | catch only `Type` |
| `catch Type as e { }` | catch only `Type`, bind to `e` |
| `catch as e { }` | catch every exception, bind to `e` |

A lone identifier before `{` is always a **type**, never a binding.

### `try` as an Expression

```aura
let parsed = try { int("nope") } catch Error as e { 0 }
print(parsed)          // 0
```

The catch body's tail expression becomes the returned value.

### Catch Ordering

Catch clauses are emitted in source order. Aura does not reorder subclasses before parents — **list subclasses first**.

## `guard cond else { ... }`

```aura
def process(data) {
  guard data != none else {
    print("No data"); return
  }
  print(data)          // data is non-none here
}
```

Runs the `else` block when `cond` is **falsy**, then continues. At module scope, a bare `return` in the guard body exits the program.

## `assert`

```aura
assert 2 + 2 == 4
assert 1 == 2, "math is broken"   // raises AssertionError("math is broken")
```

## `with` — Resource Management

```aura
with open("/tmp/data.txt") as f { print(f.read(1)) }
async with fetch(url) as resp { await resp.text() }
```

`with` uses `__enter__`/`__exit__`; `async with` uses `__aenter__`/`__aexit__`.

## Complete Example

```aura
def safe_divide(a: float, b: float) -> float {
  if b == 0 { throw ValueError("division by zero") }
  return a / b
}

def process(value: float) {
  guard value > 0 else {
    print(f"invalid value: {value}"); return
  }
  try {
    let result = safe_divide(100.0, value)
    print(f"result: {result}")
  } catch ValueError as e { print(f"error: {e}") }
  finally { print("done") }
}

def main() {
  process(0)     // invalid value: 0 / done
  process(5)     // result: 20.0 / done
}
```

## Error Patterns

### Typed Error Handling

```aura
enum ValidationError { Empty, TooLong, InvalidFormat }

def validate(name: str) -> none | ValidationError {
  guard name.length() > 0 else { return ValidationError.Empty }
  guard name.length() <= 50 else { return ValidationError.TooLong }
  return none
}
```

### Option Pattern (`T | none`)

```aura
def find_user(id: int) -> User | none {
  if id == 0 { return none }
  return User("user" + str(id))
}
```

## Gotcha

- `try` requires at least one `catch` or a `finally`.
- `throw "msg"` wraps the string in `Exception()`. `throw 5` fails at runtime.
- `catch` clauses are in source order. List subclasses before their parents.
- `guard` is not `if` — it runs the else block on falsy, then continues.
- `assert` is for invariants, not input validation. Use `guard` for preconditions.

## Anti-pattern

Do not catch `Exception` (or use bare `catch { }`) to handle expected errors. Use specific exception types. Do not use `try` for control flow — use `match` or `if` for normal branching.
