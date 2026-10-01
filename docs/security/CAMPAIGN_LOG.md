# Red-Team Campaign Log

Adversarial campaigns run against Aura HEAD (policy section 26). Each entry
records the target commit, the campaigns exercised, findings, and their
resolution. Severity uses the conservative scale (CRITICAL / HIGH / MEDIUM /
LOW / INFORMATIONAL).

## Campaign — TRAIN 1 (`92ea6bc` → fixes on `121a797`)

Attack surface: lexer, parser, checker, resolver, module graph, CLI, runtime,
filesystem/path handling, JSON. WASM/worker and live CPython boundaries were
out of scope for this CLI pass (covered separately by the CPython audit).

### Findings

| ID | Severity | Class | Summary | Status |
|----|----------|-------|---------|--------|
| F1 | CRITICAL | §31.5 host abort | A linear chain of wrapping aliases (`type A{i} = [A{i-1}]`) overflowed the 64 MiB execution stack (SIGABRT) at ~20k aliases: the flat node budget bounds *breadth*, not *depth*. | FIXED (`af909fd`) |
| F2 | HIGH | DoS (Θ(k²) memory/CPU) | The same chain cost Θ(k²) memory/time because alias resolution is memoized in declaration order (16k aliases ≈ 40 s, 3.4 GB). | FIXED (`af909fd`) |
| F3 | MEDIUM | DoS (Θ(n²) lexing) | The string scanner validated `from_utf8` over the entire remaining source once per character, so a 5 MB string literal did not lex within 40 s. | FIXED (`af909fd`) |
| F4 | MEDIUM | Cross-substrate divergence | In-source `module` nesting consumed only the substrate-calibrated backstop (native accepted 2047, WASM 767), the same native/WASM acceptance split ADR-0004 removed for types. | FIXED (`af909fd`) |
| F5 | LOW | Diagnostic quality | Deep `module`/loop nesting reported `E1015` without a `file:line:col` location. | OPEN (tracked) |
| F6 | INFO | Semantic surprise | `1e999999` evaluates to `inf`; §10.7 makes out-of-range float literals inf by design. | ACCEPTED |

### Resolution

F1/F2 share one root cause and one fix: bound the **depth** of a *resolved*
type at the semantic AST limit (256), so an over-deep alias chain is the stable
`E1015` a literally over-nested type produces (ADR-0004). The depth measure is
iterative, so the guard cannot itself overflow. F3 computes a character's UTF-8
width from its lead byte in O(1) (the lexer only receives validated `&str`).
F4 counts module nesting toward the same 256-level semantic budget, and a
physical wrapper (loaded child source) seeds that budget so it cannot bypass
the limit.

Regression coverage: `tests/adversarial.rs` (linear alias chains),
`tests/boundaries.rs` (N-1/N/N+1 for aliases and modules; annotation
positions), `tests/bench.rs` (alias-chain and long-string sub-quadratic scaling
guards), `tests/module_graph.rs` / `tests/native_source.rs` (depth attributed to
the child source; physical wrapper cannot bypass E1015).

### Campaigns with no finding

- **S1 malformed source** — 52 targeted malformed inputs plus 4000 random
  token-soup inputs: all structured diagnostics, no panic/abort/timeout.
- **S2 deep structures** (non-module) — bounded `E1015`.
- **S3 resource exhaustion** — flat large inputs bounded; the alias chain was
  the only crash (F1/F2).
- **S4 filesystem/path** — all cases structured (`E2020`/`E2022`/`E4020`).
- **S5 module graph** — cycles, diamonds, duplicate claims, re-exports, large
  sibling sets: all structured.
- **S8 JSON** — malformed/oversized/deep/cyclic: structured or bounded.
- **S10 cross-subsystem** — builtin collision, closure mutation, failed-then-
  valid REPL, cyclic repr/JSON: all safe.
