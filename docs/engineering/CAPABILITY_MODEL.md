# Aura Capability Model (Pre-0.3 Foundation)

Status: architecture record. It documents the capability boundary as it exists
and the rules future work must preserve. It changes no behavior and makes no
capability-policy decision (those would be human-gated).

Authority for current behavior: `src/host.rs` (the `Host` trait and its
implementations), `docs/security/TRUST_BOUNDARIES.md`. This file is the
consolidated map.

---

## 1. The rule

**All outside-world authority in Aura flows through exactly one interface: the
`Host` trait installed on the interpreter.** The evaluator never calls a
filesystem, clock, environment, network, or process API directly. A build,
browser, embedder, or test selects a host; the language surface is identical;
only capability availability differs.

Two runtime surfaces exist:

1. **The `Host` trait** (`src/host.rs:78`): stdout, stdin, args, filesystem
   read/write, clock, sleep, advisory cancellation.
2. **The CPython bridge** (`src/bridge/mod.rs`): `py_eval`/`py_import`/
   `py_call`/`py_version`, feature-gated by `py` and intentionally outside
   `Host` (it is a cross-language runtime, not a host primitive). See
   `docs/engineering/EMBEDDED_PYTHON_ARCHITECTURE.md` for its own boundary.

---

## 2. Capability inventory

| Capability | Aura surface | `Host` method | Owning layer | Native default | WASM default |
|---|---|---|---|---|---|
| stdout | `print` | `write_stdout` | host | process stdout (unbounded) | bounded `OutputSink` (256 KiB preview policy bound; overflow counted, not fatal) |
| stdin | `read_line()` | `read_line` | host | process stdin | provided string (`MAX_STDIN_BYTES`) |
| args | `args()` | `args` | host | process argv | provided list (`MAX_ARGS`, `MAX_ARG_BYTES`) |
| filesystem read | `read_file` | `read_file` | host | allowed | unavailable (`E5002`) |
| filesystem write | `write_file` | `write_file` | host | allowed | unavailable (`E5002`) |
| clock | `time_now`, `time_unix` | `now_unix`, `now_local` | host | allowed | unavailable (`E5002`) |
| sleep | `sleep_ms` | `sleep_ms` | host | allowed, clamped `MAX_SLEEP_MS` | unavailable (`E5002`) |
| cancellation | (runtime loop) | `should_cancel` | host | false (external termination) | false |
| Python | `py_*` | — (bridge) | bridge, feature `py` | available when built | unavailable (not part of the wasm runtime) |
| environment | none | — | — | **not exposed** | **not exposed** |
| network | none | — | — | **not exposed** | **not exposed** |
| process spawn | none | — | — | **not exposed** | **not exposed** |
| dynamic libraries | none | — | — | **not exposed** | **not exposed** |

There is no hidden global authority: no `std::env` read of program-visible
state, no direct socket, no `Command`, no `libloading`. (`std::env::temp_dir`
appears only in an error-message path for the filesystem host, not as a
language surface — `src/host.rs:313`.)

---

## 3. Capability resolution

- The interpreter holds `Box<dyn Host>`; `Interp::with_host` installs one,
  `default_host()` selects the substrate default.
- Every builtin that needs the outside world calls a `Host` method and
  converts a `HostError` into either `E4020` (I/O) or `E5002` (unavailable)
  (`src/host.rs:41`).
- `LimitedHost` (WASM substrate, native embedders) provides only
  stdout/stdin/args from plain data; every other method is `E5002`.
- `BrowserHost` adds a bounded-memory `OutputSink` (0.3.2 development,
  `OutputSink::preview`/`complete`). A full preview is **not** fatal: bytes
  past the retention bound are counted as omitted and the program continues.
  The older `with_stdout_limit` (fatal `E4020` at a fixed bound) is retained
  for the frozen-release contract; a host configured with a sink does not
  apply it.

---

## 4. Enforcement point (single):

`Host::write_stdout` is the only place bytes leave a program. The stdout bound
is enforced there, before the byte is accepted; no partial write is possible.
Likewise stdin/args/clock/sleep availability is decided by the installed host
implementation, not by the evaluator. A future capability must follow the same
pattern: **one method, one implementation choice, one diagnostic**.

---

## 5. Implications

- **Browser/WASM**: the zero-import contract means the module cannot even
  request authority; the host supplies data at the boundary. Restrictions are
  host policy, never language semantics.
- **Embedded systems**: select a `Host` with only the required methods.
- **Critical systems**: capability absence is the default; see workstream F.
- **Testing/determinism**: `silent_host()` and `LimitedHost` make execution
  deterministic; clock and randomness are not implicit.
- **Future server environments**: add capabilities by implementing `Host`
  methods plus their builtins — never by reaching around the trait.

---

## 6. What requires a human decision

- Adding a new capability (network, environment, process, dynamic libraries)
  is a security-policy change.
- Changing which capabilities a default distribution exposes is a
  compatibility/policy change.
- Introducing randomness as a language surface requires a determinism policy
  (seed source, WASM behavior, reproducibility).
