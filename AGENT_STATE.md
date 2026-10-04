# Aura Agent State

Compact current engineering state. Rules: `AGENTS.md`. Active task and exact
next action: `docs/engineering/CURRENT_HANDOFF.md`. Detailed chronology:
`STATUS.md`. If Git and this file disagree, **Git wins** — reconcile this file
before continuing substantial work.

## Repository

- Branch: `rewrite/v3-rust`
- Remote `origin/rewrite/v3-rust`: `efc66bde46a69c164f256254c47f58b96ad1e4b4`
  (R3A + B-1R3B.1 + B-1R3B.2 + B-1R3B.3 + B-1R3B.4.1; pushed and remote-closed).
  The B-1R3B.4.2 commits are **local and unpushed** on top of it.
- Local/remote relationship: authoritative value is `git rev-list
  --left-right --count origin/rewrite/v3-rust...HEAD`; a tracked file cannot
  safely hardcode its own position.
- Current release: **`v0.2.1`**, published 2026-10-01, immutable. Tag
  `v0.2.1` = commit `3f5f8702`. Release = language = runtime = `0.2.1`;
  Host ABI 1; Playground API 1.
- No successor program or version is selected; there is **no `0.2.2`**.

## Frozen release runtimes (immutable, byte-for-byte)

| Version | Bytes | SHA-256 |
|---|---|---|
| `0.0.2` | 1,366,621 | `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed` |
| `0.2.0` | 1,654,161 | `9937fd8094ef402b7a9233d02bd232405f75b9e70661404646fcda7cd295c5bc` |
| `0.2.1` | 1,768,322 | `48c456fcda6c50dd6808ccc5f15a0bca4c0b81d7d970172557817decf427cc9e` |

Never overwrite a versioned artifact under `playground/runtimes/`; advancing a
runtime means adding a version, never replacing one.

## Current Track

B-1R — ENGINE-STACK-INDEPENDENT CALL ENGINE. B-1 is OPEN (WASM implementation
nonconformance). The R3A baseline through B-1R3B.4.1 are pushed and remote-closed;
B-1R3B.4.2 map construction is complete locally and unpushed. Production still
runs the recursive evaluator.

B-1R phase state:

- **B-1:** OPEN — full iterative evaluator not implemented. Released WASM
  runtime still traps on the engine stack below the 512-frame language limit;
  `v0.2.1` is immutable and contains the defect. Native conforms.
- **B-1R1:** DESIGN COMPLETE (`docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md`).
- **B-1R2:** DIFFERENTIAL ORACLE COMPLETE AND MUTATION-VALIDATED
  (`docs/engineering/B1R2_DIFFERENTIAL_ORACLE.md`; 104-case corpus; isolated
  engines; `tests/oracle/golden.tsv`).
- **B-1R3A-ARCH-1:** RESOLVED — AST sharing uses `Arc`
  (`docs/B1R3A_AST_SHARING_DECISION.md`). Runtime `Env`/`Value`/`Closure` stay
  `Rc` intentionally.
- **B-1R3A:** COMPLETE AND REMOTELY CLOSED at `c9ade0b`. `src/run/iterative.rs`
  is a real explicit-continuation machine; the oracle has an identified R3A
  subset; all deliberate mutations detected and reverted.
- **B-1R3B.1:** COMPLETE AND PUSHED at `1a12b84` (unary `-`/`not`/`~`).
- **B-1R3B.2:** COMPLETE AND PUSHED at `1a12b84` (eager binary operators).
- **B-1R3B.3:** COMPLETE AND PUSHED at `2f34b9c` (short-circuit `and`/`or`).
- **B-1R3B.4.1:** COMPLETE AND PUSHED at `efc66bd` (list/tuple construction;
  differential-oracle equivalent; tuple is list sugar per `LANGUAGE_SPEC.md`
  §21).
- **B-1R3B.4.2:** COMPLETE LOCALLY; PUSH BLOCKERS REMEDIATED (map construction;
  differential-oracle equivalent; key-before-value source order, exactly-once,
  last-wins duplicates, runtime invalid-key `E3001` at the key span, control
  propagation, diagnostics/spans, and host-stack safety verified). A first
  adversarial push gate found and this remediation closed: the program-mode
  `main_map_element` case (list containing a Map through a real frame boundary)
  was restored to the R3B.4.2 supported set, the `r3b42_golden.tsv` LF pin was
  added, and the R3B.4.2 counts were reconciled (57 supported cases = 49
  value-mode + 8 program-mode; 7 unsupported cases; 57 golden rows). Unpushed;
  a fresh gate over the expanded range is required. See
  `docs/engineering/CURRENT_HANDOFF.md`.
- **B-1R3B.4.3/R3B.5…R3G:** NOT STARTED.

## Production vs experimental engine

- Production / default: **recursive** evaluator, unchanged and authoritative.
- Experimental: iterative evaluator, compiled only under the non-default
  `evaluator-oracle` feature, not reachable from CLI/REPL/Playground/library
  production paths. Supported subset so far: literals, name lookup, expression
  statements, blocks, `let` shadowing, `if`/`else`, unary `-`/`not`/`~`, the
  eager binary operators (`+ - * / % ^ == != < <= > >= & | << >>`),
  short-circuit `and`/`or` (the skipped operand is never evaluated), list/tuple
  construction (left-to-right, exactly once per element), and map construction
  (per entry key then value in source order, exactly once; last-wins duplicates;
  runtime invalid keys are `E3001` at the key span); everything else (calls,
  ranges, comprehensions, …) returns `E4999` and never falls back to recursion.

## Known blockers

- B-1 only. All HD/AUDIT decisions are resolved (ADR-0001…0004) and released
  in `v0.2.1`. No open CRITICAL/HIGH security blocker.

## Protected local state

- `.kilo/**`: pre-existing, **unstaged** tool-config churn unrelated to any
  engineering task. As of this checkpoint `git status --short -- .kilo/` shows
  **12 tracked deletions** (`.kilo/agent/*` × 7, `.kilo/command/*` × 3,
  `.kilo/run-script.sh`, `.kilo/setup-script.sh`) and **1 tracked modification**
  (`.kilo/kilo.jsonc`). Preserve exactly as Git reports it: do not stage,
  restore, modify, or commit any of it. Tool configuration is not engineering
  authority (`AGENTS.md`). Always trust `git status` over this summary.
- Root-level `s`: previously traced to a non-hermetic property test and fixed.
  It is **absent** and must remain absent; if it reappears, investigate rather
  than delete blindly.

## Human Decisions

Resolved by ADR-0001…0004 (see `docs/adr/`); do not reopen or re-queue.
- ADR-0001 — release and language versions are distinct.
- ADR-0002 — module members may reuse builtin spellings.
- ADR-0003 — CPython support tiers.
- ADR-0004 — AUDIT-3: structural `TypeExpr` nesting counts toward
  `MAX_AST_DEPTH = 256` on every substrate (RESOLVED).

## Verified Closed

- FSM-P1 — CLOSED
- FSM-P2 — CLOSED
- FSM-P3 — CLOSED
- FSM-P4 — CLOSED REMOTELY
- FSM-P5 — CLOSED REMOTELY
- FSM-P6 — CLOSED (included in `v0.2.1`)

## Do Not Start

Without new human authorization: package manager/manifests, browser
persistence, LSP, formatter product, async, macros, a new release or version
line, a new FSM phase, or any successor engineering program.

## Handoff

Before a model switch: finish the atomic operation, understand the diff, update
`AGENT_STATE.md` and `docs/engineering/CURRENT_HANDOFF.md` if state changed, run
`scripts/agent-state.sh`, and set writer ownership below. Synchronization is
Git + working tree + these documents; chat history is not authority.

## Exact Next Action

See `docs/engineering/CURRENT_HANDOFF.md`. In short: run a fresh adversarial
read-only push gate over the expanded range (R3B.4.2 plus push-blocker
remediation); if it passes, push and remotely close B-1R3B.4.2, then begin the
next microphase per the handoff ordering (R3B.5 range).

## Writer

NONE
