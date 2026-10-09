# MCP adapter

The **MCP adapter** (`aura mcp`) projects AIS/0.1 — the same semantic model the
[`aura ais` CLI](/docs/ais/) emits — onto the **Model Context Protocol**, so an
MCP client can ask semantic questions about Aura source.

**AIS defines the semantic data. MCP transports access to that data.** The
adapter is a **transport, not a semantic authority**: it adds no semantics, no
inference, and no state, and the compiler remains the single authority behind
every response.

```text
Aura source
  │
  ▼
compiler pipeline (lexer / parser / checker / module system)
  │
  ▼
AIS/0.1 semantic model        (src/ais.rs — the authority)
  │
  ▼
aura mcp                      (src/mcp.rs — JSON-RPC 2.0 over stdio)
  │
  ▼
MCP client
```

## Running the adapter

```bash
aura mcp
```

`aura mcp` takes no arguments. It speaks **newline-delimited JSON-RPC 2.0** on
stdin/stdout (the MCP stdio transport): one JSON object per line, one response
per line. It is available when the compiler is built with the `json` feature —
the same feature that provides `aura ais`.

## Protocol

| Method | Behavior |
|---|---|
| `initialize` | protocol version, server info, capabilities |
| `notifications/initialized` | no response (a notification) |
| `ping` | empty result |
| `tools/list` | the tool inventory |
| `tools/call` | run one semantic operation |
| `resources/list` | the read-only resource inventory |
| `resources/read` | read one resource by URI |

The loop is **stateless**: an unknown request method returns `-32601`, an
unknown *notification* is ignored (JSON-RPC 2.0 §4.1), a malformed line returns
`-32700`, and a frame larger than the request cap is refused without buffering
further. None of these ends the process — the session survives every failure,
and a hostile request produces a structured error, never a panic or hang.

`initialize` reports the server identity and the AIS version:

```json
{
  "protocolVersion": "2025-06-18",
  "serverInfo": { "name": "aura-mcp", "ais_version": "0.1", "language_version": "0.3.1" }
}
```

## Tools

The adapter exposes a small set of focused semantic operations — never one
`get_everything` call. Each is a thin projection of an AIS primitive:

| Tool | AIS primitive | Returns |
|---|---|---|
| `aura_snapshot` | snapshot | declarations, three-level type identity, capabilities, diagnostics |
| `aura_slice` | slice | the target symbol + transitive dependencies, bounded |
| `aura_symbol` | snapshot lookup | one declaration by name |
| `aura_diagnostics` | diagnostics | structured diagnostics (code / severity / range) |
| `aura_delta` | delta | declaration-level changes, resolved/new diagnostics |
| `aura_revision` | revision | the content-addressed revision |
| `aura_capabilities` | capability report | required/optional session capabilities |

`aura_slice` accepts the same bounds as the CLI: `depth` (0–16, default 1) and
`budget` (1–4096, default 32).

## Resources

Three read-only, stable resources carry session context:

| URI | Contents |
|---|---|
| `aura://schema` | the AIS version, the three identity levels, and the delivery model |
| `aura://versions` | the compiler, language, AIS, and MCP protocol versions |
| `aura://capabilities` | the session capability report |

Reading an unknown URI returns a structured error object; it does not fail the
process.

## Capability discovery

`initialize` advertises the session's shape, and `aura://capabilities` carries
the same report:

```json
{
  "compiler":    { "required": true,  "present": true },
  "ais":         { "required": true,  "present": true },
  "mcp_adapter": { "required": false, "present": true,
                   "authority": "transport only: the compiler is the semantic authority" },
  "jev":         { "required": false, "present": false,
                   "role": "adversarial measurement, never semantic authority; absence changes no Aura semantics" }
}
```

The **compiler** and **AIS** layers are required and always present. The
**MCP adapter** is this process. **Jev** is an optional, external adversarial
evaluator: the adapter discovers it by scanning `PATH` (read-only — it never
runs the binary, reads its configuration, or grants authority). Jev is **not** a
compiler authority, and its absence changes no Aura semantics: none of the
semantic tools consults it, so the same source yields byte-identical output
either way.

## Prompts

**None.** MCP *prompts* are convenience text templates; they carry no semantic
authority, and AIS defines none. The adapter advertises `tools` and `resources`
only. A client is free to write its own prompt over the tools above — but the
semantic facts always come from the compiler, through AIS.

## What this adapter does not do

To keep the boundary honest, `aura mcp` **does not**:

* define language semantics, check beyond the ordinary checker, or execute Aura;
* provide an LSP, a debugger, or general agent execution;
* read files, the environment, the network, a secret, Python, or the Host — it
  reads only the source text a client sends in the request body;
* treat Jev, or any model, as a source of truth.

## Security

The adapter grants no capability. Responses are bounded: a slice's symbol count
is capped by the requested `budget` (and the server maximum), and the request
frame is capped. Every failure mode — a malformed frame, an unknown tool or
method, a bounded-away slice — yields a structured error and leaves the session
intact.

## See also

* [AIS](/docs/ais/) — the semantic data this adapter transports.
* [CLI reference](/docs/cli/) — `aura mcp` in the command set.
* [Architecture](/architecture/) — the host boundary and the semantic authority.
