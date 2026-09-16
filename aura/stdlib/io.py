"""Aura Standard Library - File I/O module."""

import shutil as _shutil
from pathlib import Path as _Path

_ENCODING = 'utf-8'


def read(path):
    """Read entire file contents as a UTF-8 string."""
    return _Path(path).read_text(encoding=_ENCODING)


def write(path, content):
    """Write string content to file (creates/overwrites)."""
    _Path(path).write_text(content, encoding=_ENCODING)


def append(path, content):
    """Append string content to file."""
    with open(path, 'a', encoding=_ENCODING) as f:
        f.write(content)


def exists(path):
    """Check if file or directory exists."""
    return _Path(path).exists()


def is_file(path):
    """Check if path is a file."""
    return _Path(path).is_file()


def is_dir(path):
    """Check if path is a directory."""
    return _Path(path).is_dir()


def mkdir(path):
    """Create directory (including parents)."""
    _Path(path).mkdir(parents=True, exist_ok=True)


def ls(path="."):
    """List directory contents (names only, sorted for determinism)."""
    return sorted(entry.name for entry in _Path(path).iterdir())


def rm(path):
    """Remove a file. Missing files raise an error (use exists() to guard)."""
    _Path(path).unlink()


def rename(old, new):
    """Rename/move a file."""
    _Path(old).rename(new)


def extension(path):
    """Get file extension (without dot)."""
    return _Path(path).suffix.lstrip('.')


def basename(path):
    """Get filename without directory."""
    return _Path(path).name


def dirname(path):
    """Get directory part of path."""
    return str(_Path(path).parent)


def join(*parts):
    """Join path components."""
    return str(_Path(*parts))


def read_lines(path):
    """Read file as a list of lines (newlines removed, blank lines kept)."""
    text = _Path(path).read_text(encoding=_ENCODING)
    return text.splitlines()


def write_lines(path, lines):
    """Write an iterable of lines to file (adds a trailing newline).

    An empty iterable writes an empty file rather than a lone newline.
    """
    materialized = [str(line) for line in lines]
    text = '\n'.join(materialized)
    if materialized:
        text += '\n'
    _Path(path).write_text(text, encoding=_ENCODING)


def copy(src, dst):
    """Copy a file."""
    _shutil.copy2(src, dst)


def size(path):
    """Get file size in bytes."""
    return _Path(path).stat().st_size


def touch(path):
    """Create empty file or update timestamp."""
    _Path(path).touch()
