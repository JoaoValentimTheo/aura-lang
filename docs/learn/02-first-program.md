---
layout: default
title: "02 — First Program"
parent: Learn Aura
nav_order: 12
---

[English](02-first-program.md) · [Português](02-first-program.pt_BR.md)

# 02 — First Program

> **Chapter goal:** write, run and understand the smallest Aura program. Example:
> [`../../examples/hello.aura`](https://github.com/JoaoValentimTheo/aura-lang/blob/master/examples/hello.aura).

## The program

Create a file `hello.aura`:

```aura
def main() {
  print("Hello, Aura!")
}
```

Run it:

```bash
aura run hello.aura
```

Output:

```text
Hello, Aura!
```

That is a complete Aura program. Every piece matters:

| Piece | Meaning |
|---|---|
| `def` | declares a function; it is the **only** function keyword |
| `main` | the entry point — the runtime calls it for you |
| `()` | `main` takes no parameters |
| `{ ... }` | the function body, a block of statements |
| `print(...)` | writes its argument to standard output |
| `"..."` | a string literal |

## Why there is no `main()` call

In many languages you write `main()` or `if __name__ == "__main__":`. In Aura,
the runtime invokes `main` itself. Writing a trailing `main()` is unnecessary,
and **omitting `main` in an executed file is error `E310`**.

A file that is imported as a module (chapter 13) needs no `main` at all.

## Statements and newlines

Statements are separated by Aura's grammar, and a terminating `;` is
**optional**. Newlines have no syntactic meaning, so both of these are the same
program:

```aura
def main() {
  print("one")
  print("two")
}
```

```aura
def main() { print("one"); print("two") }
```

Pick one style and be consistent; `aura format` normalises to two-space
indentation and braces on the same line.

## Comments

```aura
// a line comment

/* a block comment
   spanning lines */
```

`//` always starts a line comment — Aura deliberately has **no** `//` floor
division operator. Use `int(a / b)` for an integer quotient.

## Printing more

`print` accepts several arguments (they are printed space-separated):

```aura
def main() {
  let name = "world"
  print("Hello,", name)
  print(f"Hello, {name}!")
}
```

```text
Hello, world
Hello, world!
```

The `f"..."` form is an **f-string**: `{...}` embeds an expression. F-strings
also support format specs and `!r`/`!s`/`!a` conversions:

```aura
def main() {
  let pi = 3.14159
  print(f"pi is about {pi:.2f}")
}
```

```text
pi is about 3.14
```

## A slightly larger first program

```aura
def greet(name, punctuation = "!") -> str {
  return "Hello, " + name + punctuation
}

def main() {
  print(greet("world"))
  print(greet("Aura", "?"))
}
```

```text
Hello, world!
Hello, Aura?
```

Two new ideas appear here: a second function, and a **default parameter**
(`punctuation = "!"`, used when the caller omits it). `-> str` is the return
type annotation. Functions are chapter 06; for now, notice that Aura code reads
top to bottom and every construct is explicit.

## Seeing the generated Python

Because Aura transpiles, you can inspect the target:

```bash
aura transpile hello.aura
```

The output is ordinary Python. This is a useful way to confirm what a construct
does — and a reminder that the runtime is CPython.

## What you learned

* `def main() { ... }` is the entry point; the runtime calls it.
* `print` writes output; f-strings interpolate expressions.
* Statements end at grammar boundaries; `;` is optional.
* `//` and `/* ... */` are comments.
* `aura run` executes, `aura transpile` shows the Python.

## Next step

[Language Basics →](03-language-basics.md)
