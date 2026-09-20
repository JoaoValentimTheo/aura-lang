---
layout: default
title: "Closures and Lambdas"
parent: Aura Language Reference
nav_order: 2
---

[English](closures.md) · [Português](closures.pt_BR.md)

# Closures and Lambdas

**Status:** Stable (except where labeled) · **Evidence:** `parser/to_ast.py`
(lambda parsing §2657-2700, `_is_lambda_params_ahead` §2871, `_parse_lambda_params`
§2893, `parse_lambda_body` §2946), `transpiler/transformers/expressions.py`
(`transform_LambdaExpr` §662, `_hoist_block_lambda` §689,
`_nonlocal_declaration` §711, `transform_PipeExpr` §578).

A lambda is an anonymous function **value**. A closure is a lambda that captures
variables from the enclosing scope.

---

## 1. Lambda forms

```
lambda         = ( identifier | "(" , [ param_list ] , ")" ) , "=>" , ( expression | block ) ;
```

| Form | Example |
| --- | --- |
| Single param, no parens | `x => x * 2` |
| Parens, one param | `(x) => x * x` |
| Multiple params | `(a, b) => a + b` |
| No params | `() => 42` |
| Expression body | `(x) => x + 1` |
| Block body | `(x) => { let y = x + 1; return y }` |
| Currying | `(n) => (x) => x + n` |

```aura
let double = x => x * 2
let square = (x) => x * x
let add = (a, b) => a + b
let get_answer = () => 42
```

- The `=>` marker is required; there is **no `lambda` keyword** (`to_ast.py:2650-2653`,
  *probe*).
- An **expression body** is emitted as a Python `lambda` expression, e.g.
  `(lambda x: (x * 2))` (`transform_LambdaExpr`, `expressions.py:662-669`).
- A **block body** cannot live inside a Python `lambda`; it is **hoisted** to a
  real named function `_aura_lambda_N` and the lambda value becomes that name
  (`_hoist_block_lambda`, `expressions.py:689-708`).

```aura
let compute = (x) => {
  let doubled = x * 2
  return doubled + 1
}
```

*Evidence:* `test_syntax_complete.py::test_lambda_forms`,
`::test_lambda_block_body`, `test_expressions_deep.py::test_transform_simple_lambda`.

### 1.1 Parameters

Lambda parameters use the same grammar as `def` parameters
(`_parse_lambda_params`, `to_ast.py:2893-2944`): annotated, defaulted, `*args`,
`**kwargs` and a bare `*`.

```aura
let f = (a, *rest) => a
let g = (a, **kw) => a
let h = (a, b = 1) => a + b
let typed = (x: int) => x + 1      // annotation accepted, then erased
```

- Types on lambda parameters are **accepted** in the `=>` form
  (`parse_lambda_params` calls `parse_type`, `to_ast.py:2934-2935`).
- **UNSPECIFIED / erased:** the emitted Python `lambda` carries no annotations,
  so the annotation is documentation only (*probe*: `(x: int) => x + 1` emits
  `(lambda x: (x + 1))`).
- A `->` **return type is not** part of lambda syntax: `(x: int) -> int { … }`
  is a parse error (*probe*). Use the `=>` form.

---

## 2. Closures

### 2.1 Read capture (snapshot / free variable)

A lambda that reads an enclosing local captures it by reference in the generated
Python closure.

```aura
def main() {
  let n = 10
  let f = () => n + 1
  print(f())          // 11
}
```

*Evidence:* *probe* (11).

### 2.2 Mutable capture (`nonlocal`)

When a **block lambda** assigns to an enclosing local, the hoisted function
declares `nonlocal <name>`, so the write is visible outside the lambda
(`_nonlocal_declaration`, `expressions.py:711`).

```aura
def main() {
  let mut n = 0
  let inc = () => { n = n + 1 }
  inc()
  inc()
  print(n)            // 2
}
```

Emitted form (abridged):

```python
def _aura_lambda_1():
    nonlocal n
    n = (n + 1)
inc = _aura_lambda_1
```

*Evidence:* *probe* (2), `test_expressions_deep.py::test_hoist_block_lambda_includes_nonlocal`.

### 2.3 Currying / returning a lambda

```aura
let make_adder = (n) => (x) => x + n
let add10 = make_adder(10)
print(add10(5))       // 15
```

*Evidence:* *probe* (15).

> **UNSPECIFIED:** the capture is by reference through Python's closure protocol,
> not an explicit snapshot. A lambda created in a loop shares the loop variable's
> cell, so it observes the **final** value: *probe* with
> `for i in range(3) { fs.add((x) => x + i) }` gives `fs[0](0) == 2`, not `0`. Bind
> the value as a default parameter (`(x, i = i) => x + i`) for per-iteration
> capture.

---

## 3. Lambdas with `map` / `filter` / `reduce`

These functions are auto-imported from `stdlib.collections` when used
(transformer injects `from stdlib.collections import map, filter, reduce, …`).

```aura
let numbers = [1, 2, 3, 4, 5]
let doubled = map(numbers, (x) => x * 2)
let evens = filter(numbers, (x) => x % 2 == 0)
let total = reduce(numbers, (a, b) => a + b, 0)
```

*Evidence:* *probe* — transpiles to
`map(numbers, (lambda x: (x * 2)))` etc.

---

## 4. Pipe operator

The pipe `|>` applies the right-hand callable to the left-hand value as its
**first argument** (`transform_PipeExpr`, `expressions.py:578-591`).

```aura
let result = [1, 2, 3, 4, 5]
  |> filter((x) => x > 2)
  |> map((x) => x * 10)
  |> reduce((a, b) => a + b, 0)
```

The pipeline is left-to-right. *Probe*: dropping the final stage gives
`[30, 40, 50]`, and the full chain with `reduce` gives `120`. If the right side
is a bare identifier rather than a call, it becomes `f(left)`.

*Evidence:* *probe*, `test_syntax_complete.py::test_pipe_operator`,
`test_expressions_deep.py::test_transform_pipe_into_call_inserts_argument`,
`::test_transform_pipe_into_identifier`.

---

## 5. Trailing lambda / block arguments

**Aura has no trailing-lambda call sugar.** A `{ ... }` after a call is not
attached as a final argument; *probe* shows `apply(1) { x => x + 1 }` parses as
the call `apply(1)` followed by a separate lambda expression statement, and
`transaction { print("x") }` parses as two statements. Call arguments must sit
**inside** the parentheses.

```aura
apply((x) => x + 1)          // the lambda is a normal argument
```

A `{` after a **capitalized** identifier is a struct init
(`TypeName { field: value }`, `grammar.md` §6.6), not a block argument.

*Evidence:* *probe*; `to_ast.py:2986-3052` (call argument parsing),
`to_ast.py:2678-2679` (struct-init branch).

---

## 6. Limits

- **No `lambda` keyword** — `lambda x: x` is a pointed parse error
  (`to_ast.py:2650-2653`).
- **No implicit parameter** (`it` / `$0`); name every parameter.
- A lambda accepts `(x: T) => …` (**annotated, erased**), but **no `->` return
  type**: `(x: int) -> int { … }` is a parse error (*probe*). See §1.1.
- **Recursion through a `let`-bound lambda works**, for both expression and
  block bodies: name resolution happens at call time, after the `let` binds the
  name. *Probe*: both
  `let fact = (n) => n <= 1 ? 1 : n * fact(n - 1)` and the block-body equivalent
  give `fact(5) == 120`.
- **UNSPECIFIED:** there is no implicit self-reference variable; a lambda must
  refer to itself through the name it is bound to.

---

## 7. Function types

A lambda's type is a **function type** written `(T, …) -> R`
(`grammar.md` §4, `types.md` §4).

```aura
let handler: (int, int) -> int = add
```

> **UNSPECIFIED:** function types are erased at transpile time; the emitted
> Python is untyped. See [types.md](types.md) for the type-checking rules.
