# Embedded CPython Architecture (Pre-0.3 Foundation)

Status: architecture record and boundary definition. It selects **no**
distribution or licensing strategy: that remains a human decision (see §9).
It changes no behavior.

Current consumers: `docs/CPYTHON_COMPATIBILITY_TARGET.md` (conversion table
and tiers), `docs/security/TRUST_BOUNDARIES.md` (authority statement),
`docs/adr/0003-supported-cpython-versions.md` (tier policy). This file is the
architectural map those documents assume.

---

## 1. Strategic direction

Python interoperability is a permanent Aura capability, and the standard
supported distribution should eventually not require the user to install a
separate system Python. That is a **packaging** goal, not an evaluator goal:
the evaluator must never require Python to exist. The architecture therefore
separates four layers:

```text
Aura language
      |
Python interoperability API      (aura::bridge — Aura value <-> Python value)
      |
Python runtime provider          (the narrow interface between the bridge and
      |                           whatever supplies CPython)
      |
+-------------------------+
| Embedded CPython        |  a Python shipped/owned by an Aura distribution
| System CPython          |  the user's installed Python (today's model)
| Future alternative      |  another runtime implementing the API contract
+-------------------------+
```

The provider boundary is the key long-term shape: today's bridge talks to PyO3
directly (`src/bridge/mod.rs`, feature `py`, `pyo3 0.29` with
`auto-initialize`). A provider seam would let a distribution swap the runtime
without touching language semantics.

---

## 2. Current integration inventory (verified)

| Aspect | Today |
|---|---|
| Linking | dynamic to the system CPython via PyO3; no bundled interpreter |
| Feature | `py` (default-on; `Cargo.toml:31`); no `py` = inert stubs returning `E5002` |
| API/ABI dependency | PyO3 `0.29` against CPython 3.10–3.13 (ADR-0003 tiers) |
| Initialization | `auto-initialize` (implicit); no explicit init/finalize policy (TD-11) |
| GIL | acquired per call via `Python::attach` (`src/bridge/mod.rs:88` ff.) |
| Subinterpreters | not used |
| Object lifetime | Python objects are converted to `Value` copies; container conversion is node-budgeted (`MAX_PY_NODES`, `MAX_PY_SOURCE_NODES`) |
| Conversion layer | explicit structural mapping: int/float/str/bool/None/list/dict both ways; no arbitrary object graph |
| Exception translation | CPython errors become `E5001`; unavailable/unsupported shapes `E5002` |
| Import path | the ambient CPython search path (system) |
| Extension modules | ambient (whatever the system Python has) |
| WASM | the wasm runtime does not link CPython at all; `py_*` is absent/inert there |
| Binary size | native only; no-impact on the wasm artifact |
| Licensing/NOTICE | system Python is not redistributed today, so no distribution obligation exists yet |

---

## 3. Lazy initialization (required property)

Aura itself must run ordinary programs without initializing CPython. Today the
`py` feature implies the dependency, but nothing calls Python until a `py_*`
function executes. The long-term architecture must make this explicit:

- **Initialization is capability-driven**: the first `py_*` call initializes
  the provider (or reports `E5002` if the provider is absent).
- **No boot cost**: starting `aura` must not import, initialize, or GC Python.
- **Explicit policy**: replace `auto-initialize` with an explicit provider
  init (TD-11) when the provider seam is introduced; neither approach changes
  the language-visible contract.
- **WASM**: `py` is not part of the wasm runtime; capability policy for WASM is
  "absent", not "disabled by silent stubbing".

Current human policy does not require eager initialization, so no
contradiction exists; this section records the target.

---

## 4. Security boundary (explicit)

**Embedded CPython is powerful native authority. It is not a sandbox.**
Executing Python can read and write the filesystem the process can reach, read
the environment, open sockets, load native extension modules, spawn processes
through extensions, and consume arbitrary memory. Aura's own capability
restrictions (`Host`) do **not** constrain Python code.

Consequences:

- A profile that forbids Python authority must forbid the `py` capability
  (runtime/provider presence), not merely wrap it.
- `E5002` ("capability unavailable") is the *intended* unavailability signal;
  a distribution without a provider reports it for every `py_*` call.
- Documentation must never claim Aura sandboxing survives `py_eval`.
- The bridge conversion layer is a correctness/memory bound, not a security
  boundary; it must not be relied on to isolate hostile Python.

This matches `docs/security/TRUST_BOUNDARIES.md` (arbitrary CPython authority,
no sandbox claimed).

---

## 5. Provider seam (design target)

A provider interface should cover, minimally:

```text
initialize() / is_available()
eval(source) -> Value
import(name) -> Value
call(callable, args) -> Value
convert_to_python(Value) / convert_from_python(PyObject)
error translation -> E5001/E5002
```

with these invariants:

- The evaluator core never references PyO3 types; only `aura::bridge` does.
- Provider absence yields `E5002`, never a panic or a silent no-op.
- Conversion budgets (`MAX_PY_NODES`, `MAX_PY_SOURCE_NODES`) live at the
  bridge and apply to every provider equally.
- No provider becomes hard-coded into the evaluator; the CPython provider is
  one implementation of the contract.

---

## 6. Packaging implications (recorded, not decided)

- **Embedded CPython** would add a bundled interpreter to the distribution:
  binary-size impact, import-path management, extension-module policy,
  platform-specific dynamic libraries, and licensing/NOTICE obligations.
- **Static vs dynamic linking** changes the ABI/API responsibility and the
  update story for security fixes.
- **macOS/Linux/Windows** differ in code signing, notarization, and library
  loading; each needs a documented distribution story.
- **WASM** keeps no CPython; a future in-wasm runtime (e.g. a Python
  implementation compiled to wasm) would be a *provider* choice, not a change
  to the bridge contract.
- **Import path**: embedding decides whether system site-packages remain
  visible. That is a security-policy choice, not a technical detail.

None of these is selected here. The distribution/licensing strategy selection
is a stop condition (human gate).

---

## 7. Why this does not contaminate the evaluator core

- `src/bridge/mod.rs` is the only module that names PyO3; the evaluator
  (`src/run/**`), checker, parser, and host never do.
- Builtins are registered by name; the bridge installs or stubs them. The
  language surface is identical with and without the provider.
- A provider seam preserves that property while allowing bundled runtimes.

---

## 8. Relationship to 0.3

Pre-0.3 only needs the boundary to exist and be documented; no provider work
is required now. Two tracked items remain:

- TD-11: explicit init/finalize policy when the provider seam lands.
- A future packaging decision (distribution, linking, licensing).

---

## 9. Human-gated decisions

| # | Decision | Why gated |
|---|---|---|
| P1 | Distribution strategy (bundle vs require system Python) | packaging/licensing |
| P2 | Static vs dynamic linking | supported platforms, security updates |
| P3 | Import-path policy for a bundled runtime | security boundary |
| P4 | Extension-module policy | security/compat |
| P5 | Whether `py` stays default-on in every distribution | capability policy |
