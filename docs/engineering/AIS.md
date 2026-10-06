# AIS/0.1 — Aura Intelligence Schema

Status: **implemented** in `src/ais.rs` with the `aura ais <file|->` entry
point and the `json` feature. This document is the design record; the schema
itself is the code plus its tests.

## What AIS is

A stable, external semantic protocol between the Aura compiler and intelligent
or developer tools — LSP, the Playground, IDEs, and future AI tooling.

```
Aura source
→ lexer / parser / checker / module system
→ semantic model
→ AIS
→ LSP / Playground / IDE / future AI tooling
```

AIS is produced from the AST the compiler already built. It is **not** a
serialization of internal Rust structures: the internal types may change
freely, while AIS changes only when the semantic interface changes, and then
with a version bump. It is intentionally stable.

## What AIS is not

AIS is not an OpenAI/Anthropic/DeepSeek API, an LLM provider, an agent
framework, LangChain, or the planned Aura-0.4 AI stdlib. It describes what the
compiler knows about the program the caller submitted.

## Security

AIS exposes semantics, never authority. Producing or consuming an AIS
document grants no filesystem, environment, secret, network, Python, or Host
capability. The document contains only what the caller's own source produced
plus the caller-supplied source name; it cannot read a file the caller did not
already read. `tests/cli.rs::ais_output_grants_no_capability` pins this.

## Versioning and negotiation

A document carries two independent versions:

```json
{ "ais_version": "0.1", "aura_version": "0.2.1", "language_version": "0.2.1" }
```

A consumer negotiates against `ais_version` and records `aura_version` for
reproducibility. `capabilities` advertises which parts of the model the
document carries (`symbols`, `types`, `diagnostics`, `flow`, `completion`).
Consumers ignore unknown fields (pinned by
`ais::tests::unknown_fields_are_ignored_for_forward_compatibility`), and a
producer never removes a field within a minor version.

## Schema surface (`ais_version` 0.1)

| Field | Meaning |
|---|---|
| `ais_version` | protocol version |
| `aura_version` | compiler version that produced the document |
| `language_version` | language semantics the compiler implements |
| `capabilities` | which model parts are carried |
| `source_name` | the caller-supplied source display name |
| `symbols[]` | declarations: `name`, `kind`, `range`, `public`, `type_name`, `type_parameters`, `parameters`, `fields`, `variants` |
| `diagnostics[]` | `code`, `code_text`, `severity`, `message`, `range`, `notes`, `help` |

`range` carries both human coordinates (`line`, `column`) and the byte
`offset`, so a consumer never recomputes one from the other. `kind` covers the
Keystone families: `function`, `struct`, `enum`, `variant`, `field`,
`type_alias`, `type_parameter`, `constant`, `import`, `trait`, `module`.

The design test the order proposes —

> If the compiler understands a Keystone feature, can AIS describe its
> relevant semantics without forcing a consumer to parse human diagnostic
> prose?

— is answered yes by construction:

- `T | none`: the field/parameter `type_name` is the full spelling
  (`string | none`). The union is described, not erased. Pinned by
  `struct_fields_and_optional_types_are_described` and the CLI AIS test.
- `never`: a callable's `type_name` is `never`.
- Struct fields: `fields[]` with name and type.
- Collection families: carried in `type_name` spellings (`[int]`, `{k: v}`).
- Function signatures: `parameters[]` plus `type_name` return.
- Exception identity: enum variants and their module-qualified `name` context
  (a cross-module exception is an enum variant symbol).
- Diagnostics: structured `code`/`severity`/`range`/`notes`/`help`; no ANSI
  or prose parsing required.

## Flow facts and completion (capability-gated)

`FlowFact` (`name`, `narrowed_type`, `diverges`) and `CompletionContext`
(`position`, `visible_symbols`) are part of the 0.1 schema. `block_diverges`
exposes the checker's conservative divergence rule so a tool can describe
whether a path cannot continue. These are present as data; the Playground and
LSP are not required to consume all of them in 0.3.

## Relationship to other Keystone pillars

- The CLI's colored renderer (`src/diagnostic.rs`) and the Playground's future
  diagnostic components are independent **renderers** of the same semantic
  data. AIS is the tooling-facing serialization of that data; none of the
  three parse each other's text.
- AIS describes; it never executes, checks beyond the normal checker, or
  reaches a Host capability.
