---
layout: default
title: "Functions"
parent: Aura Language Reference
nav_order: 4
---

[English](functions.md) · [Português](functions.pt_BR.md)

# Functions

**Status:** Stable (except where labeled) · **Evidence:** `parser/to_ast.py`
(`parse_function_decl` §1116, `parse_function_decl_after_keyword` §2028),
`transpiler/transformers/statements.py` (`transform_FunctionDecl` §319),
`transpiler/rules.py` (`RuleChecker`).

---

## 1. Declaration

```
function_decl  = modifiers , [ "async" ] , "def" , identifier , [ type_params ]
                 , "(" , [ param_list ] , ")" , [ "->" , type ] , block ;
type_params    = "[" , identifier , { "," , identifier } , "]" ;
```

`def` is the **only** function keyword. `fn` is rejected with a pointed message
(`to_ast.py:1123-1126`); `lambda`, `fun` and `func` are not Aura.

A function body is either a **brace block** or an **expression body**:

```aura
def greet(name) -> str {
  return "Hello, " + name
}

def square(x: int) -> int = x * x     // expression body
def double(x) = x * 2                 // no return type
```

An expression body is desugared to `return <expr>` at parse time
(`to_ast.py:1200-1204`).

*Evidence:* `test_syntax_complete.py::test_def_and_fn_forms`.

### 1.1 Return type

- **Optional.** `def f() { }` has no annotation; the emitted Python is `def f():`.
- Annotation comes after `->` (`def f() -> int`).
- **UNSPECIFIED:** the return annotation is documentation; the emitted Python
  carries no return-type annotation (`transform_FunctionDecl`,
  `statements.py:351-391`), so a mismatch between the declared and actual return
  type is not enforced by the transpiler.

### 1.2 `async`

`async def` emits `async def`, and `await` may appear in its body.

```aura
async def fetch_data(url) -> str {
  let response = await http_get(url)
  return response.body
}
```

*Evidence:* `test_syntax_complete.py::test_async_await`.

---

## 2. Parameters

```
param_list     = param , { "," , param } ;
param          = "*" | "**" , identifier
               | identifier , [ ":" , type ] , [ "=" , expression ] ;
```

Two forms per position: annotated (`name: type`) or bare, both with an optional
default `= expr`.

```aura
def f(name, greeting = "Hello") { return greeting + ", " + name }
```

### 2.1 Default parameters

Defaults are emitted directly into the Python signature (`py_safe_name(name)=default`,
`statements.py:343-345`). A call may omit any trailing defaulted argument.

```aura
def greet(name, greeting = "Hello") -> str { return greeting + ", " + name }
greet("Alice")            // "Hello, Alice"
greet("Bob", "Hi")        // "Hi, Bob"
```

*Evidence:* `test_syntax_complete.py::test_function_default_and_kwargs`.

> **UNSPECIFIED:** a call omitting a *non-trailing* argument by keyword works in
> Python, but Aura's grammar does not enforce parameter-order rules beyond what
> the target accepts.

### 2.2 Named / keyword arguments

At a call site, an argument written `name: expr` **or** `name = expr` becomes a
Python keyword argument (`to_ast.py:3021-3035`).

```aura
def create_user(name, age, email) {
  return {name: name, age: age, email: email}
}

let u = create_user(name: "Alice", age: 30, email: "a@x.com")
```

*Evidence:* *probe* — `create_user(age: 3, name: 'a')` prints `a3`.

### 2.3 Variadic parameters

- `*args` — positional variadic (tuple).
- `**kwargs` — keyword-only mapping.
- A **bare `*`** marks the following parameters keyword-only
  (`to_ast.py:1147-1155`).

```aura
def sum_all(*numbers) -> int {
  return sum(numbers)
}

def log(level, **context) {
  print(f"[{level}] {context}")
}

def f(a, *, b) { return a + b }   // b must be passed as a keyword
```

*Evidence:* `test_syntax_complete.py::test_variadic_and_kwonly`,
`::test_spread_call`, `::test_spread_dict_call`.

### 2.4 Spread at the call site

Call arguments accept `*iterable` (positional spread), `**mapping` (keyword
spread) and `...value` (adaptive: dict → kwargs, otherwise positional).

```aura
add(*nums)      // add(a, b, c) from a list
add(**kw)       // add(a=…, b=…) from a dict
add(...nums)    // adaptive
```

*Evidence:* `test_syntax_complete.py::test_spread_call`,
`::test_spread_dict_call`, `::test_adaptive_spread`.

> **UNSPECIFIED:** duplicate parameter names are not checked by the grammar; the
> target Python rejects them.

---

## 3. Return

- `return` with a value, or bare `return`.
- A bare `return` in a value-producing position yields `None`
  (`transform_ReturnStmt`, `statements.py:1287-1291`).
- Multiple values are written as a comma list and become a **tuple**
  (`to_ast.py:2215-2223`).

```aura
def swap(a, b) {
  return b, a
}

let x, y = swap(1, 2)     // x = 2, y = 1
```

Returns are covered in depth in [statements.md](statements.md) §9.

*Evidence:* `test_syntax_complete.py::test_swap`.

---

## 4. Generics

```
function_decl  = … , "def" , identifier , type_params , "(" , … ;
type_params    = "[" , identifier , { "," , identifier } , "]" ;
```

Type parameters use **square brackets** and precede the parameter list. A
constraint may follow `:` on a parameter (`grammar.md` §4).

```aura
class Comparable {
  public def compare(other: Comparable) -> int { return 0 }
}

def id[T](x: T) -> T { return x }
def smallest[T: Comparable](items: [T]) -> T { return items[0] }
def map[T, R](f: (T) -> R, items: [T]) -> [R] { return [f(i) for i in items] }
```

- **UNSPECIFIED:** type parameters are erased at transpile time — the emitted
  Python function is untyped (`transform_FunctionDecl`,
  `statements.py:319-391`), so `id(7)` needs no explicit `[int]`.
- Constraints are checked for resolution elsewhere (see `TYPES.md`), not here.

*Evidence:* *probe* — `def id[T](x: T) -> T { return x }` transpiles to
`def id(x): return x`.

---

## 5. Recursion

Direct recursion works; the call resolves against the enclosing function.

```aura
def factorial(n) -> int {
  if n <= 1 { return 1 }
  return n * factorial(n - 1)
}

factorial(5)     // 120
```

*Evidence:* *probe* (120). **UNSPECIFIED:** no tail-call optimization is
guaranteed; deep recursion is bounded by the CPython stack.

---

## 6. Nesting

A function declared inside another function body is **hoisted**: the inner `def`
is emitted at the point of declaration inside the enclosing Python function.

```aura
def main() {
  def helper(x: int) -> int { return x + 1 }
  print(helper(2))     // 3
}
```

*Evidence:* *probe* (3), `test_comprehensive.py::test_nested_function`.

---

## 7. Where functions live / visibility

- **Top-level** functions emit module-level Python `def`s.
- Inside a **class**, `transform_FunctionDecl` mangles a `private` method to
  `__name` and a `protected` one to `_name` (`statements.py:331-333`).
  Visibility rules and member uniqueness are documented in
  [classes.md](classes.md).
- `@staticmethod` is emitted for `is_static` (`statements.py:322-323`).

There is **no overloading** at the language level: a repeated name in the same
class body is **E301** (see `LANGUAGE.md` §8 "Unique members"). Top-level
functions with the same name would silently overwrite in Python.

---

## 8. Decorators

```
decorator      = "@" , dotted_name , [ "(" , [ arg_list ] , ")" ] ;
```

A decorator on a `def` is allowed; a decorator on a **field** is rejected
(`E320`, `LANGUAGE.md` §17). Built-in runtime macros include `@debug`, `@timeit`,
`@memoize`, `@cache(maxsize=…)`, `@must_return` and `@deprecated(…)`;
`@property`, `@staticmethod` and `@classmethod` are class-member decorators.

```aura
@memoize
def fib(n) -> int {
  if n < 2 { return n }
  return fib(n - 1) + fib(n - 2)
}

@timeit
def slow() { }
```

*Evidence:* `test_syntax_complete.py::test_decorator`,
`::test_builtin_macros_present`.

---

## 9. Entry point (`main`)

An **entry file** (run with `aura run`) must declare a top-level `main`; the
runtime calls it. A file imported as a module needs no `main`.

- Missing `main` in an executed file → **E310**.
- A `main` with any other signature → **E311** (`main` takes no parameters, or a
  single `args` parameter).
- A `main` inside a `module` body → **E312**.

```aura
def main() {
  print("hello")
}
```

A `main` that receives the command-line arguments:

```aura
def main(args: [string]) {
  print(f"{args.length()} argument(s)")
}
```

`main` may return an `int` to set the exit code, and may be `async`.

*Evidence:* `rules.py:_check_main`, `LANGUAGE.md` §1.
