# Tooling Reconciliation (Keystone §35)

Status: architecture record for the Aura 0.3 “Keystone” transaction. It
reconciles the historical 0.3 tooling goals against the repository as it
actually is, and it selects no product that requires a human decision.

## 1. Repository reality

Before Keystone, `PLANS.md` recorded “package manifests / package management /
remote dependency resolution / URL imports” and “LSP or a formatter product”
as **explicitly deferred until a human selects a direction**. Nothing was
built. This document does not overturn that; it states what Keystone has
established so a future tool builds on shared compiler semantics instead of
a second implementation.

## 2. What Keystone established (the shared foundation)

- **AIS/0.1** (`docs/engineering/AIS.md`, `src/ais.rs`, `aura ais`): a stable
  external semantic protocol — symbols, kinds, ranges, types (including
  `T | none` and `never`), diagnostics, capabilities, version negotiation.
- **Structured diagnostics** (`Presentation` + `src/diagnostic.rs`): one
  semantic payload with independent CLI and tooling renderers.
- **One parser/checker**: `parse::parse_type` and the module system are the
  single source of truth; no tool may fork them.

The design rule for every tool below: **consume compiler/AIS semantics; never
re-derive them.**

## 3. Formatter

Smallest coherent scope: a **source-preserving formatter** built on the real
lexer/AST, not a text rewriter.

- Contract: parse → emit from the token stream with a canonical layout;
  because Aura is newline-sensitive, the formatter must own line breaks, not
  guess them. Comments and blank-line intent must survive.
- Non-goals: no formatting while the program does not parse (report the
  diagnostic instead); no semantic changes; no configuration language beyond a
  small documented set.
- Status: **architecture only in Keystone.** A correct newline-sensitive
  formatter is a product-sized effort; shipping a half formatter that mangles
  valid programs would be worse than none. The parser and `E1015` limits it
  must respect already exist.

## 4. LSP

Smallest coherent scope: an LSP server that serves AIS documents.

- `textDocument/diagnostic` and `publishDiagnostics` map from AIS
  `diagnostics[]` (code, severity, range) with no prose parsing.
- `textDocument/documentSymbol` maps from AIS `symbols[]`.
- `textDocument/completion` uses the capabilities gate: AIS currently
  advertises `completion: false`, so an honest LSP returns scope names or
  nothing rather than inventing a completion model.
- The server must negotiate AIS version and Aura version independently, and
  must grant no capability (AIS exposes semantics, never authority).
- Status: **architecture/contract in Keystone**; the AIS surface is the
  deliverable that makes the server a thin adapter.

## 5. Package management

Smallest coherent scope: **no product decision in Keystone.**

- The repository has no package manifest format, no lockfile, no registry, and
  no remote resolution. Those require product and security decisions
  (identity/trust, supply chain, signatures, URL policy) that Keystone is not
  authorized to invent.
- What exists and is stable: the filesystem module system
  (`docs/FILESYSTEM_MODULES_DESIGN.md`) and the `use`/module-resolution
  contract. A future package manager would sit on top of that discovery
  boundary, not inside the evaluator.
- Status: **blocked on a human product/security decision**; documented here so
  the missing work is explicit rather than implied.

## 6. Disposition summary

| Goal | Keystone disposition |
|---|---|
| Formatter | architecture/contract; consumer of the real parser; not shipped |
| LSP | architecture/contract; AIS/0.1 is the semantic surface it adapts |
| Package manager | explicitly blocked on human product/security decisions; no invention |

No tool in this document adds evaluator authority, and none weakens the
capability model (`docs/engineering/CAPABILITY_MODEL.md`).
