# Aura — Road to 1.0

Program: **Aura Autonomous Engineering Organization — Road to Aura 1.0**.
Authority order: `docs/LANGUAGE_SPEC.md` > this roadmap for engineering order.

## Program state at intake

- Base `6d25985` = remote; CI green; frozen `0.0.2`/`0.2.0` intact.
- Core language COMPLETE (see `AURA_COMPLETENESS_MATRIX.md`).
- Latest release `v0.2.0`; latest dev runtime `0.2.0-dev.2`.
- Open semantic items: HD-1, AUDIT-3.

## Mission

Make Aura deserve v1.0: production-grade, secure, performant, cross-platform,
with a meaningful, precisely-scoped CPython story and a hardened release
process. v1 is evidence-based, never date-based.

## Release trains

| Train | Contents | Status |
|---|---|---|
| **T0 — Org bootstrap** | org docs, readiness, roadmap, decision queue, threat model, CPython target, benchmarks, release policy, changelog | IN PROGRESS |
| **T1 — Patch train 0.2.x** | confirmed defects; bridge diagnostic provenance; risk/debt closure | IN PROGRESS |
| **T2 — CPython interop train** | precise compatibility target + bridge hardening + interop matrix | PLANNED |
| **T3 — Security train** | threat model, trust boundaries, supply-chain, WASM import policy | PLANNED |
| **T4 — Performance/resource train** | benchmarks, budgets, scaling guards | PLANNED |
| **T5 — Platform/packaging train** | install paths, artifact matrix, provenance | PLANNED |
| **T6 — v1 semantic freeze** | close HD-1/AUDIT-3 or exclude; freeze core | PLANNED |
| **T7 — RC train** | 1.0.0-alpha/beta/rc with independent audits | PLANNED |

## Work queue (current)

| ID | Owner role | Subsystem | Risk | Depends | Status | Reviewer |
|---|---|---|---|---|---|---|
| T0-1 | Program Director | org docs | low | — | done | self |
| T0-2 | Program Director | readiness/roadmap/queue | low | T0-1 | in progress | self |
| T0-3 | Security | threat model/boundaries | med | — | queued | Red Team |
| T0-4 | CPython Compat | compatibility target | med | — | queued | Lang Arch + Security |
| T0-5 | Performance | benchmark baseline | low | — | queued | — |
| T0-6 | Release Eng | release policy + changelog | low | — | queued | Release Auditor |
| T1-1 | Runtime/CPython | bridge span provenance fix | med | — | **done** | (needs independent review) |
| T1-2 | Red Team | boundary attack pass | med | — | in progress | Security |
| T2-1 | CPython Compat | interop test matrix + policy | high | T0-4 | queued | Red Team |
| T3-1 | Security | WASM import policy | med | T0-3 | queued | WASM |
| T3-2 | Supply Chain | actions pinning/least privilege | med | — | queued | CI |

## Dependency ordering rule

Core semantics before ecosystem; compatibility before freeze; freeze before RC;
no RC code change without revalidation. Any change to a frozen artifact stops
the program for a human decision.

## Scope guards

Do not start: package manager/registry, network/async runtime, macro system,
IDE/LSP platform, or large ecosystem work — unless the normative spec makes it
necessary for v1. No new language feature may begin implementation-first.

## Milestone exit criteria

A train closes only when: scope complete, no unresolved in-scope blocker, new +
adjacent tests green, docs synchronized, full relevant local gate green, frozen
hashes verified, commits coherent, push complete, required remote CI green.
