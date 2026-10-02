# AURA COMPLETENESS MATRIX

> **Historical record (program start).** This matrix predates the ADR-0001…0004
> resolutions and the `0.2.1` release. Its "BLOCKED BY HUMAN DECISION" rows
> were true at the time and are preserved; HD-1/AUDIT-3 are now resolved. For
> current state see `AGENT_STATE.md`.

Derived from repository evidence at program start (local HEAD `fef6904`,
remote `bf95d10`). Authority order: `docs/LANGUAGE_SPEC.md` (normative),
`docs/FILESYSTEM_MODULES_DESIGN.md`, `docs/contract.md`, `docs/grammar.md`,
`docs/errors.md`, then design/roadmap/historical docs.

Classification legend:

- **COMPLETE** — implemented, tested, documented, end-to-end.
- **PARTIAL** — some part of the declared current surface is missing.
- **IMPLEMENTED BUT UNDERTESTED** — works, thin test coverage.
- **IMPLEMENTED BUT UNDERDOCUMENTED** — works, docs lag.
- **DOCUMENTED BUT MISSING** — spec/roadmap claims it, code does not.
- **DESIGNED BUT NOT IMPLEMENTED** — design exists, not built.
- **EXPERIMENTAL** — present but not part of core guarantees.
- **DEPRECATED/HISTORICAL**.
- **BLOCKED BY HUMAN DECISION**.
- **NOT PART OF CURRENT CORE** — deliberately absent per §34/§35.

## 1. Front end

| Capability | Status | Evidence |
|---|---|---|
| Lexing (ASCII idents, comments, newline/`;`) | COMPLETE | `src/lex/`, `tests/lexer.rs`, `syntax_tokens.tsv` (79 variants) |
| Numeric literals (dec/hex/bin/oct, floats, `_`) | COMPLETE | `numeric_edges`; underscore placement SPEC GAP pinned |
| Strings / char quotes / f-strings / format mini-language | COMPLETE | `tests/lexer.rs`, `syntax_conformance` |
| Multiline comments `<!-- -->` | COMPLETE | `tests/run.rs`, lexer |
| Parser (recursive descent + Pratt) | COMPLETE | `tests/parser.rs` (48) |
| Grammar/impl agreement | COMPLETE | `tests/syntax_docs.rs` byte-diff of token set |
| Nested delimiters, trailing commas, one-element lists | COMPLETE | `syntax_conformance`, `delimiter_comma_matrix` |
| AST depth bound (256, `E1015`) | COMPLETE | `parse/mod.rs`, `boundaries.rs` |

## 2. Name/resolution/scope

| Capability | Status | Evidence |
|---|---|---|
| Reserved words | COMPLETE | `tests/grammar.rs`, `parser.rs` |
| Builtin-name reservation (value namespace) | COMPLETE | `tests/builtin_reservation.rs`, hardening c8ded06 |
| Shadowing (`let`/`let mut`) | COMPLETE | `tests/shadowing.rs` (15) |
| Redeclaration (`const`/fn/param) | COMPLETE | `contract.rs`, `adversarial.rs` |
| Scope matrix | COMPLETE | `LANGUAGE_SPEC.md` §16.3 |
| Mutability capability model | COMPLETE | `tests/mutation.rs` (16), §16.6 |
| HD-1 module-member × builtin namespace | BLOCKED BY HUMAN DECISION | module members may reuse builtin spelling; separate namespace is consistent; formal decision queued |
| Type namespace separation | COMPLETE | `builtin_reservation.rs` separate-namespace tests |

## 3. Types / expressions

| Capability | Status | Evidence |
|---|---|---|
| Static type universe (`Ty`) + `Unknown` boundary | COMPLETE | `types.rs`, §2.3 |
| Union types + alias composition | COMPLETE | roadmap; `tests/generics.rs`, `maps.rs` |
| Type aliases (chained, recursive rejected `E3002`) | COMPLETE | `tests/maps.rs` |
| Field-type propagation | COMPLETE | Feature 003, `tests/checker.rs` |
| Collection static precision | COMPLETE | §21.1 |
| Generics (static, erased, nominal) | COMPLETE | `tests/generics.rs` (27), §36, `GENERICS.md` |
| Generic bounds | COMPLETE | §36.4 |
| TypeExpr nesting policy | BLOCKED BY HUMAN DECISION (AUDIT-3) | decision package; native 2048 / wasm 768 |

## 4. Functions / closures / calls

| Capability | Status | Evidence |
|---|---|---|
| Top-level `fn`, hoisting, mutual recursion | COMPLETE | `tests/run.rs`, `regressions.rs` |
| First-class functions, closures, capture | COMPLETE | `tests/run.rs` |
| Mutable captures | COMPLETE | `tests/run.rs`, `mutation.rs` |
| Lambdas (shared param model) | COMPLETE | `tests/run.rs`, `adversarial.rs` |
| Positional args + arity (checker+runtime) | COMPLETE | `contract_sync.rs` |
| Named arguments (top-level direct calls) | COMPLETE | Feature 002, `tests/regressions.rs` |
| Function/method overloading | COMPLETE | `tests/overloading.rs` (28) |
| Static argument type checks | COMPLETE | Feature 001 |
| Dynamic-call boundaries defer to runtime | COMPLETE | §34.2 |
| Named method arguments | NOT PART OF CURRENT CORE (POST-CORE RFC) | §34.1; parse-then-`E3001` confirmed |
| Default/variadic args | NOT PART OF CURRENT CORE | §34.1; rejected at parse (confirmed) |
| Nested named functions | NOT PART OF CURRENT CORE | §34.1 |

## 5. OOP / data

| Capability | Status | Evidence |
|---|---|---|
| Structs (decl/construct/access/equality) | COMPLETE | `tests/run.rs`, `oop.rs` |
| Struct methods (`impl`, `self`/`mut self`) | COMPLETE | `tests/methods.rs` |
| Traits (static contracts, bounds) | COMPLETE | `tests/traits.rs` (18), §17.7 |
| Enums (tags, payloads, equality) | COMPLETE | `tests/run.rs` |
| Match patterns (literal/binding/list/variant/guard) | COMPLETE | `tests/run.rs`, §19 |
| Destructuring `let` | COMPLETE | Feature 004 |
| Enum methods | NOT PART OF CURRENT CORE | §35; `E2003` confirmed |
| Tuple type | NOT PART OF CURRENT CORE (list sugar) | §34.1 |
| List-rest patterns | NOT PART OF CURRENT CORE (RFC) | §34.1 |

## 6. Collections / control flow

| Capability | Status | Evidence |
|---|---|---|
| Lists (index/mutate/iterate) | COMPLETE | `tests/collections.rs` |
| Maps (scalar keys, ordered) | COMPLETE | `tests/maps.rs` (27) |
| Empty-map `{:}` | COMPLETE | Feature 005 |
| Comprehensions (list/map) | COMPLETE | `tests/comprehensions.rs` (15), §21b |
| `map.items()` | COMPLETE | §21.2 |
| if/else-if/else/while/loop/for/break/continue | COMPLETE | `tests/run.rs` |
| return/throw/try/catch/finally | COMPLETE | `tests/catch_syntax.rs`, §14 |
| Pipeline `\|>` | COMPLETE | §23 |
| Ranges (`range`, `a..b`, lazy) | COMPLETE | §22 |
| Range step / `..=` / `for…else` | NOT PART OF CURRENT CORE (RFC) | §34.1; rejected (confirmed) |

## 7. Operators / numeric

| Capability | Status | Evidence |
|---|---|---|
| Arithmetic, comparison, logical, bitwise | COMPLETE | `syntax_conformance`, §9 |
| Compound assignments | COMPLETE | §9.4 |
| Numeric model, overflow `E4013`, div0 `E4007` | COMPLETE | §10, `boundaries.rs` |
| NaN/Inf/-0, promotion | COMPLETE | §10.3–10.8 |
| Conversions (`to_int`/`to_float`) | COMPLETE | §10.9; `to_int` refs in `tests/` |
| `++`/`--` | NOT PART OF CURRENT CORE (deliberate) | §9; roadmap decision |

## 8. Stdlib

| Capability | Status | Evidence |
|---|---|---|
| Core builtins (38) | COMPLETE | `signatures.rs`, `stdlib/mod.rs` |
| Every builtin has a signature + native | COMPLETE | registry-derived property test |
| Per-builtin edge-case tests | IMPLEMENTED BUT UNDERTESTED | `min`/`max` have **no direct test** (value semantics documented but unpinned); `abs`/`zip` thin |
| Method entries (string/list/map/range) | COMPLETE | `tests/methods.rs` |
| Registry/runtime non-drift | COMPLETE | `property_hardening.rs` PROPERTY 1 |
| `min`/`max` documented type-independent order | COMPLETE (docs) / UNDERTESTED | §25 row; behavior confirmed in review |
| JSON builtins | COMPLETE | `ext.rs`, §25 |
| Regex builtins | COMPLETE | `ext.rs` |
| Time builtins | COMPLETE | `ext.rs` |
| File/host builtins + host boundary | COMPLETE | `host.rs`, `tests/io.rs` (20), `host.rs` tests (15) |
| Python bridge (`py`) | COMPLETE (feature-gated) | `src/bridge/`, `tests/python.rs` |

## 9. Modules

| Capability | Status | Evidence |
|---|---|---|
| In-source modules, nesting | COMPLETE | `tests/modules.rs` (20), §27 |
| Visibility (`pub`, private `E2018`) | COMPLETE | §27 |
| `use`, aliases, re-exports | COMPLETE | §27 |
| Provider-neutral graph | COMPLETE | `module_graph.rs` (36) |
| Native filesystem provider | COMPLETE | `native_source.rs` (33) |
| InMemory/virtual provider | COMPLETE | `provider_parity.rs` |
| WASM virtual project | COMPLETE | playground differential |
| Eager structural discovery | COMPLETE | `FILESYSTEM_MODULES_DESIGN.md` |
| Collision/case/symlink/cycle handling | COMPLETE | `native_source.rs`, `e17e37c` |
| Provider parity (native/InMemory/WASM) | COMPLETE | `provider_parity.rs`, differential |
| Package/project tooling, URL imports | NOT PART OF CURRENT CORE | deferred (PLANS.md) |

## 10. Tooling / product

| Capability | Status | Evidence |
|---|---|---|
| CLI (`run`/`check`/`eval`/`repl`/`version`) | COMPLETE | `tests/cli.rs` (33), §28 |
| Entry/main rules, stdin `-`, exit codes | COMPLETE | `cli.rs`, §28 |
| REPL persistence (bindings/fns/structs/enums/aliases/traits/consts) | COMPLETE | `tests/repl.rs` (52), §29 |
| REPL rollback on failure | COMPLETE | `repl.rs` |
| REPL generics support | COMPLETE | §36.8 |
| Source provenance / diagnostics | COMPLETE | `source_provenance.rs` (14) |
| Error model / stable codes / reachability | COMPLETE | `errors.md`, §30 |
| Resource/host safety limits | COMPLETE | §31, `boundaries.rs` (30) |
| Determinism | COMPLETE | `syntax_conformance` deterministic test |
| Playground single-file | COMPLETE | `browser.test.mjs` |
| Playground multi-file (FSM-P6) | COMPLETE | `multifile.test.mjs` 42/42 |
| Playground project state model | COMPLETE | `project.test.mjs` 36/36 |
| Playground cache/integrity/ABI | COMPLETE | `cache`/`integrity`/`abi` suites |
| Website + examples conformance | COMPLETE | `website/tests/run-all.mjs`, `examples.rs` |
| Native/WASM parity (differential) | COMPLETE | `differential.test.mjs` 214/214 |
| Frozen artifact immutability | COMPLETE | `build.mjs` pins; verified hashes |
| Cross-platform CI (Linux/macOS/Windows) | COMPLETE | `.github/workflows/ci.yml` |
| MSRV 1.83 gate | COMPLETE | CI `msrv` job |
| Miri / cargo-audit | COMPLETE | CI `miri`/`audit` jobs (audit non-blocking) |
| Fuzz targets (lexer/parser/checker/runtime) | COMPLETE | `fuzz/`, CI smoke + nightly |
| Property suite | COMPLETE | `property.rs`, `property_hardening.rs` |
| `E5003` unused code / `E4099` internal | COMPLETE (documented) | §34.3 |

## Burn-down summary

```
COMPLETE:                       ~135 rows
IMPLEMENTED BUT UNDERTESTED:       1  (per-builtin edge tests: min/max)
IMPLEMENTED BUT UNDERDOCUMENTED:   0
PARTIAL:                           0
DOCUMENTED BUT MISSING:            0
DESIGNED BUT NOT IMPLEMENTED:      0
EXPERIMENTAL:                      0
BLOCKED BY HUMAN DECISION:         2  (HD-1, AUDIT-3)
NOT PART OF CURRENT CORE:         12  (deliberate)
DEPRECATED/HISTORICAL:             0
```

**No accidental PARTIAL/MISSING current-core item found.** The only actionable
gap is test depth for `min`/`max` edge behavior (M5 work).
