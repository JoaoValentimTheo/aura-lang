# Future Extension Boundaries: AI, Dart/Flutter, Quantum (Pre-0.3)

Status: boundary record. These are **not** implemented and are not commitments
to implement. Their purpose is to confirm that the Pre-0.3 architecture does
not obstruct each direction and to state where each belongs (always outside
the evaluator core).

---

## 1. Universal AI boundary (future 0.4 direction)

**Goal.** Aura may host a universal model-orchestration layer comparable in
role to LangChain, but Aura-native. Nothing of the kind is implemented now.

**Architecture rule.** No provider is hard-coded into the evaluator; AI
functionality belongs above a provider abstraction, exactly like the CPython
bridge sits behind a provider seam
(`docs/engineering/EMBEDDED_PYTHON_ARCHITECTURE.md` §5).

Conceptual shape (non-normative):

```text
Model / ChatModel / EmbeddingModel
Tool / Agent / Message / Prompt / Stream
Provider            (remote API, local model, embedded runtime)
ModelError / RateLimitError / AuthenticationError / ToolError
```

**What Pre-0.3 must not break:**

- Network authority does not exist in the evaluator and must not be added
  implicitly; a future AI provider is a host/FFI capability with an explicit
  capability policy (`CAPABILITY_MODEL.md` §6).
- Error types map onto the exception-family foundation as *catchable language
  exceptions* (`EXCEPTION_ARCHITECTURE.md` §3) — never as fatal diagnostics.
- Streaming/async semantics are a language-model question (no async today);
  the evaluator's explicit-continuation machine is compatible with resumable
  host callbacks (already used by `map`/`filter`/`reduce`), so a future
  stream protocol has a template.
- No network/provider dependency is added during Pre-0.3.

**Human gates:** provider abstraction design, async/stream language syntax,
capability policy for network access.

---

## 2. Dart/Flutter interoperability boundary (future)

**Goal.** Aura may later interoperate with a separately packaged or
Aura-distributed Dart/Flutter toolchain. Flutter is **not** embedded now and
nothing is downloaded from Google.

**Architecture rule.** Dart/Flutter interop is an FFI/toolchain boundary
outside the evaluator. The relevant surfaces are:

- **Aura calling into Dart**: an FFI host capability (dynamic-library or
  message-passing provider), following the `Host`/provider pattern; never a
  direct dependency of `src/run/**`.
- **Dart hosting Aura**: embedding the `aura` crate (library entry points
  `compile_*` / `execute_*`) or the WASM runtime behind a message protocol —
  both already exist.
- **Build/packaging**: a Dart/Flutter toolchain is a distribution concern
  (same class of decision as embedded CPython packaging), with its own
  licensing/size/platform gates.

**What Pre-0.3 must not break:** the library API surface and the process/FFI
absence in the core (no `dart:ffi`, no dynamic loading in the evaluator).
No implementation is required by any current defect.

**Human gates:** bindings design, distribution/toolchain strategy.

---

## 3. Quantum-ready architectural boundary (future)

**Goal.** Do not claim Aura is a quantum programming language, and do not
implement quantum simulation for branding. Ensure the architecture can later
host specialized computational backends through libraries/providers/FFI
without contaminating core semantics.

**Architecture rule.** A future quantum stack would be a provider plus an FFI
capability (simulator backend, hardware provider) with its own value types
(circuit IR, measurement, results) and async job model. That belongs to
future work; the Pre-0.3 requirement is only that:

- The evaluator core has no assumption that values are classical-only that
  would forbid an opaque provider-defined value type (today `Value` is a
  closed enum — a future backend value would need an explicit extension point
  decision, which is human-gated).
- Deterministic simulator testing is possible through the existing patterns:
  `silent_host`/`LimitedHost`, no randomness in the core, and a provider
  boundary that can be mocked.
- No quantum dependency or simulator is added during Pre-0.3.

**Human gates:** value-model extension point, provider API, async job model.

---

## 4. Common shape

All three directions share one architectural pattern already present in the
repository:

```text
Aura language
    |
provider / capability interface      (the only extension point)
    |
implementation (FFI, SDK, runtime)   (outside the evaluator)
```

The Pre-0.3 work (capability model, provider seams, exception families,
resource contracts) is exactly what makes these three directions additive
rather than invasive.
