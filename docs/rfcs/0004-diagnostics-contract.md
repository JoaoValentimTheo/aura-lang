# RFC 0004 — The Diagnostics Contract

- **Status:** Proposed — reconstructed for the NEBULA 0.3.2 campaign (Directive
  008, gate G02). **Not accepted.** It records one behavioural requirement that
  needs human authorization (§2.3 D3) and one hygiene repair that gate G02
  forbids implementing before acceptance (§2.2 D2).
- **Scope:** the projection from a semantic diagnostic to every presentation
  surface; location fidelity; format-injection safety; the `aura explain`
  catalogue; and golden-snapshot coverage. No grammar change.
- **Authorities:** `docs/LANGUAGE_SPEC.md` §3.6.3 (escape table, 330–357),
  §28.3 (CLI, 3450–3460), §30 (3535 ff.), §30.2 (3548), §30.3 (3601),
  §30.4 (3616); `src/error.rs` (`Diag` 30–114, `SourceDiagnostic` 117–201,
  `line_col` 203–229, `render_with_source` 224, `render_with_sources` 236,
  `DiagnosticReport` 262–313, `codes` 327–438); `src/diagnostic.rs`
  (20–166 renderer, 170–270 tests); `src/main.rs` (268–292
  `render_cli_report`, 293–323 `split_subcommand_args`, 596–616 `cmd_eval`);
  `src/run/value.rs` `display`/`write_repr` (605–648); `src/stdlib/mod.rs`
  `print` (97–115); `src/ais.rs` (40, 64–72, 130–158); `playground/runtime/src/lib.rs`
  (403–455); `docs/FILESYSTEM_MODULES_DESIGN.md` §5/§6 and 725–729;
  `docs/engineering/AIS.md`; `docs/errors.md`.
- **Human decisions:** two. **D3** — whether displayed columns must be
  *visual* (grapheme-cluster) columns instead of the Unicode-scalar columns the
  spec currently mandates (§2.3). And §7 item 13 — whether a directory whose
  name is not an ASCII identifier may be skipped silently (§1.4). Neither is
  resolved here.
- **Provenance** is stated in §0.

## 0. Provenance

The campaign brief (Directive 008 §3.4, gate `NBL-013`) requires a
*diagnostics contract* in which `SourceDiagnostic` and `SourceMap` are the only
semantic facts, every surface projects from them, no format injection is
possible, `aura explain` is a versioned catalogue, and golden snapshots of text
and ANSI exist. The brief also requires that multi-file diagnostics be
*actually exercised* through the CLI.

No pre-existing Nebula artifact (RFC draft, inventory, contract) existed in the
tree when G02 began; §0.4 of `docs/engineering/NEBULA_CAMPAIGN.md` records that
absence. This RFC is a **reconstruction from repository evidence**, not a
transcription of an approved design. Every quantitative claim below is backed
by a probe or a byte-level capture recorded in §6, re-verified against the tree
at `fc49c5ee`.

Two claims of an earlier draft of this research were falsified by later probes
and are **corrected in place** rather than deleted, because the correction is
itself evidence: the "multi-file diagnostics are never exercised via the CLI"
claim (§1.5) and the "`©` and `?` produce byte-identical diagnostics" claim
(§1.3). A third — that `--color` is silently ignored when malformed (§4.2) —
was also wrong and is corrected.

## 1. Motivation

### 1.1 The brief's requirement, and what the tree actually has

Gate G04 requires *"uma única projeção de `SourceDiagnostic`/`SourceMap`, sem
vazamento semântico para AIS/Playground/LSP"*. The tree has the substrate —
`SourceMap` exists (`src/source.rs`) and is already the multi-source holder used
by `module_graph.rs:408`, `repl.rs:56` and `error.rs:264` — but it has **five
independent renderers**, each formatting the same facts its own way, and no
shared projection layer. §1.2 enumerates them.

### 1.2 Five rendering paths, four formats, one set of facts

All five read the same `Diag` (three frozen public fields: `code`, `message`,
`span`) plus an optional `Location`. They share no formatter.

| # | Path | Entry point | Format produced |
|---|---|---|---|
| 1 | CLI `check`/`run` | `render_cli_report` `src/main.rs:268`, called at `:357` and `:389` | `file:line:col: error[Exxxx]: message` |
| 2 | `eval` | `DiagnosticReport::render` `src/error.rs:302` → `render_with_sources` `:236` → `render_with_source` `:224`; sole production call site `src/main.rs:608` | `file:line:col: Exxxx: message` |
| 3 | read failure | `render_with_source` directly at `src/main.rs:236` | same as (2) |
| 4 | AIS | `Diagnostic::from_diag` `src/ais.rs:130-148` | JSON object, no text |
| 5 | Playground | `diag_json` `playground/runtime/src/lib.rs:421-437`, `sourced_diag_json` `:439-455` | a **third** JSON schema, no span offsets |

The divergence is not cosmetic. For one identical source —
`let s: int = "x"` inside `fn main()` — the four consumer formats are:

| Consumer | Output |
|---|---|
| `check` / `run` | `a.aura:2:3: error[E3001]: `s` is annotated as `int` but its value is `string`` |
| `eval` | `<eval>:1:1: E3001: `s` is annotated as `int` but its value is `string`` |
| `ais` | `{"code":3001,"code_text":"E3001","severity":"error","message":"…","range":{…}}` |
| Playground | `{"code":"E3001","message":"…","line":…,"column":…}` (different key set, no offsets) |

`eval` differs in **three** ways at once: no `error[` / `]` brackets, no
severity word, and — see §4.1 — no colour handling whatsoever. The reason is
structural: `cmd_eval` (`src/main.rs:596-616`) routes through
`DiagnosticReport::render`, which uses `Diag`'s `Display`
(`src/error.rs:100-112`, `format!("E{:04}: {}", self.code, self.message)`),
whereas the CLI path routes through `render_diagnostic`, which builds the
`error[Exxxx]` shape itself. `Diag::render` and `Diag`'s `Display` are
byte-identical to each other, so the divergence is purely *which* formatter the
call site chose.

`ais` and the Playground additionally disagree with the CLI on **which codes
are even reported** (§1.6).

### 1.3 Location projection — spec-conformant, and still surprising

`line_col` (`src/error.rs:203-229`) counts Unicode scalar values in the line
prefix and counts lines by LF. `LANGUAGE_SPEC.md:177` says exactly that:
*"Spans are byte offsets; displayed lines count LF and columns count Unicode
scalar values."* **`line_col` is therefore correct, not defective.** Any
complaint about "wrong columns" must first decide whether the spec's rule is
the right rule, which is a decision, not a bug fix (§2.3).

The divergence between the printed column and the *visual* column is real and
runs in **both** directions. All six rows below were produced from **one
stated source shape**, with `$` as the offending character and the literal
varying only in the `a` binding:

```
line 2 == <indent>let a = <literal>; let y = $
```

Visual width is computed with Unicode `east_asian_width` `W`/`F` counted as 2,
combining marks counted as 0, and a 4-column tab stop.

| `<literal>` (line 2 indent) | Printed (scalar) | Visual | Delta |
|---|---|---|---|
| `"ab"`, two spaces | 25 | 25 | 0 |
| `"ab"`, one tab instead of two spaces | 24 | 27 | **−3** |
| `"ab"`, two spaces, CRLF line endings | 25 | 25 | 0 |
| `"😀"`, two spaces | 24 | 25 | **−1** |
| `"漢字"`, two spaces | 25 | 27 | **−2** |
| `"e" + U+0301`, two spaces | 25 | 24 | **+1** |

The **delta** column is the normative claim and is stable across source shapes;
the absolute columns are not, so a reader reproducing this must use the shape
above. An earlier draft of this table quoted absolute values measured on a
different, unstated shape; the deltas were right and the absolutes were not
reproducible, which is exactly the class of error this RFC asks implementers
not to make. The numbers above are the ones to cite.

A combining mark **over**-reports, because the scalar count includes the mark
while the visual count does not. An earlier framing of this RFC said the error
"always under-reports"; that was wrong and is withdrawn. Tabs are the largest
divergence and depend entirely on the viewer's tab width, so no implementation
can be correct for all viewers without the spec choosing a rule.
**One correction to an earlier record.** A previous note claimed `©` and `?` in
a *filename* produce byte-identical diagnostics. They do not: the *message* is
identical — both are `` `?` `` — but the *file name field* differs because the
filenames differ. Only the message is identical, and even that has a different
cause in the two cases than an earlier draft of this RFC asserted:
`?` is the literal character in the second filename, while the `` `?` `` in the
`©` case comes from the **lexer**, not from a module-name filter. See §1.4.

### 1.4 Format injection — the only defect in this RFC with a security shape

The renderer performs **no sanitisation anywhere**. `render_diagnostic`
(`src/diagnostic.rs:125-166`) concatenates the location, the code, and the
message; `styled()` (`:109-115`) returns `text.to_string()` unchanged when
colour is disabled — that is the only place any byte could be neutralised, and
it passes bytes through. `Presentation`'s doc comment claims *"no ANSI escape
ever enters it"* (`src/error.rs:117-201`); that is true of the *storage* side,
but nothing prevents a raw ESC sitting in `message`.

Four vectors were reproduced with byte capture (`od -c`) under
`NO_COLOR=1` **and** `--color never`, i.e. with colour fully disabled:

| # | Vector | Construction | Observed |
|---|---|---|---|
| A | raw ESC in the message | `fn main() { throw "<ESC>]0;pwned<BEL><ESC>[2J<ESC>[H" }` | 3 ESC bytes plus OSC-0 title-set, BEL, screen-clear and cursor-home delivered verbatim to stderr |
| B | legal **`\n`** in a string literal | `fn main() { throw "a\nb" }` | `wc -l` = **2**; the message splits into two lines |
| C | nested value | `throw ["a\nb"]` | `wc -l` = **2**; the quotes are not protective |
| D | raw newline in a **filename** | file literally named `bad<LF>name.aura` | `wc -l` = **2** — a complete forged pseudo-diagnostic line |

Vector B is the strongest, because it needs **no** unusual bytes — only a
perfectly valid escape the language already supports. Aura's escape set is
`\n \t \r \0 \\ \" \' \{ \}` (spec §3.6.3 `:330-357`, `src/lex/mod.rs:612-632`);
there is **no** `\u`, `\x` or octal form. An earlier probe that wrote `"\033"`
was therefore structurally invalid: it decodes to NUL plus the two literal
characters `33`, not to ESC. Only a **raw byte** in the file reaches the
message, and `"\0"` alone does reach it (`a\0b`; `od -c` shows `a \0 b \n`,
`wc -c` = **47**).

The mechanism is `Value::display` → `write_repr`
(`src/run/value.rs:605-648`), whose `Str` arm pushes raw bytes at both depths,
reached through `"uncaught value: {}", v.display()` at `src/run/mod.rs:217`,
`:973`, `:990`, `:1069` and `src/run/iterative.rs:2358`.

**The fix must not be global.** `print` (`src/stdlib/mod.rs:97-115`) uses the
same `display()` at **`:101`** (`line.push_str(&a.display())`) and *must* emit
raw newlines — that is its contract. So the sanitiser belongs in the
diagnostic projection, not in `Value::display` or `write_repr`.

Two aggravating facts:

* `tests/oracle/mod.rs:440` **independently duplicates** the
  `"uncaught value: {}"` format string. Sanitising only one site leaves two
  implementations free to drift.
* **No test anywhere** asserts anything about ESC bytes, newlines, or
  injection. The only `error[E` literal assertion in `tests/cli.rs` is at
  `:607`.

Module *paths* are gated by `is_aura_module_name` (`src/module_graph.rs:948-957`),
which admits only `[A-Za-z_][A-Za-z0-9_]*` that is not a keyword — so a
directory whose name fails that test is **never discovered as a module**, and
that is a separate mechanism from character rendering. Character rendering is
handled by the lexer: `src/lex/mod.rs:585-588` substitutes `?` for any
non-`is_ascii_graphic` byte before formatting the message, so a `©` in source
or in an entry filename surfaces as `` `?` `` and is *indistinguishable* from a
real `?`. **Vector D is therefore reachable only through the entry-file
argument, not through a discovered module.**

**A related finding — stated by design, silent in practice.** `is_aura_module_name`
**skips** a directory whose name is not an ASCII identifier rather than
rejecting it, and the frozen contract does specify why: *"Logical module
identifiers are byte-exact ASCII and case-sensitive, matching Aura
identifiers"* (`docs/FILESYSTEM_MODULES_DESIGN.md:594`). So the behaviour is
**specified, not a defect** — and this RFC does not propose changing it. What
the contract does not say is what the *user* sees. Probed with identical trees
differing only in the directory name: `mm/c.aura` (ASCII) is discovered and
reports `mm/c.aura:1:21: error[E1001]`, exit 1; `©/c.aura` with a matching
`©.aura` owner file is **not discovered at all** — `check` exits **0** with no
output whatsoever. A directory that is structurally indistinguishable from a
module except for its name therefore fails **silently**, and the frozen
contract's own matrix row (*"unused malformed reachable child source → parser
failure"*, `:774`) has no companion row describing this outcome. The RFC records
it as a documentation gap for a future decision, not as a defect to fix here.

### 1.5 Multi-file diagnostics **are** exercised — a corrected claim

An earlier draft asserted that multi-file diagnostics are never exercised
through the CLI. **That was false**, and the error was a probe defect, not a
tree defect: the child module must declare a *reachable* item, and the entry
must call it.

A correct reproducer — `main.aura` calling `sub::child::helper()`, an owner file
`sub.aura`, and `sub/child.aura` returning `$` — yields
`sub/child.aura:1:26: error[E1001]: unexpected character `$``.
`tests/cli.rs:394-419`
(`filesystem_child_parser_resolver_and_checker_errors_keep_file_names`) asserts
exactly this shape for `E1001`, `E2019` and `E3005`. The gate's requirement is
therefore **partially** satisfied today, for three codes.

What is *not* satisfied is the **resolvability** of the printed path:

| Property | Observed |
|---|---|
| Printed path is relative to the **entry file's parent**, not the cwd | From `$D/sub`, `aura check ../main.aura` prints `sub/child.aura:1:26` — byte-identical to the root run |
| …so it may be unresolvable from where it is printed | `test -f sub/child.aura` in `$D/sub` → **NO** |
| The **entry file itself** loses its directory | `aura check d1/e.aura` → `e.aura:1:21: error[E1001]` |
| Mechanism | `src/native_source.rs:53-56`, `portable_path` `:495-508` |
| Behaviour is specified | `docs/FILESYSTEM_MODULES_DESIGN.md:725-729` |
| Tests **enshrine** the ambiguity | `tests/cli.rs:415-418` and `:562-565` assert the absolute project path is **absent** |

This is intended behaviour with a real UX cost: an editor or CI log consumer
cannot turn the printed location into a file. Recording it as a defect would be
dishonest; recording it as *satisfying the gate* would also be dishonest. It
needs a decision about whether the printed identity should become
source-root-relative **with** the root, or stay bare.

### 1.6 The AIS surface disagrees with the CLI about the program

`aura ais` cannot see filesystem modules **at all**. `src/main.rs:434` and
`:440` call `parse(src)` and then `ais::checked_document(...)` with no
`native_source` and no module graph; `src/mcp.rs:291-292` does the same.

| Tree | `check` | `run` | `ais` |
|---|---|---|---|
| `main.aura` calling `sub::m::helper()`, owner `sub.aura`, `sub/m.aura` defining `pub fn helper` | exit 0 | exit 0, prints `1` | **exit 1**, `E2003 undefined function `sub::m::helper`` |

Two further divergences on the same input:

* `use sub::m` with no `sub/` → `check` reports **`E2019`** (`unknown module
  `sub` in `use``); `ais` reports **`E2003`** (`undefined function
  `sub::m::helper``). Same source, different code and different phase.
* A **locationless** diagnostic is silently dropped by AIS. A dangling symlink
  child makes `check` exit 1 with `E2022` (see §1.7), while `ais` exits **0**
  with no `diagnostics` key at all — not an empty list, an absent field.

AIS's documented schema is also wider than its output.
`docs/engineering/AIS.md:98` documents `notes` and `help`; the emitted key set
is exactly `['code','code_text','message','range','severity']` — nine keys short
of the documented shape. `Diagnostic::from_diag` hardcodes
`severity: Severity::Error.as_str()`, `notes: Vec::new()` and `help: None`
(`src/ais.rs:130-148`), and `with_presentation` (`:150-158`, and its twin at
`src/error.rs:166`) has **two definitions and zero call sites**. `Severity` is
effectively `Error`-only in any case: `Warning` and `Note` appear only inside
`#[cfg(test)]` (`src/diagnostic.rs:138-139`, `:254`).

AIS is also the only surface that prints a **resolvable** path —
`source_name` is the caller's path exactly as given, absolute if passed
absolute — which makes it a fourth, independent path projection:

| Invocation | `source_name` |
|---|---|
| `aura ais d1/e.aura` | `d1/e.aura` |
| `cd d1 && aura ais e.aura` | `e.aura` |
| `aura ais /abs/d1/e.aura` | `/abs/d1/e.aura` |

Finally, **no AIS CLI test covers modules**: all ten `ais_*` tests in
`tests/cli.rs` (`:678`–`:851`) are single-file.

### 1.7 Two spec claims that the implementation does not honour

**§30.3 (`:3601-3615`) says every user-visible rejection carries *"a code, a
location (span), and a message"*.** Locationless diagnostics exist and are
reachable. A dangling symlink child produces, with exit 1 and **no
`file:line:col:` prefix at all**:

```
error[E2022]: cannot enumerate children of logical module `<root>`: reachable module candidate dangle.aura is a symlink
```

Construction sites: `module_graph.rs` 811/842/869/894/908/920/968,
`resolve.rs:51/67`, `lib.rs:180/187` (all locationless), plus the sourced ones
at 560/652/665/930/995; helper `locationless_report` at
`module_graph.rs:963-975`. `Diag::locationless` explicitly refuses to invent a
fake `SourceId`, which is the right design — the **spec sentence** is what is
wrong.

**§30.4 (`:3616-3628`) says every §30.2 code is *"covered by
`tests/grammar.rs::every_documented_error_code_is_reachable`"*.** Measured
machine-side:

| Figure | Value |
|---|---|
| `codes` declarations in `src/error.rs` | **51** |
| distinct code values | **50** (`5002` is shared by `PY_UNSUPPORTED` and `CAPABILITY_UNAVAILABLE`) |
| rows in the §30.2 table (`:3548`) | **45** |
| rows in `docs/errors.md` | **50** |
| `codes::` references inside `error_samples()` (`tests/grammar.rs:119`) | **38 distinct** |
| dedicated tests outside `error_samples()` | **2** — `E4027` (`:271-276`), `E5002` (`:279-288`) |
| **codes in §30.2 that are neither sampled nor dedicated** | **6** — `2020, 2021, 2022, 4099, 4999, 5001` |

The exact gap is **six codes**, not the looser "38 of 50" framing.

**`E5003` is deliberately excluded from that gap.** It occurs exactly once in
the whole tree — its own `pub const` at `src/error.rs:413`. But the spec already
discloses this at `:3598` and `:3973` (*"defined in the implementation but is
not currently produced; it is not part of the normative surface"*), and
`docs/errors.md:93` says *"Reserved; not currently produced"*. That is honest
documentation of a dead constant, not a reachability defect. `E5003` is also
absent from the §30.2 table, so §30.4's claim does not even cover it.

Nor are the six all equally reachable:

* **`E4099`** is internal-only by design. The spec labels it `(internal)` at
  `:3593` and says at `:3974` that it is normalised at the boundary. Probed: an
  uncaught `throw` crossing two call frames reports **`E4026`**, not `E4099`.
  `E4999` is likewise guarded by §30.3's own rule (`:3612`) that it MUST NOT be
  reachable — so **§30.4 demands coverage of two codes that §30.3 forbids
  reaching.** That is a contradiction inside the specification, independent of
  the implementation.
* **`E2020` and `E2021`** were **not** reached by hand across repeated attempts —
  sibling `a/m.aura` + `m.aura` exits 0, and a symlinked child trips `E2022`
  instead. They need deliberate module-graph orchestration. This RFC therefore
  does **not** claim they are unreachable; it claims only that they are not
  *exercised*, which is what §30.4 actually promises.

### 1.8 `aura explain` and a documentation gap in the CLI contract

There is no `explain` command and no per-code catalogue reachable from the
binary. Probed:

| Invocation | Result |
|---|---|
| `aura explain E1001` | exit 2, ``unknown command `explain`; try: run, check, eval, repl, version, ais, mcp`` |
| `aura help` | exit 2, same unknown-command shape — `help` is a flag-less help, not a command |

The command inventory is `COMMANDS` (`src/main.rs:25-48`): **seven** entries
with `json`, **five** without. Both the spec's §28.3 table (`:3450-3460`) and
its feature bullet at `:88` list exactly the five
`run`/`check`/`eval`/`repl`/`version`. `ais` and `mcp` are already
undocumented; `explain` would be a third. So **adding `aura explain` requires a
spec amendment** to §28.3 and `:88`, and
`tests/syntax_conformance.rs:574-604`
(`cli_unknown_command_hint_lists_every_implemented_command`) currently asserts
only the five and will need extending.

A stale citation sits adjacent to this: the `COMMANDS` doc comment cites
*"LANGUAGE_SPEC §47/§48"*, but the specification ends at §36
(`:4009`). It is a comment, not behaviour, but it is the kind of drift this
contract exists to prevent.

Two CLI presentation inconsistencies belong here too:

* `--color` is **not** a global flag. `aura --color always check a.aura` →
  ``unknown command `--color`; …``, exit 2. It must follow the subcommand.
* `aura check --json a.aura` → `aura check takes exactly one source argument`,
  exit 2. `check` accepts no `--json`, while `run --json` interprets the *path*
  as source — two unrelated behaviours behind one flag name.

## 2. Decision

This RFC proposes one contract, one withheld repair, and one escalation.

**The contract (proposed for acceptance as-is).** A single projection function
takes `(&SourceMap, &SourceDiagnostic)` and returns a `Presentation`-shaped
value — severity, code, message, location, optional source line — with:

1. every text field sanitised (ESC and control bytes rendered as visible
   escapes, newlines included) at the **projection** boundary, never upstream;
2. the location computed **once** per render — `render_with_sources`
   (`src/error.rs:236`) currently calls `line_col` **twice per render**;
3. severity carried as data, so `ais` can stop hardcoding `Error`;
4. no message-text surgery: the
   `; filesystem modules are unavailable for <stdin>` suffix that
   `render_cli_report` appends when `d.code == UNKNOWN_MODULE`
   (`src/main.rs:288-290`) becomes a structured flag the formatter decides how
   to show;
5. every surface — CLI, `eval`, AIS, Playground — consuming that one value.

**D1 — sanitisation is accepted as required, not optional.** No semantic
decision is embedded: the sanitised form is a presentation concern only, and the
spec already says presentation is not normative (`:94`).

**D2 — the implementation is withheld.** Gate G02 forbids coding the affected
surface before RFC acceptance. `src/check/mod.rs` remains byte-identical to
baseline (`1232b687…f532`); the same discipline applies to `src/diagnostic.rs`,
`src/error.rs` and `src/main.rs` under this RFC.

### 2.3 D3 — the escalated decision (location fidelity)

Everything in §1.3 is spec-conformant. Changing it is a **specification
change**, so it is escalated rather than assumed. Three lawful outcomes:

| # | Option | Cost | Consequence |
|---|---|---|---|
| 1 | Keep Unicode-scalar columns; document the divergence and ship an editor hint (byte offset is already in the span) | Zero code change | Tabs stay wrong for every viewer with a non-4 tab width; CJK/emoji stay off by 1–2 |
| 2 | Switch displayed columns to **grapheme clusters** | Requires amending `LANGUAGE_SPEC.md:177`; changes every recorded column for non-ASCII sources; needs a Unicode segmentation dependency or a hand-rolled table | Correct for combining marks; still wrong for double-width CJK/emoji, because a grapheme cluster is not a display cell |
| 3 | Switch to **display cells** (East Asian Width + tab expansion), with tab width fixed by the spec | Requires amending `:177` and choosing a tab width; a real algorithm change | Only option that matches what a terminal shows; largest blast radius, and every golden snapshot must be regenerated |

The RFC takes **no position** on which is correct. It records only that option
1 is the status quo and that any other choice is a spec amendment, not a bug
fix. Note that option 2 alone does **not** solve the CJK case, which is the
most visible one — so the intuitive choice is not obviously the right one.

## 3. Grammar delta

**None.** No token, production, or precedence changes. `aura explain <code>`
is a new CLI subcommand, which is a §28.3 table row and a `COMMANDS` entry, not
a grammar change.

## 4. Semantics

### 4.1 What is already correct and must not change

* `line_col`'s char counting and LF line counting (§1.3) — conformant to `:177`
  until D3 says otherwise.
* `Diag`'s three frozen public fields and `Diag::locationless`'s refusal to
  invent a `SourceId` (`src/error.rs:58-114`).
* `print`'s raw output (`src/stdlib/mod.rs:97-115`).
* The source-root-relative path identity of
  `docs/FILESYSTEM_MODULES_DESIGN.md:725-729` — the *rule* stays; only the
  question of whether it should carry the root is open (§1.5).
* `render_diagnostic`'s colour policy as a *policy* (see §4.2 for the defects
  in its argument parsing).

### 4.2 What the repair changes

* **Sanitisation** at the projection boundary, applied to the message **and**
  the location string, covering ESC, C0 controls and newline.
* **Single `line_col` computation** per render.
* **Structured severity** in AIS, replacing the hardcoded `Error`.
* **Structured stdin suffix**, replacing message concatenation.
* **One JSON schema** shared by AIS and the Playground.

Colour policy belongs to the contract because it is currently
**implementation-only and self-contradictory**. The resolution order is
explicit override → `AURA_COLOR` (if parseable) → non-empty `NO_COLOR` → `Auto`,
with terminal detection applied only for `Auto`. Verified behaviours:

| Configuration | ANSI emitted |
|---|---|
| `--color always` | yes |
| `NO_COLOR=1` | no |
| `AURA_COLOR=always` | yes |
| `AURA_COLOR=never` + `--color always` | yes (flag wins) |
| `NO_COLOR=1` + `AURA_COLOR=always` | **yes** (`AURA_COLOR` wins over `NO_COLOR`) |
| `NO_COLOR=` (empty) + pipe | no |
| `NO_COLOR=0` | **no** — any non-empty value disables colour, including `0` |

Two genuine bugs sit inside that table:

* **`color_choice` (`src/main.rs:238-249`) is LAST-WINS over the whole raw
  argument list, and an unparseable value RESETS to `Auto`.**
  `check --color always --color bogus` → **0** ANSI, while
  `check --color bogus --color always` → ANSI. An earlier note said a bogus
  value is "silently ignored"; that was wrong — it *clears* an earlier valid
  override.
* **`run` double-consumes an after-path `--color`.** `run a.aura --color always`
  gives the program `["--color","always"]` **and** enables ANSI for the
  compiler; `run --color always a.aura` gives the program `[]` and also enables
  ANSI. `split_subcommand_args` (`src/main.rs:293-323`) deliberately stops at
  the source path for `run`, but `color_choice` reads the raw argv regardless,
  so the same bytes mean two things at once.

`eval` disregards all of it: probed with `AURA_COLOR=always`, `eval` emits
**zero** ESC bytes, and it rejects `--color` outright
(`aura eval takes exactly one argument; wrap the code in quotes`). Three
disregard paths, one command.

Documentation: colour is described **only** outside the specification —
`docs/release-notes/v0.3.1.md:103` and `KEYSTONE_SCOPE_MANIFEST.md:56` vs.
`:130` (a within-document drift pair). `grep -i color docs/LANGUAGE_SPEC.md`
returns exactly **one** hit, `:3368` (`shapes::Color::Red`), which is unrelated.
Either the policy is normative and belongs in the spec, or it is not and the
two external documents should stop implying a contract.

### 4.3 `aura explain` as a versioned catalogue

The catalogue must be **data**, versioned with the compiler, and generated from
the same source as `codes` so it cannot drift. `docs/errors.md` is already the
50-row human catalogue and groups codes into five phase families, which is the
natural schema:

| Phase | Range | Grouping already in `docs/errors.md` |
|---|---|---|
| Lexical | `E1xxx` | lexical errors |
| Name / module | `E2xxx` | resolution and module-graph errors |
| Type / check | `E3xxx` | static checks |
| Runtime | `E4xxx` | runtime and value errors |
| Boundary | `E5xxx` | host, capability and internal errors |

The catalogue should state, per code: the code, its phase, whether it is
internal, and — honestly — whether it is producible. `E5003` is the precedent:
it already carries *"Reserved; not currently produced"*, and a catalogue that
says so is more useful than one that pretends.

### 4.4 Deliberately not changed

* No `\u`/`\x`/octal escape is added. §1.4's vector A remains reachable through
  a raw byte; sanitisation makes it *visible* rather than removing the byte.
  Adding escapes would enlarge the lexer surface and is out of scope.
* No rich-diagnostic vocabulary (suggestions, secondary labels, applicability,
  confidence). A census finds **zero** occurrences of `Suggestion`,
  `Applicability`, `Label`, `secondary_spans`, `primary_span`, `edit_range` or
  `confidence` in the tree. The contract is about *one* projection of the facts
  that exist, not about new facts.
* No LSP server. `docs/engineering/TOOLING_RECONCILIATION.md:45-53` records the
  smallest coherent scope as "an LSP server that serves AIS documents" with
  status "architecture/contract in Keystone". There is no LSP crate and no
  implementation; this RFC only guarantees that whatever consumes the
  projection later sees the same facts.
* No snippet/caret rendering. There is no `source_line`, `code_frame`, caret
  renderer, snapshot directory, or `insta`/`expect-test`/`trybuild` dependency
  in the tree. §5 sequences that as new work rather than assuming it.

## 5. Migration

* **Sources:** none. No valid program changes behaviour. The four injection
  vectors stay *valid*; only their presentation changes, and only in the
  diagnostic text.
* **Output text:** sanitisation changes the rendered bytes of any diagnostic
  whose message contains a control byte or newline. This is a presentation
  change, permitted by `:94`. No code, phase or exit status changes.
* **`eval` shape:** unifying the formatter would make `eval` print
  `error[E3001]` instead of `E3001`. That is a **visible output change** and
  should be called out in release notes; it is also the point of the contract.
  If compatibility matters more, `eval` keeps its shape and the unification is
  documented as partial — but then the "single projection" claim is weaker and
  should say so.
* **Colour flags:** fixing last-wins and the `run` double-consumption could
  change output for a script that passes `--color` twice or after a path.
  Nothing in the tree does; checking that is part of §6.
* **Columns:** if D3 selects option 2 or 3, every recorded non-ASCII column
  changes and golden snapshots must be regenerated.
* **`aura explain`:** purely additive.

## 6. Enforcement

### 6.1 Evidence — the discriminating probes

Probes are one `.aura` file per directory (eager structural discovery makes a
stray malformed child fail the whole directory,
`docs/FILESYSTEM_MODULES_DESIGN.md:399`); nested trees need an **owner file**.
All captures below were taken with colour disabled (`NO_COLOR=1 --color never`)
and read with `od -c`, on a binary built from `fc49c5ee`.

| Probe | Input | Result |
|---|---|---|
| `raw_esc_message` | `throw "<ESC>]0;pwned<BEL><ESC>[2J<ESC>[H"` | ESC bytes present in output; `ESC_BYTES=3` |
| `nl_message` | `throw "a\nb"` | `wc -l` = **2** |
| `nl_nested` | `throw ["a\nb"]` | `wc -l` = **2** |
| `nl_filename` | file named `bad<LF>name.aura` | `wc -l` = **2** |
| `nul_message` | `throw "a\0b"` | NUL preserved verbatim; `od -c` = `a \0 b \n`, `wc -c` = **47** |
| `child_path` | `main.aura` + owner `sub.aura` + `sub/child.aura` | `sub/child.aura:1:26: error[E1001]` |
| `child_path_from_sub` | same, run as `../main.aura` from `$D/sub` | byte-identical; `test -f sub/child.aura` → **NO** |
| `entry_path` | `aura check d1/e.aura` | `e.aura:1:21: error[E1001]` — directory lost |
| `cols` | six line shapes, one stated shape (§1.3) | printed 25/24/25/24/25/25 vs visual 25/27/25/25/27/24; **deltas 0/−3/0/−1/−2/+1** |
| `eval_shape` | `eval 'let s: int = "x"'` | `<eval>:1:1: E3001: …`, exit 1 |
| `eval_colour` | same with `AURA_COLOR=always` | **0** ESC bytes |
| `ais_modules` | module tree, `ais main.aura` | exit **1**, `E2003` (vs `check` exit 0) |
| `ais_divergent_code` | `use sub::m`, no `sub/` | `check` → `E2019`; `ais` → `E2003` |
| `ais_locationless` | dangling symlink child | `check` exit **1** `E2022`; `ais` exit **0**, no `diagnostics` key |
| `ais_source_name` | three invocations | `d1/e.aura` / `e.aura` / `/abs/d1/e.aura` |
| `stdin_suffix` | `use nosuch` on stdin | `… in `use`; filesystem modules are unavailable for <stdin>` |
| `explain_absent` | `aura explain E1001` | exit 2, unknown command |
| `color_last_wins` | `check --color always --color bogus` | **0** ANSI |
| `color_reset` | `check --color bogus --color always` | ANSI |
| `run_double_consume` | `run a.aura --color always` | program args `["--color","always"]`, ANSI on |
| `nonascii_dir_skipped` | `©/c.aura` + `©.aura` owner, entry `main.aura` | `check` exit **0**, no output; identical ASCII tree → `mm/c.aura:1:21` `E1001`, exit 1 |
| `lexer_col_placeholder` | `©` in source / in entry filename | message shows `` `?` `` from `src/lex/mod.rs:585-588`; entry filename field keeps the real `©` |

### 6.2 Mutation verification in **both** directions

A test that passes on fixed code proves nothing until it fails on broken code.

| Guard | Mutation | Expected |
|---|---|---|
| sanitisation | remove the `\n` escaping | the `nl_message` test must fail |
| sanitisation | remove the ESC escaping | the `raw_esc_message` test must fail |
| sanitisation site | move sanitisation into `Value::display` | a `print` raw-output test must fail — this is the mutation that proves the fix is in the right *layer* |
| single `line_col` | restore the second call | a call-count test must fail |
| severity | hardcode `Error` again | the AIS severity test must fail |
| stdin suffix | restore message concatenation | a message-shape test must fail |
| AIS parity | drop `native_source` again | the `ais`-sees-modules test must fail |
| locationless | skip locationless in AIS | the `E2022`-in-AIS test must fail |
| colour | restore "bogus is ignored" | the last-wins test must fail |

Each row must be demonstrated failing before the corresponding fix lands.

### 6.3 Whole-suite and whole-corpus gates

* `cargo test --locked --all-targets --all-features` — **1477** tests at
  baseline; must not decrease.
* `cargo test --locked --all-targets --no-default-features --features
  cli,repl,json,regex,time` — **1361** at baseline.
* `cargo clippy --locked --all-targets --all-features -- -D warnings` — clean.
* `cargo +1.83.0 check --locked --all-features` — clean (MSRV).
* Corpus: **125** tracked `.aura` files; status changes must be **zero** unless
  a §5 migration item says otherwise. This was the discipline that made the
  RFC 0003 repair measurable.
* `node playground/tests/node/run-all.mjs` (**992**) and
  `node website/tests/run-all.mjs` — the Playground consumes path 5, so its
  suite is part of this contract, not adjacent to it.

### 6.4 Regression coverage this RFC requires before the repair may land

1. `raw_esc_message` — raw ESC never reaches stderr unescaped.
2. `nl_message` and `nl_nested` — a message is always exactly one line.
3. `nl_filename` — a filename is sanitised in the location field.
4. `nul_message` — NUL is made visible, not emitted.
5. `print_raw` — **`print` still emits raw newlines** (the anti-regression that
   pins the fix to the projection layer).
6. `single_line_col` — `line_col` is called once per render.
7. `ais_sees_modules` — `ais` agrees with `check` on a module tree.
8. `ais_severity` — severity is carried, not hardcoded.
9. `ais_locationless` — a locationless diagnostic is present, not dropped.
10. `stdin_suffix_structured` — the suffix is a flag, not concatenated text.
11. `eval_matches_cli` — the `eval` form is either identical to the CLI form or
    explicitly documented as divergent.
12. `colour_last_wins` — the reset behaviour is either fixed or documented.
13. `run_flag_ownership` — an after-path `--color` is owned by exactly one
    consumer.
14. `explain_catalogue_complete` — every non-internal code in `codes` has a
    catalogue entry, and every catalogue entry exists in `codes`.
15. `explain_is_listed` — `explain` appears in `COMMANDS`, in the help text,
    and in the unknown-command hint.
16. `golden_text` and `golden_ansi` — one recorded snapshot per diagnostic
    family, with ANSI and plain variants.
17. `multi_file_columns` — a child-file diagnostic asserts file, line **and**
    column, extending `tests/cli.rs:394-419` beyond `E1001`/`E2019`/`E3005`.
18. `spec_30_4_gap` — the six uncovered codes are either covered or explicitly
    exempted in the spec table (§1.7).
19. `nonascii_dir_silent_skip` — a directory whose name is not an ASCII
    identifier is either skipped *with a diagnostic* or documented as silently
    skipped (§1.4). Pins whichever decision the owner makes; it must not be
    introduced as golden behaviour without one.
20. `lexer_col_placeholder` — a `©`-style byte renders as `` `?` `` in the
    message *and* the substitution is asserted to be the lexer's, so a future
    module-name change cannot silently alter diagnostic text.

### 6.5 What must not be enforced yet

* No golden snapshot may be committed **before** D3 is answered for non-ASCII
  inputs, because options 2 and 3 would invalidate them.
* No `aura explain` implementation before the §28.3 / `:88` spec amendment,
  since the CLI contract would then document five commands while six exist.
* No claim that §30.4 is satisfied while the six-code gap stands.
* No change to `src/check/mod.rs` — this RFC touches presentation only.

## 7. Open items

| # | Item | Owner |
|---|---|---|
| 1 | **D3** — choose Unicode-scalar columns (status quo), grapheme clusters, or display cells for §1.3. Requires amending `LANGUAGE_SPEC.md:177` for options 2 and 3. | **Human** |
| 2 | **D2** — authorize the projection repair of §2 so implementation may begin. | **Human** |
| 3 | Decide whether `eval` must unify with the CLI form or stay divergent (§5). A visible output change either way. | **Human** |
| 4 | Decide the printed-path identity question of §1.5: bare source-root-relative (today) or root-prefixed. | **Human** |
| 5 | Amend §30.3 to admit locationless diagnostics, and §30.4 to state the real coverage (or to exempt `E4099`/`E4999`, which §30.3 forbids reaching). | Agent, post-acceptance |
| 6 | Get `E2020`/`E2021` genuinely exercised, or record them as exempt alongside `E4099`/`E4999`. | Agent, post-acceptance |
| 7 | Add the twenty regression tests of §6.4, each mutation-verified per §6.2. | Agent, post-D2 |
| 8 | Add a golden-snapshot harness — none exists today (no `insta`, `expect-test`, `trybuild`, or snapshot directory). | Agent, post-D2 |
| 9 | Amend `LANGUAGE_SPEC.md` §28.3 and `:88` for `explain`, and extend `tests/syntax_conformance.rs:574-604`. | Agent, post-acceptance |
| 10 | Reconcile `docs/engineering/AIS.md:98` (`notes`/`help` documented, never emitted) and `:52` (stale `"aura_version": "0.2.1"`). | Agent, post-D2 |
| 11 | Fix the stale `LANGUAGE_SPEC §47/§48` citation at `src/main.rs:26` — the spec ends at §36. | Agent, post-D2 |
| 12 | Decide whether the colour policy is normative, and resolve the `KEYSTONE_SCOPE_MANIFEST.md:56` vs. `:130` drift. | Agent, post-acceptance |
| 13 | Decide whether a non-ASCII-identifier directory may be skipped silently (§1.4), and whether `docs/FILESYSTEM_MODULES_DESIGN.md:774`'s matrix needs a companion row. | **Human** |
