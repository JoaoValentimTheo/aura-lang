# Standard library

Every builtin below is defined in the shared signature registry
(`src/stdlib/signatures.rs`) that both the checker and the runtime consult, so
the two cannot disagree about arity or argument types.

Some builtins mutate an argument in place (`push`, `pop`, `remove`). Under the
general mutation-capability rule, mutating through a binding requires that
binding to be `mut` (`E2001` otherwise); the registry marks which builtins
mutate, so this is one rule rather than a per-function special case.

## Core functions

| Function | Signature | Returns |
|---|---|---|
| `print` | `print(...)` | `none` — writes arguments separated by spaces, then a newline |
| `len` | `len(x)` | `int` — for string, list, array, tuple, set, map, range |
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
| `keys` | `keys(map)` | `[K]` (the map's key type) |
| `values` | `values(map)` | `[V]` (the map's value type) |
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

### array

A fixed-length sequence: the read-side list methods, but no resizing.

`len` · `first()` · `last()` · `join(s)` · `contains(v)` · `map(f)` ·
`filter(f)` · `reduce(f, init)`

### tuple

A fixed-length, immutable sequence.

`len` · `first()` · `last()` · `contains(v)` · `map(f)` · `filter(f)` ·
`reduce(f, init)`

### set

`len` · `has(v)` · `contains(v)` · `add(v)` · `remove(v)`

### map

`len` · `get(k)` · `has(k)` · `keys()` · `values()` · `items()` · `remove(k)`

`items()` yields two-element lists `[key, value]` in ascending key order.

### range

`len`

## Modules

`json`, `regex`, and `time` are part of the default feature set; `http` is a
separate, non-default native capability.

### json

| Function | Signature |
|---|---|
| `json_encode` | `json_encode(value)` → `string` |
| `json_decode` | `json_decode(string)` → value |
| `json_decode_as` | `json_decode_as(text, Type)` → `Type` (the second argument is a type, e.g. `User`, `[User]`, `[int; 3]`; strict, `E4031` on mismatch) |

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

## HTTP (`http` feature)

HTTP is a **feature-gated, native-only** capability (`--features http`) and is
**not** enabled by default: it is the only feature that grows the native
dependency tree, and a build without it has no network surface at all. Network
access is **host-owned** — the capability lives in the host, and in the browser
Playground there is no HTTP authority, so a call reports `E5002`.

| Function | Signature | Returns |
|---|---|---|
| `http_get` | `http_get(url)` | `{status: int, headers: {string: string}, body: string}` |
| `http_request` | `http_request(method, url)` | the same response map |
| `http_request` | `http_request(method, url, options)` | the same response map |

`method` is one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`
(case-insensitive). `options` is a map with optional keys:

* `headers` — a list of `[key, value]` string pairs;
* `body` — the request body string (for methods that carry one);
* `timeout_ms` — a positive integer, clamped to 30,000 ms.

The response map carries `status` (`int`), `headers` (a map of lowercased
header names to comma-joined values), and `body` (`string`).

```aura
# Only with a native build that enables the `http` feature.
# Pass the target URL as an argument, e.g. `aura run fetch.aura <url>`.
fn main() {
    let resp = http_get(args()[0])
    print(resp["status"])
    print(resp["body"])
}
```

A host that does not provide the capability reports `E5002`; a transport or
resource failure is `E4020`; a typed JSON mismatch while decoding a response
body is `E4031`.

The released `0.3.1` capability is intentionally small. It does **not** include
an `http::get` module spelling, HTTP sessions, cookie jars, streaming
downloads, multipart uploads, or HTTP server APIs — those are future `0.3.2`
candidates and are not available.
