# Functions and closures

## Declaration

```aura
fn add(a: int, b: int) -> int {
    return a + b
}
```

Parameters and the return type may be annotated. The body is a block; its value
is the value of its last statement, so `return` is optional for the final value.
A function with no `return` and no final value yields `none`.

If a function declares a return type, every `return` must be compatible with it
(`E3005`).

## Hoisting and recursion

Top-level functions are hoisted and may be mutually recursive. Recursion is
bounded by the call-frame limit: more than 512 active calls is `E4011`.

```aura
fn fib(n) -> int {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}
```

## Arguments

Calls to a **directly resolved top-level function** accept positional arguments
and, after them, named arguments:

```aura
fn box(width, height) { width * height }

box(3, 4)
box(width: 3, height: 4)
```

All positional arguments must come before named ones. Every declared parameter
must be supplied exactly once; a duplicate, an unknown name, or a missing
parameter is `E3001`. Named arguments are not accepted by builtins or methods.

There are no default parameter values and no variadic parameters in this
version.

## First-class functions

A named function used as a value is a function value. A builtin name used as a
value is a native function value.

```aura
let xs = [1, 2, 3, 4]
let evens = xs.filter(is_even)
```

## Lambdas and closures

A lambda is `x -> e`, `(x, y) -> e`, or `(x, y) -> { ... }`. It captures its
defining environment **by reference**, not by value:

```aura
let mut total = 0
let add = (n) -> { total = total + n }
add(5)
print(total)      # 5
```

A block-bodied lambda uses the block as its body, so `return` works and the last
expression is the value.

## The pipeline operator

`|>` passes its left operand as the **first argument** of a call:

```aura
let result = [1, 2, 3, 4, 5, 6]
    |> filter((x) -> x % 2 == 0)
    |> map((x) -> x * x)
```

Wait — a pipeline must stay on one line. A newline ends the statement, so write:

```aura
let result = [1, 2, 3, 4, 5, 6] |> filter((x) -> x % 2 == 0) |> map((x) -> x * x)
```

`x |> f` is `f(x)`; `x |> f(a)` is `f(x, a)`; `x |> r.m(a)` is `r.m(x, a)`.
