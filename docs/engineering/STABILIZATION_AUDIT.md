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

## B. Findings from parallel subsystem audits

*(Consolidated from three independent read-only passes: spec/grammar
consistency; evaluator/session integrity; parity and coverage.)*

_Pending — the three audits are running; this section is filled on their
return and each finding is reproduced by the writer before being accepted._

---

## C. Coverage and technical-debt observations

_To be completed from the parity/coverage audit._

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
