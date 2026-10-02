# Permanent resource corpus

## 1. Purpose

This directory holds the **permanent** minimal reproducers and boundary
fixtures for Aura's host-safety and resource contracts. Its purpose is to keep
every confirmed finding's smallest reproduction, and every documented boundary,
**executed on every CI run** — so a regression is caught immediately, not
rediscovered by a future audit.

A fixture is not documentation of intent; it is executable coverage. Every file
here runs in CI (`tests/corpus.rs`, in the `test` job) and is checked against an
explicit expected outcome.

## 2. Directory structure

```text
tests/corpus/
├── README.md            this policy
├── ast/                 AST-node nesting boundaries and generated programs
├── call-frames/         call-depth boundaries (simple, mutual, generic, else,
│                        match, method, closure, module, try)
├── cycles/              value-model cycles and shared (DAG) references
├── generics/            generics edge cases (overload, union, alias, bounds, …)
├── overload/            overload resolution (ties, identity, determinism)
├── ranges/              lazy range iteration and the materialization boundary
├── traits/              static trait contracts and bound checking
├── aliases/             transparent alias rules
├── unions/              union annotation semantics
├── unicode/             combining characters, emoji, BOM, CRLF/LF/CR
└── visibility/          module visibility (private/public fields and functions)
```

The layout mirrors the audit's concern areas. Categories are fixed; add a
fixture to the matching directory rather than creating a new top-level one
without a reason.

## 3. How fixtures are executed

`tests/corpus.rs` runs every fixture through `aura::run_source` and compares the
outcome to a table:

* `Expected::Ok("…")` — the program runs and its stdout must be exactly this;
* `Expected::Err("E####")` — the program is rejected with exactly this code
  (compared by code, never by message prose, which is not a stable contract).

The table and the directory are checked **in both directions**: a fixture on
disk with no table entry fails the suite, and a table entry with no fixture
fails the suite. This makes silent skipping impossible.

Run it with the canonical command:

```sh
cargo test --locked --all-features --test corpus
```

## 4. Expected-result conventions

* Record the **code**, not the diagnostic text, for rejections.
* Record **exact stdout** for accepted programs, including the trailing
  newline.
* A runtime diagnostic (`E4xxx`) is a valid expected outcome; a panic, abort,
  trap, or timeout is never.
* Keep each fixture as small as it can be while still exercising the boundary
  or finding. A reproducer that is one statement shorter is a better fixture.

## 5. How to add a new fixture

1. Add a minimal `.aura` file to the matching category directory.
2. Determine its outcome with the release binary:
   `./target/release/aura run tests/corpus/<category>/<name>.aura`.
3. Add its expected result to the `table()` in `tests/corpus.rs`.
4. Run `cargo test --locked --all-features --test corpus` — it must pass, and
   the inventory test must still agree with the directory.

## 6. Rule: every confirmed finding becomes a permanent fixture

Whenever a new finding is confirmed (by fuzzing, property testing, audit, or a
bug report), its **smallest reproducer** MUST be added here before the change is
closed, together with a regression test in the relevant `tests/*.rs` file. The
fixture is the durable, CI-executed record; the finding ID belongs in the
fixture name or its `tests/corpus.rs` entry's category.

## 7. Rule: the corpus only grows

The corpus is append-mostly. A fixture is removed only when it is explicitly
**deprecated** with a documented reason (for example, the language rule it
tested was removed). Silent deletion, or leaving a fixture on disk unexecuted,
is not permitted — the inventory test enforces both directions.

## 8. TypeExpr nesting — resolved, tested elsewhere

Type-annotation nesting is governed by ADR-0004: structural `TypeExpr` nesting
counts toward the `MAX_AST_DEPTH = 256` semantic limit on every substrate. The
boundary is pinned explicitly by `tests/boundaries.rs` (N-1/N/N+1 across every
annotation position, plus the flat-union exemption) and by the differential
harness's TypeExpr sweep, rather than duplicated as a corpus fixture. See
`docs/adr/0004-typeexpr-nesting-policy.md`.
