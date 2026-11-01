# RFC 0006 — The AIS Agent-Native Capability Set

- **Status:** Proposed — reconstructed for the NEBULA 0.3.2 campaign (Directive
  008, gate G02). **Not accepted.** It records eighteen defects found by
  probing a protocol that advertises itself as machine-checkable, and it
  escalates three decisions (§2.1, §2.2, §2.3). No production source was
  modified: gate G02 forbids it.
- **Scope:** the Agent Interface Schema as it actually behaves — the document
  shape, the CLI surface, the MCP surface, the advertised capability bits, the
  declaration ranges, the dependency closure, the slice diagnostics, and the
  pipeline the document is built from. **Grammar delta: none** (§3).
- **Authorities:** `docs/engineering/AIS.md` (198 lines: §Versioning `:40-55`,
  delivery model `:57-82`, schema field table `:84-101`);
  `docs/engineering/MCP.md` (120 lines, tools table `:50-58`);
  `docs/engineering/AIS_AGENT_NATIVE_RFC.md` (243 lines, §3 gaps, §5
  operations, §9 open questions); `src/ais.rs` (1440 lines),
  `src/main.rs:395-527`, `src/mcp.rs` (718 lines), `src/parse/mod.rs:528-530`,
  `src/check/mod.rs:2335-2346`.
- **Human decisions:** three. **D1** — capability honesty and the AIS version
  (§2.1). **D2** — what `Symbol.range` means (§2.2). **D3** — whether the AIS
  document path runs the resolver (§2.3). None is resolved here. Provenance is
  stated in §0.

## 0. Provenance

`NBL-014` in `docs/engineering/NEBULA_CAMPAIGN.md` (`:663`) requires an RFC for
the "AIS Agent-Native 0.2 capability set". The campaign's §0.4 records that no
pre-existing Nebula artifact existed in the tree when G02 began, so this RFC is a
**reconstruction from repository evidence**, not a transcription of an approved
design. Every claim below is backed by a probe recorded in §6.

The prior-art document `docs/engineering/AIS_AGENT_NATIVE_RFC.md` is a Keystone
proposal, not this campaign's artifact. It is cited where it already anticipated
a gap, and §1.6 records that its supporting fixture was never committed.

Nine corrections were applied to earlier working notes during this research and
are recorded here rather than silently dropped. Each was a published claim that a
later probe falsified:

1. An early note claimed `ais::block_diverges` "has no caller" on the strength of
   `grep -rn 'block_diverges' src/` returning 11 hits. That is a **name**
   collision: `src/ais.rs:1028` is the `pub fn`, and the other ten hits are
   `Checker::block_diverges`, an unrelated private method at
   `src/check/mod.rs:2728`. The narrow claim (zero callers of the AIS helper)
   survives; the grep-count claim did not.
2. An early note attributed the capability over-advertisement to the parse-error
   path only. Probing shows the **accepted** path and the **rejected** path
   advertise byte-identical capabilities (§1.4).
3. An early note reported the resolver-only code set as **five** codes. A wider,
   explicit census over `src/resolve.rs` and `src/module_graph.rs` gives **six**
   (§1.14). The earlier figure came from a subtractive census that silently
   dropped `UNKNOWN_MODULE`.
4. An early note described the resolver gap as "AIS ignores name resolution
   errors" with no boundary. The real gap is **narrow and worse**: `E2007` and
   `E3002` survive with the right code, while `E2019` vanishes and
   `E2018`/`E2023` arrive **relabelled** `E2003` (§1.13). The earlier note also
   cited an `E3009`, which is not a declared code in `src/error.rs`; it was
   removed rather than replaced, because no probe ever produced one.
5. An early draft listed a `capabilities.flow = false` document as holding a
   non-empty `narrowings[]` and called that a contradiction. It is not: `AIS.md`
   and `AIS_AGENT_NATIVE_RFC.md:49-52` reserve `flow` for *divergence* facts, so a
   narrowing may appear while `flow` stays `false`. The claim was withdrawn.
6. An early measurement reported a slice's `dependencies` closing over body-level
   call edges. It does not; `referenced_names` reads signatures only (§1.8).
7. An early draft of **this RFC** published an aggregate range understatement of
   `6.4×` computed by a script whose printed per-row column and summed total used
   two different extent definitions, and later re-computed `31.4×` from a
   different bug. Both were harness faults. The figure in §1.2 is recomputed from
   a single definition and reproduced three times; the history is preserved in
   §6.2 because the shape it describes — a plausible number produced by an
   inconsistent script — is exactly what this RFC asks the human to fund a test
   for.
8. An early note treated `aura explain E1001` as a working command whose output
   agreed with the renderer. It is not a command (`EXIT=2`). That belongs to RFC
   0004's surface, not this one, and is not repeated here.
9. An early note described the AIS CLI's fall-through as "unknown operations error
   out". They do not: `src/main.rs:404-409` matches three words and sends
   **everything else** to `cmd_ais_snapshot` (§1.3).

Correction 7 is the reason §1.2 states its method as a formula and §6.2 publishes
the arithmetic. A number in a protocol RFC is a contract; it must be reproducible
from the document alone.

## 1. Motivation

### 1.1 The protocol and its four promises

AIS exists so that an external agent can ask the compiler what a program means
without reading the source. `docs/engineering/AIS.md` makes four promises that
this RFC tests. Each is quoted from the file at `HEAD`.

The versioning promise (`AIS.md:53-55`):

> A producer never removes a field within a minor version. Consumers should
> tolerate unknown fields.

The capability promise, in code (`src/ais.rs:64-71`), on `impl Default for
Capabilities`:

> Honest advertisement: a 0.1 document produced by `document()` carries symbols
> (with their type spellings, so `types` too) and, once the caller attaches them,
> diagnostics. It does not yet populate flow facts or completion context — the
> schema reserves the capability bits for them, but a producer must not claim a
> part of the model it does not actually carry.

The slice promise, on the type itself (`src/ais.rs:848-853`):

> …and the diagnostics that mention it or its dependencies, bounded by `budget`.

The range promise, on the field (`src/ais.rs`, `Symbol::range` doc comment):

> Where it is declared.

`AIS.md:96-101` elaborates the last one only by saying the range "carries both
human coordinates (`line`, `column`) and the byte `offset`". It never says what
the range **spans**. Nothing in `docs/LANGUAGE_SPEC.md` does either: the spec
mentions AIS exactly once (`:2578`, in prose) and ends at `## 36. Generics`
(`:4009` of 4099 lines).

So the protocol has a documented field whose meaning is not documented. §1.2
measures what it actually does.

### 1.2 Finding A — every `Symbol.range` spans only the leading keyword

The authoritative fixture is a 259-byte, 10-line program holding ten symbols of
six kinds. `aura ais` reports it as `EXIT=0`, `symbols 10`, `diags 0`.

| Symbol | Kind | Reported bytes | Reported | True bytes | Understatement |
|---|---|---|---|---|---|
| `M` | module | `0..6` | 6 | 12 | 2.0× |
| `Math` | import | `13..16` | 3 | 13 | 4.3× |
| `Point` | struct | `27..33` | 6 | 36 | 6.0× |
| `Color` | enum | `64..68` | 4 | 25 | 6.2× |
| `Show` | trait | `90..95` | 5 | 14 | 2.8× |
| `Id` | type alias | `105..109` | 4 | 13 | 3.2× |
| `LIMIT` | constant | `119..124` | 5 | 21 | 4.2× |
| `helper` | function | `141..143` | 2 | 41 | 20.5× |
| `area` | function | `183..185` | 2 | 48 | 24.0× |
| `main` | function | `232..234` | 2 | 26 | 13.0× |

The reported bytes are, in every row, the declaration's **leading keyword**:
`b'module'`, `b'use'`, `b'struct'`, `b'enum'`, `b'trait'`, `b'type'`,
`b'const'`, `b'fn'`, `b'fn'`, `b'fn'`. The true extent is the declaration's own
line minus its trailing newline:

```
start = line_start(symbol.range.start.offset)
end   = start + len(line_text)
```

Aggregating with that one formula (§6.2 reproduces it):

| Quantity | Value |
|---|---|
| Reported total | 39 bytes |
| True total | 249 bytes |
| Understatement | 6.4× |

**Root cause.** `symbol_of_item` (`src/ais.rs:447-511`) builds the range from the
item's AST `span` at `:490-501`. The parser captures that span **before** it
consumes the keyword, in the recurring shape `let span = self.span(); self.bump();`.
`Parser::span` (`src/parse/mod.rs:528-530`) returns the *current token's* span, and
`Span` (`src/error.rs:13-26`) carries no extent semantics — it is a byte pair with
no rule about what it covers. There are 29 `let … = self.span();` sites in
`src/parse/mod.rs`, and exactly 12 of them are immediately followed by
`self.bump();` (`:752`, `:895`, `:1108`, `:1134`, `:1227`, `:1267`, `:1310`,
`:1415`, `:1454`, `:1508`, `:1525`, `:1574`). Those 12 are the capture points
whose span is a keyword, not a declaration.

**What is not wrong.** `line` and `column` are **not** zero: `symbol_of_item`
writes `0` and `resolve_symbol` (`src/ais.rs:603-617`) overwrites both from the
offset via `crate::error::line_col`. Only the **extent** is wrong. The nested
`parameters[].range` and `fields[].range` are resolved by the same function and,
apart from Finding I, are exact.

**What this breaks.** A consumer that highlights a symbol highlights `fn`. A
consumer that extracts an LLM context window sized by `range.end - range.start`
gets 2 bytes where 41 exist. The prior-art RFC's evaluation plan measures
"context bytes per fact", and every such ratio built on `range` is wrong by the
factor in the last column.

**No test observes it.** `tests/ais_properties.rs` (255 lines, 11 tests) mentions
`range` **zero** times. `tests/ais_differential.mjs` (329 lines) mentions it zero
times. `src/mcp.rs` mentions it twice, both in pass-through code. The defect is
unenshrined, which is why it survived.

### 1.3 Finding B — the CLI exposes three of the seven advertised operations

`AIS.md` describes seven operations. `src/main.rs:404-409` dispatches three:

```rust
match tail.first() {
    Some(&"slice") => cmd_ais_slice(&tail[1..]),
    Some(&"delta") => cmd_ais_delta(&tail[1..]),
    Some(&"snapshot") => cmd_ais_snapshot(&tail[1..]),
    _ => cmd_ais_snapshot(&tail),
}
```

The fall-through arm is the defect. `symbol`, `diagnostics`, `revision`, and
`capabilities` are not rejected — they are treated as **file paths**, and the
error a caller sees is a filesystem error naming their own subcommand:

| Invocation | Exit | Output |
|---|---|---|
| `aura ais` (no file) | 2 | `usage: aura ais <file\|->` |
| `aura ais snapshot` (no file) | 2 | `usage: aura ais <file\|->` |
| `aura ais slice` (no file) | 2 | `usage: aura ais slice <file\|-> <target> [depth] [budget]` |
| `aura ais delta` (no file) | 2 | `usage: aura ais delta <old-file\|-> <new-file\|->` |
| `aura ais symbol` | 1 | ``aura: cannot read `symbol`: No such file or directory (os error 2)`` |
| `aura ais diagnostics` | 1 | ``aura: cannot read `diagnostics`: …`` |
| `aura ais revision` | 1 | ``aura: cannot read `revision`: …`` |
| `aura ais capabilities` | 1 | ``aura: cannot read `capabilities`: …`` |
| `aura ais bogus` | 1 | ``aura: cannot read `bogus`: …`` |

With a real file present, behaviour is unchanged: `aura ais f.aura` is `EXIT=0`
(a snapshot), `snapshot` is `EXIT=0` (same document), `slice`/`delta` are `EXIT=2`
usage errors, and the other five words are still `EXIT=1` "cannot read". A real
subcommand never tries to open its own name as a file; an unknown word does.

The four missing operations are exactly the four the MCP adapter implements as
tools. So the same protocol is reachable four different ways with four different
guarantees: `aura ais` (3 of 7), `aura://schema` (6 of 7), `tools/list` (7 of 7),
and `docs/engineering/MCP.md:50-58` (7 of 7).

### 1.4 Finding C — capability bits that cannot be false

`Capabilities::default()` is constructed at exactly three sites
(`src/main.rs:429`, `src/ais.rs:987`, `src/mcp.rs:297`), and **no field is ever
assigned anywhere in `src/`** — `Capabilities { symbols: … }` appears zero times
outside the `Default` body. Every document therefore advertises:
`symbols: true, types: true, diagnostics: true, flow: false, completion: false`.

Probing three very different programs returns **byte-identical** capability
blocks:

| Program | `aura check` | `aura ais` | `symbols` | `diags` | `flow` | `completion` | `diagnostics` bit |
|---|---|---|---|---|---|---|---|
| empty file | 0 | 0 | 0 | none | false | false | **true** |
| 259-byte fixture | **1** (`E2019` at `2:1`) | **0** | 10 | **none** | false | false | **true** |
| file containing `$bad` | 1 | 1 | 0 | `['E1001']` | false | false | **true** |

The `diagnostics: true` bit means "the document carries structured diagnostics".
Row 2 is a program the compiler **rejects** and the document carries **no**
diagnostic at all (Finding M), yet the bit is `true`. Row 1 carries no
diagnostics either and the bit is still `true`.

This is the direct contradiction of the doc comment quoted in §1.1: a producer
must not claim a part of the model it does not actually carry. Because the value
is a constant, the capability block is not an advertisement — it is a constant,
and a consumer that branches on it learns nothing.

`flow` and `completion` being `false` is honest. `completion` is not a defect:
`CompletionContext` (`src/ais.rs:635`) is constructed nowhere and `diverges: true`
appears zero times in `src/`. Those are dead code, which §7 records as an
observation rather than a decision.

### 1.5 Finding D — `aura://schema` disagrees with `tools/list`

Enumerated live over JSON-RPC against one server process:

| Surface | Count | Contents |
|---|---|---|
| `tools/list` | 7 | `aura_snapshot`, `aura_slice`, `aura_symbol`, `aura_diagnostics`, `aura_delta`, `aura_revision`, `aura_capabilities` |
| `aura://schema` `.tools` | 6 | same, **without** `aura_capabilities` |
| `aura://schema` `.delivery` | 3 | `snapshot`, `slice`, `delta` |
| resources | 3 | `aura://schema`, `aura://versions`, `aura://capabilities` |

`OMITTED = ['aura_capabilities']`, `EXTRA = []`. The schema resource also reports
`ais_version 0.1`, `authority "the compiler; this adapter defines no semantics"`,
and `identity_levels ['semantic_type','type_family','runtime_value_kind']`.

Both omissions are the same hardcoded literal, eight lines apart
(`src/mcp.rs:265-266`):

```rust
"delivery": ["snapshot", "slice", "delta"],
"tools": ["aura_snapshot", "aura_slice", "aura_symbol", "aura_diagnostics", "aura_delta", "aura_revision"],
```

Two literal lists that must agree with the tool table and cannot. `MCP.md:50-58`
lists all seven correctly, so the documentation is right and the wire is wrong.

**A mitigating nuance, recorded because it weakens the finding.**
`src/mcp.rs:232-253` gives the schema resource exactly three resources, and the
schema's own description says it carries "the AIS protocol version, the three
identity levels, and the delivery model". It does **not** claim to enumerate the
tool set. So a consumer that reads `schema.tools` as authoritative is making an
inference the text never licensed. The field still should not exist in two
places, but the failure is an unowned duplicate rather than a broken promise.

**Nothing catches it.** `grep -rn "aura://schema\|delivery" tests/` is empty. The
MCP test `tools_list_advertises_small_semantic_operations` (`src/mcp.rs:503`)
asserts six names, not seven, so it would keep passing with either list.

### 1.6 Finding E — the differential harness says it runs in CI and does not

`tests/ais_differential.mjs` (329 lines) is the only harness in the tree that
compares raw-source and AIS-conditioned agent context. Its docstring
(`:14-26`) states:

> It always reports the structural comparison it can compute itself: the context
> size of each condition in bytes/tokens and the number of semantic facts each
> carries. This runs in CI with no model.

Grep across `.github/`, `scripts/`, `playground/*.mjs`, `Makefile`, `justfile`,
and `Taskfile*` for `ais_differential` returns **zero** hits (`WIRING_HITS=0`).
No `package.json` exists. The full census of `node` invocations in
`.github/workflows/ci.yml` (`:70`, `:72`, `:74-83`, `:204`, `:230`, `:304`)
contains no reference to this file.

The harness runs and exits 0 when invoked by hand:

```
list-family: 3 questions / optional-field: 2 / diagnostic: 2 / delta: 1
raw=91 tok, ais=2422 tok
Fact carriers: raw=4, ais=12
EXTERNAL DIFFERENTIAL: NOT EXECUTED (no evaluator configured).
HARNESS_EXIT=0
```

So the sentence is false in the only sense that matters: the file is present,
runs, and is never executed by any job. This is the failure mode `AGENTS.md`
rule 7 names — a suite in the tree is not a suite that executes — and the harness
contributes to it by asserting its own wiring.

Note the honest half: `EXTERNAL DIFFERENTIAL: NOT EXECUTED` is correct and
refuses to fabricate a model run. The dishonesty is confined to the CI claim.

The structure the harness reports is itself informative. `raw=91` tokens carry
**4** fact carriers; `ais=2422` tokens carry **12**. The AIS condition is 26.6×
larger for 3× the facts — and, as the harness prints, that is the ratio for a
program with a handful of declarations. Any such ratio computed from `range`
inherits Finding A.

**Prior art was never committed.** `git log --all -S "sqrt_approx"` over the whole
history returns nothing, and the fixture sizes `AIS_AGENT_NATIVE_RFC.md` cites
(529 / 5 893 / 3 221 bytes) are not reproducible from the tree. A re-measured
fixture in §6.5 gives `source=454 snapshot=5496 slice=3223`, i.e. `slice/source =
7.10×` and `snapshot/source = 12.11×`. The prior-art numbers are of the right
order but cannot be reproduced, so they are not evidence.

### 1.7 Finding F — the `const` projection drops its own type spelling

The three identity levels (`semantic_type`, `type_family`, `runtime_value_kind`)
are AIS's central claim. Probing a `const` pair:

| Declaration | `type_name` | `value_kind` | `families` |
|---|---|---|---|
| `const A: int = 1` | **`None`** | `'int'` | `['scalar']` |
| `const B = 2` | **`None`** | **`None`** | **`None`** |
| `main` | `None` | `None` | `None` |

Level 1 is missing for both, and levels 2 and 3 are missing for the unannotated
one. Mechanism: `sym.type_name` is populated by the first `match item` arm
(`src/ais.rs:565-578`), whose `Item::Const` case falls into `_ => {}` at `:577`.
Only *value* types are projected afterwards (`:583-597`), and that arms
`Item::Const { ann, .. } => ann.as_ref().map(...)` at `:590` — which yields
`families`/`value_kind` from the annotation but never writes `type_name`.

A `type alias` in the same program is projected correctly
(`Id` → `type_name='int'`), and a function returning `never` projects
`type_name='never'`, `families=['sum']`, `value_kind=None`. So the omission is
specific to `const`.

This is the smallest finding in the RFC and the cheapest to fix, which is why it
is here: it shows the projection is written per-item-kind with a silent default
arm, so a new item kind will silently project nothing.

### 1.8 Finding G — the dependency closure reads signatures only

`slice` is documented as returning "the dependencies" of a target. Probing a
five-function chain where the edges exist **only in bodies**:

| Target | `dependencies` | `omitted` |
|---|---|---|
| `base` | `[]` | 0 |
| `mid` | `[]` | 0 |
| `top` | `[]` | 0 |
| `caller` | `[]` | 0 |
| `use_c` | `['C']` | 0 |

`use_c(c: C)` mentions `C` **in its signature** and gets `['C']`. `caller()`
calls `A { v: 1 }` in its body and gets `[]`.

Mechanism: `referenced_names` (`src/ais.rs:941-965`) walks `sym.type_name`, then
`sym.parameters[].type_name`, then `sym.fields[].type_name`. It never reads a
body and never reads a call site. Depth is a true transitive closure **over that
relation**: for `use_c(c: C)` with `C { b: B }`, `B { a: A }`,

| `depth` | `dependencies` |
|---|---|
| 1 | `['C']` |
| 2 | `['B', 'C']` |
| 3 | `['A', 'B', 'C']` |
| 4 | `['A', 'B', 'C']` |

and budget is `dep_budget = budget.saturating_sub(1)`:

| `budget` | `dependencies` | `omitted` |
|---|---|---|
| 1 | `[]` | 3 |
| 2 | `['A']` | 2 |
| 3 | `['A', 'B']` | 1 |
| 4 | `['A', 'B', 'C']` | 0 |
| 8 | `['A', 'B', 'C']` | 0 |

The semantics are self-consistent. The gap is that the advertised relation is
smaller than the one an agent needs, and `Slice.dependencies` is not labelled.

An undeclared target returns `EXIT=0` with `symbol` absent and the remaining keys
present — no error, which is a defensible choice for an editor, but it makes a
typo indistinguishable from a program with no such declaration.

This gap is **already anticipated** by the prior art:
`AIS_AGENT_NATIVE_RFC.md` §9 open question 3 asks whether to expose *uses*/O1 and
notes it "needs an AST reference index the compiler does not maintain". So G is a
known open question, not a new decision, and this RFC does not escalate it.

### 1.9 Finding H — three `SymbolKind` variants can never be emitted

`SymbolKind` (`src/ais.rs:180-206`) has eleven variants under
`#[serde(rename_all = "snake_case")]`. Census over `src/`:

| Variant | Construction arms | Test assertions | Live |
|---|---|---|---|
| `Function` | 1 | 1 | yes |
| `Struct` | 1 | 1 | yes |
| `Enum` | 1 | 0 | yes |
| `Variant` | **0** | 0 | **no** |
| `Field` | **0** | 0 | **no** |
| `TypeAlias` | 1 | 1 | yes |
| `TypeParameter` | **0** | 0 | **no** |
| `Constant` | 1 | 0 | yes |
| `Import` | 1 | 0 | yes |
| `Trait` | 1 | 1 | yes |
| `Module` | 1 | 0 | yes |

`SymbolKind::` occurs 12 times in `src/`: 8 construction arms plus 4 test
assertions. Live `kind` values observed in a real document:
`['constant','enum','function','struct','trait','type_alias','module','import']`
— eight of eleven.

`docs/engineering/AIS.md:99-101` documents all eleven, describing `variant`,
`field`, and `type_parameter` as if a consumer will see them. A consumer that
writes an exhaustive `match` on the documented set has three unreachable arms; a
consumer that writes `.variant` handling has code that can never run against this
producer. The nested data those variants would describe **does** exist — enums
carry `variants: ["Red","Green"]` as plain strings, structs carry
`type_parameters: ["T"]` as plain strings, fields carry
`{name, range, type_name}` — so the information is present but untyped.

Removing the three variants is a schema-shape change, which is why it belongs to
D1 rather than being a free fix.

### 1.10 Finding I — `pub` fields and `mut` parameters inherit the range defect

Finding A is not confined to top-level declarations. On a fresh fixture:

| Element | Reported bytes | Reported text | Expected |
|---|---|---|---|
| symbol `struct S` | `0..6` | `b'struct'` | header extent |
| field `x` in `pub x: int` | `11..14` | **`b'pub'`** | `x` |
| symbol `fn f` | `24..26` | `b'fn'` | header extent |
| parameter `z` in `mut z: int` | `38..41` | **`b'mut'`** | `z` |
| parameter `_p` | `50..52` | `b'_p'` | `_p` |

`pub` at the **top level** does not corrupt the range; only a `pub` **field**
does. The control parameter `_p` is exact, so the defect is the modifier, not the
parameter mechanism.

Mechanism, both from the same capture-before-consume shape:

- `params` (`src/parse/mod.rs:1365`) takes `span` **before**
  `self.eat(&Tok::Mut)` at `:1370`.
- the struct field loop (`src/parse/mod.rs:1426`) takes `fspan` **before**
  `self.eat(&Tok::Pub)` at `:1427`.

`pub` is not legal on a parameter (`fn g(pub q: int)` is `E1009`), so `mut` is the
only parameter modifier that reaches this path.

This matters beyond bookkeeping: `pub x` and `mut z` are the two places where a
symbol's *name* is not at its reported start, so a consumer that reads
`source[range]` to recover an identifier gets a keyword instead.

### 1.11 Findings J and K — `name` is advertised, optional, and discarded

`aura_symbol`'s input schema advertises `name`, `source`, `symbol` and requires
`source` and `symbol` (`props ['name','source','symbol']`,
`required ['source','symbol']`). The handler reads only `symbol`
(`src/mcp.rs:345`, `need_str(args, "symbol")`):

| Arguments | Result |
|---|---|
| `symbol` only | works |
| `name` only | `isError=True`, ``missing required string argument `symbol` `` |
| `both` | works |
| `neither` | same error as `name` only |

So the schema offers a property that cannot satisfy the schema's own
requirement and that changes nothing when supplied. That is Finding J.

Finding K generalises it. `display` reads `name` at five sites
(`src/mcp.rs:328`, `:334`, `:346`, `:356`, `:366`) with
`unwrap_or("<mcp>")`. A per-tool sensitivity matrix:

| Tool | `name` changes output | Response carries `source_name` |
|---|---|---|
| `aura_snapshot` | **yes** | **yes** |
| `aura_slice` | no | no |
| `aura_symbol` | no | no |
| `aura_diagnostics` | no | no |
| `aura_delta` | no | no |
| `aura_revision` | no | no |

**Four of the five tools that accept `name` throw it away.** For
`aura_diagnostics` the whole response is `{revision, diagnostics}` — the display
name never reaches the wire, and two calls with and without `name` return
byte-identical `{"diagnostics": [], "revision": "rev:f89ab2b7f4ae7782"}`.

`name` is therefore not a dead property globally — `aura_snapshot` honours it —
which is precisely what makes it dangerous: the schema is uniform, the semantics
are not, and no document says which tools it applies to.

### 1.12 Finding L — `Slice.diagnostics` is never populated

The `Slice` type doc (`src/ais.rs:848-853`) promises the diagnostics that mention
the target or its dependencies. The field doc (`:868`) says *"Diagnostics whose
span touches the slice's resolution, when known."*

All three `Slice` construction sites set `diagnostics: Vec::new()`: the undeclared
target at `:893` and the normal return at `:935` (`:992` is the `Document`
builder). So the field exists, is skipped by serde when empty, and is always
empty.

Probed on a program with a live error:

```
aura check a.aura   → EXIT=1, a.aura:2:36: error[E2003]: struct `Point` has no method `nope`
aura ais a.aura     → EXIT=1, snapshot diags [('E2003', 2, 36)]
aura ais slice a.aura main  → EXIT=0, keys without `diagnostics`
aura ais slice a.aura area  → EXIT=0, keys without `diagnostics`
aura ais slice a.aura Point → EXIT=0, keys without `diagnostics`
```

**The information is in scope and discarded.** `src/mcp.rs:330-345` builds the
snapshot (which carries the diagnostic) and then calls `crate::ais::slice(&doc, …)`
with that document. `pub fn slice` receives `doc` and never reads
`doc.diagnostics`.

Zero tests reference it: the four slice unit tests (`src/ais.rs:1350-1400`) do not
assert `diagnostics`, and the MCP test `slice_tool_is_bounded_and_reports_omissions`
asserts `symbol.name`, empty `dependencies`, and `omitted >= 1` — never
`diagnostics`.

**Contrast: `Delta` gets this right.** `Delta.resolved_diagnostics` and
`Delta.new_diagnostics` are populated as `BTreeSet<u16>` differences of `E####`
codes. Two revisions differing only in a type error:

| Transition | `resolved_diagnostics` | `new_diagnostics` |
|---|---|---|
| `b → d1` (same diagnostics) | `[2003]` | absent |
| `b → d2` (also a `E3001`) | `[2003]` | `[3001]` |

So the AIS document model already knows how to carry diagnostics alongside a
projection. `Slice` simply does not do it.

**A note on `slice` exit codes.** `cmd_ais_slice` returns `ExitCode::SUCCESS`
unconditionally (`src/main.rs:474-498`), and `cmd_ais_delta` likewise
(`:501-527`) after calling `ais_snapshot_of` twice and **ignoring** the
`rejected` flag. A delta between two rejected programs exits 0. Combined with
Finding L, a caller using `slice` cannot learn from either the payload or the
status that the program failed.

### 1.13 Findings M, N, O — the document path skips the resolver

`aura check` and `aura ais` are two different pipelines:

| Command | Pipeline | Source |
|---|---|---|
| `aura check` | `compile_file_with_mode(path, CompileMode::Module)` → provider → `resolve_sourced` → checker | `src/main.rs:363-393` |
| `aura ais` | `parse(src)` → `ais_snapshot_of` → `ais::checked_document` → `Checker::module_narrowings` **only** | `src/main.rs:421-443` |

`checked_document` (`src/ais.rs:1004-1020`) calls exactly one compiler entry
point, `crate::check::Checker::module_narrowings` (`src/check/mod.rs:2335-2346`).
There is no resolver in the path. Its own doc comment (`src/ais.rs:1001-1002`)
promises:

> A rejected program still yields a document, with the diagnostic and without
> narrowings.

**Finding M — the class is dropped.** The authoritative fixture opens
`use Std::Math`. `aura check` reports `E2019` at `2:1` and exits 1. `aura ais`
returns **`EXIT=0`**, 10 symbols, and **no `diagnostics` key at all**. Minimised
to `use Std::Math` plus one `print`:

| Command | Exit | Diagnostic |
|---|---|---|
| `aura check u.aura` | 1 | `u.aura:1:1: error[E2019]: unknown module `Std` in `use`` |
| `aura ais u.aura` | **0** | none, 2 symbols |

`cmd_ais_snapshot` sets exit 1 **only** when `doc.diagnostics` is non-empty
(`src/main.rs:464-470`), so a dropped diagnostic is also a wrong status. The same
holds through MCP: `aura_diagnostics` on that program returns
`{"diagnostics": [], "revision": "rev:c103fd3f2c9392b8"}` with `isError:false`.

**Finding N — the gap is narrow, and the surviving diagnostics are mis-coded.**
Six programs, each in its own directory:

| Program | `aura check` | `aura ais` codes | Verdict |
|---|---|---|---|
| `use Nope::Thing` | 1, `E2019` | 0, none | dropped |
| `use Std::Math` | 1, `E2019` | 0, none | dropped |
| `let x: Nope = 1` | 1, `E3002` | 1, `['E3002']` | correct |
| `fn Std()` | 1, `E3002` | 1, `['E3002']` | correct |
| duplicate `fn dup()` | 1, `E2007` | 1, `['E2007']` | correct |
| `M::secret()` private | 1, **`E2018`** | 1, **`['E2003']`** | **wrong code** |

So AIS is not blind to the checker: `E3002`, `E2007`, and their class survive.
Two resolver-owned codes vanish, and one arrives under the wrong code and the
wrong message.

**Finding O — the wrong-code class has a second member.** `module Aura { … }`
invokes the reserved namespace:

```
aura check → r.aura:1:1: error[E2023]: `Aura` is reserved for built-in exception families…
aura ais   → EXIT=1, diags ['E2003']
```

Same signature as `E2018`: the resolver declines, the checker emits its own
generic member-access error, and AIS reports the checker's.

**A stale exit-code note.** `src/main.rs:396-400` documents `cmd_ais` as emitting
the document "even when checking finds diagnostics: the `diagnostics` array
carries them, and the exit code is 1 when the source was rejected". That is true
of `cmd_ais_snapshot` and false of `cmd_ais_slice` and `cmd_ais_delta` (§1.12).

### 1.14 Findings P, Q, R — the resolver-only set, the asymmetry, and `<stdin>`

**Finding P — the AIS-invisible code set is exactly six, and it is fully
enumerated.** A census of `codes::<NAME>` occurrences per file:

| Code | `resolve.rs` | `module_graph.rs` | `check/mod.rs` | `main.rs` | `ais.rs` | Ownership |
|---|---|---|---|---|---|---|
| `E2019` `UNKNOWN_MODULE` | 7 | 0 | 0 | 1 | 0 | resolver |
| `E2020` `MODULE_SOURCE_OWNERSHIP` | 0 | 2 | 0 | 0 | 0 | resolver |
| `E2021` `DUPLICATE_LOGICAL_SOURCE` | 0 | 2 | 0 | 0 | 0 | resolver |
| `E2022` `MODULE_SOURCE_PATH` | 0 | 7 | 0 | 0 | 0 | resolver |
| `E2023` `RESERVED_NAMESPACE` | 1 | 0 | 0 | 0 | 0 | resolver |
| `E4020` `IO` | 0 | 1 | 0 | 0 | 0 | resolver |
| `E2018` `PRIVATE_ACCESS` | 4 | 0 | **9** | 0 | 0 | **both** |
| `E2007` `REDECLARED` | 4 | 0 | **13** | 0 | 0 | both |
| `E1009` `RESERVED_NAME` | 1 | 0 | 4 | 0 | 0 | both |
| `E3002` `UNKNOWN_TYPE` | 2 | 0 | **15** | 0 | 0 | both |

Six codes are owned by the resolver stage. The `main.rs` hit for `E2019` is the
`<stdin>` comparison at `:288`, not an emission. Four codes are owned by **both**
stages, which is exactly why `E2018` and `E2023` come back mis-coded rather than
missing: the checker has its own path that produces a *different* diagnostic for
the same program.

`E2019`, `E2020`, `E2021`, `E2022`, `E2023`, and `E4020` are therefore
unreachable through AIS, and `E2018`/`E2023` are reachable only in their
checker-flavoured form. `LANGUAGE_SPEC.md` documents every one of them:
`:1745` (E2023 prose), `:3573-3577` (E2018-E2022 rows).

**Finding Q — the privilege asymmetry is deliberate for `E2018` and undocumented
for everything else.** The `use` item is a **no-op** in the checker at
`src/check/mod.rs:1413` and `:3689` (`Item::Use { .. } => {}`). Only the resolver
acts on `use`. So the checker cannot report `E2019` even in principle — not a bug
in the checker, a division of labour the AIS path does not respect.

For `E2018` the split is documented: `src/check/mod.rs` carries 9 hits and
`src/resolve.rs` 4, and the checker's copy exists for single-module programs
where no resolver runs. The same reasoning would apply to `E2019` — a `use` in a
single-file program is still an `E2019` — but the checker declined it, and no
document says why. That is the asymmetry.

**Finding R — `<stdin>` is the one place the compiler admits filesystem modules
are unavailable, and AIS does not.** `src/main.rs:288-290`:

```rust
if stdin_source && d.code == aura::error::codes::UNKNOWN_MODULE {
    rendered.push_str("; filesystem modules are unavailable for <stdin>");
}
```

Proven end-to-end on stdin:

```
$ printf 'use Std::Math\nfn main() { print(1) }\n' | aura check -
<stdin>:1:1: error[E2019]: unknown module `Std` in `use`; filesystem modules are unavailable for <stdin>
STDIN_CHECK=1

$ printf 'use Std::Math\nfn main() { print(1) }\n' | aura ais -
STDIN_AIS symbols 2 diags []
```

`aura ais` reads stdin too (`ais_read` → `read_source(Some("-"))`,
`src/main.rs:414-416`). The AIS path never runs the resolver, so it cannot
produce the diagnostic the note is attached to — and it has no note of its own.
The one message that explains the limitation is unreachable from the surface that
needs it most.

**No test covers any of this.** `UNKNOWN_MODULE` appears zero times in
`src/ais.rs`, `tests/ais_properties.rs`, and `src/mcp.rs`; `2019` appears zero
times in the latter two. `checked_document` has 8 references across `src/` and
`tests/`, none of which exercises the resolver-to-AIS path.

## 2. Decision

Eighteen findings, three decisions. The decisions are separated from the findings
because each one changes a **published contract** in a way a probe cannot settle.

### 2.1 D1 — Capability honesty and the AIS version

Findings C, F, H, and L all require changing what a `0.1` document contains.
`AIS.md:53-55` says a producer never removes a field within a minor version. That
sentence was written about **fields**; none of these changes removes a field. But
`Versioning/negotiation` (`AIS.md:42`) does not say what a *semantic* correction
costs, and Finding H would change the documented value set of a field that stays
in place.

Three options, none of which a probe can favour:

- **Option A — declare AIS 0.1 defective and bump to 0.2.** All four corrections
  land; `AIS_VERSION` becomes `"0.2"`; consumers must re-read the schema. This is
  the honest reading of "a capability bit must be true iff the document carries
  that part of the model" and it is the only option that lets Finding H's three
  dead variants be **removed** rather than left as reserved.
- **Option B — stay at 0.1 and add only.** Fix C by making the bits dynamic, add
  `type_name` for `const` (F), populate `Slice.diagnostics` (L), and leave the
  three dead `SymbolKind` variants in place documented as reserved-for-nested-use
  (H). Nothing a consumer can see today disappears. The cost: `AIS.md:99-101`
  keeps describing three values that never arrive, and the protocol carries a
  permanent inconsistency it will have to explain forever.
- **Option C — stay at 0.1, fix C and F and L, and remove the three variants
  without a version bump.** Fastest, and consistent with the observation that no
  external consumer of `SymbolKind` exists in this repository. The risk is
  exactly the promise in `AIS.md:53-55`: the repository cannot see its consumers,
  so "nobody depends on it" is not verifiable here.

**Why this is a decision rather than a bug fix.** Finding H's fix is one `enum`
deletion. Whether deleting it is *legal* is a protocol question, and the protocol
document answers it for fields, not for value sets.

**Recommendation (not a resolution):** Option A. The four defects share one root
cause — the document is assembled by a projection with silent default arms that
nobody tested for completeness — and shipping four semantic corrections inside a
version that promised stability is worse than a version bump.

### 2.2 D2 — What `Symbol.range` means

Finding A measured a 6.4× understatement. Fixing the arithmetic is mechanical.
Choosing the contract is not. Four candidate meanings:

- **Option A — declaration extent.** The range covers the declaration's source
  text. Cheap to implement from the parser's own span if the parser is taught to
  capture after the keyword, or from line extent as §1.2 computed. Breaks the
  invariant that `source[range]` starts at an identifier only for `pub`/`mut`,
  which Finding I already violates. This is what `"Where it is declared"` most
  plainly means.
- **Option B — the declaration header (through the closing `)` or `{`).** Enough
  for a consumer that wants to display a signature; not enough to extract a body;
  stable under body edits, which makes `same_declaration` and delta `moved`
  detection easier to reason about.
- **Option C — the declaration's name token.** Consistent with what the field
  does today for parameters, and the cheapest of all. But it makes `range`
  useless for extraction, which is the use the prior-art RFC's evaluation plan
  assumes.
- **Option D — keep `range` as the name token and add a new
  `declaration_range`.** Additive, so no consumer breaks, and `AIS.md` can state
  both meanings precisely. The cost is two overlapping range fields and one more
  thing to get out of sync.

**Two constraints bound the choice.**

1. `moved` detection depends on `same_declaration` (`src/ais.rs:720-746`), which
   **scrubs ranges** before comparing. So changing the extent cannot change delta
   classification. Whatever is chosen is safe for deltas.
2. `tests/ais_properties.rs` never mentions `range`, and no external consumer
   exists in this repository. So the current extent is **unobservable to current
   tests** — which means the repair cannot be validated by the existing suite and
   must bring its own (§6.4).

**Recommendation (not a resolution):** Option A, plus an explicit statement in
`AIS.md` of what the range covers, because the absence of that sentence is what
allowed the defect to survive six review passes over the same file.

### 2.3 D3 — Whether the AIS document path runs the resolver

Findings M, N, O, P, Q, and R are one architectural question: should
`aura ais` see what `aura check` sees?

- **Option A — yes, always.** Route the AIS path through the resolver-bearing
  pipeline (`compile_file_with_mode` → `resolve_sourced`), so `E2019`-class codes
  arrive with the right code and the right exit status. Consequences: AIS gains a
  filesystem dependency it does not have today; `aura ais -` (stdin) would have
  to keep the no-resolver path and inherit Finding R; and the six
  resolver-only codes become reachable, which expands the protocol's diagnostic
  surface and therefore its test burden.
- **Option B — no, and say so.** Keep the parse+check path, document that AIS
  reports a *checker* view and that resolver-stage codes are out of scope, and
  make the omission explicit rather than silent: a `resolution: "not_run"` field
  on the document, plus a non-zero exit when a `use` item is present.
  Consequences: AIS stays a pure function of one file's bytes (which is why
  `revision_of` is content-derived); a consumer can branch on the new field; but
  the six codes stay invisible and `E2018`/`E2023` stay mis-coded unless a fourth
  correction is added.
- **Option C — yes, but only the `use` resolution.** Run the resolver just far
  enough to report `E2019`, leaving the five provider/module-graph codes out of
  scope. Consequence: this is a third pipeline that shares code with neither of
  the existing two, and Finding Q shows how easily a shared-code split produces
  a mis-coded diagnostic.

**Why this is a decision.** Option A makes AIS non-hermetic. Option B keeps it
hermetic and makes it permanently incomplete. Both are coherent; they differ in
what the protocol *is*, not in whether the code is correct. The prior-art RFC's
§9 question 3 ("whether to expose uses/O1") is adjacent but not the same: this is
about diagnostics, not references.

**A note on the exit code, which is not a decision.** `cmd_ais_slice` and
`cmd_ais_delta` returning `ExitCode::SUCCESS` unconditionally (`src/main.rs:497`,
`:526`) contradicts the module's own doc comment at `:395-400`. Whichever option
is chosen, that is a defect and should be corrected with the rest.

## 3. Grammar delta

**None.** AIS is not a language surface. `grep -n "AIS" docs/LANGUAGE_SPEC.md`
returns one hit — prose at `:2578` — and the specification ends at
`## 36. Generics` (`:4009` of 4099 lines). No production grammar, type rule, or
runtime behaviour changes under any option in §2. `docs/grammar.md` is untouched.

The only specification-adjacent change is to `docs/engineering/AIS.md`, whose
schema field table (`:86-101`) and versioning section (`:42-55`) are the closest
thing AIS has to a normative text.

## 4. Semantics

### 4.1 What the document is

AIS is a projection of the compiler's conclusions, not a second analysis.
`src/ais.rs` derives every field from the AST plus one checker call
(`Checker::module_narrowings`). The projection has three layers — `symbols`,
`narrowings`, `diagnostics` — and three identity levels per symbol
(`semantic_type`, `type_family`, `runtime_value_kind`).

`revision_of` (`src/ais.rs:683`) is FNV-1a 64 over the exact source bytes, so a
revision is content-derived and a `Slice`/`Delta` can name the revision it was
computed against without a file handle. That is the property Option B of §2.3
preserves and Option A weakens, and it is worth naming because it is the strongest
argument for the design AIS has.

### 4.2 The four contracts this RFC found broken

| Contract | Where stated | Finding | Broken how |
|---|---|---|---|
| A producer never removes a field in a minor version | `AIS.md:53-55` | H, C | value set vs. field set is undefined |
| Capability bits are honest | `src/ais.rs:66-71` | C | bits are a constant, always `true` |
| A slice carries its diagnostics | `src/ais.rs:848-853`, `:868` | L | always `Vec::new()`, data in scope |
| A rejected program still yields its diagnostic | `src/ais.rs:1001-1002` | M | resolver diagnostics never produced |

A fifth, softer contract — `Symbol.range` is "where it is declared" — has no
stated meaning at all (§1.1), which is why it needed D2 rather than a fix.

### 4.3 What is working and must not regress

The repair touches four functions. Everything below is verified today and must
still hold afterwards:

- `revision_of` is content-derived and stable.
- `Delta` classifies by name via a `BTreeMap` and treats `moved` as not-a-change
  (`Delta::is_empty`), matching `AIS.md:74-78`.
- `same_declaration` scrubs ranges, so delta classification is extent-independent.
- Slice depth is a true transitive closure over the signature relation; budget is
  `budget.saturating_sub(1)` with `omitted` counted correctly (§1.8).
- An undeclared slice target yields `symbol: None` and exit 0, not an error.
- `CheckedDocument` is `#[must_use]` and the parse-error path truncates to one
  diagnostic (`src/main.rs:436`), matching the compiler's one-diagnostic policy.
- Nested `parameters[].range` and `fields[].range` are exact except for Finding I.
- `aura_version`/`language_version`/`ais_version` are reported on every document
  and by `aura://versions`.
- `NS_SCHEMA`'s three resources all answer, and `aura://capabilities` returns its
  four `{ais, compiler, jev, mcp_adapter}` blocks.

### 4.4 How the fixes interact

The findings are not independent, and a repair that treats them as a list will
regress one into another:

1. **D2 changes the extent; Finding I changes the start.** A `pub` field's range
   must move to the field name *and* grow to the field extent. Fixing only the
   start leaves `range.end` at `pub`'s end, i.e. a zero-width or negative range.
2. **D3 changes which diagnostics exist; the capability bit (C) must then be
   computed, not constant.** If the resolver runs, `diagnostics` becomes true for
   more documents — but the bit must still be `false` for a document produced
   without a checker pass, which is the parse-error path.
3. **Finding L must be fixed after D2**, because "diagnostics whose span touches
   the slice's resolution" needs a span that means something. Attaching
   diagnostics to keyword-only ranges would attach them to the wrong thing.
4. **Finding H's deletion must follow D1**, and if Option B is chosen the three
   variants must instead be documented as nested-only — a documentation change in
   `AIS.md:99-101`, not a code change.

### 4.5 One observation that is not a finding

`ais::block_diverges` (`src/ais.rs:1028`) has zero callers, `CompletionContext`
(`:635`) is constructed nowhere, and `diverges: true` appears zero times
outside the `false` literal at `:1016`. Together with `flow: false` and
`completion: false`, these describe a **reserved** part of the model.

Reserving is legitimate. The problem is that §1.4's honesty rule was written to
justify exactly this reservation, and it is applied to two bits while the other
three are constant-true. The dead code is not a defect; the inconsistency in how
the rule is applied is Finding C.

Note also that a `Checker::block_diverges` exists at `src/check/mod.rs:2728` and
**is** used. The AIS helper is a separate function with the same name and no
caller (§0 correction 1).

## 5. Migration

Nothing in this RFC changes a program. Migration concerns protocol consumers, and
each decision implies a different one.

| Decision | Choice | Consumer impact |
|---|---|---|
| D1 | Option A (`0.2`) | must re-read `aura://schema`; `ais_version` changes |
| D1 | Option B (additive `0.1`) | none; three `kind` values stay unreachable |
| D1 | Option C (`0.1` + removal) | a consumer matching all 11 `kind` values breaks silently |
| D2 | Option A (declaration extent) | every consumer computing with `range` gets 6.4× more bytes |
| D2 | Option D (additive field) | none; `range` keeps its current meaning |
| D3 | Option A (run resolver) | `aura ais` exits 1 on programs that exit 0 today |
| D3 | Option B (document the split) | none; a new `resolution` field appears |

The one migration that cannot be softened is D2 Option A: any external tool that
sizes a context window from `range` will change behaviour. In this repository no
such tool exists (§1.2), so the repository cannot assess it.

Findings B, D, J, and K are consumer-visible corrections with no migration
question: adding the four missing CLI verbs, deriving `schema.tools` and
`schema.delivery` from the tool table, and removing `name` from tools that discard
it (or documenting per-tool sensitivity) are all pure fixes. Finding E is a CI
change. Finding F is additive.

## 6. Enforcement

### 6.1 The evidence table

Every claim in §1 is reproduced by a command. `A` is a locally built
`target/debug/aura`; all fixtures are in fresh `mktemp -d` directories, one
`.aura` file per directory (a reused directory silently reports another file's
error — a probe discipline recorded in `AGENTS.md`).

| # | Probe | Command shape | Observed |
|---|---|---|---|
| A | range extent | `aura ais k.aura` on a 259-byte fixture | 10 symbols, reported 39 B, true 249 B, 6.4× |
| B | CLI dispatch | `aura ais <word>` for seven words | 3 verbs, 4 words read as paths, exit 1 |
| C | capability constancy | `aura ais` on empty / rejected / bad files | identical caps on all three |
| D | schema parity | JSON-RPC `tools/list` then `resources/read aura://schema` | 7 vs 6 tools, 3 of 7 delivery |
| E | harness wiring | grep for `ais_differential` in CI/scripts | 0 hits; harness exits 0 |
| F | `const` projection | `aura ais` on `const A: int` / `const B = 2` | `type_name` `None`, `None` |
| G | dependency relation | `aura ais slice` on a 5-function chain | body edges yield `[]` |
| H | `SymbolKind` census | `grep -rn 'SymbolKind::' src/` | 12 hits, 3 variants unconstructed |
| I | modifier ranges | `aura ais` on `pub x` / `mut z` | `b'pub'`, `b'mut'` |
| J | `aura_symbol` schema | four-argument matrix | `name`-only fails, `name` ignored |
| K | `name` sensitivity | six tools, with and without `name` | 1 of 6 changes output |
| L | slice diagnostics | `aura ais slice` on a program with `E2003` | slices exit 0, no `diagnostics` |
| M | resolver drop | `aura check` vs `aura ais` on `use Std::Math` | `1`/`E2019` vs `0`/none |
| N | mis-coding | six single-error programs | `E2018`→`E2003` |
| O | reserved namespace | `module Aura { … }` | `E2023`→`E2003` |
| P | resolver-only census | `codes::<NAME>` per file | six resolver-owned codes |
| Q | `use` is a no-op | `Item::Use { .. } => {}` in checker | `:1413`, `:3689` |
| R | `<stdin>` note | pipe through `check -` and `ais -` | note present, then absent |

### 6.2 The range arithmetic, reproducibly

The aggregate in §1.2 depends on one definition, stated once:

```
line_start(o) = offset of the first byte of the line containing o
line_text(o)  = the bytes of that line, excluding the trailing newline
true_extent   = len(line_text(symbol.range.start.offset))
```

For the 259-byte fixture the line extents are
`[(0,0,12),(1,13,26),(2,27,63),(3,64,89),(4,90,104),(5,105,118),(6,119,140),(7,141,182),(8,183,231),(9,232,258)]`
as `(line, start, end)`, and `sum(true_extent) = 249`, `sum(reported) = 39`,
`249 / 39 = 6.38…` → `6.4×`.

**Why this subsection exists.** Three different scripts produced `249/6.4`,
`1233/31.6`, and `1223/31.4` for the same fixture during this research. The first
was right; the second summed `line_start(next)` instead of the line end; the third
printed per-row extents with one definition and summed a different one. All three
are recorded in §0 correction 7. A protocol figure that three scripts can disagree
about is a figure that needs a test, which §6.3 provides.

The correct implementation is three lines:

```python
i = bisect.bisect_right(line_starts, start_offset) - 1
extent = line_starts[i] + len(line_texts[i]) - line_starts[i]
```

The `bisect` is not incidental: a naive
`next(i for i,x in enumerate(offs) if x <= a)` returns index `1` for **every**
symbol after the first, which is how the `1233` figure was produced.

### 6.3 Tests that would have caught each finding

The suite is large and green. Its shape is the problem: `tests/ais_properties.rs`
has 11 tests and 255 lines, `src/ais.rs` has 27, `src/mcp.rs` has 17, and none of
them asserts a range extent, a capability value, a resolver diagnostic, a slice
diagnostic, or parity between `tools/list` and `aura://schema`.

| Finding | Test that fails today | Where it belongs |
|---|---|---|
| A, I | assert `source[range]` equals the declaration text | `tests/ais_properties.rs` |
| B | assert each documented verb is reachable | `tests/cli.rs` |
| C | assert `diagnostics` bit equals `!doc.diagnostics.is_empty()` | `src/ais.rs` unit |
| D | assert `schema.tools == tools/list` and `schema.delivery` covers all | `src/mcp.rs` unit |
| E | make the harness a CI step or delete the claim | `.github/workflows/ci.yml` |
| F | assert `type_name` present for annotated `const` | `tests/ais_properties.rs` |
| G | assert a body-only edge appears, or document the relation | `tests/ais_properties.rs` |
| H | assert the emitted `kind` set equals the documented set | `src/ais.rs` unit |
| J, K | assert every advertised property is either required or inert-free | `src/mcp.rs` unit |
| L | assert a slice over a failing program carries the diagnostic | `src/ais.rs` unit |
| M, N, O | assert `check` and `ais` agree on `use`-bearing programs | `tests/cli.rs` |
| P, Q, R | assert the documented code set is reachable, or list exemptions | `tests/cli.rs` |

Each of these is a **positive** assertion — it fails when the defect is present.
That is the only kind this RFC accepts. The existing MCP test
`tools_list_advertises_small_semantic_operations` asserts six names and would
pass with either list; it is the reason Finding D survived.

### 6.4 The repair, and how it must be verified

No repair is written. Gate G02 forbids changing a source surface before its RFC is
accepted, and RFCs 0002-0005 were each reverted under the same rule. The shape
below is what §2's decisions authorize, not work performed.

Verification obligations, in the order they bind:

1. **Mutation in both directions.** For each guard, revert the fix and confirm the
   test fails; re-apply and confirm it passes. A test that passes on both is not a
   guard (`AGENTS.md` rule 6).
2. **Extent arithmetic against §6.2**, not against a new script. The fixture and
   its 249-byte answer are the oracle.
3. **Capability bits recomputed**, including the parse-error path, which must keep
   `diagnostics: false` if no diagnostic was attached.
4. **`slice` diagnostics after D2**, since the span must be meaningful first
   (§4.4 item 3).
5. **Delta classification unchanged** — `same_declaration` scrubs ranges, so this
   should hold; it must be asserted, not assumed.
6. **Both CLI axes of Finding B**, no-file and with-file, because the fall-through
   masks the difference.
7. **Corpus neutrality** — the 125-file `.aura` corpus must produce identical
   status and output for `check` and `run`.
8. **`ais_version` consistency** across `aura ais`, `aura://versions`, and
   `aura://schema` under D1's chosen option.

### 6.5 Fixtures used

| Purpose | Location | Size | Result |
|---|---|---|---|
| Authoritative range fixture | `mktemp -d`, `k.aura` | 259 B, 10 lines, md5 `cd56b1e2…68cb` | 10 symbols, 0 diagnostics |
| Dependency chain | `mktemp -d`, one file | 5 functions, 3 structs | `caller` `[]`, `use_c` `['C']` |
| Modifier ranges | `mktemp -d`, one file | `pub x`, `mut z`, `_p` | `b'pub'`, `b'mut'`, `b'_p'` |
| `const` projection | `mktemp -d`, one file | `const A: int`, `const B = 2` | `None`/`'int'`, `None`/`None` |
| Slice diagnostics | `mktemp -d`, one file | `E2003` at `2:36` | snapshot has it, slices do not |
| Resolver matrix | six `mktemp -d` dirs | one program each | six rows of §1.13 |
| Prior-art re-measure | `mktemp -d`, `app.aura` | 454 B | `slice/source = 7.10×` |

The authoritative fixture is duplicated byte-for-byte rather than rebuilt from
memory. A rebuild attempt during this research failed with `E1006: expected a
newline between statements, found `M`` because `module M` is not valid Aura — the
working form is `module M { }` — and the original uses single-line struct, enum,
and trait bodies. Rebuilding it from recollection shifts every byte offset and
invalidates the whole table.

## 7. Open items

1. **D1** (§2.1) — capability honesty and the AIS version.
2. **D2** (§2.2) — what `Symbol.range` means.
3. **D3** (§2.3) — whether the AIS document path runs the resolver.
4. **Is `AIS.md` normative?** It is an engineering document, not a specification.
   `LANGUAGE_SPEC.md` mentions AIS once and ends at §36, so four of the contracts
   in §4.2 have no specification behind them. If AIS is a protocol the project
   promises, the promise needs an authority.
5. **Prior-art status.** `AIS_AGENT_NATIVE_RFC.md` documents §9 open questions
   (precision vs. size, stable symbol ids, uses/O1, `verify(proposed_source)`)
   that this RFC does not answer. Its byte figures are unreproducible (§1.6).
   Whether it is superseded, merged into, or parallel to this RFC is undecided.
6. **The `name` property** (Findings J, K) — remove it from tools that discard
   it, or document per-tool sensitivity? The schema is currently uniform and the
   semantics are not.
7. **`aura ais slice` and `delta` exit codes** (§1.12). Not a decision: the
   behaviour contradicts the module's own doc comment (`src/main.rs:396-400`).
   Recorded here so it is not lost if D3 is deferred.
8. **Finding E's remedy.** Wire `tests/ais_differential.mjs` into CI, or delete
   the sentence claiming it runs there. Wiring costs a job; deleting the sentence
   costs the only cross-condition structural comparison in the tree.
9. **Whether `Symbol` should carry the resolved-signature `dependencies` of G at
   all**, or whether that belongs to `Slice` only. Today both compute it from the
   same signature-only function, so a change to one changes the other.

Nothing in §7 blocks the three decisions. Items 4 through 9 are consequences of
whichever options are chosen.
