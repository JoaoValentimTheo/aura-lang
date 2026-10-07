# AIS/0.1 — Aura Intelligence Schema

Status: **implemented** in `src/ais.rs` with the `aura ais` entry point and the
`json` feature. This document is the design record; the schema itself is the
code plus its tests.

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
compiler knows about the program the caller submitted. It is **not** MCP (see
"Relationship to MCP") and it is **not** Jev (see "Relationship to Jev").

## Security

AIS exposes semantics, never authority. Producing or consuming an AIS
document grants no filesystem, environment, secret, network, Python, or Host
capability. The document contains only what the caller's own source produced
plus the caller-supplied source name; it cannot read a file the caller did not
already read. `tests/cli.rs::ais_output_grants_no_capability` pins this. Every
AIS request is bounded: a slice carries at most `budget` symbols and reports
what it omitted rather than expanding without limit.

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

## Delivery model: snapshot, slice, delta

AIS is designed for long-running agent sessions, so it never requires a
consumer to re-read the world after every edit.

| Mode | Command | Carries |
|---|---|---|
| **Snapshot** | `aura ais <file\|->` | the declarations and diagnostics of one source |
| **Slice** | `aura ais slice <file\|-> <target> [depth] [budget]` | the target symbol, its transitive dependencies to `depth`, bounded by `budget` |
| **Delta** | `aura ais delta <old> <new>` | declaration-level changes between two revisions |

Every snapshot carries a **content-addressed revision** (`rev:<16 hex
digits>`, FNV-1a over the exact source bytes). Two documents over identical
bytes share a revision, so a consumer detects "nothing changed" without
comparing payloads. A revision is a content identity, not a cryptographic
digest, and MUST NOT be used for integrity or security decisions.

A **delta** is declaration-level: it reports changed/added/removed symbols,
plus diagnostics that resolved and diagnostics that appeared. A body-only edit
changes the revision but produces an empty delta — the protocol describes
declarations, not statement bodies, and states this rather than pretending
otherwise (`ais::tests::delta_is_declaration_level_not_body_level`).

A **slice** always carries the requested target, even when the budget is
exhausted; `omitted` counts the dependencies left out, so an incomplete slice
is never mistaken for an empty one.

## Schema surface (`ais_version` 0.1)

| Field | Meaning |
|---|---|
| `ais_version` | protocol version |
| `aura_version` | compiler version that produced the document |
| `language_version` | language semantics the compiler implements |
| `capabilities` | which model parts are carried |
| `source_name` | the caller-supplied source display name |
| `revision` | content-addressed revision of the source |
| `symbols[]` | declarations: `name`, `kind`, `range`, `public`, `type_name`, `families`, `value_kind`, `capabilities`, `type_parameters`, `parameters`, `fields`, `variants` |
| `diagnostics[]` | `code`, `code_text`, `severity`, `message`, `range`, `notes`, `help` |

`range` carries both human coordinates (`line`, `column`) and the byte
`offset`, so a consumer never recomputes one from the other. `kind` covers the
Keystone families: `function`, `struct`, `enum`, `variant`, `field`,
`type_alias`, `type_parameter`, `constant`, `import`, `trait`, `module`.

## Three identity levels (anti-collapse)

A symbol's resolved type is projected into the three levels of
`LANGUAGE_SPEC.md` §5.4, and they are **separate fields** so a consumer can
never conflate them:

- `type_name` — the semantic type spelling (`[[int]]`, `string | none`).
- `families[]` — the capability families (`sequence`, `object`, `scalar`, …).
- `value_kind` — the runtime value kind the type pins (`list`, `struct`,
  `string`, …), absent when the static type does not pin one (a union, a
  generic parameter, an unresolved application).

A `[[int]]` return is therefore `type_name: "[[int]]"`,
`families: ["sequence"]`, `value_kind: "list"` — three distinct
representations of one type, never one collapsed name. `capabilities[]`
(`iterable`, `indexable`, `mutable`, `orderable`, `callable`) is derived from
the family projection and mirrors the executable capability table
(`LANGUAGE_SPEC.md` §5.3), so a consumer reads what the type supports instead
of inferring it from a family name.

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
- Collection families: `families[]` and `value_kind`, plus the `type_name`
  spelling. Pinned by `a_list_symbol_reports_the_sequence_family_and_list_value_kind`.
- Function signatures: `parameters[]` plus `type_name` return.
- Exception identity: enum variants and their module-qualified `name` context
  (a cross-module exception is an enum variant symbol).
- Diagnostics: structured `code`/`severity`/`range`/`notes`/`help`; no ANSI
  or prose parsing required.

An alias symbol projects its **target** (`type Names = [string]` reports
`sequence`/`list`), a struct projects `object`, a generic parameter claims no
family (it is not yet known), and a user type never claims a map capability
merely because both are keyed.

## Flow facts and completion (capability-gated)

`FlowFact` (`name`, `narrowed_type`, `diverges`) and `CompletionContext`
(`position`, `visible_symbols`) are part of the 0.1 schema. `block_diverges`
exposes the checker's conservative divergence rule so a tool can describe
whether a path cannot continue. These are present as data; the Playground and
LSP are not required to consume all of them in 0.3.

## Relationship to MCP

AIS owns Aura semantics. MCP is an interoperability layer that may project AIS
onto a tool protocol; it must not own the semantic model. The MCP-facing
operations map directly onto the primitives here — a symbol lookup is
`document`, a task lookup is `slice`, a change lookup is `delta`, a diagnostic
lookup is `diagnostics`, and a verification request is the ordinary checker —
so an adapter is a transport, not a second implementation.

## Relationship to Jev

Jev is an optional adversarial evaluator. AIS does not depend on it, and its
absence does not reduce any semantic capability: the compiler remains the
authority. Where Jev is present it may attach measurements to proposed
changes, but its result never supersedes compiler truth.

## Relationship to other Keystone pillars

- The CLI's colored renderer (`src/diagnostic.rs`) and the Playground's future
  diagnostic components are independent **renderers** of the same semantic
  data. AIS is the tooling-facing serialization of that data; none of the
  three parse each other's text.
- AIS describes; it never executes, checks beyond the normal checker, or
  reaches a Host capability.
