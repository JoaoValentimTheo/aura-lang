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
| E1005 | Unterminated multiline comment | `<!-- never closed` |
| E1006 | Expected token | `let = 1`, `fn ()`, `int \| 5`; a `const` name not starting with an uppercase letter (`const pi = 3`) |
| E1009 | Reserved word or builtin name as a value-namespace name | `let if = 1`; `let sum = 1` (`sum` is a builtin) |
| E1015 | Expression/statement/type nests too deeply | a 5000-term expression; a 257-level nested type annotation (ADR-0004); an exponentially expanding alias chain |

## Name and rule checking (`E2xxx`, static)

| Code  | Meaning | Example trigger |
|-------|---------|-----------------|
| E2001 | Assign to immutable | `let x = 1; x = 2`; mutating through an immutable binding (`let xs = [1]; xs.push(2)`, `xs[0] = 9`, `s.f = 1`); a `mut self` method called on an immutable receiver |
| E2003 | Undefined name or function | `print(nope)`, `nope()` |
| E2005 | `let` without initializer | `let x` |
| E2007 | Redeclaration | `const X = 1; const X = 2`; two `main`s; a duplicate function, parameter, or top-level `let` (an ordinary `let`/`let mut` shadows instead, §16.3) |
| E2009 | `_param` was used | `fn f(_x) { return _x }` |
| E2010 | Invalid assignment target | `1 = 2` |
| E2011 | Invalid `main` | `fn main(x) { }` |
| E2012 | Duplicate user type | two `struct S` |
| E2013 | Duplicate enum variant tag | `enum A { X }` + `enum B { X }` |
| E2014 | Duplicate pattern binding | `match xs { [a, a] -> ... }` |
| E2015 | `break`/`continue` outside a loop | `fn main() { break }` |
| E2016 | Duplicate struct field | `struct S { x: int, x: string }` |
| E2017 | Trait implementation missing a required method | `trait T { fn a(self); fn b(self) }` with `impl T for S` providing only `a` |
| E2018 | Private item accessed across a module boundary | `module m { fn hidden() { } }` then `m::hidden()` |
| E2019 | Unknown module, or unknown item in a `use` path | `use nope`; `use shapes::Missing` |
| E2020 | Module-source ownership collision | two provider sources both claim logical module `foo`; or in-source `module foo` plus external `foo` |
| E2021 | Duplicate logical source | one provider source key is supplied for two logical modules, or supplied twice for one logical module |
| E2022 | Invalid/missing module-source path or provider key | provider advertises a missing child source; invalid logical child name such as `../foo` |

## Type-level (`E3xxx`, static)

| Code  | Meaning | Example trigger |
|-------|---------|-----------------|
| E3001 | Type mismatch | `let x: int = "a"`; `{int: string}`; bitwise/shift on a non-integer (`1.0 & 2`); a format type on an incompatible value (`f"{'s':d}"`) |
| E3002 | Unknown type / constructor | `-> Widget`, `Ghost { }` |
| E3005 | Return type mismatch | `-> int` returning a string |

## Runtime (`E4xxx`)

| Code  | Meaning | Example trigger |
|-------|---------|-----------------|
| E4007 | Division by zero | `1 / 0`, `1 % 0`, `1.0 / 0.0` |
| E4011 | Call depth limit | more than 512 active calls |
| E4013 | Integer overflow | `i64::MAX + 1`, `i64::MIN % -1`; an out-of-range shift count (`1 << 64`, `1 << -1`); a Python integer beyond `i64` via `py_eval` |
| E4018 | Value is not iterable | `for x in 1 {}` |
| E4019 | Index out of range | `[1][5]`, a huge negative index |
| E4020 | I/O operation failed | `read_file`/`write_file` failure, standard-input read failure |
| E4026 | Uncaught thrown value | `throw "x"` with no `catch` |
| E4027 | Missing `main` | `aura run` on a file without `fn main` |
| E4028 | Assertion failed | `assert(1 == 2)` |
| E4029 | No `match` arm matched | `match 5 { 1 -> "a" }` |
| E4030 | `return` cannot be used as a value | `let x = if true { return 1 } else { 2 }` |
| E4999 | Internal error | a bug in Aura itself, never a user mistake |

## Optional features (`E5xxx`)

| Code  | Meaning | Example trigger |
|-------|---------|-----------------|
| E5001 | Python error | `py_eval("1 / 0")` |
| E5002 | Capability unavailable (Python bridge, or a host capability) | `py_eval(...)` without the `py` feature; a non-string Python dict key; `time_unix`/`sleep_ms`/`read_file` in a host that does not provide the capability |

## Stability

Codes are part of the public contract. A code is never reused for a
different meaning; new diagnostics get new numbers. `tests/grammar.rs`
asserts that every code in this table can be produced by at least one
program and that `src/error.rs` agrees.
