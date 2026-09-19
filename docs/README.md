---
layout: default
title: "Aura Documentation"
nav_exclude: true
---

# Aura Documentation

The complete documentation for the Aura language and toolchain. If you are new
here, start with the [learning path](learn/README.md), then use the
[language reference](language-reference/README.md) for exact rules.

---

## Learning the language

| Document | What it covers |
|----------|----------------|
| [learn/](learn/README.md) | A numbered tutorial path — **start here** |
| [language-reference/](language-reference/README.md) | The language reference (lexical, grammar, types, expressions, statements, classes, modules, semantics, interop) |
| [language-reference/grammar.md](language-reference/grammar.md) | Canonical EBNF grammar (the source of truth for syntax) |
| [ERRORS.md](ERRORS.md) | Diagnostics reference: every `E##`/`W##` code |
| [AUP.md](AUP.md) | Aura Patterns: idiomatic solutions to common problems |

## Reference and internals

| Document | What it covers |
|----------|----------------|
| [DESIGN.md](DESIGN.md) | Transpiler architecture and project structure |
| [COMPLETENESS.md](COMPLETENESS.md) | Language coverage percentages and remaining gaps |
| [AUDIT.md](AUDIT.md) | Historical audit: bugs fixed, performance work, doc alignment |

## Project

| Document | What it covers |
|----------|----------------|
| [CHANGELOG.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/CHANGELOG.md) | Notable changes by version |
| [README.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/README.md) | Project overview and quick start |
| [CONTRIBUTING.md](https://github.com/JoaoValentimTheo/aura-lang/blob/master/CONTRIBUTING.md) | Contribution workflow |

---

## Where to go next

**Learning Aura** — read [learn/](learn/README.md), then work through
[`examples/`](https://github.com/JoaoValentimTheo/aura-lang/tree/master/examples)
in order. The [AUP catalog](AUP.md) shows idiomatic solutions once the basics
are clear.

**Writing a first program**

```bash
aura init myapp
aura run myapp/src/main.aura
```

**Contributing** — read [DESIGN.md](DESIGN.md) for the compiler architecture,
then explore `aura/parser/`, `aura/transpiler/`, and `tests/`. Run
`pytest`, `ruff check aura/`, and `mypy aura/` before opening a pull request.

**Reporting a bug** — include the Aura source, the command you ran, and the
full output. Open an issue at
<https://github.com/JoaoValentimTheo/aura-lang/issues>.
