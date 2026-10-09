# AIS Agent-Native RFC (design only — no implementation)

Status: **proposal, awaiting human review.** No AIS/0.2 code exists and none
will be written until this RFC is accepted. AIS/0.1 is unchanged and remains
the shipped protocol.

Scope: what an AI agent that must *understand, modify, and verify* Aura programs
actually needs from the compiler-facing interface, measured against what
AIS/0.1 provides today.

Normative context: `docs/engineering/AIS.md` (AIS/0.1),
`docs/engineering/MCP.md` (transport), `docs/engineering/DX_AIX.md` (the
human/machine split).

---

## 1. Method and evidence

Every claim below is grounded in the released compiler. Evidence was produced
by running the real `aura ais`/`aura mcp` on committed fixtures; measurements
are byte counts of real output, not estimates.

Reference program (`/tmp/ais-measure/app.aura`, 529 bytes): two structs, three
functions with a call chain `length → dist → sqrt_approx`, plus `main`.

| Artifact | Bytes |
|---|---:|
| Source text | 529 |
| Full AIS snapshot | 5,893 |
| `ais slice app.aura length 2 32` | 3,221 |

The first finding is uncomfortable and important: **for a small program an AIS
slice is larger than the source**, because each symbol carries `type_name`,
`families`, `value_kind`, `capabilities`, `parameters`, and a byte range as
JSON, while the source compresses far better. AIS is a *semantic* interface,
not a compression scheme; the RFC must not claim token savings it cannot
demonstrate. The value AIS can offer is **precision and non-reconstruction**
(the agent does not have to infer types, capabilities, or narrowings by parsing
text), not raw size.

## 2. What AIS/0.1 already does well

- **Compiler-proved types and capabilities per symbol.** `type_name`,
  `families`, `value_kind`, and `capabilities` are facts the checker proved;
  an agent need never re-derive that `[int; 3]` is an `array` with
  `fixed_length`/`mutable_elements`, or that `lists` and `fixed` share the
  `sequence` family yet differ in kind. This is real, and it is the right
  foundation.
- **Honest capability advertisement.** `capabilities.flow=false` /
  `completion=false` with `narrowings[]` carrying only *proved* facts is exactly
  the discipline an agent needs: it can distinguish "the compiler knows" from
  "the compiler does not claim".
- **Content-addressed revisions** (`rev:<fnv1a16>`), so an agent can confirm
  "the bytes I am reasoning about are the bytes on disk" without diffing.
- **Declaration-level delta** with `changed`/`moved` distinguished, so a
  post-edit check is cheap and does not confuse a position shift with a
  semantic change.
- **Bounded slices** with an explicit `omitted` count — the agent is told when
  context was dropped rather than silently given a truncated view.
- **Structured diagnostics** with code, severity, range (line/column/offset),
  notes, and help — never ANSI, never prose parsing.

## 3. Gaps that block agent-native workflows

Grounded in what the released AIS/0.1 **cannot** answer today (each verified
against the real compiler):

### G1 — No reverse references (definitions are one-way)

`ais slice app.aura dist` returns `dependencies: ["Point"]` — the *signature*
references of `dist`. Nothing returns the **call sites** of `dist`. The
`symbols[]` entry has no `references`/`referenced_by`. An agent asked "what
breaks if I change `dist`?" must fall back to reading source text and doing a
textual search — exactly the reconstruction the RFC exists to remove.

*Evidence*: `Symbol` fields are exactly
`name,kind,range,public,type_name,families,value_kind,capabilities,type_parameters,parameters,fields,variants`
(`src/ais.rs:235`); `referenced_names` (`src/ais.rs:941`) scans signature
positions only.

### G2 — No call graph / no reference ranges

Even where a relation exists, AIS exposes only the *declaration* range of the
target, never the *ranges of the uses*. An agent cannot jump to, slice around,
or patch a specific call site because the protocol never names one.

### G3 — Flow facts are a flat name list, not located

`narrowings[]` gives `{name, narrowed_type, diverges}` with no range and no
statement/region. An agent editing inside a guarded block cannot know *which*
`x` instance the fact qualifies, nor where the guard ends. Verified:
`FlowFact` has exactly those three fields (`src/ais.rs:623`).

### G4 — Delta is declaration-level only; no textual patch

A body-only edit is invisible to `delta` except as `moved`. There is no
minimal, machine-applicable edit representation (no "replace range R with T").
An agent that must *make* a precise change has no AIS-native way to express it,
and no way to learn from `delta` whether its intended body edit changed
anything semantic.

### G5 — No symbol identity across edits (names, not stable ids)

Symbols are identified by `name`. Two overloads share a name; an `impl` method
is `Type.method`; a local shadow reuses a name. There is no opaque, stable id
that survives a rename or a move, so an agent tracking "the same symbol" across
a refactor must match by name and position — fragile precisely where agents
operate.

### G6 — Diagnostics do not carry machine-actionable fixes

`Diagnostic.help` is a human string (`"declare it \`let mut\`"`); there is no
structured fix (edit range + replacement, or a named fix kind). An agent can
read the advice but cannot apply it deterministically.

### G7 — No explicit "unknown / unavailable" markers per symbol

`type_name` is `Option`, but there is no field that says *why* a type is
unknown (a generic parameter? an unresolved application? a union?) beyond
`value_kind` being absent. An agent planning an edit cannot distinguish "no
type" from "several types" from "the compiler did not compute it".

### G8 — Budget is symbol-count, not context-aware

`slice` bounds by *number of symbols*, not by bytes or estimated tokens, and
gives no priority ordering among omitted symbols. An agent with a hard context
budget cannot ask for "the most relevant N *bytes*" or "everything needed to
type-check this one expression".

### G9 — No verification primitive

There is no operation that answers "given this proposed source, does it still
check, and what changed semantically?" beyond running a full snapshot and
diffing by hand. `delta` compares two *files*, which is close, but the
workflow (write a temp file, snapshot both, diff) is not a single bounded call.

### G10 — Retrieval is pull-only and text-framed

MCP exposes snapshot/slice/symbol/diagnostics/delta/revision over JSON-RPC. An
agent doing an edit loop needs *push* of the relevant set after a change, and a
way to retrieve by *relation* ("edit sites of X"), not only by name.

## 4. Non-goals and constraints

- **Semantic authority stays in the compiler.** Every proposed field must be
  something the checker or parser already proved or can prove without new
  language analysis. Nothing here asks AIS to *implement* flow analysis; G3
  asks it to *expose* what the checker already proved, located.
- **Backward compatibility.** AIS/0.1 fields keep their meaning; new fields are
  additive and `#[serde(default)]`-skipped, so a 0.1 consumer keeps working.
  Any *removal* or meaning change needs a separate, human-approved versioning
  decision (`AIS/0.2`).
- **Transport unchanged.** MCP remains a transport; new operations are tools
  over the same semantic data, not a new protocol.
- **No fabricated performance claims.** The RFC proposes measurements to be
  run *after* a prototype; it asserts none.

## 5. Proposed operations (design)

Each is scoped to data the compiler already has or can produce mechanically
from the AST/checker — no new type theory.

| # | Operation | Input | Output | Existing data it needs |
|---|---|---|---|---|
| O1 | `references(symbol)` | a symbol id | every use range + the enclosing symbol | AST walk for name uses (new, mechanical) |
| O2 | `definition(range)` | a position | the declaring symbol id | AST + scope resolution (checker already resolves names) |
| O3 | `slice_by_relation(symbol, kinds, budget)` | symbol id + relation kinds (`deps`,`refs`,`both`) | a bounded subgraph | O1 + existing slice |
| O4 | `located_flow(range)` | a position | the narrowings in effect there, with the guard range | checker's narrowing table, located (new exposure) |
| O5 | `stable_ids` on every symbol | — | an opaque id (e.g. `sym:<content-and-kind hash>`) | name+kind+signature hash (new, mechanical) |
| O6 | structured `fix` on diagnostics | — | `{range, replacement}` or `{kind}` | the checker already builds `help`; add the edit |
| O7 | `unknown_reason` on `type_name` | — | one of `generic`/`union`/`unresolved`/`not-computed` | checker state (new field) |
| O8 | byte/token budgets on `slice` | a byte budget | the same slice, bounded by bytes, `omitted` in bytes | re-rank existing closure |
| O9 | `verify(proposed_source)` | source text | `{revision, diagnostics[], delta}` in one call | existing snapshot+delta composed |

Only O1–O3, O5, O8, O9 are genuinely *new* computation; O4, O6, O7 are
exposures of data the checker already holds.

## 6. Representation and wire format

The current JSON is readable but redundant for agent loops. Without changing
0.1 semantics, an agent-native surface could add:

- **Interned tables.** A `symbols` table plus a `refs` table of
  `[from_id, to_id, range]`, so relations are ids, not repeated names.
- **Delta encoding for repeated snapshots.** After a `rev`, subsequent
  snapshots send only changed symbols (the `delta` already computes them).
- **Range as `[start_byte, end_byte]`** with line/column optional, so an agent
  can patch bytes directly; 0.1's line/column/offset triple stays for humans.
- **A retrieval policy layer** separate from the data: "give me the minimal
  closure of X under a byte budget" is a *retrieval* concern; the *semantic*
  data (types, capabilities, relations) is transport-independent.

This keeps the four layers separate, as the order requires: **semantic
authority** (compiler) → **transport** (MCP) → **retrieval policy** (budgets,
ranking) → **presentation** (human DX or agent prompt).

## 7. Evaluation plan (to run before any AIS/0.2 commitment)

Define representative agent tasks and measure a **baseline (source files only)
vs AIS** workflow. Proposed tasks:

1. *Answer a type question*: "What is the element kind of `fixed`?" — baseline:
   `grep` + read; AIS: one `symbol` call.
2. *Impact analysis*: "List every call site of `dist`." — baseline: textual
   search + manual disambiguation; AIS: `references(dist)` (requires O1).
3. *Guarded edit*: "Rename `Point.x` everywhere in the package." — baseline:
   text replace with false-positive risk; AIS: `references(Point.x)` ranges.
4. *Verify a patch*: "Does this edit still check, and did anything change?" —
   baseline: run `aura check` + read; AIS: `verify(proposed)` (O9).
5. *Bounded understanding*: "Summarize what `length` needs, in ≤2 KB." —
   baseline: read the file; AIS: `slice_by_relation` with a byte budget (O3/O8).

For each task, measure: **correctness** (did the agent get it right),
**tool calls**, **wall latency**, and **context bytes consumed**. Report only
measured numbers. A plausible hypothesis, to be *tested* rather than assumed:
AIS reduces tool calls and reconstruction for relational tasks (2, 3, 4), while
for a small file it may *increase* context bytes (see §1) — so the win, if any,
is in precision and iteration count, not size.

## 8. Versioning and rollout

- Everything above is additive to AIS/0.1; a consumer ignoring unknown fields
  keeps working.
- A `AIS/0.2` label is proposed **only** if a field's *meaning* changes or a
  goal requires a breaking wire change; the human decides.
- MCP gains the operations as new tools; existing tools keep their contracts.
- Implementation is gated on this RFC's acceptance and on the stabilization
  audit's findings being triaged.

## 9. Open questions for the human

1. Is the priority **precision** (relations, located facts, verification) or
   **size** (byte/token budgets)? They imply different first moves; §1 shows
   size is not guaranteed.
2. Should stable symbol ids (O5) be content-derived (survive renames poorly) or
   name+kind-derived (survive moves poorly)? This is a real design choice with
   no free answer.
3. Does the project want AIS to expose *uses* (O1), which requires an AST-level
   reference index the compiler does not maintain today, accepting the extra
   pass?
4. Is `verify(proposed_source)` allowed to accept source *text* over the wire
   (a small authority the current read-only tools avoid), or must it stay
   path/named-source only?
