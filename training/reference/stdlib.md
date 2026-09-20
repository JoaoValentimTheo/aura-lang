# Aura Standard Library — Quick Reference

All modules: `import stdlib.<name>` or `from stdlib.<name> import ...`

## Module Overview

| Module | Purpose |
|--------|---------|
| `stdlib.collections` | List/dict/set helpers, `AuraDict`, pipe-compatible `map`/`filter`/`reduce` |
| `stdlib.math` | Constants (`PI`, `E`, `TAU`, `INF`, `NAN`) + trig, log, combinatorics |
| `stdlib.string` | `upper`, `lower`, `trim`, `split`, `join`, `replace`, `is_alpha`, etc. |
| `stdlib.json` | `loads`, `dumps`, `load`, `dump`, `pretty`, `is_valid` |
| `stdlib.regex` | `match`, `search`, `find_all`, `replace`, `split`, `compile_pattern` |
| `stdlib.http` | Client: `get`, `post`, `put`, `delete`, `get_json`, async variants |
| `stdlib.io` | File I/O: `read`, `write`, `ls`, `rm`, `mkdir`, `copy`, async variants |
| `stdlib.os` | Env vars, paths, CWD, `listdir`, `walk`, `makedirs` |
| `stdlib.time` | `now`, `sleep`, `clock`, `monotonic`, `strftime`, `iso`, `elapsed` |
| `stdlib.crypto` | Hashing, HMAC, HKDF, random bytes, post-quantum KEM/DSA |
| `stdlib.threading` | OS threads: `spawn`, `lock`, `rlock`, `event`, `map_concurrent` |
| `stdlib.asyncio` | `run`, `sleep`, `gather`, `create_task`, `wait_for`, `queue` |
| `stdlib.testing` | `test`, `equal`, `raises`, `contains`, `run_all` |
| `stdlib.python` | Bridge: `import_module`, `load`, `eval`, `getattr`, `type_name` |
| `stdlib.macros` | `assert_eq`, `swap`, `stringify`, `static_assert` |

## `stdlib.collections` — Pipe-compatible (data-first)

```aura
let doubled = map([1, 2, 3], (x) => x * 2)          // [2, 4, 6]
let evens = filter([1, 2, 3, 4], (x) => x % 2 == 0) // [2, 4]
let total = reduce([1, 2, 3], (a, b) => a + b, 0)   // 6
```

| Function | Notes | | Function | Notes |
|----------|-------|-|----------|-------|
| `map(items, fn)` | Pipe: `xs \|> map(fn)` | | `zip(*lists)` | Zip |
| `filter(items, pred)` | | | `flatten(items)` | Deep flatten |
| `reduce(items, fn, init?)` | | | `unique(items)` | Dedup (order kept) |
| `take(items, n)` | First n | | `sort(items, key?, rev?)` | Sorted copy |
| `drop(items, n)` | Skip n | | `reverse(items)` | Reversed copy |
| `find(pred, items)` | First match or `none` | | `chunk(n, items)` | Split into chunks |
| `any(pred, items)` | Any match? | | `dict_get(d, k, default?)` | |
| `all(pred, items)` | All match? | | `dict_merge(*dicts)` | |

`AuraDict`: dict with attribute access (`d.name`). Set helpers: `set_union`, `set_intersection`, `set_difference`.

## `stdlib.math`

| Constants | `PI`, `E`, `TAU`, `INF`, `NAN` (also lowercase) |
|-----------|--------------------------------------------------|
| Basic | `abs`, `min`, `max`, `round`, `floor`, `ceil` |
| Power/Log | `sqrt`, `pow`, `log`, `log10`, `log2`, `exp` |
| Trig | `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2` |
| Comb. | `factorial`, `comb`, `perm`, `gcd`, `lcm` |
| Checks | `is_finite`, `is_infinite`, `is_nan` |

## `stdlib.json`

`loads`, `dumps(obj, indent?)`, `load(path)`, `dump(obj, path)`, `pretty(obj)`, `parse` (alias), `stringify` (alias), `is_valid(s)`, `merge(*dicts)`.

## `stdlib.http`

SSRF-protected. Returns `AuraDict` with `status`, `body`, `headers`, `ok`, `url`.

| Sync | Async | | Sync | Async |
|------|-------|-|------|-------|
| `get(url)` | `aget(url)` | | `delete(url)` | `adelete(url)` |
| `post(url, data?)` | `apost(url, data?)` | | `get_json(url)` | `aget_json(url)` |
| `put(url, data?)` | `aput(url, data?)` | | `post_json(url, data)` | `apost_json(url, data)` |

## `stdlib.io` (all have `*_async` variants)

`read`, `write`, `append`, `read_lines`, `write_lines`, `exists`, `is_file`, `is_dir`, `ls`, `mkdir`, `rm`, `copy`, `size`, `touch`.

## `stdlib.os`

`get_env`, `set_env`, `cwd`, `chdir`, `home`, `listdir`, `walk`, `path_join`, `path_exists`, `pid`, `platform`.

## `stdlib.time`

`now`, `now_ms`, `sleep`, `clock`, `monotonic`, `perf_counter`, `strftime`, `parse`, `iso`, `elapsed`.

## `stdlib.crypto`

| Category | Functions |
|----------|-----------|
| Hashing | `sha256`, `sha512`, `sha3_256`, `sha3_512`, `shake_256`, `blake2b`, `digest` |
| HMAC | `hmac_sha256`, `hmac_sha3_256`, `hmac_sha512` |
| KDF | `hkdf_sha256` |
| Random | `random_bytes`, `random_hex`, `random_int` |
| PQC | `kem_keypair`, `kem_encapsulate`, `dsa_keypair`, `dsa_sign`, `dsa_verify` |

## `stdlib.testing`

```aura
import stdlib.testing as t
t.test("add", () => { t.equal(1 + 1, 2) })
t.run_all()
```

Assertions: `equal`, `not_equal`, `is_true`, `is_false`, `is_none`, `contains`, `raises`, `greater`, `less`, `approx_equal`, `has_length`, `is_empty`.

## `stdlib.threading`

`spawn(target, *args)`, `thread`, `lock`, `rlock`, `event`, `condition`, `semaphore`, `barrier`, `run_many`, `map_concurrent(fn, items, workers?)`.

## `stdlib.asyncio`

`run(coro)`, `sleep`, `gather(*coros)`, `create_task`, `wait_for`, `wait`, `queue`, `lock`, `event`, `semaphore`.

## `stdlib.python`

`import_module(name)`, `load(name)` → `ModuleProxy`, `eval(expr)`, `exec_code(code)`, `getattr`, `hasattr`, `type_name`, `is_instance`, `to_aura`, `to_python`.
