//! The Aura Playground WebAssembly execution wrapper.
//!
//! This crate is *not* a second interpreter. It links the real `aura-lang`
//! crate and exposes one minimal, versioned ABI over the existing pipeline
//! (`compile` → `Interp::with_host` → `run`). The browser never reimplements
//! Aura semantics; it only drives this wrapper.
//!
//! # Why the ABI is pointer-free
//!
//! The project enforces an unsafe-free guarantee. A conventional byte-buffer
//! ABI would need raw-pointer reads; instead the host feeds source and options
//! to the module through small integer calls (`push_word`), so **no `unsafe`
//! is required anywhere** and the module still takes plain bytes.
//!
//! # ABI contract (version 1)
//!
//! * `aura_abi_version() -> u32` — Host ABI version.
//! * `aura_version_len()` / `aura_version_byte(i)` — language version string.
//! * `aura_runtime_version_len()` / `aura_runtime_version_byte(i)` — runtime
//!   artifact version string.
//! * `aura_source_reset()` / `aura_source_push(word, nbytes)` — feed source.
//! * `aura_project_reset()` / `aura_project_push(word, nbytes)` — optionally
//!   feed a virtual multi-source project request.
//! * `aura_options_reset()` / `aura_options_push(word, nbytes)` — feed options.
//! * `aura_run() -> u32` — execute and store the JSON result.
//! * `aura_run_project() -> u32` — execute a virtual project when supplied.
//! * `aura_output_len()` / `aura_output_byte(i)` — read the JSON result.
//!
//! Everything crosses as plain integers/bytes. The module has **zero imports**
//! (no WASI, no JS): it can reach no DOM, network, storage, filesystem, clock,
//! or JS handle. Its only effect is its own linear memory, read back by the
//! host.
//!
//! # Result format
//!
//! `aura_run` returns `0` (ok), `1` (diagnostic), or `2` (internal) and leaves
//! a JSON document in the output buffer:
//!
//! ```json
//! {
//!   "status": "ok" | "diagnostic" | "internal",
//!   "stdout": "…",
//!   "result": null | "…",
//!   "diagnostics": [ { "code": 4026, "code_text": "E4026", "message": "…", "line": 1, "column": 1 } ]
//! }
//! ```
//!
//! The JSON is assembled from Aura's own structured types (`Diag`,
//! `line_col`), never by scraping rendered terminal text.

#![deny(unsafe_code)]

use std::collections::BTreeSet;

use aura::error::{codes, line_col, Diag, DiagnosticReport, Span};
use aura::host::BrowserHost;
use std::sync::Mutex;
use aura::module_graph::{InMemorySourceProvider, SourceKey};
use aura::run::Interp;
use serde::Deserialize;

/// The Host ABI version. Bump only for an incompatible ABI change; the
/// manifest records it so the Playground can refuse a mismatch.
pub const ABI_VERSION: u32 = 1;

/// The runtime artifact version.
///
/// This is the version of the *WebAssembly runtime build*, distinct from the
/// Aura *language* version reported by `aura::LANGUAGE_VERSION`. The frozen
/// `0.0.1` release predates the WebAssembly substrate (so it has no wasm
/// runtime at all); the first wasm-executable runtime was the historical
/// `0.0.2`, and the current runtime is the public `0.2.0` release. Both
/// identifiers are exposed so a version entry is unambiguous.
pub const RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Status codes returned by [`execute`] and `aura_run`.
pub mod status {
    /// Execution completed.
    pub const OK: u32 = 0;
    /// Execution stopped on an Aura diagnostic (front end or runtime).
    pub const DIAGNOSTIC: u32 = 1;
    /// An internal invariant was violated; this is an Aura bug.
    pub const INTERNAL: u32 = 2;
}

/// Application-level limits. These are *host* policies, not language rules;
/// they bound what the Playground will attempt so a hostile program cannot
/// exhaust the browser before the Worker is terminated.
pub mod limits {
    /// Maximum source size accepted, in bytes.
    pub const MAX_SOURCE_BYTES: usize = 256 * 1024;
    /// Legacy fatal stdout capture bound, in bytes.
    ///
    /// This is the historical `with_stdout_limit` bound (a write crossing it
    /// is a fatal `E4020`). It is retained for the frozen-release contract and
    /// for embedders that still want a hard refusal; the current runtime uses
    /// the bounded [`OutputSink`](aura::host::OutputSink) instead, where a
    /// full preview continues execution (0.3.2 development).
    pub const MAX_STDOUT_BYTES: usize = 1024 * 1024;
    /// Default retained preview size, in bytes, for the bounded output sink.
    ///
    /// The preview is what the UI displays; bytes past it are counted as
    /// omitted, never silently lost and never a program failure.
    pub const PREVIEW_STDOUT_BYTES: usize = 256 * 1024;
    /// Maximum complete-output capture, in bytes, for a complete-mode sink.
    ///
    /// A bounded-memory destination: beyond it, capture is capped and the
    /// runtime reports the exact bytes committed. This is not "unlimited".
    pub const COMPLETE_STDOUT_BYTES: usize = 16 * 1024 * 1024;
    /// Maximum standard-input size accepted, in bytes.
    pub const MAX_STDIN_BYTES: usize = 1024 * 1024;
    /// Maximum number of program arguments.
    pub const MAX_ARGS: usize = 256;
    /// Maximum length of a single program argument, in bytes.
    pub const MAX_ARG_BYTES: usize = 16 * 1024;
    /// Maximum encoded virtual-project request size, in bytes.
    pub const MAX_PROJECT_BYTES: usize = 2 * 1024 * 1024;
    /// Maximum number of sources accepted in one virtual project.
    pub const MAX_PROJECT_SOURCES: usize = 4096;
    /// Maximum length of one opaque virtual source key, in bytes.
    pub const MAX_VIRTUAL_KEY_BYTES: usize = 128;
    /// Maximum length of one user-facing virtual source name, in bytes.
    pub const MAX_SOURCE_NAME_BYTES: usize = 1024;
}

/// The output-retention mode requested by the embedder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    /// Bounded live preview: continue past the bound, count omitted bytes.
    Preview,
    /// Bounded complete-output capture: keep everything up to the cap, then
    /// cap the capture and report the exact committed byte count.
    Complete,
}

impl OutputMode {
    fn sink(self) -> aura::host::OutputSink {
        match self {
            OutputMode::Preview => {
                aura::host::OutputSink::preview(limits::PREVIEW_STDOUT_BYTES)
            }
            OutputMode::Complete => {
                aura::host::OutputSink::complete(limits::COMPLETE_STDOUT_BYTES)
            }
        }
    }
}

/// A per-run options object, encoded as a compact line protocol:
///
/// ```text
/// arg <text>\n                  (zero or more, in order)
/// stdin-bytes <n>\n             (then exactly n raw bytes follow)
/// output-mode preview|complete\n (optional; defaults to preview)
/// ```
struct Options {
    args: Vec<String>,
    stdin: Option<String>,
    output_mode: OutputMode,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VirtualProjectRequest {
    entry: String,
    sources: Vec<VirtualSourceRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VirtualSourceRequest {
    key: String,
    name: String,
    text: String,
    #[serde(default)]
    children: Vec<VirtualChildRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VirtualChildRequest {
    name: String,
    key: String,
}

#[derive(Debug)]
struct VirtualProjectError {
    code: u16,
    message: String,
}

impl VirtualProjectError {
    fn host(message: impl Into<String>) -> VirtualProjectError {
        VirtualProjectError {
            code: codes::IO,
            message: message.into(),
        }
    }

    fn module_source(message: impl Into<String>) -> VirtualProjectError {
        VirtualProjectError {
            code: codes::MODULE_SOURCE_PATH,
            message: message.into(),
        }
    }
}

fn parse_options(raw: &[u8]) -> Result<Options, String> {
    let mut args = Vec::new();
    let mut pos = 0usize;
    let mut stdin = None;
    let mut output_mode = OutputMode::Preview;
    while pos < raw.len() {
        let rest = &raw[pos..];
        let nl = rest
            .iter()
            .position(|&b| b == b'\n')
            .ok_or("unterminated option line")?;
        let line = &rest[..nl];
        let text = std::str::from_utf8(line).map_err(|_| "option line is not UTF-8")?;
        if let Some(n) = text.strip_prefix("stdin-bytes ") {
            let n: usize = n.parse().map_err(|_| "invalid stdin-bytes count")?;
            let start = pos + nl + 1;
            let end = start.checked_add(n).ok_or("stdin length overflow")?;
            if end > raw.len() {
                return Err("stdin-bytes exceeds the options buffer".to_string());
            }
            if n > limits::MAX_STDIN_BYTES {
                return Err(format!(
                    "standard input exceeds the {} byte limit",
                    limits::MAX_STDIN_BYTES
                ));
            }
            stdin = Some(
                String::from_utf8(raw[start..end].to_vec())
                    .map_err(|_| "standard input is not valid UTF-8")?,
            );
            pos = end;
            continue;
        }
        if let Some(mode) = text.strip_prefix("output-mode ") {
            output_mode = match mode {
                "preview" => OutputMode::Preview,
                "complete" => OutputMode::Complete,
                other => return Err(format!("unknown output mode `{other}`")),
            };
            pos += nl + 1;
            continue;
        }
        let arg = text.strip_prefix("arg ").unwrap_or(text);
        if args.len() >= limits::MAX_ARGS {
            return Err(format!("too many arguments (limit {})", limits::MAX_ARGS));
        }
        if arg.len() > limits::MAX_ARG_BYTES {
            return Err(format!(
                "argument exceeds the {} byte limit",
                limits::MAX_ARG_BYTES
            ));
        }
        args.push(arg.to_string());
        pos += nl + 1;
    }
    Ok(Options {
        args,
        stdin,
        output_mode,
    })
}

fn valid_virtual_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= limits::MAX_VIRTUAL_KEY_BYTES
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn valid_source_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= limits::MAX_SOURCE_NAME_BYTES
        && !name.chars().any(char::is_control)
}

fn build_virtual_provider(
    raw: &[u8],
) -> Result<(InMemorySourceProvider, bool), VirtualProjectError> {
    if raw.len() > limits::MAX_PROJECT_BYTES {
        return Err(VirtualProjectError::host(format!(
            "virtual project exceeds the {} byte limit",
            limits::MAX_PROJECT_BYTES
        )));
    }

    let mut project: VirtualProjectRequest = serde_json::from_slice(raw).map_err(|error| {
        VirtualProjectError::host(format!("invalid virtual project JSON: {error}"))
    })?;
    if project.sources.len() > limits::MAX_PROJECT_SOURCES {
        return Err(VirtualProjectError::host(format!(
            "virtual project has too many sources (limit {})",
            limits::MAX_PROJECT_SOURCES
        )));
    }
    if !valid_virtual_key(&project.entry) {
        return Err(VirtualProjectError::module_source(format!(
            "invalid virtual entry key `{}`",
            project.entry
        )));
    }

    project
        .sources
        .sort_by(|left, right| left.key.as_bytes().cmp(right.key.as_bytes()));

    let mut keys = BTreeSet::new();
    let mut source_names = BTreeSet::new();
    for source in &project.sources {
        if !valid_virtual_key(&source.key) {
            return Err(VirtualProjectError::module_source(format!(
                "invalid virtual source key `{}`",
                source.key
            )));
        }
        if !keys.insert(source.key.clone()) {
            return Err(VirtualProjectError::host(format!(
                "virtual source key `{}` is registered more than once",
                source.key
            )));
        }
        if !valid_source_name(&source.name) {
            return Err(VirtualProjectError::host(format!(
                "invalid virtual source name for key `{}`",
                source.key
            )));
        }
        if !source_names.insert(source.name.clone()) {
            return Err(VirtualProjectError::host(format!(
                "virtual source name `{}` is registered more than once",
                source.name
            )));
        }
        if source.text.len() > limits::MAX_SOURCE_BYTES {
            return Err(VirtualProjectError::host(format!(
                "virtual source `{}` exceeds the {} byte limit",
                source.name,
                limits::MAX_SOURCE_BYTES
            )));
        }
    }
    if !keys.contains(&project.entry) {
        return Err(VirtualProjectError::module_source(format!(
            "virtual project entry key `{}` is not registered",
            project.entry
        )));
    }

    let is_single_source = project.sources.len() == 1 && project.sources[0].children.is_empty();

    let entry = SourceKey::new(project.entry.clone());
    let mut provider = InMemorySourceProvider::new(entry);
    for source in &project.sources {
        provider
            .insert_source(
                SourceKey::new(source.key.clone()),
                source.name.clone(),
                source.text.clone(),
            )
            .map_err(|error| VirtualProjectError::module_source(error.to_string()))?;
    }

    for source in &mut project.sources {
        source.children.sort_by(|left, right| {
            left.name
                .as_bytes()
                .cmp(right.name.as_bytes())
                .then_with(|| left.key.as_bytes().cmp(right.key.as_bytes()))
        });
        let parent = SourceKey::new(source.key.clone());
        for child in &source.children {
            if !valid_virtual_key(&child.key) {
                return Err(VirtualProjectError::module_source(format!(
                    "invalid virtual child key `{}`",
                    child.key
                )));
            }
            if !keys.contains(&child.key) {
                return Err(VirtualProjectError::module_source(format!(
                    "virtual child `{}` of source `{}` references unknown key `{}`",
                    child.name, source.key, child.key
                )));
            }
            provider
                .add_child(
                    &parent,
                    child.name.clone(),
                    &SourceKey::new(child.key.clone()),
                )
                .map_err(|error| VirtualProjectError::module_source(error.to_string()))?;
        }
    }

    Ok((provider, is_single_source))
}

/// Escape a string as a JSON string literal.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn diag_json(d: &Diag, src: &str, first: &mut bool) -> String {
    let (line, col) = line_col(src, d.span.start);
    let mut s = String::new();
    if !*first {
        s.push(',');
    }
    *first = false;
    s.push_str(&format!(
        "{{\"code\":{},\"code_text\":\"E{:04}\",\"message\":{},\"line\":{},\"column\":{}}}",
        d.code,
        d.code,
        json_string(&d.message),
        line,
        col
    ));
    s
}

fn sourced_diag_json(d: &Diag, src: &str, source_name: Option<&str>, first: &mut bool) -> String {
    let (line, col) = line_col(src, d.span.start);
    let mut s = String::new();
    if !*first {
        s.push(',');
    }
    *first = false;
    let source = source_name.map_or_else(|| "null".to_string(), json_string);
    s.push_str(&format!(
        "{{\"code\":{},\"code_text\":\"E{:04}\",\"message\":{},\"line\":{},\"column\":{},\"source\":{}}}",
        d.code,
        d.code,
        json_string(&d.message),
        line,
        col,
        source
    ));
    s
}

/// Assemble the result document.
///
/// `sink` carries the bounded output: its retained bytes become `stdout` (the
/// display preview) and its counters become `stdout_retained` /
/// `stdout_omitted` / `stdout_truncated` / `stdout_capped` plus `output_mode`.
/// A `None` sink means the run produced no output state (an input rejection
/// before execution); the fields are emitted as a coherent empty preview so a
/// consumer never has to treat their absence as a case.
fn result_json(
    status: &str,
    sink: Option<&aura::host::OutputSink>,
    result: Option<&str>,
    diagnostics: &str,
) -> String {
    let (stdout, stats, mode, capped) = match sink {
        Some(s) => {
            // `capped` distinguishes a complete-mode capture that hit its
            // capacity from an ordinary preview.
            let mode = if s.capped() { "complete" } else { "preview" };
            (s.retained_text(), s.stats(), mode, s.capped())
        }
        None => (
            String::new(),
            aura::host::OutputStats::default(),
            "preview",
            false,
        ),
    };
    format!(
        "{{\"status\":{},\"stdout\":{},\"stdout_retained\":{},\"stdout_omitted\":{},\"stdout_truncated\":{},\"stdout_capped\":{},\"output_mode\":{},\"result\":{},\"diagnostics\":[{}]}}",
        json_string(status),
        json_string(&stdout),
        stats.retained,
        stats.omitted,
        stats.truncated(),
        capped,
        json_string(mode),
        result.map_or_else(|| "null".to_string(), json_string),
        diagnostics
    )
}

fn diagnostic_result(
    d: &Diag,
    source: &str,
    sink: Option<&aura::host::OutputSink>,
) -> (String, u32) {
    let mut first = true;
    let diags = diag_json(d, source, &mut first);
    if d.code == codes::INTERNAL {
        (
            result_json("internal", sink, None, &diags),
            status::INTERNAL,
        )
    } else {
        (
            result_json("diagnostic", sink, None, &diags),
            status::DIAGNOSTIC,
        )
    }
}

fn report_diagnostic_result(
    report: &DiagnosticReport,
    sink: Option<&aura::host::OutputSink>,
) -> (String, u32) {
    let diagnostic = report.diagnostic();
    let source = report
        .source_diagnostic()
        .source()
        .and_then(|source| report.sources().get(source));
    let mut first = true;
    let diags = sourced_diag_json(
        diagnostic,
        source.map_or("", aura::source::Source::text),
        source.map(aura::source::Source::name),
        &mut first,
    );
    if diagnostic.code == codes::INTERNAL {
        (
            result_json("internal", sink, None, &diags),
            status::INTERNAL,
        )
    } else {
        (
            result_json("diagnostic", sink, None, &diags),
            status::DIAGNOSTIC,
        )
    }
}

fn virtual_input_error(code: u16, message: impl Into<String>) -> (String, u32) {
    let diagnostic = Diag::new(code, message, Span::default());
    let mut first = true;
    let diags = sourced_diag_json(&diagnostic, "", None, &mut first);
    (
        result_json("diagnostic", None, None, &diags),
        status::DIAGNOSTIC,
    )
}

/// Decode the byte-oriented host input strictly before executing source text.
/// Shared by the WASM ABI and native differential harness.
#[must_use]
pub fn execute_bytes(source: &[u8], options_raw: &[u8]) -> (String, u32, &'static str) {
    match aura::lex::decode_source(source) {
        Ok(source) => execute(source, options_raw),
        Err(d) => {
            let (json, code) = diagnostic_result(&d, &String::from_utf8_lossy(source), None);
            (json, code, aura::LANGUAGE_VERSION)
        }
    }
}

/// Execute a caller-supplied virtual multi-source project through Aura's
/// provider-neutral module graph and canonical semantic pipeline.
///
/// The project is a UTF-8 JSON document with one opaque `entry` key and a
/// `sources` array. Each source has `key`, user-facing `name`, UTF-8 `text`,
/// and optional direct `children` entries containing a logical Aura module
/// `name` plus the child source `key`. Keys are transport identities only and
/// deliberately do not use host path syntax.
#[must_use]
pub fn execute_project_bytes(
    project_raw: &[u8],
    options_raw: &[u8],
) -> (String, u32, &'static str) {
    if project_raw.len() > limits::MAX_PROJECT_BYTES {
        let (json, code) = virtual_input_error(
            codes::IO,
            format!(
                "virtual project exceeds the {} byte limit",
                limits::MAX_PROJECT_BYTES
            ),
        );
        return (json, code, aura::LANGUAGE_VERSION);
    }
    let opts = match parse_options(options_raw) {
        Ok(options) => options,
        Err(message) => {
            let (json, code) = virtual_input_error(codes::IO, message);
            return (json, code, aura::LANGUAGE_VERSION);
        }
    };
    let (provider, is_single_source) = match build_virtual_provider(project_raw) {
        Ok(provider) => provider,
        Err(error) => {
            let (json, code) = virtual_input_error(error.code, error.message);
            return (json, code, aura::LANGUAGE_VERSION);
        }
    };

    let compilation = match aura::compile_provider_with_mode(&provider, aura::CompileMode::Program)
    {
        Ok(compilation) => compilation,
        Err(report) if is_single_source && report.diagnostic().code == codes::NO_MAIN => {
            return run_virtual_module(&provider, &opts);
        }
        Err(report) => {
            let (json, code) = report_diagnostic_result(&report, None);
            return (json, code, aura::LANGUAGE_VERSION);
        }
    };

    let sink = opts.output_mode.sink();
    let outcome_sink = sink.clone();
    let stdin = opts.stdin;
    let args = opts.args;
    let outcome = compilation.execute_with_host_factory(move || {
        Box::new(BrowserHost::from_parts(stdin, args).with_output_sink(outcome_sink))
    });
    match outcome {
        Ok(()) => (
            result_json("ok", Some(&sink), None, ""),
            status::OK,
            aura::LANGUAGE_VERSION,
        ),
        Err(report) => {
            let (json, code) = report_diagnostic_result(&report, Some(&sink));
            (json, code, aura::LANGUAGE_VERSION)
        }
    }
}

fn run_virtual_module(
    provider: &InMemorySourceProvider,
    opts: &Options,
) -> (String, u32, &'static str) {
    let compilation = match aura::compile_provider_with_mode(provider, aura::CompileMode::Module) {
        Ok(compilation) => compilation,
        Err(report) => {
            let (json, code) = report_diagnostic_result(&report, None);
            return (json, code, aura::LANGUAGE_VERSION);
        }
    };
    let (module, sources, source_id) = compilation.into_parts();
    let sink = opts.output_mode.sink();
    let module_sink = sink.clone();
    let stdin = opts.stdin.clone();
    let args = opts.args.clone();
    let value = aura::on_execution_stack(move || {
        let host = BrowserHost::from_parts(stdin, args).with_output_sink(module_sink);
        let mut interp = Interp::with_host(Box::new(host));
        run_module_capture(&mut interp, &module)
    });
    match value {
        Ok(value) => (
            result_json("ok", Some(&sink), value.as_deref(), ""),
            status::OK,
            aura::LANGUAGE_VERSION,
        ),
        Err(diagnostic) => {
            let report = DiagnosticReport::new(
                aura::error::SourceDiagnostic::new(diagnostic, source_id),
                sources,
            );
            let (json, code) = report_diagnostic_result(&report, Some(&sink));
            (json, code, aura::LANGUAGE_VERSION)
        }
    }
}

/// Execute `source` against a fresh [`BrowserHost`], returning the JSON result
/// document, the status code, and the module's intrinsic language version.
///
/// `main`-less sources fall back to module (toplevel) semantics, matching
/// `aura eval`, and surface the final top-level expression value as `result`.
///
/// This is the whole engine, free of global state, so it is directly testable
/// natively; the C ABI functions are thin wrappers over it.
#[must_use]
pub fn execute(source: &str, options_raw: &[u8]) -> (String, u32, &'static str) {
    if source.len() > limits::MAX_SOURCE_BYTES {
        let d = Diag::new(
            codes::IO,
            format!("source exceeds the {} byte limit", limits::MAX_SOURCE_BYTES),
            Span::default(),
        );
        let (json, code) = diagnostic_result(&d, source, None);
        return (json, code, aura::LANGUAGE_VERSION);
    }
    let opts = match parse_options(options_raw) {
        Ok(o) => o,
        Err(message) => {
            let d = Diag::new(codes::IO, message, Span::default());
            let (json, code) = diagnostic_result(&d, source, None);
            return (json, code, aura::LANGUAGE_VERSION);
        }
    };

    let sink = opts.output_mode.sink();

    let compilation =
        match aura::compile_named_with_mode(source, "<playground>", aura::CompileMode::Program) {
            Ok(c) => c,
            // No `main`: fall back to module semantics, as `aura eval` does.
            Err(report) if report.diagnostic().code == codes::NO_MAIN => {
                return run_module(source, options_raw, &opts, sink);
            }
            Err(report) => {
                let (json, code) = diagnostic_result(report.diagnostic(), source, None);
                return (json, code, aura::LANGUAGE_VERSION);
            }
        };
    let (program, sources, source_id) = compilation.into_parts();

    // Execute on the substrate's execution stack: a dedicated large stack on
    // native (so the language's own `E4011` is authoritative), inline on wasm.
    // This mirrors the library entry points exactly.
    let outcome_sink = sink.clone();
    let stdin = opts.stdin;
    let args = opts.args;
    let outcome = aura::on_execution_stack(move || {
        let host = BrowserHost::from_parts(stdin, args).with_output_sink(outcome_sink);
        let mut interp = Interp::with_host(Box::new(host));
        interp.run_iterative(&program)
    })
    .map_err(|d| aura::error::SourceDiagnostic::new(d, source_id));
    // Keep the source map alive through runtime diagnostic production. The
    // current JSON ABI remains intentionally source-name-free.
    let _sources = sources;
    finish(outcome, source, &sink, None)
}

/// Run a `main`-less source with module semantics and capture the last
/// top-level expression value.
fn run_module(
    source: &str,
    options_raw: &[u8],
    opts: &Options,
    sink: aura::host::OutputSink,
) -> (String, u32, &'static str) {
    let _ = options_raw;
    let compilation =
        match aura::compile_named_with_mode(source, "<playground>", aura::CompileMode::Module) {
            Ok(c) => c,
            Err(report) => {
                let (json, code) = diagnostic_result(report.diagnostic(), source, None);
                return (json, code, aura::LANGUAGE_VERSION);
            }
        };
    let (module, sources, source_id) = compilation.into_parts();
    let module_sink = sink.clone();
    let stdin = opts.stdin.clone();
    let args = opts.args.clone();
    let value = aura::on_execution_stack(move || {
        let host = BrowserHost::from_parts(stdin, args).with_output_sink(module_sink);
        let mut interp = Interp::with_host(Box::new(host));
        run_module_capture(&mut interp, &module)
    })
    .map_err(|d| aura::error::SourceDiagnostic::new(d, source_id));
    let _sources = sources;
    match value {
        Ok(v) => (
            result_json("ok", Some(&sink), v.as_deref(), ""),
            status::OK,
            aura::LANGUAGE_VERSION,
        ),
        Err(d) => {
            let (json, code) = diagnostic_result(d.diagnostic(), source, Some(&sink));
            (json, code, aura::LANGUAGE_VERSION)
        }
    }
}

/// Execute the declarations of a module and return the last top-level
/// expression's displayed value, using only public interpreter API.
///
/// Declaration order matches [`Interp::run`]: every declaration is registered
/// first (so forward references between functions are valid), then constants
/// and expressions run in source order. The value of the final top-level
/// expression (if any) is the `result`.
fn run_module_capture(
    interp: &mut Interp,
    module: &aura::ast::Module,
) -> Result<Option<String>, Diag> {
    use aura::ast::Item;
    for item in &module.items {
        if !matches!(item, Item::Const { .. } | Item::Expr(..)) {
            interp.run_item_iterative(item)?;
        }
    }
    let mut last: Option<String> = None;
    for item in &module.items {
        match item {
            Item::Const { name, value, .. } => {
                let ctl = interp.eval_globals_iterative(value)?;
                let v = interp.finish_global(ctl)?;
                interp.global(name, v);
            }
            Item::Expr(e, _) => {
                let ctl = interp.eval_globals_iterative(e)?;
                let v = interp.finish_global(ctl)?;
                last = Some(v.display());
            }
            _ => {}
        }
    }
    Ok(last)
}

fn finish(
    outcome: Result<(), aura::error::SourceDiagnostic>,
    source: &str,
    sink: &aura::host::OutputSink,
    _result: Option<&str>,
) -> (String, u32, &'static str) {
    match outcome {
        Ok(()) => (
            result_json("ok", Some(sink), None, ""),
            status::OK,
            aura::LANGUAGE_VERSION,
        ),
        Err(d) => {
            let (json, code) = diagnostic_result(d.diagnostic(), source, Some(sink));
            (json, code, aura::LANGUAGE_VERSION)
        }
    }
}

// ---------------------------------------------------------------------------
// The exported ABI. Every function is safe (no pointers): the host feeds data
// through `push` calls and reads the result byte by byte.
// ---------------------------------------------------------------------------

use std::sync::LazyLock;

static PENDING_SOURCE: LazyLock<Mutex<Vec<u8>>> = LazyLock::new(|| Mutex::new(Vec::new()));
static PENDING_PROJECT: LazyLock<Mutex<Vec<u8>>> = LazyLock::new(|| Mutex::new(Vec::new()));
static PENDING_OPTIONS: LazyLock<Mutex<Vec<u8>>> = LazyLock::new(|| Mutex::new(Vec::new()));
static RESULT: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

/// Lock a global slot, recovering from poison.
///
/// A poison can only follow a panic while this module held the lock. The
/// protected payloads (byte buffers, an optional result string) are never
/// left torn by an infallible mutate step, so the recovered data is
/// authoritative; silently substituting an empty value would lose a whole
/// pending request or result.
fn lock_recovering<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match m.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn set_result(s: String) {
    *lock_recovering(&RESULT) = Some(s);
}

/// The Host ABI version.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_abi_version() -> u32 {
    ABI_VERSION
}

/// Length in bytes of the language version string.
///
/// This is the *language semantics* version (`aura::LANGUAGE_VERSION`), not the
/// release version. The manifest records both so a version entry is
/// unambiguous.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_version_len() -> u32 {
    aura::LANGUAGE_VERSION.len() as u32
}

/// Byte `i` of the language version string (0 past the end).
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_version_byte(i: u32) -> u32 {
    aura::LANGUAGE_VERSION
        .as_bytes()
        .get(i as usize)
        .copied()
        .unwrap_or(0) as u32
}

/// Length in bytes of the runtime artifact version string.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_runtime_version_len() -> u32 {
    RUNTIME_VERSION.len() as u32
}

/// Byte `i` of the runtime artifact version string (0 past the end).
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_runtime_version_byte(i: u32) -> u32 {
    RUNTIME_VERSION
        .as_bytes()
        .get(i as usize)
        .copied()
        .unwrap_or(0) as u32
}

/// Discard any pending source.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_source_reset() {
    lock_recovering(&PENDING_SOURCE).clear();
}

/// Append the low `nbytes` (1..=4) bytes of `word` (little-endian) to the
/// pending source.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_source_push(word: u32, nbytes: u32) {
    push_word(&PENDING_SOURCE, word, nbytes);
}

/// Discard any pending virtual-project request.
///
/// This is an additive Host ABI 1 capability. Existing single-source hosts do
/// not need to call or even discover it.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_project_reset() {
    lock_recovering(&PENDING_PROJECT).clear();
}

/// Append the low `nbytes` (1..=4) bytes of `word` (little-endian) to the
/// pending virtual-project JSON request.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_project_push(word: u32, nbytes: u32) {
    push_word(&PENDING_PROJECT, word, nbytes);
}

/// Discard any pending options.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_options_reset() {
    lock_recovering(&PENDING_OPTIONS).clear();
}

/// Append the low `nbytes` (1..=4) bytes of `word` (little-endian) to the
/// pending options.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_options_push(word: u32, nbytes: u32) {
    push_word(&PENDING_OPTIONS, word, nbytes);
}

fn push_word(dest: &LazyLock<Mutex<Vec<u8>>>, word: u32, nbytes: u32) {
    let n = nbytes.clamp(1, 4) as usize;
    let bytes = word.to_le_bytes();
    lock_recovering(dest).extend_from_slice(&bytes[..n]);
}

/// Execute the pending program and store the JSON result.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_run() -> u32 {
    let source = lock_recovering(&PENDING_SOURCE).clone();
    let options = lock_recovering(&PENDING_OPTIONS).clone();
    let (json, status, _version) = execute_bytes(&source, &options);
    set_result(json);
    status
}

/// Execute the pending virtual project and store the JSON result.
///
/// The existing single-source `aura_run` contract is unchanged. This export
/// is feature-detected by newer hosts and therefore does not require a Host
/// ABI version bump.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_run_project() -> u32 {
    let project = lock_recovering(&PENDING_PROJECT).clone();
    let options = lock_recovering(&PENDING_OPTIONS).clone();
    let (json, status, _version) = execute_project_bytes(&project, &options);
    set_result(json);
    status
}

/// Length in bytes of the JSON result document.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_output_len() -> u32 {
    lock_recovering(&RESULT)
        .as_ref()
        .map_or(0, |s| s.len() as u32)
}

/// Byte `i` of the JSON result document (0 past the end).
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn aura_output_byte(i: u32) -> u32 {
    lock_recovering(&RESULT)
        .as_ref()
        .and_then(|s| s.as_bytes().get(i as usize).copied())
        .unwrap_or(0) as u32
}
