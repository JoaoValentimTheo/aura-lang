"""Aura Standard Library - String module."""

def upper(s):
    """Convert to uppercase."""
    return s.upper()

def lower(s):
    """Convert to lowercase."""
    return s.lower()

def title(s):
    """Convert to title case."""
    return s.title()

def capitalize(s):
    """Capitalize first character."""
    return s.capitalize()

def reverse(s):
    """Reverse string."""
    return s[::-1]

def trim(s):
    """Remove leading and trailing whitespace."""
    return s.strip()

def trim_left(s):
    """Remove leading whitespace."""
    return s.lstrip()

def trim_right(s):
    """Remove trailing whitespace."""
    return s.rstrip()

def _check_fill(char):
    if not isinstance(char, str) or len(char) != 1:
        raise ValueError("fill character must be a single character")
    return char

def pad_left(s, length, char=' '):
    """Pad string on left."""
    return s.rjust(length, _check_fill(char))

def pad_right(s, length, char=' '):
    """Pad string on right."""
    return s.ljust(length, _check_fill(char))

def pad(s, length, char=' '):
    """Pad string on both sides."""
    return s.center(length, _check_fill(char))

def repeat_string(s, times):
    """Repeat string."""
    return s * times

def split(s, separator=None, limit=None):
    """Split string.

    ``limit`` is the maximum number of splits (Python ``maxsplit``
    semantics), not a cap on the number of resulting parts.
    """
    if separator is None:
        return s.split(None, limit if limit is not None else -1)
    return s.split(separator, limit if limit is not None else -1)

def join(strings, separator=''):
    """Join strings."""
    return separator.join(str(s) for s in strings)

def starts_with(s, prefix):
    """Check if string starts with prefix."""
    return s.startswith(prefix)

def ends_with(s, suffix):
    """Check if string ends with suffix."""
    return s.endswith(suffix)

def contains(s, substring):
    """Check if string contains substring."""
    return substring in s

def index_of(s, substring):
    """Find index of substring."""
    try:
        return s.index(substring)
    except ValueError:
        return -1

def last_index_of(s, substring):
    """Find last index of substring."""
    try:
        return s.rindex(substring)
    except ValueError:
        return -1

def replace(s, old, new, count=None):
    """Replace substring."""
    if count is None:
        return s.replace(old, new)
    return s.replace(old, new, count)

def slice_string(s, start, end=None):
    """Slice string."""
    if end is None:
        return s[start:]
    return s[start:end]

def substring(s, start, length):
    """Get substring of length."""
    return s[start:start+length]

def char_at(s, index):
    """Get character at index."""
    return s[index] if 0 <= index < len(s) else None

def format_string(template, *args, **kwargs):
    """Format string with values."""
    return template.format(*args, **kwargs)

def is_empty(s):
    """Check if string is empty."""
    return len(s) == 0

def is_blank(s):
    """Check if string is blank (only whitespace)."""
    return len(s.strip()) == 0

def length(s):
    """Get string length."""
    return len(s)

def bytes_from_string(s, encoding='utf-8'):
    """Convert string to bytes."""
    return s.encode(encoding)

def string_from_bytes(b, encoding='utf-8'):
    """Convert bytes to string."""
    return b.decode(encoding)

def unicode_at(s, index):
    """Get Unicode code point at index."""
    return ord(s[index]) if 0 <= index < len(s) else None

def char_from_code(code):
    """Get character from Unicode code point."""
    return chr(code)

def is_alpha(s):
    """Check if string is alphabetic."""
    return s.isalpha()

def is_alphanumeric(s):
    """Check if string is alphanumeric."""
    return s.isalnum()

def is_digit(s):
    """Check if string is numeric."""
    return s.isdigit()

def is_space(s):
    """Check if string is whitespace."""
    return s.isspace()

def is_lower(s):
    """Check if string is lowercase."""
    return s.islower()

def is_upper(s):
    """Check if string is uppercase."""
    return s.isupper()

def is_numeric(s):
    """Check if string represents a finite number."""
    if not isinstance(s, str) or not s.strip():
        return False
    try:
        value = float(s)
    except ValueError:
        return False
    return value == value and value not in (float('inf'), float('-inf'))

def lines(s):
    """Split string into lines."""
    return s.splitlines()

def unlines(lines_list):
    """Join lines."""
    return '\n'.join(str(l) for l in lines_list)

def codes(s):
    """Get Unicode code points."""
    return [ord(c) for c in s]

def from_codes(codes_list):
    """Create string from Unicode code points."""
    return ''.join(chr(c) for c in codes_list)
