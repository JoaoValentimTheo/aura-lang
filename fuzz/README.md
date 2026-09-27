# Aura fuzz toolchain

This directory holds the four libFuzzer targets that exercise the frozen
language line:

| Target    | Input model                                                       |
|-----------|-------------------------------------------------------------------|
| `lexer`   | arbitrary bytes → lexer                                           |
| `parser`  | arbitrary source bytes → lexer → parser                           |
| `checker` | source bytes → parser → resolver → checker, **and** a direct grammar-aware AST (see `fuzz_targets/ast_gen.rs`) |
| `runtime` | grammar-aware generated, checker-passing programs (cycle/aliasing surface) |

## Reproducing locally

The fuzz targets require a nightly toolchain (libFuzzer + sanitizer) and a
pinned `cargo-fuzz`. Versions are declared in this repository, not assumed:

```sh
# Toolchain: the version pinned in ../rust-toolchain.toml (channel + components).
cat ../rust-toolchain.toml

# cargo-fuzz: the exact version is pinned here and must match CI.
cargo install cargo-fuzz --version 0.13.2 --locked

# libFuzzer itself is supplied by the nightly toolchain's `compiler_builtins`
# / libfuzzer-sys; libfuzzer-sys is pinned in fuzz/Cargo.lock (do not upgrade
# without re-running the full smoke suite).
```

Then, from the repository root:

```sh
cargo +nightly fuzz run <target> -- -max_total_time=60 -rss_limit_mb=6144 -timeout=25
```

## Sanitizer scope (not a blanket disable)

ASan is **always** on. LeakSanitizer is left **on** for `lexer`, `parser`, and
`checker`. It is scoped **off only for the `runtime` target**, and only because
Aura's documented, memory-safe `Rc` model does not reclaim a reference cycle
(`LANGUAGE_SPEC.md` §31.6): a program that builds a cycle legitimately retains
that memory, so LeakSanitizer would report a false positive. Crashes, aborts,
and timeouts are still hard failures on every target.

## Crashes

A crash artifact (`fuzz/artifacts/<target>/crash-*`) is never committed. It is
reproduced, minimised, converted into a permanent fixture under
`tests/corpus/`, and covered by a regression test before the finding is closed
(see `tests/corpus/README.md` §6).

`crash-*` and `leak-*` artifact filenames embed the SHA-1 of the crashing input
(libFuzzer's own naming convention — unrelated to Aura's SHA-256 artifact
integrity).
