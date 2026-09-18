"""Comprehensive error handling system for Aura transpiler."""
import sys
from contextlib import contextmanager
from dataclasses import dataclass, field
from enum import Enum

from aura.transpiler.ast import SourceLocation


@contextmanager
def recursion_budget(size: int):
    """Temporarily raise the interpreter recursion limit for a compiler stage.

    The checker and transformer visitors recurse once per AST level, so a
    deeply nested expression (a long operator chain or member chain) can
    exhaust Python's default limit and crash with a raw ``RecursionError``.
    The budget scales with the source size, and the original limit is always
    restored so a caller's limits are never permanently changed.
    """
    previous = sys.getrecursionlimit()
    needed = 2000 + max(0, size) * 4
    if needed > previous:
        sys.setrecursionlimit(needed)
    try:
        yield
    finally:
        sys.setrecursionlimit(previous)


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
    # The parser raises a structured ``SyntaxError`` (with line/column) rather
    # than an error code, so the E001-E003 codes were never emitted and have
    # been removed. Use the exception's ``line``/``column`` for positions.
    INVALID_SYNTAX = "E004"

    # -- Type errors (E1xx) ----------------------------------------------
    TYPE_MISMATCH = "E101"
    WRONG_ARGUMENT_COUNT = "E105"
    WRONG_ARGUMENT_TYPE = "E106"
    INCOMPATIBLE_OPERANDS = "E108"
    NON_EXHAUSTIVE_MATCH = "E109"
    UNKNOWN_TYPE_CONSTRAINT = "E110"

    # -- Semantic errors (E3xx) ------------------------------------------
    DUPLICATE_DEFINITION = "E301"
    UNREACHABLE_CODE = "E302"
    REASSIGN_IMMUTABLE = "E303"
    MISSING_VISIBILITY = "E307"
    INACCESSIBLE_MEMBER = "E308"
    UNIMPLEMENTED_ABSTRACT = "E309"
    MISSING_MAIN = "E310"
    INVALID_MAIN = "E311"
    MAIN_IN_MODULE = "E312"
    UNRESOLVED_REEXPORT = "E313"
    UNKNOWN_BASE_CLASS = "E314"
    INVALID_INHERITANCE = "E315"
    INSTANTIATE_ABSTRACT = "E316"
    SELF_IN_STATIC = "E317"
    UNKNOWN_LABEL = "E318"
    USED_BEFORE_DECLARED = "E319"
    DECORATOR_ON_FIELD = "E320"
    ABSTRACT_SUPER_CALL = "E321"

    # -- I/O and configuration (E4xx) ------------------------------------
    # The CLI reports I/O and configuration failures as plain messages plus a
    # non-zero exit code, so E401/E402 were never emitted and have been
    # removed.

    # -- Fatal (E9xx) ----------------------------------------------------
    FATAL = "E999"

    # -- Style warnings (W0xx) -------------------------------------------
    LINE_TOO_LONG = "W001"
    TRAILING_WHITESPACE = "W002"
    NAMING_CONVENTION = "W003"
    SPACING = "W004"

    # -- Unused / redundant warnings (W1xx) ------------------------------
    # W101/W102 (unused variable/import) were documented but never emitted;
    # they have been removed rather than shipped as a half-working analysis.
    UNUSED_TYPE_PARAMETER = "W103"

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

# ============================================================================
# Error message catalogue
#
# Canonical message templates for every code. Checkers may phrase a specific
# diagnostic differently, but the catalogue is the reference used by
# docs/ERRORS.md and is kept complete by tests/test_diagnostics.py.
# ============================================================================

ERROR_TEMPLATES = {
    # Syntax
    ErrorCode.INVALID_SYNTAX: "{detail}",
    # Types
    ErrorCode.TYPE_MISMATCH: "Type mismatch: expected {expected}, got {actual}",
    ErrorCode.WRONG_ARGUMENT_COUNT: "Function '{name}' expects {expected} arguments, got {actual}",
    ErrorCode.WRONG_ARGUMENT_TYPE: "Argument {index} of '{name}': expected {expected}, got {actual}",
    ErrorCode.INCOMPATIBLE_OPERANDS: "Incompatible operands for {op}: {left} and {right}",
    ErrorCode.NON_EXHAUSTIVE_MATCH: "'match' over {subject} is not exhaustive: no case handles {missing}",
    ErrorCode.UNKNOWN_TYPE_CONSTRAINT: "type parameter '{name}' has unknown constraint '{constraint}'",
    # Semantic
    ErrorCode.DUPLICATE_DEFINITION: "'{name}' is already defined",
    ErrorCode.UNREACHABLE_CODE: "Unreachable code after '{terminator}'",
    ErrorCode.REASSIGN_IMMUTABLE: "Cannot reassign immutable binding '{name}'",
    ErrorCode.MISSING_VISIBILITY: "{kind} '{name}' has no visibility modifier",
    ErrorCode.INACCESSIBLE_MEMBER: "'{name}' is {visibility} in '{owner}'",
    ErrorCode.UNIMPLEMENTED_ABSTRACT: "'{class}' must implement abstract method '{method}'",
    ErrorCode.MISSING_MAIN: "Program has no 'main' function",
    ErrorCode.INVALID_MAIN: "'main' must take no parameters, or a single 'args'",
    ErrorCode.MAIN_IN_MODULE: "'main' belongs to the entry file, not to a module or an imported file",
    ErrorCode.UNRESOLVED_REEXPORT: "no sibling source defines the re-exported name '{name}'",
    ErrorCode.UNKNOWN_BASE_CLASS: "base class '{name}' is not defined",
    ErrorCode.INVALID_INHERITANCE: "invalid inheritance for '{name}'",
    ErrorCode.INSTANTIATE_ABSTRACT: "'{name}' is abstract and cannot be instantiated",
    ErrorCode.SELF_IN_STATIC: "'{name}' is not available in a static method",
    ErrorCode.UNKNOWN_LABEL: "no enclosing loop is labeled '{label}'",
    ErrorCode.USED_BEFORE_DECLARED: "'{name}' is used before it is declared",
    ErrorCode.DECORATOR_ON_FIELD: "decorator '@{decorator}' cannot be applied to a field",
    ErrorCode.ABSTRACT_SUPER_CALL: "cannot call 'super.{name}' because it has no implementation",
    # I/O
    # Warnings
    ErrorCode.LINE_TOO_LONG: "Line {line} is too long ({length} > {limit} columns)",
    ErrorCode.TRAILING_WHITESPACE: "Line {line} has trailing whitespace",
    ErrorCode.NAMING_CONVENTION: "'{name}' does not follow the {convention} convention",
    ErrorCode.SPACING: "Multiple spaces after '{keyword}'",
    ErrorCode.UNUSED_TYPE_PARAMETER: "Type parameter '{name}' is never used",
    # Fatal
    ErrorCode.FATAL: "Too many errors; compilation stopped",
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
