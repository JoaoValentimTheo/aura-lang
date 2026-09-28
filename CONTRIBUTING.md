# Contributing to Aura

Thanks for helping. Aura is a from-scratch Rust implementation of a small
language. Correctness and clarity matter more than feature count.

## The contract

`docs/LANGUAGE_SPEC.md` is authoritative for current syntax and semantics.
`docs/grammar.md` is its canonical EBNF representation; `docs/contract.md`
records compatibility guarantees and defers to the specification. Guides and
website references explain the current contract. Roadmaps, design reports and
past audits are evidence of history, not overrides. Accepted RFCs amend the
current documents when implemented; proposals are not current syntax.

When these sources disagree, reproduce and classify the discrepancy before
changing behavior. Update the affected contract, grammar, guides and tests in
the same change. `tests/grammar.rs` and `tests/syntax_conformance.rs` exercise
the grammar; `tests/syntax_docs.rs` guards the duplicated grammar and inventory.

## Workflow

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features --features cli,repl,json,regex,time
```

CI runs all of the above on Linux, macOS, and Windows, plus Miri, an MSRV
check, `cargo audit`, and an extended property-test run.

## Rules for the language

1. **One spelling per construct.** Adding a synonym is a regression.
2. **Mutable means mutable.** Never weaken the `let` / `let mut` distinction.
3. **No silent coercion.** Conversions go through `to_int`, `to_float`,
   `to_string`.
4. **Every rejection has a stable code.** Add the code to `src/error.rs`,
   `docs/errors.md`, and `tests/grammar.rs`.
5. **`unsafe` is forbidden.** The crate denies it (`unsafe_code = "deny"`).

## Changing the language

Because syntax is a contract, a syntax change requires an RFC: open an issue
titled `RFC: <change>` describing the motivation, the exact grammar delta,
the migration path for existing programs, and the tests that will enforce it.
Accepted RFCs are recorded in `docs/rfcs/`.

## Tests

* `tests/lexer.rs`, `tests/parser.rs` — front-end unit tests.
* `tests/run.rs` — end-to-end execution.
* `tests/contract.rs` — the normative promises.
* `tests/grammar.rs` — docs-as-tests for grammar and errors.
* `tests/property.rs` — fuzzing and invariants (Proptest).
* `tests/examples.rs` — every `examples/*.aura` must run; `.out` files, when
  present, are compared byte-for-byte.
* `tests/python.rs` — the PyO3 bridge, only with `--features py`.

## Style

Rust style is `rustfmt`. Clippy runs at `pedantic` with correctness lints
(`unwrap_used`, `expect_used`, `panic`) denied. A small allow-list in
`Cargo.toml` disables purely stylistic lints; additions to it should be
justified in the commit message.

## Commits

Use conventional-commit subjects (`feat:`, `fix:`, `docs:`, `test:`,
`refactor:`, `chore:`). Keep the body focused on *why*.
