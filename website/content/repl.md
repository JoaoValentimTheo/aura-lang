# REPL

`aura repl` starts an interactive session with persistent state.

```text
$ aura repl
Aura 0.0.1 REPL — :help for commands
aura> let x = 21
aura> x * 2
42
aura> :quit
```

## Behaviour

* Each submission is parsed and checked by the real front end, then executed in
  a persistent interpreter.
* Declarations register functions, structs, enums, and aliases for later use.
* A bare expression echoes its value; declarations are silent.
* Multi-line input is accepted: the REPL waits for balanced braces before
  parsing.

## Commands

| Command | Effect |
|---|---|
| `:help` | Show the available commands |
| `:quit`, `:q`, `:exit` | Leave the session |

## Defaults

The REPL exposes no standard-input source, so `read_line()` returns `none`, and
`args()` is empty. This matches the library defaults; only `aura run` and
`aura eval` wire process input.

The whole session runs on the large execution stack, so a deeply nested but
valid submission is bounded by the language nesting limit (`E1015`) rather than
by the platform's default stack.
