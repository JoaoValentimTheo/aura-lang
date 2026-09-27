# Pre-Modules Audit — Findings and Fixes

**Scope.** A complete end-to-end audit of the Aura core before the
filesystem-based module system (`mod.aura`). Method: source inspection plus
adversarial execution on both substrates (native CLI and the committed
WebAssembly artifact) and the native/WASM differential corpus.

**Baseline.** `rewrite/v3-rust`, `HEAD` = `7260ce2` at audit start. 597 tests
green, 13 CI jobs green, published runtime `0.0.2` and development runtime
`0.0.2-dev.20` both with **zero imports**.

**Evidence classes.** CONFIRMED = reproduced with an executable path;
LIKELY = strong evidence, one condition unverified; HYPOTHESIS = flagged for
investigation.

**Instrumentation used.** Native CLI (`aura run/check/eval/repl`), the
playground native harness (`aura-playground-native`), the loader path
(`playground/web/runtime.mjs`), the committed wasm artifacts, a native/WASM
differential sweep harness, format-spec and lexer/parser fuzzers, manifest
hash verification, and manual source inspection. Scratch harnesses were
removed after use.

---

## 1. Confirmed defects

### AUDIT-1 — Host panic and WASM trap on a large f-string precision (CRITICAL)

* **Class:** CONFIRMED. Reachable from a syntactically valid, checker-passing
  program; produces a Rust panic (native) and a WASM trap.
* **Reproducer:**
  ```aura
  fn main() { print(f"{1.5:.65536}") }
  ```
* **Observed (before fix):**
  * native: `panicked at src/run/mod.rs:1845: Formatting argument out of range`,
    surfaced as `E4999` (INTERNAL) — the process is reported as a bug in Aura;
  * WebAssembly: `RuntimeError: unreachable` (a trap, not a diagnostic);
  * the frozen `0.0.2` release is unaffected (it predates format specs and
    returns `E1006`), so the exposure is the current development line.
* **Root cause:** `Interp::format_value` passes a parser-controlled `usize`
  precision directly to Rust's `format!("{f:.p$}")` / `format!("{:.*}", …)`.
  The Rust formatter represents width and precision in a `u16`, and aborts the
  formatting machinery above `u16::MAX` (`core::fmt::rt::Argument::from_usize`
  panics). The Aura checker does not bound the precision (there is no rule),
  so the value reaches the formatter. This violates `LANGUAGE_SPEC.md` §31.5
  ("No syntactically valid, well-formed program may cause a host panic …") and
  §31.2's "accepted identically on every substrate".
* **Severity:** CRITICAL — host panic/trap from valid input, and a
  native/WASM divergence (native `E4999` vs wasm trap).
* **Fix:** introduced `MAX_FORMAT_PRECISION = u16::MAX` in `src/run/mod.rs` and
  rejected a larger precision with a stable `E4013` in `format_value`, before
  the value reaches the formatter. Applied to every float presentation type.
* **Regression:** `tests/regressions.rs::audit_precision_above_u16_max_is_bounded`;
  differential cases `fstring precision at u16 max / over u16 max / far over /
  far over typed`.
* **Validation:** native now returns `E4013`; wasm now returns diagnostic
  `4013` (no trap); `65535` still formats; full suite green.

### AUDIT-2 — Unbounded f-string width: memory amplification / WASM trap (HIGH)

* **Class:** CONFIRMED. Same program class as AUDIT-1 (valid, checker-passing).
* **Reproducer:**
  ```aura
  fn main() { print(len(f"{1:2000000000}")) }
  ```
* **Observed (before fix):**
  * native: attempted the allocation (≈600 MB–1 GB RSS for the `1e9` case;
    `2e9`+ risks abort/OOM);
  * WebAssembly: `RuntimeError: unreachable` for width ≥ ~2e9 (memory
    exhaustion trap).
  * A width of `100000000` succeeded on both, i.e. a single expression can
    force a ~100 MB materialized string — an uncontrolled amplification.
* **Root cause:** `format_value` pads to `spec.width` with
  `String::with_capacity(width)` and `repeat_n(fill, pad)` with no upper bound;
  the parser accepts an arbitrary `usize`.
* **Severity:** HIGH — resource-exhaustion / substrate divergence on valid
  input; not memory-unsafe.
* **Fix:** introduced `MAX_FORMAT_WIDTH = 10_000_000` (matching the §31.4
  range-materialization cap) and rejected a larger width with `E4013` before
  allocation.
* **Regression:** `tests/regressions.rs::audit_huge_format_width_is_bounded`;
  differential cases `fstring width at bound / over bound / far over`.
* **Validation:** width `10_000_000` accepted, `10_000_001`+ is `E4013` on both
  substrates.

### AUDIT-3 — Type nesting escapes the semantic AST limit; native/WASM divergence (MEDIUM)

* **Class:** CONFIRMED divergence; the "limit escape" is CONFIRMED by source
  inspection.
* **Reproducer** (deep generic type; declaration only, never called):
  ```python
  t = "int"
  for _ in range(1000): t = f"Box<{t}>"
  src = f"struct Box<T> {{ value: T }}\nfn f(x: {t}) -> int {{ return 1 }}\nfn main() {{ print(1) }}"
  ```
* **Observed:**
  * native accepts type-annotation depth up to 2047 and reports `E1015` at
    2048 (the native parser recursion backstop);
  * WebAssembly reports `E1015` at 768 (the wasm parser recursion backstop);
  * both are well past the semantic limit of 256 (§31.1), so the **same valid
    program** is accepted natively but rejected on WebAssembly.
* **Root cause:** the semantic depth enforcer `enforce_depth` /
  `check_expr_depth` (`src/parse/mod.rs`) descends expressions and statements
  only; it never visits `TypeExpr` nodes on annotations, fields, parameters,
  return types, variant payloads, or type arguments. Type nesting is therefore
  governed solely by the substrate-calibrated parser backstop (§31.2), which
  the spec explicitly says must not make the semantic limit substrate-dependent
  (§31.2/§31.5).
* **Severity:** MEDIUM — a native/WASM acceptance divergence; no host failure
  (depth is bounded before any stack trap), so it is not a crash.
* **Status:** **reported, not changed.** Counting type nodes toward the
  semantic budget, or counting them separately, is a *semantic* change
  (it would reject programs the current frozen line accepts, on both
  substrates). It requires an explicit spec decision, so it is left as a
  finding with a `§31.5` safety regression
  (`tests/regressions.rs::audit_deep_type_annotation_never_host_fails`) that
  locks the no-host-failure invariant without asserting an acceptance
  threshold.

---

## 2. Documentation divergences

### DOC-1 — Mutable ordinary parameters are undocumented in the grammar and §16.4

* **Class:** CONFIRMED divergence (implementation vs normative text).
* The parser accepts `mut` on an ordinary parameter (`fn f(mut x: int)`) and
  grants in-body mutation capability (`src/parse/mod.rs`, `Parser::params`).
  It is an intentional, tested feature
  (`tests/mutation.rs:163`, `tests/parser.rs:92`), and §15.4 and §16.6 refer to
  "an ordinary `mut` parameter".
* But `docs/grammar.md` declares `param = IDENT [ ":" type ]` (no `mut`) and
  §16.4 says flatly "Parameters are immutable bindings in the function's
  scope", with no mention of `mut`.
* **Severity:** LOW (documentation). The implementation is the source of truth
  per the audit protocol; the text is stale.
* **Proposed fix:** add `param = [ "mut" ] IDENT [ ":" type ]` to `grammar.md`
  and `LANGUAGE_SPEC.md` §16.4/§15.4, and state that `mut x` grants mutable
  capability over `x` for the body (as §15.4 already says for lambdas).
  Not applied in this pass to keep the change set to host-safety fixes;
  flagged for a doc decision.

### DOC-2 — `build.rs` comment names a stale parser budget

* **Class:** CONFIRMED (comment only). `playground/runtime/build.rs` says the
  backstop is "1024 frames on wasm", while `parse_recursion_budget` returns
  **768** (the comment's own last paragraph was updated; its first line was
  not). Cosmetic, but misleading. Proposed: correct `1024` → `768`.

---

## 3. Areas verified sound (no defect)

* **Sandbox / zero-imports (CRITICAL property).** Both committed artifacts
  (`0.0.2`, `0.0.2-dev.20`, and the new `0.0.2-dev.21`) declare **0 imports**,
  verified by parsing the wasm import section. `Host` has no OS authority in
  `LimitedHost`/`BrowserHost`; `StdHost` is `cfg`-gated to native; the stdlib
  reaches the OS only through the `Host` trait. The loader
  (`playground/web/runtime.mjs`) refuses artifacts with imports before
  instantiation, and the published and dev manifests' SHA-256 and byte counts
  match the artifacts on disk. No path (builtin, error, panic, FFI, build
  script) was found that can add an import.
* **Python boundary.** `py` is default-on natively but disabled for the wasm
  crate (`default-features = false`); on wasm every `py_*` call is `E5002`.
  Native exposes the intended Python escape hatch (a deliberate, documented
  capability of the native host, not a cross-substrate leak).
* **Host capability semantics.** Native: real filesystem/clock/sleep/args;
  write→read round-trips; a nonexistent file is `none`. WASM:
  `read_file`/`write_file`/`time_now`/`time_unix`/`sleep_ms`/`py_*` are all
  `E5002`; `args` is `[]`; `read_line` is `none`. No `E4020`/`E5002`
  confusion observed.
* **Arithmetic (debug vs release).** All int operators use `checked_*`;
  overflow, `i64::MIN / -1`, `%`, `^`, and shift-count bounds yield `E4013`
  identically in debug and release (no `overflow-checks` divergence).
* **Resource limits.** Call frames: 510 ok / 511+ `E4011`, identical on both
  substrates. AST expression nesting via grouping: over-limit is `E1015` on
  both. Range iteration is lazy (`break` on a `10^12` range does not
  materialize). Value display/JSON depth is bounded (512) with `…`/`null`.
  Equality is a cycle-safe iterative worklist with a visited-*pair* set and an
  `Rc::ptr_eq` identity shortcut. `list↔list`, `list↔map`, `struct↔list`,
  `map↔struct`, `struct↔struct`, and enum-payload *simple* cycles terminate in
  display, equality, and JSON. **Correction (AUDIT-5):** this first-round
  verification established simple-cycle termination only; it did **not**
  establish safety against shared substructure with fan-out ≥ 2 and
  combinatorial re-traversal, which AUDIT-4 later exposed in display/JSON and
  AUDIT-5 in the Python bridge. See §8.2 and §8.6.
* **Checker ↔ runtime registry.** Every registered builtin has a signature;
  every signature method resolves at runtime; method arity/type mismatches
  (`x.upper(1)`, `x.contains()`, `x.replace("a")`, …) produce the **same**
  code in `check` and `run`.
* **Overload resolution.** Deterministic (scored, index-stable, no HashMap
  order); ties are `Ambiguous` (`E3001`); concrete beats generic; a
  same-parameter-type redeclaration is `E2007`; return type and `mut self` are
  not identity dimensions; alpha-equivalent generics (`f<T>`/`f<U>`) are the
  same overload.
* **Mutability capability.** `push`/`pop`/`remove`/`mut self` on an immutable
  binding are `E2001`; closures capture by reference and enforce capability;
  union receivers are conservative; `const` and top-level `let` cannot be
  shadowed (`E2007`); `let` shadowing takes the initializer from the previous
  binding.
* **Visibility.** Private fields/methods/structs are `E2018`/`E3002` across
  module boundaries; overload visibility is per overload.
* **Errors.** Only `throw` is catchable; `1/0`, index-out-of-range, and other
  runtime diagnostics propagate fatally and are not caught; `finally` runs and
  a control signal in `finally` overrides the pending outcome; nested
  try/catch and cross-function `throw` behave per §14.
* **Unicode / encoding.** Combining characters, emoji, 4-byte UTF-8, BOM,
  CRLF, and isolated CR are handled without panic; `line_col` clamps and slices
  on `char_indices` boundaries. Fuzzing (`~9,000` random token-soup and
  valid-program inputs, plus format-spec and method fuzzers) produced **no**
  panic other than AUDIT-1.

---

## 4. Confirmed-clean claims vs. prose

* "The checker and runtime consult the same signature registry" — verified for
  arity, types, method existence, and return types; no divergent table found.
* "Zero-import wasm" — verified empirically on all committed artifacts.
* "Immutable manifest with SHA-256" — verified: the manifest matches disk, and
  `build.mjs --check` fails on drift (exercised in CI).
* **Gap (LIKELY→CONFIRMED):** the loader does **not** verify the manifest's
  SHA-256 at load time; it fetches `artifactUrl` and validates only the ABI
  version. The hash protects *immutability at build time and in the repo*, not
  *integrity at runtime*. For a same-origin static site this is a hardening
  opportunity (the browser could recompute SHA-256 via `crypto.subtle.digest`
  and compare to the manifest before instantiation), not an exploitable hole.
  Classified LIKELY-hardening, explicitly **not** presented as a vulnerability.

---

## 5. Changes made in this pass

Host-safety fixes only; no semantic change to the frozen language line.

| File | Change |
|---|---|
| `src/run/mod.rs` | `MAX_FORMAT_PRECISION`, `MAX_FORMAT_WIDTH`; `format_value` rejects an over-limit precision/width with `E4013` before formatting or allocating |
| `tests/regressions.rs` | `audit_precision_above_u16_max_is_bounded`, `audit_huge_format_width_is_bounded`, `audit_deep_type_annotation_never_host_fails` |
| `playground/tests/node/differential.test.mjs` | 7 native/WASM parity cases for the format bounds |
| `docs/LANGUAGE_SPEC.md` | new §31.7 (format-output bounds) |
| `playground/runtime/Cargo.{toml,lock}`, `manifest.json`, `browser.test.mjs`, `website/pages/{playground,runtime}.mjs` | development runtime advanced `0.0.2-dev.20` → `0.0.2-dev.21` (immutable release `0.0.2` and all prior artifacts preserved) |

**Results:** native suite 600 green (was 597), clippy clean, fmt clean, wasm
runtime builds with 0 imports, manifest hash verified, playground suites
manifest 26 / ABI 67 / differential 186 / browser 50 / worker 12 / cache 7 —
all 0 failed.

---

## 6. Open items for a spec decision (not silently changed)

1. **AUDIT-3** — count type nodes toward the semantic 256-node limit (or give
   them their own substrate-independent bound) so the semantic limit is
   substrate-independent, as §31.2/§31.5 require.
2. **DOC-1** — document `mut` ordinary parameters in `grammar.md` and §16.4.
3. **DOC-2** — correct the stale `build.rs` budget comment (1024 → 768).
4. **Hash-at-load (hardening)** — optionally verify the artifact SHA-256 in the
   loader before instantiation.

---

## 7. Follow-ups (reviewer-requested)

> **Artifact note.** This follow-up pass edited comments in `src/parse/mod.rs`
> and `playground/runtime/build.rs` (DOC-2), which shifts embedded panic
> line-numbers and therefore the built wasm bytes. The development runtime was
> advanced `0.0.2-dev.21` → **`0.0.2-dev.22`** (its bytes reproduce exactly
> from the current source); `0.0.2-dev.21` and every earlier artifact remain on
> disk, byte-identical. The frozen `0.0.2` release is unchanged (verified by
> checksum).

### 7.1 AUDIT-1 severity: reclassified to CONFIRMED HIGH

The reviewer challenged CRITICAL unless (a) the native panic hard-aborts the
host process with no diagnostic and cannot be contained by an embedding
library, or (b) the WASM trap corrupts state that outlives the single run in
the same Worker. Both were tested directly, on a scratch build of the *pre-fix*
code (the fix was temporarily reverted in a throwaway copy) so the panic could
actually be produced:

* **(a) FALSE.** Native interpreter execution is wrapped in
  `on_execution_stack` (`src/lib.rs`), which spawns a worker thread and
  `join`s it. A panic therefore unwinds that thread and is converted to a
  structured `E4999` diagnostic on **every** entry point — CLI `run`,
  CLI `eval`, REPL, and the library. Crucially, an embedding host survives:
  a test binary calling `aura::run_source` twice observed the first call
  return `Err(E4999)` and the host process stay alive to run the second
  successfully. There is no process abort and no lost diagnostic.
* **(b) FALSE.** With the pre-fix artifact, a trap (`unreachable`) on the
  same `AuraRuntime` instance left all subsequent runs on that instance
  correct: stdout, diagnostic codes (`E4011`, `E4019`, `E4007`), and
  resource counters were unchanged, across repeated precision traps and a
  memory-exhaustion trap, and under a simulated Worker `postMessage`
  protocol. No state leaked.

Because neither condition holds, **AUDIT-1 is CONFIRMED HIGH**, not CRITICAL:
a real host panic/trap from valid input and a native/WASM divergence, but
contained to a single invocation with a structured diagnostic and no
cross-invocation or host-process damage. The fix (the precision bound) stands
unchanged; a `catch_unwind` boundary is **not** required, because
`on_execution_stack` already provides exactly that containment on the native
path.

### 7.2 Load-time SHA-256 verification — implemented

Previously the manifest hash was checked on disk (`manifest.test.mjs`) and at
build time (`build.mjs --check`) but **not** by the loader before
instantiation. Now `AuraRuntime.fromBytes(bytes, name, { expectedSha256 })`
hashes the fetched bytes and throws a structured `RuntimeIntegrityError`
(`code: "RUNTIME_INTEGRITY"`) on mismatch, *before* `WebAssembly.compile` /
`instantiate`. The worker fetches the manifest hash transitively
(`app.js` passes `entry.sha256` → `worker.js` → `fromBytes`), and the UI
surfaces the failure under the documented `E4999` code with an explicit
"integrity check failed" message rather than raw internals. (No undocumented
code is invented; the structured `RUNTIME_INTEGRITY` code remains on the
worker message for programmatic consumers.)

Crypto: `crypto.subtle.digest("SHA-256", …)` where available, with a vendored
pure-JS SHA-256 fallback for a non-secure context (both are tested to agree
byte-for-byte). The digest is computed on **every** verified load; only the
compiled instance is cached, keyed by the verified hash, so verification can
never be skipped.

Tests: `playground/tests/node/integrity.test.mjs` (27 checks — correct load,
bit-flipped / truncated / empty / stale-hash / swapped-version rejection with
structured codes, both channels, fallback-vs-platform digest equality) plus a
browser end-to-end test that serves a corrupted artifact via request
interception and requires the UI to refuse it before execution
(`browser.test.mjs`, now 53). Performance: the dev artifact is ≈1.6 MB; a
one-shot SHA-256 over that is sub-millisecond to low-single-digit ms in the
browser, negligible against fetch + compile. The verified-instance cache means
repeat runs in the same page reuse the compiled module without recompiling
(the digest itself is always recomputed).

### 7.3 AUDIT-3 decision package — see the follow-up response

The type-nesting ceilings were reconfirmed at the boundary (native
first-rejects at 2048, wasm at 768; both structured `E1015`, no host failure),
the deepest type nesting in any real program was measured at **0** (Aura
sources) / **3** (anywhere, in Rust type tables in prose), and the two options
are presented for human decision. Not implemented.

### 7.4 DOC-1 / DOC-2 — corrected

* `docs/grammar.md` and `website/content/reference-grammar.md`:
  `param = [ "mut" ] IDENT [ ":" type ]`.
* `docs/LANGUAGE_SPEC.md` §16.4: new "`mut` parameters" normative rule.
* `tests/grammar.rs`: two `mut`-parameter samples lock the documented
  production against the parser.
* `playground/runtime/build.rs`: comment now states the actual **768** budget
  and describes the reserved stack accurately. A *second* stale comment was
  found in `parse_recursion_budget` claiming the wasm stack "cannot be enlarged
  by a linker flag" (it can, via `build.rs`); corrected.

### 7.5 TypeExpr nesting differential sweep — permanent

`differential.test.mjs` now runs a 37-depth TypeExpr sweep (every 64 levels
from 0 to 2080, plus N-1/N/N+1 at both ceilings) as part of the standard
differential job (`run-all.mjs`, exercised in CI). It asserts: no host failure
at any depth on either substrate; every rejection is `E1015`; the ceilings are
exactly 2048 (native) and 768 (wasm); and the divergence shape is exactly
"accept/accept below 768, native-accept/wasm-reject in 768..2047,
reject/reject at ≥2048". No separate host-level finding was observed below the
native stack limit.


---

## 8. Hardening round (permanent harness)

### 8.1 Test-count reconciliation

The "600" figure was the count of executable Rust `#[test]` cases reported by
the canonical command `cargo test --locked --all-targets --all-features`
(summed from each `test result:` line). The follow-up round's additions did not
change it because they were **not new `#[test]` functions**:

* `tests/grammar.rs`: the two `mut`-parameter cases were added to the existing
  `GRAMMAR_SAMPLES` array inside `grammar_samples_parse_and_run`, which is one
  `#[test]`; the file's `#[test]` count stayed 6.
* `+3 browser`, `+4 differential`, `+27 integrity` are **Node/playground**
  checks, not Rust tests, so they never were part of the Rust 600.

The Node/playground totals are a separate addition. The combined number is
defined explicitly in the round report.

### 8.2 AUDIT-4 — display/JSON node-budget (HIGH)

**Class:** CONFIRMED. **Severity:** HIGH (host-level non-termination, both
substrates; no memory unsafety).

**Reproducer:**
```aura
fn main() { let mut c = []
 push(c, c)
 push(c, c)
 print(len(to_string(c))) }
```
`repr` and `to_json_depth` bounded recursion *depth* (`MAX_VALUE_DEPTH = 512`)
but not total nodes. A value with reference fan-out ≥ 2 — a cycle reachable
from more than one position, or a shared subvalue — is re-traversed at every
occurrence, so the render expands exponentially and **never terminates** on
either substrate (native spins; the wasm artifact spins too, so no trap, no
diagnostic, no return). This violates §31.5 (no host failure) and §31.6
(display/JSON terminate).

**Fix:** a shared total-node render budget (`MAX_VALUE_NODES`, 1,000,000,
`RenderBudget`) consumed once per visited node in `Value::repr` and
`to_json_depth`, alongside the depth bound. Beyond it the remainder elides
(`…` / `null`), exactly like the depth bound. A 5,000-element list and a
40,000-deep value still render fully; the worst-case adversarial render is
≈0.65 s.

**Regression:** `tests/regressions.rs::audit4_*` (3 tests); corpus
`tests/corpus/cycles/*` (9 fixtures); 4 differential cases; property
`cyclic_graphs_render_and_compare_safely`.

**Artifact:** the runtime change alters wasm bytes, so the dev channel advanced
`0.0.2-dev.22` → `0.0.2-dev.23` (dev.22 preserved byte-for-byte).

### 8.3 Permanent fuzz targets

Four libFuzzer targets under `fuzz/fuzz_targets/` — `lexer` (arbitrary bytes),
`parser` (arbitrary source bytes), `checker` (parser-accepted, resolver-walked
modules), `runtime` (grammar-aware generated, checker-passing programs with a
cycle/aliasing surface). Seed corpora under `fuzz/corpus/<target>/`. Bounded
PR smoke (60 s/target, `ci.yml` `fuzz` job) and nightly 30 min/target
(`fuzz-nightly.yml`). No crashes found in the smoke runs.

### 8.4 Property tests and permanent corpus

`tests/property_hardening.rs` (five properties, `proptest` 1.5 — already a
dev-dependency) and `tests/corpus/` (executed by `tests/corpus.rs` with a
bidirectional inventory). Both run in the existing `test` job.

### 8.5 AUDIT-5 — Python bridge fan-out/cycle blow-up (AUDIT round 5)

**Class:** CONFIRMED. **Severity:** HIGH (host-level non-termination, native;
default-on `py` feature). The surface is the native-only Python FFI, so it is
not a cross-substrate divergence; on wasm the `py_*` calls are `E5002`.

**Reproducers:**
```aura
fn main() { let c = py_eval("(lambda c: (c.append(c), c.append(c), c)[2])([])")
 print(len(to_string(c))) }
```
and, in the other direction,
```aura
fn grow(xs, n) {
 let mut cur = xs
 let mut i = 0
 while i < n { cur = [cur, cur]
 i = i + 1 }
 return cur
}
fn main() { print(len(py_call("builtins", "repr", grow([1], 40)))) }
```
`bridge::to_value_depth` (Python → Aura) and `to_py_depth` (Aura → Python)
bounded only *depth* (`MAX_VALUE_DEPTH`), not total nodes; and neither tracked
a container already on the current path. A Python object that references itself
with fan-out ≥ 2, or an Aura value with shared subvalues, re-expands
exponentially and never returns.

**Fix:** both directions now also carry a total-node budget (`MAX_PY_NODES` /
`MAX_PY_SOURCE_NODES`, 1,000,000) and a path-based identity set. A cycle is
rejected with a structured `E5002`; a shared (DAG) value is converted again,
bounded by the budget. No partial Python object is handed back.

**Regression:** `tests/python.rs` (`python_cyclic_container_is_rejected_not_hung`,
`aura_value_into_python_terminates`, `python_acyclic_values_still_round_trip`).

**Artifact:** native-only source change; it does not alter any wasm build, so
the development channel bytes are unchanged (see the round report).

### 8.6 Corrections to the previous round (AUDIT-5 review)

* **Cycle-safety claim was broader than the evidence.** §7.8 of the earlier
  report summarized "cycle-safe display/equality/JSON/teardown across all
  container combinations". That verification established *simple cycle
  termination*, not safety against **shared substructure + fan-out ≥ 2 +
  combinatorial re-traversal**. AUDIT-4 exposed exactly that missing
  dimension. This is recorded as a correction to the scope of the original
  claim, not as a new production bug in the simple-cycle cases (which were
  and remain correct).
* **Equality was safe, but under-tested.** `Value::equals` is a *separate*
  algorithm from display/JSON: an iterative worklist with a visited-pair set
  and an `Rc::ptr_eq` identity shortcut. The earlier property compared only
  `c == c`, which takes the identity shortcut and never exercises the
  visited-pair set. The property now compares a cyclic graph against an
  independently constructed, structurally identical graph, so the dangerous
  path is actually tested.
* **Fuzz generator under-represented shared fan-out.** The runtime generator's
  cycle branch produced mostly single self-cycles; simple-cycle coverage is not
  combinatorial fan-out coverage. The generator now deliberately emits
  `push(c, c)` twice, `[c, c]`, and repeated `x = [x, x]`, and those shapes are
  pinned as committed corpus fixtures.
* **"No-python Rust set: 604" was a test-count artifact.** Python/PyO3 is a
  real, default-on feature (`py`), with a live bridge (`src/bridge/mod.rs`),
  tests (`tests/python.rs`), and a CI job (`no-python`) that proves the pure
  build links no CPython. The `604` figure was the count of executable Rust
  `#[test]` functions with `py` **disabled** (610 with `py`, 6 of which require
  it); it was never a statement that no Python support exists.
* **LeakSanitizer incident.** The `crash-*`/`leak-*` filename that was
  investigated embeds libFuzzer's SHA-1 of the crashing input; for the empty
  input that is `da39a3ee…`. The diagnosis (exit-time LSan report for the
  documented, memory-safe `Rc`-cycle non-reclamation, not a real crash) was
  correct, but SHA-1 there is **libFuzzer's artifact-naming convention**, not
  Aura's SHA-256 artifact-integrity hash. Sanitizer scope is now explicit:
  LSan is enabled for `lexer`/`parser`/`checker` and scoped off only for
  `runtime` (see `fuzz/README.md`).

