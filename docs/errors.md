# Aura diagnostic codes

Every rejection carries a stable code. Codes are grouped by phase:

* `E1xxx` — lexical and syntactic.
* `E2xxx` — name and rule checking (static).
* `E3xxx` — type-level (static annotations).
* `E4xxx` — runtime.
* `E5xxx` — optional features / Python bridge.

## Lexical and syntactic (`E1xxx`)

| Code  | Meaning | Example trigger |
|-------|---------|-----------------|
| E1001 | Invalid character | `a && b`, `!x`, `&` |
| E1002 | Malformed number | `1abc`, `0xZZ`, out-of-range literal |
| E1003 | Invalid escape | `"\q"` |
| E1004 | Unterminated string | `"abc` |
| E1006 | Expected token | `let = 1`, `fn ()`, `a \| b` |
| E1009 | Reserved word as name | `let if = 1` |
| E1014 | `else if` used | `if a {} else if b {}` |
| E1015 | Expression/statement nests too deeply | a 5000-term expression |

## Name and rule checking (`E2xxx`, static)

| Code  | Meaning | Example trigger |
|-------|---------|-----------------|
| E2001 | Assign to immutable | `let x = 1; x = 2` |
| E2003 | Undefined name or function | `print(nope)`, `nope()` |
| E2005 | `let` without initializer | `let x` |
| E2007 | Redeclaration | `let x = 1; let x = 2`; two `main`s |
| E2009 | `_param` was used | `fn f(_x) { return _x }` |
| E2010 | Invalid assignment target | `1 = 2` |
| E2011 | Invalid `main` | `fn main(x) { }` |
| E2012 | Duplicate user type | two `struct S` |
| E2013 | Duplicate enum variant tag | `enum A { X }` + `enum B { X }` |
| E2014 | Duplicate pattern binding | `match xs { [a, a] -> ... }` |
| E2015 | `break`/`continue` outside a loop | `fn main() { break }` |

## Type-level (`E3xxx`, static)

| Code  | Meaning | Example trigger |
|-------|---------|-----------------|
| E3001 | Type mismatch | `let x: int = "a"`; `{int: string}` |
| E3002 | Unknown type / constructor | `-> Widget`, `Ghost { }` |
| E3005 | Return type mismatch | `-> int` returning a string |

## Runtime (`E4xxx`)

| Code  | Meaning | Example trigger |
|-------|---------|-----------------|
| E4007 | Division by zero | `1 / 0`, `1 % 0`, `1.0 / 0.0` |
| E4011 | Call depth limit | more than 512 active calls |
| E4013 | Integer overflow | `i64::MAX + 1`, `i64::MIN % -1`; a Python integer beyond `i64` via `py_eval` |
| E4018 | Value is not iterable | `for x in 1 {}` |
| E4019 | Index out of range | `[1][5]`, a huge negative index |
| E4026 | Uncaught thrown value | `throw "x"` with no `catch` |
| E4027 | Missing `main` | `aura run` on a file without `fn main` |
| E4028 | Assertion failed | `assert(1 == 2)` |
| E4029 | No `match` arm matched | `match 5 { 1 -> "a" }` |
| E4030 | `return` cannot be used as a value | `let x = return 1` |
| E4999 | Internal error | a bug in Aura itself, never a user mistake |

## Optional features (`E5xxx`)

| Code  | Meaning | Example trigger |
|-------|---------|-----------------|
| E5001 | Python error | `py_eval("1 / 0")` |
| E5002 | Python bridge unavailable or value cannot cross | `py_eval(...)` without the `py` feature; a non-string Python dict key |

## Stability

Codes are part of the public contract. A code is never reused for a
different meaning; new diagnostics get new numbers. `tests/grammar.rs`
asserts that every code in this table can be produced by at least one
program and that `src/error.rs` agrees.
