---
layout: default
title: "01 — Installation"
parent: Learn Aura
nav_order: 11
---

[English](01-installation.md) · [Português](01-installation.pt_BR.md)

# 01 — Installation

> **Aura 0.2.0a5.** This chapter covers installing the toolchain, creating a
> project and running it. The commands do not depend on the version.

## What you need

Aura runs on CPython, so you need **Python 3.10+**. Everything else — the
parser, checker, transpiler, CLI and standard library — comes with the package.

## Step 1 — Install the package

Aura is published on PyPI. The current line is a pre-release, so pass `--pre`:

```bash
python -m pip install --pre aura-language
```

Create an isolated environment first if you prefer:

```bash
python -m venv .venv
source .venv/bin/activate        # macOS/Linux
# .venv\Scripts\activate         # Windows
python -m pip install --pre aura-language
```

## Step 2 — Verify the installation

```bash
aura --version
aura --help
```

`aura --help` lists the available commands:

| Command | What it does |
|---|---|
| `aura run <file.aura>` | transpile and execute a program |
| `aura check <file.aura>` | check types and rules without running |
| `aura repl` | interactive read-eval-print loop |
| `aura init [name]` | create a starter project |
| `aura transpile <file.aura>` | print (or write) the generated Python |
| `aura format <file.aura>` | format source |
| `aura lint <file.aura>` | report style warnings |
| `aura test <dir>` | run `.aura` test files |
| `aura debug <file.aura>` | run under the trace debugger |
| `aura lsp` | start the language server (stdio) |
| `aura add` / `remove` / `install` / `deps` / `venv` | dependencies |
| `aura doctor` | check the project environment |
| `aura version` | show or bump the version |

Chapters 02 and 16 use these; the rest only need `run`, `check` and `repl`.

## Step 3 — Create a project with `aura init`

```bash
aura init demo
```

This writes a manifest and a starter program:

```text
✓ Created aura.toml
✓ Created src/main.aura
```

`aura.toml` names the project and its Python dependencies:

```toml
[project]
name = "demo"
version = "0.1.0"

[dependencies]
```

`src/main.aura` is the smallest valid program:

```aura
def main() {
  print("Hello from Aura!")
}
```

`aura init --venv` also creates `.venv` and installs declared dependencies. To
add a Python dependency later, use `aura add <package>`; it records the
requirement and installs it into the project environment.

## Step 4 — Run it

```bash
aura run src/main.aura
```

```text
Hello from Aura!
```

## Step 5 — Check without running

`aura check` runs the type and rule checkers and reports diagnostics without
executing the program. It takes a **file**, not a directory:

```bash
aura check src/main.aura
```

```text
OK src/main.aura: type check passed
```

A failing check exits non-zero and prints a positioned message:

```text
src/main.aura:3:3: ERROR [E303]
  Cannot reassign immutable binding 'x'; ...
  hint: write 'let mut x' at its declaration
```

Diagnostic codes (`E303`, `E310`, `E319`, ...) are catalogued in
[`../ERRORS.md`](../ERRORS.md).

## Step 6 — Try the REPL

```bash
aura repl
```

The REPL evaluates Aura expressions and statements interactively. It runs the
same mutability and rule checks as `aura run`, so a `let` you reassign is an
error there too.

## Installing from source (contributors)

```bash
git clone https://github.com/joao/aura-lang.git
cd aura-lang
python -m pip install -e .
```

An editable install picks up changes to the transpiler without reinstalling.

## Common problems

| Symptom | Fix |
|---|---|
| `aura: command not found` | the environment is not active, or pip's scripts directory is not on `PATH` |
| `pip` installs an old version | re-run with `--pre` to allow pre-releases |
| `E310: no main` on `aura run` | the file has no top-level `def main()`; imported module files need none |
| `aura check <dir>` errors | pass a single `.aura` file; use `aura test <dir>` for a directory of tests |

## Next step

[First Program →](02-first-program.md)
