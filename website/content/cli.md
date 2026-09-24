# CLI

The `aura` binary is the command-line interface.

| Command | Purpose |
|---|---|
| `aura run <file>` | Parse, check, and execute a program (or stdin with `-`) |
| `aura check <file>` | Parse and check without running |
| `aura eval <code>` | Run a one-liner |
| `aura repl` | Start an interactive session with persistent state |
| `aura version` | Print the version |

## `aura run`

```bash
aura run examples/tour.aura
aura run script.aura foo bar     # foo and bar become args()
aura run -                       # read the program from stdin
```

`aura run` requires the program to declare `fn main()`; a file without one is
`E4027`. Arguments after the script path are available through `args()`,
excluding the command, subcommand, and script path.

When the program is read from a file, process standard input is available to
`read_line()`. When the program itself is read from stdin (`-`), that stream has
already been consumed, so `read_line()` returns `none`.

## `aura check`

Parses and checks without executing. Useful in CI and editors.

```bash
aura check my_program.aura
```

## `aura eval`

```bash
aura eval 'print(1 + 2)'
aura eval 'let xs = [1, 2, 3]\nprint(len(xs))'
```

`eval` accepts either a statement or a bare expression. It exposes standard
input to `read_line()` but `args()` is empty.

## Exit codes

`0` on success; non-zero when a diagnostic is produced. Diagnostics are printed
to standard error with a `file:line:column:` prefix and the stable code.

```text
my_program.aura:3:12: E3001: type mismatch ...
```

## Building the binary

See [Installing Aura](/docs/install/). The `cli` feature is required for the
binary; the pure-Rust feature set is
`cli,repl,json,regex,time`.
