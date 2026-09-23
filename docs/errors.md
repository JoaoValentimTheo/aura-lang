# Aura diagnostic codes

Every rejection carries a stable code. Codes are grouped by phase:

* `E1xxx` — lexical and syntactic.
* `E2xxx` — name and rule checking.
* `E3xxx` — type-level (annotations).
* `E4xxx` — runtime.
* `E5xxx` — Python bridge.

| Code  | Meaning | Example trigger |
|-------|---------|-----------------|
| E1001 | Invalid character | `a && b`, `!x`, `a \| b` |
| E1002 | Malformed number | `1abc`, `0xZZ` |
| E1003 | Invalid escape | `"\q"` |
| E1004 | Unterminated string | `"abc` |
| E1006 | Expected token | `let = 1`, `fn ()` |
| E1014 | `else if` used | `if a {} else if b {}` |
| E1009 | Reserved word as name | `let if = 1` |
| E2001 | Assign to immutable | `let x = 1; x = 2` |
| E2003 | Undefined name | `print(nope)` |
| E2005 | `let` without initializer | `let x` |
| E2007 | Redeclaration | `let x = 1; let x = 2` |
| E2009 | `_param` was used | using an `_`-prefixed parameter |
| E2010 | Invalid assignment target | `1 = 2` |
| E3001 | Type mismatch | `"a" + 1` |
| E3002 | Unknown type | `let p = Nope {}` |
| E3005 | Return type mismatch | `-> int` returning a string |
| E4013 | Integer overflow | `i64::MAX + 1` |
| E4007 | Division by zero | `1 / 0`, `1 % 0` |
| E4011 | Recursion limit | unbounded recursion |
| E4018 | Not iterable | `for x in 1 {}` |
| E4026 | Uncaught thrown value | `throw "x"` with no `catch` |
| E4027 | Missing `main` | running a file without `fn main` |
| E5001 | Python error | `py_eval("1 / 0")` |
| E5002 | Python bridge unavailable | `py_eval` without the `py` feature |

## Stability

Codes are part of the public contract. A code is never reused for a
different meaning; new diagnostics get new numbers. `tests/errors.rs`
asserts that every documented code can be produced and that the table and
`src/error.rs` agree.
