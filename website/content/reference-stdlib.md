# Standard library

Every builtin below is defined in the shared signature registry
(`src/stdlib/signatures.rs`) that both the checker and the runtime consult, so
the two cannot disagree about arity or argument types.

## Core functions

| Function | Signature | Returns |
|---|---|---|
| `print` | `print(...)` | `none` — writes arguments separated by spaces, then a newline |
| `len` | `len(x)` | `int` — for string, list, map, range |
| `to_string` | `to_string(x)` | `string` |
| `to_int` | `to_int(x)` | `int` |
| `to_float` | `to_float(x)` | `float` |
| `range` | `range(n)` / `range(a, b)` | `range` — start-inclusive, end-exclusive |
| `abs` | `abs(n)` | number |
| `min` | `min(a, b)` | the lesser |
| `max` | `max(a, b)` | the greater |
| `assert` | `assert(cond)` / `assert(cond, msg)` | `none`; `E4028` on failure |

## Collection functions

| Function | Signature | Returns |
|---|---|---|
| `push` | `push(list, v)` | `none` |
| `keys` | `keys(map)` | `[string]` |
| `values` | `values(map)` | `[T]` |
| `sort` | `sort(list)` | sorted list |
| `reverse` | `reverse(string\|list)` | reversed |
| `map` | `map(list, f)` | `[T]` |
| `filter` | `filter(list, f)` | `[T]` |
| `reduce` | `reduce(list, f, init)` | accumulated value |
| `sum` | `sum(list)` | number |
| `enumerate` | `enumerate(list)` | `[[index, value], ...]` |
| `zip` | `zip(a, b)` | `[[a_i, b_i], ...]` |

## Scripting I/O

| Function | Signature | Returns |
|---|---|---|
| `read_line` | `read_line()` | `string \| none` |
| `read_file` | `read_file(path)` | `string \| none` |
| `write_file` | `write_file(path, content)` | `none` |
| `args` | `args()` | `[string]` |

Capabilities a host does not provide report `E5002`. See
[I/O and arguments](/docs/guide-io/).

## Methods

### string

`len` · `upper` · `lower` · `trim` · `contains(s)` · `starts_with(s)` ·
`ends_with(s)` · `split(s)` → `[string]` · `replace(a, b)` · `chars()` →
`[string]`

### list

`len` · `push(v)` · `pop()` · `first()` · `last()` · `join(s)` ·
`contains(v)` · `sort()` · `reverse()` · `map(f)` · `filter(f)` · `reduce(f, init)`

### map

`len` · `get(k)` · `has(k)` · `keys()` · `values()` · `remove(k)`

### range

`len`

## Feature-gated modules

### json

| Function | Signature |
|---|---|
| `json_encode` | `json_encode(value)` → `string` |
| `json_decode` | `json_decode(string)` → value |

### regex

| Function | Signature |
|---|---|
| `regex_match` | `regex_match(pattern, text)` → `bool` |
| `regex_find` | `regex_find(pattern, text)` → `string \| none` |
| `regex_find_all` | `regex_find_all(pattern, text)` → `[string]` |
| `regex_replace` | `regex_replace(pattern, text, replacement)` → `string` |

### time

| Function | Signature |
|---|---|
| `time_unix` | `time_unix()` → `int` (seconds since the epoch) |
| `time_now` | `time_now()` → a local-time map |
| `sleep_ms` | `sleep_ms(n)` → `none` |

The pattern is always the **first** argument to the regex functions.
