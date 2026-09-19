---
layout: default
title: "13 — Modules and Imports"
parent: Learn Aura
nav_order: 23
---

[English](13-modules-and-imports.md) · [Português](13-modules-and-imports.pt_BR.md)

# 13 — Modules and Imports

> **Chapter goal:** organise code across files and namespaces. Reference:
> [`../language-reference/modules.md`](../language-reference/modules.md).

## Files are modules

Any `.aura` file can be imported. A file imported as a module **needs no
`main`**.

```aura
// lib.aura
def greet(name: str) -> str {
  return "hi " + name
}

const VERSION = "1.0"
```

```aura
// app.aura
import lib
from lib import greet as g

def main() {
  print(lib.greet("ana"))     // hi ana
  print(g("bob"))             // hi bob
  print(lib.VERSION)          // 1.0
}
```

`import lib` binds the file `lib.aura`; `import pkg.util` binds
`pkg/util.aura`. The runtime installs an import hook that maps the dotted Aura
path to the sibling file.

## The `module` declaration

`module Name { ... }` creates a namespaced group whose functions are static.
Members are **private to the declaring file unless marked `export`**:

```aura
module Greeter {
  export def hello(name: str) -> str {
    return "hi " + name
  }

  let mut count = 0

  export def bump() -> int {
    count = count + 1
    return count
  }
}

def main() {
  print(Greeter.hello("ana"))    // hi ana
  print(Greeter.bump())          // 1
  print(Greeter.bump())          // 2
}
```

Rules:

* `export` precedes a named declaration (`def`, `class`, `trait`, `enum`,
  `type`, `let`, `const`, a nested `module`) or introduces a re-export.
* A non-exported member is **mangled** in the generated Python, so privacy is
  enforced at runtime too; reaching it from outside is `E308`.
* Module state is **not writable from outside** (`E303`); mutate it through an
  exported function.
* A `main` inside a module body is `E312`.

```aura
module M {
  def hidden() -> int { return 1 }
}

def main() {
  print(M.hidden())    // E308: 'hidden' is not exported from module 'M'
}
```

## Dotted module names

`module App.Services { ... }` nests, reached as `App.Services.member`:

```aura
module Outer.Inner {
  export def value() -> int { return 3 }
}

def main() { print(Outer.Inner.value()) }    // 3
```

## Import spellings

| Form | Example | Binds |
|---|---|---|
| Module | `import stdlib.math` | `stdlib.math` path |
| Module alias | `import stdlib.math as m` | `m` |
| Names | `from stdlib.math import sqrt, PI` | `sqrt`, `PI` |
| Name alias | `from stdlib.math import sqrt as root` | `root` |
| Brace form | `import stdlib.math { sqrt, PI }` | `sqrt`, `PI` (≡ `from`) |
| Wildcard | `from stdlib.math import *` | all public names |
| Multiple | `import a, b as c` | `a`, `c` |

```aura
import stdlib.math as m
from stdlib.math import sqrt, PI

def main() {
  print(m.sqrt(16))     // 4.0
  print(sqrt(36))       // 6.0
  print(PI)             // 3.141592653589793
}
```

The brace form cannot be combined with `as` on the same statement. There is no
`::` separator — module paths are dotted. `*` is written only as
`from module import *`.

## Facades (re-exports)

A `module` can re-export symbols from sibling files. Put the facade in a folder
named after itself:

```text
App/
  App.aura          module App { export Components, Utils }
  components.aura   class Components { ... }
  utils.aura        def double(...) / const VERSION
main.aura           import App
```

```aura
// App/App.aura
module App {
  export Components, Utils
}
```

```aura
// main.aura
import App

def main() {
  print(App.Components("header").describe())
  print(App.Utils.double(21))
  print(App.Utils.VERSION)
}
```

An explicit source is accepted with `export Name from "module"`. The path must
be a plain dotted name; separators, `..` and absolute paths are rejected. An
unresolved re-export is `E313`.

## Name resolution

Outside a module, a bare name resolves through the local scope, then the
enclosing function, then top-level declarations, then imports, then a sibling
`.aura` file, then a `py.` host module. Inside `module M`, `M`'s own members win.
`UNSPECIFIED`: Aura does not diagnose an ambiguous import; the emitted Python
binding decides ("last binding wins").

## What you learned

* Any `.aura` file is importable; imported files need no `main`.
* `module Name { export ... }`; `export` and privacy (`E308`, `E303`, `E312`).
* Dotted modules; all import spellings; facades and re-exports.
* Roughly how names resolve.

## Next step

[Python Interop →](14-python-interop.md)
