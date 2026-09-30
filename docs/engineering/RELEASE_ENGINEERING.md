# Release Engineering Policy

How Aura versions, builds, audits, and publishes releases. Complements
`docs/security/INCIDENT_RESPONSE.md` and the workflow in
`.github/workflows/release.yml`.

## Version scheme

Semantic Versioning, adjusted for Aura's pre-v1 history:

- `MAJOR.MINOR.PATCH`, pre-release suffixes `-alpha.N`, `-beta.N`, `-rc.N`.
- The **release version** (`Cargo.toml`), the **language version**
  (`aura::LANGUAGE_VERSION`), and the **runtime artifact version** are distinct
  identities and must not be conflated. `0.2.0` happens to coincide across
  release and language; this is not guaranteed.
- Version integrity is CI-checked: `Cargo.toml` == `Cargo.lock` ==
  `aura::VERSION`.

## Immutability (non-negotiable)

- Never overwrite a published tag, asset, or artifact byte under the same
  identity.
- Never force-move a release ref; never `overwrite_files` on a release.
- Frozen runtimes `0.0.2` and `0.2.0` are immutable forever.
- Every published development runtime identity (`0.2.0-dev.1`, `0.2.0-dev.2`,
  …) is immutable; a semantic change requires a **new** identity, never an
  overwrite.

## Version selection

- Never fabricate a version. Read the existing release history and current
  `Cargo.toml`.
- A tag `vX.Y.Z` must equal the crate version (enforced by the release
  workflow).
- Advancing a development runtime means bumping its pre-release identifier and
  pinning the superseded identity in `playground/build.mjs`.

## Release gate (all required)

1. Local gate green (fmt, all-features, no-default, clippy `-D warnings`, MSRV,
   Miri, runtime crate, playground, website, build `--check`).
2. Remote CI green on the release commit.
3. Independent review green (release author ≠ sole approver).
4. Security review green where the release touches a trust boundary.
5. Release audit green: commits, version, changelog, notes, artifacts, hashes,
   frozen history, no secrets, expected binaries, dependency audit.
6. Artifact hashes verified.
7. Docs + CHANGELOG ready.
8. Post-build smoke green (built executable runs `version` and a program).

## Artifact matrix

| Target | Archive | Smoke-tested in CI |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `aura-x86_64-unknown-linux-gnu.tar.gz` | yes |
| `aarch64-apple-darwin` | `aura-aarch64-apple-darwin.tar.gz` | yes |
| `x86_64-pc-windows-msvc` | `aura-x86_64-pc-windows-msvc.tar.gz` | yes |
| wasm32-unknown-unknown (Playground runtime) | `aura-playground-runtime-<version>.wasm` | Playground suites |

Each archive ships with a `.sha256`. Released binaries are built **pure-Rust**
(no CPython) for the release artifact; the `py` feature remains available to
source builders.

## Post-release gate

Download released artifacts, verify hashes, run `version`, run representative
programs (and interop smoke if `py` shipped), verify the website/Playground
references. Only then is the release checkpoint closed.

## Reproducibility

**Measured (2026-09, one host, one toolchain):** two independent clean builds of
the pure-Rust feature set in separate `CARGO_TARGET_DIR`s produced
**bit-identical** executables
(`a12b2c5555dd866b825a85869ad297d84a6943c98a78b0c533b7c1dcc042201c`). This is
consistent with the release profile (`lto = true`, `codegen-units = 1`) and the
absence of build-script non-determinism.

**Not yet claimed:** cross-host and cross-toolchain bit-reproducibility, and
reproducibility of the `py`-feature build (which links a host-provided CPython
and is expected to differ per interpreter). Until those are measured and
recorded, Aura claims reproducible builds **only** for the pure-Rust build on a
fixed toolchain/host.

## Not yet implemented (tracked debt)

- SBOM, build provenance, artifact signing (TD-03).
- Cross-host/cross-toolchain reproducible-build verification (TD-04).
- SHA-pinned GitHub Actions (TD-01) — **done**; standard actions are pinned.

## Rules

- No force-push. No history rewrite. No destructive git operations.
- Broken release → new version, never a replaced asset.
- Prereleases follow the same gates as stable, at their advertised stability
  level.
