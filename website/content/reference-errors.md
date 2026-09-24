# Diagnostics

Every rejection carries a stable code, grouped by phase:

* `E1xxx` — lexical and syntactic.
* `E2xxx` — name and rule checking (static).
* `E3xxx` — type-level (static annotations).
* `E4xxx` — runtime.
* `E5xxx` — optional features / capabilities.

Codes are part of the public contract: a code is never reused for a different
meaning. The repository's `tests/grammar.rs` asserts that every code below can
be produced by at least one program.

## Lexical and syntactic

| Code | Meaning | Example trigger |
|---|---|---|
| E1001 | Invalid character | `a && b`, `!x`, `&` |
| E1002 | Malformed number | `1abc`, `0xZZ` |
| E1003 | Invalid escape | `"\q"` |
| E1004 | Unterminated string | `"abc` |
| E1006 | Expected token | `let = 1`, `fn ()` |
| E1009 | Reserved word as name | `let if = 1` |
| E1015 | Expression/statement nests too deeply | a 5000-term expression |

## Name and rule checking

| Code | Meaning | Example trigger |
|---|---|---|
| E2001 | Assign to immutable | `let x = 1` then `x = 2` |
| E2003 | Undefined name or function | `print(nope)` |
| E2005 | `let` without initializer | `let x` |
| E2007 | Redeclaration | `let x = 1` twice |
| E2009 | `_param` was used | `fn f(_x) { return _x }` |
| E2010 | Invalid assignment target | `1 = 2` |
| E2011 | Invalid `main` | `fn main(x) { }` |
| E2012 | Duplicate user type | two `struct S` |
| E2013 | Duplicate enum variant tag | `X` in two enums |
| E2014 | Duplicate pattern binding | `match xs { [a, a] -> ... }` |
| E2015 | `break`/`continue` outside a loop | `fn main() { break }` |

## Type-level

| Code | Meaning | Example trigger |
|---|---|---|
| E3001 | Type mismatch | `let x: int = "a"` |
| E3002 | Unknown type / constructor | `-> Widget`, `Ghost { }` |
| E3005 | Return type mismatch | `-> int` returning a string |

## Runtime

| Code | Meaning | Example trigger |
|---|---|---|
| E4007 | Division by zero | `1 / 0`, `1 % 0`, `1.0 / 0.0` |
| E4011 | Call depth limit | more than 512 active calls |
| E4013 | Integer overflow | `i64::MAX + 1` |
| E4018 | Value is not iterable | `for x in 1 {}` |
| E4019 | Index out of range | `[1][5]` |
| E4020 | I/O operation failed | `read_file`/`write_file`/stdin failure |
| E4026 | Uncaught thrown value | `throw "x"` with no `catch` |
| E4027 | Missing `main` | `aura run` on a file without `fn main` |
| E4028 | Assertion failed | `assert(1 == 2)` |
| E4029 | No `match` arm matched | `match 5 { 1 -> "a" }` |
| E4030 | `return` cannot be used as a value | `let x = if true { return 1 } else { 2 }` |
| E4999 | Internal error | a bug in Aura itself, never a user mistake |

## Optional features and capabilities

| Code | Meaning | Example trigger |
|---|---|---|
| E5001 | Python error | `py_eval("1 / 0")` |
| E5002 | Capability unavailable | `py_eval(...)` without the `py` feature; `time_unix`, `sleep_ms`, or `read_file` in a host that does not provide the capability |

## Catchability

Only an explicit `throw` is catchable. Every runtime diagnostic above is
**fatal** and propagates through `try/catch`; a `finally` block still runs.
`E5001` (a Python error) and `E5002` (a missing capability) are runtime
diagnostics and are likewise not catchable.
