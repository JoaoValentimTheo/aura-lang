# Human Decisions Queue

Queued genuinely human-level decisions. The organization continues all
independent work and does not block on these. Each entry states the question,
current behavior, evidence, options, and the team recommendation.

---

## HD-1 — Module-member identifiers vs builtin-name reservation

- **Status:** QUEUED (non-blocking). Full package:
  `docs/HD1_MODULE_MEMBER_BUILTIN_NAMES_DECISION.md`.
- **Question:** Does the builtin name reservation (`E1009`) extend to module
  members (`module M { fn sum }`)?
- **Current behavior:** Module members occupy the module namespace; a member
  named `sum` shadows the builtin only inside its module and never unqualified
  in a caller. Accepted.
- **Recommendation:** Option A — keep behavior; clarify §3.3 wording later.

---

## HD-2 — AUDIT-3 TypeExpr nesting policy

- **Status:** DECISION-PENDING (blocking semantic freeze only).
- **Question:** Which TypeExpr nesting policy (Option A or B) does Aura adopt?
- **Current behavior:** Native accept ≤2047 / reject 2048 (`E1015`); WASM accept
  ≤767 / reject 768. Not normalized; property tests exclude TypeExpr-heavy
  inputs.
- **Required before v1 semantic freeze:** decide, or explicitly freeze the
  current substrate-dependent behavior as documented. Decision package:
  `docs/AUDIT3_TYPE_NESTING_DECISION.md`.

---

## HD-3 — Supported CPython version policy

- **Status:** QUEUED (blocks the CPython train, not other work).
- **Question:** Which CPython versions does Aura v1 support and test against?
- **Evidence:** local bridge currently links a single CPython (3.9 at intake);
  no version matrix in CI.
- **Recommendation:** support the currently-maintained CPython minor lines that
  CI can install on all three platforms (e.g. 3.10–3.13), and state the exact
  set in `docs/CPYTHON_COMPATIBILITY_TARGET.md`. See that file.

---

## HD-4 — Pre-v1 release line and version scheme

- **Status:** QUEUED (blocks tagging prereleases).
- **Question:** For the road to 1.0, do we release `0.3.0` minor trains, or move
  directly to `1.0.0-alpha.N`/`-beta.N`/`-rc.N`?
- **Evidence:** existing releases `v0.0.1`, `v0.0.2`, `v0.2.0`; language version
  pinned at `0.2.0`.
- **Recommendation:** use pre-1.0 development versions on the existing line and
  enter `1.0.0-rc.N` for the stabilization train. Requires human confirmation
  before the first prerelease tag.
