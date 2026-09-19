---
layout: default
title: "16 — CLI and Tooling"
parent: Learn Aura
nav_order: 26
---

[English](16-cli-and-tooling.md) | [Português](16-cli-and-tooling.pt_BR.md)

# 16 — CLI and Tooling

> **Chapter goal:** use the `aura` CLI to run, check, format, test and inspect
> programs. Reference:
> [`../language-reference/semantics.md`](../language-reference/semantics.md) §8.

## `aura run`

Transpile and execute a program:

```bash
aura run src/main.aura
aura run src/main.aura --            # pass program arguments
aura run src/main.aura -v            # also print the generated Python
```

`main` receives the arguments in its single `args` parameter. If it returns an
`int`, that becomes the process exit code.

```aura
def main(args: [string]) {
  print(f"{args.length()} argument(s)")
}
```

```bash
aura run app.aura one two
```

```text
2 argument(s)
```

## `aura check`

Check types and rules **without** running. It takes a **file**:

```bash
aura check src/main.aura
```

```text
OK src/main.aura: type check passed
```

Diagnostics carry a stable code (`E303`, `E310`, `E319`, ...) and an optional
hint. `aura check` reports `E1xx` type errors that `aura run` does not — the run
path executes only the mutability and rule checks. Codes are catalogued in
[`../ERRORS.md`](../ERRORS.md).

## `aura repl`

An interactive prompt that evaluates Aura and runs the same mutability/rule
checks as `aura run`:

```bash
aura repl
```

Useful for trying expressions and checking what a construct evaluates to.

## `aura init`

Create a starter project:

```bash
aura init demo           # writes aura.toml and src/main.aura
aura init demo --venv    # also creates .venv and installs dependencies
```

## `aura transpile`

Print (or write) the generated Python:

```bash
aura transpile src/main.aura              # to stdout
aura transpile src/main.aura -o out.py    # to a file
aura transpile src/main.aura -v           # also show the AST
```

This is the clearest way to confirm what a construct does at runtime.

## `aura format`

Normalise source to two-space indentation, braces on the same line, and one
space around binary operators:

```bash
aura format src/main.aura              # print formatted source
aura format src/main.aura -i           # rewrite in place
aura format src/main.aura -o clean.aura
```

String literal contents are never rewritten.

## `aura lint`

Report style and convention warnings without failing a build:

```bash
aura lint src/main.aura
```

```text
✓ src/main.aura: no style issues
```

## `aura test`

Run `.aura` test files in a directory. Each file is executed with `--no-main`
(the file drives itself), so a test file needs no `main`:

```bash
aura test tests/
aura test tests/ -v
aura test tests/ -p "*_test.aura"
```

A test file uses `assert` and runs its own checks:

```aura
def test_addition() {
  assert 2 + 2 == 4
}

def test_strings() {
  assert "a" + "b" == "ab"
}

test_addition()
test_strings()
print("all tests passed")
```

```text
all tests passed

1/1 passed, 0 failed (0.09s)
```

## `aura debug`

Run a file under the lightweight trace debugger:

```bash
aura debug src/main.aura
aura debug src/main.aura --trace
```

## `aura lsp`

Start the language server over stdio for editor integration:

```bash
aura lsp
```

## Dependencies and environment

| Command | What it does |
|---|---|
| `aura add <pkg>` | add a Python dependency and install it |
| `aura remove <pkg>` | remove a declared dependency |
| `aura install` | install dependencies from `aura.toml` |
| `aura deps` | list declared dependencies (`--lock` writes `aura.lock`) |
| `aura venv init` | create `.venv` and install dependencies |
| `aura doctor` | check the project environment |
| `aura version` | show or bump the version |

`aura add` writes to `aura.toml` and installs into the project environment; a
declared dependency is then reachable with the `py.` prefix (chapter 14).

## A typical loop

```bash
aura init demo
aura run src/main.aura
aura check src/main.aura
aura format src/main.aura -i
aura lint src/main.aura
aura test tests/
```

## What you learned

* `run` (with args and `-v`), `check` (types + rules), `repl`.
* `init`, `transpile`, `format`, `lint`.
* `test` (self-driving files), `debug`, `lsp`.
* Dependency and environment commands; a typical edit loop.
