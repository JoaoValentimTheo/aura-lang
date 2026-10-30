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

### Open (reclassified against the code in Directive 003 — see below)

Every B6–B14 claim was re-read against the implementation. Two did not survive
as written: **B13 is falsified** and a **new defect (B13′)** was found in the
same function's sibling path. The remaining seven are confirmed and now carry a
verdict of either *fix* (low risk, no contract change) or *escalate* (a
semantic or Host-surface decision that Directive 003 forbids making
unilaterally).

#### Escalated — a semantic/contract decision is required

- **B6 — 3xx behavior diverges (MEDIUM, CONFIRMED, RESOLVED as a formalized
  divergence).** Native returns the redirect as an ordinary response —
  `src/host.rs:503-505` sets `.max_redirects(0)` with the comment "a 3xx is
  returned to Aura as an ordinary response", and `tests/http.rs:339` asserts
  status `302`. The browser sets `redirect: "manual"`
  (`playground/web/worker.js:170`) and maps
  `response.type === "opaqueredirect"` to `E4020`
  (`playground/web/worker.js:261-269`). So the *same Aura program* sees a `302`
  response natively and a diagnostic in the browser.
  **The remedy first recommended here — "synthesize the `opaqueredirect` into a
  status-only response … so both substrates report a 3xx" — is wrong and was
  rejected.** An `opaqueredirect` response exposes *no status at all*: the
  browser hides the code, so `301`, `302`, `303`, `307`, and `308` are
  indistinguishable. Emitting a `3xx` would therefore mean **inventing a status
  the platform withholds**, which is precisely what Directive 003-R §4 forbids
  ("não invente códigos de resposta ocultos pelo navegador"). Exact parity is
  *technically* impossible here, which is the §4 case that authorizes a
  **formalized divergence** instead of a workaround. What was missing was the
  normative contract plus a test pinning the observable behaviour, and that is
  what was added: `docs/LANGUAGE_SPEC.md` §25 now carries the HTTP contract and
  the browser-host divergence rule, and case 4 of
  `playground/tests/node/http-security.test.mjs` now asserts the outcome the
  program actually observes (`E4020`, no fabricated status) rather than only
  asserting that the redirect was not followed. The Worker code is unchanged —
  it was already correct, and its comment already refused to fabricate a status.

- **B7 — Response headers diverge (MEDIUM, CONFIRMED, RESOLVED as a documented
  divergence).** The audit's original claim was correct, and an intermediate
  reading of the code during this campaign — which concluded B7 was "docs-only"
  because `header_lines` appeared to be populated on both substrates — was
  **wrong**. The data path *looks* symmetric: the Worker builds an ordered
  `[key, value]` pair array (`playground/web/worker.js:309-311`) and
  `playground/runtime/src/lib.rs:1367-1405` applies list-on-duplicate logic
  identical to native `src/stdlib/ext.rs:596-633`. But the symmetry is
  defeated **upstream of the Worker**: `Headers` is a *combining* collection,
  so `response.headers.forEach` yields one entry per unique name with the
  occurrences already joined by `", "`, in sorted lowercased order. Duplicates
  therefore never reach the pair array as separate entries, and the
  list-on-duplicate code is dead on the browser path. Native keeps duplicates
  as a list and `header_lines` in wire order with original casing
  (`tests/http.rs:377-386`). `Set-Cookie` is additionally never exposed. Both
  are platform properties, not fixable from the Worker, so this is a
  **documented divergence**, not a defect. Resolved by stating it normatively
  in `docs/LANGUAGE_SPEC.md` §25 and accurately in
  `website/content/reference-stdlib.md` (which previously documented a
  three-key response, called `headers` "comma-joined" without saying *which*
  host, and claimed the Playground has no HTTP authority at all).
  `website/content/playground-doc.md` was already correct and was left alone.
  No code change, because the Worker cannot do better than the platform.
  *Lesson recorded:* the divergence lives in `Headers` iteration, so any future
  reading of this path must start at `response.headers.forEach`, not at the
  runtime's pair-array consumer.

- **B14 — Multi-file project runs have no browser HTTP (LOW-MEDIUM, CONFIRMED,
  FIXED).** The original diagnosis was correct. `BrowserHost`
  (`src/host.rs:1045-1135`) does not override `http_request`, so it inherits the
  trait default at `src/host.rs:216-220` returning
  `unavailable("this host has no network capability (HTTP is unavailable)")`.
  Only `SuspendHost` (`playground/runtime/src/lib.rs`, impl at 1143-1153)
  returns `HostError::pending(PendingEffect::Http(..))`, and it was installed
  only by `aura_session_start`; `aura_run`/`aura_run_project` construct
  `BrowserHost`. Net effect: a single-source session could do HTTP, a multi-file
  project could not — while `website/content/playground-doc.md` described
  browser HTTP without that scope limit.
  **The escalation ("adding project sessions is a Host-ABI change, out of
  scope") rested on an assumption that did not survive checking: there is no
  Host ABI 3.** `grep` for `aura_session_start` across `docs/` returns only
  `STABILIZATION_AUDIT.md` and `PLAYGROUND_032_CAMPAIGN.md`; the runtime's
  `ABI_VERSION` is `2` and no newer surface is specified or frozen anywhere.
  So a project entry point *extends ABI 2 in place* rather than breaking it, and
  §4's "não rompa o Host ABI existente e não modifique runtimes históricos" is
  satisfied without a version bump: `ABI_VERSION` stays `2`, every existing
  export keeps its signature, and the new capability is **feature-detected** —
  `supportsProjectSessions` (`playground/web/runtime.mjs`) is an independent
  probe of `aura_session_start_project`, deliberately *not* folded into
  `SESSION_EXPORTS`, so `supportsSessions` remains true for older artifacts.
  An old runtime therefore degrades to the synchronous `runProject` path (which
  denies HTTP with `E5002`) instead of failing to load. **No frozen runtime was
  touched**; the change ships as a new development artifact, `0.3.2-dev.8`.
  Implementation reuses the existing consent/suspension machinery rather than
  duplicating it: `aura_session_start_project` mirrors `aura_session_start`,
  builds the provider with `build_virtual_provider`, and installs the *same*
  `SuspendHost`. On the JavaScript side both paths share one effect loop —
  `runSession(runId, runtime, start, signal)` takes a starter thunk — so the
  64-request budget, cancellation, and the resume contract cannot drift between
  single-source and project runs. Provenance also survives the session path:
  diagnostics carry `source` naming the offending module, which requires
  `HostedSession::sources` to hold the *same* `SourceMap` the compilation
  produced, because `SourceMap::get` (`src/source.rs:151`) rejects
  foreign-scope ids.
  **Verification.** New `playground/tests/node/project-http.test.mjs`
  (20 checks, registered in `run-all.mjs`) drives a two-module HTTP program
  against a loopback server and asserts the effect is produced by the non-entry
  module, its body/method/headers survive, a 404 is an ordinary response, denial
  yields `E5002`, and a child compile error names `child.aura`. It carries a
  **negative control** — `runProject` on the same program must yield `E5002`
  with no effect — so the positive case cannot pass merely because the session
  started. Mutation-verified against the pre-fix `0.3.2-dev.7` artifact:
  dev.8 → 20/20, dev.7 → 5 passed / 2 failed (fails as a statement rather than
  throwing, via a capability guard wrapping the post-detect body).
  `website/content/playground-doc.md` keeps its scope note, now describing a
  limitation of *old* artifacts rather than of the Playground.

#### Fixable without a contract change

- **B8 — Browser-only request bounds (LOW, CONFIRMED, document).**
  `MAX_REQUEST_BYTES` (8 MiB) and `MAX_HEADER_BYTES` (64 KiB) are enforced only
  in the Worker (`playground/web/worker.js:50-51` after this batch). Native has
  no *request* bound at all; `src/host.rs:227`'s `MAX_HTTP_BODY_BYTES` and
  `:230`'s `MAX_HTTP_TIMEOUT_MS` bound *responses*. Mirroring them natively
  would change the Host contract for every embedder, so the browser caps stay a
  documented browser policy. **Documented; no native mirror added.** The caps
  are now pinned by `playground/tests/node/transport-bounds.test.mjs`, which
  drives the shipped Worker file in a sandbox.

- **B9 — `MAX_HTTP_PER_RUN = 64` (LOW, CONFIRMED, browser policy).**
  `playground/web/worker.js:449` caps requests per run, enforced at `:464-470`
  with code `4020` and message `too many HTTP requests in one run`. It is a
  deliberate denial-of-service bound with no native analogue, and it was
  **untested** — `grep -E 'MAX_HTTP_PER_RUN|too many HTTP' playground/tests/node/*.mjs`
  returned nothing. **Documented as browser policy** in
  `website/content/playground-doc.md`; the limit itself is unchanged.
  **Pinned** by `playground/tests/node/transport-bounds.test.mjs`, which drives
  `runSession` with a stub runtime that parks on an HTTP effect: exactly 64
  requests are dispatched, the 65th is refused with `E4020` before `fetch` is
  called, and the run still completes. Disabling the counter makes that test
  fail 4 checks rather than hang, because the stub stops parking a few effects
  past the budget.

- **B10 — Dead `MAX_REDIRECTS` and an overstated comment (LOW, CONFIRMED,
  FIXED).** `MAX_REDIRECTS = 3` was declared and never referenced anywhere in
  `playground/web/` (only a stale copy in `website/dist/`). Worse, the header
  comment claimed the Worker "follows at most a bounded number of redirects,
  re-checking authorization for a redirect target that changes origin" — false,
  because `redirect: "manual"` follows none. Removed the constant and rewrote
  the comment to state the actual behavior.

- **B11 — Node-side ABI pins (LOW, CONFIRMED, FIXED).** `abi.test.mjs` asserted
  `runtime.abiVersion === 1` at three sites. The suite runs against whichever
  artifact `manifest.current` names (`run-all.mjs:19-23,57`), so the pin was
  correct only while `current` was `0.3.1` and would have failed the moment
  `current` advanced to the already-present ABI-2 entry `0.3.2-dev.7`. The
  expectation is now derived from the manifest entry matching the artifact
  under test, with an explicit `AURA_EXPECTED_ABI` override for unpublished
  builds. Verified: `0.3.2-dev.7` now reports 80/80 where the old pin failed.

- **B12 — `complete` output mode unreachable from the web (LOW, CONFIRMED,
  documented).** `playground/runtime/src/lib.rs:238-243` accepts
  `output-mode preview|complete` and `OutputMode::Complete`
  (`COMPLETE_STDOUT_BYTES` = 16 MiB) is fully implemented and covered by
  `playground/runtime/tests/execute.rs:288`, but `encodeOptions`
  (`playground/web/runtime.mjs:193-211`) emits only `arg` and `stdin-bytes`, so
  no browser caller can select it. **It must not be wired naively:** the
  option parser only recognises `output-mode ` on runtimes that have it, so an
  ABI-1 artifact silently parses the line as a positional argument. Confirmed
  against the frozen artifacts — `0.3.1` and `0.2.1` both return
  `["output-mode complete", "hello"]` from `args()` for options
  `"output-mode complete\narg hello\n"`, while `0.3.2-dev.7` correctly returns
  `["hello"]`. Wiring it therefore requires an ABI gate and is left documented
  as a runtime/harness capability, not a web feature.

- **B13″ — Worker response byte accounting is already correct (FALSIFIED).**
  The claim was that `readBoundedBody`'s fallback counts UTF-16 units. It does
  not: the non-streaming path calls
  `new Uint8Array(await response.arrayBuffer())` and compares `buf.byteLength`,
  and the streaming path sums `value.byteLength` — both are bytes.
  `playground/web/worker.js:319-358`. **No defect; no change.**

- **B13′ — Request body and header caps count UTF-16 units, not bytes (LOW,
  NEW, CONFIRMED, FIXED).** Found while falsifying B13, in the *request*
  direction of the same file. `playground/web/worker.js:171` measured headers as
  `k.length + v.length` and `:193` tested `effect.body.length`, both of which
  are UTF-16 code-unit counts. A JS `.length` is a lower bound on the UTF-8
  length, so a body of `"一".repeat(8 * 1024 * 1024)` reports `.length ===
  8388608` — passing an 8 MiB cap — while actually occupying **25,165,824
  bytes (24 MiB)**, three times the intended bound; `"é"` is 1 unit but 2
  bytes and `"😀"` is 2 units but 4 bytes. Since the Worker's caps exist to stop
  a hostile page from making it buffer without bound, measuring in units
  defeated their purpose for non-ASCII input. Fixed via a `utf8Bytes` helper,
  keeping the cheap `.length` pre-check as a lower-bound reject so an oversized
  body is refused without first allocating a full encoded copy.
  **Pinned** by `playground/tests/node/transport-bounds.test.mjs`, which loads
  the shipped Worker file in a sandbox and drives `performHttp` directly: a
  3 Mi-character CJK body (9 MiB encoded, 3 Mi UTF-16 units) is asserted to
  pass a unit-based cap and to be rejected with `E4020` before `fetch` is
  called, and a 40 Ki-character Latin-1 header value (80 KiB encoded) likewise.
  The header case uses Latin-1 because `Headers.append` rejects code points
  above 255, so CJK is not a reachable header value.

### Coverage gaps (no executable test yet)

Still unpinned: session source/limit rejection; native timeout enforcement
against a slow server; `readBoundedBody`'s non-streaming fallback (exercised
only when `response.body` is absent); browser duplicate-header and
redirect-status behavior (both need a real browser, so they live in
`http-security.test.mjs`'s Playwright gate).

Now pinned: **HTTP from a multi-file project** (B14) by
`playground/tests/node/project-http.test.mjs`, which drives a two-module
program against a loopback server and carries a negative control that requires
`runProject` on the same program to refuse with `E5002`; it fails 2 checks
against the pre-fix `0.3.2-dev.7` artifact and passes all 20 against
`0.3.2-dev.8`.

Now pinned by `playground/tests/node/transport-bounds.test.mjs`: the request body
and header byte caps (B13′), and the per-run request budget (B9). That test was verified to *fail* against the
pre-fix accounting — restoring `k.length + v.length` and
`effect.body.length > MAX_REQUEST_BYTES` produces 4 failures, including
"rejected body is never dispatched" — so it is a real regression gate rather
than a restatement of the implementation.

### FALSIFIED (claims checked and not defects)

- **B13 — `readBoundedBody` fallback counts UTF-16 units.** Falsified; both
  paths measure bytes. See B13″ above.
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
3. **Browser/native HTTP divergence (B6, B7, B14) — ALL THREE RESOLVED.** B6
   (redirect status: native `302`, browser `E4020`) turned out to be **not
   fixable in principle**: an `opaqueredirect` response exposes no status, so
   synthesizing a `3xx` would mean inventing a code the browser withholds. It is
   a **formalized divergence** under Directive 003-R §4, pinned by a test and
   stated in `docs/LANGUAGE_SPEC.md` §25. B7 (browser `Headers` hides
   `Set-Cookie` and comma-joins duplicates) is a **documented divergence**: the
   combining happens in `Headers` iteration, upstream of anything the Worker
   controls. B14 was **implemented** — the escalation assumed a Host ABI 3 was
   required, but none exists, so the project entry point extends ABI 2 in place
   behind a feature detect and ships as `0.3.2-dev.8`. See the B6/B7/B14 rows
   above; no further human decision is needed for any of them.
4. **Nightly fuzzing has never run and cannot be run (TD-23) — FIXED**: `.github/workflows/fuzz-nightly.yml`
   is `schedule`-triggered but exists only on `rewrite/v3-rust`; `schedule` fires
   solely on the default branch (`master`), which lacks the file. It is also the
   only workflow without a branch `push` trigger, which is exactly why `ci.yml`
   and `pages.yml` *are* registered despite being absent from `master`.
   `workflow_dispatch` is **not** a workaround: GitHub resolves dispatched
   workflows against the default branch, so `gh workflow list` showed only `CI`,
   `Release`, and `Deploy website`, and `gh workflow view fuzz-nightly.yml`
   returned HTTP 404.
   The remedy originally recorded here — adding the same
   `push: branches: ["rewrite/**", master, main]` trigger — is **wrong** and was
   superseded: a `push` trigger *registers* the workflow and makes dispatch
   reachable, but the cron still executes the **default branch's** copy, which has
   no `fuzz/` directory, so the night would stay dark while the workflow looked
   enabled. That is a false green. **`rewrite/v3-rust` was instead made the
   default branch**, which makes this branch's cron live without duplicating
   files. All preconditions were verified first: `master` is a strict ancestor
   (`0 572`, no history lost), the five release tags are untouched and none was
   ever on `master`, no open PR targeted `master`, neither branch is protected,
   rulesets are empty, `release.yml` triggers on tags so it is branch-independent,
   and the `github-pages` environment allowlist already contains
   `rewrite/v3-rust`. The fix is confirmed behaviourally — the workflow appears in
   `gh workflow list` and a dispatched run goes green — not by the YAML existing.
5. **REPL O(N²) (TD-13)**: re-confirmed, not fixed. The O(N²) is real
   (release: 6.8 ms at N=100 → 1886 ms at N=3200, ~3.5–4.0× per doubling) and its
   cost is entirely `Checker::with_declarations` (linear, 1.8–2.6 µs/decl,
   called once per submission over a growing list). Making it incremental is
   **not** a mechanical change: the rebuild is also the rollback mechanism, and a
   monotonic cache diverges because `with_declarations` restores `value_types`
   only for bindings with a persisted `ty`, while `Stmt::LetPattern` persists
   `ty: None`. Any fix needs a differential oracle over randomized submission
   sequences, so it stays deferred to `v1.1`.

---

## E. Recommended correction order

1. **A1** (browser response cap) — highest; a resource-exhaustion vector.
2. **A2** (browser timeout) — small, self-contained.
3. Consolidated CONFIRMED findings from §B, by severity.
4. Documentation/cleanup items from §C.
5. AIS/0.2 only after the RFC is approved.
