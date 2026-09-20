---
layout: default
title: "Aura Language Reference"
nav_order: 3
has_children: true
---

[English](index.md) · [Português](index.pt_BR.md)

# Aura Language Reference

**Version:** 0.2.0a6 · **Extracted from:** `aura/parser/to_ast.py` and
`aura/transpiler/` · **Model:** mirrors the Kof language reference.

This directory is the **Aura language reference**. It describes *what a valid
Aura program is* and *what that program means*, independently of the toolchain
that implements it.

> **Founding rule:** nothing here is invented. Every rule is extracted from the
> parser, from the transpiler, from the tests, or from behaviour verified by
> execution. Where behaviour could not be determined with confidence, the rule
> is marked **UNSPECIFIED** rather than guessed.

---

## Language ≠ Compiler ≠ Target

Three distinct levels, kept separate on purpose:

- **Aura** is the *language* — a set of rules.
- **The Aura toolchain** (`aura/parser`, `aura/transpiler`, the CLI) is *one
  implementation* of the language.
- **CPython** is the *target*: Aura transpiles to Python. When an observable
  behaviour comes from the host (e.g. `%` sign, iterator protocol), it is
  documented here as a *target-dependent* rule, never hidden.

---

## What each document answers

| Document | Question it answers |
|---|---|
| [lexical-structure.md](lexical-structure.md) | What are the valid tokens? (identifiers, literals, operators, comments, keywords) |
| [grammar.md](grammar.md) | What is the formal grammar? (EBNF, precedence, associativity) |
| [syntax.md](syntax.md) | How is each construct written? (concrete form, examples) |
| [types.md](types.md) | What types exist and how are they written? |
| [type-system.md](type-system.md) | What operations are valid? When is there a type error? |
| [expressions.md](expressions.md) | Semantics of each expression and operator. |
| [statements.md](statements.md) | Semantics of each statement and control flow. |
| [functions.md](functions.md) | Declaration, parameters, return, recursion, entry point. |
| [closures.md](closures.md) | Lambdas, function types, variable capture. |
| [classes.md](classes.md) | Classes, traits, abstract classes, inheritance, visibility. |
| [modules.md](modules.md) | Modules, imports, re-exports, name resolution. |
| [semantics.md](semantics.md) | Execution model, evaluation order, scope, lifetime, errors. |
| [python-interop.md](python-interop.md) | Host-Python access through the `py.` namespace. |

The **toolchain architecture** has its own document:
[../DESIGN.md](../DESIGN.md).

---

## Status labels

| Label | Meaning |
|---|---|
| **Stable** | Defined by the language and frozen (the syntax freeze, `0.2.0a1`). Does not change without a version bump + migration. |
| **Experimental** | Implemented and testable, but subject to change. |
| **Target-specific** | Behaviour comes from the CPython host, documented as a rule. |
| **Unspecified** | The language does not yet define this point. |

---

## How this reference is verifiable

Every normative statement points to **evidence**:

- **Code** — `aura/parser/to_ast.py:line` or `aura/transpiler/...:line`.
- **Test** — a test in `tests/` that demonstrates the rule.
- **Execution** — behaviour observed by running a program (*probe*), used when
  there is no dedicated test.

When code, test and documentation diverge, the divergence is a bug to fix —
never silently resolved in favour of one source.

---

## Learning path

New to Aura? Read [`../learn/`](../learn/index.md) first — a numbered tutorial
that builds up the language, then use this reference for exact rules.
