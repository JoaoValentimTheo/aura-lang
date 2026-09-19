---
layout: default
title: "12 — Error Handling"
parent: Learn Aura
nav_order: 22
---

[English](12-error-handling.md) | [Português](12-error-handling.pt_BR.md)

# 12 — Error Handling

> **Chapter goal:** signal and recover from failure. Reference:
> [`../language-reference/statements.md`](../language-reference/statements.md)
> §7–10 and [`../language-reference/semantics.md`](../language-reference/semantics.md)
> §5. Example: `../../examples/error_handling.aura`.

## `throw`

`throw` raises a **value**. On CPython the value is raised:

```aura
def safe_divide(a, b) -> float {
  if b == 0 {
    throw ValueError("division by zero")
  }
  return a / b
}
```

| Form | Emits |
|---|---|
| `throw ValueError("bad")` | `raise ValueError('bad')` |
| `throw "msg"` | `raise Exception("msg")` — a bare string is wrapped |

`UNSPECIFIED`: a non-string, non-exception value (`throw 5`) is emitted as
`raise 5` and fails at runtime; it is not rejected at check time.

## `try` / `catch` / `finally`

```aura
def main() {
  try {
    print(safe_divide(10, 2))
    print(safe_divide(1, 0))
  } catch Error as error {
    print(f"Caught: {error}")
  } finally {
    print("cleanup complete")
  }
}
```

```text
5.0
Caught: division by zero
cleanup complete
```

A `try` requires **at least one** `catch` or a `finally`. `finally` always runs.

### `catch` spellings

Each spelling has exactly one meaning:

| Form | Meaning |
|---|---|
| `catch { }` | catch every exception, no binding |
| `catch Type { }` | catch only `Type` |
| `catch Type as e { }` | catch only `Type`, bind to `e` |
| `catch as e { }` | catch every exception, bind to `e` |

A lone identifier before `{` is always a **type**, never a binding; use
`catch Type as name` to bind. The old ambiguous `catch e { }` is a parse error.

```aura
def main() {
  try {
    let values = [1]
    print(values[5])
  } catch IndexError {
    print("index out of range")
  }
}
```

`TARGET-SPECIFIC`: catch clauses are emitted in source order and the type is
whatever CPython resolves the name to. Aura does not reorder subclasses before
parents — list subclasses first.

## `guard` — validate early

`guard cond else { ... }` runs the block when `cond` is falsy, then continues.
It keeps the happy path unindented:

```aura
def process(value) {
  guard value > 0 else {
    print(f"invalid value: {value}")
    return
  }
  print(f"processing {value}")
}
```

At the top level a bare `return` in the guard body exits the program:

```aura
guard ready else { return }
```

## `try` as an expression

`try { ... } catch ... { ... }` can produce a value, so it doubles as a
fallback:

```aura
def main() {
  let parsed = try { int("nope") } catch Error as e { 0 }
  print(parsed)          // 0
}
```

## `assert`

```aura
assert 2 + 2 == 4, "math is broken"
```

When the condition is falsy, `assert` raises `AssertionError` with the given
message. Both the condition and the message are expressions.

## `with` — resource management

`with` uses `__enter__`/`__exit__`:

```aura
def main() {
  with open("/tmp/data.txt") as f {
    print(f.read(1))
  }
}
```

`async with` uses `__aenter__`/`__aexit__` and belongs in an `async def`.

## A complete example

```aura
def safe_divide(a, b) -> float {
  if b == 0 { throw ValueError("division by zero") }
  return a / b
}

def main() {
  try {
    print(safe_divide(10, 2))
    print(safe_divide(1, 0))
  } catch Error as e {
    print(f"Caught: {e}")
  } finally {
    print("done")
  }
}
```

```text
5.0
Caught: division by zero
done
```

## What you learned

* `throw` raises a value; a bare string is wrapped.
* `try` requires a `catch` or a `finally`; four `catch` spellings.
* `guard ... else` for early validation; `return` at top level exits.
* `try` as an expression; `assert`; `with`.

## Next step

[Modules and Imports →](13-modules-and-imports.md)
