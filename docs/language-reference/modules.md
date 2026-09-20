---
layout: default
title: "Modules, Packages and Imports"
parent: Aura Language Reference
nav_order: 7
---

[English](modules.md) · [Português](modules.pt_BR.md)

# Modules, Packages and Imports

**Status:** Stable (except where labeled) · **Evidence:**
`aura/parser/to_ast.py` (`parse_module_decl` §1567, `parse_import_stmt` §1825,
`parse_from_import_stmt` §1885, `_split_python_prefix` §494),
`aura/transpiler/transformers/statements.py` (`transform_Module` §1465,
`transform_ImportStmt` §1335, `transform_FromImport` §1390,
`_module_reexport_lines` §1569), `aura/transpiler/rules.py`
(`_check_no_main_in_modules` §246, `_check_reexports` §439,
`_check_const_member_assignment` §1288), `aura/transpiler/modules.py`
(`resolve_reexport` §93).

An Aura **module** is a named namespace declared with `module Name { ... }`.
A **file** is also a module when imported. This document defines how a module is
declared, how names cross a module boundary, and how imports are written. The
target is CPython, so an import ultimately becomes a Python `import`.

---

## 1. Module declaration

```
module-decl    = "module" , identifier , { "." , identifier } , "{" , { member } , "}" ;
member         = [ "export" ] , ( declaration | bare-export ) ;
bare-export    = identifier , { "," , identifier } , [ "from" , string ] , [ ";" ] ;
```

`module Name { ... }` creates a namespaced class whose functions are static
(`transform_Module`, `statements.py:1465-1501`). Members may be `def`, `class`,
`trait`, `enum`, `type`, `let`/`const` data, or a nested `module`
(`modules.md` §9).

```aura
module MyLib {
  export def public_function() -> int {
    return 42
  }

  export const VERSION = "1.0.0"

  // Not exported: visible only inside MyLib.
  let mut cache = 0

  def private_helper() -> int {
    return cache
  }

  export def refresh() -> int {
    cache = private_helper() + 1
    return cache
  }
}

def main() {
  print(MyLib.public_function())  // 42
  print(MyLib.VERSION)            // 1.0.0
  print(MyLib.refresh())          // 1
}
```

*Evidence:* `test_modules.py::TestModuleParsing::test_module_records_exports`,
`::test_exports_of_every_member_kind`; *probe* (emits `class MyLib:` with
`@staticmethod` members, private member mangled to `_MyLib__cache`).

### 1.1 Dotted names nest

`module App.Services { ... }` is a nested namespace reached as
`App.Services.member`. It transpiles to nested Python classes
(`transform_Module`, `statements.py:1494-1501`).

```aura
module Outer.Inner {
  export def value() -> int { return 3 }
}

def main() { print(Outer.Inner.value()) }   // 3
```

*Evidence:* `test_modules.py::test_dotted_module_name`,
`::test_dotted_module_emits_nested_classes`.

### 1.2 Private members are mangled

A member **not** marked `export` is emitted under a mangled name
(`_Lib__cache`), so privacy is enforced at runtime as well as at check time
(`_module_class`, `statements.py:1536-1543`; `_rename_module_member` §1648).

*Evidence:* `test_modules.py::test_private_member_is_mangled`,
`::test_private_data_member_is_mangled`,
`::test_internal_call_to_private_member_uses_mangled_name`.

### 1.3 A `main` inside a module is rejected

`main` belongs to the **entry file**, never to a module body. `def main` inside
`module Name { ... }` is **E312** (`_check_no_main_in_modules`,
`rules.py:246-265`).

```aura
module M {
  def main() { }   // E312: 'main' is declared inside module 'M'
}
```

*Evidence:* `test_module_facade.py::test_main_inside_a_module_reports_e312`;
*probe* (`E312`).

---

## 2. `export` and privacy (E308)

A module member is **private to the declaring file by default**. Add `export`
to a `def`, `class`, `trait`, `enum`, `type`, `let`, `const` or nested `module`
to make it public (`parse_module_decl`, `to_ast.py:1582-1619`).

| Access | Result |
| --- | --- |
| Exported member, from anywhere | OK |
| Private member, from inside its own file/module | OK |
| Private member, from outside the module | **E308** |
| `export` outside a `module` body | parse error |
| `export` with no following declaration or name | parse error |

```aura
module M {
  def hidden() -> int { return 1 }
}

def main() {
  print(M.hidden())   // E308: 'hidden' is not exported from module 'M'
}
```

The diagnostic names the member and the module, and hints to add `export`
(`_visit_member_access`, `rules.py:1143-1149`).

*Evidence:* `test_modules.py::TestModuleRules::test_accessing_a_non_exported_member_reports_e308`,
`::test_e308_message_names_the_module_and_export`;
`test_modules.py::test_export_outside_module_is_rejected`,
`::test_export_without_a_name_is_rejected`.

> **Target-specific:** a private member of an **imported** file is enforced only
> by the mangled name at runtime; the checker does not re-inspect another file's
> module. *Probe*: `lib.Lib.sec()` where `sec` is private in `lib.aura` reports
> `OK` to `aura check app.aura` and fails at runtime with
> `AttributeError: type object 'Lib' has no attribute 'sec'`.

---

## 3. Module state is not writable from outside (E303)

A module member cannot be assigned from outside the module, even an `export`ed
one. Mutate module state through an exported function, which may freely use
`let mut` members internally (`_check_const_member_assignment`,
`rules.py:1288-1316`).

```aura
module Counters {
  let mut count = 0
  export def bump() -> int {
    count = count + 1
    return count
  }
}

def main() {
  Counters.bump()
  Counters.bump()
  // Counters.count = 9   // E303: module state is not writable from outside
}
```

Inside the module body (and inside its functions) assigning to module state is
allowed (`_is_module_body`, `rules.py:1327-1330`), so `bump` above is clean.

*Evidence:* `test_modules.py::TestModuleRules::test_assigning_module_state_from_outside_reports_e303`,
`::test_internal_assignment_to_module_state_is_clean`;
`test_modules.py::TestModuleRuntime::test_module_private_state_is_mutable_internally`;
*probe* (`E303` on `M.count = 9`).

---

## 4. Imports

Aura has four import spellings, all backed by the same parser and all
transpiling to Python imports (`parse_import_stmt`, `to_ast.py:1825-1869`;
`parse_from_import_stmt` §1885-1909).

```
import-decl    = "import" , module-path , ( [ "{" , import-item , { "," , import-item } , "}" ]
                 | import-list ) , [ "as" , identifier ] , [ ";" ] ;
import-list    = module-path , [ "as" , identifier ] , { "," , module-path , [ "as" , identifier ] } ;
import-item    = identifier , [ "as" , identifier ] ;
from-import    = "from" , module-path , "import" , ( "*" | import-item , { "," , import-item } ) , [ ";" ] ;
module-path    = identifier , { "." , identifier } ;
```

| Form | Example | Emits |
| --- | --- | --- |
| Module | `import stdlib.math` | `import stdlib.math` |
| Module alias | `import stdlib.math as m` | `import stdlib.math as m` |
| Names | `from stdlib.math import sqrt, PI` | `from stdlib.math import sqrt, PI` |
| Name alias | `from stdlib.math import sqrt as root` | `from stdlib.math import sqrt as root` |
| Brace form | `import stdlib.math { sqrt, PI }` | `from stdlib.math import sqrt, PI` |
| Wildcard | `from stdlib.math import *` | `from stdlib.math import *` |
| Multiple | `import a, b as c` | `import a, b as c` |

```aura
import stdlib.math
print(stdlib.math.sqrt(16))  // 4.0

import stdlib.math as m
print(m.sqrt(25))            // 5.0

from stdlib.math import sqrt, PI
print(sqrt(36))              // 6.0

import stdlib.math { sqrt, PI }   // equivalent to from stdlib.math import sqrt, PI
print(PI)                         // 3.141592653589793
```

Rules:

- The **brace form cannot be combined with `as`** on the same statement
  (`modules.md` §16). An alias on the module plus selected names
  (`import a.b as c { x }`) is handled by emitting both an aliased import and a
  `from ... import` (`transform_ImportStmt`, `statements.py:1358-1364`).
- There is **no `::` separator**; a module path is dotted
  (`to_ast.py:1828-1830`).
- A wildcard is written only as `from module import *`; there is no `*` in the
  brace form (`modules.md` §16).
- Imports are **top-level declarations**; the parser reads them before/in the
  program statement stream. Relative and parent-relative imports are not part
  of the grammar.

*Evidence:* `syntax.md`6; *probe* (each form transpiles as the table
shows).

### 4.1 Local `.aura` files and packages

A plain Aura file is importable with the same syntax. `import util` binds the
file `util.aura`; `import pkg.util` binds `pkg/util.aura`; a file imported as a
module **needs no `main`** (`modules.md` §16.2, §1). The runtime installs an
import hook that maps the dotted Aura path to the sibling `.aura` file
(`install_aura_import_hook`, called from `_install_aura_imports`,
`cli.py:118-127`).

```aura
// lib.aura — a plain module file needs no export marker at top level
def greet(name: str) -> str {
  return "hi " + name
}

const VERSION = "9"
```

```aura
// app.aura
import lib
from lib import greet as g

def main() {
  print(lib.greet("ana"))   // hi ana
  print(g("bob"))           // hi bob
}
```

*Evidence:* *probe* (prints `hi ana` / `hi bob`, exit 0);
`test_module_facade.py::TestImportedFileRules::test_imported_library_needs_no_main`;
`test_modules.py::TestModuleImports::test_module_in_another_file`.

A top-level `export` is **only meaningful inside a `module` body**
(`to_ast.py:775-780`). A sibling file that a facade re-exports is written with
plain top-level declarations, not `export` (see §6).

*Probe:* `export def f() ...` at the top level of an imported file raises
`SyntaxError: 'export' is only meaningful inside a 'module' body`.

### 4.2 Python interop imports (`py.`)

A module path prefixed with `py.` marks a **host Python** import rather than an
Aura module (`_split_python_prefix`, `to_ast.py:494-505`;
`_transform_python_import`, `statements.py:1371-1388`). This is covered in
[python-interop.md](python-interop.md) §1–2.

```aura
import py.re             // binds `re`  (last path segment)
from py.math import pi   // binds `pi`
```

---

## 5. Name resolution

Inside a module body a bare member name resolves to the module's own namespace;
the transformer rewrites it to `Module.name` for the duration of the body
(`_module_scopes`, `statements.py:1531-1535`). A private member resolves to its
mangled name (`_module_privates`, §1536-1545).

For an identifier `x` written **outside** a module, the resolution order is:

```text
local scope (function → block chain)
  → enclosing function scope / closure cell
  → top-level declarations in the file
  → the module of an importing alias or a `from` binding
  → an Aura sibling file loadable by the import hook
  → a `py.` host module / the `python` bridge
  → otherwise: name error at runtime, or E3xx at check time
```

- Inside `module M`, `M`'s own members win over same-named outer declarations;
  the rule checker keeps nested module scopes on a stack (`_module_depth`,
  `_check_duplicate_members`, `rules.py:623-631`, §910).
- A module member may share a top-level name without conflict.

*Evidence:* `test_modules.py::TestModuleRules::test_module_member_may_share_a_top_level_name`.

> **UNSPECIFIED:** the language does not define a total, collision-resolving
> name-resolution order across `import`/`from` bindings and top-level
> declarations. Resolution ultimately follows the emitted Python name binding
> (last binding wins in the module namespace). Aura does not diagnose an
> ambiguous import.

---

## 6. Module facades (re-exports)

A module can act as the **facade** for a source folder: a bare `export Name`
(no `def`/`class`) re-exports a symbol defined in a sibling file. Put the
facade in a folder named after itself, so `App/App.aura` is the entry point of
the `App` package (`modules.md` §9; `_parse_item_export`,
`to_ast.py:1633-1651`).

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
  print(App.Components("header").describe())  // a class from components.aura
  print(App.Utils.double(21))                 // the utils module as a namespace
  print(App.Utils.VERSION)
}
```

*Evidence:* `test_module_facade.py::TestFacadeRuntime::test_export_class_and_module`
(prints `component:header` / `42` / `1.0.0`).

### 6.1 Resolution order

A re-exported name is resolved by convention, **deterministically**, against the
folder holding the facade (`resolve_reexport`, `modules.py:93-130`):

1. a declaration in the same file (the facade re-exports its own member);
2. a sibling file whose stem matches, case-insensitively
   (`Components` → `components.aura`);
3. a subfolder named after the symbol (`Components/Components.aura` or
   `Components/__init__.aura`).

When the resolved file **declares** the name, the binding is that declaration
(`App.Components` is the class). When it does not, the whole sibling module is
exposed as the namespace (`App.Utils` is the `utils` module), so its functions
and constants are reached as `App.Utils.double(...)` (`declares_name`,
`modules.py:125-129`; bindings built in `_module_reexport_lines`,
`statements.py:1616-1622`).

Resolution is **static and confined to the facade's folder**: a path must stay
inside it, so a re-export can never read outside the project
(`_safe_child`, `modules.py:41-51`). Nothing is executed at resolve time; only
the sibling's text is scanned for a declaration (`_defines`, §54-60).

### 6.2 Explicit source

An explicit source is accepted with `export Name from "module"`
(`_parse_item_export`, `to_ast.py:1639-1648`; `_candidate_files`,
`modules.py:63-75`):

```aura
module App {
  export Widgets from "widgets"      // resolves widgets.aura
  export X from "pkg.sub"            // resolves pkg/sub.aura
}
```

The path must be a **plain dotted name**: separators (`/`, `\`), traversal
(`..`) and absolute paths are rejected at parse time
(`_INVALID_MODULE_PATH`, `to_ast.py:11-13`, §1646-1648).

*Evidence:* `test_module_facade.py::test_export_from_with_traversal_is_rejected`,
`::test_export_from_with_separator_is_rejected`,
`::test_reexport_source_cannot_traverse`,
`::test_reexport_cannot_reach_absolute_path`.

### 6.3 Diagnostics

| Condition | Code | Where |
| --- | --- | --- |
| A re-export resolves to no sibling source or local declaration | **E313** | `_check_reexports`, `rules.py:439-467` |
| A `main` inside a module (including a facade) | **E312** | `rules.py:246-265` |
| Two re-exports resolving to different files under one name | conflict | `find_reexport_conflicts`, `modules.py:133-151` |

```aura
module App { export Missing }   // E313: module 'App' exports 'Missing', ...
```

*Evidence:* `test_module_facade.py::TestFacadeDiagnostics::test_unresolved_reexport_reports_e313`,
`::test_main_inside_a_facade_reports_e312`.

> **TARGET-SPECIFIC:** a package facade (`App/App.aura`) emits its members at
> **module level**, so Python imports `App` directly; a facade in a plain file
> (`facade.aura`) keeps a namespaced class (`_is_package_facade`,
> `statements.py:1503-1513`). Sibling imports are hoisted out of the class body
> to avoid shadowing (`statements.py:1519-1527`, §1625-1642).

---

## 7. What is NOT part of modules

| Not supported | Instead |
| --- | --- |
| `namespace` keyword | `module` |
| `export ... as` re-export alias | `from` binding with an alias |
| `import a.b as c { x, y }` as a single unambiguous form | supported by emitting two Python statements (*ambiguous surface*) |
| Relative import (`import .sibling`) | dotted local path (`import pkg.sibling`) |
| `::` separator | dotted paths |
| `*` in the brace form | `from module import *` |
| `main` inside a module | top-level `main` in the entry file (E312) |

---

## 8. Cross-references

- [python-interop.md](python-interop.md) — `import py.x`, `from py.x import y`,
  and the runtime `python` bridge.
- [functions.md](functions.md) §9 — the `main` entry point (E310/E311/E312).
- [statements.md](statements.md) §13 — the rule table enforced by the checker.
- [semantics.md](semantics.md) — execution model and the diagnostic model.
