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

- **Status:** QUEUED — **blocks the automatic patch release.**
- **Question:** For the road to 1.0, do we release `0.3.0` minor trains, or move
  directly to `1.0.0-alpha.N`/`-beta.N`/`-rc.N`? And: may the **release** version
  advance past the **language** version within the pre-1.0 line (a bug-fix
  patch)?
- **Evidence:**
  - existing releases `v0.0.1`, `v0.0.2`, `v0.2.0`; language pinned at `0.2.0`.
  - `tests/contract.rs:191-204` asserts `aura::VERSION == "0.2.0"` and
    `aura::LANGUAGE_VERSION == aura::VERSION`. A bug-fix release
    `0.2.1` (language unchanged at `0.2.0`) would fail this test, which was
    written for the 0.2.0 *language* release.
- **Consequence:** the T1 patch train (bridge diagnostic fix) is
  release-ready, but tagging it requires deciding whether release ≠ language is
  permitted. This is a genuine version-policy decision, not an implementation
  choice, so it is not decided unilaterally.
- **Options:**
  - (A) Allow release > language pre-1.0: relax the equality assertion to
    `VERSION >= LANGUAGE_VERSION` and cut `0.2.1` as a patch.
  - (B) Keep release == language until 1.0: fold the patch into the next
    language-release train (no standalone patch).
  - (C) Jump to a prerelease scheme now (`0.3.0-alpha.N` or `1.0.0-alpha.N`).
- **Recommendation:** Option A for the near term (enables honest, frequent
  patch releases of fixes that do not change language semantics), transitioning
  to `1.0.0-rc.N` for stabilization. Requires human confirmation before the
  first such tag.
