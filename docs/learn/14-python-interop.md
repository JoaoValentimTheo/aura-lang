---
layout: default
title: "14 — Python Interop"
parent: Learn Aura
nav_order: 24
---

[English](14-python-interop.md) | [Português](14-python-interop.pt_BR.md)

# 14 — Python Interop

> **Chapter goal:** call the Python ecosystem from Aura. Reference:
> [`../language-reference/python-interop.md`](../language-reference/python-interop.md).
> Example: `../../examples/python_interop.aura`.

## Why interop is explicit

Aura runs on CPython, so every installed Python module is reachable. But a plain
`import re` is an **Aura** module, not a Python one. To import a **host Python**
module, prefix the path with `py.`:

```aura
import py.math as math
import py.os as os
from py.json import dumps, loads

def main() {
  print(math.sqrt(16))          // 4.0
  print(os.name)                // posix
  let encoded = dumps({"n": 1})
  print(loads(encoded)["n"])    // 1
}
```

The `py.` prefix is stripped before code generation, so `import py.math as math`
emits `import math as math`.

## Binding rules

| Form | Bound name |
|---|---|
| `import py.re` | `re` (the **last** path segment) |
| `import py.os.path` | `path` — not `os` |
| `import py.re as regex` | `regex` (an explicit alias wins) |
| `from py.math import sqrt, pi as PI` | `sqrt`, `PI` |

**Keyword collisions must be aliased.** A Python name that is an Aura keyword
cannot be bound under that name. Here `type` is an Aura keyword, so it must be
aliased (this one runs):

```aura
from py.builtins import type as py_type

def main() {
  print(py_type(5))            // <class 'int'>
}
```

Without the alias the parser rejects the binding:

```aura
// from py.re import type          // error: 'type' is a reserved keyword
// from py.re import type as re_type  // ok — aliased
```

Names that commonly need an alias: `type`, `from`, `in`, `is`, `class`,
`match`, `new`, `self`, and every other reserved word. When no alias is possible
— or the name is only known at runtime — use the dynamic bridge.

## The `python` bridge

For runtime access, import the `python` bridge:

```aura
import python

def main() {
  let re = python.import_module("re")     // dynamic import
  print(python.hasattr(re, "match"))      // True
  print(python.eval("1 + 2"))             // 3
  print(python.type_name(42))             // builtins.int
  print(python.is_available("requests"))  // True/False, never raises
}
```

| Function | Purpose |
|---|---|
| `import_module(name)` / `load(name)` | import a module by name |
| `reload(module)` | reload a module |
| `is_available(name)` | `True`/`False`, never raises |
| `eval(expr)` | evaluate an expression string |
| `exec_code(src)` / `compile_source(src)` | execute / compile source |
| `call(func, *args)` | call any Python callable |
| `getattr(obj, name, default)` | attribute by name (escapes collisions) |
| `setattr(obj, name, value)` | attribute assignment by name |
| `hasattr(obj, name)` / `dir(obj)` | introspection |
| `type_name(obj)` | fully-qualified type name |
| `to_aura(obj)` / `to_python(obj)` | convert between the two worlds |
| `add_path(path)` / `site_packages()` / `modules()` | environment and discovery |

The bridge is a thin, total wrapper: failures raise ordinary Python exceptions,
so Aura `try`/`catch` works as expected.

## Choosing

* Use `import py.x` / `from py.x import y` when the module and names are known
  at compile time — direct, fast, readable.
* Use `import python` for dynamic names, `eval`, introspection, or a keyword-
  colliding attribute (`python.getattr(mod, "type")`).

## A framework example

Because a `py.` import is a real Python import, standard libraries and
frameworks work directly. Here is a value built with `py.json` and read back
through ordinary Aura indexing:

```aura
import py.json as json

def main() {
  let payload = {"name": "aura", "ok": true}
  let encoded = json.dumps(payload)
  print(encoded)                       // {"name": "aura", "ok": true}
  let decoded = json.loads(encoded)
  print(decoded["name"], decoded["ok"]) // aura True
}
```

## Python collections are Aura collections

Values cross the boundary unchanged: a Python `dict` is an Aura dict, a Python
`list` is an Aura list. So Aura conveniences apply to returned objects:

```aura
import py.json as json

def main() {
  let data = json.loads('{"items": [1, 2, 3]}')
  print(data["items"].length())    // 3
}
```

## Errors across the boundary

A Python exception raised by a host call is an ordinary exception to Aura, so
`try`/`catch` works. Catch `Error` for any exception, or a specific type:

```aura
import py.json as json

def main() {
  let data = try { json.loads("not json") } catch Error as e { none }
  print(data is none)              // true
}
```

The bridge itself is a thin wrapper: it does not swallow failures, it lets the
underlying Python exception propagate.

## What is NOT interop

| Spelling | Why | Use instead |
|---|---|---|
| `import re` expecting Python | plain imports are Aura modules | `import py.re` |
| `from re import type` | `type` is reserved | `from py.re import type as re_type` |
| `python.import_module(...)` with no `import python` | the bridge must be imported | `import python` first |
| raw CPython internals through the bridge | the bridge is the supported surface | the listed helpers |

## What you learned

* `py.` marks a host Python module; the bound name is the last segment.
* Keyword collisions require an alias.
* The `python` bridge for dynamic access and introspection.
* Values cross the boundary unchanged.

## Next step

[Macros and Decorators →](15-macros-and-decorators.md)
