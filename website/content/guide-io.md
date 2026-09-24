# I/O and arguments

Aura's scripting surface is small and explicit: read a line, read a file, write
a file, and read the program arguments.

## Program arguments

`args()` returns the arguments after the script path.

```bash
aura run greet.aura Ada Lovelace
```

```aura
fn main() {
    for a in args() { print(a) }
}
```

* `aura run script.aura a b` → `args()` is `["a", "b"]`.
* `aura eval` → `args()` is `[]`.
* The REPL and library → `args()` is `[]`.
* In the [Playground](/playground/), supply arguments one per line.

## Standard input

`read_line() -> string | none` reads one line, excluding the line terminator.
A trailing `\r\n` or `\n` is stripped; a lone trailing `\r` is preserved. End of
input yields `none`.

```aura
fn main() {
    let mut line = read_line()
    while line != none {
        print(line.upper())
        line = read_line()
    }
}
```

This is the canonical filter loop. There is no `read_all`; whole-input reads are
composed from `read_line`.

## Files

```aura
fn main() {
    let text = read_file("input.txt")
    if text == none {
        print("input.txt is missing")
    } else {
        write_file("copy.txt", text)
    }
}
```

* `read_file(path) -> string | none` — missing path is `none`; empty file is
  `""`; a directory, permission error, or invalid UTF-8 is `E4020`.
* `write_file(path, content) -> none` — creates or truncates; a missing parent
  directory or a directory path is `E4020`.

Filesystem access is local and **not sandboxed**: paths are used as given, with
the invoking user's privileges. There is no append mode, no binary I/O, and no
file-handle or streaming API in this version.

## Capability availability

The host decides which capabilities exist. In the browser Playground there is no
filesystem, clock, or sleep; those calls report `E5002` (capability
unavailable), and Aura never fakes them. `stdin`, `stdout`, and `args` are
always available.

| Capability | Native (`StdHost`) | Browser (`BrowserHost`) |
|---|---|---|
| `print` (stdout) | yes | yes |
| `read_line` (stdin) | yes | yes (supplied input) |
| `args` | yes | yes |
| `read_file` / `write_file` | yes | `E5002` |
| `time_now` / `time_unix` | yes | `E5002` |
| `sleep_ms` | yes | `E5002` |
