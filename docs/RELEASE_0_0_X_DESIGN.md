# Aura 0.0.x Release-Hardening Design

**Status:** Frozen, normative, implementation-ready.
**Baseline:** HEAD `e203af0` (`feat: add else if`), branch `rewrite/v3-rust`.
**Authority:** `docs/LANGUAGE_SPEC.md` is the semantic authority; this document
defines the release-hardening additions and must be reflected there before
implementation.

This is a **release-hardening design**, not a new language feature. It adds
system-access capabilities (standard input, text-file I/O, command-line
arguments) to the existing language without changing core semantics.

---

## 1. Release Objective

Make Aura usable as a small scripting language for an initial `0.0.x` public
release: a developer can build Aura and run a script that reads a file or
standard input, transforms it, writes a result, and parameterizes behavior
from the command line.

## 2. Current Repository Baseline

- Branch `rewrite/v3-rust`; HEAD `e203af0`, pushed to
  `origin/rewrite/v3-rust`. `master` unchanged at `d940fbe`.
- Complete: 001 (static argument checking), 002 (named arguments), 003
  (field-type propagation), 004 (destructuring `let`), 005 (empty-map
  literal), H1 (pattern correctness/safety), 006 (`else if`).
- Package `aura-lang v3.0.0-alpha.1`; MSRV 1.83; `unsafe_code = "deny"`.
- Working tree clean except untracked `session-ses_f30f.md`.

## 3. Current Core Capabilities

Language core (values, bindings incl. destructuring, functions/closures,
`if`/`else if`/`else` and `match`, loops, patterns, structs/enums/aliases,
error handling, pipeline), a conservative checker sharing one signature
registry with the runtime, one shared `parse → check → execute` pipeline
across CLI/library/REPL, and the builtin/method library listed in
`LANGUAGE_SPEC.md` §24–§25.

## 4. Release Scope Review

**Pre-release blockers (confirmed against source):**

1. **Standard input reading** — no stdin builtin is registered.
2. **Text file reading** — no filesystem builtin exists.
3. **Text file writing** — no filesystem builtin exists.
4. **Command-line argument access** — `src/main.rs:35` collects `args` only to
   dispatch subcommands; extra arguments are silently ignored and no builtin
   exposes them.

**Documentation blockers (confirmed):**

5. `README.md` states "`else if` does not exist", contradicting Feature 006.
6. `docs/FEATURE_ROADMAP.md` still lists F001/F002/F003/F005/F006 work as open.

**Distribution note (not a language blocker):** installation is source-build
only; the release workflow does `cargo publish … --dry-run`. Pleasant public
installation is `0.0.x` polish (§18).

**Reassessment of the previous "important but non-blocking" list:** environment
variables, `--help`, changelog, and language-reference population remain
non-blocking. No additional item is a blocker: file handles, streaming,
directories, process control, block comments, range step, defaults,
branch-join, struct patterns, methods, modules, and async are all
post-release.

---

## 5. Standard Input Design

**API:** one builtin, `read_line() -> string | none`.

**Normative semantics:**

- Reads one line from the interpreter's configured input source.
- The returned string **excludes** the line terminator: a trailing `\n` is
  removed, and a trailing `\r` immediately before that `\n` is also removed
  (so CRLF input yields the same string as LF input). A lone trailing `\r`
  without `\n` is preserved as data.
- **EOF:** returns `none`.
- **Empty line:** returns `""` (a real empty line).
- **EOF vs empty line:** distinguishable — `none` vs `""`.
- **Unicode:** read as UTF-8; invalid UTF-8 on the input source is an I/O
  error (`E4020`).
- **Blocking:** blocking; no timeout or async machinery.
- **Repeated calls:** supported; each call consumes one line.
- **Buffering:** the input source is buffered; the interpreter owns the
  buffer, not the process global.
- **Error:** an I/O failure other than clean EOF is fatal `E4020` (§9, §10).
- **Checker type:** `Returns::Dynamic` (the result may be `string` or `none`;
  the checker does not over-reject).

**Sufficiency for filter scripting:** yes — a `while` loop reading `read_line`
until `none` is the canonical filter. No `read_all()` is added; whole-input
reads are composed from `read_line` when needed.

**Input source:** owned by the interpreter context (§8). `aura run` and
`aura eval` configure the process standard input; the REPL and library default
to **no input source**, so `read_line()` returns `none`. `aura check` does not
execute.

**`aura run -`:** when the program source itself is read from stdin, that
stream is consumed to EOF before execution; `read_line()` in such a run
therefore returns `none`. This is documented behavior, not an error.

---

## 6. File I/O Design

**API:** two builtins.

- `read_file(path) -> string | none`
- `write_file(path, content) -> none`

**`read_file` normative semantics:**

- Returns the entire file at `path` as UTF-8 text.
- **Missing path:** returns `none` (absence is a value; `contract.md` §0.3).
- **Empty file:** returns `""` (distinct from `none`).
- **Path is a directory:** `E4020` I/O error.
- **Permission denied:** `E4020`.
- **Invalid UTF-8:** `E4020`.
- **Newlines:** preserved exactly; no normalization.
- **Checker type:** `Returns::Dynamic` (may be `string` or `none`).

**`write_file` normative semantics:**

- Writes `content` (its UTF-8 bytes) to `path`, **creating or truncating**
  the file. Overwrite is unconditional.
- Returns `none`.
- **Missing parent directory:** `E4020`.
- **Path is a directory:** `E4020`.
- **Permission denied:** `E4020`.
- **Checker type:** `Returns::Ty(Ty::Unknown)` (returns `none`).

**Resolved decisions:**

- Path encoding is the platform's native encoding as provided by the script
  string; paths are used relative to the process working directory; no `~`
  expansion, no normalization. Windows paths are handled by `std::fs` as-is.
- **Append is out of scope for `0.0.x`.** It is not required by any acceptance
  scenario; adding it solely for convenience is rejected. A future
  `append_file` can be added additively.
- **Binary I/O is explicitly out of scope.** Aura strings are UTF-8 text.
- **File handles and streaming are explicitly out of scope.** There is no
  `open`, no handle value, no streaming API.
- **Atomicity:** `write_file` is an ordinary truncate-and-write; no
  atomic-rename guarantee is made.
- **Directories and metadata APIs are out of scope.**

---

## 7. Command-Line Arguments Design

**API:** one builtin, `args() -> [string]`.

**Normative semantics:**

- Returns the program's arguments **excluding** the interpreter command, the
  subcommand (`run`), and the script path. For `aura run script.aura a b`,
  `args()` is `["a", "b"]`.
- **Ordering:** command-line order preserved.
- **Program/command/script path:** excluded.
- **Zero arguments:** `[]`.
- **Unicode:** arguments are the process arguments as UTF-8 strings.
- **Repeated calls:** each returns an independent snapshot list.
- **`aura run`:** exposes the arguments after the script path.
- **`aura eval`:** `[]` (no positional arguments).
- **`aura repl`:** `[]`.
- **Library:** `[]` by default; a context-aware entry (§8) may supply
  arguments.
- **Checker type:** `Returns::Ty(Ty::List(Box::new(Ty::String)))` — a list of
  strings. The runtime result is always a list of strings, so a concrete
  checker return type is exact and does not over-reject.
- The API does not expose interpreter-internal flags or the subcommand.

**Backward compatibility:** previously, extra arguments after the script path
were accepted and ignored. They remain accepted; they are now readable via
`args()`. Scripts that ignored them are unaffected.

---

## 8. Execution-Context Architecture

Execution flow is unchanged: OS → CLI (`src/main.rs`) → `aura::run_program` /
`run_toplevel_stdout` (`src/lib.rs`) → `execute` → `on_interp_thread` →
`run::Interp::run`.

The minimum context is **owned interpreter state**, not process globals:

- `run::Interp` gains two fields: `args: Vec<String>` and
  `input: Option<Box<dyn std::io::BufRead + Send>>`.
- `Interp::new()` sets `args: Vec::new()` and `input: None`, preserving
  current behavior for the REPL and library.
- `src/lib.rs` gains context-aware entry points that mirror the existing
  public functions so `main.rs` can supply the context without bypassing the
  pipeline:
  - `run_program_with(src, file, args, input)`;
  - `run_toplevel_with(src, file, args, input)`.
  Both funnel through a single context-aware `execute_with(module, stdout,
  args, input)`. The existing `execute`, `run_program`, and
  `run_toplevel_stdout` delegate to them with empty args and `None` input, so
  library and REPL behavior is unchanged. The closure moved onto the
  interpreter thread remains `Send + 'static`; `args` and the boxed
  `BufRead + Send` are owned and `Send`.
- `src/main.rs` calls `run_program_with` for `aura run` and
  `run_toplevel_with` for `aura eval`, passing the post-script arguments and
  the process standard input. `aura check` and `aura repl` keep using the
  context-free entries (`compile` and `repl::run`).

**Explicit distinctions:**

- **Execution state:** `Interp` fields (globals, functions, structs, tables,
  `stdout`), unchanged.
- **Process input:** `input`, owned by `Interp`, configured only by
  `run`/`eval`.
- **Process arguments:** `args`, owned by `Interp`.
- **Filesystem access:** an ordinary builtin concern. File I/O does **not**
  enter the execution context; `read_file`/`write_file` call `std::fs`
  directly, consistent with how existing natives call host operations.

No process-global mutable state, no hidden globals, no runtime singleton is
introduced.

---

## 9. Error Model and Catchability

### Error codes

The `E4xxx` runtime family is the correct taxonomy. Existing runtime codes do
not represent host I/O failures. One new code is introduced:

| Code | Meaning | Phase |
|---|---|---|
| `E4020` | I/O operation failed (read, write, or standard-input failure) | runtime |

**`E4021` is not introduced.** The prior design proposed a separate
file-not-found code, but Aura's design principle "absence is a value, not an
exception" (`contract.md` §0.3) makes a missing readable path a `none` result
from `read_file`, not an error. Therefore no filesystem-not-found code exists.
Genuine failures that remain errors:

- `read_file`: directory, permission denied, invalid UTF-8, other OS error →
  `E4020`.
- `write_file`: missing parent, directory, permission denied, other OS error →
  `E4020`.
- `read_line`: read failure other than clean EOF → `E4020`.
- Wrong argument types/counts for any new builtin → existing `E3001`/`E2003`
  via the shared signature registry (no new codes).

### Catchability (frozen)

`E4020` is an ordinary **fatal runtime diagnostic** and is **not catchable**.
This follows the existing frozen rule (`LANGUAGE_SPEC.md` §14.5): "Only
explicit `throw` is catchable. Runtime diagnostics … are not values and are
not catchable." Making I/O catchable would reopen the catchability model and
is out of scope.

Consequences (normative):

- `try { read_file(p) } catch e -> { … }` does **not** catch an `E4020`; the
  diagnostic propagates and terminates the program.
- `finally` still runs on that exit path, exactly as for other fatal runtime
  errors.
- The idiomatic way to handle a possibly-absent file is the `none` result:
  `let text = read_file(p)\nif text == none { … }`.

This is acceptable for a scripting release because the common scripting
"failure" (a file that does not exist) is represented as a value, while
genuine environment failures are fatal and clearly reported.

### Error-code documentation

Once implemented, `E4020` is added to `docs/LANGUAGE_SPEC.md` §30.2 and
`docs/errors.md`, and to the `error_samples()` reachability registry so the
documented-code contract holds.

---

## 10. Safety / Filesystem Boundary

- **Access model:** unrestricted local filesystem access with the invoking
  user's OS privileges, using paths as given. **Aura is not sandboxed.** This
  is stated explicitly; no capability system, path allow-list, or chroot is
  provided.
- **Path restrictions:** none.
- **Symlinks:** followed as `std::fs` does by default.
- **File size / allocation:** `read_file` allocates the whole file as a
  `String`, bounded by the OS file and process memory. `read_line` allocates
  one line. **No arbitrary size limit is imposed**; this is an explicit part
  of the initial runtime model.
- **stdin size:** unbounded; consumed line by line.
- **`unsafe`:** forbidden crate-wide; all I/O uses `std::fs`/`std::io`.
- The existing language resource limits (AST depth 256, parser backstop 2048,
  call frames 512, range cap 10,000,000) are unchanged and govern language
  constructs, not I/O sizes.

---

## 11. CLI / REPL / Eval / Library Semantics

| Surface | `read_line()` | `args()` | `read_file`/`write_file` |
|---|---|---|---|
| `aura run` | process stdin | args after script path | available |
| `aura run -` | `none` (source consumed stdin) | args after `-` | available |
| `aura check` | not executed | not executed | not executed |
| `aura eval` | process stdin | `[]` | available |
| `aura repl` | `none` (no input source) | `[]` | available |
| library `run_source`/`execute` | `none` | `[]` | available |
| library `execute_with` | caller-supplied | caller-supplied | available |

Differences are limited to input/args wiring; the shared parser/checker/
interpreter and the §28.5 consistency rule are preserved (same verdict,
result, and diagnostic code for a given source, apart from the documented
input/args configuration).

---

## 12. Real-Script Acceptance Criteria

After implementation, each must be expressible and correct:

1. **CLI transformer:** `aura run greet.aura Ada` uses `args()[0]`.
2. **stdin filter:** `cat in.txt | aura run upper.aura` reads `read_line` in a
   `while` loop until `none` and prints transformed lines.
3. **file transformer:** `read_file(input)`, transform, `write_file(output,
   result)`.
4. **validator:** `read_file(path)`; if `none`, report absent; else validate
   content and report errors.
5. **data-processing script:** combines functions, lists/maps, loops,
   `try/catch`, `read_line`, and `write_file`.

Each is satisfied by the four builtins in §5–§7. No additional API is needed.

---

## 13. Installation and Distribution

- **Usable after cloning/building:** yes — `cargo build --release` (README).
- **Platform artifacts:** the release workflow builds per-OS tarballs
  (Linux/macOS/Windows) on `v*` tags and creates a GitHub release.
- **crates.io:** publish is currently **dry-run only**.
- **Boundary for 0.0.x:** source build and GitHub release tarballs are
  sufficient to be usable. Enabling real `cargo publish` (or documenting
  tarball installation) is `0.0.x` polish, not a language blocker. The release
  design does not require crates.io publication.

---

## 14. Documentation Updates Required Later (not in this phase)

- **README.md:** replace the stale "`else if` does not exist" statement with
  the supported rule; add the four builtins to the standard-library list;
  document installation, CLI, stdin, file I/O, and args.
- **docs/FEATURE_ROADMAP.md:** mark F001–F006 and H1 complete; separate
  release-hardening from future work.
- **docs/LANGUAGE_SPEC.md:** add the four builtins to §25 with their normative
  semantics; add `E4020` to §30.2.
- **docs/contract.md:** add the builtins and `E4020`.
- **docs/errors.md:** add `E4020`.
- **docs/grammar.md:** no change (builtins add no syntax).

---

## 15. Existing-Language Consistency

The release APIs add no new syntax, AST, checker-semantics, or runtime
control-transfer behavior, and do not reopen:

- F001 (static argument checking), F002 (named arguments), F003 (field-type
  propagation), F004 (destructuring), F005 (empty map `{:}`), F006 (`else if`
  as nested parser-level `Expr::If`), H1 (pattern hardening).

I/O is expressed through the existing signature registry and native-builtin
mechanism; it introduces no new type-system behavior.

---

## 16. Implementation Scope (expected files)

**Source:**
- `src/stdlib/signatures.rs` — add `read_line`, `read_file`, `write_file`,
  `args` signatures (core, ungated).
- `src/stdlib/mod.rs` — register and implement the four natives.
- `src/run/mod.rs` — add `args`/`input` fields and accessors to `Interp`.
- `src/lib.rs` — add the context-aware `execute_with`; `execute` delegates
  with empty defaults.
- `src/main.rs` — pass post-script arguments and process stdin for
  `run`/`eval`.
- `src/error.rs` — add `IO` (`E4020`).

**Tests:** `tests/checker.rs` (arity/types), `tests/run.rs` (runtime
behavior), `tests/regressions.rs` (integration), `tests/repl.rs` (REPL
defaults), `tests/grammar.rs` (error reachability), `tests/boundaries.rs` if
needed. A new CLI-level test may use `std::process::Command` (none exist
today); if added, it must remain hermetic.

**Documentation (later phase):** as §14.

**Must remain untouched:** `src/lex/`, `src/parse/mod.rs`, `src/ast/mod.rs`,
`src/check/mod.rs` semantics, `src/bridge/`, `src/repl.rs` command loop,
`src/run/value.rs` (unless a helper is strictly required), resource
constants, and F001–F006/H1 implementation.

---

## 17. Test Design

**stdin:** empty input (immediate `none`); single empty line (`""`) then
`none`; multiple lines in order; Unicode; EOF at start; repeated reads; CRLF
line ending normalization.

**file:** `read_file` existing file; empty file (`""`); Unicode content;
missing file (`none`); directory (`E4020`); `write_file` new file; overwrite;
round-trip; missing parent (`E4020`); permission failure where portable.

**args:** zero (`[]`); one; many in order; Unicode; script-path exclusion;
`eval`/REPL/library defaults (`[]`).

**integration:** `run` with args; stdin transformation loop; file read/write;
I/O failure with `try/finally` (finally runs, error not caught); existing CLI
commands unchanged; pure-Rust build
(`--no-default-features --features cli,repl,json,regex,time`) supports all
four builtins.

**error reachability:** an `E4020`-producing program is included so
`every_documented_error_code_is_reachable` holds.

---

## 18. Compatibility

- **Additive API only.** New builtins extend the registry; existing builtins
  and their semantics are unchanged.
- Existing programs retain meaning; existing CLI calls remain valid.
- Extra CLI arguments that were previously ignored remain accepted and now
  readable — no existing script breaks.
- REPL and library defaults (`read_line` → `none`, `args()` → `[]`) preserve
  current observable behavior.
- Python bridge unaffected; pure-Rust builds include the new builtins.
- **Classification:** additive API + runtime plumbing + documentation
  correction; **no language syntax change**.

---

## 19. Scope Summary

**Before `0.0.x` (blockers):**
1. `read_line()`.
2. `read_file()` / `write_file()`.
3. `args()`.
4. `E4020` and its documentation/reachability.
5. Stdlib/context/CLI integration.
6. README `else if` correction; roadmap reconciliation.

**`0.0.x` polish:** environment access (`env(name) -> string | none`),
`--help`/`-h`, better unknown-command hint (currently omits `repl`), `aura
eval` statement support or explicit documentation, CHANGELOG/release notes,
populate or remove `docs/language-reference/`, real crates.io publication or
tarball guidance, optional diagnostic carets, possible multi-line
`else`/`catch`/`finally`.

**Post-`0.0.x`:** block comments, range step, default/variadic arguments,
branch-join inference, struct patterns, user-defined methods, modules,
process/system interaction, random, richer date/time, async/concurrency.

---

## 20. Implementation Sequence

1. **Spec update:** add the four builtins and `E4020` to `LANGUAGE_SPEC.md`,
   `contract.md`, `errors.md` (grammar unchanged).
2. **Execution context:** add `args`/`input` to `Interp`; add context-aware
   `execute_with`; update `main.rs` wiring.
3. **Builtins:** register/implement `read_line`, `read_file`, `write_file`,
   `args`.
4. **Focused tests:** per §17.
5. **Full release validation:** fmt, both test configs, clippy variants,
   property, contract/examples, Miri, release smoke.
6. **Documentation cleanup:** README, roadmap, changelog, language-reference.
7. **Release candidate:** tag and distribute per §13.

Dependencies: 1 → 2 → 3 → 4 → 5 → 6 → 7 (spec precedes implementation).

---

## 21. Resolved Design Decisions

1. **stdin:** single primitive `read_line() -> string | none`; strips `\n`
   and trailing CRLF `\r`; EOF → `none`; empty line → `""`; buffered,
   repeatable, blocking; read failure → `E4020`; no `read_all`.
2. **file read:** `read_file(path) -> string | none`; missing → `none`;
   empty → `""`; directory/permission/invalid-UTF-8/other → `E4020`.
3. **file write:** `write_file(path, content) -> none`; create or truncate;
   missing parent/directory/permission/other → `E4020`; no append, no binary,
   no handles, no streaming in `0.0.x`.
4. **args:** `args() -> [string]`, command/subcommand/script path excluded,
   source order, `[]` in eval/REPL/library.
5. **context:** owned `args`/`input` on `Interp`; `execute_with` entry;
   `execute` delegates with empty defaults; filesystem stays in builtins; no
   global state.
6. **errors:** one new code `E4020` (I/O error); `E4021` rejected; type/arity
   errors reuse `E3001`/`E2003`.
7. **catchability:** `E4020` is fatal and **not catchable** (§14.5);
   `finally` still runs; absence is handled via `none`.
8. **safety:** unrestricted filesystem, explicitly not sandboxed; no new size
   limits; unbounded text reads accepted as part of the initial model.
9. **REPL/eval/library:** REPL and library expose no input source and empty
   args; `run`/`eval` wire process stdin and args; `run -` yields `none` for
   `read_line`.
10. **distribution:** source build and GitHub tarballs suffice for `0.0.x`;
    crates.io publication is polish.
11. **scope:** only stdin/file-read/file-write/args are pre-release blockers.
