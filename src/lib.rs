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
pub mod diagnostic;
pub mod error;
pub mod host;
pub mod lex;
pub mod module_graph;
#[cfg(not(target_arch = "wasm32"))]
pub mod native_source;
pub mod parse;
#[cfg(feature = "repl")]
pub mod repl;
pub mod resolve;
pub mod run;
pub mod source;
pub mod stdlib;
pub mod types;

use std::sync::{Arc, Mutex};

use error::{DiagnosticReport, SourceDiagnostic};
use source::{SourceId, SourceMap};

/// Compile-time proof that the AST graph stays within the native
/// execution-thread boundary.
///
/// `parse` and every public `compile*`/`Compilation::execute_with*` entry point
/// move a `Module` (and its `Item`/`Stmt`/`Expr`/`TypeExpr`/`Arm`/`Arg`/
/// `FPart`/`Pattern` graph) across `on_parse_stack`/`on_execution_stack`
/// (`T: Send + 'static`). The AST uses [`std::sync::Arc`] precisely so those
/// bounds hold; these assertions fail to compile if any AST field ever reverts
/// to a `!Send` owner (`Rc`, `RefCell`, raw pointer, …). See
/// `docs/B1R3A_AST_SHARING_DECISION.md` and
/// `docs/engineering/ITERATIVE_EVALUATOR_DESIGN.md` §16.1.
///
/// `Sync` is asserted too: `Arc<T>` is only `Send` when `T: Send + Sync`, and
/// these nodes are only ever shared read-only, so `Sync` is the honest bound.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ast::Module>();
    assert_send_sync::<ast::Item>();
    assert_send_sync::<ast::Stmt>();
    assert_send_sync::<ast::Expr>();
    assert_send_sync::<ast::TypeExpr>();
    assert_send_sync::<ast::Pattern>();
    assert_send_sync::<ast::Arm>();
    assert_send_sync::<ast::Arg>();
    assert_send_sync::<ast::FPart>();
    assert_send_sync::<ast::Param>();
};

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
/// release version.
///
/// It advances whenever a *language-observable* behavior changes — including a
/// change in what programs are accepted or rejected — even if the release is a
/// patch. `0.2.1` is such a change: the builtin-name value-namespace
/// reservation (`E1009`) rejects programs the `0.2.0` language accepted (those
/// that collided a binding with a builtin and broke assignment lookup). See
/// `docs/adr/0001-release-vs-language-version.md`.
///
/// Invariant (checked by `tests/contract.rs`): `LANGUAGE_VERSION` is valid
/// semver and `LANGUAGE_VERSION <= VERSION`.
pub const LANGUAGE_VERSION: &str = "0.2.1";

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

fn on_source_execution_stack<T, F>(f: F) -> std::result::Result<T, SourceDiagnostic>
where
    T: Send + 'static,
    F: FnOnce() -> std::result::Result<T, SourceDiagnostic> + Send + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = std::thread::Builder::new()
            .name("aura-source-exec".to_string())
            .stack_size(INTERP_STACK)
            .spawn(f)
            .map_err(|e| {
                SourceDiagnostic::locationless(error::Diag::locationless(
                    error::codes::FOREIGN,
                    format!("cannot start source-aware execution thread: {e}"),
                ))
            })?;
        match handle.join() {
            Ok(result) => result,
            Err(_) => Err(SourceDiagnostic::locationless(error::Diag::locationless(
                error::codes::INTERNAL,
                "the source-aware execution thread aborted; this is a bug in Aura, not in your program",
            ))),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        f()
    }
}

/// A writer that appends to a shared buffer, so output can be read back
/// after the interpreter is dropped.
#[derive(Clone)]
pub struct SharedBuf(pub Arc<Mutex<Vec<u8>>>);

impl std::io::Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Recover from a poisoned lock rather than silently dropping bytes;
        // a `Vec<u8>` append is infallible, so no torn state is possible and
        // claiming a dropped write would corrupt observable output.
        let mut v = match self.0.lock() {
            Ok(v) => v,
            Err(poisoned) => poisoned.into_inner(),
        };
        v.extend_from_slice(buf);
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
pub fn run_source(src: &str, file: &str) -> error::Result<String> {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let sink = SharedBuf(buf.clone());
    let compilation = compile_named_with_mode(src, file, CompileMode::Module)
        .map_err(DiagnosticReport::into_diagnostic)?;
    compilation
        .execute_with(Some(Box::new(sink)), Vec::new(), None)
        .map_err(DiagnosticReport::into_diagnostic)?;
    // Recover a poisoned lock: the buffer's bytes are complete and
    // authoritative (appends are infallible), and dropping them would make
    // `run_source` report success while returning truncated output.
    let text = match buf.lock() {
        Ok(v) => String::from_utf8_lossy(&v).into_owned(),
        Err(poisoned) => String::from_utf8_lossy(&poisoned.into_inner()).into_owned(),
    };
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
    file: &str,
    args: Vec<String>,
    input: Option<Input>,
) -> error::Result<()> {
    run_named_toplevel_with(src, file, args, input).map_err(DiagnosticReport::into_diagnostic)
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
    file: &str,
    args: Vec<String>,
    input: Option<Input>,
) -> error::Result<()> {
    run_named_program_with(src, file, args, input).map_err(DiagnosticReport::into_diagnostic)
}

/// The two ways a module can be compiled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompileMode {
    /// A declaration set / expression list; no `main` required.
    Module,
    /// An executable program; `fn main` is required (`E4027` otherwise).
    Program,
}

/// A resolved and checked Aura module together with the source registry that
/// owns every source identity used by its diagnostics.
///
/// Named-source entry points construct one-source compilations, while
/// provider-backed compilation uses the same source registry for multi-source
/// module assembly and source-aware diagnostics.
#[derive(Debug)]
pub struct Compilation {
    module: ast::Module,
    sources: SourceMap,
    entry_source: SourceId,
    item_sources: Option<Vec<SourceId>>,
}

impl Compilation {
    /// Resolved and checked module.
    #[must_use]
    pub const fn module(&self) -> &ast::Module {
        &self.module
    }

    /// Source registry retained for diagnostics.
    #[must_use]
    pub const fn sources(&self) -> &SourceMap {
        &self.sources
    }

    /// Source selected as the compilation entry.
    #[must_use]
    pub const fn entry_source(&self) -> SourceId {
        self.entry_source
    }

    /// Consume the compilation and return its components.
    #[must_use]
    pub fn into_parts(self) -> (ast::Module, SourceMap, SourceId) {
        (self.module, self.sources, self.entry_source)
    }

    /// Execute this compilation with an explicit runtime context.
    ///
    /// One-source compilations attribute runtime diagnostics to the entry
    /// source. Provider-backed compilations retain per-item source ownership
    /// and route execution through the sourced runtime path without changing
    /// `Span` or the diagnostic data model.
    pub fn execute_with(
        self,
        stdout: Option<Output>,
        args: Vec<String>,
        input: Option<Input>,
    ) -> std::result::Result<(), DiagnosticReport> {
        self.execute_with_host_factory(move || host::host_from_parts(stdout, args, input))
    }

    /// Execute this compilation with the retained recursive evaluator.
    ///
    /// Hidden: kept as the differential reference for the oracle harness and
    /// as the rollback path until B-1R8 removes the recursive engine after the
    /// final release decision. Production (`execute_with`/
    /// `execute_with_host_factory`) runs the explicit-continuation machine.
    ///
    /// # Errors
    /// Returns the first diagnostic the recursive engine produces.
    #[doc(hidden)]
    pub fn execute_recursive(
        self,
        stdout: Option<Output>,
        args: Vec<String>,
        input: Option<Input>,
    ) -> std::result::Result<(), DiagnosticReport> {
        self.execute_recursive_host_factory(move || host::host_from_parts(stdout, args, input))
    }

    /// [`Compilation::execute_recursive`] with a host factory (the oracle's
    /// recursive side and the rollback path for provider-backed builds).
    ///
    /// # Errors
    /// Returns the first diagnostic the recursive engine produces.
    #[doc(hidden)]
    pub fn execute_recursive_host_factory<F>(
        self,
        make_host: F,
    ) -> std::result::Result<(), DiagnosticReport>
    where
        F: FnOnce() -> Box<dyn host::Host> + Send + 'static,
    {
        let Compilation {
            module,
            sources,
            entry_source,
            item_sources,
        } = self;
        if let Some(item_sources) = item_sources {
            let outcome = on_source_execution_stack(move || {
                let mut interp = run::Interp::new();
                interp.set_host(make_host());
                interp.run_sourced(&module, &item_sources, entry_source)
            });
            return outcome.map_err(|diagnostic| DiagnosticReport::new(diagnostic, sources));
        }
        let outcome = on_execution_stack(move || {
            let mut interp = run::Interp::new();
            interp.set_host(make_host());
            interp.run(&module)
        });
        match outcome {
            Ok(()) => Ok(()),
            Err(diagnostic) => Err(DiagnosticReport::new(
                SourceDiagnostic::new(diagnostic, entry_source),
                sources,
            )),
        }
    }

    /// Execute this compilation with the explicit-continuation machine.
    ///
    /// Retained as a hidden alias of the production path so the differential
    /// oracle can name the engine explicitly; production entries call the same
    /// machine internally. It never falls back to the recursive engine.
    ///
    /// # Errors
    /// Returns the first diagnostic the iterative machine produces.
    #[doc(hidden)]
    pub fn execute_iterative(
        self,
        stdout: Option<Output>,
        args: Vec<String>,
        input: Option<Input>,
    ) -> std::result::Result<(), DiagnosticReport> {
        let Compilation {
            module,
            sources,
            entry_source,
            item_sources,
        } = self;
        // Mirror `execute_with_host_factory` exactly: a provider-backed
        // compilation routes through the sourced machine path so per-item
        // source ownership is preserved.
        if let Some(item_sources) = item_sources {
            let outcome = on_source_execution_stack(move || {
                let mut interp = run::Interp::new();
                interp.set_host(host::host_from_parts(stdout, args, input));
                interp.run_iterative_sourced(&module, &item_sources, entry_source)
            });
            return outcome.map_err(|diagnostic| DiagnosticReport::new(diagnostic, sources));
        }
        let outcome = on_execution_stack(move || {
            let mut interp = run::Interp::new();
            interp.set_host(host::host_from_parts(stdout, args, input));
            interp.run_iterative(&module)
        });
        outcome.map_err(|diagnostic| {
            DiagnosticReport::new(SourceDiagnostic::new(diagnostic, entry_source), sources)
        })
    }

    /// Execute this compilation with a host constructed on Aura's execution
    /// stack.
    ///
    /// Embedders whose host trait object itself is not `Send` can move a
    /// `Send` factory across the native execution-thread boundary and create
    /// the host there. This is also the browser/WASM embedding boundary used
    /// by the Playground virtual-project runtime.
    ///
    /// # Errors
    /// Returns the first runtime diagnostic with source provenance retained.
    pub fn execute_with_host_factory<F>(
        self,
        make_host: F,
    ) -> std::result::Result<(), DiagnosticReport>
    where
        F: FnOnce() -> Box<dyn host::Host> + Send + 'static,
    {
        let Compilation {
            module,
            sources,
            entry_source,
            item_sources,
        } = self;
        if let Some(item_sources) = item_sources {
            let outcome = on_source_execution_stack(move || {
                let mut interp = run::Interp::new();
                interp.set_host(make_host());
                interp.run_iterative_sourced(&module, &item_sources, entry_source)
            });
            return outcome.map_err(|diagnostic| DiagnosticReport::new(diagnostic, sources));
        }
        let outcome = on_execution_stack(move || {
            let mut interp = run::Interp::new();
            interp.set_host(make_host());
            interp.run_iterative(&module)
        });
        match outcome {
            Ok(()) => Ok(()),
            Err(diagnostic) => Err(DiagnosticReport::new(
                SourceDiagnostic::new(diagnostic, entry_source),
                sources,
            )),
        }
    }
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
    compile_owned_source(Arc::<str>::from(src), mode)
}

fn compile_owned_source(src: Arc<str>, mode: CompileMode) -> error::Result<ast::Module> {
    // Parsing, module resolution, and checking all walk the AST recursively on
    // the caller's stack, so the whole front end runs on the execution
    // substrate: a program up to the language nesting limit (`E1015`) is a
    // deterministic diagnostic, never a host stack overflow.
    on_execution_stack(move || {
        let module = parse::parse(&src)?;
        let module = resolve::resolve(module)?;
        check::Checker::module_in_mode(&module, mode)?;
        Ok(module)
    })
}

/// Compile one source already registered in `sources`.
///
/// Internals continue to use source-local [`error::Span`] values; any
/// diagnostic leaving this source boundary is paired with `source`.
///
/// # Errors
/// Returns an `INTERNAL` locationless diagnostic if `source` does not belong
/// to `sources`, otherwise the first front-end diagnostic with source identity
/// attached.
pub fn compile_source_in_map(
    sources: &SourceMap,
    source: SourceId,
    mode: CompileMode,
) -> std::result::Result<ast::Module, SourceDiagnostic> {
    let Some(record) = sources.get(source) else {
        return Err(SourceDiagnostic::locationless(error::Diag::locationless(
            error::codes::INTERNAL,
            "source id does not belong to this source map",
        )));
    };
    compile_owned_source(record.text_handle(), mode).map_err(|d| SourceDiagnostic::new(d, source))
}

/// Compile one named UTF-8 source into a source-aware [`Compilation`].
///
/// The display name is provenance only; it does not imply filesystem module
/// semantics.
///
/// # Errors
/// Returns the first front-end diagnostic together with the owning source map.
pub fn compile_named_with_mode(
    src: &str,
    display_name: &str,
    mode: CompileMode,
) -> std::result::Result<Compilation, DiagnosticReport> {
    let mut sources = SourceMap::new();
    let source = sources.add(display_name, src);
    match compile_source_in_map(&sources, source, mode) {
        Ok(module) => Ok(Compilation {
            module,
            sources,
            entry_source: source,
            item_sources: None,
        }),
        Err(diagnostic) => Err(DiagnosticReport::new(diagnostic, sources)),
    }
}

/// Compile the provider's reachable logical module tree through Aura's
/// existing resolver/checker pipeline while retaining per-source provenance.
///
/// This is provider-neutral: native filesystem and browser/VFS adapters can
/// implement [`module_graph::SourceProvider`] without changing module
/// semantics.
pub fn compile_provider_with_mode<P: module_graph::SourceProvider>(
    provider: &P,
    mode: CompileMode,
) -> std::result::Result<Compilation, DiagnosticReport> {
    let graph = module_graph::ModuleGraphBuilder::new().build(provider)?;
    let (module, sources, entry_source, _nodes, provenance) = graph.into_compilation_parts();
    let semantic = on_source_execution_stack(move || {
        let (module, item_sources) = resolve::resolve_sourced(module, &provenance)?;
        check::Checker::module_in_mode_sourced(&module, &item_sources, entry_source, mode)?;
        Ok((module, item_sources))
    });
    match semantic {
        Ok((module, item_sources)) => Ok(Compilation {
            module,
            sources,
            entry_source,
            item_sources: Some(item_sources),
        }),
        Err(diagnostic) => Err(DiagnosticReport::new(diagnostic, sources)),
    }
}

/// Compile a selected native file together with its reachable filesystem
/// module tree.
///
/// The selected file owns the logical root and its containing directory is the
/// native source root. This API is unavailable on WebAssembly targets.
#[cfg(not(target_arch = "wasm32"))]
pub fn compile_file_with_mode(
    path: impl AsRef<std::path::Path>,
    mode: CompileMode,
) -> std::result::Result<Compilation, DiagnosticReport> {
    let provider = native_source::NativeFilesystemSourceProvider::new(path).map_err(|error| {
        DiagnosticReport::new(
            SourceDiagnostic::locationless(error::Diag::locationless(
                error::codes::MODULE_SOURCE_PATH,
                error.message(),
            )),
            SourceMap::new(),
        )
    })?;
    compile_provider_with_mode(&provider, mode)
}

/// Source-aware counterpart of [`run_program_with`].
///
/// # Errors
/// Returns a renderable diagnostic report retaining the source map.
pub fn run_named_program_with(
    src: &str,
    display_name: &str,
    args: Vec<String>,
    input: Option<Input>,
) -> std::result::Result<(), DiagnosticReport> {
    compile_named_with_mode(src, display_name, CompileMode::Program)?
        .execute_with(None, args, input)
}

/// Source-aware counterpart of [`run_toplevel_with`].
///
/// # Errors
/// Returns a renderable diagnostic report retaining the source map.
pub fn run_named_toplevel_with(
    src: &str,
    display_name: &str,
    args: Vec<String>,
    input: Option<Input>,
) -> std::result::Result<(), DiagnosticReport> {
    compile_named_with_mode(src, display_name, CompileMode::Module)?.execute_with(None, args, input)
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
        interp.run_iterative(&module)
    })
}

/// A boxed writer the interpreter can move onto its thread.
pub type Output = Box<dyn std::io::Write + Send>;

/// A boxed standard-input source the interpreter can move onto its thread.
pub type Input = Box<dyn std::io::BufRead + Send>;
