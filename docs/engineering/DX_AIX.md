# DX and AIX — two projections of one semantic truth

Status: normative architecture note. The compiler is the single semantic
authority; DX and AIX are two *presentations* of it and neither defines
meaning.

## The architecture

```
                    COMPILER SEMANTICS
                  (lexer / parser / checker)
                           │
               ┌───────────┴───────────┐
               ▼                       ▼
              DX                      AIX
        human projection        machine projection
```

Both projections read the same semantic facts. They differ only in what they
optimize:

| | DX (Developer Experience) | AIX (AI Experience) |
|---|---|---|
| Consumer | a human | a machine reasoner |
| Optimizes for | comprehension, orientation, repair | semantic identity, minimal sufficient context |
| Carries | prose, ANSI color, source excerpts, `help:`/`note:` text | stable identifiers, relations, structured fields |
| Redundancy | permitted (clarity) | removed (deduplicated, interned) |
| Authority | **none** — presentation | **none** — projection |

## The normative rule

> Human-readable diagnostics MUST NOT define the AI semantic interface. An AI
> consumer must never be forced to parse English diagnostic prose to
> reconstruct a fact the compiler already holds.

A DX diagnostic is a *rendering*. It may say:

```
error[E3003]: value may be `none`
  --> app.aura:12:9
   |
12 |     print(user.email)
   |           ^^^^^^^^^^ this value may be `none`
   |
help: guard with `if user.email != none { ... }`
```

The AIX payload for the same fact is the structured `Diagnostic`
(`src/ais.rs`):

```json
{
  "code": 3003,
  "code_text": "E3003",
  "severity": "error",
  "message": "value may be `none`",
  "range": { "start": { "line": 12, "column": 9, "offset": 187 } },
  "help": "guard with `if user.email != none { ... }`"
}
```

The `message` and `help` strings are *optional metadata*. The semantic facts —
the code, the span, the structured position — stand on their own. A machine
may render its own explanation or ignore prose entirely.

The same separation holds for types. A DX tool prints `[[int]]`. AIX receives
the three identity levels as separate fields: `type_name: "[[int]]"`,
`families: ["sequence"]`, `value_kind: "list"`. No projection re-derives one
from the other's text.

## Where each lives

| Surface | Projection | Where |
|---|---|---|
| CLI diagnostics (color, excerpts, help) | DX | `src/diagnostic.rs`, `src/main.rs` |
| REPL | DX | `src/repl.rs` |
| Playground | DX | `playground/web/` |
| aurea.css design system | DX | `website/` |
| AIS/0.1 snapshot/slice/delta | **AIX** | `src/ais.rs` |
| MCP adapter | **AIX** (transport) | `src/mcp.rs` |
| `aura ais` CLI | **AIX** | `src/main.rs` |

## What this forbids

- An AI consumer parsing `help:` text to learn a diagnostic's cause.
- An AI consumer string-matching `[[int]]` to learn that a value is iterable;
  the `families`/`capabilities` fields are the authoritative facts.
- The DX renderer inventing a code or a relation AIS does not report.
- AIS emitting ANSI, color, or prose that a machine must strip before use
  (`src/ais.rs`'s diagnostic tests pin the absence of escapes).

## Evidence

- `tests/cli.rs` — `ais_*` tests: structured diagnostics with no ANSI, three
  identity levels as separate fields, bounded output.
- `src/ais.rs` — the schema; `capabilities` advertises which model parts a
  document carries so a consumer knows what to expect.
- `tests/ais_properties.rs` — parity and vocabulary-disjointness properties.
- `src/mcp.rs` — the same facts over the transport, with no second model.
