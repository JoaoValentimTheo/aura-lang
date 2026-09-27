#![no_main]
//! Fuzz target: the checker (on a grammar-aware AST).
//!
//! INVARIANT: no input reaches a Rust panic, abort, or stack overflow; every
//! input terminates and yields either a successful check or a structured
//! `E2xxx`/`E3xxx` diagnostic.
//!
//! ## AST invariants the generator guarantees
//!
//! The "grammar-aware AST generator" here is **the parser itself**: the target
//! lexes and parses the bytes first and only checks a module the parser
//! accepted. This is deliberate. A hand-built AST could violate representation
//! invariants the rest of the compiler relies on (spans, arities, enum shapes),
//! which would fuzz *internal API misuse* rather than the checker. The parser
//! guarantees those invariants by construction, so:
//!
//! * spans are well-formed and ordered;
//! * every `Expr::Call` has a callee and a well-formed argument list;
//! * enum/struct/variant shapes come from the grammar, not from raw bytes;
//! * nesting is already bounded (`E1015`), so the checker is only reached for
//!   ASTs within the semantic limit.
//!
//! Input that does not parse is *not* a checker input; it exercises the
//! front end instead (see the `parser` target) and is discarded here.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let src = String::from_utf8_lossy(data);
    let Ok(module) = aura::parse::parse(&src) else {
        return;
    };
    // Resolution then checking, the same order the compiler uses. Resolution
    // is included because the checker consumes resolver output (canonical
    // names); fuzzing the checker without it would miss that boundary.
    let Ok(module) = aura::resolve::resolve(module) else {
        return;
    };
    let _ = aura::check::Checker::module(&module);
});
