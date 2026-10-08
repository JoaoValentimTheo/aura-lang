# Aura 0.3 “Keystone” — Canonical Scope Manifest

Status: **canonical engineering manifest** for the Aura 0.3 “Keystone”
super-transaction (closure range pushed through remote `b221e6a9`). Authority:
this file classifies
scope and points to the authoritative spec/RFC/code/tests; it does not itself
define language semantics. Normative behavior lives in `docs/LANGUAGE_SPEC.md`
and accepted `docs/adr/*`; current repository state lives in `AGENT_STATE.md`.

Bootstrap evidence (2026-10-06): branch `rewrite/v3-rust`, local HEAD
`8e4a59a1bb08372bd5f628236d275d0c7eb17a6b`, remote
`e576238f7adb52f6ab6d18431602e3fbc2bd3636`, ahead 18 / behind 0, staged empty,
protected `.kilo/**` churn unchanged, `.codex/config.toml` tracked-clean, root
`s` absent, tags `v0.0.1`/`v0.0.2`/`v0.2.0`/`v0.2.1` (`v0.2.1` = `3f5f8702…`),
declared version `0.2.1`, frozen artifact SHA-256 byte-identical
(`0.0.2` `5a4ad3f7…`, `0.2.0` `9937fd80…`, `0.2.1` `48c456fc…`).

Method: every classification below was verified by executing
`target/debug/aura` built from HEAD and/or by running the named test suite, not
by reading prose alone. Where the transaction order’s premise contradicted
repository reality, repository reality is recorded and the premise classified
under “FALSIFIED”.

---

## A. ALREADY IMPLEMENTED AND VERIFIED (probe-confirmed at HEAD)

| Item | Evidence (executed) |
|---|---|
| General union types incl. `T \| none`, permissive `Unknown` collapse | `let x: int \| none = 5` runs; `"str"` accepted (permissive); `tests/checker.rs:703` `union_with_none_is_permissive`; `tests/generics.rs:201` |
| `struct` nominal identity, fields, optional field `str \| none` | probe `struct User { name: string, email: string \| none }` constructs/reads; `tests/oop.rs`, `tests/collections.rs` |
| `impl` methods on structs, overload merge, traits (`impl T for S`) | probes run; `tests/traits.rs` 18/18 pass; `tests/oop.rs`; `LANGUAGE_SPEC.md` §17.6–17.7 |
| Module system: `module`, `pub`, `use … as`, `pub use`, `::` paths, visibility, nominal variant identity | `tests/modules.rs`; spec §27; ADR-0002 |
| `match` patterns (literal/binding/variant/list) and guards | spec §19; `tests/parser.rs`, oracle corpus |
| Named arguments for directly resolved top-level functions | spec §15.8 frozen decision 18 |
| Generics (`fn f<T>`, `struct S<T>`, bounds, `App`) | spec §36; `tests/generics.rs` |
| `json_encode`/`json_decode` builtins | probe runs; `src/stdlib/signatures.rs:504-516` |
| Diagnostics are fatal and not catchable; only explicit `throw` is caught | probes; `EXCEPTION_SYNTAX_DECISION_PACKAGE.md` §2–3 |
| REPL recovery after check/runtime error | `tests/repl.rs:72,79` `recovers_after_check_error`/`recovers_after_runtime_error` |
| CLI stdout/stderr separation for diagnostics | `src/main.rs` writes diagnostics via `eprintln!`; program output via host stdout |
| `_name` parameter must stay unused (`E2009`), `_` bare param allowed | probe `_x` used → E2009; `_` param → ok |
| Native/WASM production routing to the iterative machine | `tests/production_routing.rs`; `AGENT_STATE.md` |
| Frozen 1 MiB `BrowserHost` stdout policy, documented + tested | `playground/runtime/src/lib.rs:93`; `docs/playground.md` §2; `playground/tests/node/b1_boundary.test.mjs` |

## B. IMPLEMENTED BUT INCOMPLETE (real gap; Keystone work)

| Item | Gap | Evidence |
|---|---|---|
| Optional narrowing | `T \| none` collapses to `Unknown`; no static narrowing; `u.name` on `User \| none` yields misleading `E2003: none has no method 'name'` | probes; spec §4.3/§34.2 |
| `never` type | `-> never` is `E3002: unknown type` | probe |
| Raise-site span | `Ctl::Throw(Value)` carries no span; uncaught `E4026` reports first frame-crossing call site | decision package §5 gap 2 |
| Catch selection | `catch` binds one untyped value; `catch Name::Variant(m)` is `E1006` | probe |
| Builtin exception family registry | none exists (by design, level separation) | decision package §5 gap 3 |
| Unused analysis | unused `let`, unused `use`, unused pattern binding, unused catch binding all accepted silently | probes |
| Structured diagnostic fields | `Diag { code, message, span }` only; no severity/notes/help/labels | `src/error.rs:32-40`; taxonomy §4 |
| CLI color policy | no `NO_COLOR`/`auto/always/never`; no colored rendering | `grep` found none |
| Playground structured diagnostics | Playground does not render structured diagnostics as UI components | `playground/web/app.js` |
| Runtime codename/channel metadata | manifest has `channel` but no codename; historical runtimes have no beta channel label | `playground/runtimes/manifest.json` |
| WASM output architecture | 1 MiB bound is a documented `BrowserHost` policy (root-caused to `BrowserHost`, not the evaluator); no chunked `OutputSink`; whole-output accumulation at the boundary | `playground/runtime/src/lib.rs`; `docs/playground.md` §2 |
| Local `const` in function bodies | `const X = 5` inside `fn` is `E1006`; **the order’s §11 premise that local const is valid contradicts HEAD** | probe `const X = 5` → E1006 |
| Shadowing policy | fully specified and tested; **no change authorized** — audit only | spec §16.3; `tests/shadowing.rs` |

## C. NEW 0.3 WORK (authorized by this order)

| Item | Basis |
|---|---|
| Exception RFC + vertical slice (E1–E6 now decided) | order §20; `docs/rfcs/README.md` |
| HTTP capability (Host → provider → network), typed request/response | order §21 |
| JSON → typed Struct/value decoding (type-directed) | order §22 |
| AIS/0.1 semantic protocol (stable external schema) | order §36 |
| Aurea CSS design system + website/playground redesign | order §29–31 |
| Overflow invariant tests (320px … ultrawide, 200% zoom) | order §32 |
| CLI colored diagnostics with structured source of truth | order §28 |
| Chunked/bounded output sink where compatible with the frozen contract | order §5.1 |
| Execution-recovery hardening beyond REPL (worker/playground) | order §6 |
| Output-channel separation verification (Native/REPL/WASM/Worker) | order §7 |
| Embedded-CPython + foreign-value investigation | order §23 |
| Tooling reconciliation (formatter/LSP/package manager as architecture/contract) | order §35 |
| Runtime codename/channel metadata (Keystone = Development; historical = Beta placeholders) | order §34 |

## D. HUMAN-GATE REMAINING

| Gate | Status |
|---|---|
| E1–E6 exception decisions | **RESOLVED by this order** (§20 provides all six: keep `throw`; additive nominal/pattern-compatible catch selection chosen by smallest unambiguous RFC; nominal module-qualified identity; builtin diagnostics non-catchable; Python boundary unchanged; reserved builtin identity). RFC is the record of this authorization. |
| Shadowing policy change | **NOT authorized** — audit/document only (order §12). |
| Nested functions / block-level `const` | **NOT authorized** — do not invent; preserve existing semantics. |
| Discard syntax for unused bindings | `_` already binds nothing (spec §4.6) and `let _ = …` parses; use existing semantics, no invention. |
| Runtime publication / codename for historical versions | Keystone remains Development; no publication in this transaction; historical codenames need human approval → use explicit placeholders. |
| Package-manager remote ecosystem/security model | produce architecture/contract only. |

## E. EXPLICITLY DEFERRED (out of scope unless already normative)

Public generics expansion beyond §36, general union expansion beyond §4.3,
traits expansion, async, concurrency, bytecode VM, new codegen backend,
JS/DOM integration, `<script src="*.aura">`, Flutter/Dart, quantum APIs,
Aura 0.4 AI stdlib/LangChain-like framework, `try` without `catch`,
tuple type, default/variadic args, list-rest patterns, range step/inclusive
ranges, `for … else`. See spec §34.1/§35 and order §38.

## F. FALSIFIED / NOT ACTUALLY NEEDED (premise corrected by evidence)

| Order premise | Reality at HEAD |
|---|---|
| §11 “local const inside `fn`/`main` is valid” | **FALSE**: `const X = 5` in a body is `E1006`. Only module-scope `const`/top-level `let` exist. No change authorized; record as-is. |
| §13/§14/§15/§16/§17/§19 “close gaps in collections/Struct/functions/patterns/methods/modules” | Largely already normative and tested. Remaining justified 0.3 gaps are the ones in section B, not a re-implementation. |
| §10 “Do NOT introduce competing absences” | Audit found exactly one absence concept (`none`); no `null`/`nil`/`undefined`/`missing` in spec or lexer. Requirement already satisfied. |
| §9 “Value/Iterable/Mapping/Callable/Object/Nullish families” | Internal `Ty` already models primitives/`List`/`Map`/`Named`/`Enum`/`Union`/`Param`/`App`/`Unknown`; the diagram is conceptual and must not become public generics/traits syntax. |
| §12 “unused local binding ERROR” | Currently accepted; this is genuinely new strictness and will be implemented as an explicit, documented, additive diagnostic with a mutation-tested regression. |
| Sibling `.aura` file affecting `aura run <entry>` | **Not a defect**: frozen eager structural discovery (`docs/FILESYSTEM_MODULES_DESIGN.md` line 774 — “unused malformed reachable child source … is part of the compilation unit”). Recorded so it is not mistaken for a regression later. |

## G. Explicitly NOT changing (frozen / protected)

`playground/runtimes/**`, tag `v0.2.1`, released `VERSION`/`LANGUAGE_VERSION`
(`0.2.1`), the main oracle golden `tests/oracle/golden.tsv`, AST sharing
(`Arc`), runtime `Rc`, `.kilo/**`, `.codex/**`. No publication; no push.
Keystone metadata is Development-only until a human authorizes a runtime.

## G2. Disposition at Keystone closure (2026-10-06)

| Section B/C item | Disposition | Evidence |
|---|---|---|
| Optional narrowing | FIXED + REGRESSION TESTED | `Ty::None` retained; guards narrow; `tests/keystone_types.rs` |
| `never` | FIXED + REGRESSION TESTED | bottom type, union absorption, `E3006`; `tests/keystone_types.rs` |
| Raise-site span | FIXED + REGRESSION TESTED | both engines; oracle + R3C1 goldens |
| Catch selection | FIXED + REGRESSION TESTED | RFC 0001; `tests/catch_syntax.rs` |
| Builtin identity reservation | FIXED + REGRESSION TESTED | `E2023`; `tests/catch_syntax.rs` |
| Unused analysis | FIXED + REGRESSION TESTED | `E2008`; `tests/keystone_unused.rs`; corpus migrated |
| Structured diagnostics | FIXED + REGRESSION TESTED | `Presentation`; `src/diagnostic.rs`; CLI tests |
| CLI color policy | FIXED + REGRESSION TESTED | `AURA_COLOR`/`--color`/`NO_COLOR`; `tests/cli.rs` |
| Playground structured diagnostics | PARTIAL: Playground renders diagnostics as list items; Aurea provides `.au-diagnostic` primitives for severity mapping. Full component rewrite is future work | `playground/web/app.js`, `aurea.css` |
| Runtime codename/channel | FIXED + REGRESSION TESTED | build registry + manifest + selector; manifest tests |
| WASM output architecture | VERIFIED: bound is `BrowserHost` policy (root-caused); same-revision parity gate; chunked `OutputSink` not required by the frozen contract | `docs/playground.md`; differential 228/0 |
| Local `const` in bodies | FALSIFIED premise; spec §4.2 defines `const` as module/session scope; behavior matches | probes; spec |
| Shadowing | AUDITED, no change authorized; documented + tested | spec §16.3; `tests/shadowing.rs` |
| Exception RFC | ACCEPTED (`docs/rfcs/0001-…`) | RFC |
| HTTP capability | FIXED + REGRESSION TESTED | `http` feature; `tests/http.rs`; architecture doc |
| JSON→Struct typed decode | FIXED + REGRESSION TESTED | `json_decode_as` `E4031`; `tests/keystone_json.rs` |
| AIS/0.1 | IMPLEMENTED + TESTED | `src/ais.rs`; `aura ais`; `docs/engineering/AIS.md` |
| Aurea | IMPLEMENTED + TESTED | `website/assets/aurea.css`; `website/tests/aurea.test.mjs` |
| Overflow invariant | FIXED + REGRESSION TESTED | browser matrix 320–2560px + zoom + long token |
| REPL/Playground redesign | IMPLEMENTED (state machine + recovery) + TESTED | `playground/web/app.js`; browser/worker/multifile |
| Python/CPython + foreign values | INVESTIGATED; architecture record exists (`EMBEDDED_PYTHON_ARCHITECTURE.md`); no behavior change; provider seam is future work | docs |
| Tooling reconciliation | DISPOSITIONED (`TOOLING_RECONCILIATION.md`); formatter/LSP architecture; package manager human-gated | docs |
| Package-manager remote ecosystem | HUMAN-GATE REMAINING | PLANS.md; TOOLING_RECONCILIATION.md §5 |

## G3. Value-algebra closure (2026-10-07, super-transaction continuation)

| Item | Disposition | Evidence |
|---|---|---|
| Type families formalized as 3 identity levels | FIXED (normative spec §5.4) | semantic type vs type family vs runtime value kind, with the family table and the capability-does-not-erase-identity rule |
| Anti-collapse matrix executable | FIXED + REGRESSION TESTED | `tests/keystone_value_algebra.rs` 17/17: cross-kind equality, tuple-is-list-sugar, int/float numeric equality, display/JSON identity, per-kind indexing/iteration/patterns, static cross-kind rejection, scalar-only ordering |
| `json_encode` of unrepresentable kinds | REAL DEFECT FIXED | range/function/variant/non-finite float were silently `null`/payload (colliding with `none` and losing enum identity); now uniform `E3001`; `docs/engineering/JSON_VALUE_ALGEBRA_DECISION.md`; TD-22 closed |
| `Array`/`Set`/`Tuple` family premises | FALSIFIED WITH EVIDENCE (2026-10-06) — **SUPERSEDED 2026-10-08** | At the time this was recorded, Aura had no `Array` and no `Set` type and `Tuple` was list sugar (spec §21). The human Keystone decision later reopened this: ADR-0005 and spec §21 now define `List`/`Array`/`Tuple`/`Set`/`Map` as distinct identities, implemented end-to-end. See `docs/adr/0005-keystone-collection-model.md` |
| `range` as an annotation spelling | HUMAN-GATE REMAINING (recorded) | `Range` has no `Ty` member and `range` is not a legal annotation (`E3002`); ranges are dynamically typed (`Ty::Unknown`). Adding a spelling changes the annotation surface; recorded, not invented |

## G4. Assurance and AIX closure (2026-10-07, super-transaction continuation)

| Item | Disposition | Evidence |
|---|---|---|
| `;` as a statement separator | VERIFIED, NOT A DEFECT (campaign premise corrected) | `docs/LANGUAGE_SPEC.md` §2.4/§3.7 and `docs/grammar.md` (`terminator = NEWLINE \| ";"`) normatively define `;` as a statement terminator; CONF-PARSE-8 is CLOSED; `tests/syntax_conformance.rs` pins it (`{;;}` accepted, `1 2` and `fn a() {} fn b() {}` rejected). There is no array `;` syntax: `[1; 3]` is `E1006`. No grammar change |
| Playground FSM | REAL DEFECT FIXED + MATRIX | static markup initialized the status to the ad-hoc `idle`, not the machine's vocabulary, and the initial state was never entered through `enterState`; fixed; `browser.test.mjs` §13 pins legal transitions, forbidden edges, vocabulary closure, cycle closure. 98/0 |
| AIS delta on unrelated edits | REAL DEFECT FIXED | an inserted comment made every following symbol report `changed` (ranges compared); now `moved` vs `changed`, `is_empty` ignores moves; `tests/ais_properties.rs` + unit regressions |
| AIS/MCP adversarial surface | FIXED (new) | `fuzz/fuzz_targets/ais.rs`: 706,771 runs in 61 s, zero findings; `tests/ais_properties.rs` 10/10 properties |
| AIS/0.1 delivery + MCP adapter | IMPLEMENTED + DOCUMENTED | snapshot/slice/delta, content revision, three identity levels as fields, `aura mcp` transport-only adapter with bounded frames; `docs/engineering/AIS.md`, `MCP.md` |
| DX/AIX separation | NORMATIVE DOC | `docs/engineering/DX_AIX.md`; human prose is never the machine interface |
| Jev discovery | IMPLEMENTED, OPTIONAL, PROVEN FALLBACK | PATH scan only; `aura://capabilities`; a missing Jev leaves semantic output byte-identical |
| AIS differential dogfood | HARNESS IMPLEMENTED, EXTERNAL RUN NOT EXECUTED | `tests/ais_differential.mjs` verifies ground truth against the compiler and prints the exact command for a real evaluator; no model run was faked. Jev itself times out in this container |
| PokéAPI HTTP + typed-data dogfood | COMPLETED (deterministic) + LIVE OPT-IN | `tests/pokeapi_dogfood.rs` 6/6 over the real transport/decoders; live run blocked by the container's TLS throttle (recorded in `HTTP_ARCHITECTURE.md` §7) |
| Coverage baseline | MEASURED | `cargo llvm-cov --all-features`: 88.58% lines, 89.05% functions, 87.59% regions (ais.rs 93.01%, mcp.rs 92.27%) |
| Spec language version | REAL DRIFT FIXED + GUARDED | header said 0.0.1 while `LANGUAGE_VERSION` is 0.2.1; corrected and pinned by `tests/contract_sync.rs` |
| Website development-line claims | REAL DRIFT FIXED | "no active development line" contradicted the in-progress 0.3 line; corrected on roadmap/home/known-limitations and the stale runtime excerpt; website suite green |
| Dead production code / TODO markers | NONE FOUND | `clippy -D warnings` clean across all targets and features; zero `TODO`/`FIXME`/`HACK` in `src/` |

## H. Enforcement (this manifest)

Every item in sections B and C must end this transaction in exactly one state:
`FIXED + REGRESSION TESTED`, `FALSIFIED WITH EVIDENCE`,
`HUMAN-ACCEPTED LIMITATION`, `EXTERNALLY BLOCKED WITH REPRODUCTION`, or
`EXPLICIT FUTURE WORK WITH JUSTIFICATION`. No known remainder (order §43).
