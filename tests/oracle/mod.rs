#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// `observe` and its helpers return `ObserveResult` on purpose: a harness setup
// failure is a distinct category from a language outcome, and the future
// iterative engine may surface one. Keeping the `Result` seam now means B-1R3
// does not have to change the adapter signature.
#![allow(clippy::unnecessary_wraps)]
//! Differential evaluator oracle — test-only measuring instrument (B-1R2).
//!
//! This module is **not** an evaluator. It is the normalized, isolated,
//! comparison harness that B-1R3 will plug the explicit continuation machine
//! into. For B-1R2 both logical engine sides execute the current recursive
//! evaluator; the only value that proves today is *oracle infrastructure*.
//!
//! ## Isolation model (real, not documented)
//!
//! Each [`observe`] call reconstructs everything from the source anew:
//!
//! * front-end (`lex`/`parse`/`resolve`/`check`) runs per call;
//! * the executed module is either the owned `Compilation` or an independently
//!   built provider graph;
//! * [`aura::Compilation::execute_with`] runs on a fresh execution thread and
//!   constructs a **fresh** [`aura::run::Interp`] and **fresh** host writing to
//!   a **fresh** buffer.
//!
//! No `Interp`, `Env`, `Host`, stdout buffer, or runtime mutable state is
//! shared between observations. A diverging run cannot contaminate its
//! counterpart. This is enforced by construction, not by convention.
//!
//! ## Normalized observable
//!
//! Raw internal identities (`Rc` addresses, `SourceId` scope bits, thread ids)
//! never enter the comparison. Diagnostics are normalized to
//! `(code, message, source display name, byte span, line, column)`; the source
//! **display name** is retained so that an `E3001` at `a.aura:5:9` is never
//! equal to an `E3001` at `b.aura:5:9` even when the byte span is identical.
//!
//! ## Golden manifest
//!
//! `tests/oracle/golden.tsv` pins the *current, observed* behavior for every
//! corpus case, including the B-1 contract-boundary corpus and the
//! deliberately-preserved compound-assignment double evaluation. A future
//! engine is compared against that recorded behavior, not against the old
//! engine re-run to produce an accident.
//!
//! ## Engine adapter
//!
//! [`Engine`] is the single seam. Once B-1R3 lands, `Engine::iterative()`
//! selects the machine through the internal `evaluator-oracle` switch and the
//! exact same `observe`/`Observable`/golden machinery compares both sides
//! without redesign.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use aura::ast::{Item, Module};
use aura::error::{line_col, Diag, DiagnosticReport, Span};
use aura::host;
use aura::module_graph::{InMemorySourceProvider, SourceKey};
use aura::run::{Ctl, Interp};
use aura::{compile_named_with_mode, compile_provider_with_mode, CompileMode, SharedBuf};

/// The non-default Cargo feature that will expose the internal evaluator
/// switch in B-1R3. Declared here so the oracle's CI contract is explicit.
pub const ORACLE_FEATURE: &str = "evaluator-oracle";

// ---------------------------------------------------------------------------
// Corpus case model
// ---------------------------------------------------------------------------

/// One multi-source provider graph used by a [`Kind::Multi`] case.
pub struct MultiSource {
    /// Entry `(provider key, display name, source text)`.
    pub entry: (&'static str, &'static str, &'static str),
    /// Additional `(provider key, display name, source text)` records.
    pub extra: &'static [(&'static str, &'static str, &'static str)],
    /// Direct child edges `(parent key, logical name, child key)`.
    pub edges: &'static [(&'static str, &'static str, &'static str)],
}

/// What an [`observe`] call does with a case's source.
#[derive(Clone, Copy)]
pub enum Kind {
    /// Front end only, module mode (accept/reject, no execution).
    CompileModule,
    /// Front end only, program mode (accept/reject, no execution).
    CompileProgram,
    /// Compile module mode, then execute top-level items (no `main` needed).
    ExecuteModule,
    /// Compile program mode, then execute `main`.
    ExecuteProgram,
    /// Compile module mode, evaluate items on one interpreter, and capture the
    /// final top-level expression's value.
    Value,
    /// Build a provider graph, then compile and execute it (multi-source).
    Multi(&'static MultiSource),
}

impl Kind {
    /// A short stable tag used by the golden manifest.
    const fn tag(self) -> &'static str {
        match self {
            Kind::CompileModule => "compile-module",
            Kind::CompileProgram => "compile-program",
            Kind::ExecuteModule => "exec-module",
            Kind::ExecuteProgram => "exec-program",
            Kind::Value => "value",
            Kind::Multi(_) => "multi",
        }
    }
}

/// A named corpus case.
pub struct Case {
    /// Corpus group (used for the golden key and reporting).
    pub group: &'static str,
    /// Case name, unique within its group.
    pub name: &'static str,
    /// Display name / synthetic file name for single-source cases.
    pub file: &'static str,
    /// Source text.
    pub source: &'static str,
    /// How to observe the case.
    pub kind: Kind,
}

impl Case {
    /// The stable golden key `group/name`.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}/{}", self.group, self.name)
    }
}

// ---------------------------------------------------------------------------
// Normalized observable model
// ---------------------------------------------------------------------------

/// A normalized diagnostic: only contract-meaningful fields, with the source
/// **display name** retained (never a raw `SourceId`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NormDiag {
    pub code: u16,
    pub message: String,
    pub source_name: Option<String>,
    pub span_start: usize,
    pub span_end: usize,
    pub line: u32,
    pub column: u32,
}

/// How a run completed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Completion {
    /// Ran to completion (possibly with stdout).
    Ok,
    /// Rejected by the front end before execution.
    Compile(NormDiag),
    /// Rejected during execution.
    Runtime(NormDiag),
}

/// A normalized final value.
///
/// Values are **type-tagged** because `display()` erases type: `1` (int),
/// `1.0` (float), and `"1"` (string) all render as `1` at top level. Comparing
/// only the display would let a type change escape. This is the Step 6 value
/// comparison model: `none`, `bool`, `int`, `float`, `string`, `list`, `map`,
/// `struct`, `enum`, `fn`, `range` are all distinguishable; container element
/// types are visible through the recursive debug representation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NormValue {
    /// Runtime type name (`int`, `float`, `string`, ...).
    pub ty: String,
    /// The value's debug representation (quotes strings inside collections).
    pub repr: String,
}

/// The complete normalized, comparable observable.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Observable {
    /// Exact stdout bytes; never trimmed, sorted, or newline-normalized.
    pub stdout: Vec<u8>,
    /// Final observable value (only the [`Kind::Value`] path produces one).
    pub value: Option<NormValue>,
    /// Completion category and normalized diagnostic.
    pub completion: Completion,
}

impl Observable {
    fn ok(stdout: Vec<u8>, value: Option<NormValue>) -> Observable {
        Observable {
            stdout,
            value,
            completion: Completion::Ok,
        }
    }
}

/// Build a normalized value from a runtime value, preserving its runtime type.
fn norm_value(v: &aura::run::value::Value) -> NormValue {
    NormValue {
        ty: v.type_name().to_string(),
        repr: v.debug_repr(),
    }
}

// ---------------------------------------------------------------------------
// Engine adapter
// ---------------------------------------------------------------------------

/// The differential engine seam.
///
/// For B-1R2 [`Engine::recursive`] and [`Engine::iterative`] both run the
/// current tree-walking evaluator; the point is to prove the *harness*.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Engine {
    name: &'static str,
}

impl Engine {
    /// The current, in-tree recursive evaluator.
    #[must_use]
    pub const fn recursive() -> Engine {
        Engine { name: "recursive" }
    }

    /// The future explicit-continuation engine.
    ///
    /// Available only with the non-default `evaluator-oracle` feature. Until
    /// B-1R3 wires the machine it executes the same current path, which is the
    /// honest state for B-1R2: the adapter exists, the second engine does not.
    #[cfg(feature = "evaluator-oracle")]
    #[must_use]
    pub const fn iterative() -> Engine {
        Engine { name: "iterative" }
    }

    /// Stable engine identifier.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }
}

/// The engines this build compares.
///
/// Default builds compare the recursive engine with itself (baseline). The
/// `evaluator-oracle` feature adds the iterative side for B-1R3.
#[must_use]
pub fn engines() -> Vec<Engine> {
    #[allow(unused_mut)]
    let mut out = vec![Engine::recursive()];
    #[cfg(feature = "evaluator-oracle")]
    out.push(Engine::iterative());
    out
}

// ---------------------------------------------------------------------------
// Observation
// ---------------------------------------------------------------------------

/// The result of observing a case: an observable, or a harness setup failure.
///
/// A harness failure is a bug in the oracle (not a language outcome) and must
/// never be silently compared as if it were a production result.
pub type ObserveResult = std::result::Result<Observable, String>;

/// Observe one case with one engine.
///
/// Every call is fully self-contained: new front end, new compilation, new
/// interpreter, new host, new stdout buffer, new runtime state. The `engine`
/// parameter is threaded to the single execution seam [`run_compiled`] so a
/// real second engine plugs in there without touching the observable,
/// comparison, corpus, or golden machinery.
pub fn observe(case: &Case, engine: Engine) -> ObserveResult {
    match case.kind {
        Kind::CompileModule => observe_compile(case, CompileMode::Module),
        Kind::CompileProgram => observe_compile(case, CompileMode::Program),
        Kind::ExecuteModule => observe_execute(case, CompileMode::Module, engine),
        Kind::ExecuteProgram => observe_execute(case, CompileMode::Program, engine),
        Kind::Value => observe_value(case, engine),
        Kind::Multi(multi) => observe_multi(case, multi, engine),
    }
}

/// **The single execution seam.** Every compiled module is executed here, and
/// this is the only place an engine selects its execution path.
///
/// For B-1R2 both variants execute the current recursive interpreter through
/// `Compilation::execute_with`; no fake machine exists. In B-1R3 the
/// `Engine::iterative` branch (already feature-gated) becomes a call that runs
/// the explicit continuation machine on the same compiled module. The
/// observable/comparison/corpus/golden machinery is unchanged by that work.
fn run_compiled(
    compilation: aura::Compilation,
    sink: SharedBuf,
    engine: Engine,
) -> std::result::Result<(), DiagnosticReport> {
    match engine.name() {
        "recursive" => compilation.execute_with(Some(Box::new(sink)), Vec::new(), None),
        #[cfg(feature = "evaluator-oracle")]
        "iterative" => {
            // B-1R3A wiring point: run the explicit continuation machine here
            // on `compilation`. Until that engine exists this is the same
            // current path, which is the honest B-1R2 state.
            compilation.execute_with(Some(Box::new(sink)), Vec::new(), None)
        }
        _ => compilation.execute_with(Some(Box::new(sink)), Vec::new(), None),
    }
}

/// The value-path seam: run a main-less module's items on a fresh interpreter
/// and return the last top-level expression's normalized value. Both engines
/// share this entry point; B-1R3 selects the machine inside it.
fn run_value_items(module: &Module, _engine: Engine, sink: SharedBuf) -> Result<NormValue, Diag> {
    let mut interp = Interp::with_host(host::host_from_parts(
        Some(Box::new(sink)),
        Vec::new(),
        None,
    ));
    eval_module_items(&mut interp, module)
}

fn observe_compile(case: &Case, mode: CompileMode) -> ObserveResult {
    match compile_named_with_mode(case.source, case.file, mode) {
        Ok(_) => Ok(Observable::ok(Vec::new(), None)),
        Err(report) => Ok(Observable {
            stdout: Vec::new(),
            value: None,
            completion: Completion::Compile(norm_report(report)),
        }),
    }
}

fn observe_execute(case: &Case, mode: CompileMode, engine: Engine) -> ObserveResult {
    let compilation = match compile_named_with_mode(case.source, case.file, mode) {
        Ok(c) => c,
        Err(report) => {
            return Ok(Observable {
                stdout: Vec::new(),
                value: None,
                completion: Completion::Compile(norm_report(report)),
            })
        }
    };
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = SharedBuf(buf.clone());
    let outcome = run_compiled(compilation, sink, engine);
    let stdout = buf.lock().map(|v| v.clone()).unwrap_or_default();
    match outcome {
        Ok(()) => Ok(Observable::ok(stdout, None)),
        Err(report) => Ok(Observable {
            stdout,
            value: None,
            completion: Completion::Runtime(norm_report(report)),
        }),
    }
}

fn observe_value(case: &Case, engine: Engine) -> ObserveResult {
    let compilation = match compile_named_with_mode(case.source, case.file, CompileMode::Module) {
        Ok(c) => c,
        Err(report) => {
            return Ok(Observable {
                stdout: Vec::new(),
                value: None,
                completion: Completion::Compile(norm_report(report)),
            })
        }
    };
    let (module, _sources, _entry) = compilation.into_parts();
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = SharedBuf(buf.clone());
    match run_value_items(&module, engine, sink) {
        Ok(value) => {
            let stdout = buf.lock().map(|v| v.clone()).unwrap_or_default();
            Ok(Observable::ok(stdout, Some(value)))
        }
        Err(diag) => {
            let stdout = buf.lock().map(|v| v.clone()).unwrap_or_default();
            Ok(Observable {
                stdout,
                value: None,
                completion: Completion::Runtime(norm_diag(
                    diag,
                    Some(case.file.to_string()),
                    Some(case.source),
                )),
            })
        }
    }
}

/// Evaluate a main-less module's items on one interpreter, returning the
/// display of the last top-level expression. Declarations register; `Const`
/// items evaluate; expression items yield the observable value.
fn eval_module_items(interp: &mut Interp, module: &Module) -> Result<NormValue, Diag> {
    let mut value = NormValue {
        ty: "none".to_string(),
        repr: "(no top-level value)".to_string(),
    };
    for item in &module.items {
        match item {
            Item::Expr(e, _) => match interp.eval_globals(e)? {
                Ctl::Val(v) => value = norm_value(&v),
                Ctl::Return(_) => {
                    return Err(Diag::new(
                        4030,
                        "`return` cannot be used here",
                        Span::default(),
                    ))
                }
                Ctl::Break | Ctl::Continue => {
                    return Err(Diag::new(
                        2015,
                        "loop control outside a loop",
                        Span::default(),
                    ))
                }
                Ctl::Throw(v) => {
                    return Err(Diag::new(
                        4026,
                        format!("uncaught value: {}", v.display()),
                        Span::default(),
                    ))
                }
            },
            other => interp.run_item(other)?,
        }
    }
    Ok(value)
}

fn observe_multi(case: &Case, multi: &MultiSource, engine: Engine) -> ObserveResult {
    let mut provider = InMemorySourceProvider::new(SourceKey::new(multi.entry.0));
    let insert = |provider: &mut InMemorySourceProvider,
                  key: &str,
                  name: &'static str,
                  text: &'static str|
     -> std::result::Result<(), String> {
        provider
            .insert_source(SourceKey::new(key), name, text)
            .map_err(|e| e.message().to_string())
    };
    insert(&mut provider, multi.entry.0, multi.entry.1, multi.entry.2)?;
    for (key, name, text) in multi.extra {
        insert(&mut provider, key, name, text)?;
    }
    for (parent, logical, child) in multi.edges {
        provider
            .add_child(&SourceKey::new(*parent), *logical, &SourceKey::new(*child))
            .map_err(|e| e.message().to_string())?;
    }

    let compilation = match compile_provider_with_mode(&provider, CompileMode::Program) {
        Ok(c) => c,
        Err(report) => {
            return Ok(Observable {
                stdout: Vec::new(),
                value: None,
                completion: Completion::Compile(norm_report(report)),
            })
        }
    };
    let _ = case;
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = SharedBuf(buf.clone());
    let outcome = run_compiled(compilation, sink, engine);
    let stdout = buf.lock().map(|v| v.clone()).unwrap_or_default();
    match outcome {
        Ok(()) => Ok(Observable::ok(stdout, None)),
        Err(report) => Ok(Observable {
            stdout,
            value: None,
            completion: Completion::Runtime(norm_report(report)),
        }),
    }
}

// ---------------------------------------------------------------------------
// Diagnostic normalization
// ---------------------------------------------------------------------------

fn norm_report(report: DiagnosticReport) -> NormDiag {
    let code = report.diagnostic().code;
    let message = report.diagnostic().message.clone();
    let span = report.diagnostic().span;
    let (source_name, line, column, span_start, span_end) = match report.location() {
        Some(location) => {
            let name = report
                .sources()
                .get(location.source)
                .map(|s| s.name().to_string());
            let (line, column) = report
                .sources()
                .get(location.source)
                .map_or((0, 0), |s| line_col(s.text(), location.span.start));
            (name, line, column, location.span.start, location.span.end)
        }
        None => (None, 0, 0, span.start, span.end),
    };
    NormDiag {
        code,
        message,
        source_name,
        span_start,
        span_end,
        line,
        column,
    }
}

fn norm_diag(diag: Diag, source_name: Option<String>, source_text: Option<&str>) -> NormDiag {
    let (line, column) = source_text.map_or((0, 0), |s| line_col(s, diag.span.start));
    NormDiag {
        code: diag.code,
        message: diag.message,
        source_name,
        span_start: diag.span.start,
        span_end: diag.span.end,
        line,
        column,
    }
}

// ---------------------------------------------------------------------------
// Golden manifest encoding
// ---------------------------------------------------------------------------

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn unesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn unhex(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16).unwrap_or(0) as u8;
        let lo = (bytes[i + 1] as char).to_digit(16).unwrap_or(0) as u8;
        out.push((hi << 4) | lo);
        i += 2;
    }
    out
}

/// Serialize an observable to one tab-separated golden row (no leading key).
#[must_use]
pub fn encode_observable(obs: &Observable) -> String {
    let (kind, diag) = match &obs.completion {
        Completion::Ok => ("ok", None),
        Completion::Compile(d) => ("compile", Some(d)),
        Completion::Runtime(d) => ("runtime", Some(d)),
    };
    let (code, message, source, start, end, line, col) = match diag {
        Some(d) => (
            d.code.to_string(),
            esc(&d.message),
            d.source_name.clone().unwrap_or_else(|| "-".to_string()),
            d.span_start.to_string(),
            d.span_end.to_string(),
            d.line.to_string(),
            d.column.to_string(),
        ),
        None => (
            "0".to_string(),
            String::new(),
            "-".to_string(),
            "0".to_string(),
            "0".to_string(),
            "0".to_string(),
            "0".to_string(),
        ),
    };
    let (value_ty, value_repr) = match &obs.value {
        Some(v) => (v.ty.clone(), esc(&v.repr)),
        None => ("-".to_string(), "-".to_string()),
    };
    format!(
        "{hex}\t{kind}\t{code}\t{message}\t{source}\t{start}\t{end}\t{line}\t{col}\t{value_ty}\t{value_repr}",
        hex = hex(&obs.stdout),
    )
}

/// Parse a golden row (the value after the key) back into an observable.
#[must_use]
pub fn decode_observable(row: &str) -> Observable {
    let f: Vec<&str> = row.split('\t').collect();
    assert!(
        f.len() >= 11,
        "golden row must have at least 11 fields, got {}: {row:?}",
        f.len()
    );
    let stdout = unhex(f[0]);
    let completion = match f[1] {
        "ok" => Completion::Ok,
        "compile" => Completion::Compile(decode_diag(&f[2..9])),
        "runtime" => Completion::Runtime(decode_diag(&f[2..9])),
        other => panic!("unknown completion tag {other:?}"),
    };
    let value = if f[9] == "-" || f[9].is_empty() {
        None
    } else {
        Some(NormValue {
            ty: f[9].to_string(),
            repr: unesc(f[10]),
        })
    };
    Observable {
        stdout,
        value,
        completion,
    }
}

fn decode_diag(f: &[&str]) -> NormDiag {
    NormDiag {
        code: f[0].parse().unwrap_or(0),
        message: unesc(f[1]),
        source_name: if f[2] == "-" {
            None
        } else {
            Some(f[2].to_string())
        },
        span_start: f[3].parse().unwrap_or(0),
        span_end: f[4].parse().unwrap_or(0),
        line: f[5].parse().unwrap_or(0),
        column: f[6].parse().unwrap_or(0),
    }
}

/// A parsed golden manifest, keyed by `group/name`.
#[must_use]
pub fn parse_golden(text: &str, tag: &str) -> BTreeMap<String, Observable> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, rest)) = line.split_once('\t') else {
            continue;
        };
        let _ = tag;
        out.insert(key.to_string(), decode_observable(rest));
    }
    out
}

/// The number of distinct completion categories present, used by coverage
/// assertions without hard-coding a count.
#[must_use]
pub fn distinct_completion_kinds(cases: &[Case]) -> usize {
    let mut seen = std::collections::BTreeSet::new();
    for case in cases {
        let obs = match observe(case, Engine::recursive()) {
            Ok(o) => o,
            Err(_) => continue,
        };
        seen.insert(match obs.completion {
            Completion::Ok => 0,
            Completion::Compile(_) => 1,
            Completion::Runtime(_) => 2,
        });
    }
    seen.len()
}

/// Path of the committed golden manifest.
pub const GOLDEN_PATH: &str = "tests/oracle/golden.tsv";

/// Serialize a full corpus observation into golden-file text.
#[must_use]
pub fn encode_golden(cases: &[Case], observations: &BTreeMap<String, Observable>) -> String {
    let mut out = String::from("# Aura differential evaluator oracle golden manifest.\n");
    out.push_str(
        "# Generated by tests/evaluator_oracle.rs::regenerate_golden; do not hand-edit.\n",
    );
    out.push_str("# key\tstdout-hex\tcompletion\tcode\tmessage\tsource\tspan_start\tspan_end\tline\tcol\tvalue\n");
    for case in cases {
        if let Some(obs) = observations.get(&case.key()) {
            out.push_str(&case.key());
            out.push('\t');
            out.push_str(&encode_observable(obs));
            out.push('\n');
        }
    }
    out
}

/// Report the count of cases per kind tag (used by the corpus test to assert
/// coverage breadth without hard-coding brittle numbers).
#[must_use]
pub fn kind_tag(kind: Kind) -> &'static str {
    kind.tag()
}
