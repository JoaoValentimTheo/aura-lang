# Aura 0.3.2 — Playground Execution & Browser HTTP Campaign

Status: **Gate 2 and Gate 3 complete; Gate 4 (browser HTTP) blocked on a
human ABI decision. Local, unpushed.** This is a *development* record for the
authorized 0.3.2 campaign. It never describes 0.3.2 behavior as a feature of
the published Aura 0.3.1 release.

Baseline: `7bdc9fcd` (local = remote). Published `0.3.1` Keystone and all
frozen runtimes are immutable. No push, tag, release, or deployment is
authorized for this campaign.

---

## 1. Resource inventory (Phase 1)

Every effective bound in the Playground path, with its classification.
`LANGUAGE CONTRACT` = normative semantics (never changed here);
`SECURITY REQUIREMENT` = protects the embedder from hostile input;
`HOST POLICY` = an application choice that may be revised with evidence;
`IMPLEMENTATION BOTTLENECK` = an unnecessary limitation to remove;
`HISTORICAL COMPATIBILITY` = pinned by a frozen artifact or published contract.

| Resource | Value | Where | Class |
|---|---:|---|---|
| Source file | 256 KiB | `playground/runtime/src/lib.rs` `MAX_SOURCE_BYTES` | HOST POLICY |
| Captured stdout | 1 MiB | `MAX_STDOUT_BYTES` | **IMPLEMENTATION BOTTLENECK** |
| Standard input | 1 MiB | `MAX_STDIN_BYTES` | HOST POLICY |
| Serialized virtual project | 2 MiB | `MAX_PROJECT_BYTES` | HOST POLICY |
| Virtual project sources | 4096 | `MAX_PROJECT_SOURCES` | HOST POLICY |
| Program arguments | 256 | `MAX_ARGS` | HOST POLICY |
| Bytes per argument | 16 KiB | `MAX_ARG_BYTES` | HOST POLICY |
| Virtual key | 128 B | `MAX_VIRTUAL_KEY_BYTES` | SECURITY REQUIREMENT |
| Source name | 1024 B | `MAX_SOURCE_NAME_BYTES` | SECURITY REQUIREMENT |
| AST depth | 256 | `src/run/mod.rs` `MAX_AST_DEPTH` (`E1015`) | LANGUAGE CONTRACT |
| Call frames | 512 | `MAX_CALL_FRAMES` (`E4011`) | LANGUAGE CONTRACT |
| Range materialization | 10M | (`E4013`) | LANGUAGE CONTRACT |
| Value render depth/nodes | 512 / 1M | `src/run/value.rs` | SECURITY REQUIREMENT |
| Parser recursion | 2048 native / 768 wasm | parser budget | HISTORICAL COMPATIBILITY |
| HTTP response body | 8 MiB | `src/host.rs` `MAX_HTTP_BODY_BYTES` | SECURITY REQUIREMENT |
| HTTP timeout | 30 s | `MAX_HTTP_TIMEOUT_MS` | SECURITY REQUIREMENT |
| Sleep clamp | 60 s | `MAX_SLEEP_MS` | HOST POLICY |

Additional constraints identified but not yet bounded (to add with HTTP):

- HTTP request body size, header count/size, request count per execution,
  pending operations, and total bytes per execution.
- Output transport chunk size and the DOM text-node growth.

### Confirmed defects (reproduced)

1. **stdout overflow → `E4020`** for a finite program. Reproduced: a
   `for i in 0..600000 { print(i) }` on the fresh wasm terminates
   `diagnostic` with `E4020 standard output exceeded the 1048576 byte limit`
   and 1,048,573 bytes retained. The bound is a Host policy, not a language
   rule.
2. **Outgoing HTTP headers are ignored** (native): `HttpRequest.headers` is
   populated by the stdlib but `src/host.rs::http::perform` never applies it.
3. **Response body is lossily decoded** (`String::from_utf8_lossy`), so binary
   payloads are corrupted silently.
4. **Repeated headers are comma-joined**, which is wrong for `Set-Cookie`.
5. **Request bodies for GET/HEAD/DELETE are silently discarded** (no documented
   policy).

---

## 2. Output architecture (Phase 2)

The host gains a bounded-memory **output sink** abstraction:

```text
Aura execution
    │
    ▼
Host::write_stdout                        (unchanged single enforcement point)
    │
    ▼
OutputSink  (src/host.rs)
    ├── OutputMode::Preview { limit }     bounded live preview; overflow is
    │                                     counted, never fatal
    ├── OutputMode::Complete { sink }     bounded-memory staged destination
    └── counters: written / retained / omitted
    │
    ▼
worker → main-thread transport (chunked)
    │
    ▼
Aurea output panel (bounded DOM)
```

### Preview mode (default)

- Retains at most `previewLimit` bytes (default 256 KiB).
- **Continues execution** when the preview fills; excess bytes are counted in
  `omitted` and dropped from the preview only.
- Never claims complete capture: the result payload carries
  `stdout_truncated: true|false`, `stdout_retained`, and `stdout_omitted`, so
  the UI can say "showing N of M bytes".
- Preserves accepted ordering; never splits a UTF-8 sequence at a chunk
  boundary (the retained preview is truncated on a char boundary).
- Exceeding the preview capacity is **not** `E4020`.

### Complete-output mode

- A bounded-memory destination that records the whole output; the default is a
  staging buffer with an explicit `completeLimit` (default 16 MiB). Overflow
  stops *capture* with a structured, non-fatal cap notice (`stdout_capped`),
  not a program failure.
- The architecture (`OutputSink` trait) allows an embedder to stream to disk or
  browser storage later without touching the evaluator; the wasm host implements
  the in-memory bounded variant and reports the exact bytes committed.

### Large single write

`write_stdout` accepts a write larger than a preview chunk: it retains the
prefix up to the remaining preview budget, counts the rest as omitted, and
returns `Ok` (preview) — never requiring the whole string to fit. Preview
truncation lands on a UTF-8 char boundary.

---

## 3. Continuation and Host-effect architecture (Phase 4)

The machine (`src/run/iterative.rs`) is already an explicit-continuation
interpreter: `Ctrl` / `Cont` / `Vec<Cont>` + `Vec<UserFrame>` fully describe a
suspended program, and `Machine::run` drives one `loop`. It **already** has a
resumable-native protocol (`NativeOutcome` / `NativeResume`,
`Interp::native_resumable`), used today by `map`/`filter`/`reduce` callbacks.

HTTP suspension reuses that protocol rather than inventing a parallel one:

- The HTTP builtins register as resumable natives. When the host reports
  `Pending`, the native returns a `NativeOutcome` that parks the machine at the
  call site with all state intact (frames, envs, continuations, spans, depth,
  cancellation).
- **Native** (feature `http`): the host performs the synchronous `ureq`
  request inside `http_request`, so it never suspends — behavior unchanged.
- **WASM/Browser**: the host returns `HostError::Pending(op)`; the machine
  parks, the wrapper returns control to JS with a pending-effect payload; the
  Worker performs `fetch()`; JS resumes the *same* machine with the response
  value. The program is never restarted and the request expression is never
  re-evaluated.

Zero-import is **preserved**: the module still imports nothing; the pending
effect is surfaced through the existing integer/byte ABI, and JS feeds the
result back through new ABI-1-additive export calls. Historical runtimes
feature-detect the new exports and are never sent the new commands.

### Decision package — cross-boundary parking (requires human sign-off)

The suspension mechanism exists *inside* one `run` call. Parking across the
**WASM export boundary** (so JS can do `fetch` and resume) requires two
decisions the campaign may not take unilaterally.

**D1 — Evaluator ownership.** Today `Machine<'i> { interp: &'i mut Interp }`
borrows the interpreter; `Interp::run_iterative(&mut self)` creates the machine
inside the call. To park between exports, the machine — and the `Interp` it
drives — must *outlive the export call*. The minimal principled change is
`Machine` owning `Interp` by value, driven by a `Parked` wrapper stored in a
wasm `thread_local` slot (`Interp` holds `Rc`, so it is not `Send`, and wasm is
single-threaded). This touches `src/run/iterative.rs` and `src/run/mod.rs` and
must preserve every existing invariant (62-target matrix + the recursive
`drive_resumable` parity). It is a B-1-class core change, not a wrapper.

**D2 — ABI surface.** The parked protocol needs additive export calls, e.g.
`aura_run_start(...)` (returns a result **or** a pending-effect token) and
`aura_run_resume(...)` (feeds the response back). This is an ABI-2 surface with
a new `playground_api_version` and a feature-detect on the JS side so frozen
runtimes are never sent the new commands. ADR-0001 requires deciding whether a
*browser HTTP capability* is a language-contract change (the language already
has `http_get`; only the host changes) — the campaign's reading is **no** for
the language and **yes** for the runtime identity, but the ABI versioning
belongs to a human-approved decision.

**Recommendation.** Approve D1+D2 as one scoped order with its own Gate 4 and
the suspension-invariant tests the campaign order lists (nested calls, loops,
closures, mutation, exceptions, multi-file). This pass implements everything
that does not depend on D1/D2 and does **not** fake browser HTTP: the browser
host continues to report `E5002`.

---

## 4. Native HTTP foundations (Phase 5)

- **A. Outgoing headers**: apply `request.headers` in the ureq dispatch; pinned
  by a loopback test that captures the received request bytes.
- **B. Binary body**: `HttpResponse` gains `body_bytes: Vec<u8>` and a `body`
  view. A new map key `body_bytes` (a list of ints, 0..=255) exposes raw bytes;
  `body` stays the lossy view for text programs. Additive, so it needs no
  compatibility decision.
- **C. Repeated headers**: keep the ordered `headers` list; join only the
  headers that are genuinely comma-safe, and expose every occurrence so
  `Set-Cookie` is not mangled.
- **D. Method/body policy**: document that GET/HEAD send no body; a body passed
  for GET/HEAD is **reported** (`E3001`-class policy error) rather than
  silently dropped. DELETE may carry a body.
- **E. Error taxonomy**: unchanged — `E5002`/`E4020`/`E4031` keep their
  meanings.

---

## 5. What this pass implements vs defers

Implemented and tested locally:

- Output sink with preview/complete modes, counters, and truncation flags
  (bounded memory, no fatal overflow). Reproduced defect fixed; benchmarks in
  §7.
- The new Playground runtime identity `0.3.2-dev.1` carrying the sink; 0.3.1
  and every frozen artifact untouched; manifest and `--check` green; the
  published `0.3.1` stays the public default (`PROMOTE_DEV_TO_DEFAULT` false).
- UI disclosure of retained/omitted bytes in the output panel.
- Native HTTP header fix (applied on the wire, captured by a loopback test) +
  binary `body_bytes` + ordered `header_lines` (repeated headers preserved) +
  a reported method/body policy + `Pending`/`PendingEffect` groundwork in the
  host error type.

Deferred (explicit, with the decision package in §3 pending human review):

- **Browser HTTP via suspension** — requires the cross-boundary parking driver
  (D1) and the additive ABI (D2). Until approved, the browser host continues to
  deny the capability with `E5002`; the website does not advertise browser HTTP.

## 6. Benchmark — new vs old output path (measured)

Fresh wasm, this machine, `AuraRuntime` driven directly (Node):

| Case | Old (0.3.1) | New (0.3.2-dev.1) |
|---|---|---|
| `for i in 0..600000 { print(i) }` | **E4020**, 1,048,573 B captured, run failed | **ok**, 262,144 B retained / 3,826,746 B omitted, 421 ms |
| `for i in 0..2000000` compute, one print | ok, 1.03 s | ok, 1.03 s (unchanged) |
| 2,000,000 × 32 B prints | E4020 | **ok**, 262,144 B retained / 65,737,856 B omitted, 1254 ms |
| 10 × 50,000 prints, heap delta | — | ≈ 0 MiB (bounded) |
| Preview render ceiling | — | ≤ 256 KiB DOM text, always |

The compute path is unchanged (the sink only affects `write_stdout`), and the
retained preview never exceeds its bound, so browser memory is bounded by
policy, not by program output volume.

---

## 7. Invariants

Frozen `0.0.2`/`0.2.0`/`0.2.1`/`0.3.1` byte-identical; tags unmoved; no
`unsafe`; no second interpreter; the compiler remains the sole authority.
