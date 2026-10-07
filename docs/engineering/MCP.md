# Aura MCP adapter (`aura mcp`)

Status: **implemented** in `src/mcp.rs`, exposed as `aura mcp` behind the `json`
feature. This document is the design record.

## What it is

MCP (Model Context Protocol) is an interoperability layer for AI tools. The
Aura adapter projects the compiler's AIS/0.1 semantic model onto that layer so
an MCP client can ask semantic questions about Aura source.

It is a **transport**, not a semantic authority:

```
Aura source
  → compiler pipeline (lexer / parser / checker / module system)
  → AIS/0.1 semantic model  (src/ais.rs)
  → aura mcp                (src/mcp.rs, JSON-RPC 2.0 over stdio)
  → MCP client
```

Every operation is a thin projection of an AIS primitive. The adapter adds no
semantics, no inference, and no state.

## Protocol

Newline-delimited JSON-RPC 2.0 on stdin/stdout (the MCP stdio transport). One
JSON object per line; one response per line. Implemented:

| Method | Behavior |
|---|---|
| `initialize` | protocol version, server info, capabilities |
| `notifications/initialized` | no response (a notification) |
| `ping` | empty result |
| `tools/list` | the tool inventory |
| `tools/call` | one semantic operation |
| `resources/list` | the read-only resource inventory |
| `resources/read` | one resource by URI |

An unknown request method returns `-32601`. An unknown *notification* is
ignored and produces no response (JSON-RPC 2.0 §4.1). A malformed line returns
`-32700`; a line larger than `MAX_REQUEST_BYTES` (8 MiB) is refused without
buffering further. None of these ends the process: the loop is stateless and
the session survives every failure.

## Tools

Small semantic operations — never one `get_everything` call.

| Tool | AIS primitive | Returns |
|---|---|---|
| `aura_snapshot` | `ais::document` | declarations, three-level type identity, capabilities, diagnostics |
| `aura_slice` | `ais::slice` | the target symbol + transitive dependencies, bounded |
| `aura_symbol` | snapshot lookup | one declaration by name |
| `aura_diagnostics` | `ais::Diagnostic` | structured diagnostics (code/severity/range) |
| `aura_delta` | `ais::delta` | declaration-level changes, resolved/new diagnostics |
| `aura_revision` | `ais::revision_of` | the content-addressed revision |
| `aura_capabilities` | `mcp::capabilities_report` | required/optional session capabilities |

## Capability discovery (Jev is optional)

The `initialize` result advertises `experimental` capabilities and the
`aura://capabilities` resource carries the same report, so a client learns the
session's shape in one handshake:

```json
{
  "compiler": { "required": true,  "present": true },
  "ais":      { "required": true,  "present": true },
  "mcp_adapter": { "required": false, "present": true,
                   "authority": "transport only: the compiler is the semantic authority" },
  "jev":      { "required": false, "present": false,
                "role": "adversarial measurement, never semantic authority; absence changes no Aura semantics" }
}
```

Jev is discovered by scanning `PATH` for an executable named `jev`
(`mcp::discover_jev`). Discovery is **read-only**: it never runs the binary,
reads its configuration, or grants authority. Jev's absence changes no Aura
semantics — none of the semantic tools consults it, and
`src/mcp.rs::tests::a_missing_optional_capability_changes_no_semantics` pins
that the same source yields byte-identical semantic output either way.

## Resources

Read-only, stable information:

- `aura://schema` — the AIS version, the three identity levels, the delivery
  model, and the explicit statement that the compiler is the authority.
- `aura://versions` — the compiler, language, AIS, and MCP protocol versions.
- `aura://capabilities` — the session capability report above.

Unknown URIs return an error object; they do not fail the process.

## Prompts

**None.** Prompt templates are convenience text; they carry no semantic
authority and AIS defines none. A client may write its own prompt over the
tools above.

## Security

The adapter grants no capability. It reads only the source text the client
sends in the request body — it never opens a file, reads the environment,
reaches the network, or touches Python or the Host. Responses are bounded: a
slice's symbol count is capped by the requested `budget` (and
`MAX_SLICE_SYMBOLS`), and the request frame is capped by `MAX_REQUEST_BYTES`.
A malformed or hostile request produces a structured error, never a panic,
abort, hang, or corrupt session. `src/mcp.rs` unit tests pin each of these
(responses, notification silence, malformed frames, unknown tools/methods,
bounded slices).

## Why AIS remains transport-independent

`src/ais.rs` has no dependency on `src/mcp.rs`; the dependency is one-way.
AIS is consumed by the CLI (`aura ais`), by tests, and by this adapter, and it
would be consumed identically by any future LSP, IDE plugin, or web tool. The
adapter's `tools()` and `handle_message` translate AIS values into JSON-RPC
shapes and nothing else, so removing `src/mcp.rs` would not change any semantic
behavior — a property the CLI and library tests hold independently.
