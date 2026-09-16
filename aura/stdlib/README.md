# Aura Standard Library

The Aura Standard Library provides core functionality for common programming
tasks. Aura code imports it with the `stdlib.` prefix.

```aura
from stdlib.math import sqrt, PI
from stdlib.collections import map, filter, reduce
```

The three supported import forms are:

```aura
import stdlib.collections as c          // module alias
from stdlib.collections import map, filter
import stdlib.collections { map, filter }   // brace form
```

## Modules

### json

JSON parsing and serialization.

```aura
import stdlib.json

let data = stdlib.json.loads('{"name": "Aura"}')
let text = stdlib.json.dumps(data, indent=2)
let pretty = stdlib.json.pretty(data)
let valid = stdlib.json.is_valid('{"ok": true}')
```

**Functions:**
- `loads(s)` - Parse JSON string
- `dumps(obj, indent)` - Serialize to JSON (strict: rejects NaN/Infinity)
- `load(path)` - Read and parse JSON file
- `dump(obj, path, indent)` - Write JSON to file
- `pretty(obj)` - Pretty-print JSON (indent=2, strict)
- `parse(s)` - Alias for loads
- `stringify(obj)` - Alias for dumps
- `is_valid(s)` - Check if string is standards-compliant JSON
- `merge(*dicts)` - Merge dicts (must be mappings), return JSON string

### time

Time and date utilities.

```aura
import stdlib.time

let t = stdlib.time.now()         // timestamp (seconds)
let ms = stdlib.time.now_ms()     // timestamp (milliseconds)
let iso = stdlib.time.iso()       // ISO 8601 string
stdlib.time.sleep(0.5)            // sleep 500ms
let elapsed = stdlib.time.elapsed(start)
```

**Functions:**
- `now()` - Current timestamp (seconds)
- `now_ms()` - Current timestamp (milliseconds)
- `sleep(seconds)` - Sleep for given seconds
- `clock()` - CPU time used by process
- `monotonic()` - Monotonic clock
- `perf_counter()` - High-resolution performance counter
- `strftime(fmt)` - Format current time
- `parse(date_string, fmt)` - Parse date string
- `iso()` - Current time as ISO 8601
- `timestamp(dt_obj)` - Convert datetime to timestamp
- `elapsed(start)` - Elapsed time since start

### io

File and directory operations.

```aura
import stdlib.io

stdlib.io.write("out.txt", "hello")
let content = stdlib.io.read("out.txt")
let lines = stdlib.io.read_lines("data.csv")

if stdlib.io.exists("config.json") {
  let cfg = stdlib.io.read("config.json")
}

stdlib.io.mkdir("output")
stdlib.io.rm("temp.txt")
let files = stdlib.io.ls(".")
```

**Functions:**
- `read(path)` - Read entire file as string
- `write(path, content)` - Write string to file
- `append(path, content)` - Append to file
- `exists(path)` - Check if file/dir exists
- `is_file(path)` - Check if path is a file
- `is_dir(path)` - Check if path is a directory
- `mkdir(path)` - Create directory
- `ls(path)` - List directory contents
- `rm(path)` - Remove file
- `rename(old, new)` - Rename/move file
- `extension(path)` - Get file extension
- `basename(path)` - Get filename without dir
- `dirname(path)` - Get directory part
- `join(*parts)` - Join path components
- `read_lines(path)` - Read file as list of lines (newlines removed)
- `write_lines(path, lines)` - Write list of lines (adds trailing newline)
- `copy(src, dst)` - Copy file
- `size(path)` - Get file size in bytes
- `touch(path)` - Create empty file

### collections

List, dict and set utilities.

```aura
from stdlib.collections import map, filter, reduce

let numbers = [1, 2, 3, 4, 5]
let doubled = map(numbers, (x) => x * 2)       // [2, 4, 6, 8, 10]
let even = filter(numbers, (x) => x % 2 == 0)  // [2, 4]
let sum = reduce(numbers, (a, b) => a + b, 0)  // 15
```

`map`, `filter`, `reduce`, `take` and `drop` are **data-first**, so they work
with the pipe operator:

```aura
let result = [1, 2, 3, 4, 5]
  |> filter((x) => x > 2)
  |> map((x) => x * 10)
  |> reduce((a, b) => a + b, 0)
```

**Data-first helpers:**
- `map(items, fn)` - Map function over items
- `filter(items, predicate)` - Keep items matching predicate
- `reduce(items, fn, initial)` - Reduce to a single value
- `take(items, n)` - First n items
- `drop(items, n)` - Items after the first n

**List helpers (fn-first):**
- `list_map(fn, items)`, `list_filter(predicate, items)`, `list_reduce(fn, items, initial)`
- `list_find(predicate, items)`, `list_any(predicate, items)`, `list_all(predicate, items)`
- `list_take(n, items)`, `list_drop(n, items)`, `list_chunk(n, items)`
- `list_zip(*iterables)`, `list_flatten(items)`, `list_unique(items)`
- `list_sort(items, key, reverse)`, `list_reverse(items)`

**Dict helpers:**
- `dict_get(d, key, default)`, `dict_keys(d)`, `dict_values(d)`, `dict_items(d)`
- `dict_merge(*dicts)`, `dict_filter(predicate, d)`, `dict_map(fn, d)`

**Set helpers:**
- `set_union(*sets)`, `set_intersection(*sets)`, `set_difference(a, *rest)`

`AuraDict` is the runtime dict used for dict literals; it supports both
`user["name"]` and `user.name`.

### itertools

Iterator and sequence utilities.

```aura
from stdlib.itertools import combinations, permutations, chain, groupby

let items = [1, 2, 3]
let combos = combinations(items, 2)     // [(1,2), (1,3), (2,3)]
let perms = permutations(items, 2)      // [(1,2), (1,3), (2,1), ...]
let combined = chain([1, 2], [3, 4])    // [1, 2, 3, 4]
let grouped = groupby([1, 1, 2, 2, 3])  // {1: [1, 1], 2: [2, 2], 3: [3]}
```

**Functions:**
- `range_iter(start, end, step)` - Create a range
- `cycle(iterable)`, `repeat(value, times)`, `count(start, step)`
- `chain(*iterables)`, `product(*iterables, repeat)`
- `combinations(iterable, r)`, `permutations(iterable, r)`
- `enumerate_iter(iterable, start)`, `islice(iterable, *args)`
- `takewhile(predicate, iterable)`, `dropwhile(predicate, iterable)`
- `groupby(iterable, key)`, `filterfalse(predicate, iterable)`
- `starmap(fn, iterable)`, `tee(iterable, n)`
- `zip_longest(*iterables, fillvalue)`, `pairwise(iterable)`

### math

Mathematical functions and constants.

```aura
from stdlib.math import sqrt, PI, floor, comb

print(PI)              // 3.141592653589793
print(sqrt(16))        // 4.0
print(floor(7 / 2))    // 3
print(comb(10, 3))     // 120
```

**Constants:** `PI`, `E`, `TAU`, `INF`, `NAN` (lowercase aliases too).

**Functions:**
- `sqrt(x)`, `pow(x, y)`, `exp(x)`, `log(x, base)`, `log10(x)`, `log2(x)`
- `sin(x)`, `cos(x)`, `tan(x)` and inverses `asin`, `acos`, `atan`, `atan2`
- `sinh(x)`, `cosh(x)`, `tanh(x)`
- `degrees(x)`, `radians(x)`
- `floor(x)`, `ceil(x)`, `round(x, ndigits)`
- `abs(x)`, `min(*values)`, `max(*values)`
- `gcd(*numbers)`, `lcm(*numbers)`
- `factorial(n)`, `comb(n, k)`, `perm(n, k)`
- `is_finite(x)`, `is_infinite(x)`, `is_nan(x)`

### string

String manipulation functions.

```aura
from stdlib.string import upper, lower, trim, split, join, replace

print(upper("hello"))                       // "HELLO"
print(trim("  hi  "))                       // "hi"
let parts = split("a,b,c", ",")             // ["a", "b", "c"]
let joined = join(["a", "b", "c"], "-")     // "a-b-c"
print(replace("hello", "l", "L"))           // "heLLo"
```

**Functions:**
- `upper(s)`, `lower(s)`, `title(s)`, `capitalize(s)`, `reverse(s)`
- `trim(s)`, `trim_left(s)`, `trim_right(s)`
- `pad_left(s, length, char)`, `pad_right(s, length, char)`, `pad(s, length, char)`
- `repeat_string(s, times)`, `split(s, separator, limit)`, `join(strings, separator)`
  (in `split`, `limit` is the maximum number of splits, like Python `maxsplit`)
- `starts_with(s, prefix)`, `ends_with(s, suffix)`, `contains(s, substring)`
- `index_of(s, substring)`, `last_index_of(s, substring)`, `replace(s, old, new, count)`
- `slice_string(s, start, end)`, `substring(s, start, length)`, `char_at(s, index)`
- `format_string(template, *args)`, `is_empty(s)`, `is_blank(s)`, `length(s)`
- `is_alpha(s)`, `is_alphanumeric(s)`, `is_digit(s)`
- `is_space(s)`, `is_lower(s)`, `is_upper(s)`, `is_numeric(s)`
- `lines(s)`, `unlines(lines)`, `codes(s)`, `from_codes(codes)`

### regex

Regular expressions (wraps Python's `re`).

```aura
import stdlib.regex as re

print(re.find_all(r"\d+", "a1b22c333"))   // ["1", "22", "333"]
print(re.groups(r"(\w+)@(\w+)", "u@h"))   // ["u", "h"]
print(re.replace(r"\s+", "_", "a b  c"))  // "a_b_c"
print(re.split(r",\s*", "a, b,c"))        // ["a", "b", "c"]
```

**Functions:** `match`, `full_match`, `search`, `find_all`, `find_iter`,
`split`, `replace`, `replace_fn`, `groups`, `escape`, `compile_pattern`.
**Flags:** `IGNORECASE`, `MULTILINE`, `DOTALL`, `VERBOSE`, `ASCII`.

### os

Environment, paths and process information. Does **not** expose process
execution, so Aura never shells out implicitly.

```aura
import stdlib.os as osx

print(osx.cwd())
print(osx.path_join("a", "b", "c.txt"))    // "a/b/c.txt"
print(osx.path_splitext("file.tar.gz"))    // ["file.tar", ".gz"]
let home = osx.get_env("HOME", "/tmp")
```

**Environment:** `get_env`, `set_env`, `unset_env`, `env`.
**Paths:** `cwd`, `chdir`, `home`, `temp_dir`, `path_join`, `path_abspath`,
`path_exists`, `path_is_file`, `path_is_dir`, `path_basename`, `path_dirname`,
`path_split`, `path_splitext`, `path_normpath`, `sep`, `linesep`.
**System:** `name`, `platform`, `pid`, `listdir`, `walk`, `makedirs`, `remove`,
`rename`, `get_size`.

### http

HTTP client using the standard library (`urllib`), with automatic use of
`requests` when installed. Responses are `AuraDict`, so both
`response.status` and `response["status"]` work.

```aura
import stdlib.http as http

let r = http.get("https://example.com")
print(r.status, r.ok)

let data = http.get_json("https://api.example.com/items")
print(data["count"])

let created = http.post_json("https://api.example.com/items", {"name": "x"})
```

**Functions:** `request`, `get`, `post`, `put`, `delete`, `get_json`,
`post_json`, `quote`, `unquote`, `build_url`.

Only `http`/`https` URLs are allowed; loopback/private hosts are blocked
(`AURA_HTTP_ALLOW_PRIVATE=1` to opt out), redirects are re-validated, and
response bodies are capped (`AURA_HTTP_MAX_BYTES`).

### threading

Native OS threads for I/O-bound work. Globals are shared across threads; guard
shared state with a lock.

```aura
import stdlib.threading as threading

let mut total = 0
let lock = threading.lock()

def worker(n) {
  let mut i = 0
  while i < n {
    lock.acquire()
    total += 1
    lock.release()
    i += 1
  }
}

let t = threading.spawn((x) => worker(x), 100)
t.join()
print(total)
```

**Functions:** `spawn`, `thread`, `lock`, `rlock`, `event`, `condition`,
`semaphore`, `barrier`, `local`, `run_many`, `map_concurrent`, `current_name`,
`active_count`, `enumerate_threads`, `main_thread`, `get_ident`, `stack_size`.
**Types:** `Thread` (`.start/.join/.is_alive/.name/.ident`), `_Lock`, `_RLock`.

### asyncio

Coroutines and structured concurrency. Aura has `async def`/`await` natively;
this module adds tasks, queues and gathering.

```aura
import stdlib.asyncio as aio

async def work(n) {
  await aio.sleep(0.01)
  return n * 2
}

async def main() {
  let results = await aio.gather(work(1), work(2), work(3))
  print(results)
}

await main()
```

**Functions:** `run`, `sleep`, `create_task`, `gather`, `wait_for`, `wait`,
`as_completed`, `queue`, `lock`, `event`, `semaphore`, `current_task`,
`all_tasks`, `is_running`, `new_event_loop`, `set_event_loop`, `get_event_loop`.

`aio.run(coro)` drives a coroutine from synchronous top-level code; inside an
already-running loop (`await`, or a program the CLI already wraps) use `await`
instead.

## String methods

Strings also support direct method calls, which map to the Python equivalents:

```aura
let s = "Hello, World"
s.to_upper()        // "HELLO, WORLD"
s.to_lower()        // "hello, world"
s.trim()            // strips whitespace
s.starts_with("He") // true
s.ends_with("ld")   // true
s.contains("ell")   // true
s.index_of("ell")   // 1
s.slice(0, 5)       // "Hello"
s.length()          // 12
s.is_alpha()        // false
```

## Adding to the stdlib

To add a new standard library module:

1. Create `aura/stdlib/mymodule.py` with functions
2. Add it to the imports in `aura/stdlib/__init__.py`
3. Register it in the source shim `stdlib/__init__.py`
4. Document it here
5. Add a test under `tests/`