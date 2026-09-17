"""Comprehensive error handling system for Aura transpiler."""
from dataclasses import dataclass, field
from enum import Enum

from aura.transpiler.ast import SourceLocation


class ErrorSeverity(Enum):
    """Error severity levels."""
    NOTE = "note"
    WARNING = "warning"
    ERROR = "error"
    FATAL = "fatal"

class ErrorCode(Enum):
    """Aura diagnostic codes.

    This enum mirrors ``docs/ERRORS.md``, the single source of truth. A test
    (``tests/test_diagnostics.py``) asserts the two stay in sync, so a code
    that is defined here must be documented there and vice versa.

    Numbering:

    * ``E0xx`` lexical / syntax
    * ``E1xx`` type
    * ``E3xx`` semantic / structural
    * ``E4xx`` I/O and configuration
    * ``E9xx`` fatal / internal
    * ``W0xx`` style warnings
    * ``W1xx`` unused / redundant warnings
    * ``W2xx`` semantic warnings (reserved)
    """

    # -- Syntax errors (E0xx) ---------------------------------------------
    SYNTAX_ERROR = "E001"
    UNEXPECTED_TOKEN = "E002"
    UNEXPECTED_EOF = "E003"
    INVALID_SYNTAX = "E004"

    # -- Type errors (E1xx) ----------------------------------------------
    TYPE_MISMATCH = "E101"
    UNDEFINED_VARIABLE = "E102"
    UNDEFINED_FUNCTION = "E103"
    UNDEFINED_CLASS = "E104"
    WRONG_ARGUMENT_COUNT = "E105"
    WRONG_ARGUMENT_TYPE = "E106"
    CANNOT_CALL_NON_FUNCTION = "E107"
    INCOMPATIBLE_OPERANDS = "E108"

    # -- Semantic errors (E3xx) ------------------------------------------
    DUPLICATE_DEFINITION = "E301"
    UNREACHABLE_CODE = "E302"
    MISSING_VISIBILITY = "E307"
    INACCESSIBLE_MEMBER = "E308"
    UNIMPLEMENTED_ABSTRACT = "E309"
    MISSING_MAIN = "E310"
    INVALID_MAIN = "E311"

    # -- I/O and configuration (E4xx) ------------------------------------
    IO_ERROR = "E401"
    CONFIGURATION_ERROR = "E402"

    # -- Fatal (E9xx) ----------------------------------------------------
    FATAL = "E999"

    # -- Style warnings (W0xx) -------------------------------------------
    LINE_TOO_LONG = "W001"
    TRAILING_WHITESPACE = "W002"
    NAMING_CONVENTION = "W003"
    SPACING = "W004"

    # -- Unused / redundant warnings (W1xx) ------------------------------
    UNUSED_VARIABLE = "W101"
    UNUSED_IMPORT = "W102"

@dataclass
class AuraError:
    """Single error in Aura code."""
    code: ErrorCode
    severity: ErrorSeverity
    message: str
    location: SourceLocation | None = None
    hint: str | None = None
    related: list[SourceLocation] = field(default_factory=list)

    def format(self) -> str:
        """Format error for display."""
        lines = []

        # Location line
        if self.location:
            lines.append(f"{self.location}: {self.severity.value.upper()} [{self.code.value}]")
        else:
            lines.append(f"{self.severity.value.upper()} [{self.code.value}]")

        # Message
        lines.append(f"  {self.message}")

        # Hint
        if self.hint:
            lines.append(f"  hint: {self.hint}")

        # Related locations
        if self.related:
            lines.append("  related:")
            for loc in self.related:
                lines.append(f"    {loc}")

        return "\n".join(lines)

    def __str__(self):
        return self.format()

class ErrorCollector:
    """Collect and manage errors during compilation."""

    def __init__(self, filename: str = ""):
        self.filename = filename
        self.errors: list[AuraError] = []
        self.max_errors = 10
        self.stopped = False

    def add(self, code: ErrorCode, message: str, location: SourceLocation | None = None,
            hint: str | None = None, severity: ErrorSeverity = ErrorSeverity.ERROR) -> None:
        """Add error, stopping after ``max_errors`` to bound collection."""
        if self.stopped:
            return
        if location and not location.filename:
            location.filename = self.filename

        self.errors.append(AuraError(code, severity, message, location, hint))

        if len(self.errors) >= self.max_errors:
            self.stopped = True
            self.errors.append(AuraError(
                ErrorCode.FATAL,
                ErrorSeverity.FATAL,
                f"Too many errors (max {self.max_errors}), stopping compilation"
            ))

    def add_warning(self, code: ErrorCode, message: str, location: SourceLocation | None = None,
                    hint: str | None = None) -> None:
        """Add a non-fatal diagnostic.

        The code must live in the ``W`` namespace; passing an ``E`` code is a
        programming error, because it would be reported as a warning under an
        error code (the historical linter bug this guards against).
        """
        if not code.value.startswith('W'):
            raise ValueError(
                f"add_warning requires a W-code, got {code.value} ({code.name})")
        self.add(code, message, location, hint, ErrorSeverity.WARNING)

    def has_errors(self) -> bool:
        """Check if any errors."""
        return any(e.severity in (ErrorSeverity.ERROR, ErrorSeverity.FATAL) for e in self.errors)

    def error_count(self) -> int:
        """Count errors."""
        return sum(1 for e in self.errors if e.severity == ErrorSeverity.ERROR)

    def warning_count(self) -> int:
        """Count warnings."""
        return sum(1 for e in self.errors if e.severity == ErrorSeverity.WARNING)

    def format(self) -> str:
        """Format all errors."""
        if not self.errors:
            return "No errors"

        lines = [str(e) for e in self.errors]
        lines.append(f"\n{self.error_count()} error(s), {self.warning_count()} warning(s)")
        return "\n".join(lines)

    def __str__(self):
        return self.format()

    def __bool__(self):
        return self.has_errors()

class CompilationException(Exception):
    """Raised when compilation has errors."""
    def __init__(self, collector: ErrorCollector):
        self.collector = collector
        super().__init__(f"Compilation failed: {collector.error_count()} errors")

    def __str__(self):
        return self.collector.format()

# ============================================================================
# Error message templates
# ============================================================================

ERROR_TEMPLATES = {
    # Syntax
    ErrorCode.SYNTAX_ERROR: "Invalid syntax",
    ErrorCode.UNEXPECTED_TOKEN: "Unexpected token '{token}'",
    ErrorCode.UNEXPECTED_EOF: "Unexpected end of file",
    ErrorCode.INVALID_SYNTAX: "{detail}",
    # Types
    ErrorCode.TYPE_MISMATCH: "Type mismatch: expected {expected}, got {actual}",
    ErrorCode.UNDEFINED_VARIABLE: "Undefined variable '{name}'",
    ErrorCode.UNDEFINED_FUNCTION: "Undefined function '{name}'",
    ErrorCode.UNDEFINED_CLASS: "Undefined class '{name}'",
    ErrorCode.WRONG_ARGUMENT_COUNT: "Function '{name}' expects {expected} arguments, got {actual}",
    ErrorCode.WRONG_ARGUMENT_TYPE: "Argument {index} of '{name}': expected {expected}, got {actual}",
    ErrorCode.CANNOT_CALL_NON_FUNCTION: "Cannot call non-function type '{type}'",
    ErrorCode.INCOMPATIBLE_OPERANDS: "Incompatible operands for {op}: {left} and {right}",
    # Semantic
    ErrorCode.DUPLICATE_DEFINITION: "'{name}' is already defined",
    ErrorCode.UNREACHABLE_CODE: "Unreachable code after '{terminator}'",
    ErrorCode.MISSING_VISIBILITY: "{kind} '{name}' has no visibility modifier",
    ErrorCode.INACCESSIBLE_MEMBER: "'{name}' is {visibility} in '{owner}'",
    ErrorCode.UNIMPLEMENTED_ABSTRACT: "'{class}' must implement abstract method '{method}'",
    ErrorCode.MISSING_MAIN: "Program has no 'main' function",
    ErrorCode.INVALID_MAIN: "'main' must take no parameters, or a single 'args'",
    # I/O
    ErrorCode.IO_ERROR: "{detail}",
    ErrorCode.CONFIGURATION_ERROR: "{detail}",
    # Warnings
    ErrorCode.LINE_TOO_LONG: "Line {line} is too long ({length} > {limit} columns)",
    ErrorCode.TRAILING_WHITESPACE: "Line {line} has trailing whitespace",
    ErrorCode.NAMING_CONVENTION: "'{name}' does not follow the {convention} convention",
    ErrorCode.SPACING: "Multiple spaces after '{keyword}'",
    ErrorCode.UNUSED_VARIABLE: "Variable '{name}' is declared but never used",
    ErrorCode.UNUSED_IMPORT: "Import '{name}' is never used",
}


def code_area(code: ErrorCode) -> str:
    """Return the human-readable area a code belongs to."""
    value = code.value
    if code in (ErrorCode.FATAL,):
        return "fatal"
    if value.startswith('W0'):
        return "style warning"
    if value.startswith('W1'):
        return "warning"
    if value.startswith('W'):
        return "semantic warning"
    prefix = value[1:2]
    return {
        '0': "syntax",
        '1': "type",
        '3': "semantic",
        '4': "configuration",
        '9': "fatal",
    }.get(prefix, "error")


def format_error_message(code: ErrorCode, **kwargs) -> str:
    """Format error message from template."""
    if code in ERROR_TEMPLATES:
        try:
            return ERROR_TEMPLATES[code].format(**kwargs)
        except KeyError as e:
            return f"Error formatting message for {code}: missing key {e}"
    return str(code)
