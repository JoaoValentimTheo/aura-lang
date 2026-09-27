#![no_main]
//! Fuzz target: the checker (on grammar-aware ASTs).
//!
//! INVARIANT: no input reaches a Rust panic, abort, or stack overflow; every
//! input terminates and yields either a successful check or a structured
//! `E2xxx`/`E3xxx` diagnostic.
//!
//! ## Two input paths, both required
//!
//! 1. **Source bytes -> parser -> checker.** The bytes are lexed and parsed; a
//!    parse failure is discarded (it exercises the front end, covered by the
//!    `parser` target), and a parsed module is resolved then checked.
//!
//! 2. **Direct grammar-aware AST generation.** The bytes also seed
//!    [`ast_gen`], which builds a structurally valid [`aura::ast::Module`]
//!    directly and hands it to the checker without going through the parser.
//!    This reaches checker states a random byte stream almost never produces
//!    (deeply mixed statement/expression shapes, unusual but legal nesting,
//!    empty bodies, adjacent generic/alias/enum declarations).
//!
//! ## Checker rejection is not silently discarded
//!
//! A generated AST may legitimately be type-incorrect, so an `Err` diagnostic
//! is a valid result and is *not* a fuzz failure. What must never happen is a
//! panic, an abort, or an `INTERNAL` (`E4999`) diagnostic — the latter means a
//! checker bug. The target asserts on that distinction rather than swallowing
//! the outcome.

use libfuzzer_sys::fuzz_target;

#[path = "ast_gen.rs"]
mod ast_gen;

fn seed_from(data: &[u8]) -> u64 {
    let mut seed: u64 = 0;
    for (i, b) in data.iter().take(8).enumerate() {
        seed |= u64::from(*b) << (8 * i);
    }
    seed
}

fuzz_target!(|data: &[u8]| {
    // Path 1: source bytes -> parser -> resolve -> checker.
    {
        let src = String::from_utf8_lossy(data);
        if let Ok(module) = aura::parse::parse(&src) {
            if let Ok(module) = aura::resolve::resolve(module) {
                let _ = aura::check::Checker::module(&module);
            }
        }
    }

    // Path 2: direct grammar-aware AST.
    {
        let seed = seed_from(data);
        let module = ast_gen::module(seed);
        if let Err(d) = aura::check::Checker::module(&module) {
            // An INTERNAL diagnostic means the checker itself panicked on an
            // input it should reject structurally; that is a real finding.
            assert_ne!(
                d.code,
                aura::error::codes::INTERNAL,
                "checker reported INTERNAL (E{}) on a generated AST (seed {seed}): {}",
                d.code,
                d.message
            );
        }
    }
});
