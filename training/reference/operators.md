# Aura Operators — Quick Reference

## Precedence (lowest → highest)

| Prec | Operators | Assoc | Notes |
|------|-----------|-------|-------|
| 1 | `\|>`, `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `**=`, `&=`, `\|=`, `^=`, `<<=`, `>>=`, `??=` | left (pipe right) | Pipe is loosest |
| 2 | `? :` | right | Ternary conditional |
| 3 | `or` | left | Short-circuit, returns operand |
| 4 | `and` | left | Short-circuit, returns operand |
| 5 | `==`, `!=`, `<`, `>`, `<=`, `>=`, `in`, `not in`, `is`, `is not` | left | No chaining: `a < b < c` → `((a < b) < c)` |
| 6 | `\|` | left | Bitwise OR |
| 7 | `^` | left | Bitwise XOR |
| 8 | `&` | left | Bitwise AND |
| 9 | `<<`, `>>` | left | Shifts |
| 10 | `..`, `..<` | none | Ranges (→ `range(...)`) |
| 11 | `??`, `?:` | right/left | Null-coalesce / Elvis |
| 12 | `+`, `-` | left | Arithmetic |
| 13 | `*`, `/`, `%`, `as` | left | Arithmetic + cast |
| 14 | `**` | right | Power |
| 15 | unary `-`, `+`, `~`, `not`, `await`, `...` | prefix | |
| 16 | call, index, slice, member, safe-nav, struct-init | postfix | `f()`, `a[i]`, `a.b`, `a?.b` |

## Arithmetic

| Op | Example | Result | Notes |
|----|---------|--------|-------|
| `+` | `3 + 2` | `5` | String concatenation too |
| `-` | `5 - 3` | `2` | |
| `*` | `4 * 3` | `12` | String repeat: `"ab" * 2` → `"abab"` |
| `/` | `7 / 2` | `3.5` | **True division** — never truncates |
| `%` | `-7 % 3` | `2` | Divisor sign (Python semantics) |
| `**` | `2 ** 3` | `8` | Right-assoc: `2 ** 3 ** 2` → `512` |

> **No `//` floor division.** Use `int(a / b)` for integer quotient.
> **`-2 ** 2` → `-4`** — unary `-` binds looser than `**`.

## Comparison & Equality

| Op | Example | Notes |
|----|---------|-------|
| `==`, `!=` | Value equality (Python `==`) | |
| `<`, `>`, `<=`, `>=` | Standard ordering | |
| `in`, `not in` | Membership: `3 in [1,2,3]` | |
| `is`, `is not` | Identity: `x is none` | **Only `none` allowed** with `is`; `x is 5` is a parse error |

## Logical

| Op | Example | Notes |
|----|---------|-------|
| `and` | `a and b` | Returns operand, not bool |
| `or` | `a or b` | Returns operand, not bool |
| `not` | `not a` | `!`, `&&`, `\|\|` are **rejected** |

## Bitwise

| Op | Example | Notes |
|----|---------|-------|
| `&` | `a & b` | AND |
| `\|` | `a \| b` | OR |
| `^` | `a ^ b` | XOR |
| `~` | `~x` | Complement (unary prefix) |
| `<<`, `>>` | `a << 2` | Shifts |

## Null-aware

| Op | Example | Notes |
|----|---------|-------|
| `??` | `x ?? "fallback"` | Yields RHS when LHS is `none` |
| `?:` | `x ?: "zero-ish"` | Yields RHS when LHS is **falsy** |
| `??=` | `x ??= y` | Compound: `x = x if x is not None else y` |

## Ternary & Elvis

| Form | Example | Compiles to |
|------|---------|-------------|
| `c ? t : f` | `x > 0 ? "pos" : "neg"` | `(t if c else f)` |
| `a ?: b` | `name ?: "anon"` | `_aura_elvis(a, b)` |
| `a ?? b` | `val ?? 0` | `_aura_null_coalesce(a, b)` |

## Ranges

| Form | Example | Meaning |
|------|---------|---------|
| `a..b` | `1..5` | Inclusive: `range(1, 6)` |
| `a..<b` | `0..<3` | Exclusive: `range(0, 3)` |
| `a..b step n` | `0..10 step 2` | `range(0, 11, 2)` |
| `a..` | `0..` | Infinite: `itertools.count(0)` |

## Pipe

| Form | Compiles to |
|------|-------------|
| `a \|> f` | `f(a)` |
| `a \|> f(b)` | `f(a, b)` |
| `a \|> f \|> g` | `g(f(a))` |

## Cast

| Form | Compiles to |
|------|-------------|
| `x as int` | `int(x)` |
| `x as str` | `str(x)` |
| `x as float` | `float(x)` |

## Safe Navigation

| Form | Compiles to |
|------|-------------|
| `a?.b` | `(a.b if a is not None else None)` |
| `a?[i]` | `(a[i] if a is not None else None)` |

## Spread

| Context | Form | Notes |
|---------|------|-------|
| Call | `f(*a)`, `f(**m)` | Positional / keyword spread |
| Call | `f(...v)` | Adaptive: unpacks dict as kwargs |
| List | `[*a, *b]` | Concat |
| Dict | `{**a, **b}` | Merge |

## Lambdas

| Form | Example |
|------|---------|
| Single param | `x => x * 2` |
| Multi param | `(a, b) => a + b` |
| No params | `() => 42` |
| Block body | `(x) => { return x + 1 }` |
