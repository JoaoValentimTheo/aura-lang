# Aura Built-ins — Quick Reference

## Primitive Types

| Type | Example | Notes |
|------|---------|-------|
| `int` | `42` | Arbitrary precision (Python `int`) |
| `float` | `3.14` | 64-bit floating point |
| `str` | `"hello"` | Unicode, immutable |
| `bool` | `true` / `false` | Not numeric — `true + 1` is E108 |
| `bytes` | `b"data"` | Annotation accepted, not checked |
| `none` | `none` | Null value **and** type |

## Collection Types

| Type | Example | Notes |
|------|---------|-------|
| `[T]` | `[1, 2, 3]` | List (also `List[T]`) |
| `{K: V}` | `{a: 1}` | Dict (also `Dict[K, V]`) |
| `{T}` | `{1, 2}` | Set (also `Set[T]`) |
| `(A, B)` | `(1, "x")` | Tuple — comma required: `(42,)` |
| `{name: T}` | `{x: float}` | Structural dict (names not checked) |

## Type Modifiers

| Syntax | Example | Meaning |
|--------|---------|---------|
| `T?` | `str?` | Optional — same as `T \| none` |
| `T \| U` | `int \| str` | Union |
| `(A) -> B` | `(int) -> str` | Function type |
| `Box[T]` | `Box[int]` | Generic (brackets only, never `<...>`) |

## Type Casting

| Form | Compiles to | Example |
|------|-------------|---------|
| `x as int` | `int(x)` | `3.0 as int` → `3` |
| `x as str` | `str(x)` | `42 as str` → `"42"` |
| `x as float` | `float(x)` | `"3" as float` → `3.0` |

## Collection Conveniences (on any collection)

| Method | Compiles to | Example |
|--------|-------------|---------|
| `.length()` / `.size()` / `.len()` | `len(x)` | `[1,2].length()` → `2` |
| `.contains(x)` | `x in xs` | `xs.contains(3)` |
| `.is_empty()` | `not xs` | `xs.is_empty()` |
| `.add(x)` | `xs.append(x)` | `xs.add(4)` |

> Unknown member calls pass through to Python: `.append`, `.pop`, `.sort`, etc.

## String Methods

| Method | Maps to | Example |
|--------|---------|---------|
| `.to_upper()` | `.upper()` | `"hi".to_upper()` → `"HI"` |
| `.to_lower()` | `.lower()` | `"HI".to_lower()` → `"hi"` |
| `.starts_with(p)` | `.startswith(p)` | `"abc".starts_with("a")` |
| `.ends_with(p)` | `.endswith(p)` | `"abc".ends_with("c")` |
| `.trim()` | `.strip()` | `" x ".trim()` → `"x"` |
| `.trim_left()` | `.lstrip()` | |
| `.trim_right()` | `.rstrip()` | |
| `.slice(a, b)` | `s[a:b]` | `"hello".slice(1, 4)` → `"ell"` |
| `.char_at(i)` | `s[i]` | `"abc".char_at(1)` → `"b"` |
| `.index_of(sub)` | `.find(sub)` | `"abc".index_of("b")` → `1` |
| `.is_alpha()` | `.isalpha()` | |
| `.is_digit()` | `.isdigit()` | |
| `.is_numeric()` | — | Checks if parseable as number |
| `.length()` | `len(s)` | `"hi".length()` → `2` |
| `.reverse()` | `s[::-1]` | `"abc".reverse()` → `"cba"` |
| `.lines()` | `.splitlines()` | |

## Null Checks

| Form | Meaning | Example |
|------|---------|---------|
| `x is none` | Identity null check | `if name is none { ... }` |
| `x ?? y` | Fallback if `none` | `val ?? "default"` |
| `x ?: y` | Fallback if falsy | `0 ?: "zero-ish"` → `"zero-ish"` |

## Built-in Functions

| Function | Example | Notes |
|----------|---------|-------|
| `print(...)` | `print("hi", 42)` | Space-separated, trailing newline |
| `range(n)` | `range(5)` | `0..4` |
| `range(a, b)` | `range(1, 6)` | `1..5` inclusive |
| `range(a, b, s)` | `range(0, 10, 2)` | Step |
| `len(x)` | `len([1,2])` → `2` | |
| `int(x)` | `int("42")` → `42` | |
| `float(x)` | `float("3.14")` → `3.14` | |
| `str(x)` | `str(42)` → `"42"` | |
| `bool(x)` | `bool(0)` → `false` | |
| `type(x)` | — | Returns type name |
| `open(path)` | `open("f.txt")` | Python file handle |
| `isinstance(x, T)` | — | Type check |
| `abs(x)` | `abs(-5)` → `5` | |
| `min(a, b)` | `min(1, 2)` → `1` | |
| `max(a, b)` | `max(1, 2)` → `2` | |
| `sum(xs)` | `sum([1,2,3])` → `6` | |
| `sorted(xs)` | `sorted([3,1,2])` → `[1,2,3]` | |
| `reversed(xs)` | — | Returns iterator |
| `enumerate(xs)` | — | `(index, value)` pairs |
| `zip(a, b)` | — | Paired iteration |
| `map(f, xs)` | — | Auto-imported from stdlib |
| `filter(f, xs)` | — | Auto-imported from stdlib |
| `reduce(f, xs, init)` | — | Auto-imported from stdlib |

## Comprehensions

| Form | Example |
|------|---------|
| List | `[x * 2 for x in range(5)]` |
| List + filter | `[x for x in xs if x > 0]` |
| Dict | `{k: v for k, v in pairs}` |
| Set | `{x % 3 for x in range(10)}` |
| Generator | `(x for x in xs)` — lazy |

## Decorators (built-in)

| Decorator | Effect |
|-----------|--------|
| `@debug` | Prints call args and result |
| `@timeit` | Prints execution time |
| `@memoize` | Unbounded result cache |
| `@cache(maxsize=n)` | Bounded LRU cache |
| `@property` | Getter/setter on class |
| `@staticmethod` | Static method |
| `@classmethod` | Class-level method |

## Compile-time Macros (from `macros`)

| Macro | Effect |
|-------|--------|
| `assert_eq(a, b)` | Assert equality |
| `assert_ne(a, b)` | Assert inequality |
| `static_assert(x)` | Checked at compile time |
| `identity(x)` | Expands to `x` |
| `stringify(x)` | Literal → string at compile time |
| `swap(a, b)` | Exchange two values |
| `debug_value(x)` | Print `x = <value>`, yield value |
