# Aura Filesystem Modules — FSM-P1 Design Contract

Status: **FROZEN FSM-P1 DESIGN CONTRACT**

This document preserves the design contract approved in FSM-P1. Its original
phase-status language was written before production implementation began;
production work has since progressed through the provider-neutral graph,
native filesystem provider, and local virtual/WASM VFS foundation. The design
decisions below remain the architecture contract unless repository evidence and
an explicit later decision supersede them.

References below to “Phase 2” or other bare phase numbers are historical
planning vocabulary from the time this document was written. They must not be
confused with the current `FSM-P<N>` filesystem-module namespace.

The existing resolver remains the semantic authority. Filesystem loading must
produce the same logical module structure that equivalent in-source `module`
items produce; it must not implement a second name resolver, visibility model,
import system, alias system, re-export system, type namespace, or runtime.

## 1. Status vocabulary

- **DECIDED** — Phase 2 implementation must follow this rule unless new
  repository evidence proves it inconsistent.
- **OPEN** — evidence is insufficient; implementation must not silently choose.
- **DEFERRED** — intentionally outside the first filesystem-module
  implementation.

## 2. Existing evidence

### DECIDED — current semantic pipeline

The verified pipeline is:

```text
source text
  -> lexer
  -> parser -> nested ast::Module
  -> resolve::resolve -> flat canonical ast::Module
  -> checker
  -> runtime
```

`src/resolve.rs` already owns canonical Aura paths, `use`, `pub use`, module
aliases, visibility, variants, canonical-name collisions, and flattening.
Checker and runtime consume the resolved flat program and remain
module-semantics agnostic.

Filesystem support therefore enters before the resolver:

```text
SourceProvider
  -> physical/source discovery
  -> UTF-8 decode + parse per source
  -> source-aware logical Aura module tree
  -> existing resolve::resolve semantics
  -> checker
  -> runtime
```

The source-aware metadata described below is provenance, not a second module
semantics.

### DECIDED — current initialization semantics

The current runtime registers declarations first, then evaluates top-level
constants and expressions in flattened source order, then calls `main`.
The checker enforces the same direct constant forward-reference rule.

Verified in-source behavior at Phase 1:

| Case | Result |
|---|---|
| mutually recursive functions `A -> B -> A` with a terminating argument | succeeds |
| `A -> B -> C -> A` function-reference cycle | succeeds |
| reciprocal module aliases used by mutually recursive functions | succeeds |
| re-export cycle with concrete declarations on both sides | succeeds |
| constant reads a later module constant | `E2003` |
| same constant dependency with provider order reversed | succeeds |

Therefore a module-reference cycle is not, by itself, an error. Initialization
order is separately observable.

## 3. Source provenance

### Current audit

Current `Span` is a source-local UTF-8 byte range:

```text
Span { start: usize, end: usize }
```

It is `Copy + Clone + PartialEq + Eq + Default`; it is not `Hash`/`Ord`.
Tokens and many AST nodes embed it. Resolver/checker tables retain it. Runtime
native calls receive it. `Diag` contains a `Span` but no file identity.

Line/column formatting currently receives the source text separately.
The CLI keeps the file label outside the compiler, and the library entry
points currently discard that label. The WASM/Playground diagnostic JSON
contains code/message/line/column but no source identity and formats offsets
against the single source buffer supplied to the call. Python bridge failures
likewise carry Aura spans or `Span::default()` but no independent source id.

Exact-offset tests rely on spans remaining byte offsets local to one UTF-8
source. Changing `Span` from two `usize`s to `{ SourceId, usize, usize }` would
normally grow it from 16 to 24 bytes on 64-bit targets and expand many token
and AST layouts.

### Provenance options

| Option | Correctness | Cost / risk | Decision |
|---|---|---|---|
| **P1 — `SourceId` inside every `Span`** | Strong invariant; equality becomes cross-source safe | likely 16 -> 24 byte `Span`; expands tokens/AST broadly; synthetic spans need a sentinel | rejected |
| **P2 — source only on `Diag` / outer owner** | Cheap initial migration | resolver/checker declaration spans and runtime spans remain ambiguous; provenance becomes a side channel | rejected |
| **P3 — source-local `Span` + `SourceMap` at semantic boundaries** | Preserves byte-range model and avoids AST-wide expansion | needs explicit source ownership whenever a span crosses source boundaries | accepted basis |
| **P4 — P3 plus a first-class `Location { source, span }`** | Makes every cross-source location explicit while keeping `Span` source-local | requires source-owner metadata on parsed/flattened items and cross-source tables | **selected** |

### DECIDED — selected provenance model (P4)

The conceptual model is:

```text
SourceId       opaque compile-session id
SourceName     stable user-facing display name
SourceKey      opaque provider identity
SourceText     owned UTF-8 text
Span           source-local byte range; layout remains start/end
Location       { source: SourceId, span: Span }
SourceMap      SourceId -> { SourceName, SourceKey, SourceText }
```

Names above are conceptual; implementation naming may follow repository
conventions, but these responsibilities must remain distinct.

Rules:

1. `Span` remains a range within exactly one source.
2. A bare `Span` must not cross a source boundary without its owning
   `SourceId`.
3. Lexer/parser operate with one active `SourceId`; tokens may keep local
   `Span`s.
4. Lexer/parser diagnostics convert the local span to `Location` immediately.
5. Each parsed physical source retains its `SourceId` while its AST is inserted
   into the logical module tree.
6. In-source child modules inherit the source id of the source that declared
   them.
7. Filesystem child modules switch to the source id of their owning source.
8. Resolver/checker tables that retain a location across source boundaries use
   `Location`, never a naked `Span`.
9. Flattening must retain source ownership per emitted item. A suitable
   conceptual representation is `Sourced<Item> { source, item }`; the exact
   Rust shape is a Phase 2 implementation detail.
10. Checker and runtime establish the current source from the sourced item or
    closure before turning local spans into diagnostics.
11. `Diag` must support a primary `Location` and a genuinely locationless
    diagnostic. `Span::default()` must no longer be interpreted as “the first
    byte of whichever source the renderer happens to have.”
12. Formatting uses `SourceMap`; APIs that accept an arbitrary `(src, span)`
    pair are single-source compatibility helpers only.
13. `SourceMap` owns source text for as long as any compile/runtime diagnostic
    may refer to that source. A multi-source compilation/execution object must
    therefore retain the map together with the resolved program.
14. Raw `SourceId` values are internal and are not language or WASM ABI
    identity.

This keeps exact byte-offset tests meaningful and avoids a pervasive AST size
increase while making cross-source provenance explicit.

### REPL, WASM, Playground, Python consequences

- **REPL:** persisted declarations eventually need a source id per submission.
  A submission source may use a monotonically allocated id and its source text
  must remain available while retained declarations can diagnose against it.
- **WASM:** the current one-buffer ABI remains valid for single-file execution.
  A future VFS ABI passes source records, not host filesystem paths. Multi-file
  diagnostics expose `SourceName`, never raw `SourceId`.
- **Playground:** runtime/VFS support may land before any multi-file editor UI.
  The existing editor may continue to provide a one-source virtual filesystem.
- **Python bridge:** errors tied to an Aura call site combine the current
  source owner with the existing local span. Conversion failures with no Aura
  location remain explicitly locationless.

## 4. Logical and physical identity

### DECIDED

The authoritative module identity is an Aura logical path:

```text
[]
["foo"]
["foo", "bar"]
```

Segments use Aura identifier spelling and are case-sensitive.

Physical/provider identity is provenance and duplicate-detection data only. A
native path must never become the semantic module name. `SourceId` is likewise
not semantic identity; it is an ephemeral index into `SourceMap`.

One logical module has exactly one authoritative owner.

```text
ONE LOGICAL MODULE
ONE AUTHORITATIVE OWNER
```

No partial-module or augmentation semantics are introduced.

## 5. Source root and CLI contract

### DECIDED — file entry

For both:

```text
aura run path/to/main.aura
aura check path/to/main.aura
```

the selected entry file owns the logical root module `[]`, regardless of its
basename. The source root is exactly the entry file's parent directory after
native path normalization. Aura does not search ancestors for a package
manifest, `mod.aura`, repository root, or current working directory marker.

Examples:

```text
aura run app/main.aura
root source: app/main.aura
source root: app/
logical root owner: main.aura
```

```text
aura check app/nested/tool.aura
root source: app/nested/tool.aura
source root: app/nested/
logical root owner: tool.aura
```

Negative example: `app/foo.aura` is not automatically logical `foo` when it
is the selected entry; for that invocation it owns `[]`.

### DECIDED — root `mod.aura`

If the selected entry is `root/mod.aura`, that file owns `[]` and `root/` is
the source root.

If another root entry is selected and the same source root also contains
`mod.aura`, both files would claim logical `[]`; that is a duplicate-root
ownership error. Aura does not merge them.

### DECIDED — stdin

```text
aura run -
aura check -
```

create one source named `<stdin>` that owns logical `[]` and has **no native
filesystem module root**.

- In-source `module` declarations work normally.
- Filesystem children are unavailable.
- The process current directory is never an implicit module root.
- A reference/import that would require an external filesystem module is
  reported as `E2019` at the reference/import span, with wording that states
  filesystem modules are unavailable for `<stdin>`.

Future explicit CLI root support such as `--root` is **DEFERRED**; it must be
explicit rather than silently using the process current directory.

## 6. `mod.aura` and module ownership

### DECIDED — meaning of `mod.aura`

A non-root `dir/mod.aura` owns the logical module represented by `dir`.

Example:

```text
root/main.aura       -> []
root/foo/mod.aura    -> foo
root/foo/bar.aura    -> foo::bar
```

Any reachable module directory may contain `mod.aura`.

`mod.aura` does not merge with another owner and does not create partial
modules. Child discovery is defined by the eager structural rules below.

### DECIDED — file vs directory module forms

For a logical child `foo`, exactly one of these forms may own it:

```text
root/foo.aura
root/foo/mod.aura
```

If both exist, compilation fails deterministically before either is treated as
the winner.

A file-form module may still have a child directory used only as a container:

```text
root/foo.aura          -> foo
root/foo/bar.aura      -> foo::bar
```

A directory-form module behaves identically:

```text
root/foo/mod.aura      -> foo
root/foo/bar.aura      -> foo::bar
```

For `foo::bar`, `foo/bar.aura` and `foo/bar/mod.aura` are likewise mutually
exclusive owners.

If no owner exists for `foo`, `root/foo/bar.aura` is not independently
promoted to `foo::bar`; the directory is outside the reachable module tree and
`foo` remains unknown.

### DECIDED — filesystem module visibility

Every filesystem-created module wrapper is conceptually `pub module`.

This is necessary because a physical child has no parent-source declaration
site on which the programmer could write `pub`, and private implicit nested
modules would make deep filesystem paths unreachable from their ancestors.

The contents of the physical module keep normal Aura visibility: functions,
types, constants, fields, methods, traits, and in-source child modules remain
private unless their source says `pub`.

Equivalent conceptual expansion:

```text
foo/bar.aura
```

is a public module container equivalent to:

```text
pub module foo {
    pub module bar {
        // contents of bar.aura; declaration visibility unchanged
    }
}
```

## 7. In-source + filesystem interaction

### DECIDED

An in-source child and a filesystem child with the same logical path are an
ownership collision.

Given `foo.aura`:

```aura
module bar {
    // ...
}
```

and also:

```text
foo/bar.aura
```

both claim `foo::bar`; compilation fails. Aura does not append, merge, reopen,
or augment modules.

The same rule applies at every depth and regardless of `pub`.

Alias collisions remain under the existing resolver. A filesystem ownership
collision is resolved before alias/name resolution because it concerns which
source owns the logical module, not which name an import binds.

## 8. Discovery strategy and deterministic assembly

### Alternatives considered

**Reference-driven discovery** was rejected for the first implementation.
Aura permits qualified module references without requiring `use`; reproducing
the resolver's lexical lookup only to decide what file to load would create a
second resolver.

**Hybrid discovery** (index everything but parse on demand) still needs enough
semantic path interpretation to decide which indexed source a bare or nested
path denotes. It also creates different behavior for unused malformed module
sources than the equivalent in-source module tree.

### DECIDED — eager structural discovery

The first implementation uses eager discovery of the **reachable physical
module tree**.

This matches the in-source model: an in-source child is parsed even when never
referenced, so a physical child that conceptually becomes an in-source module
is also part of the compilation unit.

Discovery is bounded by ownership:

1. The selected entry owns `[]`.
2. In each owned module directory, enumerate direct eligible child owners.
3. Recurse into a child directory only when that child module itself has an
   owner (`name.aura` or `name/mod.aura`).
4. Unowned unrelated directories are not recursively scanned.

This prevents directories such as `venv/`, `target/`, or arbitrary sibling
trees from becoming compilation input unless an Aura module owner makes them
reachable.

An eligible file child has a `.aura` stem that is valid as an Aura module
identifier. The exact `mod.aura` spelling is the directory-module form.
Other files are ignored by module discovery unless explicitly selected as the
root entry.

### DECIDED — deterministic source/discovery order

Directory enumeration order is never semantic.

At each parent, child logical names are sorted by the bytewise ASCII spelling
of the Aura identifier. Because Aura identifiers are ASCII, this order is
portable and independent of locale and host filesystem enumeration.

Root source load/parse is attempted first. After a parent source parses,
physical ownership/case collisions and in-source/filesystem child collisions
for that parent are adjudicated before child source parsing. Children are then
processed depth-first in sorted logical-name order.

This order also deterministically allocates `SourceId`s, although raw ids are
not user-visible semantics.

### DECIDED — deep filesystem module nesting

Physical splitting must not bypass the existing host-safety bound for nested
in-source `module` blocks. The graph builder should discover iteratively, but
the assembled logical module depth consumes the current
`parse::parse_recursion_budget()` exactly as equivalent nested in-source
module containers do. The first child whose logical depth exceeds that
substrate-calibrated budget is `E1015`; its primary location is the start of
that child source (`Location { child_source, Span::default() }`).

This is a host-safety backstop, not a new language nesting limit. It preserves
the existing native/WASM calibration (currently 2048 native / 768 wasm) and
prevents a deep multi-file tree from reaching resolver/checker recursion with
a depth the parser would have rejected in one source.

### DECIDED — conceptual insertion and top-level order

Filesystem child module wrappers are conceptually appended **after all items
written in the owning parent source**, in sorted child-name order.

Example physical tree:

```text
main.aura
a.aura
b.aura
```

is semantically assembled as if the end of `main.aura` contained:

```aura
pub module a { /* a.aura */ }
pub module b { /* b.aura */ }
```

Each child recursively appends its own filesystem children after its written
items.

This is the total-order rule for constants and top-level expressions across
files. It is intentionally mechanical and does not depend on import order,
directory iteration, hashing, timestamps, or OS behavior.

Consequences:

- a parent/root constant cannot read a filesystem-child constant that is
  conceptually later; the existing forward-constant rule yields `E2003`;
- functions may freely refer to later module declarations because declarations
  are registered before initialization;
- sibling constant ordering is the sorted logical-module order;
- top-level side effects preserve the same conceptual order;
- no dependency-based initialization algorithm is introduced.

## 9. Cycle semantics

### DECIDED — declaration/name cycles are allowed

Aura already supports mutually recursive declarations across in-source
modules. Filesystem representation must not reject the same logical program
merely because its module containers come from separate sources.

Therefore:

- `A -> B -> A` declaration/function reference cycles are allowed;
- `A -> B -> C -> A` declaration/function reference cycles are allowed;
- diamonds are allowed and a physical source is loaded once;
- reciprocal module aliases are allowed when existing resolver rules resolve
  the final references;
- re-export behavior remains exactly the existing resolver behavior.

### DECIDED — initialization remains source ordered

Aura does **not** introduce SCC-based module initialization in the first
filesystem implementation.

The selected rule is closest to C2 but deliberately preserves the stronger
existing invariant:

```text
declaration/reference cycles: allowed when resolver semantics allow them
initialization: one deterministic flattened source order
forward constant read: E2003
```

A constant cycle necessarily contains a forward read in that total order and
is rejected by the existing constant rule. An indirect read through a function
may be detected only at runtime today; that behavior remains authoritative
until a separate, evidence-backed constant-initialization analysis is designed.

There is **no filesystem dependency-cycle error** in the selected eager design.
Physical discovery follows an ownership tree, not a reference graph; semantic
reference cycles are resolver/checker/runtime concerns. Consequently no new
cycle-path diagnostic is emitted in Phase 2 solely because modules reference
one another.

**DEFERRED:** if a future implementation adds a dedicated initialization-cycle
diagnostic, its displayed cycle must be canonicalized independently of hash or
DFS order (rotate to the lexicographically smallest logical path, then choose
the lexicographically smaller direction where both are possible).

## 10. Physical path model

### DECIDED — Aura syntax never contains host paths

Aura module syntax remains `Name(::Name)*`. It never accepts `/`, `\\`, `.`,
`..`, drive letters, UNC prefixes, or absolute filesystem paths as module
syntax.

OS path syntax exists only at the CLI/provider boundary for the selected entry
and native source root.

### DECIDED — entry path normalization

The native provider may accept relative, absolute, Windows drive, and UNC entry
paths as supported by the host OS. The chosen entry path is normalized for
native access, then its parent becomes the source root. This physical form does
not affect logical module names.

Redundant physical separators and `.` components are provider concerns. `..`
in the CLI entry path is resolved before root selection; it does not become an
Aura module segment.

Provider child keys must remain under the established source root. A provider
that returns a child outside the root produces a path-escape failure.

### DECIDED — symlinks

The first implementation does **not** follow symlinks for module ownership.

- a symlink selected as the entry source is rejected;
- a reachable candidate module file that is a symlink is rejected;
- a reachable candidate module directory that is a symlink is rejected;
- a symlink loop therefore fails at the first reachable symlink rather than
  being traversed;
- a symlink that would escape the source root is rejected before traversal;
- a broken symlink is rejected as a symlink/source-path failure.

This rule is deliberately stricter than `std::fs::canonicalize()` and prevents
host-specific aliasing, escape, and loop behavior from becoming language
semantics.

Because symlinks are rejected, canonical filesystem paths are provenance and
diagnostic data only; they are not logical identity.

**DEFERRED:** hard-link identity. Two distinct symlink-free directory entries
are treated as distinct physical sources even if a host filesystem reports the
same inode/file-id. Supporting hard-link de-duplication would require a
portable provider identity contract and is unnecessary for deterministic Aura
module semantics.

## 11. Case sensitivity and portability

### DECIDED

Logical module identifiers are byte-exact ASCII and case-sensitive, matching
Aura identifiers.

The native provider must enumerate directory entries and compare the actual
spelling; it must not probe an expected filename and let a case-insensitive OS
choose a match.

Examples:

```text
Foo.aura exists
source references Foo::x  -> may resolve
source references foo::x  -> E2019
```

This result is identical on Linux, default macOS, and Windows.

If a reachable parent contains two logical candidates whose names differ only
by ASCII case, such as:

```text
foo.aura
Foo.aura
```

the compilation is rejected as a portability/case collision even on a
case-sensitive filesystem. A source tree that cannot be represented
portably is not allowed to gain different semantics on different hosts.

The same collision rule applies to the virtual provider.

## 12. Duplicate-source and ownership rules

### DECIDED

| Situation | Result |
|---|---|
| `foo.aura` + `foo/mod.aura` | physical ownership collision |
| `foo/bar.aura` + `foo/bar/mod.aura` | physical ownership collision |
| in-source `module bar {}` + physical `foo/bar.aura` | logical ownership collision |
| same provider `SourceKey` offered twice for one logical module | duplicate logical source |
| same provider `SourceKey` offered for two logical modules | duplicate logical source |
| `foo.aura` + `Foo.aura` under one reachable parent | case/portability collision |
| repeated traversal request for the same logical module | load once; reuse the same `SourceId` |
| import alias collides with a declaration | existing resolver `E2007` |
| two semantic declarations collide inside one module | existing checker/resolver code |

No “first file wins” rule exists.

## 13. Provider-neutral loading and WASM/VFS

### DECIDED — one graph builder

Native filesystem and browser virtual sources feed the same source/module
assembly algorithm.

Conceptually a provider returns records equivalent to:

```text
SourceKey    opaque provider identity
SourceName   display name
SourceText   owned UTF-8 text after strict decode
```

The graph builder assigns `SourceId`, constructs `SourceMap`, validates
ownership, parses each source, and assembles the logical module tree.

Native provider responsibilities:

- establish the selected root;
- enumerate eligible direct children deterministically;
- enforce the no-symlink/root-boundary rules;
- read bytes and strictly decode UTF-8.

Virtual provider responsibilities:

- expose the same logical source hierarchy from an in-memory source set;
- reject `.`/`..`, absolute, drive, UNC, and separator tricks in virtual keys;
- apply the same exact-case and duplicate-owner rules;
- provide UTF-8 source text without pretending browser keys are OS paths.

### DECIDED — Playground scope

The runtime may gain VFS input support while the current Playground UI remains
single-file. A one-file editor is simply a virtual provider containing one root
source and no physical children.

Multi-file tabs/tree UI are **DEFERRED** and must not block compiler/runtime
provenance work.

## 14. Error taxonomy

Existing numeric codes are part of Aura's contract, so new numbers are not
allocated casually in this design phase. Rows marked **NEW** require a
dedicated symbolic code in Phase 2; the exact numeric assignment is deferred
until the implementation updates the error-code contract and tests together.

| Failure | Code policy | Owning phase | Primary location | Required secondary information |
|---|---|---|---|---|
| unknown/missing logical module | reuse `E2019` | resolver / source-availability bridge | reference or `use` `Location` | normalized logical path; stdin explanation when relevant |
| `foo.aura` + `foo/mod.aura` | **NEW: module-source ownership collision** | graph builder | locationless | both `SourceName`s + logical module path |
| in-source child + filesystem child | **NEW: module-source ownership collision** | graph builder | in-source module declaration `Location` | filesystem `SourceName` + logical path |
| same provider source claims multiple logical modules | **NEW: duplicate logical source** (may share ownership-collision code if wording is unambiguous) | graph builder | locationless | both logical paths + one `SourceName` |
| case-only collision | **NEW: module-source case collision** | graph builder | locationless | both spellings/paths |
| invalid provider/module path | **NEW: module-source path** | provider/graph builder | locationless unless caused by a source construct | normalized provider/display path |
| root escape | **NEW: module-source path** | provider | locationless | rejected child key + source root name |
| reachable symlink / symlink loop / broken symlink | **NEW: module-source path** | native provider | locationless | offending display path; stable reason category |
| source read failure after a valid owner is known | reuse `E4020` | provider | locationless | `SourceName` + normalized stable I/O category |
| invalid UTF-8 | reuse `E1001` | decode | exact `Location` covering invalid bytes | `SourceName` resolved through `SourceMap` |
| semantic declaration collision | existing `E2007`/`E2012`/`E2013` etc. | resolver/checker | existing semantic span converted to `Location` | unchanged |
| private cross-module access | reuse `E2018` | resolver | reference `Location` | canonical target as today |
| declaration/reference cycle | no loader error | existing resolver/checker | as existing semantics require | no cycle path because the cycle is allowed |
| constant initialization cycle/forward read | reuse `E2003` | checker/runtime | offending read `Location` | canonical constant name |
| stdin attempts filesystem module access | reuse `E2019` | source-availability bridge/resolver | `<stdin>` reference `Location` | wording: no filesystem root for `<stdin>` |

### DECIDED — deterministic wording and ordering

User-visible module-source diagnostics must not depend on raw OS error text,
directory enumeration order, hash-map order, locale, or path separator.

Stable ordering rules:

1. root source read/decode/parse first;
2. at each parsed parent, ownership/case/in-source collisions before child
   parsing;
3. children by bytewise ASCII logical name, depth-first;
4. within one source, existing parser/resolver/checker source order;
5. physical collision path lists sorted by normalized `SourceName`;
6. semantic diagnostics rendered through the diagnostic's `Location` and the
   same `SourceMap` used for compilation.

Native display names are source-root-relative and use `/` as the diagnostic
separator, for example `main.aura`, `foo.aura`, `foo/bar.aura`. The provider
may retain an absolute native path internally for I/O, but that path is not the
portable diagnostic identity.

## 15. Adversarial design matrix

This matrix is the permanent Phase 1 contract. Phase 2 tests should materialize
these rows using both native and virtual providers where applicable.

| Case | Expected contract |
|---|---|
| one-file program | existing single-source semantics unchanged |
| equivalent in-source module program | same canonical resolver/checker/runtime semantics |
| equivalent multi-file program | implicit public module wrappers, same resolver semantics |
| nested modules | recursively owned file/mod forms; logical `::` paths |
| deep nesting | existing parser/AST limits; filesystem depth must not bypass them |
| missing child | `E2019` at referring source location |
| malformed child | parser diagnostic naming child `SourceName` |
| checker failure in child | existing checker code at child `Location` |
| runtime failure in child | existing runtime code at child/call-site `Location` according to existing span semantics |
| direct function-reference cycle | allowed if terminating/otherwise semantically valid |
| indirect function-reference cycle | allowed |
| pure unresolved re-export cycle | existing resolver failure (`E2019`) |
| re-export cycle with concrete resolvable declarations | existing resolver semantics; no filesystem-cycle ban |
| diamond | allowed; shared physical module loaded exactly once |
| duplicate file/directory ownership | new ownership-collision diagnostic |
| in-source/filesystem ownership collision | new ownership-collision diagnostic; no merge |
| private cross-file access | `E2018` |
| public cross-file access | succeeds under existing visibility rules |
| `pub use` | existing re-export rules |
| alias | existing lexical module-alias rules |
| transitive re-export | existing deterministic resolver rules |
| canonical type/variant collision | existing deterministic `E3002` behavior |
| case-only physical collision | rejected portably |
| wrong-case reference to one existing module | `E2019` on every host |
| symlink duplicate | rejected at symlink before duplicate traversal |
| symlink loop | rejected at first reachable symlink |
| root escape | rejected by provider path rule |
| stdin with only in-source modules | allowed |
| stdin requiring filesystem child | `E2019`; no implicit cwd |
| native provider | same logical graph contract |
| virtual provider | same logical graph contract; no OS paths |
| diagnostic ordering with multiple broken children | first sorted logical child after parent adjudication |
| repeated deterministic run | same code/message/source/line/column for semantic failures and same normalized path ordering for loader failures |
| root constant reads filesystem-child constant | `E2003` because implicit child wrapper is appended after parent items |
| function in root reads filesystem-child constant at runtime | allowed after initialization if otherwise valid |
| child constant reads an earlier parent constant | allowed when canonical visibility permits it |
| sibling constant reads a later sorted sibling constant | `E2003` |
| unused malformed reachable child source | parser failure; eager tree means it is part of the compilation unit |
| malformed file in unrelated unowned directory | ignored by module discovery |

## 16. Implementation boundaries for Phase 2

### DECIDED

Phase 2 may implement provenance and loading only after preserving these
boundaries:

- no second resolver;
- no checker/runtime filesystem awareness beyond source provenance and the
  source-aware compiled-item wrapper;
- no module merging;
- no manifest/package manager requirement;
- no implicit cwd root for stdin;
- no host-path semantics in WASM/VFS;
- no symlink traversal;
- deterministic source and diagnostic ordering;
- existing single-source APIs retained through compatibility paths where
  practical.

### OPEN

No semantic blocker remains for the Phase 2 filesystem loader itself.

The exact Rust public API shape for a source-aware `Compilation` /
`CompiledProgram` is an implementation-level choice and must be reviewed for
backward compatibility, but it must satisfy the provenance/lifetime contract
above.

### DEFERRED

- case-collision and filesystem-specific module-source error subclasses beyond
  the provider-neutral Phase 3 allocations (`E2020` ownership collision,
  `E2021` duplicate logical source, `E2022` invalid/missing provider key/path);
- hard-link identity/de-duplication;
- explicit CLI `--root` for stdin/eval;
- REPL filesystem-relative loading;
- multi-file Playground editor UI;
- package manifests and package management;
- user-configurable module initialization order;
- dedicated static initialization-cycle graph analysis;
- symlink support.

## 17. AUDIT-3 boundary

> **SUPERSEDED (status note).** When this design was frozen, AUDIT-3 was
> DECISION-PENDING. It has since been **RESOLVED** by ADR-0004
> (`docs/adr/0004-typeexpr-nesting-policy.md`): structural `TypeExpr` nesting
> counts toward `MAX_AST_DEPTH = 256` on every substrate, and the property-test
> exclusion cited below was lifted. The original text is preserved as the
> frozen contract's historical boundary statement; it is not current status.

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

Filesystem module design does not alter that decision package or its property
test exclusion.
