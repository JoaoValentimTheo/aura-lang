//! Errors, source spans, and diagnostic codes.
//!
//! Every diagnostic has a stable `E####` code. The code is part of the
//! language contract: `tests/contract.rs` asserts that each one can be
//! produced by at least one program.

use std::fmt;

use crate::source::{Location, SourceId, SourceMap};

/// A byte span in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Span {
    /// First byte.
    pub start: usize,
    /// One past the last byte.
    pub end: usize,
}

impl Span {
    /// Build a span.
    pub const fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }
}

/// Diagnostic severity.
///
/// Every diagnostic produced today is an error: compilation stops at the
/// first one. The field exists so the structured model is complete for
/// renderers and for the AIS/0.1 surface (Keystone diagnostics), and so a
/// future non-fatal diagnostic has a place to live without changing the
/// contract. Severity is *data*; no ANSI or presentation state belongs here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Severity {
    /// The program is rejected or the run fails.
    #[default]
    Error,
    /// A non-fatal finding.
    Warning,
    /// Informational context.
    Note,
}

impl Severity {
    /// The stable lowercase spelling used by structured output (AIS, JSON).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        }
    }
}

/// A source-local diagnostic: code, message, and byte span.
///
/// Source identity intentionally lives outside this legacy public struct so
/// downstream users that construct `Diag` with struct literals remain source
/// compatible. [`SourceDiagnostic`] adds provenance at multi-source boundaries.
///
/// `Diag` is deliberately frozen at three fields: the tests
/// (`tests/source_provenance.rs::legacy_diag_struct_literal_remains_source_compatible`)
/// pin that a struct literal keeps compiling. Structured presentation data
/// (severity, notes, help) lives on [`SourceDiagnostic`] instead, which has
/// never been struct-literal constructible.
#[derive(Debug, Clone)]
pub struct Diag {
    /// The stable code, e.g. `1006`.
    pub code: u16,
    /// Human-readable explanation.
    pub message: String,
    /// Where it happened.
    pub span: Span,
}

impl Diag {
    /// Build a diagnostic.
    pub fn new(code: u16, message: impl Into<String>, span: Span) -> Diag {
        Diag {
            code,
            message: message.into(),
            span,
        }
    }

    /// Build a diagnostic that deliberately has no source location.
    ///
    /// `span` remains `Span::default()` for compatibility with callers that
    /// still inspect the legacy field, but no fake [`SourceId`] is invented.
    pub fn locationless(code: u16, message: impl Into<String>) -> Diag {
        Diag {
            code,
            message: message.into(),
            span: Span::default(),
        }
    }

    /// Render as `E1006: ...` for the CLI.
    #[must_use]
    pub fn render(&self) -> String {
        format!("E{:04}: {}", self.code, self.message)
    }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E{:04}: {}", self.code, self.message)
    }
}

impl std::error::Error for Diag {}

/// A diagnostic paired with source identity at a multi-source boundary.
///
/// Compiler phases continue to produce compact source-local [`Diag`] values.
/// This wrapper is introduced only when the owning source is known.
#[derive(Debug, Clone)]
pub struct SourceDiagnostic {
    diagnostic: Diag,
    source: Option<SourceId>,
    /// How serious this is, and the structured context a renderer shows
    /// (Keystone §28). Presentation data never changes `code`, which is the
    /// identity.
    presentation: Presentation,
}

/// Structured presentation attached to a diagnostic at its source boundary.
///
/// This is data, not text: renderers (CLI, Playground, AIS) consume the fields
/// and never parse the rendered message, and no ANSI escape ever enters it.
#[derive(Debug, Clone, Default)]
pub struct Presentation {
    /// Severity; all diagnostics produced today are errors.
    pub severity: Severity,
    /// Additional context lines, shown after the primary message.
    pub notes: Vec<String>,
    /// A suggested remedy, if one exists.
    pub help: Option<String>,
}

impl SourceDiagnostic {
    /// Pair a source-local diagnostic with its owning source.
    #[must_use]
    pub fn new(diagnostic: Diag, source: SourceId) -> SourceDiagnostic {
        SourceDiagnostic {
            diagnostic,
            source: Some(source),
            presentation: Presentation::default(),
        }
    }

    /// Preserve a diagnostic that deliberately has no meaningful source.
    #[must_use]
    pub fn locationless(diagnostic: Diag) -> SourceDiagnostic {
        SourceDiagnostic {
            diagnostic,
            source: None,
            presentation: Presentation::default(),
        }
    }

    /// Attach structured presentation (severity, notes, help).
    #[must_use]
    pub fn with_presentation(mut self, presentation: Presentation) -> SourceDiagnostic {
        self.presentation = presentation;
        self
    }

    /// The structured presentation for this diagnostic.
    #[must_use]
    pub const fn presentation(&self) -> &Presentation {
        &self.presentation
    }

    /// The source-local diagnostic payload.
    #[must_use]
    pub const fn diagnostic(&self) -> &Diag {
        &self.diagnostic
    }

    /// Owning source, when one exists.
    #[must_use]
    pub const fn source(&self) -> Option<SourceId> {
        self.source
    }

    /// Source-aware location, when this diagnostic has one.
    #[must_use]
    pub fn location(&self) -> Option<Location> {
        self.source
            .map(|source| Location::new(source, self.diagnostic.span))
    }

    /// Consume the wrapper and return the compatibility diagnostic.
    #[must_use]
    pub fn into_diagnostic(self) -> Diag {
        self.diagnostic
    }
}

/// The result type used throughout the compiler.
pub type Result<T> = std::result::Result<T, Diag>;

/// Compute a 1-based (line, column) for a byte offset.
#[must_use]
pub fn line_col(src: &str, offset: usize) -> (u32, u32) {
    let offset = offset.min(src.len());
    let head = &src[..offset];
    let line = u32::try_from(head.bytes().filter(|b| *b == b'\n').count())
        .unwrap_or(u32::MAX)
        .saturating_add(1);
    let col = head.rsplit('\n').next().map_or(1, |s| {
        u32::try_from(s.chars().count())
            .unwrap_or(u32::MAX)
            .saturating_add(1)
    });
    (line, col)
}

/// Render a diagnostic with `file:line:col` prefix.
#[must_use]
pub fn render_with_source(file: &str, src: &str, d: &Diag) -> String {
    let (line, col) = line_col(src, d.span.start);
    format!("{file}:{line}:{col}: {d}")
}

/// Render a source-aware diagnostic through its owning [`SourceMap`].
///
/// Returns `None` when the diagnostic is deliberately locationless or when
/// its [`SourceId`] does not belong to `sources`. The latter is a hard safety
/// property: locations from unrelated compilations never silently resolve by
/// numeric index alone.
#[must_use]
pub fn render_with_sources(sources: &SourceMap, d: &SourceDiagnostic) -> Option<String> {
    let location = d.location()?;
    let source = sources.get(location.source)?;
    let text = source.text();
    if location.span.start > location.span.end
        || location.span.end > text.len()
        || !text.is_char_boundary(location.span.start)
        || !text.is_char_boundary(location.span.end)
    {
        return None;
    }
    let (line, col) = line_col(text, location.span.start);
    Some(format!(
        "{}:{line}:{col}: {}",
        source.name(),
        d.diagnostic()
    ))
}

/// A diagnostic together with the authoritative source map needed to render
/// its [`Location`].
///
/// This is the source-aware error boundary used by compilation/execution APIs.
/// Legacy APIs may still extract the underlying [`Diag`] and use its local
/// [`Span`], while callers that need provenance retain the map as well.
#[derive(Debug)]
pub struct DiagnosticReport {
    diagnostic: SourceDiagnostic,
    sources: Box<SourceMap>,
}

impl DiagnosticReport {
    /// Pair a diagnostic with the map that owns its source ids.
    #[must_use]
    pub fn new(diagnostic: SourceDiagnostic, sources: SourceMap) -> DiagnosticReport {
        DiagnosticReport {
            diagnostic,
            sources: Box::new(sources),
        }
    }

    /// The underlying diagnostic.
    #[must_use]
    pub const fn diagnostic(&self) -> &Diag {
        self.diagnostic.diagnostic()
    }

    /// Source-aware diagnostic payload.
    #[must_use]
    pub const fn source_diagnostic(&self) -> &SourceDiagnostic {
        &self.diagnostic
    }

    /// Source-aware location, when this report has one.
    #[must_use]
    pub fn location(&self) -> Option<Location> {
        self.diagnostic.location()
    }

    /// The authoritative source map for this report.
    #[must_use]
    pub const fn sources(&self) -> &SourceMap {
        &self.sources
    }

    /// Render with source name/line/column when the diagnostic has a valid
    /// location, otherwise fall back to the ordinary `E####: ...` form.
    #[must_use]
    pub fn render(&self) -> String {
        render_with_sources(&self.sources, &self.diagnostic)
            .unwrap_or_else(|| self.diagnostic.diagnostic().to_string())
    }

    /// Consume the report and return the compatibility diagnostic.
    #[must_use]
    pub fn into_diagnostic(self) -> Diag {
        self.diagnostic.into_diagnostic()
    }
}

impl fmt::Display for DiagnosticReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl std::error::Error for DiagnosticReport {}

/// Error codes, one constant per contract row.
pub mod codes {
    /// Invalid character in source.
    pub const INVALID_CHAR: u16 = 1001;
    /// Malformed numeric literal (e.g. `1abc`).
    pub const INVALID_NUMBER: u16 = 1002;
    /// Unknown escape sequence.
    pub const INVALID_ESCAPE: u16 = 1003;
    /// Unterminated string literal.
    pub const UNTERMINATED_STRING: u16 = 1004;
    /// Unterminated multiline comment (`<!--` without `--!>`).
    pub const UNTERMINATED_COMMENT: u16 = 1005;
    /// Expected a different token.
    pub const EXPECTED: u16 = 1006;
    /// An expression, block, or statement nests too deeply.
    pub const NESTING: u16 = 1015;
    /// A reserved word was used as a name.
    pub const RESERVED_NAME: u16 = 1009;
    /// Assignment to an immutable binding.
    pub const ASSIGN_IMMUTABLE: u16 = 2001;
    /// Undefined name.
    pub const UNDEFINED: u16 = 2003;
    /// `let` without an initializer.
    pub const LET_NO_INIT: u16 = 2005;
    /// Redeclaration in the same scope.
    pub const REDECLARED: u16 = 2007;
    /// A binding, parameter, pattern binding, or catch binding is declared
    /// but never used, and is not an explicit discard (`_`/`_name`).
    pub const UNUSED_BINDING: u16 = 2008;
    /// `_param` was used.
    pub const UNUSED_PARAM: u16 = 2009;
    /// Invalid assignment target.
    pub const INVALID_ASSIGN: u16 = 2010;
    /// `main` was declared with parameters or otherwise invalid.
    pub const INVALID_MAIN: u16 = 2011;
    /// A user type (struct, enum, alias) was declared twice.
    pub const DUPLICATE_TYPE: u16 = 2012;
    /// Two enums declare the same variant tag.
    pub const DUPLICATE_VARIANT: u16 = 2013;
    /// A pattern binds the same name more than once.
    pub const DUPLICATE_BINDING: u16 = 2014;
    /// `break` or `continue` used outside a loop.
    pub const LOOP_CONTROL: u16 = 2015;
    /// A struct declares the same field name more than once.
    pub const DUPLICATE_FIELD: u16 = 2016;
    /// A trait implementation omits a method the trait requires.
    pub const TRAIT_INCOMPLETE: u16 = 2017;
    /// A private item was accessed across a module boundary.
    pub const PRIVATE_ACCESS: u16 = 2018;
    /// An unknown module, or an unknown item in a `use` path.
    pub const UNKNOWN_MODULE: u16 = 2019;
    /// More than one source claims the same logical module.
    pub const MODULE_SOURCE_OWNERSHIP: u16 = 2020;
    /// One provider source is supplied for conflicting logical ownership.
    pub const DUPLICATE_LOGICAL_SOURCE: u16 = 2021;
    /// A provider/module source key or logical child path is invalid.
    pub const MODULE_SOURCE_PATH: u16 = 2022;
    /// A reserved namespace root was claimed by a user declaration (RFC 0001
    /// E6: `module Aura` cannot counterfeit builtin exception identity).
    pub const RESERVED_NAMESPACE: u16 = 2023;
    /// Type mismatch.
    pub const TYPE_MISMATCH: u16 = 3001;
    /// A member or element was accessed on a value that may be `none`
    /// (`T | none`) without narrowing it first (Keystone optionality).
    pub const POSSIBLE_NONE: u16 = 3003;
    /// Return type mismatch.
    pub const RETURN_MISMATCH: u16 = 3005;
    /// A function declared `-> never` can complete normally, contradicting
    /// its declared bottom return type (Keystone `never`).
    pub const NEVER_RETURNS: u16 = 3006;
    /// Value is not iterable.
    pub const NOT_ITERABLE: u16 = 4018;
    /// Integer overflow.
    pub const OVERFLOW: u16 = 4013;
    /// Division by zero.
    pub const DIV_ZERO: u16 = 4007;
    /// Uncaught thrown value.
    pub const FOREIGN: u16 = 4026;
    /// An in-flight `throw` crossing a call boundary (internal signal).
    pub const THROWN: u16 = 4099;
    /// Recursion limit.
    pub const RECURSION: u16 = 4011;
    /// Python bridge disabled or unsupported.
    pub const PY_UNSUPPORTED: u16 = 5002;
    /// A host capability is not available in this execution environment.
    /// Shares the code with [`PY_UNSUPPORTED`]: both mean "this build or host
    /// cannot provide the requested capability".
    pub const CAPABILITY_UNAVAILABLE: u16 = 5002;
    /// An optional standard-library feature is not compiled into this build.
    pub const FEATURE_UNAVAILABLE: u16 = 5003;
    /// Python error.
    pub const PY_ERROR: u16 = 5001;
    /// Missing `main`.
    pub const NO_MAIN: u16 = 4027;
    /// Unknown type name.
    pub const UNKNOWN_TYPE: u16 = 3002;
    /// Index out of range.
    pub const INDEX: u16 = 4019;
    /// An I/O operation failed (file read/write or standard input).
    pub const IO: u16 = 4020;
    /// A user assertion failed.
    pub const ASSERT: u16 = 4028;
    /// No `match` arm matched the value.
    pub const NO_MATCH: u16 = 4029;
    /// `return` appeared in a position where it cannot produce a value.
    pub const RETURN_POSITION: u16 = 4030;
    /// An internal invariant of the runtime was violated. This indicates a
    /// bug in Aura itself, never a user mistake.
    pub const INTERNAL: u16 = 4999;
}
