# Resource limits

Aura bounds resource use so that a hostile or runaway program produces a
language diagnostic rather than crashing its host. The limits below are part of
the runtime model.

| Limit | Value | Exceeding it |
|---|---|---|
| AST nesting depth | 256 | `E1015` |
| Type-annotation nesting depth | 256 | `E1015` |
| Call-frame limit (recursion) | 512 | `E4011` |
| Range materialization | 10,000,000 elements | `E4013` |
| Runtime value display/JSON depth | 512 | truncated (display) / `null` (JSON) |
| Runtime value display/JSON nodes | 1,000,000 | truncated (display) / `null` (JSON) |
| Parser recursion backstop | substrate-calibrated | `E1015` |

## AST nesting (E1015)

The checker and evaluator bound how deeply expressions and statements may nest.
A program at the limit is valid; beyond it is `E1015`, reported before
execution.

Structural nesting of a type annotation — a generic application (`Box<…>`), a
list (`[T]`), or a map (`{K: V}`) — counts toward the same 256-level budget,
uniformly on every substrate. A *flat union* (`A | B | …`) lists alternatives
rather than nesting and is not penalized per member, so a long union is still
accepted.

## Call frames (E4011)

Recursion is bounded by the number of simultaneously active user calls,
including the entry call to `main`. More than 512 active calls is `E4011`. This
is a language rule, not a host-stack limitation: the native runtime runs on a
dedicated 64 MiB stack sized so this limit is reached first.

**Fixed in the `0.3.1` WebAssembly runtime.** The `0.0.2`–`0.2.1` browser
runtimes executed inline on the JavaScript engine stack, whose size is not under
the artifact's control; on that substrate a mainstream recursive program could
exhaust the engine stack *below* the 512-frame limit and surface as a Worker
error (`E4999: Maximum call stack size exceeded`) instead of `E4011`. This was
an implementation defect in the WebAssembly runtime (tracked as B-1), never a
change to the 512-frame contract. The `0.3.1` Keystone runtime runs the
explicit-continuation evaluator and reports `E4011` at the language boundary,
matching native; the released `0.2.1` runtime stays frozen and keeps the old
behavior. See the [known limitations](/docs/known-limitations/).

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
is handled safely rather than recursing forever. Display and JSON encoding are
bounded by both a depth (512) and a total-node budget (1,000,000): the depth
bound alone does not bound work, because a value whose reference fan-out is two
or more — a cycle reachable from more than one position, or a shared subvalue —
would otherwise re-traverse exponentially and never terminate. Past either
bound the remainder is elided.

The node budget is scoped **per rendering operation** (one display or one
`json_encode`), not per program: each render is guaranteed to terminate in
bounded work, while a program that renders in a loop performs that bounded work
per render. The Python bridge applies the same depth-and-nodes discipline when a
value crosses the Aura/Python boundary, and rejects a reference cycle that Aura
cannot represent with a structured diagnostic.

## Fuzzing and the permanent corpus

The resource guarantees above are continuously checked by four libFuzzer
targets (`fuzz/`) and by the permanent fixture corpus (`tests/corpus/`), which
runs in CI. The sanitizer scope is explicit: LeakSanitizer runs on the lexer,
parser, and checker targets, and is scoped off only for the runtime target,
where Aura's documented, memory-safe `Rc` model legitimately does not reclaim a
reference cycle.
