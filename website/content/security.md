# Security

Aura's security posture is documented in the repository. This page summarizes
it and links to the normative documents.

## Trust boundaries

The standard library never calls the operating system directly: all authority
flows through a **host** boundary. A native host has filesystem access; the
browser (Playground) host has none. The optional **`py`** feature is the one
deliberate escape hatch to full host authority through CPython.

## What is not claimed

- **The `py` feature is not a sandbox.** It runs arbitrary CPython with full
  host authority. Sandboxing untrusted Python is a separate, unclaimed program.
- **No memory-unsafety.** The production Rust contains no `unsafe`, and Miri is
  a blocking CI gate.
- **No reproducible-build claim beyond single-host pure-Rust builds.**

## Controls

- **Fuzzing.** Lexer, parser, checker, and runtime run under continuous fuzzing;
  the property suite asserts no host panic under arbitrary input.
- **Resource limits.** Every user-controlled size is bounded with a structured
  diagnostic, never a crash (`E1015` nesting, `E4013` overflow/range, and so
  on).
- **Determinism.** Diagnostics are single-output and maps are ordered, so a
  program's behavior and messages do not depend on hash iteration order.
- **Supply chain.** Dependencies are minimal and locked; `cargo audit` runs in
  CI; third-party GitHub Actions are SHA-pinned; releases carry a CycloneDX
  SBOM and signed SLSA build provenance.
- **Immutable artifacts.** Published runtimes and releases are never
  overwritten; a fix ships as a new version.

## Reporting a vulnerability

Please report suspected vulnerabilities privately through the repository's
security policy
([`SECURITY.md`](https://github.com/JoaoValentimTheo/aura-lang/blob/rewrite/v3-rust/SECURITY.md))
rather than a public issue. The full threat model, trust boundaries, security
architecture, and incident-response plan live under
[`docs/security/`](https://github.com/JoaoValentimTheo/aura-lang/tree/rewrite/v3-rust/docs/security).
