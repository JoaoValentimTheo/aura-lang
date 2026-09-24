# Aura v3 — Language Contract

This document is the **high-level compatibility contract**. The **normative
semantic specification** is [`LANGUAGE_SPEC.md`](LANGUAGE_SPEC.md): it defines
what Aura programs mean. This contract restates the guarantees a user can rely
on and names the tests that enforce them. If the two disagree, `LANGUAGE_SPEC.md`
is authoritative for semantics and this file is out of date.

Every rule here is enforced by an automated test (`tests/contract.rs`). If a
rule changes, the RFC process in `CONTRIBUTING.md` applies.

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
A `type Name = T` alias is transparent: `Name` denotes `T` in every position.
An annotation is enforced where the checker can prove a mismatch: annotated
bindings, function return types, function parameters used in a body, struct
field values at construction, and enum payload values at construction. A value
the checker cannot type is not rejected (it may still fail at runtime under the
runtime rules in §7).

## 3. Declarations

```
let name = expr              # immutable binding
let mut name = expr          # reassignable binding
let name: int = 3            # annotated
let [a, b] = pair            # destructuring: binds each name (immutably)
let Ok(x) = result          # variant destructuring
fn add(a: int, b: int) -> int { return a + b }
struct Point { x: float, y: float }
enum Result { Ok(string), Err(string) }
type UserId = int
use stdlib.math              # reserved; currently inert (see §10)
```

* Every binding requires an initializer. `let x` alone is `E2005`.
* A `let` may bind a pattern: an identifier, or a (possibly nested) list or
  variant pattern. Every name in the pattern is bound immutably from the
  value. Only identifiers, list patterns, and variant patterns are allowed;
  a literal or `none` pattern in `let` is `E1006`, as is an annotation or
  `mut` on a non-identifier pattern. A destructuring `let` is atomic: if the
  value does not match the pattern the statement produces the existing
  `E3001` runtime diagnostic and binds nothing (see §6).
* At the top level, `let` declares an immutable module constant; `let mut`
  is rejected there.
* A function is declared with `fn` and returns `none` unless annotated.
* Redefining a name in the same scope is `E2007`.
* Parameter names starting with `_` must be unused (`E2009`).
* A struct is built with named fields (`S { a: 1 }`) or positionally in
  declaration order (`S(1)`). Named construction must supply every declared
  field exactly once; an unknown field is `E2003`, and a missing, duplicate,
  extra, or wrong-typed field is `E3001`. A field value the checker cannot type
  is accepted.
* An enum variant is built positionally (`Ok(x)`); named payload arguments are
  rejected (`E3001`). The payload count and, where statically known, each
  payload value's type must match the declaration (`E3001`).

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

* Lambdas: `(x) -> x * x` or `x -> x * x`. A block-bodied lambda
  `(x) -> { ... }` uses the block as its body, so `return` works and the last
  expression is the value.
* `if` is an expression. With an `else` branch it yields that branch's
  value; without `else` it yields `none` when the condition is false:
  `let m = if a > b { a } else { b }`.
* Pipeline passes the left operand as the **first argument**:
  `x |> f` is `f(x)`, `x |> f(a)` is `f(x, a)`, and `x |> r.m(a)` is
  `r.m(x, a)`. A bare callable `x |> c` is `c(x)`.
* `match` is an expression; every arm must yield when used as a value.
* There is no ternary `? :`, no `&&`, no `||`, no `!` as `not`.
* Map literals are string-keyed and ordered by key: `{"a": 1}`. `{:}` is the
  empty-map literal. `{}` is a **block**, not an empty map, and yields `none`;
  whitespace and newlines do not change this (`{ : }` is the same as `{:}`).
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

* `else if` is allowed: `if A { X } else if B { Y } else { Z }` is the nested
  form `if A { X } else { if B { Y } else { Z } }`. Each condition is evaluated
  at most once, in source order.
* `else`, `catch`, and `finally` must be written on the same line as the
  closing `}` of the block they follow; a newline before them is `E1006`.
* `for` iterates lists, strings (characters), maps (keys), and `range(...)`.

## 6. Rules enforced at check time

The checker runs before execution and rejects, at minimum:

| Code  | Rule |
|-------|------|
| E1001 | invalid character |
| E1002 | malformed number (including `1abc`) |
| E1004 | unterminated string |
| E1006 | expected token |
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
| E2015 | `break`/`continue` outside a loop |
| E3001 | type mismatch (annotations are checked) |
| E3002 | unknown type or constructor |
| E3005 | return type mismatch |

Beyond annotations, the checker also enforces, when it can prove the types
involved:

* **Calls.** A builtin call is validated against a shared signature table:
  the argument count and the type of each argument are checked (`E3001`), and
  an undefined builtin is `E2003`. The same table drives the runtime, so the
  checker and the evaluator cannot drift.
* **Methods.** A method call on a known receiver type must name a method that
  exists on that type (`E2003`); its argument count and argument types are
  checked (`E3001`).
* **User-function calls.** A call the checker resolves to a specific top-level
  `fn` declaration is checked against its declared parameters (`E3001`): the
  argument count, and each argument's inferred type against the parameter's
  annotation (an `Unknown` argument is not rejected).
* **Named arguments.** A directly resolved top-level function call may supply
  arguments as `name: value`. Positional arguments must precede named ones
  (`E1006` otherwise). A parameter supplied twice, a named argument naming no
  parameter, and a declared parameter left unfilled are each `E3001`. Named
  arguments are rejected on built-ins, methods, and dynamic callables
  (`E3001`). Argument expressions are evaluated in source order, once,
  independent of parameter binding.
* **Ordering.** `<`, `<=`, `>`, `>=` are only valid on number/number,
  string/string, and bool/bool operand pairs. A provably incomparable pair
  (for example `[1] < [2]`) is `E3001` before execution.
* **Return types.** A call to a function with a declared return type infers
  that type, so a mismatch against an annotated binding is caught statically.
* **Field reads.** A field read `s.field` on a receiver whose inferred type is
  a declared struct infers the field's declared (alias-resolved) type, so it is
  checked wherever an annotation applies — annotated bindings, function
  arguments, returns, and construction. A field read on any other receiver
  (an `Unknown` type, a primitive, a list, a map, an enum, or a struct without
  that field) infers `Unknown` and is not rejected. Field validity and field
  type inference are separate: reading a missing field on a known struct is
  still the runtime's decision (`E2003` when writing), unchanged by this rule.
* **Function equality.** Function values compare by identity: a function is
  equal only to itself. Distinct functions are never `==`, and `NaN == NaN`
  is `false`.

The full, authoritative list lives in `docs/errors.md`, and
`tests/grammar.rs` asserts that every code there is reachable.

## 7. Runtime model

* `int` is 64-bit and checked: overflow is `E4013`, not wraparound. The
  literals `-9223372036854775808` (`i64::MIN`) and `i64::MAX` are valid; the
  bare magnitude `9223372036854775808` is not.
* Division by zero is `E4007`: this covers `int` and `float` for both `/` and
  `%`, and treats `0.0` and `-0.0` alike.
* Ordering comparisons on NaN yield `false`; comparing values of
  incomparable types is `E3001`.
* `==` compares structurally (lists, maps, structs, enums). `NaN == NaN` is
  `false`. Functions compare by identity (see §6).
* `print(x)` writes `x.to_string()` plus a newline.
* `try/catch` catches **only** an explicit `throw` value (including one
  thrown inside a called function). Runtime diagnostics such as division by
  zero, overflow, or an out-of-range index are fatal and are *not* catchable.
  An uncaught `throw` is `E4026`.
* `finally` runs on every exit path (normal, `return`, `break`, `continue`,
  `throw`, and a fatal runtime error) exactly once.
* If `finally` itself raises a control-flow signal (`return`, `break`,
  `continue`, or `throw`), that signal **replaces** the pending outcome. For
  example, `try { return 1 } catch e -> { } finally { return 2 }` evaluates to
  `2`, and `try { throw "a" } catch e -> { } finally { throw "b" }` throws
  `"b"`. A `finally` block that runs to completion leaves the pending outcome
  untouched.
* Expressions nest at most **256 AST levels**; deeper nesting is `E1015`,
  never a crash. The limit counts AST nodes (calls, operators, collections,
  and so on), not grouping parentheses: `((((x))))` adds no depth. An
  extreme chain of grouping parentheses is still bounded by the parser's own
  recursion guard, which also reports `E1015`, so the diagnostic is the same
  whether the limit is semantic or a host-safety backstop.
* `to_int` accepts an `int`, a `bool`, a numeric string, or a finite float in
  range; anything else is `E4013`. It never silently saturates.
* Standard-library functions reject a wrong number of arguments (`E3001`)
  and require exact argument types; there is no implicit argument coercion.
* `sort` and `min`/`max` on values that cannot be ordered place the
  incomparable values in source order rather than failing.
* Float formatting is canonical: an integral float prints with one decimal
  (`1.0`), and very large magnitudes print in full digits.
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

`pub` is accepted uniformly on `fn`, `struct`, `enum`, and `type` items and is
inert on all of them; it carries no visibility meaning and is never consulted
by the checker or the runtime. `use` is likewise accepted anywhere an item may
appear and is inert; `use stdlib` and `use a.b.c` are equivalent to a no-op.

`type Name = T` declares a transparent alias. It is validated (the target
type must exist) but does not create a distinct nominal type.
