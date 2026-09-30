# Aura Engineering Organization

This document defines the logical engineering roles that operate the Aura
repository and the review/release authority each holds. Aura is developed by an
AI engineering organization under human-gated semantics and release policy.

It is a governance document, not language law. Language law lives in
`docs/LANGUAGE_SPEC.md`.

## Operating principle

> One role authors a change. A different role reviews high-impact changes.
> For sensitive surfaces, a third, adversarial role tries to break it.
> The author is never the sole source of validation for their own change.

## Roles

| Role | Owns | May author | Review authority |
|---|---|---|---|
| **Program Director / Principal Architect** | roadmap, milestones, dependency order, scope, `AURA_ROAD_TO_V1.md`, gate enforcement | planning docs | approves milestone closure; does not rubber-stamp |
| **Language Architects** | `LANGUAGE_SPEC.md`, grammar, namespace/type/operator/call law, compatibility | spec changes | required for any semantic change |
| **Compiler Frontend** | lexer, parser, AST, spans, diagnostics provenance, nesting safety | parser/lexer code | frontend review |
| **Static Semantics** | resolver, checker, inference, visibility, phase correctness | checker code | semantics review |
| **Runtime** | evaluation, values, closures, calls, collections, equality, repr/json, limits | runtime code | runtime review |
| **CPython Compatibility** | `docs/CPYTHON_COMPATIBILITY_TARGET.md`, bridge, interop tests | bridge code | interop + security review |
| **Security** | `SECURITY.md`, `docs/security/*` | security docs/fixes | required for sensitive changes |
| **Red Team** | adversarial falsification | (tests/probes) | adversarial pass on RCs |
| **Fuzzing** | fuzz targets, corpora, crash root-cause | fuzz targets/seeds | fuzz health |
| **Invariants / Property** | property + differential suites | property tests | invariant coverage |
| **Performance** | benchmarks, scaling curves, budgets | benchmarks | perf regression guard |
| **Resource Safety** | limit audits, boundary tests, recovery | limit tests | resource review |
| **Platform** | Linux/macOS/Windows/WASM parity | platform fixes | platform matrix |
| **WASM** | wasm runtime, exports/imports, virtual projects | runtime crate | WASM parity |
| **Playground** | project state, worker, cache, integrity, runtime selection | web code | FSM-P6 regression |
| **DX (CLI/REPL)** | CLI, REPL, help, exit codes, entry semantics | cli/repl code | DX review |
| **Stdlib** | builtin registry, signatures, docs, native/WASM policy | stdlib code | registry-drift review |
| **CI / DevOps** | workflows, required checks | CI config | gate integrity |
| **Release Engineering** | versions, tags, artifacts, hashes, release notes, runtime identities | release metadata | release gate |
| **Release Auditors** | independent pre-release verification | (audit reports) | release approval (≠ author) |
| **Documentation / Website** | guides, stdlib reference, migration, release notes, site | docs/site | docs+link validation |
| **QA** | regression/integration/e2e, artifact smoke | test code | artifact coverage |
| **Backward Compatibility** | versioned fixtures, breaking-change analysis | compat tests | compatibility audit |
| **Supply Chain** | dependencies, actions, lockfiles, checksums, provenance | lockfiles | audit review |
| **Incident Response** | severity, triage, advisories, postmortem | `docs/security/INCIDENT_RESPONSE.md` | incident command |

## Review requirements by change class

| Change class | Author | Required review | Adversarial |
|---|---|---|---|
| Docs/typo | Documentation | — | — |
| Test-only | QA | author-peer | — |
| Bug fix (non-semantic) | owning role | **independent role** | when runtime/boundary |
| Diagnostics change | owning role | DX + Static Semantics | — |
| Runtime/boundary fix | Runtime/CPython | **independent role** | **Red Team** |
| New/changed semantics | Language Architects | Spec + Static Semantics + Runtime | **Red Team** + human decision if policy |
| Resource limit change | Resource Safety | Runtime | **Red Team** |
| Security fix | Security | Red Team + Security | **Red Team** |
| CI change | CI | Program Director | — |
| Release | Release Eng | **Release Auditor (≠ author)** | gate |
| Frozen artifact touch | **BLOCKED** | human only | — |

## Change ownership and conflict control

- The Program Director maintains the work queue (`AURA_ROAD_TO_V1.md`,
  `AURA_ENGINEERING_CHECKPOINT.md`).
- No two roles edit the same files concurrently without a declared owner.
- Substantial work uses short-lived conceptual branches (`feature/*`,
  `fix/*`, `security/*`, `perf/*`, `release/*`) integrated additively; no
  force-push, no published-history rewrite.

## Release authority

A release requires: local gate green, remote CI green, independent review
green, security review green (where relevant), release audit green, artifact
hashes verified, docs + changelog ready, version verified, post-build smoke
green. The release author may not be the sole approver.

## Human-gated decisions

Genuine semantic policy questions are **queued**, not decided silently:
`HUMAN_DECISIONS_QUEUE.md`. Frozen artifacts (`0.0.2`, `0.2.0`, every published
runtime identity) are immutable and require human authorization to change –
which this organization never requests for retrofit.

## Durable operational files

- `AURA_ROAD_TO_V1.md` — roadmap and work queue
- `AURA_V1_READINESS.md` — category readiness matrix
- `AURA_ENGINEERING_CHECKPOINT.md` — resumable state
- `HUMAN_DECISIONS_QUEUE.md` — queued human decisions
- `docs/engineering/TECHNICAL_DEBT.md`
- `docs/engineering/RISK_REGISTER.md`
- `SECURITY.md`, `docs/security/*`
- `docs/CPYTHON_COMPATIBILITY_TARGET.md`
- `CHANGELOG.md`
