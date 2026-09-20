---
title: "07 — Modules"
---

# Modules

An Aura module is a named namespace declared with `module Name { ... }`. A file is also a module when imported. The target is CPython, so imports ultimately become Python imports.

See `aura/parser/to_ast.py:parse_module_decl`, `aura/transpiler/transformers/statements.py:transform_Module`.

## Module Declaration

```aura
module MyLib {
  export def public_function() -> int { return 42 }
  export const VERSION = "1.0.0"

  // Not exported: visible only inside MyLib.
  let mut cache = 0
  def private_helper() -> int { return cache }

  export def refresh() -> int {
    cache = private_helper() + 1
    return cache
  }
}
```

Members may be `def`, `class`, `trait`, `enum`, `type`, `let`/`const`, or a nested `module`. A member **not** marked `export` is private to the declaring file. Private members are emitted under mangled names (`_Lib__cache`).

### Dotted Names Nest

`module App.Services { ... }` is reached as `App.Services.member`. Nested `module` names produce nested Python classes.

### No `main` Inside a Module

`def main` inside a `module` body is **E312**.

## Export and Privacy

```aura
module M {
  def hidden() -> int { return 1 }
}
def main() {
  print(M.hidden())   // E308: 'hidden' is not exported from module 'M'
}
```

`export` is only meaningful inside a `module` body. A top-level `export` outside a module is a parse error.

## Module State Is Not Writable From Outside

```aura
module Counters {
  let mut count = 0
  export def bump() -> int { count += 1; return count }
}
Counters.bump()
// Counters.count = 9   // E303: module state is not writable from outside
```

Mutate module state through an exported function.

## Imports

```aura
import stdlib.math                         // import stdlib.math
import stdlib.math as m                    // import stdlib.math as m
from stdlib.math import sqrt, PI           // from stdlib.math import sqrt, PI
from stdlib.math import sqrt as root       // alias
import stdlib.math { sqrt, PI }            // brace form
from stdlib.math import *                  // wildcard
import a, b as c                           // multiple
```

There is no `::` separator; a module path is dotted. The brace form cannot be combined with `as`.

## Local `.aura` Files

```aura
// lib.aura
def greet(name: str) -> str { return "hi " + name }
const VERSION = "9"

// app.aura
import lib
from lib import greet as g
def main() {
  print(lib.greet("ana"))   // hi ana
  print(g("bob"))           // hi bob
}
```

A file imported as a module needs no `main`. The runtime installs an import hook mapping dotted paths to `.aura` files.

## Python Interop

```aura
import py.re             // binds `re`
from py.math import pi   // binds `pi`
```

A `py.` prefix marks a host Python import. The prefix is stripped before code generation.

## Module Facades (Re-exports)

```aura
// App/App.aura
module App {
  export Components, Utils
}
```

A bare `export Name` re-exports a symbol from a sibling file. Resolution: (1) same file, (2) sibling matching case-insensitively, (3) subfolder. Explicit source: `export Widgets from "widgets"`.

## Name Resolution

1. Local scope → enclosing function / closure → top-level declarations
2. Module of an importing alias or `from` binding
3. Aura sibling file (import hook) → `py.` host module → name error

## Gotcha

- `export` outside a `module` body is a parse error.
- Private members of imported files are enforced only by mangled names at runtime.
- `def main` inside a module is E312.

## Anti-pattern

Do not use wildcard imports (`from module import *`). They pollute the namespace and make name resolution ambiguous.
