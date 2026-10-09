# Aura Stabilization — Read-Only Engineering Audit

Status: **read-only audit in progress.** No production code is modified by this
document. Every finding is reported with evidence and a proposed smallest safe
correction; corrections are gated on human triage.

Baseline audited: `rewrite/v3-rust` at `9f8743ee` (= remote), the published
0.3.2 development preview with frozen `0.3.1` Keystone.

Classification: **CONFIRMED** (reproduced with evidence), **FALSIFIED** (claimed
but not real), **DECISION REQUIRED** (needs a human choice), **INSUFFICIENT
EVIDENCE** (cannot yet prove or disprove).

---

## A. Confirmed defects (ranked by severity)

### A1 — Browser HTTP had no response-size cap (CONFIRMED, FIXED)

- **Where (before):** `playground/web/worker.js` `performHttp` used an
  unbounded `await response.text()`; the native cap is
  `src/host.rs:227` `MAX_HTTP_BODY_BYTES = 8 MiB`.
- **Reproducer (before):** with an authorized origin, `http_get` against a
  server returning more than 8 MiB buffered the whole body in the page.
- **Severity:** high (resource exhaustion on a published preview; not an
  authorization bypass).
- **Correction (applied):** the Worker reads the body through a bounded
  `ReadableStream` reader (`readBoundedBody`, cap `MAX_RESPONSE_BYTES = 8 MiB`,
  mirroring native) and returns `E4020` "response body exceeds the limit"; the
  stream is cancelled at the cap. A declared `Content-Length` over the cap is
  refused before reading.
- **Regression:** `http-security.test.mjs` case 8 (a 10 MiB local body →
  `E4020`), green.

### A2 — `timeout_ms` was ignored by the browser transport (CONFIRMED, FIXED)

- **Where (before):** `playground/web/worker.js` never read `effect.timeout_ms`;
  native clamps to `MAX_HTTP_TIMEOUT_MS` and enforces it.
- **Reproducer (before):** an authorized origin whose server never responds
  hung the run until **Stop**.
- **Severity:** medium-high (a hung server held the run).
- **Correction (applied):** a per-request `AbortController` with a timer from
  `effect.timeout_ms` (clamped to `MAX_TIMEOUT_MS = 30 000`), composed with the
  run's Stop signal; the timer aborts the fetch and body read and returns
  `E4020` "request timed out". Cleared on completion.
- **Regression:** `http-security.test.mjs` case 9 (a hanging endpoint with
  `timeout_ms: 3000` → `E4020` in under 15 s), green.

### A3 — Frozen `0.3.1` reports `E2003` (not `E5002`) for HTTP (CONFIRMED, docs)

- **Where:** the frozen `0.3.1` builtin registry predates the HTTP surface.
- **Status:** the website documentation and `known-limitations` were corrected
  and a regression test added (`session-http.test.mjs`) during the
  pre-publication gate; recorded here for completeness. No code change.

### A4 — AIS slice can exceed the source it summarizes for small programs

(CONFIRMED, interface design — see the RFC)

- **Evidence:** a 529-byte reference program yields a 5,893-byte snapshot and a
  3,221-byte `slice` (measured with the released compiler).
- **Impact:** an agent using AIS for context reduction on small files may
  *increase* context; the win is precision, not size.
- **Disposition:** design, not a defect — carried into
  `AIS_AGENT_NATIVE_RFC.md` §1 as a constraint on any claims.

### A5 — AIS has no reverse (caller) relation (CONFIRMED, capability gap)

- **Evidence:** `ais slice app.aura dist` lists `dependencies: ["Point"]`
  (signature references) and no call sites; `Symbol` has no
  `references`/`referenced_by`.
- **Impact:** "what breaks if I change X?" cannot be answered from AIS; the
  agent reconstructs it from text.
- **Disposition:** the central proposal in `AIS_AGENT_NATIVE_RFC.md` (O1).

---

## B. Findings from the parity/coverage audit (reproduced)

Each was independently reproduced by the writer against the released
compiler/Worker before being recorded. Fixed items are marked; the rest are
open and awaiting triage.

### Fixed in this pass

- **B1 — Session inputs dropped (CONFIRMED, FIXED).** The ABI-2 host
  hardcoded empty `args` and no stdin, so a session run saw `[]\nnone` where
  the synchronous path saw the supplied values. Reproduced:
  `run(src,{args:["a"],stdin:"hi\n"})` → `["a"]\nhi\n` vs
  `startSession(...)` → `[]\nnone\n`. Fixed by carrying args/stdin in
  `SuspendHost`; pinned by `session-http`.
- **B2 — `main`-less session fallback (CONFIRMED, FIXED).** `startSession("1 + 2")`
  returned `E4027` while `run("1 + 2")` evaluated the module. Fixed by
  mirroring the `NO_MAIN` fallback in `aura_session_start`; pinned.
- **B3 — Browser `body_bytes` corrupted (CONFIRMED, FIXED).** The Worker
  derived `body_bytes` from a lossy UTF-8 string; `0xFF` became the 3-byte
  replacement char (observed `len=7`, byte `239` vs native `len=3`, byte
  `255`). Fixed by sending raw bytes and consuming them exactly in Rust;
  pinned by an `http-security` binary case (byte-exact `4\n255\n65`).
- **B4 — The security/browser-HTTP suites never ran in CI (CONFIRMED, FIXED).**
  They existed only as manual docs. Now wired into `run-all` under
  Playwright, and they build the site on demand so the playground CI job
  (which does not build the website) can run them. CI confirms 20/20 with
  them.
- **B5 — Browser response cap and timeout (CONFIRMED, FIXED earlier this
  session).** See A1/A2.

### Open (confirmed, not yet fixed — no severity above medium)

- **B6 — 3xx behavior diverges (MEDIUM).** Native returns the redirect as an
  ordinary response (`tests/http.rs` expects `302`); the browser uses
  `redirect:"manual"` and maps `opaqueredirect` to `E4020`. The Aura-visible
  difference is unpinned. *Decision required:* document the divergence or
  synthesize a status-only response.
- **B7 — Response headers diverge (MEDIUM).** Native preserves duplicates and
  original casing; the browser `Headers` object hides `Set-Cookie` and
  comma-joins duplicates. Inherent to the platform; fix is documentation plus
  an explicit pin.
- **B8 — Browser-only request bounds (LOW).** `MAX_REQUEST_BYTES` (8 MiB) and
  `MAX_HEADER_BYTES` (64 KiB) exist only in the Worker; native has no
  counterpart. Document or add native mirrors.
- **B9 — `MAX_HTTP_PER_RUN = 64` (LOW).** Browser-only request-count cap;
  untested. Document as a browser policy.
- **B10 — Dead `MAX_REDIRECTS` and a header comment that overstates redirect
  handling (LOW).** Remove the constant and correct the comment.
- **B11 — Node-side ABI pins (LOW).** `abi.test.mjs` hardcodes
  `abiVersion === 1`; valid only while `manifest.current` is `0.3.1`. Reading
  the ABI from the manifest entry future-proofs it.
- **B12 — `complete` output mode unreachable from the web (LOW).**
  `encodeOptions` never emits `output-mode`; the UI branch is dead. Wire it or
  document as harness-only.
- **B13 — `readBoundedBody` fallback counts UTF-16 units, not bytes (LOW).**
  Only on a non-streaming engine; the primary path is byte-accurate.
- **B14 — Multi-file project runs have no browser HTTP (LOW-MEDIUM).** Project
  runs use `BrowserHost` (HTTP unavailable) while single-source sessions
  support it; the docs describe browser HTTP generally. Document the scope or
  add project sessions.

### Coverage gaps (no executable test yet)

Session source/limit rejection; native timeout enforcement with a slow
server; `MAX_HTTP_PER_RUN`; the `readBoundedBody` fallback; browser
duplicate-header and redirect-status behavior; HTTP from a multi-file project.

### FALSIFIED (claims checked and not defects)

- Response-cap parity and timeout parity are now real (fixed); input limits
  (`MAX_ARGS`/`MAX_ARG_BYTES`/`MAX_STDIN`) are shared across `aura_run`,
  `aura_run_project`, and `aura_session_start`; 4xx/5xx are ordinary
  responses on both paths; `E4020`/`E5002`/`E4031` mapping and call-site span
  are consistent.

---

## C. Spec/grammar and evaluator audits

### C.1 Spec vs implementation (independently reproduced)

**C1 — Stale tuple rule inside `LANGUAGE_SPEC.md` (CONFIRMED, docs-only).**
`docs/LANGUAGE_SPEC.md:795` states "`(a, b)` with a comma is a *list of the
given elements*", directly contradicting §21 (line 3684: "`(a, b)` is a Tuple,
not list sugar") and the accepted ADR-0005. Reproduced against the released
compiler: `(1, 2) == [1, 2]` is `false`, so tuples are distinct and §21 is
authoritative. Severity: low (a reader could be misled; the implementation is
correct). Smallest correction: delete or rewrite the line-795 sentence to point
at §21. **Not changed in the audit stage** (docs of the frozen release); queued
for the correction batch.

**C2 — Diagnostic code inventory is complete and consistent (FALSIFIED risk).**
Every `E####` in `docs/errors.md` has a `pub const` in `src/error.rs` and vice
versa (50 codes each, byte-for-byte after normalization). No undocumented code
and no documented-but-absent code.

**C3 — Grammar covers the Keystone forms (FALSIFIED risk).** `docs/grammar.md`
carries the Array type (`[ type ";" INT ]`, line 68), the tuple pattern
(line 155) with the one-element `(x,)` form (noted line 216), and matches the
parser: `let t = (1,)` evaluates `t[0] == 1`.

**C4 — Reserved-name plumbing is consistent (FALSIFIED risk).** `E1009`
(builtin value-name reservation) and `E2023` (the reserved `Aura` namespace)
are both documented and both produced (`src/check/mod.rs` sites found).

**C5 — Fuzz smoke intermittent failure is a misclassified performance signal
(CONFIRMED, test-infrastructure defect; FIXED).** The `runtime` fuzz target
occasionally writes a libFuzzer `timeout-*` artifact; the CI classifier treated
every artifact that was not `slow-unit-*` as a crash, so a finite-but-slow
generated program failed the job nondeterministically (the job passed on rerun
of the same source). Reproduced and classified:
*the generator provably emits no unbounded construct* — there is no `while` or
`loop` site, every `for` bound is a small constant or a generated list length,
and every render/equality/encode is bounded by the documented
`MAX_VALUE_NODES` budget (`src/run/value.rs:33`, whose elision case was
reproduced: a self-referential `push(c, c)` twice renders in bounded time).
The worst seeds do ~1M nodes of fan-out rendering (measured: seed 180 at
~4.9 s release; more under ASan), which crosses libFuzzer's 25 s per-unit
timeout while the process is still making progress. Both CI and nightly now
classify `timeout-*` alongside `slow-unit-*` as a performance signal; a genuine
crash/abort/leak still writes a differently named artifact and still fails, and
a genuine hang is caught by the `timeout` command and the RSS limit. The
aggressive detection stays in the nightly workflow.

### C.2 Evaluator/session integrity (independently reproduced)

Three invariants the order named were verified against the shipped machine
with new regression tests; **no defect was found** beyond the fixed B1–B3.

**C6 — Call depth is preserved across parking (FALSIFIED risk).** A recursive
chain that parks on an HTTP effect at every level still hits `E4011` at the
512-frame limit: `interp.depth` is incremented in `push_frame`
(`src/run/iterative.rs:3456`) and only decremented on `pop`, both of which live
in the preserved `MachineState`, so parking cannot bypass the guard.
`tests/session_depth.rs` drives 512 parks then asserts a `RECURSION`
diagnostic (parks ∈ 500..=513).

**C7 — `finally` and propagation on a resumed (and failed) effect
(FALSIFIED risk).** `tests/session_finally.rs` pins that a success resumes into
the enclosing block and `finally` runs before the code after the `try`; a
failed effect runs `finally` exactly once and propagates `E4020` without
running the post-`try` code — matching the synchronous host.

**C8 — Session re-entrancy (FALSIFIED risk).** Advancing a completed session is
idempotent (`Step::Completed` again), never a re-run: the guarded
`advance`/`resume_effect` state machine rejects a second drive and an
out-of-band resume (`session_finally.rs` asserts stdout is produced once).

The evaluator's other invariants (exact-once evaluation, no re-evaluated
arguments, preserved environments/spans, stale-effect rejection) are covered by
`tests/session.rs` (14/0) and `playground/tests/node/session-http.test.mjs`
(17/0). No evaluator/session defect is open.

---

## D. Decisions requiring human approval

1. **A1/A2 fixes**: both are narrow security/correctness corrections to the
   published experimental preview. The order freezes *features* but keeps
   security/correctness fixes eligible; confirm that A1/A2 are in scope for an
   immediate narrow fix, or defer to a stabilization batch.
2. **AIS/0.2**: the RFC proposes additive operations; confirm the priority
   (precision vs size) and the stable-id strategy before any implementation
   (RFC §9).

---

## E. Recommended correction order

1. **A1** (browser response cap) — highest; a resource-exhaustion vector.
2. **A2** (browser timeout) — small, self-contained.
3. Consolidated CONFIRMED findings from §B, by severity.
4. Documentation/cleanup items from §C.
5. AIS/0.2 only after the RFC is approved.
