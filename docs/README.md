# Aura Documentation

The complete documentation for the Aura language and toolchain. If you are new
here, read [LANGUAGE.md](LANGUAGE.md) and then run the programs in
[`examples/`](https://github.com/JoaoValentimTheo/aura-lang/tree/master/examples).

---

## Learning the language

| Document | What it covers |
|----------|----------------|
| [LANGUAGE.md](LANGUAGE.md) | Complete syntax reference (English) — **start here** |
| [LANGUAGE_PT.md](LANGUAGE_PT.md) | Referência completa de sintaxe (Português) |
| [GRAMMAR.md](GRAMMAR.md) | Canonical EBNF grammar (the source of truth for syntax) |
| [TYPES.md](TYPES.md) | Type system reference (English) |
| [TYPES_PT.md](TYPES_PT.md) | Referência do sistema de tipos (Português) |
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

**Learning Aura** — read [LANGUAGE.md](LANGUAGE.md), then work through
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