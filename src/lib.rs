//! Aura: a small expression-oriented language with a native Rust runtime
//! and optional Python interop.
//!
//! The pipeline is:
//!
//! 1. [`lex`] tokenizes source.
//! 2. [`parse`] builds an [`ast::Module`].
//! 3. [`check`] validates names and mutability.
//! 4. [`run`] executes the module.
//!
//! See `docs/contract.md` for the normative language specification.

pub mod ast;
pub mod bridge;
pub mod check;
pub mod error;
pub mod lex;
pub mod parse;
#[cfg(feature = "repl")]
pub mod repl;
pub mod run;
pub mod stdlib;

use std::sync::{Arc, Mutex};

/// The compiler version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Stack size for the interpreter thread. Recursive Aura programs recurse
/// through several Rust frames per call, so a generous but bounded stack
/// keeps the language's own recursion limit (`E4011`) authoritative.
pub const INTERP_STACK: usize = 64 * 1024 * 1024;

/// Run `f` on a thread with a large stack, so deep Aura recursion surfaces
/// as `E4011` instead of a native stack overflow.
fn on_interp_thread<T, F>(f: F) -> error::Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> error::Result<T> + Send + 'static,
{
    let handle = std::thread::Builder::new()
        .name("aura-interp".to_string())
        .stack_size(INTERP_STACK)
        .spawn(f)
        .map_err(|e| {
            error::Diag::new(
                error::codes::FOREIGN,
                format!("cannot start interpreter: {e}"),
                error::Span::default(),
            )
        })?;
    match handle.join() {
        Ok(r) => r,
        Err(_) => Err(error::Diag::new(
            error::codes::INTERNAL,
            "the interpreter thread aborted; this is a bug in Aura, not in your program",
            error::Span::default(),
        )),
    }
}

/// A writer that appends to a shared buffer, so output can be read back
/// after the interpreter is dropped.
#[derive(Clone)]
pub struct SharedBuf(pub Arc<Mutex<Vec<u8>>>);

impl std::io::Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut v) = self.0.lock() {
            v.extend_from_slice(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Compile and run `src`, returning collected stdout.
///
/// # Errors
/// Returns the first diagnostic produced by lexing, parsing, checking, or
/// execution.
pub fn run_source(src: &str, _file: &str) -> error::Result<String> {
    let src = src.to_string();
    let buf = Arc::new(Mutex::new(Vec::new()));
    let sink = SharedBuf(buf.clone());
    on_interp_thread(move || {
        let module = parse::parse(&src)?;
        check::Checker::module(&module)?;
        let mut interp = run::Interp::new();
        interp.stdout = Box::new(sink);
        interp.run(&module)?;
        Ok(())
    })?;
    let text = buf
        .lock()
        .map(|v| String::from_utf8_lossy(&v).into_owned())
        .unwrap_or_default();
    Ok(text)
}

/// Compile and run `src` without requiring an entry point, writing output to
/// stdout. Used by `aura eval`, where a bare expression or declaration set is
/// valid.
///
/// # Errors
/// Returns the first front-end or runtime diagnostic.
pub fn run_toplevel_stdout(src: &str, _file: &str) -> error::Result<()> {
    let src = src.to_string();
    on_interp_thread(move || {
        let module = parse::parse(&src)?;
        check::Checker::module(&module)?;
        let mut interp = run::Interp::new();
        interp.run(&module)
    })
}

/// Compile and run `src`, writing output to the process stdout.
///
/// This is the execution path used by `aura run`: it requires the program to
/// declare `fn main()` and executes the entry point after loading
/// declarations.
///
/// # Errors
/// Returns `E4027` when there is no `main`, plus any front-end or runtime
/// diagnostic.
pub fn run_program(src: &str, _file: &str) -> error::Result<()> {
    let src = src.to_string();
    on_interp_thread(move || {
        let module = parse::parse(&src)?;
        check::Checker::module_with_main(&module)?;
        let mut interp = run::Interp::new();
        interp.run(&module)
    })
}
