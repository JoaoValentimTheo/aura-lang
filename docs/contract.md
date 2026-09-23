# Aura v3 — Language Contract

This document is **normative**. Every rule here is enforced by an automated
test (`tests/contract.rs`). If the implementation and this file disagree, the
implementation is wrong. If a rule changes, the RFC process in
`CONTRIBUTING.md` applies.

## 0. Design principles

1. **One spelling per construct.** A synonym is a bug. `and` exists; `&&`
   does not. `fn` exists; `def`/`function`/`func` do not.
2. **Mutable means mutable, visibly.** A binding is immutable unless it is
   declared `let mut`. Reading is free; writing is explicit.
3. **Absence is a value, not an exception.** `none` is a first-class value.
   There is no `null`, `nil`, or `undefined`.
4. **No silent coercion.** `1 + "1"` is a compile error. `to_string(1)` is
   how you ask for a string.
5. **Runtime is native Rust.** The compiler and runtime are one Rust binary.
   Python is optional and lives behind an explicit `py.` namespace.
6. **Diagnose everything.** Every rejection has a stable code `E####`, an
   exact location, and a message that names the offending construct.

## 1. Source and lexical rules

* Source is UTF-8. Identifiers are `[A-Za-z_][A-Za-z0-9_]*`.
* Line comments start with `#`. There are no block comments.
* A newline or `;` terminates a statement.
* Literals: decimal `123`, hex `0xff`, binary `0b1011`, float `1.5`, `1e9`;
  strings `"abc"` and `'abc'` (interchangeable); f-strings `f"x = {x}"`.
* `1abc` is an error, never three tokens.
* Reserved words may not be used as names.

## 2. Types

The primitive types are `int`, `float`, `string`, `bool`.
The composite types are:

| Syntax          | Meaning                                  |
|-----------------|------------------------------------------|
| `[T]`           | list of `T`                              |
| `{K: V}`        | map from `K` to `V`                      |
| `Name`          | user type declared with `struct`/`enum`  |
| `T | none`      | optional `T`                             |

Types are optional annotations. When present they are checked before
execution. There are no implicit conversions between `int` and `string`.

## 3. Declarations

```
let name = expr              # immutable binding
let mut name = expr          # reassignable binding
let name: int = 3            # annotated
fn add(a: int, b: int) -> int { return a + b }
struct Point { x: float, y: float }
enum Result { Ok(string), Err(string) }
type UserId = int
use stdlib.math              # reserved; currently inert (see §10)
```

* Every binding requires an initializer. `let x` alone is `E2005`.
* At the top level, `let` declares an immutable module constant; `let mut`
  is rejected there.
* A function is declared with `fn` and returns `none` unless annotated.
* Redefining a name in the same scope is `E2007`.
* Parameter names starting with `_` must be unused (`E2009`).

## 4. Expressions

Precedence, lowest to highest:

```
1  x |> f                    pipeline
2  or
3  and
4  == !=
5  < <= > >=
6  + -
7  * / %
8  ^                         right associative
9  -x  not x                 unary
10 a.b  a.b()  a[b]  f(x)    postfix
11 literals, names, groups, lists, maps, lambdas
```

* Lambdas: `(x) -> x * x` or `x -> x * x`.
* `if` is an expression and **requires** `else` when it yields a value:
  `let m = if a > b { a } else { b }`.
* `match` is an expression; every arm must yield when used as a value.
* There is no ternary `? :`, no `&&`, no `||`, no `!` as `not`.
* `to_string`, `to_int`, `to_float` are conversions; there is no cast syntax.

## 5. Statements

```
if cond { } else { }
match value { 1 -> "one"; _ -> "other" }
while cond { }
loop { break }
for x in items { }
return expr
break / continue
try { } catch e -> { } finally { }
throw expr
```

* `else if` is **not** allowed; use nested braces or `match`.
* `for` iterates lists, strings (characters), maps (keys), and `range(...)`.

## 6. Rules enforced at check time

The checker runs before execution and rejects, at minimum:

| Code  | Rule |
|-------|------|
| E1001 | invalid character |
| E1002 | malformed number (including `1abc`) |
| E1004 | unterminated string |
| E1006 | expected token |
| E1014 | `else if` used |
| E2001 | assignment to immutable binding |
| E2003 | undefined name or function |
| E2005 | `let` without initializer |
| E2007 | redeclaration in the same scope |
| E2009 | `_param` was used |
| E2010 | invalid assignment target |
| E2011 | `main` with parameters |
| E2012 | duplicate user type |
| E2013 | duplicate enum variant tag across the program |
| E2014 | a pattern binds the same name twice |
| E3001 | type mismatch (annotations are checked) |
| E3002 | unknown type or constructor |
| E3005 | return type mismatch |

The full, authoritative list lives in `docs/errors.md`, and
`tests/grammar.rs` asserts that every code there is reachable.

## 7. Runtime model

* `int` is 64-bit and checked: overflow is `E4013`, not wraparound.
* Division by zero is `E4007`.
* `==` compares structurally (lists, maps, structs, enums).
* `print(x)` writes `x.to_string()` plus a newline.
* `try/catch` catches **only** an explicit `throw` value (including one
  thrown inside a called function). Runtime diagnostics such as division by
  zero, overflow, or an out-of-range index are fatal and are *not* catchable.
  An uncaught `throw` is `E4026`.
* Recursion is capped at **512 simultaneously active call frames**,
  including the entry call to `main`; exceeding this is `E4011`. This is a
  language rule, not a host limitation.

## 8. Python interop (feature `py`)

The Python boundary is a set of explicit functions:

* `py_eval(code)` evaluates a Python expression and converts the result.
* `py_import(name)` imports a Python module and lists its public names.
* `py_call(module, attr, args...)` calls an attribute of a module.
* `py_version()` returns the Python version string.

Values cross structurally: `int`, `float`, `bool`, `string`, `none`, lists,
and maps (`dict`) convert in both directions. Opaque objects become their
`repr` string.

Without the `py` feature the same names exist but every call is `E5002`, and
the binary links no CPython.

## 9. Entry point

A runnable file declares `fn main()` with no parameters. `aura run` loads all
declarations, evaluates top-level constants and expressions in source order,
then calls `main`. A missing `main` is `E4027`; a `main` with parameters is
`E2011`.

## 10. Reserved syntax

`use` and `pub` are **parsed and reserved but have no effect** in this
version. They exist so that future module and visibility semantics can be
introduced without a syntax break. Using them is not an error; they simply do
nothing. Nothing in the language depends on them.

`type Name = T` declares a transparent alias. It is validated (the target
type must exist) but does not create a distinct nominal type.
