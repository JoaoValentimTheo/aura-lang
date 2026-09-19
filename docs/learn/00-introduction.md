---
layout: default
title: "00 — Introduction"
parent: Learn Aura
nav_order: 10
---

[English](00-introduction.md) · [Português](00-introduction.pt_BR.md)

# 00 — Introduction

> **Aura 0.2.0a4.** Aura transpiles to Python and runs on CPython. This chapter
> explains the mental model; chapters 01–16 teach the language.

## What Aura is

Aura is a **programming language** with its own syntax, transpiled to Python
source and executed by CPython. When you write:

```aura
def main() {
  print("Hello, Aura!")
}
```

the toolchain parses it, checks it, emits Python, and runs that Python. The
language is small on purpose; the power comes from the target, since **every
Python module is reachable** (chapter 14).

Three levels are kept separate, and the distinction matters all through this
tutorial:

| Level | What it is |
|---|---|
| **Aura** | the language — the rules you write against |
| **The Aura toolchain** | the parser, checker, transpiler and CLI (`aura ...`) |
| **CPython** | the target that actually runs the emitted Python |

## Why transpile to Python?

Because you get Python's ecosystem and CPython's runtime without Python's
surface syntax. A library you already have — `json`, `re`, `requests`, a web
framework — is callable from Aura through the `py.` prefix:

```aura
import py.math as math

def main() {
  print(math.sqrt(16))   // 4.0
}
```

## What Aura changes, and what it keeps

Aura is **not** a Python dialect with a few renames. The mental model is a
language that deliberately:

* requires `let`/`let mut`/`const` instead of bare assignment, so *mutability is
  visible at the declaration* (`x = 1` alone is not a declaration);
* uses `def` — there is no `fn`, `fun`, `lambda`, or `function`;
* writes types with brackets: `Box[T]`, `[int]`, `{str: int}`; `<T>` does not
  exist;
* spells the null value `none`; `null`, `None` and `True`/`False` are rejected;
* uses `and`, `or`, `not`; `&&`, `||`, `!` are rejected;
* uses `extends` for inheritance; `implements` and `class C(A)` are rejected;
* makes every class member declare a visibility (`public`/`private`/`protected`);
* has real language features Python lacks as syntax: `match` with guards and
  destructuring, `unless`, `until`, `guard`, `loop`, `?.`, `??`, `?:`, `|>`.

Because the target is CPython, some behaviour is inherited rather than invented:

| Behaviour | Rule |
|---|---|
| Integers | arbitrary precision (Python `int`) |
| `7 / 2` | `3.5` — true division; there is **no** `//` floor operator |
| `-7 % 3` | `2` — sign follows the divisor |
| Dict ordering | insertion order |
| Object lifetime | CPython reference counting + GC |
| `str` | Unicode |

These are **target-specific** rules, documented in the reference rather than
hidden. See
[`../language-reference/semantics.md`](../language-reference/semantics.md) §6.

## A first real program

```aura
def classify(score) -> str {
  match score {
    case s if s >= 90 { return "A" }
    case s if s >= 80 { return "B" }
    case _ { return "F" }
  }
  return "?"
}

def main() {
  for score in [95, 83, 40] {
    print(f"{score}: {classify(score)}")
  }
}
```

```text
95: A
83: B
40: F
```

This already shows several Aura ideas at once: `def`, `match` with guards,
f-strings, `for ... in`, and a `main` entry point.

## The entry point

A file executed with `aura run` **must** declare a top-level `def main()`. The
runtime calls `main` for you — never write a trailing `main()` call. A file used
only as an import (a module) needs no `main`.

```aura
def main() {
  // your program
}
```

`main` may take no parameters or a single `args` parameter, may be `async`, and
may return an `int` to set the process exit code. Reference:
[`../language-reference/functions.md`](../language-reference/functions.md) §9.

## How this path is organised

* Chapters **00–06** teach the core: variables, types, control flow, functions.
* Chapters **07–08** cover object orientation.
* Chapters **09–10** cover collections and functional programming.
* Chapters **11–12** cover pattern matching and errors.
* Chapters **13–16** cover modules, Python interop, decorators/macros, and the
  CLI.

Every chapter ends with a **Next step** link and, where relevant, a pointer to
the exact rule in the language reference. The tutorial teaches; the reference
defines. When you need a precise answer about a construct, go to the reference.

## A 60-second tour

This one file touches most of the language. You are not expected to understand
all of it yet — the chapters that follow take it apart:

```aura
const GREETING = "Hello"

class Person {
  public let name: str = ""
  public def new(name: str) { self.name = name }
  public def greet() -> str { return GREETING + ", " + self.name }
}

def main() {
  let people = [Person("Ana"), Person("Bob")]
  let greetings = people |> map((p) => p.greet())
  for g in greetings { print(g) }

  let score = 83
  match score {
    case s if s >= 90 { print("A") }
    case s if s >= 80 { print("B") }
    case _ { print("F") }
  }

  try {
    throw ValueError("boom")
  } catch Error as e {
    print(f"caught: {e}")
  } finally {
    print("done")
  }
}
```

```text
Hello, Ana
Hello, Bob
B
caught: boom
done
```

## Next step

[Installation →](01-installation.md)
