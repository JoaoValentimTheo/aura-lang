# Errors

Aura separates two very different things:

1. **Throwables** — values raised explicitly with `throw`, catchable with
   `catch`.
2. **Runtime diagnostics** — language failures such as division by zero,
   overflow, or an out-of-range index. These are **fatal and not catchable**.

This distinction is deliberate. A missing file is represented as the value
`none`, while a genuine environment failure is a fatal diagnostic — the code you
write rarely has to wrap ordinary “not found” cases in exception handling.

## `throw`, `catch`, `finally`

```aura
fn safe_div(a, b) -> int {
    if b == 0 {
        throw "division by zero"
    }
    return a / b
}

fn main() {
    try {
        print(safe_div(10, 2))
        print(safe_div(1, 0))
    } catch e {
        print(f"caught: {e}")
    } finally {
        print("done")
    }
}
```

* `try` requires `catch`; there is no `try` without it.
* `finally`, if present, runs once on **every** exit path: normal completion,
  `return`, `break`, `continue`, `throw`, and a fatal runtime error.
* If `finally` itself raises a control-flow signal, that signal replaces the
  pending outcome.

## Selecting a catch

The clause after `catch` is a **full pattern** (§4.7), exactly as in a `match`
arm. This lets one `try` dispatch on the thrown value:

```aura
enum Failure { BadInput(string), Timeout }

try {
    throw BadInput("missing field")
} catch BadInput(msg) {
    print(f"bad input: {msg}")
} catch Timeout {
    print("timed out")
} catch e {
    print(f"other: {e}")
}
```

`catch e` binds any thrown value; `catch _` catches with no binding;
`catch E::V(x)` selects one nominal variant; `catch none` selects a thrown
`none`; and `catch 7` selects that literal. When a catch pattern does **not**
match, the throw continues to the next enclosing `try` (or terminates with
`E4026`) — it is never silently discarded. The built-in exception namespace
root `Aura` is reserved (`E2023`), so a user module cannot counterfeit built-in
exception identity.

## An uncaught throwable

If a `throw` reaches the top with no matching `catch`, the program terminates
with `E4026` (uncaught thrown value).

## Runtime diagnostics are not catchable

```aura
try {
    print(1 / 0)     # E4007 — not caught; the program terminates
} catch e {
    print("never reached")
}
```

Division by zero, integer overflow, index-out-of-range, and similar failures
propagate through `try/catch`. `finally` still runs.

## The I/O model

`read_file(path)` returns the file text, or `none` when the path does not exist.
A genuine failure — a directory, a permission error, invalid UTF-8 — is `E4020`.
`read_line()` returns `none` at end of input. See
[I/O and arguments](/docs/guide-io/).

## Diagnostic codes

Every rejection carries a stable `E####` code. The
[diagnostics reference](/docs/reference-errors/) lists them all, and the codes
are part of the public contract: a code is never reused for a different meaning.
