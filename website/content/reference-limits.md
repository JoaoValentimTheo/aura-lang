# Resource limits

Aura bounds resource use so that a hostile or runaway program produces a
language diagnostic rather than crashing its host. The limits below are part of
the runtime model.

| Limit | Value | Exceeding it |
|---|---|---|
| AST nesting depth | 256 | `E1015` |
| Call-frame limit (recursion) | 512 | `E4011` |
| Range materialization | 10,000,000 elements | `E4013` |
| Runtime value display/JSON depth | 512 | truncated (display) / `null` (JSON) |
| Parser recursion backstop | substrate-calibrated | `E1015` |

## AST nesting (E1015)

The checker and evaluator bound how deeply expressions and statements may nest.
A program at the limit is valid; beyond it is `E1015`, reported before
execution.

## Call frames (E4011)

Recursion is bounded by the number of simultaneously active user calls,
including the entry call to `main`. More than 512 active calls is `E4011`. This
is a language rule, not a host-stack limitation: the native runtime runs on a
dedicated 64 MiB stack sized so this limit is reached first.

## Range materialization

Iterating a range lazily never materializes it, so an immediate `break` on a
huge range is safe. Materializing a range elsewhere is capped at 10,000,000
elements (`E4013`).

## Runtime value depth

Displaying or JSON-encoding a deeply nested or cyclic value is bounded at depth
512. A cyclic value is detected and safe to display; it never causes a stack
overflow or a crash.

## The parser backstop

The parser has an implementation-safety recursion backstop for grouping
(parentheses) that adds no AST depth. It is calibrated per substrate: native
runs the parser on a large stack, while WebAssembly runs it inline on the engine
stack. Over-limit input is always `E1015`, never a host failure. The normative
rule is that a program valid under the semantic AST limit (256) **must** be
accepted on every execution substrate.

## Value equality and cycles

Equality and display detect reference cycles, so a self-referential list or map
is handled safely rather than recursing forever.
