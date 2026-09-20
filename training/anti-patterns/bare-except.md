# Anti-Pattern: Bare `catch` (Catching Everything)

**Severity:** Medium · **Category:** Error Handling · **See also:**
[Error Handling](../../docs/learn/12-error-handling.md), [Statements Reference](../../docs/language-reference/statements.md)

---

## Problem

A bare `catch` (with no type) catches **every** exception, including
`KeyboardInterrupt`, `SystemExit`, and programming errors. It silently swallows
bugs and makes debugging nearly impossible.

## The BAD code

```aura
def read_file(path: str) -> str {
  try {
    return open(path).read()
  } catch {
    return ""     // swallowed: permission error? typo? who knows
  }
}

def divide(a: float, b: float) -> float {
  try {
    return a / b
  } catch as e {
    print(f"error: {e}")
    return 0.0    // hides the real problem
  }
}
```

## The GOOD code

```aura
def read_file(path: str) -> str {
  try {
    return open(path).read()
  } catch FileNotFoundError {
    return ""     // only the expected case
  }
}

def divide(a: float, b: float) -> float {
  guard b != 0 else {
    throw ValueError("division by zero")
  }
  return a / b
}
```

## Key takeaway

Always catch a **specific** exception type. Use `catch as e` only as a true
last resort. Aura's four catch spellings — `catch { }`, `catch Type { }`,
`catch Type as e { }`, `catch as e { }` — exist so you pick the narrowest one.

**Rule:** Never write `catch { }` without a specific type unless you are
certain every possible exception is acceptable to swallow.
