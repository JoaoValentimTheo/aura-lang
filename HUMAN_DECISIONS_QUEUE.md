# Human Decisions Queue

Genuinely human-level decisions. Under the permanent engineering organization
the Architecture Decision Council holds delegated authority to resolve these;
each resolution produces an ADR. This file records the outcomes and any truly
owner-level items (legal/licensing/credentials).

## Closed by Architecture Decision Council

- **HD-4 (version scheme)** — RESOLVED. See
  `docs/adr/0001-release-vs-language-version.md`. Release and language versions
  are distinct; `LANGUAGE_VERSION <= VERSION`; HEAD advances to `0.2.1`
  (release + language) because the builtin reservation changed observable
  behavior after tag `v0.2.0`.
- **HD-1 (module-member vs builtin reservation)** — RESOLVED. See
  `docs/adr/0002-module-member-builtin-names.md`.
- **HD-3 (CPython version policy)** — RESOLVED. See
  `docs/adr/0003-supported-cpython-versions.md`.
- **HD-2 (AUDIT-3 TypeExpr nesting)** — RESOLVED. See
  `docs/adr/0004-typeexpr-nesting-policy.md`.

No owner-level decisions are currently pending.


---

## HD-1 — Module-member identifiers vs builtin-name reservation

- **Status:** RESOLVED by ADR-0002 (`docs/adr/0002-module-member-builtin-names.md`). Original package:
  `docs/HD1_MODULE_MEMBER_BUILTIN_NAMES_DECISION.md`.
- **Question:** Does the builtin name reservation (`E1009`) extend to module
  members (`module M { fn sum }`)?
- **Current behavior:** Module members occupy the module namespace; a member
  named `sum` shadows the builtin only inside its module and never unqualified
  in a caller. Accepted.
- **Recommendation:** Option A — keep behavior; clarify §3.3 wording later.

---

## HD-2 — AUDIT-3 TypeExpr nesting policy

- **Status:** RESOLVED by ADR-0004 (Option A adopted). Original package:
- **Question:** Which TypeExpr nesting policy (Option A or B) does Aura adopt?
- **Current behavior:** Native accept ≤2047 / reject 2048 (`E1015`); WASM accept
  ≤767 / reject 768. Not normalized; property tests exclude TypeExpr-heavy
  inputs.
- **Required before v1 semantic freeze:** decide, or explicitly freeze the
  current substrate-dependent behavior as documented. Decision package:
  `docs/AUDIT3_TYPE_NESTING_DECISION.md`.

---

## HD-3 — Supported CPython version policy

- **Status:** RESOLVED by ADR-0003 (`docs/adr/0003-supported-cpython-versions.md`).
- **Question:** Which CPython versions does Aura v1 support and test against?
- **Evidence:** local bridge currently links a single CPython (3.9 at intake);
  no version matrix in CI.
- **Recommendation:** support the currently-maintained CPython minor lines that
  CI can install on all three platforms (e.g. 3.10–3.13), and state the exact
  set in `docs/CPYTHON_COMPATIBILITY_TARGET.md`. See that file.

---

## HD-4 — Pre-v1 release line and version scheme

- **Status:** RESOLVED by ADR-0001 (`docs/adr/0001-release-vs-language-version.md`). Original package:
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
  - **Version-identity divergence (independently found):** HEAD declares
    `VERSION = LANGUAGE_VERSION = "0.2.0"`, but HEAD is *semantically different*
    from tag `v0.2.0` (commit `668722f`, 2026-09-28). The builtin-name
    reservation (`c8ded06`, 2026-09-30) is not an ancestor of `v0.2.0`
    (`git tag --contains c8ded06` is empty). So two distinct language surfaces
    both claim `0.2.0`: the released tag accepts `let sum = 1`; HEAD rejects it
    with `E1009`. This is the concrete reason the version scheme needs a
    decision before the next tag.
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
