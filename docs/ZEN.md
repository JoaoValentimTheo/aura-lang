# The Aura Manifesto — Zen-to-Win

Aura is built on one asymmetry:

> **Zen-to-win optimizes the complexity exposed to the user; engineering
> judgment optimizes the necessary internal complexity.**

The language you read is small on purpose. The compiler that runs it is allowed
to be as rigorous as it needs to be to make that smallness real, correct, and
trustworthy. A simple language does **not** mean a simple compiler.

---

> **Compacto é melhor que confuso.
> Simples é melhor que complicado.
> Claro é melhor que mágico.**
>
> **Se existe mais de uma forma de fazer o simples,
> então já não é simples.**
>
> **Menos exceções,
> menos cerimônia,
> menos magia,
> mais clareza.**
>
> **Para humanos lerem.
> Para máquinas entenderem.
> Para IAs gerarem.
> Para todos corrigirem.**
>
> **Não buscamos ter tudo.
> Buscamos precisar de pouco.**
>
> **Zen-to-win.**
>
> **Pequeno o bastante para entender.
> Claro o bastante para confiar.
> Poderoso o bastante para criar.**

---

## What this means for the language

* **One construct, one spelling.** `and` exists; `&&` does not. `fn` exists;
  `def`, `function`, and `func` do not. A synonym is treated as a defect, not a
  convenience.
* **One obvious meaning.** `^` is exponentiation, never XOR. `|` is bitwise OR,
  and in a type position a union. There is no overloading of meaning that the
  reader has to disambiguate by context they cannot see.
* **Few exceptions.** A trailing comma is allowed before every closing
  delimiter. Mutating through a binding requires `mut`, whether the write is
  `xs[0] = v`, `m[k] = v`, `s.f = v`, `push(xs, v)`, or a `mut self` method —
  one rule, not a table of cases.
* **Nothing accidental.** If a construct is absent, it is absent on purpose and
  recorded as such: no inheritance, no implicit conversions, no `++`/`--`, no
  hidden dynamic dispatch. Absence is a decision, documented as one.

## What this means for the compiler

The Rust core is deliberately not minimal. It is:

* **explicit** — the mutation rule is a capability carried by a binding and
  checked at the place's root, not a special case bolted onto individual
  operations;
* **defensive** — every limit reports a stable `E####` diagnostic, and no
  well-formed program may panic, overflow the stack, or behave
  non-deterministically;
* **deterministic** — diagnostics are selected by declaration order, never by a
  hash map's iteration;
* **portable** — the native and WebAssembly runtimes share one parser, one
  checker, and one interpreter, and are verified to agree;
* **user-centered in its errors** — a diagnostic names the real source
  location, exactly, including inside an f-string interpolation.

## The asymmetry

```text
simple Aura
      ↓
precise semantics
      ↓
exceptionally strong Rust compiler
      ↓
predictable runtime
      ↓
native / WASM consistency
      ↓
trust
```

The goal is not *simple language + simple compiler*. The goal is a language a
person can hold in their head, implemented by a compiler that can be held to a
standard.
