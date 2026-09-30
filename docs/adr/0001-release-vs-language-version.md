# ADR-0001 — Release version vs language version scheme

- **Status:** Accepted (Architecture Decision Council, 2026-09-30)
- **Supersedes:** the queued human decision HD-4
- **Related:** `docs/engineering/RELEASE_ENGINEERING.md`,
  `src/lib.rs` (`VERSION`, `LANGUAGE_VERSION`), `tests/contract.rs`,
  `playground/runtime/src/lib.rs` (`ABI_VERSION`, `RUNTIME_VERSION`)

## Context

Aura carries several version identities. At the time of this decision:

| Identity | Source | Value at HEAD |
|---|---|---|
| Release version `aura::VERSION` | `CARGO_PKG_VERSION` | `0.2.0` |
| Language version `LANGUAGE_VERSION` | `src/lib.rs` literal | `0.2.0` |
| Host ABI `ABI_VERSION` | `playground/runtime/src/lib.rs` | `1` |
| Playground API | Playground manifest/ABI | `1` |
| Runtime version `RUNTIME_VERSION` | `playground/runtime/Cargo.toml` | `0.2.0-dev.2` |

The repository evolved **semantically past** the released tag `v0.2.0`
(commit `668722f`, 2026-09-28). The builtin-name value-namespace reservation
(`E1009`, commit `c8ded06`, 2026-09-30) is **not** an ancestor of `v0.2.0`:

- tag `v0.2.0`: `let sum = 1` is **accepted**;
- HEAD: `let sum = 1` is **rejected** with `E1009`;
- `git tag --contains c8ded06` is empty.

Yet HEAD declared `VERSION == LANGUAGE_VERSION == "0.2.0"`, and
`tests/contract.rs` asserted both equal `"0.2.0"`. Two artifacts (tag `v0.2.0`
and current HEAD) therefore claimed the **same** language identity while
behaving differently — exactly the situation a version number must never
create. This blocked any honest release from HEAD.

## Decision

Adopt a two-identity model, plus the existing independent ABI/runtime
identities.

1. **Release version** (`RELEASE_VERSION` = `aura::VERSION` =
   `CARGO_PKG_VERSION`) identifies a *published artifact* (crate, binary,
   release tag). It advances by Semantic Versioning: patch for fixes, minor for
   backward-compatible feature additions, major for breaks.

2. **Language version** (`LANGUAGE_VERSION`) identifies the *observable
   language contract* — the accept/reject behavior and semantics a program can
   rely on. It advances whenever language-observable behavior changes
   (including a change in what is accepted or rejected), independently of
   runtime/tooling-only releases.

3. **Invariant:** `LANGUAGE_VERSION` **≤** `RELEASE_VERSION` (compared as
   semver). A release may advance without a language change (runtime, tooling,
   packaging, diagnostics wording); a language change never ships ahead of the
   release that carries it. They are equal when a release is a language
   release.

4. The two identities are **never** conflated. A published artifact whose
   language behavior differs from a prior artifact MUST have a different
   `LANGUAGE_VERSION`.

5. `ABI_VERSION` (host ABI) and the Playground API version remain separate,
   monotonic-integer compatibility contracts; `RUNTIME_VERSION` remains the
   independent identity of a WASM runtime artifact. No new identity dimensions
   are introduced.

### Immediate application

Because the builtin-name reservation changed observable language behavior
after tag `v0.2.0`, HEAD must not be published as `0.2.0`:

- `RELEASE_VERSION`: `0.2.0` → **`0.2.1`**
- `LANGUAGE_VERSION`: `0.2.0` → **`0.2.1`**

The reservation is classified as a **patch-level language correction**: it
intentionally rejects programs that were never semantically well defined (the
binding collided with a builtin and broke assignment lookup), so no
well-defined program changes meaning. Patch (not minor) is therefore the
honest increment.

Historical artifacts keep their identities unchanged and immutable:
tag `v0.2.0`, frozen runtimes `0.0.2`/`0.2.0`, and development runtimes
`0.2.0-dev.1`/`0.2.0-dev.2`. Their `language_version` metadata remains correct
*for those artifacts*. A future runtime that carries the `0.2.1` language
contract receives a **new** identity (`0.2.1-dev.2`; the earlier `0.2.1-dev.1`
build was superseded by the ADR-0004 parser change); dev.2 is never relabeled.

## Consequences

- `tests/contract.rs` generalizes: assert `VERSION == CARGO_PKG_VERSION`,
  both identities are valid semver, and `LANGUAGE_VERSION ≤ RELEASE_VERSION`.
  The hardcoded `"0.2.0"` assertions are removed.
- A machine-readable metadata consistency test is added so the identities can
  never silently drift.
- Pre-1.0, honest development/prerelease progression becomes possible:
  `0.2.1` → later `0.3.0` → `1.0.0-rc.N` → `1.0.0`.
- The Playground reports each runtime's own `language_version` from the
  manifest, so a user on dev.2 correctly sees `0.2.0` while the current line is
  `0.2.1`.

## Alternatives considered

- **Keep both at `0.2.0` and document the difference.** Rejected: reuses one
  version number for two incompatible language behaviors.
- **Advance release only, keep language `0.2.0`.** Rejected: would claim two
  artifacts share a language identity while rejecting different programs.
- **Introduce further dimensions (separate syntax/semantics/ABI versions).**
  Rejected: needless complexity; the existing identities suffice.
- **Jump straight to `1.0.0`.** Rejected: v1 requires the full readiness gate.

## References

- `docs/engineering/RELEASE_ENGINEERING.md`
- `src/lib.rs` (`VERSION`, `LANGUAGE_VERSION`)
- `tests/contract.rs` (version consistency)
- `HUMAN_DECISIONS_QUEUE.md` HD-4 (now closed by this ADR)
