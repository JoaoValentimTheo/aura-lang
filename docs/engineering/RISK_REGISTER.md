# Aura Risk Register

Program-level risks with likelihood (L), impact (I), mitigation, and status.
Likelihood/impact: Low / Medium / High.

| ID | Category | Risk | L | I | Mitigation | Status |
|---|---|---|---|---|---|---|
| R-01 | Semantic | HD-1 ambiguity resolved by accident | L | M | Decision package; no accidental coding; queued | Open |
| R-02 | Semantic | AUDIT-3 nesting policy silently changed | L | H | Decision-pending; tests exclude TypeExpr-heavy inputs | Open |
| R-03 | CPython | Users assume `py` is sandboxed | M | H | `SECURITY.md` + trust boundaries state arbitrary authority | Mitigated (documented) |
| R-04 | CPython | Boundary conversion blow-up (cyclic/aliased) | L | H | Node budget + cycle detection; regression tests | Mitigated |
| R-05 | Security | Panic/abort from crafted input | L | H | Fuzz + property + limits; `panic`/`unwrap` denied lints | Mitigated |
| R-06 | Security | Artifact substitution in Playground | L | H | SHA-256 pinned; loader verifies; releases immutable | Mitigated |
| R-07 | Supply chain | Compromised dependency/action | L | H | Minimal deps; `cargo audit`; SHA-pin actions (TD-01) | Partially mitigated |
| R-08 | Performance | Undetected asymptotic regression | M | M | Add benchmark suite + scaling guards (TD-05) | Open |
| R-09 | Platform | Windows/macOS divergence from Linux-only logic | L | M | 3-OS CI matrix | Mitigated |
| R-10 | WASM | Divergence between native and wasm semantics | L | H | Differential suite; new dev runtime per semantics change | Mitigated |
| R-11 | Release | Publishing a broken release | L | H | Gated release workflow; smoke tests; release audit | Mitigated |
| R-12 | Compatibility | Breaking change shipped without note | M | M | Compatibility fixtures + policy (TD-06) | Open |
| R-13 | Compatibility | Pre-1.0 version scheme confusion | M | L | HD-4 decision queued | Open |
| R-14 | CI | Required CI green but wrong thing tested | L | M | Contract/spec/artifact jobs; independent audits | Monitored |
| R-15 | Resource | New feature introduces unbounded work | M | M | Resource review per change; boundary tests | Monitored |

## Rules

- New risks are added here with an ID and explicit mitigation.
- An open HIGH-impact risk with no mitigation blocks the v1 GO gate.
