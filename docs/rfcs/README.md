# Aura RFCs

A syntax or semantics change requires an RFC. Open an issue titled
`RFC: <change>` with:

1. **Motivation** — the problem it solves, with real examples.
2. **Grammar delta** — the exact change to `docs/grammar.md`.
3. **Migration** — how existing programs are affected.
4. **Enforcement** — the tests that will keep the promise.

Accepted RFCs are recorded here as `NNNN-short-title.md`.

## Index

| RFC | Title | Status |
|---|---|---|
| [0001](./0001-exception-catch-selection.md) | Exception Catch Selection | Accepted (Keystone, 2026-10-06) |
| [0002](./0002-result-and-recoverable-errors.md) | `Result` and Recoverable Error Semantics | Proposed — D1 escalated (D1a/D1b/D1c) |
| [0003](./0003-try-as-value-expression.md) | `try` as a Value Expression | Proposed — D1, D2 escalated |
| [0004](./0004-diagnostics-contract.md) | The Diagnostics Contract | Proposed — D3 + open item 13 escalated |
| [0005](./0005-inference-and-error-propagation.md) | Inference and Error-Propagation Rules | Proposed — D1 escalated |
| [0006](./0006-ais-agent-native-capabilities.md) | The AIS Agent-Native Capability Set | Proposed — D1, D2, D3 escalated |
| [0007](./0007-project-structure-init-and-os.md) | Project Structure, `aura init`, and the `os` Capability | Proposed — D1, D2, D3 escalated |
| [0008](./0008-host-database-capability.md) | Host Database Capability and the Protected Profile | Proposed — D1, D2, D3 escalated |

“Proposed” means the RFC is authored and reviewed but is **not** authorized for
implementation. Each lists its open questions under `## 7. Open items`, and no
code may change until the human decisions it names are resolved.
