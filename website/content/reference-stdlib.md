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

HTTP is a **feature-gated** capability (`--features http`) and is **not**
enabled by default: it is the only feature that grows the native dependency
tree, and a native build without it has no network surface at all. Network
access is **host-owned**: the capability lives in the host, so a host that does
not provide it reports `E5002`. The native host provides it when built with
`--features http`; the browser Playground provides it through a session (see
[the Playground document](/docs/playground-doc/)), asking for per-origin consent
before the first request.

| Function | Signature | Returns |
|---|---|---|
| `http_get` | `http_get(url)` | the response map |
| `http_request` | `http_request(method, url)` | the response map |
| `http_request` | `http_request(method, url, options)` | the response map |

`method` is one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`
(case-insensitive). `options` is a map with optional keys:

* `headers` — a list of `[key, value]` string pairs;
* `body` — the request body string (for methods that carry one);
* `timeout_ms` — a positive integer, clamped to 30,000 ms.

The response map has exactly five keys:

* `status` (`int`) — the response status.
* `body` (`string`) — the body as text; a lossy rendering when the payload is
  not valid UTF-8.
* `body_bytes` (`[int]`) — the body as bytes in `0..=255`. Use this to recover
  binary payloads exactly.
* `headers` (`{string: string | [string]}`) — keyed by the **lowercased** header
  name. On a native build, a header that occurred once maps to its value string;
  a header that occurred more than once maps to a **list** of every occurrence,
  in order — never comma-joined, because that would destroy repeated headers
  such as `Set-Cookie`. The Playground cannot observe duplicates at all; see
  [browser differences](#browser-differences-deliberate-not-defects) below.
* `header_lines` (`[[string, string]]`) — the flat, ordered list of
  `[name, value]` pairs. On a native build it preserves duplicates and the
  original casing; the Playground has one pair per unique name.
Redirects are **not followed**: the response describes the redirect itself, so
no `Location` target is requested. On native builds `status` is the `3xx` the
server sent; the Playground cannot observe it (see browser differences below).

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

### Browser differences (deliberate, not defects)

The Playground runs behind the browser's network security model, which diverges
from a native build in three ways. None is worked around by inventing data:

* **Redirects.** A manual-mode fetch yields an *opaque redirect* whose status,
  headers, and body are all hidden. The browser has no way to tell `301` from
  `302`, `303`, `307`, or `308`, so the call reports `E4020` naming the redirect
  instead of a fabricated `3xx`. Native builds report the real status.
* **`Set-Cookie`.** The browser's `Headers` interface never exposes it, so it is
  absent from `headers` and `header_lines` in the Playground.
* **Repeated headers.** `Headers` is a combining collection: it yields one
  entry per unique name, with occurrences already joined by `", "`, in sorted
  lowercased order. On the browser host each name therefore maps to a single
  combined string and `header_lines` has one pair per unique name — the
  list-on-duplicate rule above is what **native** builds observe. Original
  casing and wire order are not recoverable in the Playground because the
  platform discards them before Aura sees the response.

The released `0.3.1` capability is intentionally small. It does **not** include
an `http::get` module spelling, HTTP sessions, cookie jars, streaming
downloads, multipart uploads, or HTTP server APIs — those are future `0.3.2`
candidates and are not available.
