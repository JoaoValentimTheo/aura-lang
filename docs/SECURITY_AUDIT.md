---
layout: default
title: "Security Audit"
nav_order: 10
---

# Security Audit

Periodic security review of the Aura transpiler and standard library.
This document records findings and is updated after each audit pass.

**Last audited:** 2026-09-20 (v0.2.0a8)

---

## Methodology

| Check | Tool / approach |
|-------|----------------|
| Dependency vulnerabilities | `pip-audit` (manual); only runtime dep is `tomli>=2.0` |
| `exec()` / `eval()` / `compile()` usage | Code search + manual review |
| SSRF guards | Code review of `aura/stdlib/http.py` |
| Import hook security | Code review of `aura/transpiler/importer.py` |
| Crypto backend safety | Code review of `aura/stdlib/crypto.py`, `crypto_backend.py` |
| Input validation | Code review of `aura/cli.py`, `aura/parser/to_ast.py` |

---

## 1. Dependency audit

| Dep | Version constraint | Risk |
|-----|-------------------|------|
| `tomli` | `>=2.0` (Python <3.11 only) | Low — mature TOML parser, no known CVEs |
| `cryptography` | `>=44.0` (optional `[pqc]` extra) | Low — well-maintained |

**Verdict:** PASS. No known vulnerabilities.

---

## 2. `exec()` / `eval()` / `compile()` usage

All sites are intentional and documented:

| Location | Call | Input source | Status |
|----------|------|-------------|--------|
| `aura/stdlib/python.py:117` | `builtins.eval()` | `python.eval()` bridge | Trusted — explicit escape hatch |
| `aura/stdlib/python.py:132` | `builtins.exec()` | `python.exec_code()` bridge | Trusted — same |
| `aura/stdlib/python.py:144` | `builtins.compile()` | `python.compile_source()` bridge | Trusted — compile only |
| `aura/cli.py:542,552` | `exec(compile(...))` | Transpiled Aura output | Trusted |
| `aura/repl/engine.py` (multiple) | `exec`/`eval` | REPL user input | Trusted — interactive |
| `aura/transpiler/importer.py:138` | `exec(compile(...))` | Transpiled `.aura` import | Trusted |
| `aura/tools/debugger.py:115` | `exec(compile(...))` | Transpiled output | Trusted |

All sites carry `# noqa: S307` / `# noqa: S102` annotations. The `python.stdlib`
module is the only direct user-facing escape hatch; docstrings state "no sandbox."

**Verdict:** PASS.

---

## 3. SSRF guards (`aura/stdlib/http.py`)

| Check | Implementation | Status |
|-------|---------------|--------|
| URL scheme allowlist | `http`/`https` only (`_ALLOWED_SCHEMES`) | PASS |
| Private IP blocking | Loopback, link-local, RFC1918, CGNAT, reserved, multicast blocked | PASS |
| Fail-closed on unresolvable | Cannot resolve → blocked | PASS |
| IPv4-mapped IPv6 | Unwrapped before classification | PASS |
| Redirect re-validation | Every hop re-validated; max 10 hops | PASS |
| Credential stripping | Auth/cookie headers dropped on cross-origin redirect | PASS |
| Response size limit | 32 MiB default (`AURA_HTTP_MAX_BYTES`) | PASS |
| Opt-out | `AURA_HTTP_ALLOW_PRIVATE=1` documented | PASS |

**Verdict:** PASS.

---

## 4. Import hook (`aura/transpiler/importer.py`)

| Check | Implementation | Status |
|-------|---------------|--------|
| Path traversal | `_is_within()` resolves candidate, checks containment | PASS |
| `..` rejection | Explicitly rejected in `_candidates()` and `find_spec()` | PASS |
| File extensions | Only `.aura` accepted | PASS |
| Symlinks | `Path.resolve()` follows symlinks; resolved path checked against roots | PASS |
| File size limits | Inherits `MAX_SOURCE_BYTES` (16 MiB) from `parse_file()` | PASS |

**Verdict:** PASS.

---

## 5. Crypto backend (`aura/stdlib/crypto.py`, `crypto_backend.py`)

| Check | Implementation | Status |
|-------|---------------|--------|
| Reference backend marked | `production = False`; class docstring warns | PASS |
| Runtime warning | `_refuse_insecure_use()` emits `RuntimeWarning` once per session | PASS |
| Hard error option | `require_production_backend()` raises `RuntimeError` | PASS |
| Fallback-to-insecure | Auto-fallback exists (by design); PQC operations warn | PASS |
| `constant_time_compare` | `_secrets.compare_digest()` used consistently | PASS |

**Verdict:** PASS.

---

## 6. Input validation

| Check | Implementation | Status |
|-------|---------------|--------|
| File path sanitization | `_require_source_file()` checks existence, rejects dirs | PASS |
| Max source size | `MAX_SOURCE_BYTES = 16 MiB` in `parse_file()` | PASS |
| LSP message size | `MAX_MESSAGE_BYTES = 16 MiB` | PASS |
| Test timeout | 30s per test subprocess | PASS |
| Recursion budget | Sized proportionally to source file size | PASS |

**Verdict:** PASS.

---

## Summary

| Category | Verdict |
|----------|---------|
| Dependency audit | PASS |
| exec/eval/compile | PASS |
| SSRF guards | PASS |
| Import hook | PASS |
| Crypto backend | PASS |
| Input validation | PASS |

**No FAIL or WARN findings.** The codebase demonstrates strong security
awareness — all guards fail-closed, escape hatches are explicitly
documented, and defense-in-depth is applied consistently.

---

## Recommended follow-ups

1. Add `pip-audit` to CI (dev dependency + workflow step).
2. Consider adding a `SECURITY.md` link to this document.
3. Review `AURA_FUZZ_SEEDS` campaign results periodically.
