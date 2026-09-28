# AURA COMPLETE LANGUAGE CONFORMANCE & HARDENING REPORT

Phases 2 → 12 plus final adversarial closure. Phase 1 was closed at
`573ec8bc2811b0a4cf1db43e02bf57479132fd6b`; this program resumes from there.

## 1. Final Repository State

* Branch: `rewrite/v3-rust`.
* Final HEAD: `23d47de2ece1d3721aab226dd2ff53ccc3e98710`.
* Worktree: clean (no transient artifacts).
* Commits added over Phase 1 (20):
  `8aa1c7f`, `5d54ab8`, `0f3555d`, `2462d3f`, `39f4005`, `73c7b63`, `43b7cef`,
  `3812f1e`, `04621a2`, `965d188`, `b45b8bd`, `acf5535`, `fbaa713`, `226b889`,
  `6dfc8da`, `f066da4`, `bf68538`, `f0d2f47`, `082238e`, `ab7c29c`.

## 2. Phase Summary

| Phase | Scope | Findings | Status |
|---|---|---|---|
| 1 | syntax/lexer/parser/grammar | 12 fixes, 3 SPEC GAPs | CLOSED |
| 2 | AST/resolution/modules/visibility/scopes | 5 fixes + 5 open questions | VERIFIED |
| 3 | type system/checker/generics/traits | 2 fixes (CONF-TYPE-1, CONF-GENERIC-1) | VERIFIED |
| 4 | execution/control flow/exceptions | none | VERIFIED |
| 5 | value model/equality/rendering/patterns/ranges | none | VERIFIED |
| 6 | stdlib/builtins | none | VERIFIED |
| 7 | Python bridge | none | VERIFIED |
| 8 | native/WASM/browser | none | VERIFIED |
| 9 | diagnostics | none (2 internal/unused codes, 1 shared code) | VERIFIED |
| 10 | robustness/fuzz/CI reliability | 2 fixes (CONF-RESOURCE-1/2) + CI-RELIABILITY-1 | VERIFIED |
| 11 | documentation reconciliation | none new | VERIFIED |
| 12 | build/artifacts/website/CI | dev.25 + dev.26 staged | VERIFIED |
| Final | adversarial closure | 2 counterexamples, both fixed | CLOSED |

## 3. AST / Resolution Model

Six node families (`Item`, `Expr`, `Stmt`, `Pattern`, `TypeExpr`, `FParam`);
every `Expr` carries a `Span`; `TypeExpr` is the sole span-less node (its
diagnostics borrow the enclosing declaration). The resolver runs two global
passes (collect, then apply imports) and one flatten pass, rewriting every free
reference to a canonical name, so declaration order never affects resolution.
Three namespaces: value, type, variant. Canonical variant name = module path +
tag (`LANGUAGE_SPEC.md` §27); a canonical name denoting **both** a type and a
variant is rejected deterministically (it cannot be represented by the flat
item table).

## 4. Module / Visibility / Scope Model

In-source nested modules, `::` paths, `use path`, `use path as Alias`, private
items (`E2018`), descendant-can-see-ancestor-private, and per-module variant
tags are verified. Open questions (SPEC GAPs, §21): variant-tag uniqueness
(global §18.1 vs module-scoped §27); whether a nested module is a visibility
boundary; `pub use` re-exports (unimplemented); module aliases (accepted but
inert); and the type/variant canonical-name collision.

## 5. Type System Model

`Ty`: `Int Float Bool String List Map Named Enum Union Param App Unknown`.
Union normalization is total and deterministic; `Unknown`/`none` collapses to
`Unknown` (permissive boundary §2.3). Assignability is nominal for user types,
no implicit int↔float. Inference computes real result types for arithmetic
(`CONF-TYPE-1`).

## 6. Generic / Trait / Method Model

Static, erased, nominal parametric polymorphism. Generic inference, explicit
arguments, nested applications, bounds, generic structs/enums/aliases/
functions/impls, and generic methods work; a type parameter may not shadow a
visible declared type (`E2007`). Trait method agreement, missing/extra
methods, and overload selection are deterministic and order-independent. A
generic `return` body must match the declared return type (`CONF-GENERIC-1`).

## 7. Checker ↔ Runtime Contract

The checker and runtime agree except for the two documented boundaries:
`Unknown` permissiveness (§2.3) and the §9.1 split (arithmetic operand type
errors are runtime `E3001`; ordering is statically checked when provable).
A large differential sweep found **no** other acceptance disagreement after the
`CONF-TYPE-1` and `CONF-GENERIC-1` fixes. The registry oracle proves every
checker-accepted well-shaped builtin call has a runtime dispatcher.

## 8. Execution / Control Flow Model

Call frames 512 (`E4011`), recursion/mutual recursion, closures with fresh
per-iteration binding, `for`/`while`/`loop`, and `try`/`catch`/`finally`
(including a `finally` signal replacing the pending outcome) match §14–15.
Only explicit `throw` is catchable; uncaught is `E4026`.

## 9. Value / Equality / Rendering Model

`BTreeMap`-backed deterministic maps; iterative cycle-safe equality (`NaN`≠
`NaN`, `0.0`=`-0.0`, function identity); rendering bounded at depth 512 and
1,000,000 nodes; exact-length list patterns; half-open ranges with saturating
length and a 10,000,000-element materialization cap.

## 10. Pattern / Range Model

Patterns: wildcard, name, literals, exact list, variant (bare and enum-path),
guards. No rest/struct/range/negative/float patterns. Ranges: half-open,
right-associative, `len` saturating, reverse-associative `a..(b..c)`.

## 11. Standard Library Matrix

38 builtin functions and 67 method names declared in
`stdlib::signatures.rs` (single source shared by checker and runtime):
output/diagnostics, collections, strings, conversions, numeric, IO, JSON
(`json_encode`/`json_decode`), regex, time, and Python. Checker signature,
runtime implementation, feature gate, and diagnostics agree for every entry.

## 12. Python / Host Bridge Result

Bounded and cycle-safe in both directions (address/`Rc` identity path cycle
detection; 1,000,000-node budgets; depth fallback to `repr`). Oversized
Python `int` → `E4013`, non-string dict key → `E5002`, cycle → `E5002`,
exceptions → `E5001`. No identity loss, mutation, GIL misuse, or crash.

## 13. Native / WASM Conformance

Generated differential parity: **195** comparisons (status, stdout, code,
message, line, column), **43** explicit syntax comparisons, byte-boundary
cases. WASM imports: **0**. The only deliberate divergence remains the AUDIT-3
TypeExpr ceiling (native 2048 / WASM 768).

## 14. Diagnostic Inventory

43 codes across `E1xxx`–`E5xxx`. `docs/errors.md` and the implementation agree;
`E4099`/`E5003` are documented internal/unused; `E5002` is intentionally shared
by two synonymous constants. Deterministic wording; in-source spans.

## 15. Resource / Robustness Result

No production `unsafe`, `unwrap`/`expect`/`panic!`; two guarded `unreachable!`.
All arithmetic boundaries (`i64::MIN/MAX`, `/0`, `%0`, shift range, overflow,
`abs`/`sum` overflow) yield stable `E4xxx`. Every nesting construct is bounded
by `E1015`/`E4011` — including unary chains (`CONF-RESOURCE-1`) and nested modules (`CONF-RESOURCE-2`).

## 16. Property / Fuzz Result

Registry oracle, native/WASM differential, normalization idempotence, and
generated AST→checker properties. Fuzz targets `lexer parser checker runtime`
(ASan on; LSan scoped off for `runtime`): ~1.3M / 0.5M / 0.27M runs locally with
zero crash artifacts. Fuzz seeds added for both closure findings.

## 17. Documentation Alignment

The six-level authority model holds (`LANGUAGE_SPEC` normative → grammar →
contract → guides/site → roadmap/history → RFCs). Phase 2/3 fixes moved code
*toward* the already-normative text, so no specification edit was required.
Website examples (19) and links (2086) execute/pass.

## 18. Artifact Inventory

| Version | Path | Bytes | SHA-256 |
|---|---|---|---|
| 0.0.2 (frozen) | `playground/runtimes/0.0.2/…wasm` | 1,366,621 | `5a4ad3f7e3f786164d65df437d607e7ddd5e25947ea2c8dd9b436a5490b334ed` |
| dev.23 | `…/0.0.2-dev.23/…wasm` | 1,604,958 | `71072150e67384120c63e22d6176f3683110735b84f74723bea315f79778a528` |
| dev.24 | `…/0.0.2-dev.24/…wasm` | 1,604,902 | `16882fe60d7fa52f9e204b3841cc59764c79f50c1068f33b7c3e3350f1adfd39` |
| dev.25 | `…/0.0.2-dev.25/…wasm` | 1,612,018 | `a9462510c963beb3b721c9f1da680f39722bfce6906eb73ad9ba304fe344a406` |
| dev.26 | `…/0.0.2-dev.26/…wasm` | 1,612,422 | `1e91a070bfd6762d1e2ec6fb504e1a362f2f64450b963adbaadd14731e316185` |
| dev.27 | `…/0.0.2-dev.27/…wasm` | 1,613,542 | `605d18927bcd8c2a77105975e62d916b64a9ca803ca1050dd9c6f4dd3c9b1451` |
| dev.28 | `…/0.0.2-dev.28/…wasm` | 1,614,143 | `b1cf4d6d1593f0079d0d3c97fcad091e07975421220537dfcfe83c6bfa401ed0` |
| dev.29 (current) | `…/0.0.2-dev.29/…wasm` | 1,614,239 | `aa832ba72578897f6b99650574939011dda25e0d390fdb5efb6fae825816bdd3` |

dev.25–dev.29 were each advanced by real production changes; all
earlier artifacts are byte-identical and untouched. Each new artifact is
reproducible from source, import-free, and staged on the website. No
release/tag created.

## 19. Validation

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo test --locked --all-targets --all-features` | **650 passed, 0 failed** |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | pass |
| `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time` | **641 passed, 0 failed** |
| `node playground/tests/node/run-all.mjs` | manifest 26, ABI 67, integrity 27, differential 195, syntax 43, browser 53, worker 12, cache 7 — 0 failed |
| `node playground/build.mjs --check` | pass (3 versions) |
| WASM release build | reproducible; 0 imports |
| `node website/tests/run-all.mjs` | examples 19, links 2086, a11y 70, browser 343 — 0 failed |

Counts are per category and not summed: 652 Rust `#[test]` functions in the
tree, 79 token variants, 43 error codes, 38 builtin functions, 103 corpus
fixtures (11 syntax), 195 differential + 43 syntax Node comparisons, plus
property iterations and fuzz executions.

## 20. CI

* Workflow: **CI**, run **36381952455**, commit **ab7c29c3**, 14 jobs, **all
  success** (rustfmt, clippy, cargo audit, MSRV 1.83, miri, extended property
  tests, language contract, pure-Rust-no-CPython, tests on ubuntu/macos/windows,
  playground wasm runtime, website, fuzz smoke).
* Deploy: **Deploy website (GitHub Pages)**, run **36381952414**, commit
  **ab7c29c3**, success.
* The earlier run `36374660319` failed only on the fuzz smoke job
  (`runtime` `slow-unit`), classified CI-RELIABILITY-1; it passed on rerun, and
  all subsequent runs passed on the first attempt.

## 21. Remaining SPEC GAPs

* `CONF-PARSE-8` — general separator policy (separator-free adjacency).
* Numeric underscore placement (`1__0_`, `0x_f_f_`).
* F-string outer-quote / lone-`}` edge rules.
* `CONF-GRAM-4` — generic-head `::` continuation.
* `CONF-RESOLVE-6` — variant-tag uniqueness: global (§18.1) vs module-scoped
  (§27).
* `CONF-RESOLVE-7` — is a nested module a visibility boundary? (`pub module`
  is inert today.)
* `CONF-RESOLVE-8` — `pub use` re-exports unimplemented.
* `CONF-RESOLVE-9` — module aliases accepted but inert.
* `CONF-RESOLVE-10` — a struct and an enum variant may share a canonical name;
  the collision is now rejected deterministically, but the namespace question
  is unresolved.

## 22. Remaining DESIGN LIMITATIONS

* Named method arguments parse but are rejected (`E3001`) — RFC candidate.
* Multiline pipelines, list-rest patterns, and range step are not implemented
  (RFC candidates).
* Recursive local closures are unsupported (lexical binding).
* Cross-file/filesystem modules are not a current feature.
* Struct field read on a struct lacking the field is `Unknown` and the runtime
  is authoritative (§17.5) — intentional, not a bug.

## 23. DECISION-PENDING

Follow-up 3 / AUDIT-3 remains DECISION-PENDING; no code or doc changes beyond the existing decision package; property test AST-limit explicitly excludes TypeExpr-heavy inputs pending that decision.

Native accepts TypeExpr nesting through 2047 and rejects 2048; WASM accepts
through 767 and rejects 768, both `E1015`. Not normalized in this program.

## 24. Final Classifications

* PHASE 1 SYNTAX: CLOSED
* AST INVARIANTS: VERIFIED
* NAME RESOLUTION: VERIFIED
* MODULES: VERIFIED
* VISIBILITY: VERIFIED
* SCOPES: VERIFIED
* TYPE SYSTEM: VERIFIED
* CHECKER: VERIFIED
* GENERICS: VERIFIED
* TRAITS / METHODS: VERIFIED
* CONTROL FLOW: VERIFIED
* RUNTIME: VERIFIED
* VALUE MODEL: VERIFIED
* PATTERNS / RANGES: VERIFIED
* STDLIB: VERIFIED
* PYTHON BRIDGE: VERIFIED
* NATIVE/WASM: VERIFIED
* DIAGNOSTICS: VERIFIED
* RESOURCE SAFETY: VERIFIED
* FUZZ / PROPERTY HARDENING: VERIFIED
* DOCUMENTATION: VERIFIED
* ARTIFACT ACCOUNTING: VERIFIED
* CI: VERIFIED
* SPEC GAPS: DOCUMENTED
* AUDIT-3: DECISION-PENDING

## 25. Final Remaining Uncertainty

* The nine SPEC GAPs (§21) are unresolved by design; behavior is locked but the
  normative decision is the human's.
* The TypeExpr nesting divergence is deliberately unequal across substrates.
* CI-RELIABILITY-1 remains: the `runtime` fuzz target can nondeterministically
  emit a `slow-unit`, which the workflow's blanket artifact check fails. This
  is a harness policy question, not a proven production nontermination.
* No formal proof was produced; conformance is evidence-based (reproduction,
  classification, regression, adversarial falsification), not formally
  verified. The words "bug-free" and "perfect" are not claimed.
