# Incident Response

How Aura handles security incidents, from report to postmortem.

## Severity

| Level | Definition | Examples |
|---|---|---|
| **CRITICAL** | Host compromise, memory unsafety, frozen-artifact corruption, artifact substitution, sandbox escape where a sandbox is claimed | RCE via runtime, tampered release bytes |
| **HIGH** | Wrong program result, invalid program accepted, valid program fundamentally rejected, native/WASM semantic divergence, unbounded resource amplification | Silently wrong arithmetic in a documented case |
| **MEDIUM** | Misleading diagnostic, bounded inconsistency, REPL recovery defect, user-affecting doc contradiction | Wrong error code with correct rejection |
| **LOW** | Minor docs, minor diagnostics, negligible inefficiency | Stale comment |

## Lifecycle

1. **Report** — private GitHub Security Advisory. Record reporter, version,
   commit, minimal reproduction.
2. **Triage** — assign severity; confirm the affected surface (native/wasm/CLI/
   playground/bridge/release).
3. **Safe reproduction** — reproduce in an isolated checkout; never run
   untrusted exploits against shared infrastructure.
4. **Root cause** — locate the exact code path; distinguish symptom from cause.
5. **Fix** — smallest justified change, with a regression test; independent
   review; red-team recheck for HIGH/CRITICAL.
6. **Advisory** — describe affected versions, impact, and the fix; withhold
   exploit details until a fix is available (coordinated disclosure).
7. **Release** — publish an additive patch release; never rewrite published
   history or overwrite artifacts.
8. **Postmortem** — record cause, detection gap, and prevention; update
   `docs/engineering/TECHNICAL_DEBT.md` and `RISK_REGISTER.md`.

## Emergency patches

Security fixes may trigger an out-of-band patch release, which still passes:
local gate, remote CI, independent review, release audit, artifact hash
verification, and post-release smoke.

## Non-negotiable rules

- Never delete evidence (crash artifacts, failing tests) to obtain green.
- Never force-push or rewrite published history to hide a failure.
- Never overwrite a released artifact or tag; a new version is the recovery.
- Never weaken CI to pass a failure.

## Contact

Use GitHub Security Advisories on
`github.com/JoaoValentimTheo/aura-lang`. Do not open a public issue for an
unfixed vulnerability.
