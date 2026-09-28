#![no_main]
//! Fuzz target: the lexer + parser (the production front-end path).
//!
//! INVARIANT: no byte input reaches a Rust panic, abort, or stack overflow;
//! every input terminates and yields either an AST or a structured `E1xxx`
//! diagnostic. The AST nesting backstop must be respected: an over-deep input
//! is `E1015`, never a host failure.
//!
//! Input model: **arbitrary source bytes**, because the real production path is
//! `source bytes → lexer → parser`. A token-stream target would bypass the
//! lexer and is deliberately not used as the primary input.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = aura::lex::decode_source(data);
    let src = String::from_utf8_lossy(data);
    let _ = aura::parse::parse(&src);
});
