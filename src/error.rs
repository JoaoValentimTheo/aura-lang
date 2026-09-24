//! Errors, source spans, and diagnostic codes.
//!
//! Every diagnostic has a stable `E####` code. The code is part of the
//! language contract: `tests/contract.rs` asserts that each one can be
//! produced by at least one program.

use std::fmt;

/// A byte span in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

/// A diagnostic: a code, a message, and a location.
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
    /// Type mismatch.
    pub const TYPE_MISMATCH: u16 = 3001;
    /// Return type mismatch.
    pub const RETURN_MISMATCH: u16 = 3005;
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
