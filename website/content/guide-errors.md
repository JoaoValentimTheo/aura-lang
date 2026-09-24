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
    } catch e -> {
        print(f"caught: {e}")
    } finally {
        print("done")
    }
}
```

* `try` requires `catch`; there is no `try` without it.
* The thrown value is bound to the catch name.
* `finally`, if present, runs once on **every** exit path: normal completion,
  `return`, `break`, `continue`, `throw`, and a fatal runtime error.
* If `finally` itself raises a control-flow signal, that signal replaces the
  pending outcome.

## An uncaught throwable

If a `throw` reaches the top with no matching `catch`, the program terminates
with `E4026` (uncaught thrown value).

## Runtime diagnostics are not catchable

```aura
try {
    print(1 / 0)     # E4007 — not caught; the program terminates
} catch e -> {
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
