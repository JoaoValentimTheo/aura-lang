# Aura Module & Import Architecture (Pre-0.3 Foundation)

Status: architecture record of implemented behavior. No change is proposed
here. Normative loading contract: `docs/FILESYSTEM_MODULES_DESIGN.md` (frozen
design); this file is the consolidated map.

---

## 1. Three identity layers (do not conflate)

| Layer | Type | Scope | Meaning |
|---|---|---|---|
| Provider key | `SourceKey(Arc<str>)` (`src/module_graph.rs:22`) | provider-local | opaque handle used to load a source; **never** interpreted as a module name |
| Logical module path | `LogicalModulePath(Vec<String>)` (`:332`) | Aura semantics | the `a::b::c` name programs use and diagnostics report |
| Source identity | `SourceId` (`src/source.rs`) | compilation session | diagnostic attribution; allocated by `SourceMap`, deterministic insertion order, never derived from paths |

`Location { source: SourceId, span: Span }` pairs source identity with a
source-local span; `Span` alone is never interpreted across sources.

## 2. Normalization and validation

- A child claim's logical name must be a valid Aura module identifier;
  otherwise `E2022` (`src/module_graph.rs:841-848`).
- Case-only collisions inside one parent are rejected (`E2022`), both in the
  graph builder (`:852-878`) and in the filesystem provider's directory scan
  (`src/native_source.rs:439-443`).
- Filesystem paths are normalized (`native_source.rs:474`); symlink chains in
  module discovery are rejected (`:365-369`, `:391-398`).
- No two different logical names may own the same provider source:
  `E2021` (`DUPLICATE_LOGICAL_SOURCE`, `:601-612`, `:887-899`).

## 3. Cycles

Two distinct notions:

- **Provider ownership cycles**: a source reachable under more than one
  logical path (A -> B -> A at the source-key level). Because each key may own
  at most one logical path, a cycle terminates as
  `E2021` — pinned by `tests/module_graph.rs:236` and `:244`.
- **Semantic reference cycles**: `A -> B -> C -> A` via function references or
  re-exports. These are **not** loader errors; they are resolver/checker/
  runtime concerns and are allowed when the reference semantics allow them
  (`docs/FILESYSTEM_MODULES_DESIGN.md` §"DECIDED: declaration/name cycles are
  allowed"; `tests/module_graph.rs:253`, `:276`).

There is deliberately **no filesystem dependency-cycle error**: the selected
eager design makes cycles representable and resolved by name, not by load
order.

## 4. Determinism

- Provider enumeration order is explicitly non-semantic
  (`src/module_graph.rs:193-195`). Child candidates are validated and sorted
  by (logical name, source name, source key) before any parse (`:821-838`).
- Traversal is an explicit `Vec<BuildFrame>` worklist — no host recursion
  (`:584-693`).
- `ModuleGraph` records nodes in deterministic depth-first discovery order
  (`:443`); the `SourceMap` is allocated in that same order (`:431`), and
  `SourceMap::order` exposes it (`src/source.rs:159-171`).
- Lowering to nested `Item::Module` is post-order with a single-consumption
  `Option` slot per node (`:695-755`); a lost node is `E4999` (internal bug
  guard), never silent.

## 5. Caching and duplicates

There is no text memoization: each accepted claim loads its source once and
inserts it into the `SourceMap`. Duplication is prevented structurally by the
single-owner rule (`source_owners`, `:601`), not by a cache. `InMemorySourceProvider`
refuses duplicate keys at insert and keeps multiple child claims distinct
(`:242-262`).

## 6. Diagnostic attribution

- Parse/check diagnostics carry `SourceDiagnostic` with the owning source
  (`:560`, `:665`).
- Provider/ownership/path failures are locationless (`SourceDiagnostic::locationless`)
  with codes `E2020`/`E2021`/`E2022`, because no Aura source text is at fault.
- Physical wrapper modules get `Span::default()` (synthesized), while item
  provenance rides in `ItemProvenance { source, children }` (`:748-753`).
- Provenance rendering is pinned by `tests/source_provenance.rs:168-201`
  (`a.aura:1:1:`, `b.aura:1:1:`, `<stdin>:1:1:`).

## 7. Substrates

| Aspect | Native (`native_source.rs`) | In-memory | Playground |
|---|---|---|---|
| Entry | file path | `SourceKey` chosen at construction | virtual project request |
| Discovery | `X.aura` + `X/mod.aura` directory scan, sorted, symlinks rejected | explicit child claims | caller-supplied files |
| Diagnostics | path-flavored messages | provider names | same core attribution |

The provider interface (`SourceProvider`: `entry`/`load`/`children`,
`src/module_graph.rs:180-200`) is the only extension point; a future packaged
application or Python-bridge source is a provider, not a core change.

## 8. What future work must preserve

1. One canonical identity per module; no path-derived `SourceId`.
2. Deterministic, provider-order-independent discovery.
3. Single-owner sources; `E2021` on violations.
4. No loader-level cycle error for semantic reference cycles.
5. Locationless diagnostics only for provider-level failures.
