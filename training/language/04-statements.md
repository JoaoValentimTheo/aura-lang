---
title: "04 — Statements"
---

# Statements

A statement executes an effect and, with exceptions of expression-bodied `if`/`match`/`try`, **does not produce a value**. Semicolons are optional; newlines are not significant.

See `aura/parser/to_ast.py:750-946` (`parse_statement`), `aura/transpiler/transformers/statements.py`.

## Blocks

```aura
def main() {
  {
    let inner = 1
    print(inner)
  }
  // inner is out of scope here
}
```

A block `{ ... }` introduces a new scope. Declarations inside a block do not leak out. An empty `{}` in expression position is an empty dict literal, not an empty block.

## `if` / `else if` / `else`

```aura
let n = 2
if n == 1 { print("one") }
else if n == 2 { print("two") }
else { print("other") }
```

Braces are mandatory. `else if` chains to any depth. `elif` / `elsif` are rejected. `if` in expression position produces a value: `let x = if n > 0 { n } else { -n }`.

## `unless` (Inverted `if`)

`unless cond { A }` runs `A` when `cond` is falsy. Transpiles to `if not (cond): ...`. `else` chains like `if`.

## `guard cond else { ... }`

```aura
def process(data) {
  guard data != none else {
    print("No data"); return
  }
  print(data)          // data is non-none here
}
```

Runs the `else` block when `cond` is **falsy**, then continues. At module scope, a bare `return` in the guard body exits the program.

## Loops

```aura
let mut i = 0
while i < 3 { print(i); i += 1 }
until i >= 3 { print(i); i += 1 }     // inverted while
loop { if read() == "quit" { break } } // infinite

for item in items { print(item) }
for (k, v) in pairs { print(k) }
for i in 0..<10 step 2 { print(i) }
```

Labels target enclosing loops: `outer: for i in ... { break outer }`. `break` / `continue` outside a loop is rejected (E004).

## `match` (Statement)

```aura
match status {
  case 0 { print("inactive") }
  case n if n > 100 { print("overflow") }
  case _ { print("unknown") }
}
```

A case body is a **block** `{ ... }` or an **arrow** `-> expression`. No fallthrough. Patterns: wildcard `_`, literal, binding, guard, or (`1 | 2`), destructuring (`[a, b]`), enum (`Color.RED`). A guarded `case _ if cond` does not count as a catch-all.

## `try` / `catch` / `finally`

```aura
try {
  process_file("data.txt")
} catch IOError as e {
  print("File not found")
} finally {
  print("Cleanup complete")
}
```

`try` requires at least one `catch` or a `finally`. Four catch spellings: `catch { }`, `catch Type { }`, `catch Type as e { }`, `catch as e { }`. A lone identifier before `{` is always a type, never a binding.

## `throw`

```aura
throw ValueError("bad")
throw "msg"        // wrapped: raise Exception("msg")
```

A string literal is wrapped in `Exception()`. A non-string, non-exception expression is emitted as-is and may fail at runtime.

## `return`

```aura
def swap(a, b) { return b, a }   // returns tuple
return              // bare, no value
```

Comma-separated values become a tuple. `return` outside a function is rejected (E004), except inside a top-level `guard`.

## `assert` and `with`

```aura
assert 1 + 1 == 2, "math is broken"   // AssertionError on failure

with open("x") as fh { print(fh) }
async with fetch(url) as resp { await resp.text() }
```

## Rules Enforced by the Checker

| Rule | Code |
|------|------|
| `return` outside a function | E004 |
| `break` / `continue` outside a loop | E004 |
| Unknown / non-enclosing label | E318 |
| Unreachable code after `return` / `throw` | E302 |
| Local used before its `let` / `const` | E319 |
| `await` outside `async` | E004 |

## Gotcha

- `break` followed by `case` on the next line parses as `break case` — use a block body or trailing `;`.
- `match` needs a catch-all for finite domains, or E109 warns.
- `async with` outside `async def` is not enforced at check time; it fails only if the target Python is invalid.

## Anti-pattern

Do not use `loop` with `break` to simulate `while true`. Use `loop` for infinite loops with mid-body exits; use `while condition { ... }` for condition-driven loops.
