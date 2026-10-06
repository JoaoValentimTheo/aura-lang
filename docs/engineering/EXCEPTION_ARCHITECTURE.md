# Aura Exception Architecture (Pre-0.3 Foundation)

**Status: DESIGN-ONLY. No public syntax is chosen here.** This document is the
Pre-0.3 foundation for exception families and custom exceptions. Every syntax
proposal below is explicitly non-normative and requires a human language-design
decision before implementation. Nothing in this document changes current
behavior.

**Even the taxonomy itself carries no normative weight.** The family names,
the tree shape, the number of families, and the level of hierarchy in §3 are
placeholders for discussion; only the *separation of levels* in §2 and the
security constraints in §7 are architectural commitments, because they merely
restate existing behavior and non-negotiable safety properties. Do not cite
§3 as a specification; cite §8's decision list.

Authorities: `docs/LANGUAGE_SPEC.md` §14.5–14.6 (current `throw`/`try`)
remains the normative answer for today's semantics; this file is the design
record for what 0.3 may add.

---

## 1. Current reality (verified)

- `throw e` raises the value of `e`; `try { … } catch x { … }` catches any
  explicit `throw` raised in the body, including through called functions, and
  binds `x` to the thrown value (`Stmt::Try { catch: String, catch_body,
  finally_body }`, `src/ast/mod.rs:570`).
- Only explicit `throw` is catchable. Every runtime diagnostic is fatal and
  propagates through `try`; `finally` still runs (`LANGUAGE_SPEC.md` §14.5).
- Diagnostics carry stable phase-grouped codes (`src/error.rs:256`): `E1xxx`
  lexical/syntactic, `E2xxx` static, `E3xxx` type-level, `E4xxx` runtime,
  `E5xxx` capability/feature.
- The evaluator distinguishes inside `try` regions:
  `TryResult::Completed(Vec<Stmt>)` vs `TryResult::Fatal(Diag)`
  (`src/run/iterative.rs:224-230`); `Cont::TryCatchEnd`/`Cont::TryFinally`
  snapshot frames/depth so an error raised in a catch/finally body cannot leak
  frames.
- `Ctl::Throw(Value)` is the control signal; `E4026` is the uncaught-throw
  diagnostic; `E4099` is the internal cross-frame signal, never user-visible
  (except via internal-error reporting).

An exception family model must be **additive** to all of this.

---

## 2. Separation of five failure levels

| Level | Produced by | Catchable? | Examples | Current code family |
|---|---|---|---|---|
| **Compile/check diagnostic** | lexer, parser, checker, module graph | No (no program runs) | syntax, undefined name, type mismatch | `E1xxx`–`E3xxx` (+ module `E2020-2022`) |
| **Runtime diagnostic** | evaluator value operations | **No (fatal)** | overflow, division by zero, index out of range, no match, not iterable | `E4xxx` |
| **Catchable language exception** | explicit `throw` today | Yes | user-thrown values | `E4026` only if uncaught |
| **Host/resource failure** | host boundary and limits | **No (fatal)** | stdout bound `E4020`, capability `E5002`, frame limit `E4011`, interp internal `E4999` | `E4xxx`/`E5xxx` |
| **Internal interpreter failure** | invariant violation | **Never** | `E4999`, aborts | `E4999` |

**Normative commitment for 0.3:** the new exception hierarchy must NOT make
level 2, 4, or 5 catchable. Security and resource enforcement are enforcement,
not program-flow. Specifically, the following stay fatal and suppressible-free:

- `E4011` call-frame limit (resource)
- `E4020` stdout/stdin/args/filesystem I/O failure and the browser capture
  bound (resource + host)
- `E5001`/`E5002` Python/capability failures (host)
- `E4999` internal errors (never user-visible as recoverable)
- all `E1xxx`–`E3xxx` (there is no running program to catch them)
- `E1000`-family parse/pre-execution `E1015` nesting

If a future profile wants to catch optimizer/resource signals, that is a
separate, explicit policy decision with its own evidence — it is not granted
here.

---

## 3. Proposed family taxonomy (NON-NORMATIVE PROPOSAL)

The working sketch, to be reconciled with the existing diagnostics before any
syntax decision:

```text
Exception                     (root; message + optional payload + span)
├── RuntimeError
│   ├── DivisionByZeroError   (mirrors E4007)
│   ├── OverflowError         (mirrors E4013)
│   ├── IndexError            (mirrors E4019)
│   ├── KeyError              (missing map key, E2003/E4019 surface)
│   ├── NoMatchError          (mirrors E4029)
│   ├── NotIterableError      (mirrors E4018)
│   └── AssertionError        (mirrors E4028)
├── TypeError                 (mirrors E3001 runtime application)
├── NameError                 (mirrors E2003 runtime lookup)
├── ValueError                (invalid conversion/argument)
├── AttributeError            (unknown member, mirrors E2003 member surface)
├── ImportError               (module load failure at runtime)
├── IOError                   (host I/O, mirrors E4020 — see §2: NOT catchable
│                              when it is a resource/host failure)
├── ResourceError             (limits; NOT catchable by default)
└── User exceptions           (declared by the program; see §4)
```

Open questions this proposal does NOT settle:

1. Whether families are nominal types, tagged values, or both.
2. Whether builtin family names are reserved words, builtin types, or plain
   imported names; and how they interact with ADR-0002 (module members may
   reuse builtin spellings).
3. Whether the hierarchy root is `Exception` or the current untyped value is
   grandfathered as the root.
4. Whether crossing `py` remains `E5001`/`E5002` fatal or gains an
   `ImportError` mirror.

Each is a human-gated language-design decision (§26 of the transaction).

---

## 4. Custom exceptions (design requirements; syntax NOT chosen)

Desired properties, consistent with the type system and evaluator:

- **Family relationship**: a user exception can belong to a family so a
  catch can select it (if inheritance is chosen, it must match the type
  system's nominal rules; if tagging is chosen, no inheritance is implied).
- **Message**: a human-readable string, always present.
- **Optional structured payload**: any Aura value, preserved exactly.
- **Source/span preservation**: the raising site's source identity and span
  survive to the catch and to any uncaught report (the machine already tracks
  `SourceId`+`Span` per `Diag`; a structured throwable must carry the same).
- **Pattern/type-based catching**: catch can discriminate by family; whether
  the syntax is `catch e if e.family == X`, `catch Name as e`, a `match` over
  the throwable, or a typed catch is an open syntax decision.
- **Predictable display**: uncaught rendering must be deterministic and
  identical on native and WASM (same rule as value display, `MAX_VALUE_DEPTH`
  bounded).
- **Deterministic equality policy**: where equality is defined for throwables,
  it must be structural and substrate-independent; if equality is not
  meaningful, it must be explicitly unsupported rather than accidental.
- **Safe propagation through native/WASM**: a throwable must cross call
  frames, module boundaries, and the machine's `TryResult`/`Ctl::Throw`
  without host-stack growth; the machine's existing snapshots
  (`frames_len`/`depth`/`saved_expr_depth`) apply unchanged.
- **No dependence on Python**: the model must be pure Aura; `py` may *mirror*
  Python exceptions into it (and back) but cannot define it.

Syntax candidates (for the human decision, none normative):

```aura
# A: tagged value, minimal
throw ValueError { message: "bad input", payload: 42 }

# B: typed throw with a declared name
throw MyError("bad input", 42)

# C: declaration + construction
struct MyError : Exception { code: int }
throw MyError { code: 7 }
```

`throw` currently accepts any expression; option A is the only one that is
purely additive to the grammar. Options B/C introduce new declaration syntax.

---

## 5. Catchability matrix (target for 0.3, subject to the human decision)

| Throwable | Catchable today | Proposed |
|---|---|---|
| explicit `throw <value>` | yes | yes |
| structured language exception | n/a | yes |
| user exception | n/a (plain value today) | yes |
| runtime diagnostic (`E4xxx` operation) | no | **still no** |
| host/resource/capability (`E4020`,`E4011`,`E5002`) | no | **still no** |
| internal (`E4999`) | no | **never** |

---

## 6. Migration-safe path (additive)

1. **Model first (no syntax)**: introduce an internal `Throwable` shape
   `{ family: Tag, message: String, payload: Value, location: Location }`
   behind the existing `Ctl::Throw(Value)`; a plain thrown value maps to a
   wildcard family so today's programs behave identically.
2. **Family registry**: builtin family identifiers + codes mirrored from
   diagnostics (one table; no duplicated semantic rules).
3. **Catch selector**: extend catching to test the family tag; a plain
   `catch e` keeps catching everything (backward compatible).
4. **Public syntax**: only after the human decision; introduced as one
   additive change with conformance, oracle, and compatibility coverage.

Every step must keep `tests/oracle/golden.tsv` byte-identical unless the
step is itself an intended, human-approved semantic addition.

---

## 7. Security constraints (non-negotiable in the design)

- Resource enforcement never becomes catchable.
- A catch handler cannot observe or suppress host policy decisions
  (`E5002`, capture bound) other than through normal program termination.
- Exception payloads obey the existing value bounds (`MAX_VALUE_DEPTH`,
  `MAX_VALUE_NODES`, render budgets); a payload cannot be used to force
  unbounded memory or render work.
- WASM parity: family matching and payload propagation are pure evaluator
  behavior; nothing depends on host facilities.

---

## 8. Decisions requiring human authorization

| # | Decision | Why gated |
|---|---|---|
| E1 | Syntax for constructing a structured exception | new public syntax |
| E2 | Syntax for catching by family | new public syntax |
| E3 | Whether families are nominal types, tags, or both | type-system semantics |
| E4 | Whether any runtime diagnostic family becomes catchable (proposal: no) | breaking compat / security |
| E5 | Whether `py` exceptions gain an Aura family mirror | cross-language guarantee |
| E6 | Reserved-name policy for builtin family names (ADR-0002 interaction) | compatibility |

Until E1–E6 are answered, this architecture constrains *how* exceptions may be
added, not *whether* or *when*.
