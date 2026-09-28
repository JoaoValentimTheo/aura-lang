# Aura Current Status

Authoritative continuation checkpoint.

## Repository

- Verified on 2026-09-28.
- Branch: `rewrite/v3-rust`.
- HEAD: `08378d5` (`fix(maps): infer union key/value types from every literal entry`).

Commit stack (oldest first) since the verified conformance baseline
`34eac815b6a56df7bc27f23ed4fb51565df4ff77`:

- `81e7410 docs: reconcile release-readiness documentation surfaces`
- `bacba57 fix(cli): list repl in the unknown-command hint`
- `6bdeecb feat(maps): support generic map key types`
- `8cc0b82 test(maps): cover generic keys and native/wasm parity`
- `1627c40 chore(runtime): stage dev.30 generic-map runtime`
- `55f864e docs(maps): document generic map key semantics`
- `c9d9583 docs(site): publish generic map keys`
- `876cb21 chore(ai): add Aura development agent workflows`
- `08378d5 fix(maps): infer union key/value types from every literal entry`

- Untracked: a pre-existing stray 1-byte file `s` (mtime 2026-09-28 13:11,
  before this task). Not produced by this work; left untouched.

## Last Verified Global State

- Rust all-features: **677 passed, 0 failed**.
- Rust no-default (`cli,repl,json,regex,time`): 664 passed, 0 failed.
- Playground node suite: differential 208, syntax 43, browser 53, worker 12,
  cache 7, manifest 26, ABI 67, integrity 27 — 0 failed; 0 WASM imports.
- Website suite: examples 20, links 2087, browser 343, a11y 70, plus the
  playground suites — 0 failed.
- GitHub CI for the current commits: **UNVERIFIED** (not pushed).

## Current Feature: Generic Map Keys (DESIGN APPROVED)

`DESIGN APPROVED: GENERIC MAP TYPES AND GENERIC MAP KEYS`.

Approved semantics, now implemented and adversarially verified:

- Canonical syntax `{K: V}`; `type Map<K, V> = {K: V}` is an ordinary alias.
- Key-capable = `string`, `int`, `bool`, or a union every member of which is.
- Not key-capable: `float` (NaN unordered; `0.0 == -0.0`; `1 == 1.0` across
  types), `none`, lists, maps, structs, enums, ranges, functions.
- Runtime keys are a closed scalar enum `MapKey { Int, Bool, Str }` in a
  `BTreeMap` (order `Int < Bool < Str`), so key order agrees with equality.
- Generic `K` is allowed in a declaration and must be key-capable when
  instantiated; `Unknown` is permissive (the existing §2.3 boundary).
- Literal inference folds every entry into a union per dimension.
- Map assignability checks both key and value; `1` and `"1"` are distinct keys.
- JSON objects stay string-keyed; `json_encode` of a non-string-keyed map is a
  deterministic `E3001`, never a silent stringification.
- Python bridge accepts `str`/`int`/`bool` keys (bool before int) and rejects
  others with `E5002`.

## Confirmed Adversarial Findings (both fixed)

- Pass 1 found: map-literal inference used only the first entry, so
  `{1: "a", "1": "b"}` was mis-typed `{int: string}` and `keys()` was unsound —
  a regression the feature introduced. Fixed in `08378d5` by folding all entry
  types with `Ty::union`; whole-literal annotation checking now applies.
- Pass 2 re-verified the fix: no remaining unsoundness, no new false rejection,
  no map-vs-union-list divergence, deterministic, artifact reproduces.

## Files Changed (feature)

- Source: `src/types.rs`, `src/check/mod.rs`, `src/run/value.rs`,
  `src/run/mod.rs`, `src/stdlib/signatures.rs`, `src/stdlib/mod.rs`,
  `src/stdlib/ext.rs`, `src/bridge/mod.rs`.
- Tests: `tests/maps.rs` (new, 25 tests), `tests/checker.rs`,
  `tests/python.rs`, `tests/regressions.rs`, `tests/property_hardening.rs`,
  `playground/tests/node/differential.test.mjs`.
- Docs: `docs/LANGUAGE_SPEC.md`, `docs/contract.md`, `docs/grammar.md`,
  `docs/GENERICS.md`, `README.md`, `website/content/*`, `website/pages/*`,
  `website/examples/examples.mjs`, `examples/maps.aura`(+`.out`).
- Artifact: `playground/runtimes/0.0.2-dev.30/`, `manifest.json`.

## Artifact State

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| `0.0.2` (frozen) | 1,366,621 | `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed` |
| `0.0.2-dev.23` (historical) | 1,604,958 | `71072150e67384120c63e22d6176f3683110735b84f74723bea315f79778a528` |
| `0.0.2-dev.29` (prior current) | 1,614,239 | `aa832ba72578897f6b99650574939011dda25e0d390fdb5efb6fae825816bdd3` |
| `0.0.2-dev.30` (current) | 1,619,576 | `87a4ae642b9126d5a3da38dca32930ea7f5902203b6f33535044b895d6cda05c` |

dev.30 reproduces byte-for-byte across three independent clean builds, with
zero WASM imports. `0.0.2` and dev.23–dev.29 are untouched.

## Open Decisions / SPEC GAPs

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

Nine gaps remain unresolved and were NOT implemented: CONF-PARSE-8; numeric
underscore placement; f-string outer edge rules; CONF-GRAM-4;
CONF-RESOLVE-6; CONF-RESOLVE-7; CONF-RESOLVE-8; CONF-RESOLVE-9;
CONF-RESOLVE-10.

## Known Pre-existing Limitations (not this feature)

- A generic parameter bound to a union and then re-used in a second argument
  position is rejected (`fn pick<K>(xs: [K], k: K)` on `[Key]`); reproduces on
  baseline `bacba57` without maps. Out of scope.
- `m.get(missing)` returns `none` but is typed `V`; pre-existing.

## Next Exact Action

Push `rewrite/v3-rust` (fast-forward, 9 commits ahead) only if the human
authorizes publication, then observe GitHub CI. Otherwise the feature is
locally complete and verified; CI remains UNVERIFIED.

## Do Not Reopen

- The generic-map feature: implemented, both adversarial passes satisfied.
- Completed conformance/hardening without a reproduced regression.
- AUDIT-3 without the exact approval token.
- The nine SPEC GAPs as "bugs".
- Historical runtime artifacts; never overwrite them.
