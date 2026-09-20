---
title: "03 — Expressions"
---

# Expressions

An expression produces a value. The grammar is in `docs/language-reference/grammar.md` §6; this document covers semantics.

See `aura/parser/to_ast.py:465-488` (`_PRECEDENCE`), `aura/transpiler/transformers/expressions.py`.

## Precedence (Lowest to Highest)

| Prec | Operators | Associativity |
|------|-----------|---------------|
| 1 | `=` `+=` `-=` `*=` `/=` `%=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `??=` `\|>` | right (pipe left) |
| 2 | `? :` | right |
| 3 | `or` | left |
| 4 | `and` | left |
| 5 | `==` `!=` `<` `>` `<=` `>=` `in` `not in` `is` `is not` | left |
| 6 | `\|` | left |
| 7 | `^` | left |
| 8 | `&` | left |
| 9 | `<<` `>>` | left |
| 10 | `..` `..<` | none |
| 11 | `??` `?:` | right / left |
| 12 | `+` `-` | left |
| 13 | `*` `/` `%` `as` | left |
| 14 | `**` | right |
| 15 | unary `-` `+` `~` `not` `await` `...` | prefix |
| 16 | call, index, slice, member, safe-nav, struct-init | postfix |

Comparison is looser than bitwise and shifts, exactly as in Python. `1 & 2 == 2` → `((1 & 2) == 2)`. Comparison does **not** chain: `a < b < c` → `((a < b) < c)`.

## Literals

| Source | Compiles to |
|--------|-------------|
| `42` | `42` |
| `3.5` | `3.5` |
| `"hi"` | `'hi'` |
| `true` / `false` | `True` / `False` |
| `none` | `None` |
| `f"x{1+2}"` | `f'x{1 + 2}'` |

## Arithmetic: `+ - * / % **`

| Expression | Result | Why |
|------------|--------|-----|
| `7 / 2` | `3.5` | `/` is true division, never truncates |
| `-7 % 3` | `2` | `%` follows the divisor's sign |
| `2 ** 3 ** 2` | `512` | `**` is right-associative |
| `-2 ** 2` | `-4` | unary `-` binds looser than `**` |

There is **no floor division**. `//` starts a line comment. Use `int(total / count)`.

`+` on strings is concatenation; `*` with an integer repeats. These are Python semantics.

## Comparison and Identity

- `==` / `!=` are value equality.
- `is` / `is not` are identity (Python `is`/`is not`).
- `x is none` / `x is not none` is the idiomatic null check.
- Identity against any other literal is a parse error: `x is "a"` → error.
- `in` / `not in` check membership.

## Logical: `and or not`

`and`/`or` use Python truthiness and return an operand, not a coerced `bool`:
`1 and 2` → `2`, `0 or "fallback"` → `"fallback"`. Short-circuit applies.

`not` binds tighter than `and`/`or`: `not a and b` → `((not a) and b)`.

`&&`, `||`, `!` are rejected with messages to use `and`/`or`/`not`.

## Ternary, Elvis, Null-Coalescing

| Form | Meaning | Compiles to |
|------|---------|-------------|
| `c ? t : f` | conditional | `(t if c else f)` |
| `a ?: b` | `b` when `a` is falsy | `_aura_elvis(a, b)` |
| `a ?? b` | `b` only when `a` is `none` | `_aura_null_coalesce(a, b)` |

`a ?? b ?? c` is right-associative. `a ?: b ?: c` is left-associative.

## Pipe: `|>`

`a |> f` applies `f` to `a`; `a |> f(b)` inserts `a` as the **first** argument:

```aura
let result = [1, 2, 3] |> map((x) => x * 2) |> sum()
// compiles to: sum(map([1, 2, 3], (lambda x: x * 2)))
```

Pipe is the loosest expression operator (level 1) and chains left to right.

## Ranges: `..`, `..<`, `step`

| Source | Compiles to |
|--------|-------------|
| `1..10` | `range(1, 10 + 1)` (inclusive) |
| `0..<100` | `range(0, 100)` (exclusive) |
| `0..100 step 5` | `range(0, 100 + 1, 5)` |
| `0..` | `itertools.count(0)` (open-ended) |

A range evaluates to a Python `range` object. Using a range as an index is a slice, not `range(...)`.

## Safe Navigation: `?.` and `?[`

```aura
let city = user?.address?.city
// compiles to: (user.address.city if user is not None else None)
```

Guarding is against `is not None`, not falsiness. `?.` and `?[` can be chained.

## Comprehensions

```aura
let evens = [x for x in 0..<100 if x % 2 == 0]
let cubes = {x: x ** 3 for x in 1..10}
let unique = {x for x in items}
let gen = (x * 2 for x in items)
```

Multiple `for` and multiple `if` clauses are supported. Comprehensions are eager; generator expressions are lazy.

## Lambdas

```aura
let double = (x) => x * 2
let add = (a, b) => a + b
let block_fn = (x) => {
  let y = x + 1
  return y
}
```

Single-parameter lambdas can omit parentheses: `x => x`. Block-bodied lambdas are hoisted to real functions.

## Spread Operators

| Context | Form | Compiles to |
|---------|------|-------------|
| call | `f(*args)` | `f(*args)` |
| call | `f(**kw)` | `f(**kw)` |
| call | `f(...val)` | `_aura_call(f, val)` (adaptive) |
| list | `[*a, *b]` | `[*a, *b]` |
| dict | `{**a}` | `AuraDict({**a})` |

`...` is adaptive: when the sole call argument, it unpacks dicts as keyword arguments and anything else as positional.

## Gotcha

- `-2 ** 2` is `-4`, not `4`. Unary `-` binds looser than `**`.
- `(1)` is not a tuple. Use `(1,)`.
- `a == b == c` does not chain. Write `a == b and b == c`.
- `1..10` is inclusive; `0..<10` is exclusive. The `..` adds 1 to the upper bound.
- `x is "a"` is a parse error — use `x == "a"`.

## Anti-pattern

Do not write deeply nested ternaries. `a ? b ? c : d : e ? f : g` is hard to read. Use `if`/`else` blocks for clarity.
