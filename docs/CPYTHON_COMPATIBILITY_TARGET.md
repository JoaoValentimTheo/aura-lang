# Aura CPython Compatibility Target

**Status:** draft for human review (HD-3). Defines precisely what "CPython
compatibility" means for Aura, so no vague claim is ever made.

## The claim rule

Aura never claims to "be CPython" or to be "fully Python-compatible". Aura
provides **explicit interoperability**: Aura programs can call CPython, and
values cross a defined, bounded, documented boundary. The precise supported
surface is stated below and enforced by tests.

## Compatibility dimensions

Each dimension is independently scoped and independently claimed.

| Dim | Meaning | Aura v1 target |
|---|---|---|
| **A** Python syntax compatibility | Parsing Python source *as Aura* | **Not supported.** Aura and Python are different languages; Aura's grammar is its own. Aura does not intend to parse Python. |
| **B** Python semantic compatibility | Same behavior as CPython for Python code | **Delegated.** Python code is executed *by CPython* via the bridge, so its semantics are CPython's, not reimplemented. Aura does not re-implement Python semantics. |
| **C** CPython C-extension compatibility | Loading/using native extensions | **Supported transitively.** `py_import`/`py_call` can reach any importable module, including those backed by native extensions, with full host authority. |
| **D** Python module interoperability | Importing/using Python modules | **Supported** via `py_import`, `py_call`, `py_eval`. See the conversion table. |
| **E** Embedding CPython | Running Aura inside a Python process | **Not supported in v1.** Aura provides no stable C ABI for embedding. |
| **F** Calling Python from Aura | `py_eval`, `py_import`, `py_call` | **Supported** (feature `py`). This is the core v1 story. |
| **G** Calling Aura from Python | A Python module exposing Aura execution | **Not supported in v1** (no embedding ABI). |
| **H** Stdlib compatibility | Python standard library reachable | **Supported transitively** through CPython, to the extent CPython imports it in the host environment. Not a reimplementation. |
| **I** Object-model compatibility | Aura values ↔ Python objects | **Partial, explicit, bidirectional structural mapping** (below). Non-mappable objects are rejected or shown as `repr`, never silently coerced. |
| **J** Exception compatibility | Python exceptions at the boundary | **Structural only.** A Python exception becomes an Aura `E5001` diagnostic (fatal, not catchable as an Aura value in v1). Aura does not raise Python exceptions. |
| **K** Packaging/import compatibility | pip/venv/import path | Python import resolution uses the host CPython's normal `sys.path`/environment (including an active virtualenv if the process runs in one). Aura does not manage Python packages. |
| **L** ABI compatibility | CPython ABI across versions | **Depends on the linked CPython** (PyO3). Aura exposes **no** stable Python C-ABI of its own. Building against different CPython versions may produce different binaries; the supported build matrix is in the version policy below. |

## Value conversion (dimension I) — normative

**Aura → Python**

| Aura | Python |
|---|---|
| `none` | `None` |
| `bool` | `bool` |
| `int` (i64) | `int` |
| `float` (f64) | `float` |
| `string` | `str` |
| list `[T]` | `list` |
| map `{K: V}` | `dict` (`string`/`int`/`bool` keys only) |
| anything else (struct, enum, range, fn) | **rejected** `E5002` |

**Python → Aura**

| Python | Aura |
|---|---|
| `None` | `none` |
| `bool` (exact type) | `bool` (checked before `int`; `bool` is an `int` subclass) |
| `int` within i64 | `int` |
| `int` outside i64 | **rejected** `E4013` (never demoted to `float`) |
| `float` (exact type) | `float` |
| `str` (exact type) | `string` |
| `list` | list |
| `dict` with `str`/`int`/`bool` keys | map (other key kinds **rejected** `E5002`) |
| any other object | its `repr`, as a `string` |
| reference cycle | **rejected** `E5002` |

Conversion matches on **type identity**, not on whether a coercion would
succeed. A value converts only when it is an instance of the builtin scalar
(`bool`/`int`/`float`/`str`); a Python subclass of a builtin scalar *is* such an
instance and converts as its base type, which is sound (a subclass may narrow
behavior but never changes the base type's representation). A plain user object
implementing `__float__`, `__index__`, `__bool__`, or `__str__` is **not** an
instance of the builtin, so it is **not** silently coerced; it takes the `repr`
fallback like any other non-mappable object. This prevents silent precision
loss (e.g. an `__index__` returning a value beyond `i64`) and prevents a
raising dunder from being swallowed. The same identity rule governs **map
keys**: an `__index__`-implementing object is not a valid `int` key and is
rejected `E5002`, never coerced. `bytes`/`bytearray`/`tuple`/`set`/`complex`
and other opaque objects likewise cross as their `repr` string (they are **not**
rejected).

Both directions are bounded: a node budget (1,000,000) and a depth bound
(`MAX_VALUE_DEPTH`) make a pathological or cyclic graph a diagnostic, not a
host overflow.

## Supported surface (v1)

- `py_eval(code: string) -> value`
- `py_import(module: string) -> map` (public names; values denoted `<python>`)
- `py_call(module: string, attr: string, args...) -> value`
- `py_version() -> string`

Errors are `E5001` (Python error) or `E5002` (crossing not representable),
attributed to the Aura call site. Requires the `py` feature; without it the
names exist and return `E5002`.

## Python version policy (proposed — HD-3)

- Support the currently-maintained CPython minor lines that CI can install on
  Linux, macOS, and Windows.
- **Proposed matrix:** CPython **3.10 – 3.13**, tested in CI with a
  representative subset; the exact linked version is reported by `py_version()`.
- The pre-1.0 bridge is built with PyO3 `0.29` and `auto-initialize`; no stable
  Aura↔Python ABI is promised.

## Security (see `SECURITY.md`)

The `py` feature grants **arbitrary Python code execution with full host
authority**. There is no Python sandbox and none is claimed. Deployments that
must not allow this build without `py`.

## Not in scope for v1

- Parsing Python as Aura (A).
- Embedding CPython as a stable Aura C-ABI (E, G, L-stability).
- Reimplementing the Python stdlib (H is transitive only).
- Python raising Aura errors (J is one-way).
- Sandboxing Python (separate program; not claimed).

## Evidence plan

An interop matrix (`tests/interop_matrix.rs`, feature `py`) covering: `None`,
`bool`, `int` (in/out of range), `float`, `str` (ASCII/Unicode), opaque objects
(`bytes`, `tuple`, `set`, user types with `__float__`/`__bool__`/`__str__`) as
`repr`, `list`, `dict` (valid keys, rejected keys), exceptions, non-mappable
Aura values (struct/enum/fn/range) rejected, cycles, repeated calls, and
recovery after failure. Cross-platform and per-Python-version coverage runs in
CI as a separate job.
