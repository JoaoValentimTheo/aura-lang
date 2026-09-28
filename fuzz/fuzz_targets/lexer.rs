#![no_main]
//! Fuzz target: the lexer.
//!
//! INVARIANT: no byte input reaches a Rust panic, abort, or stack overflow;
//! every input terminates and yields either a token stream or a structured
//! `E1xxx` diagnostic. A process-level failure (panic, abort, sanitizer
//! report, timeout) is a finding.
//!
//! Input model: **arbitrary bytes** (the lexer's real attack surface). The
//! bytes are passed through `String::from_utf8_lossy`, to exercise valid text containing replacement characters. The CLI and WASM
//! source-byte boundary instead reject invalid UTF-8 via `decode_source`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = aura::lex::decode_source(data);
    let src = String::from_utf8_lossy(data);
    // The lexer must always return; a panic propagates and libFuzzer records it.
    let _ = aura::lex::lex(&src);
});
