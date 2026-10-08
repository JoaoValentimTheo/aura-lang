# AURA LANGUAGE CONFORMANCE PASS — PHASE 12

## 1. Scope

Build, artifacts, website, versioning, and CI: proving the repository
deliverables correspond to source.

Starting HEAD: `04621a2`.

## 2. Dev Version Discipline

* The Phase-2/3 production fixes (resolver and checker) **change the wasm
  artifact**, so exactly one new development identity was created: **dev.25**.
* dev.24 was not mutated; it remains historical and byte-identical.
* dev.23 and the frozen 0.0.2 were not touched.

## 3. Artifact Accounting

| Artifact | Path | Bytes | SHA-256 |
|---|---|---|---|
| Frozen 0.0.2 | `playground/runtimes/0.0.2/aura_playground_runtime.wasm` | 1,366,621 | `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed` |
| dev.23 | `playground/runtimes/0.0.2-dev.23/aura_playground_runtime.wasm` | 1,604,958 | `71072150e67384120c63e22d6176f3683110735b84f74723bea315f79778a528` |
| dev.24 | `playground/runtimes/0.0.2-dev.24/aura_playground_runtime.wasm` | 1,604,902 | `16882fe60d7fa52f9e204b3841cc59764c79f50c1068f33b7c3e3350f1adfd39` |
| dev.25 | `playground/runtimes/0.0.2-dev.25/aura_playground_runtime.wasm` | 1,612,018 | `a9462510c963beb3b721c9f1da680f39722bfce6906eb73ad9ba304fe344a406` |
| **dev.26** | `playground/runtimes/0.0.2-dev.26/aura_playground_runtime.wasm` | **1,612,422** | `1e91a070bfd6762d1e2ec6fb504e1a362f2f64450b963adbaadd14731e316185` |

dev.25 was established when the Phase-2/3 resolver and checker fixes changed the
artifact; it became historical on push. The later `CONF-RESOURCE-1` /
`CONF-GENERIC-1` fixes again changed the bytes, so **dev.26** is the current
development identity. Neither dev.25 nor dev.26 overwrites an earlier artifact.

Verification for dev.26:

* Rebuilt from the current source with
  `cargo build --locked --manifest-path playground/runtime/Cargo.toml --release
  --target wasm32-unknown-unknown` → byte-identical hash (reproducible).
* WASM imports: **0**.
* Manifest entry: `current = 0.0.2-dev.26`, `runtime_version = 0.0.2-dev.26`,
  `host_abi_version = 1`, `channel = development`.
* Website staged copy: `website/dist/playground/runtimes/0.0.2-dev.26/…` has
  the same SHA-256.
* `node playground/build.mjs --check` → "manifest matches 3 version(s)".
* Runtime crate version advanced `0.0.2-dev.25 → 0.0.2-dev.26` in
  `playground/runtime/Cargo.toml` and `Cargo.lock`.

## 4. Website

Only version references required by the runtime advance were updated:
`website/pages/runtime.mjs`, `website/pages/playground.mjs`, and the
`playground/tests/node/browser.test.mjs` selector assertions. No appearance
redesign. `node website/build.mjs` succeeded (39 pages) and the website suite
(generated examples, links, base paths, browser, accessibility, reused
playground suite) passed; the browser test derives the development id from the
manifest where applicable.

## 5. Build Matrix

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo test --locked --all-targets --all-features` | 642 passed, 0 failed |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | pass |
| `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` | 632 passed, 0 failed |
| `node playground/tests/node/run-all.mjs` | manifest 26, ABI 67, integrity 27, differential 195, syntax 43, browser 53, worker 12, cache 7 — 0 failed |
| `node playground/build.mjs --check` | pass |
| `node website/tests/run-all.mjs` | pass |
| WASM release build | reproducible, 0 imports |

Linux/Windows/macOS/MSRV jobs run in CI; local host is macOS.

## 6. CI

Push and observed GitHub CI results are recorded in the final program report.
If publication is not performed in this session, CI is reported as
**UNVERIFIED** with the exact pending command.

## 7. Findings

No artifact defect. One dev version was advanced because production runtime
bytes changed; historical artifacts are intact.
