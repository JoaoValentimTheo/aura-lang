---
description: Validate current Aura changes and artifact invariants without broadening scope.
agent: aura-orchestrator
---

Validate the current Aura worktree. Start with focused tests relevant to the
diff, then run the applicable canonical checks:

- `cargo fmt --all -- --check`
- `cargo test --locked --all-targets --all-features`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time`
- `node playground/tests/node/run-all.mjs`
- `node playground/build.mjs --check`
- `node website/tests/run-all.mjs`

Delegate independent artifact verification to `aura-artifact-verifier` and
adversarial review to `aura-adversarial-reviewer`. Do not fix unrelated
findings; report them separately. Never call local checks "GitHub CI".
