# RFC 0007 — Project structure, `aura init`, and the `os` capability

- **Backlog item:** `NBL-015` (`| NBL-015 | G02 | RFC: Aura.toml, aura init, std::os | P0 |`)
- **Gate:** `G02` — every P0 RFC is authored and escalated before any production
  surface changes.
- **Status:** Proposed. **Not authorized for implementation.**
- **Baseline:** `rewrite/v3-rust` @ `4406c873a46c47aaff85a0315ddce87a769af1ac`.
- **Grammar delta:** none for `Aura.toml` and `aura init`. The `os` surface as
  specified below requires **no grammar change**; §3 records why.
- **Decisions escalated:** `D1`, `D2`, `D3` (§2).

This RFC is bound by the `G06` acceptance line, quoted verbatim from
`docs/engineering/NEBULA_CAMPAIGN.md`:

> **G06** | Project structure, `init`, configuration, `std::os` | deterministic
> reversible loading, module compatibility, Host can deny env, no secrets in
> output, cross-OS tests where relevant

Every clause of that line is treated below as a *requirement to be satisfied or
explicitly refused*, never as a description of work already done.

## 0. Provenance

This RFC is a reconstruction. The §0.4 trigger permits it: none of the source
documents named by the directive (`AQL_Nebula_RFC_Draft.md`,
`NEBULA_SQLGUARD_CONTRACT.md`, `Aura_0.3.2_Nebula_Inventario_Geral.md`) exists
in the tree. What survives of the original intent is **four sentences**, in
three files, and nothing else:

1. `docs/engineering/NEBULA_CAMPAIGN.md:684` —
   `| NBL-015 | G02 | RFC: Aura.toml, aura init, std::os | P0 |`
2. `docs/engineering/NEBULA_CAMPAIGN.md:583` — lists `aura init` among the
   deferred DX surface.
3. `AGENT_STATE.md:1968` and
   `docs/engineering/CURRENT_HANDOFF.md:886`, `:897` — the same three tokens,
   restated.

No design document, no schema, no prior art. This RFC therefore does **not**
claim to recover an intended design. It reports what the repository actually
does today, states the smallest coherent design that satisfies `G06`, and
escalates the three choices that cannot be made from evidence.

Corrections made during authoring, recorded so a reader can see the shape of
the uncertainty:

1. The backlog item reads `std::os`. **No such namespace exists** (Finding B).
   The token is a name that cannot resolve; the RFC addresses the *capability*
   it implies, not the spelling.
2. The first three probes of this RFC were run with a relative `target/debug/aura`
   after a `cd`, and all reported "not found" rather than the program's verdict.
   Every probe below was re-run with an absolute binary path. The invalid runs
   are not cited.
3. The `NBL-015` backlog row was first read at `:663`; the correct line is
   `:684`. Only the correct citation appears above.
4. A claim that `aura init` appears in the FS design was checked and is false;
   it appears only at the four sites listed. The FS design governs *source
   root and discovery*, which `init` must obey but does not itself define.
5. The first draft cited "RFC 0006 §1.5" for the `Std::Math` evidence. §1.5 is
   Finding D (`aura://schema` versus `tools/list`); the resolver-skip evidence
   is **§1.13**. Both cross-references were corrected and re-verified against
   the section headings.
6. `E2019` is not a literal in `src/error.rs`. It is
   `pub const UNKNOWN_MODULE: u16 = 2019` (`:374`), rendered by
   `format!("E{:04}: {}", ...)` (`:103`, `:109`). The code is real; its spelling
   is derived. Cited as `E2019` throughout because that is what the compiler
   prints.

## 1. Motivation

### 1.1 What `G06` asks for, and which parts already hold

`G06` names five acceptance properties. Four of them constrain work this RFC
proposes; two of the five already hold today and must not regress. The state of
each, established by reading the code and running the compiler:

| `G06` clause | State today | Evidence |
|---|---|---|
| deterministic reversible loading | **Holds** | §1.2 |
| module compatibility | **Holds**; no config surface exists to break it | §1.3 |
| Host can deny env | **Vacuously holds** — no environment access exists | §1.5 |
| no secrets in output | **Holds** — nothing prints environment or config | §1.6 |
| cross-OS tests where relevant | **Absent** — no project/config surface exists to test | §1.7 |

This is the central finding of the RFC and the reason it is short: **`G06` is
almost entirely greenfield.** Four of its five clauses are satisfied by the
absence of the feature. The work is therefore not repair but the cautious
introduction of a surface that could *break* three clauses that hold today.

### 1.2 Finding — discovery is eagerly structural and extension-filtered

`std::os` implies a project root. Aura's root is not chosen by configuration: it
is the **entry file's parent directory** (`docs/FILESYSTEM_MODULES_DESIGN.md:211`,
`## 5. Source root and CLI contract`). Discovery is **eager and structural**
(`:399`, `### DECIDED — eager structural discovery`): every reachable child
source is part of the compilation unit even if unused.

Module identifiers are restricted by `is_aura_module_name`
(`src/module_graph.rs:948-957`): ASCII alphabetic or `_` first, then ASCII
alphanumeric or `_`. This is the existing answer to "what may a file be called".

**Probed:** a directory containing `main.aura` **and** `Aura.toml` checks
cleanly — `RC=0`. A configuration file at the source root is therefore harmless
to discovery, because discovery admits only module-shaped file names. This is a
verified property, not an assumption, and it is the property that makes `D1`
(everything in `Aura.toml`) safe to defer.

### 1.3 Finding — `Aura.toml` is not read by anything

At the baseline commit, `Aura.toml` appears in exactly **four** tracked
Markdown files — `AGENT_STATE.md` (1), `docs/engineering/NEBULA_CAMPAIGN.md`
(1), `docs/engineering/CURRENT_HANDOFF.md` (2) — and every one of them is
planning prose naming the future work, not a specification of it. (This RFC
deliberately names the files rather than printing a repo-wide count: its own
text mentions `Aura.toml` twenty-three times, so a bare count would be
self-falsifying on the next run.)

`grep -rn 'Aura\.toml' --include=*.rs --include=*.toml --include=*.json`
returns **0**. There is no `config.rs`, no TOML parser, no dependency on a
TOML crate. A file named `Aura.toml` today has exactly one effect on the
compiler: none.

This matters for `G06`'s "module compatibility" clause. Because no config
surface exists, no config surface can currently break module compatibility. Any
configuration this RFC introduces becomes a *new* compatibility obligation.

### 1.4 Finding — `aura init` does not exist

`aura init` → `RC=2`, `unknown command `init`; try: run, check, eval, repl, version, ais, mcp`.
The command table is `COMMANDS: [(&str, &str); 7]`
(`src/main.rs:28-36`), with a `5`-entry reduced build under
`#[cfg(not(feature = "json"))]` (`:39-45`). `init` joins both or the two builds
disagree on their own help text.

### 1.5 Finding — the string `std::os` names a namespace that cannot exist

This is the RFC's sharpest finding, and it invalidates the backlog item's
literal wording.

Aura's standard library is **not a set of modules**. It is a **flat prelude of
builtin functions**: `src/stdlib/mod.rs:20` `pub fn builtin_names()` derives its
list from `signatures::builtins()`, described in that file's own doc comment as
"the single source of truth shared by the runtime installer and the static
checker". The registry holds **91** entries.

Probed, in three separate directories:

| Program | Verdict |
|---|---|
| `use std::os` | `RC=1`, ``error[E2019]: unknown module `std` in `use` `` |
| `use Std::os` | `RC=1`, ``error[E2019]: unknown module `Std` in `use` `` |
| `use Std::Math` | `RC=1`, ``error[E2019]: unknown module `Std` in `use` `` |

The third row is the decisive one. `Std::Math` is the spelling used throughout
this campaign's earlier probes and **it does not resolve either**. There is no
`Std` module. The literal `"Std"` appears in `src/` zero times.

So `std::os` is not "a module we have not written". It is a token referring to
a namespace architecture Aura does not have. Calling functions directly — the
prelude style — is the only mechanism that exists.

### 1.6 Finding — no environment access of any kind

`grep -n 'name: "env"\|name: "getenv"\|name: "var"' src/stdlib/signatures.rs`
→ **NONE**. There is no builtin that reads the process environment.

Every `std::env` occurrence in `src/` is Rust-internal and outside the language
surface: `src/native_source.rs:43` (`current_dir`), `src/main.rs:151`
(`args`), `src/main.rs:259` and `src/diagnostic.rs:50` (the colour-override
lookup), `src/mcp.rs:59`, `:66` (`PATH`, to locate JEV). None is callable from
an Aura program.

`docs/security/TRUST_BOUNDARIES.md:30` lists "network access, environment
variables, subprocess spawning" as things the **`py` feature** exposes. The
pure-Rust feature set exposes none of them. `G06`'s "Host can deny env" clause
is therefore **vacuously satisfied today** — there is nothing to deny.

This is the single most important sentence in the RFC: **every design that adds
`os` access is a net loss of a security property the project currently holds
unconditionally.** That is why `D2` is a decision and not a task.

### 1.7 Finding — `args()` already covers the common `os` need

`args()` is a prelude builtin (`src/stdlib/signatures.rs:533-539`), `0`
arguments, returning `list<string>`. A program that wants its command-line
arguments needs no `os`.

Combined with §1.6, the residual demand for an `os` surface is narrower than
the backlog item implies: environment variables and process metadata, not
argument passing.

### 1.8 Finding — there is no cross-OS test surface

`G06` requires "cross-OS tests where relevant". The repository has no
project-structure or configuration tests because it has no project structure or
configuration. CI runs on the matrix defined in `.github/workflows/ci.yml`; no
test asserts a cross-OS property of a config file, because no config file
exists.

"Where relevant" is doing real work in that clause, and `D3` is where it is
resolved.

### 1.9 What is working and must not regress

1. **Discovery is extension- and shape-filtered** — `Aura.toml` at the root is
   inert (§1.2, probed).
2. **The prelude is flat and single-sourced** — checker and runtime agree
   because both read `signatures::builtins()` (§1.5).
3. **No environment access exists** (§1.6).
4. **The `py` feature is the documented boundary**, and CI verifies `pyo3` is
   absent from the pure-Rust build (`TRUST_BOUNDARIES.md:30-38`).
5. **`args()` already passes argv** (§1.7).
6. **The command table is duplicated between two `cfg` builds** and both must
   stay consistent (`src/main.rs:28-45`).

## 2. Decision

Three decisions are escalated. None is resolved here.

### 2.1 D1 — What, if anything, `Aura.toml` may contain

**The question.** May `Aura.toml` carry compiler-affecting configuration, or is
it a marker file only?

Three lawful outcomes:

- **(a) Marker only.** `Aura.toml` names the project root; the compiler reads
  nothing from it. Zero new compatibility obligations, zero config-driven
  behaviour, and the `G06` "module compatibility" clause stays trivially true.
  `aura init` writes it and nothing else changes.
- **(b) Metadata only.** Name, version, authors — read by tooling, never by the
  compiler. Requires a parser and a schema; cannot change a program's meaning.
- **(c) Compiler-affecting.** Source root, feature flags, lint levels. This is
  the only outcome that can break "module compatibility", and the only one that
  makes `Aura.toml` load-bearing.

**What the evidence says.** `Aura.toml` is proven inert today (§1.2, §1.3), so
**any** of the three is a strict addition. Outcome (c) is the only one that
creates the risk `G06` asks to be guarded, and the `G06` clause "deterministic
reversible loading" is most naturally read as a *constraint on (c)* — a config
that cannot be un-loaded deterministically is a config that fails the gate.

**Recommendation, not a decision.** (a) or (b). (c) should require its own RFC.

### 2.2 D2 — Whether an `os` surface exists at all, and in what form

**The question.** `G06` says "Host can deny env". Denial presupposes a request.
Does Aura gain an environment-reading surface, and if so which, spelled how?

Four lawful outcomes:

- **(a) Nothing.** `G06` is satisfied vacuously, exactly as today (§1.6).
  `R-17` does not apply. This is the zero-risk option and it is genuinely
  available.
- **(b) Host-only.** The surface exists in the Host ABI, not in the language.
  An Aura program cannot read the environment; a Host embedding can supply
  values. Preserves the language's current property entirely.
- **(c) Prelude functions.** `env(name) -> string?` and friends join the flat
  prelude, spelled without a namespace, consistent with §1.5. Requires a
  denial mechanism or the security property is lost outright.
- **(d) A module namespace.** Rejected on evidence, not taste: there is no
  namespace machinery (§1.5), so this outcome requires inventing module-scoped
  stdlib — a much larger change than the backlog item implies, and one that
  would collide with the filesystem-module design.

**The load-bearing observation.** Options (c) and (d) **remove** a property the
project holds unconditionally today. `G06`'s "no secrets in output" clause is
the mitigation: if `env` exists, `print(env("TOKEN"))` must not become a leak
vector in diagnostics, AIS documents, or MCP responses. That constraint is not
hypothetical — RFC 0006 §1.13 documented that `aura ais` runs a reduced
pipeline that skips name resolution entirely. **Any `env` that can reach a
diagnostic message reaches four output surfaces.**

**Recommendation, not a decision.** (a) or (b). (c) requires a denial design
that does not exist yet; (d) requires a namespace design that does not exist
yet.

### 2.3 D3 — What `aura init` writes, and how it refuses

**The question.** `init` is the first command in the compiler that would
**write to disk**. `grep -rn 'create_dir' src/` → **no matches**: the codebase
has never created a directory. This is new failure surface.

Sub-questions that must be answered together:

1. **What files?** `Aura.toml` alone, or `Aura.toml` + a `main.aura` skeleton?
   The FS design's eager discovery (`:399`) means a skeleton that does not
   check cleanly makes the very first `aura check` fail — a bad first
   impression caused by our own template.
2. **What if the target exists?** Refuse, overwrite, or merge. The repository
   has a documented precedent for preservation over convenience
   (`AGENTS.md`: "Preserve valid unrelated dirty-worktree changes"). Refusing
   is consistent with it.
3. **What if the directory is unwritable?** The error must be a *diagnostic*,
   not a panic, and must name the path. `G06`'s "no secrets in output" applies
   to paths.
4. **Cross-OS.** `aura init` must behave identically on the CI matrix; path
   separators and permission errors are precisely where an untested write path
   fails first.

**Recommendation, not a decision.** Refuse-if-exists, write `Aura.toml` only,
and print the path written. A skeleton program is a second decision hiding
inside this one.

## 3. Grammar delta

**None.**

`Aura.toml` and `aura init` are not language surfaces. `Aura.toml` is admitted
by the existing extension and shape filter (§1.2) and read by nothing.

For the `os` surface, if `D2` resolves to (c), the functions join the existing
flat prelude and are called by their bare name, which is already valid call
syntax. If `D2` resolves to (d), a namespace would be required and that is a
grammar question belonging to a different RFC. **As written, this RFC proposes
no token, no keyword, and no syntax.**

## 4. Semantics

### 4.1 What a project is

On this RFC's reading, a "project" is not a new concept. It is exactly what
§1.2 already says: **the entry file's parent directory, eagerly and structurally
discovered, extension- and shape-filtered.** `aura init` would make that
directory's existence explicit; it would not create a second notion of root.

If `D1` resolves outside (a), a second notion of root would exist, and the
question "which root wins" becomes a specification choice this RFC does not
make.

### 4.2 The four contracts at risk

1. **Discovery determinism.** A config file must not change which files are
   compiled, unless `D1` (c) is chosen deliberately.
2. **No environment leakage.** Any `os` surface must not let a value reach a
   diagnostic, an AIS document, or an MCP response (§2.2).
3. **Precedent for writing.** `aura init` establishes how the compiler treats
   the filesystem. Refuse-if-exists is the conservative precedent.
4. **Vacuity is a property.** Three `G06` clauses hold *because* the feature is
   absent. Adding the feature converts a guarantee into a policy that must be
   enforced and tested forever.

### 4.3 What is working and must not regress

The six items in §1.9. In particular, `Aura.toml` must remain inert unless `D1`
says otherwise, and the prelude must remain flat and single-sourced.

### 4.4 How the three decisions interact

`D2` (c) without `D1` is incoherent: a denial policy needs a place to be
declared, which is config. `D3` without `D1` is under-specified: `init` cannot
write a file whose contents are undecided. **`D1` is therefore the root
decision; `D2` and `D3` are blocked on it** — and all three are blocked on `D2`
(a) if the `os` surface is dropped entirely, in which case `D1` shrinks to a
marker file and `D3` to writing one file.

### 4.5 One observation that is not a finding

`G06` uses the spelling `std::os`, and RFC 0006 §1.13 showed `Std::Math` does not
resolve either. The backlog token is therefore not evidence that a `Std`
namespace was ever planned — it is the same aspiration, written twice with
different capitalisation, against an architecture that has never existed. This
is recorded so no future reader treats the token as a design commitment.

## 5. Migration

**Nothing to migrate.** No program can depend on `Aura.toml`, `aura init`, or an
`os` surface, because none exists (§1.3, §1.4, §1.5). Every decision here is a
strict addition.

The one constraint that is *not* additive: if `D2` resolves to (c), programs
gain the ability to read the environment, and any Host that relied on Aura being
unable to do so loses that guarantee. That is a behavioural change for embedders
even though no Aura source changes — and it is the reason `D2` (a) and (b) are
recommended.

## 6. Enforcement

### 6.1 The evidence table

| # | Claim | Probe | Result |
|---|---|---|---|
| 1 | `aura init` absent | `aura init` | `RC=2`, unknown command |
| 2 | `Aura.toml` inert at source root | `check` with `main.aura` + `Aura.toml` | `RC=0` |
| 3 | `std` unresolvable | `use std::os` | `RC=1`, `E2019` |
| 4 | `Std` unresolvable | `use Std::Math` | `RC=1`, `E2019` |
| 5 | No namespace machinery | `grep '"Std"' src/` | 0 hits |
| 6 | Prelude is flat | `builtin_names()` doc + registry | 91 entries |
| 7 | No env builtin | `grep 'name: "env"'` | NONE |
| 8 | `args()` exists | `signatures.rs:533-539` | `list<string>`, arity 0 |
| 9 | No dir creation in compiler | `grep -rn 'create_dir' src/` | 0 hits |
| 10 | Source root is entry parent | `FILESYSTEM_MODULES_DESIGN.md:211` | `## 5.` |
| 11 | Eager discovery | `FILESYSTEM_MODULES_DESIGN.md:399` | `### DECIDED` |
| 12 | Module name shape | `is_aura_module_name` | `module_graph.rs:948-957` |
| 13 | Command table sizes | `src/main.rs:28-45` | `7` and `5` |
| 14 | Env is a `py`-feature concern | `TRUST_BOUNDARIES.md:30` | documented |

### 6.2 Tests that would have caught each finding

1. `aura init` on a fresh directory creates exactly `Aura.toml`, and a second
   invocation refuses without modifying it.
2. `aura init` in an unwritable directory reports a diagnostic naming the path
   and exits non-zero without panicking.
3. A project containing `Aura.toml` checks identically to one without it, byte
   for byte (this is the "deterministic reversible loading" clause, made
   executable).
4. If `D2` (c): a denial-configured Host makes `env(name)` return `none`, and
   the value never appears in any diagnostic, AIS document, or MCP response.
5. Both `COMMANDS` builds list `init` (or neither does).
6. Cross-OS: the `init` contract test runs on the CI matrix, not only on Linux.

### 6.3 Fixtures used

Three single-file directories, each holding one `.aura` file, per the campaign's
established probe rule (one `.aura` file per directory, always):

- `main.aura` containing `use std::os`
- `main.aura` containing `use Std::os`
- `main.aura` containing `use Std::Math`

Plus one directory holding `main.aura` **and** `Aura.toml` for the inertness
probe. All probes used the absolute path to `target/debug/aura`; the earlier
relative-path run that reported "not found" is discarded (§0 correction 2).

## 7. Open items

1. **`D1` — what `Aura.toml` may contain** (§2.1). Root decision; blocks `D2` and
   `D3`.
2. **`D2` — whether an `os` surface exists, and in what form** (§2.2). The only
   decision that can *reduce* an existing security property.
3. **`D3` — what `aura init` writes, and how it refuses** (§2.3). Blocked on
   `D1`.
4. **`G06`'s "no secrets in output" has no owner yet.** If `D2` is (a) or (b),
   the clause is vacuous and needs no test. If `D2` is (c), it needs a
   four-surface test (diagnostic, AIS, MCP, Playwright) that does not exist.
5. **`G06`'s cross-OS clause is unevaluable until `D3` resolves.** Whether
   cross-OS tests are "relevant" is exactly the question `D3` answers.
6. **The backlog spelling `std::os` is misleading and should be corrected** to
   match whatever `D2` resolves to, in the same batch, so the campaign record
   does not carry a token that cannot resolve.
