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
    """Aura error codes (from errors.md)."""
    # Syntax errors
    SYNTAX_ERROR = "E001"
    UNEXPECTED_TOKEN = "E002"
    UNEXPECTED_EOF = "E003"
    INVALID_SYNTAX = "E004"

    # Type errors
    TYPE_MISMATCH = "E101"
    UNDEFINED_VARIABLE = "E102"
    UNDEFINED_FUNCTION = "E103"
    UNDEFINED_CLASS = "E104"
    WRONG_ARGUMENT_COUNT = "E105"
    WRONG_ARGUMENT_TYPE = "E106"
    CANNOT_CALL_NON_FUNCTION = "E107"
    INCOMPATIBLE_OPERANDS = "E108"

    # Runtime errors
    DIVISION_BY_ZERO = "E201"
    NULL_POINTER = "E202"
    INDEX_OUT_OF_BOUNDS = "E203"
    KEY_ERROR = "E204"

    # Semantic errors
    DUPLICATE_DEFINITION = "E301"
    UNREACHABLE_CODE = "E302"
    INFINITE_LOOP = "E303"
    MISSING_RETURN = "E304"
    UNUSED_VARIABLE = "E305"
    UNUSED_IMPORT = "E306"
    MISSING_VISIBILITY = "E307"
    INACCESSIBLE_MEMBER = "E308"
    UNIMPLEMENTED_ABSTRACT = "E309"

    # Other
    IO_ERROR = "E401"
    CONFIGURATION_ERROR = "E402"
    FATAL = "E999"

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
        """Add warning."""
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
    ErrorCode.TYPE_MISMATCH: "Type mismatch: expected {expected}, got {actual}",
    ErrorCode.UNDEFINED_VARIABLE: "Undefined variable '{name}'",
    ErrorCode.UNDEFINED_FUNCTION: "Undefined function '{name}'",
    ErrorCode.UNDEFINED_CLASS: "Undefined class '{name}'",
    ErrorCode.WRONG_ARGUMENT_COUNT: "Function '{name}' expects {expected} arguments, got {actual}",
    ErrorCode.WRONG_ARGUMENT_TYPE: "Argument {index} of '{name}': expected {expected}, got {actual}",
    ErrorCode.CANNOT_CALL_NON_FUNCTION: "Cannot call non-function type '{type}'",
    ErrorCode.INCOMPATIBLE_OPERANDS: "Incompatible operands for {op}: {left} and {right}",
    ErrorCode.DUPLICATE_DEFINITION: "'{name}' is already defined",
    ErrorCode.MISSING_RETURN: "Function '{name}' must return a value of type {type}",
    ErrorCode.UNUSED_VARIABLE: "Variable '{name}' is declared but never used",
}

def format_error_message(code: ErrorCode, **kwargs) -> str:
    """Format error message from template."""
    if code in ERROR_TEMPLATES:
        try:
            return ERROR_TEMPLATES[code].format(**kwargs)
        except KeyError as e:
            return f"Error formatting message for {code}: missing key {e}"
    return str(code)
