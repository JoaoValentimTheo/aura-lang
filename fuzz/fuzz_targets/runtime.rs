#![no_main]
//! Fuzz target: the safe runtime / interpreter.
//!
//! INVARIANT: no program reaches a Rust panic, process abort, or non-terminating
//! loop; every generated program terminates with either a normal runtime result
//! or a structured `E4xxx`/`E5xxx` diagnostic.
//!
//! ## Input model
//!
//! Grammar-aware generation, not raw bytes: the bytes seed the shared
//! deterministic generator (`tests/support/program_gen.rs`, included via
//! `#[path]`), which emits programs that **pass the checker by construction**.
//! This is required, because raw-byte fuzzing mostly rediscovers checker
//! rejection and never reaches the interpreter. The generator's contract and
//! the AST invariants it respects are documented in its module header.
//!
//! ## Aliasing / cycle surface
//!
//! The generator deliberately builds reference cycles (`push(c, c)`) and
//! shared subvalues, because the value model's aliasing/cycle handling is the
//! highest-risk runtime area (AUDIT-4). The target therefore exercises display,
//! equality, and JSON encoding over cyclic graphs.
//!
//! ## Checker-rejection is a generator defect
//!
//! If a generated program is rejected by the checker, that is a **failure**
//! (`panic!`), never silently discarded: the generator's whole contract is to
//! produce checker-passing programs.

use libfuzzer_sys::fuzz_target;

#[path = "../../tests/support/program_gen.rs"]
mod program_gen;

fuzz_target!(|data: &[u8]| {
    // Derive a seed from the input bytes; empty input uses seed 0.
    let mut seed: u64 = 0;
    for (i, b) in data.iter().take(8).enumerate() {
        seed |= u64::from(*b) << (8 * i);
    }
    let src = program_gen::generate(seed);
    // A normal result or a structured runtime diagnostic is both success.
    let _ = aura::run_source(&src, "<fuzz-runtime>");
    // A checker rejection would surface above indistinguishably from a runtime
    // diagnostic, so assert the generator produced a checkable program — a
    // rejection is a generator defect, never silently discarded.
    if let Err(d) = aura::compile(&src, "module") {
        panic!(
            "generator produced a non-checking program (seed {seed}): E{}: {}\n{src}",
            d.code, d.message
        );
    }
});
