# Pattern matching

`match` selects an arm whose pattern matches the subject and evaluates that
arm's body. It is an expression.

```aura
let name = match code {
    200 -> "OK"
    404 -> "Not Found"
    _ -> "Other"
}
```

## Patterns

| Pattern | Matches |
|---|---|
| `_` | anything, binding nothing |
| a lowercase name | anything, binding the value |
| `42`, `"s"`, `true`, `none` | the literal value |
| `[a, b]` | a list of exactly that arity, then sub-patterns |
| `Circle(r)` | the variant `Circle`, binding its payload |
| `Empty` | the variant `Empty` |

Pattern names bind; a capitalized name is a variant. A pattern may bind the same
name only once (`E2014`).

## Guards

An arm may carry a guard after `if`. If the guard is falsy, matching continues
with later arms.

```aura
let kind = match n {
    x if x < 0 -> "negative"
    0 -> "zero"
    _ -> "positive"
}
```

## Value versus binding patterns

An arm's `if` guard can also disambiguate. A bare lowercase name always binds,
so to match a constant, compare it in a guard:

```aura
let threshold = 10
match value {
    x if x == threshold -> "at threshold"
    _ -> "elsewhere"
}
```

## No match

If no arm matches and there is no catch-all, the program fails with `E4029` (no
`match` arm matched). Always include a `_` arm when a subject may fall through.

## Where patterns appear

* `match` arms (with optional guards).
* `let` destructuring, for list and variant patterns.
* `for` loop variables, binding each element.

Aura has no rest/spread pattern. List patterns must match the list's exact
arity:

```aura
let [a, b] = [1, 2]         # ok
let [a] = [1, 2]            # E3001: arity mismatch
```
