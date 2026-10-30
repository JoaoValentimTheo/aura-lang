## Summary

Brief description of the changes.

## Related issues

Closes #...

## Changes

- ...

## Checklist

Check what you actually ran, and say where. A command listed without its
result is not evidence; CI is the only place a remote gate becomes real.

- [ ] Tests pass (`cargo test --locked --all-targets --all-features`)
- [ ] Lint clean (`cargo fmt --all -- --check` and
      `cargo clippy --locked --all-targets --all-features -- -D warnings`)
- [ ] Pure-Rust build unaffected
      (`cargo test --locked --all-targets --no-default-features --features cli,repl,json,regex,time`)
- [ ] Playground suites pass (`node playground/tests/node/run-all.mjs`), plus
      `node playground/build.mjs --check` if a runtime artifact changed
- [ ] Website suites pass (`node website/tests/run-all.mjs`), or the full
      sequence in `.github/workflows/pages.yml` if `website/` changed
- [ ] Documentation updated (if applicable)
- [ ] CHANGELOG.md updated (if applicable)
- [ ] No unintended change to observable language behavior, the Host ABI, or
      a frozen runtime artifact

## Fixing a red gate

State the failure, the root cause, and whether it predates your change. A red
gate on your PR is not automatically your bug — prove it on the base commit
before "fixing" it. Never remove a test, weaken a warning, or skip a check to
get green.
