---
layout: default
title: "Learn Aura"
nav_order: 2
has_children: true
---

[English](index.md) · [Português](index.pt_BR.md)

# Learn Aura

Welcome to the Aura learning path.

Aura is a small, readable programming language that **transpiles to Python** and
runs on CPython. It keeps Python's ecosystem — every installed Python module is
reachable — while offering a cleaner, more explicit surface syntax: `let`/`const`
instead of bare assignment, `def` for functions, classes with mandatory
visibility, pattern matching, lambdas with `=>`, a pipe operator, and a null
value called `none`.

Because the target is CPython, an Aura program is ordinary Python at runtime:
`int` is Python's arbitrary-precision integer, `/` is true division, `%` follows
the divisor's sign, and dictionaries keep insertion order. The differences are in
the **syntax and the checking**, not in a new runtime.

## What Aura is today

**Version:** `0.2.0a5`. The language reference in
[`../language-reference/`](../language-reference/index.md) is the exact source
of truth; this tutorial builds intuition and links there for detail.

* Gradually typed: annotations are optional and, except for the checks in the
  type checker, **erased** before the Python emit.
* Functions declared with `def`, expression bodies (`def f(x) = x * 2`),
  defaults, variadics, recursion, generics (`def id[T](x: T) -> T`).
* Classes with header fields, manual `def new`, visibility
  (`public`/`private`/`protected`), `extends` inheritance, `trait`,
  `abstract class`, `enum`, `@property`/`@staticmethod`/`@classmethod`.
* Collections: lists `[T]`, dicts `{K: V}`, sets, tuples, slices, and
  comprehensions.
* Lambdas (`x => x * 2`), closures, function types, and the pipe operator `|>`.
* `if`/`else if`, `unless`, `guard ... else`, `while`, `until`, `loop`, `for`
  with ranges and labels, and `match` with guards and destructuring.
* `try`/`catch`/`finally`, `throw`, `assert`, `with`.
* Modules (`module Name { export ... }`), local `.aura` imports, and Python
  interop through the `py.` prefix and the `python` bridge.
* Decorators: `@debug`, `@timeit`, `@memoize`, `@cache(...)`, plus compile-time
  macros (`assert_eq`, `static_swap`, `stringify`, ...).

## Who it is for

* **Beginners** — start at chapter 00 and follow the order. Each chapter is
  short and every snippet runs.
* **Python developers** — start with the introduction, then read what is
  *different* from Python (chapters 03–06 are mostly familiar).
* **Contributors** — after the tutorial, read the language reference and
  [`../DESIGN.md`](../DESIGN.md).

## Recommended order

```
00 → 01 → 02 → 03 → 04 → 05 → 06 → 07 → 08 → 09
   → 10 → 11 → 12 → 13 → 14 → 15 → 16
```

Chapters 00–06 are the core language. 07–08 cover object orientation. 09–10
cover data and functional programming. 11–12 cover control and failure. 13–16
cover modules, Python interop, decorators/macros and the toolchain.

## Index

| # | Chapter | One line |
|---|---------|----------|
| 00 | [Introduction](00-introduction.md) | What Aura is, and how it relates to Python |
| 01 | [Installation](01-installation.md) | Install, create a project, run it |
| 02 | [First Program](02-first-program.md) | `hello.aura` line by line |
| 03 | [Language Basics](03-language-basics.md) | `let`, `let mut`, `const`, `def`, `main`, `print` |
| 04 | [Variables and Types](04-variables-and-types.md) | The type catalog and annotations |
| 05 | [Control Flow](05-control-flow.md) | `if`, loops, `match` overview |
| 06 | [Functions](06-functions.md) | Parameters, defaults, variadics, recursion |
| 07 | [Classes and Objects](07-classes-and-objects.md) | Constructors, fields, methods, properties |
| 08 | [Traits and Abstract Classes](08-traits-and-abstract-classes.md) | Contracts and polymorphism |
| 09 | [Collections](09-collections.md) | Lists, dicts, sets, tuples, comprehensions |
| 10 | [Lambdas and Functional](10-lambdas-and-functional.md) | `=>`, closures, `map`/`filter`/`reduce`, `&#124;>` |
| 11 | [Pattern Matching](11-pattern-matching.md) | `match`, guards, destructuring, enums |
| 12 | [Error Handling](12-error-handling.md) | `try`/`catch`, `throw`, `guard`, `assert` |
| 13 | [Modules and Imports](13-modules-and-imports.md) | `module`, `export`, imports |
| 14 | [Python Interop](14-python-interop.md) | `py.` imports and the `python` bridge |
| 15 | [Macros and Decorators](15-macros-and-decorators.md) | Runtime decorators and compile-time macros |
| 16 | [CLI and Tooling](16-cli-and-tooling.md) | `run`, `check`, `repl`, `init`, `format`, ... |

## Runnable examples

The repository ships runnable programs under [`../../examples/`](https://github.com/JoaoValentimTheo/aura-lang/tree/master/examples).
Each chapter points to the example closest to its topic; the full set is:

| Example | Chapter | What it shows |
|---|---|---|
| `hello.aura` | 02 | the smallest program |
| `fibonacci.aura` | 05, 06 | loops and mutable bindings |
| `prime_checker.aura` | 05, 09 | functions and comprehensions |
| `functional.aura` | 10 | lambdas, `&#124;>`, closures |
| `pattern_matching.aura` | 11 | `match` with guards and destructuring |
| `error_handling.aura` | 12 | `try`/`catch`/`finally`, `guard`, `throw` |
| `classes.aura` | 07, 08 | classes, `extends`, `@property`, visibility |
| `macros.aura` | 15 | built-in runtime decorators |
| `compile_time_macros.aura` | 15 | compile-time macro expansions |
| `crypto.aura` | — | `stdlib.crypto` hashing and post-quantum primitives |
| `python_interop.aura` | 14 | `py.` imports and the `python` bridge |
| `abstract_classes.aura` | 08 | `abstract class` and polymorphism |
| `tour.aura` | all | a single-file tour of the language |

Run any of them directly:

```bash
aura run examples/tour.aura
```

## Status labels used in this path

* **Verified** — the snippet was executed with `aura run` while writing this
  tutorial.
* **Reference** — a link to the exact rule lives in
  [`../language-reference/`](../language-reference/index.md); the tutorial does
  not repeat it in depth.
* **Limitation** — behaviour the reference describes but the current build
  rejects or does not enforce; called out where it matters.
* **Gotcha** — a common mistake or surprising behaviour worth knowing about.
* **UNSPECIFIED** — the language does not yet define this point; behaviour may
  change.
* **TARGET-SPECIFIC** — behaviour comes from the CPython host, documented as a
  rule.

Start here: [Introduction →](00-introduction.md)
