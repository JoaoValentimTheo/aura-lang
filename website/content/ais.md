# Aura Intelligence Schema (AIS)

**AIS/0.1** is Aura's structured semantic interface. It exposes the facts the
compiler already proved about a program — declarations, resolved types,
capabilities, and diagnostics — in a stable, machine-readable JSON form. It is
the **AI Experience (AIX)** projection of the compiler, as distinct from the
human **Developer Experience (DX)** rendering of the same facts.

AIS is produced by the compiler from the AST it already built; it is not a
serialization of the compiler's internal Rust structures. Those may change
freely, while AIS changes only when the *semantic interface* changes, and then
with a protocol-version bump.

## What AIS is not

AIS is **not**:

* a language runtime — it describes a program, it never executes one;
* an AI model provider or agent framework — it calls no model and holds no
  conversation state;
* an LSP server — it is the data an LSP *could* be built on;
* a second compiler or a second checker — the compiler is the only semantic
  authority, and AIS only reports what that compiler concluded.

See [the compiler, AIS, and MCP](#the-compiler-ais-and-mcp) below.

## Architecture

AIS is a projection of the compiler's semantic model, consumed by the CLI and
by the MCP adapter (which transports the same data over a tool protocol):

```text
Aura source
    │
    ▼
lexer / parser / checker / module system
    │
    ▼
compiler semantic model      ← the single semantic authority
    │
    ▼
AIS/0.1  (a stable JSON projection; src/ais.rs)
    │
    ├── CLI:       aura ais          (snapshot / slice / delta)
    ├── MCP:       aura mcp          (JSON-RPC 2.0 stdio adapter)
    └── future tooling integrations  (LSP, IDE, web tools)
```

The compiler stays authoritative: `src/ais.rs` has no dependency on the
transport, and removing the MCP adapter would change no semantics.

## Version identities

Three identities are deliberately kept distinct, so a consumer never conflates
them:

| Identity | Meaning | Current value |
|---|---|---|
| Aura compiler version | the toolchain that produced the document (`aura_version`) | `0.3.1` |
| Aura language version | the language semantics implemented (`language_version`) | `0.3.1` |
| AIS protocol version | the shape of the semantic interface (`ais_version`) | `0.1` |

A consumer **negotiates against `ais_version`** and records `aura_version` for
reproducibility. The MCP adapter additionally reports a `mcp_protocol_version`.
Producers never remove a field within a minor version, and consumers ignore
unknown fields, so the protocol is forward-compatible.

## Capabilities

Every document advertises which parts of the model it actually carries:

```json
"capabilities": {
  "symbols": true,
  "types": true,
  "diagnostics": true,
  "flow": false,
  "completion": false
}
```

This is an **honest advertisement**. A `0.1` document carries symbols (with
their type spellings, so `types` is also true) and, once the caller attaches
them, diagnostics. The schema **reserves** the `flow` and `completion` bits for
later use, but a producer does not claim a part of the model it does not
populate — so both are `false` today. Do not treat the presence of
flow/completion *structures* in the schema as a shipped general capability.

When a document is built by the checker (`checked_document`), it can additionally
attach the **narrowings the checker proved** in `narrowings[]` — for example
that a binding guarded by `!= none` is now `string`. These are recorded facts
from the checker's own pass, not a promise that general flow analysis is
advertised. A document built without checking carries no narrowings.

## Three levels of type identity

A symbol's resolved type is projected into **three separate fields**, so no
consumer can collapse them into one name:

| Level | Field | Example |
|---|---|---|
| semantic type | `type_name` | `[[int]]` |
| type family (capabilities) | `families[]` | `["sequence"]` |
| runtime value kind | `value_kind` | `list` |

The Keystone collection model keeps five distinct collection identities, and
AIS preserves that: a **List**, **Array**, **Tuple**, **Set**, and **Map** may
share a family yet never share a value kind. Running
`aura ais collections.aura` over:

```aura
struct Point { x: int, y: int }

fn lists() -> [[int]] { return [[1, 2], [3, 4]] }
fn fixed() -> [int; 3] { return [1, 2, 3] }
fn pair() -> (int, string) { return (1, "Aura") }
fn members() -> {int} { return {1, 2, 3} }
fn mapping() -> {string: Point} { return {"origin": Point { x: 0, y: 0 }} }

fn main() { print(lists()) }
```

projects each symbol as follows (fields elided for width):

| Symbol | `type_name` | `families` | `value_kind` | `capabilities` |
|---|---|---|---|---|
| `lists` | `[[int]]` | `["sequence"]` | `list` | `indexable`, `iterable`, `mutable`, `sized` |
| `fixed` | `[int; 3]` | `["sequence"]` | `array` | `fixed_length`, `indexable`, `iterable`, `mutable_elements`, `sized` |
| `pair` | `(int, string)` | `["sequence"]` | `tuple` | `fixed_length`, `indexable`, `iterable`, `sized` |
| `members` | `{int}` | `["set_like"]` | `set` | `iterable`, `mutable_membership`, `sized` |
| `mapping` | `{string: Point}` | `["mapping"]` | `map` | `indexable`, `iterable`, `mutable`, `sized` |

`families` and `value_kind` are separate because "can be iterated" and "is a
list" are different facts: `lists` and `fixed` are both `sequence`, but only
`fixed` is an `array`, and only `lists` is `mutable` (an Array is
`mutable_elements` — element writes without resizing). The `capabilities[]`
array is derived from the family projection and mirrors the executable
capability table, so a consumer reads what a type *supports* rather than
inferring it from a family name.

An alias symbol projects its **target** (`type Names = [string]` reports
`sequence`/`list`), a struct projects `object`/`struct`, and an unresolved
generic parameter claims **no** family (`families` omitted) and no `value_kind`
rather than a wrong one — the checker does not know the instantiated type, so
the document says so instead of guessing.

## Structured diagnostics

Diagnostics are structured data, not prose. A tooling consumer must never parse
terminal colour or human-readable error text to reconstruct a fact:

```json
{
  "code": 3001,
  "code_text": "E3001",
  "severity": "error",
  "message": "`x` is annotated as `int` but its value is `string`",
  "range": {
    "start": { "line": 2, "column": 5, "offset": 16 },
    "end": { "line": 2, "column": 8, "offset": 19 }
  }
}
```

The `message` is optional metadata; the semantic facts — `code`, `code_text`,
`severity`, and the `range` — stand on their own. `range` carries both human
coordinates (`line`, `column`, 1-based) and the byte `offset`, so a consumer
never recomputes one from the other. Diagnostics contain **no ANSI escapes**;
that separation between the human DX renderer and the machine AIX payload is a
protocol invariant.

## Delivery model: snapshot, slice, delta

AIS is designed for long-running sessions, so a consumer never has to re-read
the whole world after an edit.

### Snapshot

`aura ais <file>` (or `aura ais snapshot <file>`) prints the full document:
declarations, three-level type identity, capabilities, and diagnostics.

```bash
aura ais main.aura
aura ais snapshot main.aura      # explicit equivalent
aura ais -                       # read source from stdin
```

A snapshot is emitted **even when the source is rejected**: the `diagnostics`
array carries the checker's findings, and the command **exits non-zero** so a
caller can branch without parsing the document.

### Slice

`aura ais slice <file> <target> [depth] [budget]` returns the minimal sufficient
context for a task anchored at one declaration: the target symbol plus its
transitive dependencies.

```bash
aura ais slice main.aura mapping        # depth defaults to 1, budget to 32
aura ais slice main.aura mapping 2 32   # two levels deep, at most 32 symbols
```

* `depth` is the transitive-closure depth (`1` = direct references).
* `budget` caps the number of symbols carried, the target included.

A slice **always carries the requested target**, even when the budget is
exhausted, and reports how many symbols it left out in `omitted` — so an
incomplete slice is never mistaken for an empty one. A target that is not
declared yields an empty slice (`symbol: null`) rather than an error.

### Delta

`aura ais delta <old-file> <new-file>` reports declaration-level changes between
two revisions:

```bash
aura ais delta before.aura after.aura
```

Each entry in `symbols[]` is `added`, `removed`, `changed`, or `moved`:

* `changed` — the declaration's *semantics* differ (kind, type, families, value
  kind, capabilities, type parameters, parameters, fields, variants, or
  visibility);
* `moved` — only the source range moved (an unrelated insertion above it shifted
  its position) without changing what is declared, so a consumer refreshes
  positions without being told the declaration changed;
* plus `resolved_diagnostics[]` and `new_diagnostics[]` (codes that went away
  and codes that appeared).

**Delta is declaration-level, not body-level.** A body-only edit that does not
change a signature changes the revision but reports no *semantic* change
(`is_empty()` ignores `moved` position refreshes). This is an honest protocol
boundary: AIS describes declarations, not statement bodies. A consumer always
sees *that* the source changed, via the revision, even when no declaration did.

## Revision identity

Every document carries a content-addressed revision:

```text
rev:e62f79310558e878
```

Two documents over identical bytes share a revision, so a consumer detects
"nothing changed" without comparing payloads, and a delta is computed between
two revisions rather than two ad-hoc copies.

The revision is **FNV-1a 64-bit** over the exact source bytes — a *content
identifier*, **not** a cryptographic digest. It must never be used for
integrity or security decisions.

## Schema surface (`ais_version` 0.1)

| Field | Meaning |
|---|---|
| `ais_version` | protocol version |
| `aura_version` | compiler version that produced the document |
| `language_version` | language semantics the compiler implements |
| `capabilities` | which model parts this document carries |
| `source_name` | the caller-supplied source display name |
| `revision` | content-addressed revision of the source |
| `symbols[]` | declarations: `name`, `kind`, `range`, `public`, `type_name`, `families`, `value_kind`, `capabilities`, `type_parameters`, `parameters`, `fields`, `variants` |
| `narrowings[]` | flow facts the checker *proved*: `name`, `narrowed_type`, `diverges` |
| `diagnostics[]` | `code`, `code_text`, `severity`, `message`, `range`, `notes`, `help` |

`kind` covers the Keystone families: `function`, `struct`, `enum`, `variant`,
`field`, `type_alias`, `type_parameter`, `constant`, `import`, `trait`,
`module`.

## Security and limits

AIS **exposes semantics, never authority**. Producing or consuming an AIS
document grants no filesystem, network, Python, secret, environment, or Host
capability. It contains only what the caller's own source produced, plus the
caller-supplied source name; it cannot read a file the caller did not already
read, and it never opens the repository on the caller's behalf. The CLI reads
only the path you name (or stdin); the MCP adapter reads only the source text a
client sends in a request.

Every request is bounded: a slice carries at most its `budget` symbols and
reports what it omitted, and parsing and checking are subject to the ordinary
language resource limits.

## The compiler, AIS, and MCP

**AIS defines the semantic data. MCP transports access to that data.** The
[`aura mcp` adapter](/docs/mcp/) is a thin project of AIS primitives onto
JSON-RPC 2.0; it defines no semantics of its own. The compiler remains the sole
authority for both.

```text
compiler semantics
   ├─ DX  (human)   → colored diagnostics, REPL, Playground
   └─ AIX (machine) → AIS/0.1 → aura ais  and  aura mcp
```

A human-readable diagnostic and the AIS diagnostic for the same fact are two
*renderings* of one truth; neither defines meaning, and neither parses the
other's text.

## See also

* [CLI reference](/docs/cli/) — `aura ais` and `aura mcp` in the command set.
* [MCP adapter](/docs/mcp/) — the JSON-RPC 2.0 stdio transport and its tools.
* [Architecture](/architecture/) — where the semantic boundary sits.
* [Diagnostics](/docs/reference-errors/) — the stable `E####` codes AIS carries.
