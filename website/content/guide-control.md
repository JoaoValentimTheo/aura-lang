# Control flow

## `if`

`if` is an expression. Without an `else`, a false condition yields `none`.

```aura
let label = if score > 90 { "high" } else { "normal" }
```

`else if` is the nested form `else { if ... }` and is written on one line:

```aura
if n < 0 {
    print("negative")
} else if n == 0 {
    print("zero")
} else {
    print("positive")
}
```

`else`, `catch`, and `finally` must appear on the same line as the closing `}`
of the block they follow; a newline before them is `E1006`.

## Loops

```aura
while condition {
    # repeat while truthy
}

loop {
    # repeat until break/return/throw
}

for x in iterable {
    # bind x to each element
}
```

`for` iterates a list's elements, a string's characters, a map's keys (ascending
order), and a range's integers. Iterating anything else is `E4018`. Iteration
uses a snapshot, so mutating a collection during iteration does not change the
sequence being iterated.

## `break` and `continue`

`break` ends the innermost loop; `continue` skips to the next iteration. Both
must appear inside a loop, or the checker reports `E2015`. A lambda resets the
loop context, so a `break` inside a lambda is not licensed by an outer loop.

## Ranges

```aura
for i in range(5) { }        # 0, 1, 2, 3, 4
for i in range(2, 6) { }     # 2, 3, 4, 5
```

Ranges are start-inclusive and end-exclusive with a fixed step of `+1`. An empty
or descending range is empty. `len(range(a, b))` is `max(0, b - a)` with
saturating arithmetic. A range is materialized lazily in a `for`, so an early
`break` never builds the whole range.

## `return` and `throw`

`return [e]` ends the innermost function with `e` (or `none`). `throw e` raises
`e` as a throwable, catchable only by an explicit `catch`.

```aura
fn classify(n) {
    if n < 0 { throw "negative not allowed" }
    return n
}
```

## `match`

`match` is an expression and destructures patterns; see
[Pattern matching](/docs/guide-matching/).
