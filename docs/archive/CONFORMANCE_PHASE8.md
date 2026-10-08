# AURA LANGUAGE CONFORMANCE PASS — PHASE 8

## 1. Scope

Native / WASM / browser platform conformance: generated differential programs,
the byte boundary, WASM imports, resource limits, and the browser/worker/cache
surfaces.

Starting HEAD: `73c7b63`.

## 2. Surfaces

Rust library, CLI, native playground harness, WASM runtime, browser runtime,
worker, and the cache/integrity layer.

## 3. Generated Differential Parity

`playground/tests/node/differential.test.mjs` generates fresh programs at test
time (not a static corpus) and compares native vs WASM on **status, stdout,
diagnostic code, diagnostic message, line, and column**, normalizing the
structured result. Result: **195 passed, 0 failed**, including the invalid-UTF-8
byte-boundary cases. The generator is deterministic; it excludes TypeExpr-heavy
AUDIT-3 inputs and intentionally host-specific features.

## 4. Byte Boundary

Strict UTF-8 decoding (`lex::decode_source`, Phase 1) is shared by CLI
file/stdin and the Playground `execute_bytes` path, so malformed bytes yield
`E1001` identically on native and WASM. Browser JS strings are `TextEncoder`
encoded before that boundary. The Node syntax suite confirms this with explicit
byte buffers.

## 5. WASM Imports

`playground/tests/node/abi.test.mjs` asserts `WebAssembly.Module.imports(...)
.length === 0`. Zero imports is preserved; no import was added by any Phase
change.

## 6. Resource Limits

| Limit | Native | WASM | Spec |
|---|---|---|---|
| Parser AST depth | 256 (`E1015`) | 256 | §31 |
| Expression nesting | 256 | 256 | §31 |
| Call frames | 512 (`E4011`) | 512 | §31.3 |
| Value render depth | 512 | 512 | §31.6 |
| TypeExpr nesting | **2048 native / 768 WASM** | | DECISION-PENDING (AUDIT-3) |

The TypeExpr ceilings remain deliberately unequal and unchanged; the
differential suite reports `native ceiling=2048, wasm ceiling=768, host
failures=0`.

## 7. Browser / Worker / Cache

* Browser: 53 passed, 0 failed — worker lifecycle, cancellation, repeated
  runtime use after diagnostics, malformed runtime bytes, and wrong expected
  SHA.
* Worker: 12 passed, 0 failed.
* Cache: 7 passed, 0 failed — integrity failure and manifest checks.

## 8. Integrity

The frozen `0.0.2` and historical `dev.23` artifacts are byte-identical
(Phase 12 verifies). The playground SHA-256 integrity architecture was not
modified.

## 9. Findings

**No new defects.** Native/WASM observable behavior agrees wherever the
contract requires it; the only deliberate divergence is the AUDIT-3 TypeExpr
ceiling, which is DECISION-PENDING and untouched.

## 10. Tests

`node playground/tests/node/run-all.mjs` — manifest 26, ABI 67, integrity 27,
differential 195, syntax 43, browser 53, worker 12, cache 7; all pass. The
WASM artifact is rebuilt and re-verified in Phase 12.
