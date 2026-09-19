---
layout: default
title: "Python Interop"
parent: Aura Language Reference
nav_order: 8
---

[English](python-interop.md) · [Português](python-interop.pt_BR.md)

# Python Interop

**Status:** Stable · **Evidence:** `aura/parser/to_ast.py` (`parse_import_stmt`,
`parse_from_import_stmt`, `_split_python_prefix`),
`aura/transpiler/transformers/statements.py` (`_transform_python_import`),
`aura/stdlib/python.py` (the dynamic bridge).

Aura transpiles to Python and runs on CPython, so every Python module is
reachable. Interop is **explicit**: a host module is imported through the `py.`
prefix, which separates Python modules from Aura modules at the call site.

---

## 1. Importing a Python module

```aura
import py.re                 // binds `re`
import py.os.path            // binds `path` (the last segment)
import py.re as regex        // binds `regex`
import py.re, py.json        // binds `re` and `json`
```

Rules:

* The `py.` prefix marks a **host Python** module. A plain `import re` (no
  prefix) is an **Aura** module — the two never mix by accident.
* The bound name is the **last path segment**: `import py.os.path` binds
  `path`, not `os` (this differs from Python's `import os.path`, which binds
  `os`). An explicit `as` alias always wins.
* The `py.` prefix is stripped before code generation: `import py.re` emits
  `import re as re`; `import py.os.path` emits `import os.path as path`.

```aura
import py.re as re
import py.json

def main() {
  print(re.match("a+", "aaa") != none)   // True
  print(json.loads('{"n": 1}')["n"])     // 1
}
```

---

## 2. Importing names from a Python module

```aura
from py.math import sqrt, pi as PI
from py.re import match as re_match, sub as re_sub
```

A name that collides with an Aura keyword **cannot** be bound under that name;
use an alias:

```aura
// from py.re import type         // error: 'type' is a reserved keyword
from py.re import type as re_type  // ok
```

Keywords that require an alias include `type`, `from`, `in`, `is`, `class`,
`match`, and every other reserved word. When a Python name has no usable alias —
or the name is only known at runtime — reach it through the dynamic bridge
(§4) instead.

---

## 3. Local Aura modules vs Python modules

| Form | Resolves to |
|---|---|
| `import util` / `import pkg.util` | an Aura module (a sibling `.aura` file or package) |
| `from util import x` | a name in an Aura module |
| `import stdlib.math` | the Aura standard library (`aura/stdlib`) |
| `import py.re` / `from py.math import sqrt` | a host Python module |

A `py.` import is always treated as external — it is never mistaken for an Aura
sibling, even if an Aura module of the same name exists.

---

## 4. The dynamic bridge (`python`)

For runtime access — a module name built at run time, a keyword-colliding
attribute, or introspection — import the `python` bridge:

```aura
import python

let re = python.import_module("re")   // dynamic import
let math = python.load("math")        // alias of import_module
print(math.sqrt(2))                   // 1.4142135623730951

print(python.eval("1 + 2"))           // 3
print(python.type_name(42))           // "builtins.int"
print(python.is_available("requests"))// True/False, never raises
print(python.getattr(re, "type"))     // attribute with a keyword name
```

The bridge is a thin, total wrapper; failures raise ordinary Python exceptions
so Aura `try`/`catch` works as expected.

### 4.1 Bridge surface

| Function | Purpose |
|---|---|
| `import_module(name)` / `load(name)` | import a module by name, returns a module proxy |
| `reload(module)` | reload a module |
| `is_available(name)` | `True`/`False`, never raises |
| `eval(expr)` | evaluate an expression string |
| `exec_code(src)` / `compile_source(src)` | execute / compile source |
| `call(func, *args)` | call any Python callable |
| `getattr(obj, name, default=None)` | attribute access by name (escapes keyword collisions) |
| `setattr(obj, name, value)` | attribute assignment by name |
| `hasattr(obj, name)` / `dir(obj)` | introspection |
| `type_name(obj)` | fully-qualified type name |
| `is_module` / `is_callable` / `is_class` / `is_instance` | predicates |
| `to_aura(obj)` / `to_python(obj)` | convert between the two worlds |
| `add_path(path)` / `site_packages()` / `modules()` | environment and discovery |
| `interpreter_version()` | the CPython version string |

---

## 5. Choosing between `py.` and `python.*`

* Use `import py.re` / `from py.math import sqrt` when the module and names are
  known at compile time — it is direct, fast and readable.
* Use `import python` (the bridge) for dynamic names, `eval`, introspection, or
  an attribute whose name is an Aura keyword (`python.getattr(mod, "type")`).

Both forms lower to ordinary Python imports/calls; Aura adds no runtime layer
beyond the `py.` prefix stripping and the bridge helpers.

---

## 6. What is NOT part of interop

| Spelling | Why | Use instead |
|---|---|---|
| `import python` reaching arbitrary CPython internals directly | the bridge is the supported surface | `import python` then `python.*` |
| `import re` expecting a Python module | plain imports are Aura modules | `import py.re` |
| `from re import type` | `type` is a reserved keyword | `from py.re import type as re_type` |
| `python.import_module` at module head with no `import python` | the bridge must be imported | `import python` first |
