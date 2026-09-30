# Aura Threat Model

Scope: the Aura compiler/runtime (native), the CLI/REPL, the WebAssembly
runtime, the browser Playground, the CPython interoperability layer, the module
system and its source providers, and the release/distribution pipeline.

This model describes assets, adversaries, trust boundaries, and the authority
each component holds. It is deliberately conservative.

## Assets

- **Correctness of Aura program semantics** (no silent misinterpretation).
- **Host integrity** (no memory unsafety, no uncontrolled panic/abort).
- **User data** reachable through the process (filesystem, environment).
- **Frozen release artifacts** and their hashes.
- **Release pipeline integrity** (tags, artifacts, checksums).
- **Playground users' browsers** (no ambient authority from wasm).

## Adversaries

| ID | Adversary | Capability |
|---|---|---|
| A1 | Malicious/curious Aura program author | full control of Aura source |
| A2 | Malicious sibling/module source | controls some `.aura` files in a tree |
| A3 | Malicious Python code reached via `py` | full Python |
| A4 | Malicious web content calling the Playground runtime | Aura source to wasm |
| A5 | Network/release attacker | attempts artifact substitution/tampering |
| A6 | Dependency/action attacker | compromised crate/action |
| A7 | Local attacker | controls filesystem paths/symlinks the CLI reads |

## Trust boundaries and authority

| Boundary | Authority granted | Notes |
|---|---|---|
| Aura source → compiler | none beyond CPU/memory | bounded by resource limits; must not panic |
| Aura program → native runtime | full process authority via documented host builtins (`read_file`, `write_file`, `args`, stdin/stdout, clock, sleep) | intentional; not a sandbox |
| Aura program → CPython (`py` feature) | **arbitrary Python with full host authority** | see `TRUST_BOUNDARIES.md`; no Python sandbox |
| Aura source → wasm runtime | none: **zero host imports** | Playground/browser; verified |
| Playground web → worker/wasm | Aura text + project JSON only | runtime is self-contained |
| Runtime manifest → browser loader | artifact selection by id + hash | hash verified; capability detection |
| Filesystem (`NativeFilesystemSourceProvider`) → module graph | reads owned child sources | symlink/path policy enforced |
| CLI argument → entry selection | selects entry; eager discovery reads owned siblings | defined behavior |
| Tag push → release workflow | builds 3 native targets + wasm; publishes | gated on validate; hashes recorded |
| Dependencies/actions → build | arbitrary code at build time | trusted-supply-chain assumption |

## Threats and status

| ID | Threat | Surfaces | Status / mitigation |
|---|---|---|---|
| TH-1 | Host panic/abort from crafted Aura input | lexer/parser/checker/runtime/modules | Fuzz + property tests; limits; no known defect |
| TH-2 | Unbounded resource use (CPU/memory/stack) | nesting, ranges, collections, conversion | Limits with `E1xxx`/`E4xxx`; boundary tests |
| TH-3 | Path traversal / uncontrolled file access | `read_file`/`write_file`, module discovery | File builtins are ambient authority by design; module discovery bounded to owned children |
| TH-4 | Python sandbox escape | `py` feature | **N/A — no sandbox exists**; documented as arbitrary authority |
| TH-5 | WASM host escape / ambient authority | wasm runtime | Zero imports; no OS authority; verified |
| TH-6 | Artifact substitution | runtime manifest, release assets | SHA-256 pinned; loader verifies; releases immutable |
| TH-7 | Cache poisoning | Playground cache | Keyed by runtime id + integrity hash |
| TH-8 | Unsafe deserialization | JSON builtins, project transport | JSON is data; project payloads validated; node budgets |
| TH-9 | Native/WASM semantic divergence | both engines | Differential suite (218/218) |
| TH-10 | Dependency confusion / typosquat | Cargo, Actions | Minimal deps; `cargo audit`; actions pinning is debt |
| TH-11 | Diagnostic information leak | diagnostics | No secrets in diagnostics; paths shown as given |
| TH-12 | Boundary cycle/aliasing blow-up | Python conversion | Node budget + cycle detection |

## Out of scope

- Sandboxing untrusted Aura/Python code (separate future
  `AURA SANDBOX PROGRAM` if required; WASM is one substrate but not a complete
  sandbox for native execution).
- Side channels, physical attacks, and threats requiring a compromised host.

## Review cadence

Sensitive surfaces (bridge, WASM, module loader, release pipeline) receive an
independent adversarial pass before each release candidate.
