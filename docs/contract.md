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
use stdlib.math            # bring a module into scope
```

* Every binding requires an initializer. `let x` alone is `E2005`.
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

| Code  | Rule |
|-------|------|
| E1001 | invalid character |
| E1002 | malformed number (including `1abc`) |
| E1004 | unterminated string |
| E1006 | expected token |
| E1014 | `else if` used |
| E2001 | assignment to immutable binding |
| E2003 | undefined name |
| E2005 | `let` without initializer |
| E2007 | redeclaration in the same scope |
| E2009 | `_param` was used |
| E2010 | invalid assignment target |
| E3001 | type mismatch |
| E3005 | return type mismatch |
| E4018 | value is not iterable |

The full list lives in `docs/errors.md`.

## 7. Runtime model

* `int` is 64-bit and checked: overflow is `E4013`, not wraparound.
* Division by zero is `E4007`.
* `==` compares structurally (lists, maps, records, enums).
* `print(x)` writes `x.to_string()` plus a newline.
* A `throw` value is caught by `try/catch`; uncaught, it is `E4026`.
* Recursion is capped at 512 frames (`E4011`).

## 8. Python interop (feature `py`)

With the `py` feature, `use py.<module>` exposes Python modules; attribute
access and calls cross the boundary with structural conversion. Without the
feature, `use py.*` is `E5002` and the binary links no CPython at all.

## 9. Entry point

A runnable file declares `fn main()`. `aura run` calls `main` after loading
all top-level declarations. Missing `main` is `E4027`.
