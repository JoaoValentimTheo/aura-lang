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
pub mod host;
pub mod lex;
pub mod parse;
#[cfg(feature = "repl")]
pub mod repl;
pub mod run;
pub mod stdlib;
pub mod types;

use std::sync::{Arc, Mutex};

/// The release version of the Aura implementation (the package version).
///
/// This tracks the *release* identity used by the CLI, the Git tag, and the
/// release artifacts. It is distinct from [`LANGUAGE_VERSION`]: a release can
/// ship runtime, tooling, or packaging changes without changing the language's
/// semantics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The version of the Aura *language semantics*.
///
/// The language specification (`docs/LANGUAGE_SPEC.md`) is normative at this
/// identifier. It changes only through the RFC process, independently of the
/// release version. Aura `0.0.2` now ships the evolved language: general union
/// types, transparent union composition through aliases, the Rust-style
/// `a..b` range literal, and `<!-- ... --!>` multiline comments. The numeric
/// identifier is retained because it is a release-boundary identity asserted
/// by `tests/contract.rs`; the semantics it denotes are those of the current
/// `docs/LANGUAGE_SPEC.md`.
pub const LANGUAGE_VERSION: &str = "0.0.1";

/// Stack size for the interpreter thread. Recursive Aura programs recurse
/// through several Rust frames per call, so a generous but bounded stack
/// keeps the language's own recursion limit (`E4011`) authoritative.
pub const INTERP_STACK: usize = 64 * 1024 * 1024;

/// Run `f` on the execution substrate, returning its result.
///
/// **Native:** `f` runs on a dedicated 64 MiB stack so that deeply recursive
/// Aura programs surface the language's own limits (`E1015`, `E4011`) rather
/// than a host stack overflow. A panic in the worker becomes an `INTERNAL`
/// diagnostic instead of crossing the boundary.
///
/// **WebAssembly:** there are no OS threads and no stack-size knob, so `f`
/// runs inline on the engine stack. Language limits remain the only bound a
/// program can hit; the parser uses a smaller grouping budget on this
/// substrate (see [`parse::parse_recursion_budget`]).
///
/// # Errors
/// Returns whatever `f` returns. On native, a failure to start the thread is
/// `FOREIGN`; a panic in the worker becomes `INTERNAL`.
pub fn on_execution_stack<T, F>(f: F) -> error::Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> error::Result<T> + Send + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = std::thread::Builder::new()
            .name("aura-exec".to_string())
            .stack_size(INTERP_STACK)
            .spawn(f)
            .map_err(|e| {
                error::Diag::new(
                    error::codes::FOREIGN,
                    format!("cannot start execution thread: {e}"),
                    error::Span::default(),
                )
            })?;
        match handle.join() {
            Ok(r) => r,
            Err(_) => Err(error::Diag::new(
                error::codes::INTERNAL,
                "the execution thread aborted; this is a bug in Aura, not in your program",
                error::Span::default(),
            )),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        f()
    }
}

/// Run `f` on the large execution stack, returning its result.
///
/// Retained as the public name used by the REPL; delegates to
/// [`on_execution_stack`].
///
/// # Errors
/// Returns whatever `f` returns.
pub fn on_large_stack<T, F>(f: F) -> error::Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> error::Result<T> + Send + 'static,
{
    on_execution_stack(f)
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
    execute(compile(&src, "module")?, Some(Box::new(sink)))?;
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
pub fn run_toplevel_stdout(src: &str, file: &str) -> error::Result<()> {
    run_toplevel_with(src, file, Vec::new(), None)
}

/// [`run_toplevel_stdout`] with an explicit execution context (args and stdin).
///
/// # Errors
/// Returns the first front-end or runtime diagnostic.
pub fn run_toplevel_with(
    src: &str,
    _file: &str,
    args: Vec<String>,
    input: Option<Input>,
) -> error::Result<()> {
    let src = src.to_string();
    execute_with(compile(&src, "module")?, None, args, input)
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
pub fn run_program(src: &str, file: &str) -> error::Result<()> {
    run_program_with(src, file, Vec::new(), None)
}

/// [`run_program`] with an explicit execution context (args and stdin): the
/// entry point used by `aura run`.
///
/// # Errors
/// Returns `E4027` when there is no `main`, plus any front-end or runtime
/// diagnostic.
pub fn run_program_with(
    src: &str,
    _file: &str,
    args: Vec<String>,
    input: Option<Input>,
) -> error::Result<()> {
    let src = src.to_string();
    execute_with(compile(&src, "program")?, None, args, input)
}

/// The two ways a module can be compiled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompileMode {
    /// A declaration set / expression list; no `main` required.
    Module,
    /// An executable program; `fn main` is required (`E4027` otherwise).
    Program,
}

impl CompileMode {
    /// Parse a canonical mode name (`"module"` / `"program"`).
    #[must_use]
    pub fn parse(s: &str) -> Option<CompileMode> {
        match s {
            "module" => Some(CompileMode::Module),
            "program" => Some(CompileMode::Program),
            _ => None,
        }
    }
}

/// **The** front-end: lex, parse, and check `src` in the given [`CompileMode`].
///
/// Every entry point (library, CLI, REPL) goes through here, so "what is a
/// valid program" is defined in exactly one place.
///
/// # Errors
/// Returns the first lexer, parser, or checker diagnostic.
pub fn compile(src: &str, mode: &str) -> error::Result<ast::Module> {
    let mode = CompileMode::parse(mode).ok_or_else(|| {
        error::Diag::new(
            error::codes::INTERNAL,
            format!("unknown compile mode `{mode}`"),
            error::Span::default(),
        )
    })?;
    compile_with_mode(src, mode)
}

/// [`compile`] with an explicit [`CompileMode`].
///
/// Parsing and checking both run on a large stack, so a valid program up to
/// the language nesting limit never exhausts the caller's stack (`E1015` is
/// the only bound).
///
/// # Errors
/// Returns the first lexer, parser, or checker diagnostic.
pub fn compile_with_mode(src: &str, mode: CompileMode) -> error::Result<ast::Module> {
    let module = parse::parse(src)?;
    check_on_big_stack(module, mode)
}

/// Run the checker on the large execution stack, returning the module so the
/// caller can still execute or inspect it.
fn check_on_big_stack(module: ast::Module, mode: CompileMode) -> error::Result<ast::Module> {
    on_execution_stack(move || {
        check::Checker::module_in_mode(&module, mode)?;
        Ok(module)
    })
}

/// Check an already-parsed module in the given mode.
///
/// The REPL parses statements incrementally, so it checks pieces rather than a
/// whole source string; this keeps the mode decision in one place for it too.
///
/// # Errors
/// Returns the first checker diagnostic.
pub fn compile_module(module: &ast::Module, mode: CompileMode) -> error::Result<()> {
    check::Checker::module_in_mode(module, mode)
}

/// Execute a compiled module on the interpreter thread, optionally capturing
/// stdout.
///
/// # Errors
/// Returns the first runtime diagnostic.
pub fn execute(module: ast::Module, stdout: Option<Output>) -> error::Result<()> {
    execute_with(module, stdout, Vec::new(), None)
}

/// Execute a compiled module with an explicit execution context: an optional
/// output sink, program arguments for `args()`, and an optional standard-input
/// source for `read_line()`. `execute` delegates here with empty defaults, so
/// existing callers keep their behavior (process stdout, no input, no
/// arguments).
///
/// # Errors
/// Returns the first runtime diagnostic.
pub fn execute_with(
    module: ast::Module,
    stdout: Option<Output>,
    args: Vec<String>,
    input: Option<Input>,
) -> error::Result<()> {
    on_execution_stack(move || {
        let mut interp = run::Interp::new();
        interp.set_host(host::host_from_parts(stdout, args, input));
        interp.run(&module)
    })
}

/// A boxed writer the interpreter can move onto its thread.
pub type Output = Box<dyn std::io::Write + Send>;

/// A boxed standard-input source the interpreter can move onto its thread.
pub type Input = Box<dyn std::io::BufRead + Send>;
