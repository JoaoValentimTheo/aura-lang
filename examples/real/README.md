# Real-world examples

Complete, runnable programs that use Aura the way an application would —
not feature demos. Each one passes `aura check` and runs with `aura run`,
and all four are exercised as smoke tests in CI
(`tests/test_real_examples.py`).

| File | What it is | Shows |
|------|-----------|-------|
| `wordcount.aura` | A command-line word counter | `sys.argv` parsing, file I/O (`stdlib.io`), dict aggregation, sorting, slicing |
| `todo_api.aura` | A Flask JSON service | Dotted `@app.route` decorators, JSON request/response, path parameters, dict store |
| `async_fetch.aura` | An async HTTP client | `async def`/`await`, `stdlib.asyncio`, a local `http.server`, SSRF guard |
| `sales_pipeline.aura` | A data-processing pipeline | The pipe operator `\|>` with `map`/`filter`/`reduce`, grouping, a formatted report |

## Running

```bash
# CLI / files  (uses the bundled sample.txt)
aura run examples/real/wordcount.aura examples/real/sample.txt --top 3

# Flask service  (exercised through Flask's test client — no server blocks)
aura run examples/real/todo_api.aura

# Async HTTP  (starts its own loopback server; the HTTP client blocks
# private addresses by default, so allow it for this example only)
AURA_HTTP_ALLOW_PRIVATE=1 aura run examples/real/async_fetch.aura

# Data pipeline
aura run examples/real/sales_pipeline.aura
```

## Friction found while writing these

Writing real programs surfaced one transpiler bug, filed with a minimal
reproduction:

- [issue #10](https://github.com/JoaoValentimTheo/aura-lang/issues/10) —
  stdlib module member calls are rewritten by `METHOD_ALIASES`
  (`strings.trim(...)` becomes `strings.strip(...)` and fails at runtime).
  The examples use `from stdlib.string import trim` (function style) as a
  workaround.
